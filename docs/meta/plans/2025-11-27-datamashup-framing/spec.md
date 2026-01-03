# Mini-spec: 2025-11-27-datamashup-framing

## 1. Scope

### 1.1 In scope

This cycle introduces a minimal but robust DataMashup “framing” layer and a host API for Excel workbooks:

* **New domain type**

  * `RawDataMashup` (final name may vary), representing the **top-level MS‑QDEFF framing**:

    * `version: u32`
    * `package_parts: Vec<u8>`
    * `permissions: Vec<u8>`
    * `metadata: Vec<u8>`
    * `permission_bindings: Vec<u8>`

    Each field corresponds to a single length-prefixed region in the binary stream, per the MS‑QDEFF layout described in the M/Query blueprint. 

* **New host API for Excel**

  * A public function on the `excel_diff` crate (module placement up to implementer), conceptually:

    ```text
    fn open_data_mashup(
        path: impl AsRef<std::path::Path>
    ) -> Result<Option<RawDataMashup>, ExcelOpenError>;
    ```

    * Only `.xlsx`/`.xlsm`/`.xlsb` Excel containers are in scope.
    * PBIX/PBIT support is explicitly out-of-scope for this cycle (Phase 3.5). 

* **Error modeling**

  * Extend `ExcelOpenError` with **DataMashup-specific variants**, such as (names are indicative):

    * `DataMashupBase64Invalid`
    * `DataMashupUnsupportedVersion { version: u32 }`
    * `DataMashupFramingInvalid`

  * Existing error variants and behaviors for basic container opening and grid parsing must remain unchanged.

* **Tests and fixtures**

  * A new integration test module (e.g. `core/tests/data_mashup_tests.rs`) using the existing `common::fixture_path` helper. 
  * New unit tests for the framing parser using synthetic `dm_bytes` in-memory.
  * Reuse existing fixtures:

    * `minimal.xlsx` (no Power Query, therefore no DataMashup).
    * `corrupt_base64.xlsx` (DataMashup bytes corrupted at the base64 level).
    * `m_change_literal_b.xlsx` (valid DataMashup, used later for M-diff milestones).

### 1.2 Out of scope (for this cycle)

* Parsing `package_parts` into `PackageParts` / OPC structures, or discovering `Section1.m` and metadata inside it. That belongs to “Semantic sections: PackageParts / Permissions / Metadata / Bindings” and the M parser milestones.
* Attaching a fully parsed `DataMashup` domain object to `Workbook` (e.g. `Workbook.mashup: Option<DataMashup>`). For now, `open_data_mashup` is a separate API.
* PBIX/PBIT host support (root `DataMashup` file). That is reserved for Phase 3.5 and will reuse the framing parser. 
* Any M AST parsing or semantic M diff behavior (Milestone 6 and beyond). This cycle stops at raw framing.

## 2. Behavioral Contract

### 2.1 `open_data_mashup(path)` (Excel host)

**Signature (conceptual)**

```text
fn open_data_mashup(
    path: impl AsRef<Path>
) -> Result<Option<RawDataMashup>, ExcelOpenError>;
```

**Behavior examples**

1. **Workbook with no Power Query / no `<DataMashup>`**

   * Example fixture: `minimal.xlsx` or any PG1-only workbook.
   * Expected result:

     * `Ok(None)`
     * No new error variants should be used; failure to find a DataMashup part is **not** an error.

2. **Workbook with a single, well-formed DataMashup**

   * Example fixture: `m_change_literal_b.xlsx` (generated from `templates/base_query.xlsx` via `mashup_inject`).
   * Expected result:

     * `Ok(Some(raw))` where:

       * `raw.version == 0`
       * All four Vec fields are present (possibly some are empty).
       * `raw.package_parts.len() + raw.permissions.len() + raw.metadata.len() + raw.permission_bindings.len() + 4*4 + 4 == dm_bytes.len()` (enforced by the framing parser).

3. **Workbook with corrupted DataMashup base64**

   * Fixture: `corrupt_base64.xlsx` (generated via `MashupCorruptGenerator` with `mode: byte_flip`).
   * Expected result:

     * `Err(ExcelOpenError::DataMashupBase64Invalid)` (or equivalent).
     * No panics or undefined behavior.
     * Underlying container errors (e.g., file missing, not a ZIP, missing `[Content_Types].xml`) must continue to surface via existing `ExcelOpenError` variants unchanged.

