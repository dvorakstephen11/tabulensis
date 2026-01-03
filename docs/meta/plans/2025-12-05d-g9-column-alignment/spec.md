# 2025-12-05-g9-column-alignment – Single-column alignment for mid-sheet insert/delete (G9)

## 1. Scope

### 1.1 Modules and files in play

**Core Rust modules**

- `core/src/grid_view.rs`
  - Reuse existing `GridView`, `RowMeta`, and `ColMeta`.
  - Extend `HashStats` to support column-level statistics (either via a new
    constructor for `ColHash` or a generic helper). 
- `core/src/row_alignment.rs`
  - Serves as a design reference for gating, statistics usage, and integration
    with `diff_grids`; no functional changes expected. :contentReference[oaicite:19]{index=19}
- `core/src/engine.rs`
  - Extend `diff_grids` with a column-alignment fast path analogous to the
    existing row-alignment path:
    - Call into a new `align_single_column_change` helper when appropriate.
    - Use the resulting column mapping for cell comparison and
      `ColumnAdded/ColumnRemoved` emission. 
- New module: `core/src/column_alignment.rs` (or similar)
  - Houses `ColumnAlignment` and `align_single_column_change`.
  - Strictly internal (`pub(crate)`).

**Tests**

- New integration test file:
  - `core/tests/g9_column_alignment_grid_workbook_tests.rs`
- Possible small unit test file:
  - `core/tests/column_alignment_tests.rs` (optional but recommended for
    direct tests of `ColumnAlignment` logic).

**Fixtures and generators**

- `fixtures/manifest.yaml`
  - Add new entries for G9 workbook pairs:
    - `col_insert_middle_{a,b}.xlsx`
    - `col_delete_middle_{a,b}.xlsx`
    - `col_insert_with_edit_{a,b}.xlsx` (fallback-gating scenario)
- `fixtures/src/generators/grid.py`
  - Extend the existing grid generator module with a
    `ColumnAlignmentG9Generator` (or equivalent) that mirrors the patterns used
    for `RowAlignmentG8Generator`. 

### 1.2 In-scope behavior

- Spreadsheet-mode comparison of small, dense-ish grids where:
  - Row counts are equal.
  - Column counts differ by exactly 1 (one column inserted or deleted).
  - Total rows ≤ `MAX_ALIGN_ROWS` (currently 2000).
  - Total columns ≤ `MAX_ALIGN_COLS` (currently 64 for G8; reuse or align
    with that constant). 
- Only **single** column insert/delete in the middle of the sheet for now.
- Workbook-level diff via `diff_workbooks` when both workbooks contain a
  single relevant sheet of this form.

### 1.3 Out of scope (for this cycle)

- Multiple column inserts/deletes in one diff.
- Column reordering (moves) or rectangular block moves (G12 / UC‑12 / UC‑13).
- Column-first dimension ordering in general; we implement a special-case
  column-alignment fast path, not the full adaptive dimension-ordering engine. 
- Database-mode integration or key-based alignment.
- Adversarial large-grid performance cases (G8a / P1 / P2); we keep this cycle
  focused on correctness and simple gating for small grids. 

---

## 2. Behavioral Contract

The goal is to make **single mid-sheet column insert/delete** behave as cleanly
for columns as G8 does for rows: one structural op, no spurious changes.

### 2.1 Canonical success cases (G9 fixtures)

#### 2.1.1 `col_insert_middle_{a,b}.xlsx` – Column inserted between C and D

Fixture sketch from the testing plan: :contentReference[oaicite:25]{index=25}

- Workbook A:
  - One sheet (e.g., `"Data"`), columns A–H.
  - Header row with stable labels.
  - Several data rows with stable values.
- Workbook B:
  - Same as A, except:
    - One new column inserted between C and D (indexing is IR-level 0-based;
      exact numerical value is covered by tests).
    - Data in the new column may be distinct but does not alter other columns.

**Expected diff behavior:**

- At sheet level:
  - Exactly one `DiffOp::ColumnAdded { sheet, col_idx, col_signature }`:
    - `sheet` identifies the same sheet in both workbooks.
    - `col_idx` corresponds to the inserted column’s index in grid B.
    - `col_signature` (if populated) matches the hash of the inserted column.
- No `RowAdded` or `RowRemoved` ops for this sheet.
- No `CellEdited` operations for cells in unaffected columns:
  - Cells in columns A–C compare at the same indices.
  - Cells in columns after the insertion (original D–H) compare to their
    shifted counterparts:
    - Conceptually: original column D (A) is compared with column E (B), etc.
- Cells in the new column are not compared to anything (they are purely
  additions); they appear only as part of the `ColumnAdded` structure. 

