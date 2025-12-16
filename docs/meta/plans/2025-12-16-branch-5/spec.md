Below is an implementation plan that takes your *current* codebase state (separate `open_workbook` / `open_data_mashup`, `diff_workbooks`, and `diff_m_queries -> Vec<MQueryDiff>`) into **full Branch 5 completion**: a single `WorkbookPackage` open + unified diff stream where **M query changes are `DiffOp` variants in the same report**.

---

## 0) Target end state for Branch 5

After this work, consumers should primarily use:

* `WorkbookPackage::open(...) -> Result<WorkbookPackage, PackageError>` (reader-based; path helper optional)
* `WorkbookPackage::diff(&self, other, config) -> DiffReport`
* `WorkbookPackage::diff_streaming(&self, other, config, sink) -> Result<DiffSummary, DiffError>`
* `DiffOp` includes **query ops**:

  * `QueryAdded`, `QueryRemoved`, `QueryRenamed`
  * `QueryDefinitionChanged { change_kind, old_hash, new_hash }`
  * `QueryMetadataChanged { field, old, new }`
* `diff_m_queries` now returns `Vec<DiffOp>` (not `Vec<MQueryDiff>`) and `MQueryDiff` becomes deprecated/internal.
* `PackageError` becomes the unified “open/package parse” error type (replacing the role of `ExcelOpenError`).

---

## 1) Add `WorkbookPackage` and unify open errors as `PackageError`

### 1.1 Create a new module: `core/src/package.rs`

Add `WorkbookPackage` and the new `open` API. The sprint plan wants `open<R: Read + Seek>` that parses both workbook + mashup in one go.

Create **new** file:

```rust
use crate::config::DiffConfig;
use crate::datamashup::DataMashup;
use crate::diff::{DiffError, DiffReport, DiffSummary};
use crate::sink::DiffSink;
use crate::workbook::Workbook;

#[derive(Debug, Clone)]
pub struct WorkbookPackage {
    pub workbook: Workbook,
    pub data_mashup: Option<DataMashup>,
}

impl WorkbookPackage {
    #[cfg(feature = "excel-open-xml")]
    pub fn open<R: std::io::Read + std::io::Seek + 'static>(reader: R) -> Result<Self, crate::excel_open_xml::PackageError> {
        crate::with_default_session(|session| {
            let mut container = crate::container::OpcContainer::open_from_reader(reader)?;
            let workbook = crate::excel_open_xml::open_workbook_from_container(&mut container, &mut session.strings)?;
            let raw = crate::excel_open_xml::open_data_mashup_from_container(&mut container)?;
            let data_mashup = match raw {
                Some(raw) => Some(crate::datamashup::build_data_mashup(&raw)?),
                None => None,
            };
            Ok(Self { workbook, data_mashup })
        })
    }

    pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
        crate::with_default_session(|session| {
            let mut report = crate::engine::diff_workbooks(&self.workbook, &other.workbook, &mut session.strings, config);

            let mut m_ops = crate::m_diff::diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            report.ops.append(&mut m_ops);
            report.strings = session.strings.strings().to_vec();
            report
        })
    }

    pub fn diff_streaming<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            let mut summary = crate::engine::try_diff_workbooks_streaming(
                &self.workbook,
                &other.workbook,
                &mut session.strings,
                config,
                sink,
            )?;

            let m_ops = crate::m_diff::diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            for op in m_ops {
                sink.emit(op)?;
                summary.op_count += 1;
            }

            Ok(summary)
        })
    }
}
```

Notes:

* This follows Branch 5’s single package model and unified diff method signatures.
* `WorkbookPackage::diff` re-snapshots `report.strings` after emitting M ops so newly-interned query strings are present.
* `diff_streaming` increments `summary.op_count` for M ops to keep summary consistent.

### 1.2 Rename `ExcelOpenError` to `PackageError` and keep a deprecated alias

Branch 5 explicitly calls for consolidating errors into `PackageError`. You already have a unified `ExcelOpenError` in `core/src/excel_open_xml.rs`.

Replace this block in `core/src/excel_open_xml.rs`:

