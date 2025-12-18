//! Core diffing engine for workbook comparison.
//!
//! Provides the main entry point [`diff_workbooks`] for comparing two workbooks
//! and generating a [`DiffReport`] of all changes.

use crate::alignment::align_rows_amr_with_signatures_from_views;
use crate::alignment::move_extraction::moves_from_matched_pairs;
use crate::alignment_types::{RowAlignment, RowBlockMove};
use crate::column_alignment::{
    ColumnAlignment, ColumnBlockMove, align_single_column_change_from_views,
    detect_exact_column_block_move,
};
use crate::config::{DiffConfig, LimitBehavior};
use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary, FormulaDiffResult, SheetId};
use crate::formula_diff::{FormulaParseCache, diff_cell_formulas_ids};
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase, PhaseGuard};
use crate::rect_block_move::{RectBlockMove, detect_exact_rect_block_move};
use crate::region_mask::RegionMask;
use crate::row_alignment::{
    align_row_changes_from_views, detect_exact_row_block_move, detect_fuzzy_row_block_move,
};
use crate::sink::{DiffSink, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::{
    Cell, CellAddress, CellSnapshot, ColSignature, Grid, RowSignature, Sheet, SheetKind, Workbook,
};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Default)]
struct DiffContext {
    warnings: Vec<String>,
    formula_cache: FormulaParseCache,
}

const DATABASE_MODE_SHEET_ID: &str = "<database>";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

fn make_sheet_key(sheet: &Sheet, pool: &StringPool) -> SheetKey {
    SheetKey {
        name_lower: pool.resolve(sheet.name).to_lowercase(),
        kind: sheet.kind.clone(),
    }
}

fn sheet_kind_order(kind: &SheetKind) -> u8 {
    match kind {
        SheetKind::Worksheet => 0,
        SheetKind::Chart => 1,
        SheetKind::Macro => 2,
        SheetKind::Other => 3,
    }
}

fn emit_op<S: DiffSink>(sink: &mut S, op_count: &mut usize, op: DiffOp) -> Result<(), DiffError> {
    sink.emit(op)?;
    *op_count = op_count.saturating_add(1);
    Ok(())
}

struct EmitCtx<'a, S: DiffSink> {
    sheet_id: &'a SheetId,
    pool: &'a StringPool,
    config: &'a DiffConfig,
    cache: &'a mut FormulaParseCache,
    sink: &'a mut S,
    op_count: &'a mut usize,
}

impl<'a, S: DiffSink> EmitCtx<'a, S> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        emit_op(self.sink, self.op_count, op)
    }
}

struct SheetGridDiffer<'a, 'b, S: DiffSink> {
    emit_ctx: EmitCtx<'a, S>,
    old: &'b Grid,
    new: &'b Grid,
    old_view: GridView<'b>,
    new_view: GridView<'b>,
    old_mask: RegionMask,
    new_mask: RegionMask,
    #[cfg(feature = "perf-metrics")]
    metrics: Option<&'a mut DiffMetrics>,
}

impl<'a, 'b, S: DiffSink> SheetGridDiffer<'a, 'b, S> {
    fn new(
        sheet_id: &'a SheetId,
        old: &'b Grid,
        new: &'b Grid,
        config: &'a DiffConfig,
        pool: &'a StringPool,
        cache: &'a mut FormulaParseCache,
        sink: &'a mut S,
        op_count: &'a mut usize,
        #[cfg(feature = "perf-metrics")] metrics: Option<&'a mut DiffMetrics>,
    ) -> Self {
        let old_view = GridView::from_grid_with_config(old, config);
        let new_view = GridView::from_grid_with_config(new, config);
        let old_mask = RegionMask::all_active(old.nrows, old.ncols);
        let new_mask = RegionMask::all_active(new.nrows, new.ncols);

        Self {
            emit_ctx: EmitCtx {
                sheet_id,
                pool,
                config,
                cache,
                sink,
                op_count,
            },
            old,
            new,
            old_view,
            new_view,
            old_mask,
            new_mask,
            #[cfg(feature = "perf-metrics")]
            metrics,
        }
    }

    fn move_detection_enabled(&self) -> bool {
        self.old.nrows.max(self.new.nrows) <= self.emit_ctx.config.max_move_detection_rows
            && self.old.ncols.max(self.new.ncols) <= self.emit_ctx.config.max_move_detection_cols
    }

    fn detect_moves(&mut self) -> Result<u32, DiffError> {
        if !self.move_detection_enabled() {
            return Ok(0);
        }

        let mut iteration = 0u32;
        let config = self.emit_ctx.config;

        loop {
            if iteration >= config.max_move_iterations {
                break;
            }

            if !self.old_mask.has_active_cells() || !self.new_mask.has_active_cells() {
                break;
            }

            let mut found_move = false;

            if let Some(mv) = detect_exact_rect_block_move_masked(
                self.old,
                self.new,
                &self.old_mask,
                &self.new_mask,
                config,
            ) {
                emit_rect_block_move(&mut self.emit_ctx, mv)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = self.metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                self.old_mask.exclude_rect_cells(
                    mv.src_start_row,
                    mv.src_row_count,
                    mv.src_start_col,
                    mv.src_col_count,
                );
                self.new_mask.exclude_rect_cells(
                    mv.dst_start_row,
                    mv.src_row_count,
                    mv.dst_start_col,
                    mv.src_col_count,
                );
                self.old_mask.exclude_rect_cells(
                    mv.dst_start_row,
                    mv.src_row_count,
                    mv.dst_start_col,
                    mv.src_col_count,
                );
                self.new_mask.exclude_rect_cells(
                    mv.src_start_row,
                    mv.src_row_count,
                    mv.src_start_col,
                    mv.src_col_count,
                );
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && let Some(mv) = detect_exact_row_block_move_masked(
                    self.old,
                    self.new,
                    &self.old_mask,
                    &self.new_mask,
                    config,
                )
            {
                emit_row_block_move(&mut self.emit_ctx, mv)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = self.metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                self.old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                self.new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && let Some(mv) = detect_exact_column_block_move_masked(
                    self.old,
                    self.new,
                    &self.old_mask,
                    &self.new_mask,
                    config,
                )
            {
                emit_column_block_move(&mut self.emit_ctx, mv)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = self.metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                self.old_mask.exclude_cols(mv.src_start_col, mv.col_count);
                self.new_mask.exclude_cols(mv.dst_start_col, mv.col_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && config.enable_fuzzy_moves
                && let Some(mv) = detect_fuzzy_row_block_move_masked(
                    self.old,
                    self.new,
                    &self.old_mask,
                    &self.new_mask,
                    config,
                )
            {
                emit_row_block_move(&mut self.emit_ctx, mv)?;
                emit_moved_row_block_edits(&mut self.emit_ctx, &self.old_view, &self.new_view, mv)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = self.metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                self.old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                self.new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move {
                break;
            }

            if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
                break;
            }
        }

        Ok(iteration)
    }

    fn has_mask_exclusions(&self) -> bool {
        self.old_mask.has_exclusions() || self.new_mask.has_exclusions()
    }

    fn diff_with_masks(&mut self) -> Result<bool, DiffError> {
        if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
            if diff_aligned_with_masks(
                &mut self.emit_ctx,
                self.old,
                self.new,
                &self.old_mask,
                &self.new_mask,
            )? {
                return Ok(true);
            }
            positional_diff_with_masks(
                &mut self.emit_ctx,
                self.old,
                self.new,
                &self.old_mask,
                &self.new_mask,
            )?;
        } else {
            positional_diff_masked_equal_size(
                &mut self.emit_ctx,
                self.old,
                self.new,
                &self.old_mask,
                &self.new_mask,
            )?;
        }
        Ok(true)
    }
}

fn compute_formula_diff(
    pool: &StringPool,
    cache: &mut FormulaParseCache,
    old_cell: Option<&Cell>,
    new_cell: Option<&Cell>,
    row_shift: i32,
    col_shift: i32,
    config: &DiffConfig,
) -> FormulaDiffResult {
    let old_f = old_cell.and_then(|c| c.formula);
    let new_f = new_cell.and_then(|c| c.formula);
    diff_cell_formulas_ids(pool, cache, old_f, new_f, row_shift, col_shift, config)
}

fn emit_cell_edit<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    addr: CellAddress,
    old_cell: Option<&Cell>,
    new_cell: Option<&Cell>,
    row_shift: i32,
    col_shift: i32,
) -> Result<(), DiffError> {
    let from = snapshot_with_addr(old_cell, addr);
    let to = snapshot_with_addr(new_cell, addr);
    let formula_diff = compute_formula_diff(
        ctx.pool, ctx.cache, old_cell, new_cell, row_shift, col_shift, ctx.config,
    );
    ctx.emit(DiffOp::cell_edited(
        ctx.sheet_id.clone(),
        addr,
        from,
        to,
        formula_diff,
    ))
}

pub fn diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    match try_diff_workbooks(old, new, pool, config) {
        Ok(report) => report,
        Err(e) => panic!("{}", e),
    }
}

pub fn diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> DiffSummary {
    match try_diff_workbooks_streaming(old, new, pool, config, sink) {
        Ok(summary) => summary,
        Err(e) => panic!("{}", e),
    }
}

pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut sink = VecSink::new();
    let summary = try_diff_workbooks_streaming(old, new, pool, config, &mut sink)?;
    let ops = sink.into_ops();
    let mut report = DiffReport::new(ops);
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    #[cfg(feature = "perf-metrics")]
    {
        report.metrics = summary.metrics;
    }
    report.strings = pool.strings().to_vec();
    Ok(report)
}

