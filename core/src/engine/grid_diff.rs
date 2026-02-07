use crate::config::{DiffConfig, LimitBehavior};
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary};
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::progress::ProgressCallback;
use crate::sink::{DiffSink, SinkFinishGuard, VecSink};
use crate::string_pool::StringPool;
use crate::hashing::hash_row_content_128;
use crate::workbook::{CellAddress, CellContent, CellValue, Grid, GridStorage, RowSignature};
use std::collections::{HashMap, HashSet};

use crate::diff::SheetId;
use super::context::{DiffContext, EmitCtx};
use super::grid_primitives::{
    cells_content_equal, emit_cell_edit, positional_diff_for_rows,
    run_positional_diff_with_metrics,
};
use super::move_mask::SheetGridDiffer;

use crate::database_alignment::diff_table_by_key;
use crate::matching::hungarian;

const GRID_MODE_SHEET_ID: &str = "<grid>";
const DATABASE_MODE_SHEET_ID: &str = "<database>";
const DUPLICATE_CLUSTER_EXACT_MAX: usize = 16;
const DUPLICATE_MATCH_THRESHOLD: f64 = 0.5;
const TABLE_COLUMN_FILL_RATIO: f64 = 0.5;

pub fn diff_grids(
    old: &Grid,
    new: &Grid,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let mut sink = VecSink::new();
    match try_diff_grids_streaming(old, new, pool, config, &mut sink) {
        Ok(summary) => {
            let strings = pool.strings().to_vec();
            DiffReport::from_ops_and_summary(sink.into_ops(), summary, strings)
        }
        Err(e) => {
            let strings = pool.strings().to_vec();
            DiffReport {
                version: DiffReport::SCHEMA_VERSION.to_string(),
                strings,
                ops: sink.into_ops(),
                complete: false,
                warnings: vec![e.to_string()],
                #[cfg(feature = "perf-metrics")]
                metrics: None,
            }
        }
    }
}

pub fn try_diff_grids(
    old: &Grid,
    new: &Grid,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    let summary = try_diff_grids_streaming(old, new, pool, config, &mut sink)?;
    let strings = pool.strings().to_vec();
    Ok(DiffReport::from_ops_and_summary(
        sink.into_ops(),
        summary,
        strings,
    ))
}