```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExcelOpenError {
    #[error("container error: {0}")]
    Container(#[from] ContainerError),
    #[error("grid parse error: {0}")]
    GridParse(#[from] GridParseError),
    #[error("DataMashup error: {0}")]
    DataMashup(#[from] DataMashupError),
    #[error("workbook.xml missing or unreadable")]
    WorkbookXmlMissing,
    #[error("worksheet XML missing for sheet {sheet_name}")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("serialization error: {0}")]
    SerializationError(String),
}
```

with:

```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PackageError {
    #[error("container error: {0}")]
    Container(#[from] ContainerError),
    #[error("grid parse error: {0}")]
    GridParse(#[from] GridParseError),
    #[error("DataMashup error: {0}")]
    DataMashup(#[from] DataMashupError),
    #[error("workbook.xml missing or unreadable")]
    WorkbookXmlMissing,
    #[error("worksheet XML missing for sheet {sheet_name}")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("serialization error: {0}")]
    SerializationError(String),
}

#[deprecated(note = "use PackageError")]
pub type ExcelOpenError = PackageError;
```

This preserves existing tests that pattern-match `ExcelOpenError::...` while letting new API speak in terms of `PackageError`.

---

## 2) Refactor `excel_open_xml` to support container-based parsing

Branch 5 wants `WorkbookPackage::open` to parse both workbook + mashup from a single container. That means you need “from container” helpers. 

### 2.1 Add internal helpers

In `core/src/excel_open_xml.rs`, take the existing logic from `open_workbook` and `open_data_mashup` and extract:

* `pub(crate) fn open_workbook_from_container(container: &mut OpcContainer, pool: &mut StringPool) -> Result<Workbook, PackageError>`
* `pub(crate) fn open_data_mashup_from_container(container: &mut OpcContainer) -> Result<Option<RawDataMashup>, PackageError>`

#### Replace: `open_workbook(...)` function body

Old (current):

```rust
pub fn open_workbook(
    path: impl AsRef<Path>,
    pool: &mut StringPool,
) -> Result<Workbook, ExcelOpenError> {
    let mut container = OpcContainer::open_from_path(path.as_ref())?;

    let shared_strings = match container
        .read_file_optional("xl/sharedStrings.xml")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_shared_strings(&bytes, pool)?,
        None => Vec::new(),
    };

    let workbook_bytes = container
        .read_file("xl/workbook.xml")
        .map_err(|_| ExcelOpenError::WorkbookXmlMissing)?;

    let sheets = parse_workbook_xml(&workbook_bytes)?;

    let relationships = match container
        .read_file_optional("xl/_rels/workbook.xml.rels")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_relationships(&bytes)?,
        None => HashMap::new(),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes =
            container
                .read_file(&target)
                .map_err(|_| ExcelOpenError::WorksheetXmlMissing {
                    sheet_name: sheet.name.clone(),
                })?;
        let grid = parse_sheet_xml(&sheet_bytes, &shared_strings, pool)?;
        sheet_ir.push(Sheet {
            name: pool.intern(&sheet.name),
            kind: SheetKind::Worksheet,
            grid,
        });
    }

    Ok(Workbook { sheets: sheet_ir })
}
```

New:

```rust
pub(crate) fn open_workbook_from_container(
    container: &mut OpcContainer,
    pool: &mut StringPool,
) -> Result<Workbook, PackageError> {
    let shared_strings = match container
        .read_file_optional("xl/sharedStrings.xml")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_shared_strings(&bytes, pool)?,
        None => Vec::new(),
    };

    let workbook_bytes = container
        .read_file("xl/workbook.xml")
        .map_err(|_| PackageError::WorkbookXmlMissing)?;

    let sheets = parse_workbook_xml(&workbook_bytes)?;

    let relationships = match container
        .read_file_optional("xl/_rels/workbook.xml.rels")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_relationships(&bytes)?,
        None => HashMap::new(),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes = container
            .read_file(&target)
            .map_err(|_| PackageError::WorksheetXmlMissing {
                sheet_name: sheet.name.clone(),
            })?;
        let grid = parse_sheet_xml(&sheet_bytes, &shared_strings, pool)?;
        sheet_ir.push(Sheet {
            name: pool.intern(&sheet.name),
            kind: SheetKind::Worksheet,
            grid,
        });
    }

    Ok(Workbook { sheets: sheet_ir })
}

pub fn open_workbook(
    path: impl AsRef<Path>,
    pool: &mut StringPool,
) -> Result<Workbook, PackageError> {
    let mut container = OpcContainer::open_from_path(path.as_ref())?;
    open_workbook_from_container(&mut container, pool)
}
```

