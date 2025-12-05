use crate::grid_view::{GridView, HashStats, RowHash};
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowAlignment {
    pub matched: Vec<(u32, u32)>, // (row_idx_a, row_idx_b)
    pub inserted: Vec<u32>,       // row indices in B
    pub deleted: Vec<u32>,        // row indices in A
}

const MAX_ALIGN_ROWS: u32 = 2_000;
const MAX_ALIGN_COLS: u32 = 64;
const MAX_HASH_REPEAT: u32 = 8;
const _HASH_COLLISION_NOTE: &str = "64-bit hash collision probability ~0.00006% at 2K rows; \
                                    secondary verification deferred to G8a (50K-row adversarial)";

pub(crate) fn align_single_row_change(old: &Grid, new: &Grid) -> Option<RowAlignment> {
    if !is_within_size_bounds(old, new) {
        return None;
    }

    if old.ncols != new.ncols {
        return None;
    }

    let row_diff = new.nrows as i64 - old.nrows as i64;
    if row_diff.abs() != 1 {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats) {
        return None;
    }

    if row_diff == 1 {
        find_single_gap_alignment(
            &view_a.row_meta,
            &view_b.row_meta,
            &stats,
            RowChange::Insert,
        )
    } else {
        find_single_gap_alignment(
            &view_a.row_meta,
            &view_b.row_meta,
            &stats,
            RowChange::Delete,
        )
    }
}

enum RowChange {
    Insert,
    Delete,
}

fn find_single_gap_alignment(
    rows_a: &[crate::grid_view::RowMeta],
    rows_b: &[crate::grid_view::RowMeta],
    stats: &HashStats<RowHash>,
    change: RowChange,
) -> Option<RowAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = false;

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < rows_a.len() && idx_b < rows_b.len() {
        let meta_a = rows_a[idx_a];
        let meta_b = rows_b[idx_b];

        if meta_a.hash == meta_b.hash {
            matched.push((meta_a.row_idx, meta_b.row_idx));
            idx_a += 1;
            idx_b += 1;
            continue;
        }

        if skipped {
            return None;
        }

        match change {
            RowChange::Insert => {
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
                idx_b += 1;
            }
            RowChange::Delete => {
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
                idx_a += 1;
            }
        }

        skipped = true;
    }

    if idx_a < rows_a.len() || idx_b < rows_b.len() {
        if skipped {
            return None;
        }

        match change {
            RowChange::Insert if idx_a == rows_a.len() && rows_b.len() == idx_b + 1 => {
                let meta_b = rows_b[idx_b];
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
            }
            RowChange::Delete if idx_b == rows_b.len() && rows_a.len() == idx_a + 1 => {
                let meta_a = rows_a[idx_a];
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
            }
            _ => return None,
        }
    }

    if inserted.len() + deleted.len() != 1 {
        return None;
    }

    let alignment = RowAlignment {
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

fn is_unique_to_b(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 0
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
}

fn is_unique_to_a(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 0
}

fn is_within_size_bounds(old: &Grid, new: &Grid) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= MAX_ALIGN_ROWS && cols <= MAX_ALIGN_COLS
}

fn low_info_dominated(view: &GridView<'_>) -> bool {
    if view.row_meta.is_empty() {
        return false;
    }

    let low_info_count = view.row_meta.iter().filter(|m| m.is_low_info).count();
    low_info_count * 2 > view.row_meta.len()
}

fn has_heavy_repetition(stats: &HashStats<RowHash>) -> bool {
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

    fn grid_from_rows(rows: &[&[i32]]) -> Grid {
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
    fn aligns_single_insert_with_unique_row() {
        let base = (1..=10)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base_refs.clone();
        rows_b.insert(
            5,
            &[999, 1000, 1001], // inserted at position 6 (1-based)
        );
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 10);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
        assert_eq!(alignment.matched[5], (5, 6));
        assert_eq!(alignment.matched.last(), Some(&(9, 10)));
    }

    #[test]
    fn rejects_non_monotonic_alignment_with_extra_mismatch() {
        let base_rows = [
            [11, 12, 13],
            [21, 22, 23],
            [31, 32, 33],
            [41, 42, 43],
        ];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = vec![
            base_refs[0],          // same
            &[999, 1000, 1001],    // inserted unique row
            base_refs[2],          // move row 3 before row 2 to break monotonicity
            base_refs[1],
            base_refs[3],
        ];
        let grid_b = grid_from_rows(&rows_b);

        assert!(align_single_row_change(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn aligns_insert_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_first_row = [999, 1000, 1001];
        let mut rows_b = vec![new_first_row.as_slice()];
        rows_b.extend(base_refs.iter().copied());
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![0]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 1));
        assert_eq!(alignment.matched[4], (4, 5));
    }

    #[test]
    fn aligns_insert_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_last_row = [999, 1000, 1001];
        let mut rows_b: Vec<&[i32]> = base_refs.clone();
        rows_b.push(new_last_row.as_slice());
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
    }

    #[test]
    fn aligns_delete_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[1..].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![0]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (1, 0));
        assert_eq!(alignment.matched[3], (4, 3));
    }

    #[test]
    fn aligns_delete_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[..4].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![4]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[3], (3, 3));
    }

    #[test]
    fn aligns_single_row_to_two_rows_via_insert() {
        let single_row = [[42, 43, 44]];
        let single_refs: Vec<&[i32]> = single_row.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&single_refs);

        let new_row = [999, 1000, 1001];
        let rows_b: Vec<&[i32]> = vec![single_refs[0], new_row.as_slice()];
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![1]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn aligns_two_rows_to_single_row_via_delete() {
        let two_rows = [[42, 43, 44], [99, 100, 101]];
        let two_refs: Vec<&[i32]> = two_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&two_refs);

        let single_refs: Vec<&[i32]> = vec![two_refs[0]];
        let grid_b = grid_from_rows(&single_refs);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![1]);
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn monotonicity_helper_accepts_valid_sequence() {
        let valid: Vec<(u32, u32)> = vec![(0, 0), (1, 2), (3, 4), (5, 7)];
        assert!(super::is_monotonic(&valid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_a() {
        let invalid: Vec<(u32, u32)> = vec![(0, 0), (2, 2), (1, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_b() {
        let invalid: Vec<(u32, u32)> = vec![(0, 3), (1, 2), (2, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_accepts_empty_and_single() {
        assert!(super::is_monotonic(&[]));
        assert!(super::is_monotonic(&[(5, 10)]));
    }
}
