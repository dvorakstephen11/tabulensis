# Remediation Plan: 2025-11-30-sheet-identity-ci

## Overview

This plan addresses the three minor gaps identified in the post-implementation verification:

1. No test that directly asserts the **release-mode** behavior for duplicate sheet identity keys (“last writer wins”).
2. No tests that explicitly exercise **`SheetKind::Macro` and `SheetKind::Other`** in the sheet identity / ordering logic.
3. No tests that verify the **JSON helper** (`diff_workbooks_to_json`) against the new **case-only sheet-rename fixtures**, even though the engine-level behavior is covered.

All fixes are test-only and non-breaking. No changes to the public API surface (`diff_workbooks`, `DiffOp`, JSON schema) are required. :contentReference[oaicite:0]{index=0} :contentReference[oaicite:1]{index=1}

---

## Fixes Required

### Fix 1: Release-mode test for duplicate sheet identity “last writer wins”

- **Addresses Finding**:  
  “Duplicate sheet identity ‘last writer wins’ behavior is not directly asserted.”

- **Severity Target**: Minor (non-blocking, but good to lock down)

- **Changes**

  **Files:**
  - `core/tests/engine_tests.rs` :contentReference[oaicite:2]{index=2}

  **Add:**
  - A new test that *only* runs when `debug_assert!`s are disabled (release-like builds) to assert deterministic “last writer wins” behavior for duplicate sheet identity keys.

  **Proposed shape:**

  1. Reuse or mirror the helper `make_sheet_with_kind` already in `engine_tests.rs`. :contentReference[oaicite:3]{index=3}
  2. Construct an `old` workbook with two sheets that collide on `(lowercase(name), SheetKind)`:

     - `SheetKind::Worksheet` for both.
     - Names differ by case so that `name.to_lowercase()` matches:
       - `"Sheet1"` (first)
       - `"sheet1"` (second)

  3. Keep `new` workbook empty (so the diff produces only `SheetRemoved` ops).
  4. Gate the test with `#[cfg(not(debug_assertions))]` so it is only compiled/run when `debug_assertions` are off:

     - In that configuration, `debug_assert!` is a no-op and `diff_workbooks` will complete.
     - Assert:
       - `report.ops.len() == 1`
       - The single op is `DiffOp::SheetRemoved { sheet }`
       - `sheet` equals the **second** sheet’s name (the last one inserted into `Workbook.sheets`), proving last-writer-wins semantics for invalid IR.

  **Notes for implementer:**

  - Keep the existing `duplicate_sheet_identity_panics_in_debug` test untouched; it already validates the debug-build contract. :contentReference[oaicite:4]{index=4}
  - This new test complements it by pinning the **release-mode** deterministic behavior described in the spec and remediation docs. :contentReference[oaicite:5]{index=5}

- **Tests**

  - New test (name suggestion):  
    `duplicate_sheet_identity_last_writer_wins_release`
  - Confirm it is conditionally compiled:

    ```rust
    #[cfg(not(debug_assertions))]
    #[test]
    fn duplicate_sheet_identity_last_writer_wins_release() { /* ... */ }
    ```

  - Ensure CI runs at least one `cargo test --release` job so this test executes somewhere in the pipeline (or document that it’s a “release-only” guard).

---

### Fix 2: Explicit coverage for `SheetKind::Macro` / `SheetKind::Other`

- **Addresses Finding**:  
  “No explicit tests exercise `SheetKind::Macro` / `SheetKind::Other` ordering.”

- **Severity Target**: Minor

- **Goals**

  1. Pin the **ranking** used by `sheet_kind_order` (`Worksheet < Chart < Macro < Other`) so future refactors cannot accidentally reorder or omit kinds. :contentReference[oaicite:6]{index=6} :contentReference[oaicite:7]{index=7}  
  2. Exercise the sheet-identity behavior for Macro/Other variants, analogous to the existing Worksheet/Chart test (`sheet_identity_includes_kind`). :contentReference[oaicite:8]{index=8}

- **Changes**

  **Files:**
  - `core/src/engine.rs`
  - `core/tests/engine_tests.rs` :contentReference[oaicite:9]{index=9}

  #### 2.1 Unit test for `sheet_kind_order` ranking

  **In `core/src/engine.rs`:**

  - Add a small `#[cfg(test)]` module at the bottom of the file that directly tests `sheet_kind_order`:

    - Assert that:

      ```rust
      sheet_kind_order(&SheetKind::Worksheet) < sheet_kind_order(&SheetKind::Chart);
      sheet_kind_order(&SheetKind::Chart)     < sheet_kind_order(&SheetKind::Macro);
      sheet_kind_order(&SheetKind::Macro)     < sheet_kind_order(&SheetKind::Other);
      ```

    - This guarantees that:

      - The helper continues to assign a rank for *every* enum variant (including Macro/Other).
      - The ordering contract documented in the mini-spec stays enforced by tests. :contentReference[oaicite:10]{index=10}

  #### 2.2 Engine-level identity test for Macro vs Other

  **In `core/tests/engine_tests.rs`:**

  - Add an identity test mirroring `sheet_identity_includes_kind`, but using `SheetKind::Macro` and `SheetKind::Other`:

    - Construct a tiny `Grid` with a single cell (e.g., A1 = 1.0).
    - Build:

      - `old` workbook with a single `Sheet`:
        - `name: "Code"`
        - `kind: SheetKind::Macro`
        - `grid`: the tiny grid
      - `new` workbook with a single `Sheet`:
        - `name: "Code"`
        - `kind: SheetKind::Other`
        - `grid`: same tiny grid

    - Call `diff_workbooks(&old, &new)` and assert:

      - Exactly one `SheetRemoved { sheet: "Code" }`
      - Exactly one `SheetAdded { sheet: "Code" }`
      - No other ops (`report.ops.len() == 2`)

    - This mirrors the existing Worksheet/Chart test and ensures kind is part of sheet identity even for the “less common” variants. :contentReference[oaicite:11]{index=11}

