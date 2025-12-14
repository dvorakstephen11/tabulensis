Here’s what I’m seeing in the updated `codebase_context.md` + the latest full-scale benchmark runs, and the concrete optimization targets + a detailed plan to hit them.

## Snapshot of the current state

### Tests

Your latest cycle summary shows the unit/integration suite passing (with the perf tests still ignored). 

### Performance: baseline vs latest

Using the full-scale benchmark JSONs:

**Branch 2 baseline (commit `8bec00c35df1`)**

* Total: **25,357 ms** 
* Key tests:

  * `perf_50k_adversarial_repetitive`: **7,016 ms**, move-detect **398 ms** 
  * `perf_50k_dense_single_edit`: **8,070 ms**, move-detect **1,596 ms** 
  * `perf_50k_completely_different`: **7,744 ms**, move-detect **0 ms** 
  * `perf_50k_identical`: **2,076 ms**, move-detect **0 ms** 

**Latest run (commit `d43dd59a9ecb`, `2025-12-14_005407_fullscale.json`)**

* Total: **37,370 ms** 
* Key tests:

  * `perf_50k_adversarial_repetitive`: **11,583 ms**, move-detect **5,014 ms** 
  * `perf_50k_dense_single_edit`: **11,499 ms**, move-detect **9,628 ms** 
  * `perf_50k_completely_different`: **13,005 ms**, move-detect **9,004 ms** 
  * `perf_50k_identical`: **1,100 ms** (improved) 
  * `perf_50k_99_percent_blank`: **183 ms** + massive reduction in `cells_compared` (also improved) 

So: **you gained a lot on sparse/blank sheets and identical sheets**, but **you regressed hard on dense sheets** and the regression is overwhelmingly inside the **Move Detection phase**.

## Most likely root cause (and why it matches the numbers)

Your Branch 3 masked/iterative move detection loop calls masked move detectors that *build projected grids* by iterating through **all cells** and cloning/inserting them into a new grid. This is exactly the kind of work that explodes on dense 50k×100 sheets.

Concretely:

* The move detection loop repeatedly calls:

  * `detect_exact_rect_block_move_masked(...)`
  * `detect_exact_row_block_move_masked(...)`
  * `detect_exact_column_block_move_masked(...)`
  * `detect_fuzzy_row_block_move_masked(...)`

* And those masked row/col/fuzzy move detectors always do:

  * `let (old_proj, ...) = build_masked_grid(old, old_mask);`
  * `let (new_proj, ...) = build_masked_grid(new, new_mask);`

* `build_masked_grid` and `build_projected_grid_from_maps` iterate over `source.cells.values()` and clone/insert into the projected grid.

In your “no-move” benchmarks (dense single edit, completely different, adversarial repetitive), the masks start “fully active” and usually stay that way because no move is found. So you’re paying the cost of building these projected grids **even though there are no exclusions** (and thus no reason to project at all).

That maps directly to the observed “MoveDetection jumped from ~0.4–1.6s to 5–9.6s” regressions.

## Optimization targets

I’d set targets that preserve your wins on sparse/identical while eliminating the dense-sheet regression.

### Primary targets (fullscale)

1. **Total full-scale time**: bring **37,370 ms → ≤ 28,000 ms** (close to your earlier best Branch 3 runs and within striking distance of baseline).

   * Reference: earlier Branch 3 run was **26,783 ms**. 

2. **MoveDetection phase caps (dense scenarios)**:

   * `perf_50k_dense_single_edit`: move-detect **9,628 ms → ≤ 1,000 ms** 
   * `perf_50k_completely_different`: move-detect **9,004 ms → ≤ 500 ms** 
   * `perf_50k_adversarial_repetitive`: move-detect **5,014 ms → ≤ 1,000 ms** 

3. **Keep (don’t regress)**

   * `perf_50k_99_percent_blank`: **≤ 300 ms** (currently 183 ms) 
   * `perf_50k_identical`: **≤ 1,200 ms** (currently 1,100 ms) 

## Implementation plan to hit targets

### 1) Add a “no-exclusions” fast path to every masked move detector

This is the highest ROI change. When `RegionMask` has no exclusions, **don’t project**. Just call the unmasked move detector directly.

`RegionMask` already exposes `has_exclusions()` (and the exclude operations).

#### 1A. `detect_exact_row_block_move_masked`

**Before**: always builds projected grids via `build_masked_grid`.

**After (patch sketch)**:

```rust
fn detect_exact_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<LegacyRowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    // ✅ Fast path: no exclusions => no projection
    if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
        let mv = detect_exact_row_block_move_with_config(old, new, config)?;
        return Some(LegacyRowBlockMove {
            src_start_row: mv.src_start_row,
            dst_start_row: mv.dst_start_row,
            row_count: mv.row_count,
        });
    }

    // Existing slow path (projection) for masked cases
    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_row_block_move_with_config(&old_proj, &new_proj, config)?;
    Some(LegacyRowBlockMove {
        src_start_row: *old_rows.get(mv_local.src_start_row as usize)?,
        dst_start_row: *new_rows.get(mv_local.dst_start_row as usize)?,
        row_count: mv_local.row_count,
    })
}
```

This alone should collapse the dense-sheet move-detection cost in the “no move” benchmarks (because masks are full and remain full).

#### 1B. `detect_exact_column_block_move_masked`

Same pattern (fast path → call `detect_exact_column_block_move_with_config(old, new, config)` directly), then fall back to projection only when needed.

#### 1C. `detect_fuzzy_row_block_move_masked`

