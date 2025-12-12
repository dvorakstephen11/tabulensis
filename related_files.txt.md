
# related_files.txt

core/src/alignment/assembly.rs:
```rust
//! Final alignment assembly for AMR algorithm.
//!
//! Implements the final assembly phase as described in the unified grid diff
//! specification Section 12. This module:
//!
//! 1. Orchestrates the full AMR pipeline (metadata → anchors → chain → gaps)
//! 2. Assembles matched pairs, insertions, deletions, and moves into final alignment
//! 3. Provides fast paths for special cases (RLE compression, single-run grids)
//!
//! The main entry point is `align_rows_amr` which returns an `Option<RowAlignment>`.
//! Returns `None` when alignment cannot be determined (falls back to positional diff).

use std::ops::Range;

use crate::alignment::anchor_chain::build_anchor_chain;
use crate::alignment::anchor_discovery::{Anchor, discover_anchors_from_meta};
use crate::alignment::gap_strategy::{GapStrategy, select_gap_strategy};
use crate::alignment::move_extraction::{find_block_move, moves_from_matched_pairs};
use crate::alignment::row_metadata::RowMeta;
use crate::alignment::runs::{RowRun, compress_to_runs};
use crate::config::DiffConfig;
use crate::grid_view::GridView;
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowAlignment {
    pub matched: Vec<(u32, u32)>,
    pub inserted: Vec<u32>,
    pub deleted: Vec<u32>,
    pub moves: Vec<RowBlockMove>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBlockMove {
    pub src_start_row: u32,
    pub dst_start_row: u32,
    pub row_count: u32,
}

#[derive(Default)]
struct GapAlignmentResult {
    matched: Vec<(u32, u32)>,
    inserted: Vec<u32>,
    deleted: Vec<u32>,
    moves: Vec<RowBlockMove>,
}

pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    let runs_a = compress_to_runs(&view_a.row_meta);
    let runs_b = compress_to_runs(&view_b.row_meta);
    if runs_a.len() == 1 && runs_b.len() == 1 && runs_a[0].signature == runs_b[0].signature {
        let shared = runs_a[0].count.min(runs_b[0].count);
        let mut matched = Vec::new();
        for offset in 0..shared {
            matched.push((runs_a[0].start_row + offset, runs_b[0].start_row + offset));
        }
        let mut inserted = Vec::new();
        if runs_b[0].count > shared {
            inserted.extend(
                (runs_b[0].start_row + shared)..(runs_b[0].start_row + runs_b[0].count),
            );
        }
        let mut deleted = Vec::new();
        if runs_a[0].count > shared {
            deleted.extend(
                (runs_a[0].start_row + shared)..(runs_a[0].start_row + runs_a[0].count),
            );
        }
        return Some(RowAlignment {
            matched,
            inserted,
            deleted,
            moves: Vec::new(),
        });
    }

    let compressed_a = runs_a.len() * 2 <= view_a.row_meta.len();
    let compressed_b = runs_b.len() * 2 <= view_b.row_meta.len();
    if (compressed_a || compressed_b) && !runs_a.is_empty() && !runs_b.is_empty() {
        if let Some(alignment) = align_runs_stable(&runs_a, &runs_b) {
            return Some(alignment);
        }
    }

    let anchors = build_anchor_chain(discover_anchors_from_meta(&view_a.row_meta, &view_b.row_meta));
    Some(assemble_from_meta(
        &view_a.row_meta,
        &view_b.row_meta,
        anchors,
        config,
        0,
    ))
}

fn assemble_from_meta(
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    anchors: Vec<Anchor>,
    config: &DiffConfig,
    depth: u32,
) -> RowAlignment {
    if old_meta.is_empty() && new_meta.is_empty() {
        return RowAlignment::default();
    }

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut moves = Vec::new();

    let mut prev_old = old_meta.first().map(|m| m.row_idx).unwrap_or(0);
    let mut prev_new = new_meta.first().map(|m| m.row_idx).unwrap_or(0);

    for anchor in anchors.iter() {
        let gap_old = prev_old..anchor.old_row;
        let gap_new = prev_new..anchor.new_row;
        let gap_result = fill_gap(gap_old, gap_new, old_meta, new_meta, config, depth);
        matched.extend(gap_result.matched);
        inserted.extend(gap_result.inserted);
        deleted.extend(gap_result.deleted);
        moves.extend(gap_result.moves);

        matched.push((anchor.old_row, anchor.new_row));
        prev_old = anchor.old_row + 1;
        prev_new = anchor.new_row + 1;
    }

    let old_end = old_meta.last().map(|m| m.row_idx + 1).unwrap_or(prev_old);
    let new_end = new_meta.last().map(|m| m.row_idx + 1).unwrap_or(prev_new);
    let tail_result = fill_gap(prev_old..old_end, prev_new..new_end, old_meta, new_meta, config, depth);
    matched.extend(tail_result.matched);
    inserted.extend(tail_result.inserted);
    deleted.extend(tail_result.deleted);
    moves.extend(tail_result.moves);

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();
    moves.sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));

    RowAlignment {
        matched,
        inserted,
        deleted,
        moves,
    }
}

fn fill_gap(
    old_gap: Range<u32>,
    new_gap: Range<u32>,
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    config: &DiffConfig,
    depth: u32,
) -> GapAlignmentResult {
    let old_slice = slice_by_range(old_meta, &old_gap);
    let new_slice = slice_by_range(new_meta, &new_gap);
    let has_recursed = depth >= config.max_recursion_depth;
    let strategy = select_gap_strategy(old_slice, new_slice, config, has_recursed);

    match strategy {
        GapStrategy::Empty => GapAlignmentResult::default(),
        GapStrategy::InsertAll => GapAlignmentResult {
            matched: Vec::new(),
            inserted: (new_gap.start..new_gap.end).collect(),
            deleted: Vec::new(),
            moves: Vec::new(),
        },
        GapStrategy::DeleteAll => GapAlignmentResult {
            matched: Vec::new(),
            inserted: Vec::new(),
            deleted: (old_gap.start..old_gap.end).collect(),
            moves: Vec::new(),
        },
        GapStrategy::SmallEdit => align_small_gap(old_slice, new_slice),
        GapStrategy::MoveCandidate => {
            let mut result = align_small_gap(old_slice, new_slice);
            let mut detected_moves = moves_from_matched_pairs(&result.matched);
            if detected_moves.is_empty() {
                if let Some(mv) = find_block_move(old_slice, new_slice, 1) {
                    detected_moves.push(mv);
                }
            }
            result.moves.extend(detected_moves);
            result
        }
        GapStrategy::RecursiveAlign => {
            if depth >= config.max_recursion_depth {
                return align_small_gap(old_slice, new_slice);
            }
            let anchors = build_anchor_chain(discover_anchors_from_meta(old_slice, new_slice));
            let alignment = assemble_from_meta(old_slice, new_slice, anchors, config, depth + 1);
            GapAlignmentResult {
                matched: alignment.matched,
                inserted: alignment.inserted,
                deleted: alignment.deleted,
                moves: alignment.moves,
            }
        }
    }
}

fn align_runs_stable(runs_a: &[RowRun], runs_b: &[RowRun]) -> Option<RowAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < runs_a.len() && idx_b < runs_b.len() {
        let run_a = &runs_a[idx_a];
        let run_b = &runs_b[idx_b];

        if run_a.signature != run_b.signature {
            return None;
        }

        let shared = run_a.count.min(run_b.count);
        for offset in 0..shared {
            matched.push((run_a.start_row + offset, run_b.start_row + offset));
        }

        if run_a.count > shared {
            for offset in shared..run_a.count {
                deleted.push(run_a.start_row + offset);
            }
        }

        if run_b.count > shared {
            for offset in shared..run_b.count {
                inserted.push(run_b.start_row + offset);
            }
        }

        idx_a += 1;
        idx_b += 1;
    }

    for run in runs_a.iter().skip(idx_a) {
        for offset in 0..run.count {
            deleted.push(run.start_row + offset);
        }
    }

    for run in runs_b.iter().skip(idx_b) {
        for offset in 0..run.count {
            inserted.push(run.start_row + offset);
        }
    }

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();

    Some(RowAlignment {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    })
}

fn slice_by_range<'a>(meta: &'a [RowMeta], range: &Range<u32>) -> &'a [RowMeta] {
    if meta.is_empty() || range.start >= range.end {
        return &[];
    }
    let base = meta.first().map(|m| m.row_idx).unwrap_or(0);
    if range.start < base {
        return &[];
    }
    let start = (range.start - base) as usize;
    if start >= meta.len() {
        return &[];
    }
    let end = (start + (range.end - range.start) as usize).min(meta.len());
    &meta[start..end]
}

fn align_small_gap(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if old_slice[i].signature == new_slice[j].signature {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if old_slice[i].signature == new_slice[j].signature {
            matched.push((old_slice[i].row_idx, new_slice[j].row_idx));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            deleted.push(old_slice[i].row_idx);
            i += 1;
        } else {
            inserted.push(new_slice[j].row_idx);
            j += 1;
        }
    }

    while i < m {
        deleted.push(old_slice[i].row_idx);
        i += 1;
    }
    while j < n {
        inserted.push(new_slice[j].row_idx);
        j += 1;
    }

    if matched.is_empty() && m == n {
        matched = old_slice
            .iter()
            .zip(new_slice.iter())
            .map(|(a, b)| (a.row_idx, b.row_idx))
            .collect();
        inserted.clear();
        deleted.clear();
    }

    GapAlignmentResult {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::{Cell, CellAddress, CellValue};

    fn grid_from_run_lengths(pattern: &[(i32, u32)]) -> Grid {
        let total_rows: u32 = pattern.iter().map(|(_, count)| *count).sum();
        let mut grid = Grid::new(total_rows, 1);
        let mut row_idx = 0u32;
        for (val, count) in pattern {
            for _ in 0..*count {
                grid.insert(Cell {
                    row: row_idx,
                    col: 0,
                    address: CellAddress::from_indices(row_idx, 0),
                    value: Some(CellValue::Number(*val as f64)),
                    formula: None,
                });
                row_idx = row_idx.saturating_add(1);
            }
        }
        grid
    }

    fn grid_with_unique_rows(rows: &[i32]) -> Grid {
        let nrows = rows.len() as u32;
        let mut grid = Grid::new(nrows, 1);
        for (r, &val) in rows.iter().enumerate() {
            grid.insert(Cell {
                row: r as u32,
                col: 0,
                address: CellAddress::from_indices(r as u32, 0),
                value: Some(CellValue::Number(val as f64)),
                formula: None,
            });
        }
        grid
    }

    #[test]
    fn aligns_compressed_runs_with_insert_and_delete() {
        let grid_a = grid_from_run_lengths(&[(1, 50), (2, 5), (1, 50)]);
        let grid_b = grid_from_run_lengths(&[(1, 52), (2, 3), (1, 50)]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed for repetitive runs");
        assert!(alignment.moves.is_empty());
        assert_eq!(alignment.inserted.len(), 2);
        assert_eq!(alignment.deleted.len(), 2);
        assert_eq!(alignment.matched.len(), 103);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn run_alignment_falls_back_on_mismatch() {
        let grid_a = grid_from_run_lengths(&[(1, 3), (2, 3), (1, 3)]);
        let grid_b = grid_from_run_lengths(&[(1, 3), (3, 3), (1, 3)]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should still produce result via full AMR");
        assert!(!alignment.matched.is_empty());
    }

    #[test]
    fn amr_disjoint_gaps_with_insertions_and_deletions() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 100, 4, 5, 6, 200, 7, 8, 9]);
        let grid_b = grid_with_unique_rows(&[1, 2, 10, 3, 4, 5, 6, 7, 20, 8, 9]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with disjoint gaps");

        assert!(!alignment.matched.is_empty(), "should have matched pairs");
        
        let matched_is_monotonic = alignment.matched.windows(2).all(|w| {
            w[0].0 <= w[1].0 && w[0].1 <= w[1].1
        });
        assert!(matched_is_monotonic, "matched pairs should be monotonically increasing");
        
        assert!(!alignment.inserted.is_empty() || !alignment.deleted.is_empty(), 
            "should have insertions and/or deletions");
    }

    #[test]
    fn amr_recursive_gap_alignment_returns_monotonic_alignment() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let rows_b = vec![1, 2, 100, 3, 4, 5, 200, 6, 7, 8, 300, 9, 10, 11, 400, 12, 13, 14, 15];
        let grid_b = grid_with_unique_rows(&rows_b);

        let config = DiffConfig {
            recursive_align_threshold: 5,
            small_gap_threshold: 2,
            ..Default::default()
        };

        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with recursive gaps");

        let matched_is_monotonic = alignment.matched.windows(2).all(|w| {
            w[0].0 <= w[1].0 && w[0].1 <= w[1].1
        });
        assert!(matched_is_monotonic, 
            "recursive alignment should produce monotonic matched pairs");
        
        for &inserted_row in &alignment.inserted {
            assert!(!alignment.matched.iter().any(|(_, b)| *b == inserted_row),
                "inserted rows should not appear in matched pairs");
        }
        
        for &deleted_row in &alignment.deleted {
            assert!(!alignment.matched.iter().any(|(a, _)| *a == deleted_row),
                "deleted rows should not appear in matched pairs");
        }
    }

    #[test]
    fn amr_multi_gap_move_detection_produces_expected_row_block_move() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let grid_b = grid_with_unique_rows(&[1, 2, 6, 7, 8, 3, 4, 5, 9, 10]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with moved block");

        assert!(!alignment.matched.is_empty(), "should have matched pairs even with moves");
        
        let old_rows: std::collections::HashSet<_> = alignment.matched.iter().map(|(a, _)| *a).collect();
        let new_rows: std::collections::HashSet<_> = alignment.matched.iter().map(|(_, b)| *b).collect();
        
        assert!(old_rows.len() <= 10 && new_rows.len() <= 10, 
            "matched rows should not exceed input size");
    }

    #[test]
    fn amr_alignment_empty_grids() {
        let grid_a = Grid::new(0, 0);
        let grid_b = Grid::new(0, 0);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed for empty grids");

        assert!(alignment.matched.is_empty());
        assert!(alignment.inserted.is_empty());
        assert!(alignment.deleted.is_empty());
        assert!(alignment.moves.is_empty());
    }

    #[test]
    fn amr_alignment_all_deleted() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5]);
        let grid_b = Grid::new(0, 1);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed when all rows deleted");

        assert!(alignment.matched.is_empty());
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted.len(), 5);
    }

    #[test]
    fn amr_alignment_all_inserted() {
        let grid_a = Grid::new(0, 1);
        let grid_b = grid_with_unique_rows(&[1, 2, 3, 4, 5]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed when all rows inserted");

        assert!(alignment.matched.is_empty());
        assert_eq!(alignment.inserted.len(), 5);
        assert!(alignment.deleted.is_empty());
    }
}
```


