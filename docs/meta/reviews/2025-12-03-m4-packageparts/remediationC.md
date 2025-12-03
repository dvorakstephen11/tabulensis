```markdown
# Remediation Plan: 2025-12-03-m4-packageparts

## Overview

The verification pass for branch `2025-12-03-m4-packageparts` found two minor gaps in the otherwise complete PackageParts implementation and test suite:

1. No dedicated regression test that exercises **UTF-8 BOM handling for embedded `/Formulas/Section1.m` inside `Content/*.package`**.
2. No explicit test that covers **PackageParts zips with an empty `Content/` directory entry but no embedded packages**.

The goal of this remediation is to add narrowly scoped tests that lock in the intended behavior without changing the existing `parse_package_parts` API, error semantics, or performance characteristics. All changes are confined to the Rust test modules and, if needed, minimal test-only helpers. :contentReference[oaicite:0]{index=0} :contentReference[oaicite:1]{index=1} :contentReference[oaicite:2]{index=2} :contentReference[oaicite:3]{index=3} :contentReference[oaicite:4]{index=4}

## Fixes Required

### Fix 1: BOM regression test for embedded `Section1.m` content

- **Addresses Finding**: Missing regression test for UTF-8 BOM in *nested* embedded Section1 documents (embedded `Content/*.package`), even though the implementation already strips a single leading BOM via `find_section_document`/`strip_leading_bom`. This is the test-only gap identified in the verification report.  

- **Changes**:

  - **File**: `core/tests/m4_package_parts_tests.rs` :contentReference[oaicite:5]{index=5}  
  - Add a new unit test that mirrors the existing `package_parts_section1_with_bom_parses_via_parse_section_members`, but exercises the *embedded* path instead of the main Section1:

    1. Use the existing helpers and constants already defined in the test module:
       - `BOM_SECTION`: the BOM-prefixed `"section Section1; shared Foo = 1;"` string used by the current main-section BOM test.
       - `build_embedded_section_zip(section_bytes: Vec<u8>)` to construct an in-memory ZIP with `Formulas/Section1.m`.  
       - `build_minimal_package_parts_with(...)` to wrap extra entries into a minimal outer PackageParts container containing both `Config/Package.xml` and `Formulas/Section1.m`. :contentReference[oaicite:6]{index=6}  

    2. Construct an outer ZIP where:
       - `/Config/Package.xml` is a valid minimal XML (e.g., `MIN_PACKAGE_XML`).
       - `/Formulas/Section1.m` is a clean, non-BOM main section (e.g., `MIN_SECTION`).
       - One `Content/` entry (e.g., `"Content/bom_embedded.package"`) contains an inner ZIP built by `build_embedded_section_zip(BOM_SECTION.as_bytes().to_vec())`.

    3. Call `parse_package_parts(&bytes)` and assert:
       - The outer package parses successfully (no error).
       - `parts.embedded_contents.len() >= 1`.
       - For each embedded content (or at least the one we constructed):
         - `!embedded.section.source.starts_with('\u{FEFF}')` — the single leading BOM is absent in the embedded section’s source.
         - `parse_section_members(&embedded.section.source)` succeeds and yields at least one member with:
           - `section_name == "Section1"`.
           - A member name like `"Foo"` (matching `BOM_SECTION`) and an expression of `"1"`.

  - Suggested test name (non-binding):  
    - `embedded_content_section1_with_bom_parses_via_parse_section_members`.

- **Tests**:

  - New test to add in `core/tests/m4_package_parts_tests.rs`:
    - `embedded_content_section1_with_bom_parses_via_parse_section_members` (or similar, grouped with the existing BOM and embedded-content tests).

- **Notes**:

  - This test should **not** alter production code; it only verifies the current behavior of `find_section_document` and `strip_leading_bom` for nested packages. :contentReference[oaicite:7]{index=7}  
  - Keep the test fully in-memory (no filesystem I/O), consistent with existing helpers.

---

### Fix 2: Robustness test for empty `Content/` directories

- **Addresses Finding**: Missing targeted test for the robustness requirement that **empty `Content/` directories are tolerated**—i.e., a PackageParts ZIP may contain a `Content/` directory entry and no `Content/*.package` files, and `parse_package_parts` must still succeed with `embedded_contents.is_empty()`. The implementation already skips directory entries via `file.is_dir()`, but this is not locked down by tests. :contentReference[oaicite:8]{index=8} :contentReference[oaicite:9]{index=9} :contentReference[oaicite:10]{index=10}  

- **Changes**:

  - **File**: `core/tests/m4_package_parts_tests.rs` :contentReference[oaicite:11]{index=11}  

  - Add a new unit test that constructs a minimal PackageParts ZIP containing:
    - A valid `Config/Package.xml`.
    - A valid `Formulas/Section1.m`.
    - An explicit **directory entry** for `"Content/"` but **no** `Content/*.package` files.

  - Implementation sketch for the test helper logic:

    1. Extend the existing `build_zip` helper or add a new helper to support directory entries:
       - You can represent entries as an enum, or more simply, treat names that end with `'/'` as directories and use `ZipWriter::add_directory(name, options)` instead of `start_file(name, options)`. The goal is that, when reading back, `ZipArchive::by_index(i)` returns entries where `file.is_dir()` is `true` for `"Content/"`. :contentReference[oaicite:12]{index=12}  

    2. Construct the outer PackageParts ZIP using something like:

       - `"Config/Package.xml"` → `MIN_PACKAGE_XML.as_bytes().to_vec()`.
       - `"Formulas/Section1.m"` → `MIN_SECTION.as_bytes().to_vec()`.
       - `"Content/"` → directory entry (no content bytes).

    3. Call `parse_package_parts(&bytes)` and assert:
       - The call returns `Ok(parts)`.
       - `parts.package_xml.raw_xml` and `parts.main_section.source` look normal (non-empty, as already covered by other tests).
       - `parts.embedded_contents.is_empty()`, confirming that the presence of a bare `Content/` directory does not produce spurious embedded contents.

  - Suggested test name (non-binding):
    - `empty_content_directory_is_ignored` or similar.

- **Tests**:

  - New test in `core/tests/m4_package_parts_tests.rs`:
    - `empty_content_directory_is_ignored` (or similarly named), located near the existing embedded-content robustness tests (`embedded_content_invalid_zip_is_skipped`, `embedded_content_missing_section1_is_skipped`, etc.).

- **Notes**:

  - This test should rely solely on in-memory ZIPs and the existing helper style; no Excel fixtures are required.
  - Do **not** change `parse_package_parts` itself—its `file.is_dir()` behavior is already correct and is what we want to pin down via this test. :contentReference[oaicite:13]{index=13}  

---

## Constraints

- **API Stability**  
  - Do not modify the public signatures or semantics of:
    - `PackageXml`, `SectionDocument`, `EmbeddedContent`, `PackageParts`, or `parse_package_parts`. :contentReference[oaicite:14]{index=14} :contentReference[oaicite:15]{index=15}  
  - All changes must be limited to `core/tests/m4_package_parts_tests.rs` and, if strictly necessary, test-only helpers in that file.

- **Error Semantics**  
  - The existing behavior must remain unchanged:
    - Invalid or missing core parts (`Config/Package.xml`, `Formulas/Section1.m`) → `Err(DataMashupError::FramingInvalid)`.  
    - Malformed `Content/*` entries (non-ZIP, missing Section1, invalid UTF-8, etc.) → skipped, not fatal; outer parse succeeds. :contentReference[oaicite:16]{index=16}  

- **Performance & Robustness**  
  - New tests must:
    - Use small in-memory buffers only.
    - Avoid filesystem or fixture I/O.
    - Keep fuzz/loop sizes small enough to remain negligible in CI time. :contentReference[oaicite:17]{index=17}  

- **Behavioral Contract**  
  - The tests must reflect (not change) the stated contract in the mini-spec and main specification:
    - BOM is stripped from *all* Section1 documents (outer and embedded) before M parsing.
    - Empty `Content/` directories are tolerated and produce no embedded contents. :contentReference[oaicite:18]{index=18} :contentReference[oaicite:19]{index=19} :contentReference[oaicite:20]{index=20}  

## Expected Outcome

After implementing this remediation plan:

- **BOM Handling is Fully Pinned Down**  
  - Both the main `Formulas/Section1.m` and any embedded `Content/*.package` `Formulas/Section1.m` are covered by explicit tests that ensure a single leading BOM is stripped and that `parse_section_members` can parse the resulting `SectionDocument`. This guards against future regressions in nested BOM handling. :contentReference[oaicite:21]{index=21} :contentReference[oaicite:22]{index=22}  

- **Empty `Content/` Directory Behavior is Locked In**  
  - A dedicated test confirms that a PackageParts ZIP with an empty `Content/` directory and no embedded packages parses successfully and results in `embedded_contents.is_empty()`. Any future changes that inadvertently treat directory entries as embedded packages will be caught immediately. :contentReference[oaicite:23]{index=23} :contentReference[oaicite:24]{index=24}  

- **No Production Behavior Changes**  
  - The behavior of `parse_package_parts` remains exactly as today; only the test suite is extended to encode the spec’s robustness expectations more completely. CI stays fast, and the branch remains safe to release once these tests are added and passing. :contentReference[oaicite:25]{index=25}  

```
