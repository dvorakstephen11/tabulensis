Here’s a concrete refactor sketch that implements the performance win we just discussed: **stop materializing “masked projected grids” during move detection** when the mask is effectively *row-only* (or *col-only*), and **cache any projection you truly need so you only build it once per iteration**.

This directly targets the exact pathology you’re seeing in the perf logs: in `perf_50k_alignment_block_move`, move detection dominates at ~5.3s and the counters show ~5,000,000 hash lookups during that phase.  That’s extremely consistent with “(roughly) 2.5M cells inserted into a hash-backed grid = about 2 hash lookups per insert”, i.e., building projected grids inside the loop.

## Root cause in the current structure

`SheetGridDiffer::detect_moves()` loops and, once there are exclusions, it calls the “masked” detection helpers which **build projected grids** via `build_masked_grid(...)` (and then run the exact/fuzzy move detector over those temporary grids).  

That’s fine if exclusions are arbitrary rectangles or mixed row+col masks, but for the block-move perf case the hot path is:

1. Iteration 1: exact row move is detected (no exclusions yet) using the already-built `GridView` metadata → cheap. 
2. The move is emitted, then the moved rows are excluded (`exclude_rows`). 
3. Iteration 2: because `has_exclusions()` is now true, the code goes down the “masked” path and builds one or more projected grids by iterating *millions of cells* (`iter_cells` + `insert_cell`) even though the columns are unchanged and we only excluded full rows. 
4. It finds no further move and breaks. 

That step (3) is where the 5M hash lookups and multi-second move detection time are coming from. 

## Refactor goal

Make move detection under masks **signature-first**, not “rebuild-grid-first”:

* If the mask exclusions are **rows-only** (no excluded cols, no excluded rects), then the projected row signatures are literally a subsequence of `old_view.row_meta` / `new_view.row_meta`. We can run the move detector on that subsequence **without building a grid**.
* Same idea for **cols-only**.
* Only fall back to building a projected grid when the mask is **mixed** (rect exclusions and/or both axes affected) because in that case the projected signatures are genuinely different.

Second, **cache any projection that you do need** so you don’t rebuild it 3–4 times in the same iteration for exact-row / exact-col / rect / fuzzy-row checks.

## Sketch of the refactor

### 1) Add mask “shape” introspection

Add a small classifier that answers: “Is this mask rows-only, cols-only, or mixed?”

You already have `RegionMask::has_exclusions()` and tests around excluded rects. 
You can extend `RegionMask` with a cheap shape query:

* `has_excluded_rows()`
* `has_excluded_cols()`
* `has_excluded_rects()` (already exists per tests)
* `exclusion_shape() -> ExclusionShape`

Where:

```rust
enum ExclusionShape {
    None,
    RowsOnly,
    ColsOnly,
    Mixed,
}
```

“Mixed” means anything that makes projected signatures differ from the original axis meta (rect exclusions, or both rows and cols excluded).

### 2) Extract “meta-slice” move detection helpers

Right now the exact/fuzzy row move detectors are implemented as “from_views” functions that operate on `old_view.row_meta` and `new_view.row_meta` after some guards.  

Refactor so the core algorithm can run on *any* `&[RowMeta]` sequence (full grid or masked subsequence):

* `detect_exact_row_block_move_from_meta(meta_a: &[RowMeta], meta_b: &[RowMeta], config: &DiffConfig) -> Option<RowBlockMove>`
* `detect_fuzzy_row_block_move_from_meta(meta_a: &[RowMeta], meta_b: &[RowMeta], config: &DiffConfig) -> Option<RowBlockMove>`

Then `detect_exact_row_block_move_from_views` becomes a thin wrapper:

* checks size bounds / low-info dominated (as today)
* calls `detect_exact_row_block_move_from_meta(&old_view.row_meta, &new_view.row_meta, config)`

Same pattern for fuzzy.

This is a pure mechanical extraction: it makes the move detector reusable for the masked-subsequence case.

### 3) Introduce a per-iteration “masked signature view” cache in `detect_moves`

Inside `SheetGridDiffer::detect_moves`, build a small per-iteration cache object that holds the active axis index lists **once**, plus lazily-computed projections only if necessary.

Conceptually:

```rust
struct MaskedAxisCache {
    shape: ExclusionShape,
    old_rows: Vec<u32>,
    new_rows: Vec<u32>,
    old_cols: Vec<u32>,
    new_cols: Vec<u32>,
}

impl MaskedAxisCache {
    fn new(old_mask: &RegionMask, new_mask: &RegionMask) -> Self;
}
```

