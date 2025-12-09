Yes, this all hangs together and it does what you intended. Here’s how I see it, test by test, and how it lines up with the Branch 1 remediation notes.

---

### Big picture

The implementation details doc says:

* Only four tests need to be strengthened:

  * `g14_rect_move_plus_row_insertion_outside_no_silent_data_loss`
  * `g14_rect_move_plus_row_deletion_outside_no_silent_data_loss`
  * `g14_two_disjoint_row_block_moves_detected`
  * `g14_row_move_plus_column_move_both_detected`
* Everything else in Branch 1 is already implemented and tested.

That matches the “what’s left” summary in the combined remediation doc: the remaining gaps are (1) rect move + outside changes tests being too weak, and (2) multi‑move tests being too weak.

The new code in `cursor_strengthen_g14_move_combination.md` tightens exactly those four tests and doesn’t touch the engine, which is consistent with the plan.

---

### 1) `g14_rect_move_plus_row_insertion_outside_no_silent_data_loss`

**What changed**

Previously this test just asserted “some row addition or rect move or ops exist”. Now it constructs the same scenario but requires:

* Exactly one `BlockMovedRect`
* At least one `RowAdded`

Concretely:

* `grid_a`: 12×10 base grid with a distinctive 2×2 block at (2,2).
* `grid_b`: 13×10 base grid where:

  * Row 0 is a new row with `50000 + col` values.
  * Rows 1–12 are re-populated so that `grid_b[row]` matches `grid_a[row-1]` (clean one-row insertion).
  * The same 2×2 block is placed at (9,6).

Then:

````rust
let rect_moves = collect_rect_moves(&report);
let row_adds = collect_row_adds(&report);

assert_eq!(rect_moves.len(), 1, ...);
assert!(!row_adds.is_empty(), ...);
``` :contentReference[oaicite:4]{index=4}  

**Does it make sense?**

Yes:

- The grid construction really is “rect move + row insertion outside the block”.
- The assertions now encode the Branch 1.1 requirement “rect move + row insertion → both reported”, not just “something happened”. :contentReference[oaicite:5]{index=5}
- Indices are consistent with the existing rect move tests (block at 0‑based `(2,2)` to `(8,6)` patterns, but here shifted to `(9,6)` in B with the new row at index 0).

---

### 2) `g14_rect_move_plus_row_deletion_outside_no_silent_data_loss`

**What changed**

Same shape as insertion, but for deletion:

- `grid_a`: 14×10, block at (3,3).
- `grid_b`: 13×10, with the block moved to (8,6); last row effectively removed.   

Assertions now:

```rust
let rect_moves = collect_rect_moves(&report);
let row_removes = collect_row_removes(&report);

assert_eq!(rect_moves.len(), 1, ...);
assert!(!row_removes.is_empty(), ...);
````

**Does it make sense?**

Yes:

* Same pattern: a single rect move plus a structural change (row deletion) outside the block.
* Test now demands both: one `BlockMovedRect` and some `RowRemoved`, which matches the remediation text: “rect move + row deletion → both reported”.

---

### 3) `g14_two_disjoint_row_block_moves_detected`

**What changed**

Previously: move one block and just assert “some row change or ops exist”.

Now:

* `rows` is 24 rows, each with 3 integers `[r*10 + 1, r*10 + 2, r*10 + 3]`. 

* You build `rows_b` as a very deliberate permutation:

  ```rust
  // A = rows[3..7], B = rows[10..13] in the original
  rows_b.extend_from_slice(&rows[0..3]);   // rows 1–3
  rows_b.extend_from_slice(&rows[7..10]);  // rows 8–10
  rows_b.extend_from_slice(&rows[13..24]); // rows 14–24
  rows_b.extend_from_slice(&rows[3..7]);   // rows 4–7  (block A)
  rows_b.extend_from_slice(&rows[10..13]); // rows 11–13 (block B)
  ```

* So you get two clean, disjoint moves:

  * Block A: src `[3..7)` (rows 4–7), dst `[17..21)` → `(src_start_row=3, row_count=4, dst_start_row=17)`
  * Block B: src `[10..13)` (rows 11–13), dst `[21..24)` → `(10, 3, 21)`

