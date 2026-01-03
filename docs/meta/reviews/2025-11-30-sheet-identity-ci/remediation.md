# Remediation Plan: 2025-11-30-sheet-identity-ci

## Overview

The branch is safe to release as-is. The items below are non-blocking improvements aimed at:

* Guarding the new deterministic ordering behavior with tests.
* Adding an end-to-end integration check for case-insensitive sheet identity.
* Aligning the docs-vs-implementation report with the new reality.
* Optionally clarifying how duplicate sheet identity keys on a single side should be treated.

All of these can be scheduled in a follow-up cycle without blocking this release.

---

## Fixes Required (Follow-up / Non-blocking)

### Fix 1: Add explicit test for deterministic workbook-level operation ordering

* **Addresses Finding**: 3

* **Changes**:

  * File: `core/tests/engine_tests.rs`. 
  * Add a new test that constructs workbooks with multiple sheet-level changes and asserts the exact order of `DiffOp`s. For example:

    * Old: `["Budget", "Sheet1"]`, New: `["Budget", "sheet1", "Summary"]` with a mix of adds and cell edits.
    * Check that the operations are ordered by `name_lower` (`"budget"`, `"sheet1"`, `"summary"`) and then by `SheetKind` where applicable.
  * The test should call `diff_workbooks`, capture `report.ops`, and assert the full ordered list of op variants (e.g., `SheetRemoved`, `SheetAdded`, `CellEdited`) and their `sheet` fields, not just the presence of ops.

* **Tests**:

  * New test: `deterministic_sheet_op_ordering` (name is flexible).
  * Ensure this test fails if the sort is removed or if iteration order depends on `HashMap` traversal.

---

### Fix 2: Add an Excel fixture pair for case-only sheet-name differences

* **Addresses Finding**: 4

* **Changes**:

  * Fixtures:

    * Add a pair like `sheet_case_only_rename_a.xlsx` and `sheet_case_only_rename_b.xlsx` under `fixtures/generated/`, using the existing Python generator framework. 

      * A: one sheet named `Sheet1`, with a small grid (e.g., A1 = 1.0).
      * B: same workbook but sheet renamed to `sheet1` (or `Summary` → `SUMMARY`), with identical cell contents.
    * Optionally a second pair where the sheet has a case-only rename **and** a cell edit (A1 1.0 → 2.0) to mirror the unit tests.
  * Tests:

    * File: `core/tests/output_tests.rs` or `core/tests/integration_test.rs`.
    * New test (e.g., `json_diff_case_only_sheet_name_no_changes`):

      * Open both workbooks via `open_workbook`.
      * Call `diff_workbooks` or `diff_workbooks_to_json`.
      * Assert that the diff is empty for the “no changes” pair.
    * Optional second test (e.g., `json_diff_case_only_sheet_name_cell_edit`):

      * Assert that the diff contains exactly one `CellEdited` with the **old** sheet name.

* **Tests**:

  * `json_diff_case_only_sheet_name_no_changes`
  * `json_diff_case_only_sheet_name_cell_edit` (optional but recommended)

---

### Fix 3: Update docs-vs-implementation D1 status

* **Addresses Finding**: 5

* **Changes**:

  * File: `docs/rust_docs/2025-11-30-docs-vs-implementation.md`. 
  * In the “Known Discrepancies” table, change the D1 row:

    * Status from `Open` → `Resolved (2025-11-30-sheet-identity-ci)`.
    * Recommendation can be adjusted to reference ongoing work (e.g., focus on grid diff, row signatures).
  * Optionally add a brief note in the body stating that sheet identity semantics now match the spec (case-insensitive name + type) and are locked in with engine tests.

* **Tests**:

  * None (documentation-only change).

---

### Fix 4: Decide and document behavior for duplicate sheet identity keys on a single side

* **Addresses Finding**: 6

* **Changes**:

  * Clarify in `excel_diff_specification.md` and/or `spec_2025-11-30-sheet-identity-ci.md` how to treat the (theoretical) case where a single workbook’s `sheets` vector contains multiple `Sheet` entries with the same `(lowercase(name), kind)` identity.

    * Options:

      1. Declare this invalid and state that producers (including `open_workbook`) must not create such IR; optionally add `debug_assert!` in `diff_workbooks` to catch it in debug builds.
      2. Define a deterministic rule (e.g., first-wins or last-wins) and document it explicitly.
  * Implementation-side (optional, depending on chosen policy):

    * In `diff_workbooks`, as `old.sheets.iter()` and `new.sheets.iter()` are collected into maps, detect when a `SheetKey` already exists:

      * Either assert (for “invalid IR”) or log a warning if logging is available.
  * This change is purely defensive; no behavior should change for real Excel inputs.

* **Tests**:

  * If you decide to assert on duplicates, add a unit test that constructs a `Workbook` with two `Sheet`s that have the same identity key and verifies the chosen behavior:

    * For “invalid IR”: test that debug builds hit the assertion (or that a specific error path is taken if you choose to make it fallible).
    * For “defined duplicates behavior”: test that the diff is consistent with that policy.

---

## Constraints

* Do **not** change the public API surface (`diff_workbooks` signature, `DiffOp` schema, `SheetId` type alias) as per the mini-spec; all changes should remain internal or in tests/docs. 
* Keep performance characteristics the same or better:

  * New tests and fixtures may increase CI time slightly but should not affect runtime for library consumers.
* Maintain backward compatibility of JSON output (operation content, not order, is the externally observable contract today; new ordering tests should focus on internal consistency, not JSON schema changes).

---

## Expected Outcome

After these follow-up items:

* Deterministic ordering of workbook-level operations will be explicitly tested and thus robust against future refactors.
* Case-insensitive sheet identity will be validated end-to-end against real `.xlsx` fixtures, not just in-memory IR.
* The docs-vs-implementation report will accurately reflect that D1 is resolved, reducing cognitive dissonance for future reviewers.
* The behavior around hypothetical duplicate sheet identity keys on a single side will be clearly documented and, if desired, asserted, closing a minor conceptual gap.

The current branch is suitable for release now; the remediation items above are quality and safety improvements for future cycles rather than blockers.
