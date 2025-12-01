## 1. Scope

### 1.1 Modules and types in play

**Implementation**

* `core/src/engine.rs`

  * `diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport`

    * Entry point for workbook diff. Must retain its public API and overall behavior.
  * `fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>)`

    * Internal grid diff helper. Will be updated to:

      * Compare only the overlapping rectangle by cell.
      * Emit `RowAdded`/`RowRemoved` and `ColumnAdded`/`ColumnRemoved` for tail shape changes.
      * Avoid emitting `CellEdited` for rows/columns that are entirely added/removed at the end.
* `core/src/workbook.rs`

  * Read‑only use of:

    * `Grid { nrows, ncols, cells, row_signatures, col_signatures }`
    * `Cell`, `CellAddress`, `CellSnapshot`, `CellValue`
  * No shape or semantics changes to these types in this cycle.
* `core/src/diff.rs`

  * Read‑only use of:

    * `DiffOp` enum variants: `RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`, `CellEdited`
    * `DiffReport::new`
    * Helper constructors: `DiffOp::cell_edited`, `DiffOp::row_added`, `DiffOp::row_removed`,
      `DiffOp::column_added`, `DiffOp::column_removed`
  * No changes to JSON schema, serialization, or existing PG4 tests.

**Tests**

* New test module:

  * `core/tests/pg5_grid_diff_tests.rs`

    * Pure in‑memory tests that build `Workbook`/`Sheet`/`Grid` objects with tiny grids and call
      `diff_workbooks`.
    * Encodes all PG5 scenarios with explicit expectations on emitted `DiffOp`s.
* Existing tests (unchanged, but must still pass):

  * `core/tests/engine_tests.rs` – sanity checks for `diff_workbooks` on simple 1×1 workbooks.
  * `core/tests/pg4_diffop_tests.rs` – DiffOp construction, JSON shape, and round‑trip behavior.
  * `core/tests/output_tests.rs` – JSON diff helpers and cell‑only views.
  * `core/tests/signature_tests.rs`, `core/tests/sparse_grid_tests.rs` – grid/signature basics.

### 1.2 Out of scope

* No row/column reordering, move detection, or database‑mode behavior.
* No changes to:

  * Excel Open XML parsing (`excel_open_xml`).
  * DataMashup framing or M‑related types.
  * DiffOp variants beyond the basic row/column/cell operations.
* No performance tuning or algorithmic alignment beyond the simple spreadsheet‑mode “tail add/remove”
  semantics described below.

---

## 2. Behavioral Contract

This spec defines how `diff_grids` (and therefore `diff_workbooks`) must behave when comparing two
grids in **spreadsheet mode** with **no row/column reordering**. It pins down semantics for simple
shape changes and scattered edits on tiny in‑memory grids.

### 2.1 General rules

For a given sheet pair `(old_sheet, new_sheet)` with `SheetKind::Worksheet` and the same logical
identity (same name and kind, matched by `diff_workbooks`):

1. `diff_workbooks` must:

   * Call `diff_grids` exactly once for that pair.
   * Only emit grid‑level `DiffOp`s (`Row*`, `Column*`, `CellEdited`, etc.) for that sheet; sheet‐level
     add/remove ops are already handled in `diff_workbooks` and must not be duplicated at grid level.

2. `diff_grids` must:

   * Compare `old.grid` and `new.grid` in **two phases**:

     1. **Overlapping rectangle**: rows `[0 .. min(nrows_old, nrows_new))`,
        columns `[0 .. min(ncols_old, ncols_new))`:

        * For each `(row, col)` in this rectangle:

          * Fetch `old_cell = old.get(row, col)` and `new_cell = new.get(row, col)`.
          * Map to `CellSnapshot` via `CellSnapshot::from_cell` or `CellSnapshot::empty(addr)`.
          * If snapshots differ, emit exactly one `DiffOp::CellEdited` using `DiffOp::cell_edited`.
     2. **Tail shape differences**:

        * For rows beyond the overlap:

          * If `nrows_new > nrows_old`, rows `[nrows_old .. nrows_new)` are **appended rows**.
            Emit one `RowAdded` per appended row.
          * If `nrows_old > nrows_new`, rows `[nrows_new .. nrows_old)` are **truncated rows**.
            Emit one `RowRemoved` per truncated row.
        * For columns beyond the overlap:

          * If `ncols_new > ncols_old`, columns `[ncols_old .. ncols_new)` are **appended columns**.
            Emit one `ColumnAdded` per appended column.
          * If `ncols_old > ncols_new`, columns `[ncols_new .. ncols_old)` are **truncated columns**.
            Emit one `ColumnRemoved` per truncated column.
   * Never emit `CellEdited` ops for cells that lie entirely in:

     * Appended rows (row index ≥ `nrows_old` when `nrows_new > nrows_old`), or
     * Appended columns (col index ≥ `ncols_old` when `ncols_new > ncols_old`), or
     * Rows/columns that are only present in the old grid (these are represented as removals).

