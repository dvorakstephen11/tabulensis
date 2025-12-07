## 1. Scope

### 1.1 Rust modules

This cycle touches a narrow set of modules focused on database-mode row alignment and its tests:

* `core/src/lib.rs`

  * `pub fn diff_grids_database_mode(old: &Grid, new: &Grid, key_columns: &[u32]) -> DiffReport`
* `core/src/database_alignment.rs`

  * `KeyColumnSpec`
  * `KeyValue` / `KeyValueRepr`
  * `KeyedAlignment`
  * `DatabaseAlignmentError`
  * `diff_table_by_key`
* `core/tests/d1_database_mode_tests.rs`

  * Existing D1 tests for single-column keys and duplicate-key fallback.
  * New D5-focused composite-key tests will be added here to keep all D-series engine-level tests in one place.

No changes are planned to:

* Spreadsheet-mode alignment (`diff_grids` and the G8–G13 move detection logic).
* Public workbook-level entrypoints (`diff_workbooks`).
* M/DataMashup or DAX subsystems.

Database Mode remains an engine-only capability, exercised directly via `diff_grids_database_mode` in tests, consistent with the existing D1 coverage.

### 1.2 Milestone alignment

The work explicitly targets **D5 – Composite primary key** in the Phase 4 testing plan:

* Key is a multi-column combination (e.g. `[Country, CustomerID]`).
* Tests must cover:

  * A new composite key pair producing a `RowAdded`.
  * A changed non-key field for an existing composite key pair producing a `CellEdited`.
  * No false matches when only part of the composite key matches. 

D6 (duplicate key clusters), D7–D9 (key priority and inference), and D10 (mixed sheet semantics) are explicitly **out of scope** for this cycle.

## 2. Behavioral Contract

All examples below describe the behavior of:

```text
diff_grids_database_mode(&grid_a, &grid_b, &[0, 1])
```

where columns 0 and 1 form the composite key, and all remaining columns are treated as non-key data fields. The `sheet` identity in `DiffOp` for these tests remains the synthetic `DATABASE_MODE_SHEET_ID` used today. 

### 2.1 Composite keys with reorder only → empty diff

**Grid A**

```text
Row 0: [1, 10, 100]
Row 1: [1, 20, 200]
Row 2: [2, 10, 300]
```

**Grid B** (same rows, different order)

```text
Row 0: [2, 10, 300]
Row 1: [1, 10, 100]
Row 2: [1, 20, 200]
```

Key columns: `[0, 1]`
Non-key column: `2`

**Contract**

* The diff report is empty:

  * No `RowAdded` / `RowRemoved`.
  * No `CellEdited`.
* Row order differences are completely ignored; alignment is based solely on the composite key `(col0, col1)`.

This generalizes the existing D1 reordering behavior (single-column key) to composite keys.

### 2.2 New composite key pair + non-key change → RowAdded + CellEdited

**Grid A**

```text
Row 0: [1, 10, 100]   # key (1,10), value 100
Row 1: [1, 20, 200]   # key (1,20), value 200
```

**Grid B**

```text
Row 0: [1, 10, 150]   # same key (1,10), changed non-key value
Row 1: [1, 20, 200]   # same key (1,20), unchanged
Row 2: [2, 30, 300]   # new key (2,30)
```

Key columns: `[0, 1]`
Non-key column: `2`

**Contract**

* Exactly **one** `RowAdded` operation for the new composite key `(2,30)`.
* Exactly **one** `CellEdited` operation for the changed non-key value on key `(1,10)`.
* **No** `RowRemoved` operations.
* The `CellEdited` must:

  * Reference the row in B where `(1,10)` lives (row index determined by alignment, not necessarily 0).
  * Use the non-key column index (2) for `addr.col`.
  * Leave key columns untouched (no `CellEdited` for key columns that are unchanged).

### 2.3 Partial key match → RowRemoved + RowAdded, no CellEdited

**Grid A**

```text
Row 0: [1, 10, 100]   # key (1,10)
```

**Grid B**

```text
Row 0: [1, 20, 100]   # key (1,20)
```

Key columns: `[0, 1]`
Non-key column: `2` (unchanged)

**Contract**

* Exactly **one** `RowRemoved` for `(1,10)` (present only in A).
* Exactly **one** `RowAdded` for `(1,20)` (present only in B).
* **No** `CellEdited` operations.

