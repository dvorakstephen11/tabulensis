# 2025-12-03-m4-packageparts – DataMashup PackageParts semantics (Milestone 4.1)

This cycle progresses **Milestone 4 – Semantic sections: PackageParts / Permissions / Metadata / Bindings** by implementing the **PackageParts semantic parser** and associated tests:

* Interpret the `package_parts` slice of `RawDataMashup` as a ZIP/OPC package.
* Extract `/Config/Package.xml` and `/Formulas/Section1.m`.
* Discover embedded mini‑mashups under `/Content/{GUID}` and read their own `/Formulas/Section1.m`.

It deliberately **does not** yet parse Permissions or Metadata XML, nor does it wire a full `DataMashup` domain struct into `Workbook` IR; those remain for later Milestone 4/5 cycles.

---

## 1. Scope

### 1.1 Modules and types

**Rust crate:** `core` (library `excel_diff`)

New module:

* `core/src/datamashup_package.rs`

  * Semantic view over the `package_parts` slice from `RawDataMashup`.
  * Responsible for opening the inner OPC/ZIP, extracting key parts, and representing them as strongly typed structs.

New public types and function:

```rust
// core/src/datamashup_package.rs

use crate::datamashup_framing::DataMashupError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageXml {
    pub raw_xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDocument {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedContent {
    /// Normalized path of the embedded package within PackageParts (e.g. "Content/{GUID}.package").
    pub name: String,
    /// The embedded package's /Formulas/Section1.m content.
    pub section: SectionDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageParts {
    pub package_xml: PackageXml,
    pub main_section: SectionDocument,
    pub embedded_contents: Vec<EmbeddedContent>,
}

/// Parse the `PackageParts` OPC/ZIP from the given byte slice.
///
/// `bytes` should be exactly the `package_parts` slice from `RawDataMashup`.
pub fn parse_package_parts(bytes: &[u8]) -> Result<PackageParts, DataMashupError>;
```

Exports from `core/src/lib.rs`:

```rust
pub mod datamashup_package;

pub use datamashup_package::{
    EmbeddedContent,
    PackageParts,
    PackageXml,
    SectionDocument,
    parse_package_parts,
};
```

These APIs sit **between** the binary framing (`RawDataMashup`) and higher‑level M/metadata semantics. They do not depend on workbook/grid IR or the diff engine.

### 1.2 Tests

New Rust test module:

* `core/tests/m4_package_parts_tests.rs`

This module uses real `.xlsx` fixtures and the `open_data_mashup` API (gated behind the `excel-open-xml` feature) to exercise `parse_package_parts` end‑to‑end.

Planned tests:

* `package_parts_contains_expected_entries`
* `embedded_content_detection`
* `parse_package_parts_rejects_non_zip`

See §5 for details.

### 1.3 Fixtures and Python generators

We extend the **fixtures manifest** to add two new Excel scenarios generated via `fixtures/src/generators/mashup.py`:

* `one_query.xlsx`

  * Single query in `Section1` (no embedded contents).
  * Used by `package_parts_contains_expected_entries`.

* `multi_query_with_embedded.xlsx`

  * At least one regular query and at least one query using `Embedded.Value`, so that PackageParts contains one or more `/Content/{GUID}` embedded packages.
  * Used by `embedded_content_detection`.

Manifest sketch (conceptual; exact schema follows the existing manifest):

```yaml
- id: m4_packageparts_one_query
  kind: excel_single
  path: "fixtures/generated/one_query.xlsx"
  generator: "mashup:one_query"

- id: m4_packageparts_multi_embedded
  kind: excel_single
  path: "fixtures/generated/multi_query_with_embedded.xlsx"
  generator: "mashup:multi_query_with_embedded"
```

The Python side is responsible for:

* Creating workbooks from `templates/base_query.xlsx`.
* Injecting DataMashup with appropriate PackageParts layout (`/Config/Package.xml`, `/Formulas/Section1.m`, `/Content/{GUID}` nested OPCs).

### 1.4 Out of scope

Explicitly **not** in this cycle:

* Parsing of **Permissions XML** (`CanEvaluateFuturePackages`, `FirewallEnabled`, `WorkbookGroupType`). 
* Parsing of **Metadata XML** and `QueryMetadata` (`Formulas` entries, load destinations, groups).
* Creation of a full `DataMashup` domain struct or wiring it into `Workbook { mashup: Option<DataMashup> }`.
* M query domain objects (`Query`, `MStep`, AST) or textual M diff operations.
* Any changes to grid IR, DiffOps, or the diff engine behavior.

---

## 2. Behavioral Contract

### 2.1 Input

`parse_package_parts(bytes: &[u8])` expects:

* `bytes` to be the **exact** `package_parts` slice from a successfully parsed `RawDataMashup` (MS‑QDEFF `PackageParts` payload).
* The content to be a ZIP/OPC package with at least:

  * A part corresponding to `/Config/Package.xml`.
  * A part corresponding to `/Formulas/Section1.m`.
  * Zero or more `/Content/{GUID}` embedded packages, each itself a ZIP/OPC with its own `/Formulas/Section1.m`.

The function **must not** perform base64 or MS‑QDEFF framing; callers are responsible for:

```rust
let raw = open_data_mashup(path)?
    .expect("fixture should contain a mashup");
let parts = parse_package_parts(&raw.package_parts)?;
```

### 2.2 PackageXml semantics

When parsing succeeds:

* `package_xml.raw_xml` is the UTF‑8 text of `/Config/Package.xml` in the PackageParts ZIP.

Behavior:

* Treat entry names without a leading `/` as canonical (e.g. `"Config/Package.xml"`). The parser should accept both `"Config/Package.xml"` and `"/Config/Package.xml"` if present; if both somehow exist, first found wins.
* Decode as UTF‑8. If decoding fails, return `Err(DataMashupError::FramingInvalid)` for this cycle rather than attempting lossy recovery. Later cycles may introduce a more precise error.
* The parser does **not** interpret XML fields yet (client version, culture, etc.); tests only assert that the XML is present and non‑empty.

Example (conceptual):

```rust
let parts = parse_package_parts(&raw.package_parts).unwrap();
assert!(parts.package_xml.raw_xml.contains("<Package"));
```

### 2.3 Main SectionDocument semantics

* `main_section.source` is the UTF‑8 text content of `/Formulas/Section1.m` in the PackageParts ZIP.

Rules:

* Same path handling as for PackageXml: accept `"Formulas/Section1.m"` (with or without leading `/`).
* Decode as UTF‑8. On decoding failure or missing entry, return `Err(DataMashupError::FramingInvalid)`.
* Strip a single leading UTF-8 BOM (`\u{FEFF}`) when present so downstream M parsing sees canonical text.
* Do not strip leading/trailing whitespace beyond normal UTF‑8 decoding. The caller (e.g. `parse_section_members`) is responsible for any higher‑level normalization.
* Preserve newlines and internal spacing exactly as stored in the package.

Example:

```rust
let src = &parts.main_section.source;
assert!(src.contains("section Section1;"));
assert!(src.contains("shared"));
```

### 2.4 EmbeddedContent semantics

For embedded content:

* Inspect all file entries whose names start with `"Content/"` (directory semantics as per the ZIP library).

For each such entry:

1. Read its bytes into memory.

2. Treat them as another ZIP/OPC package.

3. Look for `"Formulas/Section1.m"` inside this nested package.

4. If found and decodable as UTF‑8 (stripping a single leading BOM if present), create an `EmbeddedContent`:

   ```rust
   EmbeddedContent {
       name: normalize_path(outer_entry_name), // e.g., "Content/{GUID}.package" (never starts with '/')
       section: SectionDocument { source }, // nested Section1.m text, BOM removed if present
   }
   ```

   `EmbeddedContent.name` is always stored without a leading `/`, even if the raw ZIP entry included one.

5. If nested ZIP parsing fails or `Formulas/Section1.m` is missing/invalid, **silently skip that embedded content** for this cycle (keep parsing best‑effort), rather than failing the entire call.

The resulting `PackageParts` must satisfy:

* `embedded_contents.len() == 0` for workbooks without `Embedded.Value`.
* `embedded_contents.len() >= 1` for `multi_query_with_embedded.xlsx` fixtures.

### 2.5 Error handling

`parse_package_parts` returns `Result<PackageParts, DataMashupError>` with these semantics:

* Non‑ZIP data (e.g. random bytes) ➜ `Err(DataMashupError::FramingInvalid)`.
* ZIP that is structurally valid but missing **either** `/Config/Package.xml` or `/Formulas/Section1.m` ➜ `Err(DataMashupError::FramingInvalid)`.
* I/O or ZIP errors from the inner `zip` crate ➜ mapped to `DataMashupError::FramingInvalid` (no new variants in this cycle).
* XML or UTF‑8 decoding errors for `/Config/Package.xml` ➜ `Err(DataMashupError::FramingInvalid)`.

Parsing must **never panic** on arbitrary byte slices, including fuzzed data. This aligns with the robustness expectations already set for `parse_data_mashup`.

---

## 3. Constraints

### 3.1 Complexity and performance

* Complexity is expected to be **O(N)** in `bytes.len()`:

  * Single pass over the outer ZIP entries to find the three relevant regions: `Config/Package.xml`, `Formulas/Section1.m`, and `Content/*`.
  * For each `Content/*` entry, a second pass over the nested ZIP entries.

* Avoid reading unnecessary files:

  * Do not decompress unrelated entries (e.g., `/Config/Formulas.xml` inside embedded packages) in this cycle.

* Memory usage:

  * Reading `/Config/Package.xml`, `/Formulas/Section1.m`, and nested `Section1.m` into `String` is acceptable; these are small compared to grid data.

No perf‑critical tuning is required yet; this path is executed once per workbook and is cheap relative to grid diff.

### 3.2 Robustness

* The parser must tolerate:

  * Extra entries in the outer PackageParts ZIP (it only cares about the three expected paths).
  * Embedded contents that are not well‑formed nested packages (skip, don’t crash).
  * Empty `Content/` directories.

* All failures must be expressed as `DataMashupError`, not panics.

### 3.3 Forward compatibility

This cycle intentionally keeps PackageParts semantics **minimal**:

* `PackageXml` is just `raw_xml: String`; later cycles may add parsed fields without breaking existing code.
* `SectionDocument` is just `source: String`; higher‑level M parsing (AST, steps) will build atop it.
* `EmbeddedContent` only exposes `name` and `section`; later we can add `package_xml` or other metadata if needed.

No `DataMashup` domain struct is introduced yet; future cycles can safely wrap `RawDataMashup` plus `PackageParts`, Permissions, and Metadata into that type.

---

## 4. Interfaces

### 4.1 Public API for this cycle

As exported from `lib.rs`:

```rust
pub struct PackageXml {
    pub raw_xml: String,
}

pub struct SectionDocument {
    pub source: String,
}

pub struct EmbeddedContent {
    pub name: String,
    pub section: SectionDocument,
}

pub struct PackageParts {
    pub package_xml: PackageXml,
    pub main_section: SectionDocument,
    pub embedded_contents: Vec<EmbeddedContent>,
}

pub fn parse_package_parts(bytes: &[u8]) -> Result<PackageParts, DataMashupError>;
```

Contract notes:

* `PackageParts` is a simple POD type that later `DataMashup` and `Query` domain structs can embed directly.
* `parse_package_parts` is **pure** and performs no I/O; it operates solely on the `bytes` slice passed in.
* Callers must obtain `bytes` from a successfully parsed `RawDataMashup` or equivalent.

### 4.2 Interfaces that must remain stable

For this cycle, **no existing public APIs are allowed to change**:

* `RawDataMashup`, `DataMashupError`, `parse_data_mashup`, and `open_data_mashup` stay as‑is.
* `Workbook`, `Sheet`, `Grid`, `Cell`, `DiffOp`, `DiffReport`, and all grid diff behavior remain unchanged.
* JSON output and `DiffReport` schema remain untouched.

New APIs (`PackageParts`, `PackageXml`, `SectionDocument`, `EmbeddedContent`, `parse_package_parts`) must be designed so they can be reused by a future `DataMashup` struct without breaking callers.