pub fn try_diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    sink.begin(pool)?;

    let mut ctx = DiffContext::default();
    let mut op_count = 0usize;
    #[cfg(feature = "perf-metrics")]
    let mut metrics = {
        let mut m = DiffMetrics::default();
        m.start_phase(Phase::Total);
        m
    };

    let mut old_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        let key = make_sheet_key(sheet, pool);
        let was_unique = old_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in old workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut new_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        let key = make_sheet_key(sheet, pool);
        let was_unique = new_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in new workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut all_keys: Vec<SheetKey> = old_sheets
        .keys()
        .chain(new_sheets.keys())
        .cloned()
        .collect();
    all_keys.sort_by(|a, b| match a.name_lower.cmp(&b.name_lower) {
        std::cmp::Ordering::Equal => sheet_kind_order(&a.kind).cmp(&sheet_kind_order(&b.kind)),
        other => other,
    });
    all_keys.dedup();

    for key in all_keys {
        match (old_sheets.get(&key), new_sheets.get(&key)) {
            (None, Some(new_sheet)) => {
                emit_op(
                    sink,
                    &mut op_count,
                    DiffOp::SheetAdded {
                        sheet: new_sheet.name,
                    },
                )?;
            }
            (Some(old_sheet), None) => {
                emit_op(
                    sink,
                    &mut op_count,
                    DiffOp::SheetRemoved {
                        sheet: old_sheet.name,
                    },
                )?;
            }
            (Some(old_sheet), Some(new_sheet)) => {
                let sheet_id: SheetId = old_sheet.name;
                try_diff_grids(
                    &sheet_id,
                    &old_sheet.grid,
                    &new_sheet.grid,
                    config,
                    pool,
                    sink,
                    &mut op_count,
                    &mut ctx,
                    #[cfg(feature = "perf-metrics")]
                    Some(&mut metrics),
                )?;
            }
            (None, None) => unreachable!(),
        }
    }

    #[cfg(feature = "perf-metrics")]
    {
        metrics.end_phase(Phase::Total);
    }
    sink.finish()?;
    let complete = ctx.warnings.is_empty();
    Ok(DiffSummary {
        complete,
        warnings: ctx.warnings,
        op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: Some(metrics),
    })
}

