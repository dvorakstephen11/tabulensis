Section 3 (“Parallelization: the cheapest high‑impact targets”) calls for three concrete parallelization wins:

1. parallel **row/col signature** computation,
2. parallel **cell diff** work within aligned row pairs (while keeping deterministic output ordering), and
3. optionally, parallel **scoring** inside move detection. 

Based on the current `codebase_context.md` and the two benchmark JSONs, Section 3 is **mostly** implemented and looks **correct where implemented**, but it is **not fully complete** because **column signature computation is still single‑threaded** in the main signature build path.

## What’s implemented correctly

### Parallel feature plumbing + WASM safety

* The core crate has a `parallel` feature that enables `rayon`. 
* The code explicitly rejects compiling `parallel` on `wasm32` (so WASM remains single‑threaded as intended). 
* CI runs tests both normally and with `--features parallel`. 

### 3.1 Parallel row signature computation (partial: rows yes, cols no)

Row-side work is parallelized (with thresholds):

* Row cell sorting uses `rows.par_iter_mut()` when the grid is large enough. 
* Row metadata/signatures are computed in parallel via `rows.par_iter().enumerate()...collect()`. 

Benchmarks corroborate that this is “real” and beneficial: for example, on `perf_50k_dense_single_edit`, total time drops **1212ms → 980ms**, and signature build time drops **1061ms → 798ms** in the parallel run.

### 3.2 Parallel cell diff within aligned row pairs

The engine parallelizes *planning* per row pair chunk and then emits sequentially (preserving deterministic ordering):

* `plan_row_pair_chunk(...)` switches to `chunk.par_iter()` under the parallel feature and a size threshold. 

This design choice (parallel plan + sequential emission) is exactly the “careful ordering” approach Section 3 is hinting at, and it’s a correctness‑preserving way to introduce parallelism without reordering ops.

### 3.3 Optional parallel scoring inside move detection

Move candidate scoring is parallelized under the `parallel` feature:

* `score_candidates(...)` uses `seeds.par_iter()` to compute similarities. 

Benchmarks also show meaningful improvement in the move-heavy case: `perf_50k_alignment_block_move` total time **9313ms → 7767ms**, and move detection time **8626ms → 7225ms** with parallel enabled.

### Determinism guardrail exists (but coverage is narrow)

There’s a parallel determinism test that compares output ops across thread counts. 
That’s the right kind of test to have, but it currently covers only one dense “single edit” scenario.

## What’s missing (and why Section 3 is not “complete”)

### Column signature computation is still sequential in the signature-build hot path

Even with `parallel` enabled, column hashing is performed by mutating `col_hashers[idx]` while scanning rows sequentially. 

Because Section 3 explicitly calls out “parallel row/col signature computation”, this is a gap. 
It’s also consistent with the benchmarks: even after parallel improvements, signature building remains the dominant cost in several tests (e.g., 798ms of a 980ms run in `perf_50k_dense_single_edit`). 

## Remediation plan to make Section 3 fully implemented

### Goal

Parallelize **column signature computation** in the same “deterministic and safe” style already used elsewhere: parallelize the expensive work while keeping output stable.

### Plan overview

1. Implement `build_col_meta(...)` in `core/src/grid_view.rs` with a parallel path under `feature = "parallel"`.
2. Replace the current sequential `col_hashers` loop in `GridView::from_grid_with_config` with a call to `build_col_meta`.
3. Extend determinism testing to cover at least one move-heavy scenario (recommended).

---

## 1) `core/src/grid_view.rs`: replace the sequential column hashing block

### Code to replace (inside `GridView::from_grid_with_config`)

```rust
        let mut col_hashers: Vec<Xxh3> = (0..ncols).map(|_| Xxh3::new()).collect();
        for row_view in &rows {
            for (col, cell) in &row_view.cells {
                let idx = *col as usize;
                if idx >= col_hashers.len() {
                    continue;
                }
                let hasher = &mut col_hashers[idx];
                hash_cell_value(&cell.value, hasher);
                cell.formula.hash(hasher);
            }
        }

        let col_meta: Vec<ColMeta> = (0..ncols)
            .map(|col_idx| ColMeta {
                col_idx: col_idx as u32,
                hash: ColSignature {
                    hash: col_hashers[col_idx].digest128(),
                },
                non_blank_count: to_u16(col_counts.get(col_idx).copied().unwrap_or(0)),
                first_non_blank_row: col_first_non_blank
                    .get(col_idx)
                    .and_then(|r| r.map(to_u16))
                    .unwrap_or(0),
            })
            .collect();
```

### New code to replace it

```rust
        let col_meta = build_col_meta(&rows, &col_counts, &col_first_non_blank, total_cells);
```

---

