I reviewed the **updated** snapshot (the one that introduces `grid_metadata.rs`, `alignment_types.rs`, and the new `EmitCtx` emission context), and overall the refactor is moving in the right direction: you’ve made the “shape” of the diff engine much more *inevitable*—shared domain concepts live in one place, and the main diff path reads more like a pipeline than a tangle. 

Below is (1) how the codebase looks now relative to the elegant-simplicity goals, and (2) a **detailed remediation plan** for what still looks incomplete / inconsistent so the refactor feels *finished*.

---

## How the updated codebase looks now

### What’s genuinely improved (and should be preserved)

**1) “Row metadata” is now a first-class concept with a single home.**
`grid_metadata.rs` establishes `RowMeta`, `FrequencyClass`, and the frequency classification utilities as canonical, rather than scattering “row rarity/low-info” logic across alignment and diff code. That’s a major reduction in accidental complexity. 

**2) Alignment outputs are centralized.**
`alignment_types.rs` cleanly defines the data you want to flow out of alignment stages (`RowAlignment`, `RowBlockMove`) without forcing the algorithmic modules to define ad-hoc variants of the same shape. This is exactly the kind of “inevitability” that makes later refactors cheaper. 

**3) Diff emission is materially simpler with `EmitCtx`.**
The introduction of `EmitCtx` (pool/config/formula-cache/sink/op_count bundled together) is a big readability win—especially because it also codifies the “emit increments op_count” invariant in one place. 

**4) Heuristics moved closer to the data they reason about.**
`GridView` now exposes things like `is_low_info_dominated()` / `is_blank_dominated()` which makes the call sites in move detection / fallback alignment easier to read and less error-prone. 

**5) The “legacy row alignment” boundary is now explicit.**
`row_alignment.rs` is clearly framed as legacy/fallback logic and is wired into the main path in a controlled way, rather than pretending it’s the primary architecture. That reduces cognitive load. 

So: the refactor is **real**—it’s not just cosmetic renames.

---

## What still looks missing / not fully “complete” for Elegant Simplicity

Even with the above improvements, there are a few areas where the refactor still feels *half-migrated*:

1. **Database mode diff is still not using `EmitCtx` / `emit_cell_edit`**, so you have duplicate logic for snapshots + formula diff + op emission + op_count updates.
2. The **move detection loop in `diff_grids_core` remains repetitive** (each detector repeats the same “detect → emit → exclude → metrics → iteration” ceremony). It works, but it’s still “code you must execute mentally.” 
3. The masked/projection helpers (`build_masked_grid`, `build_projected_grid_from_maps`) return **tuples** `(Grid, Vec<u32>, Vec<u32>)`, which forces a lot of local unpacking and index-mapping boilerplate. That’s “accidental complexity.”
4. Sink lifecycle (`begin`/`finish`) has been added and used, but **error-path finishing** is still easy to miss in the “engine-level” streaming entrypoints unless callers wrap it carefully. (Package streaming covers it with `NoFinishSink`, but workbook streaming should be robust on its own.)
5. There’s still some **type vocabulary asymmetry**: `RowHash` is effectively a signature newtype, but `ColHash` is still `u128` even though `ColSignature` exists. That’s a small but real “two ways to say the same thing” friction.
6. `alignment/mod.rs` still carries lint silencing (`allow(unused_imports)`) that hints the public surface and internal usage aren’t fully reconciled yet. 

Everything below is aimed at closing these gaps without changing behavior.

---

# Detailed remediation plan to “finish” the elegant simplicity refactor

## 1) Finish the `EmitCtx` migration in database mode diff

### Problem

`diff_grids_database_mode_streaming` still manually does:

* `snapshot_with_addr`
* `compute_formula_diff`
* `emit_op` (or direct sink emission)
* `op_count` accounting

That duplicates the exact logic already centralized in `emit_cell_edit` + `EmitCtx`.

### Goal

Make database-mode cell changes use the same “one true pathway” as grid diff.

### Implementation sketch (drop-in refactor)

Replace the inner matched-row loop with `emit_cell_edit`, and emit row add/remove via `ctx.emit`:

```rust
fn diff_grids_database_mode_streaming(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
    config: &DiffConfig,
    pool: &mut StringPool,
    sink: &mut impl DiffSink,
    op_count: &mut usize,
) -> Result<DiffSummary, DiffError> {
    let sheet_id = pool.intern(DATABASE_MODE_SHEET_ID);

    // Start the sink once.
    sink.begin(pool)?;

    let mut warnings = Vec::new();
    let mut formula_cache = FormulaParseCache::default();

    // Do the alignment attempt.
    let result: Result<(), DiffError> =
        match database_alignment::diff_table_by_key(old, new, key_columns, config) {
            Ok(alignment) => {
                // Keep ctx scoped to this arm so the mutable borrow of sink ends before finish().
                {
                    let mut ctx = EmitCtx {
                        sheet_id: &sheet_id,
                        pool,
                        config,
                        cache: &mut formula_cache,
                        sink,
                        op_count,
                    };

                    for row_a in &alignment.left_only_rows {
                        ctx.emit(DiffOp::row_removed(sheet_id, *row_a, None))?;
                    }
                    for row_b in &alignment.right_only_rows {
                        ctx.emit(DiffOp::row_added(sheet_id, *row_b, None))?;
                    }

                    for (row_a, row_b) in &alignment.matched_rows {
                        for col in 0..new.ncols {
                            if key_columns.contains(&col) {
                                continue;
                            }
                            let old_cell = old.get_cell(*row_a, col);
                            let new_cell = new.get_cell(*row_b, col);
                            if cells_content_equal(old_cell, new_cell) {
                                continue;
                            }

                            let addr = CellAddress::from_indices(*row_b, col);
                            let row_shift = *row_b as i32 - *row_a as i32;
                            emit_cell_edit(&mut ctx, addr, old_cell, new_cell, row_shift, 0)?;
                        }
                    }
                }
                Ok(())
            }
            Err(warn) => {
                warnings.push(warn);
                let mut ctx = DiffContext::default();
                try_diff_grids(
                    &sheet_id, old, new, config, pool, sink, op_count, &mut ctx, None,
                )?;
                warnings.extend(ctx.warnings);
                Ok(())
            }
        };

    // Always attempt to finish the sink.
    let finish = sink.finish();

    // Prefer the “real” error, but still finish on failure.
    match (result, finish) {
        (Ok(()), Ok(())) => Ok(DiffSummary {
            complete: warnings.is_empty(),
            warnings,
            op_count: *op_count,
            metrics: None,
        }),
        (Err(e), _) => {
            let _ = finish;
            Err(e)
        }
        (Ok(()), Err(e)) => Err(e),
    }
}
```

### Why this completes the simplicity story

* One implementation of “cell edit emission” (`emit_cell_edit`) becomes universal.
* Database mode benefits from any later improvements to snapshots/formula diff semantics for free.

### Tests to add/adjust

You likely won’t need to change test expectations (same ops), but I would add one integration test specifically asserting DB-mode uses the same formula-diff behavior as spreadsheet mode for a formula-only change.

---

## 2) Replace tuple-return “masked grids” with a `GridProjection` struct

### Problem

Returning `(Grid, Vec<u32>, Vec<u32>)` forces call sites to remember which vector is which, and leads to code like:

```rust
let (old_proj, _, old_cols) = build_masked_grid(old, old_mask);
```

This is small-but-pervasive mental overhead.

### Fix

Introduce a tiny struct that makes the mapping explicit:

```rust
#[derive(Debug)]
struct GridProjection {
    grid: Grid,
    row_map: Vec<u32>,
    col_map: Vec<u32>,
}

impl GridProjection {
    #[inline]
    fn map_row(&self, local: u32) -> Option<u32> {
        self.row_map.get(local as usize).copied()
    }
    #[inline]
    fn map_col(&self, local: u32) -> Option<u32> {
        self.col_map.get(local as usize).copied()
    }
}
```

Then change:

```rust
fn build_masked_grid(source: &Grid, mask: &RegionMask) -> GridProjection { ... }
fn build_projected_grid_from_maps(source: &Grid, row_map: &[u32], col_map: &[u32]) -> GridProjection { ... }
```

### Example call-site cleanup

Before:

```rust
let (old_proj, _, old_cols) = build_masked_grid(old, old_mask);
let (new_proj, _, new_cols) = build_masked_grid(new, new_mask);
let mv = detect_exact_column_block_move(&old_proj, &new_proj, config)?;
let src_start_col = *old_cols.get(mv.src_start_col as usize)?;
let dst_start_col = *new_cols.get(mv.dst_start_col as usize)?;
```

