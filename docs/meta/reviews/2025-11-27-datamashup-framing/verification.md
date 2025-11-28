```markdown
# Verification Report: 2025-11-27-datamashup-framing

## Summary

The implementation closely follows the mini-spec and decision record for the `2025-11-27-datamashup-framing` branch. The new `RawDataMashup` type, `open_data_mashup` API, framing parser, and error variants are all present and wired correctly behind the `excel-open-xml` feature gate. Unit tests for `parse_data_mashup` and integration tests for `open_data_mashup` match the specified test plan and add several useful extra cases (UTF‑16 handling, base64 whitespace, duplicate detection). I did not find any issues that rise to **Critical** or **Moderate** severity. There are a few **Minor** points: (1) DataMashup element detection is looser than the architecture blueprint (no namespace/root check), (2) malformed non‑DataMashup `customXml` parts cause `XmlParseError` instead of being ignored, and (3) a small set of additional tests could further harden the implementation. Overall, the behavioral contract in the mini‑spec is satisfied.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### 1. DataMashup XML detection ignores namespace and root element

- **Severity**: Minor  
- **Category**: Gap / Spec Deviation (from architecture blueprint, not the mini-spec)  
- **Description**:  
  The DataMashup host-layer extraction looks for any `<DataMashup>` element in any `customXml/*.xml` part, based solely on the **local name** `"DataMashup"`:

  * `extract_datamashup_bytes_from_excel` iterates every ZIP entry whose name starts with `customXml/` and ends with `.xml`. For each, it calls `read_datamashup_text`. :contentReference[oaicite:0]{index=0}  
  * `read_datamashup_text` uses `is_datamashup_element(e.name().as_ref())` on every `Start` / `End` event, and collects the text of the first such element. :contentReference[oaicite:1]{index=1}  
  * `is_datamashup_element` matches by suffix after the last colon (`dm:DataMashup` → `"DataMashup"`), but does **not** check the namespace URI or that the element is the **document root**. :contentReference[oaicite:2]{index=2}  

  The architecture blueprint `excel_diff_m_query_parse.md` describes a stricter invariant: enumerate `/customXml/item*.xml` and “look for a document whose root element is `DataMashup` in namespace `http://schemas.microsoft.com/DataMashup`.” :contentReference[oaicite:3]{index=3}  
- **Evidence**:  
  * `extract_datamashup_bytes_from_excel`, `read_datamashup_text`, and `is_datamashup_element` in `core/src/excel_open_xml.rs`.   
  * Architecture blueprint section “1. Outer container → DataMashup bytes”. :contentReference[oaicite:5]{index=5}  
- **Impact**:  
  * On typical Excel workbooks (where only the real Power Query part uses `DataMashup`), behavior is correct and well-tested.
  * In a hypothetical workbook where some **other** `customXml` part contains an element named `DataMashup` in a different namespace or deeper in the tree, `open_data_mashup` might:
    * Attempt to interpret that text as base64 and then as a DataMashup binary stream.
    * Return `DataMashupBase64Invalid` / `DataMashupFramingInvalid` rather than `Ok(None)` even though there is no true Power Query DataMashup.   
  * The code still fails **gracefully** (no panics; errors are structured), but this is slightly looser than the blueprint and could force consumers to handle unexpected DataMashup errors in edge-case workbooks.

---

### 2. Malformed non‑DataMashup customXml parts are treated as fatal

- **Severity**: Minor  
- **Category**: Gap / Behavioral nuance  
- **Description**:  
  When scanning `customXml/*.xml` parts, any XML parse error in `read_datamashup_text` is turned into `ExcelOpenError::XmlParseError`, which short‑circuits `extract_datamashup_bytes_from_excel` and thus `open_data_mashup`:

  * `read_datamashup_text` uses `quick_xml::Reader` and returns `Err(ExcelOpenError::XmlParseError(e.to_string()))` on any `Err(e)` from the XML reader. :contentReference[oaicite:7]{index=7}  
  * `extract_datamashup_bytes_from_excel` propagates that `Err` immediately, even if the part doesn’t actually contain a `<DataMashup>` element. :contentReference[oaicite:8]{index=8}  

  The mini‑spec’s “no DataMashup” example describes a clean workbook (e.g. `minimal.xlsx`) where `open_data_mashup` should return `Ok(None)`. :contentReference[oaicite:9]{index=9} It does not explicitly specify what should happen when **other** `customXml` parts are malformed.
- **Evidence**:  
  * `read_datamashup_text` and `extract_datamashup_bytes_from_excel` in `excel_open_xml.rs`.   
  * Spec section “2. Behavioral contract” and the “Workbook with no Power Query / DataMashup” example. :contentReference[oaicite:11]{index=11}  
- **Impact**:  
  * For valid workbooks (all XML well-formed), behavior is exactly as specified and verified by tests like `workbook_without_datamashup_returns_none`. :contentReference[oaicite:12]{index=12}  
  * For a workbook with **corrupt** `customXml` that does **not** contain a real DataMashup, `open_data_mashup` will return `Err(ExcelOpenError::XmlParseError(..))` instead of `Ok(None)`, while `open_workbook` will still be able to open the workbook (since it never touches those parts).   
  * This is arguably acceptable (it surfaces actual host corruption), but it’s a subtle divergence from a naïve “no DataMashup ⇒ always Ok(None)” reading. Callers that assume “no Power Query ⇒ no errors” might need to be aware of this behavior.

---

### 3. Additional tests that could harden behavior (non-blocking)

- **Severity**: Minor  
- **Category**: Missing Test / Coverage gap  
- **Description**:  
  The tests that were **promised** in the mini‑spec are all present and meaningful:

  * Unit tests for `parse_data_mashup`: zero-length, non-zero lengths, unsupported version, truncated stream, trailing bytes, and header-length < min size, plus a fuzz-style no-panic test.   
  * Integration tests for `open_data_mashup`: no DataMashup, valid DataMashup, corrupt base64, NotZip/NotExcel errors, duplicate parts, duplicate elements, and UTF‑16 / base64-whitespace cases.   

  From a robustness perspective, a couple of extra tests would tighten things further, even though they weren’t explicitly required:
  1. **Integration test for unsupported version**  
     * Today, `ExcelOpenError::DataMashupUnsupportedVersion { .. }` is only exercised via a **unit test** that feeds synthetic bytes directly into `parse_data_mashup`.   
     * There is no fixture that routes this through `open_data_mashup`, so if the mapping from `parse_data_mashup` errors to host errors was accidentally changed later, CI might not catch it.
  2. **Targeted test for “overflow length” invariants**  
     * The parser uses `usize::try_from(len)` and `checked_add` to guard against integer overflow and out-of-bounds slicing. :contentReference[oaicite:17]{index=17}  
     * Current tests cover “length larger than buffer” (truncated stream) and “extra trailing bytes,” but not a crafted case where some lengths are large enough to stress the `checked_add` path. This is more theoretical, but a small synthetic test would lock in those invariants.

- **Evidence**:  
  * Existing unit tests in `core/src/excel_open_xml.rs` test module.   
  * Existing integration tests in `core/tests/data_mashup_tests.rs`.   
- **Impact**:  
  * No current bugs identified; these are hardening suggestions.
  * Adding such tests would reduce the risk of future regressions when the DataMashup stack is refactored or extended.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  * `RawDataMashup` struct with the expected fields is implemented and re-exported.   
  * `open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, ExcelOpenError>` exists in `excel_open_xml` and is re-exported from `lib.rs`.   
  * New `ExcelOpenError` variants for DataMashup (`DataMashupBase64Invalid`, `DataMashupUnsupportedVersion`, `DataMashupFramingInvalid`) are present and used in the expected places.   

- [x] All specified tests created  
  * The mini‑spec’s unit tests for `parse_data_mashup` (zero-length, non-zero, unsupported version, truncated stream, trailing bytes) are present and assert the correct variants.   
  * The integration tests described for `open_data_mashup` (no DataMashup, valid DataMashup, corrupt base64, container error propagation) exist and use the specified fixtures.   

- [x] Behavioral contract satisfied  
  * `Ok(None)` for valid workbooks without DataMashup (e.g. `minimal.xlsx`) is implemented and covered by tests.   
  * `Ok(Some(RawDataMashup{…}))` for valid DataMashup (`m_change_literal_b.xlsx`) with correct slicing and reassembly is implemented and tested via `assemble_top_level_bytes` round-trip.   
  * Container errors (IO, not ZIP, not Excel Open XML) are shared with `open_workbook` via `open_zip` and corresponding tests.   
  * Base64 and framing errors return the correct new variants and are validated by both unit and integration tests.   

- [x] No undocumented deviations from spec  
  * The minor differences identified are deviations from the **broader architecture blueprint** (namespace/root strictness, handling of malformed unrelated `customXml`) rather than from the mini‑spec’s behavioral contract. The spec’s promised behaviors are met.

- [x] Error handling adequate  
  * DataMashup paths use `Result` and explicit error mapping, with no `unwrap`/`expect` on untrusted data.   
  * Duplicate DataMashup parts or elements are rejected with `DataMashupFramingInvalid`, as intended, with dedicated fixtures and tests.   

- [x] No obvious performance regressions  
  * The framing parser is linear in the size of `dm_bytes` and allocates a small number of `Vec<u8>` buffers as allowed by the spec’s performance notes.   
  * The host-layer scanning over `customXml` is a simple single pass; no new nested loops over large structures were introduced, and existing workbook parsing tests still pass.   
```