pub fn diff_grids_database_mode(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> DiffReport {
    let mut sink = VecSink::new();
    let mut op_count = 0usize;
    let summary = diff_grids_database_mode_streaming(
        old,
        new,
        key_columns,
        pool,
        config,
        &mut sink,
        &mut op_count,
    )
    .unwrap_or_else(|e| panic!("{}", e));
    let mut report = DiffReport::new(sink.into_ops());
    report.complete = summary.complete;
    report.warnings = summary.warnings;
    report.strings = pool.strings().to_vec();
    report
}

fn diff_grids_database_mode_streaming<S: DiffSink>(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
    op_count: &mut usize,
) -> Result<DiffSummary, DiffError> {
    let mut formula_cache = FormulaParseCache::default();
    let spec = KeyColumnSpec::new(key_columns.to_vec());
    let alignment = match diff_table_by_key(old, new, key_columns) {
        Ok(alignment) => alignment,
        Err(_) => {
            let sheet_id: SheetId = pool.intern(DATABASE_MODE_SHEET_ID);
            sink.begin(pool)?;
            let mut ctx = DiffContext::default();
            try_diff_grids(
                &sheet_id,
                old,
                new,
                config,
                pool,
                sink,
                op_count,
                &mut ctx,
                #[cfg(feature = "perf-metrics")]
                None,
            )?;
            sink.finish()?;
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

    let sheet_id: SheetId = pool.intern(DATABASE_MODE_SHEET_ID);
    sink.begin(pool)?;
    let max_cols = old.ncols.max(new.ncols);

    for row_idx in &alignment.left_only_rows {
        emit_op(
            sink,
            op_count,
            DiffOp::row_removed(sheet_id, *row_idx, None),
        )?;
    }

    for row_idx in &alignment.right_only_rows {
        emit_op(sink, op_count, DiffOp::row_added(sheet_id, *row_idx, None))?;
    }

    for (row_a, row_b) in &alignment.matched_rows {
        for col in 0..max_cols {
            if spec.is_key_column(col) {
                continue;
            }

            let old_cell = old.get(*row_a, col);
            let new_cell = new.get(*row_b, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(*row_b, col);
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

    sink.finish()?;
    Ok(DiffSummary {
        complete: true,
        warnings: Vec::new(),
        op_count: *op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: None,
    })
}

fn try_diff_grids<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == 0 && new.nrows == 0 {
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.rows_processed = m
            .rows_processed
            .saturating_add(old.nrows as u64)
            .saturating_add(new.nrows as u64);
        m.start_phase(Phase::MoveDetection);
    }

    let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
        || old.ncols.max(new.ncols) > config.max_align_cols;
    if exceeds_limits {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            pool.resolve(*sheet_id),
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.max_align_rows,
            config.max_align_cols
        );
        match config.on_limit_exceeded {
            LimitBehavior::FallbackToPositional => {
                let mut emit_ctx = EmitCtx {
                    sheet_id,
                    pool,
                    config,
                    cache: &mut ctx.formula_cache,
                    sink,
                    op_count,
                };
                positional_diff(&mut emit_ctx, old, new)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnPartialResult => {
                ctx.warnings.push(warning);
                let mut emit_ctx = EmitCtx {
                    sheet_id,
                    pool,
                    config,
                    cache: &mut ctx.formula_cache,
                    sink,
                    op_count,
                };
                positional_diff(&mut emit_ctx, old, new)?;
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id.clone(),
                    rows: old.nrows.max(new.nrows),
                    cols: old.ncols.max(new.ncols),
                    max_rows: config.max_align_rows,
                    max_cols: config.max_align_cols,
                });
            }
        }
        return Ok(());
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
        #[cfg(feature = "perf-metrics")]
        metrics,
    )?;
    Ok(())
}

fn try_diff_with_amr<S: DiffSink>(
    emit_ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<bool, DiffError> {
    let Some(amr_result) =
        align_rows_amr_with_signatures_from_views(old_view, new_view, emit_ctx.config)
    else {
        return Ok(false);
    };

    let mut alignment = amr_result.alignment;

    if emit_ctx.config.max_move_iterations > 0 {
        let row_signatures_old = amr_result.row_signatures_a;
        let row_signatures_new = amr_result.row_signatures_b;
        inject_moves_from_insert_delete(
            old,
            new,
            &mut alignment,
            &row_signatures_old,
            &row_signatures_new,
        );
    } else {
        let mut deleted_from_moves = Vec::new();
        let mut inserted_from_moves = Vec::new();
        for mv in &alignment.moves {
            deleted_from_moves
                .extend(mv.src_start_row..mv.src_start_row.saturating_add(mv.row_count));
            inserted_from_moves
                .extend(mv.dst_start_row..mv.dst_start_row.saturating_add(mv.row_count));
        }

        let multiset_equal = row_signature_multiset_equal(old, new);
        if multiset_equal {
            for (a, b) in &alignment.matched {
                if row_signature_at(old, *a) != row_signature_at(new, *b) {
                    deleted_from_moves.push(*a);
                    inserted_from_moves.push(*b);
                }
            }
        }

        if !deleted_from_moves.is_empty() || !inserted_from_moves.is_empty() {
            let deleted_set: HashSet<u32> = deleted_from_moves.iter().copied().collect();
            let inserted_set: HashSet<u32> = inserted_from_moves.iter().copied().collect();

            alignment
                .matched
                .retain(|(a, b)| !deleted_set.contains(a) && !inserted_set.contains(b));

            alignment.deleted.extend(deleted_set);
            alignment.inserted.extend(inserted_set);
            alignment.deleted.sort_unstable();
            alignment.deleted.dedup();
            alignment.inserted.sort_unstable();
            alignment.inserted.dedup();
        }

        alignment.moves.clear();
    }

    let has_structural_rows = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();
    if has_structural_rows && alignment.matched.is_empty() {
        #[cfg(feature = "perf-metrics")]
        run_positional_diff_with_metrics(emit_ctx, old, new, metrics.as_deref_mut())?;
        #[cfg(not(feature = "perf-metrics"))]
        run_positional_diff_with_metrics(emit_ctx, old, new)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(true);
    }

    if has_structural_rows {
        let has_row_edits = alignment
            .matched
            .iter()
            .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));
        if has_row_edits && emit_ctx.config.max_move_iterations > 0 {
            #[cfg(feature = "perf-metrics")]
            run_positional_diff_with_metrics(emit_ctx, old, new, metrics.as_deref_mut())?;
            #[cfg(not(feature = "perf-metrics"))]
            run_positional_diff_with_metrics(emit_ctx, old, new)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return Ok(true);
        }
    }

    if alignment.moves.is_empty()
        && alignment.inserted.is_empty()
        && alignment.deleted.is_empty()
        && old.ncols != new.ncols
        && let Some(col_alignment) =
            align_single_column_change_from_views(old_view, new_view, emit_ctx.config)
    {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        emit_column_aligned_diffs(emit_ctx, old, new, &col_alignment)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            let overlap_rows = old.nrows.min(new.nrows) as u64;
            m.add_cells_compared(overlap_rows.saturating_mul(col_alignment.matched.len() as u64));
            m.end_phase(Phase::CellDiff);
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(true);
    }

    let alignment_is_trivial_identity = alignment.moves.is_empty()
        && alignment.inserted.is_empty()
        && alignment.deleted.is_empty()
        && old.nrows == new.nrows
        && alignment.matched.len() as u32 == old.nrows
        && alignment.matched.iter().all(|(a, b)| a == b);

    if !alignment_is_trivial_identity
        && alignment.moves.is_empty()
        && row_signature_multiset_equal(old, new)
        && emit_ctx.config.max_move_iterations > 0
    {
        #[cfg(feature = "perf-metrics")]
        run_positional_diff_with_metrics(emit_ctx, old, new, metrics.as_deref_mut())?;
        #[cfg(not(feature = "perf-metrics"))]
        run_positional_diff_with_metrics(emit_ctx, old, new)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(true);
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }
    let compared = emit_row_aligned_diffs(emit_ctx, old_view, new_view, &alignment)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.add_cells_compared(compared);
        m.anchors_found = m
            .anchors_found
            .saturating_add(alignment.matched.len() as u32);
        m.moves_detected = m
            .moves_detected
            .saturating_add(alignment.moves.len() as u32);
    }
    #[cfg(not(feature = "perf-metrics"))]
    let _ = compared;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.end_phase(Phase::CellDiff);
    }
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.end_phase(Phase::Alignment);
    }

    Ok(true)
}

fn diff_grids_core<S: DiffSink>(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    pool: &StringPool,
    sink: &mut S,
    op_count: &mut usize,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == new.nrows && old.ncols == new.ncols && grids_non_blank_cells_equal(old, new) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(cells_in_overlap(old, new));
        }
        return Ok(());
    }

    let mut differ = SheetGridDiffer::new(
        sheet_id,
        old,
        new,
        config,
        pool,
        &mut ctx.formula_cache,
        sink,
        op_count,
        #[cfg(feature = "perf-metrics")]
        metrics.as_deref_mut(),
    );

    differ.detect_moves()?;

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.end_phase(Phase::MoveDetection);
    }

    if differ.has_mask_exclusions() {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        let result = differ.diff_with_masks()?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.end_phase(Phase::CellDiff);
        }
        if result {
            return Ok(());
        }
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.start_phase(Phase::Alignment);
    }

    #[cfg(feature = "perf-metrics")]
    if try_diff_with_amr(
        &mut differ.emit_ctx,
        old,
        new,
        &differ.old_view,
        &differ.new_view,
        differ.metrics.as_deref_mut(),
    )? {
        return Ok(());
    }
    #[cfg(not(feature = "perf-metrics"))]
    if try_diff_with_amr(
        &mut differ.emit_ctx,
        old,
        new,
        &differ.old_view,
        &differ.new_view,
    )? {
        return Ok(());
    }

    if let Some(alignment) =
        align_row_changes_from_views(&differ.old_view, &differ.new_view, config)
    {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        let compared = emit_row_aligned_diffs(
            &mut differ.emit_ctx,
            &differ.old_view,
            &differ.new_view,
            &alignment,
        )?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.add_cells_compared(compared);
            m.end_phase(Phase::CellDiff);
        }
        #[cfg(not(feature = "perf-metrics"))]
        let _ = compared;
    } else if let Some(alignment) =
        align_single_column_change_from_views(&differ.old_view, &differ.new_view, config)
    {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        emit_column_aligned_diffs(&mut differ.emit_ctx, old, new, &alignment)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            let overlap_rows = old.nrows.min(new.nrows) as u64;
            m.add_cells_compared(overlap_rows.saturating_mul(alignment.matched.len() as u64));
            m.end_phase(Phase::CellDiff);
        }
    } else {
        #[cfg(feature = "perf-metrics")]
        run_positional_diff_with_metrics(
            &mut differ.emit_ctx,
            old,
            new,
            differ.metrics.as_deref_mut(),
        )?;
        #[cfg(not(feature = "perf-metrics"))]
        run_positional_diff_with_metrics(&mut differ.emit_ctx, old, new)?;
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.end_phase(Phase::Alignment);
    }

    Ok(())
}

fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), None) | (None, Some(cell_a)) => {
            cell_a.value.is_none() && cell_a.formula.is_none()
        }
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
    }
}

fn grids_non_blank_cells_equal(old: &Grid, new: &Grid) -> bool {
    if old.cells.len() != new.cells.len() {
        return false;
    }

    for (coord, cell_a) in old.cells.iter() {
        let Some(cell_b) = new.cells.get(coord) else {
            return false;
        };
        if cell_a.value != cell_b.value || cell_a.formula != cell_b.formula {
            return false;
        }
    }

    true
}

fn collect_differences_in_grid(old: &Grid, new: &Grid) -> Vec<(u32, u32)> {
    let mut diffs = Vec::new();

    for row in 0..old.nrows {
        for col in 0..old.ncols {
            if !cells_content_equal(old.get(row, col), new.get(row, col)) {
                diffs.push((row, col));
            }
        }
    }

    diffs
}

fn contiguous_ranges<I>(indices: I) -> Vec<(u32, u32)>
where
    I: IntoIterator<Item = u32>,
{
    let mut values: Vec<u32> = indices.into_iter().collect();
    if values.is_empty() {
        return Vec::new();
    }

    values.sort_unstable();
    values.dedup();

    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut start = values[0];
    let mut prev = values[0];

    for &val in values.iter().skip(1) {
        if val == prev + 1 {
            prev = val;
            continue;
        }

        ranges.push((start, prev));
        start = val;
        prev = val;
    }
    ranges.push((start, prev));

    ranges
}

fn group_rows_by_column_patterns(diffs: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if diffs.is_empty() {
        return Vec::new();
    }

    let mut row_to_cols: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (row, col) in diffs {
        row_to_cols.entry(*row).or_default().push(*col);
    }

    for cols in row_to_cols.values_mut() {
        cols.sort_unstable();
        cols.dedup();
    }

    let mut rows: Vec<u32> = row_to_cols.keys().copied().collect();
    rows.sort_unstable();

    let mut groups: Vec<(u32, u32)> = Vec::new();
    if let Some(&first_row) = rows.first() {
        let mut start = first_row;
        let mut prev = first_row;
        let mut current_cols = row_to_cols.get(&first_row).cloned().unwrap_or_default();

        for row in rows.into_iter().skip(1) {
            let cols = row_to_cols.get(&row).cloned().unwrap_or_default();
            if row == prev + 1 && cols == current_cols {
                prev = row;
            } else {
                groups.push((start, prev));
                start = row;
                prev = row;
                current_cols = cols;
            }
        }
        groups.push((start, prev));
    }

    groups
}

fn row_signature_at(grid: &Grid, row: u32) -> Option<RowSignature> {
    if let Some(sig) = grid
        .row_signatures
        .as_ref()
        .and_then(|rows| rows.get(row as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_row_signature(row))
}

fn row_signature_multiset_equal(a: &Grid, b: &Grid) -> bool {
    if a.nrows != b.nrows {
        return false;
    }

    let mut a_sigs: Vec<RowSignature> = (0..a.nrows)
        .filter_map(|row| row_signature_at(a, row))
        .collect();
    let mut b_sigs: Vec<RowSignature> = (0..b.nrows)
        .filter_map(|row| row_signature_at(b, row))
        .collect();

    a_sigs.sort_unstable_by_key(|s| s.hash);
    b_sigs.sort_unstable_by_key(|s| s.hash);

    a_sigs == b_sigs
}

fn col_signature_at(grid: &Grid, col: u32) -> Option<ColSignature> {
    if let Some(sig) = grid
        .col_signatures
        .as_ref()
        .and_then(|cols| cols.get(col as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_col_signature(col))
}

fn align_indices_by_signature<T: Copy + Eq>(
    idx_a: &[u32],
    idx_b: &[u32],
    sig_a: impl Fn(u32) -> Option<T>,
    sig_b: impl Fn(u32) -> Option<T>,
) -> Option<(Vec<u32>, Vec<u32>)> {
    if idx_a.is_empty() || idx_b.is_empty() {
        return None;
    }

    if idx_a.len() == idx_b.len() {
        return Some((idx_a.to_vec(), idx_b.to_vec()));
    }

    let (short, long, short_is_a) = if idx_a.len() <= idx_b.len() {
        (idx_a, idx_b, true)
    } else {
        (idx_b, idx_a, false)
    };

    let diff = long.len() - short.len();
    let mut best_offset = 0usize;
    let mut best_matches = 0usize;

    for offset in 0..=diff {
        let mut matches = 0usize;
        for (i, &short_idx) in short.iter().enumerate() {
            let long_idx = long[offset + i];
            let (sig_short, sig_long) = if short_is_a {
                (sig_a(short_idx), sig_b(long_idx))
            } else {
                (sig_b(short_idx), sig_a(long_idx))
            };
            if let (Some(sa), Some(sb)) = (sig_short, sig_long)
                && sa == sb
            {
                matches += 1;
            }
        }
        if matches > best_matches {
            best_matches = matches;
            best_offset = offset;
        }
    }

    if short_is_a {
        let aligned_b = long[best_offset..best_offset + short.len()].to_vec();
        Some((idx_a.to_vec(), aligned_b))
    } else {
        let aligned_a = long[best_offset..best_offset + short.len()].to_vec();
        Some((aligned_a, idx_b.to_vec()))
    }
}

fn inject_moves_from_insert_delete(
    old: &Grid,
    new: &Grid,
    alignment: &mut RowAlignment,
    row_signatures_old: &[RowSignature],
    row_signatures_new: &[RowSignature],
) {
    if alignment.inserted.is_empty() || alignment.deleted.is_empty() {
        return;
    }

    let mut deleted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.deleted {
        let sig = row_signatures_old
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(old, *row));
        if let Some(sig) = sig {
            deleted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut inserted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.inserted {
        let sig = row_signatures_new
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(new, *row));
        if let Some(sig) = sig {
            inserted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut matched_pairs = Vec::new();
    for (sig, deleted_rows) in deleted_by_sig.iter() {
        if deleted_rows.len() != 1 {
            continue;
        }
        if let Some(insert_rows) = inserted_by_sig.get(sig) {
            if insert_rows.len() != 1 {
                continue;
            }
            matched_pairs.push((deleted_rows[0], insert_rows[0]));
        }
    }

    if matched_pairs.is_empty() {
        return;
    }

    let new_moves = moves_from_matched_pairs(&matched_pairs);
    if new_moves.is_empty() {
        return;
    }

    let mut moved_src = HashSet::new();
    let mut moved_dst = HashSet::new();
    for mv in &new_moves {
        for r in mv.src_start_row..mv.src_start_row.saturating_add(mv.row_count) {
            moved_src.insert(r);
        }
        for r in mv.dst_start_row..mv.dst_start_row.saturating_add(mv.row_count) {
            moved_dst.insert(r);
        }
    }

    alignment.deleted.retain(|r| !moved_src.contains(r));
    alignment.inserted.retain(|r| !moved_dst.contains(r));
    alignment.moves.extend(new_moves);
    alignment
        .moves
        .sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));
}

fn build_projected_grid_from_maps(
    source: &Grid,
    mask: &RegionMask,
    row_map: &[u32],
    col_map: &[u32],
) -> (Grid, Vec<u32>, Vec<u32>) {
    let nrows = row_map.len() as u32;
    let ncols = col_map.len() as u32;

    let mut row_lookup: Vec<Option<u32>> = vec![None; source.nrows as usize];
    for (new_idx, old_row) in row_map.iter().enumerate() {
        row_lookup[*old_row as usize] = Some(new_idx as u32);
    }

    let mut col_lookup: Vec<Option<u32>> = vec![None; source.ncols as usize];
    for (new_idx, old_col) in col_map.iter().enumerate() {
        col_lookup[*old_col as usize] = Some(new_idx as u32);
    }

    let mut projected = Grid::new(nrows, ncols);

    for ((row, col), cell) in source.iter_cells() {
        if !mask.is_cell_active(row, col) {
            continue;
        }
        let Some(new_row) = row_lookup.get(row as usize).and_then(|v| *v) else {
            continue;
        };
        let Some(new_col) = col_lookup.get(col as usize).and_then(|v| *v) else {
            continue;
        };

        projected.insert_cell(new_row, new_col, cell.value.clone(), cell.formula);
    }

    (projected, row_map.to_vec(), col_map.to_vec())
}

fn build_masked_grid(source: &Grid, mask: &RegionMask) -> (Grid, Vec<u32>, Vec<u32>) {
    let row_map: Vec<u32> = mask.active_rows().collect();
    let col_map: Vec<u32> = mask.active_cols().collect();

    let nrows = row_map.len() as u32;
    let ncols = col_map.len() as u32;

    let mut row_lookup: Vec<Option<u32>> = vec![None; source.nrows as usize];
    for (new_idx, old_row) in row_map.iter().enumerate() {
        row_lookup[*old_row as usize] = Some(new_idx as u32);
    }

    let mut col_lookup: Vec<Option<u32>> = vec![None; source.ncols as usize];
    for (new_idx, old_col) in col_map.iter().enumerate() {
        col_lookup[*old_col as usize] = Some(new_idx as u32);
    }

    let mut projected = Grid::new(nrows, ncols);

    for ((row, col), cell) in source.iter_cells() {
        if !mask.is_cell_active(row, col) {
            continue;
        }

        let Some(new_row) = row_lookup.get(row as usize).and_then(|v| *v) else {
            continue;
        };
        let Some(new_col) = col_lookup.get(col as usize).and_then(|v| *v) else {
            continue;
        };

        projected.insert_cell(new_row, new_col, cell.value.clone(), cell.formula);
    }

    (projected, row_map, col_map)
}

fn detect_exact_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
        return detect_exact_row_block_move(old, new, config);
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_row_block_move(&old_proj, &new_proj, config)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(RowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}

fn detect_exact_column_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<ColumnBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
        return detect_exact_column_block_move(old, new, config);
    }

    let (old_proj, _, old_cols) = build_masked_grid(old, old_mask);
    let (new_proj, _, new_cols) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_column_block_move(&old_proj, &new_proj, config)?;
    let src_start_col = *old_cols.get(mv_local.src_start_col as usize)?;
    let dst_start_col = *new_cols.get(mv_local.dst_start_col as usize)?;

    Some(ColumnBlockMove {
        src_start_col,
        dst_start_col,
        col_count: mv_local.col_count,
    })
}

