# Verification Report: 2025-11-27-datamashup-framing

## Summary

The implementation closely follows the mini-spec: `RawDataMashup` and `open_data_mashup` are implemented with the expected signatures and error modeling, the MS‑QDEFF framing parser (`parse_data_mashup`) enforces the documented invariants, and all tests explicitly called out in the spec exist and are passing. I did not find a functional bug in the new behavior, but there is one **moderate** gap in test coverage around the “multiple DataMashup parts” invariant, plus a few **minor** coverage/structure issues. Because of the moderate finding, I recommend remediation before treating this branch as fully signed off.

## Recommendation

[ ] Proceed to release
[x] Remediation required

---

## Findings

### Finding 1: “Multiple DataMashup parts” invariant is untested

* **Severity**: Moderate

* **Category**: Missing Test / Gap

* **Description**:
  The implementation explicitly treats “more than one DataMashup” as a hard error, in line with the decision record guidance (“reserve hard errors for malformed base64, invalid framing, or multiple DataMashup parts”). However, there are no tests that exercise this behavior.

  Concretely:

  * `extract_datamashup_bytes_from_excel` scans all `customXml/*.xml` parts; if it finds more than one part whose XML contains a `<DataMashup>` element, it returns `Err(ExcelOpenError::DataMashupFramingInvalid)`.
  * `read_datamashup_text` also rejects multiple `<DataMashup>` elements within a single XML file (a second `<DataMashup>` start tag while `in_datamashup` is already true yields `DataMashupFramingInvalid`).

  The tests cover:

  * No DataMashup (`minimal.xlsx` → `Ok(None)`),
  * One well‑formed DataMashup (`m_change_literal_b.xlsx` → `Ok(Some(raw))`),
  * Corrupt base64 (`corrupt_base64.xlsx` → `DataMashupBase64Invalid`),
  * Nonexistent file → `Io(NotFound)`, and
  * Non‑Excel ZIP → `NotExcelOpenXml`.

  None of them cover the “duplicate DataMashup” scenarios, even though the code path is non-trivial and specifically called out in the decision record/spec narrative.

* **Evidence**:

  * Duplicate handling in `extract_datamashup_bytes_from_excel` and `read_datamashup_text`.
  * Integration tests in `core/tests/data_mashup_tests.rs` do not construct or assert on duplicate DataMashup fixtures.
  * Mini-spec and decision record emphasize “exactly one DataMashup part” and hard errors for multiple.

* **Impact**:
  Right now, the code *appears* correct for duplicate cases, but because it lacks tests, any future refactor (e.g., when PBIX support is added or the customXml traversal changes) could easily break the invariant without being caught. In the wild, non‑standard or tool‑generated workbooks *can* end up with multiple DataMashup‑like parts; silently choosing the first one instead of erroring would be a subtle correctness bug for downstream M diffing. Given this is an explicit robustness goal for the framing layer, I consider the missing tests moderate severity.

---

### Finding 2: Some `open_data_mashup` error paths are not directly exercised

* **Severity**: Minor

* **Category**: Missing Test

* **Description**:
  The behavioral contract says `open_data_mashup` should surface container and IO errors via the existing `ExcelOpenError` variants, including non‑ZIP inputs, just like `open_workbook`.

  The current integration tests validate:

  * Nonexistent file → `ExcelOpenError::Io` with `ErrorKind::NotFound`.
  * ZIP that is *not* an Excel Open XML package (`random_zip.zip` missing `[Content_Types].xml`) → `ExcelOpenError::NotExcelOpenXml`.

  But there is no test that calls `open_data_mashup` on a file that is not a ZIP container at all (e.g. a plain text file) and asserts `ExcelOpenError::NotZipContainer`, even though:

  * `open_data_mashup` uses `open_zip(path)` as its first step,
  * `open_zip` maps `ZipError::InvalidArchive/UnsupportedArchive` to `NotZipContainer`, and
  * `open_workbook` already has tests that verify this behavior for the workbook API.

  Likewise, the spec’s top-level behavioral section explicitly calls out the `bytes.len() < 4 + 4*4` case for `parse_data_mashup` (it must return `DataMashupFramingInvalid` and never panic), but there is no *direct* unit test for the “shorter than header” case. The fuzz test (`fuzz_style_never_panics`) does cover small buffers for the no‑panic guarantee, but it doesn’t assert on the specific error variant.

* **Evidence**:

  * `open_data_mashup` → `open_zip` path and `NotZipContainer` mapping.
  * `core/tests/data_mashup_tests.rs` has no non‑ZIP test; `core/tests/excel_open_xml_tests.rs` tests non‑ZIP only for `open_workbook`.
  * Fuzz test exists but lacks explicit assertion of `DataMashupFramingInvalid` for undersized buffers.

* **Impact**:
  These error paths are already exercised indirectly via `open_workbook`, and the implementation is straightforward (delegating to shared helpers), so the practical risk is low. The main hazard is future refactors to `open_data_mashup` or `open_zip` that accidentally diverge from `open_workbook` semantics; direct tests would lock in the intended behavior. I treat this as a quality/coverage improvement rather than a release blocker.

