use crate::config::{DiffConfig, LimitBehavior};
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary};
use crate::formula_diff::FormulaParseCache;
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::progress::ProgressCallback;
use crate::sink::{DiffSink, SinkFinishGuard, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::{Grid, RowSignature};
use std::collections::{HashMap, HashSet};

use crate::diff::SheetId;
use super::context::{DiffContext, EmitCtx, emit_op};
use super::grid_primitives::{
    cells_content_equal, compute_formula_diff, positional_diff_for_rows,
    run_positional_diff_with_metrics, snapshot_with_addr,
};
use super::move_mask::SheetGridDiffer;

use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};

const GRID_MODE_SHEET_ID: &str = "<grid>";
const DATABASE_MODE_SHEET_ID: &str = "<database>";

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

    let old_view = GridView::from_grid_with_config(old, config);
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
    let mut warnings: Vec<String> = Vec::new();
    let mut hardening = super::hardening::HardeningController::new(config, None);
    let mut formula_cache = FormulaParseCache::default();
    let spec = KeyColumnSpec::new(key_columns.to_vec());

    sink.begin(pool)?;
    let mut finish_guard = SinkFinishGuard::new(sink);
    if hardening.check_timeout(&mut warnings) {
        finish_guard.finish_and_disarm()?;
        return Ok(DiffSummary {
            complete: false,
            warnings,
            op_count: *op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        });
    }

    let alignment = match diff_table_by_key(old, new, key_columns) {
        Ok(alignment) => alignment,
        Err(_) => {
            let mut ctx = DiffContext::default();
            warnings.push(
                "database-mode: duplicate keys for requested columns; falling back to spreadsheet mode"
                    .to_string(),
            );
            ctx.warnings = warnings;
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
    };

    let max_cols = old.ncols.max(new.ncols);

    for row_idx in &alignment.left_only_rows {
        if hardening.check_timeout(&mut warnings) {
            break;
        }
        emit_op(
            sink,
            op_count,
            DiffOp::row_removed(sheet_id, *row_idx, None),
        )?;
    }

    for row_idx in &alignment.right_only_rows {
        if hardening.check_timeout(&mut warnings) {
            break;
        }
        emit_op(sink, op_count, DiffOp::row_added(sheet_id, *row_idx, None))?;
    }

    for (row_a, row_b) in &alignment.matched_rows {
        if hardening.check_timeout(&mut warnings) {
            break;
        }
        for col in 0..max_cols {
            if spec.is_key_column(col) {
                continue;
            }

            let old_cell = old.get(*row_a, col);
            let new_cell = new.get(*row_b, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = crate::workbook::CellAddress::from_indices(*row_b, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            let formula_diff = compute_formula_diff(
                pool,
                &mut formula_cache,
                old_cell,
                new_cell,
                *row_b as i32 - *row_a as i32,
                0,
                config,
            );

            emit_op(
                sink,
                op_count,
                DiffOp::cell_edited(sheet_id, addr, from, to, formula_diff),
            )?;
        }
    }

    finish_guard.finish_and_disarm()?;
    Ok(DiffSummary {
        complete: warnings.is_empty(),
        warnings,
        op_count: *op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: None,
    })
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

    PreflightLite {
        decision: PreflightDecision::RunFullPipeline,
        mismatched_rows: Vec::new(),
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
    let mut out = Vec::with_capacity(grid.nrows as usize);
    for row in 0..grid.nrows {
        out.push(grid.compute_row_signature(row));
    }
    out
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