fn detect_exact_rect_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<RectBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    // Fast path: allow the strict detector to short-circuit when it succeeds, but
    // fall back to the masked search if it fails (e.g., when extra diffs exist).
    if !old_mask.has_exclusions()
        && !new_mask.has_exclusions()
        && old.nrows == new.nrows
        && old.ncols == new.ncols
        && let Some(mv) = detect_exact_rect_block_move(old, new, config)
    {
        return Some(mv);
    }

    let aligned_rows = align_indices_by_signature(
        &old_mask.active_rows().collect::<Vec<_>>(),
        &new_mask.active_rows().collect::<Vec<_>>(),
        |r| row_signature_at(old, r),
        |r| row_signature_at(new, r),
    )?;
    let aligned_cols = align_indices_by_signature(
        &old_mask.active_cols().collect::<Vec<_>>(),
        &new_mask.active_cols().collect::<Vec<_>>(),
        |c| col_signature_at(old, c),
        |c| col_signature_at(new, c),
    )?;
    let (old_proj, old_rows, old_cols) =
        build_projected_grid_from_maps(old, old_mask, &aligned_rows.0, &aligned_cols.0);
    let (new_proj, new_rows, new_cols) =
        build_projected_grid_from_maps(new, new_mask, &aligned_rows.1, &aligned_cols.1);

    let map_move = |mv_local: RectBlockMove,
                    row_map_old: &[u32],
                    row_map_new: &[u32],
                    col_map_old: &[u32],
                    col_map_new: &[u32]|
     -> Option<RectBlockMove> {
        let src_start_row = *row_map_old.get(mv_local.src_start_row as usize)?;
        let dst_start_row = *row_map_new.get(mv_local.dst_start_row as usize)?;
        let src_start_col = *col_map_old.get(mv_local.src_start_col as usize)?;
        let dst_start_col = *col_map_new.get(mv_local.dst_start_col as usize)?;

        Some(RectBlockMove {
            src_start_row,
            dst_start_row,
            src_start_col,
            dst_start_col,
            src_row_count: mv_local.src_row_count,
            src_col_count: mv_local.src_col_count,
            block_hash: mv_local.block_hash,
        })
    };

    if let Some(mv_local) = detect_exact_rect_block_move(&old_proj, &new_proj, config)
        && let Some(mapped) = map_move(mv_local, &old_rows, &new_rows, &old_cols, &new_cols)
    {
        return Some(mapped);
    }

    let diff_positions = collect_differences_in_grid(&old_proj, &new_proj);
    if diff_positions.is_empty() {
        return None;
    }

    let row_ranges = group_rows_by_column_patterns(&diff_positions);
    let col_ranges_full = contiguous_ranges(diff_positions.iter().map(|(_, c)| *c));
    let has_prior_exclusions = old_mask.has_exclusions() || new_mask.has_exclusions();
    if !has_prior_exclusions && row_ranges.len() <= 2 && col_ranges_full.len() <= 2 {
        return None;
    }

    let range_len = |range: (u32, u32)| range.1.saturating_sub(range.0).saturating_add(1);
    let in_range = |idx: u32, range: (u32, u32)| idx >= range.0 && idx <= range.1;
    let rectangles_match = |src_rows: (u32, u32),
                            src_cols: (u32, u32),
                            dst_rows: (u32, u32),
                            dst_cols: (u32, u32)|
     -> bool {
        let row_count = range_len(src_rows);
        let col_count = range_len(src_cols);

        for dr in 0..row_count {
            for dc in 0..col_count {
                let src_row = src_rows.0 + dr;
                let src_col = src_cols.0 + dc;
                let dst_row = dst_rows.0 + dr;
                let dst_col = dst_cols.0 + dc;

                if !cells_content_equal(
                    old_proj.get(src_row, src_col),
                    new_proj.get(dst_row, dst_col),
                ) {
                    return false;
                }
            }
        }

        true
    };

    for (row_idx, &row_a) in row_ranges.iter().enumerate() {
        for &row_b in row_ranges.iter().skip(row_idx + 1) {
            if range_len(row_a) != range_len(row_b) {
                continue;
            }

            let cols_row_a: Vec<u32> = diff_positions
                .iter()
                .filter_map(|(r, c)| if in_range(*r, row_a) { Some(*c) } else { None })
                .collect();
            let cols_row_b: Vec<u32> = diff_positions
                .iter()
                .filter_map(|(r, c)| if in_range(*r, row_b) { Some(*c) } else { None })
                .collect();
            let col_ranges_a = contiguous_ranges(cols_row_a);
            let col_ranges_b = contiguous_ranges(cols_row_b);
            let mut col_pairs: Vec<((u32, u32), (u32, u32))> = Vec::new();

            for &col_a in &col_ranges_a {
                for &col_b in &col_ranges_b {
                    if range_len(col_a) != range_len(col_b) {
                        continue;
                    }
                    col_pairs.push((col_a, col_b));
                }
            }

            if col_pairs.is_empty() {
                continue;
            }

            for (col_a, col_b) in col_pairs {
                let mut scoped_old_mask = RegionMask::all_active(old_proj.nrows, old_proj.ncols);
                let mut scoped_new_mask = RegionMask::all_active(new_proj.nrows, new_proj.ncols);

                for row in 0..old_proj.nrows {
                    if !in_range(row, row_a) && !in_range(row, row_b) {
                        scoped_old_mask.exclude_row(row);
                        scoped_new_mask.exclude_row(row);
                    }
                }

                for col in 0..old_proj.ncols {
                    if !in_range(col, col_a) && !in_range(col, col_b) {
                        scoped_old_mask.exclude_col(col);
                        scoped_new_mask.exclude_col(col);
                    }
                }

                let (old_scoped, scoped_old_rows, scoped_old_cols) =
                    build_masked_grid(&old_proj, &scoped_old_mask);
                let (new_scoped, scoped_new_rows, scoped_new_cols) =
                    build_masked_grid(&new_proj, &scoped_new_mask);

                if old_scoped.nrows != new_scoped.nrows || old_scoped.ncols != new_scoped.ncols {
                    continue;
                }

                if let Some(candidate) =
                    detect_exact_rect_block_move(&old_scoped, &new_scoped, config)
                {
                    let scoped_row_map_old: Option<Vec<u32>> = scoped_old_rows
                        .iter()
                        .map(|idx| old_rows.get(*idx as usize).copied())
                        .collect();
                    let scoped_row_map_new: Option<Vec<u32>> = scoped_new_rows
                        .iter()
                        .map(|idx| new_rows.get(*idx as usize).copied())
                        .collect();
                    let scoped_col_map_old: Option<Vec<u32>> = scoped_old_cols
                        .iter()
                        .map(|idx| old_cols.get(*idx as usize).copied())
                        .collect();
                    let scoped_col_map_new: Option<Vec<u32>> = scoped_new_cols
                        .iter()
                        .map(|idx| new_cols.get(*idx as usize).copied())
                        .collect();

                    if let (
                        Some(row_map_old),
                        Some(row_map_new),
                        Some(col_map_old),
                        Some(col_map_new),
                    ) = (
                        scoped_row_map_old,
                        scoped_row_map_new,
                        scoped_col_map_old,
                        scoped_col_map_new,
                    ) && let Some(mapped) = map_move(
                        candidate,
                        &row_map_old,
                        &row_map_new,
                        &col_map_old,
                        &col_map_new,
                    ) {
                        return Some(mapped);
                    }
                }

                let row_len = range_len(row_a);
                let col_len = range_len(col_a);
                if row_len == 0 || col_len == 0 {
                    continue;
                }

                let candidates = [
                    (row_a, col_a, row_b, col_b),
                    (row_a, col_b, row_b, col_a),
                    (row_b, col_a, row_a, col_b),
                    (row_b, col_b, row_a, col_a),
                ];

                for (src_rows, src_cols, dst_rows, dst_cols) in candidates {
                    if range_len(src_rows) != range_len(dst_rows)
                        || range_len(src_cols) != range_len(dst_cols)
                    {
                        continue;
                    }

                    if rectangles_match(src_rows, src_cols, dst_rows, dst_cols) {
                        let mapped = RectBlockMove {
                            src_start_row: *old_rows.get(src_rows.0 as usize)?,
                            dst_start_row: *new_rows.get(dst_rows.0 as usize)?,
                            src_start_col: *old_cols.get(src_cols.0 as usize)?,
                            dst_start_col: *new_cols.get(dst_cols.0 as usize)?,
                            src_row_count: range_len(src_rows),
                            src_col_count: range_len(src_cols),
                            block_hash: None,
                        };
                        return Some(mapped);
                    }
                }
            }
        }
    }

    None
}

