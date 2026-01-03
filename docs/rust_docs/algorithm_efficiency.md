
# Algorithm Efficiency Critical Fix Plan

**Date**: 2025-12-11  
**Priority**: P0 - Product Viability Blocker  
**Status**: Planning

---

## Executive Summary

The current AMR (Anchor-Move-Refine) alignment algorithm has **O(n²) worst-case complexity** that makes the product non-viable at scale. Testing with 50,000 row grids shows:

| Test Case              | Target Time | Actual Time | Status   |
|------------------------|-------------|-------------|----------|
| 50k identical rows     | <200ms      | >7 minutes  | **FAIL** |
| 50k single edit        | <500ms      | >7 minutes  | **FAIL** |
| 50k completely different | <500ms    | >7 minutes  | **FAIL** |
| 50k 99% blank          | <1s         | 20 seconds  | FAIL     |

This contradicts the core product differentiator: “Compare 100MB files in under 2 seconds.”

The problem is that large gaps inside AMR are still fed into an O(n²) LCS dynamic programming routine. The fix is to constrain LCS to genuinely small gaps, lean on recursive AMR for large gaps, and fall back to a **monotone, LIS-filtered hash matcher** when recursion can’t reduce a gap any further.

---

## Root Cause Analysis

### The O(n²) Bottleneck

The performance catastrophe originates in `align_small_gap()` in `assembly.rs`:

```rust
fn align_small_gap(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let m = old_slice.len();
    let n = new_slice.len();

    // O(m*n) space allocation
    let mut dp = vec![vec![0u32; n + 1]; m + 1];

    // O(m*n) time complexity (classic LCS DP)
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if old_slice[i].signature == new_slice[j].signature {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    // Backtrack to build GapAlignmentResult...
}
````

For two 50k-row grids with no anchors:

* **Space**: 50,000 × 50,000 × 4 bytes ≈ **10 GB RAM**
* **Time**: 2.5 billion iterations → **minutes, not milliseconds**

### Why This Happens

The `select_gap_strategy()` function in `gap_strategy.rs` routes every gap to a strategy. In the current implementation, anything that is not handled via recursion or a small-gap path eventually falls back into `SmallEdit`, which always calls `align_small_gap`:

```rust
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

    // Fallback: O(n²) on arbitrarily large gaps
    GapStrategy::SmallEdit
}
```

So:

* Large gaps that fail to recurse (e.g. recursion depth limit) end in `SmallEdit`.
* Large move-candidate gaps call into LCS via `SmallEdit` before move detection.
* Grids with no usable anchors (e.g. highly repetitive data) become one giant gap that’s fed straight into LCS.

### Scenarios That Trigger O(n²)

1. **Identical Grids With Repetition**

   If the grid has repeated content (e.g., generated patterns or templates), many rows aren’t unique and no anchors are produced. The entire 50k×50k region is treated as a single gap and aligned via LCS.

2. **Completely Different Grids**

   No signatures match between grids → zero anchors → entire grid is one 50k×50k gap → `SmallEdit` → LCS.

3. **Post-Recursion Large Gaps**

   After `max_recursion_depth` (default 10), any remaining large gap is still routed to `SmallEdit`, regardless of size.

4. **MoveCandidate Gaps**

   When `MoveCandidate` is selected for a large gap, the implementation calls `align_small_gap()` up front. So “there might be a move in here” still means “run LCS on the whole gap.”

---

## Solution Architecture

### Design Principles

1. **Hard complexity cap**

   No gap alignment should exceed roughly O(k²) work for `k ≤ MAX_LCS_GAP_SIZE`, and overall complexity across a sheet should be closer to O(N log N) or better.

2. **Recursive first, cap second**

   For big gaps, always try recursive AMR with anchors **before** considering an LCS cap. The hard cap is a safety net when recursion fails, not a gate that bypasses AMR.

3. **Quasi-linear degradation**

   When smart alignment is infeasible (oversized gaps at recursion limit), fall back to an **O(N log N) LIS-filtered hash matcher**, not positional matching inside AMR. The log factor is small enough that this behaves like linear time at our target sizes.

4. **Engine-level positional fallback only**

   The legacy positional diff remains a global fallback when AMR cannot produce an alignment at all, not a per-gap strategy inside AMR.

5. **Configurable limits**

   Keep “small gap” and recursion thresholds in `DiffConfig`. The LCS hard cap is a fixed constant chosen for safe performance and adjusted only when needed.

### Strategy Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                    align_rows_amr()                         │
├─────────────────────────────────────────────────────────────┤
│  1. FAST PATH: Identical signature check                    │
│     - If row_meta lengths and signatures match 1:1 → O(N)   │
│                                                             │
│  2. FAST PATH: RLE compression (existing)                   │
│     - If >50% rows compress → run-based alignment           │
│                                                             │
│  3. STANDARD PATH: Anchor-based alignment                   │
│     - Discover anchors (O(N))                               │
│     - Build LIS chain (O(A log A))                          │
│     - For each gap, use NEW strategy selection:             │
│         MoveCandidate / SmallEdit / RecursiveAlign /        │
│         HashFallback (~O(N log N))                          │
│                                                             │
│  4. ENGINE FALLBACK: Positional diff                        │
│     - If AMR returns None → legacy O(N) positional diff     │
└─────────────────────────────────────────────────────────────┘
```

