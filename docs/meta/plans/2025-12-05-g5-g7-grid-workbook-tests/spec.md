# 2025-12-05-g5-g7-grid-workbook-tests — Mini-Spec

Phase-3 spreadsheet-mode grid diff fixtures and tests for G5–G7 on real Excel workbooks, leveraging the existing PG5 in-memory semantics and G1–G2 integration tests.

## 1. Scope

### 1.1 Rust modules

In scope:

- `core/tests/g5_g7_grid_workbook_tests.rs`
  - New integration test module that:
    - Opens Python-generated Excel fixture pairs for G5–G7.
    - Invokes `excel_diff::diff_workbooks`.
    - Asserts on `DiffOp` sequences for:
      - Multiple scattered cell edits (G5).
      - Simple bottom row append/delete (G6).
      - Simple right-edge column append/delete (G7).

Out of scope (no planned changes unless tests reveal bugs):

- `core/src/diff.rs`
  - `DiffOp` enum and `DiffReport` structure.
- `core/src/engine.rs`
  - `diff_workbooks` and `diff_grids`.
  - Any alignment or signature-based logic (reserved for later phases).

### 1.2 Python fixtures

In scope:

- `fixtures/src/generators/grid.py`
  - New generator(s) to produce the Excel pairs required by G5–G7.

- `fixtures/src/generate.py`
  - Register new generator names in the generator registry.

- `fixtures/manifest.yaml`
  - New scenario entries under the Phase 3 spreadsheet-mode section:
    - `g5_multi_cell_edits`
    - `g6_row_append_bottom`
    - `g6_row_delete_bottom`
    - `g7_col_append_right`
    - `g7_col_delete_right`

The concrete `.xlsx` outputs live under `fixtures/generated/` as usual and are treated as build artifacts.

## 2. Behavioral Contract

All expectations below refer to the behavior of `diff_workbooks(&Workbook, &Workbook) -> DiffReport` when run on real Excel workbooks parsed via `open_workbook`.

### 2.1 G5 – Multiple independent cell edits in a fixed grid

**Fixture shape**

- Workbook A:
  - Sheet `Sheet1`, 20×10 grid of literal values (numbers and/or strings).
- Workbook B:
  - Same sheet and grid size.
  - 5–10 cells scattered across different rows and columns changed.
    - A mix of numeric and text changes.

**Expected diff behavior**

- `DiffReport.ops` contains:
  - Exactly one `DiffOp::CellEdited` per changed cell.
- No structural operations:
  - No `RowAdded`, `RowRemoved`, `ColumnAdded`, or `ColumnRemoved`.
- Each `CellEdited`:
  - Has `sheet == "Sheet1"`.
  - `addr` matches the edited cell’s A1 address.
  - `from.value` equals the value in A, `to.value` equals the value in B.
  - `from.formula` and `to.formula` are identical (or both `None`); we are not testing formula semantics here.

**Order semantics**

- Tests treat the set of operations as unordered:
  - They check membership and counts, not ordering.
- This deliberately leaves room for later changes in operation ordering when advanced alignment is introduced.

### 2.2 G6 – Simple row append / truncate at bottom

**Fixtures**

1. `row_append_bottom_{a,b}.xlsx`:

   - A: Sheet `Sheet1` with rows 1–10 populated.
     - Column A contains simple sequential IDs (1..10).
   - B: Same first 10 rows, plus rows 11–12 appended at the bottom.

2. `row_delete_bottom_{a,b}.xlsx`:

   - A: Sheet `Sheet1` with rows 1–12 populated.
   - B: Same as A but only rows 1–10 remain (rows 11–12 removed).

**Expected diff behavior**

- Row indices in `DiffOp` are **0-based** grid indices, consistent with PG5 tests:
  - Logical excel row 11 → `row_idx = 10`.
  - Logical excel row 12 → `row_idx = 11`.

- Append case:

  - `DiffReport.ops` contains exactly two `DiffOp::RowAdded` operations:
    - With `sheet == "Sheet1"`.
    - `row_idx` equal to 10 and 11 (in any order).
    - `row_signature` is `None` (signatures are not wired into diff ops yet).
  - No `RowRemoved`, `Column*`, or `CellEdited` ops.

- Delete case:

  - `DiffReport.ops` contains exactly two `DiffOp::RowRemoved` operations:
    - With `sheet == "Sheet1"`.
    - `row_idx` equal to 10 and 11.
    - `row_signature` is `None`.
  - No `RowAdded`, `Column*`, or `CellEdited` ops.

### 2.3 G7 – Simple column append / truncate at right edge

**Fixtures**

1. `col_append_right_{a,b}.xlsx`:

   - A: Sheet `Sheet1` with columns A–D filled.
   - B: Same as A but with new columns E and F appended at the right.

