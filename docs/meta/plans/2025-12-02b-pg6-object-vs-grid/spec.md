## 1. Scope

**Primary goal:** Implement PG6.1–PG6.4 from the testing plan (“Object graph vs grid responsibilities”) as concrete, automated tests backed by generated Excel fixtures, without changing the external DiffOp contract.  

### 1.1 Modules and types in play

Rust:

- `core/src/engine.rs`
  - `diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport`
  - `diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, report: &mut DiffReport)` (used but not modified beyond what tests require).
- `core/src/workbook.rs`
  - `Workbook`, `Sheet`, `SheetKind`, `Grid`, `SheetId` (as string alias), and `Cell`/`CellSnapshot` types used in `DiffOp`.
- `core/src/output/json.rs`
  - Used indirectly in existing integration tests; no schema changes this cycle.:contentReference[oaicite:18]{index=18}
- New test module:
  - `core/tests/pg6_object_vs_grid_tests.rs` (name is a suggestion; exact file name may vary but should clearly map to PG6).  

Python fixtures:

- `fixtures/src/generators/grid.py`
  - Add a new generator class dedicated to PG6 scenarios (see 5.2).:contentReference[oaicite:19]{index=19}
- `fixtures/manifest.yaml`
  - Add entries for the new PG6 fixture pairs:
    - `pg6_sheet_added_{a,b}.xlsx`
    - `pg6_sheet_removed_{a,b}.xlsx`
    - `pg6_sheet_renamed_{a,b}.xlsx`
    - `pg6_sheet_and_grid_change_{a,b}.xlsx`

Out of scope for this cycle:

- Introducing a `SheetRenamed` DiffOp variant or implementing true rename detection.
- PG6.5 “Rename plus grid edits semantics”; that remains a follow-up milestone once rename support is designed.:contentReference[oaicite:21]{index=21}
- Any changes to row/column signature computation or usage; existing PG5 tests assume `row_signature`/`col_signature` remain `None`.

---

## 2. Behavioral Contract

This section describes, in plain language, what `diff_workbooks` must report for each PG6 scenario when invoked on the PG6 Excel fixtures. All expectations are expressed in terms of `DiffOp` values.

Across all PG6 tests in this cycle:

- **No spurious grid ops** (row/column/cell) may be emitted for sheets whose grids are bitwise-identical between A and B.
- Sheet-level operations (`SheetAdded`, `SheetRemoved`) must fully describe changes in the workbook’s sheet graph when names or counts differ.
- When both sheet-level and grid-level changes happen in the same workbook pair, they must be cleanly attributed to the correct sheet.

### 2.1 PG6.1 – Sheet addition doesn’t trigger grid ops on unchanged sheets

**Fixtures:**  

- `pg6_sheet_added_a.xlsx`
  - Single worksheet `Main` with a 5×5 grid of simple constants (e.g., `"R1C1"`, `"R1C2"`, …).  
- `pg6_sheet_added_b.xlsx`
  - Sheet `Main`: identical 5×5 grid.
  - Additional sheet `NewSheet`: small 3×3 grid (values arbitrary but stable).

**Expected diff behavior:**

- `diff_workbooks(A, B)` emits **exactly one** `DiffOp::SheetAdded { sheet: "NewSheet" }`.
- It emits **no** `RowAdded/Removed`, `ColumnAdded/Removed`, or `CellEdited` ops for sheet `"Main"`.
- It emits no other operations.

Informally: adding a brand-new sheet must not cause the engine to “re-diff” or claim changes on the unchanged `Main` sheet.

### 2.2 PG6.2 – Sheet removal symmetrical

**Fixtures:**  

- `pg6_sheet_removed_a.xlsx`
  - Sheets:
    - `Main`: 5×5 grid of constants.
    - `OldSheet`: small 3×3 grid.
- `pg6_sheet_removed_b.xlsx`
  - Only `Main`, unchanged.

**Expected diff behavior:**

- `diff_workbooks(A, B)` emits **exactly one** `DiffOp::SheetRemoved { sheet: "OldSheet" }`.
- It emits **no** grid-level ops (`Row*`, `Column*`, `CellEdited`) for `Main`.
- No other operations are allowed.

This proves sheet deletion doesn’t cascade into bogus grid ops on unchanged sheets.

### 2.3 PG6.3 – Sheet rename vs remove+add (rename-only, no grid changes)

**Fixtures:**  

