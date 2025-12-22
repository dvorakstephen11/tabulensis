## Branch 6 goal

Branch 6 (“datamashup-embedded-hardening”) is about making the DataMashup surface complete for diffing by (1) turning embedded mini-packages into first-class diffable entities, (2) hardening `PackageParts` parsing for real-world variability, and (3) expanding fuzz + golden coverage so pathological embedded content cannot crash parsing or semantic diffing. 

The practical outcome: if only an embedded `Content/*` package changes, the diff should show an explicit query-level change for the embedded query, and not “spill” unrelated diffs onto top-level queries. 

---

## Where the code is today (relevant baseline)

### Embedded content is already extracted, but not diffed

`parse_package_parts_with_limits` already scans `Content/*` entries, tries to open them as nested zips, and extracts `Formulas/Section1.m` as `embedded_contents`. It also strips BOMs and has a strong suite of parsing/limit tests.

### Query building only considers the main `Formulas/Section1.m`

`build_queries(dm)` parses members only from `dm.package_parts.main_section.source` and builds query names like `Section1/Foo`. It does not incorporate `embedded_contents`.

### The diff pipeline only diffs what `build_queries` returns

`diff_m_ops_for_packages` currently calls `build_queries` and passes only those queries into `diff_queries_to_ops`. Embedded content, even if extracted, cannot produce diffs because it never becomes queries.

Branch 6 is essentially wiring the already-extracted `embedded_contents` into the “queries surface” and then through the existing diff engine. 

---

## Design choice (Branch 6 “Option A”)

Use Option A from the sprint plan: treat each embedded package’s `Section1.m` as its own namespace of queries with synthetic names:

```
Embedded/<content-name>/<SectionName>/<MemberName>
```

Example:

```
Embedded/Content/efgh.package/Section1/Inner
```

This is intentionally “dumb but stable”: no need to follow references, no need to interpret how the outer query uses embedded content. It just ensures embedded code changes are visible in diffs. 

---

## Implementation plan

### Workstream 1: Build embedded queries

#### 1. Add `build_embedded_queries(dm) -> Vec<Query>`

Location: `core/src/datamashup.rs`

Behavior:

* Iterate `dm.package_parts.embedded_contents`.
* For each embedded `SectionDocument`, call `parse_section_members`.
* For each shared member, construct:

  * `section_name = Embedded/<content-name>/<SectionName>`
  * `name = <section_name>/<MemberName>`
  * Default metadata: connection-only, no load targets, no group path.
* If an embedded section fails to parse, skip it (best-effort). This avoids dropping all diffs just because one embedded blob is malformed.

This matches the branch plan’s intended shape. 

**New code to add (place near `build_queries`)**

```rust
pub fn build_embedded_queries(dm: &DataMashup) -> Vec<Query> {
    let mut queries: Vec<Query> = Vec::new();
    let mut positions: HashMap<String, usize> = HashMap::new();

    for embedded in &dm.package_parts.embedded_contents {
        let members = match parse_section_members(&embedded.section.source) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for member in members {
            let section_name = format!("Embedded/{}/{}", embedded.name, member.section_name);
            let name = format!("{}/{}", section_name, member.member_name);

            let q = Query {
                name: name.clone(),
                section_member: member.member_name.clone(),
                expression_m: member.expression_m,
                metadata: QueryMetadata {
                    item_path: name,
                    section_name,
                    formula_name: member.member_name,
                    load_to_sheet: false,
                    load_to_model: false,
                    is_connection_only: true,
                    group_path: None,
                },
            };

            if let Some(idx) = positions.get(&q.name).copied() {
                queries[idx] = q;
            } else {
                positions.insert(q.name.clone(), queries.len());
                queries.push(q);
            }
        }
    }

    queries
}
```

Notes:

* This mirrors `build_queries` behavior (name de-dupe via `positions`) so you keep deterministic “last one wins” semantics if odd inputs duplicate names.

