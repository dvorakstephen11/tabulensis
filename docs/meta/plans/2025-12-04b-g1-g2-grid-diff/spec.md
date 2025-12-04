# 2025-12-04-g1-g2-grid-diff mini-spec

Branch: `2025-12-04-g1-g2-grid-diff`  
Work type: `milestone_progress`  
Target milestones: Phase 3 spreadsheet-mode G1 (identical sheet) and G2 (single literal change).

---

## 1. Scope

### 1.1 Rust modules

In scope (behavioral surface under test):

- `core/src/engine.rs`
  - `pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport`
  - `fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>)`
- `core/src/lib.rs`
  - Re-exports:
    - `pub use engine::diff_workbooks;`
    - `#[cfg(feature = "excel-open-xml")] pub use excel_open_xml::open_workbook;`
    - `pub use diff::{DiffOp, DiffReport};`
    - `pub use workbook::{CellValue, CellAddress, CellSnapshot};`

New tests:

- `core/tests/g1_g2_grid_workbook_tests.rs` (new file)
  - Integration tests that:
    - Open real Excel fixtures via `open_workbook`.
    - Call `diff_workbooks` on parsed `Workbook`s.
    - Assert on `DiffReport.ops`.

Out of scope for this branch:

- DataMashup parsing and diff (`datamashup*`, `m_section`, query-domain types).
- Advanced grid alignment (row/column signatures, LCS, block moves) beyond what is already used.
- Database-mode diff and key inference.
- JSON projection and serialization behavior (`core/src/output/json.rs`), except indirect use in tests if helpful.

Implementation note:  
The intent is “tests first”. Code changes in `diff_grids` or `diff_workbooks` are allowed only if G1/G2 reveal bugs or mismatches with the documented behavior; any such change must keep all existing PG1–PG6 and PG4 tests passing.

---

### 1.2 Fixtures and generators

In scope:

- `fixtures/manifest.yaml`
  - Add two new scenarios with explicit output filenames:

    ```yaml
    - id: "g1_equal_sheet"
      generator: "basic_grid"
      args:
        rows: 5
        cols: 5
        sheet: "Sheet1"
      output:
        - "equal_sheet_a.xlsx"
        - "equal_sheet_b.xlsx"

    - id: "g2_single_cell_value"
      generator: "single_cell_diff"
      args:
        rows: 5
        cols: 5
        sheet: "Sheet1"
        target_cell: "C3"
        value_a: 1.0
        value_b: 2.0
      output:
        - "single_cell_value_a.xlsx"
        - "single_cell_value_b.xlsx"
    ```

- `fixtures/src/generators/grid.py`
  - Reuse existing `BasicGrid` and `SingleCellDiff` generators; no new Python code is strictly required beyond manifest wiring.

Generated fixture files (under `fixtures/generated/`):

- `equal_sheet_a.xlsx`
- `equal_sheet_b.xlsx`
- `single_cell_value_a.xlsx`
- `single_cell_value_b.xlsx`

These are single-worksheet workbooks with sheet name `"Sheet1"`.

Out of scope:

- Any large-sheet performance fixtures (G8+, perf tests).
- Multi-sheet or database-mode fixtures.

---

## 2. Behavioral contract

All examples below assume `use excel_diff::{diff_workbooks, DiffOp, CellValue, open_workbook};`.

### 2.1 G1 – identical sheet produces an empty diff

**Setup**

- `equal_sheet_a.xlsx` and `equal_sheet_b.xlsx`:

  - One worksheet named `"Sheet1"`.
  - 5x5 grid (`A1:E5`).
  - Every cell is a simple value (numbers or strings) produced by `BasicGrid` (e.g., `R1C1`, `R1C2`, ...), but the exact pattern does not matter as long as the two workbooks are identical from the parser’s point of view.

**Contract**

Given:

```rust
let wb_a = open_workbook(fixture_path("equal_sheet_a.xlsx"))?;
let wb_b = open_workbook(fixture_path("equal_sheet_b.xlsx"))?;
let report = diff_workbooks(&wb_a, &wb_b);
````

We require:

* `report.ops.is_empty()`.

More explicitly:

* No `DiffOp::SheetAdded` or `DiffOp::SheetRemoved`.
* No `DiffOp::RowAdded` / `RowRemoved`.
* No `DiffOp::ColumnAdded` / `ColumnRemoved`.
* No `DiffOp::CellEdited`.

Workbook-level “we compared these sheets” metadata is allowed elsewhere in the system, but the `DiffReport.ops` list for this scenario must be exactly empty.

---

### 2.2 G2 – single cell literal change yields a single CellEdited

**Setup**

* `single_cell_value_a.xlsx` and `single_cell_value_b.xlsx`:

  * One worksheet `"Sheet1"`.
  * 5x5 grid produced by `SingleCellDiff` with:

    * `value_a = 1.0`
    * `value_b = 2.0`
    * `target_cell = "C3"`
  * All other cells share identical contents between A and B.

**Contract**

Given:

```rust
let wb_a = open_workbook(fixture_path("single_cell_value_a.xlsx"))?;
let wb_b = open_workbook(fixture_path("single_cell_value_b.xlsx"))?;
let report = diff_workbooks(&wb_a, &wb_b);
```

We require:

* `report.ops.len() == 1`.
* The single op is a `DiffOp::CellEdited` on `"Sheet1"` at address `C3` with the correct old/new values.
* No row or column structural ops for this scenario.

Concrete expectations:

* `matches!(report.ops[0], DiffOp::CellEdited { .. })` holds.

* If we match it:

  ```rust
  match &report.ops[0] {
      DiffOp::CellEdited { sheet, addr, from, to } => {
          assert_eq!(sheet, "Sheet1");
          assert_eq!(addr.to_a1(), "C3");
          assert_eq!(from.value, Some(CellValue::Number(1.0)));
          assert_eq!(to.value, Some(CellValue::Number(2.0)));
          // formula fields should be identical and typically None for this fixture
          assert_eq!(from.formula, None);
          assert_eq!(to.formula, None);
      }
      other => panic!("expected CellEdited, got {other:?}"),
  }
  ```

* There must be no `RowAdded`, `RowRemoved`, `ColumnAdded`, or `ColumnRemoved` ops in this diff.

Ordering: since there is only one op, tests can assert it is at index 0.

---

### 2.3 Non-goals for this branch

These are explicitly *out of scope* for this mini-spec:

* Distinguishing “formula formatting only” vs semantic changes (G3).
* Ignoring or surfacing pure formatting edits (G4).
* Multiple scattered edits (G5), row/column appends/truncates (G6/G7) beyond what is already covered indirectly by PG5 tests.
* Any advanced alignment or block move detection (G8+).
* Any configuration interface to ignore formatting or formulas.

---

## 3. Constraints and invariants

### 3.1 Performance and memory

* The naive `diff_grids` algorithm is O(R * C) over the overlapping rectangle plus O(extra_rows + extra_cols) for tails. G1/G2 use 5x5 grids; there must be no additional asymptotic cost introduced by this branch.
* No new heap allocations proportional to grid size beyond what `Grid` already holds.
* Fixture sizes are tiny; no additional performance constraints are needed for this branch.

### 3.2 Determinism

* `DiffReport.ops` must remain deterministic for these scenarios:

  * G1: always an empty vector.
  * G2: always a single `CellEdited` for `"Sheet1"` at `C3`.

Any changes to `diff_workbooks` or `diff_grids` must preserve existing ordering guarantees already enforced and exercised by the PG4 and engine tests.

### 3.3 DiffOp invariants

Existing invariants from PG4 remain in force and must not be violated:

* `DiffOp::CellEdited` must be constructed via the canonical helper, ensuring:

  * `from.addr` and `to.addr` both match the outer `addr`.
  * Snapshots carry the same address but may have different values/formulas.
* `CellSnapshot` equality ignores address and considers only `(value, formula)`; tests must treat address as a separate field.

G1/G2 tests should rely on the public `DiffOp` shape and these invariants, not on internal implementation details.

---

## 4. Interfaces and IR

Public API surfaces touched by the tests (but not changed):

* `excel_diff::open_workbook(path: impl AsRef<Path>) -> Result<Workbook, ExcelOpenError>`
* `excel_diff::diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport`
* `excel_diff::DiffReport` and `excel_diff::DiffOp`
* `excel_diff::CellValue`, `excel_diff::CellAddress`, `excel_diff::CellSnapshot`

IR expectations for these fixtures:

* The workbook parser produces:

  ```text
  Workbook {
      sheets: [
          Sheet {
              name: "Sheet1",
              kind: Worksheet,
              grid: Grid { nrows = 5, ncols = 5, ... }
          }
      ]
  }
  ```

* `Grid` indices are zero-based, while `CellAddress::to_a1()` converts to one-based A1 notation; tests must check addresses via `to_a1()` rather than raw indices.

No new configuration types (such as a `DiffConfig`) are introduced in this branch.

---

## 5. Test plan

### 5.1 Fixture generation

1. **Add manifest entries** for `g1_equal_sheet` and `g2_single_cell_value` as specified in Section 1.2.
2. **Regenerate fixtures** via `fixtures/src/generate.py` so the four `.xlsx` files appear in `fixtures/generated/`.
3. Sanity checks (manual or via small smoke tests):

   * `equal_sheet_a.xlsx` and `equal_sheet_b.xlsx` both open successfully via `open_workbook` and yield identical `Workbook` IR.
   * `single_cell_value_{a,b}.xlsx` open successfully and differ only at `Sheet1!C3` with values `1.0` vs `2.0`.

### 5.2 Rust tests

Create `core/tests/g1_g2_grid_workbook_tests.rs` containing at least:

1. **G1: identical sheet -> empty diff**

   ```rust
   use excel_diff::{diff_workbooks, open_workbook, DiffOp};

   mod common;
   use common::fixture_path;

   #[test]
   fn g1_equal_sheet_produces_empty_diff() {
       let wb_a = open_workbook(fixture_path("equal_sheet_a.xlsx"))
           .expect("equal_sheet_a.xlsx should open");
       let wb_b = open_workbook(fixture_path("equal_sheet_b.xlsx"))
           .expect("equal_sheet_b.xlsx should open");

       let report = diff_workbooks(&wb_a, &wb_b);

       assert!(
           report.ops.is_empty(),
           "identical 5x5 sheet should produce an empty diff"
       );
   }
   ```

2. **G2: single literal change -> one CellEdited**

   ```rust
   use excel_diff::{diff_workbooks, open_workbook, CellValue, DiffOp};

   #[test]
   fn g2_single_cell_literal_change_produces_one_celledited() {
       let wb_a = open_workbook(fixture_path("single_cell_value_a.xlsx"))
           .expect("single_cell_value_a.xlsx should open");
       let wb_b = open_workbook(fixture_path("single_cell_value_b.xlsx"))
           .expect("single_cell_value_b.xlsx should open");

       let report = diff_workbooks(&wb_a, &wb_b);

       assert_eq!(
           report.ops.len(),
           1,
           "expected exactly one diff op for a single edited cell"
       );

       match &report.ops[0] {
           DiffOp::CellEdited { sheet, addr, from, to } => {
               assert_eq!(sheet, "Sheet1");
               assert_eq!(addr.to_a1(), "C3");
               assert_eq!(from.value, Some(CellValue::Number(1.0)));
               assert_eq!(to.value, Some(CellValue::Number(2.0)));
               assert_eq!(from.formula, to.formula, "no formula changes expected");
           }
           other => panic!("expected CellEdited, got {other:?}"),
       }
   }
   ```

3. **Negative check (optional but recommended)**

   * Add a small assertion that there are no structural row/column ops in the G2 diff:

     ```rust
     assert!(
         !report.ops.iter().any(|op| matches!(
             op,
             DiffOp::RowAdded { .. }
                 | DiffOp::RowRemoved { .. }
                 | DiffOp::ColumnAdded { .. }
                 | DiffOp::ColumnRemoved { .. }
         )),
         "single cell change should not produce row/column structure ops"
     );
     ```

### 5.3 Success criteria

This branch is complete when:

1. The four fixtures are generated and checked into `fixtures/generated/` (or reproducible via the generator script).
2. New tests in `g1_g2_grid_workbook_tests.rs` pass under:

   * `cargo test --all-features` from `core/`.
   * `cargo test` from the workspace root.
3. All existing tests (PG1–PG6, PG4, engine tests, JSON/output tests, DataMashup framing tests) continue to pass without modification.
4. The G1 and G2 sections of the testing plan can be marked as “implemented” with this branch as their first implementation reference.

---

```

Key sources for this plan:

- Phase 3 spreadsheet-mode milestones G1–G7 (including fixture names and expected behavior for G1/G2) are defined in the testing plan.   
- The current implementation of `diff_workbooks`, `diff_grids`, and existing PG5/engine tests comes from the codebase context and development history.   
- The grid diff architecture and difficulty framing (H1 grid alignment as the hardest work) come from the main specification, the unified grid diff algorithm design, and the difficulty analysis. 
