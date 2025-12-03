# Remediation Plan: 2025-12-03-m4-packageparts

## Overview

The core `PackageParts` implementation appears correct and robust, and most of the behavioral surface is well tested. The remaining issues are primarily about *locking down contracts with tests* (invalid UTF‑8 in main Section1, BOM semantics at the PackageParts boundary, mixed valid/invalid embedded content) and tidying up minor spec/implementation drift. This plan scopes those fixes to small, focused changes in tests and documentation, plus one tiny robustness tweak to align `EmbeddedContent.name` with the spec wording.

## Fixes Required

### Fix 1: Add negative test for invalid UTF‑8 in `/Formulas/Section1.m`

- **Addresses Finding**: Finding 1 (invalid UTF‑8 in main Section1.m untested)  
- **Changes**:
  - File: `core/tests/m4_package_parts_tests.rs`  
  - Add a new test that mirrors `invalid_utf8_in_package_xml_errors` but targets Section1 instead:
    - Build an in-memory ZIP using the existing `build_zip` helper with:
      - `"Config/Package.xml"` set to `MIN_PACKAGE_XML.as_bytes().to_vec()` (valid XML).
      - `"Formulas/Section1.m"` set to a small invalid UTF‑8 byte sequence, e.g. `vec![0xFF, 0xFF]`.
    - Call `parse_package_parts(&bytes)` and assert it returns `Err(DataMashupError::FramingInvalid)`.
- **Tests**:
  - New test: `invalid_utf8_in_section1_errors`  
  - Example shape:

    ```rust
    #[test]
    fn invalid_utf8_in_section1_errors() {
        let bytes = build_zip(vec![
            ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
            ("Formulas/Section1.m", vec![0xFF, 0xFF]),
        ]);

        let err = parse_package_parts(&bytes)
            .expect_err("invalid UTF-8 in Section1.m should error");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }
    ```

---

### Fix 2: Assert BOM is stripped at the `PackageParts` boundary

- **Addresses Finding**: Finding 2 (BOM behavior not asserted at SectionDocument boundary)  
- **Changes**:
  - File: `core/tests/m4_package_parts_tests.rs`  
  - Enhance the existing `package_parts_section1_with_bom_parses_via_parse_section_members` test to assert that `parts.main_section.source` does **not** start with `'\u{FEFF}'`:
    - Immediately after calling `parse_package_parts`, assert:

      ```rust
      assert!(
          !parts.main_section.source.starts_with('\u{FEFF}'),
          "PackageParts should strip a single leading BOM from Section1.m"
      );
      ```

    - Then continue to call `parse_section_members` and verify members as currently done.
- **Tests**:
  - Modified test: `package_parts_section1_with_bom_parses_via_parse_section_members`  
  - No new test file; just a stronger assertion in the existing integration test.

---

### Fix 3: Clarify PackageXml behavior in the main specification

- **Addresses Finding**: Finding 3 (spec suggests structured XML parsing that isn’t implemented yet)  
- **Changes**:
  - File: `docs/rust_docs/excel_diff_specification.md` (Section 4.2 “PackageParts (Embedded OPC/ZIP)”)   
  - Adjust bullet 3 of the “Practical parsing strategy” to reflect the current milestone’s behavior, e.g.:

    **Replace this prose (conceptually):**

    > Read `/Config/Package.xml` as UTF‑8 XML; parse fields such as client version, minimum compatible version, culture, etc.

    **With something like:**

    > Read `/Config/Package.xml` as UTF‑8 XML. For the current milestone, surface it as an opaque `PackageXml { raw_xml }` string without parsing individual fields; later milestones may extract structured fields (client version, minimum reader, culture, etc.).

  - Ensure this wording is consistent with the mini-spec’s Section 2.2, which already describes `raw_xml` as an opaque string for now.   
- **Tests**:
  - No test changes required; this is documentation-only.

---

### Fix 4: Align `EmbeddedContent.name` with `normalize_path` semantics

