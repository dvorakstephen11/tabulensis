
# Mini-spec: G12 Rectangular Block Move (BlockMovedRect)

## 1. Overview and Milestone Link

This cycle extends the grid diff engine to detect and report **rectangular block moves** as a first-class structural operation.

It advances **Phase 4 – Algorithmic Heavy Lifting**, specifically **G12 – Column / rectangular block move**, by:

- Adding a `BlockMovedRect` variant to `DiffOp` and PG4 serialization tests.
- Implementing a conservative “exact rectangular move” detector for spreadsheet mode.
- Adding a `rect_block_move_{a,b}.xlsx` workbook pair and corresponding G12 tests.

Textual M diff (M6) and basic grid diff (G1–G7) are already in place, along with row alignment, column alignment, row block alignment, row block moves (G11), column block moves (G12a), and D1 database-mode keyed equality. 

This spec does **not** attempt fuzzy moves (G13) or database-mode rectangles; it targets a narrow, exact 2D move case.

---

## 2. Scope

### 2.1 Rust modules and types

**Core types / engine:**

- `core/src/diff.rs`
  - Add a new `DiffOp::BlockMovedRect` variant.
  - Keep existing variants and semantics unchanged.

- `core/src/engine.rs`
  - Extend `diff_grids` (spreadsheet mode) to detect a single exact rectangular block move.
  - Emit `BlockMovedRect` when appropriate.
  - Preserve existing behavior for pure row moves, pure column moves, and all other cases.

**Helper layers (used but minimally changed):**

- `core/src/grid_view.rs`
  - Reuse `GridView`, `RowMeta`, `ColMeta`, and `HashStats` for candidate selection; avoid structural changes. 

- `core/src/row_alignment.rs`
  - Optionally expose a small helper (if needed) for locating unique moved row ranges, reusing the G11 logic without changing existing fast paths. 

- `core/src/column_alignment.rs`
  - Likewise for columns, building on G12a column block move detection. 

**Tests and fixtures:**

- `core/tests/pg4_*` / `core/tests/output_tests.rs`
  - Update PG4 DiffOp construction / JSON round-trip tests to include `BlockMovedRect`. 

- `core/tests/g12_rect_block_move_grid_workbook_tests.rs` (new)
  - Workbook-level tests for G12 rectangular move behavior.

- `fixtures/src/generators/grid.py`
  - Add a `RectBlockMoveG12Generator` for `rect_block_move_{a,b}.xlsx`.
- `fixtures/manifest.yaml`
  - Add `g12_rect_block_move` entry.

**Out of scope for this cycle:**

- No changes to database-mode alignment (`database_alignment.rs`, `diff_grids_database_mode`). 
- No fuzzy move detection (LAPJV or similarity thresholds) – that remains for G13. 
- No additional CLI or JSON surface changes beyond serializing/deserializing a new `DiffOp` variant.

---

## 3. Behavioral Contract

### 3.1 What counts as a rectangular block move (for this cycle)

This cycle targets **exact** rectangular moves in spreadsheet mode:

- A single rectangular region of the grid moves from one location to another.
- The block’s internal cell content (values and formulas, as seen by the existing grid diff) is **identical** between source and destination.
- Everything outside the block is identical between A and B.
- The block moves in both row and column dimensions (i.e., at least one row index and one column index change).

We treat this as a **structural move**, not a series of cell edits.

#### Example A – Simple 3×3 block move

Fixture: `rect_block_move_{a,b}.xlsx`.

- Grid A:
  - Sheet `"Data"`.
  - A 3×3 block with distinctive numbers at rows `3..=5`, columns `B..=D` (0-based rows 2–4, cols 1–3).
  - All other cells in a moderately small row×column window are filled with simple, repeatable values (e.g., row/col indices) and identical between A and B.
- Grid B:
  - The same 3×3 block appears at rows `10..=12`, columns `G..=I` (0-based rows 9–11, cols 6–8).
  - Cells at the original block location now contain the background pattern (again identical in A/B outside the moved region).

**Expected diff:**

