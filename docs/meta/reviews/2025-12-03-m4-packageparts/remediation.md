# Remediation Plan: 2025-12-03-m4-packageparts

## Overview

The current implementation of `parse_package_parts` is functionally correct and release-worthy, but there are several follow-ups that will make the PackageParts layer more robust and easier to evolve:

1. Lock in error semantics for missing/invalid core parts via negative tests.
2. Explicitly exercise nested `Content/*` failure modes, including corrupted payloads.
3. Test and document leading-slash handling and `EmbeddedContent.name` normalization.
4. Add a cheap fuzz-style robustness test for `parse_package_parts`.
5. Address the new observation about possible BOM (`\u{FEFF}`) in `Section1.m` by clarifying behavior and adding tests.

These fixes are scoped to tests, documentation, and very small, localized parsing tweaks. No API shape changes are required. 

---

## Fixes Required

### Fix 1: Error-path tests for missing/invalid PackageParts entries

- **Addresses Finding**: 1 (Error-path semantics untested)
- **Changes**:
  - Add negative tests to `core/tests/m4_package_parts_tests.rs` that build small in-memory ZIPs, without involving Excel fixtures:
    - ZIP with **no** `Config/Package.xml`, but a valid `Formulas/Section1.m`.
    - ZIP with **no** `Formulas/Section1.m`, but a valid `Config/Package.xml`.
    - ZIP where `Config/Package.xml` exists but contains intentionally invalid UTF-8 bytes.
  - Each test should call `parse_package_parts(&bytes)` directly and assert that it returns `Err(DataMashupError::FramingInvalid)` (and never panics). :contentReference[oaicite:1]{index=1}
- **Tests**:
  - `missing_config_package_xml_errors`
  - `missing_section1_errors`
  - `invalid_utf8_in_package_xml_errors`

---

### Fix 2: Tests for nested `Content/*` failure modes (including corruption)

- **Addresses Finding**: 
  - Original Finding 2 (“Nested `Content/*` failure paths untested”)
  - New `[Gap] Missing Negative Test for Embedded Content Corruption`
- **Changes**:
  - Extend the current embedded-content coverage so it explicitly exercises the “best-effort, skip on failure” contract for nested packages (spec §3.2 robustness constraints). :contentReference[oaicite:2]{index=2}
  - Add unit tests (no new Excel fixtures required) that build an outer ZIP with:
    - Valid `Config/Package.xml` and `Formulas/Section1.m`.
    - One or more `Content/*` entries that cover:
      1. **Non-ZIP content**: `Content/bogus.package` whose bytes are random text or truncated ZIP header.
      2. **Nested ZIP missing Section1**: `Content/no_section1.package` where the inner ZIP has no `Formulas/Section1.m`.
      3. **Nested invalid UTF-8**: `Content/bad_utf8.package` where inner `Formulas/Section1.m` contains invalid UTF-8.
  - Each test should assert:
    - `parse_package_parts` returns `Ok(PackageParts)`.
    - `embedded_contents` either stays empty or only includes entries for which nested `Section1.m` was valid; corrupted/missing ones must be silently skipped (no error, no panic). :contentReference[oaicite:3]{index=3}
- **Tests**:
  - `embedded_content_invalid_zip_is_skipped`
  - `embedded_content_missing_section1_is_skipped`
  - `embedded_content_invalid_utf8_is_skipped`

---

### Fix 3: Explicit tests and docs for leading slashes and `EmbeddedContent.name`

- **Addresses Findings**: 3 (Leading-slash tolerance untested), 4 (`EmbeddedContent.name` normalization vs. spec wording)
- **Changes**:

  1. **Path tests**  
     - Add unit tests that build in-memory ZIPs with:
       - Entries spelled with leading slashes: `"/Config/Package.xml"`, `"/Formulas/Section1.m"`, `"/Content/{GUID}.package"`.
       - Optionally, both `"/Config/Package.xml"` and `"Config/Package.xml"` present to verify “first found wins”.
     - Assert that:
       - `parse_package_parts` succeeds.
       - `package_xml.raw_xml` and `main_section.source` are populated as expected.
       - `embedded_contents[i].name` is the **canonical** path without a leading slash (e.g., `"Content/{GUID}.package"`). :contentReference[oaicite:4]{index=4}

  2. **Doc/spec update**  
     - Update the Rust doc comment and/or the mini-spec for `EmbeddedContent.name` to acknowledge canonicalization:
       - Clarify that `name` is the normalized PackageParts path without a leading `/`, even if the raw ZIP entry name has one.
       - This reconciles the current implementation (`trim_start_matches('/')`) with the original spec text that described `name` as the “path of the embedded package” (e.g., `"Content/{GUID}.package"`). :contentReference[oaicite:5]{index=5}

- **Tests**:
  - `leading_slash_paths_are_accepted`
  - `embedded_content_name_is_canonicalized`

---

### Fix 4: Fuzz-style robustness test for `parse_package_parts` (optional but recommended)

