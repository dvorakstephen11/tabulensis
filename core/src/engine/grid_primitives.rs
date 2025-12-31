use crate::alignment_types::{RowAlignment, RowBlockMove};
use crate::column_alignment::{
    ColumnAlignment, ColumnBlockMove, align_single_column_change_from_views,
};
use crate::config::DiffConfig;
use crate::diff::{DiffError, DiffOp, FormulaDiffResult};
use crate::formula_diff::{FormulaParseCache, diff_cell_formulas_ids};
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::Phase;
use crate::rect_block_move::RectBlockMove;
use crate::row_alignment::align_row_changes_from_views;
use crate::sink::DiffSink;
use crate::string_pool::StringPool;
use crate::workbook::{Cell, CellAddress, CellSnapshot, Grid};

use super::context::EmitCtx;

struct PendingCell<'a> {
    col: u32,
    old_cell: Option<&'a Cell>,
    new_cell: Option<&'a Cell>,
}

pub(super) struct RowDiffResult<'a> {
    pub(super) compared: u64,
    pub(super) replaced: bool,
    pending: Vec<PendingCell<'a>>,
}

struct PendingRect {
    start_old: u32,
    start_new: u32,
    row_count: u32,
}

struct RowPairPlan<'a> {
    row_a: u32,
    row_b: u32,
    skipped: bool,
    replaced: bool,
    compared: u64,
    pending: Vec<PendingCell<'a>>,
}

pub(super) fn compute_formula_diff(
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

pub(super) fn emit_cell_edit<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
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
        ctx.sheet_id,
        addr,
        from,
        to,
        formula_diff,
    ))
}

fn dense_row_replace_threshold(config: &DiffConfig, total_cols: u32) -> Option<usize> {
    if config.include_unchanged_cells || total_cols == 0 {
        return None;
    }

    let ratio = config.dense_row_replace_ratio;
    if !ratio.is_finite() || ratio <= 0.0 {
        return None;
    }

    if config.dense_row_replace_min_cols > 0 && total_cols < config.dense_row_replace_min_cols {
        return None;
    }

    let threshold = (ratio * total_cols as f64).ceil() as usize;
    Some(threshold.max(1))
}

fn flush_pending_rect<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    pending: &mut Option<PendingRect>,
    overlap_cols: u32,
) -> Result<(), DiffError> {
    let Some(rect) = pending.take() else {
        return Ok(());
    };

    if overlap_cols == 0 {
        return Ok(());
    }

    let min_rows = ctx.config.dense_rect_replace_min_rows;
    if min_rows > 0 && rect.row_count >= min_rows {
        ctx.emit(DiffOp::RectReplaced {
            sheet: ctx.sheet_id,
            start_row: rect.start_new,
            row_count: rect.row_count,
            start_col: 0,
            col_count: overlap_cols,
        })?;
    } else {
        for offset in 0..rect.row_count {
            ctx.emit(DiffOp::RowReplaced {
                sheet: ctx.sheet_id,
                row_idx: rect.start_new + offset,
            })?;
        }
    }

    Ok(())
}

fn emit_pending_cells<'a, S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    row_a: u32,
    row_b: u32,
    pending: Vec<PendingCell<'a>>,
) -> Result<(), DiffError> {
    let row_shift = row_b as i32 - row_a as i32;
    for cell in pending {
        let addr = CellAddress::from_indices(row_b, cell.col);
        emit_cell_edit(ctx, addr, cell.old_cell, cell.new_cell, row_shift, 0)?;
    }
    Ok(())
}

pub(super) fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
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

pub(super) fn snapshot_with_addr(cell: Option<&Cell>, addr: CellAddress) -> CellSnapshot {
    match cell {
        Some(cell) => CellSnapshot {
            addr,
            value: cell.value,
            formula: cell.formula,
        },
        None => CellSnapshot::empty(addr),
    }
}