- `pg6_sheet_renamed_a.xlsx`: single sheet `OldName` with a small grid (e.g., 3×3 constants).
- `pg6_sheet_renamed_b.xlsx`: same grid contents, but sheet is named `NewName`.

**Engine design choice for this cycle:**

- The current engine does **not** support an explicit `SheetRenamed` DiffOp; sheet identity is keyed by `(name_lower, SheetKind)`, so a rename is naturally represented as one removal plus one addition.

**Expected diff behavior:**

- `diff_workbooks(A, B)` emits:
  - One `DiffOp::SheetRemoved { sheet: "OldName" }`.
  - One `DiffOp::SheetAdded { sheet: "NewName" }`.
- **No grid diff ops** are emitted; the grids are identical and must not be re-diffed as edits.

Tests must explicitly assert both the presence of exactly one add and one remove and the absence of any `Row*` / `Column*` / `Cell*` ops for this pair.

### 2.4 PG6.4 – Sheet & grid changes composed cleanly

**Fixtures:**  

- `pg6_sheet_and_grid_change_a.xlsx`:
  - Sheet `Main`: 5×5 grid of constants (e.g., `"R{r}C{c}"`).
  - Sheet `Aux`: small grid (dimensions not critical) with constants.  
- `pg6_sheet_and_grid_change_b.xlsx`:
  - `Main`: same size as in A, with a small number (e.g., 2–3) of cell values changed.
  - `Aux`: unchanged.
  - Extra sheet `Scratch`: small constant grid.

**Expected diff behavior:**

- `SheetAdded`:
  - Exactly one `DiffOp::SheetAdded { sheet: "Scratch" }`.
- `Main` grid ops:
  - One or more `DiffOp::CellEdited { sheet: "Main", .. }` (and only on `Main`), representing the changed cells.
  - No `RowAdded/Removed` or `ColumnAdded/Removed` are expected for `Main` in this scenario, because the shapes are identical; tests should assert that all non-sheet DiffOps are `CellEdited` for sheet `"Main"`.
- `Aux` sheet:
  - **No grid-level ops** for `Aux`. The test must explicitly assert that no `DiffOp` with `sheet == "Aux"` appears.
- No other operations are allowed.

The key contract: when one sheet is added and another has cell-level edits, the diff must keep these concerns separate; unchanged sheets (`Aux`) stay quiet.

### 2.5 PG6.5 – Rename plus grid edits (explicitly deferred)

PG6.5 (`pg6_renamed_and_changed_{a,b}.xlsx`) defines behavior for rename + grid edits (e.g., `Summary` → `P&L` plus cell changes).  

This cycle **does not** attempt to implement or pin down behavior for PG6.5. It is reserved for a future milestone once rename detection and/or more advanced sheet matching are designed. The only requirement for this cycle is that nothing we do for PG6.1–PG6.4 makes PG6.5 materially harder.

---

## 3. Constraints

### 3.1 Performance and complexity