fn detect_fuzzy_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
        return detect_fuzzy_row_block_move(old, new, config);
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_fuzzy_row_block_move(&old_proj, &new_proj, config)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(RowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}

#[cfg(feature = "perf-metrics")]
fn cells_in_overlap(old: &Grid, new: &Grid) -> u64 {
    let overlap_rows = old.nrows.min(new.nrows) as u64;
    let overlap_cols = old.ncols.min(new.ncols) as u64;
    overlap_rows.saturating_mul(overlap_cols)
}

#[cfg(feature = "perf-metrics")]
fn run_positional_diff_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }

    positional_diff(ctx, old, new)?;

    if let Some(m) = metrics.as_mut() {
        m.add_cells_compared(cells_in_overlap(old, new));
        m.end_phase(Phase::CellDiff);
    }

    Ok(())
}

#[cfg(not(feature = "perf-metrics"))]
fn run_positional_diff_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
) -> Result<(), DiffError> {
    positional_diff(ctx, old, new)
}

fn positional_diff<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
) -> Result<(), DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    for row in 0..overlap_rows {
        diff_row_pair(ctx, old, new, row, row, overlap_cols)?;
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            ctx.emit(DiffOp::row_added(ctx.sheet_id.clone(), row_idx, None))?;
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            ctx.emit(DiffOp::row_removed(ctx.sheet_id.clone(), row_idx, None))?;
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            ctx.emit(DiffOp::column_added(ctx.sheet_id.clone(), col_idx, None))?;
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            ctx.emit(DiffOp::column_removed(ctx.sheet_id.clone(), col_idx, None))?;
        }
    }

    Ok(())
}

fn diff_aligned_with_masks<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Result<bool, DiffError> {
    let old_rows: Vec<u32> = old_mask.active_rows().collect();
    let new_rows: Vec<u32> = new_mask.active_rows().collect();
    let old_cols: Vec<u32> = old_mask.active_cols().collect();
    let new_cols: Vec<u32> = new_mask.active_cols().collect();

    let Some((rows_a, rows_b)) = align_indices_by_signature(
        &old_rows,
        &new_rows,
        |r| row_signature_at(old, r),
        |r| row_signature_at(new, r),
    ) else {
        return Ok(false);
    };

    let (cols_a, cols_b) = align_indices_by_signature(
        &old_cols,
        &new_cols,
        |c| col_signature_at(old, c),
        |c| col_signature_at(new, c),
    )
    .unwrap_or((old_cols.clone(), new_cols.clone()));

    if rows_a.len() != rows_b.len() || cols_a.len() != cols_b.len() {
        return Ok(false);
    }

    for (row_a, row_b) in rows_a.iter().zip(rows_b.iter()) {
        for (col_a, col_b) in cols_a.iter().zip(cols_b.iter()) {
            if !old_mask.is_cell_active(*row_a, *col_a) || !new_mask.is_cell_active(*row_b, *col_b)
            {
                continue;
            }
            let old_cell = old.get(*row_a, *col_a);
            let new_cell = new.get(*row_b, *col_b);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(*row_b, *col_b);
            let row_shift = *row_b as i32 - *row_a as i32;
            let col_shift = *col_b as i32 - *col_a as i32;
            emit_cell_edit(ctx, addr, old_cell, new_cell, row_shift, col_shift)?;
        }
    }

    let rows_a_set: HashSet<u32> = rows_a.iter().copied().collect();
    let rows_b_set: HashSet<u32> = rows_b.iter().copied().collect();

    for row_idx in new_rows.iter().filter(|r| !rows_b_set.contains(r)) {
        if new_mask.is_row_active(*row_idx) {
            ctx.emit(DiffOp::row_added(ctx.sheet_id.clone(), *row_idx, None))?;
        }
    }

    for row_idx in old_rows.iter().filter(|r| !rows_a_set.contains(r)) {
        if old_mask.is_row_active(*row_idx) {
            ctx.emit(DiffOp::row_removed(ctx.sheet_id.clone(), *row_idx, None))?;
        }
    }

    let cols_a_set: HashSet<u32> = cols_a.iter().copied().collect();
    let cols_b_set: HashSet<u32> = cols_b.iter().copied().collect();

    for col_idx in new_cols.iter().filter(|c| !cols_b_set.contains(c)) {
        if new_mask.is_col_active(*col_idx) {
            ctx.emit(DiffOp::column_added(ctx.sheet_id.clone(), *col_idx, None))?;
        }
    }

    for col_idx in old_cols.iter().filter(|c| !cols_a_set.contains(c)) {
        if old_mask.is_col_active(*col_idx) {
            ctx.emit(DiffOp::column_removed(ctx.sheet_id.clone(), *col_idx, None))?;
        }
    }

    Ok(true)
}

