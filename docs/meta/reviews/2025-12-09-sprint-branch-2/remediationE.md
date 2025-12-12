# Algorithm Efficiency Critical Fix Plan v2.1

**Date**: 2025-12-11
**Priority**: P0 — Product viability blocker
**Scope**: Row alignment (AMR), large-gap behavior, recursion, move-candidate safety
**Core Promise**: No O(n*m) work beyond a fixed cap; overall alignment must degrade ~O(N log N) or better

---

## Executive summary

The current AMR pipeline still routes large gaps into **classic LCS DP** (`align_small_gap`) which is O(n*m) time and memory, explaining the observed multi-minute hangs on 50k rows. 

This plan fixes that while preserving the “excellence” of your current architecture (anchors + LIS + gap-filling + move extraction) by:

1. Introducing a **true hard cap** for quadratic alignment and enforcing it in **two places**:

   * in gap-strategy selection, and
   * inside `align_small_gap` itself (seatbelt).
2. Adding a **monotone hash/LIS fallback** (`HashFallback`) for oversized gaps.
3. Ensuring **MoveCandidate** and recursion-limit paths **never** funnel huge gaps into LCS.
4. Fixing a correctness landmine in run compression (`start_row` must use `row_idx`).
5. Upgrading recursion (optional but recommended) to use **local anchors** (uniqueness within the gap), and optionally **context anchors** (k-grams), so recursion actually reduces hard cases instead of futilely recursing and then falling back.

---

## Root cause

The bottleneck is `align_small_gap()` DP LCS, which allocates `m+1` vectors of `n+1` each and does O(m*n) iterations. Your original plan already identifies this correctly. 

What makes it catastrophic today is dispatch:

* `MoveCandidate` currently calls `align_small_gap` first (so “maybe move” becomes “definitely quadratic”).
* `RecursiveAlign` at depth limit currently calls `align_small_gap`.
* “No anchors” cases produce one giant gap that falls into `SmallEdit`, which is LCS.

---

## Non-negotiable performance invariants

1. **No code path may perform O(m*n) alignment work when `m` or `n` exceeds `MAX_LCS_GAP_SIZE`.**
2. The cap is enforced twice: strategy + DP function.
3. Recursion must not devolve into “keep recursing until depth, then do huge LCS.”
4. Large-gap fallback must return a **monotone** matched spine (no crossing pairs).
5. Preserve existing UX behavior: if `m == n` and **no signatures match**, treat as “changed rows” via **positional pairing**, not delete+insert everything.

---

## Solution overview

### Strategies used per gap

* **Empty / InsertAll / DeleteAll**: trivial
* **SmallEdit**: LCS DP (only when both sides are truly small)
* **RecursiveAlign**: recursive AMR (but short-circuit if anchors don’t split)
* **MoveCandidate**: align (non-quadratic if large), then detect moves
* **HashFallback**: monotone hash+LIS for large gaps (O(k log k))

---

## Phase 1 — Gap strategy fix + hard cap + HashFallback

### File: `core/src/alignment/gap_strategy.rs`

#### 1) Replace enum + selector to add `MAX_LCS_GAP_SIZE`, clamp thresholds, and add `HashFallback`

**Code to replace** (entire `GapStrategy` enum and `select_gap_strategy` function):

```rust
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
```

**New code to replace it**:

```rust
pub const MAX_LCS_GAP_SIZE: u32 = 1500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapStrategy {
    Empty,
    InsertAll,
    DeleteAll,
    SmallEdit,
    MoveCandidate,
    RecursiveAlign,
    HashFallback,
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

    let is_move_candidate = has_matching_signatures(old_slice, new_slice);

    let small_threshold = config.small_gap_threshold.min(MAX_LCS_GAP_SIZE);
    if old_len <= small_threshold && new_len <= small_threshold {
        return if is_move_candidate {
            GapStrategy::MoveCandidate
        } else {
            GapStrategy::SmallEdit
        };
    }

    if (old_len > config.recursive_align_threshold || new_len > config.recursive_align_threshold)
        && !has_recursed
    {
        return GapStrategy::RecursiveAlign;
    }

    if is_move_candidate {
        return GapStrategy::MoveCandidate;
    }

    if old_len > MAX_LCS_GAP_SIZE || new_len > MAX_LCS_GAP_SIZE {
        return GapStrategy::HashFallback;
    }

    GapStrategy::SmallEdit
}
```