- **Addresses Finding**: 5 (No fuzz-style test dedicated to PackageParts)
- **Changes**:
  - Add a fuzz-style unit test analogous to `fuzz_style_never_panics` in `datamashup_framing.rs`, but targeting `parse_package_parts` directly. :contentReference[oaicite:6]{index=6}
  - For a deterministic range of seeds, generate short/medium random byte slices and call `parse_package_parts(&bytes)`, asserting that it never panics (it may return `Err(DataMashupError::FramingInvalid)` freely).
  - Keep this test fast by:
    - Capping maximum random length to something small (e.g., 128–512 bytes).
    - Avoiding any filesystem I/O inside the loop.
- **Tests**:
  - `parse_package_parts_never_panics_on_random_bytes`

---

### Fix 5: BOM handling for `Section1.m` (tests + behavior clarification)

- **Addresses Finding**: `[Observation] Potential UTF-8 BOM Handling`
- **Changes**:
  - The current parser uses `String::from_utf8` for `/Formulas/Section1.m`; if that text begins with an UTF-8 BOM (`0xEF 0xBB 0xBF`), the resulting `String` will begin with `\u{FEFF}`. Downstream, `parse_section_members` expects to see a `section` header at the start of a line and may not treat a leading BOM as ignorable whitespace. 

  To de-risk this:

  1. **Define desired behavior**  
     - Pick one of these equivalent behaviors for the M side and codify it in tests/docs:
       - **Preferred**: A single leading BOM codepoint, if present in `Section1.m`, is stripped before handing the string to M parsing (`parse_section_members`). This keeps the spec’s “don’t strip ordinary whitespace” promise, while treating BOM as an encoding artifact, not semantic content.
       - **Alternative**: Teach `parse_section_members` to tolerate a leading `\u{FEFF}` on the first line when looking for `section Section1;` (effectively treating BOM as ignorable).
     - Whichever you choose, document it briefly in the M-section parsing docs so future cycles know BOM is explicitly accounted for. :contentReference[oaicite:8]{index=8}

  2. **Implementation sketch (non-binding)**  
     - If you choose the “strip at boundary” approach:
       - In `parse_package_parts`, after reading `Formulas/Section1.m` into a `String`, strip a single leading `\u{FEFF}` if present:
         - e.g., `if let Some(stripped) = text.strip_prefix('\u{FEFF}') { text = stripped.to_string(); }`
       - Do **not** touch `PackageXml` handling; XML BOM/encoding is already handled at a different layer.
     - If you choose the “parser-tolerant” approach:
       - In `find_section_name` or `try_parse_section_header`, treat an initial BOM character on the first non-empty line as ignorable before checking for `"section"`.

  3. **Tests**  
     - Add a focused M-section unit test (string-only, no ZIP required):
       - Build a `SectionDocument` whose `source` starts with `'\u{FEFF'}` followed by a normal `section Section1;` header and a simple shared member.
       - Assert `parse_section_members(&source)` succeeds and returns the expected members.
     - Add an integration-style test at the PackageParts boundary:
       - Construct an in-memory ZIP where `/Formulas/Section1.m` is a BOM-prefixed UTF-8 file.
       - Call `parse_package_parts`, then pass `parts.main_section.source` into `parse_section_members`.
       - Assert that members are parsed correctly (no error on the section header).

- **Tests**:
  - `section_parsing_tolerates_utf8_bom`
  - `package_parts_section1_with_bom_parses_via_parse_section_members`

---

## Constraints

- **Performance & memory**:
  - New tests should be fast and self-contained:
    - Prefer synthetic in-memory ZIPs over filesystem writes wherever possible.
    - Keep fuzz-style tests bounded (small random sizes, fixed seed range) so they don’t dominate CI time.
- **API stability**:
  - Do **not** change the public type signatures for:
    - `PackageXml`, `SectionDocument`, `EmbeddedContent`, `PackageParts`, or `parse_package_parts`.
  - BOM handling and path normalization should be invisible to existing callers except where explicitly documented.
- **Behavioral contracts**:
  - The core contract must remain:
    - Required parts missing or invalid → `DataMashupError::FramingInvalid`.
    - Malformed `Content/*` entries → skipped, not fatal.
    - Arbitrary bytes → never panic.

---

## Expected Outcome

After applying this remediation:

- Error semantics for core PackageParts entries are enforced by tests instead of just code review, making regressions (e.g., accidentally treating missing Section1.m as success) much less likely.
- The “best-effort, skip on failure” guarantees for nested `Content/*` packages are explicitly covered, including corrupted and malformed embedded content.
- Leading-slash tolerance and `EmbeddedContent.name` normalization are both tested and documented, aligning implementation with the mini-spec.
- A fuzz-style test gives additional assurance that `parse_package_parts` is panic-free on arbitrary input, in line with the existing `parse_data_mashup` robustness guarantees.
- The BOM observation is resolved: `Section1.m` parsing is explicitly BOM-aware, so a leading UTF-8 BOM cannot silently break M section parsing in future milestones.

Together, these changes harden the PackageParts layer without altering its public API, and they close the gaps called out in the updated verification report. 