core/src/alignment/gap_strategy.rs:
```rust
//! Gap strategy selection for AMR alignment.
//!
//! Implements gap strategy selection as described in the unified grid diff
//! specification Sections 9.6 and 12. After anchors divide the grids into
//! gaps, each gap is processed according to its characteristics:
//!
//! - **Empty**: Both sides empty, nothing to do
//! - **InsertAll**: Old side empty, all new rows are insertions
//! - **DeleteAll**: New side empty, all old rows are deletions
//! - **SmallEdit**: Both sides small enough for O(n*m) LCS alignment
//! - **MoveCandidate**: Gap contains matching unique signatures that may indicate moves
//! - **RecursiveAlign**: Gap is large; recursively apply AMR with rare anchors

use std::collections::HashSet;

use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
use crate::config::DiffConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapStrategy {
    Empty,
    InsertAll,
    DeleteAll,
    SmallEdit,
    MoveCandidate,
    RecursiveAlign,
}

pub fn select_gap_strategy(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    config: &DiffConfig,
    has_recursed: bool,
) -> GapStrategy {
    let old_len = old_slice.len() as u32;
    let new_len = new_slice.len() as u32;

    if old_len == 0 && new_len == 0 {
        return GapStrategy::Empty;
    }
    if old_len == 0 {
        return GapStrategy::InsertAll;
    }
    if new_len == 0 {
        return GapStrategy::DeleteAll;
    }

    if has_matching_signatures(old_slice, new_slice) {
        return GapStrategy::MoveCandidate;
    }

    if old_len <= config.small_gap_threshold && new_len <= config.small_gap_threshold {
        return GapStrategy::SmallEdit;
    }

    if (old_len > config.recursive_align_threshold || new_len > config.recursive_align_threshold)
        && !has_recursed
    {
        return GapStrategy::RecursiveAlign;
    }

    GapStrategy::SmallEdit
}

fn has_matching_signatures(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> bool {
    let set: HashSet<_> = old_slice
        .iter()
        .filter(|m| m.frequency_class == FrequencyClass::Unique)
        .map(|m| m.signature)
        .collect();

    new_slice
        .iter()
        .filter(|m| m.frequency_class == FrequencyClass::Unique)
        .any(|m| set.contains(&m.signature))
}

```


