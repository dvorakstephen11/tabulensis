Here’s a concrete plan to finish Branch 1, plus code for the hardest parts.

---

## 1. What’s left in Branch 1

Given the current code and the Cursor log:

* **1.3–1.5 are done**

  * Row hashes are content‑only, 128‑bit xxHash3.
  * Float normalization for hashing is in place and well‑tested.

* **1.1 “Fix silent data loss” is functionally fixed but not fully in-spec**

  * There *was* an early return after any move; that has been replaced with a masked positional diff (`positional_diff_with_masks` / `positional_diff_masked_equal_size`).
  * New tests prove:

    * rect move + cell edit outside → both reported,
    * rect move + row insert outside → move plus additional ops,
    * row/column/fuzzy moves + outside edits → no silent data loss.
  * **Remaining gap**: move detectors still operate on full grids and ignore `RegionMask`. They are not mask‑aware as the plan originally specified.

* **1.2 “Make move detection iterative” is partially implemented**

  * `DiffConfig::max_move_iterations` is implemented and used.
  * There is an iterative loop with `RegionMask` and overlap checks, and tests for disjoint row moves and row+column moves.
  * **Remaining gaps:**

    * Detectors still ignore masks, so they can’t “see” only the remaining active region.
    * This makes scenarios like “three pure rect moves” brittle; spec explicitly calls for that test.

So, to truly “finish everything” for Branch 1, you need:

1. Mask‑aware move detection (via either refactoring detectors or wrapping them with a masked view).
2. Reliable multi‑move detection for rects as well as rows/columns.
3. A tiny bit more test coverage (e.g., the explicit “three rect moves” test).

---

## 2. High-level implementation plan

### Phase A – Add masked grid projection

**Goal:** Build a “view” of the grid that only includes rows/cols still active in the `RegionMask`, and a mapping back to original indices.

**Where:** `core/src/engine.rs` (near `diff_grids_with_config`), using existing `Grid`, `Cell`, `CellAddress`, and `RegionMask`.

**Steps:**

1. Implement `build_masked_grid(grid: &Grid, mask: &RegionMask) -> (Grid, Vec<u32>, Vec<u32>)`:

   * Use `mask.active_rows()` / `mask.active_cols()` to build ordered lists of active row/col indices.
   * Create lookup tables from original index → new index.
   * Allocate a new `Grid` with `nrows = active_row_count`, `ncols = active_col_count`.
   * For each cell in the original grid:

     * If its row and col are active, copy into the new grid at `(new_row, new_col)`, with a new `CellAddress` created from those indices.
   * Return the new grid and the `row_map` / `col_map` from new index → original index.

This gives you a clean, mask‑aware view without touching the existing move detectors.

### Phase B – Masked wrappers around move detectors

**Goal:** Make move detection respect `RegionMask` without rewriting the detector internals.

**Where:** Also in `engine.rs`, next to the move‑related helpers (`emit_row_block_move`, etc.).

**Strategy:**

* Define new helpers:

  * `detect_exact_row_block_move_masked`
  * `detect_exact_column_block_move_masked`
  * `detect_exact_rect_block_move_masked`
  * `detect_fuzzy_row_block_move_masked`
* Each helper:

  * Calls `build_masked_grid` on `old` and `new` with their masks.
  * Calls the existing detector on the projected grids.
  * Maps the resulting move (if any) back to original coordinates using the row/col maps.

This matches the plan’s intent (“detectors only consider unmasked cells”) without changing the algorithms themselves.

### Phase C – Wire masked detectors into `diff_grids_with_config`

**Goal:** Use the masked detectors in the move loop so iteration really walks through remaining active regions.

**Where:** `diff_grids_with_config` in `engine.rs`.

**Changes:**

1. Replace calls to `detect_exact_*` and `detect_fuzzy_row_block_move` with the masked wrappers.
2. Keep the existing iteration loop and `RegionMask` updates:

   * On a row move: `old_mask.exclude_rows`, `new_mask.exclude_rows`.
   * On a column move: `exclude_cols`.
   * On a rect move: `exclude_rect`.
3. Keep `max_move_iterations` and the `found_move` loop condition.
4. Keep the existing “Phase 2” masked positional diff:

   * If any rows/cols were excluded, run `positional_diff_with_masks` or `positional_diff_masked_equal_size` and then return.

Once the detectors see only the active region, repeated iterations will naturally discover multiple disjoint moves of the same type (including multiple rects).

### Phase D – Tests for remaining scenarios