- `diff_workbooks(&wb_a, &wb_b)` produces a `DiffReport` with **exactly one** operation:

  ```rust
  DiffOp::BlockMovedRect {
      sheet: "Data".to_string(),
      src_start_row: 2,        // row 3 in Excel notation
      src_row_count: 3,
      src_start_col: 1,        // column B
      src_col_count: 3,
      dst_start_row: 9,        // row 10
      dst_start_col: 6,        // column G
      block_hash: None,        // or Some(_) – tests must not depend on the exact value
  }
````

* No `RowAdded`, `RowRemoved`, `ColumnAdded`, or `ColumnRemoved` that correspond to the moved region.
* No `CellEdited` operations for any cell inside the moved 3×3 block.
* Outside the rectangle, the diff is empty.

This matches the G12 requirement: “For rectangle: a single rectangular move op … and no `CellEdited` in the block.”

#### Example B – Rectangular move with ambiguous repetition

We intentionally **do not** classify ambiguous patterns as rectangular moves.

* Construct two identical 3×3 blocks inside the grid in A.
* In B, swap these two blocks (each block’s content appears in the other’s location).
* Many rows/columns share identical signatures; there is no unique way to pair up a single source rectangle with a single destination.

**Expected behavior:**

* `diff_workbooks` **does not** emit `BlockMovedRect`.
* Diff degenerates to a mixture of row/column structure changes and/or `CellEdited` operations, as dictated by existing alignment/move logic.
* The test asserts “no `BlockMovedRect`, but some diff ops exist” rather than pinning the exact fallback script.

#### Example C – Rectangular move with internal edits (out of scope)

For this cycle, if the block **moves and is edited**, we treat it as **not** an exact rectangular move:

* Some cells inside the candidate rectangle differ between A and B.
* The future fuzzy move logic (G13) will handle this, using a similarity threshold and separate `CellEdited` ops inside the moved region.

**Expected behavior now:**

* No `BlockMovedRect` is emitted.
* Existing alignment/move paths may detect a row/column block move (if applicable) or fall back to cell-level diffs.
* Tests for this milestone should explicitly assert the absence of `BlockMovedRect` for a “move+edit” case; G13 will add tests with the opposite expectation.

### 3.2 Interaction with row and column move detection

Existing behavior:

* `BlockMovedRows` is emitted for unambiguous row block moves (G11).
* `BlockMovedColumns` is emitted for unambiguous column block moves (G12a).

New behavior:

* If the engine validates a **correlated rectangular move**, it emits a **single** `BlockMovedRect` and **does not** also emit the corresponding `BlockMovedRows` or `BlockMovedColumns` for the same rectangle. This keeps the op stream free of redundant moves.
* If only rows move (pure row block move), we keep emitting `BlockMovedRows` as today.
* If only columns move (pure column block move), we keep emitting `BlockMovedColumns` as today.
* Rectangular detection must not change the results of existing G11 and G12 column tests.

---

## 4. Constraints and Invariants

### 4.1 Performance and gating

Rectangular detection runs under the same general constraints as existing alignment and move detection (G8–G12a).

* **Grid size gates**:

  * Only attempt rectangular detection when:

    * `rows <= 2000` and `cols <= 128` (same order of magnitude as current row/column alignment gates).
  * For larger grids, skip rectangular detection entirely and keep current behavior.
* **Repetition and low-information guards**:

  * If the `GridView` indicates more than a configured fraction of rows/columns are “low-info” or if row/column hashes are highly repetitive (as per `HashStats`), do **not** attempt rectangular detection.
  * This mirrors the guards for row and column block move detection, and avoids quadratic pathologies on repetitive sheets.
* **Complexity**:

  * Rectangular detection must be **O(R·C)** or better in practice on the gated region.
  * No allocations of size `R * C` for large R/C (keep to vectors of rows, columns, and candidate ranges).

### 4.2 Determinism and uniqueness

* Detection must be **deterministic**: the same inputs always produce the same `BlockMovedRect` (or none), regardless of hash map iteration order.
* If more than one plausible rectangle candidate exists (due to repeated content), rectangular detection **must bail** and fall back, rather than picking an arbitrary one.
* Overlaps or conflicting candidates (e.g., two different potential destinations for the same source rectangle) also force a bail-out.

### 4.3 Mode limitations

* Rectangular move detection applies **only in spreadsheet mode** (`diff_grids`).
* Database mode (`diff_grids_database_mode`) ignores `BlockMovedRect` and continues to emit keyed row/cell diffs only.

---

## 5. Interfaces

### 5.1 New DiffOp variant

Extend `core/src/diff.rs` with a new variant, following the existing style for row/column block moves:

```rust
pub enum DiffOp {
    // ...existing variants...