---

### Finding 3: UTF‑16 DataMashup XML decoding has limited explicit coverage

* **Severity**: Minor

* **Category**: Missing Test

* **Description**:
  The host extraction path needs to handle DataMashup XML stored as UTF‑16 LE/BE with BOM, which is the normal Excel encoding for `customXml` items. The implementation includes explicit BOM detection and a `decode_utf16_xml` helper that converts to UTF‑8 before feeding the XML to `quick_xml`.

  The valid DataMashup integration test (`m_change_literal_b.xlsx`) almost certainly goes through this code path, but there is no small, focused unit test that:

  * Builds a tiny UTF‑16 DataMashup XML buffer with a known base64 payload,
  * Calls `read_datamashup_text`, and
  * Verifies we get back exactly the expected base64 string (and that the UTF‑16 error handling works as expected).

  As a result, regressions to `decode_utf16_xml` (for example, mishandling of BOM, odd-length buffers, or endianness) would only be caught indirectly and only if the real fixtures happen to cover both LE and BE cases.

* **Evidence**:

  * BOM detection and UTF‑16 decoding logic in `read_datamashup_text` and `decode_utf16_xml`.
  * No tests directly target these helpers; test coverage is via the higher-level integration test.

* **Impact**:
  Today’s fixtures very likely hit the common UTF‑16‑LE case, so this is not an immediate correctness issue. However, a minimal unit test would make the encoding behavior much more robust against subtle refactors or changes to the XML parsing stack. I classify this as a minor, “nice-to-have” improvement.

---

### Finding 4: DataMashup framing mostly aligns with spec; no functional deviations found

(Not a problem, but worth explicitly stating.)

* `RawDataMashup` has exactly the fields and types specified.

* `open_data_mashup` has the expected signature, is re-exported from `lib.rs`, and is gated behind the `excel-open-xml` feature alongside `open_workbook`, satisfying the WASM/feature‑gating constraint.

* `parse_data_mashup` enforces:

  * Minimum size (`bytes.len() < 4 + 4*4` → `DataMashupFramingInvalid`),
  * Version check (`version != 0` → `DataMashupUnsupportedVersion { version }`),
  * Checked length arithmetic via `read_length`/`take_segment`, and
  * The final `offset == bytes.len()` invariant (trailing bytes → `DataMashupFramingInvalid`).

* Unit tests in `excel_open_xml::tests` cover all explicit framing cases from the mini-spec (zero-length stream, basic non-zero slices, unsupported version, truncated stream, trailing bytes) plus the fuzz-style no‑panic test.

Given this, I don’t see a functional spec deviation; the findings above are about hardening and coverage.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed
* [x] All specified tests created (unit + integration + fuzz-style)
* [x] Behavioral contract satisfied (open_data_mashup / parse_data_mashup semantics and error modeling)
* [x] No undocumented deviations from spec (new behavior is additive; `open_workbook` unchanged)
* [x] Error handling adequate (no `unwrap`/`expect` on untrusted paths; all fallible operations use `Result`)
* [x] No obvious performance regressions (one extra ZIP scan over `customXml/`, single Vec for DataMashup bytes, consistent with spec constraints)

Because of the moderate missing-test finding, I recommend remediation before treating this branch as fully signed off for release.

---

# Remediation Plan: 2025-11-27-datamashup-framing

## Overview

The implementation appears functionally correct and aligned with the mini-spec, but test coverage is missing for one important invariant (duplicate DataMashup parts/elements) and a couple of secondary error/encoding paths. The goal of this remediation is to:

* Lock in the duplicate‑DataMashup error semantics with explicit tests.
* Add light tests for the remaining error/encoding paths where behavior is currently inferred from code rather than asserted.

No production code changes should be necessary unless these new tests reveal a real bug.

---

## Fixes Required

### Fix 1: Test the “duplicate DataMashup” invariants

* **Addresses Finding**: Finding 1 (Moderate – Duplicate DataMashup behavior untested)

* **Changes**:

  1. **Fixtures**
     *Add one new generated Excel fixture that violates the “exactly one DataMashup” rule.*

     Suggested approach:

     * Extend the existing Python fixture generator infrastructure under `fixtures/` (e.g., the `MashupBaseGenerator` / `MashupInjectGenerator` family) to create a workbook where there are **two** DataMashup occurrences:

       * Option A (simpler and enough to exercise the code path):
         Duplicate the `customXml` part that contains the `<DataMashup>` element, producing `customXml/item1.xml` and `customXml/item2.xml` that both contain a DataMashup payload.
       * Option B (also exercising the inner XML invariant):
         In a single `customXml/item1.xml`, create a second `<dm:DataMashup>` element after the first, each with valid base64 content.

     * Name the resulting fixture something like `duplicate_datamashup_parts.xlsx` or `duplicate_datamashup_elements.xlsx` and add it to the `fixtures/generated/` manifest.

  2. **Tests**
     Add an integration test in `core/tests/data_mashup_tests.rs`:

     ```rust
     #[test]
     fn duplicate_datamashup_parts_are_rejected() {
         let path = fixture_path("duplicate_datamashup_parts.xlsx");
         let err = open_data_mashup(&path)
             .expect_err("duplicate DataMashup parts should be rejected");
         assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
     }
     ```

     If you implement the “two elements in one XML file” variant as well, a separate test can target that explicitly; both should end up hitting `DataMashupFramingInvalid` via either `read_datamashup_text` or `extract_datamashup_bytes_from_excel`.

