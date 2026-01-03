````markdown
# Remediation Plan: 2025-11-28-cell-snapshots-pg3

## Overview

This plan closes the two remaining minor test gaps identified in the latest verification report:

1. PG3 formula snapshot integration tests do not assert `addr` for the `Types!B1:B3` formula cells.
2. JSON diff tests exercise numeric changes only and do not touch boolean (and implicitly error-text) cases.

The fixes are small, localized changes to existing tests and fixtures. No production code changes are required.

## Fixes Required

### Fix 1: Assert addresses for formula snapshots B1–B3

- **Addresses Finding**: “PG3.2 tests don’t assert addresses for B1–B3 formula snapshots”
- **Changes**:
  - File: `core/tests/pg3_snapshot_tests.rs` :contentReference[oaicite:0]{index=0}  
    - In `pg3_value_and_formula_cells_snapshot_from_excel`, after constructing each snapshot for `B1`, `B2`, and `B3`, add explicit address assertions:
      - `assert_eq!(b1.addr.to_string(), "B1");`
      - `assert_eq!(b2.addr.to_string(), "B2");`
      - `assert_eq!(b3.addr.to_string(), "B3");`
    - This mirrors the existing A1–A4 checks and fully matches the PG3.2 expectations in the mini-spec that snapshots from the fixture preserve both value/formula and A1 addresses for all `Types` cells under test. :contentReference[oaicite:1]{index=1}  
- **Tests**:
  - Re-run:
    - `cargo test --test pg3_snapshot_tests`
    - `cargo test` (full suite, to keep the PG3 milestone and VAL-001 coverage green) :contentReference[oaicite:2]{index=2}  

### Fix 2: Add JSON diff coverage for booleans (and error-text shape)

- **Addresses Finding**: “JSON diff tests are value-shape–light (numeric-only)”
- **Changes**:
  1. **Strengthen JSON shape unit test with non-numeric values**
     - File: `core/tests/output_tests.rs` :contentReference[oaicite:3]{index=3}  
     - In `test_json_format`, extend the manually-constructed `diffs: Vec<CellDiff>` to include at least one additional entry using a boolean-like display and one using an error-like display:
       - Example:
         ```rust
         let diffs = vec![
             CellDiff {
                 coords: "A1".into(),
                 value_file1: Some("100".into()),
                 value_file2: Some("200".into()),
             },
             CellDiff {
                 coords: "B2".into(),
                 value_file1: Some("true".into()),
                 value_file2: Some("false".into()),
             },
             CellDiff {
                 coords: "C3".into(),
                 value_file1: Some("#DIV/0!".into()),
                 value_file2: None,
             },
         ];
         ```
       - Extend the JSON assertions to check these extra elements:
         - `coords == "B2"`, `value_file1 == "true"`, `value_file2 == "false"`.
         - `coords == "C3"`, `value_file1 == "#DIV/0!"`, `value_file2 == Value::Null`.
       - This confirms that the public JSON schema (`coords`, `value_file1`, `value_file2`) correctly handles the string shapes produced for booleans (`true`/`false`) and error text (e.g. `"#DIV/0!"`).   

  2. **Add an integration test that drives boolean values through the diff pipeline**
     - Files:
       - `fixtures/src/generators/grid.py`
       - `fixtures/manifest.yaml`
       - `core/tests/output_tests.rs`   
     - Add a new manifest scenario for a boolean-only single-cell diff, reusing the existing grid generator pattern:
       - Example manifest entry:
         ```yaml
         - id: "json_diff_single_bool"
           generator: "single_cell_diff_bool"
           args:
             rows: 3
             cols: 3
             sheet: "Sheet1"
             target_cell: "C3"
             value_a: true
             value_b: false
           output:
             - "json_diff_bool_a.xlsx"
             - "json_diff_bool_b.xlsx"
         ```
       - Implement `SingleCellBoolDiffGenerator` (or extend `SingleCellDiffGenerator`) so that it writes actual boolean cells, not strings, for `target_cell`:
         - Use `ws[target_cell] = True` / `False` with `openpyxl` so that `convert_value` produces `CellValue::Bool` and `render_cell_value` in `output::json` sees the `Bool` branch. :contentReference[oaicite:6]{index=6}  
     - Add a new test in `core/tests/output_tests.rs`:
       ```rust
       #[test]
       fn test_json_non_empty_diff_bool() {
           let a = fixture_path("json_diff_bool_a.xlsx");
           let b = fixture_path("json_diff_bool_b.xlsx");

           let json =
               diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
           let value: serde_json::Value =
               serde_json::from_str(&json).expect("json should parse");

           let arr = value
               .as_array()
               .expect("top-level should be an array of cell diffs");
           assert_eq!(arr.len(), 1, "expected a single cell difference");

           let first = &arr[0];
           assert_eq!(first["coords"], serde_json::Value::String("C3".into()));
           assert_eq!(first["value_file1"], serde_json::Value::String("true".into()));
           assert_eq!(first["value_file2"], serde_json::Value::String("false".into()));
       }
       ```
       - This drives the `CellValue::Bool` → `render_cell_value` → `CellDiff` → JSON path end-to-end, complementing the IR-level value semantics tests already in `excel_open_xml.rs` (VAL-001).   

- **Tests**:
  - Regenerate fixtures (Python) so the new boolean diff workbooks exist:
    - `cd fixtures && python -m src.generate --force` (or your usual fixture-generation command).   
  - Re-run:
    - `cargo test --test output_tests`
    - `cargo test` (full suite)
    - `cargo check --target wasm32-unknown-unknown --no-default-features` (to keep the JSON helpers and added tests compatible with the WASM guardrail in the testing plan).   

## Constraints

- Do **not** change the `CellSnapshot`, `CellDiff`, or `diff_workbooks_to_json` public signatures; this remediation is test-only and should not alter the API surface committed by the PG3 mini-spec and JSON output spec.   
- Keep new fixtures small (3×3 grids) to avoid impacting CI runtime; they should mirror the structure of the existing `json_diff_single_cell_*` fixtures already in the manifest.   

## Expected Outcome

After this remediation:

- PG3.2 integration tests will verify that snapshots from Excel preserve A1-style addresses for both value and formula cells (A1–A4 and B1–B3), fully matching the mini-spec. :contentReference[oaicite:12]{index=12}  
- The JSON diff path (`diff_workbooks_to_json`) will be covered by:
  - Empty-diff tests,
  - Numeric single-cell diff tests,
  - Boolean single-cell diff tests,
  while the JSON shape unit test will explicitly cover boolean and error-text string values.   
- No production behavior changes are required; only stronger tests are added, increasing confidence in the snapshot and JSON diff behavior for future cycles.
````
