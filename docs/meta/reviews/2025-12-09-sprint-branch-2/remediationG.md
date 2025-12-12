## What your benchmarks are really telling you

The standout outlier is the “no-op” case:

* `perf_p5_identical`: **4318 ms** to diff two identical 1000x100 grids (100,000 overlapped cells). 
* Compare that to:

  * `perf_p1_large_dense`: **359 ms** for a 1000x20 grid with a single cell edit (20,000 cells). 
  * `perf_p4_99_percent_blank`: **24.6 ms** for a huge mostly-empty overlap. 
  * `perf_p3_adversarial_repetitive`: **1237 ms** (49,950 cells). 

A “no changes” diff should be among your fastest cases. When it’s your *slowest*, it almost always means you have an expensive “verification” or “safety fallback” path that does far more work than necessary.

In your codebase, there is exactly such a path.

---

## Primary bottleneck: an accidental O(rows * cells) signature loop

### The trigger

Inside `try_diff_grids_with_config`, after you already ran AMR alignment, you have this branch:

* If there are no moves and the *multiset* of row signatures matches, you fall back to positional diff. 

That check is implemented as:

* `row_signature_multiset_equal -> row_signature_counts -> row_signature_at` 

### Why it explodes on dense sheets

When `grid.row_signatures` is not precomputed, `row_signature_at` calls:

* `grid.compute_row_signature(row)` 

And `Grid::compute_row_signature` is implemented as:

* iterate **all cells** and filter by row (`self.cells.values().filter(|cell| cell.row == row)`), i.e. a full scan per row. 

So `row_signature_counts` is effectively:

* For each row (R): scan all cells (M) to compute that row’s signature
* Total: **O(R * M)**

For `perf_p5_identical` that’s roughly 1000 rows * 100k cells = 100 million cell-visits *per grid* (plus sorting/hash work), which maps extremely well to “why is the identical case the slowest.” 

### Fix strategy

You want `row_signature_counts(grid)` to compute all row signatures in *one pass* over the cells (bucket by row), not by rescanning the whole grid per row.

This preserves your current semantics (still uses `hash_row_content_128`), but changes the complexity from **O(R*M)** to roughly **O(M + sum sort(row_cells))**, i.e. closer to **O(M log C)** on dense data (C = columns per row).

### Drop-in code replacement

Replace the slow “scan whole grid once per row” approach in the *engine-side* helper.

**Code to replace** (current pattern):

```rust
fn row_signature_at(grid: &Grid, row: u32) -> Option<RowSignature> {
    if let Some(sig) = grid
        .row_signatures
        .as_ref()
        .and_then(|rows| rows.get(row as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_row_signature(row))
}

fn row_signature_counts(grid: &Grid) -> HashMap<RowSignature, u32> {
    let mut counts = HashMap::new();
    for row in 0..grid.nrows {
        if let Some(sig) = row_signature_at(grid, row) {
            *counts.entry(sig).or_insert(0) += 1;
        }
    }
    counts
}

fn row_signature_multiset_equal(a: &Grid, b: &Grid) -> bool {
    if a.nrows != b.nrows {
        return false;
    }
    row_signature_counts(a) == row_signature_counts(b)
}
```

**Replace with** (single-pass bucket + hash):

```rust
fn row_signature_at(grid: &Grid, row: u32) -> Option<RowSignature> {
    if let Some(sig) = grid
        .row_signatures
        .as_ref()
        .and_then(|rows| rows.get(row as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_row_signature(row))
}

fn row_signature_counts(grid: &Grid) -> HashMap<RowSignature, u32> {
    if let Some(rows) = grid.row_signatures.as_ref() {
        let mut counts: HashMap<RowSignature, u32> = HashMap::with_capacity(rows.len());
        for &sig in rows {
            *counts.entry(sig).or_insert(0) += 1;
        }
        return counts;
    }

    use crate::hashing::hash_row_content_128;

    let nrows = grid.nrows as usize;
    let mut rows: Vec<Vec<(u32, &Cell)>> = vec![Vec::new(); nrows];

    for cell in grid.cells.values() {
        rows[cell.row as usize].push((cell.col, cell));
    }

    let mut counts: HashMap<RowSignature, u32> = HashMap::with_capacity(nrows);
    for mut row_cells in rows {
        row_cells.sort_by_key(|(col, _)| *col);
        let hash = hash_row_content_128(&row_cells);
        let sig = RowSignature { hash };
        *counts.entry(sig).or_insert(0) += 1;
    }

    counts
}

fn row_signature_multiset_equal(a: &Grid, b: &Grid) -> bool {
    if a.nrows != b.nrows {
        return false;
    }
    row_signature_counts(a) == row_signature_counts(b)
}
```

