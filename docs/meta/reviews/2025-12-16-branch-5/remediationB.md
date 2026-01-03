I reviewed the updated `codebase_context.md` against the Branch 5 requirements in `next_sprint_plan.md`.

## What looks complete

* **`WorkbookPackage` exists and is doing the “package = workbook + DataMashup” aggregation**, with `open`, `diff`, and `diff_streaming` implemented. `diff` merges M-query ops into the same `DiffReport.ops` stream as grid changes, and refreshes the `strings` table afterward. 
* **`DiffOp` has M-query variants** (added/removed/renamed/definition-changed/metadata-changed), and **`DiffReport` has `grid_ops()` and `m_ops()` helpers**.
* **Old APIs are deprecated with a migration note** pointing to the package API. 

So the “happy path” unified API is mostly there.

## What is still unfinished or incorrect vs the plan

### 1) `diff_m_queries` is still the legacy API (wrong return type)

Branch 5.2 explicitly calls for `diff_m_queries` to return unified `DiffOp` (and take query slices) instead of returning `MQueryDiff`. 
But your `m_diff::diff_m_queries` still returns `Result<Vec<MQueryDiff>, ...>`, and the legacy `MQueryDiff` / legacy `QueryChangeKind` are still defined. 

This is also the source of at least one warning in the build output you attached. 

### 2) Missing the “reserved DAX placeholders” comment block in `DiffOp`

Branch 5.2 asks you to add reserved/commented variants for future DAX ops. 
Your `DiffOp` enum currently ends after `QueryMetadataChanged` with no such reserved/commented section. 

### 3) `WorkbookPackage` is module-gated behind `excel-open-xml`, but `diff_workbooks` is deprecated everywhere

Right now, `package` (and thus `WorkbookPackage`) is only compiled when `excel-open-xml` is enabled. 
But Branch 5’s migration story deprecates `diff_workbooks` in favor of `WorkbookPackage::diff`. In **no-default-features** builds (you have a wasm/no-default-features workflow), that migration target may not exist, because `excel-open-xml` is not a default-less feature in wasm.

This is a migration-footgun: the deprecation note points at an API that may not be available in that build configuration.

### 4) Public JSON diff helper still diffs only the workbook, not the full package

`output::json::diff_workbooks(...)` still opens only the workbook and runs the workbook diff. It does **not** open/build the DataMashup and therefore **cannot include M-query changes** in JSON output even though Branch 5’s unified stream expects them to show up there too.

---

# Patch plan

Below are concrete, minimal patches to close the gaps above.

---

## Patch 1: Make `WorkbookPackage` always available (gate only `open`)

Goal: `WorkbookPackage` is the unified model in all builds; only `WorkbookPackage::open` is feature-gated.

### 1A) `core/src/lib.rs`: ungate the module + re-export, and stop re-exporting legacy `m_diff` items

**Replace this code:**

```rust
pub mod output;

#[cfg(feature = "excel-open-xml")]
pub mod package;
```

**With this code:**

```rust
pub mod output;
pub mod package;
```



---

**Replace this code:**

```rust
#[allow(deprecated)]
pub use m_diff::{MQueryDiff, QueryChangeKind as LegacyQueryChangeKind, diff_m_queries};

#[cfg(feature = "excel-open-xml")]
pub use package::WorkbookPackage;
```

**With this code:**

```rust
pub use package::WorkbookPackage;
```



Notes:

* This aligns with “public API surface reduced” and avoids re-exporting deprecated legacy M-diff types at the crate root.
* Users can still access `excel_diff::m_diff::...` if you keep the module public, but it’s no longer promoted at the top level.

### 1B) `core/src/package.rs`: no structural change required

You already have `#[cfg(feature = "excel-open-xml")]` directly on `WorkbookPackage::open`, and you use fully qualified paths inside it. 
So once the module is ungated in `lib.rs`, this file should still compile in non-`excel-open-xml` builds because the `open` method is not compiled.

---

## Patch 2: Implement Branch 5.2 `diff_m_queries` (unified `DiffOp`) and remove legacy `MQueryDiff`

Goal: match the plan: `diff_m_queries(old_queries, new_queries, config) -> Vec<DiffOp>` and eliminate the internal deprecation warning.

### 2A) `core/src/m_diff.rs`: replace the entire legacy block with the unified wrapper

**Replace the legacy block that defines**:

* `QueryChangeKind` (deprecated legacy)
* `MQueryDiff` (deprecated legacy)
* `diff_m_queries(old_dm, new_dm, ...) -> Result<Vec<MQueryDiff>, ...>`
* `diff_queries_legacy(...) -> Vec<MQueryDiff>`

This is the block currently present. 

**With this code:**

```rust
use std::collections::{BTreeMap, BTreeSet};

use crate::datamashup::{DataMashup, Query, QueryMetadata};
use crate::diff::{DiffOp, QueryChangeKind as DiffQueryChangeKind, QueryMetadataField};
use crate::diff_config::DiffConfig;
use crate::string_pool::{StringId, StringPool};

#[deprecated(note = "use WorkbookPackage::diff instead")]
pub fn diff_m_queries(old_queries: &[Query], new_queries: &[Query], config: &DiffConfig) -> Vec<DiffOp> {
    crate::with_default_session(|session| {
        diff_queries_to_ops(old_queries, new_queries, &mut session.strings, config)
    })
}
```

