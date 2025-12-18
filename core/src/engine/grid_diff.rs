use crate::alignment_types::RowAlignment;
use crate::column_alignment::ColumnAlignment;
use crate::config::{DiffConfig, LimitBehavior};
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary, FormulaDiffResult};
use crate::formula_diff::{FormulaParseCache, diff_cell_formulas_ids};
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::sink::{DiffSink, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::{Cell, CellAddress, CellSnapshot, Grid};

use super::context::{DiffContext, EmitCtx, emit_op};
use super::move_mask::SheetGridDiffer;
use super::SheetId;

use crate::column_alignment::align_single_column_change_from_views;
use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};
use crate::row_alignment::align_row_changes_from_views;

const DATABASE_MODE_SHEET_ID: &str = "<database>";

pub(crate) fn try_diff_grids<S: DiffSink>(
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
    }

    let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
        || old.ncols.max(new.ncols) > config.max_align_cols;

    if exceeds_limits {
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            pool.resolve(*sheet_id),
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.max_align_rows,
            config.max_align_cols
        );

        match config.on_limit_exceeded {
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id.clone(),
                    rows: old.nrows.max(new.nrows),
                    cols: old.ncols.max(new.ncols),
                    max_rows: config.max_align_rows,
                    max_cols: config.max_align_cols,
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
                );

                #[cfg(feature = "perf-metrics")]
                run_positional_diff_with_metrics(&mut emit_ctx, old, new, metrics.as_deref_mut())?;
                #[cfg(not(feature = "perf-metrics"))]
                run_positional_diff_with_metrics(&mut emit_ctx, old, new)?;

                return Ok(());
            }
        }
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::MoveDetection);
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
        differ.diff_with_masks()?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.end_phase(Phase::CellDiff);
        }
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.start_phase(Phase::Alignment);
    }

    if differ.try_amr()? {
        return Ok(());
    }

    if differ.try_row_alignment()? {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(());
    }

    if differ.try_single_column_alignment()? {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
        return Ok(());
    }

    differ.positional()?;

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
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
    let strings = pool.strings().to_vec();
    DiffReport::from_ops_and_summary(sink.into_ops(), summary, strings)
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

pub(crate) fn compute_formula_diff(
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

pub(crate) fn emit_cell_edit<S: DiffSink>(
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

pub(crate) fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
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

pub(crate) fn grids_non_blank_cells_equal(old: &Grid, new: &Grid) -> bool {
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

#[cfg(feature = "perf-metrics")]
pub(crate) fn cells_in_overlap(old: &Grid, new: &Grid) -> u64 {
    let overlap_rows = old.nrows.min(new.nrows) as u64;
    let overlap_cols = old.ncols.min(new.ncols) as u64;
    overlap_rows.saturating_mul(overlap_cols)
}

#[cfg(feature = "perf-metrics")]
pub(crate) fn run_positional_diff_with_metrics<S: DiffSink>(
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
pub(crate) fn run_positional_diff_with_metrics<S: DiffSink>(
    ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
) -> Result<(), DiffError> {
    positional_diff(ctx, old, new)
}

pub(crate) fn positional_diff<S: DiffSink>(
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

pub(crate) fn emit_row_aligned_diffs<S: DiffSink>(
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

pub(crate) fn diff_row_pair_sparse<S: DiffSink>(
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

pub(crate) fn diff_row_pair<S: DiffSink>(
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

pub(crate) fn emit_column_aligned_diffs<S: DiffSink>(
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

pub(crate) fn snapshot_with_addr(cell: Option<&Cell>, addr: CellAddress) -> CellSnapshot {
    match cell {
        Some(cell) => CellSnapshot {
            addr,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
        },
        None => CellSnapshot::empty(addr),
    }
}

use crate::alignment_types::RowBlockMove;
use crate::column_alignment::ColumnBlockMove;
use crate::rect_block_move::RectBlockMove;

pub(crate) fn emit_row_block_move<S: DiffSink>(
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

pub(crate) fn emit_column_block_move<S: DiffSink>(
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

pub(crate) fn emit_rect_block_move<S: DiffSink>(
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

pub(crate) fn emit_moved_row_block_edits<S: DiffSink>(
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

pub(crate) fn try_row_alignment_internal<S: DiffSink>(
    emit_ctx: &mut EmitCtx<'_, S>,
    old_view: &GridView,
    new_view: &GridView,
    #[cfg(feature = "perf-metrics")] metrics: &mut Option<&mut DiffMetrics>,
) -> Result<bool, DiffError> {
    let Some(alignment) =
        align_row_changes_from_views(old_view, new_view, emit_ctx.config)
    else {
        return Ok(false);
    };

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }
    let compared = emit_row_aligned_diffs(emit_ctx, old_view, new_view, &alignment)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.add_cells_compared(compared);
        m.end_phase(Phase::CellDiff);
    }
    #[cfg(not(feature = "perf-metrics"))]
    let _ = compared;

    Ok(true)
}

pub(crate) fn try_single_column_alignment_internal<S: DiffSink>(
    emit_ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
    #[cfg(feature = "perf-metrics")] metrics: &mut Option<&mut DiffMetrics>,
) -> Result<bool, DiffError> {
    let Some(alignment) = align_single_column_change_from_views(old_view, new_view, emit_ctx.config)
    else {
        return Ok(false);
    };

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }
    emit_column_aligned_diffs(emit_ctx, old, new, &alignment)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        let overlap_rows = old.nrows.min(new.nrows) as u64;
        m.add_cells_compared(overlap_rows.saturating_mul(alignment.matched.len() as u64));
        m.end_phase(Phase::CellDiff);
    }

    Ok(true)
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
}