This directly targets the pathological scaling that makes `perf_p5_identical` so slow.

---

## Secondary bottleneck: cloning snapshots for every compared cell

Your core inner loop does this:

* Build `from` and `to` snapshots *first* (which clones `String`s for text/formulas), then compare snapshots. 

That’s fine for numeric-only synthetic benches, but in real Excel files with lots of strings/formulas it causes:

* tons of allocations and memory traffic even when the sheets are identical
* “no-op diff” becoming expensive for the wrong reason

### Fix strategy

Compare cell content first (no allocations), and only construct snapshots when a difference is found.

You already have `cells_content_equal(...)` implemented in the same module. 

### Drop-in code replacement for `diff_row_pair`

**Code to replace**:

```rust
fn diff_row_pair(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    ops: &mut Vec<DiffOp>,
) {
    for col in 0..overlap_cols {
        let addr = CellAddress::from_indices(row_b, col);
        let old_cell = old.get(row_a, col);
        let new_cell = new.get(row_b, col);

        let from = snapshot_with_addr(old_cell, addr);
        let to = snapshot_with_addr(new_cell, addr);

        if from != to {
            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
        }
    }
}
```

**Replace with**:

```rust
fn diff_row_pair(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    ops: &mut Vec<DiffOp>,
) {
    for col in 0..overlap_cols {
        let old_cell = old.get(row_a, col);
        let new_cell = new.get(row_b, col);

        if cells_content_equal(old_cell, new_cell) {
            continue;
        }

        let addr = CellAddress::from_indices(row_b, col);
        let from = snapshot_with_addr(old_cell, addr);
        let to = snapshot_with_addr(new_cell, addr);

        ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
    }
}
```

You should apply the same pattern anywhere else you do “snapshot then compare”, e.g. database-mode column loops and masked positional loops.

---

## Likely next hotspot once the above is fixed: float normalization in equality and hashing

Your float canonicalization:

* calls `log10()` and `powi()` per number normalization. 

Even if it’s “only” a few hundred thousand calls, transcendental math plus exponentiation adds up fast—especially once you stop wasting time on the O(R*M) scans.

### Two practical directions

1. **Cache normalized form once per numeric cell**

* Store `raw: f64` for display/output
* Store `norm: u64` (or `i64`) for equality + hashing
* Equality becomes `self.norm == other.norm`
* Hash becomes `self.norm.hash(state)`

This tends to be the biggest win because it eliminates repeated normalization across:

* row/col hashing
* alignment checks
* cell comparisons

2. **Make normalization cheaper**
   If you want to keep it computed-on-demand:

* avoid `powi()` via a precomputed `POW10: [f64; N]` table
* avoid `log10()` by estimating decimal magnitude from the IEEE-754 exponent and doing a small correction step

I’d do (1) first if you care about “diff identical big workbooks fast”.

---

## A small but very meaningful behavioral optimization

Even after fixing `row_signature_counts`, you’re still doing extra work here conceptually:

* You run AMR alignment, then you potentially do a multiset check and fall back to positional diff. 

For the common case where alignment is already identity (no inserted/deleted/moves and `(row_a == row_b)` for all matches), the multiset check is unnecessary overhead. You can gate it behind “alignment is not trivially identity”.

That reduces redundant passes on the hot “normal” path.

---

## Suggested order of attack

1. **Fix `row_signature_counts` to be single-pass**
   This directly targets the pathological `perf_p5_identical` runtime.

2. **Stop cloning snapshots for unchanged cells**
   This will matter hugely on real spreadsheets (strings/formulas), and also reduces noise in CPU profiles.

3. **Then profile again; optimize float normalization if it rises to the top**
   Because right now it’s probably masked by the O(R*M) rescanning behavior. 

If you want, paste an updated `benchmark_results.txt` after doing (1) + (2) and I’ll help you interpret what moved to the top next.
