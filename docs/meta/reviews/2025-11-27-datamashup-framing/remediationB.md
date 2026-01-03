[file-tag: docs/meta/reviews/2025-11-27-datamashup-framing/remediation.md]\# Verification Report: 2025-11-27-datamashup-framing

## Summary

This report synthesizes the findings of this verification review with the parallel analysis provided separately. Both analyses confirm that the implementation successfully delivers the core scope of the mini-spec (MS-QDEFF framing, Excel host API, and error modeling) and that remediation is required due to gaps in test coverage.

Crucially, this review identified a **Moderate implementation bug** that was missed in the parallel analysis: the current XML parser incorrectly accepts files containing multiple *sibling* `<DataMashup>` elements by silently processing only the first one, rather than rejecting the file as malformed.

Additionally, this review identified further minor gaps in test coverage (Base64 whitespace handling) and technical debt (XML namespace validation) not captured elsewhere. A consolidated remediation plan addressing all findings is provided below.

## Recommendation

[ ] Proceed to release
[X] Remediation required

## Findings

### A. Shared Findings (Acknowledged from Parallel Analysis)

The following findings were identified in the parallel analysis and are acknowledged here.

1.  **Missing Test Coverage: Multiple DataMashup Parts (Files)** (Severity: Moderate): The invariant against multiple `customXml` files containing DataMashup is implemented but lacks an integration test.
2.  **Missing Test Coverage: UTF-16 Encoded customXml** (Severity: Moderate): Logic exists to handle UTF-16 LE/BE encoded XML parts, but no integration fixtures exercise this path. (Note: We assess this as Moderate due to the prevalence of UTF-16 in real-world Excel files).
3.  **Missing Test Coverage: Specific Error Paths** (Severity: Minor): `open_data_mashup` lacks explicit tests for `NotZipContainer` (non-ZIP input), and `parse_data_mashup` lacks an explicit test for the `MIN_SIZE` (buffer too short) check.

### B. Unique Findings (Detailed Analysis)

#### Finding 4: Implementation Bug: Sibling `<DataMashup>` Elements Silently Accepted

  - **Severity**: Moderate
  - **Category**: Bug
  - **Description**: The function `read_datamashup_text` incorrectly handles multiple *sibling* `<DataMashup>` elements within a single XML part. It processes the first element and returns `Ok(Some(content))` immediately upon seeing the closing tag, ignoring any subsequent sibling elements. While it correctly rejects *nested* elements (L228), it fails the robustness requirement to reject files with multiple sibling definitions. The parallel analysis incorrectly stated that this scenario was handled correctly.
  - **Evidence**: In `core/src/excel_open_xml.rs`, L246:
    ```rust
    Ok(Event::End(e)) if in_datamashup && is_datamashup_element(e.name().as_ref()) => {
        return Ok(Some(content)); // Exits prematurely
    }
    ```
  - **Impact**: The engine may silently accept malformed workbooks by processing only the first definition, leading to incorrect or incomplete analysis. This violates the behavioral contract for robust error handling.

#### Finding 5: Missing Test Coverage: Base64 Whitespace Stripping

  - **Severity**: Minor
  - **Category**: Missing Test
  - **Description**: The implementation includes logic to strip whitespace before decoding the Base64 payload (`decode_datamashup_base64`, L261) to handle XML indentation or newlines. This behavior is not covered by tests.
  - **Evidence**: The logic exists, but no test fixture includes whitespace in the Base64 payload.
  - **Impact**: This robustness behavior is not locked in and could regress if the decoding logic is refactored.

#### Finding 6: Technical Debt: Fragile XML Element Name Matching (No Namespace Check)

  - **Severity**: Minor
  - **Category**: Technical Debt
  - **Description**: The function `is_datamashup_element` identifies the element using `name.ends_with(b"DataMashup")` (L267). This ignores the required XML namespace URI (`http://schemas.microsoft.com/DataMashup`).
  - **Impact**: This could lead to misidentification if `customXml` contains the local name "DataMashup" in a different namespace. Deferred for future hardening.

## Checklist Verification

  - [X] All scope items from mini-spec addressed
  - [X] All specified tests created
  - [ ] Behavioral contract satisfied (Finding 4 indicates a bug in the robustness behavior contract)
  - [X] No undocumented deviations from spec
  - [X] Error handling adequate
  - [X] No obvious performance regressions

# Consolidated Remediation Plan: 2025-11-27-datamashup-framing

## Overview

This plan consolidates remediation actions for all findings identified in both verification analyses. It addresses the implementation bug (Finding 4) and the comprehensive set of test coverage gaps (Findings 1, 2, 3, 5). This involves significant enhancements to the Python fixture generators, adding corresponding Rust tests, and correcting the XML parsing logic in the Rust core.

## Prerequisite: Python Generator Enhancements

To support the required fixes, the Python fixture generators must be enhanced. This requires substantial effort to correctly handle ZIP manipulation, XML encoding (BOMs), and structure modification.

1.  **`fixtures/src/generate.py`**: Update the generator registry as needed.
2.  **`fixtures/src/generators/mashup.py`**:
      * Implement a new `MashupEncodeGenerator` (or extend `MashupBaseGenerator`):
          * Handle `encoding` (utf-16-le, utf-16-be) to re-encode `customXml/item*.xml` with the correct BOM (Fix 2).
          * Handle `whitespace` argument to insert spaces/newlines into the Base64 payload (Fix 5).