This preserves your “recursive first” intent, while guaranteeing large gaps don’t route into DP. 

---

## Phase 2 — Wire HashFallback, fix MoveCandidate + recursion-limit behavior

### File: `core/src/alignment/assembly.rs`

#### 2) Replace the `match strategy { ... }` inside `fill_gap(...)`

**Code to replace** (the whole match block inside `fill_gap`):

```rust
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
```

**New code to replace it**:

```rust
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
            result.moves.extend(moves_from_matched_pairs(&result.matched));
            result
        }

        GapStrategy::MoveCandidate => {
            let mut result = if old_slice.len() as u32 > crate::alignment::gap_strategy::MAX_LCS_GAP_SIZE
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

                if has_nonzero_offset {
                    if let Some(mv) = find_block_move(old_slice, new_slice, 1) {
                        detected_moves.push(mv);
                    }
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

            let anchors = build_anchor_chain(discover_anchors_from_meta(old_slice, new_slice));
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
```

This is the “no giant-LCS in MoveCandidate / recursion-limit / no-anchor recursion” fix. 

---

## Phase 3 — Implement the O(N log N) monotone hash fallback

### File: `core/src/alignment/assembly.rs`

#### 3) Add these new helper functions (no replacement; paste below `align_small_gap`)

```rust
fn align_gap_via_hash(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    use std::collections::{HashMap, VecDeque};

    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let mut sig_to_new: HashMap<crate::workbook::RowSignature, VecDeque<u32>> = HashMap::new();
    for (j, meta) in new_slice.iter().enumerate() {
        sig_to_new.entry(meta.signature).or_default().push_back(j as u32);
    }

    let mut candidate_pairs: Vec<(u32, u32)> = Vec::new();
    for (i, meta) in old_slice.iter().enumerate() {
        if let Some(q) = sig_to_new.get_mut(&meta.signature) {
            if let Some(j) = q.pop_front() {
                candidate_pairs.push((i as u32, j));
            }
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
            matched.push((old_slice[old_i as usize].row_idx, new_slice[new_j as usize].row_idx));
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
```

Key properties:

* O(k log k) time (via LIS)
* monotone matched spine
* preserves positional pairing when `m == n` and zero matches

---

## Phase 4 — Seatbelt: prevent future regressions inside `align_small_gap`

This is mandatory. Dispatch bugs happen; this prevents a future “oops” from reintroducing 7-minute diffs.

### File: `core/src/alignment/assembly.rs`

#### 4) Replace the start of `align_small_gap` to refuse oversized gaps

**Code to replace** (the first part of `align_small_gap` up through the early return, i.e. replace from `let m = ...` down to the `dp` allocation line):

```rust
fn align_small_gap(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let mut dp = vec![vec![0u32; n + 1]; m + 1];
```

**New code to replace it**:

```rust
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

    let mut dp = vec![vec![0u32; n + 1]; m + 1];
```

This ensures the quadratic routine is never used beyond the cap, no matter what. 

---

## Phase 5 — Correctness fix: RLE run start row bug

This one isn’t your 7-minute hang, but it’s wrong and will bite you as soon as you compress slices not starting at row 0.

### File: `core/src/alignment/runs.rs`

#### 5) Fix `start_row` to use `row_idx`, not local index

**Code to replace**:

```rust
        runs.push(RowRun {
            signature: sig,
            start_row: start as u32,
            count: (i - start) as u32,
        });
```

