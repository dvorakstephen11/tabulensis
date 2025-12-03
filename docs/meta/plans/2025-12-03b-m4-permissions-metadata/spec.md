# 2025-12-03b-m4-permissions-metadata – Mini-spec

Goal: advance **Milestone 4 – Semantic sections: PackageParts / Permissions / Metadata / Bindings** by adding a typed `DataMashup` IR, Permissions and Metadata parsers, and the associated tests, building directly on the existing framing (M3) and PackageParts (M4.1) work. 

This cycle does **not** implement query/metadata join (`Query` API) or M diff; it only establishes the semantic sections underneath those milestones.

---

## 1. Scope

### 1.1 Rust modules and types

**New module**

- `core/src/datamashup.rs`
  - Defines the **domain-level DataMashup IR** (Section 5.3 of the spec). 
  - Types:
    - `DataMashup`
    - `Permissions`
    - `Metadata`
    - `QueryMetadata` (metadata for a single `SectionName/FormulaName` entry; “query metadata”, not the full `Query` API from M5).
  - Functions:
    - `build_data_mashup(raw: &RawDataMashup) -> Result<DataMashup, DataMashupError>`
    - `parse_permissions(xml_bytes: &[u8]) -> Permissions`
    - `parse_metadata(metadata_bytes: &[u8]) -> Result<Metadata, DataMashupError>`

**Existing modules touched**

- `core/src/datamashup_framing.rs`
  - Reused `RawDataMashup` and `DataMashupError`; no structural changes expected. 
- `core/src/datamashup_package.rs`
  - Used by `build_data_mashup` to obtain `PackageParts` from `raw.package_parts`. 
- `core/src/m_section.rs`
  - Used by tests for cross-checking Metadata against parsed `Section1.m`. 
- `core/src/lib.rs`
  - Export new IR types and helpers:
    - `DataMashup`, `Permissions`, `Metadata`, `QueryMetadata`
    - `build_data_mashup`

### 1.2 Test code and fixtures

**New test file**

- `core/tests/m4_permissions_metadata_tests.rs`
  - Houses the Milestone 4 Permissions and Metadata tests:
    - `permissions_parsed_flags_default_vs_firewall_off`
    - `permissions_missing_or_malformed_yields_defaults`
    - `metadata_formulas_match_section_members`
    - `metadata_load_destinations_simple`
    - `metadata_groups_basic_hierarchy`
    - `metadata_hidden_queries_connection_only`
    - `permission_bindings_present_flag`
    - `permission_bindings_missing_ok` :contentReference[oaicite:21]{index=21}

**Existing test files extended**

- `core/tests/data_mashup_tests.rs`
  - Add a small smoke test `build_data_mashup_smoke_from_fixture` that:
    - Uses an existing mashup fixture (e.g., `one_query.xlsx`) via `open_data_mashup`.
    - Calls `build_data_mashup`.
    - Asserts basic invariants (version, non-empty `package_parts.main_section`, successful Permissions/Metadata parsing). 

**Fixtures (Python repo / manifest)**

The testing plan already defines fixtures for M4.2–4.4; this cycle assumes they exist or will be generated to these paths. :contentReference[oaicite:23]{index=23}

- `fixtures/generated/permissions_defaults.xlsx`
- `fixtures/generated/permissions_firewall_off.xlsx`
- `fixtures/generated/metadata_simple.xlsx`
- `fixtures/generated/metadata_query_groups.xlsx`
- `fixtures/generated/metadata_hidden_queries.xlsx`

Manifest IDs (used in `fixtures/manifest.yaml`):

- `permissions_defaults`
- `permissions_firewall_off`
- `metadata_simple`
- `metadata_query_groups`
- `metadata_hidden_queries` 

---

## 2. Behavioral Contract

### 2.1 DataMashup aggregation

`build_data_mashup(raw: &RawDataMashup) -> Result<DataMashup, DataMashupError>`

Given a valid `RawDataMashup` from `parse_data_mashup` / `open_data_mashup`, `build_data_mashup` constructs a fully-typed `DataMashup`:

```text
DataMashup {
    version: u32,                 // from raw.version
    package_parts: PackageParts,  // parsed from raw.package_parts
    permissions: Permissions,     // parsed from raw.permissions (with defaults)
    metadata: Metadata,           // parsed from raw.metadata (strict)
    permission_bindings_raw: Vec<u8>, // copy of raw.permission_bindings
}
```

#### Example: one-query workbook

For `fixtures/generated/one_query.xlsx` (already used in M4.1 tests):

* `open_data_mashup` → `RawDataMashup` with:

  * `version == 0`
  * non-empty `package_parts` slice containing `/Config/Package.xml` and `/Formulas/Section1.m`
* `build_data_mashup` must yield:

  * `data_mashup.version == 0`
  * `data_mashup.package_parts.main_section.source` is non-empty and contains a `section Section1;` header and at least one `shared` member.
  * `data_mashup.permissions` populated from the Permissions XML (or defaults if absent).
  * `data_mashup.metadata.formulas.len() >= 1` and includes a `QueryMetadata` entry whose `item_path` corresponds to the query (`Section1/Foo` in the testing plan).

Errors from `parse_package_parts` or `parse_metadata` propagate as `Err(DataMashupError::FramingInvalid | XmlError(_))`; permission parsing never fails the entire build (see below).

---

### 2.2 Permissions semantics

`Permissions` is a small struct reflecting three key values from the Permissions XML:

```text
Permissions {
    can_evaluate_future_packages: bool,
    firewall_enabled: bool,
    workbook_group_type: Option<String>, // raw string from XML, e.g. "None" / "Organizational"
}
```

Defaults:

```text
Permissions::default() = Permissions {
    can_evaluate_future_packages: false,
    firewall_enabled: true,
    workbook_group_type: None,
}
```

`parse_permissions(xml_bytes: &[u8]) -> Permissions` behaves as follows:

* If `xml_bytes` is empty: return `Permissions::default()`.
* If `xml_bytes` is non-empty but decoding or XML parsing fails:

  * Return `Permissions::default()`.
  * Do **not** propagate a `DataMashupError`; permission failure is non-fatal at this layer.
* If XML is valid:

  * `can_evaluate_future_packages` is `true` only if the corresponding element/attribute is present and `true` (string semantics up to implementer).
  * `firewall_enabled` reflects the `FirewallEnabled` field (default `true` if missing or malformed).
  * `workbook_group_type` is populated with the raw string, if present; unknown values are preserved.

#### Permissions fixtures

For `permissions_defaults.xlsx` vs `permissions_firewall_off.xlsx`: 

* `build_data_mashup` for `permissions_defaults.xlsx`:

  * `permissions.firewall_enabled == true`
  * `permissions.can_evaluate_future_packages == false`
  * `permissions.workbook_group_type.is_some()` (exact value left to tests, but consistent across both fixtures).

* `build_data_mashup` for `permissions_firewall_off.xlsx`:

  * `permissions.firewall_enabled == false`
  * `permissions.workbook_group_type` matches the defaults fixture (group type is a policy, not a firewall toggle).

If Permissions XML is corrupted (e.g., first few bytes garbled), `build_data_mashup` must still succeed and return `Permissions::default()`.

---

### 2.3 Metadata semantics

`Metadata` captures per-query metadata required for later milestones (M5 join, M6 diff) but is intentionally minimal in this cycle.

```text
Metadata {
    formulas: Vec<QueryMetadata>,
}

QueryMetadata {
    item_path: String,         // decoded "SectionName/FormulaName"
    section_name: String,      // "Section1"
    formula_name: String,      // "LoadToSheet"
    load_to_sheet: bool,       // query loads to a worksheet table
    load_to_model: bool,       // query loads to the data model
    is_connection_only: bool,  // true if not loaded anywhere
    group_path: Option<String> // e.g., "Inputs/DimTables"
}
```

Derived invariants:

* `item_path == format!("{section_name}/{formula_name}")` for successful parses.
* `is_connection_only == !(load_to_sheet || load_to_model)`.

`parse_metadata(metadata_bytes: &[u8]) -> Result<Metadata, DataMashupError>`:

* Treats `metadata_bytes` as UTF‑8 XML for this milestone (no separate “Metadata Content OPC” parsing).
* If bytes are empty → `Ok(Metadata { formulas: vec![] })`.
* If decoding or XML parsing fails → `Err(DataMashupError::XmlError(_))`.
* On success:

  * Locates the `LocalPackageMetadataFile` root and the `Formulas` collection described in the spec. 
  * For each per-query entry:

    * Extracts the URL-encoded `ItemPath` (`SectionName/FormulaName`) and decodes it.
    * Splits into `section_name` and `formula_name`.
    * Interprets load destination flags into `load_to_sheet` and `load_to_model`.
    * Determines `is_connection_only` from those flags.
    * Determines `group_path` from the group hierarchy in `AllFormulas` (e.g., `"Inputs/DimTables"`).

#### Metadata fixtures and behaviors

1. **`metadata_simple.xlsx`** – 2 queries: `Section1/LoadToSheet`, `Section1/LoadToModel`. 

   After `build_data_mashup`:

   * `metadata.formulas` contains exactly two entries where `section_name == "Section1"` and `!is_connection_only`.
   * The entry with `formula_name == "LoadToSheet"`:

     * `load_to_sheet == true`
     * `load_to_model == false`
     * `is_connection_only == false`
   * The entry with `formula_name == "LoadToModel"`:

     * `load_to_sheet == false`
     * `load_to_model == true`
     * `is_connection_only == false`

2. **`metadata_query_groups.xlsx`** – grouped queries. 

   * Each query’s `group_path` matches the folder structure created in the fixture (`"Inputs/DimTables"`, etc.).
   * Group paths are stable strings; no attempt is made to enforce a full tree structure this cycle.

3. **`metadata_hidden_queries.xlsx`** – connection-only queries. 

   * At least one `QueryMetadata` entry exists with:

     * `load_to_sheet == false`
     * `load_to_model == false`
     * `is_connection_only == true`

4. **`metadata_formulas_match_section_members` invariant**

   Using `metadata_simple.xlsx` (and any additional fixtures created for coverage):

   * Parse `PackageParts` and `Metadata`, then parse `Section1.m` into members.
   * Let:

     * `formula_count = metadata.formulas.iter().filter(|m| m.section_name == "Section1" && !m.is_connection_only).count()`
     * `member_count = parse_section_members(&package.main_section.source)?.len()`
   * Assert `formula_count == member_count` (or member_count minus any documented, deliberate exclusions like step-only entries; for this cycle, we assume 1:1).

---

### 2.4 Permission bindings

`DataMashup.permission_bindings_raw: Vec<u8>`

* The raw bytes are copied directly from `RawDataMashup.permission_bindings`.
* No interpretation, decryption, or hashing is performed in this milestone.

Behaviors:

* For a normal workbook (e.g., `permissions_defaults.xlsx` or `one_query.xlsx`):

  * `permission_bindings_raw.len() > 0`.
* For a synthetic `RawDataMashup` with `permission_bindings` set to an empty slice:

  * `build_data_mashup` returns `Ok(DataMashup { permission_bindings_raw.is_empty() == true, .. })`.
  * Permissions defaulting still applies as usual.

---

### 2.5 Non-goals for this cycle

Out of scope (explicitly **not** done in this branch):

* No `Query` struct or `Vec<Query>` domain API (that is Milestone 5).
* No query/metadata join logic (`metadata_join_simple`, URL-decoded key matching, etc.) – tests for those remain pending.
* No integration of `DataMashup` into `Workbook` yet; `Workbook` continues to expose only `sheets`, and query/mashup consumers call `open_data_mashup` + `build_data_mashup` directly.
* No M diff or `MQueryDiff` implementation (M6+).

---

## 3. Constraints and Invariants

### 3.1 Streaming and memory

* `parse_data_mashup` framing behavior remains unchanged and continues to be strict about version and length invariants.
* Permissions and Metadata sections are small by design; it is acceptable to allocate a `String` for the entire XML blob and parse it with `quick_xml`. No additional buffering beyond these strings and the resulting IR structures.
* `build_data_mashup` must not allocate on the order of the entire workbook; main cost is:

  * One ZIP open for `PackageParts` (already present from M4.1).
  * One XML parse for Permissions.
  * One XML parse for Metadata.