pub(super) fn emit_row_block_move<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    mv: RowBlockMove,
) -> Result<(), DiffError> {
    ctx.emit(DiffOp::BlockMovedRows {
        sheet: ctx.sheet_id,
        src_start_row: mv.src_start_row,
        row_count: mv.row_count,
        dst_start_row: mv.dst_start_row,
        block_hash: None,
    })
}

pub(super) fn emit_column_block_move<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    mv: ColumnBlockMove,
) -> Result<(), DiffError> {
    ctx.emit(DiffOp::BlockMovedColumns {
        sheet: ctx.sheet_id,
        src_start_col: mv.src_start_col,
        col_count: mv.col_count,
        dst_start_col: mv.dst_start_col,
        block_hash: None,
    })
}

pub(super) fn emit_rect_block_move<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    mv: RectBlockMove,
) -> Result<(), DiffError> {
    ctx.emit(DiffOp::BlockMovedRect {
        sheet: ctx.sheet_id,
        src_start_row: mv.src_start_row,
        src_row_count: mv.src_row_count,
        src_start_col: mv.src_start_col,
        src_col_count: mv.src_col_count,
        dst_start_row: mv.dst_start_row,
        dst_start_col: mv.dst_start_col,
        block_hash: mv.block_hash,
    })
}

pub(super) fn emit_moved_row_block_edits<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old_view: &GridView,
    new_view: &GridView,
    mv: RowBlockMove,
) -> Result<(), DiffError> {
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols);
    let mut offset = 0u32;

    while offset < mv.row_count {
        if ctx.hardening.should_abort() {
            break;
        }

        let chunk_len = cell_diff_chunk_len(overlap_cols)
            .min((mv.row_count - offset) as usize);
        let mut pairs: Vec<(u32, u32)> = Vec::with_capacity(chunk_len);
        for idx in 0..chunk_len {
            let row_a = mv.src_start_row + offset + idx as u32;
            let row_b = mv.dst_start_row + offset + idx as u32;
            pairs.push((row_a, row_b));
        }

        let plans = plan_row_pair_chunk(old_view, new_view, &pairs, overlap_cols, ctx.config);

        for plan in plans {
            if plan.skipped {
                continue;
            }

            if plan.replaced {
                ctx.emit(DiffOp::RowReplaced {
                    sheet: ctx.sheet_id,
                    row_idx: plan.row_b,
                })?;
            } else {
                emit_pending_cells(ctx, plan.row_a, plan.row_b, plan.pending)?;
            }
        }

        offset = offset.saturating_add(chunk_len as u32);

        if ctx.hardening.check_timeout(ctx.warnings) {
            break;
        }
    }
    Ok(())
}