3. Structural ops:

   * Use the `DiffOp` helper constructors where available:

     * `DiffOp::row_added(sheet, row_idx, None)`
     * `DiffOp::row_removed(sheet, row_idx, None)`
     * `DiffOp::column_added(sheet, col_idx, None)`
     * `DiffOp::column_removed(sheet, col_idx, None)`
   * For this milestone, `row_signature` and `col_signature` should be `None` (signatures will be
     populated in a later cycle). Tests must not depend on their presence or hash value.

4. Determinism and ordering:

   * The set of emitted operations must be fully determined by the input grids.
   * `DiffReport::new` already preserves op order; this milestone does not change its behavior.
   * For PG5 tests, assertions will treat `DiffReport.ops` as a stable sequence:

     * Structural ops count and indices must match expectations.
     * No extra or missing ops.

### 2.2 PG5 scenario contracts

The following are **hard behavioral contracts** to be enforced by tests.

#### PG5.1 – 1×1 identical grids → empty diff

* Setup:

  * `GridA`: `nrows = 1`, `ncols = 1`, cell `A1 = 1`.
  * `GridB`: identical copy (same shape, same value).
  * Each grid is wrapped in a single‑sheet `Workbook` with sheet `"Sheet1"`.
* Behavior:

  * `diff_workbooks(&A, &B)` returns a `DiffReport` whose `ops` is empty.
  * No `Sheet*`, `Row*`, `Column*`, or `CellEdited` operations are emitted for `"Sheet1"`.

#### PG5.2 – 1×1 value change → single CellEdited

* Setup:

  * `GridA`: `A1 = 1`.
  * `GridB`: `A1 = 2`. Same `nrows`, `ncols`.
* Behavior:

  * `ops.len() == 1`.
  * The single op is:

    * `DiffOp::CellEdited { sheet: "Sheet1", addr: "A1", from, to }`,
      with:

      * `from.value == Some(CellValue::Number(1.0))`
      * `to.value == Some(CellValue::Number(2.0))`
  * No row/column structural ops.

#### PG5.3 – Row appended at end → RowAdded only

* Setup:

  * `GridA`: `nrows = 1`, `ncols = 1`, `A1 = 1`.
  * `GridB`: `nrows = 2`, `ncols = 1`, `A1 = 1`, `A2 = 2`.
* Behavior:

  * `ops.len() == 1`.
  * The single op is:

    * `DiffOp::RowAdded { sheet: "Sheet1", row_idx: 1, row_signature: None }`

      * Row indices are 0‑based (`A1` → row 0, `A2` → row 1).
  * No `CellEdited` for `A1`, and no `CellEdited` for the new `A2`.

#### PG5.4 – Column appended at end → ColumnAdded only

* Setup:

  * `GridA`: `nrows = 2`, `ncols = 1`, cells `A1`, `A2`.
  * `GridB`: `nrows = 2`, `ncols = 2`, columns `A` and `B`:

    * Column A identical to `GridA`.
    * Column B has new values (e.g., `B1`, `B2`).
* Behavior:

  * `ops` contains exactly one structural op:

    * `DiffOp::ColumnAdded { sheet: "Sheet1", col_idx: 1, col_signature: None }`

      * Column indices are 0‑based (`A` → 0, `B` → 1).
  * No `CellEdited` ops for any of the existing `A1`, `A2` cells.

#### PG5.5 – Same shape, multiple cell edits, no structure change

* Setup:

  * `GridA`: `nrows = 3`, `ncols = 3`, values 1..9 laid out row‑major.
  * `GridB`: same shape, but three scattered cells changed (e.g., `A1`, `B2`, `C3`).