* **Tests**:

  * New fixture‑backed integration test(s) in `core/tests/data_mashup_tests.rs` asserting that any workbook with more than one `<DataMashup>` logically present results in `Err(ExcelOpenError::DataMashupFramingInvalid)`.

---

### Fix 2: Directly test remaining `open_data_mashup` and framing error paths

* **Addresses Finding**: Finding 2 (Minor – Some error paths not directly exercised)

* **Changes**:

  1. **Non‑ZIP container for `open_data_mashup`**

     Add a test in `core/tests/data_mashup_tests.rs` that mirrors the existing `open_workbook` container test but calls `open_data_mashup` instead:

     ```rust
     #[test]
     fn non_zip_file_returns_not_zip_error() {
         // Reuse an existing non-zip fixture if available; otherwise add a small .txt file.
         let path = fixture_path("not_a_zip.txt");
         let err = open_data_mashup(&path)
             .expect_err("non-ZIP input should not be treated as Excel");
         assert!(matches!(err, ExcelOpenError::NotZipContainer));
     }
     ```

     If there’s already a “not zip” fixture used by `excel_open_xml_tests.rs`, re-use that to avoid adding new assets.

  2. **Explicit MIN_SIZE framing error**

     Add a focused unit test to the `excel_open_xml::tests` module for the “bytes shorter than header” behavior that the spec calls out:

     ```rust
     #[test]
     fn too_short_stream_is_framing_invalid() {
         // Fewer than 4 (version) + 4*4 (lengths) bytes.
         let bytes = vec![0u8; 8];
         let err = parse_data_mashup(&bytes)
             .expect_err("buffer shorter than header must be invalid");
         assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
     }
     ```

* **Tests**:

  * New integration test in `core/tests/data_mashup_tests.rs` for `NotZipContainer`.
  * New unit test in `core/src/excel_open_xml.rs`’s test module for the MIN_SIZE constraint on `parse_data_mashup`.

---

### Fix 3 (Optional / Nice-to-have): Targeted UTF‑16 decoding test

* **Addresses Finding**: Finding 3 (Minor – UTF‑16 decoding lacks direct tests)

* **Changes**:

  *This fix is recommended but not required to clear the moderate finding.*

  1. Add a small, synthetic UTF‑16 DataMashup XML buffer in the `excel_open_xml::tests` module:

     ```rust
     #[test]
     fn utf16_datamashup_xml_decodes_correctly() {
         // Minimal UTF-16LE XML with a single DataMashup element and short base64 payload "QQ==" ("A").
         let xml_le: Vec<u8> = /* construct with BOM 0xFF 0xFE and UTF-16LE-encoded XML */;
         let text = read_datamashup_text(&xml_le)
             .expect("UTF-16 XML should parse")
             .expect("DataMashup element should be found");
         assert_eq!(text.trim(), "QQ==");
     }
     ```

     This directly exercises:

     * BOM detection,
     * Endianness handling in `decode_utf16_xml`, and
     * The happy path of `read_datamashup_text` for UTF‑16 input.

* **Tests**:

  * One new unit test in `excel_open_xml::tests` that calls `read_datamashup_text` on a handcrafted UTF‑16 XML buffer and asserts the returned base64 string.

---

## Constraints

* Do **not** change the public API signatures or semantics of:

  * `open_workbook`
  * `open_data_mashup`
  * `RawDataMashup`
  * `ExcelOpenError` variants (beyond adding tests).

* Keep all filesystem and ZIP access under the existing `excel-open-xml` feature gate; new tests should rely on the current fixture layout and `common::fixture_path`.

* Prefer to implement remediation purely via new tests and fixtures. Only touch production code if a new test exposes a real bug.

---

## Expected Outcome

After implementing this remediation:

1. Duplicate DataMashup scenarios (multiple parts and/or multiple elements) will be covered by explicit tests that assert `ExcelOpenError::DataMashupFramingInvalid`.
2. `open_data_mashup` will have direct tests for all major container error modes (`Io(NotFound)`, `NotZipContainer`, `NotExcelOpenXml`), aligning it with `open_workbook`’s error behavior.
3. The MS‑QDEFF `MIN_SIZE` invariant will be explicitly locked in by a unit test, and optionally the UTF‑16 decoding path will be validated by a small, deterministic test.
4. No existing tests regress, and no production-code changes should be required unless a previously-hidden bug is found.

At that point, assuming the new tests pass, I would expect to flip the recommendation in the verification report to **“Proceed to release”** with all checklist items and coverage concerns resolved.