fn diff_row_pair_sparse_plan<'a>(
    config: &DiffConfig,
    overlap_cols: u32,
    old_cells: &[(u32, &'a Cell)],
    new_cells: &[(u32, &'a Cell)],
) -> RowDiffResult<'a> {
    let Some(threshold) = dense_row_replace_threshold(config, overlap_cols) else {
        return diff_row_pair_sparse_thresholdless(config, overlap_cols, old_cells, new_cells);
    };

    let mut compared = 0u64;
    let mut changed_cells = 0usize;
    let mut pending: Vec<PendingCell<'a>> = Vec::new();

    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_cells.len() || j < new_cells.len() {
        let col = match (old_cells.get(i), new_cells.get(j)) {
            (Some((ca, _)), Some((cb, _))) => (*ca).min(*cb),
            (Some((ca, _)), None) => *ca,
            (None, Some((cb, _))) => *cb,
            (None, None) => break,
        };

        if col >= overlap_cols {
            break;
        }

        let mut old_cell = None;
        let mut new_cell = None;

        if let Some((c, cell)) = old_cells.get(i) {
            if *c == col {
                old_cell = Some(*cell);
                i += 1;
            }
        }
        if let Some((c, cell)) = new_cells.get(j) {
            if *c == col {
                new_cell = Some(*cell);
                j += 1;
            }
        }

        compared = compared.saturating_add(1);

        if cells_content_equal(old_cell, new_cell) {
            if config.include_unchanged_cells {
                pending.push(PendingCell {
                    col,
                    old_cell,
                    new_cell,
                });
            }
            continue;
        }

        changed_cells = changed_cells.saturating_add(1);
        if changed_cells >= threshold {
            return RowDiffResult {
                compared,
                replaced: true,
                pending: Vec::new(),
            };
        }

        pending.push(PendingCell {
            col,
            old_cell,
            new_cell,
        });
    }

    RowDiffResult {
        compared,
        replaced: false,
        pending,
    }
}

fn diff_row_pair_sparse_thresholdless<'a>(
    config: &DiffConfig,
    overlap_cols: u32,
    old_cells: &[(u32, &'a Cell)],
    new_cells: &[(u32, &'a Cell)],
) -> RowDiffResult<'a> {
    let mut compared = 0u64;
    let mut pending: Vec<PendingCell<'a>> = Vec::new();

    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_cells.len() || j < new_cells.len() {
        let col = match (old_cells.get(i), new_cells.get(j)) {
            (Some((ca, _)), Some((cb, _))) => (*ca).min(*cb),
            (Some((ca, _)), None) => *ca,
            (None, Some((cb, _))) => *cb,
            (None, None) => break,
        };

        if col >= overlap_cols {
            break;
        }

        let mut old_cell = None;
        let mut new_cell = None;

        if let Some((c, cell)) = old_cells.get(i) {
            if *c == col {
                old_cell = Some(*cell);
                i += 1;
            }
        }
        if let Some((c, cell)) = new_cells.get(j) {
            if *c == col {
                new_cell = Some(*cell);
                j += 1;
            }
        }

        compared = compared.saturating_add(1);

        if config.include_unchanged_cells || !cells_content_equal(old_cell, new_cell) {
            pending.push(PendingCell {
                col,
                old_cell,
                new_cell,
            });
        }
    }

    RowDiffResult {
        compared,
        replaced: false,
        pending,
    }
}

pub(super) fn diff_row_pair_sparse<'a, S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    _row_a: u32,
    _row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &'a Cell)],
    new_cells: &[(u32, &'a Cell)],
) -> Result<RowDiffResult<'a>, DiffError> {
    Ok(diff_row_pair_sparse_plan(
        ctx.config,
        overlap_cols,
        old_cells,
        new_cells,
    ))
}

pub(super) fn diff_row_pair<'a, S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &'a Grid,
    new: &'a Grid,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
) -> Result<RowDiffResult<'a>, DiffError> {
    let mut compared = 0u64;
    let mut changed_cells = 0usize;
    let mut pending: Vec<PendingCell<'a>> = Vec::new();
    let threshold = dense_row_replace_threshold(ctx.config, overlap_cols);

    for col in 0..overlap_cols {
        let old_cell = old.get(row_a, col);
        let new_cell = new.get(row_b, col);
        compared = compared.saturating_add(1);

        let changed = !cells_content_equal(old_cell, new_cell);
        if changed {
            changed_cells = changed_cells.saturating_add(1);
            if let Some(limit) = threshold {
                if changed_cells >= limit {
                    return Ok(RowDiffResult {
                        compared,
                        replaced: true,
                        pending: Vec::new(),
                    });
                }
            }
        }

        if changed || ctx.config.include_unchanged_cells {
            pending.push(PendingCell {
                col,
                old_cell,
                new_cell,
            });
        }
    }

    Ok(RowDiffResult {
        compared,
        replaced: false,
        pending,
    })
}

fn cell_diff_chunk_len(overlap_cols: u32) -> usize {
    if overlap_cols >= 2048 {
        64
    } else if overlap_cols >= 512 {
        256
    } else {
        1024
    }
}

fn plan_row_pair_chunk<'a>(
    old_view: &'a GridView<'a>,
    new_view: &'a GridView<'a>,
    chunk: &[(u32, u32)],
    overlap_cols: u32,
    config: &DiffConfig,
) -> Vec<RowPairPlan<'a>> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        chunk
            .par_iter()
            .map(|(row_a, row_b)| {
                plan_one_row_pair(old_view, new_view, *row_a, *row_b, overlap_cols, config)
            })
            .collect()
    }

    #[cfg(not(feature = "parallel"))]
    chunk
        .iter()
        .map(|(row_a, row_b)| {
            plan_one_row_pair(old_view, new_view, *row_a, *row_b, overlap_cols, config)
        })
        .collect()
}