- The implementation must not introduce additional asymptotic complexity:
  - `diff_workbooks` remains **O(#sheets + sum of grid diff costs)**.
  - All new tests operate on tiny sheets (≤5×5) so they are negligible compared to existing tests.
- No changes to `diff_grids` algorithmic structure in this cycle; tests are intended to codify current behavior, not to introduce row/column alignment yet.

### 3.2 Memory and streaming

- New fixtures must be small and comparable in size to existing PG1/PG3 grid fixtures generated by `BasicGridGenerator` and friends. No large-sheet perf concerns are introduced.
- No change to streaming behavior or WASM constraints; PG6 tests run fully in memory on small files.

### 3.3 Invariants

- DiffOp wire format (`DiffReport` JSON) is unchanged; existing output and round-trip tests must continue to pass.
- Sheet identity remains defined by `(lowercased name, SheetKind)`. This is relied upon by existing engine tests and should not be altered in this cycle.
- Row/column signatures:
  - `RowAdded`/`RowRemoved`/`ColumnAdded`/`ColumnRemoved` continue to emit `row_signature: None` / `col_signature: None`, as pinned by PG5 tests; this cycle must not flip them to `Some(...)`.

---

## 4. Interfaces

### 4.1 Public APIs / IR types that must remain stable

For this cycle, the following must **not** change in a breaking way:

- `pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport` signature and its ownership / borrowing semantics.
- `DiffOp` enum variants and fields:
  - Especially `SheetAdded`, `SheetRemoved`, `CellEdited`, `RowAdded/Removed`, `ColumnAdded/Removed`.
- JSON serialization format of `DiffReport`, as used by `diff_workbooks_to_json` and tested in `output_tests.rs`.
- `open_workbook(path)` behavior; PG6 tests may rely on it to read fixtures, but they should do so through existing APIs without new knobs.

### 4.2 Internal interfaces that may be extended

Allowed changes (if helpful for tests) include:

- Adding small helper functions in tests (e.g., utility to count ops by kind).
- Adding a new Python generator class in `grid.py` and wiring it into the manifest/generate script.
- Extending `fixtures/manifest.yaml` with new scenario entries for PG6.

No changes to `SheetKey` internals or ordering behavior are required or encouraged in this cycle.

---

## 5. Test Plan

This section defines the concrete tests and fixtures that constitute the milestone.

### 5.1 New Rust tests: `core/tests/pg6_object_vs_grid_tests.rs`

Create a new test module (or equivalent) with four tests, each aligned to one PG6 subtest.

All tests use:

- `fixtures::common::fixture_path(..)` (or the existing helper used in `excel_open_xml_tests.rs`) to locate `.xlsx` files.
- `excel_diff::{open_workbook, diff_workbooks, DiffOp}` as imported in other integration tests.

#### 5.1.1 `pg6_1_sheet_added_no_grid_ops_on_main`

Pseudo-structure:

```rust
#[test]
fn pg6_1_sheet_added_no_grid_ops_on_main() {
    let a_path = fixture_path("pg6_sheet_added_a.xlsx");
    let b_path = fixture_path("pg6_sheet_added_b.xlsx");

    let old = open_workbook(a_path).expect("open A");
    let new = open_workbook(b_path).expect("open B");
    let report = diff_workbooks(&old, &new);

    // Exactly one SheetAdded("NewSheet")
    let mut sheet_added = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "NewSheet" => sheet_added += 1,
            DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::CellEdited { sheet, .. } if sheet == "Main" => {
                panic!("unexpected grid op on Main: {:?}", op);
            }
            _ => {}
        }
    }
    assert_eq!(sheet_added, 1);
    assert_eq!(report.ops.len(), 1);
}
````

Key assertions:

* Exactly one `SheetAdded("NewSheet")`.
* No `Row*`/`Column*`/`CellEdited` ops on `"Main"`.
* No extra operations.

#### 5.1.2 `pg6_2_sheet_removed_no_grid_ops_on_main`

Structure mirrors 5.1.1 but for removal:

* Input: `pg6_sheet_removed_a.xlsx` vs `pg6_sheet_removed_b.xlsx`.
* Assertions:

  * Exactly one `SheetRemoved("OldSheet")`.
  * No grid ops on `"Main"`.
  * No other ops.

#### 5.1.3 `pg6_3_rename_as_remove_plus_add_no_grid_ops`

* Input: `pg6_sheet_renamed_a.xlsx` vs `pg6_sheet_renamed_b.xlsx`.
* Assertions:

  * `report.ops.len() == 2`.
  * Exactly one `SheetRemoved("OldName")`.
  * Exactly one `SheetAdded("NewName")`.
  * No grid-level ops at all (`matches!(op, SheetAdded|SheetRemoved)` for every op).

This codifies the “remove+add” rename semantics, consistent with current implementation.

#### 5.1.4 `pg6_4_sheet_and_grid_change_composed_cleanly`

* Input: `pg6_sheet_and_grid_change_a.xlsx` vs `_b.xlsx`.
* Assertions:

  * Exactly one `SheetAdded("Scratch")`.
  * At least one `CellEdited` whose `sheet == "Main"`.
  * **All** non-sheet-level ops must be `CellEdited` for `"Main"`:

    * No `Row*`/`Column*` ops for this scenario.
    * No grid-level ops for `"Aux"`.

Pseudo-structure:

```rust
#[test]
fn pg6_4_sheet_and_grid_change_composed_cleanly() {
    let old = open_workbook(fixture_path("pg6_sheet_and_grid_change_a.xlsx")).unwrap();
    let new = open_workbook(fixture_path("pg6_sheet_and_grid_change_b.xlsx")).unwrap();
    let report = diff_workbooks(&old, &new);

    let mut scratch_added = 0;
    let mut main_cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Scratch" => scratch_added += 1,
            DiffOp::CellEdited { sheet, .. } => {
                assert_eq!(sheet, "Main", "only Main should have cell edits");
                main_cell_edits += 1;
            }
            DiffOp::SheetRemoved { .. } => {
                panic!("no sheets should be removed in this scenario: {:?}", op);
            }
            DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. } => {
                panic!("no structural row/column ops expected in PG6.4: {:?}", op);
            }
        }
    }

    assert_eq!(scratch_added, 1);
    assert!(main_cell_edits > 0);
}
```

### 5.2 New Python generator: PG6 fixtures

Extend `fixtures/src/generators/grid.py` with a new generator, for example:

```python
class Pg6SheetScenarioGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
        if len(output_names) != 2:
            raise ValueError("PG6 scenarios expect exactly two outputs (A and B)")

        mode = self.args.get("mode")
        a_name, b_name = output_names

        if mode == "sheet_added":
            self._gen_sheet_added(output_dir / a_name, output_dir / b_name)
        elif mode == "sheet_removed":
            self._gen_sheet_removed(output_dir / a_name, output_dir / b_name)
        elif mode == "sheet_renamed":
            self._gen_sheet_renamed(output_dir / a_name, output_dir / b_name)
        elif mode == "sheet_and_grid_change":
            self._gen_sheet_and_grid_change(output_dir / a_name, output_dir / b_name)
        else:
            raise ValueError(f"Unsupported PG6 mode: {mode}")
