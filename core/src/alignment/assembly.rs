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
use crate::alignment::anchor_discovery::{
    Anchor, discover_anchors_from_meta, discover_context_anchors, discover_local_anchors,
};
use crate::alignment::gap_strategy::{GapStrategy, select_gap_strategy};
use crate::alignment::move_extraction::{find_block_move, moves_from_matched_pairs};
use crate::alignment::row_metadata::RowMeta;
use crate::alignment::runs::{RowRun, compress_to_runs};
use crate::config::DiffConfig;
use crate::grid_view::GridView;
use crate::workbook::{Grid, RowSignature};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowAlignmentWithSignatures {
    pub alignment: RowAlignment,
    pub row_signatures_a: Vec<RowSignature>,
    pub row_signatures_b: Vec<RowSignature>,
}

#[allow(dead_code)]
pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
    align_rows_amr_with_signatures(old, new, config).map(|result| result.alignment)
}

pub fn align_rows_amr_with_signatures(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignmentWithSignatures> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    let alignment = align_rows_from_meta(&view_a.row_meta, &view_b.row_meta, config)?;
    let row_signatures_a: Vec<RowSignature> =
        view_a.row_meta.iter().map(|meta| meta.signature).collect();
    let row_signatures_b: Vec<RowSignature> =
        view_b.row_meta.iter().map(|meta| meta.signature).collect();

    Some(RowAlignmentWithSignatures {
        alignment,
        row_signatures_a,
        row_signatures_b,
    })
}