fn plan_one_row_pair<'a>(
    old_view: &'a GridView<'a>,
    new_view: &'a GridView<'a>,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    config: &DiffConfig,
) -> RowPairPlan<'a> {
    let Some(row_view_a) = old_view.rows.get(row_a as usize) else {
        return RowPairPlan {
            row_a,
            row_b,
            skipped: true,
            replaced: false,
            compared: 0,
            pending: Vec::new(),
        };
    };
    let Some(row_view_b) = new_view.rows.get(row_b as usize) else {
        return RowPairPlan {
            row_a,
            row_b,
            skipped: true,
            replaced: false,
            compared: 0,
            pending: Vec::new(),
        };
    };

    if !config.include_unchanged_cells {
        let sig_a = old_view.row_meta.get(row_a as usize).map(|m| m.signature);
        let sig_b = new_view.row_meta.get(row_b as usize).map(|m| m.signature);
        if let (Some(a), Some(b)) = (sig_a, sig_b) {
            if a == b {
                return RowPairPlan {
                    row_a,
                    row_b,
                    skipped: true,
                    replaced: false,
                    compared: 0,
                    pending: Vec::new(),
                };
            }
        }
    }

    let r = diff_row_pair_sparse_plan(
        config,
        overlap_cols,
        &row_view_a.cells,
        &row_view_b.cells,
    );

    RowPairPlan {
        row_a,
        row_b,
        skipped: false,
        replaced: r.replaced,
        compared: r.compared,
        pending: r.pending,
    }
}

pub(super) fn emit_row_aligned_diffs<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &RowAlignment,
) -> Result<u64, DiffError> {
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols);
    let mut compared = 0u64;
    let mut pending_rect: Option<PendingRect> = None;
    let matched = &alignment.matched;
    let mut idx = 0usize;

    while idx < matched.len() {
        if ctx.hardening.should_abort() {
            break;
        }

        let chunk_len = cell_diff_chunk_len(overlap_cols);
        let end = (idx + chunk_len).min(matched.len());
        let chunk = &matched[idx..end];

        let plans = plan_row_pair_chunk(old_view, new_view, chunk, overlap_cols, ctx.config);

        for plan in plans {
            if plan.skipped {
                flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
                continue;
            }

            compared = compared.saturating_add(plan.compared);

            if plan.replaced {
                if let Some(existing) = pending_rect.as_mut() {
                    let expected_old = existing.start_old.saturating_add(existing.row_count);
                    let expected_new = existing.start_new.saturating_add(existing.row_count);
                    if plan.row_a == expected_old && plan.row_b == expected_new {
                        existing.row_count = existing.row_count.saturating_add(1);
                    } else {
                        flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
                        pending_rect = Some(PendingRect {
                            start_old: plan.row_a,
                            start_new: plan.row_b,
                            row_count: 1,
                        });
                    }
                } else {
                    pending_rect = Some(PendingRect {
                        start_old: plan.row_a,
                        start_new: plan.row_b,
                        row_count: 1,
                    });
                }
            } else {
                flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
                emit_pending_cells(ctx, plan.row_a, plan.row_b, plan.pending)?;
            }
        }

        idx = end;

        if ctx.hardening.check_timeout(ctx.warnings) {
            break;
        }
    }

    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;

    for row_idx in &alignment.inserted {
        ctx.emit(DiffOp::row_added(ctx.sheet_id, *row_idx, None))?;
    }

    for row_idx in &alignment.deleted {
        ctx.emit(DiffOp::row_removed(ctx.sheet_id, *row_idx, None))?;
    }

    for mv in &alignment.moves {
        emit_row_block_move(ctx, *mv)?;
    }

    if new_view.source.ncols > old_view.source.ncols {
        for col_idx in old_view.source.ncols..new_view.source.ncols {
            ctx.emit(DiffOp::column_added(ctx.sheet_id, col_idx, None))?;
        }
    } else if old_view.source.ncols > new_view.source.ncols {
        for col_idx in new_view.source.ncols..old_view.source.ncols {
            ctx.emit(DiffOp::column_removed(ctx.sheet_id, col_idx, None))?;
        }
    }

    Ok(compared)
}