/// Stream a grid diff into `sink`.
///
/// Streaming output follows the contract in `docs/streaming_contract.md`.
pub fn diff_grids_streaming<S: DiffSink>(
    old: &Grid,
    new: &Grid,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> DiffSummary {
    match try_diff_grids_streaming(old, new, pool, config, sink) {
        Ok(summary) => summary,
        Err(e) => DiffSummary {
            complete: false,
            warnings: vec![e.to_string()],
            op_count: 0,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        },
    }
}

pub fn diff_grids_streaming_with_progress<S: DiffSink>(
    old: &Grid,
    new: &Grid,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    progress: &dyn ProgressCallback,
) -> DiffSummary {
    match try_diff_grids_streaming_with_progress(old, new, pool, config, sink, progress) {
        Ok(summary) => summary,
        Err(e) => DiffSummary {
            complete: false,
            warnings: vec![e.to_string()],
            op_count: 0,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        },
    }
}

/// Like [`diff_grids_streaming`], but returns errors instead of embedding them in the summary.
pub fn try_diff_grids_streaming<S: DiffSink>(
    old: &Grid,
    new: &Grid,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    let mut op_count = 0usize;
    try_diff_grids_streaming_with_op_count(old, new, pool, config, sink, &mut op_count, None)
}

pub fn try_diff_grids_streaming_with_progress<S: DiffSink>(
    old: &Grid,
    new: &Grid,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    progress: &dyn ProgressCallback,
) -> Result<DiffSummary, DiffError> {
    let mut op_count = 0usize;
    try_diff_grids_streaming_with_op_count(
        old,
        new,
        pool,
        config,
        sink,
        &mut op_count,
        Some(progress),
    )
}

#[allow(clippy::too_many_arguments)]
fn try_diff_grids_streaming_with_op_count<'p, S: DiffSink>(
    old: &Grid,
    new: &Grid,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    op_count: &mut usize,
    progress: Option<&'p dyn ProgressCallback>,
) -> Result<DiffSummary, DiffError> {
    let sheet_id: SheetId = pool.intern(GRID_MODE_SHEET_ID);

    sink.begin(pool)?;
    let mut finish_guard = SinkFinishGuard::new(sink);

    let mut ctx = DiffContext::default();
    let mut hardening = super::hardening::HardeningController::new(config, progress);

    if hardening.check_timeout(&mut ctx.warnings) {
        finish_guard.finish_and_disarm()?;
        return Ok(DiffSummary {
            complete: false,
            warnings: ctx.warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    }

    try_diff_grids_internal(
        sheet_id,
        old,
        new,
        config,
        pool,
        sink,
        op_count,
        &mut ctx,
        &mut hardening,
        #[cfg(feature = "perf-metrics")]
        None,
    )?;

    finish_guard.finish_and_disarm()?;
    let complete = ctx.warnings.is_empty();
    Ok(DiffSummary {
        complete,
        warnings: ctx.warnings,
        op_count: *op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_diff_grids_internal<'p, S: DiffSink>(
    sheet_id: SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    hardening: &mut super::hardening::HardeningController<'p>,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == 0 && new.nrows == 0 {
        return Ok(());
    }

    if hardening.check_timeout(&mut ctx.warnings) {
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.rows_processed = m
            .rows_processed
            .saturating_add(old.nrows as u64)
            .saturating_add(new.nrows as u64);
    }

    let exceeds_limits = old.nrows.max(new.nrows) > config.alignment.max_align_rows
        || old.ncols.max(new.ncols) > config.alignment.max_align_cols;

    if exceeds_limits {
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            pool.resolve(sheet_id),
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.alignment.max_align_rows,
            config.alignment.max_align_cols
        );

        match config.hardening.on_limit_exceeded {
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id,
                    rows: old.nrows.max(new.nrows),
                    cols: old.ncols.max(new.ncols),
                    max_rows: config.alignment.max_align_rows,
                    max_cols: config.alignment.max_align_cols,
                });
            }
            behavior => {
                if matches!(behavior, LimitBehavior::ReturnPartialResult) {
                    ctx.warnings.push(warning);
                }

                let mut emit_ctx = EmitCtx::new(
                    sheet_id,
                    pool,
                    config,
                    &mut ctx.formula_cache,
                    sink,
                    op_count,
                    &mut ctx.warnings,
                    hardening,
                    #[cfg(feature = "perf-metrics")]
                    metrics.as_deref_mut(),
                );

                run_positional_diff_with_metrics(&mut emit_ctx, old, new)?;

                return Ok(());
            }
        }
    }

    diff_grids_core(
        sheet_id,
        old,
        new,
        config,
        pool,
        sink,
        op_count,
        ctx,
        hardening,
        #[cfg(feature = "perf-metrics")]
        metrics,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn diff_grids_core<'p, S: DiffSink>(
    sheet_id: SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    hardening: &mut super::hardening::HardeningController<'p>,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == new.nrows && old.ncols == new.ncols && grids_non_blank_cells_equal(old, new) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(cells_in_overlap(old, new));
        }
        return Ok(());
    }

    if hardening.check_timeout(&mut ctx.warnings) {
        return Ok(());
    }

    let sheet_name = pool.resolve(sheet_id);
    let context = format!("sheet '{sheet_name}'");
    if hardening.memory_guard_or_warn(
        crate::memory_estimate::estimate_advanced_sheet_diff_peak(old, new),
        &mut ctx.warnings,
        &context,
    ) {
        let mut emit_ctx = EmitCtx::new(
            sheet_id,
            pool,
            config,
            &mut ctx.formula_cache,
            sink,
            op_count,
            &mut ctx.warnings,
            hardening,
            #[cfg(feature = "perf-metrics")]
            metrics.as_deref_mut(),
        );
        run_positional_diff_with_metrics(&mut emit_ctx, old, new)?;
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::SignatureBuild);
    }
    let preflight = preflight_decision_from_grids(old, new, config);

    if matches!(
        preflight.decision,
        PreflightDecision::ShortCircuitNearIdentical | PreflightDecision::ShortCircuitDissimilar
    ) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::SignatureBuild);
        }
        let mut emit_ctx = EmitCtx::new(
            sheet_id,
            pool,
            config,
            &mut ctx.formula_cache,
            sink,
            op_count,
            &mut ctx.warnings,
            hardening,
            #[cfg(feature = "perf-metrics")]
            metrics.as_deref_mut(),
        );

        if preflight.decision == PreflightDecision::ShortCircuitNearIdentical
            && old.nrows == new.nrows
            && old.ncols == new.ncols
        {
            let rows = rows_with_context(
                &preflight.mismatched_rows,
                config.preflight.max_context_rows,
                old.nrows,
            );
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = emit_ctx.metrics.as_deref_mut() {
                m.start_phase(Phase::CellDiff);
            }
            let compared = positional_diff_for_rows(&mut emit_ctx, old, new, &rows)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = emit_ctx.metrics.as_deref_mut() {
                m.add_cells_compared(compared);
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(not(feature = "perf-metrics"))]
            let _ = compared;
        } else {
            run_positional_diff_with_metrics(&mut emit_ctx, old, new)?;
        }

        return Ok(());
    }

    #[cfg(feature = "parallel")]
    let (old_view, new_view) = rayon::join(
        || GridView::from_grid_with_config(old, config),
        || GridView::from_grid_with_config(new, config),
    );
    #[cfg(not(feature = "parallel"))]
    let old_view = GridView::from_grid_with_config(old, config);
    #[cfg(not(feature = "parallel"))]
    let new_view = GridView::from_grid_with_config(new, config);
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        let lookups = old.cell_count() as u64 + new.cell_count() as u64;
        let allocations = old.nrows as u64
            + old.ncols as u64
            + new.nrows as u64
            + new.ncols as u64;
        m.add_hash_lookups_est(lookups);
        m.add_allocations_est(allocations);
        m.end_phase(Phase::SignatureBuild);
    }

    let emit_ctx = EmitCtx::new(
        sheet_id,
        pool,
        config,
        &mut ctx.formula_cache,
        sink,
        op_count,
        &mut ctx.warnings,
        hardening,
        #[cfg(feature = "perf-metrics")]
        metrics.as_deref_mut(),
    );

    let mut differ = SheetGridDiffer::from_views(
        emit_ctx,
        old,
        new,
        old_view,
        new_view,
    );

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
        m.start_phase(Phase::MoveDetection);
    }

    differ.emit_ctx.hardening.progress("move_detection", 0.0);
    if differ.emit_ctx.hardening.check_timeout(differ.emit_ctx.warnings) {
        return Ok(());
    }
    differ.detect_moves()?;
    differ.emit_ctx.hardening.progress("move_detection", 1.0);

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
        m.end_phase(Phase::MoveDetection);
    }

    if differ.emit_ctx.hardening.should_abort() {
        return Ok(());
    }

    if differ.has_mask_exclusions() {
        differ.emit_ctx.hardening.progress("alignment", 1.0);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
            m.start_phase(Phase::CellDiff);
        }
        differ.diff_with_masks()?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
            m.end_phase(Phase::CellDiff);
        }
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
        m.start_phase(Phase::Alignment);
    }

    differ.emit_ctx.hardening.progress("alignment", 0.0);
    if differ.emit_ctx.hardening.check_timeout(differ.emit_ctx.warnings) {
        return Ok(());
    }
    if differ.try_amr()? {
        differ.emit_ctx.hardening.progress("alignment", 1.0);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(());
    }

    if differ.try_row_alignment()? {
        differ.emit_ctx.hardening.progress("alignment", 1.0);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(());
    }

    if differ.try_single_column_alignment()? {
        differ.emit_ctx.hardening.progress("alignment", 1.0);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(());
    }

    differ.positional()?;

    differ.emit_ctx.hardening.progress("alignment", 1.0);

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.emit_ctx.metrics.as_deref_mut() {
        m.end_phase(Phase::Alignment);
    }

    Ok(())
}

pub fn diff_grids_database_mode(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let sheet_id: SheetId = pool.intern(DATABASE_MODE_SHEET_ID);
    let mut sink = VecSink::new();
    let mut op_count = 0usize;
    match try_diff_grids_database_mode_streaming(
        sheet_id,
        old,
        new,
        key_columns,
        pool,
        config,
        &mut sink,
        &mut op_count,
    ) {
        Ok(summary) => {
            let strings = pool.strings().to_vec();
            DiffReport::from_ops_and_summary(sink.into_ops(), summary, strings)
        }
        Err(e) => {
            let strings = pool.strings().to_vec();
            DiffReport {
                version: DiffReport::SCHEMA_VERSION.to_string(),
                strings,
                ops: sink.into_ops(),
                complete: false,
                warnings: vec![e.to_string()],
                #[cfg(feature = "perf-metrics")]
                metrics: None,
            }
        }
    }
}

