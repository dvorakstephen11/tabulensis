From what I can see in the current state:

* The Branch 3 deliverables (config plumbing) look implemented: `DiffConfig` + `LimitBehavior`, presets, builder/validation, serde support + tests, and config threading through the engine and alignment/move detection paths.
* The main thing that looks “wrong” relative to your observation is the *performance metric* `cells_compared`: it’s being counted in a way that can easily double for dense rows, which makes it look like the work exploded even when runtime didn’t. This is coming from the sparse row-diff path.

Below is a concrete plan to fix the “cells skyrocketed” issue, plus a small regression test so it doesn’t come back.

---

## Why the cell count jumped

In the sparse row emitters (`emit_aligned_diffs` / `emit_amr_aligned_diffs`), `cells_compared` is currently incremented like this:

* add `(old_row.cells.len() + new_row.cells.len())` per matched row pair

For dense grids, `old_row.cells.len()` and `new_row.cells.len()` are both basically “number of columns”, so the metric becomes `2 * overlap_cols` per matched row. That *exactly doubles* the metric in dense fixtures, even though you’re not actually comparing twice as many `(row,col)` positions.

If you want `cells_compared` to represent “cell pairs compared” (one per `(row,col)` you evaluate), it should count the number of iterations the sparse merge loop performs (the union of column indices touched), not the sum of both sides’ storage lengths. 

---

## Implementation plan to fix it

### Step 1) Make `diff_row_pair_sparse` return the number of comparisons performed

Instead of trying to compute “compared” outside, count one per loop iteration inside the merge loop and return that count.

This makes the metric:

* Dense row: `overlap_cols`
* Sparse row: `|union(cols_with_cells_in_either_row)|`
* Never double-counts matching columns

### Step 2) Update `emit_aligned_diffs` and `emit_amr_aligned_diffs` to accumulate that return value

Replace the current `len(old)+len(new)` increment with:

* `compared += diff_row_pair_sparse(...)`

### Step 3) Update other call sites of `diff_row_pair_sparse`

At least `emit_moved_row_block_edits` calls it; after changing the return type, either:

* accumulate it too, or
* explicitly ignore it (`let _ = ...;`)

### Step 4) Add a small unit test guarding the metric semantics

Add a unit test that calls `diff_row_pair_sparse` directly (it’s in the same module) and asserts:

* dense 3-col row reports `3` comparisons, not `6`
* sparse disjoint cols report union size (eg `2`)

This avoids needing to rely on the full pipeline’s heuristic ordering.

### Step 5) Re-run your perf harness

After the change, you should see the dense fixtures’ `cells_compared` drop roughly by half (because they were being double-counted), while runtimes should remain roughly the same.

---

## Code changes

All changes are in `core/src/engine.rs`. The snippets below are directly tied to the code that currently adds `(old_row.cells.len() + new_row.cells.len())`.

### 1) Fix `emit_aligned_diffs` counting

Replace this (inside the `for (row_a, row_b) in &alignment.matched` loop):

```rust
compared = compared.saturating_add((old_row.cells.len() + new_row.cells.len()) as u64);
diff_row_pair_sparse(
    sheet_id,
    *row_b,
    overlap_cols,
    &old_row.cells,
    &new_row.cells,
    ops,
    config,
);
```

With this:

```rust
compared = compared.saturating_add(diff_row_pair_sparse(
    sheet_id,
    *row_b,
    overlap_cols,
    &old_row.cells,
    &new_row.cells,
    ops,
    config,
));
```

### 2) Fix `emit_amr_aligned_diffs` counting

Replace this (inside the matched loop):

```rust
compared = compared.saturating_add((old_row.cells.len() + new_row.cells.len()) as u64);
diff_row_pair_sparse(
    sheet_id,
    row_b,
    overlap_cols,
    &old_row.cells,
    &new_row.cells,
    ops,
    config,
);
```

With this:

```rust
compared = compared.saturating_add(diff_row_pair_sparse(
    sheet_id,
    row_b,
    overlap_cols,
    &old_row.cells,
    &new_row.cells,
    ops,
    config,
));
```

### 3) Change `diff_row_pair_sparse` to return `u64 compared`

Replace this:

```rust
fn diff_row_pair_sparse(
    sheet_id: &SheetId,
    row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &Cell)],
    new_cells: &[(u32, &Cell)],
    ops: &mut Vec<DiffOp>,
    config: &DiffConfig,
) {
    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_cells.len() || j < new_cells.len() {
        let col_a = old_cells.get(i).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col_b = new_cells.get(j).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col = col_a.min(col_b);

        if col >= overlap_cols {
            break;
        }

        let old_cell = if col_a == col {
            let cell = old_cells[i].1;
            i += 1;
            Some(cell)
        } else {
            None
        };

        let new_cell = if col_b == col {
            let cell = new_cells[j].1;
            j += 1;
            Some(cell)
        } else {
            None
        };

        let changed = !cells_content_equal(old_cell, new_cell);

        if changed || config.include_unchanged_cells {
            let addr = CellAddress::from_indices(row_b, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
        }
    }
}
```