**Goal:** Lock in behavior and guard against regressions.

**Where:** `core/tests/g14_move_combination_tests.rs`.

Add:

1. **`g14_three_disjoint_rect_block_moves_detected`**

   * Construct a base numeric grid.
   * Copy three non‑overlapping rect blocks from A into B, then re‑insert them at three distinct target locations in B.
   * Assert that exactly three `DiffOp::BlockMovedRect` are produced and there are no extraneous structural ops.

2. (Optional but nice) `g14_two_disjoint_rect_moves_plus_outside_edits_no_silent_data_loss`

   * Same as above, but also sprinkle a couple of cell edits outside all moved rects.
   * Assert:

     * There is at least one `BlockMovedRect`.
     * There are `CellEdited` ops outside the moved rects.

### Phase E – Documentation and cleanup

* Update any comments/docstrings that still refer to “detectors ignore masks”.
* Confirm that 1.1 and 1.2 deliverables are all satisfied:

  * Mask (RegionMask) tracked.
  * Move detectors respect masks.
  * Iterative loop with mask subtraction + config cap.
  * All combo/multi‑move tests pass.

---

## 3. Code for the hard pieces

Below is concrete Rust for the core changes. You’ll need to adapt imports slightly depending on exact module paths.

### 3.1 Masked grid projection helper

Add this to `core/src/engine.rs` (or a small internal module it can use):

```rust
fn build_masked_grid(source: &Grid, mask: &RegionMask) -> (Grid, Vec<u32>, Vec<u32>) {
    let row_map: Vec<u32> = mask.active_rows().collect();
    let col_map: Vec<u32> = mask.active_cols().collect();

    let nrows = row_map.len() as u32;
    let ncols = col_map.len() as u32;

    let mut row_lookup: Vec<Option<u32>> = vec![None; source.nrows as usize];
    for (new_idx, old_row) in row_map.iter().enumerate() {
        row_lookup[*old_row as usize] = Some(new_idx as u32);
    }

    let mut col_lookup: Vec<Option<u32>> = vec![None; source.ncols as usize];
    for (new_idx, old_col) in col_map.iter().enumerate() {
        col_lookup[*old_col as usize] = Some(new_idx as u32);
    }

    let mut projected = Grid::new(nrows, ncols);

    for cell in source.cells.values() {
        let old_row = cell.addr.row;
        let old_col = cell.addr.col;

        let Some(new_row) = row_lookup[old_row as usize] else {
            continue;
        };
        let Some(new_col) = col_lookup[old_col as usize] else {
        continue;
        };

        let addr = CellAddress::from_indices(new_row, new_col);
        let mut new_cell = cell.clone();
        new_cell.addr = addr;

        projected.cells.insert((new_row, new_col), new_cell);
    }

    (projected, row_map, col_map)
}
```

### 3.2 Masked move detector wrappers

Also in `engine.rs`, near the existing move helpers:

```rust
fn detect_exact_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<RowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_row_block_move(&old_proj, &new_proj)?;
    let src_start_row = old_rows[mv_local.src_start_row as usize];
    let dst_start_row = new_rows[mv_local.dst_start_row as usize];

    Some(RowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}

fn detect_exact_column_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<ColumnBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, _, old_cols) = build_masked_grid(old, old_mask);
    let (new_proj, _, new_cols) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_column_block_move(&old_proj, &new_proj)?;
    let src_start_col = old_cols[mv_local.src_start_col as usize];
    let dst_start_col = new_cols[mv_local.dst_start_col as usize];

    Some(ColumnBlockMove {
        src_start_col,
        dst_start_col,
        col_count: mv_local.col_count,
    })
}

fn detect_exact_rect_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<RectBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, old_cols) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, new_cols) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_rect_block_move(&old_proj, &new_proj)?;

    let src_start_row = old_rows[mv_local.src_start_row as usize];
    let dst_start_row = new_rows[mv_local.dst_start_row as usize];
    let src_start_col = old_cols[mv_local.src_start_col as usize];
    let dst_start_col = new_cols[mv_local.dst_start_col as usize];

    Some(RectBlockMove {
        src_start_row,
        dst_start_row,
        src_start_col,
        dst_start_col,
        src_row_count: mv_local.src_row_count,
        src_col_count: mv_local.src_col_count,
    })
}

fn detect_fuzzy_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<RowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_fuzzy_row_block_move(&old_proj, &new_proj)?;
    let src_start_row = old_rows[mv_local.src_start_row as usize];
    let dst_start_row = new_rows[mv_local.dst_start_row as usize];

    Some(RowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}
```

