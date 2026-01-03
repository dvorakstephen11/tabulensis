````markdown
# Verification Report: 2025-12-02b-pg6-object-vs-grid

## Summary

The implementation on branch **2025-12-02b-pg6-object-vs-grid** cleanly delivers PG6.1–PG6.4 as described in the mini-spec: new PG6 fixtures are generated via a dedicated Python generator, wired into `fixtures/manifest.yaml`, and exercised by a focused Rust integration test module. The core diff engine and DiffOp/JSON contracts remain unchanged and continue to be validated by existing PG1–PG5 and PG4 JSON tests. I found **no Critical or Moderate issues**. There are a few **Minor coverage gaps / future-work notes** (e.g., PG6.5 still intentionally deferred, and PG6 behavior is not explicitly re-asserted through the JSON/CLI path), but these do not block release.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### Finding 1: PG6 scope narrowed to PG6.1–PG6.4; PG6.5 still outstanding

- **Severity**: Minor  
- **Category**: Gap  
- **Description**:  
  The original testing plan’s PG6 milestone describes **five** scenarios (PG6.1–PG6.5), including `pg6_renamed_and_changed` for rename-plus-grid-edits semantics. The **cycle mini-spec** explicitly scopes this branch to PG6.1–PG6.4 and calls out PG6.5 as a follow-up milestone once rename semantics are decided. The current codebase has fixtures and tests only for:

  - `pg6_sheet_added_{a,b}.xlsx`  
  - `pg6_sheet_removed_{a,b}.xlsx`  
  - `pg6_sheet_renamed_{a,b}.xlsx`  
  - `pg6_sheet_and_grid_change_{a,b}.xlsx`   

  There is no generator or manifest entry for `pg6_renamed_and_changed_{a,b}.xlsx`, and no corresponding test. This matches the mini-spec but is a partial implementation of the broader testing-plan section for PG6.   

- **Evidence**:  
  - PG6 section in `excel_diff_testing_plan.md` lists PG6.5 (“Rename plus grid edits semantics”) with its own fixture and expectations.   
  - The cycle plan explicitly scopes the work to **PG6.1–PG6.4** and lists PG6.5 as follow-up, not in scope for this branch. :contentReference[oaicite:3]{index=3}  
  - `fixtures/manifest.yaml` only contains PG6 entries for `pg6_sheet_added`, `pg6_sheet_removed`, `pg6_sheet_renamed`, and `pg6_sheet_and_grid_change`.   

- **Impact**:  
  - No impact on the **correctness** of this branch; the behavior contract for the four implemented PG6 scenarios is thoroughly exercised.
  - PG6 as a *full* milestone (including rename+edits semantics) is still incomplete from the perspective of the global testing plan. This should be tracked as a separate, explicit follow-up milestone, as the mini-spec already suggests.

---

