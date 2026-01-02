use crate::alignment_types::RowBlockMove;
use crate::column_alignment::{ColumnBlockMove, detect_exact_column_block_move};
use crate::config::DiffConfig;
use crate::diff::DiffError;
use crate::grid_view::GridView;
use crate::rect_block_move::{RectBlockMove, detect_exact_rect_block_move};
use crate::region_mask::RegionMask;
use crate::row_alignment::{detect_exact_row_block_move, detect_fuzzy_row_block_move};
use crate::sink::DiffSink;
use crate::workbook::{CellAddress, ColSignature, Grid, RowSignature};

use std::collections::{BTreeMap, HashSet};

use super::amr::try_diff_with_amr;
use super::context::EmitCtx;
use super::grid_primitives::{
    cells_content_equal, emit_cell_edit, emit_column_block_move, emit_moved_row_block_edits,
    emit_rect_block_move, emit_row_block_move, run_positional_diff_from_views_with_metrics,
    try_row_alignment_internal, try_single_column_alignment_internal,
};

pub(super) struct SheetGridDiffer<'a, 'p, 'b, S: DiffSink> {
    pub(super) emit_ctx: EmitCtx<'a, 'p, S>,
    pub(super) old: &'b Grid,
    pub(super) new: &'b Grid,
    pub(super) old_view: GridView<'b>,
    pub(super) new_view: GridView<'b>,
    pub(super) old_mask: RegionMask,
    pub(super) new_mask: RegionMask,
}

impl<'a, 'p, 'b, S: DiffSink> SheetGridDiffer<'a, 'p, 'b, S> {
    pub(super) fn from_views(
        emit_ctx: EmitCtx<'a, 'p, S>,
        old: &'b Grid,
        new: &'b Grid,
        old_view: GridView<'b>,
        new_view: GridView<'b>,
    ) -> Self {
        let old_mask = RegionMask::all_active(old.nrows, old.ncols);
        let new_mask = RegionMask::all_active(new.nrows, new.ncols);

        Self {
            emit_ctx,
            old,
            new,
            old_view,
            new_view,
            old_mask,
            new_mask,
        }
    }

    fn move_detection_enabled(&self) -> bool {
        self.old.nrows.max(self.new.nrows) <= self.emit_ctx.config.moves.max_move_detection_rows
            && self.old.ncols.max(self.new.ncols) <= self.emit_ctx.config.moves.max_move_detection_cols
    }