With this:

```rust
fn diff_row_pair_sparse(
    sheet_id: &SheetId,
    row_b: u32,
    overlap_cols: u32,
    old_cells: &[(u32, &Cell)],
    new_cells: &[(u32, &Cell)],
    ops: &mut Vec<DiffOp>,
    config: &DiffConfig,
) -> u64 {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut compared = 0u64;

    while i < old_cells.len() || j < new_cells.len() {
        let col_a = old_cells.get(i).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col_b = new_cells.get(j).map(|(c, _)| *c).unwrap_or(u32::MAX);
        let col = col_a.min(col_b);

        if col >= overlap_cols {
            break;
        }

        compared = compared.saturating_add(1);

        let old_cell = if col_a == col {
            let cell = old_cells[i].1;
            i += 1;
            Some(cell)
        } else {
            None
        };

        let new_cell = if col_b == col {
            let cell = new_cells[j].1;
            j += 1;
            Some(cell)
        } else {
            None
        };

        let changed = !cells_content_equal(old_cell, new_cell);

        if changed || config.include_unchanged_cells {
            let addr = CellAddress::from_indices(row_b, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
        }
    }

    compared
}
```

### 4) Update any other call sites

For example, if you have a call like this in `emit_moved_row_block_edits`:

```rust
diff_row_pair_sparse(
    sheet_id,
    dst_row,
    overlap_cols,
    &old_row.cells,
    &new_row.cells,
    ops,
    config,
);
```

Replace it with:

```rust
let _ = diff_row_pair_sparse(
    sheet_id,
    dst_row,
    overlap_cols,
    &old_row.cells,
    &new_row.cells,
    ops,
    config,
);
```

Then do a quick search for all remaining call sites:

* search string: `diff_row_pair_sparse(`

---

## Regression test to lock in the correct behavior

Add this under the existing `#[cfg(test)]` tests in `core/src/engine.rs` (or create a new `mod tests` if one doesn’t exist in that file).

```rust
#[cfg(test)]
mod sparse_row_pair_metrics_tests {
    use super::*;
    use crate::workbook::{Cell, CellValue};

    fn cell_num(v: f64) -> Cell {
        Cell {
            row: 0,
            col: 0,
            address: CellAddress::from_indices(0, 0),
            value: Some(CellValue::Number(v)),
            formula: None,
        }
    }

    #[test]
    fn diff_row_pair_sparse_counts_union_columns_not_sum_lengths() {
        let sheet_id: SheetId = "Sheet1".to_string();
        let config = DiffConfig::default();
        let mut ops = Vec::new();

        let a_cells = vec![cell_num(1.0), cell_num(2.0), cell_num(3.0)];
        let b_cells = vec![cell_num(1.0), cell_num(2.0), cell_num(4.0)];

        let old_cells: Vec<(u32, &Cell)> = vec![(0, &a_cells[0]), (1, &a_cells[1]), (2, &a_cells[2])];
        let new_cells: Vec<(u32, &Cell)> = vec![(0, &b_cells[0]), (1, &b_cells[1]), (2, &b_cells[2])];

        let compared = diff_row_pair_sparse(
            &sheet_id,
            0,
            3,
            &old_cells,
            &new_cells,
            &mut ops,
            &config,
        );

        assert_eq!(compared, 3);
    }

    #[test]
    fn diff_row_pair_sparse_sparse_union_is_counted() {
        let sheet_id: SheetId = "Sheet1".to_string();
        let config = DiffConfig::default();
        let mut ops = Vec::new();

        let a_cells = vec![cell_num(1.0)];
        let b_cells = vec![cell_num(2.0)];

        let old_cells: Vec<(u32, &Cell)> = vec![(0, &a_cells[0])];
        let new_cells: Vec<(u32, &Cell)> = vec![(2, &b_cells[0])];

        let compared = diff_row_pair_sparse(
            &sheet_id,
            0,
            3,
            &old_cells,
            &new_cells,
            &mut ops,
            &config,
        );

        assert_eq!(compared, 2);
    }
}
```

---

## Quick “everything else” checklist

If you want a fast sanity sweep beyond this metric fix:

1. Run tests with all features (because config + metrics are often feature-gated):

   * `cargo test --all-features`
2. Run the perf harness you used to generate the JSON.
3. Confirm `total_cells_compared` is no longer inflated in the dense fixtures.

If those pass, I don’t see other Branch 3 deliverables that are still missing based on the code + tests currently present.