2. `col_delete_right_{a,b}.xlsx`:

   - A: Sheet `Sheet1` with columns A–F filled.
   - B: Same data but only columns A–D remain.

**Expected diff behavior**

- Column indices in `DiffOp` are 0-based:
  - Logical column E → `col_idx = 4`.
  - Logical column F → `col_idx = 5`.

- Append case:

  - `DiffReport.ops` contains exactly two `DiffOp::ColumnAdded` operations:
    - With `sheet == "Sheet1"`.
    - `col_idx` equal to 4 and 5 (any order).
    - `col_signature` is `None`.
  - No `ColumnRemoved`, `Row*`, or `CellEdited` ops.

- Delete case:

  - `DiffReport.ops` contains exactly two `DiffOp::ColumnRemoved` operations:
    - With `sheet == "Sheet1"`.
    - `col_idx` equal to 4 and 5.
    - `col_signature` is `None`.
  - No `ColumnAdded`, `Row*`, or `CellEdited` ops.

## 3. Constraints

1. **No algorithmic upgrades in this branch**

   - `diff_grids` remains the simple PG5 implementation:
     - Compares only the overlapping rectangle.
     - Emits `CellEdited` for cell-wise differences in that overlap.
     - Emits `RowAdded/RowRemoved` and `ColumnAdded/ColumnRemoved` for tail shape differences only.
   - Any changes to `diff_grids` are limited to bug fixes required to satisfy G5–G7; no alignment or signature-based logic is introduced here.

2. **DiffOp invariants**

   - `DiffOp` semantics and JSON schema remain as defined in the spec and enforced by PG4 tests:
     - `addr` in `CellEdited` is a valid A1 address and matches snapshot addresses.
     - Row/column indices are 0-based and consistent with PG5 tests.
   - New tests must not weaken any existing PG4/PG5/PG6 assertions.

3. **Fixture simplicity**

   - All new fixtures use small, regular grids:
     - G5: 20×10.
     - G6: 10–12 rows, a small number of columns.
     - G7: 4–6 columns, a small number of rows.
   - No formulas, formatting, or merged cells in G5–G7 fixtures:
     - Values are simple numbers and/or plain strings.
     - This avoids mixing G3/G4 concerns into this branch.

4. **Streaming and memory**

   - Fixture sizes stay small enough that they do not influence streaming or memory guard behavior.
   - No new performance or memory constraints are introduced.

## 4. Interfaces

### 4.1 Public Rust interfaces

No public API changes.

- `excel_diff::diff_workbooks(path_a, path_b)` and `excel_diff::diff_workbooks(&Workbook, &Workbook)`:
  - Signatures remain unchanged.
  - New tests call the existing APIs exactly as G1–G2 tests do.

- `excel_diff::DiffOp`:
  - No new variants.
  - Existing variants used in this spec:
    - `CellEdited { sheet, addr, from, to }`
    - `RowAdded { sheet, row_idx, row_signature }`
    - `RowRemoved { sheet, row_idx, row_signature }`
    - `ColumnAdded { sheet, col_idx, col_signature }`
    - `ColumnRemoved { sheet, col_idx, col_signature }`

### 4.2 Python generator interface

Add new generator names to `fixtures/src/generate.py`:

- `"multi_cell_diff"`:
  - Creates a pair of workbooks identical except for a configurable set of edited cells.

- `"grid_tail_diff"` (name is illustrative; actual name must match manifest entries):
  - Creates a pair of workbooks differing only by bottom-row or right-column append/delete patterns.

Arguments are simple primitives (rows, cols, sheet name, mode, list of A1 addresses and new values) so that Rust tests do not need to understand generator internals.

## 5. Test Plan

### 5.1 Python fixtures

Extend `fixtures/manifest.yaml` with new scenarios under the Phase 3 section (after `g2_single_cell_value`):