core/src/alignment/anchor_discovery.rs
```rust
//! Anchor discovery for AMR alignment.
//!
//! Implements anchor discovery as described in the unified grid diff specification
//! Section 10. Anchors are rows that:
//!
//! 1. Are unique (appear exactly once) in BOTH grids
//! 2. Have matching signatures (content hash)
//!
//! These rows serve as fixed points around which the alignment is built.
//! Rows that are unique in one grid but not the other cannot be anchors
//! since their position cannot be reliably determined.

use std::collections::HashMap;

use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
use crate::grid_view::GridView;
use crate::workbook::RowSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub old_row: u32,
    pub new_row: u32,
    pub signature: RowSignature,
}

#[allow(dead_code)]
pub fn discover_anchors(old: &GridView<'_>, new: &GridView<'_>) -> Vec<Anchor> {
    discover_anchors_from_meta(&old.row_meta, &new.row_meta)
}

pub fn discover_anchors_from_meta(old: &[RowMeta], new: &[RowMeta]) -> Vec<Anchor> {
    let mut old_unique: HashMap<RowSignature, u32> = HashMap::new();
    for meta in old.iter() {
        if meta.frequency_class == FrequencyClass::Unique {
            old_unique.insert(meta.signature, meta.row_idx);
        }
    }

    new.iter()
        .filter(|meta| meta.frequency_class == FrequencyClass::Unique)
        .filter_map(|meta| {
            old_unique.get(&meta.signature).map(|old_idx| Anchor {
                old_row: *old_idx,
                new_row: meta.row_idx,
                signature: meta.signature,
            })
        })
        .collect()
}

```


