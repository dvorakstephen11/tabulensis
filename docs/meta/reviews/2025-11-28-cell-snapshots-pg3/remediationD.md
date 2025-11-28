````markdown
# Verification Report: 2025-11-28-cell-snapshots-pg3

## Summary

The PG3 cell snapshot milestone and the VAL-001 value-semantics coverage are fully implemented and well tested: `CellSnapshot` exists with the expected structure and equality semantics, the Excel fixture `pg3_value_and_formula_cells.xlsx` is exercised via integration tests, and new unit tests pin down shared string, inline string, boolean, and error-cell parsing behavior.  Prior remediation findings (missing PG3 integration tests, `CellAddress` JSON representation, invalid shared-string indices, and JSON output structure/dependencies/empty-diff coverage) have been addressed.  The only remaining issues are: (1) architectural/doc drift around using `serde_json` as a runtime dependency and exposing JSON diff helpers from the core crate, and (2) a missing positive (non-empty) integration test for the JSON diff path. Error handling and WASM compatibility look solid based on the code and the test commands run. 

## Recommendation

[ ] Proceed to release  
[X] Remediation required

The required remediation is narrow and focused on the JSON diff output path and documentation alignment, not on the PG3 snapshot work itself.

## Findings

### Finding 1: PG3 scope implemented and tests complete (no action required)

- **Severity**: N/A (positive confirmation)
- **Category**: Scope Verification
- **Description**: All items in the PG3 mini-spec and the referenced testing milestones (PG3.1–PG3.4 plus VAL-001) are implemented:

  - `CellSnapshot` is defined as `{ addr: CellAddress, value: Option<CellValue>, formula: Option<String> }`, with `Clone`, `Debug`, `Serialize`, and `Deserialize` derives and a `from_cell` constructor. :contentReference[oaicite:3]{index=3}  
  - Custom `PartialEq`/`Eq` for `CellSnapshot` compares only `(value, formula)`, ignoring `addr`, as specified.   
  - `CellSnapshot` is re-exported from `core::lib` alongside the existing IR types. :contentReference[oaicite:5]{index=5}  
  - PG3.1 and PG3.3 unit tests exist in `workbook::tests` (`snapshot_from_number_cell`, `snapshot_from_text_cell`, `snapshot_from_bool_cell`, `snapshot_from_empty_cell`, and the four equality tests).   
  - PG3.2 and PG3.4 integration tests exist in `core/tests/pg3_snapshot_tests.rs`, using `pg3_value_and_formula_cells.xlsx` and validating values, formulas, address correctness, JSON round-tripping, and equality semantics (including an explicit addr equality assertion and a tampering test). :contentReference[oaicite:7]{index=7}  
  - VAL-001 value semantics tests for shared strings, inline strings, booleans, and error cells are implemented in `excel_open_xml_tests.rs`.   

- **Evidence**: PG3 mini-spec and decision record; workbook.rs, lib.rs, excel_open_xml.rs and their tests; pg3_snapshot_tests.rs; excel_open_xml_tests.rs; test run output. 
- **Impact**: This confirms that the core goal of this cycle (cell snapshots and value semantics) is met and stable enough to support downstream diff work (PG4/PG5).

---

### Finding 2: `serde_json` is a runtime dependency and JSON diff helpers are public, diverging from PG3 mini-spec text

- **Severity**: Moderate  
- **Category**: Spec Deviation / Architectural Drift  
- **Description**:

  The PG3 mini-spec explicitly suggested adding `serde` as a dependency and `serde_json` only as a dev-dependency, with serialization used primarily for snapshot tests. 

  The current `core/Cargo.toml` and lib surface differ:

  - `serde_json = "1.0"` is listed under `[dependencies]`, not `[dev-dependencies]`. :contentReference[oaicite:11]{index=11}  
  - The crate exposes a new output module and public JSON helpers:

    ```rust
    pub mod output;
    pub use output::json::{CellDiff, serialize_cell_diffs};
    #[cfg(feature = "excel-open-xml")]
    pub use output::json::{diff_workbooks, diff_workbooks_to_json};
    ``` :contentReference[oaicite:12]{index=12}  

  This aligns with the separate JSON-output remediation plan (feature/implement-json-output), which intentionally moved JSON formatting into the core crate and added explicit `serde`/`serde_json` dependencies.  However, it does not match the original PG3 wording, which assumed `serde_json` would stay test-only and the core library would remain format-agnostic for now.

