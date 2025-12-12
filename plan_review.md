The plan you provided (`algorithm_efficiency.md`) is **excellent and directly addresses the root causes** of the performance failure. You have correctly identified the $O(N^2)$ bottlenecks in `align_small_gap` and the fallback logic.

However, to ensure this solution is robust and avoids "performance landmines" (like infinite recursion loops on large, disjoint grids), I have identified specific mathematical and logic refinements to your plan.

### **Critique & Refinements**

1.  **Monotonicity Enforcement (Algorithmic improvement)**:

      * **Problem**: A naive "Queue" hash match (as suggested in the plan) is greedy and can produce "spaghetti" crossing matches (e.g., Row 10 matches Row 50, Row 11 matches Row 5). This corrupts the "spine" of the diff.
      * **Fix**: I have implemented `align_gap_via_hash` using **Longest Increasing Subsequence (LIS)** filtering. This guarantees that the fallback alignment is monotonic (non-crossing) in $O(N \log N)$ time, preserving the diff structure while solving the performance issue.

2.  **Infinite Recursion Trap**:

      * **Problem**: If `RecursiveAlign` is selected for a huge gap but `discover_anchors` finds nothing (common in noisy/blank data), your original plan would recurse `max_recursion_depth` (10) times, effectively performing 10 expensive scans before failing.
      * **Fix**: Short-circuit immediately if `anchors.is_empty()` to jump straight to the fallback.

3.  **Move Detection Cap**:

      * **Problem**: `find_block_move` iterates through *all* candidates. For 50k identical rows, this triggers $2.5$ billion comparisons ($O(N^2)$).
      * **Fix**: Explicitly cap the candidate search to the first 50 matches per signature.

-----

### **Optimized Code Implementation**

These changes replace the problematic files. I have also updated `anchor_chain.rs` to expose the LIS logic for reuse in the hash fallback.

#### **1. `core/src/alignment/gap_strategy.rs`**

*Updates: Adds `HashFallback`, `MAX_LCS_GAP_SIZE`, and proper strategy ordering.*

```rust
use std::collections::HashSet;
use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
use crate::config::DiffConfig;

// Hard cap for LCS. 1500^2 = 2.25M cells (approx 10-50ms). 
// Safe for sub-second execution.
pub const MAX_LCS_GAP_SIZE: u32 = 1500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapStrategy {
    Empty,
    InsertAll,
    DeleteAll,
    SmallEdit,
    MoveCandidate,
    RecursiveAlign,
    HashFallback, // NEW: O(N log N) fallback for massive gaps
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

    // 1. Move candidates: Cheap check, worth prioritizing
    if has_matching_signatures(old_slice, new_slice) {
        return GapStrategy::MoveCandidate;
    }

    // 2. Small Edit: Safe zone for LCS
    if old_len <= config.small_gap_threshold && new_len <= config.small_gap_threshold {
        return GapStrategy::SmallEdit;
    }

    // 3. Recursive Align: Try to break big gaps down
    if (old_len > config.recursive_align_threshold || new_len > config.recursive_align_threshold)
        && !has_recursed
    {
        return GapStrategy::RecursiveAlign;
    }

    // 4. Hard Cap: If we are here, the gap is large and recursion failed/isn't allowed.
    // We MUST NOT run LCS on 50k rows.
    if old_len > MAX_LCS_GAP_SIZE || new_len > MAX_LCS_GAP_SIZE {
        return GapStrategy::HashFallback;
    }

    // 5. Default to SmallEdit (LCS) if within safe bounds
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

#### **2. `core/src/alignment/assembly.rs`**

*Updates: Adds Identical Grid Fast Path, LIS-Filtered Hash Alignment, and optimized `fill_gap`.*

```rust
use std::ops::Range;
use std::collections::{HashMap, VecDeque};

