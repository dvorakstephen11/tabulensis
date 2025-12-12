Not completely — the **core performance fix is largely in**, but you **missed two specific “don’t ever go quadratic again” guardrails** that were part of what I was pushing you toward.

## What you did implement correctly

### 1) The hard LCS cap + HashFallback wiring is in place

You added `MAX_LCS_GAP_SIZE = 1500` and the `HashFallback` strategy, and the strategy selection now routes oversized gaps away from raw O(n*m) LCS. 

Your `fill_gap()` dispatch handles `HashFallback` by going through the monotone hash/LIS alignment. 

Also, you added a second safety net: `align_small_gap()` itself refuses to run if either side exceeds the cap and falls back to hash alignment; and you added a Myers-based path for medium gaps so DP is not the default. 

That combination is the main thing that prevents the 50k-row case from melting the machine again.

### 2) Large MoveCandidate gaps no longer force LCS

In `fill_gap()`, your `MoveCandidate` arm explicitly switches to `align_gap_via_hash()` when the gap exceeds the cap. 
That’s the correct “never funnel big gaps into LCS” behavior.

### 3) Recursion is no longer “infinite thrash” when anchors are absent

You improved recursion by using local/context anchors at depth>0, and you also short-circuit to hash/LCS depending on cap when the anchor chain comes back empty.
That’s exactly the kind of “don’t recurse pointlessly” safety you want.

## What is still missing (and can still blow up)

### A) The “identical grids” fast path is still not in `align_rows_amr()`

The plan explicitly included a cheap O(N) early-exit when both sides’ row signatures match 1:1. 
But your current `align_rows_amr()` goes straight from `GridView` creation into the run-compression logic without that exact-match check. 

This won’t usually cause 7-minute behavior, but it’s a correctness/perf omission relative to what you said you were implementing, and it’s a free win on copied sheets.

### B) `find_block_move()` is still unbounded (still capable of quadratic/cubic behavior)

Your plan section about capping move detection exists for a reason. 
But the actual `find_block_move()` implementation still iterates **every candidate old position for a signature** (`for &old_idx in candidates`) with no cap.

Even though you reduced when it gets called (only if `has_nonzero_offset` and no moves found), **this is still a latent time bomb** on repetitive data or adversarial cases.

If your “50k rows took 7 minutes” dataset had lots of repeated/low-info rows and your pipeline ends up calling this even once on a giant gap, you can still get catastrophic runtime.

---

# Concrete fixes (code)

Below are two targeted replacements to make the implementation match the intent.

## 1) Add the identical-grid fast path in `align_rows_amr`

### Replace this code (in `core/src/alignment/assembly.rs`, inside `align_rows_amr` right after the two `GridView` lines)

```rust
pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    let runs_a = compress_to_runs(&view_a.row_meta);
    let runs_b = compress_to_runs(&view_b.row_meta);
```

### With this code

```rust
pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if view_a.row_meta.len() == view_b.row_meta.len()
        && view_a
            .row_meta
            .iter()
            .zip(view_b.row_meta.iter())
            .all(|(a, b)| a.signature == b.signature)
    {
        let mut matched = Vec::with_capacity(view_a.row_meta.len());
        for (a, b) in view_a.row_meta.iter().zip(view_b.row_meta.iter()) {
            matched.push((a.row_idx, b.row_idx));
        }
        return Some(RowAlignment {
            matched,
            inserted: Vec::new(),
            deleted: Vec::new(),
            moves: Vec::new(),
        });
    }

    let runs_a = compress_to_runs(&view_a.row_meta);
    let runs_b = compress_to_runs(&view_b.row_meta);
```

## 2) Cap and de-risk `find_block_move`

### Replace this code (entire `find_block_move` function in `core/src/alignment/move_extraction.rs`)

```rust
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
```

### With this code

```rust
pub fn find_block_move(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    min_len: u32,
) -> Option<RowBlockMove> {
    const MAX_SLICE_LEN: usize = 10_000;
    const MAX_CANDIDATES_PER_SIG: usize = 16;

    if old_slice.len() > MAX_SLICE_LEN || new_slice.len() > MAX_SLICE_LEN {
        return None;
    }

    let mut positions: HashMap<RowSignature, Vec<usize>> = HashMap::new();
    for (idx, meta) in old_slice.iter().enumerate() {
        if meta.is_low_info() {
            continue;
        }
        positions.entry(meta.signature).or_default().push(idx);
    }

    let mut best: Option<RowBlockMove> = None;
    let mut best_len: usize = 0;

    for (new_idx, meta) in new_slice.iter().enumerate() {
        if meta.is_low_info() {
            continue;
        }

        let Some(candidates) = positions.get(&meta.signature) else {
            continue;
        };

        for &old_idx in candidates.iter().take(MAX_CANDIDATES_PER_SIG) {
            let max_possible = (old_slice.len() - old_idx).min(new_slice.len() - new_idx);
            if max_possible <= best_len {
                continue;
            }

            let mut len = 0usize;
            while len < max_possible
                && old_slice[old_idx + len].signature == new_slice[new_idx + len].signature
            {
                len += 1;
            }

            if len >= min_len as usize && len > best_len {
                best_len = len;
                best = Some(RowBlockMove {
                    src_start_row: old_slice[old_idx].row_idx,
                    dst_start_row: new_slice[new_idx].row_idx,
                    row_count: len as u32,
                });
            }
        }
    }

    best
}
```

This keeps the function from becoming an accidental O(N²)/O(N³) sink and matches the intent of “move detection must not reintroduce the original performance cliff.”

---

## Bottom line

* **Yes**, your updated alignment path now has the crucial protections (cap + hash fallback + Myers + better recursion) that should eliminate the “50k rows → minutes” failure mode.
* **No**, it does *not* yet implement everything I would consider “done” because:

  1. The **identical-grid early exit** is absent.
  2. `find_block_move()` is **still unbounded**, and that’s unacceptable in a “fast diff engine” because it can resurrect catastrophic runtime.

If you apply the two replacements above, you’ll be much closer to “can’t get stuck for 7 minutes no matter what file you throw at it.”