**New code to replace it**:

```rust
        runs.push(RowRun {
            signature: sig,
            start_row: meta[start].row_idx,
            count: (i - start) as u32,
        });
```

---

## Phase 6 — Upgrade recursion to actually split hard gaps (local anchors)

Your original plan’s recursion can be ineffective because anchor discovery relies on precomputed `FrequencyClass::Unique` which is typically global. The fix is to discover “unique within the gap” anchors at recursion depth > 0. 

### Minimal implementation (recommended)

Add a new local-anchor helper (you can put this in `anchor_discovery.rs` or directly in `assembly.rs`):

```rust
fn discover_local_anchors(old: &[RowMeta], new: &[RowMeta]) -> Vec<Anchor> {
    use std::collections::HashMap;

    let mut count_old: HashMap<crate::workbook::RowSignature, u32> = HashMap::new();
    for m in old.iter() {
        if !m.is_low_info() {
            *count_old.entry(m.signature).or_insert(0) += 1;
        }
    }

    let mut count_new: HashMap<crate::workbook::RowSignature, u32> = HashMap::new();
    for m in new.iter() {
        if !m.is_low_info() {
            *count_new.entry(m.signature).or_insert(0) += 1;
        }
    }

    let mut pos_old: HashMap<crate::workbook::RowSignature, u32> = HashMap::new();
    for m in old.iter() {
        if !m.is_low_info() && count_old.get(&m.signature).copied().unwrap_or(0) == 1 {
            pos_old.insert(m.signature, m.row_idx);
        }
    }

    let mut out = Vec::new();
    for m in new.iter() {
        if m.is_low_info() {
            continue;
        }
        if count_new.get(&m.signature).copied().unwrap_or(0) != 1 {
            continue;
        }
        if let Some(old_row) = pos_old.get(&m.signature) {
            out.push(Anchor {
                old_row: *old_row,
                new_row: m.row_idx,
                signature: m.signature,
            });
        }
    }
    out
}
```

Then, in the `RecursiveAlign` arm (inside `fill_gap`), replace anchor discovery for recursion:

* for depth 0: current global unique anchors are fine
* for depth > 0: use `discover_local_anchors`

This keeps behavior stable at top-level while making recursion effective.

---

## Phase 7 — Context anchors (k-grams) for highly repetitive sheets (optional but powerful)

When individual rows aren’t unique, sequences often are. If local anchors are still too sparse:

* choose k = 4 or 8
* rolling-hash k consecutive row signatures into a “window signature”
* treat windows unique-in-both as anchors
* chain via LIS the same way

This transforms “no anchors” repetitive cases from “fallback immediately” into “recursion splits the gap.”

(Implementation is straightforward but longer; it’s the next escalation after local anchors.)

---

## Phase 8 — Myers diff for medium gaps (optional quality+speed win)

For medium gaps where edits are small relative to N, Myers runs in ~O((n+m)·D) and is usually far faster than DP LCS. Policy:

* small: DP LCS (already)
* medium: Myers
* huge: recursion + hash fallback

This is an upgrade, not needed to stop the current hangs.

---

## Testing and perf gates

Keep all tests from your plan, and add these invariants:

1. **Hard cap test**: calling `align_small_gap` with `m > MAX_LCS_GAP_SIZE` must not allocate DP and must return quickly.
2. **MoveCandidate safety**: large move-candidate gap must not call LCS.
3. **No-anchor recursion**: recursion should short-circuit; never “recurse to depth then LCS.”
4. **Positional parity**: `m == n` and zero matches ⇒ matched-by-position, not delete+insert.

---

## Implementation order (do this in order)

1. Phase 1–4 (cap + HashFallback + wiring + seatbelt)
2. Phase 5 (RLE correctness fix)
3. Phase 6 (local anchors)
4. Phase 7/8 if alignment quality/perf needs further improvement


