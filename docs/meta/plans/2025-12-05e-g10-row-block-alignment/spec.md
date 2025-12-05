# docs/meta/plans/2025-12-05-g10-row-block-alignment/spec.md

# 2025-12-05-g10-row-block-alignment mini-spec

Goal: implement milestone G10 (contiguous block of rows inserted / deleted) for small spreadsheet-mode grids by extending the existing row alignment path and adding focused tests and fixtures.

---

## 1. Scope

This branch advances spreadsheet-mode grid alignment for row blocks in the middle of a sheet.

In scope:

- `core/src/row_alignment.rs`
  - Extend the row alignment helper so it can recognize a single contiguous multi-row
    insert or delete in the middle of a sheet (block size > 1), under the same kind
    of safety gates used for the current single-row G8 path.
  - Produce an alignment mapping that:
    - Lists all inserted or deleted row indices in Grid B or Grid A respectively.
    - Provides a monotonic list of matched row index pairs for unchanged rows.

- `core/src/engine.rs`
  - Update the grid diff path (used by `diff_workbooks`) to:
    - Invoke the new row-block alignment logic when row counts differ by more than one.
    - Translate the alignment into a sequence of `RowAdded` or `RowRemoved` `DiffOp`s.
    - Fall back to the existing positional diff or single-row alignment when the
      block alignment helper declines to handle a case.

- Fixtures and generators:
  - `fixtures/manifest.yaml`:
    - Add entries for:
      - `row_block_insert_{a,b}.xlsx`
      - `row_block_delete_{a,b}.xlsx`
  - `fixtures/src/generators/...`:
    - Extend an existing grid generator (for example `grid_tail_diff` or a dedicated
      `row_alignment_g10` generator) to produce the above pairs.

- Tests:
  - New Rust test module:
    - `core/tests/g10_row_block_alignment_grid_workbook_tests.rs`
  - Optional but recommended:
    - Additional unit tests in the `row_alignment.rs` `#[cfg(test)]` module that
      exercise the pure alignment helper on synthetic numeric grids.

Out of scope:

- Column block insert/delete (column analog of G10).
- Any detection of block moves (G11) or diagonal/block moves.
- Database mode keyed diff (D1-D10) and M AST diff (M7).
- Changes to public crate APIs or the `DiffOp` enum shape and JSON schema.

---

## 2. Behavioral contract

### 2.1 Block insert in the middle

Scenario:

- Grid A:
  - One sheet, for example `Sheet1`.
  - Rows 1-10 populated with simple, distinct content (for example an ID column
    and a few numeric columns).
- Grid B:
  - Same as A, except a block of four new rows is inserted at positions 4-7
    (1-based) between rows 3 and 8 of the original grid.
  - Inserted rows have distinctive content that does not appear elsewhere.

Contract:

- `diff_workbooks(&wb_a, &wb_b)` produces:
  - Exactly four `DiffOp::RowAdded` operations.
  - Each `RowAdded`:
    - Has `sheet == "Sheet1"` (or whatever sheet name is used in the fixtures).
    - Has `row_idx` values `[3, 4, 5, 6]` (zero-based indices for rows 4-7).
    - Has `row_signature == None` (consistent with existing G6-G9 tests).
- No `RowRemoved` operations are emitted.
- No `CellEdited` operations are emitted for this fixture:
  - There should be no edits on unchanged rows above or below the inserted block.
  - There should be no `CellEdited` ops for the newly inserted rows; they are
    represented purely as `RowAdded`.
- The relative order and contents of rows outside the block (rows 1-3 and 8-10)
  are preserved and compared correctly.

### 2.2 Block delete in the middle

Scenario:

- Grid A:
  - Same base grid as above (rows 1-10).
- Grid B:
  - Rows 4-7 removed; remaining rows compacted upward.

Contract:

- `diff_workbooks(&wb_a, &wb_b)` produces:
  - Exactly four `DiffOp::RowRemoved` operations.
  - Each `RowRemoved`:
    - Has `sheet == "Sheet1"`.
    - Has `row_idx` values `[3, 4, 5, 6]` (zero-based indices for rows 4-7).
    - Has `row_signature == None`.
- No `RowAdded` operations are emitted.
- No `CellEdited` operations are emitted for this fixture.
- Rows outside the deleted block (1-3 and 8-10) are aligned and compared as
  expected with no spurious edits.

### 2.3 Fallback behavior and safety

The new row-block alignment should be used only when:

- The difference in row counts between Grid A and Grid B is greater than one and
  small (for example up to some fixed threshold such as 32).
- A single contiguous gap in row indices can explain the difference (all other
  rows can be matched monotonically).
- Row hashes or signatures indicate that the rows outside the gap match cleanly
  (no ambiguous multiple matches).
- The grid passes existing safety gates such as:
  - Size bounds (rows and columns below configured limits for this path).
  - Row repetition and low-info checks (no heavy repetition that would make
    matching ambiguous or expensive).

If any of these conditions fail, the alignment helper must return `None` and the
engine must fall back to the existing behavior (single-row alignment for G8 or
plain positional diff). Fallback behavior is intentionally not re-specified here;
tests should confirm that we have not regressed the existing G6-G9 scenarios.

### 2.4 Non-goals

- Do not attempt to coalesce `RowAdded`/`RowRemoved` into a new block DiffOp
  variant. Presentation-level coalescing can be added later without changing the
  core engine contract.
- Do not attempt to treat moved blocks (where the same content appears elsewhere
  in the sheet) as pure moves; that is G11 territory and should remain untouched.
- Do not populate `row_signature` on the G10 `RowAdded`/`RowRemoved` ops in this
  cycle; keep it `None` to avoid touching existing tests and output contracts.

---

## 3. Constraints and invariants

Implementation constraints:

- Time and space complexity:
  - The additional alignment logic must be at most linear in the number of rows
    in the grids and in the size of the candidate block region.
  - It must not allocate structures proportional to rows * columns.
- Size gates:
  - Reuse or refine the existing `is_within_size_bounds`, `low_info_dominated`,
    and `has_heavy_repetition` checks from `row_alignment.rs`.
  - Do not widen the gates in this branch; it is acceptable for the block
    alignment to decline more cases rather than risk quadratic behavior.
- Determinism:
  - For a given pair of grids, the alignment and resulting `DiffOp` sequence must
    be deterministic (no dependence on map iteration order).

Diff invariants:

- For a block insert fixture:
  - All new rows appear exactly once in `RowAdded` ops.
  - No `RowRemoved` or `CellEdited` ops are emitted.
- For a block delete fixture:
  - All removed rows appear exactly once in `RowRemoved` ops.
  - No `RowAdded` or `CellEdited` ops are emitted.
- For grids that previously satisfied G1-G9:
  - The existing G1-G9 tests continue to pass unchanged.
  - The numeric values of `row_idx` and `col_idx` in existing tests are not
    altered by this change (alignment behavior must be backwards-compatible).

Internal alignment invariants:

- The alignment helper returns:
  - A list of `inserted` row indices (in Grid B) or `deleted` row indices (in
    Grid A) that are strictly increasing.
  - A list of `matched` pairs `(row_a, row_b)` that is strictly increasing in
    both components and covers all non-inserted/deleted rows.
- No row index participates in more than one alignment role (inserted, deleted,
  or matched).

---

## 4. Interfaces

Public APIs:

- No changes to the `excel_diff` public API:
  - `diff_workbooks` signature remains the same.
  - `DiffReport` structure and JSON serialization remain unchanged.
  - `DiffOp` enum variants and fields remain unchanged.

Internal interfaces likely to change:

- `core/src/row_alignment.rs`:
  - The `Alignment` struct used internally may gain support for multiple inserted
    or deleted indices, or a new helper struct may be introduced for block cases.
  - The main entry point used by the engine:
    - Either generalize `align_single_row_change` to `align_row_changes`, or
      introduce a new function (for example `align_row_block_change`) that
      handles multi-row cases and reuses the single-row logic when appropriate.
- `core/src/engine.rs`:
  - The grid diff function that currently calls the single-row alignment helper
    will be updated to:
    - Detect when the row count difference is greater than one.
    - Call the block alignment path first, then fall back to single-row or
      positional diff.
- Test fixtures:
  - `fixtures/manifest.yaml` will gain two new entries for G10 fixtures.
  - The associated Python generator will add modes or parameters to create
    the block insert/delete grids.

Any changes to internal helper signatures should remain confined to the core
grid diff modules and tests.

---

## 5. Test plan

All new work must be expressed via tests. This section defines the concrete
tests to add or rely on.

### 5.1 New fixtures

Add the following entries to `fixtures/manifest.yaml`:

- `g10_row_block_insert`
  - `kind: excel_pair`
  - `output` (or equivalent):
    - `row_block_insert_a.xlsx`
    - `row_block_insert_b.xlsx`
