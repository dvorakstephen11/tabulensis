## 1. Scope

### 1.1 Modules and types in play

**Core engine and IR**

- `core/src/engine.rs`
  - Add a dedicated **Database Mode** entry point that operates on `Grid` values and
    returns a `DiffReport`, reusing existing `DiffOp` variants (`RowAdded`,
    `RowRemoved`, `CellEdited`, etc.).
- New module: `core/src/database_alignment.rs`
  - Implements the keyed row-alignment algorithm for the **unique-key** case only
    (no duplicate-key clusters), following the Database Mode design in the unified
    grid diff spec.
- `core/tests/d1_database_mode_tests.rs` (new)
  - Integration tests that load the D1 fixtures and exercise the new database-mode
    entry point.

**Existing data structures (reused, not redesigned)**

- `Workbook`, `Sheet`, `Grid` IR and related cell types from `core/src/workbook.rs`
  and `core/src/lib.rs`.
- `DiffReport` and `DiffOp` definitions (PG4) from `core/src/diff.rs`.

**Fixtures and generators**

- D1 fixture pair:
  - `fixtures/generated/db_equal_ordered_a.xlsx`
  - `fixtures/generated/db_equal_ordered_b.xlsx`
- Backed by the `db_keyed` / `KeyedTableGenerator` in `fixtures/src/generators/database.py`
  and registered in `fixtures/manifest.yaml`.

### 1.2 Out of scope

- Workbook-level Database Mode wiring (no changes to `excel_diff::diff_workbooks` or
  JSON helpers this cycle).
- Key inference / discovery, composite keys, and duplicate-key cluster matching
  (D5–D9 / UC-19).
- Mixed-mode segmentation within a single sheet (table region in DB mode plus
  surrounding notes in Spreadsheet Mode) – reserved for D10 / UC-20.
- PBIX/PBIT table and DAX support (later phases in the testing plan and product
  roadmap).


## 2. Behavioral contract (D1 / UC-17)

This mini-spec defines behavior for **Database Mode keyed equality** in the D1 test
slice and UC-17 “Keyed rows, same keys different order”.

### 2.1 Database Mode semantics (unique-key slice)

When the caller explicitly invokes the **Database Mode** grid diff with a known key:

- Row identity is determined **by key value**, not by physical row index.
- Row **order is ignored**; pure reordering of keyed rows must not appear as a diff.
- Keys are supplied as a list of zero-based column indices (this cycle only exercises
  single-column keys).
- For D1, the keys are globally unique in both grids; duplicate keys are treated as
  “unsupported for now” and may return a structured internal error in tests.

Row comparison rules for this slice:

- For a given key, if every non-key column’s `(value, formula)` pair matches under
  existing `CellSnapshot` semantics, the row is considered equal.
- If both the set of keys and the non-key cell contents match between `old` and `new`,
  the diff for that table region is **empty** – no `RowAdded`, `RowRemoved`, or
  `CellEdited` ops.

Database Mode is surfaced via a **new engine API** in this cycle and is not wired into
the top-level `diff_workbooks` pipeline yet.

### 2.2 Examples

#### Example 1 – In-memory tiny table, no reorder

- Grid A:
  - Header: `["ID","Name","Amount"]`
  - Rows: `(1,"Alice",10)`, `(2,"Bob",20)`
- Grid B: identical header and rows in the same order.
- Key: column 0 (ID).

Behavior:

- `diff_grids_database_mode(..., key_columns=[0])` returns a `DiffReport` with `ops`
  empty (no structural or cell-level operations).

#### Example 2 – In-memory tiny table, pure reorder

- Same header and row content as Example 1.
- Grid B rows swapped: `(2,"Bob",20)`, `(1,"Alice",10)`.
- Key: column 0 (ID).

Behavior:

- Database Mode diff returns an empty `DiffReport`; row reordering is ignored because
  row identity is keyed. This is the minimal UC-17 case.

#### Example 3 – D1 fixture pair: keyed equality with random row order

- A: `db_equal_ordered_a.xlsx` (IDs 1..N, ordered).
- B: `db_equal_ordered_b.xlsx` (same IDs/rows, randomly permuted).
- Header row and all non-key columns match between A and B.
- Key: column 0 (ID).

