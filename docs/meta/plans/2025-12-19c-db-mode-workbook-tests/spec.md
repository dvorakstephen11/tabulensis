# Database Mode D2-D4 workbook fixtures and tests

## Scope

In scope (this cycle):
- Fixture generation:
  - `fixtures/src/generators/database.py` (`KeyedTableGenerator` / `db_keyed`)
  - `fixtures/manifest.yaml` (new scenario entries)
  - Regenerated outputs under `fixtures/generated/`
- Tests:
  - Add fixture-backed integration tests for database mode behavior (prefer: extend `core/tests/d1_database_mode_tests.rs`, or create a focused new file like `core/tests/d2_d4_database_mode_workbook_tests.rs`)

Out of scope (explicitly not this cycle):
- Any changes to database-mode matching/alignment algorithms (duplicate-key cluster solving, key inference, mixed table+grid region detection)
- Any changes to `DiffOp` shapes / JSON schema
- Any integration of database mode into the main workbook diff pipeline via `DiffConfig` (selection/configuration work deferred)

## Behavioral Contract

Assume the fixture table is on a worksheet named `Data` with columns:

- Col 0: `ID` (key)
- Col 1: `Name`
- Col 2: `Amount`
- Col 3: `Category`

Database-mode diff is executed as:
- `diff_grids_database_mode(grid_a, grid_b, keys=[0], ...)`

The contract below is expressed in terms of observable `DiffOp`s and workbook-derived row locations.

### D2: Row added emits RowAdded only

Fixture pair:
- A: `db_equal_ordered_a.xlsx` (baseline table)
- B: `db_row_added_b.xlsx` (same table plus one new row with a new key)

Expected result:
- Exactly 1 `DiffOp::RowAdded`
- 0 `DiffOp::RowRemoved`
- 0 `DiffOp::CellEdited`

And:
- Let `row_b = find_row_by_id(grid_b, <new_id>)`.
- The emitted `RowAdded { row_idx }` must satisfy `row_idx == row_b`.

Notes:
- `<new_id>` should match the generator's extra-row key (currently expected to be 1001 in the existing generator pattern).

### D2: Row removed emits RowRemoved only

Fixture pair (reuse existing files by swapping sides):
- A: `db_row_added_b.xlsx` (superset table that includes the extra key)
- B: `db_equal_ordered_a.xlsx` (baseline table)

Expected result:
- Exactly 1 `DiffOp::RowRemoved`
- 0 `DiffOp::RowAdded`
- 0 `DiffOp::CellEdited`

And:
- Let `row_a = find_row_by_id(grid_a, <removed_id>)` (same value as the extra-row key).
- The emitted `RowRemoved { row_idx }` must satisfy `row_idx == row_a`.

### D3: Row update emits CellEdited only (non-key column)

Fixture pair:
- A: `db_equal_ordered_a.xlsx`
- B: `db_row_update_b.xlsx` (same keys; exactly one non-key cell changed)

Generator intent:
- For `ID = 7`, change `Amount` from its baseline value to `120`.

Expected result:
- Exactly 1 `DiffOp::CellEdited`
- 0 `DiffOp::RowAdded`
- 0 `DiffOp::RowRemoved`

And:
- Let `row_b = find_row_by_id(grid_b, 7)`.
- The emitted `CellEdited { addr, from, to, ... }` must satisfy:
  - `addr.row == row_b`
  - `addr.col == 2` (Amount column)
  - `from.value` equals the baseline Amount for ID 7 (as stored in A)
  - `to.value == Number(120.0)` (or equivalent numeric representation)
  - `from.formula == None` and `to.formula == None` (no formulas in these fixtures)

### D4: Reorder + change ignores reorder, still emits only the edit

Fixture pair:
- A: `db_equal_ordered_a.xlsx` (ordered baseline)
- B: `db_reorder_and_change_b.xlsx` (rows shuffled, plus the same single `Amount` edit for `ID = 7`)

Expected result:
- Exactly 1 `DiffOp::CellEdited`
- 0 `DiffOp::RowAdded`
- 0 `DiffOp::RowRemoved`

And:
- Let `row_b = find_row_by_id(grid_b, 7)` (must be discovered from B, since shuffle changes position).
- The emitted `CellEdited` must target `(row_b, col=2)` with the same from/to value expectations as D3.

Non-goal:
- No attempt is made to emit a "row moved" operation in database mode. Reordering is intentionally treated as non-semantic when keys are provided.

## Constraints

- Determinism:
  - Fixtures must be reproducible from `fixtures/manifest.yaml` using a fixed seed.
  - The new generator argument for updates must not change outputs of existing scenarios that do not specify updates.
- Size budget:
  - Keep the keyed table fixture size at the existing order of magnitude (currently 1000 rows) to avoid repo bloat and slow tests.
- Test robustness:
  - Tests must not hardcode row indices for shuffled workbooks.
  - Tests must locate rows by scanning the key column in the parsed `Grid`.
- Stability:
  - No changes to core diff IR or output schema this cycle.

## Interfaces

### Fixture generator interface (Python)

Extend `db_keyed` generator args with an optional `updates` list.

Proposed manifest shape:
```yaml
args:
  count: 1000
  seed: 42
  shuffle: true|false
  extra_rows: [{ id: 1001, name: "New Row", amount: 999 }]
  updates:
    - { id: 7, amount: 120 }
