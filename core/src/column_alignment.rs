use crate::config::DiffConfig;
use crate::grid_view::{ColHash, ColMeta, GridView, HashStats};
use crate::hashing::hash_col_content_unordered_128;
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnAlignment {
    pub(crate) matched: Vec<(u32, u32)>, // (col_idx_a, col_idx_b)
    pub(crate) inserted: Vec<u32>,       // columns present only in B
    pub(crate) deleted: Vec<u32>,        // columns present only in A
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnBlockMove {
    pub src_start_col: u32,
    pub dst_start_col: u32,
    pub col_count: u32,
}

fn unordered_col_hashes(grid: &Grid) -> Vec<ColHash> {
    let mut col_cells: Vec<Vec<&crate::workbook::Cell>> = vec![Vec::new(); grid.ncols as usize];
    for cell in grid.cells.values() {
        let idx = cell.col as usize;
        col_cells[idx].push(cell);
    }
    col_cells
        .iter()
        .map(|cells| hash_col_content_unordered_128(cells))
        .collect()
}

pub(crate) fn detect_exact_column_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<ColumnBlockMove> {
    if old.ncols != new.ncols || old.nrows != new.nrows {
        return None;
    }

    if old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    let unordered_a = unordered_col_hashes(old);
    let unordered_b = unordered_col_hashes(new);

    let col_meta_a: Vec<ColMeta> = view_a
        .col_meta
        .iter()
        .enumerate()
        .map(|(idx, meta)| ColMeta {
            hash: *unordered_a.get(idx).unwrap_or(&meta.hash),
            ..*meta
        })
        .collect();
    let col_meta_b: Vec<ColMeta> = view_b
        .col_meta
        .iter()
        .enumerate()
        .map(|(idx, meta)| ColMeta {
            hash: *unordered_b.get(idx).unwrap_or(&meta.hash),
            ..*meta
        })
        .collect();

    if blank_dominated(&view_a) || blank_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_col_meta(&col_meta_a, &col_meta_b);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    let meta_a = &col_meta_a;
    let meta_b = &col_meta_b;
    let n = meta_a.len();

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let prefix = (0..n).find(|&idx| meta_a[idx].hash != meta_b[idx].hash)?;

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = n - 1 - suffix_len;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }
    let tail_start = n - suffix_len;

    let try_candidate = |src_start: usize, dst_start: usize| -> Option<ColumnBlockMove> {
        if src_start >= tail_start || dst_start >= tail_start {
            return None;
        }

        let mut len = 0usize;
        while src_start + len < tail_start && dst_start + len < tail_start {
            if meta_a[src_start + len].hash != meta_b[dst_start + len].hash {
                break;
            }
            len += 1;
        }

        if len == 0 {
            return None;
        }

        let src_end = src_start + len;
        let dst_end = dst_start + len;

        if !(src_end <= dst_start || dst_end <= src_start) {
            return None;
        }

        let mut idx_a = 0usize;
        let mut idx_b = 0usize;

        loop {
            if idx_a == src_start {
                idx_a = src_end;
            }
            if idx_b == dst_start {
                idx_b = dst_end;
            }

            if idx_a >= n && idx_b >= n {
                break;
            }

            if idx_a >= n || idx_b >= n {
                return None;
            }

            if meta_a[idx_a].hash != meta_b[idx_b].hash {
                return None;
            }

            idx_a += 1;
            idx_b += 1;
        }

        for meta in &meta_a[src_start..src_end] {
            if stats.freq_a.get(&meta.hash).copied().unwrap_or(0) != 1
                || stats.freq_b.get(&meta.hash).copied().unwrap_or(0) != 1
            {
                return None;
            }
        }

        Some(ColumnBlockMove {
            src_start_col: meta_a[src_start].col_idx,
            dst_start_col: meta_b[dst_start].col_idx,
            col_count: len as u32,
        })
    };

    if let Some(src_start) =
        (prefix..tail_start).find(|&idx| meta_a[idx].hash == meta_b[prefix].hash)
        && let Some(mv) = try_candidate(src_start, prefix)
    {
        return Some(mv);
    }

    if let Some(dst_start) =
        (prefix..tail_start).find(|&idx| meta_b[idx].hash == meta_a[prefix].hash)
        && let Some(mv) = try_candidate(prefix, dst_start)
    {
        return Some(mv);
    }

    None
}