This enforces **tuple semantics** for composite keys: a row must match on **all** key columns to be treated as the same logical entity. Sharing only the first component (`1`) is not sufficient to classify the change as a non-key edit.

### 2.4 Duplicate composite keys remain spreadsheet-mode fallback

Existing behavior for duplicate keys is preserved:

* If any composite key value appears more than once on either side, `diff_grids_database_mode` continues to fall back to spreadsheet-mode `diff_grids`, producing positional row/column operations instead of database-mode keyed alignment.
* This cycle does **not** introduce a `DuplicateKeyCluster`-style `DiffOp`; that is reserved for D6.

The D1 `d1_duplicate_keys_fallback_to_spreadsheet_mode` test remains the canonical behavior for both single-column and composite keys encountering duplicates.

## 3. Constraints & Invariants

### 3.1 Performance and complexity

* Composite-key alignment must preserve the existing expected complexity of **O(N)** hash-join on keys for database mode, where N is the number of rows. 
* Using multiple key columns:

  * Continues to use a single `KeyValue` object per row as a hash key (a small `Vec<KeyComponent>` or equivalent), not a separate map per key column.
  * Does **not** introduce additional per-row allocations beyond what `KeyValue` already does.
* The D5 tests will use small synthetic grids; no performance micro-benchmarking is required in this cycle, but no algorithmic changes are allowed that would obviously degrade large-table behavior.

### 3.2 Memory and data model invariants

* `KeyColumnSpec` continues to be the single source of truth for which columns are considered key columns. It must correctly handle arbitrary (but small) key column lists such as `[0, 1]`, `[2, 4]`, etc. 
* `KeyValue` equality and hashing must treat the key as an **ordered tuple** of components; no re-ordering or partial matching is permitted.
* No changes to:

  * `DiffOp` enum shape (no new variants introduced in this branch).
  * `DiffReport` structure or JSON serialization contracts. 

### 3.3 Mode and fallback invariants

* `diff_grids_database_mode` continues to:

  * Attempt a keyed alignment via `diff_table_by_key`.
  * On any `DatabaseAlignmentError` (including duplicate keys), fall back to spreadsheet mode by calling `diff_grids` with a synthetic sheet id.
* This cycle **must not** alter what constitutes an error in the alignment layer; duplicate key handling beyond fallback is deferred to D6.

## 4. Interfaces

No public API signatures change in this cycle.

### 4.1 `diff_grids_database_mode`

Existing signature (unchanged):

```rust
pub fn diff_grids_database_mode(old: &Grid, new: &Grid, key_columns: &[u32]) -> DiffReport
```

Clarified contract (documentation-level):

* `key_columns` may contain one or more column indices.
* When `key_columns.len() > 1`, row identity is determined by the composite tuple of key column values, in order.
* Key columns are **not** reported as edited when their values are unchanged; edits are restricted to non-key columns in database mode, consistent with UC-17/UC-18 semantics.

### 4.2 `KeyColumnSpec` and `diff_table_by_key`

`KeyColumnSpec` and `diff_table_by_key` remain internal to the core crate, but their behavior is clarified and locked via tests:

* `KeyColumnSpec::new(columns: Vec<u32>)`:

  * Must preserve the order of the given column indices.
  * `is_key_column(col)` returns true iff `col` appears in `columns`.
* `diff_table_by_key(old: &Grid, new: &Grid, key_columns: &[u32])`:

  * Must support `key_columns.len() >= 1`.
  * For composite keys:

    * Unique composite key values align 1:1.
    * Rows missing in A or B are surfaced via `left_only_rows` / `right_only_rows`.
    * Matched rows are surfaced via `matched_rows` as `(row_a, row_b)` pairs.

## 5. Test Plan

All tests are **engine-level** Rust tests; no new `.xlsx` fixtures are required for this cycle. Workbook-level fixtures for composite keys can be added in a later integration-focused cycle.

### 5.1 New D5 tests in `core/tests/d1_database_mode_tests.rs`

Extend the existing D-series test file to cover composite keys, reusing the `grid_from_numbers` helper and the `DiffOp`-based assertions already used for D1.

#### Test 1 – `d5_composite_key_equal_reordered_database_mode_empty_diff`

**Setup**