- **Tests**

  - New tests (name suggestions):

    - In `engine.rs`:
      - `sheet_kind_order_ranking_includes_macro_and_other`
    - In `engine_tests.rs`:
      - `sheet_identity_includes_kind_for_macro_and_other`

  - Together, these:

    - Explicitly cover all `SheetKind` variants in ordering logic.
    - Extend identity coverage beyond Worksheet/Chart to Macro/Other.

---

### Fix 3: JSON-level tests for case-only sheet renames

- **Addresses Finding**:  
  “Case-insensitive identity is not validated via the JSON helper for the new fixtures.”

- **Severity Target**: Minor

- **Goals**

  - Ensure `diff_workbooks_to_json` preserves the case-insensitive sheet identity semantics all the way out at the JSON surface, using the new real-Excel fixtures.
  - Close the loop between:

    - Engine unit tests (`sheet_name_case_insensitive_*`), and
    - Integration tests that already open the fixtures but call `diff_workbooks` directly. :contentReference[oaicite:12]{index=12} :contentReference[oaicite:13]{index=13}

- **Changes**

  **Files:**
  - `core/tests/output_tests.rs` :contentReference[oaicite:14]{index=14}  

  **Existing context:**

  - There are already tests like `test_json_empty_diff`, `test_json_non_empty_diff`, and `test_json_diff_value_to_empty` that exercise `diff_workbooks_to_json` with other fixtures. :contentReference[oaicite:15]{index=15}  
  - There are *new* tests `json_diff_case_only_sheet_name_no_changes` and `json_diff_case_only_sheet_name_cell_edit` which:
    - Open `sheet_case_only_rename*.xlsx` fixtures via `open_workbook`
    - Call `diff_workbooks` directly, **not** the JSON helper.
    - Assert engine-level behavior (empty ops vs single `CellEdited`). :contentReference[oaicite:16]{index=16}  

  **Add:**

  Two additional tests that mirror these scenarios but go through `diff_workbooks_to_json` and JSON parsing:

  1. **`test_json_case_only_sheet_name_no_changes`**

     - Use `fixture_path("sheet_case_only_rename_a.xlsx")` and `_b.xlsx` (same fixtures as the existing test). :contentReference[oaicite:17]{index=17}  
     - Call:

       ```rust
       let json = diff_workbooks_to_json(&a, &b)
           .expect("diffing case-only sheet rename should succeed");
       let report: DiffReport = serde_json::from_str(&json)
           .expect("json should parse");
       ```

     - Assert:

       - `report.ops.is_empty()` – no ops for pure case-only rename with identical content.

  2. **`test_json_case_only_sheet_name_cell_edit`**

     - Use `sheet_case_only_rename_edit_a.xlsx` / `_b.xlsx`.
     - Call `diff_workbooks_to_json` and parse into `DiffReport`.
     - Assert:

       - `report.ops.len() == 1`
       - The single op is `DiffOp::CellEdited { sheet, addr, from, to, .. }`.
       - `sheet == "Sheet1"` (canonical old name).
       - `addr.to_a1() == "A1"`.
       - `render_value(&from.value) == Some("1".into())`.
       - `render_value(&to.value) == Some("2".into())`.

     - This parallels the existing engine-level fixture test, but validates the behavior at the JSON helper boundary.

- **Tests**

  - New tests (name suggestions):

    - `test_json_case_only_sheet_name_no_changes`
    - `test_json_case_only_sheet_name_cell_edit_via_helper`

  - They should sit alongside the existing JSON tests in `output_tests.rs`, using the same `render_value` helper and fixture harness. :contentReference[oaicite:18]{index=18}

---

## Constraints

- **API Stability**  
  - Do **not** change:
    - `pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport`
    - `SheetId` alias
    - `DiffOp`/`DiffReport` public shape
    - JSON schema produced by `serialize_diff_report`/`diff_workbooks_to_json` :contentReference[oaicite:19]{index=19}  

- **Performance**  
  - All changes are tests; they run only in CI/dev and do not affect library runtime complexity or memory behavior. :contentReference[oaicite:20]{index=20}  

- **Debug vs Release**  
  - For Fix 1, use `#[cfg(not(debug_assertions))]` to avoid conflicts with the existing debug-assert test; ensure CI has at least one release-mode test run if you want that guard to actually execute.

- **Fixture Usage**  
  - Reuse existing fixtures (`sheet_case_only_rename*.xlsx`) and helpers (`fixture_path`, `open_workbook`) rather than adding new files. :contentReference[oaicite:21]{index=21}  

---

## Expected Outcome

After completing this remediation:

1. **Duplicate sheet identity semantics** are fully pinned in tests:
   - Debug builds assert on invalid IR.
   - Release builds deterministically choose the last sheet and this behavior is exercised by a dedicated test.

2. **All `SheetKind` variants** (Worksheet, Chart, Macro, Other) are covered:
   - The internal kind ranking is locked in by unit tests.
   - Identity semantics are verified for Macro/Other, not just Worksheet/Chart.

3. **Case-insensitive sheet identity** is verified end-to-end:
   - From Excel fixtures through `open_workbook` and `diff_workbooks` to `diff_workbooks_to_json` and JSON parsing.
   - Any future change that accidentally re-introduces case-sensitive behavior at the JSON helper layer will be caught by tests.

These changes close the minor coverage gaps identified during verification without altering the implementation’s external behavior or performance characteristics. :contentReference[oaicite:22]{index=22} :contentReference[oaicite:23]{index=23}
