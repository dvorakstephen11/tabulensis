use crate::grid_view::{ColHash, ColMeta, GridView, HashStats};
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnAlignment {
    pub(crate) matched: Vec<(u32, u32)>, // (col_idx_a, col_idx_b)
    pub(crate) inserted: Vec<u32>,       // columns present only in B
    pub(crate) deleted: Vec<u32>,        // columns present only in A
}

const MAX_ALIGN_ROWS: u32 = 2_000;
const MAX_ALIGN_COLS: u32 = 64;
const MAX_HASH_REPEAT: u32 = 8;

pub(crate) fn align_single_column_change(old: &Grid, new: &Grid) -> Option<ColumnAlignment> {
    if !is_within_size_bounds(old, new) {
        return None;
    }

    if old.nrows != new.nrows {
        return None;
    }

    let col_diff = new.ncols as i64 - old.ncols as i64;
    if col_diff.abs() != 1 {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    let stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);
    if has_heavy_repetition(&stats) {
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

fn is_within_size_bounds(old: &Grid, new: &Grid) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= MAX_ALIGN_ROWS && cols <= MAX_ALIGN_COLS
}

fn has_heavy_repetition(stats: &HashStats<ColHash>) -> bool {
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > MAX_HASH_REPEAT
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

        let alignment =
            align_single_column_change(&grid_a, &grid_b).expect("alignment should succeed");

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

        assert!(align_single_column_change(&grid_a, &grid_b).is_none());
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

        assert!(align_single_column_change(&grid_a, &grid_b).is_none());
    }
}