```yaml
  # --- Phase 3: Spreadsheet-mode G5–G7 ---

  - id: "g5_multi_cell_edits"
    generator: "multi_cell_diff"
    args:
      rows: 20
      cols: 10
      sheet: "Sheet1"
      edits:
        - { addr: "B2", value_a: 1.0, value_b: 42.0 }
        - { addr: "D5", value_a: 2.0, value_b: 99.0 }
        - { addr: "H7", value_a: 3.0, value_b: 3.5 }
        - { addr: "J10", value_a: "x", value_b: "y" }
        # implementation may add more edits (5–10 total), but tests will only assert on the configured set
    output:
      - "multi_cell_edits_a.xlsx"
      - "multi_cell_edits_b.xlsx"

  - id: "g6_row_append_bottom"
    generator: "grid_tail_diff"
    args:
      mode: "row_append_bottom"
      sheet: "Sheet1"
      base_rows: 10
      tail_rows: 2
    output:
      - "row_append_bottom_a.xlsx"
      - "row_append_bottom_b.xlsx"

  - id: "g6_row_delete_bottom"
    generator: "grid_tail_diff"
    args:
      mode: "row_delete_bottom"
      sheet: "Sheet1"
      base_rows: 10
      tail_rows: 2
    output:
      - "row_delete_bottom_a.xlsx"
      - "row_delete_bottom_b.xlsx"

  - id: "g7_col_append_right"
    generator: "grid_tail_diff"
    args:
      mode: "col_append_right"
      sheet: "Sheet1"
      base_cols: 4
      tail_cols: 2
    output:
      - "col_append_right_a.xlsx"
      - "col_append_right_b.xlsx"

  - id: "g7_col_delete_right"
    generator: "grid_tail_diff"
    args:
      mode: "col_delete_right"
      sheet: "Sheet1"
      base_cols: 4
      tail_cols: 2
    output:
      - "col_delete_right_a.xlsx"
      - "col_delete_right_b.xlsx"
````

Generator responsibilities (high-level):

* `multi_cell_diff`:

  * Build workbook A using the same pattern as `basic_grid` for `rows × cols` on `sheet`.
  * Clone A to B.
  * For each entry in `edits`:

    * Set A’s cell at `addr` to `value_a`.
    * Set B’s cell at `addr` to `value_b`.
* `grid_tail_diff`:

  * For `row_append_bottom`:

    * A: `base_rows` populated rows.
    * B: same first `base_rows`, plus `tail_rows` new rows at bottom.
  * For `row_delete_bottom`:

    * A: `base_rows + tail_rows` rows.
    * B: only the first `base_rows` rows.
  * For `col_append_right`:

    * A: `base_cols` populated columns.
    * B: same plus `tail_cols` new columns at the right.
  * For `col_delete_right`:

    * A: `base_cols + tail_cols` columns.
    * B: only the first `base_cols` columns.

### 5.2 Rust tests

Create `core/tests/g5_g7_grid_workbook_tests.rs` with the following tests:

1. **`g5_multi_cell_edits_produces_only_celledited_ops`**

   * Load `multi_cell_edits_a.xlsx` / `multi_cell_edits_b.xlsx`.
   * Run `diff_workbooks`.
   * Assert:

     * `report.ops.len()` equals the number of configured edits.
     * All ops are `DiffOp::CellEdited`.
     * For each expected address (e.g., B2, D5, H7, J10):

       * There is exactly one `CellEdited` with that `addr.to_a1()`.
       * `from.value` == configured `value_a`, `to.value` == `value_b`.
     * No `Row*` or `Column*` ops.

2. **`g6_row_append_bottom_emits_two_rowadded_and_no_celledited`**

   * Load `row_append_bottom_a.xlsx` / `row_append_bottom_b.xlsx`.
   * Run diff.
   * Assert:

     * Exactly two `RowAdded` ops for `sheet == "Sheet1"`.
     * `row_idx` values are `{10, 11}` (0-based).
     * No `RowRemoved`, `Column*`, or `CellEdited` ops.

3. **`g6_row_delete_bottom_emits_two_rowremoved_and_no_celledited`**

   * Same as above but for `row_delete_bottom_{a,b}.xlsx`.
   * Assert exactly two `RowRemoved` at indices `{10, 11}` and no other ops.

4. **`g7_col_append_right_emits_two_columnadded_and_no_celledited`**

   * Load `col_append_right_{a,b}.xlsx`.
   * Run diff.
   * Assert:

     * Exactly two `ColumnAdded` ops for `sheet == "Sheet1"`.
     * `col_idx` values are `{4, 5}` (0-based for E and F).
     * No `ColumnRemoved`, `Row*`, or `CellEdited` ops.

5. **`g7_col_delete_right_emits_two_columnremoved_and_no_celledited`**

   * Load `col_delete_right_{a,b}.xlsx`.
   * Run diff.
   * Assert:

     * Exactly two `ColumnRemoved` ops for `sheet == "Sheet1"`.
     * `col_idx` values are `{4, 5}`.
     * No `ColumnAdded`, `Row*`, or `CellEdited` ops.

All tests should reuse the existing `tests::common::fixture_path` helper to locate fixtures, mirroring the pattern in `g1_g2_grid_workbook_tests.rs`.

### 5.3 Relationship to existing tests

* PG5 in-memory tests already pin the same behaviors at the `Grid` level.
* G1–G2 workbook tests pin identical and single-cell-change behavior on real Excel.
* New G5–G7 tests extend this to:

  * Multi-cell edits (UC‑03) on real workbooks.
  * Simple tail row/column operations (UC‑04/UC‑05) on real workbooks.

Together, these will mark the basic spreadsheet-mode slice (G1–G2, G5–G7) as fully covered at both IR and Excel levels, providing a solid guardrail for future alignment and move-detection work.