### 3.2 Error handling

* `parse_data_mashup` invariants and error mapping remain as-is (no panics on malformed M3 framing).
* `parse_package_parts`’s existing behavior and `DataMashupError` mapping remain unchanged; new code only *consumes* it.
* Permissions:

  * Never cause `build_data_mashup` to return `Err`.
  * Malformed or missing XML → `Permissions::default()`.
* Metadata:

  * Malformed XML is considered a hard error: `parse_metadata` returns `Err(DataMashupError::XmlError(_))`, and `build_data_mashup` surfaces that error.
* No new panics; all malformed inputs must hit an explicit error or default path.

### 3.3 IR invariants

* `DataMashup.version == raw.version`.
* `DataMashup.package_parts.main_section.source` remains unchanged from `parse_package_parts` (BOM removal already handled there; do not strip further).
* For every `QueryMetadata`:

  * `item_path == section_name + "/" + formula_name`.
  * `is_connection_only == !(load_to_sheet || load_to_model)`.
* `Metadata.formulas` may contain zero or more entries; uniqueness of `item_path` is **not** enforced or required in this milestone (M5 will add invariants and tests for orphans/duplicates).

---

## 4. Interfaces

### 4.1 Public API changes

In `core/src/lib.rs`:

* New module export and re-exports:

  ```rust
  pub mod datamashup;

  pub use datamashup::{
      DataMashup,
      Permissions,
      Metadata,
      QueryMetadata,
      build_data_mashup,
  };
  ```

* `RawDataMashup`, `parse_data_mashup`, `open_data_mashup`, `PackageParts`, and `SectionMember` remain exported as they are today.

### 4.2 Compatibility expectations

* Existing APIs used by grid diff, JSON output, and snapshot tests (`diff_workbooks`, `diff_workbooks_to_json`, `snapshot` helpers) are unchanged and must continue to pass all current tests.
* The new `DataMashup` IR is considered part of the long-term public surface; future milestones may **add** fields to `Metadata` / `QueryMetadata` but should avoid breaking or renaming existing fields without an explicit migration plan.

---

## 5. Test Plan

This cycle is tied to **Milestone 4** in the testing plan, specifically sections 4.2, 4.3, and 4.4.

### 5.1 New tests (core/tests/m4_permissions_metadata_tests.rs)

1. **`permissions_parsed_flags_default_vs_firewall_off`**

   * For each of `permissions_defaults.xlsx` and `permissions_firewall_off.xlsx`:

     * Use `fixture_path` helper and `open_data_mashup` to obtain `RawDataMashup`.
     * Call `build_data_mashup`.
   * Assertions:

     * Both results have `version == 0`.
     * Defaults fixture:

       * `permissions.firewall_enabled == true`.
       * `permissions.can_evaluate_future_packages == false`.
     * Firewall-off fixture:

       * `permissions.firewall_enabled == false`.
       * `permissions.workbook_group_type` is equal between fixtures.

2. **`permissions_missing_or_malformed_yields_defaults`**

   * Construct a synthetic `RawDataMashup` with:

     * Valid `package_parts` slice from an existing fixture (e.g., `one_query.xlsx`).
     * `permissions` set to:

       * `Vec::new()` in one subtest.
       * Clearly invalid bytes (e.g., `b"<not-xml"` truncated) in another.
     * Valid `metadata` (can be empty).
     * Empty `permission_bindings`.
   * Call `build_data_mashup` for each.
   * Assertions:

     * `Ok(dm)` returned.
     * `dm.permissions == Permissions::default()`.