Behavior:

- Database Mode diff with `key_columns = [0]` for the primary data sheet:
  - `DiffReport.ops` contains no row, column, or cell operations for that sheet.
- Spreadsheet Mode diff (`diff_workbooks`) is allowed to see structural changes;
  the new API is the one required to show **no difference** for D1.


### 2.3 Non-behaviors (explicitly not provided yet)

- No automatic key inference from table metadata or heuristics; D1 tests pass explicit
  key columns.
- No multi-column keys or duplicate-key cluster matching; these belong to later
  database-mode milestones (D5–D9).
- No mixing of Database Mode and Spreadsheet Mode within the same sheet; tests call
  the database-mode API directly on a single `Grid`.


## 3. Constraints and invariants

### 3.1 Performance and complexity

- Unique-key alignment must be implemented as a **hash join** across keys:
  - Build hash maps from key → row index for each grid.
  - Perform a single pass to align rows by key.
- Complexity for the D1 slice:
  - Hash-map construction: O(N) in row count.
  - Cell comparison: O(N × M) for N rows and M columns, reusing existing
    `CellSnapshot` equality rules.
- The algorithm must comfortably handle at least the D1 fixture size and fit within
  the broader performance envelope defined for grid diffing in the unified spec
  (tens of thousands of rows in later phases).

### 3.2 Determinism

- Diff results must be deterministic across runs:
  - Same inputs → identical `DiffReport` (op ordering, contents) every time.
- Implementation must not rely on raw `HashMap` iteration order when emitting ops;
  even if D1 produces an empty diff, the alignment data structures should be written
  in a deterministic-friendly style for later milestones.

### 3.3 Safety and fallback behavior

- D1 tests and fixtures only cover unique keys; duplicate-key behavior is **not**
  part of this acceptance.
- If a caller supplies key columns that are not globally unique:
  - It is acceptable in this cycle for the helper to return an internal error used
    only in tests (for example, “DuplicateKeyUnsupported”), or to treat such tables
    as “not-database-mode-safe yet”.
  - It must not silently misalign rows or panic in release builds.

### 3.4 Non-goals

- No changes to JSON wire shape or PG4 expectations; `DiffReport` and `DiffOp` remain
  the canonical diff IR.
- No streaming/segmentation of very large tables yet; D1 assumes the entire grid
  fits in memory, consistent with the current engine and codebase context.


## 4. Interfaces

### 4.1 New engine API (database-mode entry point)

Add a public function in `core/src/engine.rs`:

```text
pub fn diff_grids_database_mode(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
) -> DiffReport
````

Semantics:

* Applies Database Mode keyed alignment on `old` and `new` using the supplied key
  columns, and produces a `DiffReport` reusing the existing `DiffOp` variants.
* For the D1 milestone:

  * Only single-column keys (`key_columns.len() == 1`) are exercised in tests.
  * Unique keys are assumed by tests; duplicate keys may cause a controlled error.
* Does **not** modify any existing behavior of `diff_workbooks` or the JSON helpers;
  it is an opt-in engine-level primitive used by integration tests and future callers.

### 4.2 Internal helper module

Create `core/src/database_alignment.rs` with internal types such as:

* `KeyColumnSpec` – wraps `Vec<u32>` of key column indices.
* `KeyedRow` – holds a precomputed key representation and a row index.
* `KeyedAlignment` – contains:

  * `matched_rows: Vec<(u32, u32)>` (row index in A, row index in B)
  * `left_only_rows: Vec<u32>`
  * `right_only_rows: Vec<u32>`

And a main internal function:

```text
fn diff_table_by_key(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
) -> KeyedAlignment
```

This function encapsulates the key-extraction and hash-join logic and is used by
`diff_grids_database_mode` to drive `DiffOp` emission.

These types remain internal for now and can evolve as later database milestones add
duplicate-key handling and mixed-mode segmentation.

## 5. Test plan

Per the meta-programming guide, this cycle must be grounded in explicit tests that
advance a defined milestone (here: D1 in the testing plan).

### 5.1 New integration tests (fixtures-based)

File: `core/tests/d1_database_mode_tests.rs`

Shared utilities:

* Use the existing `tests::common::fixture_path` helper and `open_workbook` to load
  Excel fixtures.
* Extract the primary data sheet’s `Grid` for the D1 fixtures.

Tests:

1. **`d1_equal_ordered_database_mode_empty_diff`**

   * Load `db_equal_ordered_a.xlsx` as both “old” and “new”.
   * Extract the main data sheet grid from each.
   * Call `diff_grids_database_mode(&grid_a, &grid_b, &[0])`.
   * Assert:

     * `report.ops.is_empty()` – no row, column, or cell operations.

2. **`d1_equal_reordered_database_mode_empty_diff`**

   * Load `db_equal_ordered_a.xlsx` as “old”, `db_equal_ordered_b.xlsx` as “new”.
   * Extract the main data sheet grids.
   * Call `diff_grids_database_mode(&grid_a, &grid_b, &[0])`.
   * Assert:

     * `report.ops.is_empty()` for the data sheet.

3. **Optional contrast test (non-blocking)**

   * For documentation value only:

     * Call existing `diff_workbooks` on the same pair and assert that it **does**
       surface structural changes (e.g., row additions/removals) in Spreadsheet Mode.
   * This demonstrates the semantic difference between Spreadsheet Mode and
     Database Mode on the same fixture pair.

### 5.2 New unit tests (alignment logic)

File: `core/src/database_alignment.rs` (module-local tests)

1. **`unique_keys_reorder_no_changes`**

   * Construct two in-memory `Grid`s representing the same keyed table with rows in
     different orders; key is column 0.
   * Call the internal `diff_table_by_key`.
   * Assert:

     * All keys appear in `matched_rows` as 1:1 matches.
     * `left_only_rows` and `right_only_rows` are empty.

2. **`unique_keys_insert_delete_classified` (forward-looking)**

   * Similar keyed table, but:

     * A has keys {1, 2}, B has keys {1, 2, 3}.
   * Call `diff_table_by_key`.
   * Assert:

     * `matched_rows` covers keys 1 and 2.
     * `right_only_rows` contains the row index for key 3 (insert).
     * `left_only_rows` is empty.
   * This prepares the path for the D2 “Keyed insert/delete” milestone without
     fully shipping D2 behavior through to `DiffReport` yet.

3. **`duplicate_keys_error_or_unsupported`**

   * Build `Grid`s where a key appears more than once in at least one side.
   * Call `diff_table_by_key`.
   * Assert:

     * The function returns an “unsupported due to duplicate key” outcome (whether
       as `Result::Err` or a flagged alignment struct), not silent misalignment or
       panics.

### 5.3 Existing tests that must stay green

* All PG1–PG6 grid and object-graph tests, plus PG5 in-memory grid-diff tests, must
  continue to pass unchanged; they validate Spreadsheet Mode behavior and tail-row/
  column operations.
* DataMashup/M tests (M1–M6, M4/M5 domain layers) must remain unaffected; the
  database-mode work touches only Grid alignment and engine API, not M extraction or
  query diffing.
* Signature and hashing tests for row/column signatures must remain green; D1 uses
  existing Grid IR and does not change signature semantics.

### 5.4 Documentation and “docs vs implementation” updates

* Update the “docs vs implementation” tracking doc to mark D1 as **implemented**
  with any simplifications called out (explicit key only, unique-key only).
* Ensure `excel_diff_testing_plan.md` still describes D1–D10 as the full database-mode
  roadmap, with only D1 claimed as done.
* Optionally, add a short note to the product differentiation / roadmap docs that
  “Database Mode keyed equality (unique key, explicit key column) is implemented at
  the engine level and exercised in the D1 fixtures,” tying back to the broader
  market story.

---

**Acceptance criteria for this cycle**

* New database-mode API (`diff_grids_database_mode`) exists and is covered by unit
  and fixture-based tests.
* D1 fixtures (`db_equal_ordered_{a,b}.xlsx`) pass the Database Mode keyed-equality
  tests with empty diffs.
* No regressions in existing Spreadsheet Mode behavior or tests.
* Documentation updated to reflect that D1 is now implemented as a vertical slice
  of the broader Database Mode (H1) roadmap.