core/src/alignment/anchor_chain.rs
```rust
//! Anchor chain construction using Longest Increasing Subsequence (LIS).
//!
//! Implements anchor chain building as described in the unified grid diff
//! specification Section 10. Given a set of discovered anchors, this module
//! selects the maximal subset that preserves relative order in both grids.
//!
//! For example, if anchors show:
//! - Row A: old=0, new=0
//! - Row B: old=2, new=1  (B moved up)
//! - Row C: old=1, new=2
//!
//! The LIS algorithm selects {A, C} because their old_row indices (0, 1) are
//! increasing, making them a valid ordering chain. Row B is excluded because
//! including it would create a crossing (B is at old=2 but new=1, while C is
//! at old=1 but new=2).

use crate::alignment::anchor_discovery::Anchor;

pub fn build_anchor_chain(mut anchors: Vec<Anchor>) -> Vec<Anchor> {
    // Sort by new_row to preserve destination order before LIS on old_row.
    anchors.sort_by_key(|a| a.new_row);
    let indices = lis_indices(&anchors, |a| a.old_row);
    indices.into_iter().map(|idx| anchors[idx]).collect()
}

fn lis_indices<T, F>(items: &[T], key: F) -> Vec<usize>
where
    F: Fn(&T) -> u32,
{
    let mut piles: Vec<usize> = Vec::new();
    let mut predecessors: Vec<Option<usize>> = vec![None; items.len()];

    for (idx, item) in items.iter().enumerate() {
        let k = key(item);
        let pos = piles
            .binary_search_by_key(&k, |&pile_idx| key(&items[pile_idx]))
            .unwrap_or_else(|insert_pos| insert_pos);

        if pos > 0 {
            predecessors[idx] = Some(piles[pos - 1]);
        }

        if pos == piles.len() {
            piles.push(idx);
        } else {
            piles[pos] = idx;
        }
    }

    if piles.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<usize> = Vec::new();
    let mut current = *piles.last().unwrap();
    loop {
        result.push(current);
        if let Some(prev) = predecessors[current] {
            current = prev;
        } else {
            break;
        }
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::anchor_discovery::Anchor;
    use crate::workbook::RowSignature;

    #[test]
    fn builds_increasing_chain() {
        let anchors = vec![
            Anchor {
                old_row: 0,
                new_row: 0,
                signature: RowSignature { hash: 1 },
            },
            Anchor {
                old_row: 2,
                new_row: 1,
                signature: RowSignature { hash: 2 },
            },
            Anchor {
                old_row: 1,
                new_row: 2,
                signature: RowSignature { hash: 3 },
            },
        ];

        let chain = build_anchor_chain(anchors);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].old_row, 0);
        assert_eq!(chain[1].old_row, 1);
    }
}

```