---

## Detailed Implementation Plan (Corrected)

### Phase 1: Gap Strategy Ordering + Hard LCS Cap

**Goal**: Keep LCS strictly for small gaps, ensure recursive AMR is attempted first on large gaps, and add a **hard-capped** hash-based fallback (`HashFallback`) for oversized gaps when recursion can’t help.

#### New cap and gap strategy enum

```rust
// 1500^2 = 2.25M cells. Safe for sub-second LCS.
const MAX_LCS_GAP_SIZE: u32 = 1500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapStrategy {
    Empty,
    InsertAll,
    DeleteAll,
    SmallEdit,
    MoveCandidate,
    RecursiveAlign,
    // NEW: O(N log N) fallback when LCS is too expensive.
    HashFallback,
}
```

#### Corrected `select_gap_strategy`

Ordering is the key correction: we must not apply the hard cap before recursion, or AMR is effectively disabled on large files.

```rust
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

    // 1. Move candidates (cheap).
    if has_matching_signatures(old_slice, new_slice) {
        return GapStrategy::MoveCandidate;
    }

    // 2. Small edits: safe LCS zone.
    if old_len <= config.small_gap_threshold && new_len <= config.small_gap_threshold {
        return GapStrategy::SmallEdit;
    }

    // 3. Recursive AMR for large gaps.
    if (old_len > config.recursive_align_threshold || new_len > config.recursive_align_threshold)
        && !has_recursed
    {
        return GapStrategy::RecursiveAlign;
    }

    // 4. Hard cap: LCS is disallowed beyond this size.
    if old_len > MAX_LCS_GAP_SIZE || new_len > MAX_LCS_GAP_SIZE {
        return GapStrategy::HashFallback;
    }

    // 5. Default: still small enough for LCS.
    GapStrategy::SmallEdit
}
```

This:

* Preserves AMR’s ability to break a 50k-row gap into small subproblems via anchors.
* Only uses the cap when recursion is exhausted or impossible.
* Guarantees no large gap is ever sent to `SmallEdit`.

---

### Phase 2: Early Exit for Identical Grids

**Goal**: Detect identical content in O(N) time and bypass the entire AMR pipeline when sheets are copies of each other.

```rust
pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    // NEW: Fast path for identical grids.
    if view_a.row_meta.len() == view_b.row_meta.len() {
        let exact_match = view_a
            .row_meta
            .iter()
            .zip(view_b.row_meta.iter())
            .all(|(a, b)| a.signature == b.signature);

        if exact_match {
            let matched: Vec<(u32, u32)> = (0..view_a.row_meta.len() as u32)
                .map(|i| (i, i))
                .collect();

            return Some(RowAlignment {
                matched,
                inserted: Vec::new(),
                deleted: Vec::new(),
                moves: Vec::new(),
            });
        }
    }

    // Existing RLE fast path...
    // Existing anchor-based AMR path...
}
```

**Complexity**: O(N) for the check, O(N) to build the matched list.

---

### Phase 3: O(N log N) LIS-Filtered Hash Alignment for Large Gaps

**Goal**: Provide a quasi-linear, **monotone** alternative to LCS for large gaps. We want:

* No quadratic behavior.
* Matched pairs that are **monotone** (no crossing) so the diff “spine” stays sane.

#### Why naive hash matching is not enough

A naive queue-based hash matcher is O(N) but greedy: with repeated content it can happily match `(old=10 → new=50, old=11 → new=5)`, producing “spaghetti” alignments that cross over themselves and corrupt the diff structure.

We fix this by:

1. Using hash buckets and queues to find candidate matches in O(N).
2. Running **LIS (Longest Increasing Subsequence)** on the sequence of candidate match positions to extract a maximal **monotone** spine in O(N log N).
3. Treating discarded candidates as insertions and any unmatched old rows as deletions.