This keeps your existing API (path-based open) while enabling `WorkbookPackage::open(reader)` to reuse the same parser.

#### Replace: `open_data_mashup(...)` function body

Old (current):

```rust
pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, ExcelOpenError> {
    let mut container = OpcContainer::open_from_path(path.as_ref())?;
    let mut found: Option<RawDataMashup> = None;

    for i in 0..container.len() {
        let name = {
            let file = container.archive.by_index(i).ok();
            file.map(|f| f.name().to_string())
        };

        if let Some(name) = name {
            if !name.starts_with("customXml/") || !name.ends_with(".xml") {
                continue;
            }

            let bytes = container
                .read_file(&name)
                .map_err(|e| ContainerError::Zip(e.to_string()))?;

            if let Some(text) = read_datamashup_text(&bytes)? {
                let decoded = decode_datamashup_base64(&text)?;
                let parsed = parse_data_mashup(&decoded)?;
                if found.is_some() {
                    return Err(DataMashupError::FramingInvalid.into());
                }
                found = Some(parsed);
            }
        }
    }

    Ok(found)
}
```

New:

```rust
pub(crate) fn open_data_mashup_from_container(
    container: &mut OpcContainer,
) -> Result<Option<RawDataMashup>, PackageError> {
    let mut found: Option<RawDataMashup> = None;

    for i in 0..container.len() {
        let name = {
            let file = container.archive.by_index(i).ok();
            file.map(|f| f.name().to_string())
        };

        if let Some(name) = name {
            if !name.starts_with("customXml/") || !name.ends_with(".xml") {
                continue;
            }

            let bytes = container
                .read_file(&name)
                .map_err(|e| ContainerError::Zip(e.to_string()))?;

            if let Some(text) = read_datamashup_text(&bytes)? {
                let decoded = decode_datamashup_base64(&text)?;
                let parsed = parse_data_mashup(&decoded)?;
                if found.is_some() {
                    return Err(DataMashupError::FramingInvalid.into());
                }
                found = Some(parsed);
            }
        }
    }

    Ok(found)
}

pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, PackageError> {
    let mut container = OpcContainer::open_from_path(path.as_ref())?;
    open_data_mashup_from_container(&mut container)
}
```

Now `WorkbookPackage::open` can call `open_data_mashup_from_container` and then `build_data_mashup(&raw)`.

---

## 3) Extend `DiffOp` with M query operations (Branch 5.2)

Branch 5.2 requires new `DiffOp` variants and a new `QueryChangeKind` enum.

### 3.1 Update `core/src/diff.rs`

Add the new query ops and related enums to `diff.rs` so they are part of the schema (and used by both engine + consumers).

You don’t currently have these variants; the sprint plan lays out the shape. 