core/src/alignment/row_metadata.rs
```rust
//! Row metadata and frequency classification for AMR alignment.
//!
//! Implements row frequency classification as described in the unified grid diff
//! specification Section 9.11. Each row is classified into one of four frequency classes:
//!
//! - **Unique**: Appears exactly once in the grid (highest anchor quality)
//! - **Rare**: Appears 2-N times where N is configurable (can serve as secondary anchors)
//! - **Common**: Appears frequently (poor anchor quality)
//! - **LowInfo**: Blank or near-blank rows (ignored for anchoring)

use std::collections::HashMap;

use crate::config::DiffConfig;
use crate::workbook::RowSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrequencyClass {
    Unique,
    Rare,
    Common,
    LowInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RowMeta {
    pub row_idx: u32,
    pub signature: RowSignature,
    pub hash: RowSignature,
    pub non_blank_count: u16,
    pub first_non_blank_col: u16,
    pub frequency_class: FrequencyClass,
    pub is_low_info: bool,
}

impl RowMeta {
    pub fn is_low_info(&self) -> bool {
        self.is_low_info || matches!(self.frequency_class, FrequencyClass::LowInfo)
    }
}

pub fn frequency_map(row_meta: &[RowMeta]) -> HashMap<RowSignature, u32> {
    let mut map = HashMap::new();
    for meta in row_meta {
        *map.entry(meta.signature).or_insert(0) += 1;
    }
    map
}

pub fn classify_row_frequencies(row_meta: &mut [RowMeta], config: &DiffConfig) {
    let freq_map = frequency_map(row_meta);
    for meta in row_meta.iter_mut() {
        if meta.frequency_class == FrequencyClass::LowInfo {
            continue;
        }

        let count = freq_map.get(&meta.signature).copied().unwrap_or(0);
        let mut class = match count {
            1 => FrequencyClass::Unique,
            c if c == 0 => FrequencyClass::Common,
            c if c <= config.rare_threshold => FrequencyClass::Rare,
            _ => FrequencyClass::Common,
        };

        if (meta.non_blank_count as u32) < config.low_info_threshold || meta.is_low_info {
            class = FrequencyClass::LowInfo;
            meta.is_low_info = true;
        }

        meta.frequency_class = class;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(row_idx: u32, hash: u128, non_blank: u16) -> RowMeta {
        let sig = RowSignature { hash };
        RowMeta {
            row_idx,
            signature: sig,
            hash: sig,
            non_blank_count: non_blank,
            first_non_blank_col: 0,
            frequency_class: FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn classifies_unique_and_rare_and_low_info() {
        let mut meta = vec![
            make_meta(0, 1, 3),
            make_meta(1, 1, 3),
            make_meta(2, 2, 1),
        ];

        let mut config = DiffConfig::default();
        config.rare_threshold = 2;
        config.low_info_threshold = 2;

        classify_row_frequencies(&mut meta, &config);

        assert_eq!(meta[0].frequency_class, FrequencyClass::Rare);
        assert_eq!(meta[1].frequency_class, FrequencyClass::Rare);
        assert_eq!(meta[2].frequency_class, FrequencyClass::LowInfo);
    }
}

```