## 2) `core/src/grid_view.rs`: extend the helper section to include `build_col_meta`

This replaces the existing “parallel helpers” block (the one that currently defines `PAR_MIN_ROWS`, `PAR_MIN_CELLS`, `should_parallelize_rows`, `sort_row_cells`, and `build_row_meta`) with an extended version that also supports columns.

### Code to replace

```rust
#[cfg(feature = "parallel")]
const PAR_MIN_ROWS: usize = 2048;
#[cfg(feature = "parallel")]
const PAR_MIN_CELLS: usize = 200_000;

#[cfg(feature = "parallel")]
fn should_parallelize_rows(row_len: usize, total_cells: usize) -> bool {
    row_len >= PAR_MIN_ROWS && total_cells >= PAR_MIN_CELLS
}

#[cfg(feature = "parallel")]
fn sort_row_cells(rows: &mut [RowView<'_>], total_cells: usize) {
    if should_parallelize_rows(rows.len(), total_cells) {
        use rayon::prelude::*;
        rows.par_iter_mut()
            .for_each(|r| r.cells.sort_unstable_by_key(|(c, _)| *c));
        return;
    }

    for r in rows.iter_mut() {
        r.cells.sort_unstable_by_key(|(c, _)| *c);
    }
}

#[cfg(not(feature = "parallel"))]
fn sort_row_cells(rows: &mut [RowView<'_>], _total_cells: usize) {
    for r in rows.iter_mut() {
        r.cells.sort_unstable_by_key(|(c, _)| *c);
    }
}

#[cfg(feature = "parallel")]
fn build_row_meta<'a>(
    rows: &[RowView<'a>],
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
    _config: &DiffConfig,
    total_cells: usize,
) -> Vec<RowMeta> {
    if should_parallelize_rows(rows.len(), total_cells) {
        use rayon::prelude::*;
        return rows
            .par_iter()
            .enumerate()
            .map(|(idx, row_view)| {
                row_meta_for_row(idx, row_view, row_counts, row_first_non_blank)
            })
            .collect();
    }

    rows.iter()
        .enumerate()
        .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
        .collect()
}

#[cfg(not(feature = "parallel"))]
fn build_row_meta<'a>(
    rows: &[RowView<'a>],
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
    _config: &DiffConfig,
    _total_cells: usize,
) -> Vec<RowMeta> {
    rows.iter()
        .enumerate()
        .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
        .collect()
}
```

### New code to replace it