#### 2.1.2 `col_delete_middle_{a,b}.xlsx` – Column D deleted

Fixture sketch from the testing plan: :contentReference[oaicite:27]{index=27}

- Workbook A:
  - Same as in `col_insert_middle`, columns A–H.
- Workbook B:
  - Same header row and data, except column D is removed (columns E–H shift
    left).

**Expected diff behavior:**

- Exactly one `DiffOp::ColumnRemoved { sheet, col_idx, col_signature }`:
  - `col_idx` corresponds to the deleted column’s index in grid A.
- No `RowAdded` or `RowRemoved`.
- No `CellEdited` in unaffected data:
  - Columns A–C line up.
  - Columns after D in A compare correctly with their shifted positions in B.
- No `ColumnAdded` for this sheet.

### 2.2 Fallback / bail-out case

#### 2.2.1 `col_insert_with_edit_{a,b}.xlsx` – Insert plus additional edits

This mirrors `row_insert_with_edit_{a,b}.xlsx` for G8: a case that **should
not** use the alignment fast path. 

Fixture sketch:

- Start from the same base workbook used for `col_insert_middle`.
- Workbook A: baseline columns A–H, stable data.
- Workbook B:
  - Insert a new column between C and D.
  - Additionally change one or more cells in columns after the insertion
    (e.g., tweak a value in original column F in a row below the insertion).

**Expected diff behavior:**

- The column-alignment path must **bail out**:
  - No `ColumnAdded` or `ColumnRemoved` at the interior insertion index.
- Diff falls back to the existing positional PG5 semantics for this sheet:
  - It is acceptable (and expected) for this fallback to emit:
    - At least one `ColumnAdded`/`ColumnRemoved` at the right edge **or**
      a pattern of `CellEdited` reflecting positional misalignment.
- The tests will assert:
  - The diff **does not** contain a `ColumnAdded` at the middle insert index
    (which would imply the alignment path ran).
  - There is at least one `CellEdited` in columns after the insertion,
    confirming we are seeing the positional baseline rather than an aligned,
    “clean” result.

This mirrors G8’s “insert_with_edit” guard: alignment is only used when the
change is a pure structural insert/delete; otherwise, we prefer a noisy but
safe diff to a misleading clean one. 

---

## 3. Constraints and invariants

### 3.1 Performance and size limits

- Only apply column alignment when:
  - `max(old.nrows, new.nrows) ≤ MAX_ALIGN_ROWS` (current G8 bound: 2000).
  - `max(old.ncols, new.ncols) ≤ MAX_ALIGN_COLS` (current G8 bound: 64).
  - Row counts are equal.
  - Column count difference is exactly 1. 
- `GridView::from_grid` must remain an ephemeral view:
  - No persistent allocation beyond the diff operation.
  - Reuse GridView for both row and column alignment; no duplicate materialization.

### 3.2 Hashing and statistics

- Column alignment must use the existing hashing strategy:
  - Column hashes are sensitive to row index and cell content as defined in the
    unified spec (position-dependent XXHash64). :contentReference[oaicite:31]{index=31}
- Extend `HashStats` for column hashes:
  - Add `HashStats<ColHash>::from_col_meta(&[ColMeta], &[ColMeta])`.
  - Keep frequency semantics consistent with row-hash usage:
    - `is_unique` means “appears exactly once in each grid”.
    - `is_rare` / `is_common` semantics reuse the same thresholds as G8 unless
      tests prove otherwise. 
- Do not attempt full classification (rare/common/low-info) for all advanced
  paths yet; for this milestone we only need enough stats to:
  - Identify a single uniquely inserted/deleted column.
  - Detect heavy repetition in column hashes and bail out if necessary.

### 3.3 Alignment invariants

- The internal `ColumnAlignment` structure should mirror `RowAlignment`:
  - Monotonic `matched` pairs: for any `(a_i, b_i)` and `(a_j, b_j)` with
    `i < j`, we must have `a_i < a_j` and `b_i < b_j`. :contentReference[oaicite:33]{index=33}
  - `inserted` and `deleted` lists contain column indices in strictly
    increasing order for each grid.
- For this milestone:
  - Exactly one column is inserted **or** exactly one column is deleted.
  - Mixed insert/delete is not supported; bail out in those cases.

### 3.4 Diff invariants

- `DiffOp::ColumnAdded` and `DiffOp::ColumnRemoved` semantics remain unchanged:
  - `col_idx` is always expressed in the coordinate system of the grid where
    the column appears (A = old, B = new).
  - `col_signature`, when present, corresponds to the hash of the column as
    produced by `ColMeta::hash`.