    BlockMovedRect {
        sheet: SheetId,
        src_start_row: u32,
        src_row_count: u32,
        src_start_col: u32,
        src_col_count: u32,
        dst_start_row: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },

    CellEdited { /* unchanged */ },
}
```

Notes:

* Indices are 0-based, consistent with existing grid ops (`RowAdded`, `BlockMovedRows`, `BlockMovedColumns`). Tests continue to map to Excel notation via helpers when needed.
* `block_hash` is an optional summary hash of the moved rectangle’s content; tests for this cycle should not rely on its value and may accept `None`.

### 5.2 JSON / wire format

* Serialization uses the existing `#[serde(tag = "kind")]` scheme:

  * JSON examples:

    ```json
    {
      "kind": "BlockMovedRect",
      "sheet": "Data",
      "src_start_row": 2,
      "src_row_count": 3,
      "src_start_col": 1,
      "src_col_count": 3,
      "dst_start_row": 9,
      "dst_start_col": 6
    }
    ```

* PG4 tests must:

  * Construct at least two `BlockMovedRect` values (with and without `block_hash`).
  * Serialize them and assert `kind == "BlockMovedRect"` and the field names/values are stable.
  * Round-trip via deserialization and assert equality.

### 5.3 Engine entrypoints

* `diff_workbooks` already returns a `DiffReport` of `DiffOp` values; no signature change required.

* `engine::diff_grids`:

  * Add a new rectangular-move fast path that runs *before* falling back to row/column block moves or alignment-based cell diff.
  * If `BlockMovedRect` is emitted, `diff_grids` should return immediately for that sheet (no further row/column alignment or cell-level diff).

* JSON helpers such as `diff_report_to_cell_diffs` should continue to ignore the new structural op, just like `BlockMovedRows` and `BlockMovedColumns` today; tests may add a regression asserting that `BlockMovedRect` does not cause panics or spurious cell diffs.

---

## 6. Test Plan

All work in this cycle is driven by tests. The tests below are the acceptance criteria for the milestone.

### 6.1 PG4 – DiffOp plumbing updates

Extend the existing PG4 tests to cover `BlockMovedRect`.

1. **`pg4_construct_block_rect_diffops` (unit test)**

   * Location: `core/tests/pg4_diffops_tests.rs` (or equivalent existing PG4 test file).
   * Actions:

     * Construct:

       ```rust
       let rect_with_hash = DiffOp::BlockMovedRect {
           sheet: "Sheet1".to_string(),
           src_start_row: 5,
           src_row_count: 3,
           src_start_col: 2,
           src_col_count: 4,
           dst_start_row: 10,
           dst_start_col: 6,
           block_hash: Some(0xCAFEBABE),
       };
       let rect_without_hash = DiffOp::BlockMovedRect {
           sheet: "Sheet1".to_string(),
           src_start_row: 0,
           src_row_count: 1,
           src_start_col: 0,
           src_col_count: 1,
           dst_start_row: 20,
           dst_start_col: 10,
           block_hash: None,
       };
       ```
     * Assert that all fields are present and distinguish the two values.

2. **`pg4_block_rect_json_shape_and_roundtrip` (unit test)**

   * Construct a small `DiffReport` containing a `BlockMovedRect`.
   * Serialize to JSON using existing helpers.
   * Assert:

     * JSON tags include `"kind": "BlockMovedRect"`.
     * The coordinate fields are present.
   * Deserialize back and assert equality.

These tests ensure the new variant is part of the stable wire contract.

### 6.2 Fixtures – G12 rectangular workbook pair

