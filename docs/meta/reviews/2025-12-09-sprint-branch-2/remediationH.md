Here are the biggest things that jump out from the updated benchmarks + what I’d tackle next.

## 1) The quick-test numbers improved a lot, but “identical” is still the slowest case

From the newer quick benchmarks (1K-row tests), you’re at: 

* **P1 dense + single cell edit:** 92 ms
* **P2 noise (everything different):** 106 ms
* **P3 repetitive + single cell edit:** 432 ms
* **P4 99% blank + single cell edit:** 13 ms
* **P5 identical:** **1170 ms** (worst)

Compared to the earlier run you also attached, this is a huge step forward (e.g., P5 went from 4318 ms → 1170 ms).

What’s still “off” is the ordering: **identical should usually be your fastest path** (no ops to emit), but it’s currently your slowest. That almost always means there’s an expensive *validation / preprocessing* step that scales badly and doesn’t early-exit.

## 2) The main hot-path bug: `has_row_edits` is computed unconditionally and can trigger an O(R * M) signature scan

In `diff_workbooks_with_config`’s grid diff path, you compute `has_row_edits` like this: 

```rust
let has_structural_rows = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();

if has_structural_rows && alignment.matched.is_empty() {
    positional_diff(sheet_id, old, new, ops);
    return;
}

let has_row_edits = alignment
    .matched
    .iter()
    .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));

if has_structural_rows && has_row_edits {
    positional_diff(sheet_id, old, new, ops);
    return;
}
```

Two key problems here:

### Problem A: you compute `has_row_edits` even when you don’t use it

You only use it under `if has_structural_rows && has_row_edits`, but the expensive `.any(...)` runs even when `has_structural_rows` is false. That’s exactly the case for:

* P5 identical (no inserted/deleted)
* P1 single-cell edit
* P3 single-cell edit
* P4 single-cell edit

…and those are *precisely* the ones where you’d want to be extremely fast.

### Problem B: `row_signature_at()` can devolve into “scan the entire grid per row”

`row_signature_at()` falls back to `grid.compute_row_signature(row)` when the grid doesn’t already have cached row signatures.

And `compute_row_signature(row)` currently does this (conceptually): iterate **all cells in the grid** and filter to that row, for *each row you ask about*. 

So if you call it for many matched rows, you get:

* **O(R * M)** where:

  * R = number of rows you end up checking
  * M = number of stored cells in the HashMap (dense grid ⇒ M ~ rows * cols)

That matches your benchmark shape perfectly:

* In P5 identical, there’s never a mismatch, so `.any(...)` checks *all* matched rows.
* In P1/P3 “single cell edit at row 500”, `.any(...)` checks about half the rows then stops.

This one issue can absolutely dominate runtime on dense data.

## 3) Highest-impact fix: gate `has_row_edits`, and don’t use per-row signature computation in a loop

### Minimal fix (immediate big win for P1/P3/P5)

Move the computation behind `has_structural_rows`, since that’s the only time you use it.

**Code to replace**

```rust
let has_structural_rows = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();

if has_structural_rows && alignment.matched.is_empty() {
    positional_diff(sheet_id, old, new, ops);
    return;
}

let has_row_edits = alignment
    .matched
    .iter()
    .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));

if has_structural_rows && has_row_edits {
    positional_diff(sheet_id, old, new, ops);
    return;
}
```

**New code**

```rust
let has_structural_rows = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();

if has_structural_rows && alignment.matched.is_empty() {
    positional_diff(sheet_id, old, new, ops);
    return;
}

if has_structural_rows {
    let has_row_edits = alignment
        .matched
        .iter()
        .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));

    if has_row_edits {
        positional_diff(sheet_id, old, new, ops);
        return;
    }
}
```

This alone should take a huge bite out of **P5 identical** and also speed up **P1/P3** materially, because you stop doing that scan in the common “no structural changes” cases. 

### Better fix (prevents the same O(R * M) from coming back elsewhere)

Anywhere you *do* need to compare row signatures across many rows, avoid `row_signature_at()` unless signatures are already precomputed.

Two practical approaches:

1. **Compute row signatures once per grid (bulk), then index**

   * Use a bulk pass like your `compute_all_signatures()` strategy (one scan to build per-row lists), but return a `Vec<RowSignature>` instead of doing per-row scans. 
   * Then `has_row_edits` becomes an O(R) vector compare of matched pairs.

2. **Return row signatures from `align_rows_amr`**

   * `align_rows_amr` already computes row hashes via `GridView` internally; the engine then recomputes them again through `row_signature_at`. If `align_rows_amr` returned `(alignment, old_row_sigs, new_row_sigs)`, the engine could reuse them.

If you do only one “real” optimization pass, I’d do #2: it eliminates duplicated work and avoids building more views.

## 4) Secondary hotspot: numeric equality and hashing normalize floats with `log10/powi` repeatedly

You’ve implemented float tolerance by normalizing floats inside `PartialEq` for `CellValue::Number`, and the normalization uses `log10`, `powi`, rounding, etc.

That means:

* Every `cell_a.value == cell_b.value` on numbers can become “do transcendental math twice”.
* Every row/col hashing pass does the same normalization again. 

This may not be *the* dominant issue right now (the row-signature loop is worse), but once you fix #2, this becomes a likely next limiter for dense numeric sheets.

Two good options:

* **Cache a normalized representation per numeric cell**

  * Store `raw: f64` plus `norm_bits: u64` (or `norm_i64` for a scaled integer scheme).
  * Equality + hashing uses `norm_bits`, display uses `raw`.
  * You pay normalization once at parse/fixture creation, instead of per-compare/per-hash.

* **Switch to a cheaper tolerance model**

  * If your real goal is “tiny ULP drift shouldn’t diff”, compare floats by ULP distance (very cheap with `to_bits`), rather than decimal-significant-digit rounding.
  * This changes semantics slightly, so only do it if it matches your product expectations.

## 5) Make the benchmark output show phase breakdowns (it’ll validate the win instantly)

Your metrics machinery already tracks richer fields like `move_detection_time_ms`, `alignment_time_ms`, `cell_diff_time_ms`, anchors, moves, etc. 

But your perf harness prints only:

```
PERF_METRIC ... total_time_ms=... rows_processed=... cells_compared=...
```

and your parsing script only extracts `total_time_ms`.

If you print the phase timings per test (and parse them), you’ll be able to confirm that:

* after gating/fixing `has_row_edits`, the “alignment validation” chunk collapses, and
* whether float normalization is the next ceiling.

## What I’d do first (in order)

1. **Gate `has_row_edits` behind `has_structural_rows`** (tiny change, very high impact on P1/P3/P5). 
2. **Remove per-row `compute_row_signature()` from any loop** (bulk compute or reuse hashes computed by alignment).
3. **Add phase timing to perf output + JSON export** so each future regression is obvious.
4. If dense numeric still dominates: **cache normalized float** per cell.

If you want, paste the specific function containing that `has_row_edits` block (the real file, not the context summary), and I’ll rewrite it to reuse the alignment’s computed hashes so you don’t pay for row signatures twice.