After:

```rust
let old_p = build_masked_grid(old, old_mask);
let new_p = build_masked_grid(new, new_mask);

let mv_local = detect_exact_column_block_move(&old_p.grid, &new_p.grid, config)?;
let src_start_col = old_p.map_col(mv_local.src_start_col)?;
let dst_start_col = new_p.map_col(mv_local.dst_start_col)?;

Some(ColumnBlockMove { src_start_col, dst_start_col, col_count: mv_local.col_count })
```

### Why this is worth it

This isn’t abstraction for its own sake—it removes a steady drip of tuple unpacking and index mapping noise across all masked detectors.

---

## 3) Encapsulate mask-exclusion rules for each move type

### Problem

Move detection currently repeats “which mask gets which exclusion” at every callsite. That duplication is small but risky because it’s *semantic duplication* (easy to make inconsistent). 

### Fix

Add tiny helpers local to `engine.rs`:

```rust
#[inline]
fn apply_row_move_exclusions(old_mask: &mut RegionMask, new_mask: &mut RegionMask, mv: RowBlockMove) {
    old_mask.exclude_rows(mv.src_start_row, mv.row_count);
    new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
}

#[inline]
fn apply_col_move_exclusions(old_mask: &mut RegionMask, new_mask: &mut RegionMask, mv: ColumnBlockMove) {
    old_mask.exclude_cols(mv.src_start_col, mv.col_count);
    new_mask.exclude_cols(mv.dst_start_col, mv.col_count);
}

#[inline]
fn apply_rect_move_exclusions(old_mask: &mut RegionMask, new_mask: &mut RegionMask, mv: RectBlockMove) {
    old_mask.exclude_rect_cells(mv.src_start_row, mv.src_start_col, mv.src_row_count, mv.src_col_count);
    old_mask.exclude_rect_cells(mv.dst_start_row, mv.dst_start_col, mv.src_row_count, mv.src_col_count);

    new_mask.exclude_rect_cells(mv.src_start_row, mv.src_start_col, mv.src_row_count, mv.src_col_count);
    new_mask.exclude_rect_cells(mv.dst_start_row, mv.dst_start_col, mv.src_row_count, mv.src_col_count);
}
```

Then the move detection loop reads like “business logic” rather than operational detail.

---

## 4) Make move detection orchestration read like a pipeline

### Problem

The current loop is correct, but still repetitive: each detector repeats the same ceremony and uses let-chains that increase “scan cost.” 

### Fix

Represent detection result as a single enum, then handle “emit + exclusions + metrics” in one place:

```rust
enum DetectedMove {
    Rect(RectBlockMove),
    Row(RowBlockMove),       // exact
    Col(ColumnBlockMove),    // exact
    FuzzyRow(RowBlockMove),  // requires per-cell edits
}

fn detect_one_move(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    config: &DiffConfig,
) -> Option<DetectedMove> {
    detect_exact_rect_block_move_masked(old, new, old_mask, new_mask, config).map(DetectedMove::Rect)
        .or_else(|| detect_exact_row_block_move_masked(old, new, old_mask, new_mask, config).map(DetectedMove::Row))
        .or_else(|| detect_exact_column_block_move_masked(old, new, old_mask, new_mask, config).map(DetectedMove::Col))
        .or_else(|| detect_fuzzy_row_block_move_masked(old, new, old_mask, new_mask, config).map(DetectedMove::FuzzyRow))
}

fn apply_move<S: DiffSink>(
    mv: DetectedMove,
    ctx: &mut EmitCtx<'_, S>,
    old_view: &GridView,
    new_view: &GridView,
    old_mask: &mut RegionMask,
    new_mask: &mut RegionMask,
    metrics: Option<&mut PerfMetrics>,
) -> Result<(), DiffError> {
    match mv {
        DetectedMove::Rect(mv) => {
            emit_rect_block_move(ctx, mv)?;
            emit_moved_rect_block_edits(ctx, old_view, new_view, mv)?;
            apply_rect_move_exclusions(old_mask, new_mask, mv);
        }
        DetectedMove::Row(mv) => {
            emit_row_block_move(ctx, mv)?;
            apply_row_move_exclusions(old_mask, new_mask, mv);
        }
        DetectedMove::Col(mv) => {
            emit_col_block_move(ctx, mv)?;
            apply_col_move_exclusions(old_mask, new_mask, mv);
        }
        DetectedMove::FuzzyRow(mv) => {
            emit_row_block_move(ctx, mv)?;
            emit_moved_row_block_edits(ctx, old_view, new_view, mv)?;
            apply_row_move_exclusions(old_mask, new_mask, mv);
        }
    }

    if let Some(m) = metrics {
        m.moves_detected += 1;
    }
    Ok(())
}
```