```rust
let grid_a = grid_from_numbers(&[
    &[1, 10, 100],
    &[1, 20, 200],
    &[2, 10, 300],
]);

let grid_b = grid_from_numbers(&[
    &[2, 10, 300],
    &[1, 10, 100],
    &[1, 20, 200],
]);

let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);
```

**Assertions**

* `report.ops.is_empty()` (no structural or cell operations).
* Optional: assert that adding/removing an extra non-key column would still keep this test meaningful if extended later.

**Purpose**

* Proves that composite-key alignment ignores row order differences, generalizing D1’s single-column-key behavior.

#### Test 2 – `d5_composite_key_row_added_and_cell_edited`

**Setup**

```rust
let grid_a = grid_from_numbers(&[
    &[1, 10, 100],
    &[1, 20, 200],
]);

let grid_b = grid_from_numbers(&[
    &[1, 10, 150], // non-key value changed
    &[1, 20, 200],
    &[2, 30, 300], // new composite key
]);

let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);
```

**Assertions**

* Count of `RowAdded` ops is exactly 1.
* Count of `RowRemoved` ops is 0.
* Count of `CellEdited` ops is exactly 1.
* The `CellEdited`:

  * Has `addr.col == 2` (non-key column).
  * References the row index in B corresponding to key `(1,10)`.

**Purpose**

* Confirms that:

  * New composite key pairs are surfaced as `RowAdded`.
  * Non-key field changes on existing composite keys are surfaced as `CellEdited`.
  * Key columns themselves are not treated as editable fields when unchanged.

#### Test 3 – `d5_composite_key_partial_key_mismatch_yields_add_and_remove`

**Setup**

```rust
let grid_a = grid_from_numbers(&[
    &[1, 10, 100],
]);

let grid_b = grid_from_numbers(&[
    &[1, 20, 100],
]);

let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);
```

**Assertions**

* Count of `RowRemoved` ops is exactly 1.
* Count of `RowAdded` ops is exactly 1.
* Count of `CellEdited` ops is 0.

**Purpose**

* Proves that composite-key matching uses the **full tuple** `(col0, col1)`:

  * Changing any key component is treated as a remove+add, not as a cell edit.
* Guards against any accidental implementation that only keys on the first column. 

### 5.2 New unit tests in `core/src/database_alignment.rs`

Add or extend a `#[cfg(test)]` module to lock in composite-key semantics at the alignment layer, independent of `DiffOp` emission.

#### Test 4 – `composite_key_alignment_matches_rows_correctly`

**Setup**

* Build a small `Grid` with three rows and three columns:

  ```text
  A:
    [1, 10, 100]
    [1, 20, 200]
    [2, 10, 300]

  B:
    [1, 20, 200]
    [2, 10, 300]
    [1, 10, 100]
  ```

* Call `diff_table_by_key(&grid_a, &grid_b, &[0, 1])`.

**Assertions**

* `alignment.left_only_rows` is empty.
* `alignment.right_only_rows` is empty.
* `alignment.matched_rows` contains three pairs and each pair joins rows that share the same `(col0, col1)` tuple, regardless of original index.

**Purpose**

* Validates the `KeyValue` and `KeyColumnSpec` behavior for multi-column keys at the lowest layer, independent of `DiffOp` emission.

### 5.3 Regression guard on duplicate-key fallback

To ensure D5 doesn’t unintentionally change duplicate-key behavior before D6:

* Keep the existing `d1_duplicate_keys_fallback_to_spreadsheet_mode` test as-is.

* Optionally add a composite-key variant:

  ```rust
  let grid_a = grid_from_numbers(&[
      &[1, 10, 100],
      &[1, 10, 200], // duplicate composite key
  ]);

  let grid_b = grid_from_numbers(&[
      &[1, 10, 100],
  ]);

  let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);
  ```

* Assert that:

  * `report.ops` is non-empty (spreadsheet-mode diff ran).
  * At least one `RowRemoved` is present.

This ensures the D1/D6 fallback story remains stable while composite key support is hardened.

---

**Out of Scope for this Cycle**

* New `.xlsx` fixtures for `db_composite_key_{a,b}.xlsx`; those can follow in a later integration-focused branch that adds workbook-level D5 tests.
* Introduction of a `DuplicateKeyCluster` `DiffOp` or any in-engine Hungarian matching for clusters (D6 / UC-19).
* Mode selection, key inference, and metadata-driven key priority (D7–D9).
