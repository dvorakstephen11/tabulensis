# Remediation Plan: 2025-11-29-refactor

## Overview

The current branch is safe to release, but there are a few improvements that should be addressed in a follow‑up commit or next cycle:

1. Strengthen tests for the new `DiffReport` → `CellDiff` helper.  
2. Add targeted tests for less common `ExcelOpenError` cases, especially the new variants.  
3. Bring the long‑form spec in line with the new layered error and diff design.  
4. Clarify performance expectations for signatures/diff or adjust implementation in a future perf‑focused cycle.  
5. Add small unit tests around `CellValue` convenience methods.

These changes are non‑blocking but will reduce future risk and keep documentation and code aligned.

---

## Fixes Required

### Fix 1: Test `diff_report_to_cell_diffs` behavior

- **Addresses Finding**: 1 (Missing test for `diff_report_to_cell_diffs`)  
- **Changes**:
  - File: `core/tests/output_tests.rs`:contentReference[oaicite:33]{index=33}  
  - Add new tests that:
    1. Manually construct a `DiffReport` with a couple of `DiffOp::CellEdited` entries on the same and different sheets and verify:
       * `diff_report_to_cell_diffs` returns one `CellDiff` per `CellEdited`.  
       * `coords` equals `addr.to_a1()` for each op.  
       * `value_file1` and `value_file2` are strings derived from `CellValue` using the same rendering logic as `diff_workbooks_to_json` (number/text/bool).  
    2. Include at least one `SheetAdded` / `SheetRemoved` / `RowAdded` (using `DiffOp` constructors) in the same `DiffReport` and assert they are **ignored** by `diff_report_to_cell_diffs`.  
- **Tests**:
  - `diff_report_to_cell_diffs_filters_non_cell_ops()`  
  - `diff_report_to_cell_diffs_maps_values_correctly()`  

---

### Fix 2: Add tests for unexercised `ExcelOpenError` variants

- **Addresses Finding**: 2 (Untested error variants)  
- **Changes**:
  - File: `core/tests/excel_open_xml_tests.rs`:contentReference[oaicite:36]{index=36}  
  - Add tests that construct minimal adversarial OPC packages (via small `.xlsx` fixtures or in‑test ZIP generation) to trigger specific error paths:
    1. **WorkbookXmlMissing**  
       * Create an OPC container with `[Content_Types].xml` but no `xl/workbook.xml`.  
       * Assert `open_workbook` returns `Err(ExcelOpenError::WorkbookXmlMissing)`.:contentReference[oaicite:37]{index=37}  
    2. **WorksheetXmlMissing { sheet_name }**  
       * Create a workbook XML that declares a sheet but omit the corresponding `xl/worksheets/...` part.  
       * Assert the error is `WorksheetXmlMissing { sheet_name: <declared sheet name> }`.:contentReference[oaicite:38]{index=38}  

  - File: `core/tests/output_tests.rs`:contentReference[oaicite:39]{index=39}  
    * Add a unit test that exercises the `SerializationError` mapping without needing a real Excel file:
      - Manually build a `DiffReport` containing a `CellSnapshot` with `CellValue::Number(f64::NAN)`, call `serialize_diff_report`, and assert it returns an error. Then wrap that error exactly as `diff_workbooks_to_json` does into an `ExcelOpenError::SerializationError(String)` and assert the variant and that the message contains a helpful description (e.g., `NaN` not supported).  
      - Optionally, in a later perf/robustness cycle, add a Python‑generated Excel fixture that tries to encode `NaN` to exercise this end‑to‑end through `diff_workbooks_to_json`.
- **Tests**:
  - `missing_workbook_xml_returns_workbookxmlmissing()`  
  - `missing_worksheet_xml_returns_worksheetxmlmissing()`  
  - `serialize_diff_report_nan_maps_to_serialization_error()`  

---

### Fix 3: Update architecture docs for new error and diff layering