Then in `diff_grids_core`:

```rust
let mut iteration = 0;
while move_detection_enabled && iteration < config.max_move_iterations {
    let Some(mv) = detect_one_move(old, new, &old_mask, &new_mask, config) else {
        break;
    };
    apply_move(mv, &mut emit_ctx, &old_view, &new_view, &mut old_mask, &mut new_mask, metrics.as_deref_mut())?;
    iteration += 1;

    // Preserve existing behavior: don’t iterate move detection on shape changes.
    if old.nrows != new.nrows || old.ncols != new.ncols {
        break;
    }
}
```

This change keeps behavior but makes the code “tell a story.”

---

## 5) Make sink lifecycle robust on error for workbook-level streaming

### Problem

`try_diff_workbooks_streaming` now correctly calls `sink.begin(pool)` and `sink.finish()` on the success path. 
But if any inner operation returns `Err`, the `?` exits early and **`finish()` isn’t guaranteed** unless the caller does special wrapping (package diff does, but workbook diff should be safe by itself). 

### Fix pattern

Wrap the body so `finish()` happens in both success and error paths:

```rust
pub fn try_diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    pool: &mut StringPool,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError> {
    sink.begin(pool)?;

    let mut op_count = 0usize;
    let mut ctx = DiffContext::default();

    let result: Result<DiffSummary, DiffError> = (|| {
        // existing implementation body…
        Ok(DiffSummary {
            complete: ctx.warnings.is_empty(),
            warnings: std::mem::take(&mut ctx.warnings),
            op_count,
            metrics: None,
        })
    })();

    let finish = sink.finish();

    match (result, finish) {
        (Ok(summary), Ok(())) => Ok(summary),
        (Err(e), _) => {
            let _ = finish;
            Err(e)
        }
        (Ok(_), Err(e)) => Err(e),
    }
}
```

### Test to add

Add a test like `workbook_streaming_finishes_on_error` using a sink that fails on the Nth emit and records whether `finish()` was called (mirror of the package-level finish tests).

---

## 6) Optional but “completes the vocabulary”: make `ColHash` a signature newtype too

### Problem

You already have `ColSignature`, but `ColHash` is still `u128` in `grid_view.rs` and column alignment. That asymmetry adds unnecessary vocabulary and slightly undermines type-driven clarity.

### Recommended end-state

Mirror rows:

```rust
pub type RowHash = RowSignature;
pub type ColHash = ColSignature;
```

Then wrap digest outputs:

```rust
let hash = ColSignature { hash: col_hashers[idx].digest128() };
```

And wrap unordered hash functions:

```rust
let h: ColHash = ColSignature { hash: hash_col_content_unordered_128(cells) };
```

### Scope note

This is the most “cross-cutting” of the remaining items (it touches tests and several modules). If you want the refactor to end with a crisp vocabulary, it’s worth doing; if you’re trying to minimize churn, you can defer it.

---

## 7) Remove lint-silencing and make module exports intentional

`alignment/mod.rs` currently has `allow(unused_imports)` around re-exports. 
Finishing the refactor means either:

* those exports are intentionally part of the crate’s internal API (then they should be used and not silenced), or
* they aren’t (then they should be removed).

Concrete plan:

1. Remove `#![allow(unused_imports)]`.
2. Run `cargo test` and `cargo clippy`.
3. For any unused re-exports, either:

   * remove the re-export, or
   * move it behind `#[cfg(test)]` if it’s test-only convenience.

---

# What I would do first (priority order)

1. **Database mode → `EmitCtx` migration** (biggest simplicity win per line changed).
2. **GridProjection struct** (shrinks a lot of small complexity everywhere).
3. **Move detection orchestration refactor** (makes `diff_grids_core` read like a spec).
4. **Workbook streaming finish-on-error** (stability + simplicity for callers).
5. **ColSignature unification** (optional, but “finishes the vocabulary”).
6. **Remove lint silencing** (final polish).