/// Stream a database-mode grid diff into `sink`.
///
/// Streaming output follows the contract in `docs/streaming_contract.md`.
pub fn try_diff_grids_database_mode_streaming<S: DiffSink>(
    sheet_id: SheetId,
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    op_count: &mut usize,
) -> Result<DiffSummary, DiffError> {
    let mut ctx = DiffContext::default();
    let mut hardening = super::hardening::HardeningController::new(config, None);

    sink.begin(pool)?;
    let mut finish_guard = SinkFinishGuard::new(sink);
    if hardening.check_timeout(&mut ctx.warnings) {
        finish_guard.finish_and_disarm()?;
        return Ok(DiffSummary {
            complete: false,
            warnings: ctx.warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    }

    if key_columns.is_empty() {
        ctx.warnings.push(
            "database-mode: no key columns provided; falling back to spreadsheet mode"
                .to_string(),
        );
        try_diff_grids_internal(
            sheet_id,
            old,
            new,
            config,
            pool,
            sink,
            op_count,
            &mut ctx,
            &mut hardening,
            #[cfg(feature = "perf-metrics")]
            None,
        )?;
        finish_guard.finish_and_disarm()?;
        let complete = ctx.warnings.is_empty();
        return Ok(DiffSummary {
            complete,
            warnings: ctx.warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    }

    if key_columns
        .iter()
        .any(|&col| col >= old.ncols || col >= new.ncols)
    {
        ctx.warnings.push(
            "database-mode: invalid key columns; falling back to spreadsheet mode".to_string(),
        );
        try_diff_grids_internal(
            sheet_id,
            old,
            new,
            config,
            pool,
            sink,
            op_count,
            &mut ctx,
            &mut hardening,
            #[cfg(feature = "perf-metrics")]
            None,
        )?;
        finish_guard.finish_and_disarm()?;
        let complete = ctx.warnings.is_empty();
        return Ok(DiffSummary {
            complete,
            warnings: ctx.warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    }

    let Some(table_scope) = build_table_scope(old, new, key_columns) else {
        ctx.warnings.push(
            "database-mode: no non-empty keys found; falling back to spreadsheet mode"
                .to_string(),
        );
        try_diff_grids_internal(
            sheet_id,
            old,
            new,
            config,
            pool,
            sink,
            op_count,
            &mut ctx,
            &mut hardening,
            #[cfg(feature = "perf-metrics")]
            None,
        )?;
        finish_guard.finish_and_disarm()?;
        let complete = ctx.warnings.is_empty();
        return Ok(DiffSummary {
            complete,
            warnings: ctx.warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    };

    let table_rows =
        table_scope.rows_old.len().max(table_scope.rows_new.len()) as u32;
    let table_cols = table_scope.cols_union.len() as u32;
    let exceeds_limits = table_rows > config.alignment.max_align_rows
        || table_cols > config.alignment.max_align_cols;

    if exceeds_limits {
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            pool.resolve(sheet_id),
            table_rows,
            table_cols,
            config.alignment.max_align_rows,
            config.alignment.max_align_cols
        );

        match config.hardening.on_limit_exceeded {
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id,
                    rows: table_rows,
                    cols: table_cols,
                    max_rows: config.alignment.max_align_rows,
                    max_cols: config.alignment.max_align_cols,
                });
            }
            behavior => {
                if matches!(behavior, LimitBehavior::ReturnPartialResult) {
                    ctx.warnings.push(warning);
                }

                let mut emit_ctx = EmitCtx::new(
                    sheet_id,
                    pool,
                    config,
                    &mut ctx.formula_cache,
                    sink,
                    op_count,
                    &mut ctx.warnings,
                    &mut hardening,
                    #[cfg(feature = "perf-metrics")]
                    None,
                );
                run_positional_diff_with_metrics(&mut emit_ctx, old, new)?;
                finish_guard.finish_and_disarm()?;
                let complete = ctx.warnings.is_empty();
                return Ok(DiffSummary {
                    complete,
                    warnings: ctx.warnings,
                    op_count: *op_count,
                    #[cfg(feature = "perf-metrics")]
                    metrics: None,
                });
            }
        }
    }

    let (table_old, row_map_old) =
        build_table_grid(old, &table_scope.rows_old, &table_scope.cols_union);
    let (table_new, row_map_new) =
        build_table_grid(new, &table_scope.rows_new, &table_scope.cols_union);

    let Some(table_key_cols) = map_key_columns(key_columns, &table_scope.cols_union) else {
        ctx.warnings.push(
            "database-mode: invalid key columns; falling back to spreadsheet mode".to_string(),
        );
        try_diff_grids_internal(
            sheet_id,
            old,
            new,
            config,
            pool,
            sink,
            op_count,
            &mut ctx,
            &mut hardening,
            #[cfg(feature = "perf-metrics")]
            None,
        )?;
        finish_guard.finish_and_disarm()?;
        let complete = ctx.warnings.is_empty();
        return Ok(DiffSummary {
            complete,
            warnings: ctx.warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    };

    let key_col_set: HashSet<u32> = table_key_cols.iter().copied().collect();
    let max_cols = table_old.ncols.max(table_new.ncols);
    let compare_cols: Vec<u32> = (0..max_cols)
        .filter(|col| !key_col_set.contains(col))
        .collect();

    let alignment = diff_table_by_key(&table_old, &table_new, &table_key_cols);

    {
        let mut emit_ctx = EmitCtx::new(
            sheet_id,
            pool,
            config,
            &mut ctx.formula_cache,
            sink,
            op_count,
            &mut ctx.warnings,
            &mut hardening,
            #[cfg(feature = "perf-metrics")]
            None,
        );
        let should_abort = |emit_ctx: &mut EmitCtx<'_, '_, S>| {
            let hardening = &mut *emit_ctx.hardening;
            let warnings = &mut *emit_ctx.warnings;
            hardening.check_timeout(warnings) || hardening.should_abort()
        };

        for row_idx in &alignment.left_only_rows {
            if should_abort(&mut emit_ctx) {
                break;
            }
            if let Some(row) = row_map_old.get(*row_idx as usize).copied() {
                emit_ctx.emit(DiffOp::row_removed(sheet_id, row, None))?;
            }
        }

        for row_idx in &alignment.right_only_rows {
            if should_abort(&mut emit_ctx) {
                break;
            }
            if let Some(row) = row_map_new.get(*row_idx as usize).copied() {
                emit_ctx.emit(DiffOp::row_added(sheet_id, row, None))?;
            }
        }

        for (row_a, row_b) in &alignment.matched_rows {
            if should_abort(&mut emit_ctx) {
                break;
            }
            let Some(row_a_orig) = row_map_old.get(*row_a as usize).copied() else {
                continue;
            };
            let Some(row_b_orig) = row_map_new.get(*row_b as usize).copied() else {
                continue;
            };
            let row_shift = row_b_orig as i32 - row_a_orig as i32;

            for col in 0..max_cols {
                if key_col_set.contains(&col) {
                    continue;
                }

                let old_cell = table_old.get(*row_a, col);
                let new_cell = table_new.get(*row_b, col);

                if cells_content_equal(old_cell, new_cell) {
                    continue;
                }

                let Some(col_orig) = table_scope.cols_union.get(col as usize).copied() else {
                    continue;
                };
                let addr = CellAddress::from_indices(row_b_orig, col_orig);
                emit_cell_edit(&mut emit_ctx, addr, old_cell, new_cell, row_shift, 0)?;
            }
        }

        for cluster in &alignment.duplicate_clusters {
            if should_abort(&mut emit_ctx) {
                break;
            }

            let left_rows: Vec<u32> = cluster
                .left_rows
                .iter()
                .filter_map(|idx| row_map_old.get(*idx as usize).copied())
                .collect();
            let right_rows: Vec<u32> = cluster
                .right_rows
                .iter()
                .filter_map(|idx| row_map_new.get(*idx as usize).copied())
                .collect();

            emit_ctx.emit(DiffOp::DuplicateKeyCluster {
                sheet: sheet_id,
                key: cluster.key.as_cell_values(),
                left_rows: left_rows.clone(),
                right_rows: right_rows.clone(),
            })?;

            let cluster_match = match_duplicate_cluster(
                &table_old,
                &table_new,
                &cluster.left_rows,
                &cluster.right_rows,
                &compare_cols,
            );

            for (row_a, row_b) in cluster_match.matched {
                if should_abort(&mut emit_ctx) {
                    break;
                }
                let Some(row_a_orig) = row_map_old.get(row_a as usize).copied() else {
                    continue;
                };
                let Some(row_b_orig) = row_map_new.get(row_b as usize).copied() else {
                    continue;
                };
                let row_shift = row_b_orig as i32 - row_a_orig as i32;

                for col in 0..max_cols {
                    if key_col_set.contains(&col) {
                        continue;
                    }

                    let old_cell = table_old.get(row_a, col);
                    let new_cell = table_new.get(row_b, col);
                    if cells_content_equal(old_cell, new_cell) {
                        continue;
                    }
                    let Some(col_orig) = table_scope.cols_union.get(col as usize).copied() else {
                        continue;
                    };
                    let addr = CellAddress::from_indices(row_b_orig, col_orig);
                    emit_cell_edit(&mut emit_ctx, addr, old_cell, new_cell, row_shift, 0)?;
                }
            }

            for row_idx in cluster_match.left_unmatched {
                if should_abort(&mut emit_ctx) {
                    break;
                }
                if let Some(row) = row_map_old.get(row_idx as usize).copied() {
                    emit_ctx.emit(DiffOp::row_removed(sheet_id, row, None))?;
                }
            }

            for row_idx in cluster_match.right_unmatched {
                if should_abort(&mut emit_ctx) {
                    break;
                }
                if let Some(row) = row_map_new.get(row_idx as usize).copied() {
                    emit_ctx.emit(DiffOp::row_added(sheet_id, row, None))?;
                }
            }
        }
    }

    if !hardening.should_abort()
        && (has_cells_outside_rect(
            old,
            table_scope.row_start,
            table_scope.row_end,
            table_scope.col_start,
            table_scope.col_end,
        ) || has_cells_outside_rect(
            new,
            table_scope.row_start,
            table_scope.row_end,
            table_scope.col_start,
            table_scope.col_end,
        ))
    {
        let old_free = mask_grid_excluding_rect(
            old,
            table_scope.row_start,
            table_scope.row_end,
            table_scope.col_start,
            table_scope.col_end,
        );
        let new_free = mask_grid_excluding_rect(
            new,
            table_scope.row_start,
            table_scope.row_end,
            table_scope.col_start,
            table_scope.col_end,
        );

        if !hardening.check_timeout(&mut ctx.warnings) {
            let mut emit_ctx = EmitCtx::new(
                sheet_id,
                pool,
                config,
                &mut ctx.formula_cache,
                sink,
                op_count,
                &mut ctx.warnings,
                &mut hardening,
                #[cfg(feature = "perf-metrics")]
                None,
            );
            run_positional_diff_with_metrics(&mut emit_ctx, &old_free, &new_free)?;
        }
    }

    finish_guard.finish_and_disarm()?;
    Ok(DiffSummary {
        complete: ctx.warnings.is_empty(),
        warnings: ctx.warnings,
        op_count: *op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: None,
    })
}

#[derive(Debug, Clone)]
struct TableScope {
    rows_old: Vec<u32>,
    rows_new: Vec<u32>,
    cols_union: Vec<u32>,
    row_start: u32,
    row_end: u32,
    col_start: u32,
    col_end: u32,
}

fn build_table_scope(old: &Grid, new: &Grid, key_columns: &[u32]) -> Option<TableScope> {
    let rows_old = table_rows_for_grid(old, key_columns);
    let rows_new = table_rows_for_grid(new, key_columns);
    let rows_union = union_sorted(&rows_old, &rows_new);
    if rows_union.is_empty() {
        return None;
    }

    let cols_old = table_cols_for_grid(old, &rows_old, key_columns);
    let cols_new = table_cols_for_grid(new, &rows_new, key_columns);
    let cols_union = union_sorted(&cols_old, &cols_new);
    if cols_union.is_empty() {
        return None;
    }

    let row_start = *rows_union.first()?;
    let row_end = *rows_union.last()?;
    let col_start = *cols_union.first()?;
    let col_end = *cols_union.last()?;

    Some(TableScope {
        rows_old,
        rows_new,
        cols_union,
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

fn table_rows_for_grid(grid: &Grid, key_columns: &[u32]) -> Vec<u32> {
    if grid.nrows == 0 || grid.ncols == 0 || key_columns.is_empty() {
        return Vec::new();
    }
    if key_columns.iter().any(|&col| col >= grid.ncols) {
        return Vec::new();
    }

    let mut rows = Vec::new();
    'row: for row in 0..grid.nrows {
        for &col in key_columns {
            if !cell_value_is_non_empty(grid.get(row, col)) {
                continue 'row;
            }
        }
        rows.push(row);
    }
    rows
}

fn table_cols_for_grid(grid: &Grid, table_rows: &[u32], key_columns: &[u32]) -> Vec<u32> {
    if grid.ncols == 0 {
        return Vec::new();
    }

    let mut cols: HashSet<u32> = HashSet::new();
    for &col in key_columns {
        if col < grid.ncols {
            cols.insert(col);
        }
    }

    let row_count = table_rows.len();
    if row_count == 0 {
        let mut out: Vec<u32> = cols.into_iter().collect();
        out.sort_unstable();
        return out;
    }

    for col in 0..grid.ncols {
        let mut filled = 0usize;
        for &row in table_rows {
            if grid.get(row, col).is_some() {
                filled += 1;
            }
        }
        let ratio = filled as f64 / row_count as f64;
        if ratio >= TABLE_COLUMN_FILL_RATIO {
            cols.insert(col);
        }
    }

    let mut out: Vec<u32> = cols.into_iter().collect();
    out.sort_unstable();
    out
}

fn union_sorted(left: &[u32], right: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = left.iter().copied().chain(right.iter().copied()).collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn cell_value_is_non_empty(cell: Option<&crate::workbook::Cell>) -> bool {
    match cell.and_then(|cell| cell.value.as_ref()) {
        Some(CellValue::Blank) | None => false,
        Some(_) => true,
    }
}

fn map_key_columns(key_columns: &[u32], cols_union: &[u32]) -> Option<Vec<u32>> {
    let mut mapped = Vec::with_capacity(key_columns.len());
    for &col in key_columns {
        let idx = cols_union.iter().position(|&c| c == col)? as u32;
        mapped.push(idx);
    }
    Some(mapped)
}

fn build_table_grid(grid: &Grid, rows: &[u32], cols: &[u32]) -> (Grid, Vec<u32>) {
    let mut table = Grid::new(rows.len() as u32, cols.len() as u32);
    for (row_idx, &row) in rows.iter().enumerate() {
        for (col_idx, &col) in cols.iter().enumerate() {
            if let Some(cell) = grid.get(row, col) {
                table.insert_cell(
                    row_idx as u32,
                    col_idx as u32,
                    cell.value.clone(),
                    cell.formula,
                );
            }
        }
    }
    (table, rows.to_vec())
}

fn has_cells_outside_rect(
    grid: &Grid,
    row_start: u32,
    row_end: u32,
    col_start: u32,
    col_end: u32,
) -> bool {
    for ((row, col), _) in grid.iter_cells() {
        if row < row_start || row > row_end || col < col_start || col > col_end {
            return true;
        }
    }
    false
}

fn mask_grid_excluding_rect(
    grid: &Grid,
    row_start: u32,
    row_end: u32,
    col_start: u32,
    col_end: u32,
) -> Grid {
    let mut out = Grid::new(grid.nrows, grid.ncols);
    for ((row, col), cell) in grid.iter_cells() {
        if row < row_start || row > row_end || col < col_start || col > col_end {
            out.insert_cell(row, col, cell.value.clone(), cell.formula);
        }
    }
    out
}

#[derive(Debug, Default)]
struct ClusterMatch {
    matched: Vec<(u32, u32)>,
    left_unmatched: Vec<u32>,
    right_unmatched: Vec<u32>,
}

fn match_duplicate_cluster(
    old: &Grid,
    new: &Grid,
    left_rows: &[u32],
    right_rows: &[u32],
    compare_cols: &[u32],
) -> ClusterMatch {
    if left_rows.is_empty() && right_rows.is_empty() {
        return ClusterMatch::default();
    }
    if left_rows.is_empty() {
        return ClusterMatch {
            matched: Vec::new(),
            left_unmatched: Vec::new(),
            right_unmatched: right_rows.to_vec(),
        };
    }
    if right_rows.is_empty() {
        return ClusterMatch {
            matched: Vec::new(),
            left_unmatched: left_rows.to_vec(),
            right_unmatched: Vec::new(),
        };
    }

    let unmatched_cost = duplicate_unmatched_cost(compare_cols.len());

    let mut costs: Vec<Vec<i64>> = Vec::with_capacity(left_rows.len());
    for &left_row in left_rows {
        let mut row_costs = Vec::with_capacity(right_rows.len());
        for &right_row in right_rows {
            row_costs.push(duplicate_pair_cost(old, new, left_row, right_row, compare_cols));
        }
        costs.push(row_costs);
    }

    if left_rows.len().max(right_rows.len()) <= DUPLICATE_CLUSTER_EXACT_MAX {
        let assignment = hungarian::solve_rect(&costs, unmatched_cost);
        let mut right_used = vec![false; right_rows.len()];
        let mut matched = Vec::new();
        let mut left_unmatched = Vec::new();

        for (row_idx, &col_idx) in assignment.iter().take(left_rows.len()).enumerate() {
            if col_idx >= right_rows.len() {
                left_unmatched.push(left_rows[row_idx]);
                continue;
            }
            let cost = costs
                .get(row_idx)
                .and_then(|row| row.get(col_idx))
                .copied()
                .unwrap_or(unmatched_cost);
            if cost >= unmatched_cost {
                left_unmatched.push(left_rows[row_idx]);
                continue;
            }
            if !right_used[col_idx] {
                matched.push((left_rows[row_idx], right_rows[col_idx]));
                right_used[col_idx] = true;
            }
        }

        let mut right_unmatched = Vec::new();
        for (idx, &row) in right_rows.iter().enumerate() {
            if !right_used[idx] {
                right_unmatched.push(row);
            }
        }

        return ClusterMatch {
            matched,
            left_unmatched,
            right_unmatched,
        };
    }

    let mut candidates = Vec::new();
    for (left_idx, _) in left_rows.iter().enumerate() {
        for (right_idx, _) in right_rows.iter().enumerate() {
            let cost = costs[left_idx][right_idx];
            if cost < unmatched_cost {
                candidates.push((cost, left_idx, right_idx));
            }
        }
    }
    candidates.sort_by(|a, b| a.cmp(b));

    let mut left_used = vec![false; left_rows.len()];
    let mut right_used = vec![false; right_rows.len()];
    let mut matched = Vec::new();

    for (_, left_idx, right_idx) in candidates {
        if left_used[left_idx] || right_used[right_idx] {
            continue;
        }
        left_used[left_idx] = true;
        right_used[right_idx] = true;
        matched.push((left_rows[left_idx], right_rows[right_idx]));
    }

    let mut left_unmatched = Vec::new();
    for (idx, &row) in left_rows.iter().enumerate() {
        if !left_used[idx] {
            left_unmatched.push(row);
        }
    }
    let mut right_unmatched = Vec::new();
    for (idx, &row) in right_rows.iter().enumerate() {
        if !right_used[idx] {
            right_unmatched.push(row);
        }
    }

    ClusterMatch {
        matched,
        left_unmatched,
        right_unmatched,
    }
}

fn duplicate_unmatched_cost(compare_cols_len: usize) -> i64 {
    if compare_cols_len == 0 {
        return 1;
    }
    let threshold = DUPLICATE_MATCH_THRESHOLD.clamp(0.0, 1.0);
    let max_mismatches =
        ((compare_cols_len as f64) * (1.0 - threshold)).floor() as i64;
    (max_mismatches + 1).max(1)
}

fn duplicate_pair_cost(
    old: &Grid,
    new: &Grid,
    left_row: u32,
    right_row: u32,
    compare_cols: &[u32],
) -> i64 {
    let mut cost = 0i64;
    for &col in compare_cols {
        let old_cell = old.get(left_row, col);
        let new_cell = new.get(right_row, col);
        if !cells_content_equal(old_cell, new_cell) {
            cost += 1;
        }
    }
    cost
}

fn grids_non_blank_cells_equal(old: &Grid, new: &Grid) -> bool {
    if old.cell_count() != new.cell_count() {
        return false;
    }

    if old.cell_count() == 0 {
        return true;
    }

    if let (Some(old_sigs), Some(new_sigs)) = (&old.row_signatures, &new.row_signatures) {
        if old_sigs != new_sigs {
            return false;
        }
        if let (Some(old_col_sigs), Some(new_col_sigs)) = (&old.col_signatures, &new.col_signatures)
        {
            if old_col_sigs != new_col_sigs {
                return false;
            }
            return true;
        }
    }

    old.cells_equal(&new.cells)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreflightDecision {
    RunFullPipeline,
    ShortCircuitNearIdentical,
    ShortCircuitDissimilar,
}

#[derive(Debug)]
struct PreflightLite {
    decision: PreflightDecision,
    mismatched_rows: Vec<u32>,
}

fn preflight_decision_from_grids(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> PreflightLite {
    let nrows_old = old.nrows as usize;
    let nrows_new = new.nrows as usize;
    let ncols_old = old.ncols as usize;
    let ncols_new = new.ncols as usize;

    if nrows_old != nrows_new || ncols_old != ncols_new {
        return PreflightLite {
            decision: PreflightDecision::RunFullPipeline,
            mismatched_rows: Vec::new(),
        };
    }

    let nrows = nrows_old;
    if nrows < config.preflight.preflight_min_rows as usize {
        return PreflightLite {
            decision: PreflightDecision::RunFullPipeline,
            mismatched_rows: Vec::new(),
        };
    }

    let dense_scan = should_use_dense_row_scan(old, new);
    if dense_scan {
        let max_mismatches = config.preflight.preflight_in_order_mismatch_max as usize;
        let mismatched_rows =
            mismatched_rows_by_content_limit(old, new, max_mismatches.saturating_add(1));
        let in_order_mismatches = mismatched_rows.len();
        let in_order_matches = nrows.saturating_sub(in_order_mismatches);
        let in_order_match_ratio = if nrows > 0 {
            in_order_matches as f64 / nrows as f64
        } else {
            1.0
        };

        if in_order_mismatches <= max_mismatches
            && in_order_match_ratio >= config.preflight.preflight_in_order_match_ratio_min
        {
            let (multiset_equal, multiset_edit_distance_rows) =
                multiset_equal_and_edit_distance_for_mismatched_rows(old, new, &mismatched_rows);

            let reorder_suspected = (in_order_mismatches as u64) > multiset_edit_distance_rows;
            let near_identical = !multiset_equal && !reorder_suspected;

            if near_identical {
                return PreflightLite {
                    decision: PreflightDecision::ShortCircuitNearIdentical,
                    mismatched_rows,
                };
            }
        }
    }

    let sample_rows = sample_row_indices(old.nrows, 4096);
    let jaccard = sample_row_signature_jaccard(old, new, &sample_rows);

    if jaccard < config.preflight.bailout_similarity_threshold {
        return PreflightLite {
            decision: PreflightDecision::ShortCircuitDissimilar,
            mismatched_rows: Vec::new(),
        };
    }

    if !dense_scan {
        let old_signatures = row_signatures_for_grid(old);
        let new_signatures = row_signatures_for_grid(new);

        let (in_order_matches, old_sig_set, new_sig_set) =
            compute_row_signature_stats(&old_signatures, &new_signatures);

        let in_order_mismatches = nrows.saturating_sub(in_order_matches);
        let in_order_match_ratio = if nrows > 0 {
            in_order_matches as f64 / nrows as f64
        } else {
            1.0
        };

        let intersection_size = old_sig_set.intersection(&new_sig_set).count();
        let union_size = old_sig_set.union(&new_sig_set).count();
        let jaccard = if union_size > 0 {
            intersection_size as f64 / union_size as f64
        } else {
            1.0
        };

        if jaccard < config.preflight.bailout_similarity_threshold {
            return PreflightLite {
                decision: PreflightDecision::ShortCircuitDissimilar,
                mismatched_rows: Vec::new(),
            };
        }

        let (multiset_equal, multiset_edit_distance_rows) =
            multiset_equal_and_edit_distance(&old_signatures, &new_signatures);

        let reorder_suspected = (in_order_mismatches as u64) > multiset_edit_distance_rows;

        let near_identical = in_order_mismatches
            <= config.preflight.preflight_in_order_mismatch_max as usize
            && in_order_match_ratio >= config.preflight.preflight_in_order_match_ratio_min
            && !multiset_equal
            && !reorder_suspected;

        if near_identical {
            return PreflightLite {
                decision: PreflightDecision::ShortCircuitNearIdentical,
                mismatched_rows: mismatched_rows_from_signatures(&old_signatures, &new_signatures),
            };
        }
    }

    PreflightLite {
        decision: PreflightDecision::RunFullPipeline,
        mismatched_rows: Vec::new(),
    }
}

fn should_use_dense_row_scan(old: &Grid, new: &Grid) -> bool {
    let max_cells = (old.nrows as u64).saturating_mul(old.ncols as u64);
    if max_cells == 0 {
        return false;
    }
    let old_dense = (old.cell_count() as u64) > (max_cells / 2);
    let new_dense = (new.cell_count() as u64) > (max_cells / 2);
    old_dense && new_dense
}

fn mismatched_rows_by_content_limit(old: &Grid, new: &Grid, max_out: usize) -> Vec<u32> {
    let mut out = Vec::new();
    for row in 0..old.nrows {
        if !rows_content_equal(old, new, row) {
            out.push(row);
            if out.len() >= max_out {
                break;
            }
        }
    }
    out
}

fn rows_content_equal(old: &Grid, new: &Grid, row: u32) -> bool {
    for col in 0..old.ncols {
        if !cells_content_equal(old.get(row, col), new.get(row, col)) {
            return false;
        }
    }
    true
}

fn multiset_equal_and_edit_distance_for_mismatched_rows(
    old: &Grid,
    new: &Grid,
    mismatched_rows: &[u32],
) -> (bool, u64) {
    let mut delta: HashMap<RowSignature, i32> = HashMap::new();
    for &row in mismatched_rows {
        let a = old.compute_row_signature(row);
        let b = new.compute_row_signature(row);
        *delta.entry(a).or_insert(0) += 1;
        *delta.entry(b).or_insert(0) -= 1;
    }

    let mut equal = true;
    let mut sum_abs: u64 = 0;
    for (_sig, d) in delta {
        if d != 0 {
            equal = false;
            sum_abs = sum_abs.saturating_add(d.unsigned_abs() as u64);
        }
    }

    (equal, sum_abs / 2)
}

fn sample_row_indices(nrows: u32, max_samples: usize) -> Vec<u32> {
    let n = nrows as usize;
    if n == 0 {
        return Vec::new();
    }
    let target = max_samples.min(n);
    let step = (n / target).max(1);

    let mut out = Vec::with_capacity(target + 1);
    let mut idx = 0usize;
    while idx < n && out.len() < target {
        out.push(idx as u32);
        idx = idx.saturating_add(step);
    }

    if *out.last().unwrap_or(&0) != nrows.saturating_sub(1) {
        out.push(nrows.saturating_sub(1));
    }

    out.sort_unstable();
    out.dedup();
    out
}

fn sample_row_signature_jaccard(old: &Grid, new: &Grid, sample_rows: &[u32]) -> f64 {
    let mut a = HashSet::with_capacity(sample_rows.len());
    let mut b = HashSet::with_capacity(sample_rows.len());

    for &row in sample_rows {
        a.insert(old.compute_row_signature(row));
        b.insert(new.compute_row_signature(row));
    }

    let intersection = a.intersection(&b).count();
    let union = a.len() + b.len() - intersection;
    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

fn compute_row_signature_stats(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> (usize, HashSet<RowSignature>, HashSet<RowSignature>) {
    let mut in_order_matches = 0usize;
    let mut old_sig_set = HashSet::with_capacity(old_signatures.len());
    let mut new_sig_set = HashSet::with_capacity(new_signatures.len());

    for (old_sig, new_sig) in old_signatures.iter().zip(new_signatures.iter()) {
        if old_sig == new_sig {
            in_order_matches += 1;
        }
        old_sig_set.insert(*old_sig);
        new_sig_set.insert(*new_sig);
    }

    (in_order_matches, old_sig_set, new_sig_set)
}

fn multiset_equal_and_edit_distance(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> (bool, u64) {
    let mut delta: HashMap<RowSignature, i32> = HashMap::new();
    for sig in old_signatures {
        *delta.entry(*sig).or_insert(0) += 1;
    }
    for sig in new_signatures {
        *delta.entry(*sig).or_insert(0) -= 1;
    }

    let mut equal = true;
    let mut sum_abs: u64 = 0;
    for (_sig, d) in delta {
        if d != 0 {
            equal = false;
            sum_abs = sum_abs.saturating_add(d.unsigned_abs() as u64);
        }
    }

    (equal, sum_abs / 2)
}

fn mismatched_rows_from_signatures(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> Vec<u32> {
    let mut rows = Vec::new();
    let count = old_signatures.len().min(new_signatures.len());
    for idx in 0..count {
        let a = old_signatures[idx];
        let b = new_signatures[idx];
        if a != b {
            rows.push(idx as u32);
        }
    }
    rows
}

fn row_signatures_for_grid(grid: &Grid) -> Vec<RowSignature> {
    if let Some(sigs) = &grid.row_signatures {
        return sigs.clone();
    }
    match &grid.cells {
        GridStorage::Dense(_) => {
            let mut out = Vec::with_capacity(grid.nrows as usize);
            for row in 0..grid.nrows {
                out.push(grid.compute_row_signature(row));
            }
            out
        }
        GridStorage::Sparse(map) => {
            let nrows = grid.nrows as usize;
            let mut row_counts = vec![0usize; nrows];
            for ((row, _col), _cell) in map.iter() {
                let idx = *row as usize;
                if idx < nrows {
                    row_counts[idx] = row_counts[idx].saturating_add(1);
                }
            }

            let mut row_cells: Vec<Vec<(u32, &CellContent)>> = row_counts
                .iter()
                .map(|count| Vec::with_capacity(*count))
                .collect();

            for ((row, col), cell) in map.iter() {
                let idx = *row as usize;
                if idx < nrows {
                    row_cells[idx].push((*col, cell));
                }
            }

            for row in row_cells.iter_mut() {
                row.sort_unstable_by_key(|(col, _)| *col);
            }

            row_cells
                .iter()
                .map(|row| RowSignature {
                    hash: hash_row_content_128(row),
                })
                .collect()
        }
    }
}

fn rows_with_context(rows: &[u32], context: u32, max_rows: u32) -> Vec<u32> {
    if rows.is_empty() || context == 0 {
        return rows.to_vec();
    }

    let mut out = Vec::new();
    for &row in rows {
        let start = row.saturating_sub(context);
        let end = row.saturating_add(context).min(max_rows.saturating_sub(1));
        for r in start..=end {
            out.push(r);
        }
    }

    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(feature = "perf-metrics")]
fn cells_in_overlap(old: &Grid, new: &Grid) -> u64 {
    let overlap_rows = old.nrows.min(new.nrows) as u64;
    let overlap_cols = old.ncols.min(new.ncols) as u64;
    overlap_rows.saturating_mul(overlap_cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::VecSink;
    use crate::string_pool::StringPool;
    use crate::workbook::{Cell, CellValue};

    fn numbered_cell(value: f64) -> Cell {
        Cell {
            value: Some(CellValue::Number(value)),
            formula: None,
        }
    }

    #[test]
    fn grids_non_blank_cells_equal_requires_matching_entries() {
        let base_cell = Cell {
            value: Some(CellValue::Number(1.0)),
            formula: None,
        };

        let mut grid_a = Grid::new(2, 2);
        let mut grid_b = Grid::new(2, 2);
        grid_a.insert_cell(0, 0, base_cell.value.clone(), base_cell.formula);
        grid_b.insert_cell(0, 0, base_cell.value.clone(), base_cell.formula);

        assert!(grids_non_blank_cells_equal(&grid_a, &grid_b));

        let mut grid_b_changed = grid_b.clone();
        let mut changed_cell = base_cell.clone();
        changed_cell.value = Some(CellValue::Number(2.0));
        grid_b_changed.insert_cell(0, 0, changed_cell.value.clone(), changed_cell.formula);

        assert!(!grids_non_blank_cells_equal(&grid_a, &grid_b_changed));

        grid_a.insert_cell(1, 1, Some(CellValue::Blank), None);

        assert!(!grids_non_blank_cells_equal(&grid_a, &grid_b));
    }

    #[test]
    fn diff_row_pair_sparse_counts_union_columns_not_sum_lengths() {
        use super::super::grid_primitives::diff_row_pair_sparse;
        use crate::formula_diff::FormulaParseCache;

        let mut pool = StringPool::new();
        let sheet_id: SheetId = pool.intern("Sheet1");
        let config = DiffConfig::default();
        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let mut cache = FormulaParseCache::default();
        let mut warnings: Vec<String> = Vec::new();
        let mut hardening = super::super::hardening::HardeningController::new(&config, None);

        let old_cells_storage = [numbered_cell(1.0), numbered_cell(2.0), numbered_cell(3.0)];
        let new_cells_storage = [numbered_cell(1.0), numbered_cell(2.0), numbered_cell(4.0)];

        let old_cells: Vec<(u32, &Cell)> = old_cells_storage
            .iter()
            .enumerate()
            .map(|(idx, cell)| (idx as u32, cell))
            .collect();
        let new_cells: Vec<(u32, &Cell)> = new_cells_storage
            .iter()
            .enumerate()
            .map(|(idx, cell)| (idx as u32, cell))
            .collect();

        let mut emit_ctx = EmitCtx::new(
            sheet_id,
            &pool,
            &config,
            &mut cache,
            &mut sink,
            &mut op_count,
            &mut warnings,
            &mut hardening,
            #[cfg(feature = "perf-metrics")]
            None,
        );
        let result = diff_row_pair_sparse(&mut emit_ctx, 0, 0, 3, &old_cells, &new_cells)
            .expect("diff should succeed");

        assert_eq!(result.compared, 3);
        assert!(!result.replaced);
    }

    #[test]
    fn diff_row_pair_sparse_counts_union_for_sparse_columns() {
        use super::super::grid_primitives::diff_row_pair_sparse;
        use crate::formula_diff::FormulaParseCache;

        let mut pool = StringPool::new();
        let sheet_id: SheetId = pool.intern("Sheet1");
        let config = DiffConfig::default();
        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let mut cache = FormulaParseCache::default();
        let mut warnings: Vec<String> = Vec::new();
        let mut hardening = super::super::hardening::HardeningController::new(&config, None);

        let old_cells_storage = [numbered_cell(1.0)];
        let new_cells_storage = [numbered_cell(2.0)];

        let old_cells: Vec<(u32, &Cell)> = vec![(0, &old_cells_storage[0])];
        let new_cells: Vec<(u32, &Cell)> = vec![(2, &new_cells_storage[0])];

        let mut emit_ctx = EmitCtx::new(
            sheet_id,
            &pool,
            &config,
            &mut cache,
            &mut sink,
            &mut op_count,
            &mut warnings,
            &mut hardening,
            #[cfg(feature = "perf-metrics")]
            None,
        );
        let result = diff_row_pair_sparse(&mut emit_ctx, 0, 0, 3, &old_cells, &new_cells)
            .expect("diff should succeed");

        assert_eq!(result.compared, 2);
        assert!(!result.replaced);
    }

    #[test]
    fn preflight_detects_row_swap_with_edit_as_not_near_identical() {
        let mut grid_a = Grid::new(6000, 10);
        let mut grid_b = Grid::new(6000, 10);

        for row in 0..6000u32 {
            for col in 0..10u32 {
                let value = (row * 1000 + col) as f64;
                grid_a.insert_cell(row, col, Some(CellValue::Number(value)), None);
                grid_b.insert_cell(row, col, Some(CellValue::Number(value)), None);
            }
        }

        let row_0_cells: Vec<_> = (0..10u32).map(|c| c as f64).collect();
        let row_1_cells: Vec<_> = (0..10u32).map(|c| 1000.0 + c as f64).collect();

        for col in 0..10u32 {
            grid_b.insert_cell(0, col, Some(CellValue::Number(row_1_cells[col as usize])), None);
            grid_b.insert_cell(1, col, Some(CellValue::Number(row_0_cells[col as usize])), None);
        }

        grid_b.insert_cell(2999, 5, Some(CellValue::Number(999999.0)), None);

        let config = DiffConfig::default();
        let decision = preflight_decision_from_grids(&grid_a, &grid_b, &config);

        assert_eq!(
            decision.decision,
            PreflightDecision::RunFullPipeline,
            "small row swap + edit should NOT short-circuit to near-identical"
        );
    }

    #[test]
    fn preflight_short_circuit_dissimilar_skips_gridview_build() {
        use crate::grid_view::{gridview_build_count, reset_gridview_build_count};

        let mut grid_a = Grid::new(200, 10);
        let mut grid_b = Grid::new(200, 10);

        for row in 0..200u32 {
            for col in 0..10u32 {
                let value = (row * 1000 + col) as f64;
                grid_a.insert_cell(row, col, Some(CellValue::Number(value)), None);
                grid_b.insert_cell(row, col, Some(CellValue::Number(value + 1.0)), None);
            }
        }

        let config = DiffConfig::builder()
            .preflight_min_rows(0)
            .bailout_similarity_threshold(0.05)
            .build()
            .expect("valid config");

        let mut pool = StringPool::new();
        let sheet_id: SheetId = pool.intern("PreflightTest");
        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let mut ctx = DiffContext::default();
        let mut hardening = super::super::hardening::HardeningController::new(&config, None);

        reset_gridview_build_count();

        #[cfg(feature = "perf-metrics")]
        {
            diff_grids_core(
                sheet_id,
                &grid_a,
                &grid_b,
                &config,
                &pool,
                &mut sink,
                &mut op_count,
                &mut ctx,
                &mut hardening,
                None,
            )
            .expect("diff should succeed");
        }

        #[cfg(not(feature = "perf-metrics"))]
        {
            diff_grids_core(
                sheet_id,
                &grid_a,
                &grid_b,
                &config,
                &pool,
                &mut sink,
                &mut op_count,
                &mut ctx,
                &mut hardening,
            )
            .expect("diff should succeed");
        }

        assert_eq!(
            gridview_build_count(),
            0,
            "low-similarity preflight should avoid GridView construction"
        );
    }

    #[test]
    fn multiset_edit_distance_computes_correctly() {
        let mut grid_a = Grid::new(10, 5);
        let mut grid_b = Grid::new(10, 5);

        for row in 0..10u32 {
            for col in 0..5u32 {
                grid_a.insert_cell(row, col, Some(CellValue::Number((row * 100 + col) as f64)), None);
            }
        }

        for row in 0..10u32 {
            for col in 0..5u32 {
                grid_b.insert_cell(row, col, Some(CellValue::Number((row * 100 + col) as f64)), None);
            }
        }

        let old_signatures = row_signatures_for_grid(&grid_a);
        let new_signatures = row_signatures_for_grid(&grid_b);

        let (equal, edit_distance) =
            multiset_equal_and_edit_distance(&old_signatures, &new_signatures);
        assert!(equal);
        assert_eq!(edit_distance, 0);

        for col in 0..5u32 {
            grid_b.insert_cell(0, col, Some(CellValue::Number(99999.0 + col as f64)), None);
        }

        let new_signatures_edited = row_signatures_for_grid(&grid_b);
        let (equal2, edit_distance2) =
            multiset_equal_and_edit_distance(&old_signatures, &new_signatures_edited);
        assert!(!equal2);
        assert_eq!(edit_distance2, 1);
    }
}