---

## 5. Test Plan

This cycle is explicitly tied to **Milestone 4.1 – PackageParts / OPC tests** in the testing plan.

### 5.1 New tests

File: `core/tests/m4_package_parts_tests.rs`

#### 5.1.1 `package_parts_contains_expected_entries`

**Fixture:**

* `fixtures/generated/one_query.xlsx`

  * Single query in `Section1`, no `Embedded.Value`.

**Test sketch:**

```rust
use excel_diff::{
    DataMashupError,
    ExcelOpenError,
    PackageParts,
    parse_package_parts,
    open_data_mashup,
};

mod common;
use common::fixture_path;

#[test]
fn package_parts_contains_expected_entries() {
    let path = fixture_path("one_query.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts)
        .expect("PackageParts should parse");

    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(
        parts.main_section.source.contains("section Section1;"),
        "main Section1.m should be present"
    );
    assert!(
        parts.main_section.source.contains("shared"),
        "at least one shared query should be present"
    );
    assert!(
        parts.embedded_contents.is_empty(),
        "one_query.xlsx should not contain embedded contents"
    );
}
```

This codifies the **happy path** for a simple PackageParts ZIP: both main parts present, no embedded content.

#### 5.1.2 `embedded_content_detection`

**Fixture:**

* `fixtures/generated/multi_query_with_embedded.xlsx`

  * At least one regular query and one `Embedded.Value` query, so PackageParts contains at least one `/Content/{GUID}` nested package.

**Test sketch:**

```rust
#[test]
fn embedded_content_detection() {
    let path = fixture_path("multi_query_with_embedded.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts)
        .expect("PackageParts should parse");

    assert!(
        !parts.embedded_contents.is_empty(),
        "multi_query_with_embedded.xlsx should expose at least one embedded content"
    );

    for embedded in &parts.embedded_contents {
        assert!(
            embedded.section.source.contains("section Section1"),
            "embedded Section1.m should be present for {}",
            embedded.name
        );
        assert!(
            embedded.section.source.contains("shared"),
            "embedded Section1.m should contain at least one shared member for {}",
            embedded.name
        );
    }
}
```

This test ensures:

* `/Content/{GUID}` packages are detected.
* Nested `/Formulas/Section1.m` is opened and exposed via `EmbeddedContent.section`.

#### 5.1.3 `parse_package_parts_rejects_non_zip`

**No fixture required** – uses synthetic bytes.

**Test sketch:**

```rust
#[test]
fn parse_package_parts_rejects_non_zip() {
    let bogus = b"this is not a zip file";
    let err = parse_package_parts(bogus).expect_err("non-zip bytes should fail");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}
```

This locks in the contract that `parse_package_parts` fails gracefully (and does not panic) on arbitrary non‑ZIP input, matching the robustness expectations for `parse_data_mashup`.

### 5.2 Existing tests to keep unchanged

* All DataMashup framing tests in `core/tests/data_mashup_tests.rs` remain as‑is; they continue to verify MS‑QDEFF framing and base64/UTF‑16 handling. 
* All PG1–PG6 grid IR and diff tests remain unchanged and serve as regression nets for other subsystems.

### 5.3 Future tests enabled by this work (not in this cycle)

This cycle prepares the ground for later Milestone 4 and 5 tests:

* `permissions_parsed_flags` – build on `RawDataMashup.permissions` to expose privacy/firewall flags.
* `metadata_formulas_match_section_members` – combine `PackageParts.main_section` (via `parse_section_members`) with Metadata XML `Formulas` entries.
* `metadata_join_simple` / `metadata_join_url_encoding` – map `SectionName/FormulaName` to queries defined in Section1, using `PackageParts` as the source of text.
* `query_names_unique` and `metadata_orphan_entries` – operate on a `Vec<Query>` built from Section members + Metadata, with `PackageParts` providing the Section1 document.

Those tests remain explicitly **out of scope** for this cycle; they are listed here to show how PackageParts semantics advance the M‑side roadmap toward textual M diff (Milestone 6).