4. **Intermediate container failures**

   * If the file does not exist, `open_data_mashup` must return the same `ExcelOpenError::Io` behavior as `open_workbook` for nonexistent paths.
   * If the file is not a ZIP or not an Excel Open XML package, the existing `NotZipContainer` / `NotExcelOpenXml` errors apply.

### 2.2 `parse_data_mashup(bytes: &[u8])` (top-level framing)

This is an internal helper; tests will treat it as if it were `pub(crate)`.

**Input invariants**

* `bytes` is the raw, base64-decoded DataMashup stream as defined by MS‑QDEFF: `Version (4 bytes LE) + length-prefixed PackageParts + Permissions + Metadata + PermissionBindings`. 

**Expected behavior**

* If `bytes.len() < 4 + 4*4` (header + four length fields), return `Err(ExcelOpenError::DataMashupFramingInvalid)`; never panic.

* Read fields in order:

  1. `version: u32`
  2. `package_parts_len: u32`
  3. `permissions_len: u32`
  4. `metadata_len: u32`
  5. `permission_bindings_len: u32`

* If `version != 0`, return `Err(ExcelOpenError::DataMashupUnsupportedVersion { version })`.

* For each length, use checked arithmetic to compute slice bounds; if any slice would overflow the buffer or overlap incorrectly, return `DataMashupFramingInvalid`.

* After slicing all four segments, `offset` **must equal** `bytes.len()`; trailing garbage is treated as framing invalid rather than silently ignored.

* On success, return a `RawDataMashup` whose fields are copies (or owned slices) of the respective segments.

### 2.3 Error vs `None` semantics

* **`Ok(None)`**: Excel container is fine, but no DataMashup is present (no `<DataMashup>` element found in any `customXml/item*.xml`).
* **`Err(…)`**: any of the following:

  * Container/IO errors (existing ExcelOpenError variants).
  * Base64 decoding failure (`DataMashupBase64Invalid`).
  * Unsupported `version != 0`.
  * Lengths/offsets inconsistent with the buffer (`DataMashupFramingInvalid`).

There should be **no observable behavioral change** to existing callers of `open_workbook`; the new API is additive.

## 3. Constraints

* **Robustness**

  * All parsing of DataMashup bytes must be done via `Result`, with **no `unwrap`/`expect`** on untrusted data paths.
  * Invalid data must surface as structured `ExcelOpenError` values, not panics.

* **Performance**

  * DataMashup streams are typically small relative to full workbook size; for this cycle it is acceptable to allocate a single `Vec<u8>` for the full stream and copy out sections into separate Vecs.
  * No streaming or incremental parsing is required yet, but the design should not preclude it later (i.e., keep the framing logic separate from any deep parsing of PackageParts).

* **WASM & feature gating**

  * All new code that uses `std::fs::File` or `zip::ZipArchive` must live under the existing `excel-open-xml` feature gate so that `cargo check --target wasm32-unknown-unknown --no-default-features` continues to succeed.

* **API stability**

  * `open_workbook`’s signature and behavior must not change this cycle.
  * The new `open_data_mashup` API and `RawDataMashup` type are expected to remain stable through subsequent M milestones, even if their internal implementation is refactored.

## 4. Interfaces

### 4.1 New public interfaces

* `RawDataMashup` (name can be finalized by implementer)

  * Fields:

    ```text
    pub struct RawDataMashup {
        pub version: u32,
        pub package_parts: Vec<u8>,
        pub permissions: Vec<u8>,
        pub metadata: Vec<u8>,
        pub permission_bindings: Vec<u8>,
    }
    ```

* `open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, ExcelOpenError>`

  * Located either in `excel_open_xml` or a new `data_mashup` module and re-exported from `lib.rs`.

### 4.2 Extended public interfaces

* `ExcelOpenError` gains DataMashup-related variants; exact names may be:

  * `DataMashupBase64Invalid`
  * `DataMashupUnsupportedVersion { version: u32 }`
  * `DataMashupFramingInvalid`

These variants must implement `std::error::Error` like existing entries via `thiserror`.

### 4.3 Internal / allowed-to-change interfaces

* Helper functions:

  * `fn extract_datamashup_bytes_from_excel(archive: &mut ZipArchive<File>) -> Result<Option<Vec<u8>>, ExcelOpenError>`
  * `fn parse_data_mashup(bytes: &[u8]) -> Result<RawDataMashup, ExcelOpenError>`

These can be reshaped or relocated in later cycles so long as the public APIs in 4.1 and 4.2 remain stable.