#[allow(dead_code)]
pub(crate) fn align_single_column_change(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<ColumnAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    align_single_column_change_from_views(&view_a, &view_b, config)
}

pub(crate) fn align_single_column_change_from_views(
    view_a: &GridView,
    view_b: &GridView,
    config: &DiffConfig,
) -> Option<ColumnAlignment> {
    if !is_within_size_bounds(view_a.source, view_b.source, config) {
        return None;
    }

    if view_a.source.nrows != view_b.source.nrows {
        return None;
    }

    let col_diff = view_b.source.ncols as i64 - view_a.source.ncols as i64;
    if col_diff.abs() != 1 {
        return None;
    }

    let stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    if col_diff == 1 {
        find_single_gap_alignment(
            &view_a.col_meta,
            &view_b.col_meta,
            &stats,
            ColumnChange::Insert,
        )
    } else {
        find_single_gap_alignment(
            &view_a.col_meta,
            &view_b.col_meta,
            &stats,
            ColumnChange::Delete,
        )
    }
}

enum ColumnChange {
    Insert,
    Delete,
}

fn find_single_gap_alignment(
    cols_a: &[ColMeta],
    cols_b: &[ColMeta],
    stats: &HashStats<ColHash>,
    change: ColumnChange,
) -> Option<ColumnAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = false;

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < cols_a.len() && idx_b < cols_b.len() {
        let meta_a = cols_a[idx_a];
        let meta_b = cols_b[idx_b];

        if meta_a.hash == meta_b.hash {
            matched.push((meta_a.col_idx, meta_b.col_idx));
            idx_a += 1;
            idx_b += 1;
            continue;
        }

        if skipped {
            return None;
        }

        match change {
            ColumnChange::Insert => {
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.col_idx);
                idx_b += 1;
            }
            ColumnChange::Delete => {
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.col_idx);
                idx_a += 1;
            }
        }

        skipped = true;
    }

    if idx_a < cols_a.len() || idx_b < cols_b.len() {
        if skipped {
            return None;
        }

        match change {
            ColumnChange::Insert if idx_a == cols_a.len() && cols_b.len() == idx_b + 1 => {
                let meta_b = cols_b[idx_b];
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.col_idx);
            }
            ColumnChange::Delete if idx_b == cols_b.len() && cols_a.len() == idx_a + 1 => {
                let meta_a = cols_a[idx_a];
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.col_idx);
            }
            _ => return None,
        }
    }

    if inserted.len() + deleted.len() != 1 {
        return None;
    }

    let alignment = ColumnAlignment {
        matched,
        inserted,
        deleted,
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn is_monotonic(pairs: &[(u32, u32)]) -> bool {
    pairs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1)
}

fn is_unique_to_b(hash: ColHash, stats: &HashStats<ColHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 0
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
}

fn is_unique_to_a(hash: ColHash, stats: &HashStats<ColHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 0
}

fn is_within_size_bounds(old: &Grid, new: &Grid, config: &DiffConfig) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= config.max_align_rows && cols <= config.max_align_cols
}

fn has_heavy_repetition(stats: &HashStats<ColHash>, config: &DiffConfig) -> bool {
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > config.max_hash_repeat
}