Then add lightweight helpers that produce *meta subsequences* without grid projection:

* `fn row_meta_slices(&self, old_view: &GridView, new_view: &GridView) -> (Vec<RowMeta>, Vec<RowMeta>)`
* `fn col_meta_slices(&self, old_view: &GridView, new_view: &GridView) -> (Vec<ColMeta>, Vec<ColMeta>)`

Implementation choices:

* easiest: clone the `RowMeta` values for active indices (50k clones is fine; it’s tiny compared to building a 2.5M cell projected grid)
* more advanced: keep an index list and implement “meta accessor by projected index” without cloning

For the perf fix, cloning is perfectly acceptable.

### 4) Replace the masked row/col move calls with “rows-only/cols-only” fast paths

Right now `detect_exact_row_block_move_masked(...)` builds projected grids unless there are no exclusions. 

Change the call site in `detect_moves()`:

* If `shape == None`: keep current `detect_exact_row_block_move_from_views(&old_view, &new_view, config)`

* If `shape == RowsOnly`: run:

  * build `(meta_a, meta_b)` from `row_meta_slices`
  * call `detect_exact_row_block_move_from_meta(&meta_a, &meta_b, config)`
  * (returns `RowBlockMove` with original `row_idx` values because the underlying meta includes original `row_idx`, as today)

* If `shape == Mixed`: fall back to the current grid-projection approach (but via the cache described below)

Same for fuzzy row move and for column move (ColsOnly).

### 5) Cache the expensive projected grid build when you truly need it

When `shape == Mixed`, you may still need a projected grid to compute signatures “under excluded columns/rects”.

Refactor `build_masked_grid` usage into a single lazy construction per iteration, shared by all detectors that need it:

```rust
struct MaskedProjectionCache<'b> {
    built: bool,
    old_proj: Option<Grid>,
    new_proj: Option<Grid>,
    old_view: Option<GridView<'b>>,
    new_view: Option<GridView<'b>>,
    old_row_map: Vec<u32>,
    new_row_map: Vec<u32>,
    old_col_map: Vec<u32>,
    new_col_map: Vec<u32>,
}
```

Then in the iteration:

* exact row move needs projection? ask cache for projected views
* exact col move needs projection? reuse the same projected views
* fuzzy row move needs projection? reuse again
* rect move detection already has a masked-from-views helper; keep it, but if it needs scoped projections internally, that’s separate 

This alone prevents 3–4 full projected grid builds in the same iteration.

### 6) Add the “remaining region is aligned” early exit

After you successfully emit a move and update the masks, you often don’t need another iteration.

Add a cheap early-break check that runs **before** attempting any more move detection.

For the common case after an exact row move:

* exclusions are rows-only
* columns are unchanged
* if the active rows in old and new match *by row signature in order*, then the remaining active region is identical and there cannot be another move or cell diff to emit.

This is a simple O(active_rows) check using existing `GridView.row_meta` signatures (no projection required). It avoids the “second iteration does expensive masked work just to discover nothing” pattern that’s killing the benchmark.  

You can do the symmetric check for cols-only after a column move.

### 7) Where the refactor lives (files)

A clean file-level plan:

* `core/src/row_alignment.rs`

  * extract `detect_exact_row_block_move_from_meta`
  * extract `detect_fuzzy_row_block_move_from_meta`
  * keep existing `*_from_views` wrappers, now delegating

* `core/src/column_alignment.rs`

  * same extraction pattern for column moves if you want symmetry (it will pay off for column-move-heavy cases)

* `core/src/region_mask.rs`

  * add the exclusion-shape/introspection APIs

* `core/src/engine/move_mask.rs`

  * introduce `MaskedAxisCache` and (optional) `MaskedProjectionCache`
  * modify `detect_moves()` to:

    * compute cache once per iteration
    * take rows-only/cols-only fast paths
    * only build projections for Mixed masks
    * add early-exit aligned check

## Why this refactor should move the needle on your perf case

In `perf_50k_alignment_block_move`, you’re effectively paying for **projecting a ~2.5M cell grid** after the first move just to learn “no more moves”. The metrics show move detection is the bottleneck and hash lookups are enormous in that phase. 

After this refactor:

* Iteration 2 would do:

  * build `old_rows`/`new_rows` lists (~50k)
  * compare row signatures (~50k)
  * break immediately

That turns the “second iteration” from **O(cells)** to **O(rows)** and eliminates the huge hash-map insertion workload from `build_masked_grid` in this scenario. 