This implements the **exact “After” shape** from the plan (unified ops, query slices). 

Why this shape works with your current interned-string model:

* `diff_queries_to_ops(...)` already needs a `StringPool` to intern query names/metadata into `StringId`.
* This wrapper uses the same default session pool as the rest of the new “easy mode” API.

### 2B) Leave these internal helpers intact

Do not remove:

* `diff_m_ops_for_packages(...)` (used by `WorkbookPackage::diff`)
* `diff_queries_to_ops(...)` (the real implementation)

They’re already aligned with emitting `DiffOp` variants.

---

## Patch 3: Add reserved DAX placeholders in `DiffOp` (comment-only)

Goal: satisfy the explicit “reserved/commented variants for DAX” item in the plan. 

In `core/src/diff.rs`, locate the end of `DiffOp`:

**Replace this code:**

```rust
    QueryMetadataChanged {
        name: StringId,
        field: QueryMetadataField,
        old: Option<StringId>,
        new: Option<StringId>,
    },
}
```

**With this code:**

```rust
    QueryMetadataChanged {
        name: StringId,
        field: QueryMetadataField,
        old: Option<StringId>,
        new: Option<StringId>,
    },

    // Future: DAX operations
    // MeasureAdded { name: StringId }
    // MeasureRemoved { name: StringId }
    // MeasureDefinitionChanged { name: StringId, change_kind: QueryChangeKind }
}
```

---

## Patch 4: Update `output::json::diff_workbooks` to include DataMashup and emit M-query ops

Goal: if a consumer uses the public JSON helper, they should get the same unified behavior as `WorkbookPackage::diff`.

### 4A) `core/src/output/json.rs`: expand imports and update `diff_workbooks`

**Replace this import:**

```rust
use crate::excel_open_xml::{PackageError, open_workbook};
```

**With this import block:**

```rust
use crate::datamashup::build_data_mashup;
use crate::excel_open_xml::{PackageError, open_data_mashup, open_workbook};
```



---

**Replace the existing `diff_workbooks(...)` implementation:**

```rust
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<DiffReport, PackageError> {
    let mut session = DiffSession::new();

    let wb_a = open_workbook(path_a, session.strings_mut())?;
    let wb_b = open_workbook(path_b, session.strings_mut())?;

    let mut sink = VecSink::new();
    let summary = crate::engine::try_diff_workbooks_streaming(
        &wb_a,
        &wb_b,
        session.strings_mut(),
        config,
        &mut sink,
    )
    .map_err(|e| PackageError::SerializationError(e.to_string()))?;

    Ok(build_report_from_sink(sink, summary, session))
}
```

**With this unified implementation:**

```rust
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<DiffReport, PackageError> {
    let path_a = path_a.as_ref();
    let path_b = path_b.as_ref();

    let mut session = DiffSession::new();

    let wb_a = open_workbook(path_a, session.strings_mut())?;
    let wb_b = open_workbook(path_b, session.strings_mut())?;

    let dm_a = open_data_mashup(path_a)?
        .map(|raw| build_data_mashup(&raw))
        .transpose()?;
    let dm_b = open_data_mashup(path_b)?
        .map(|raw| build_data_mashup(&raw))
        .transpose()?;

    let mut sink = VecSink::new();
    let summary = crate::engine::try_diff_workbooks_streaming(
        &wb_a,
        &wb_b,
        session.strings_mut(),
        config,
        &mut sink,
    )
    .map_err(|e| PackageError::SerializationError(e.to_string()))?;

    let m_ops = crate::m_diff::diff_m_ops_for_packages(&dm_a, &dm_b, session.strings_mut(), config);

    let mut report = build_report_from_sink(sink, summary, session);
    report.ops.extend(m_ops);
    Ok(report)
}
```

Why this specific structure:

* It preserves your current **deterministic, per-call `DiffSession`** behavior for JSON output (important for tests and repeated calls).
* It emits M-query ops using the same internal machinery `WorkbookPackage::diff` uses, and ensures any new strings interned during M diff are included because the `DiffSession` is consumed only after M ops are computed.

---

## Optional cleanup: eliminate the remaining deprecation warnings in tests

You still have warnings from deprecated APIs during tests (and at least one internal warning from the legacy M types).

After Patch 2, the internal warning from `m_diff::QueryChangeKind` should disappear. The remaining warnings are mostly from tests that still call deprecated `open_workbook` / `diff_workbooks`.

A straightforward migration pattern for tests that read `.xlsx` fixtures:

**Replace:**

```rust
let wb_a = open_workbook(&path_a)?;
let wb_b = open_workbook(&path_b)?;
let report = diff_workbooks(&wb_a, &wb_b, &config);
```

**With:**

```rust
let pkg_a = WorkbookPackage::open(File::open(&path_a)?)?;
let pkg_b = WorkbookPackage::open(File::open(&path_b)?)?;
let report = pkg_a.diff(&pkg_b, &config);
```

This keeps behavior consistent while using the unified API.

---

If you apply **Patches 1–4**, you’ll be aligned with the Branch 5 spec items that are currently mismatched: the unified `diff_m_queries` API, the reserved DAX placeholders, the feature gating/migration correctness, and package-aware JSON output.