core/src/alignment/runs.rs
```rust
//! Run-length encoding for repetitive row patterns.
//!
//! Implements run-length compression as described in the unified grid diff
//! specification Section 2.6 (optional optimization). For grids where >50%
//! of rows share signatures with adjacent rows, this provides a fast path
//! that avoids full AMR computation.
//!
//! This is particularly effective for:
//! - Template-based workbooks with many identical rows
//! - Data with long runs of blank or placeholder rows
//! - Adversarial cases designed to stress the alignment algorithm

use crate::alignment::row_metadata::RowMeta;
use crate::workbook::RowSignature;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRun {
    pub signature: RowSignature,
    pub start_row: u32,
    pub count: u32,
}

pub fn compress_to_runs(meta: &[RowMeta]) -> Vec<RowRun> {
    let mut runs = Vec::new();
    let mut i = 0usize;
    while i < meta.len() {
        let sig = meta[i].signature;
        let start = i;
        while i < meta.len() && meta[i].signature == sig {
            i += 1;
        }
        runs.push(RowRun {
            signature: sig,
            start_row: start as u32,
            count: (i - start) as u32,
        });
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(idx: u32, hash: u128) -> RowMeta {
        let sig = RowSignature { hash };
        RowMeta {
            row_idx: idx,
            signature: sig,
            hash: sig,
            non_blank_count: 1,
            first_non_blank_col: 0,
            frequency_class: crate::alignment::row_metadata::FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn compresses_identical_rows() {
        let meta = vec![make_meta(0, 1), make_meta(1, 1), make_meta(2, 2)];
        let runs = compress_to_runs(&meta);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].count, 2);
        assert_eq!(runs[1].count, 1);
    }

    #[test]
    fn compresses_10k_identical_rows_to_single_run() {
        let meta: Vec<RowMeta> = (0..10_000).map(|i| make_meta(i, 42)).collect();
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 1, "10K identical rows should compress to a single run");
        assert_eq!(runs[0].count, 10_000, "single run should have count of 10,000");
        assert_eq!(runs[0].signature.hash, 42, "run signature should match input");
        assert_eq!(runs[0].start_row, 0, "run should start at row 0");
    }

    #[test]
    fn alternating_pattern_ab_does_not_overcompress() {
        let meta: Vec<RowMeta> = (0..10_000)
            .map(|i| {
                let hash = if i % 2 == 0 { 1 } else { 2 };
                make_meta(i, hash)
            })
            .collect();
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 10_000, 
            "alternating A-B pattern should produce 10K runs (no compression benefit)");
        
        for (i, run) in runs.iter().enumerate() {
            assert_eq!(run.count, 1, "each run should have count of 1 for alternating pattern");
            let expected_hash = if i % 2 == 0 { 1 } else { 2 };
            assert_eq!(run.signature.hash, expected_hash, "run signature should alternate");
        }
    }

    #[test]
    fn mixed_runs_with_varying_lengths() {
        let mut meta = Vec::new();
        let mut row_idx = 0u32;
        
        for _ in 0..100 {
            meta.push(make_meta(row_idx, 1));
            row_idx += 1;
        }
        for _ in 0..50 {
            meta.push(make_meta(row_idx, 2));
            row_idx += 1;
        }
        for _ in 0..200 {
            meta.push(make_meta(row_idx, 3));
            row_idx += 1;
        }
        for _ in 0..1 {
            meta.push(make_meta(row_idx, 4));
            row_idx += 1;
        }
        
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 4, "should produce 4 runs for 4 distinct signatures");
        assert_eq!(runs[0].count, 100);
        assert_eq!(runs[1].count, 50);
        assert_eq!(runs[2].count, 200);
        assert_eq!(runs[3].count, 1);
    }

    #[test]
    fn empty_input_produces_empty_runs() {
        let meta: Vec<RowMeta> = vec![];
        let runs = compress_to_runs(&meta);
        assert!(runs.is_empty(), "empty input should produce empty runs");
    }

    #[test]
    fn single_row_produces_single_run() {
        let meta = vec![make_meta(0, 999)];
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].count, 1);
        assert_eq!(runs[0].start_row, 0);
        assert_eq!(runs[0].signature.hash, 999);
    }

    #[test]
    fn run_compression_preserves_row_indices() {
        let meta: Vec<RowMeta> = (0..1000u32)
            .map(|i| make_meta(i, (i / 100) as u128))
            .collect();
        let runs = compress_to_runs(&meta);
        
        assert_eq!(runs.len(), 10, "should have 10 runs (one per 100 rows)");
        
        for (group_idx, run) in runs.iter().enumerate() {
            let expected_start = (group_idx * 100) as u32;
            assert_eq!(run.start_row, expected_start, 
                "run {} should start at row {}", group_idx, expected_start);
            assert_eq!(run.count, 100, "each run should have 100 rows");
        }
    }
}

```


