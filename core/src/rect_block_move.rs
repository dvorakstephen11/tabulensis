//! Rectangular block move detection.
//!
//! This module implements detection of rectangular regions that have moved
//! between two grids. A rect block move is when a 2D region (rows × cols)
//! moves from one position to another without internal edits.
//!
//! This is used by the engine's masked move detection loop to identify
//! structural changes that preserve content but change position.

use crate::config::DiffConfig;
use crate::grid_view::{ColHash, GridView, HashStats, RowHash};
use crate::workbook::{Cell, Grid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RectBlockMove {
    pub src_start_row: u32,
    pub src_row_count: u32,
    pub src_start_col: u32,
    pub src_col_count: u32,
    pub dst_start_row: u32,
    pub dst_start_col: u32,
    pub block_hash: Option<u64>,
}

pub(crate) fn detect_exact_rect_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RectBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 || old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if view_a.is_low_info_dominated() || view_b.is_low_info_dominated() {
        return None;
    }

    if view_a.is_blank_dominated() || view_b.is_blank_dominated() {
        return None;
    }

    let row_stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    let col_stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);

    if row_stats.has_heavy_repetition(config.alignment.max_hash_repeat)
        || col_stats.has_heavy_repetition(config.alignment.max_hash_repeat)
    {
        return None;
    }

    let shared_rows = row_stats
        .freq_a
        .keys()
        .filter(|h| row_stats.freq_b.contains_key(*h))
        .count();
    let shared_cols = col_stats
        .freq_a
        .keys()
        .filter(|h| col_stats.freq_b.contains_key(*h))
        .count();
    if shared_rows == 0 && shared_cols == 0 {
        return None;
    }

    let diff_positions = collect_differences(old, new);
    if diff_positions.is_empty() {
        return None;
    }

    let row_ranges = find_two_equal_ranges(diff_positions.iter().map(|(r, _)| *r))?;
    let col_ranges = find_two_equal_ranges(diff_positions.iter().map(|(_, c)| *c))?;

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    let expected_mismatches = row_count.checked_mul(col_count)?.checked_mul(2)?;
    if diff_positions.len() as u32 != expected_mismatches {
        return None;
    }

    let mismatches = count_rect_mismatches(old, new, row_ranges.0, col_ranges.0)
        + count_rect_mismatches(old, new, row_ranges.1, col_ranges.1);
    if mismatches != diff_positions.len() as u32 {
        return None;
    }

    if !has_unique_meta(
        &view_a, &view_b, &row_stats, &col_stats, row_ranges, col_ranges,
    ) {
        return None;
    }

    let primary = validate_orientation(old, new, row_ranges, col_ranges);
    let swapped_ranges = ((row_ranges.1, row_ranges.0), (col_ranges.1, col_ranges.0));
    let alternate = validate_orientation(old, new, swapped_ranges.0, swapped_ranges.1);

    match (primary, alternate) {
        (Some(mv), None) => Some(mv),
        (None, Some(mv)) => Some(mv),
        _ => None,
    }
}

fn validate_orientation(
    old: &Grid,
    new: &Grid,
    row_ranges: ((u32, u32), (u32, u32)),
    col_ranges: ((u32, u32), (u32, u32)),
) -> Option<RectBlockMove> {
    if ranges_overlap(row_ranges.0, row_ranges.1) && ranges_overlap(col_ranges.0, col_ranges.1) {
        return None;
    }

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    if rectangles_correspond(
        old,
        new,
        row_ranges.0,
        col_ranges.0,
        row_ranges.1,
        col_ranges.1,
    ) {
        return Some(RectBlockMove {
            src_start_row: row_ranges.0.0,
            src_row_count: row_count,
            src_start_col: col_ranges.0.0,
            src_col_count: col_count,
            dst_start_row: row_ranges.1.0,
            dst_start_col: col_ranges.1.0,
            block_hash: None,
        });
    }

    None
}