* Behavior:

  * Exactly three `DiffOp::CellEdited` operations:

    * Each for one of the changed addresses.
    * Old/new values correct.
  * No `Row*` or `Column*` ops.

#### PG5.6 – Degenerate grids

* Case 1 – empty vs empty:

  * `GridA`: `nrows = 0`, `ncols = 0`.
  * `GridB`: `nrows = 0`, `ncols = 0`.
  * Behavior:

    * `ops` is empty.
* Case 2 – empty vs 1×1:

  * `GridA`: `nrows = 0`, `ncols = 0`.
  * `GridB`: `nrows = 1`, `ncols = 1`, `A1 = 1`.
  * Behavior:

    * `ops` contains:

      * `DiffOp::RowAdded    { sheet: "Sheet1", row_idx: 0, row_signature: None }`
      * `DiffOp::ColumnAdded { sheet: "Sheet1", col_idx: 0, col_signature: None }`
    * No `CellEdited` ops at all (the new cell is implied by the structural operations for this
      milestone).

---

## 3. Constraints

### 3.1 Performance and complexity

* The implementation is allowed to remain **O(nrows × ncols)** for the overlapping rectangle,
  plus **O(nrows + ncols)** for tail handling. This is acceptable for the tiny PG5 grids.
* No new dense `nrows × ncols` buffers or matrices may be introduced; the algorithm must continue to
  operate directly over the sparse `Grid` representation (HashMap of `(row, col) -> Cell`).

### 3.2 Memory and allocation

* Tail handling must reuse existing `Grid` metadata (`nrows`, `ncols`) and must not allocate
  temporary arrays proportional to the full grid size.
* Only minimal additional heap allocations are allowed (the new `DiffOp` instances themselves).

### 3.3 Invariants

* `CellEdited` invariants established by existing tests must remain true:

  * `addr`, `from.addr`, and `to.addr` must all match and represent the same cell.
* `Row*` and `Column*` invariants remain as per existing PG4 tests:

  * Indices within bounds of the **relevant grid** (`row_idx < nrows_new` for added rows, etc.).
  * `row_signature`/`col_signature` is **optionally** present; in this milestone it is always `None`.
* Sheet‑level diff invariants must be preserved:

  * Sheet add/remove/rename behavior remains unchanged.
  * No grid ops are emitted for sheets that are added or removed wholesale (grid diff only runs when
    both old and new sheets exist with the same key).

---

## 4. Interfaces

### 4.1 Public APIs whose behavior changes

* `excel_diff::diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport`

  * Previously: For all grid differences, effectively only emitted `CellEdited` ops (no structural
    row/column ops for simple tail changes).
  * After this milestone:

    * For tail append/truncate scenarios in spreadsheet mode, will emit:

      * `RowAdded`/`RowRemoved` and/or `ColumnAdded`/`ColumnRemoved` operations, as specified above.
    * Scattered edits in a fixed‑shape grid still produce only `CellEdited` ops.
* `excel_diff::DiffOp` and `DiffReport`

  * Type definitions unchanged.
  * JSON representation unchanged; structural variants are already covered by PG4 tests.
  * New code paths will instantiate `Row*` and `Column*` variants during actual workbook diffs.

### 4.2 Interfaces intentionally unchanged

* `DiffOp` JSON schema and versioning (`DiffReport::SCHEMA_VERSION`).
* `diff_report_to_cell_diffs` and JSON helpers:

  * These already filter down to `CellEdited` operations, and will simply ignore the new row/column
    ops. Existing JSON cell‑diff tests must continue to pass unchanged.
* `Grid`, `Cell`, `CellValue`, `CellSnapshot`, `RowSignature`, `ColSignature` layouts and semantics.

---

## 5. Test Plan

All tests in this section are **must‑have** additions for this cycle.

### 5.1 New test module: `core/tests/pg5_grid_diff_tests.rs`

#### 5.1.1 Test helpers

* Define a helper to build a grid of numeric cells:

  * `fn grid_from_numbers(values: &[[i32]]) -> Grid`

    * `nrows = values.len() as u32`
    * `ncols = if nrows == 0 { 0 } else { values[0].len() as u32 }`
    * For each `(row, col)`:

      * Create `Cell` with:

        * `row`, `col`
        * `address = CellAddress::from_indices(row, col)`
        * `value = Some(CellValue::Number(v as f64))`
        * `formula = None`
      * Insert into `Grid::new(nrows, ncols)`.
