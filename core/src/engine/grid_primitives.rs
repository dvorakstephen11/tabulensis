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

pub(super) fn diff_row_pair_sparse<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
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

pub(super) fn diff_row_pair<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
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

pub(super) fn emit_row_aligned_diffs<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old_view: &GridView,
    new_view: &GridView,
    alignment: &RowAlignment,
) -> Result<u64, DiffError> {
    let overlap_cols = old_view.source.ncols.min(new_view.source.ncols);
    let mut compared = 0u64;

    for (row_a, row_b) in &alignment.matched {
        if !ctx.config.include_unchanged_cells {
            let old_sig = old_view.row_meta.get(*row_a as usize).map(|m| m.signature);
            let new_sig = new_view.row_meta.get(*row_b as usize).map(|m| m.signature);
            if let (Some(a), Some(b)) = (old_sig, new_sig) {
                if a == b {
                    continue;
                }
            }
        }
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

    ctx.hardening.progress("cell_diff", 0.0);

    for row in 0..overlap_rows {
        if ctx.hardening.check_timeout(ctx.warnings) {
            return Ok(());
        }

        if overlap_rows > 0 && row % 256 == 0 {
            ctx.hardening
                .progress("cell_diff", row as f32 / overlap_rows as f32);
        }

        diff_row_pair(ctx, old, new, row, row, overlap_cols)?;
    }

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

    for row in 0..overlap_rows {
        if ctx.hardening.check_timeout(ctx.warnings) {
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

        compared = compared.saturating_add(diff_row_pair_sparse(
            ctx,
            row,
            row,
            overlap_cols,
            old_cells,
            new_cells,
        )?);
    }

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

pub(super) fn positional_diff_from_views_for_rows<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
    rows: &[u32],
) -> Result<u64, DiffError> {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);
    let mut compared: u64 = 0;

    ctx.hardening.progress("cell_diff", 0.0);

    let mut rows_sorted: Vec<u32> = rows.to_vec();
    rows_sorted.sort_unstable();
    rows_sorted.dedup();

    let total_rows = rows_sorted.len();
    for (idx, &row) in rows_sorted.iter().enumerate() {
        if ctx.hardening.check_timeout(ctx.warnings) {
            break;
        }
        if total_rows > 0 && idx % 64 == 0 {
            ctx.hardening
                .progress("cell_diff", idx as f32 / total_rows as f32);
        }

        if row >= overlap_rows {
            continue;
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

        compared = compared.saturating_add(diff_row_pair_sparse(
            ctx,
            row,
            row,
            overlap_cols,
            old_cells,
            new_cells,
        )?);
    }

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