use crate::alignment::anchor_chain::{build_anchor_chain, lis_indices};
use crate::alignment::anchor_discovery::{Anchor, discover_anchors_from_meta};
use crate::alignment::gap_strategy::{GapStrategy, select_gap_strategy, MAX_LCS_GAP_SIZE};
use crate::alignment::move_extraction::{find_block_move, moves_from_matched_pairs};
use crate::alignment::row_metadata::RowMeta;
use crate::alignment::runs::{RowRun, compress_to_runs};
use crate::config::DiffConfig;
use crate::grid_view::GridView;
use crate::workbook::Grid;
use crate::alignment::RowBlockMove;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowAlignment {
    pub matched: Vec<(u32, u32)>,
    pub inserted: Vec<u32>,
    pub deleted: Vec<u32>,
    pub moves: Vec<RowBlockMove>,
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

    // --- PHASE 2 FAST PATH: Identical Grid Check ---
    // If grids are structural clones, O(N) exit.
    if view_a.row_meta.len() == view_b.row_meta.len() {
        let exact_match = view_a.row_meta.iter()
            .zip(view_b.row_meta.iter())
            .all(|(a, b)| a.signature == b.signature);
        
        if exact_match {
            return Some(RowAlignment {
                matched: (0..view_a.row_meta.len() as u32).map(|i| (i, i)).collect(),
                inserted: Vec::new(),
                deleted: Vec::new(),
                moves: Vec::new(),
            });
        }
    }

    let runs_a = compress_to_runs(&view_a.row_meta);
    let runs_b = compress_to_runs(&view_b.row_meta);
    
    // (Existing RLE fast path logic)
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

    // Fallthrough to standard AMR
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
        
        // PHASE 3: Hash Fallback for Massive Gaps with LIS Filtering
        GapStrategy::HashFallback => {
            let mut result = align_gap_via_hash(old_slice, new_slice);
            let detected = moves_from_matched_pairs(&result.matched);
            result.moves.extend(detected);
            result
        },

        GapStrategy::MoveCandidate => {
            // Safety: If gap is huge, avoid O(N^2) LCS. 
            // Use hash alignment to find the spine, then look for moves.
            let mut result = if old_slice.len() > MAX_LCS_GAP_SIZE as usize 
                || new_slice.len() > MAX_LCS_GAP_SIZE as usize 
            {
                align_gap_via_hash(old_slice, new_slice)
            } else {
                align_small_gap(old_slice, new_slice)
            };

            let mut detected_moves = moves_from_matched_pairs(&result.matched);
            if detected_moves.is_empty() {
                // Heuristic scan for block moves (now capped)
                if let Some(mv) = find_block_move(old_slice, new_slice, 1) {
                    detected_moves.push(mv);
                }
            }
            result.moves.extend(detected_moves);
            result
        }

        GapStrategy::RecursiveAlign => {
            // SAFETY: recursion limit check
            if depth >= config.max_recursion_depth {
                if old_slice.len() > MAX_LCS_GAP_SIZE as usize || new_slice.len() > MAX_LCS_GAP_SIZE as usize {
                    return align_gap_via_hash(old_slice, new_slice);
                }
                return align_small_gap(old_slice, new_slice);
            }

            let anchors = build_anchor_chain(discover_anchors_from_meta(old_slice, new_slice));
            
            // CRITICAL OPTIMIZATION:
            // If we found no anchors, recursion is futile. Don't recurse depth times.
            if anchors.is_empty() {
                 if old_slice.len() > MAX_LCS_GAP_SIZE as usize || new_slice.len() > MAX_LCS_GAP_SIZE as usize {
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

/// O(N log N) alignment that approximates LCS for large gaps.
/// 1. Maps signatures to queues of indices (O(N)).
/// 2. Finds candidate matches.
/// 3. Filters candidates using Longest Increasing Subsequence (LIS) to ensure monotonicity (O(N log N)).
fn align_gap_via_hash(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let mut sig_to_old: HashMap<crate::workbook::RowSignature, VecDeque<u32>> = HashMap::new();
    for (i, meta) in old_slice.iter().enumerate() {
        sig_to_old.entry(meta.signature).or_default().push_back(i as u32);
    }

    // Collect candidate matches (Greedy by signature)
    let mut candidate_pairs = Vec::new();
    let mut unmatched_new = Vec::new();

    for (j, meta) in new_slice.iter().enumerate() {
        if let Some(queue) = sig_to_old.get_mut(&meta.signature) {
             if let Some(old_local_idx) = queue.pop_front() {
                 candidate_pairs.push((old_local_idx, j as u32));
             } else {
                 unmatched_new.push(meta.row_idx);
             }
        } else {
            unmatched_new.push(meta.row_idx);
        }
    }

    // Filter for Monotonicity using LIS.
    // Pairs must be strictly increasing in both dimensions to form a valid "spine".
    candidate_pairs.sort_by_key(|(old, _)| *old);
    
    // lis_indices returns indices into `candidate_pairs` that form the LIS of the `new` coordinate.
    let lis_indexes = lis_indices(&candidate_pairs, |(_, new)| *new);
    let lis_set: std::collections::HashSet<usize> = lis_indexes.iter().cloned().collect();

    let mut matched = Vec::new();
    let mut deleted_local_indices = std::collections::HashSet::new();
    for i in 0..old_slice.len() {
        deleted_local_indices.insert(i as u32);
    }

    for (i, pair) in candidate_pairs.iter().enumerate() {
        if lis_set.contains(&i) {
            let (old_local, new_local) = *pair;
            matched.push((old_slice[old_local as usize].row_idx, new_slice[new_local as usize].row_idx));
            deleted_local_indices.remove(&old_local);
        } else {
            // Discarded by LIS -> Treat as insertion
            let (_, new_local) = *pair;
            unmatched_new.push(new_slice[new_local as usize].row_idx);
        }
    }

    // Remaining old indices are deletions
    let mut deleted: Vec<u32> = deleted_local_indices
        .into_iter()
        .map(|idx| old_slice[idx as usize].row_idx)
        .collect();
    
    // unmatched_new contains the actual row indices already
    let mut inserted = unmatched_new;

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

// ... (Existing implementation of align_runs_stable, slice_by_range, align_small_gap) ...
// (Omitted for brevity, but they should be kept as in the original file)
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
```

#### **3. `core/src/alignment/move_extraction.rs`**

*Updates: Add `MAX_CANDIDATES` cap to `find_block_move`.*

```rust
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
    
    // NEW: Cap search to prevent O(N^2) on identical grids.
    // 50 candidates is enough to find valid moves without blowing up.
    const MAX_CANDIDATES: usize = 50; 

    for (new_idx, meta) in new_slice.iter().enumerate() {
        if let Some(candidates) = positions.get(&meta.signature) {
            // Only check the first N candidates for this signature
            for &old_idx in candidates.iter().take(MAX_CANDIDATES) {
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

// ... (moves_from_matched_pairs logic remains unchanged)
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

#### **4. `core/src/alignment/anchor_chain.rs`**

*Updates: Make `lis_indices` available to `assembly.rs`.*

```rust
//! Anchor chain construction using Longest Increasing Subsequence (LIS).

use crate::alignment::anchor_discovery::Anchor;

pub fn build_anchor_chain(mut anchors: Vec<Anchor>) -> Vec<Anchor> {
    // Sort by new_row to preserve destination order before LIS on old_row.
    anchors.sort_by_key(|a| a.new_row);
    let indices = lis_indices(&anchors, |a| a.old_row);
    indices.into_iter().map(|idx| anchors[idx]).collect()
}

// Change to pub(crate) so assembly.rs can use it for HashFallback LIS
pub(crate) fn lis_indices<T, F>(items: &[T], key: F) -> Vec<usize>
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
```