fn positional_diff_with_masks<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Result<(), DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    for row in 0..overlap_rows {
        for col in 0..overlap_cols {
            if !old_mask.is_cell_active(row, col) || !new_mask.is_cell_active(row, col) {
                continue;
            }
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, col);
            emit_cell_edit(ctx, addr, old_cell, new_cell, 0, 0)?;
        }
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            if new_mask.is_row_active(row_idx) {
                ctx.emit(DiffOp::row_added(ctx.sheet_id.clone(), row_idx, None))?;
            }
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            if old_mask.is_row_active(row_idx) {
                ctx.emit(DiffOp::row_removed(ctx.sheet_id.clone(), row_idx, None))?;
            }
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            if new_mask.is_col_active(col_idx) {
                ctx.emit(DiffOp::column_added(ctx.sheet_id.clone(), col_idx, None))?;
            }
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            if old_mask.is_col_active(col_idx) {
                ctx.emit(DiffOp::column_removed(ctx.sheet_id.clone(), col_idx, None))?;
            }
        }
    }

    Ok(())
}

fn positional_diff_masked_equal_size<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Result<(), DiffError> {
    let row_shift_zone =
        compute_combined_shift_zone(old_mask.row_shift_bounds(), new_mask.row_shift_bounds());
    let col_shift_zone =
        compute_combined_shift_zone(old_mask.col_shift_bounds(), new_mask.col_shift_bounds());

    let stable_rows: Vec<u32> = (0..old.nrows)
        .filter(|&r| !is_in_zone(r, &row_shift_zone))
        .collect();
    let stable_cols: Vec<u32> = (0..old.ncols)
        .filter(|&c| !is_in_zone(c, &col_shift_zone))
        .collect();

    for &row in &stable_rows {
        for &col in &stable_cols {
            if !old_mask.is_cell_active(row, col) || !new_mask.is_cell_active(row, col) {
                continue;
            }
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, col);
            emit_cell_edit(ctx, addr, old_cell, new_cell, 0, 0)?;
        }
    }

    Ok(())
}

fn compute_combined_shift_zone(a: Option<(u32, u32)>, b: Option<(u32, u32)>) -> Option<(u32, u32)> {
    match (a, b) {
        (Some((a_min, a_max)), Some((b_min, b_max))) => Some((a_min.min(b_min), a_max.max(b_max))),
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}

fn is_in_zone(idx: u32, zone: &Option<(u32, u32)>) -> bool {
    match zone {
        Some((min, max)) => idx >= *min && idx <= *max,
        None => false,
    }
}

fn emit_row_block_move<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    mv: RowBlockMove,
) -> Result<(), DiffError> {
    ctx.emit(DiffOp::BlockMovedRows {
        sheet: ctx.sheet_id.clone(),
        src_start_row: mv.src_start_row,
        row_count: mv.row_count,
        dst_start_row: mv.dst_start_row,
        block_hash: None,
    })
}

fn emit_column_block_move<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    mv: ColumnBlockMove,
) -> Result<(), DiffError> {
    ctx.emit(DiffOp::BlockMovedColumns {
        sheet: ctx.sheet_id.clone(),
        src_start_col: mv.src_start_col,
        col_count: mv.col_count,
        dst_start_col: mv.dst_start_col,
        block_hash: None,
    })
}

fn emit_rect_block_move<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    mv: RectBlockMove,
) -> Result<(), DiffError> {
    ctx.emit(DiffOp::BlockMovedRect {
        sheet: ctx.sheet_id.clone(),
        src_start_row: mv.src_start_row,
        src_row_count: mv.src_row_count,
        src_start_col: mv.src_start_col,
        src_col_count: mv.src_col_count,
        dst_start_row: mv.dst_start_row,
        dst_start_col: mv.dst_start_col,
        block_hash: mv.block_hash,
    })
}

fn emit_moved_row_block_edits<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old_view: &GridView,
    new_view: &GridView,
    mv: RowBlockMove,
) -> Result<(), DiffError> {
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols);
    for offset in 0..mv.row_count {
        let old_idx = (mv.src_start_row + offset) as usize;
        let new_idx = (mv.dst_start_row + offset) as usize;
        let Some(old_row) = old_view.rows.get(old_idx) else {
            continue;
        };
        let Some(new_row) = new_view.rows.get(new_idx) else {
            continue;
        };

        let _ = diff_row_pair_sparse(
            ctx,
            mv.src_start_row + offset,
            mv.dst_start_row + offset,
            overlap_cols,
            &old_row.cells,
            &new_row.cells,
        )?;
    }
    Ok(())
}

fn emit_row_aligned_diffs<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &RowAlignment,
) -> Result<u64, DiffError> {
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols);
    let mut compared = 0u64;

    for (row_a, row_b) in &alignment.matched {
        if let (Some(old_row), Some(new_row)) = (
            old_view.rows.get(*row_a as usize),
            new_view.rows.get(*row_b as usize),
        ) {
            compared = compared.saturating_add(diff_row_pair_sparse(
                ctx,
                *row_a,
                *row_b,
                overlap_cols,
                &old_row.cells,
                &new_row.cells,
            )?);
        }
    }

    for row_idx in &alignment.inserted {
        ctx.emit(DiffOp::row_added(ctx.sheet_id.clone(), *row_idx, None))?;
    }

    for row_idx in &alignment.deleted {
        ctx.emit(DiffOp::row_removed(ctx.sheet_id.clone(), *row_idx, None))?;
    }

    for mv in &alignment.moves {
        emit_row_block_move(ctx, *mv)?;
    }

    if new_view.source.ncols > old_view.source.ncols {
        for col_idx in old_view.source.ncols..new_view.source.ncols {
            ctx.emit(DiffOp::column_added(ctx.sheet_id.clone(), col_idx, None))?;
        }
    } else if old_view.source.ncols > new_view.source.ncols {
        for col_idx in new_view.source.ncols..old_view.source.ncols {
            ctx.emit(DiffOp::column_removed(ctx.sheet_id.clone(), col_idx, None))?;
        }
    }

    Ok(compared)
}