Right now it always builds projected grids.
Add the same fast path:

```rust
if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
    let mv = detect_fuzzy_row_block_move_with_config(old, new, config)?;
    return Some(LegacyRowBlockMove { ... });
}
```

This is especially important because fuzzy move detection is currently in the loop in some variants even without checking `config.enable_fuzzy_moves`. (See section 2 below.)

#### 1D. `detect_exact_rect_block_move_masked`

This one is a big offender because it can align rows/cols and then build projected grids from maps.

Add:

```rust
if !old_mask.has_exclusions() && !new_mask.has_exclusions() {
    // Only attempt if dims match; otherwise keep masked logic.
    if old.nrows == new.nrows && old.ncols == new.ncols {
        return detect_exact_rect_block_move(old, new, config);
    }
}
```

Then continue with your existing masked logic for true masked cases (where exclusions exist, or dims differ and you want signature-based subprojection).

### 2) Ensure fuzzy moves are actually gated by config (and consolidate duplicate loop variants)

In `codebase_context.md` there are **two variants** of the move detection loop:

* One calls `detect_fuzzy_row_block_move_masked` unconditionally as the last step. 
* Another version gates it with `config.enable_fuzzy_moves`. 

Make sure the real code has only one loop and that it matches:

```rust
if !found_move
   && config.enable_fuzzy_moves
   && let Some(mv) = detect_fuzzy_row_block_move_masked(...)
{
   ...
}
```

This matters both for **performance** and for **config correctness**.

### 3) Stop rect-move detection from doing catastrophic work on “completely different”

Your unmasked rect-move detector currently does:

* Build grid views
* Compute row/col stats
* Then call `collect_differences(old, new)` which iterates **every cell** and pushes every mismatch position into a vector. 

On “completely different”, that mismatch vector becomes enormous.

Add an early “overlap sanity check” **before** `collect_differences`:

```rust
let shared_rows = row_stats
    .freq_a
    .keys()
    .filter(|h| row_stats.freq_b.contains_key(*h))
    .count();

let shared_cols = col_stats
    .freq_a
    .keys()
    .filter(|h| col_stats.freq_b.contains_key(*h))
    .count();

// If there is zero hash overlap, a clean rectangle swap is impossible.
if shared_rows == 0 || shared_cols == 0 {
    return None;
}
```

Where to put it: right after `row_stats` / `col_stats` and heavy repetition guards, immediately before `collect_differences`. 

This should dramatically reduce the “completely different” case without harming true rect-move cases (real rect moves should leave *lots* of row/col hashes unchanged outside the moved block).

### 4) Reduce or eliminate projections even when exclusions exist (second wave, if needed)

If after steps 1–3 you’re still above targets, then the next biggest win is: **stop creating projected `Grid`s at all** for move detection. Instead, operate on `GridView` metadata + row/col index maps.

A workable intermediate step (low risk, still uses projection but much less often):

#### 4A. Build projections once per iteration (and reuse)

Right now, in one iteration when no move is found, you can end up building projected grids up to:

* rect (2 projections)
* row (2)
* col (2)
* fuzzy row (2)

Even if steps 1A–1D fix the “no exclusions” case, when exclusions exist you can still thrash.

Implement an iteration-local cache:

```rust
struct MaskedProjections {
    old: Grid,
    new: Grid,
    old_rows: Vec<u32>,
    new_rows: Vec<u32>,
    old_cols: Vec<u32>,
    new_cols: Vec<u32>,
}

fn get_or_build_projections(
   cache: &mut Option<MaskedProjections>,
   old: &Grid, new: &Grid,
   old_mask: &RegionMask, new_mask: &RegionMask
) -> &MaskedProjections {
   cache.get_or_insert_with(|| {
       let (old_proj, old_rows, old_cols) = build_masked_grid(old, old_mask);
       let (new_proj, new_rows, new_cols) = build_masked_grid(new, new_mask);
       MaskedProjections { old: old_proj, new: new_proj, old_rows, new_rows, old_cols, new_cols }
   })
}
```

Then have row/col/fuzzy masked detectors accept `&MaskedProjections` instead of rebuilding internally.

Rect move masked is special (it does signature-based row/col alignment), so you may keep it separate for now.

#### 4B. Long-term: make move detectors accept `GridView` (no projections at all)

You already have `GridView::from_grid_with_config` used in row/col/fuzzy detectors.
Refactor:

* `detect_exact_row_block_move_with_config(old: &Grid, new: &Grid, ...)`
* into:

  * `detect_exact_row_block_move_from_meta(meta_a: &[RowMeta], meta_b: &[RowMeta], stats: &HashStats, ...)`

Then your masked version can:

* Filter `meta_a/meta_b` by mask active rows
* Map back to original row indices
* No cloning, no projection grids.

This is more code churn, but it’s the correct “endgame” for stable performance.

## How to validate improvements

1. Re-run the same fullscale benchmark harness that produced:

   * `2025-12-14_005407_fullscale.json` 
     and compare to baseline `2025-12-13_155200_fullscale.json`. 

2. Confirm:

   * MoveDetection for the three dense cases drops from 5–9s to ≤1s.
   * Blank + identical remain at least as good as they are now.

3. Add a regression “structure” test (not time-based):

   * Under `#[cfg(test)]`, add an atomic counter increment in `build_masked_grid` / `build_projected_grid_from_maps`, and assert for a “no move expected” diff that the counter stays at 0 when masks have no exclusions.
     This prevents the exact regression you’re seeing from returning.