#### 2. Export it from the crate root

Your integration tests live in `core/tests/*` and cannot reach `crate::datamashup::*` unless it’s publicly re-exported (the `datamashup` module is private in `lib.rs` today). 

So update `core/src/lib.rs` to re-export `build_embedded_queries` alongside `build_queries`.

**Code to replace (representative existing export line)**

```rust
pub use datamashup::{DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries};
```

**New code to replace it**

```rust
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_embedded_queries,
    build_queries,
};
```

(Exact wrapping/formatting can match your file’s style; the key is adding `build_embedded_queries`.)

---

### Workstream 2: Feed embedded queries into the diff pipeline

Location: `core/src/m_diff.rs`

Goal:

* Wherever we compute `old_q` and `new_q`, append embedded queries:

  * `queries.extend(build_embedded_queries(dm));`
* Preserve existing behavior for parse errors in the main section (`build_queries` still gates; if it fails we keep returning `Vec::new()` like today).

This is the minimum-change wiring that makes embedded changes visible.

#### Replace `diff_m_ops_for_packages` to include embedded queries

**Code to replace** 

```rust
use crate::datamashup::{DataMashup, Query, build_queries};
use crate::diff::{DiffConfig, DiffOp};
use crate::m_section::SectionParseError;
use crate::string_pool::StringPool;

pub fn diff_m_ops_for_packages(
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    match (old_dm, new_dm) {
        (None, None) => Vec::new(),
        (Some(old_dm), None) => match build_queries(old_dm) {
            Ok(queries) => queries
                .into_iter()
                .map(|q| DiffOp::QueryRemoved {
                    name: pool.intern(&q.name),
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        (None, Some(new_dm)) => match build_queries(new_dm) {
            Ok(queries) => queries
                .into_iter()
                .map(|q| DiffOp::QueryAdded {
                    name: pool.intern(&q.name),
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        (Some(old_dm), Some(new_dm)) => {
            let old_q = match build_queries(old_dm) {
                Ok(q) => q,
                Err(_) => return Vec::new(),
            };
            let new_q = match build_queries(new_dm) {
                Ok(q) => q,
                Err(_) => return Vec::new(),
            };

            diff_queries_to_ops(&old_q, &new_q, pool, config)
        }
    }
}
```

**New code to replace it**

```rust
use crate::datamashup::{DataMashup, Query, build_embedded_queries, build_queries};
use crate::diff::{DiffConfig, DiffOp};
use crate::m_section::SectionParseError;
use crate::string_pool::StringPool;

fn build_all_queries(dm: &DataMashup) -> Result<Vec<Query>, SectionParseError> {
    let mut q = build_queries(dm)?;
    q.extend(build_embedded_queries(dm));
    Ok(q)
}

pub fn diff_m_ops_for_packages(
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    match (old_dm, new_dm) {
        (None, None) => Vec::new(),
        (Some(old_dm), None) => match build_all_queries(old_dm) {
            Ok(queries) => queries
                .into_iter()
                .map(|q| DiffOp::QueryRemoved {
                    name: pool.intern(&q.name),
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        (None, Some(new_dm)) => match build_all_queries(new_dm) {
            Ok(queries) => queries
                .into_iter()
                .map(|q| DiffOp::QueryAdded {
                    name: pool.intern(&q.name),
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        (Some(old_dm), Some(new_dm)) => {
            let old_q = match build_all_queries(old_dm) {
                Ok(q) => q,
                Err(_) => return Vec::new(),
            };
            let new_q = match build_all_queries(new_dm) {
                Ok(q) => q,
                Err(_) => return Vec::new(),
            };
            diff_queries_to_ops(&old_q, &new_q, pool, config)
        }
    }
}
```

Why this is safe:

* `build_queries` is unchanged, so top-level query names and metadata join behavior remain identical.
* Embedded queries only add additional names, so they won’t affect diffs unless they themselves change.

---

### Workstream 3: PackageParts hardening for variability