fn align_rows_from_meta(
    rows_a: &[RowMeta],
    rows_b: &[RowMeta],
    config: &DiffConfig,
) -> Option<RowAlignment> {
    if rows_a.len() == rows_b.len()
        && rows_a
            .iter()
            .zip(rows_b.iter())
            .all(|(a, b)| a.signature == b.signature)
    {
        let mut matched = Vec::with_capacity(rows_a.len());
        for (a, b) in rows_a.iter().zip(rows_b.iter()) {
            matched.push((a.row_idx, b.row_idx));
        }
        return Some(RowAlignment {
            matched,
            inserted: Vec::new(),
            deleted: Vec::new(),
            moves: Vec::new(),
        });
    }

    let runs_a = compress_to_runs(rows_a);
    let runs_b = compress_to_runs(rows_b);
    if runs_a.len() == 1 && runs_b.len() == 1 && runs_a[0].signature == runs_b[0].signature {
        let shared = runs_a[0].count.min(runs_b[0].count);
        let mut matched = Vec::new();
        for offset in 0..shared {
            matched.push((runs_a[0].start_row + offset, runs_b[0].start_row + offset));
        }
        let mut inserted = Vec::new();
        if runs_b[0].count > shared {
            inserted
                .extend((runs_b[0].start_row + shared)..(runs_b[0].start_row + runs_b[0].count));
        }
        let mut deleted = Vec::new();
        if runs_a[0].count > shared {
            deleted.extend((runs_a[0].start_row + shared)..(runs_a[0].start_row + runs_a[0].count));
        }
        return Some(RowAlignment {
            matched,
            inserted,
            deleted,
            moves: Vec::new(),
        });
    }

    let compressed_a = runs_a.len() * 2 <= rows_a.len();
    let compressed_b = runs_b.len() * 2 <= rows_b.len();
    if (compressed_a || compressed_b)
        && !runs_a.is_empty()
        && !runs_b.is_empty()
        && let Some(alignment) = align_runs_stable(&runs_a, &runs_b)
    {
        return Some(alignment);
    }

    let anchors = build_anchor_chain(discover_anchors_from_meta(rows_a, rows_b));
    Some(assemble_from_meta(rows_a, rows_b, anchors, config, 0))
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
    let tail_result = fill_gap(
        prev_old..old_end,
        prev_new..new_end,
        old_meta,
        new_meta,
        config,
        depth,
    );
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

        GapStrategy::HashFallback => {
            let mut result = align_gap_via_hash(old_slice, new_slice);
            result
                .moves
                .extend(moves_from_matched_pairs(&result.matched));
            result
        }

        GapStrategy::MoveCandidate => {
            let mut result = if old_slice.len() as u32
                > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
                || new_slice.len() as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
            {
                align_gap_via_hash(old_slice, new_slice)
            } else {
                align_small_gap(old_slice, new_slice)
            };

            let mut detected_moves = moves_from_matched_pairs(&result.matched);

            if detected_moves.is_empty() {
                let has_nonzero_offset = result
                    .matched
                    .iter()
                    .any(|(a, b)| (*b as i64 - *a as i64) != 0);

                if has_nonzero_offset && let Some(mv) = find_block_move(old_slice, new_slice, 1) {
                    detected_moves.push(mv);
                }
            }

            result.moves.extend(detected_moves);
            result
        }

        GapStrategy::RecursiveAlign => {
            let at_limit = depth >= config.max_recursion_depth;
            if at_limit {
                if old_slice.len() as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
                    || new_slice.len() as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
                {
                    return align_gap_via_hash(old_slice, new_slice);
                }
                return align_small_gap(old_slice, new_slice);
            }

            let anchor_candidates = if depth == 0 {
                discover_anchors_from_meta(old_slice, new_slice)
            } else {
                let mut anchors = discover_local_anchors(old_slice, new_slice);
                if anchors.is_empty() {
                    anchors = discover_context_anchors(old_slice, new_slice, 4);
                    if anchors.is_empty() {
                        anchors = discover_context_anchors(old_slice, new_slice, 8);
                    }
                } else {
                    let mut ctx_anchors = discover_context_anchors(old_slice, new_slice, 4);
                    if anchors.len() < 4 {
                        anchors.append(&mut ctx_anchors);
                    }
                }
                anchors
            };

            let anchors = build_anchor_chain(anchor_candidates);
            if anchors.is_empty() {
                if old_slice.len() as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
                    || new_slice.len() as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
                {
                    return align_gap_via_hash(old_slice, new_slice);
                }
                return align_small_gap(old_slice, new_slice);
            }

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

    if m as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
        || n as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
    {
        return align_gap_via_hash(old_slice, new_slice);
    }

    const LCS_DP_WORK_LIMIT: usize = 20_000;
    if m.saturating_mul(n) > LCS_DP_WORK_LIMIT {
        return align_gap_via_myers(old_slice, new_slice);
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

fn align_gap_via_myers(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let edits = myers_edit_script(old_slice, new_slice);

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    for edit in edits {
        match edit {
            Edit::Match(i, j) => matched.push((old_slice[i].row_idx, new_slice[j].row_idx)),
            Edit::Insert(j) => inserted.push(new_slice[j].row_idx),
            Edit::Delete(i) => deleted.push(old_slice[i].row_idx),
        }
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

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();

    GapAlignmentResult {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edit {
    Match(usize, usize),
    Insert(usize),
    Delete(usize),
}

fn myers_edit_script(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> Vec<Edit> {
    let n = old_slice.len() as isize;
    let m = new_slice.len() as isize;
    if n == 0 {
        return (0..m as usize).map(Edit::Insert).collect();
    }
    if m == 0 {
        return (0..n as usize).map(Edit::Delete).collect();
    }

    let max = (n + m) as usize;
    let offset = max as isize;
    let mut v = vec![0isize; 2 * max + 1];
    let mut trace: Vec<Vec<isize>> = Vec::new();

    for d in 0..=max {
        let mut v_next = v.clone();
        for k in (-(d as isize)..=d as isize).step_by(2) {
            let idx = (k + offset) as usize;
            let x_start = if k == -(d as isize) || (k != d as isize && v[idx - 1] < v[idx + 1]) {
                v[idx + 1]
            } else {
                v[idx - 1] + 1
            };

            let mut x = x_start;
            let mut y = x - k;
            while x < n
                && y < m
                && old_slice[x as usize].signature == new_slice[y as usize].signature
            {
                x += 1;
                y += 1;
            }
            v_next[idx] = x;
            if x >= n && y >= m {
                trace.push(v_next);
                return reconstruct_myers(trace, old_slice.len(), new_slice.len(), offset);
            }
        }
        trace.push(v_next.clone());
        v = v_next;
    }

    Vec::new()
}

fn reconstruct_myers(
    trace: Vec<Vec<isize>>,
    old_len: usize,
    new_len: usize,
    offset: isize,
) -> Vec<Edit> {
    let mut edits = Vec::new();
    let mut x = old_len as isize;
    let mut y = new_len as isize;

    for d_rev in (0..trace.len()).rev() {
        let v = &trace[d_rev];
        let k = x - y;
        let idx = (k + offset) as usize;

        let (prev_x, prev_y, from_down);
        if d_rev == 0 {
            prev_x = 0;
            prev_y = 0;
            from_down = false;
        } else {
            let use_down =
                k == -(d_rev as isize) || (k != d_rev as isize && v[idx - 1] < v[idx + 1]);
            let prev_k = if use_down { k + 1 } else { k - 1 };
            let prev_idx = (prev_k + offset) as usize;
            let prev_v = &trace[d_rev - 1];
            prev_x = prev_v[prev_idx].max(0);
            prev_y = (prev_x - prev_k).max(0);
            from_down = use_down;
        }

        let mut cur_x = x;
        let mut cur_y = y;
        while cur_x > prev_x && cur_y > prev_y {
            cur_x -= 1;
            cur_y -= 1;
            edits.push(Edit::Match(cur_x as usize, cur_y as usize));
        }

        if d_rev > 0 {
            if from_down {
                edits.push(Edit::Insert(prev_y as usize));
            } else {
                edits.push(Edit::Delete(prev_x as usize));
            }
        }

        x = prev_x;
        y = prev_y;
    }

    edits.reverse();
    edits
}

fn align_gap_via_hash(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    use std::collections::{HashMap, VecDeque};

    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let mut sig_to_new: HashMap<crate::workbook::RowSignature, VecDeque<u32>> = HashMap::new();
    for (j, meta) in new_slice.iter().enumerate() {
        sig_to_new
            .entry(meta.signature)
            .or_default()
            .push_back(j as u32);
    }

    let mut candidate_pairs: Vec<(u32, u32)> = Vec::new();
    for (i, meta) in old_slice.iter().enumerate() {
        if let Some(q) = sig_to_new.get_mut(&meta.signature)
            && let Some(j) = q.pop_front()
        {
            candidate_pairs.push((i as u32, j));
        }
    }

    if candidate_pairs.is_empty() && m == n {
        let matched = old_slice
            .iter()
            .zip(new_slice.iter())
            .map(|(a, b)| (a.row_idx, b.row_idx))
            .collect();

        return GapAlignmentResult {
            matched,
            inserted: Vec::new(),
            deleted: Vec::new(),
            moves: Vec::new(),
        };
    }

    let lis = lis_indices_u32(&candidate_pairs, |&(_, new_j)| new_j);

    let mut keep = vec![false; candidate_pairs.len()];
    for idx in lis {
        keep[idx] = true;
    }

    let mut used_old = vec![false; m];
    let mut used_new = vec![false; n];
    let mut matched: Vec<(u32, u32)> = Vec::new();

    for (k, (old_i, new_j)) in candidate_pairs.iter().copied().enumerate() {
        if keep[k] {
            used_old[old_i as usize] = true;
            used_new[new_j as usize] = true;
            matched.push((
                old_slice[old_i as usize].row_idx,
                new_slice[new_j as usize].row_idx,
            ));
        }
    }

    let mut deleted: Vec<u32> = Vec::new();
    for i in 0..m {
        if !used_old[i] {
            deleted.push(old_slice[i].row_idx);
        }
    }

    let mut inserted: Vec<u32> = Vec::new();
    for j in 0..n {
        if !used_new[j] {
            inserted.push(new_slice[j].row_idx);
        }
    }

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();

    GapAlignmentResult {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    }
}

fn lis_indices_u32<T, F>(items: &[T], key: F) -> Vec<usize>
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
    use crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE;
    use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
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

    fn row_meta_from_hashes(start_row: u32, hashes: &[u128]) -> Vec<RowMeta> {
        hashes
            .iter()
            .enumerate()
            .map(|(idx, &hash)| {
                let signature = crate::workbook::RowSignature { hash };
                RowMeta {
                    row_idx: start_row + idx as u32,
                    signature,
                    hash: signature,
                    non_blank_count: 1,
                    first_non_blank_col: 0,
                    frequency_class: FrequencyClass::Common,
                    is_low_info: false,
                }
            })
            .collect()
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

        let matched_is_monotonic = alignment
            .matched
            .windows(2)
            .all(|w| w[0].0 <= w[1].0 && w[0].1 <= w[1].1);
        assert!(
            matched_is_monotonic,
            "matched pairs should be monotonically increasing"
        );

        assert!(
            !alignment.inserted.is_empty() || !alignment.deleted.is_empty(),
            "should have insertions and/or deletions"
        );
    }

    #[test]
    fn amr_recursive_gap_alignment_returns_monotonic_alignment() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let rows_b = vec![
            1, 2, 100, 3, 4, 5, 200, 6, 7, 8, 300, 9, 10, 11, 400, 12, 13, 14, 15,
        ];
        let grid_b = grid_with_unique_rows(&rows_b);

        let config = DiffConfig {
            recursive_align_threshold: 5,
            small_gap_threshold: 2,
            ..Default::default()
        };

        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with recursive gaps");

        let matched_is_monotonic = alignment
            .matched
            .windows(2)
            .all(|w| w[0].0 <= w[1].0 && w[0].1 <= w[1].1);
        assert!(
            matched_is_monotonic,
            "recursive alignment should produce monotonic matched pairs"
        );

        for &inserted_row in &alignment.inserted {
            assert!(
                !alignment.matched.iter().any(|(_, b)| *b == inserted_row),
                "inserted rows should not appear in matched pairs"
            );
        }

        for &deleted_row in &alignment.deleted {
            assert!(
                !alignment.matched.iter().any(|(a, _)| *a == deleted_row),
                "deleted rows should not appear in matched pairs"
            );
        }
    }

    #[test]
    fn amr_multi_gap_move_detection_produces_expected_row_block_move() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let grid_b = grid_with_unique_rows(&[1, 2, 6, 7, 8, 3, 4, 5, 9, 10]);

        let config = DiffConfig::default();
        let alignment = align_rows_amr(&grid_a, &grid_b, &config)
            .expect("alignment should succeed with moved block");

        assert!(
            !alignment.matched.is_empty(),
            "should have matched pairs even with moves"
        );

        let old_rows: std::collections::HashSet<_> =
            alignment.matched.iter().map(|(a, _)| *a).collect();
        let new_rows: std::collections::HashSet<_> =
            alignment.matched.iter().map(|(_, b)| *b).collect();

        assert!(
            old_rows.len() <= 10 && new_rows.len() <= 10,
            "matched rows should not exceed input size"
        );
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
    fn align_rows_amr_with_signatures_exposes_row_hashes() {
        let grid_a = grid_with_unique_rows(&[1, 2, 3, 4]);
        let grid_b = grid_with_unique_rows(&[1, 2, 3, 4]);

        let config = DiffConfig::default();
        let result =
            align_rows_amr_with_signatures(&grid_a, &grid_b, &config).expect("should align");

        assert_eq!(result.row_signatures_a.len(), grid_a.nrows as usize);
        assert_eq!(result.row_signatures_b.len(), grid_b.nrows as usize);
        assert_eq!(result.alignment.matched.len(), grid_a.nrows as usize);

        for row in 0..grid_a.nrows {
            let expected_a = grid_a.compute_row_signature(row);
            let expected_b = grid_b.compute_row_signature(row);
            assert_eq!(
                Some(expected_a),
                result.row_signatures_a.get(row as usize).copied(),
                "row {} signature for grid A should match compute_row_signature",
                row
            );
            assert_eq!(
                Some(expected_b),
                result.row_signatures_b.get(row as usize).copied(),
                "row {} signature for grid B should match compute_row_signature",
                row
            );
        }
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

    #[test]
    fn align_small_gap_enforces_cap_with_hash_fallback() {
        let large = (MAX_LCS_GAP_SIZE + 1) as usize;
        let old_hashes: Vec<u128> = (0..large as u32).map(|i| i as u128).collect();
        let new_hashes: Vec<u128> = (0..large as u32).map(|i| (10_000 + i) as u128).collect();

        let old_meta = row_meta_from_hashes(10, &old_hashes);
        let new_meta = row_meta_from_hashes(20, &new_hashes);

        let result = align_small_gap(&old_meta, &new_meta);
        assert_eq!(result.matched.len(), large);
        assert!(result.inserted.is_empty());
        assert!(result.deleted.is_empty());
        assert_eq!(result.matched.first(), Some(&(10, 20)));
        assert_eq!(
            result.matched.last(),
            Some(&(10 + large as u32 - 1, 20 + large as u32 - 1))
        );
    }

    #[test]
    fn hash_fallback_produces_monotone_pairs() {
        let old_meta = row_meta_from_hashes(0, &[1, 2, 3, 4]);
        let new_meta = row_meta_from_hashes(0, &[2, 1, 3, 4]);

        let result = align_gap_via_hash(&old_meta, &new_meta);
        assert_eq!(result.matched, vec![(1, 0), (2, 2), (3, 3)]);

        let is_monotone = result
            .matched
            .windows(2)
            .all(|w| w[0].0 <= w[1].0 && w[0].1 <= w[1].1);
        assert!(is_monotone, "hash fallback must preserve monotone ordering");
        assert_eq!(result.inserted, vec![1]);
        assert_eq!(result.deleted, vec![0]);
    }

    #[test]
    fn myers_handles_medium_gap_with_single_insertion() {
        let count = 300usize;
        let old_hashes: Vec<u128> = (0..count as u128).collect();
        let mut new_hashes: Vec<u128> = old_hashes.clone();
        new_hashes.insert(150, 9_999);

        let old_meta = row_meta_from_hashes(0, &old_hashes);
        let new_meta = row_meta_from_hashes(0, &new_hashes);

        let result = align_small_gap(&old_meta, &new_meta);
        assert_eq!(result.inserted, vec![150]);
        assert!(result.deleted.is_empty());
        assert_eq!(result.matched.len(), count);
        assert_eq!(result.matched.first(), Some(&(0, 0)));
        assert_eq!(
            result.matched.last(),
            Some(&(count as u32 - 1, (count + 1) as u32 - 1))
        );
    }
}