- `g10_row_block_delete`
  - `kind: excel_pair`
  - `output`:
    - `row_block_delete_a.xlsx`
    - `row_block_delete_b.xlsx`

Fixture generation sketch:

- `row_block_insert_{a,b}.xlsx`:
  - A:
    - Sheet name: `Sheet1`.
    - 10 rows x N columns (for example 5).
    - Column A contains a simple ID sequence 1..10; other columns contain
      deterministic but unimportant numeric values.
  - B:
    - Same as A, plus 4 new rows inserted between ID 3 and ID 4.
    - New rows have distinct IDs (for example 1001-1004) and content not used
      elsewhere.
- `row_block_delete_{a,b}.xlsx`:
  - A:
    - Same layout as A above, but with rows 1-10 present.
  - B:
    - Rows 4-7 removed so that IDs 1-3 and 8-10 remain.

These follow the G10 fixture sketch and exercise both insert and delete cases.

### 5.2 Row-alignment unit tests (Rust)

In `core/src/row_alignment.rs` under `#[cfg(test)]`, add small synthetic tests
that exercise the pure alignment helper without going through Excel:

1. `aligns_contiguous_block_insert_middle`:
   - Build `Grid` A from rows:
     - 10 rows of small integer arrays with unique content.
   - Build `Grid` B by inserting 4 new unique rows between rows 3 and 4.
   - Assert:
     - `alignment.inserted == [3, 4, 5, 6]`.
     - `alignment.deleted.is_empty()`.
     - `alignment.matched` covers all other rows and is monotonic.

2. `aligns_contiguous_block_delete_middle`:
   - Build `Grid` A and B with 10 rows and rows 4-7 removed in B.
   - Assert:
     - `alignment.deleted == [3, 4, 5, 6]`.
     - `alignment.inserted.is_empty()`.
     - `alignment.matched` is monotonic and covers the remaining rows.

3. `block_alignment_bails_on_noncontiguous_changes`:
   - Build grids where multiple rows differ but not as a single contiguous block
     (for example new rows inserted at two separate positions or a mix of
     insert and delete).
   - Assert the block alignment helper returns `None`.

These tests define the internal behavior of the helper and act as guardrails
for future refactors.

### 5.3 Grid workbook tests for G10

New test module: `core/tests/g10_row_block_alignment_grid_workbook_tests.rs`.

Tests:

1. `g10_row_block_insert_middle_emits_four_rowadded_and_no_noise`:
   - Open `row_block_insert_a.xlsx` and `row_block_insert_b.xlsx`.
   - Run `diff_workbooks`.
   - Collect `RowAdded` ops:
     - Assert there are exactly 4.
     - Assert all have `sheet == "Sheet1"`.
     - Assert all have `row_signature.is_none()`.
     - Assert `row_idx` values are `[3, 4, 5, 6]` in order (or as a set).
   - Assert there are:
     - No `RowRemoved` ops.
     - No `CellEdited` ops.

2. `g10_row_block_delete_middle_emits_four_rowremoved_and_no_noise`:
   - Open `row_block_delete_a.xlsx` and `row_block_delete_b.xlsx`.
   - Run `diff_workbooks`.
   - Collect `RowRemoved` ops:
     - Assert there are exactly 4.
     - Assert all have `sheet == "Sheet1"`.
     - Assert all have `row_signature.is_none()`.
     - Assert `row_idx` values are `[3, 4, 5, 6]`.
   - Assert there are:
     - No `RowAdded` ops.
     - No `CellEdited` ops.

These tests mirror the style of existing G6-G9 workbook tests and codify the
expected edit script.

### 5.4 Regression and coverage

No explicit changes are needed to existing tests; the CI suite will run them
automatically. The implementer should locally ensure that at least the
following groups remain green:

- `core/tests/pg5_grid_diff_tests.rs` (PG5 grid diff micro-cases).
- `core/tests/g1_g2_grid_workbook_tests.rs` (basic equal/single-cell scenarios).
- `core/tests/g5_g7_grid_workbook_tests.rs` (row/column append/truncate).
- `core/tests/g8_row_alignment_grid_workbook_tests.rs` (single-row alignment).
- `core/tests/g9_column_alignment_grid_workbook_tests.rs` (single-column alignment).
- `core/tests/grid_view_tests.rs` and `grid_view_hashstats_tests.rs` (GridView
  and hash statistics behavior).

The G10 tests should be considered release-gating for milestone G10: the branch
is complete when all existing tests plus the new G10 tests pass.