### Finding 2: PG6 invariants not re-asserted through JSON/CLI path

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The new PG6 tests operate directly on the core API:

  ```rust
  let report = diff_workbooks(&old, &new);
````

and assert on `DiffOp` sequences in memory.

There are **no tests** that:

* Call `diff_workbooks_to_json` on the PG6 fixtures, and
* Parse the resulting `DiffReport` via JSON to assert the same absence/presence of ops.

Existing JSON tests (PG4, simple cell changes, case-only rename) validate the JSON shape and mapping of DiffOps, including `CellEdited`, `RowAdded/Removed`, `ColumnAdded/Removed`, and sheet-case-only rename behavior.

So the mapping from `DiffReport` to JSON is well covered in general, but the **specific PG6 fixture scenarios** are not exercised end-to-end through the JSON helper.

* **Evidence**:

  * PG6 tests live in `core/tests/pg6_object_vs_grid_tests.rs` and exclusively use `diff_workbooks`.
  * JSON tests in `core/tests/output_tests.rs` cover basic cell edits and case-only sheet rename, but they don’t reference any `pg6_*` fixtures.

* **Impact**:

  * If a future change accidentally alters how `DiffReport` is converted to JSON (e.g., filtering or transforming ops in `diff_workbooks_to_json`), PG6 invariants might be violated at the JSON/CLI layer without being caught by tests.
  * Risk is mitigated by strong PG4 JSON tests and the fact that `diff_workbooks_to_json` is a thin wrapper around the same engine; this is more of a defense-in-depth / regression-hardening gap than a functional bug.

---

### Finding 3: PG6.4 test intentionally under-specifies number/locations of edits

* **Severity**: Minor

* **Category**: Missing Test (precision)

* **Description**:
  In `pg6_4_sheet_and_grid_change_composed_cleanly`, the test asserts:

  * Exactly one `SheetAdded("Scratch")`.
  * That **all** non-sheet-level operations are `CellEdited` and their `sheet` is `"Main"` (no ops on `"Aux"`; no row/col ops).
  * That there is **at least one** `CellEdited` on `"Main"`.

  The test does **not** pin:

  * The exact number of `CellEdited` operations (the fixture currently makes three edits), or
  * The specific cell addresses (e.g., `A1`, `B2`, `C3`).

  The mini-spec and testing plan were deliberately written to *allow* this slack—“whatever `CellEdited` ops are appropriate for the few cell tweaks” rather than a fixed count—so this is not a spec deviation.

* **Evidence**:

  * PG6.4 test code in `core/tests/pg6_object_vs_grid_tests.rs`. 
  * PG6.4 description in `cycle_plan.md` and `excel_diff_testing_plan.md`.
  * Fixture generator `_gen_sheet_and_grid_change` creates exactly three edited cells on `"Main"` (A1, B2, C3), but this is not asserted in tests. 

* **Impact**:

  * Future refactors of grid diff that change the *pattern* of edits on `"Main"` (for the same underlying cell changes) could go unnoticed if they still emit at least one `CellEdited` and keep all ops on `"Main"`.
  * From a contract perspective, the key property—which sheets get grid ops, and that no row/column ops appear—is still fully enforced. This is why this is classified as Minor: it’s about precision of regression checks, not about correctness of current behavior.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed

  * New PG6 fixtures generator `Pg6SheetScenarioGenerator` added and wired.
  * `fixtures/manifest.yaml` contains the four PG6 fixture pairs.
  * New Rust test module `core/tests/pg6_object_vs_grid_tests.rs` implements PG6.1–PG6.4 tests exactly per mini-spec structure.
  * No changes to `diff_workbooks` or grid diff algorithms beyond existing design.

* [x] All specified tests created

  * PG6.1: `pg6_1_sheet_added_no_grid_ops_on_main`
  * PG6.2: `pg6_2_sheet_removed_no_grid_ops_on_main`
  * PG6.3: `pg6_3_rename_as_remove_plus_add_no_grid_ops`
  * PG6.4: `pg6_4_sheet_and_grid_change_composed_cleanly`
  * PG6.5 is explicitly out of scope for this cycle per mini-spec.

* [x] Behavioral contract satisfied

  * **Sheet add (PG6.1)**: Only `SheetAdded("NewSheet")`; no row/column/cell ops on `"Main"`; no extra ops (`report.ops.len() == 1`).
  * **Sheet remove (PG6.2)**: Only `SheetRemoved("OldSheet")`; no grid ops on `"Main"`; no extra ops (`report.ops.len() == 1`).
  * **Rename (PG6.3)**: Exactly one `SheetRemoved("OldName")` plus one `SheetAdded("NewName")`; no grid-level ops.
  * **Mixed sheet+grid change (PG6.4)**: Exactly one `SheetAdded("Scratch")`; all non-sheet ops are `CellEdited` for `"Main"`; no ops on `"Aux"`; no row/column/block-move ops.
  * This matches the intended separation of responsibilities described in the spec and grid algorithm documents.

* [x] No undocumented deviations from spec

  * Cycle summary explicitly states “No deviations from the mini-spec; grid diff code unchanged.” 
  * Review of `core/src/engine.rs`, `core/tests/engine_tests.rs`, and PG4/PG5 tests shows behavior consistent with prior sheet identity, ordering, and grid diff semantics.

* [x] Error handling adequate

  * PG6 generator validates its inputs (`len(output_names) == 2` and a supported `mode`), raising clear `ValueError`s otherwise. 
  * Existing error-handling tests (e.g., invalid ZIP container, invalid addresses) still cover the I/O and JSON boundaries.
  * PG6 tests are designed to panic on **any** unexpected DiffOp variant, which aggressively surfaces deviations as test failures.

* [x] No obvious performance regressions

  * No changes were made to core diff algorithms; only fixtures and tests were added.
  * PG6 fixtures are tiny (3×3, 5×5 grids) and won’t materially affect runtime or memory of the test suite.

---

Given the above, **no remediation is required for this branch**. The remaining items (PG6.5 semantics and JSON-level PG6 tests) are best handled as separate, planned follow-up work rather than blocking this cycle’s release.

```
```
