```markdown
# Verification Report: 2025-12-03b-m4-permissions-metadata

## Summary

The implemented `DataMashup` IR, `parse_permissions`, `parse_metadata`, and `build_data_mashup` functions match the mini-spec for Milestone 4, including the behavior described in the cycle plan and the follow‑up remediation document. All required types and APIs are present and exported, all specified tests have been implemented with meaningful assertions, and prior remediation findings (Fixes 1–6) have been addressed and are now covered by targeted tests. I did not find any correctness issues that would block release; the remaining items are documentation / future‑proofing concerns and small test-coverage opportunities.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### 1. Group path derivation differs from the narrative in the mini-spec (but matches remediation plan)

- **Severity**: Minor  
- **Category**: Spec Deviation / Documentation Gap  
- **Description**:  
  The mini-spec describes `group_path` as being derived from the `AllFormulas` tree in `LocalPackageMetadataFile`. :contentReference[oaicite:0]{index=0}  
  The actual implementation derives `group_path` solely from per‑formula `Entry` values (e.g., `QueryGroupPath`, `QueryGroupId`, `QueryGroupID`, `QueryGroup`) on each `ItemType=Formula` item and does **not** parse the `AllFormulas` tree. This behavior is explicitly documented in a code comment:

  > `// Group paths are derived solely from per-formula entries for now; the AllFormulas tree is not parsed yet.` :contentReference[oaicite:1]{index=1}  

  This is also the path explicitly blessed in the remediation plan as an acceptable simplification (“Option 2: rely on per-formula entries and document it”). :contentReference[oaicite:2]{index=2}
- **Evidence**:  
  - `entry_string` helper and usage when constructing `QueryMetadata`: keys are drawn from per-item entries, not from `AllFormulas`. :contentReference[oaicite:3]{index=3}  
  - Comment in `parse_metadata` clarifying that `AllFormulas` is not parsed. :contentReference[oaicite:4]{index=4}  
  - The mini-spec text still talks about deriving `group_path` from the `AllFormulas` group hierarchy. :contentReference[oaicite:5]{index=5}  
- **Impact**:  
  - For current fixtures (`metadata_query_groups.xlsx`) where per‑formula group entries are present, behavior matches the spec’s *observable* semantics: tests confirm that `group_path` equals `"Inputs/DimTables"` for a grouped query and is `None` for a root query. :contentReference[oaicite:6]{index=6}  
  - For future workbooks that rely solely on `AllFormulas` for grouping with no per‑formula `QueryGroupPath` entries, `group_path` would end up `None`, losing grouping information. Since `group_path` is not yet consumed elsewhere in the product, this is a future-compatibility/documentation concern rather than a current correctness bug.

---

### 2. Metadata XML handling extends the mini-spec (header + XML) but the docs still assume “XML-only”

- **Severity**: Minor  
- **Category**: Spec Deviation / Documentation Gap  
- **Description**:  
  The mini-spec states that `parse_metadata` “treats `metadata_bytes` as UTF‑8 XML for this milestone (no separate ‘Metadata Content OPC’ parsing).” :contentReference[oaicite:7]{index=7}  
  The implementation is more precise and aligned with the real Excel layout discovered in this cycle: it first checks whether the bytes “look like XML” and otherwise interprets the first eight bytes as two little-endian `u32` length fields (content length and XML length) and extracts the XML slice accordingly.   
  This matches the activity log note that real metadata is “two u32 lengths (first zero, second XML length) before BOM/XML,” and the remediation plan adds targeted tests for the header and error cases. 
- **Evidence**:  
  - `metadata_xml_bytes` implements a header-aware strategy (two `u32` + XML) with strict length overflow checks and a fallback “XML detected by leading `<`/BOM” path. :contentReference[oaicite:10]{index=10}  
  - New tests validate: empty metadata, too-short header, invalid length prefix, invalid UTF‑8, and malformed XML behavior. :contentReference[oaicite:11]{index=11}  
- **Impact**:  
  - Behavior is strictly *more* robust than the mini-spec, and it aligns with the actual Excel metadata format, so there is no correctness risk.  
  - Future readers of the spec may be confused, since the spec still suggests a simpler “treat bytes as XML” model. Aligning the docs with the implemented layout (header + XML) will make debugging and future refactors easier.

---

### 3. Behavioral contract for permissions is satisfied, but tests don’t exercise all minor parse_bool variants

- **Severity**: Minor  
- **Category**: Missing Test (non-critical)  
- **Description**:  
  The `parse_permissions` implementation supports tolerant boolean parsing (`"1"`, `"0"`, `"true"`, `"false"`, `"yes"`, `"no"`, and the `l0`/`l1` style).   
  The tests focus on realistic fixtures and key error semantics:
  - Defaults vs firewall off (`permissions_defaults.xlsx` vs `permissions_firewall_off.xlsx`).  
  - Missing permissions or malformed XML default to `Permissions::default()`.  
  - Invalid XML entities in text cause a fallback to defaults. :contentReference[oaicite:13]{index=13}  
  They do not explicitly verify every individual `parse_bool` token variant (e.g., `"yes"`, `"no"`, `"l1"`, `"l0"`).
