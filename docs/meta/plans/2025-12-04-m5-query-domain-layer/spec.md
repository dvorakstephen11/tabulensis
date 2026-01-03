# 2025-12-04-m5-query-domain-layer – Domain `Query` Model & Metadata Join

## 1. Scope

### 1.1 In-scope modules and types

Rust:

- `core/src/datamashup.rs`
  - Add domain-level `Query` type.
  - Add a helper to build `Vec<Query>` from an existing `DataMashup`.
- `core/src/lib.rs`
  - Re-export the new `Query` type and builder function.
- `core/src/m_section.rs` (read-only dependency)
  - `SectionMember` and `parse_section_members` are used to split Section1.m. 
- Tests:
  - New test module: `core/tests/m5_query_domain_tests.rs` (name can be adjusted, but content must reflect M5.2–5.3).
  - Existing tests remain intact:
    - `m4_permissions_metadata_tests.rs` (Metadata and Permissions behavior). :contentReference[oaicite:15]{index=15}
    - `m_section_splitting_tests.rs` (Section1 splitting). :contentReference[oaicite:16]{index=16}
    - `data_mashup_tests.rs` for framing/package sanity. 

Python fixtures:

- `fixtures/src/generators/mashup.py`
  - Extend or add generators to produce new M5-specific Excel fixtures.
- `fixtures/manifest.yaml`
  - Add entries for new fixtures used by M5 tests. :contentReference[oaicite:18]{index=18}

### 1.2 Out of scope

- No changes to:
  - Grid diff algorithms (`engine.rs`, `diff_grids`, unified grid diff implementation).
  - Diff IR types (`DiffOp`, `DiffReport`) beyond possible future use of `Query` (none in this cycle).
  - CLI / JSON output API.
- No M AST parsing or semantic diff:
  - Milestone 7 (semantic M diff) and H3/H4 work items remain out of scope. 
- No query-level diffing:
  - Milestone 6 (MQueryDiff, query alignment, Myers text diff) is **not** implemented here, but this spec must make that work straightforward in the next cycle. :contentReference[oaicite:20]{index=20}

---

## 2. Behavioral Contract

### 2.1 Domain `Query` type

Introduce a domain type representing a single Power Query as seen in the DataMashup:

```rust
pub struct Query {
    pub name: String,           // "Section1/Foo"
    pub section_member: String, // "Foo"
    pub expression_m: String,   // "let ... in ..."
    pub metadata: QueryMetadata,
}
````

* `name`:

  * Always of the form `"{SectionName}/{MemberName}"`.
  * Derived from `SectionMember.section_name` and `SectionMember.member_name`.
* `section_member`:

  * Just the member name (no `Section1/` prefix).
  * Exact identifier from the `shared` declaration in Section1.m.
* `expression_m`:

  * The exact expression body for this query, as returned by `parse_section_members`:

    * Whitespace trimmed.
    * Section header removed.
    * Without the trailing `;`.
* `metadata`:

  * The joined `QueryMetadata` entry whose `section_name` and `formula_name` match this query.

### 2.2 Query builder behavior

Add a helper on the DataMashup domain layer:

```rust
pub fn build_queries(dm: &DataMashup) -> Result<Vec<Query>, SectionParseError>;
```

Informal semantics:

1. **Split Section1.m into members**

   * Use `dm.package_parts.main_section.source` and `parse_section_members` to get a `Vec<SectionMember>`.
   * Only `shared` members are returned by `parse_section_members` today; private members are ignored and should remain ignored for this milestone.

2. **Index Metadata formulas**

   * For each `QueryMetadata` in `dm.metadata.formulas`, a mapping is already available:

     * `item_path` (e.g. `"Section1/Foo Bar"`).
     * `section_name` (e.g. `"Section1"`).
     * `formula_name` (e.g. `"Foo Bar"`).
   * `parse_metadata` has already decoded percent-encoded `ItemPath` into Unicode and split it into `section_name` and `formula_name`.

3. **Join members to metadata**

   * For each `SectionMember`:

     * Compose `name = format!("{}/{}", section_name, member_name)`.
     * Look up a `QueryMetadata` where:

       * `section_name == member.section_name` AND
       * `formula_name == member.member_name`.
     * If found:

       * Emit a `Query { name, section_member: member_name, expression_m, metadata: cloned_metadata }`.
     * If not found:

       * Still emit a `Query` with **synthetic metadata**:

         * `item_path = "{SectionName}/{MemberName}"`
         * `load_to_sheet = false`, `load_to_model = false`
         * `is_connection_only = true`
         * `group_path = None`

4. **Ordering**

   * The returned `Vec<Query>` is ordered in the same order as the `SectionMember` list produced by `parse_section_members` (i.e., the order of shared members in Section1.m).
   * This order will be the default order for:

     * M diff alignment when existing and new queries share names.
     * UI/CLI enumeration of queries.

5. **Domain invariants**

   * `query_names_unique`:

     * For valid workbooks, all `Query.name` must be unique within a `DataMashup`.
     * Implementation strategy:

       * Enforce in code with a `debug_assert!` over a temporary `HashSet<String>` as queries are built.
       * For release builds, last writer wins deterministically if duplicates exist; tests only cover the unique case.

   * `metadata_orphan_entries`:

     * Metadata entries that have no corresponding `SectionMember` (e.g. `Section1/Nonexistent`) **do not** cause errors or panics.
     * This cycle’s behavior:

       * Orphans remain accessible via `dm.metadata.formulas` but are not surfaced as `Query` values.
       * Tests codify that `build_queries` returns only queries backed by Section1 members.

### 2.3 Examples

1. **Simple two-query workbook**

   Fixture: `metadata_simple.xlsx`, with queries:

   * `Section1/LoadToSheet` → loads to table on sheet.
   * `Section1/LoadToModel` → loads only to the data model.

   Behavior:

   ```rust
   let dm = load_datamashup("metadata_simple.xlsx");
   let queries = build_queries(&dm).unwrap();

   // Ordering and naming
   assert_eq!(queries.len(), 2);
   assert_eq!(queries[0].name, "Section1/LoadToSheet");
   assert_eq!(queries[0].section_member, "LoadToSheet");
   assert_eq!(queries[1].name, "Section1/LoadToModel");
   assert_eq!(queries[1].section_member, "LoadToModel");

   // Join to metadata
   let sheet = &queries[0];
   assert!(sheet.metadata.load_to_sheet);
   assert!(!sheet.metadata.load_to_model);

   let model = &queries[1];
   assert!(!model.metadata.load_to_sheet);
   assert!(model.metadata.load_to_model);
   ```

2. **URL-encoded query names**

   Fixture: `metadata_url_encoding.xlsx` (new):

   * Section1.m contains `shared "Query with space & #" = ...;`
   * Metadata `ItemPath` is stored percent-encoded: `Section1/Query%20with%20space%20%26%20%23`.

   Behavior:

   ```rust
   let dm = load_datamashup("metadata_url_encoding.xlsx");
   let queries = build_queries(&dm).unwrap();

   assert_eq!(queries.len(), 1);
   let q = &queries[0];
   assert_eq!(q.name, "Section1/Query with space & #");
   assert_eq!(q.section_member, "Query with space & #");
   assert!(q.metadata.load_to_sheet || q.metadata.load_to_model); // as configured in fixture
   ```

3. **Metadata orphan entry**

   Fixture: `metadata_orphan_entries.xlsx` (new):

   * Section1.m defines one shared member: `shared Foo = 1;`
   * Metadata formulas contain:

     * `Section1/Foo`
     * `Section1/Nonexistent` (manually edited into XML)

   Behavior:

   ```rust
   let dm = load_datamashup("metadata_orphan_entries.xlsx");
   let queries = build_queries(&dm).unwrap();

   // Only the real query appears
   assert_eq!(queries.len(), 1);
   assert_eq!(queries[0].name, "Section1/Foo");

   // Orphan metadata still present in the raw metadata list
   let has_orphan = dm
       .metadata
       .formulas
       .iter()
       .any(|m| m.item_path == "Section1/Nonexistent");
   assert!(has_orphan);
   ```

---

## 3. Constraints and Invariants

### 3.1 Performance and memory

* The builder is **O(Q)** where `Q` is the number of queries:

  * One pass over Section1 members.
  * One pass to index Metadata formulas (e.g. building a `HashMap<(String,String), &QueryMetadata>`).
* Allocations:

  * Cloning of `QueryMetadata` per `Query`.
  * Cloning of `expression_m` and identifier strings from `SectionMember`.
* Section1.m text is already fully loaded by `parse_package_parts`; this cycle does not change its memory footprint.

### 3.2 Error handling

* `build_queries` can fail only due to `SectionParseError` coming from `parse_section_members`:

  * Missing/invalid section header.
  * Invalid member syntax (a line beginning with `shared` that lacks an identifier, `=`, `;`, or a complete expression results in `Err(SectionParseError::InvalidMemberSyntax)` instead of being ignored).
* For normal DataMashup fixtures:

  * `parse_package_parts` already ensures `Section1.m` exists and is UTF-8; tests cover BOM handling and invalid UTF-8.
  * Metadata parsing errors are handled earlier in `build_data_mashup` and defaulted appropriately.
* This milestone does **not** add new error variants to `ExcelOpenError` or `DataMashupError`; `build_queries` is an internal/domain helper for now.

### 3.3 Behavioral invariants

* For any successfully-built query list:

  * Synthetic metadata is possible when `Metadata` is incomplete; `queries.len()` may exceed `dm.metadata.formulas.len()` in those cases.
  * Every `Query.name` occurs at most once:

    * Enforced via `debug_assert!` in the builder; tests confirm uniqueness on realistic fixtures.
  * Every `Query` has:

    * A Section1 member, and
    * A `QueryMetadata` struct (either joined from XML or synthesized with the defaults above).
* Orphan metadata entries:

  * Never cause a panic.
  * Are not projected as `Query` values in this milestone.

### 3.4 Future-proofing

* The API is shaped to be a stable foundation for:

  * Milestone 6 `MQueryDiff` (textual M diff), which assumes `name`, `expression_m`, and `metadata` per query. 
  * Later rename detection based on `Query.name` & potential `query_signature`.
  * Semantic M diff (Milestone 7), which will parse `expression_m` into ASTs but can keep using the same Query container.

---

## 4. Interfaces

### 4.1 `datamashup.rs`

New types and functions:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub name: String,
    pub section_member: String,
    pub expression_m: String,
    pub metadata: QueryMetadata,
}

pub fn build_queries(dm: &DataMashup) -> Result<Vec<Query>, SectionParseError>;
```