fn blank_dominated(view: &GridView<'_>) -> bool {
    if view.col_meta.is_empty() {
        return false;
    }

    let blank_cols = view
        .col_meta
        .iter()
        .filter(|meta| meta.non_blank_count == 0)
        .count();

    blank_cols * 2 > view.col_meta.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::{Cell, CellAddress, CellValue};

    fn grid_from_numbers(rows: &[&[i32]]) -> Grid {
        let nrows = rows.len() as u32;
        let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
        let mut grid = Grid::new(nrows, ncols);

        for (r_idx, row_vals) in rows.iter().enumerate() {
            for (c_idx, value) in row_vals.iter().enumerate() {
                grid.insert(Cell {
                    row: r_idx as u32,
                    col: c_idx as u32,
                    address: CellAddress::from_indices(r_idx as u32, c_idx as u32),
                    value: Some(CellValue::Number(*value as f64)),
                    formula: None,
                });
            }
        }

        grid
    }

    #[test]
    fn single_insert_aligns_all_columns() {
        let base_rows: Vec<Vec<i32>> =
            vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8], vec![9, 10, 11, 12]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&base_refs);

        let inserted_rows: Vec<Vec<i32>> = base_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let mut new_row = row.clone();
                new_row.insert(2, 100 + idx as i32); // insert at index 2 (0-based)
                new_row
            })
            .collect();
        let inserted_refs: Vec<&[i32]> = inserted_rows.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&inserted_refs);

        let alignment = align_single_column_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");

        assert_eq!(alignment.inserted, vec![2]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[1], (1, 1));
        assert_eq!(alignment.matched[2], (2, 3));
        assert_eq!(alignment.matched[3], (3, 4));
    }

    #[test]
    fn multiple_unique_columns_causes_bailout() {
        let base_rows: Vec<Vec<i32>> = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&base_refs);

        let mut rows_b: Vec<Vec<i32>> = base_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let mut new_row = row.clone();
                new_row.insert(1, 100 + idx as i32); // inserted column
                new_row
            })
            .collect();
        if let Some(cell) = rows_b.get_mut(1).and_then(|row| row.get_mut(3)) {
            *cell = 999;
        }
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&rows_b_refs);

        assert!(align_single_column_change(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn heavy_repetition_causes_bailout() {
        let repetitive_cols = 9;
        let rows: usize = 3;

        let values_a: Vec<Vec<i32>> = (0..rows).map(|_| vec![1; repetitive_cols]).collect();
        let refs_a: Vec<&[i32]> = values_a.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&refs_a);

        let values_b: Vec<Vec<i32>> = (0..rows)
            .map(|row_idx| {
                let mut row = vec![1; repetitive_cols];
                row.insert(4, 2 + row_idx as i32);
                row
            })
            .collect();
        let refs_b: Vec<&[i32]> = values_b.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&refs_b);

        assert!(align_single_column_change(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_simple_case() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

        let grid_b = grid_from_numbers(&[&[10, 30, 40, 20], &[11, 31, 41, 21]]);

        let mv = detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected column move found");
        assert_eq!(mv.src_start_col, 1);
        assert_eq!(mv.col_count, 1);
        assert_eq!(mv.dst_start_col, 3);
    }

    #[test]
    fn detect_exact_column_block_move_rejects_internal_edits() {
        let grid_a = grid_from_numbers(&[&[1, 2, 3, 4], &[5, 6, 7, 8], &[9, 10, 11, 12]]);

        let grid_b = grid_from_numbers(&[
            &[1, 3, 4, 2],
            &[5, 7, 8, 6],
            &[9, 11, 12, 999], // edit inside moved column
        ]);

        assert!(detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_rejects_repetition() {
        let grid_a = grid_from_numbers(&[&[1, 1, 2, 2], &[10, 10, 20, 20]]);
        let grid_b = grid_from_numbers(&[&[2, 2, 1, 1], &[20, 20, 10, 10]]);

        assert!(detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_multi_column_block() {
        let grid_a = grid_from_numbers(&[
            &[10, 20, 30, 40, 50, 60],
            &[11, 21, 31, 41, 51, 61],
            &[12, 22, 32, 42, 52, 62],
        ]);

        let grid_b = grid_from_numbers(&[
            &[10, 40, 50, 20, 30, 60],
            &[11, 41, 51, 21, 31, 61],
            &[12, 42, 52, 22, 32, 62],
        ]);

        let mv = detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected multi-column move");
        assert_eq!(mv.src_start_col, 3);
        assert_eq!(mv.col_count, 2);
        assert_eq!(mv.dst_start_col, 1);
    }

    #[test]
    fn detect_exact_column_block_move_rejects_two_independent_moves() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40, 50, 60], &[11, 21, 31, 41, 51, 61]]);

        let grid_b = grid_from_numbers(&[&[20, 10, 30, 40, 60, 50], &[21, 11, 31, 41, 61, 51]]);

        assert!(
            detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "two independent column swaps must not be detected as a single block move"
        );
    }

    #[test]
    fn detect_exact_column_block_move_swap_as_single_move() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

        let grid_b = grid_from_numbers(&[&[20, 10, 30, 40], &[21, 11, 31, 41]]);

        let mv = detect_exact_column_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("swap of adjacent columns should be detected as single-column move");
        assert_eq!(mv.col_count, 1);
        assert!(
            (mv.src_start_col == 0 && mv.dst_start_col == 1)
                || (mv.src_start_col == 1 && mv.dst_start_col == 0),
            "swap should be represented as moving one column past the other"
        );
    }
}