```

Each helper (`_gen_sheet_added`, `_gen_sheet_removed`, `_gen_sheet_renamed`, `_gen_sheet_and_grid_change`) should:

* Use openpyxl to build small workbooks exactly matching the fixture sketches in Section 2.
* Ensure sheet names (`"Main"`, `"NewSheet"`, `"OldSheet"`, `"OldName"`, `"NewName"`, `"Aux"`, `"Scratch"`) and shapes match the PG6 descriptions.

### 5.3 Manifest entries

Add entries to `fixtures/manifest.yaml` to wire the generator:

Example shape (pseudo-YAML):

```yaml
- id: pg6_sheet_added
  kind: excel_pair
  generator: pg6_sheet_scenario
  mode: sheet_added
  a: "fixtures/generated/pg6_sheet_added_a.xlsx"
  b: "fixtures/generated/pg6_sheet_added_b.xlsx"

- id: pg6_sheet_removed
  kind: excel_pair
  generator: pg6_sheet_scenario
  mode: sheet_removed
  a: "fixtures/generated/pg6_sheet_removed_a.xlsx"
  b: "fixtures/generated/pg6_sheet_removed_b.xlsx"

- id: pg6_sheet_renamed
  kind: excel_pair
  generator: pg6_sheet_scenario
  mode: sheet_renamed
  a: "fixtures/generated/pg6_sheet_renamed_a.xlsx"
  b: "fixtures/generated/pg6_sheet_renamed_b.xlsx"

- id: pg6_sheet_and_grid_change
  kind: excel_pair
  generator: pg6_sheet_scenario
  mode: sheet_and_grid_change
  a: "fixtures/generated/pg6_sheet_and_grid_change_a.xlsx"
  b: "fixtures/generated/pg6_sheet_and_grid_change_b.xlsx"
```

The generator key (`pg6_sheet_scenario`) should match the name used to register `Pg6SheetScenarioGenerator` in the generator mapping used by `fixtures/src/generate.py`.

### 5.4 Existing tests to keep an eye on

When implementing this cycle, the implementer should ensure:

* `core/tests/engine_tests.rs` still pass; PG6 tests effectively tighten the expectations around sheet identity but must remain consistent with prior tests for case-insensitivity and kind-based identity.
* `core/tests/pg5_grid_diff_tests.rs` remain unchanged and passing, ensuring no accidental change to grid diff semantics while we only wrap new tests around `diff_workbooks`.
* `core/tests/output_tests.rs` (including existing `sheet_case_only_rename` integration tests) still behave as before; PG6 does not alter JSON shape or rename behavior for those pre-existing fixtures.

---

## 6. Milestone linkage

This mini-spec advances:

* **Testing milestone:** PG6 – Object graph vs grid responsibilities (PG6.1–PG6.4 fully implemented and codified as tests; PG6.5 explicitly deferred).
* **Phase:** Phase 2 of the testing plan (IR semantics and streaming budget), by solidifying the separation between sheet-level diff and grid-level diff before moving to G1–G7 end-to-end grid diff tests.

Future follow-ups:

* A small, dedicated milestone to implement PG6.5 once sheet rename detection policy is designed (either new `SheetRenamed` DiffOp or an extended remove+add+grid-diff scheme).
* Subsequent cycles can then safely layer G1–G3 fixture-based grid tests on top of this PG6 suite, confident that object graph vs grid responsibilities are locked down.