Notes:

* `build_queries`:

  * Uses `dm.package_parts.main_section.source` as its Section1.m source.
  * Uses `parse_section_members` from `crate::m_section`.
  * Uses `dm.metadata.formulas` as the metadata source.

No changes to existing signatures:

* `build_data_mashup(raw: &RawDataMashup) -> Result<DataMashup, DataMashupError>` remains unchanged, and **does not** populate queries yet.

### 4.2 `lib.rs` exports

Extend the public exports:

```rust
pub use datamashup::{
    DataMashup,
    Metadata,
    Permissions,
    Query,
    QueryMetadata,
    build_data_mashup,
    build_queries,
};
```

* This makes `Query` and `build_queries` available to:

  * Test modules.
  * Future M diff code (Milestone 6) and CLI.

No other module exports change in this cycle. 

---

## 5. Test Plan

All tests must compile and run under the existing `cargo test` setup from the repo root, using the Python-generated fixtures.

### 5.1 New fixtures

Add to `fixtures/manifest.yaml` and corresponding generators:

1. **`metadata_url_encoding.xlsx`**

   * Generator: extend `MashupPermissionsMetadataGenerator` or add a new generator in `fixtures/src/generators/mashup.py`.
   * Shape:

     * Single query named `Query with space & #`.
     * Loads to sheet (or model) with deterministic load flags.
   * Metadata:

     * `ItemPath` uses encoded name `Section1/Query%20with%20space%20%26%20%23`.