#### Corrected `align_gap_via_hash`

```rust
fn align_gap_via_hash(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    use std::collections::{HashMap, VecDeque, HashSet};
    use crate::workbook::RowSignature;

    // Signature -> queue of local old indices.
    let mut sig_to_old: HashMap<RowSignature, VecDeque<u32>> = HashMap::new();
    for (i, meta) in old_slice.iter().enumerate() {
        sig_to_old.entry(meta.signature).or_default().push_back(i as u32);
    }

    // Collect candidate matches in local coordinates.
    let mut candidate_pairs = Vec::new();
    let mut unmatched_new = Vec::new();

    for (j, meta) in new_slice.iter().enumerate() {
        if let Some(queue) = sig_to_old.get_mut(&meta.signature) {
            if let Some(old_local) = queue.pop_front() {
                candidate_pairs.push((old_local, j as u32));
            } else {
                unmatched_new.push(meta.row_idx);
            }
        } else {
            unmatched_new.push(meta.row_idx);
        }
    }

    // Enforce monotonicity via LIS on the new indices.
    candidate_pairs.sort_by_key(|(old_local, _)| *old_local);

    let lis_indexes = crate::alignment::anchor_chain::lis_indices(
        &candidate_pairs,
        |(_, new_local)| *new_local,
    );
    let lis_set: HashSet<usize> = lis_indexes.into_iter().collect();

    let mut matched = Vec::new();
    let mut deleted_local = HashSet::new();
    for i in 0..old_slice.len() {
        deleted_local.insert(i as u32);
    }

    for (i, (old_local, new_local)) in candidate_pairs.iter().copied().enumerate() {
        if lis_set.contains(&i) {
            matched.push((
                old_slice[old_local as usize].row_idx,
                new_slice[new_local as usize].row_idx,
            ));
            deleted_local.remove(&old_local);
        } else {
            unmatched_new.push(new_slice[new_local as usize].row_idx);
        }
    }

    let mut deleted: Vec<u32> = deleted_local
        .into_iter()
        .map(|idx| old_slice[idx as usize].row_idx)
        .collect();

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
```

**Complexity**: O(k log k) for a gap of size k, even with highly repetitive data, while preserving a monotone spine of matches.

This function is used for:

* `GapStrategy::HashFallback` (large gaps at recursion limit or with no anchors).
* Large `MoveCandidate` gaps.
* Large gaps in `RecursiveAlign` once depth is exhausted or anchors cannot be found.

---

### Phase 4: Fix `MoveCandidate` and Recursion to Never Fall Back to LCS on Huge Gaps

**Goal**: Make sure all large-gap paths inside AMR either recurse or use `align_gap_via_hash`, never raw LCS, and avoid useless deep recursion when there are no anchors.

#### Updated `fill_gap` logic

```rust
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
            let detected = moves_from_matched_pairs(&result.matched);
            result.moves.extend(detected);
            result
        }

        GapStrategy::MoveCandidate => {
            // Large move-candidate gaps use hash alignment, small ones still use LCS.
            let mut result = if old_slice.len() > MAX_LCS_GAP_SIZE as usize
                || new_slice.len() > MAX_LCS_GAP_SIZE as usize
            {
                align_gap_via_hash(old_slice, new_slice)
            } else {
                align_small_gap(old_slice, new_slice)
            };

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
            // At recursion limit: never drop a huge gap into LCS.
            if depth >= config.max_recursion_depth {
                if old_slice.len() > MAX_LCS_GAP_SIZE as usize
                    || new_slice.len() > MAX_LCS_GAP_SIZE as usize
                {
                    return align_gap_via_hash(old_slice, new_slice);
                }
                return align_small_gap(old_slice, new_slice);
            }

            let anchors = build_anchor_chain(discover_anchors_from_meta(old_slice, new_slice));

            // If we found no anchors, recursion is futile: jump straight to fallback.
            if anchors.is_empty() {
                if old_slice.len() > MAX_LCS_GAP_SIZE as usize
                    || new_slice.len() > MAX_LCS_GAP_SIZE as usize
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
```

With this in place:

* No gap larger than `MAX_LCS_GAP_SIZE` ever goes through LCS.
* `MoveCandidate` is safe even on very large repetitive gaps.
* Recursion termination **and** “no anchors” cases no longer funnel huge gaps into `align_small_gap`.

---

### Phase 5: Cap `find_block_move` to Avoid O(N²) on Repetitive Data

**Goal**: Prevent move detection from reintroducing O(N²) behavior in heavily duplicated data.

