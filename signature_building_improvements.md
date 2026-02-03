From the full-scale benchmark run you attached, the biggest lever is to reduce time spent in **signature building / GridView construction**, because it dominates the slowest tests.

### What the current benchmarks are really measuring

In the full-scale results (50K-row suite), the worst offender is:

* `perf_50k_alignment_block_move`: **559 ms total**, of which **468 ms is `signature_build_time_ms`** (~84% of the runtime). 
* `perf_50k_dense_single_edit`: **282 ms total**, with **201 ms `signature_build_time_ms`** (~71%). 

That means: for the “hard” scenarios, you’re mostly paying for **constructing metadata (row/col signatures, counts, first-non-blank info, etc.)**, not for the later diff/move-detection steps.

### Why signature building is expensive in this codebase

`GridView::from_grid_with_config` currently does the following, roughly:

1. **First full pass over all non-empty cells** to compute `row_counts`, `col_counts`, and “first non-blank” positions. 
2. **Second full pass** to populate per-row cell lists (`rows[row].cells.push((col, cell))`). 
3. **Sort each row’s cell list by column** (necessary for sparse grids where iteration order is arbitrary). 
4. **Hash all cells again** to produce row signatures (`build_row_meta` → `hash_row_content_128`). 
5. **Hash all cells again** to produce column signatures (`build_col_meta_*`). 

So for dense sheets, you’re effectively touching the same 5M cells multiple times *plus* doing 50K small sorts.

### The key observation: Dense grids already iterate in row-major order

For dense storage, your iterator is `DenseCellIter`, and it yields `(row, col)` by converting a linear index into `(idx / ncols, idx % ncols)`. That is **row-major, increasing column order inside each row**. 

Also, your `Grid` auto-upgrades from sparse → dense once the fill ratio is high enough (`DENSE_RATIO_THRESHOLD = 0.40`, `DENSE_MIN_CELLS = 4096`). 
The 50K dense benchmarks are overwhelmingly likely to be on `GridStorage::Dense`, so this is directly relevant.

---

## A concrete way to improve the benchmark numbers

### Add a DenseGrid fast-path in GridView construction (single-pass build)

When the backing storage is dense, you can build **everything you need** in **one pass** over `iter_cells()`, and you can skip per-row sorting entirely.

**Goal:** Turn the dense path from “2 passes + per-row sort + 2 extra hash passes” into “1 pass + small O(nrows+ncols) finalize”.

#### What to do in the single pass (dense only)

While iterating each non-empty cell `(row, col, cell)`:

* Push `(col, &cell)` into `rows[row].cells` (already in column order → no sort needed).
* Increment `row_counts[row]` / `col_counts[col]`.
* Set `row_first_non_blank[row]` the first time you see a cell for that row.
* Set `col_first_non_blank[col]` the first time you see a cell for that column.
* Update **row hash** and **col hash** incrementally:

  * `hash_cell_value(&cell.value, &mut row_hasher[row])`
  * `cell.formula.hash(&mut row_hasher[row])`
  * and similarly for `col_hasher[col]`

Then, after the pass:

* Build `row_meta` from `(row_counts, row_first_non_blank, row_hashers.digest128())`
* Build `col_meta` from `(col_counts, col_first_non_blank, col_hashers.digest128())`
* Run your existing `classify_row_frequencies(...)` unchanged.

This keeps the exact same hashing semantics as the current dense behavior because:

* Row signatures are based on hashing cells in column order (which dense iteration already guarantees). 
* Column signatures are based on hashing cells in row order (row-major iteration guarantees that too). 

#### Why this will move the benchmark needle

Because the slowest benchmarks are dominated by signature build time (468 ms of 559 ms in the worst case). 
Reducing the number of full-grid traversals and removing sorting should materially reduce `signature_build_time_ms`, which in turn reduces total time almost one-for-one.

Even a conservative improvement (say, shaving 30–40% off signature build for dense grids) would noticeably improve the full-scale wall-clock numbers, because that’s where the time is currently going.

---

## Optional quick win: run perf gates with the existing parallel feature

Your benchmark JSON indicates `parallel: false`. 
And the perf gate runner (`check_perf_thresholds.py`) invokes `cargo test --features perf-metrics` (no `parallel`). 

You already have a script (`export_perf_metrics.py`) that supports `--parallel` and adds the `parallel` feature (Rayon). 

If your goal is “better benchmark numbers” (lower times) rather than “optimize default single-thread behavior”, adding a `--parallel` option to `check_perf_thresholds.py` + updating the workflows would likely drop the large-grid times on multi-core CI runners. The tradeoff is potentially more variance depending on runner load, so you might need to revisit slack/baselines.