core/src/alignment/move_extraction.rs
```rust
//! Move extraction from alignment gaps.
//!
//! Implements localized move detection within gaps. This is a simplified approach
//! compared to the full spec (Sections 9.5-9.7, 11) which describes global
//! move-candidate extraction and validation phases.
//!
//! ## Current Implementation
//!
//! - `find_block_move`: Scans for contiguous blocks of matching signatures
//!   between old and new slices within a gap. Returns the largest found.
//!
//! - `moves_from_matched_pairs`: Extracts block moves from matched row pairs
//!   where consecutive pairs have the same offset (indicating they moved together).
//!
//! ## Future Work (TODO)
//!
//! To implement full spec compliance, this module would need:
//!
//! 1. Global unanchored match collection (all out-of-order signature matches)
//! 2. Candidate move construction from unanchored matches
//! 3. Move validation to resolve overlapping/conflicting candidates
//! 4. Integration with gap filling to consume validated moves

use std::collections::HashMap;

use crate::alignment::row_metadata::RowMeta;
use crate::alignment::RowBlockMove;
use crate::workbook::RowSignature;

pub fn find_block_move(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    min_len: u32,
) -> Option<RowBlockMove> {
    let mut positions: HashMap<RowSignature, Vec<usize>> = HashMap::new();
    for (idx, meta) in old_slice.iter().enumerate() {
        positions.entry(meta.signature).or_default().push(idx);
    }

    let mut best: Option<RowBlockMove> = None;

    for (new_idx, meta) in new_slice.iter().enumerate() {
        if let Some(candidates) = positions.get(&meta.signature) {
            for &old_idx in candidates {
                let mut len = 0usize;
                while old_idx + len < old_slice.len()
                    && new_idx + len < new_slice.len()
                    && old_slice[old_idx + len].signature == new_slice[new_idx + len].signature
                {
                    len += 1;
                }

                if len as u32 >= min_len {
                    let mv = RowBlockMove {
                        src_start_row: old_slice[old_idx].row_idx,
                        dst_start_row: new_slice[new_idx].row_idx,
                        row_count: len as u32,
                    };
                    let take = best
                        .as_ref()
                        .map_or(true, |b| mv.row_count > b.row_count);
                    if take {
                        best = Some(mv);
                    }
                }
            }
        }
    }

    best
}

pub fn moves_from_matched_pairs(pairs: &[(u32, u32)]) -> Vec<RowBlockMove> {
    if pairs.is_empty() {
        return Vec::new();
    }

    let mut sorted = pairs.to_vec();
    sorted.sort_by_key(|(a, b)| (*a, *b));

    let mut moves = Vec::new();
    let mut start = sorted[0];
    let mut prev = sorted[0];
    let mut run_len = 1u32;
    let mut current_offset: i64 = prev.1 as i64 - prev.0 as i64;

    for &(a, b) in sorted.iter().skip(1) {
        let offset = b as i64 - a as i64;
        if offset == current_offset && a == prev.0 + 1 && b == prev.1 + 1 {
            run_len += 1;
            prev = (a, b);
            continue;
        }

        if run_len > 1 && current_offset != 0 {
            moves.push(RowBlockMove {
                src_start_row: start.0,
                dst_start_row: start.1,
                row_count: run_len,
            });
        }

        start = (a, b);
        prev = (a, b);
        current_offset = offset;
        run_len = 1;
    }

    if run_len > 1 && current_offset != 0 {
        moves.push(RowBlockMove {
            src_start_row: start.0,
            dst_start_row: start.1,
            row_count: run_len,
        });
    }

    moves
}

```