fn rectangles_correspond(
    old: &Grid,
    new: &Grid,
    src_rows: (u32, u32),
    src_cols: (u32, u32),
    dst_rows: (u32, u32),
    dst_cols: (u32, u32),
) -> bool {
    let row_count = range_len(src_rows);
    let col_count = range_len(src_cols);

    if row_count != range_len(dst_rows) || col_count != range_len(dst_cols) {
        return false;
    }

    for dr in 0..row_count {
        for dc in 0..col_count {
            let src_r = src_rows.0 + dr;
            let src_c = src_cols.0 + dc;
            let dst_r = dst_rows.0 + dr;
            let dst_c = dst_cols.0 + dc;

            if !cell_content_equal(old.get(src_r, src_c), new.get(dst_r, dst_c)) {
                return false;
            }
        }
    }

    true
}

fn collect_differences(old: &Grid, new: &Grid) -> Vec<(u32, u32)> {
    let mut diffs = Vec::new();

    for row in 0..old.nrows {
        for col in 0..old.ncols {
            if !cell_content_equal(old.get(row, col), new.get(row, col)) {
                diffs.push((row, col));
            }
        }
    }

    diffs
}

fn cell_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
        (Some(cell_a), None) => cell_a.value.is_none() && cell_a.formula.is_none(),
        (None, Some(cell_b)) => cell_b.value.is_none() && cell_b.formula.is_none(),
    }
}

fn count_rect_mismatches(old: &Grid, new: &Grid, rows: (u32, u32), cols: (u32, u32)) -> u32 {
    let mut mismatches = 0u32;
    for row in rows.0..=rows.1 {
        for col in cols.0..=cols.1 {
            if !cell_content_equal(old.get(row, col), new.get(row, col)) {
                mismatches = mismatches.saturating_add(1);
            }
        }
    }
    mismatches
}

fn has_unique_meta(
    view_a: &GridView<'_>,
    view_b: &GridView<'_>,
    row_stats: &HashStats<RowHash>,
    col_stats: &HashStats<ColHash>,
    row_ranges: ((u32, u32), (u32, u32)),
    col_ranges: ((u32, u32), (u32, u32)),
) -> bool {
    for range in [row_ranges.0, row_ranges.1] {
        for idx in range.0..=range.1 {
            if !is_unique_row_in_a(idx, view_a, row_stats)
                || !is_unique_row_in_b(idx, view_b, row_stats)
            {
                return false;
            }
        }
    }

    for range in [col_ranges.0, col_ranges.1] {
        for idx in range.0..=range.1 {
            if !is_unique_col_in_a(idx, view_a, col_stats)
                || !is_unique_col_in_b(idx, view_b, col_stats)
            {
                return false;
            }
        }
    }

    true
}

fn is_unique_row_in_a(idx: u32, view: &GridView<'_>, stats: &HashStats<RowHash>) -> bool {
    view.row_meta
        .get(idx as usize)
        .map(|meta| unique_in_a(meta.signature, stats))
        .unwrap_or(false)
}

fn is_unique_row_in_b(idx: u32, view: &GridView<'_>, stats: &HashStats<RowHash>) -> bool {
    view.row_meta
        .get(idx as usize)
        .map(|meta| unique_in_b(meta.signature, stats))
        .unwrap_or(false)
}

fn is_unique_col_in_a(idx: u32, view: &GridView<'_>, stats: &HashStats<ColHash>) -> bool {
    view.col_meta
        .get(idx as usize)
        .map(|meta| unique_in_a(meta.hash, stats))
        .unwrap_or(false)
}

fn is_unique_col_in_b(idx: u32, view: &GridView<'_>, stats: &HashStats<ColHash>) -> bool {
    view.col_meta
        .get(idx as usize)
        .map(|meta| unique_in_b(meta.hash, stats))
        .unwrap_or(false)
}