**Generator:**

* Add `RectBlockMoveG12Generator` in `fixtures/src/generators/grid.py`:

  * Produces `rect_block_move_a.xlsx` and `rect_block_move_b.xlsx`.
  * A:

    * One sheet `"Data"`.
    * Grid of moderate size (e.g., 15×15).
    * 3×3 “distinctive block” with deterministic values at rows 3–5, columns B–D.
    * Remaining cells follow a simple pattern (e.g., `value = 1000 * row + col`) and are identical in A and B outside the moved block.
  * B:

    * Same background pattern.
    * 3×3 block relocated to new rows/cols (e.g., rows 10–12, columns G–I).

**Manifest:**

* Add an entry to `fixtures/manifest.yaml`:

  ```yaml
  - id: g12_rect_block_move
    kind: excel_pair
    a: fixtures/generated/rect_block_move_a.xlsx
    b: fixtures/generated/rect_block_move_b.xlsx
  ```

### 6.3 G12 rectangular block move tests (integration)

New file: `core/tests/g12_rect_block_move_grid_workbook_tests.rs`.

1. **`g12_rect_block_move_emits_single_blockmovedrect`**

   * Load the fixtures:

     ```rust
     let wb_a = open_workbook(fixture_path("rect_block_move_a.xlsx")).unwrap();
     let wb_b = open_workbook(fixture_path("rect_block_move_b.xlsx")).unwrap();
     let report = diff_workbooks(&wb_a, &wb_b);
     ```

   * Assertions:

     * `report.ops.len() == 1`.
     * The single op matches `DiffOp::BlockMovedRect { .. }`.

       * Verify `sheet == "Data"`.
       * Verify `src_row_count == 3`, `src_col_count == 3`.
       * Verify `src_start_row`, `src_start_col`, `dst_start_row`, `dst_start_col` correspond to the generator’s layout.
       * Do **not** assert on `block_hash` value; just check its `Option` shape (`Some` or `None`).
     * Ensure there are **no** `RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`, or `CellEdited` ops in the report.

2. **`g12_rect_block_move_ambiguous_repetition_does_not_emit_blockmovedrect`**

   * Build grids in-memory with `grid_from_numbers` / `single_sheet_workbook`.
   * Scenario:

     * Two identical 2×2 or 3×3 blocks appear in A.
     * In B, the blocks are swapped.
   * Assertions:

     * `report.ops.iter().all(|op| !matches!(op, DiffOp::BlockMovedRect { .. }))`.
     * `!report.ops.is_empty()` (fallback path emits some other diff ops).

3. **Optional (if needed): `g12_rect_move_with_internal_edit_falls_back`**

   * Take the base `rect_block_move` scenario.
   * Edit one cell inside the moved block in B.
   * Assert:

     * No `BlockMovedRect`.
     * The diff includes at least one `CellEdited`, and possibly row/column moves per existing logic.

### 6.4 Engine-level detection tests (optional but recommended)

If rectangular detection is factored into a small helper (e.g., `detect_exact_rect_block_move`), add module-level tests alongside row/column move tests:

* Success case: small synthetic grid where the helper returns a single rectangle.
* Negative cases:

  * Multiple candidate rectangles.
  * Size mismatch or partial overlap.
  * Internal cell differences.

These tests should mirror the workbook-level expectations but keep the inputs tiny for speed.

---

## 7. Milestone Mapping

* **Primary milestone:** G12 – Column / rectangular block move (rectangular portion, exact move). 
* **Supporting milestone:** PG4 – DiffOp plumbing & wire contract (extended to include BlockMovedRect).

Completion criteria for this cycle:

1. All new and existing tests pass:

   * PG4 extended tests.
   * New G12 rectangular workbook tests.
   * Any added helper/unit tests.
2. Existing G11/G12a tests still pass unmodified.
3. The public `DiffOp` enum includes `BlockMovedRect` and round-trips via JSON without breaking existing consumers.

Once this mini-spec is implemented, the engine will have full **exact** block move coverage (rows, columns, and rectangles), setting the stage for fuzzy moves (G13) and performance validation on larger grids.