* Then you assert:

  ```rust
  let row_moves = collect_row_moves(&report);
  assert_eq!(row_moves.len(), 2, ...);

  let actual: Vec<(u32,u32,u32)> = row_moves
      .iter()
      .map(|op| match **op {
          DiffOp::BlockMovedRows { src_start_row, row_count, dst_start_row, .. } =>
              (src_start_row, row_count, dst_start_row),
          _ => unreachable!(),
      })
      .collect();

  let expected = vec![(3,4,17), (10,3,21)];
  ```

**Does it make sense?**

Yes:

* The permutation is carefully constructed so the two blocks are unique and non‑overlapping, ideal for the RegionMask-based multi-move detection.
* Sorting both `actual` and `expected` makes the test invariant to the order in which the engine discovers the two moves.
* It enforces exactly the Branch 1.2 requirement: “two disjoint row block moves → two `BlockMovedRows` with the right ranges.” 

---

### 4) `g14_row_move_plus_column_move_both_detected`

**What changed**

Previously: it only checked “has_any_move || !ops.is_empty()`.

Now the test is:

* `grid_a`: 15×10, values `(r+1)*100 + c + 1`, i.e. each row is unique & regular. 

* `rows_b`:

  1. Move rows `[2..5)` (three rows) to position 10:

     ```rust
     let moved_rows: Vec<Vec<i32>> = rows_b.drain(2..5).collect();
     rows_b.splice(10..10, moved_rows);
     ```

     That’s a 3-row `BlockMovedRows` with `(src_start_row=2, row_count=3, dst_start_row=10)`.

  2. For each row, move one column:

     ```rust
     let moved_col = row.remove(1);
     row.insert(7, moved_col);
     ```

     That’s a 1-column `BlockMovedColumns` with `(src_start_col=1, col_count=1, dst_start_col=7)`.

* Then you assert:

  ```rust
  let row_moves = collect_row_moves(&report);
  let col_moves = collect_col_moves(&report);

  assert_eq!(row_moves.len(), 1, ...);
  assert_eq!(col_moves.len(), 1, ...);

  if let DiffOp::BlockMovedRows { src_start_row, row_count, dst_start_row, .. } = *row_moves[0] {
      assert_eq!(src_start_row, 2);
      assert_eq!(row_count, 3);
      assert_eq!(dst_start_row, 10);
  } else { panic!(...) }

  if let DiffOp::BlockMovedColumns { src_start_col, col_count, dst_start_col, .. } = *col_moves[0] {
      assert_eq!(src_start_col, 1);
      assert_eq!(col_count, 1);
      assert_eq!(dst_start_col, 7);
  } else { panic!(...) }
  ```

**Does it make sense?**

Yes:

* The scenario is a clean combination of a row block move and a column block move with unambiguous ranges.
* The test enforces exactly one of each move type and validates the coordinates, which is exactly what the remediation doc calls for: “row block move + column block move → both detected (with correct ranges).” 

---

### Any red flags or missing pieces?

The only small asymmetry is:

* The three rect+outside tests (`g14_rect_move_plus_cell_edit_no_silent_data_loss`, `g14_rect_move_plus_row_insertion_outside_no_silent_data_loss`, `g14_rect_move_plus_row_deletion_outside_no_silent_data_loss`) fully enforce “both reported” semantics for rect moves.
* The row‑block + row‑insert test (`g14_row_block_move_plus_row_insertion_outside_no_silent_data_loss`) still only asserts “no silent data loss” (just `!report.ops.is_empty()`), not “one `BlockMovedRows` and at least one `RowAdded`”.

That’s not wrong, and it matches what your new implementation doc explicitly scopes as “already good enough”; just be aware it’s weaker than the rect tests. If you ever want perfect symmetry with the Gap 1 test matrix, you could strengthen that one in the same style (collect row moves + row adds and assert both).

---

### Bottom line

* The implementation details are internally consistent.
* They line up with the Branch 1 remediation plan and close the concrete gaps for:

  * Rect move + row insertion/deletion → both reported.
  * Two disjoint row block moves → two `BlockMovedRows` with exact ranges.
  * Row block move + column block move → both move types emitted with correct ranges.
* No obvious off‑by‑one or indexing mistakes given the existing `BlockMoved*` tests.

So yes: this makes sense, and it’s a solid way to finish the remaining Branch 1 test work.