## 5. Test Plan

All tests live under `core/` and are run via `cargo test --manifest-path core/Cargo.toml` as in the existing pipeline.

### 5.1 Unit tests: framing parser (`parse_data_mashup`)

1. **Minimal zero-length stream**

   * Construct `dm_bytes` in-memory with:

     * `version = 0`
     * All four lengths = 0
     * No payload bytes.

   * Expected:

     * `parse_data_mashup` returns `Ok(RawDataMashup { version: 0, all vecs empty })`.

2. **Basic non-zero lengths**

   * Build `dm_bytes` like:

     * `version = 0`
     * `package_parts_len = 4` with body `"AAAA"`
     * `permissions_len = 4` body `"BBBB"`
     * `metadata_len = 4` body `"CCCC"`
     * `permission_bindings_len = 4` body `"DDDD"`

   * Expected:

     * Each field in `RawDataMashup` equals the corresponding section.
     * No error.

3. **Unsupported version**

   * Same as (2) but with `version = 1`.

   * Expected:

     * `Err(ExcelOpenError::DataMashupUnsupportedVersion { version: 1 })`.

4. **Truncated stream / bounds errors**

   * Provide a buffer where one of the length fields exceeds the remaining bytes.

   * Expected:

     * `Err(ExcelOpenError::DataMashupFramingInvalid)`.
     * No panics, even under Miri or similar tools.

5. **Trailing bytes**

   * Provide a buffer where lengths sum to less than `bytes.len()` (extra garbage at end).

   * Expected:

     * `Err(ExcelOpenError::DataMashupFramingInvalid)`.

### 5.2 Integration tests: Excel host extraction (`open_data_mashup`)

Use a new test module `core/tests/data_mashup_tests.rs`:

All tests use:

```rust
use excel_diff::{ExcelOpenError, RawDataMashup, open_data_mashup};

mod common;
use common::fixture_path;
```

1. **Workbook without DataMashup**

   * Fixture: `minimal.xlsx`. 
   * Test:

     * Call `open_data_mashup(fixture_path("minimal.xlsx"))`.
     * Assert `Ok(None)`.

2. **Workbook with valid DataMashup**

   * Fixture: `m_change_literal_b.xlsx`.
   * Test:

     * Call `open_data_mashup(fixture_path("m_change_literal_b.xlsx"))`.
     * Assert `Ok(Some(raw))`.
     * Check:

       * `raw.version == 0`.
       * `raw.package_parts.len() > 0`.
       * `raw.metadata.len() > 0` (most real-world mashups will have non-empty metadata).
       * The sum-of-lengths invariant holds (optionally re-check within the test).

3. **Corrupt base64**

   * Fixture: `corrupt_base64.xlsx`.
   * Test:

     * Call `open_data_mashup(fixture_path("corrupt_base64.xlsx"))`.
     * Assert that result is `Err(ExcelOpenError::DataMashupBase64Invalid)` (using `matches!`).

4. **Nonexistent file**

   * Use a path that does not exist in fixtures (e.g. `fixture_path("missing_mashup.xlsx")`).

   * Test:

     * Ensure behavior matches existing IO error semantics (`ExcelOpenError::Io(e)` with `NotFound`).

5. **Non-Excel container**

   * Fixture: `random_zip.zip` (already used for container tests).
   * Test:

     * Calling `open_data_mashup` should yield the same `NotExcelOpenXml` behavior as `open_workbook`, not a DataMashup-specific error.

### 5.3 Negative tests: resilience and no-panic guarantee

* Add a small fuzz-style test that:

  * Generates a handful of random byte arrays (length up to some small N).
  * Calls `parse_data_mashup` on each.
  * Asserts it never panics and always returns either `Ok` or a well-typed `Err`.

This doesn’t replace proper fuzzing (later milestone) but locks in the no-panic expectation on arbitrary bytes.

### 5.4 No regressions to existing tests

* After implementation, all existing tests must still pass:

  * Addressing unit tests.
  * PG1 IR tests.
  * Container error tests.
  * Fixture path integration test.

No changes to existing test expectations are planned in this cycle.

---

This mini-spec, together with the decision record above, defines a small, well-bounded milestone: **Excel DataMashup host extraction + MS‑QDEFF framing** with clear APIs and tests. It advances the testing blueprint through Milestones 2 and 3 and unlocks the next cycles for PackageParts/metadata parsing and, eventually, semantic M diffing.