```rust
#[cfg(feature = "parallel")]
const PAR_MIN_ROWS: usize = 2048;
#[cfg(feature = "parallel")]
const PAR_MIN_CELLS: usize = 200_000;
#[cfg(feature = "parallel")]
const PAR_MIN_COLS: usize = 8;

#[cfg(feature = "parallel")]
fn should_parallelize_rows(row_len: usize, total_cells: usize) -> bool {
    row_len >= PAR_MIN_ROWS && total_cells >= PAR_MIN_CELLS
}

#[cfg(feature = "parallel")]
fn should_parallelize_cols(col_len: usize, total_cells: usize) -> bool {
    col_len >= PAR_MIN_COLS && total_cells >= PAR_MIN_CELLS
}

#[cfg(feature = "parallel")]
fn sort_row_cells(rows: &mut [RowView<'_>], total_cells: usize) {
    if should_parallelize_rows(rows.len(), total_cells) {
        use rayon::prelude::*;
        rows.par_iter_mut()
            .for_each(|r| r.cells.sort_unstable_by_key(|(c, _)| *c));
        return;
    }

    for r in rows.iter_mut() {
        r.cells.sort_unstable_by_key(|(c, _)| *c);
    }
}

#[cfg(not(feature = "parallel"))]
fn sort_row_cells(rows: &mut [RowView<'_>], _total_cells: usize) {
    for r in rows.iter_mut() {
        r.cells.sort_unstable_by_key(|(c, _)| *c);
    }
}

#[cfg(feature = "parallel")]
fn build_row_meta<'a>(
    rows: &[RowView<'a>],
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
    _config: &DiffConfig,
    total_cells: usize,
) -> Vec<RowMeta> {
    if should_parallelize_rows(rows.len(), total_cells) {
        use rayon::prelude::*;
        return rows
            .par_iter()
            .enumerate()
            .map(|(idx, row_view)| {
                row_meta_for_row(idx, row_view, row_counts, row_first_non_blank)
            })
            .collect();
    }

    rows.iter()
        .enumerate()
        .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
        .collect()
}

#[cfg(not(feature = "parallel"))]
fn build_row_meta<'a>(
    rows: &[RowView<'a>],
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
    _config: &DiffConfig,
    _total_cells: usize,
) -> Vec<RowMeta> {
    rows.iter()
        .enumerate()
        .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
        .collect()
}

fn build_col_meta_sequential<'a>(
    rows: &[RowView<'a>],
    col_counts: &[u32],
    col_first_non_blank: &[Option<u32>],
) -> Vec<ColMeta> {
    let ncols = col_counts.len();
    let mut col_hashers: Vec<Xxh3> = (0..ncols).map(|_| Xxh3::new()).collect();

    for row_view in rows {
        for (col, cell) in &row_view.cells {
            let idx = *col as usize;
            if idx >= ncols {
                continue;
            }
            let hasher = &mut col_hashers[idx];
            hash_cell_value(&cell.value, hasher);
            cell.formula.hash(hasher);
        }
    }

    (0..ncols)
        .map(|col_idx| ColMeta {
            col_idx: col_idx as u32,
            hash: ColSignature {
                hash: col_hashers[col_idx].digest128(),
            },
            non_blank_count: to_u16(col_counts[col_idx]),
            first_non_blank_row: col_first_non_blank[col_idx].map(to_u16).unwrap_or(0),
        })
        .collect()
}

#[cfg(feature = "parallel")]
fn build_col_meta<'a>(
    rows: &[RowView<'a>],
    col_counts: &[u32],
    col_first_non_blank: &[Option<u32>],
    total_cells: usize,
) -> Vec<ColMeta> {
    let ncols = col_counts.len();
    if !should_parallelize_cols(ncols, total_cells) {
        return build_col_meta_sequential(rows, col_counts, col_first_non_blank);
    }

    let mut col_cells: Vec<Vec<&'a Cell>> = (0..ncols)
        .map(|i| Vec::with_capacity(col_counts[i] as usize))
        .collect();

    for row_view in rows {
        for (col, cell) in &row_view.cells {
            let idx = *col as usize;
            if idx < ncols {
                col_cells[idx].push(*cell);
            }
        }
    }

    use rayon::prelude::*;
    let mut out: Vec<ColMeta> = col_cells
        .par_iter()
        .enumerate()
        .map(|(col_idx, cells)| {
            let mut hasher = Xxh3::new();
            for &cell in cells {
                hash_cell_value(&cell.value, &mut hasher);
                cell.formula.hash(&mut hasher);
            }
            ColMeta {
                col_idx: col_idx as u32,
                hash: ColSignature {
                    hash: hasher.digest128(),
                },
                non_blank_count: to_u16(col_counts[col_idx]),
                first_non_blank_row: col_first_non_blank[col_idx].map(to_u16).unwrap_or(0),
            }
        })
        .collect();

    out.sort_unstable_by_key(|m| m.col_idx);
    out
}

#[cfg(not(feature = "parallel"))]
fn build_col_meta<'a>(
    rows: &[RowView<'a>],
    col_counts: &[u32],
    col_first_non_blank: &[Option<u32>],
    _total_cells: usize,
) -> Vec<ColMeta> {
    build_col_meta_sequential(rows, col_counts, col_first_non_blank)
}
```

Why this works:

* It keeps the **exact same per-column hash semantics** as the current sequential code by ensuring each column hashes its cells in **row order**.
* It parallelizes over columns without requiring shared mutable hashers (so no locking, no atomics).
* Determinism is preserved because `col_idx` is explicit and the final vector is sorted by `col_idx`.

---

## 3) Recommended: expand determinism testing to cover move detection

You already have one thread-count determinism test. 
To be confident Section 3 is “correct” (not just fast), add one more that forces the move path.

### Remediation idea (high value, low effort)

Create a smaller “block move” grid fixture (so the test is fast) and assert ops match between 1 thread and 4 threads, similar to the existing test harness pattern.

I’m not pasting a full file rewrite here unless you want it, but the pattern should mirror the existing `ops_are_identical_across_thread_counts` structure: build `Grid` A/B, run in two pools, compare `report.ops`.

---

## How to validate Section 3 is complete after these changes

1. Run the existing parallel test suite path (same as CI): `cargo test -p excel_diff --features parallel`. 
2. Run fullscale benchmarks again and compare `signature_build_time_ms` and `total_time_ms` against the current baselines.
3. Confirm determinism across thread counts for at least:

   * a dense single-edit case (already covered), and
   * a move-heavy case (recommended add).

---

## Final verdict

* Section 3 is **implemented correctly** for:

  * parallel row signature building, 
  * parallel row-pair diff planning, 
  * and parallel move scoring. 
* Section 3 is **not implemented completely**, because **column signature computation is still sequential** in the hot signature-build path.

If you apply the `build_col_meta` remediation above, Section 3’s “row/col signature computation” requirement will be satisfied, and you should see additional wins specifically in `signature_build_time_ms`-dominated benchmarks.
