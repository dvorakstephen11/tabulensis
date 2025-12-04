# Remediation Plan: 2025-12-03b-m4-permissions-metadata

## Overview

This remediation plan addresses the identified correctness and coverage gaps in the new `DataMashup` IR and associated parsers:

1. Ensure `ItemPath` decoding correctly handles URL‑encoded UTF‑8 for non‑ASCII query and section names.  
2. Align group path derivation and tests with the intended `AllFormulas`‑based design (or explicitly document the simpler per‑formula approach).  
3. Add robust tests for `parse_metadata`’s error and header handling.  
4. Improve test coverage for URL‑encoded ItemPaths.  
5. Strengthen the smoke test for `build_data_mashup` on `one_query.xlsx`.  
6. Optionally align `parse_permissions` error semantics fully with the spec.

The fixes are localized to `core/src/datamashup.rs`, `core/tests/m4_permissions_metadata_tests.rs`, and existing fixtures/generators, and should not affect the framing or PackageParts logic.

## Fixes Required

### Fix 1: Correct URL decoding for ItemPath and support non‑ASCII names

- **Addresses Finding**: 1 (and underpins 4)  
- **Changes**:  
  - **File**: `core/src/datamashup.rs` :contentReference[oaicite:27]{index=27}  
  - **Function**: `decode_item_path`  
    - Change implementation to:
      - Accumulate decoded bytes in a `Vec<u8>` rather than pushing directly to a `String`.  
      - For non‑`%` bytes, push the raw byte; for `%xx`, decode `hi`/`lo` nibbles into a single byte and push into the buffer.  
      - After the loop, call `String::from_utf8(decoded_bytes)` and map invalid UTF‑8 to `DataMashupError::XmlError("invalid UTF‑8 in ItemPath".into())`.  
    - Keep the existing strict behavior for malformed percent‑encoding (length < 3, non‑hex digits) returning `DataMashupError::XmlError`.  
- **Tests**:  
  - **File**: `core/tests/m4_permissions_metadata_tests.rs` :contentReference[oaicite:28]{index=28}  
  - Add a new test, e.g. `metadata_itempath_decodes_percent_encoded_utf8`, that:
    - Constructs a small synthetic metadata XML string in‑test with:
      - `ItemType` = `"Formula"`.  
      - `ItemPath` = `Section1/Foo%20Bar%C3%A9`.  
      - `Entry` elements for at least one load flag (`FillEnabled="l1"`) so the item is not connection‑only.  
    - Calls `parse_metadata` on the UTF‑8 bytes (no length header needed).  
    - Asserts that:
      - There is a single `QueryMetadata`.  
      - `item_path == "Section1/Foo Baré"`.  
      - `section_name == "Section1"`.  
      - `formula_name == "Foo Baré"`.  

---

### Fix 2: Clarify and (optionally) align group_path derivation with AllFormulas

- **Addresses Finding**: 2  
- **Changes**:  
  There are two reasonable options; choose one and document it:

  1. **Full alignment (preferred)**:  
     - Extend `parse_metadata` to actually inspect the `AllFormulas` `Item` and derive group paths from that hierarchy, as described in the spec.   
     - Maintain the current behavior of reading per‑formula `QueryGroupPath` entries as an override or fallback if present.  
     - This likely requires:
       - Tracking group definitions (e.g., group IDs → paths) while processing `AllFormulas`.  
       - Using those mappings when processing each `Formula` item.  

  2. **Conscious simplification**:  
     - If full `AllFormulas` parsing is out of scope for this cycle, explicitly document in the code and spec comments that:
       - Group paths are currently taken from per‑formula `QueryGroupPath`/`QueryGroup` entries only.  
       - `AllFormulas` is ignored for now and will be implemented in a later milestone.  
     - Add a comment above `entry_string`/group‑path logic in `parse_metadata` explaining this temporary constraint.

- **Tests**:  
  - If you implement full `AllFormulas` parsing:
    - Add a new test, e.g. `metadata_groups_from_allformulas`, using either:
      - A new fixture generated to mirror real Excel’s group metadata (AllFormulas only), or  
      - A hand‑constructed metadata XML snippet where:
        - `AllFormulas` defines a group tree with IDs and paths.  
        - Individual `Formula` items reference those groups by ID, but do **not** carry `QueryGroupPath` entries.  
      - Assert that `group_path` on each `QueryMetadata` matches the hierarchical path.  
  - If you choose the simplified approach:
    - Ensure existing `metadata_query_groups.xlsx` fixtures and tests explicitly rely on per‑formula group entries only (which they already do), and document this in the test comments.   

---

### Fix 3: Add coverage for metadata header and error paths

- **Addresses Finding**: 3  
- **Changes**:  
  - **File**: `core/src/datamashup.rs`  
    - No functional changes required if code review confirms current logic is correct; the main need is tests.   