- **Addresses Finding**: Finding 4 (future drift between `normalize_path` and `EmbeddedContent.name`)  
- **Changes**:
  - File: `core/src/datamashup_package.rs`   
  - When constructing `EmbeddedContent`, use `normalize_path(&raw_name)` instead of reimplementing the trimming logic manually. This keeps the data model in lockstep with the path-normalization function.

  **Code to replace:**

  ```rust
  embedded_contents.push(EmbeddedContent {
      name: raw_name.trim_start_matches('/').to_string(),
      section: SectionDocument { source: section },
  });
````

**New code:**

```rust
embedded_contents.push(EmbeddedContent {
    name: normalize_path(&raw_name).to_string(),
    section: SectionDocument { source: section },
});
```

* This is behaviorally identical today, but it ensures any future enhancements to `normalize_path` automatically apply to `EmbeddedContent.name` as the spec intends.
* **Tests**:

  * Existing tests `leading_slash_paths_are_accepted` and `embedded_content_name_is_canonicalized` should continue to pass unchanged.

---

### Fix 5: Add a mixed valid/invalid embedded-content test

* **Addresses Finding**: Finding 5 (no test proving malformed embedded contents don’t suppress valid ones)
* **Changes**:

  * File: `core/tests/m4_package_parts_tests.rs`
  * Add a new test that builds a PackageParts ZIP with:

    * Valid `Config/Package.xml` and main `Formulas/Section1.m` using existing helpers/fixtures.
    * Two entries under `Content/`:

      * One nested ZIP built via `build_embedded_section_zip` containing a valid Section1 (`MIN_SECTION` or similar).
      * One malformed entry, e.g. `"Content/bad.package"` containing `b"not a zip"`.
  * Call `parse_package_parts` and assert:

    * `embedded_contents.len() == 1`.
    * The surviving `EmbeddedContent` matches the valid nested ZIP (e.g. its `section.source` contains `shared` and `section Section1;`).
* **Tests**:

  * New test: `embedded_content_partial_failure_retains_valid_entries`
  * Example structure:

    ```rust
    #[test]
    fn embedded_content_partial_failure_retains_valid_entries() {
        let good_nested =
            build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
        let bytes = build_minimal_package_parts_with(vec![
            ("Content/good.package", good_nested),
            ("Content/bad.package", b"not a zip".to_vec()),
        ]);

        let parts = parse_package_parts(&bytes).expect("outer package should parse");
        assert_eq!(parts.embedded_contents.len(), 1);
        let embedded = &parts.embedded_contents[0];
        assert_eq!(embedded.name, "Content/good.package");
        assert!(embedded.section.source.contains("section Section1;"));
        assert!(embedded.section.source.contains("shared"));
    }
    ```

---

## Constraints

* **API stability**:

  * No changes to public type signatures (`PackageXml`, `SectionDocument`, `EmbeddedContent`, `PackageParts`, `parse_package_parts`) are allowed, per the mini-spec.
  * All proposed changes stay within tests, documentation, or a tiny internal helper usage change.

* **Performance**:

  * New tests should be in-memory only (no disk I/O) and bounded:

    * Invalid UTF‑8 test uses very small buffers.
    * Mixed embedded-content test uses tiny in-memory ZIPs via existing helpers.
  * No changes to parsing complexity; only one line in the core parser is modified (use of `normalize_path`).

* **Behavioral contracts**:

  * Required parts missing or invalid still map to `DataMashupError::FramingInvalid`.
  * Malformed `Content/*` entries are still best-effort skipped.
  * Arbitrary bytes must continue not to panic (fuzz test already exists and should remain).

## Expected Outcome

After this remediation:

* Core error paths for **both** `Config/Package.xml` and `Formulas/Section1.m` have explicit negative tests for invalid UTF‑8, reducing the risk of regressions in future refactors.
* BOM semantics at the `PackageParts` boundary are directly asserted, so `SectionDocument.source` is guaranteed to be BOM‑free even if M parsing behavior changes later.
* The behavior for mixed valid/invalid embedded contents is locked in: malformed `Content/*` entries are skipped without suppressing valid ones.
* Documentation in `excel_diff_specification.md` cleanly reflects the milestone’s scope (opaque `raw_xml` now, structured field parsing later), reducing confusion for future work.
* `EmbeddedContent.name` is formally tied to `normalize_path`, eliminating a subtle potential for future drift.

At that point, the PackageParts layer is not only functionally correct but also well pinned down by tests and documentation, making it a solid foundation for subsequent Permissions, Metadata, and M-domain milestones.