pub(super) fn emit_column_aligned_diffs<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
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
        ctx.emit(DiffOp::column_added(ctx.sheet_id, *col_idx, None))?;
    }

    for col_idx in &alignment.deleted {
        ctx.emit(DiffOp::column_removed(ctx.sheet_id, *col_idx, None))?;
    }

    for mv in &alignment.moves {
        emit_column_block_move(ctx, *mv)?;
    }

    Ok(())
}

pub(super) fn positional_diff<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
) -> Result<(), DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);
    let mut pending_rect: Option<PendingRect> = None;

    ctx.hardening.progress("cell_diff", 0.0);

    for row in 0..overlap_rows {
        if ctx.hardening.check_timeout(ctx.warnings) {
            flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
            return Ok(());
        }

        if overlap_rows > 0 && row % 256 == 0 {
            ctx.hardening
                .progress("cell_diff", row as f32 / overlap_rows as f32);
        }

        let result = diff_row_pair(ctx, old, new, row, row, overlap_cols)?;
        if result.replaced {
            if let Some(existing) = pending_rect.as_mut() {
                let expected_row = existing.start_new + existing.row_count;
                if row == expected_row {
                    existing.row_count = existing.row_count.saturating_add(1);
                } else {
                    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
                    pending_rect = Some(PendingRect {
                        start_old: row,
                        start_new: row,
                        row_count: 1,
                    });
                }
            } else {
                pending_rect = Some(PendingRect {
                    start_old: row,
                    start_new: row,
                    row_count: 1,
                });
            }
        } else {
            flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
            emit_pending_cells(ctx, row, row, result.pending)?;
        }
    }

    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;

    if overlap_rows > 0 {
        ctx.hardening.progress("cell_diff", 1.0);
    }

    if ctx.hardening.check_timeout(ctx.warnings) {
        return Ok(());
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            if row_idx % 4096 == 0 && ctx.hardening.check_timeout(ctx.warnings) {
                return Ok(());
            }
            ctx.emit(DiffOp::row_added(ctx.sheet_id, row_idx, None))?;
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            if row_idx % 4096 == 0 && ctx.hardening.check_timeout(ctx.warnings) {
                return Ok(());
            }
            ctx.emit(DiffOp::row_removed(ctx.sheet_id, row_idx, None))?;
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            if col_idx % 4096 == 0 && ctx.hardening.check_timeout(ctx.warnings) {
                return Ok(());
            }
            ctx.emit(DiffOp::column_added(ctx.sheet_id, col_idx, None))?;
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            if col_idx % 4096 == 0 && ctx.hardening.check_timeout(ctx.warnings) {
                return Ok(());
            }
            ctx.emit(DiffOp::column_removed(ctx.sheet_id, col_idx, None))?;
        }
    }

    Ok(())
}