Previously, `find_block_move` iterated all candidate positions for each signature, which is O(N²) in worst case (e.g., 50k blank rows). We cap the number of candidates we explore per signature:

```rust
pub fn find_block_move(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    min_len: u32,
) -> Option<RowBlockMove> {
    use std::collections::HashMap;
    use crate::workbook::RowSignature;

    let mut positions: HashMap<RowSignature, Vec<usize>> = HashMap::new();
    for (idx, meta) in old_slice.iter().enumerate() {
        positions.entry(meta.signature).or_default().push(idx);
    }

    let mut best: Option<RowBlockMove> = None;
    const MAX_CANDIDATES: usize = 50;

    for (new_idx, meta) in new_slice.iter().enumerate() {
        if let Some(candidates) = positions.get(&meta.signature) {
            // Cap per-signature candidate exploration.
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
```

We still infer useful block moves but with bounded work, even with huge numbers of identical rows.

---

## Complexity Analysis After Fix

| Operation                             | Before                             | After                                                |
| ------------------------------------- | ---------------------------------- | ---------------------------------------------------- |
| Identical grids (50k rows)            | O(N²) (single huge LCS gap)        | O(N) fast path                                       |
| Single cell edit                      | O(N²)                              | O(N log N) anchors + small LCS gaps                  |
| Completely different grids (50k rows) | O(N²)                              | O(N log N) via `HashFallback` (`align_gap_via_hash`) |
| Large gap fill                        | O(N²)                              | O(N log N) (`align_gap_via_hash`)                    |
| Move detection in repetitive data     | O(N²) from uncapped candidate scan | O(N) + constant-capped candidate exploration         |

The only remaining LCS work is on gaps with both sides ≤ `MAX_LCS_GAP_SIZE` and not at recursion limit.

---

## Testing Strategy

### Unit Tests

1. **Gap Strategy Ordering & Caps**

   * `gap_strategy_orders_recursion_before_cap`
   * `gap_strategy_uses_hashfallback_for_large_nonrecursive_gaps`

2. **Hash Fallback Behavior**

   * `hash_fallback_matches_lcs_on_small_examples`
   * `hash_fallback_is_quasilinear_on_repetitive_data`

3. **Fast Path**

   * `identical_grid_fast_path_produces_full_match`

4. **Move Detection**

   * `find_block_move_respects_candidate_cap`
   * `move_candidate_uses_hash_for_large_gaps`

### Performance Tests (50k Scale)

1. `perf_50k_identical_under_200ms`
2. `perf_50k_single_edit_under_1s`
3. `perf_50k_completely_different_under_500ms`
4. `perf_50k_adversarial_repetitive_under_15s` (lots of blank or low-info rows)

### Regression Tests

* All existing AMR tests.
* All existing `limit_behavior` tests (depth limits, thresholds).
* Validation that diff semantics (inserted/deleted content) remain correct even if alignment details differ.

---

## Implementation Order

1. **Day 1**: Implement Phase 1 (gap strategy ordering + hard LCS cap + `HashFallback`).
2. **Day 2**: Implement Phase 2 (identical grid fast path).
3. **Day 3**: Implement Phase 3 (LIS-filtered `align_gap_via_hash` + wiring into `HashFallback`).
4. **Day 4**: Implement Phase 4–5 (MoveCandidate/RecursiveAlign wiring + `find_block_move` cap + no-anchors recursion short-circuit).
5. **Day 5**: Run full 50k perf suite, profile, and tune thresholds.

---

## Risk Assessment

| Risk                                                                                         | Mitigation                                                                                                                                                  |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hash-based alignment produces different alignments than optimal LCS for large gaps           | Document that large-gap alignment is “good enough” vs strictly optimal; correctness is enforced by cell-level diff; retain LCS for small gaps.              |
| Hash fallback or capped move detection misses some subtle move patterns                      | This affects visual grouping, not correctness; we still compute the right set of inserted/deleted rows. Keep LCS + anchor-based moves where gaps are small. |
| Poor choice of `MAX_LCS_GAP_SIZE` either reintroduces perf issues or harms alignment quality | Start with 1500 (2.25M DP cells), validate in perf/regression tests, and adjust if necessary.                                                               |
| Extra hash maps / queues and LIS step increase memory or CPU use                             | The fallback is O(N log N) with small constants; confirm 50k×100 grids stay under ~1GB and within perf targets in perf tests.                               |

---

## Success Criteria

1. All 50k perf tests pass within 10× of target times initially (conservative goal).
2. No test takes longer than 30 seconds.
3. All existing AMR/limit behavior tests pass.
4. Memory usage stays under 1GB for 50k×100 grids.

```