    pub(super) fn detect_moves(&mut self) -> Result<u32, DiffError> {
        if !self.move_detection_enabled() {
            return Ok(0);
        }

        let mut iteration = 0u32;
        let config = self.emit_ctx.config;

        loop {
            if iteration >= config.moves.max_move_iterations {
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
                if let Some(m) = self.emit_ctx.metrics.as_deref_mut() {
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
                if let Some(m) = self.emit_ctx.metrics.as_deref_mut() {
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
                if let Some(m) = self.emit_ctx.metrics.as_deref_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                self.old_mask.exclude_cols(mv.src_start_col, mv.col_count);
                self.new_mask.exclude_cols(mv.dst_start_col, mv.col_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && config.moves.enable_fuzzy_moves
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
                if let Some(m) = self.emit_ctx.metrics.as_deref_mut() {
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

    pub(super) fn has_mask_exclusions(&self) -> bool {
        self.old_mask.has_exclusions() || self.new_mask.has_exclusions()
    }

    pub(super) fn diff_with_masks(&mut self) -> Result<(), DiffError> {
        if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
            if diff_aligned_with_masks(
                &mut self.emit_ctx,
                self.old,
                self.new,
                &self.old_mask,
                &self.new_mask,
            )? {
                return Ok(());
            }
            positional_diff_with_masks(
                &mut self.emit_ctx,
                self.old,
                self.new,
                &self.old_mask,
                &self.new_mask,
            )?;
            return Ok(());
        }

        positional_diff_masked_equal_size(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_mask,
            &self.new_mask,
        )?;

        Ok(())
    }

    pub(super) fn try_amr(&mut self) -> Result<bool, DiffError> {
        let handled =
            try_diff_with_amr(&mut self.emit_ctx, self.old, self.new, &self.old_view, &self.new_view)?;
        Ok(handled)
    }

    pub(super) fn try_row_alignment(&mut self) -> Result<bool, DiffError> {
        return try_row_alignment_internal(&mut self.emit_ctx, &self.old_view, &self.new_view);
    }

    pub(super) fn try_single_column_alignment(&mut self) -> Result<bool, DiffError> {
        return try_single_column_alignment_internal(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_view,
            &self.new_view,
        );
    }

    pub(super) fn positional(&mut self) -> Result<(), DiffError> {
        run_positional_diff_from_views_with_metrics(
            &mut self.emit_ctx,
            self.old,
            self.new,
            &self.old_view,
            &self.new_view,
        )?;
        Ok(())
    }
}

pub(super) fn row_signature_at(grid: &Grid, row: u32) -> Option<RowSignature> {
    if let Some(sig) = grid
        .row_signatures
        .as_ref()
        .and_then(|rows| rows.get(row as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_row_signature(row))
}

pub(super) fn col_signature_at(grid: &Grid, col: u32) -> Option<ColSignature> {
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

        projected.insert_cell(new_row, new_col, cell.value, cell.formula);
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

        projected.insert_cell(new_row, new_col, cell.value, cell.formula);
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

fn diff_aligned_with_masks<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
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

    ctx.hardening.progress("cell_diff", 0.0);

    let total_rows = rows_a.len();
    for (idx, (row_a, row_b)) in rows_a.iter().zip(rows_b.iter()).enumerate() {
        if ctx.hardening.check_timeout(ctx.warnings) {
            return Ok(true);
        }

        if total_rows > 0 && idx % 64 == 0 {
            ctx.hardening
                .progress("cell_diff", idx as f32 / total_rows as f32);
        }

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

    ctx.hardening.progress("cell_diff", 1.0);

    let rows_a_set: HashSet<u32> = rows_a.iter().copied().collect();
    let rows_b_set: HashSet<u32> = rows_b.iter().copied().collect();

    for row_idx in new_rows.iter().filter(|r| !rows_b_set.contains(r)) {
        if new_mask.is_row_active(*row_idx) {
            ctx.emit(crate::diff::DiffOp::row_added(ctx.sheet_id, *row_idx, None))?;
        }
    }

    for row_idx in old_rows.iter().filter(|r| !rows_a_set.contains(r)) {
        if old_mask.is_row_active(*row_idx) {
            ctx.emit(crate::diff::DiffOp::row_removed(
                ctx.sheet_id,
                *row_idx,
                None,
            ))?;
        }
    }

    let cols_a_set: HashSet<u32> = cols_a.iter().copied().collect();
    let cols_b_set: HashSet<u32> = cols_b.iter().copied().collect();

    for col_idx in new_cols.iter().filter(|c| !cols_b_set.contains(c)) {
        if new_mask.is_col_active(*col_idx) {
            ctx.emit(crate::diff::DiffOp::column_added(
                ctx.sheet_id,
                *col_idx,
                None,
            ))?;
        }
    }

    for col_idx in old_cols.iter().filter(|c| !cols_a_set.contains(c)) {
        if old_mask.is_col_active(*col_idx) {
            ctx.emit(crate::diff::DiffOp::column_removed(
                ctx.sheet_id,
                *col_idx,
                None,
            ))?;
        }
    }

    Ok(true)
}

fn positional_diff_with_masks<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
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
            if new_mask.is_row_active(row_idx) {
                ctx.emit(crate::diff::DiffOp::row_added(ctx.sheet_id, row_idx, None))?;
            }
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            if row_idx % 4096 == 0 && ctx.hardening.check_timeout(ctx.warnings) {
                return Ok(());
            }
            if old_mask.is_row_active(row_idx) {
                ctx.emit(crate::diff::DiffOp::row_removed(
                    ctx.sheet_id,
                    row_idx,
                    None,
                ))?;
            }
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            if col_idx % 4096 == 0 && ctx.hardening.check_timeout(ctx.warnings) {
                return Ok(());
            }
            if new_mask.is_col_active(col_idx) {
                ctx.emit(crate::diff::DiffOp::column_added(
                    ctx.sheet_id,
                    col_idx,
                    None,
                ))?;
            }
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            if col_idx % 4096 == 0 && ctx.hardening.check_timeout(ctx.warnings) {
                return Ok(());
            }
            if old_mask.is_col_active(col_idx) {
                ctx.emit(crate::diff::DiffOp::column_removed(
                    ctx.sheet_id,
                    col_idx,
                    None,
                ))?;
            }
        }
    }

    Ok(())
}

fn positional_diff_masked_equal_size<S: DiffSink>(
    ctx: &mut EmitCtx<'_, '_, S>,
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

    ctx.hardening.progress("cell_diff", 0.0);

    let total_rows = stable_rows.len();
    for (idx, &row) in stable_rows.iter().enumerate() {
        if ctx.hardening.check_timeout(ctx.warnings) {
            return Ok(());
        }
        if total_rows > 0 && idx % 64 == 0 {
            ctx.hardening
                .progress("cell_diff", idx as f32 / total_rows as f32);
        }
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

    ctx.hardening.progress("cell_diff", 1.0);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::CellValue;

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