2. **`metadata_orphan_entries.xlsx`**

   * Generator: similar path, or manual XML edit.
   * Shape:

     * Section1.m defines `shared Foo = 1;`.
     * Metadata formulas list:

       * `Section1/Foo`
       * `Section1/Nonexistent`.

These sit alongside existing M4 fixtures like `metadata_simple.xlsx`, `metadata_query_groups.xlsx`, `metadata_hidden_queries.xlsx`.

### 5.2 New Rust tests

Create `core/tests/m5_query_domain_tests.rs` with the following tests (names can be adapted but semantics must match):

#### 5.2.1 `metadata_join_simple`

* Fixture: `metadata_simple.xlsx`.
* Steps:

  1. Load `DataMashup`:

     ```rust
     let dm = load_datamashup("metadata_simple.xlsx");
     ```

  2. Build queries:

     ```rust
     let queries = excel_diff::build_queries(&dm).expect("queries should build");
     ```

  3. Assertions:

     * `queries.len() == 2`.
     * Names set equals `{"Section1/LoadToSheet", "Section1/LoadToModel"}`.
     * For `LoadToSheet`:

       * `metadata.load_to_sheet == true`.
       * `metadata.load_to_model == false`.
     * For `LoadToModel`:

       * `metadata.load_to_sheet == false`.
       * `metadata.load_to_model == true`.

#### 5.2.2 `metadata_join_url_encoding`

* Fixture: `metadata_url_encoding.xlsx`.
* Steps:

  1. Load `DataMashup` from the fixture.
  2. Call `build_queries`.
  3. Assertions:

     * `queries.len() == 1`.
     * `queries[0].name == "Section1/Query with space & #"` (decoded form).
     * `queries[0].section_member == "Query with space & #"`.

This confirms that URL-decoding in Metadata survives through to the domain `Query` and that the join correctly matches metadata to Section1 members.

#### 5.2.3 `query_names_unique`

* Fixture: any of the existing fixtures with multiple queries (e.g. `metadata_simple.xlsx`).

* Steps:

  1. Load `DataMashup`.
  2. Build queries.
  3. Assertions:

     * Iterate over names and ensure all are unique, e.g.:

       ```rust
       let mut seen = std::collections::HashSet::new();
       for q in &queries {
           assert!(seen.insert(&q.name));
       }
       ```

This locks in the uniqueness invariant the spec calls out, using a realistic workbook.

#### 5.2.4 `metadata_orphan_entries`

* Fixture: `metadata_orphan_entries.xlsx`.

* Steps:

  1. Load `DataMashup`.
  2. Call `build_queries`.
  3. Assertions:

     * Queries include `Section1/Foo` but **not** `Section1/Nonexistent`:

       * `queries.len() == 1`.
       * `queries[0].name == "Section1/Foo"`.
     * Raw metadata still includes the orphan:

       ```rust
       assert!(dm
           .metadata
           .formulas
           .iter()
           .any(|m| m.item_path == "Section1/Nonexistent"));
       ```

This test codifies the “drop orphan in domain view, preserve in raw metadata” behavior.

### 5.3 Regression guard

Add a small regression-style test ensuring that building queries does not affect M4 tests:

* Option: in `m4_permissions_metadata_tests.rs`, add a smoke test:

```rust
#[test]
fn build_queries_is_compatible_with_metadata_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = excel_diff::build_queries(&dm).expect("queries should build");
    assert!(!queries.is_empty());
}
```

This gives early detection if future refactors break `build_queries` for fixtures that already relied on M4 invariants.

---

With this mini-spec, the next cycle’s implementer can:

* Implement the `Query` type and `build_queries` helper in `datamashup.rs`.
* Wire up exports in `lib.rs`.
* Add/extend Python generators and fixtures for M5.
* Implement the M5.2–M5.3 tests in Rust.

Once this is in place, Milestone 6 (Textual M diff engine) can focus purely on building `MQueryDiff` on top of `Vec<Query>` without worrying about how Metadata and Section1.m are glued together.