- `CellEdited` invariants from PG4 still apply:
  - `addr` must be a valid cell address.
  - Snapshots’ `(value, formula)` equality semantics remain unchanged. 

---

## 4. Interfaces and data flow

### 4.1 New internal types

**`ColumnAlignment` (new, internal)**

- Location: `core/src/column_alignment.rs`.
- Shape (conceptual):

  ```rust
  pub(crate) struct ColumnAlignment {
      pub(crate) matched: Vec<(u32, u32)>,  // (col_idx_a, col_idx_b)
      pub(crate) inserted: Vec<u32>,        // cols present only in B
      pub(crate) deleted: Vec<u32>,         // cols present only in A
  }
````

* For G9:

  * `matched` contains pairs for all columns except the single inserted or
    deleted column.
  * Either `inserted.len() == 1 && deleted.is_empty()` or vice versa.

**`align_single_column_change` (new, internal)**

* Signature (conceptual):

  ```rust
  pub(crate) fn align_single_column_change(
      grid_a: &Grid,
      grid_b: &Grid,
  ) -> Option<ColumnAlignment>;
  ```

* Responsibilities:

  * Build `GridView` for both grids (or accept an existing view if the engine
    chooses to share it).
  * Check size bounds and shape constraints (equal rows, columns differ by 1).
  * Build `HashStats<ColHash>` from `view_a.col_meta` and `view_b.col_meta`.
  * Identify a unique inserted/deleted column using hash frequencies.
  * Construct a monotonic `matched` mapping plus `inserted`/`deleted` indices.
  * Apply guards:

    * Bail out if multiple columns appear unique or if heavy repetition is
      detected (e.g., max column-hash frequency exceeds a small threshold).
    * Bail out if column count difference != 1.

### 4.2 Engine integration

**`diff_grids` flow (spreadsheet mode)**

High-level desired flow (simplified):

1. Build or reuse `GridView` for both grids.
2. Attempt row alignment first (existing G8 behavior):

   * If `align_single_row_change` returns `Some(_)`, use the row-aligned path
     and skip column alignment.
3. If row alignment returns `None`:

   * Attempt `align_single_column_change`.
   * If this returns `Some(column_alignment)` and the sheet is within size and
     repetition bounds:

     * Use a new helper `emit_column_aligned_diffs` to:

       * Emit the appropriate `ColumnAdded` or `ColumnRemoved`.
       * Compare cells using the column mapping (analogous to
         `emit_aligned_diffs` for rows).
   * If `align_single_column_change` returns `None`:

     * Fall back to existing positional PG5 behavior (`positional_diff`).

**`emit_column_aligned_diffs` (new helper)**

* Similar to `emit_aligned_diffs` but for columns:

  * Iterate rows by index.
  * For each aligned pair of columns `(col_a, col_b)`:

    * Compare cells and emit `CellEdited` only for true content differences.
  * Emit a single `ColumnAdded` or `ColumnRemoved` based on the alignment.
* Must preserve existing behavior for:

  * Equal-shape sheets (pure cell edits).
  * Tail-only column adds/deletes (G5/G7 tests). These still use the
    positional path; the column-alignment path is only for mid-sheet single
    changes and must not interfere with existing tail semantics.

### 4.3 Public API stability

* No changes to public types or functions:

  * `diff_workbooks`, `DiffReport`, and `DiffOp` remain unchanged at the API
    level.
* Behavioral change is **only** in which DiffOps are emitted for small,
  mid-sheet single column insert/delete cases.

---

## 5. Test Plan

This cycle is complete when the following tests are implemented and passing.

### 5.1 Python fixture generation

Extend `fixtures/manifest.yaml` with new entries (IDs illustrative; names must
match tests):

* `col_insert_middle`

  * `kind: excel_pair`
  * `generator: "grid:column_alignment_g9"`
  * `output`:

    * `col_insert_middle_a.xlsx`
    * `col_insert_middle_b.xlsx`
* `col_delete_middle`

  * Similar outputs:

    * `col_delete_middle_a.xlsx`
    * `col_delete_middle_b.xlsx`
* `col_insert_with_edit`

  * Outputs:

    * `col_insert_with_edit_a.xlsx`
    * `col_insert_with_edit_b.xlsx`

Generator behavior (`ColumnAlignmentG9Generator` in `fixtures/src/generators/grid.py`):

* Build a base 10×8 grid (headers in row 1, numbers or simple strings in
  rows 2–10), similar to the G8 generator’s base-table design.
* For `col_insert_middle`:

  * Workbook A: baseline.
  * Workbook B: insert a new column between C and D; fill with deterministic
    values (e.g., `Inserted_1 … Inserted_9` under a header like `"Inserted"`).
* For `col_delete_middle`:

  * Workbook A: baseline.
  * Workbook B: remove column D entirely.
* For `col_insert_with_edit`:

  * Workbook B: same as `col_insert_middle`, plus:

    * Modify at least one cell in a column to the right of the inserted column
      (e.g., change a value in original column F, row 8).

### 5.2 Rust integration tests

New file: `core/tests/g9_column_alignment_grid_workbook_tests.rs`.

Use the existing helpers:

* `open_workbook` and `diff_workbooks` from the public API.
* `fixture_path` from `tests/common`.

#### 5.2.1 `g9_col_insert_middle_emits_one_columnadded_and_no_noise`

* Open `col_insert_middle_a.xlsx` and `col_insert_middle_b.xlsx`.
* Run `diff_workbooks`.
* Assertions:

  * Exactly one `DiffOp::ColumnAdded { .. }` for the sheet under test.
  * No `DiffOp::ColumnRemoved`, `RowAdded`, or `RowRemoved` for that sheet.
  * No `DiffOp::CellEdited` for that sheet.
* Optional: assert that the column index for `ColumnAdded` corresponds to the
  inserted position by comparing against the header row (e.g., ensure the
  inserted header text matches the expected column).

#### 5.2.2 `g9_col_delete_middle_emits_one_columnremoved_and_no_noise`

* Open `col_delete_middle_{a,b}.xlsx` and diff them.
* Assertions:

  * Exactly one `DiffOp::ColumnRemoved { .. }`.
  * No `ColumnAdded`, `RowAdded`, `RowRemoved`, or `CellEdited` for that sheet.

#### 5.2.3 `g9_alignment_bails_out_when_additional_edits_present`

* Open `col_insert_with_edit_{a,b}.xlsx`.
* Run `diff_workbooks`.
* Assertions:

  * There is **no** `DiffOp::ColumnAdded` whose `col_idx` matches the
    interior insertion index (the implementer can derive this index from the
    header row; the test can compute it by scanning row 1).
  * There is at least one `DiffOp::CellEdited` in a column to the right of the
    inserted column, indicating that we fell back to positional semantics.
  * Optionally, assert that the set of operations is not empty.

This mirrors the existing G8 fallback test and codifies the expectation that
column alignment is used only for pure structural edits.

### 5.3 ColumnAlignment unit tests

New file: `core/tests/column_alignment_tests.rs` (small, focused):

1. **`single_insert_aligns_all_columns`**

   * Build two small in-memory `Grid`s where one has a column inserted in the
     middle with unique hash.
   * Call `align_single_column_change` directly.
   * Assert:

     * `Some(ColumnAlignment)` returned.
     * `inserted.len() == 1`, `deleted.is_empty()`.
     * `matched` covers all other columns and is strictly monotonic.

2. **`multiple_unique_columns_causes_bailout`**

   * Construct grids with two independent column inserts/deletes so that more
     than one column could be considered unique.
   * Assert `align_single_column_change` returns `None`.

3. **`heavy_repetition_causes_bailout` (optional)**

   * Build grids with many identical columns (same hash repeated above the
     repetition threshold).
   * Assert `align_single_column_change` returns `None`, exercising the
     repetition guard.

### 5.4 Regression guard – existing tests

The implementer must re-run the full test suite and ensure the following
existing tests remain green:

* `core/tests/g5_g7_grid_workbook_tests.rs`:

  * Tail row/column append/delete behavior (PG5/PG7).
* `core/tests/g8_row_alignment_grid_workbook_tests.rs`:

  * Middle row insert/delete and G8 fallback behavior (to ensure the new
    column path does not interfere with row alignment).
* `core/tests/pg5_grid_diff_tests.rs`:

  * Positional diff semantics for equal-shape grids and mixed content/shape
    interactions.

No changes are expected to these tests; they act as regression guards to
ensure the column-alignment path is strictly additive and correctly gated.

---

## 6. Follow-ups (explicitly out-of-scope work)

These are **not** part of this cycle but should be noted for future planning:

* Extend column alignment from “single column insert/delete” to:

  * Multiple columns.
  * Column moves (UC‑13, G12) with `BlockMovedColumns`.
* Integrate column alignment into the full adaptive dimension-ordering
  pipeline (column-first mode with row-hash recomputation).
* Implement G8a adversarial repetitive-grid tests (`adversarial_grid_repetitive_*`)
  once row + column alignment both exist for small grids.
* Database-mode column stability and key inference milestones (D1–D10).

For this mini-spec, success is defined narrowly: a clean, fixture-backed G9
column alignment path that mirrors G8’s small-grid row alignment and leaves
all existing tests passing.