3. **`metadata_formulas_match_section_members`**

   * Fixture: `metadata_simple.xlsx`.
   * Steps:

     * `RawDataMashup` via `open_data_mashup`.
     * `PackageParts` via `parse_package_parts(&raw.package_parts)`.
     * `Metadata` via `parse_metadata(&raw.metadata)`.
     * Members via `parse_section_members(&package.main_section.source)`.
   * Assertions:

     * Count of non-connection-only formulas in `Metadata` where `section_name == "Section1"` equals the number of `SectionMember`s returned by `parse_section_members`.
     * Every such `QueryMetadata` has `section_name == "Section1"` and non-empty `formula_name`.

4. **`metadata_load_destinations_simple`**

   * Fixture: `metadata_simple.xlsx`.
   * Steps:

     * `DataMashup` via `build_data_mashup`.
     * Find `QueryMetadata` entries for `"Section1/LoadToSheet"` and `"Section1/LoadToModel"`.
   * Assertions:

     * `LoadToSheet`:

       * `load_to_sheet == true`
       * `load_to_model == false`
       * `is_connection_only == false`
     * `LoadToModel`:

       * `load_to_sheet == false`
       * `load_to_model == true`
       * `is_connection_only == false`

5. **`metadata_groups_basic_hierarchy`**

   * Fixture: `metadata_query_groups.xlsx`.
   * Steps:

     * `DataMashup` via `build_data_mashup`.
     * Pick at least one query from a nested group (e.g., `"Inputs/DimTables/Foo"` depending on fixture).
   * Assertions:

     * `QueryMetadata.group_path` matches the expected group path string (e.g., `"Inputs/DimTables"`).
     * Queries in the root group have `group_path == None` or an explicitly empty group, depending on design choice encoded in the test.

6. **`metadata_hidden_queries_connection_only`**

   * Fixture: `metadata_hidden_queries.xlsx`.
   * Steps:

     * `DataMashup` via `build_data_mashup`.
     * Find at least one `QueryMetadata` with both load flags false.
   * Assertions:

     * `load_to_sheet == false`
     * `load_to_model == false`
     * `is_connection_only == true`

7. **`permission_bindings_present_flag`**

   * Fixture: `permissions_defaults.xlsx` (or `one_query.xlsx`).
   * Steps:

     * `DataMashup` via `build_data_mashup`.
   * Assertion:

     * `!dm.permission_bindings_raw.is_empty()`.

8. **`permission_bindings_missing_ok`**

   * Synthetic `RawDataMashup` with valid framing, non-empty `package_parts`, valid `permissions` and `metadata`, but `permission_bindings` set to an empty vector.
   * Assertion:

     * `build_data_mashup` returns `Ok(dm)` and `dm.permission_bindings_raw.is_empty()`.

### 5.2 Extended tests (core/tests/data_mashup_tests.rs)

Add a small integration test:

* **`build_data_mashup_smoke_from_fixture`**

  * Fixture: a simple Power Query workbook (e.g., `one_query.xlsx`).
  * Steps:

    * Use existing helpers to open workbook and extract `RawDataMashup`.
    * Call `build_data_mashup`.
  * Assertions:

    * `dm.version == 0`.
    * `dm.package_parts.main_section.source` contains `"section Section1;"`.
    * `dm.metadata.formulas.len() >= 1` (basic sanity).
    * No panic or error even when run on fuzz-style DataMashup bytes (optional: for random bytes, we expect `build_data_mashup` to error cleanly, not panic).

### 5.3 Fixture and manifest updates

In the Python fixtures repo / manifest (outside this Rust crate):

* Ensure scenario entries exist for:

  * `permissions_defaults`
  * `permissions_firewall_off`
  * `metadata_simple`
  * `metadata_query_groups`
  * `metadata_hidden_queries`
* Confirm they write to `fixtures/generated/*.xlsx` filenames as referenced in tests.

### 5.4 Milestone linkage

This mini-spec advances the following testing-plan items:

* **Milestone 4** – Semantic sections:

  * 4.2 `permissions_parsed_flags`
  * 4.3 `metadata_formulas_match_section_members`, `metadata_load_destinations`, `metadata_groups`
  * 4.4 `permission_bindings_present_flag`, `permission_bindings_missing_ok`

Milestone 5 (Query/metadata join) and Milestone 6 (textual M diff) remain open and will be explicit targets of later cycles once this semantic layer is in place.