**Add these types near `DiffOp`:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueryChangeKind {
    Semantic,
    FormattingOnly,
    Renamed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueryMetadataField {
    LoadToSheet,
    LoadToModel,
    GroupPath,
    ConnectionOnly,
}
```

**Then extend `DiffOp`:**

```rust
pub enum DiffOp {
    // existing variants...

    QueryAdded { name: StringId },
    QueryRemoved { name: StringId },
    QueryRenamed { from: StringId, to: StringId },
    QueryDefinitionChanged {
        name: StringId,
        change_kind: QueryChangeKind,
        old_hash: u64,
        new_hash: u64,
    },
    QueryMetadataChanged {
        name: StringId,
        field: QueryMetadataField,
        old: Option<StringId>,
        new: Option<StringId>,
    },

    // Future: DAX operations (reserved)
    // MeasureAdded { .. },
    // MeasureRemoved { .. },
    // MeasureDefinitionChanged { .. },
}
```

This is compatible with the Branch 5 intent (“M query changes appear in same DiffOp stream”).

### 3.2 Update serialization tests

You already have serialization roundtrip tests for `DiffOp` (pg4). Add at least one new op instance for:

* `QueryAdded`
* `QueryDefinitionChanged`
* `QueryMetadataChanged`

so serde roundtrip includes the new variants.

---

## 4) Make the M AST hashable so query hashes are stable

Branch 5.2 requires `old_hash` / `new_hash` in `QueryDefinitionChanged`.

Your `MModuleAst` is currently `Clone + Debug + PartialEq + Eq` (no `Hash`). 

Replace this derive block:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MModuleAst {
    root: MExpr,
}
```

with:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MModuleAst {
    root: MExpr,
}
```

Then similarly add `Hash` to internal AST nodes:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MExpr { /* ... */ }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LetBinding { /* ... */ }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MToken { /* ... */ }
```

This lets you produce deterministic `u64` hashes using your existing hash utilities (you already depend on `xxhash-rust`).

---

## 5) Refactor M diff to emit `Vec<DiffOp>` (Branch 5.2)

Right now:

* `diff_m_queries(old_dm, new_dm, config) -> Result<Vec<MQueryDiff>, SectionParseError>`
* It suppresses formatting-only diffs when semantic diff is enabled.

Branch 5.2 wants:

* `diff_m_queries -> Vec<DiffOp>` (and `MQueryDiff` deprecated/internal).

### 5.1 Introduce an internal helper to produce M ops for packages

Add this to `core/src/m_diff.rs` (new function), so `WorkbookPackage::diff` can call it without exposing pool/session complexity publicly:

```rust
use crate::config::DiffConfig;
use crate::datamashup::{DataMashup, Query, build_queries};
use crate::diff::{DiffOp, QueryChangeKind, QueryMetadataField};
use crate::hashing::XXH64_SEED;
use crate::m_ast::{canonicalize_m_ast, parse_m_expression};
use crate::m_section::SectionParseError;
use crate::string_pool::{StringId, StringPool};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

fn hash64<T: Hash>(value: &T) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    value.hash(&mut h);
    h.finish()
}

fn intern_bool(pool: &mut StringPool, v: bool) -> StringId {
    if v {
        pool.intern("true")
    } else {
        pool.intern("false")
    }
}

fn canonical_ast_and_hash(expr: &str) -> Option<(crate::m_ast::MModuleAst, u64)> {
    let mut ast = parse_m_expression(expr).ok()?;
    canonicalize_m_ast(&mut ast);
    let h = hash64(&ast);
    Some((ast, h))
}

fn definition_change(
    old_expr: &str,
    new_expr: &str,
    enable_semantic: bool,
) -> Option<(QueryChangeKind, u64, u64)> {
    if old_expr == new_expr {
        return None;
    }

    if enable_semantic {
        if let (Some((_a, old_h)), Some((_b, new_h))) = (canonical_ast_and_hash(old_expr), canonical_ast_and_hash(new_expr)) {
            let kind = if old_h == new_h {
                QueryChangeKind::FormattingOnly
            } else {
                QueryChangeKind::Semantic
            };
            return Some((kind, old_h, new_h));
        }
    }

    let old_h = hash64(&old_expr);
    let new_h = hash64(&new_expr);
    Some((QueryChangeKind::Semantic, old_h, new_h))
}

fn emit_metadata_diffs(
    pool: &mut StringPool,
    out: &mut Vec<DiffOp>,
    name: StringId,
    old_q: &Query,
    new_q: &Query,
) {
    if old_q.metadata.load_to_worksheet != new_q.metadata.load_to_worksheet {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::LoadToSheet,
            old: Some(intern_bool(pool, old_q.metadata.load_to_worksheet)),
            new: Some(intern_bool(pool, new_q.metadata.load_to_worksheet)),
        });
    }

    if old_q.metadata.load_to_model != new_q.metadata.load_to_model {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::LoadToModel,
            old: Some(intern_bool(pool, old_q.metadata.load_to_model)),
            new: Some(intern_bool(pool, new_q.metadata.load_to_model)),
        });
    }

    if old_q.metadata.is_connection_only != new_q.metadata.is_connection_only {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::ConnectionOnly,
            old: Some(intern_bool(pool, old_q.metadata.is_connection_only)),
            new: Some(intern_bool(pool, new_q.metadata.is_connection_only)),
        });
    }

    if old_q.metadata.group_path != new_q.metadata.group_path {
        let old = old_q.metadata.group_path.as_deref().map(|s| pool.intern(s));
        let new = new_q.metadata.group_path.as_deref().map(|s| pool.intern(s));
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::GroupPath,
            old,
            new,
        });
    }
}

fn diff_queries_to_ops(
    old_queries: &[Query],
    new_queries: &[Query],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut old_by_name: BTreeMap<&str, &Query> = BTreeMap::new();
    let mut new_by_name: BTreeMap<&str, &Query> = BTreeMap::new();

    for q in old_queries {
        old_by_name.insert(q.name.as_str(), q);
    }
    for q in new_queries {
        new_by_name.insert(q.name.as_str(), q);
    }

    let old_only: Vec<&Query> = old_by_name
        .iter()
        .filter_map(|(name, q)| if new_by_name.contains_key(*name) { None } else { Some(*q) })
        .collect();

    let new_only: Vec<&Query> = new_by_name
        .iter()
        .filter_map(|(name, q)| if old_by_name.contains_key(*name) { None } else { Some(*q) })
        .collect();

    let mut renamed_old: BTreeSet<&str> = BTreeSet::new();
    let mut renamed_new: BTreeSet<&str> = BTreeSet::new();
    let mut rename_ops: Vec<(StringId, StringId, &Query, &Query)> = Vec::new();

    let mut old_hash_map: BTreeMap<u64, Vec<&Query>> = BTreeMap::new();
    let mut new_hash_map: BTreeMap<u64, Vec<&Query>> = BTreeMap::new();

    for q in &old_only {
        let h = if config.enable_m_semantic_diff {
            canonical_ast_and_hash(&q.expression_m).map(|(_, h)| h).unwrap_or_else(|| hash64(&q.expression_m))
        } else {
            hash64(&q.expression_m)
        };
        old_hash_map.entry(h).or_default().push(*q);
    }

    for q in &new_only {
        let h = if config.enable_m_semantic_diff {
            canonical_ast_and_hash(&q.expression_m).map(|(_, h)| h).unwrap_or_else(|| hash64(&q.expression_m))
        } else {
            hash64(&q.expression_m)
        };
        new_hash_map.entry(h).or_default().push(*q);
    }

    for (h, olds) in &old_hash_map {
        if let Some(news) = new_hash_map.get(h) {
            if olds.len() == 1 && news.len() == 1 {
                let old_q = olds[0];
                let new_q = news[0];
                let from = pool.intern(old_q.name.as_str());
                let to = pool.intern(new_q.name.as_str());
                renamed_old.insert(old_q.name.as_str());
                renamed_new.insert(new_q.name.as_str());
                rename_ops.push((from, to, old_q, new_q));
            }
        }
    }

    rename_ops.sort_by(|a, b| {
        let from_a = pool.resolve(a.0);
        let from_b = pool.resolve(b.0);
        from_a.cmp(from_b)
    });

    let mut ops: Vec<DiffOp> = Vec::new();

    for (from, to, old_q, new_q) in rename_ops {
        ops.push(DiffOp::QueryRenamed { from, to });
        emit_metadata_diffs(pool, &mut ops, to, old_q, new_q);
    }

    let mut all_names: Vec<&str> = old_by_name
        .keys()
        .copied()
        .chain(new_by_name.keys().copied())
        .collect();
    all_names.sort();
    all_names.dedup();

    for name in all_names {
        if renamed_old.contains(name) || renamed_new.contains(name) {
            continue;
        }

        match (old_by_name.get(name), new_by_name.get(name)) {
            (None, Some(new_q)) => {
                ops.push(DiffOp::QueryAdded { name: pool.intern(name) });
                let _ = new_q;
            }
            (Some(old_q), None) => {
                ops.push(DiffOp::QueryRemoved { name: pool.intern(name) });
                let _ = old_q;
            }
            (Some(old_q), Some(new_q)) => {
                let name_id = pool.intern(name);

                if let Some((kind, old_h, new_h)) = definition_change(
                    &old_q.expression_m,
                    &new_q.expression_m,
                    config.enable_m_semantic_diff,
                ) {
                    ops.push(DiffOp::QueryDefinitionChanged {
                        name: name_id,
                        change_kind: kind,
                        old_hash: old_h,
                        new_hash: new_h,
                    });
                }

                emit_metadata_diffs(pool, &mut ops, name_id, old_q, new_q);
            }
            (None, None) => {}
        }
    }

    ops
}

pub(crate) fn diff_m_ops_for_packages(
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    match (old_dm.as_ref(), new_dm.as_ref()) {
        (None, None) => Vec::new(),
        (Some(old_dm), None) => {
            let old_q = match build_queries(old_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let mut ops = Vec::new();
            for q in old_q {
                ops.push(DiffOp::QueryRemoved { name: pool.intern(&q.name) });
            }
            ops
        }
        (None, Some(new_dm)) => {
            let new_q = match build_queries(new_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let mut ops = Vec::new();
            for q in new_q {
                ops.push(DiffOp::QueryAdded { name: pool.intern(&q.name) });
            }
            ops
        }
        (Some(old_dm), Some(new_dm)) => {
            let old_q = match build_queries(old_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let new_q = match build_queries(new_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            diff_queries_to_ops(&old_q, &new_q, pool, config)
        }
    }
}
```

What this gives you:

* `QueryDefinitionChanged` always emitted when text differs, but classifies formatting-only vs semantic when semantic diff is enabled. This matches Branch 5’s new `QueryChangeKind` intent.
* A minimal rename detector that upgrades a remove+add into `QueryRenamed` when the (canonical) hash matches uniquely. (The longer-term “Hungarian matching” rename logic described in your specification can come later.) 
* Metadata changes are field-level and typed (`QueryMetadataField`), rather than the prior coarse `MetadataChangedOnly`.

### 5.2 Deprecate `MQueryDiff` and the old `QueryChangeKind`

The old `m_diff.rs` exports `MQueryDiff` and a different `QueryChangeKind` that conflicts with the Branch 5 definition.

You have two viable options:

**Option A (cleaner, Branch 5 aligned):**

* Remove `MQueryDiff` entirely (or make it `pub(crate)`).
* Remove old `QueryChangeKind` and rely on the schema one in `diff.rs`.

**Option B (more conservative):**

* Move old types into `m_diff_legacy` module, and keep them behind a `legacy-api` feature.

Branch 5 explicitly allows “deprecate or make internal”.

---

## 6) Implement unified `WorkbookPackage::diff` and `diff_streaming` (Branch 5.3)

Branch 5.3 shows `WorkbookPackage::diff` as the unifying API.

The `package.rs` code above already provides these methods, but you should also ensure:

* If `build_queries(...)` fails (Section parse error), you either:

  * emit no M ops (current skeleton does that), or
  * attach a warning to the report/summary.

Given `DiffReport` already has `warnings`, I recommend adding a warning so failures are visible (but do not crash the entire workbook diff). This preserves the strong behavior of workbook diffs even if Power Query text is malformed.

If you choose to surface warnings, adjust `diff_m_ops_for_packages` to return `(Vec<DiffOp>, Option<String>)` and plumb the warning into `report.warnings` / `summary.warnings`.

---

## 7) Add `DiffReport::grid_ops()` and `DiffReport::m_ops()` helpers

Branch 5 calls out projection helpers.

Add to `core/src/diff.rs`:

```rust
impl DiffOp {
    pub fn is_m_op(&self) -> bool {
        matches!(
            self,
            DiffOp::QueryAdded { .. }
                | DiffOp::QueryRemoved { .. }
                | DiffOp::QueryRenamed { .. }
                | DiffOp::QueryDefinitionChanged { .. }
                | DiffOp::QueryMetadataChanged { .. }
        )
    }
}

impl DiffReport {
    pub fn grid_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| !op.is_m_op())
    }

    pub fn m_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| op.is_m_op())
    }
}
```

This makes it easy for downstream tooling to ignore M ops when producing “cell diff only” outputs.

---

## 8) Update `core/src/lib.rs`: export new API, deprecate old entry points, update docs

Your crate doc “Quick Start” currently uses `open_workbook` + `diff_workbooks`. 

### 8.1 Update the Quick Start example

Replace:

```rust
//! use excel_diff::{open_workbook, diff_workbooks};
//!
//! let wb_a = open_workbook("file_a.xlsx")?;
//! let wb_b = open_workbook("file_b.xlsx")?;
//! let report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());
```

with:

```rust
//! use excel_diff::WorkbookPackage;
//!
//! let pkg_a = WorkbookPackage::open(std::fs::File::open("file_a.xlsx")?)?;
//! let pkg_b = WorkbookPackage::open(std::fs::File::open("file_b.xlsx")?)?;
//! let report = pkg_a.diff(&pkg_b, &excel_diff::DiffConfig::default());
```

### 8.2 Export `WorkbookPackage` and stop re-exporting legacy M diff types

Your current `lib.rs` re-exports `MQueryDiff` and the old `QueryChangeKind`.

Replace:

```rust
pub mod m_diff;
// ...
pub use m_diff::{MQueryDiff, QueryChangeKind, diff_m_queries};
```

with:

```rust
pub mod package;
pub mod m_diff;

pub use package::WorkbookPackage;
pub use diff::{QueryChangeKind, QueryMetadataField};
```

And either remove `diff_m_queries` from the root exports, or keep it but deprecated.

### 8.3 Deprecate legacy functions

In `lib.rs`, mark these as deprecated (Branch 5 wants a clear migration path):

* `open_workbook(...)`
* `open_data_mashup(...)`
* `diff_workbooks(...)`
* `diff_m_queries(...)`

Example:

```rust
#[deprecated(note = "use WorkbookPackage::diff")]
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport { ... }
```

This keeps your existing users unbroken while clearly pushing them to the package API.

---

## 9) Update tests for the new schema and new query diff behavior

### 9.1 Rewrite `m6_textual_m_diff_tests.rs` to assert `DiffOp`s

Current tests assert `diffs[i].name` as a `String` and `diff.kind` as the old `QueryChangeKind`.

After Branch 5, these should instead pattern-match `DiffOp`.

Example: “basic add query diff” becomes:

```rust
let diffs = excel_diff::m_diff::diff_m_ops_for_packages(
    &Some(dm_a),
    &Some(dm_b),
    &mut excel_diff::with_default_session(|s| s.strings.clone()),
    &excel_diff::DiffConfig::default(),
);
```

But tests shouldn’t clone pools like that; instead, test through `WorkbookPackage` or through a small helper that calls the internal diff with the default session pool.

Recommended test strategy:

* Build `WorkbookPackage { workbook: empty, data_mashup: Some(dm) }` and call `pkg_a.diff(&pkg_b, config)`.
* Then filter: `report.m_ops().collect::<Vec<_>>()`.
* Assert on `DiffOp` patterns and use `report.strings[id.0 as usize]` to resolve names.

That also validates the end-to-end Branch 5 behavior (unified diff).

### 9.2 Update “formatting only” expectations

Previously you suppressed definition diffs when semantic diff says “equal”.

With Branch 5 `QueryChangeKind::FormattingOnly`, update tests to expect a `QueryDefinitionChanged` op with:

* `change_kind == FormattingOnly`
* `old_hash == new_hash`

That matches the new schema intent and gives consumers the option to ignore formatting-only changes themselves.

### 9.3 Update rename test

Previously `rename_reports_add_and_remove` expected 2 diffs. 

After implementing the minimal rename detector, change to expect:

* `DiffOp::QueryRenamed { from: sid("Section1/Foo"), to: sid("Section1/Bar") }`

(and optionally metadata diffs if any).

---

## 10) Final integration checklist for Branch 5 completion

Use this checklist to ensure you’ve hit everything in the Branch 5 plan.

### 10.1 Branch 5.1

* [ ] `WorkbookPackage` exists and is exported
* [ ] `WorkbookPackage::open(reader)` parses workbook + mashup from one container
* [ ] Error type unified as `PackageError` (with deprecated `ExcelOpenError` alias)
* [ ] Docs updated to show package API

### 10.2 Branch 5.2

* [ ] `DiffOp` contains M query variants
* [ ] `QueryChangeKind` in schema matches Branch 5
* [ ] `diff_m_queries` no longer returns `Vec<MQueryDiff>`
* [ ] `MQueryDiff` deprecated or internal
* [ ] Serialization tests cover query variants

### 10.3 Branch 5.3

* [ ] `WorkbookPackage::diff` returns a single `DiffReport` combining grid + query ops
* [ ] `WorkbookPackage::diff_streaming` streams both grid + query ops
* [ ] `DiffReport::grid_ops()` and `DiffReport::m_ops()` implemented

---

If you want, I can also produce a “migration notes” section you can drop into your README (old API -> new API mappings), but the plan above is enough to implement Branch 5 end-to-end in your current structure.