- **Evidence**:
  - PG3 decision record notes recommending `serde_json` “only in tests for PG3.4”. :contentReference[oaicite:14]{index=14}  
  - `core/Cargo.toml` shows `serde_json` as a regular dependency. :contentReference[oaicite:15]{index=15}  
  - `core/src/lib.rs` re-exports the JSON output module and symbols as part of the public API. :contentReference[oaicite:16]{index=16}  
  - Combined remediation history and cycle summary document the intentional addition of JSON output helpers and dependencies, but the PG3 spec text has not been updated to reflect this broader scope.   

- **Impact**:

  - The library is now *committed* to having `serde_json` as a runtime dependency and to exposing a JSON output surface. This is technically fine (WASM build passes), but it contradicts the PG3 expectation that JSON would be test-only at this stage.   
  - Without updating the spec/architecture docs, future work may incorrectly assume the core is still format-agnostic, leading to further architectural drift or duplicated JSON logic elsewhere.
  - This does not block PG3 functionality, but it should be reconciled so the decision record and specification reflect the true public surface.

---

### Finding 3: JSON diff path lacks a positive (non-empty) integration test

- **Severity**: Moderate  
- **Category**: Missing Test  
- **Description**:

  The new JSON diff helpers are implemented as:

  - `CellDiff { coords, value_file1, value_file2 }` with serde-backed JSON serialization.   
  - `compute_cell_diffs(a, b)` that walks the union of sheet names and cell coordinates, comparing rendered cell values and producing `CellDiff`s.   
  - `serialize_cell_diffs(&[CellDiff]) -> serde_json::Result<String>` and `diff_workbooks_to_json(path_a, path_b)` that combines diffing and JSON serialization.   

  The test coverage for this path is currently:

  - `output_tests.rs::test_json_format` – unit test that builds a small `Vec<CellDiff>` by hand, serializes it, and asserts that the JSON is an array and that the first element has the expected `coords`, `value_file1`, and `value_file2` keys and values. :contentReference[oaicite:22]{index=22}  
  - `output_tests.rs::test_json_empty_diff` – integration test that calls `diff_workbooks_to_json` on `pg1_basic_two_sheets.xlsx` vs itself and asserts that the resulting JSON is an empty array. :contentReference[oaicite:23]{index=23}  

  There is **no test** that exercises `diff_workbooks_to_json` (or `diff_workbooks`) on a pair of workbooks that actually differ, and then checks that:

  - At least one `CellDiff` is produced.
  - The `coords` and `value_file1`/`value_file2` fields match the expected changed cells.

- **Evidence**: Review of `core/tests/output_tests.rs`; presence of only the two tests described above, both of which avoid the “non-empty diff” case for the `diff_workbooks` pipeline.   

- **Impact**:

  - A bug in `compute_cell_diffs` (e.g., off-by-one in dimensions, incorrect sheet handling, or incorrect value rendering) would not be caught by the current test suite as long as the empty-diff and manual-`CellDiff` tests continue to pass.
  - Since the JSON output is exposed as a public API and will likely be consumed by CLI/UX layers, this represents a real risk of producing incorrect diff output for real-world comparisons, even though no failing tests are visible today.
  - Given that adding one positive-case fixture-based test is straightforward, this is best treated as remediation for this cycle rather than deferred.

---

### Finding 4: JSON serialization errors are surfaced as `XmlParseError`

