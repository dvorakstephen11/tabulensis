# Verification Report: 2025-11-27-datamashup-framing

## Summary

The implementation closely matches the mini-spec and decision record for this cycle. The new `RawDataMashup` type, `open_data_mashup` API, and DataMashup-related `ExcelOpenError` variants are present and behave as specified. The framing parser (`parse_data_mashup`) enforces the documented invariants, and the host-level extraction from Excel containers correctly handles presence/absence of DataMashup, UTF‑16 encoding, base64 decoding, and duplicate-part/element edge cases. All tests described in the mini-spec were implemented (plus several useful extras), and test results are clean. I found only **minor** issues: slightly over-broad XML element detection, a narrow UTF‑16 detection strategy, a somewhat tautological integration invariant check, and one missing-but-easy test case around content-types for `open_data_mashup`. None of these are release-blocking, but a remediation plan would strengthen the foundation for the next cycle.

## Recommendation

[ ] Proceed to release
[ ] Remediation required
[x] Remediation advised

---

## Findings

### Finding 1: DataMashup element detection is slightly over-broad

* **Severity**: Minor
* **Category**: Bug (robustness)
* **Description**:
  The helper used to detect the `<DataMashup>` element treats any element whose qualified name *ends with* `"DataMashup"` as a match:

  ````rust
  fn is_datamashup_element(name: &[u8]) -> bool {
      name.ends_with(b"DataMashup")
  }
  ``` :contentReference[oaicite:0]{index=0}  

  This is correct for `DataMashup` and `dm:DataMashup`, but it would also match unlikely tag names like `<FooDataMashup>` if they appeared in a `customXml` part.
  ````
* **Evidence**: `core/src/excel_open_xml.rs`, `is_datamashup_element`. 
* **Impact**:
  In realistic Excel-generated DataMashup XML this shouldn’t misfire, but in adversarial or third‑party workbooks with custom XML that happens to contain other elements ending with `"DataMashup"`, `read_datamashup_text` could incorrectly treat them as the mashup payload and either:

  * Try to base64-decode unrelated text, or
  * Treat multiple occurrences as a DataMashup framing error.

  The result would be a spurious `DataMashupFramingInvalid` or `DataMashupBase64Invalid` error for a workbook that technically has no DataMashup. Given the low likelihood and the fact that this only affects unusual custom XML, this is a minor robustness concern.

---

### Finding 2: UTF‑16 handling relies solely on BOM

* **Severity**: Minor
* **Category**: Gap
* **Description**:
  The XML decoding path only treats a DataMashup custom XML part as UTF‑16 if the byte stream begins with a UTF‑16 BOM:

  ````rust
  let utf8_xml = if xml.starts_with(&[0xFF, 0xFE]) {
      Some(decode_utf16_xml(xml, true)?)
  } else if xml.starts_with(&[0xFE, 0xFF]) {
      Some(decode_utf16_xml(xml, false)?)
  } else {
      None
  };
  ``` :contentReference[oaicite:2]{index=2}  

  There is no fallback that inspects the XML declaration (e.g. `encoding="utf-16"`) when a BOM is absent.
  ````
* **Evidence**: `read_datamashup_text` and `decode_utf16_xml` in `core/src/excel_open_xml.rs`.
* **Impact**:
  Excel itself is likely to emit UTF‑16 with a BOM, and the fixtures explicitly cover UTF‑16LE/BE with BOM and succeed.
  However, a third‑party tool that writes DataMashup XML as UTF‑16 without a BOM but with `encoding="utf-16"` would be treated as raw bytes and fed to `quick_xml::Reader` as if it were UTF‑8, leading to an `XmlParseError` instead of a successful parse. This is outside the current spec’s explicit requirements but narrows interoperability slightly. It’s safe to ship, but worth documenting or broadening later.

---

### Finding 3: Length invariant check in integration test is weaker than it appears

* **Severity**: Minor
* **Category**: Missing Test / Weak Test
* **Description**:
  The “happy path” integration test for a valid DataMashup reconstructs a top-level byte stream from `RawDataMashup` and then asserts that its length equals `4 * 5 + sum(section_lengths)`:

  ```rust
  let assembled = assemble_top_level_bytes(&raw);
  let expected_len = 4 * 5
      + raw.package_parts.len()
      + raw.permissions.len()
      + raw.metadata.len()
      + raw.permission_bindings.len();
  assert_eq!(assembled.len(), expected_len);
  ```

  But `assemble_top_level_bytes` itself builds `assembled` from those same lengths and payloads, so `assembled.len() == expected_len` is effectively tautological and does not check the *original* `dm_bytes` from the workbook. The real invariant enforcement is in `parse_data_mashup` (which is already well-covered by separate unit tests).
* **Evidence**:

  * `assemble_top_level_bytes` in `core/tests/data_mashup_tests.rs`.
  * Integration test `workbook_with_valid_datamashup_parses`. 