- **Addresses Finding**: 3 (Documentation drift)  
- **Changes**:
  - File: `docs/rust_docs/excel_diff_specification.md`  
  - Update sections that:
    * Present the `ExcelOpenError` enum to reflect the current layered design:
      - Top‑level variants: `Container(#[from] ContainerError)`, `GridParse(#[from] GridParseError)`, `DataMashup(#[from] DataMashupError)`, `WorkbookXmlMissing`, `WorksheetXmlMissing{…}`, `SerializationError`.  
      - Move host‑ and DataMashup‑specific details into `ContainerError` and `DataMashupError` sections.  
    * Describe the diff pipeline to emphasize:
      - `engine::diff_workbooks` as the canonical IR producer (`DiffReport`).  
      - `output::json::diff_report_to_cell_diffs` and `serialize_diff_report` as separate projection/serialization layers, with `diff_workbooks_to_json` as a convenience shim for the CLI/fixtures.  
  - Optionally, annotate older PG3/PG4/PG5 sections with a note that they reflect “pre‑refactor shape; see spec_2025‑11‑29‑refactor for current architecture”.
- **Tests**:
  - No code tests needed; but a quick manual/agent doc review in the next cycle should confirm that the architecture docs and mini‑spec tell a consistent story.

---

### Fix 4: Clarify or adjust performance expectations for signatures and diff

- **Addresses Finding**: 4 (Signatures/diff still O(R×C) in practice)  
- **Changes**:
  - **Short term (docs)**:
    * In `spec_2025-11-29-refactor.md` Section 8.3, clarify that the current implementation still iterates across the full used range and that “O(populated_cells) in practice” is aspirational, to be realized in a future H1 cycle.  
  - **Longer term (future perf cycle)**:
    * When working on H1 “high‑performance grid diff”, adjust:
      - `Grid::compute_all_signatures` to compute row/col signatures by iterating over `self.cells` and aggregating per row/col, so work scales with populated cells rather than full dimensions.  
      - `engine::diff_grids` to leverage signatures and/or sparse iteration instead of scanning every coordinate in `0..nrows × 0..ncols`.  
    * Tie these changes explicitly to the P1/P2 performance milestones in `excel_diff_testing_plan.md` with new large‑grid fixtures and metrics.  
- **Tests**:
  - New performance/behavioral tests in a perf‑focused cycle:
    * Large sparse sheet fixture (`grid_large_sparse_*`) asserting diff runtime is closer to O(populated) and that results are identical to the current implementation.  

---

### Fix 5: Add unit tests for `CellValue` helpers

- **Addresses Finding**: 5 (Missing tests for `CellValue::as_*`)  
- **Changes**:
  - File: `core/src/workbook.rs` (test module)  
  - Add small, focused tests that:
    * Create instances of `CellValue::Text`, `CellValue::Number`, and `CellValue::Bool`.  
    * Assert:
      - `as_text` returns `Some(&str)` only for the `Text` variant and `None` otherwise.  
      - `as_number` returns `Some(f64)` only for `Number`.  
      - `as_bool` returns `Some(bool)` only for `Bool`.  
- **Tests**:
  - `cellvalue_as_text_number_bool_match_variants()`  

---

## Constraints

- Keep changes backwards‑compatible with the interfaces marked “expected stable” in the decision record (especially `Grid` API, `engine::diff_workbooks`, `DiffOp`, `ExcelOpenError`, and `OpcContainer`).:contentReference[oaicite:51]{index=51}  
- Do not change the JSON wire shape of `CellDiff` or `DiffReport` when adding tests or small helpers; existing fixtures and consumers rely on these schemas.  
- Any performance‑related changes (Fix 4, longer term) should be gated behind explicit P1/P2 perf tests and metrics as described in the testing plan.

---

## Expected Outcome

After these remediation steps:

1. The `DiffReport` → `CellDiff` projection will be covered by explicit tests, reducing risk when adding new `DiffOp` variants or algorithms.  
2. All high‑level `ExcelOpenError` variants introduced in this cycle will have at least one targeted test, making refactors to the facade safer.  
3. The architecture docs, mini‑spec, and implementation will tell a consistent story about error layering and the diff IR.  
4. Performance expectations around signatures and diff will be clearly documented, with a roadmap to exploit sparsity in a future perf‑focused cycle.  
5. `CellValue`’s convenience helpers will have direct unit tests, making the core IR more robust to future evolution.