- **Severity**: Minor  
- **Category**: Gap / Spec Deviation  
- **Description**:

  The JSON helper `diff_workbooks_to_json` propagates JSON serialization failures using `ExcelOpenError::XmlParseError`:

  ```rust
  pub fn diff_workbooks_to_json(path_a: &Path, path_b: &Path) -> Result<String, ExcelOpenError> {
      let diffs = diff_workbooks(path_a, path_b)?;
      serialize_cell_diffs(&diffs).map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))
  }
````

While `ExcelOpenError::XmlParseError` already exists and is reused in many parsing contexts, using it for JSON serialization errors is semantically misleading.

* **Evidence**: Implementation of `diff_workbooks_to_json` and the definition of `ExcelOpenError` in `excel_open_xml.rs`.

* **Impact**:

  * In the unlikely event of a JSON serialization failure (e.g., resource exhaustion or internal serde bug), consumers would see an error labeled as XML-related, which complicates debugging and error handling.
  * This does not affect correctness for normal usage; serde_json serialization of a simple Vec of structs is extremely reliable. Hence, this is a minor issue and can be addressed either by documenting the generic use of `XmlParseError` for all serialization issues or by adding a more appropriate variant in a future error model refactor.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed
* [x] All specified tests created (PG3.1–PG3.4 and VAL-001)
* [x] Behavioral contract satisfied for PG3 snapshots and value semantics
* [x] No undocumented deviations from spec (all deviations — JSON output surface and dependency changes — are documented in prior remediation reports and the cycle summary)
* [x] Error handling adequate (invalid shared-string indices, DataMashup framing, UTF-16 handling, and IO/container errors are all explicitly tested; JSON serialization errors are mapped but not ideal in naming)
* [x] No obvious performance regressions (new code walks grids in straightforward nested loops; WASM build still passes; no heavy allocations or pathological behavior added for PG3)

````

---

```markdown
# Remediation Plan: 2025-11-28-cell-snapshots-pg3

## Overview

The PG3 snapshot milestone itself is complete and well-tested. Remaining work is focused on (1) tightening test coverage for the JSON diff helpers introduced via prior remediation for the JSON output feature, and (2) reconciling the public surface and dependency model with the original PG3 documentation. No changes are required to `CellSnapshot` or the core Excel parsing/value semantics.

## Fixes Required

### Fix 1: Add a positive-case JSON diff integration test

- **Addresses Finding**: Finding 3 – JSON diff path lacks a positive (non-empty) integration test.

- **Changes**:

  1. **Fixture choice**  
     Pick or introduce a small pair of Excel files that differ in a known, simple way (e.g., a single-cell literal change):

     - Option A (preferred if you want to reuse PG testing concepts):  
       Add a fixture pair similar to the G2 “single cell literal change” scenario in the testing plan — for example, `single_cell_value_a.xlsx` and `single_cell_value_b.xlsx`, where only `C3` differs.   

     - Option B (minimal fixture churn):  
       Start from the existing `pg1_basic_two_sheets.xlsx` and create a “B” version with one cell edited (e.g., change `Sheet1!A1` from `"R1C1"` to `"X"`).   

  2. **New test in `core/tests/output_tests.rs`**  

     Add a test that calls `diff_workbooks_to_json` on the chosen `{a,b}` pair and asserts that:

     - The top-level JSON is an array.
     - The array is non-empty and its length matches the expected number of changed cells (for a single-cell change, `len() == 1`).
     - At least one `CellDiff` entry has:

       - `coords` equal to the expected address (e.g., `"C3"`).
       - `value_file1` and `value_file2` strings matching the before/after values.

     Sketch:

     ```rust
     #[test]
     fn test_json_non_empty_diff() {
         let a = fixture_path("single_cell_value_a.xlsx");
         let b = fixture_path("single_cell_value_b.xlsx");

         let json =
             diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
         let value: serde_json::Value =
             serde_json::from_str(&json).expect("json should parse");

         let arr = value
             .as_array()
             .expect("top-level should be an array of cell diffs");
         assert!(!arr.is_empty(), "changed files should produce diffs");

         let first = &arr[0];
         assert_eq!(first["coords"], serde_json::Value::String("C3".into()));
         assert_eq!(first["value_file1"], serde_json::Value::String("1".into()));
         assert_eq!(first["value_file2"], serde_json::Value::String("2".into()));
     }
     ```

     Adapt the addresses and values to match your chosen fixture.

- **Tests**:

  - Ensure the new test passes alongside existing JSON tests:

    - `cargo test --test output_tests`  
    - `cargo test` for the full suite. :contentReference[oaicite:35]{index=35}  

  - This will validate the full `diff_workbooks -> compute_cell_diffs -> serialize_cell_diffs` pipeline on a non-empty diff.

---

### Fix 2: Align documentation and decisions with the JSON output surface and `serde_json` dependency

- **Addresses Finding**: Finding 2 – runtime `serde_json` dependency and JSON helpers exposed from core diverge from PG3 mini-spec text.