pub(super) fn positional_diff_from_views<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
) -> Result<u64, DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    ctx.hardening.progress("cell_diff", 0.0);

    let mut compared: u64 = 0;
    let mut pending_rect: Option<PendingRect> = None;

    for row in 0..overlap_rows {
        if ctx.hardening.check_timeout(ctx.warnings) {
            flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
            break;
        }
        if overlap_rows > 0 {
            ctx.hardening
                .progress("cell_diff", (row as f32) / (overlap_rows as f32));
        }

        if !ctx.config.include_unchanged_cells {
            let old_sig = old_view.row_meta.get(row as usize).map(|m| m.signature);
            let new_sig = new_view.row_meta.get(row as usize).map(|m| m.signature);
            if let (Some(a), Some(b)) = (old_sig, new_sig) {
                if a == b {
                    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
                    continue;
                }
            }
        }

        let old_cells = old_view
            .rows
            .get(row as usize)
            .map(|r| r.cells.as_slice())
            .unwrap_or(&[]);
        let new_cells = new_view
            .rows
            .get(row as usize)
            .map(|r| r.cells.as_slice())
            .unwrap_or(&[]);

        let result = diff_row_pair_sparse(
            ctx,
            row,
            row,
            overlap_cols,
            old_cells,
            new_cells,
        )?;
        compared = compared.saturating_add(result.compared);
        if result.replaced {
            if let Some(existing) = pending_rect.as_mut() {
                let expected_row = existing.start_new + existing.row_count;
                if row == expected_row {
                    existing.row_count = existing.row_count.saturating_add(1);
                } else {
                    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
                    pending_rect = Some(PendingRect {
                        start_old: row,
                        start_new: row,
                        row_count: 1,
                    });
                }
            } else {
                pending_rect = Some(PendingRect {
                    start_old: row,
                    start_new: row,
                    row_count: 1,
                });
            }
        } else {
            flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
            emit_pending_cells(ctx, row, row, result.pending)?;
        }
    }

    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;

    if old.nrows > new.nrows {
        for row in new.nrows..old.nrows {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::row_removed(ctx.sheet_id, row, None))?;
        }
    } else if new.nrows > old.nrows {
        for row in old.nrows..new.nrows {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::row_added(ctx.sheet_id, row, None))?;
        }
    }

    if old.ncols > new.ncols {
        for col in new.ncols..old.ncols {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::column_removed(ctx.sheet_id, col, None))?;
        }
    } else if new.ncols > old.ncols {
        for col in old.ncols..new.ncols {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::column_added(ctx.sheet_id, col, None))?;
        }
    }

    ctx.hardening.progress("cell_diff", 1.0);

    Ok(compared)
}

pub(super) fn positional_diff_for_rows<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    rows: &[u32],
) -> Result<u64, DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);
    let mut compared: u64 = 0;
    let mut pending_rect: Option<PendingRect> = None;

    ctx.hardening.progress("cell_diff", 0.0);

    let mut rows_sorted: Vec<u32> = rows.to_vec();
    rows_sorted.sort_unstable();
    rows_sorted.dedup();

    let total_rows = rows_sorted.len();
    for (idx, &row) in rows_sorted.iter().enumerate() {
        if ctx.hardening.check_timeout(ctx.warnings) {
            flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
            break;
        }
        if total_rows > 0 && idx % 64 == 0 {
            ctx.hardening
                .progress("cell_diff", idx as f32 / total_rows as f32);
        }

        if row >= overlap_rows {
            flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
            continue;
        }

        let result = diff_row_pair(ctx, old, new, row, row, overlap_cols)?;
        compared = compared.saturating_add(result.compared);
        if result.replaced {
            if let Some(existing) = pending_rect.as_mut() {
                let expected_row = existing.start_new + existing.row_count;
                if row == expected_row {
                    existing.row_count = existing.row_count.saturating_add(1);
                } else {
                    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
                    pending_rect = Some(PendingRect {
                        start_old: row,
                        start_new: row,
                        row_count: 1,
                    });
                }
            } else {
                pending_rect = Some(PendingRect {
                    start_old: row,
                    start_new: row,
                    row_count: 1,
                });
            }
        } else {
            flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;
            emit_pending_cells(ctx, row, row, result.pending)?;
        }
    }

    flush_pending_rect(ctx, &mut pending_rect, overlap_cols)?;

    if old.nrows > new.nrows {
        for row in new.nrows..old.nrows {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::row_removed(ctx.sheet_id, row, None))?;
        }
    } else if new.nrows > old.nrows {
        for row in old.nrows..new.nrows {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::row_added(ctx.sheet_id, row, None))?;
        }
    }

    if old.ncols > new.ncols {
        for col in new.ncols..old.ncols {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::column_removed(ctx.sheet_id, col, None))?;
        }
    } else if new.ncols > old.ncols {
        for col in old.ncols..new.ncols {
            if ctx.hardening.check_timeout(ctx.warnings) {
                break;
            }
            ctx.emit(DiffOp::column_added(ctx.sheet_id, col, None))?;
        }
    }

    ctx.hardening.progress("cell_diff", 1.0);

    Ok(compared)
}

