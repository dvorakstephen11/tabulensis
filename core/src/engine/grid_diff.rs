use crate::config::{DiffConfig, LimitBehavior};
use crate::diff::{DiffError, DiffOp, DiffReport, DiffSummary};
use crate::formula_diff::FormulaParseCache;
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::sink::{DiffSink, VecSink};
use crate::string_pool::StringPool;
use crate::workbook::{Grid, RowSignature};
use std::collections::HashSet;

use super::SheetId;
use super::context::{DiffContext, EmitCtx, emit_op};
use super::grid_primitives::{
    cells_content_equal, compute_formula_diff, run_positional_diff_with_metrics, snapshot_with_addr,
};
use super::move_mask::SheetGridDiffer;

use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};

const DATABASE_MODE_SHEET_ID: &str = "<database>";

#[allow(clippy::too_many_arguments)]
pub(super) fn try_diff_grids<S: DiffSink>(
    sheet_id: SheetId,
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
            pool.resolve(sheet_id),
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.max_align_rows,
            config.max_align_cols
        );

        match config.on_limit_exceeded {
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id,
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

#[allow(clippy::too_many_arguments)]
fn diff_grids_core<S: DiffSink>(
    sheet_id: SheetId,
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

    let old_view = GridView::from_grid_with_config(old, config);
    let new_view = GridView::from_grid_with_config(new, config);

    let preflight = should_short_circuit_to_positional(&old_view, &new_view, config);

    if matches!(
        preflight,
        PreflightDecision::ShortCircuitNearIdentical | PreflightDecision::ShortCircuitDissimilar
    ) {
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

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = differ.metrics.as_mut() {
        m.start_phase(Phase::MoveDetection);
    }

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
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = differ.metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
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
                sheet_id,
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

    sink.finish()?;
    Ok(DiffSummary {
        complete: true,
        warnings: Vec::new(),
        op_count: *op_count,
        #[cfg(feature = "perf-metrics")]
        metrics: None,
    })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreflightDecision {
    RunFullPipeline,
    ShortCircuitNearIdentical,
    ShortCircuitDissimilar,
}

pub(super) fn should_short_circuit_to_positional(
    old_view: &GridView<'_>,
    new_view: &GridView<'_>,
    config: &DiffConfig,
) -> PreflightDecision {
    let nrows_old = old_view.row_meta.len();
    let nrows_new = new_view.row_meta.len();
    let ncols_old = old_view.col_meta.len();
    let ncols_new = new_view.col_meta.len();

    if nrows_old != nrows_new || ncols_old != ncols_new {
        return PreflightDecision::RunFullPipeline;
    }

    let nrows = nrows_old;
    if nrows < config.preflight_min_rows as usize {
        return PreflightDecision::RunFullPipeline;
    }

    let (in_order_matches, old_sig_set, new_sig_set) =
        compute_row_signature_stats(old_view, new_view);

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

    let multiset_equal = are_multisets_equal(old_view, new_view);

    let near_identical = in_order_mismatches <= config.preflight_in_order_mismatch_max as usize
        && in_order_match_ratio >= config.preflight_in_order_match_ratio_min
        && !multiset_equal;

    if near_identical {
        return PreflightDecision::ShortCircuitNearIdentical;
    }

    if jaccard < config.bailout_similarity_threshold {
        return PreflightDecision::ShortCircuitDissimilar;
    }

    PreflightDecision::RunFullPipeline
}

fn compute_row_signature_stats(
    old_view: &GridView<'_>,
    new_view: &GridView<'_>,
) -> (usize, HashSet<RowSignature>, HashSet<RowSignature>) {
    let mut in_order_matches = 0usize;
    let mut old_sig_set = HashSet::with_capacity(old_view.row_meta.len());
    let mut new_sig_set = HashSet::with_capacity(new_view.row_meta.len());

    for (old_meta, new_meta) in old_view.row_meta.iter().zip(new_view.row_meta.iter()) {
        if old_meta.signature == new_meta.signature {
            in_order_matches += 1;
        }
        old_sig_set.insert(old_meta.signature);
        new_sig_set.insert(new_meta.signature);
    }

    (in_order_matches, old_sig_set, new_sig_set)
}

fn are_multisets_equal(old_view: &GridView<'_>, new_view: &GridView<'_>) -> bool {
    use std::collections::HashMap;

    if old_view.row_meta.len() != new_view.row_meta.len() {
        return false;
    }

    let mut old_freq: HashMap<RowSignature, u32> = HashMap::new();
    for meta in &old_view.row_meta {
        *old_freq.entry(meta.signature).or_insert(0) += 1;
    }

    let mut new_freq: HashMap<RowSignature, u32> = HashMap::new();
    for meta in &new_view.row_meta {
        *new_freq.entry(meta.signature).or_insert(0) += 1;
    }

    old_freq == new_freq
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

        grid_a.insert_cell(1, 1, None, None);

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
        );
        let compared = diff_row_pair_sparse(&mut emit_ctx, 0, 0, 3, &old_cells, &new_cells)
            .expect("diff should succeed");

        assert_eq!(compared, 3);
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
        );
        let compared = diff_row_pair_sparse(&mut emit_ctx, 0, 0, 3, &old_cells, &new_cells)
            .expect("diff should succeed");

        assert_eq!(compared, 2);
    }
}