- **Evidence**:  
  - `parse_bool` implementation includes several accepted tokens and lowercasing logic. :contentReference[oaicite:14]{index=14}  
  - Tests cover behavior at the `Permissions` struct level for realistic Excel fixtures and error modes but not each token. :contentReference[oaicite:15]{index=15}  
- **Impact**:  
  - No immediate correctness risk: real-world fixtures likely use `l0`/`l1` or `true`/`false`, which are covered by the implementation and at least partially by the tests.  
  - Slight test gap: if someone “optimizes” `parse_bool` later, they could inadvertently drop support for some tokens without a failing test. This is a “nice-to-have” extra assertion cluster, not a release blocker.

---

### 4. `parse_metadata` does not explicitly validate the `LocalPackageMetadataFile` / `Formulas` hierarchy

- **Severity**: Minor  
- **Category**: Gap / Spec Deviation (non-observable today)  
- **Description**:  
  The mini-spec says that on success `parse_metadata` “locates the `LocalPackageMetadataFile` root and the `Formulas` collection” and then iterates per-query entries under that collection. :contentReference[oaicite:16]{index=16}  
  The implementation is structurally simpler: it walks the XML event stream and reacts to `Item`, `ItemType`, `ItemPath`, and `Entry` elements regardless of their exact ancestry in the tree. It filters to `ItemType == "Formula"` and uses `Entry` attributes for load flags and group path. It does **not** assert that these elements are under the expected `LocalPackageMetadataFile/Formulas` path.   
- **Evidence**:  
  - `parse_metadata` uses a generic `element_stack` but only branches on element names; it never checks the full qualified path (e.g., “we are inside `LocalPackageMetadataFile/Formulas/Item`”).   
  - Tests assert correct behavior for known-good fixtures (`metadata_simple`, `metadata_query_groups`, `metadata_hidden_queries`) and for malformed XML, but there is no test where metadata XML has a valid shape but elements are mis-nested.   
- **Impact**:  
  - For properly formed Excel metadata, behavior is correct and well-covered by tests.  
  - For a hypothetical workbook where metadata XML is structurally valid but `Item`/`Entry` elements are moved under some unexpected parent, the parser might still pick them up or miss them; the spec would prefer either strict rejection or clearly documented “best effort.”  
  - This is not a practical risk for real Excel files today; it’s an internal robustness/clarity issue.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - `core/src/datamashup.rs` exists with `DataMashup`, `Permissions`, `Metadata`, `QueryMetadata`, `build_data_mashup`, `parse_permissions`, and `parse_metadata`. :contentReference[oaicite:20]{index=20}  
  - Existing framing (`datamashup_framing`), package parts, and `m_section` modules are reused without breaking changes.   
  - Public exports in `core/src/lib.rs` match the plan.   

- [x] All specified tests created  
  - `core/tests/m4_permissions_metadata_tests.rs` includes all eight named tests from the plan:  
    - `permissions_parsed_flags_default_vs_firewall_off`  
    - `permissions_missing_or_malformed_yields_defaults`  
    - `metadata_formulas_match_section_members`  
    - `metadata_load_destinations_simple`  
    - `metadata_groups_basic_hierarchy`  
    - `metadata_hidden_queries_connection_only`  
    - `permission_bindings_present_flag`  
    - `permission_bindings_missing_ok`   
  - `core/tests/data_mashup_tests.rs` has the strengthened `build_data_mashup_smoke_from_fixture` test as required by the plan and remediation Fix 5.   
  - Additional remediation tests around metadata header/UTF‑8/XML error paths and ItemPath decoding are present.   

- [x] Behavioral contract satisfied  
  - `build_data_mashup` correctly wires `version`, `package_parts`, `permissions`, `metadata`, and `permission_bindings_raw` from `RawDataMashup`, propagating only `parse_package_parts`/`parse_metadata` errors and never failing on permissions.   
  - `parse_permissions` defaults on any decode/XML/unescape error and populates flags correctly for the tested fixtures.   
  - `parse_metadata` enforces the empty/invalid/error semantics described in the plan and the remediation tests.   
  - Invariants like `item_path == format!("{section_name}/{formula_name}")` and `is_connection_only == !(load_to_sheet || load_to_model)` are enforced and exercised in tests (`build_data_mashup_smoke_from_fixture`, `metadata_load_destinations_simple`).   

- [x] No undocumented deviations from spec  
  - The only notable deviations (group path derived from per‑formula entries and richer metadata header handling) are either documented in code comments and the remediation plan or strengthen robustness without changing observable semantics for the current fixtures.   

- [x] Error handling adequate  
  - Permissions: any decoding/XML/unescape error results in `Permissions::default()` without failing `build_data_mashup`.   
  - Metadata: malformed headers, invalid lengths, invalid UTF‑8, and malformed XML produce `DataMashupError::XmlError(_)` as intended.   
  - Underlying framing and container tests, including fuzz testing for `parse_data_mashup`, still pass and guard against panics.   

- [x] No obvious performance regressions  
  - Parsing routines are single-pass, streaming over XML with `quick_xml` and storing only small vectors of formulas/entries, which is consistent with prior architecture guidance and streaming constraints.   
  - No new allocations or data structures with superlinear behavior were introduced in hot paths.

```

Since none of the findings rise to Moderate or Critical severity, I’m not including a separate remediation plan; the notes above can be folded into future documentation and test-hardening work as needed.