- **Changes**:

  1. **Clarify design intent in docs**  

     Update the relevant docs to reflect the current, intentional design:

     - In `decision_2025-11-28-cell-snapshots-pg3.yaml` and/or a follow-up decision record, add a short note that:

       - The crate now intentionally exposes JSON diff helpers (`CellDiff`, `serialize_cell_diffs`, `diff_workbooks`, `diff_workbooks_to_json`) as part of the public API.   
       - `serde_json` is a first-class runtime dependency to support that JSON output surface.

     - In `excel_diff_specification.md`, add a brief section under the IR/output or CLI/API contract area summarizing:

       - The JSON cell-diff schema (object with `coords`, `value_file1`, `value_file2`) as now implemented.   
       - That this JSON is part of the public contract for downstream consumers (CLI, GUI, etc.), even though deeper diff structures will evolve in later milestones.

     - In the PG3 mini-spec (`spec_2025-11-28-cell-snapshots-pg3.md`), either:

       - Relax the earlier guidance about “serde_json only in tests” to acknowledge that subsequent JSON-output work extended the scope, or
       - Add a note explicitly deferring to the newer JSON-output decision record for library-level JSON behavior.   

  2. **Keep code as-is unless you want to re-isolate JSON**

     Given that:

     - `serde_json` builds fine on `wasm32-unknown-unknown` in the current configuration, and  
     - The JSON helpers are already used in tests and are likely to be useful for CLI/UX,   

     there is no immediate need to revert `serde_json` back to a dev-dependency.

     If you later decide to re-isolate JSON to a higher-level crate, that should be done under a separate decision record and mini-spec, not as part of this remediation.

- **Tests**:

  - No new Rust tests required for doc alignment; just rerun the standard test commands to ensure no incidental changes break anything.

---

### Fix 3: Clarify or refine error mapping for JSON serialization failures

- **Addresses Finding**: Finding 4 – JSON serialization errors surfaced as `XmlParseError`.

- **Changes** (choose one of these low-cost options):

  **Option A – Documentation-only clarification (minimal code change)**

  - In `excel_diff_specification.md` and/or the error-model documentation, note that `ExcelOpenError::XmlParseError` is currently used as a general “serialization/parsing of structured text” error, covering both XML and JSON operations.   
  - Make it explicit that a future refinement may introduce a more granular error enum (e.g., `JsonError`) once the public API surface stabilizes.

  **Option B – Introduce a `JsonError` variant (small code change)**

  - Extend `ExcelOpenError` in `excel_open_xml.rs` with a new `JsonError(String)` variant.

  - Update `diff_workbooks_to_json` to map serialization failures to this new variant:

    ```rust
    pub fn diff_workbooks_to_json(path_a: &Path, path_b: &Path) -> Result<String, ExcelOpenError> {
        let diffs = diff_workbooks(path_a, path_b)?;
        serialize_cell_diffs(&diffs).map_err(|e| ExcelOpenError::JsonError(e.to_string()))
    }
    ```

  - Optionally add a small unit test that constructs a deliberately invalid `CellDiff` (e.g., via a custom type) and verifies that a failure is surfaced as `JsonError`. This may be contrived, so Option A is perfectly acceptable if you do not want to complicate the error model yet.

- **Tests**:

  - For Option A: no new tests required; existing suite suffices.  
  - For Option B: add a focused test exercising the new variant (even if via a synthetic error) and then rerun the full suite.

---

## Constraints

- Do **not** change `CellSnapshot`’s Rust shape or equality semantics; later diff milestones (PG4/PG5) assume the current contract (addr + optional value + optional formula; equality = value/formula only).   
- Keep the existing `open_workbook`/`open_data_mashup` signatures and error variants intact; this remediation is about JSON output and documentation alignment, not container or XML parsing behavior.   
- Maintain WASM compatibility: any changes must continue to pass `cargo check --target wasm32-unknown-unknown --no-default-features` for the `core` crate.   

## Expected Outcome

After remediation:

- PG3 snapshot functionality and value semantics remain unchanged and fully tested.
- The JSON diff pipeline is covered by both empty- and non-empty-diff tests, increasing confidence that `diff_workbooks_to_json` produces correct `CellDiff` objects for real comparisons.   
- The documentation and decision records clearly reflect that:

  - JSON diff helpers are part of the public API, and  
  - `serde_json` is an intentional runtime dependency.

- The error model around JSON serialization is either explicitly documented as reusing `XmlParseError` or refined to use a dedicated variant, reducing potential confusion for downstream consumers.

At that point, the branch will be ready to proceed without further PG3-related remediation, and future work (PG4/PG5 and richer diff output) can build on a clearly-specified, well-tested foundation.
````