The current `Content/*` extractor assumes each `Content/*` entry is a nested zip “.package” and tries to open it. That skips other plausible shapes (notably: unpacked embedded folders).

The branch plan explicitly cites embedded paths like `Content/{GUID}/Formulas/Section1.m` and calls for “hardening parsing against more real-world variability.” 

#### 1. Normalize path separators more aggressively

Today `normalize_path` only trims a leading `/`.

To tolerate “slightly different paths”, add:

* Trim leading `\` as well.
* Convert `\` to `/` for comparisons and stored names.

#### 2. Support unpacked embedded packages (directory form)

If an archive entry looks like:

```
Content/<something>/Formulas/Section1.m
```

…treat it as an embedded section directly, without attempting nested zip extraction.

Implementation approach:

* In the `Content/` branch, check for the suffix `/Formulas/Section1.m`.
* If present, parse the entry bytes as UTF-8 M text, strip BOM, and store an `EmbeddedContent` whose name is the prefix before the suffix:

  * name: `Content/<something>`
  * section.source: the file text

This aligns with the plan’s “Content/{GUID}/Formulas/Section1.m” framing. 

#### 3. Keep resource ceilings enforced

Continue to apply:

* `max_inner_entries`
* `max_inner_part_bytes`
* `max_inner_total_bytes`

The reserve/budgeting mechanism already exists and is tested.
The new “unpacked Section1.m” path must pass through the same `reserve_inner_read_budget` call before reading its bytes.

#### Code changes in `core/src/datamashup_package.rs`

**Code to replace** (this includes `normalize_path` and the `Content/` handling inside `parse_package_parts_with_limits`)

```rust
fn normalize_path(name: &str) -> &str {
    name.trim_start_matches('/')
}

pub fn parse_package_parts_with_limits(
    bytes: &[u8],
    limits: DataMashupLimits,
) -> Result<PackageParts, DataMashupError> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|_| DataMashupError::FramingInvalid)?;

    if archive.len() > limits.max_inner_entries {
        return Err(DataMashupError::InnerTooManyEntries {
            count: archive.len(),
            max: limits.max_inner_entries,
        });
    }

    let mut total_read: u64 = 0;
    let mut package_xml: Option<PackageXml> = None;
    let mut main_section: Option<SectionDocument> = None;
    let mut embedded_contents: Vec<EmbeddedContent> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|_| DataMashupError::FramingInvalid)?;
        if file.is_dir() {
            continue;
        }
        let raw_name = file.name().to_string();
        let name = normalize_path(&raw_name);

        if package_xml.is_none() && name == "Config/Package.xml" {
            reserve_inner_read_budget(&mut total_read, name, file.size(), limits)?;
            package_xml = Some(PackageXml {
                xml: read_file_to_string(&mut file)?,
            });
            continue;
        }
        if main_section.is_none() && name == "Formulas/Section1.m" {
            reserve_inner_read_budget(&mut total_read, name, file.size(), limits)?;
            let mut s = read_file_to_string(&mut file)?;
            s = strip_leading_bom(s);
            main_section = Some(SectionDocument { source: s });
            continue;
        }
        if name.starts_with("Content/") {
            reserve_inner_read_budget(&mut total_read, name, file.size(), limits)?;
            let mut content_bytes = Vec::new();
            if file.read_to_end(&mut content_bytes).is_err() {
                continue;
            }
            if let Some(section) = extract_embedded_section(&content_bytes, limits, name)? {
                embedded_contents.push(EmbeddedContent {
                    name: normalize_path(&raw_name).to_string(),
                    section: SectionDocument { source: section },
                });
            }
        }
    }

    if package_xml.is_none() || main_section.is_none() {
        return Err(DataMashupError::FramingInvalid);
    }

    Ok(PackageParts {
        package_xml: package_xml.unwrap(),
        main_section: main_section.unwrap(),
        embedded_contents,
    })
}
```

**New code to replace it**

```rust
fn normalize_path(name: &str) -> String {
    let trimmed = name.trim_start_matches(|c| c == '/' || c == '\\');
    trimmed.replace('\\', "/")
}