fn find_two_equal_ranges<I>(indices: I) -> Option<((u32, u32), (u32, u32))>
where
    I: IntoIterator<Item = u32>,
{
    let mut values: Vec<u32> = indices.into_iter().collect();
    if values.is_empty() {
        return None;
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

    match ranges.len() {
        1 => Some((ranges[0], ranges[0])),
        2 => {
            let len0 = range_len(ranges[0]);
            let len1 = range_len(ranges[1]);
            if len0 != len1 {
                return None;
            }
            Some((ranges[0], ranges[1]))
        }
        _ => None,
    }
}

fn range_len(range: (u32, u32)) -> u32 {
    range.1.saturating_sub(range.0).saturating_add(1)
}

fn ranges_overlap(a: (u32, u32), b: (u32, u32)) -> bool {
    !(a.1 < b.0 || b.1 < a.0)
}

fn is_within_size_bounds(old: &Grid, new: &Grid, config: &DiffConfig) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= config.alignment.max_align_rows && cols <= config.alignment.max_align_cols
}

fn unique_in_a<H>(hash: H, stats: &HashStats<H>) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) <= 1
}

fn unique_in_b<H>(hash: H, stats: &HashStats<H>) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_a.get(&hash).copied().unwrap_or(0) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::CellValue;

    fn grid_from_numbers(values: &[&[i32]]) -> Grid {
        let nrows = values.len() as u32;
        let ncols = if nrows == 0 {
            0
        } else {
            values[0].len() as u32
        };

        let mut grid = Grid::new(nrows, ncols);
        for (r, row_vals) in values.iter().enumerate() {
            for (c, v) in row_vals.iter().enumerate() {
                grid.insert_cell(r as u32, c as u32, Some(CellValue::Number(*v as f64)), None);
            }
        }

        grid
    }

    fn base_background(rows: usize, cols: usize) -> Vec<Vec<i32>> {
        (0..rows)
            .map(|r| (0..cols).map(|c| (r as i32) * 1_000 + c as i32).collect())
            .collect()
    }

    fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
        for (r_offset, row_vals) in block.iter().enumerate() {
            for (c_offset, value) in row_vals.iter().enumerate() {
                let row = top + r_offset;
                let col = left + c_offset;
                if let Some(row_slice) = target.get_mut(row)
                    && let Some(cell) = row_slice.get_mut(col)
                {
                    *cell = *value;
                }
            }
        }
    }

    fn grid_from_matrix(matrix: Vec<Vec<i32>>) -> Grid {
        let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
        grid_from_numbers(&refs)
    }

    #[test]
    fn detect_simple_rect_block_move_success() {
        let mut grid_a = base_background(12, 12);
        let mut grid_b = base_background(12, 12);

        let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

        place_block(&mut grid_a, 1, 1, &block);
        place_block(&mut grid_b, 7, 6, &block);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_some(),
            "should detect exact rectangular block move"
        );

        let mv = result.unwrap();
        assert_eq!(mv.src_start_row, 1);
        assert_eq!(mv.src_row_count, 3);
        assert_eq!(mv.src_start_col, 1);
        assert_eq!(mv.src_col_count, 3);
        assert_eq!(mv.dst_start_row, 7);
        assert_eq!(mv.dst_start_col, 6);
    }

    #[test]
    fn detect_rect_block_move_with_shared_columns() {
        let mut grid_a = base_background(10, 10);
        let mut grid_b = base_background(10, 10);

        let block = vec![vec![11, 12], vec![21, 22]];

        place_block(&mut grid_a, 1, 2, &block);
        place_block(&mut grid_b, 6, 2, &block);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_some(),
            "should detect a vertical rect move when columns overlap"
        );

        let mv = result.unwrap();
        assert_eq!(mv.src_start_row, 1);
        assert_eq!(mv.dst_start_row, 6);
        assert_eq!(mv.src_start_col, 2);
        assert_eq!(mv.dst_start_col, 2);
        assert_eq!(mv.src_row_count, 2);
        assert_eq!(mv.src_col_count, 2);
    }

    #[test]
    fn detect_bails_on_different_grid_dimensions() {
        let old = grid_from_numbers(&[&[1, 2], &[3, 4]]);
        let new = grid_from_numbers(&[&[1, 2, 5], &[3, 4, 6]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(result.is_none(), "different dimensions should bail");
    }

    #[test]
    fn detect_bails_on_empty_grid() {
        let old = Grid::new(0, 0);
        let new = Grid::new(0, 0);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(result.is_none(), "empty grid should bail");
    }

    #[test]
    fn detect_bails_on_identical_grids() {
        let old = grid_from_numbers(&[&[1, 2], &[3, 4]]);
        let new = grid_from_numbers(&[&[1, 2], &[3, 4]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "identical grids should bail (no differences)"
        );
    }

    #[test]
    fn detect_bails_on_internal_cell_edit() {
        let mut grid_a = base_background(10, 10);
        let mut grid_b = base_background(10, 10);

        let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

        place_block(&mut grid_a, 1, 1, &block);
        place_block(&mut grid_b, 6, 4, &block);
        grid_b[7][5] = 9_999;

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "move with internal edit should not be detected as exact rectangular move"
        );
    }

    #[test]
    fn detect_bails_on_ambiguous_block_swap() {
        let base: Vec<Vec<i32>> = (0..6)
            .map(|r| (0..6).map(|c| 100 * r + c).collect())
            .collect();
        let mut grid_a = base.clone();
        let mut grid_b = base.clone();

        let block_one = vec![vec![900, 901], vec![902, 903]];
        let block_two = vec![vec![700, 701], vec![702, 703]];

        place_block(&mut grid_a, 0, 0, &block_one);
        place_block(&mut grid_a, 3, 3, &block_two);

        place_block(&mut grid_b, 0, 0, &block_two);
        place_block(&mut grid_b, 3, 3, &block_one);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "ambiguous block swap should not emit a rectangular move"
        );
    }

    #[allow(clippy::field_reassign_with_default)]
    #[test]
    fn detect_bails_on_oversized_row_count() {
        let mut config = DiffConfig::default();
        config.alignment.max_align_rows = 10;
        let old = Grid::new(config.alignment.max_align_rows + 1, 10);
        let new = Grid::new(config.alignment.max_align_rows + 1, 10);

        let result = detect_exact_rect_block_move(&old, &new, &config);
        assert!(
            result.is_none(),
            "grids exceeding configured max_align_rows should bail"
        );
    }

    #[allow(clippy::field_reassign_with_default)]
    #[test]
    fn detect_bails_on_oversized_col_count() {
        let mut config = DiffConfig::default();
        config.alignment.max_align_cols = 8;
        let old = Grid::new(10, config.alignment.max_align_cols + 1);
        let new = Grid::new(10, config.alignment.max_align_cols + 1);

        let result = detect_exact_rect_block_move(&old, &new, &config);
        assert!(
            result.is_none(),
            "grids exceeding configured max_align_cols should bail"
        );
    }

    #[test]
    fn detect_bails_on_single_cell_edit() {
        let old = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);
        let new = grid_from_numbers(&[&[1, 2, 3], &[4, 99, 6], &[7, 8, 9]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "single cell edit is not a rectangular block move"
        );
    }

    #[test]
    fn detect_bails_on_pure_row_move_pattern() {
        let old = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9], &[10, 11, 12]]);
        let new = grid_from_numbers(&[&[7, 8, 9], &[4, 5, 6], &[1, 2, 3], &[10, 11, 12]]);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "pure row swap without column displacement is not a rectangular block move"
        );
    }

    #[test]
    fn detect_bails_on_non_contiguous_differences() {
        let mut grid_a = base_background(8, 8);
        let mut grid_b = base_background(8, 8);

        grid_a[1][1] = 111;
        grid_a[5][5] = 555;
        grid_a[1][5] = 115;
        grid_b[1][1] = 555;
        grid_b[5][5] = 111;
        grid_b[1][5] = 999;

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new, &DiffConfig::default());
        assert!(
            result.is_none(),
            "non-contiguous differences should not form a rectangular block move"
        );
    }
}