* **Impact**:
  If `parse_data_mashup` were changed in a way that still produced self-consistent `RawDataMashup` sections but mis-aligned them with the original `dm_bytes` (for example, by ignoring trailing garbage and truncating silently), this integration test would not catch it.

  That said, the spec’s invariants for framing are already enforced and tested directly via:

  * Minimum size and bounds checks.
  * Unit tests for truncated streams, trailing bytes, unsupported versions, and too-short buffers.

  So this is more about strengthening redundancy than plugging a dangerous hole.

---

### Finding 4: No direct `open_data_mashup` test for missing `[Content_Types].xml`

* **Severity**: Minor
* **Category**: Missing Test
* **Description**:
  The spec states that `open_data_mashup` should surface container errors in the same way as `open_workbook`, including treating a ZIP without `[Content_Types].xml` as `NotExcelOpenXml`.

  The implementation does this:

  ````rust
  pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, ExcelOpenError> {
      let mut archive = open_zip(path.as_ref())?;

      if archive.by_name("[Content_Types].xml").is_err() {
          return Err(ExcelOpenError::NotExcelOpenXml);
      }

      let dm_bytes = match extract_datamashup_bytes_from_excel(&mut archive)? {
          Some(bytes) => bytes,
          None => return Ok(None),
      };

      parse_data_mashup(&dm_bytes).map(Some)
  }
  ``` :contentReference[oaicite:11]{index=11}  

  There is, however, no specific test in `data_mashup_tests.rs` that calls `open_data_mashup` on the existing `no_content_types.xlsx` fixture. Instead:
  * `excel_open_xml_tests.rs` covers `no_content_types.xlsx` via `open_workbook`. :contentReference[oaicite:12]{index=12}  
  * `data_mashup_tests.rs` covers container behavior for non-Excel random ZIPs and non-zip files. 
  ````
* **Evidence**:

  * `open_data_mashup` implementation. 
  * `excel_open_xml_tests.rs` `no_content_types_is_not_excel`. 
  * `data_mashup_tests.rs` list of tests.
* **Impact**:
  Currently, a regression where `open_data_mashup` stopped checking `[Content_Types].xml` (or handled it differently) would not be caught by tests, even though `open_workbook` would remain protected. The actual code is simple and obviously correct, so this is a low-risk gap, but adding a mirrored test would lock in the container behavior for both APIs.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed

  * `RawDataMashup` defined exactly as specified and re-exported.
  * `open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, ExcelOpenError>` implemented and re-exported from `lib.rs`.
  * DataMashup-related `ExcelOpenError` variants added with `thiserror` annotations. 
  * Helper functions `extract_datamashup_bytes_from_excel`, `parse_data_mashup`, `read_datamashup_text`, etc., implemented per spec intent.

* [x] All specified tests created

  * Unit tests for framing: minimal zero-length, basic non-zero lengths, unsupported version, truncated stream, trailing bytes, plus an additional “too short” guard.
  * Integration tests: no DataMashup, valid DataMashup, corrupt base64, nonexistent file, non-Excel random ZIP; plus additional tests for base64 whitespace, UTF‑16 LE/BE, and duplicate parts/elements.
  * Fuzz-style “never panics” test for `parse_data_mashup` on arbitrary small byte arrays.

* [x] Behavioral contract satisfied

  * `Ok(None)` when no `<DataMashup>` element is found.
  * `Ok(Some(raw))` with `version == 0` and four sections when a single well-formed DataMashup exists; invariants enforced by `parse_data_mashup` (header size, bounds, trailing bytes).
  * `Err(DataMashupBase64Invalid)` for corrupt base64, `Err(DataMashupUnsupportedVersion {..})` for nonzero versions, `Err(DataMashupFramingInvalid)` for length/bounds violations or multiple mashups.
  * Container and IO errors surfaced via unchanged `Io`, `NotZipContainer`, and `NotExcelOpenXml` variants.

* [x] No undocumented deviations from spec

  * All differences from the “nice-to-have” suggestions (e.g., location of parser code inside `excel_open_xml.rs` rather than a separate module) fall within the “internal interfaces may change” guidance in the decision record.

* [x] Error handling adequate

  * DataMashup-related paths use `Result` and custom error variants; no `unwrap`/`expect` on untrusted data.
  * XML and ZIP errors are mapped into structured `ExcelOpenError` variants.

* [x] No obvious performance regressions

  * DataMashup streams are read into a single `Vec<u8>` and sliced; complexity is linear in stream size with no nested heavy loops.
  * Feature gating remains unchanged: all ZIP/FS-related code is under `excel-open-xml`, and dependencies are marked optional accordingly, preserving the WASM no-default-features build.

---

While all issues identified are **minor** and do not compromise correctness, robustness within the supported envelope, or API contracts, **remediation is advised** in order to ensure the strongest possible forward position for the next cycle.