fn diff_row_pair_sparse<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &Cell)],
    new_cells: &[(u32, &Cell)],
) -> Result<u64, DiffError> {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut compared = 0u64;

    let row_shift = row_b as i32 - row_a as i32;

    while i < old_cells.len() || j < new_cells.len() {
        let col_a = old_cells.get(i).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col_b = new_cells.get(j).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col = col_a.min(col_b);

        if col >= overlap_cols {
            break;
        }

        compared = compared.saturating_add(1);

        let old_cell = if col_a == col {
            let (_, cell) = old_cells[i];
            i += 1;
            Some(cell)
        } else {
            None
        };

        let new_cell = if col_b == col {
            let (_, cell) = new_cells[j];
            j += 1;
            Some(cell)
        } else {
            None
        };

        let changed = !cells_content_equal(old_cell, new_cell);

        if changed || ctx.config.include_unchanged_cells {
            let addr = CellAddress::from_indices(row_b, col);
            emit_cell_edit(ctx, addr, old_cell, new_cell, row_shift, 0)?;
        }
    }

    Ok(compared)
}

fn diff_row_pair<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
) -> Result<(), DiffError> {
    let row_shift = row_b as i32 - row_a as i32;
    for col in 0..overlap_cols {
        let old_cell = old.get(row_a, col);
        let new_cell = new.get(row_b, col);

        if cells_content_equal(old_cell, new_cell) {
            continue;
        }

        let addr = CellAddress::from_indices(row_b, col);
        emit_cell_edit(ctx, addr, old_cell, new_cell, row_shift, 0)?;
    }
    Ok(())
}

fn emit_column_aligned_diffs<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    alignment: &ColumnAlignment,
) -> Result<(), DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);

    for row in 0..overlap_rows {
        for (col_a, col_b) in &alignment.matched {
            let old_cell = old.get(row, *col_a);
            let new_cell = new.get(row, *col_b);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, *col_b);
            let col_shift = *col_b as i32 - *col_a as i32;
            emit_cell_edit(ctx, addr, old_cell, new_cell, 0, col_shift)?;
        }
    }

    for col_idx in &alignment.inserted {
        ctx.emit(DiffOp::column_added(ctx.sheet_id.clone(), *col_idx, None))?;
    }

    for col_idx in &alignment.deleted {
        ctx.emit(DiffOp::column_removed(ctx.sheet_id.clone(), *col_idx, None))?;
    }

    Ok(())
}

fn snapshot_with_addr(cell: Option<&Cell>, addr: CellAddress) -> CellSnapshot {
    match cell {
        Some(cell) => CellSnapshot {
            addr,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
        },
        None => CellSnapshot::empty(addr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::VecSink;
    use crate::string_pool::StringPool;
    use crate::workbook::CellValue;

    fn numbered_cell(value: f64) -> Cell {
        Cell {
            value: Some(CellValue::Number(value)),
            formula: None,
        }
    }

    fn grid_from_matrix(values: &[Vec<i32>]) -> Grid {
        let nrows = values.len() as u32;
        let ncols = if nrows == 0 {
            0
        } else {
            values[0].len() as u32
        };
        let mut grid = Grid::new(nrows, ncols);
        for (r, row) in values.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                grid.insert_cell(
                    r as u32,
                    c as u32,
                    Some(CellValue::Number(*val as f64)),
                    None,
                );
            }
        }
        grid
    }

    #[test]
    fn sheet_kind_order_ranking_includes_macro_and_other() {
        assert!(
            sheet_kind_order(&SheetKind::Worksheet) < sheet_kind_order(&SheetKind::Chart),
            "Worksheet should rank before Chart"
        );
        assert!(
            sheet_kind_order(&SheetKind::Chart) < sheet_kind_order(&SheetKind::Macro),
            "Chart should rank before Macro"
        );
        assert!(
            sheet_kind_order(&SheetKind::Macro) < sheet_kind_order(&SheetKind::Other),
            "Macro should rank before Other"
        );
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

        grid_a.insert_cell(1, 1, None, None);

        assert!(!grids_non_blank_cells_equal(&grid_a, &grid_b));
    }

    #[test]
    fn diff_row_pair_sparse_counts_union_columns_not_sum_lengths() {
        let mut pool = StringPool::new();
        let sheet_id: SheetId = pool.intern("Sheet1");
        let config = DiffConfig::default();
        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let mut cache = FormulaParseCache::default();

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

        let mut emit_ctx = EmitCtx {
            sheet_id: &sheet_id,
            pool: &pool,
            config: &config,
            cache: &mut cache,
            sink: &mut sink,
            op_count: &mut op_count,
        };
        let compared = diff_row_pair_sparse(&mut emit_ctx, 0, 0, 3, &old_cells, &new_cells)
            .expect("diff should succeed");

        assert_eq!(compared, 3);
    }

    #[test]
    fn diff_row_pair_sparse_counts_union_for_sparse_columns() {
        let mut pool = StringPool::new();
        let sheet_id: SheetId = pool.intern("Sheet1");
        let config = DiffConfig::default();
        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let mut cache = FormulaParseCache::default();

        let old_cells_storage = [numbered_cell(1.0)];
        let new_cells_storage = [numbered_cell(2.0)];

        let old_cells: Vec<(u32, &Cell)> = vec![(0, &old_cells_storage[0])];
        let new_cells: Vec<(u32, &Cell)> = vec![(2, &new_cells_storage[0])];

        let mut emit_ctx = EmitCtx {
            sheet_id: &sheet_id,
            pool: &pool,
            config: &config,
            cache: &mut cache,
            sink: &mut sink,
            op_count: &mut op_count,
        };
        let compared = diff_row_pair_sparse(&mut emit_ctx, 0, 0, 3, &old_cells, &new_cells)
            .expect("diff should succeed");

        assert_eq!(compared, 2);
    }

    #[test]
    fn rect_move_masked_falls_back_when_outside_edit_exists() {
        let rows = 12usize;
        let cols = 12usize;
        let base: Vec<Vec<i32>> = (0..rows)
            .map(|r| {
                (0..cols)
                    .map(|c| 10_000 + (r as i32) * 100 + c as i32)
                    .collect()
            })
            .collect();
        let mut changed = base.clone();

        let src = (2usize, 2usize);
        let dst = (8usize, 6usize);
        let size = (2usize, 3usize);

        for dr in 0..size.0 {
            for dc in 0..size.1 {
                let src_r = src.0 + dr;
                let src_c = src.1 + dc;
                let dst_r = dst.0 + dr;
                let dst_c = dst.1 + dc;

                let src_val = base[src_r][src_c];
                let dst_val = base[dst_r][dst_c];

                changed[dst_r][dst_c] = src_val;
                changed[src_r][src_c] = dst_val;
            }
        }

        changed[0][0] = 77_777;

        let old = grid_from_matrix(&base);
        let new = grid_from_matrix(&changed);
        let old_mask = RegionMask::all_active(old.nrows, old.ncols);
        let new_mask = RegionMask::all_active(new.nrows, new.ncols);

        let mv = detect_exact_rect_block_move_masked(
            &old,
            &new,
            &old_mask,
            &new_mask,
            &DiffConfig::default(),
        )
        .expect("masked detector should fall back and still detect the move");

        assert_eq!(mv.src_start_row, src.0 as u32);
        assert_eq!(mv.src_start_col, src.1 as u32);
        assert_eq!(mv.src_row_count, size.0 as u32);
        assert_eq!(mv.src_col_count, size.1 as u32);
        assert_eq!(mv.dst_start_row, dst.0 as u32);
        assert_eq!(mv.dst_start_col, dst.1 as u32);
    }
}