#[cfg(feature = "perf-metrics")]
pub(super) fn cells_in_overlap(old: &Grid, new: &Grid) -> u64 {
    let overlap_rows = old.nrows.min(new.nrows) as u64;
    let overlap_cols = old.ncols.min(new.ncols) as u64;
    overlap_rows.saturating_mul(overlap_cols)
}

#[cfg(feature = "perf-metrics")]
pub(super) fn run_positional_diff_from_views_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
) -> Result<(), DiffError> {
    if let Some(m) = ctx.metrics.as_deref_mut() {
        m.start_phase(Phase::CellDiff);
    }
    let compared = positional_diff_from_views(ctx, old, new, old_view, new_view)?;
    if let Some(m) = ctx.metrics.as_deref_mut() {
        m.add_cells_compared(compared);
        m.end_phase(Phase::CellDiff);
    }
    Ok(())
}

#[cfg(not(feature = "perf-metrics"))]
pub(super) fn run_positional_diff_from_views_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
) -> Result<(), DiffError> {
    let _ = positional_diff_from_views(ctx, old, new, old_view, new_view)?;
    Ok(())
}

#[cfg(feature = "perf-metrics")]
pub(super) fn run_positional_diff_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
) -> Result<(), DiffError> {
    if let Some(m) = ctx.metrics.as_deref_mut() {
        m.start_phase(Phase::CellDiff);
    }
    positional_diff(ctx, old, new)?;
    if let Some(m) = ctx.metrics.as_deref_mut() {
        m.add_cells_compared(cells_in_overlap(old, new));
        m.end_phase(Phase::CellDiff);
    }

    Ok(())
}

#[cfg(not(feature = "perf-metrics"))]
pub(super) fn run_positional_diff_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
) -> Result<(), DiffError> {
    positional_diff(ctx, old, new)
}

pub(super) fn try_row_alignment_internal<S: DiffSink>(
    emit_ctx: &mut EmitCtx<'_, '_, S>,
    old_view: &GridView,
    new_view: &GridView,
) -> Result<bool, DiffError> {
    let Some(alignment) = align_row_changes_from_views(old_view, new_view, emit_ctx.config) else {
        return Ok(false);
    };

    emit_ctx.hardening.progress("cell_diff", 0.0);

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = emit_ctx.metrics.as_deref_mut() {
        m.start_phase(Phase::CellDiff);
    }
    let compared = emit_row_aligned_diffs(emit_ctx, old_view, new_view, &alignment)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = emit_ctx.metrics.as_deref_mut() {
        m.add_cells_compared(compared);
        m.end_phase(Phase::CellDiff);
    }

    emit_ctx.hardening.progress("cell_diff", 1.0);

    #[cfg(not(feature = "perf-metrics"))]
    let _ = compared;

    Ok(true)
}

pub(super) fn try_single_column_alignment_internal<S: DiffSink>(
    emit_ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
) -> Result<bool, DiffError> {
    let Some(alignment) =
        align_single_column_change_from_views(old_view, new_view, emit_ctx.config)
    else {
        return Ok(false);
    };

    emit_ctx.hardening.progress("cell_diff", 0.0);

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = emit_ctx.metrics.as_deref_mut() {
        m.start_phase(Phase::CellDiff);
    }
    emit_column_aligned_diffs(emit_ctx, old, new, &alignment)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = emit_ctx.metrics.as_deref_mut() {
        let overlap_rows = old.nrows.min(new.nrows) as u64;
        m.add_cells_compared(overlap_rows.saturating_mul(alignment.matched.len() as u64));
        m.end_phase(Phase::CellDiff);
    }

    emit_ctx.hardening.progress("cell_diff", 1.0);

    Ok(true)
}
