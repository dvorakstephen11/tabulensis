Milestone: **M6 – Textual M diff engine (basic)**  
Scope for this branch: **6.1 Basic M diffs + 6.2 rename scenario (as add/remove), no embedded or AST diff.**

---

## 1. Scope

### Rust modules

**New**

- `core/src/m_diff.rs`
  - Houses the *domain-level* M query diff types and logic.
  - Implements a pure-Rust API over the existing `DataMashup` and `Query` domain.

**Edited**

- `core/src/lib.rs`
  - Expose the new types and functions:
    - `QueryChangeKind`
    - `MQueryDiff`
    - `diff_m_queries`

- `core/tests/m6_textual_m_diff_tests.rs` (new test module)
  - Integration tests for Milestone 6 using real `.xlsx` fixtures.

### Fixtures & generators

**Edited**

- `fixtures/manifest.yaml`
  - Add scenarios to generate A/B workbooks for M6:

    - `m_add_query_a.xlsx`
    - `m_add_query_b.xlsx`
    - `m_remove_query_a.xlsx`
    - `m_remove_query_b.xlsx`
    - `m_change_literal_a.xlsx`
    - `m_change_literal_b.xlsx` (already present, treat as “B”; add “A”)
    - `m_metadata_only_change_a.xlsx`
    - `m_metadata_only_change_b.xlsx`
    - `m_rename_query_a.xlsx`
    - `m_rename_query_b.xlsx`

  - Scenarios should reuse existing mashup generators (`MashupInjectGenerator`, `MashupPermissionsMetadataGenerator`, etc.) where possible.

- `fixtures/src/generators/mashup.py`
  - Extend or reuse generator modes so that each scenario produces the right DataMashup contents for A and B (details in Test Plan).

**No changes**

- No changes to `core/src/diff.rs`, `core/src/engine.rs`, or JSON/CLI output modules in this cycle.

---

## 2. Behavioral Contract

### 2.1 Types

New domain-level diff types:

```rust
pub enum QueryChangeKind {
    Added,
    Removed,
    Renamed { from: String, to: String }, // not produced yet in this branch
    DefinitionChanged,
    MetadataChangedOnly,
}

pub struct MQueryDiff {
    pub name: String,            // fully-qualified query name, e.g. "Section1/Foo"
    pub kind: QueryChangeKind,
}
````

**Notes**

* `name` is always the *fully qualified* query name `SectionName/MemberName` as produced by `build_queries` (e.g., `"Section1/Foo"`).
* For this branch, `QueryChangeKind::Renamed` exists for forward compatibility but **is never constructed**; rename scenarios are surfaced as `Removed` + `Added` pairs.
* `MQueryDiff` derives `Debug`, `Clone`, `PartialEq`, and `Eq` to keep tests straightforward.

### 2.2 Public API

```rust
pub fn diff_m_queries(
    old_dm: &DataMashup,
    new_dm: &DataMashup,
) -> Result<Vec<MQueryDiff>, SectionParseError>;
```

Semantics:

* Internally:

  * Calls `build_queries(old_dm)` and `build_queries(new_dm)`.
  * Computes diffs on the resulting `Vec<Query>` slices.
* Externally:

  * Returns a **deterministic**, **sorted** vector of `MQueryDiff` values.
  * Sorting rule: ascending lexicographic order by `name` (`Section1/Foo`, then `Section1/Bar`, etc.).
* Errors:

  * Propagates `SectionParseError` if either mashup’s `Section1.m` fails to parse (e.g., malformed `shared` syntax).
  * This is consistent with `build_queries`.

The internal helper is private:

```rust
fn diff_queries(old_queries: &[Query], new_queries: &[Query]) -> Vec<MQueryDiff>;
```

### 2.3 Diff classification rules

Let `old` and `new` be maps keyed by `Query.name` (`Section1/Foo`), values of type `Query`:

* **Added query**

  * If `name ∉ old` and `name ∈ new`:

    ```rust
    MQueryDiff { name, kind: QueryChangeKind::Added }
    ```

* **Removed query**

  * If `name ∈ old` and `name ∉ new`:

    ```rust
    MQueryDiff { name, kind: QueryChangeKind::Removed }
    ```

* **Unchanged query**

  * If `name ∈ old` and `name ∈ new` and:

    * `old.expression_m == new.expression_m`
    * `old.metadata == new.metadata`

  * Then **no diff** is emitted for that query.

* **DefinitionChanged**

  * If `name ∈ old` and `name ∈ new` and:

    * `old.expression_m != new.expression_m`

  * Regardless of metadata equality, we classify as:

    ```rust
    MQueryDiff { name, kind: QueryChangeKind::DefinitionChanged }
    ```

  * This is a *text-level* classification: we only compare the full `expression_m` strings and do not yet compute a structured or Myers-style diff.

* **MetadataChangedOnly**

  * If `name ∈ old` and `name ∈ new` and:

    * `old.expression_m == new.expression_m`
    * `old.metadata != new.metadata`

  * Then:

    ```rust
    MQueryDiff { name, kind: QueryChangeKind::MetadataChangedOnly }
    ```

* **Renames (current behavior)**

  * If A has `"Section1/Foo"` and B has `"Section1/Bar"` with identical `expression_m` and `metadata`, there is no name-based match.

  * This branch **does not implement rename detection**.

  * The diff is:

    ```text
    Removed("Section1/Foo")
    Added("Section1/Bar")
    ```

  * Tests will explicitly codify this behavior for now.

### 2.4 Behavioral examples

Assume all queries live in `Section1`.

1. **Add query**

   * A: `Foo`
   * B: `Foo`, `Bar`
   * Result (sorted by name):

     ```text
     [
       MQueryDiff { name: "Section1/Bar", kind: Added }
     ]
     ```

2. **Remove query**

   * A: `Foo`, `Bar`
   * B: `Foo`
   * Result:

     ```text
     [
       MQueryDiff { name: "Section1/Bar", kind: Removed }
     ]
     ```

3. **Change literal in M code**

   * `Foo` exists in both.
   * A: `shared Foo = 1;`
   * B: `shared Foo = 2;`
   * Metadata identical.
   * Result:

     ```text
     [
       MQueryDiff { name: "Section1/Foo", kind: DefinitionChanged }
     ]
     ```

4. **Metadata-only change**

   * A: `Foo` loads to **sheet only**.
   * B: `Foo` loads to **model only**.
   * `expression_m` identical.
   * Result:

     ```text
     [
       MQueryDiff { name: "Section1/Foo", kind: MetadataChangedOnly }
     ]
     ```

5. **No changes**

   * A and B are bitwise-identical with respect to:

     * DataMashup layout,
     * `Section1.m` contents,
     * Metadata.
   * Result:

     ```text
     []
     ```

6. **Rename (current behavior)**

   * A: `Foo` (some body, metadata).

   * B: `Bar` (same body, metadata).

   * Result (sorted):

     ```text
     [
       MQueryDiff { name: "Section1/Bar", kind: Added },
       MQueryDiff { name: "Section1/Foo", kind: Removed },
     ]
     ```

   * There is **no** `Renamed` variant emitted in this branch; future work may change this.

---

## 3. Constraints and Invariants

### 3.1 Determinism

* `diff_m_queries` must be **deterministic** given the same `DataMashup` inputs.
* HashMap iteration order must not leak into the result:

  * Implementation must gather the union of query names, sort them, and iterate that sorted list.

### 3.2 Complexity

* Let `N = max(|old_queries|, |new_queries|)`.
* Time:

  * Building the maps: `O(N)`
  * Creating and sorting the name union: `O(N log N)`
  * Comparisons: `O(N)`
  * Overall: `O(N log N)` for realistic sizes (N typically ≪ 1000).
* Memory:

  * `O(N)` maps plus `O(N)` diff vector.

### 3.3 No new external dependencies

* Implementation must use only the Rust standard library and existing crate dependencies.
* No additional diff or regex crates for this milestone; `DefinitionChanged` is determined by string equality only.

### 3.4 Error handling

* Any `SectionParseError` from `build_queries` must be returned unchanged from `diff_m_queries`.
* No panics in normal operation; `debug_assert!` is allowed for invariants (e.g., unreachable cases).

### 3.5 Non-goals (this branch)

* No AST parsing of M code.
* No semantic diffing of M expressions.
* No change to `DiffOp` or `DiffReport`.
* No JSON/CLI surfacing of MQueryDiff.
* No handling of queries from embedded `PackageParts.embedded_contents` (M6.3 is deferred).

---

## 4. Interfaces

### 4.1 New module: `core/src/m_diff.rs`

**Public items**

```rust
use crate::datamashup::{DataMashup, Query};
use crate::m_section::SectionParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryChangeKind {
    Added,
    Removed,
    Renamed { from: String, to: String }, // not produced yet
    DefinitionChanged,
    MetadataChangedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MQueryDiff {
    pub name: String,
    pub kind: QueryChangeKind,
}

pub fn diff_m_queries(
    old_dm: &DataMashup,
    new_dm: &DataMashup,
) -> Result<Vec<MQueryDiff>, SectionParseError> {
    let old_queries = crate::datamashup::build_queries(old_dm)?;
    let new_queries = crate::datamashup::build_queries(new_dm)?;
    Ok(diff_queries(&old_queries, &new_queries))
}

// private helper
fn diff_queries(old_queries: &[Query], new_queries: &[Query]) -> Vec<MQueryDiff> {
    // Implementation per behavioral contract; see Section 2.3.
}
```

### 4.2 `core/src/lib.rs` exports

Add:

```rust
pub mod m_diff; // new module

pub use m_diff::{MQueryDiff, QueryChangeKind, diff_m_queries};
```

* No other public interfaces change.
* `DiffOp` and `DiffReport` remain untouched.

---

## 5. Test Plan

All tests live under `core/tests` and use `fixtures/generated/*.xlsx` via the existing `common::fixture_path` helper.

### 5.1 Fixtures (Excel workbooks)

The goal is to match the Milestone 6 scenarios described in the testing plan while keeping contents simple and explicit.

#### 5.1.1 m_add_query

* **m_add_query_a.xlsx**

  * DataMashup:

    * `Section1.m` defines exactly one shared query:

      ```m
      section Section1;
      shared Foo = 1;
      ```

    * Metadata:

      * Single formula entry for `Section1/Foo`, with `load_to_sheet = true`, `load_to_model = false`.

* **m_add_query_b.xlsx**

  * Starting from the same base, `Section1.m` defines:

    ```m
    section Section1;
    shared Foo = 1;
    shared Bar = 2;
    ```

  * Metadata:

    * Entries for `Section1/Foo` and `Section1/Bar`, both load destinations arbitrary but consistent between A and B for each query.

#### 5.1.2 m_remove_query

* **m_remove_query_a.xlsx**

  * `Section1.m`:

    ```m
    section Section1;
    shared Foo = 1;
    shared Bar = 2;
    ```

* **m_remove_query_b.xlsx**

  * `Section1.m`:

    ```m
    section Section1;
    shared Foo = 1;
    ```

* Metadata: consistent and correctly aligned in both files.

#### 5.1.3 m_change_literal

* **m_change_literal_a.xlsx**

  * `Section1/Foo` uses a literal `1`.

* **m_change_literal_b.xlsx** (already present as a fixture)

  * Same query name, but literal changed from `1` to `2`.

* Permissions/metadata are otherwise identical across A and B.

#### 5.1.4 m_metadata_only_change

* **m_metadata_only_change_a.xlsx**

  * `Section1/Foo` loads to sheet only (`load_to_sheet = true`, `load_to_model = false`).
  * `expression_m` is some simple literal (e.g., `shared Foo = 1;`).

* **m_metadata_only_change_b.xlsx**

  * Exact same `Section1.m` (identical M text).
  * Metadata changed only:

    * For example, `load_to_sheet = false`, `load_to_model = true`.

#### 5.1.5 m_rename_query (behavior as add/remove)

* **m_rename_query_a.xlsx**

  * `Section1.m`:

    ```m
    section Section1;
    shared Foo = 1;
    ```

* **m_rename_query_b.xlsx**

  * `Section1.m`:

    ```m
    section Section1;
    shared Bar = 1;
    ```

* Metadata: both queries load to the same destination; only the name differs.

#### 5.1.6 Generator wiring

* Add manifest entries like:

  ```yaml
  - id: "m_add_query_a"
    generator: "mashup_inject"
    args:
      base_file: "templates/base_query_single.xlsx"   # or reuse an existing template
      m_code: "section Section1;\nshared Foo = 1;\n"
    output: "m_add_query_a.xlsx"

  - id: "m_add_query_b"
    generator: "mashup_inject"
    args:
      base_file: "templates/base_query_single.xlsx"
      m_code: "section Section1;\nshared Foo = 1;\nshared Bar = 2;\n"
    output: "m_add_query_b.xlsx"
  ```

* Similar patterns for `m_remove_query_*`, `m_change_literal_*`, `m_metadata_only_change_*`, and `m_rename_query_*`.

* Exact generator choices are flexible as long as the resulting DataMashup contents match the behavioral expectations above.

### 5.2 Rust tests

New file: `core/tests/m6_textual_m_diff_tests.rs`

Common helper:

```rust
use excel_diff::{DataMashup, QueryChangeKind, diff_m_queries, build_data_mashup, open_data_mashup};

mod common;
use common::fixture_path;

fn load_datamashup(name: &str) -> DataMashup {
    let raw = open_data_mashup(fixture_path(name))
        .expect("fixture should open")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}
```

#### 5.2.1 basic_add_query_diff

```rust
#[test]
fn basic_add_query_diff() {
    let dm_a = load_datamashup("m_add_query_a.xlsx");
    let dm_b = load_datamashup("m_add_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected exactly one diff for added query");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Bar");
    assert_eq!(diff.kind, QueryChangeKind::Added);
}
```

#### 5.2.2 basic_remove_query_diff

```rust
#[test]
fn basic_remove_query_diff() {
    let dm_a = load_datamashup("m_remove_query_a.xlsx");
    let dm_b = load_datamashup("m_remove_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected exactly one diff for removed query");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Bar");
    assert_eq!(diff.kind, QueryChangeKind::Removed);
}
```

#### 5.2.3 literal_change_produces_definitionchanged

```rust
#[test]
fn literal_change_produces_definitionchanged() {
    let dm_a = load_datamashup("m_change_literal_a.xlsx");
    let dm_b = load_datamashup("m_change_literal_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for changed literal");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}
```

* Optional future extension (not required in this branch): assert that the raw `expression_m` values differ as expected, but no structured diff is checked yet.

#### 5.2.4 metadata_change_produces_metadataonly

```rust
#[test]
fn metadata_change_produces_metadataonly() {
    let dm_a = load_datamashup("m_metadata_only_change_a.xlsx");
    let dm_b = load_datamashup("m_metadata_only_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for metadata-only change");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::MetadataChangedOnly);
}
```

#### 5.2.5 identical_workbooks_produce_no_diffs

Use an existing simple mashup fixture (`one_query.xlsx`) for a sanity check.

```rust
#[test]
fn identical_workbooks_produce_no_diffs() {
    let dm = load_datamashup("one_query.xlsx");

    let diffs = diff_m_queries(&dm, &dm).expect("diff should succeed");

    assert!(diffs.is_empty(), "identical DataMashup should produce no diffs");
}
```

#### 5.2.6 rename_reports_add_and_remove

Codify the current behavior for rename (no rename detection yet):

```rust
#[test]
fn rename_reports_add_and_remove() {
    let dm_a = load_datamashup("m_rename_query_a.xlsx");
    let dm_b = load_datamashup("m_rename_query_b.xlsx");

    let mut diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");
    diffs.sort_by(|a, b| a.name.cmp(&b.name)); // defensive if impl ever changes ordering

    assert_eq!(diffs.len(), 2, "expected add + remove for rename scenario");

    let names: Vec<_> = diffs.iter().map(|d| (&d.name, &d.kind)).collect();
    assert!(
        names.contains(&(&"Section1/Foo".to_string(), &QueryChangeKind::Removed)),
        "Foo should be reported as Removed"
    );
    assert!(
        names.contains(&(&"Section1/Bar".to_string(), &QueryChangeKind::Added)),
        "Bar should be reported as Added"
    );
}
```

---

## 6. Future Work Hints (not in this branch)

* Introduce a more detailed `MDefinitionChange` payload holding `before`/`after` and eventually a structured line/segment diff for M code.
* Add `DiffOp` variants for M query changes and bridge `diff_m_queries` into the top-level `DiffReport`.
* Extend diffing to include queries from `PackageParts.embedded_contents` (M6.3).
* Later milestone: AST-based semantic M diff (H4), layered on top of this textual diff contract.

This spec keeps the branch small and test-driven: we get a solid, deterministic textual M diff at the Query level, exercised via real Excel fixtures, without yet entangling the grid diff engine or output formats.