3.  **`fixtures/src/generators/corrupt.py` or `mashup.py`**: Extend a generator (e.g., `MashupCorruptGenerator`):
      * `mode: "duplicate_part"` (Fix 1): Inject a second `customXml/item*.xml` file and update relationships/Content\_Types.
      * `mode: "duplicate_element"` (Fix 4): Duplicate the `<DataMashup>` element within the existing XML file.

## Fixes Required

### Fix 1: Add Test for Multiple DataMashup Parts (Files)

  - **Addresses Finding**: 1
  - **Changes**:
    1.  **`fixtures/manifest.yaml`**: Add a new scenario.
        ```yaml
          # --- Milestone 2.X: Multiple DataMashup parts (Invalid) ---
          - id: "container_multiple_datamashup"
            generator: "mashup_corrupt"
            args: { mode: "duplicate_part", base_file: "templates/base_query.xlsx" }
            output: "multiple_datamashup.xlsx"
        ```
  - **Tests**:
      * **`core/tests/data_mashup_tests.rs`**: Add a new integration test asserting `Err(ExcelOpenError::DataMashupFramingInvalid)`.

### Fix 2: Add Tests for UTF-16 Encoded customXml

  - **Addresses Finding**: 2
  - **Changes**:
    1.  **`fixtures/manifest.yaml`**: Add scenarios for `utf-16-le` and `utf-16-be` using `MashupEncodeGenerator`.
        ```yaml
          # --- Milestone 2.X: UTF-16 Robustness ---
          - id: "mashup_utf16_le"
          # ... (args for utf-16-le)
            output: "mashup_utf16_le.xlsx"
            
          - id: "mashup_utf16_be"
          # ... (args for utf-16-be)
            output: "mashup_utf16_be.xlsx"
        ```
  - **Tests**:
      * **`core/tests/data_mashup_tests.rs`**: Add integration tests verifying successful parsing of both fixtures.

### Fix 3: Add Tests for Specific Error Paths (`NotZipContainer`, `MIN_SIZE`)

  - **Addresses Finding**: 3
  - **Changes**:
    1.  **`core/tests/data_mashup_tests.rs`**: Add a test for non-ZIP input (using a temporary file).
        ```rust
          #[test]
          fn non_zip_file_returns_not_zip_error() {
              let path = std::env::temp_dir().join("excel_diff_dm_not_zip.txt");
              std::fs::write(&path, "this is not a zip container").expect("write temp file");
              let err = open_data_mashup(&path).expect_err("non-ZIP input should fail");
              assert!(matches!(err, ExcelOpenError::NotZipContainer));
              let _ = std::fs::remove_file(&path);
          }
        ```
    2.  **`core/src/excel_open_xml.rs` (tests module)**: Add a unit test for `parse_data_mashup`.
        ```rust
        #[test]
        fn too_short_stream_is_framing_invalid() {
            // Fewer than 20 bytes (4 version + 4*4 lengths).
            let bytes = vec![0u8; 19];
            let err = parse_data_mashup(&bytes)
                .expect_err("buffer shorter than header must be invalid");
            assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
        }
        ```

### Fix 4: Reject Multiple Sibling DataMashup Elements in Single XML File

  - **Addresses Finding**: 4 (Implementation Bug)
  - **Changes**:
    1.  **`core/src/excel_open_xml.rs`**: Modify `read_datamashup_text`.
          * Remove the early return at L246.
          * Introduce state to track the found content (e.g., `found_content: Option<String>`).
          * On `End` event (L246): set `in_datamashup = false`, store content in `found_content`.
          * On `Start` event (L226): If `in_datamashup` is true (nested) OR if `found_content.is_some()` (sibling), return `Err(ExcelOpenError::DataMashupFramingInvalid)`.
          * On `Eof` event: Return `Ok(found_content)`.
    2.  **`fixtures/manifest.yaml`**: Add scenario `mashup_multiple_elements` using `mashup_corrupt` with `mode: "duplicate_element"`.
  - **Tests**:
      * **`core/tests/data_mashup_tests.rs`**: Add integration test asserting `Err(ExcelOpenError::DataMashupFramingInvalid)`.

### Fix 5: Add Test for Base64 Whitespace Stripping

  - **Addresses Finding**: 5
  - **Changes**:
    1.  **`fixtures/manifest.yaml`**: Add scenario `mashup_base64_whitespace` using `MashupEncodeGenerator` with `whitespace: true`.
  - **Tests**:
      * **`core/tests/data_mashup_tests.rs`**: Add an integration test verifying successful parsing.

## Constraints

  * The Python fixture generator modifications are complex and critical for implementing these fixes.
  * The Rust implementation change (Fix 4) must be carefully implemented to handle the XML event state transitions correctly without breaking existing valid cases.

## Expected Outcome

After remediation, the implementation bug regarding sibling XML elements will be fixed, and all identified robustness features and error paths will be covered by explicit regression tests, ensuring the stability and correctness of the DataMashup framing layer.