- **Tests**:  
  Add a small cluster of tests using `parse_metadata` directly, likely also in `core/tests/m4_permissions_metadata_tests.rs`:

  1. `metadata_empty_bytes_returns_empty_struct` (sanity check; currently implied but not asserted):  
     - Call `parse_metadata(&[])`.  
     - Assert `Ok(Metadata { formulas })` and `formulas.is_empty()`.  

  2. `metadata_invalid_header_too_short_errors`:  
     - Call `parse_metadata(&[0x01])` or any non‑empty slice shorter than 8 bytes.  
     - Assert that it returns `Err(DataMashupError::XmlError(_))` (you can pattern match by string contains `"metadata XML not found"` if needed).  

  3. `metadata_invalid_length_prefix_errors`:  
     - Build bytes with two little‑endian `u32`s where `start + xml_len` exceeds the slice len:  
       - Example: `content_len=0`, `xml_len=100`, but only 8 + 10 bytes total.  
     - Assert `Err(DataMashupError::XmlError(_))` with `"metadata length prefix invalid"` or similar.  

  4. `metadata_invalid_utf8_errors`:  
     - Construct a valid header where `xml_len` is small (e.g., 2), followed by an invalid UTF‑8 sequence (e.g., `0xFF, 0xFF`).  
     - Assert `Err(DataMashupError::XmlError(_))` (“metadata is not valid UTF‑8”).  

  5. `metadata_malformed_xml_errors`:  
     - Use a valid header/UTF‑8 sequence containing truncated XML (e.g., `<LocalPackageMetadataFile><foo`).  
     - Assert `Err(DataMashupError::XmlError(_))`.

These tests will exercise both the header parsing (`metadata_xml_bytes`) and the core XML/error behavior of `parse_metadata`.

---

### Fix 4: Add tests specifically for URL‑encoded ItemPaths (spaces and basic symbols)

- **Addresses Finding**: 4 (complements Fix 1)  
- **Changes**:  
  - After Fix 1’s implementation, add additional test cases to ensure:
    - `%20` becomes space.  
    - Escaped path separators or other punctuation behave as expected (e.g., `%2F` inside the formula name should be preserved and not treated as a path separator).  
- **Tests**:  
  - Extend the new `metadata_itempath_decodes_percent_encoded_utf8` test (Fix 1) or add siblings:
    - `metadata_itempath_decodes_space_and_slash`:  
      - `ItemPath="Section1/Foo%20Bar%2FInner"` should produce `section_name="Section1"`, `formula_name="Foo Bar/Inner"`, and `item_path` matching.  

---

### Fix 5: Strengthen `build_data_mashup` smoke test for `one_query.xlsx`

- **Addresses Finding**: 5  
- **Changes**:  
  - **File**: `core/tests/data_mashup_tests.rs` :contentReference[oaicite:32]{index=32}  
  - **Test**: `build_data_mashup_smoke_from_fixture`  
    - Extend assertions to verify metadata alignment, for example:
      - Assert that there is exactly one non‑connection‑only `QueryMetadata` entry for `Section1`.  
      - Assert that the `item_path`, `section_name`, and `formula_name` fields are consistent (e.g., `item_path == format!("{section_name}/{formula_name}")`).  
      - If the known formula name in `one_query.xlsx` is stable (e.g., `Foo`), assert that the metadata contains an entry for `"Section1/Foo"`.  
- **Tests**:  
  - Update the existing test instead of adding a new one to keep the test suite lean while improving its discriminative power.

---

### Fix 6 (Optional): Align `parse_permissions` error semantics with spec

- **Addresses Finding**: 6  
- **Changes**:  
  - **File**: `core/src/datamashup.rs` :contentReference[oaicite:33]{index=33}  
  - **Function**: `parse_permissions`  
    - Decide whether unescape errors (`t.unescape()`) should be treated as parse failures that cause `Permissions::default()` or should remain ignored.  
    - If strict alignment is desired:
      - In the `Event::Text` arm, change the `Err(_)` branch to immediately return `Permissions::default()` instead of continuing.  
    - Add a short comment clarifying the chosen policy.  
- **Tests**:  
  - Add a new test in `m4_permissions_metadata_tests.rs`, e.g. `permissions_invalid_entities_yield_defaults`:
    - Construct a synthetic `RawDataMashup` with permissions XML where one of the fields contains an invalid XML entity or otherwise causes `unescape()` to fail.  
    - Call `build_data_mashup` and assert `dm.permissions == Permissions::default()` if you adopt the strict policy.  

---

## Constraints

- The rustdoc snapshot in `codebase_context.md` has some formatting artifacts (e.g., `0.4` where the actual code is `0..4`), but the live code compiles and passes tests. Implementers should edit the actual Rust sources, not the markdown snapshot.   
- Changes must preserve existing behavior for all current fixtures:
  - `permissions_defaults.xlsx`  
  - `permissions_firewall_off.xlsx`  
  - `metadata_simple.xlsx`  
  - `metadata_query_groups.xlsx`  
  - `metadata_hidden_queries.xlsx`   
- Public API surface (`DataMashup`, `Permissions`, `Metadata`, `QueryMetadata`, `build_data_mashup`) should remain stable; fixes should be internal implementation and test additions.   

## Expected Outcome

After remediation:

- `decode_item_path` correctly handles URL‑encoded UTF‑8, ensuring `item_path`, `section_name`, and `formula_name` are accurate for both ASCII and non‑ASCII query names.  
- `group_path` semantics are either fully aligned with the `AllFormulas` structure or clearly documented as a temporary simplification, with tests that reflect the chosen approach.  
- `parse_metadata`’s header and error behavior is exercised by targeted tests, reducing the risk of regressions on malformed or unusual metadata streams.  
- URL‑encoded ItemPaths (spaces, slashes, non‑ASCII) are explicitly covered by tests.  
- The `build_data_mashup` smoke test provides stronger guarantees that metadata for the simplest real fixture (`one_query.xlsx`) is parsed as expected.  
- Permissions parsing semantics are well‑defined and, if adjusted, fully consistent with the “default on failure” behavior described in the mini‑spec.

Together, these changes will bring the implementation into closer alignment with the mini‑spec and improve robustness for future milestones that build on this IR.