These wrappers use the existing detectors and simply remap indices back into the original coordinate space.

### 3.3 Updated `diff_grids_with_config` core loop

Finally, wire the masked detectors into the main diff engine. Replace the body of `diff_grids_with_config` with something like this (preserving your existing helper calls and config handling):

```rust
fn diff_grids_with_config(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
) {
    if old.nrows == 0 && new.nrows == 0 {
        return;
    }

    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);

    let mut iteration = 0u32;

    loop {
        if iteration >= config.max_move_iterations {
            break;
        }
        if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
            break;
        }

        let mut found_move = false;

        if let Some(mv) = detect_exact_rect_block_move_masked(old, new, &old_mask, &new_mask) {
            emit_rect_block_move(sheet_id, &mv, ops);
            old_mask.exclude_rect(
                mv.src_start_row,
                mv.src_row_count,
                mv.src_start_col,
                mv.src_col_count,
            );
            new_mask.exclude_rect(
                mv.dst_start_row,
                mv.src_row_count,
                mv.dst_start_col,
                mv.src_col_count,
            );
            iteration += 1;
            found_move = true;
        }

        if !found_move {
            if let Some(mv) = detect_exact_row_block_move_masked(old, new, &old_mask, &new_mask) {
                emit_row_block_move(sheet_id, &mv, ops);
                old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }
        }

        if !found_move {
            if let Some(mv) =
                detect_exact_column_block_move_masked(old, new, &old_mask, &new_mask)
            {
                emit_column_block_move(sheet_id, &mv, ops);
                old_mask.exclude_cols(mv.src_start_col, mv.col_count);
                new_mask.exclude_cols(mv.dst_start_col, mv.col_count);
                iteration += 1;
                found_move = true;
            }
        }

        if !found_move {
            if let Some(mv) = detect_fuzzy_row_block_move_masked(old, new, &old_mask, &new_mask)
            {
                emit_row_block_move(sheet_id, &mv, ops);
                emit_moved_row_block_edits(sheet_id, old, new, &mv, ops);
                old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }
        }

        if !found_move {
            break;
        }
    }

    if old_mask.has_excluded_rows() || old_mask.has_excluded_cols() {
        if old.nrows != new.nrows || old.ncols != new.ncols {
            positional_diff_with_masks(sheet_id, old, new, &old_mask, &new_mask, ops);
        } else {
            positional_diff_masked_equal_size(sheet_id, old, new, &old_mask, &new_mask, ops);
        }
        return;
    }

    if let Some(alignment) = align_row_changes(old, new) {
        emit_row_alignment_ops(sheet_id, old, new, &alignment, ops);
    } else if let Some(col_alignment) = align_single_column_change(old, new) {
        emit_column_alignment_ops(sheet_id, old, new, &col_alignment, ops);
    } else {
        positional_diff(sheet_id, old, new, ops);
    }
}
```

You may need to adjust the exact names of `emit_row_alignment_ops` / `emit_column_alignment_ops` to match your existing engine; the idea is to keep the old “no moves” path unchanged.

### 3.4 “Three rect moves” test

Add to `core/tests/g14_move_combination_tests.rs`:

```rust
#[test]
fn g14_three_disjoint_rect_block_moves_detected() {
    let mut grid_a = base_grid(20, 10);
    let mut grid_b = base_grid(20, 10);

    let block1 = vec![vec![1001, 1002], vec![1003, 1004]];
    let block2 = vec![vec![2001, 2002], vec![2003, 2004]];
    let block3 = vec![vec![3001, 3002], vec![3003, 3004]];

    place_block(&mut grid_a, 2, 1, &block1);
    place_block(&mut grid_a, 6, 3, &block2);
    place_block(&mut grid_a, 12, 5, &block3);

    place_block(&mut grid_b, 10, 1, &block1);
    place_block(&mut grid_b, 4, 6, &block2);
    place_block(&mut grid_b, 16, 2, &block3);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    let rect_moves: Vec<_> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRect { .. }))
        .collect();

    assert_eq!(
        rect_moves.len(),
        3,
        "expected exactly three rect block moves to be detected"
    );
}
```

This nails the last explicit deliverable in 1.2.

---

If you like, next step after you wire this in is to run the full test suite and then generate a short “Branch 1 is done” checklist against `next_sprint_plan.md`, but from here the remaining work is mechanical.