pub fn parse_package_parts_with_limits(
    bytes: &[u8],
    limits: DataMashupLimits,
) -> Result<PackageParts, DataMashupError> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|_| DataMashupError::FramingInvalid)?;

    if archive.len() > limits.max_inner_entries {
        return Err(DataMashupError::InnerTooManyEntries {
            count: archive.len(),
            max: limits.max_inner_entries,
        });
    }

    let mut total_read: u64 = 0;
    let mut package_xml: Option<PackageXml> = None;
    let mut main_section: Option<SectionDocument> = None;
    let mut embedded_contents: Vec<EmbeddedContent> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|_| DataMashupError::FramingInvalid)?;
        if file.is_dir() {
            continue;
        }

        let raw_name = file.name().to_string();
        let name = normalize_path(&raw_name);

        if package_xml.is_none() && name == "Config/Package.xml" {
            reserve_inner_read_budget(&mut total_read, &name, file.size(), limits)?;
            package_xml = Some(PackageXml {
                xml: read_file_to_string(&mut file)?,
            });
            continue;
        }

        if main_section.is_none() && name == "Formulas/Section1.m" {
            reserve_inner_read_budget(&mut total_read, &name, file.size(), limits)?;
            let mut s = read_file_to_string(&mut file)?;
            s = strip_leading_bom(s);
            main_section = Some(SectionDocument { source: s });
            continue;
        }

        if name.starts_with("Content/") {
            reserve_inner_read_budget(&mut total_read, &name, file.size(), limits)?;

            let mut content_bytes = Vec::new();
            if file.read_to_end(&mut content_bytes).is_err() {
                continue;
            }

            let unpacked_suffix = "/Formulas/Section1.m";
            if name.ends_with(unpacked_suffix) {
                if let Some(root) = name.strip_suffix(unpacked_suffix) {
                    if embedded_contents.iter().all(|e| e.name != root) {
                        if let Ok(text) = std::str::from_utf8(&content_bytes) {
                            let s = strip_leading_bom(text.to_string());
                            embedded_contents.push(EmbeddedContent {
                                name: root.to_string(),
                                section: SectionDocument { source: s },
                            });
                        }
                    }
                }
                continue;
            }

            if let Some(section) = extract_embedded_section(&content_bytes, limits, &name)? {
                embedded_contents.push(EmbeddedContent {
                    name: name.clone(),
                    section: SectionDocument { source: section },
                });
            }
        }
    }

    if package_xml.is_none() || main_section.is_none() {
        return Err(DataMashupError::FramingInvalid);
    }

    Ok(PackageParts {
        package_xml: package_xml.unwrap(),
        main_section: main_section.unwrap(),
        embedded_contents,
    })
}
```

What this buys you:

* Existing behavior remains for zipped `.package` blobs. 
* New support for “unpacked” embedded `Content/<id>/Formulas/Section1.m` entries.
* Stored embedded names are now slash-normalized, which stabilizes synthetic query names.

---

### Workstream 4: Golden fixtures + integration tests

You already have good M diff integration tests that assert specific query ops and resolved names.

Branch 6 needs a new golden pair where:

* Outer Section1.m is unchanged (so top-level queries do not diff).
* Only the embedded `Content/*.package` `Section1.m` changes.

#### 1. Add two new fixtures via the existing generator

The `MashupMultiEmbeddedGenerator` already accepts `embedded_section` and `embedded_guid`.

Update `fixtures/manifest_cli_tests.yaml` to generate two files (A/B) with different `embedded_section` bodies and the same guid.

**Suggested insert block (add near other M fixtures)** 

```yaml
  - id: "m_embedded_change_a"
    generator: "mashup:multi_query_with_embedded"
    args:
      base_file: "templates/base_query.xlsx"
      embedded_guid: "efgh"
      embedded_section: |
        section Section1;
        shared Inner = let
          Source = 1
        in
          Source;
    output: "m_embedded_change_a.xlsx"

  - id: "m_embedded_change_b"
    generator: "mashup:multi_query_with_embedded"
    args:
      base_file: "templates/base_query.xlsx"
      embedded_guid: "efgh"
      embedded_section: |
        section Section1;
        shared Inner = let
          Source = 2
        in
          Source;
    output: "m_embedded_change_b.xlsx"
```

(These align with how other “A/B” M fixtures are generated today. )

#### 2. Add a new integration test: embedded-only change yields exactly one M diff for the embedded query

Create: `core/tests/m10_embedded_m_diff_tests.rs` (name can be whatever fits your numbering scheme)

Use the established helpers/patterns from `m6_textual_m_diff_tests.rs`: load packages, `pkg_a.diff(&pkg_b, ...)`, then `report.m_ops()`, then resolve names.

**New test file (full content)**

```rust
use excel_diff::{DiffConfig, DiffOp, DiffReport, QueryChangeKind, WorkbookPackage};
use std::fs::File;

mod common;
use common::fixture_path;

fn load_package(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).expect("fixture file should open");
    WorkbookPackage::open(file).expect("fixture should parse as WorkbookPackage")
}

fn m_ops(report: &DiffReport) -> Vec<&DiffOp> {
    report.m_ops().collect()
}

fn resolve_name<'a>(report: &'a DiffReport, op: &DiffOp) -> &'a str {
    let name_id = match op {
        DiffOp::QueryAdded { name } => *name,
        DiffOp::QueryRemoved { name } => *name,
        DiffOp::QueryRenamed { from, .. } => *from,
        DiffOp::QueryDefinitionChanged { name, .. } => *name,
        DiffOp::QueryMetadataChanged { name, .. } => *name,
        _ => panic!("not a query op"),
    };
    &report.strings[name_id.0 as usize]
}

#[test]
fn embedded_only_change_produces_embedded_definitionchanged() {
    let pkg_a = load_package("m_embedded_change_a.xlsx");
    let pkg_b = load_package("m_embedded_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(def_changed.len(), 1, "expected one definition change in embedded content");

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged { change_kind, .. } => {
            assert_eq!(*change_kind, QueryChangeKind::Semantic);
        }
        _ => unreachable!(),
    }

    assert_eq!(
        resolve_name(&report, def_changed[0]),
        "Embedded/Content/efgh.package/Section1/Inner"
    );
}
```

This directly enforces the branch DoD: embedded-only change yields an explicit embedded diff, with no unrelated top-level diffs.

#### 3. Add a domain-level unit test for `build_embedded_queries`

Add to `core/tests/m5_query_domain_tests.rs` or a new `core/tests/m5_embedded_query_domain_tests.rs`.

High-value assertions:

* `build_embedded_queries(dm)` contains the expected synthetic name.
* `build_queries(dm)` does not contain names starting with `Embedded/`.

(You already have query-domain tests and patterns for asserting query metadata defaults. )

---

### Workstream 5: Expand fuzz coverage for M section splitting + AST parsing

The codebase already has fuzz harnesses for DataMashup parsing/diff grids.
Branch 6 explicitly asks for a fuzz target that exercises `parse_section_members` and then feeds the resulting member expressions into `parse_m_expression`. 

#### 1. Add a new fuzz target in `core/fuzz`

**Update `core/fuzz/Cargo.toml`**

**Code to replace** (existing `[[bin]]` list excerpt) 

```toml
[[bin]]
name = "fuzz_diff_grids"
path = "fuzz_targets/fuzz_diff_grids.rs"
test = false
doc = false
bench = false
```

**New code to replace it** (append a new bin after it; showing both entries here for clarity)

```toml
[[bin]]
name = "fuzz_diff_grids"
path = "fuzz_targets/fuzz_diff_grids.rs"
test = false
doc = false
bench = false

[[bin]]
name = "fuzz_m_section_and_ast"
path = "fuzz_targets/fuzz_m_section_and_ast.rs"
test = false
doc = false
bench = false
```

**Add `core/fuzz/fuzz_targets/fuzz_m_section_and_ast.rs`**

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use excel_diff::{parse_m_expression, parse_section_members};

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    let expr = s.as_ref().get(..4096).unwrap_or(s.as_ref());
    let section = format!("section Section1;\nshared Foo = {};\n", expr);

    if let Ok(members) = parse_section_members(&section) {
        for m in members {
            let _ = parse_m_expression(&m.expression_m);
        }
    }
});
```

Why this is the right harness:

* It forces `parse_section_members` to succeed frequently (because the wrapper is syntactically “section + shared Foo = ...;”), which means it will regularly generate `expression_m` strings to stress `parse_m_expression` with. That’s exactly the “together” coverage branch 6 is targeting. 

#### 2. Corpus seeds

Even if CI doesn’t ship corpora, keep a small local seed set:

* A few realistic `let ... in ...` expressions
* A few strings with nested delimiters
* A few with unusual whitespace / comments

Put them under:

```
core/fuzz/corpus/fuzz_m_section_and_ast/
```

…and document in a short note in `core/fuzz/README.md` (if present) how to run:

* `cargo fuzz run fuzz_m_section_and_ast`

---

## Additional hardening tests (PackageParts)

You already have strong tests for:

* leading slash canonicalization
* empty `Content/` dir ignored
* invalid zips skipped
* missing `Section1.m` in embedded ignored
* limits: too many entries, per-part too large, total too large, nested limits

Branch 6 “hardening” should add coverage specifically for the new behaviors:

1. **Unpacked embedded Section1.m is extracted**

* Build a minimal `PackageParts` zip with:

  * `Config/Package.xml`
  * `Formulas/Section1.m`
  * `Content/abcd/Formulas/Section1.m` containing valid M
* Assert `embedded_contents[0].name == "Content/abcd"` and the source matches.

2. **Backslash paths normalize correctly**

* Add `Content\\abcd\\Formulas\\Section1.m` and ensure it still extracts.

3. **Boundary limit tests**

* Verify equality passes:

  * part size exactly `max_part_uncompressed_bytes` should succeed
  * total size exactly `max_total_uncompressed_bytes` should succeed
* This prevents future off-by-one regressions around the existing budget logic.

---

## Definition-of-done checklist (Branch 6)

This is a concrete “done means done” list mapped to branch 6’s stated outcomes. 

1. **Embedded content becomes diffable**

* `build_embedded_queries` exists and produces `Embedded/...` synthetic query names.
* `diff_m_ops_for_packages` includes embedded queries in old/new sets.

2. **Golden proof**

* A fixture pair where only embedded content changes produces:

  * exactly one `QueryDefinitionChanged` for `Embedded/Content/<guid>.package/Section1/Inner`
  * no changes for top-level queries

3. **Parsing hardening**

* `parse_package_parts_with_limits` handles:

  * nested `.package` zips (existing)
  * unpacked `Content/<id>/Formulas/Section1.m` (new)
  * path separator variability (`\` vs `/`) (new)
* Limits still enforced and tested.

4. **Fuzz coverage**

* New fuzz target exercises `parse_section_members` -> `parse_m_expression` pipeline without panics.

---

## Optional (only if you see it in real files quickly)

If you later discover embedded packages that contain multiple section files (e.g., `Formulas/Section2.m`) and you want to diff them too, the next incremental extension is:

* extract all `Formulas/*.m` inside an embedded package (or inside an unpacked content folder)
* generate queries under:

  * `Embedded/<content-name>/<SectionX>/<Member>`

I would keep that out of branch 6 unless you already have a corpus/fixture showing it, because it expands surface area and test matrix.