* Define a helper to wrap a grid into a single‑sheet workbook:

  * `fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook`

    * `Workbook { sheets: vec![Sheet { name: name.to_string(), kind: SheetKind::Worksheet, grid }] }`

These helpers are test‑only and live in `pg5_grid_diff_tests.rs`. They should not be exposed in the
library API.

#### 5.1.2 Tests

1. `pg5_1_grid_diff_1x1_identical_empty_diff`

   * Build `GridA = grid_from_numbers(&[[1]])`, `GridB = grid_from_numbers(&[[1]])`.
   * Wrap in workbooks and call `diff_workbooks`.
   * Assert `report.ops.is_empty()`.

2. `pg5_2_grid_diff_1x1_value_change_single_cell_edited`

   * `GridA = [[1]]`, `GridB = [[2]]`.
   * Assert:

     * `report.ops.len() == 1`.
     * Match on `DiffOp::CellEdited { sheet, addr, from, to }`:

       * `sheet == "Sheet1"`.
       * `addr.to_a1() == "A1"`.
       * `from.value == Some(CellValue::Number(1.0))`.
       * `to.value == Some(CellValue::Number(2.0))`.

3. `pg5_3_grid_diff_row_appended_row_added_only`

   * `GridA = [[1]]` (1×1).
   * `GridB` built manually or via helper: `nrows = 2`, `ncols = 1`, cells `[1, 2]`.
   * Assert:

     * `report.ops.len() == 1`.
     * The op is `DiffOp::RowAdded { sheet, row_idx, row_signature }`:

       * `sheet == "Sheet1"`.
       * `row_idx == 1`.
       * `row_signature.is_none()`.

4. `pg5_4_grid_diff_column_appended_column_added_only`

   * `GridA`: `nrows = 2`, `ncols = 1`, column A values `[1, 2]`.
   * `GridB`: `nrows = 2`, `ncols = 2`, column A `[1, 2]`, column B `[10, 20]`.
   * Assert:

     * Exactly one `DiffOp::ColumnAdded { sheet, col_idx, col_signature }`:

       * `sheet == "Sheet1"`.
       * `col_idx == 1`.
       * `col_signature.is_none()`.
     * No `CellEdited` ops.

5. `pg5_5_grid_diff_same_shape_scattered_cell_edits`

   * `GridA`: 3×3 with values 1..9 row‑major.
   * `GridB`: same except change three cells (e.g., `A1`, `B2`, `C3`) to distinct values.
   * Assert:

     * `report.ops.len() == 3`.
     * All ops are `DiffOp::CellEdited`.
     * `addr.to_a1()` set equals the set of edited addresses.
     * No `Row*` or `Column*` ops.

6. `pg5_6_grid_diff_degenerate_grids`

   * Case 1: empty vs empty:

     * `GridA = Grid::new(0, 0)` with no cells.
     * `GridB = Grid::new(0, 0)` with no cells.
     * Assert `report.ops.is_empty()`.
   * Case 2: empty vs 1×1:

     * `GridA = Grid::new(0, 0)`.
     * `GridB = grid_from_numbers(&[[1]])`.
     * Assert:

       * `report.ops.len() == 2`.
       * One `RowAdded` with `row_idx == 0`, `row_signature.is_none()`.
       * One `ColumnAdded` with `col_idx == 0`, `col_signature.is_none()`.
       * No `CellEdited` ops.

### 5.2 Regression coverage

* Re‑run and keep passing:

  * `core/tests/engine_tests.rs` (1×1 workbook tests).
  * `core/tests/pg4_diffop_tests.rs` (DiffOp construction/JSON).
  * `core/tests/output_tests.rs` (JSON diff for simple fixtures).
  * `core/tests/signature_tests.rs`, `core/tests/sparse_grid_tests.rs`.

These ensure that introducing row/column structural ops in `diff_grids` does not break the existing
public JSON interfaces or basic IR invariants.

---

**References for this plan (not part of the files themselves):**

* PG5/PG6 and Phase 2 description in the testing plan. 
* Unified grid diff algorithm specification and its emphasis on structural row/column ops and H1 difficulty.
* Core IR and diff pipeline overview in the main specification. 
* Current `diff_grids` implementation and DiffOp definitions/tests in the codebase context.
* Difficulty analysis marking the grid diff engine as H1 (hardest component). 
