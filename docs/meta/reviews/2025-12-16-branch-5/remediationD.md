I reviewed your three updated artifacts against **Branch 5: API Unification** acceptance criteria (“single `WorkbookPackage::open`, single `WorkbookPackage::diff`, M query changes emitted in the same `DiffOp` stream, old APIs deprecated with a clear migration path, and reduced public API surface”). 

## What looks complete (Branch 5 core deliverables)

### 1) `WorkbookPackage` exists and is the new “unit of diff”

Your `core/src/package.rs` defines `WorkbookPackage { workbook, data_mashup }` and provides `open(...)`, `diff(...)`, and `diff_streaming(...)`. 

### 2) Unified error type is in place

You have an `enum PackageError` and `ExcelOpenError` is now a **deprecated alias** pointing at `PackageError` (which matches the migration guidance you wanted). 

### 3) PowerQuery / “M” changes are represented as `DiffOp` and flow through the unified API

`DiffOp` includes query-related variants like `QueryAdded`, `QueryRemoved`, `QueryRenamed`, `QueryDefinitionChanged`, and `QueryMetadataChanged`. 
And package-level diffing computes M ops and appends/emits them in the same run. 

### 4) Benchmarks are updated for Branch 5

Your `benchmark_results.json` is on `git_branch: 2025-12-16-branch-5` and reports 5 perf tests totaling `29934 ms`. 

So: **the Branch 5 “new path” exists and is wired together.**

---

## What is *not* “closed” yet (remaining gaps)

Even though the functionality is present, your latest `cycle_summary.txt` shows you still have a *large* amount of cleanup to do before this branch is “complete” in the sense of being tidy, warning-free, and fully migrated:

### Gap A — The test suite still heavily uses deprecated APIs (261 warnings)

Your test run is successful, but you’re generating tons of warnings from:

* `open_workbook` (deprecated → should use `WorkbookPackage::open`)
* `diff_workbooks` (deprecated → should use `WorkbookPackage::diff`)
* `ExcelOpenError` alias (deprecated → should use `PackageError`)  

This undermines “clear migration path” because your own codebase isn’t following it yet.

### Gap B — A couple of dead-code / unused warnings remain

You still have:

* An unused helper function `attach_strings` in `pg4_diffop_tests.rs` 
* A private unused helper `diff_grids` in `core/src/engine.rs`  
* An unused import `Cell` in `core/tests/engine_tests.rs` (per the warning text in the run log) 

### Gap C — “Public API surface reduced” is not clearly satisfied yet

Branch 5 explicitly calls for a reduced/cleaner public surface. 
But `core/src/lib.rs` still exposes a lot of modules/re-exports (including `engine` etc.), which likely violates the spirit of that criterion unless you’re planning to hide them via `#[doc(hidden)]` or feature-gate internals. 

---

# Patch plan to close the remaining gaps (with code)

Below is a concrete, safe plan that gets you to “Branch 5 complete” status without changing the core algorithm again.

## 1) Add a shared test helper module so migrations are mechanical

Create: `core/tests/common/mod.rs`

```rust
use std::fs::File;
use std::path::{Path, PathBuf};

use excel_diff::{DiffConfig, DiffReport, Workbook, WorkbookPackage};

pub fn fixture_path(name: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

pub fn open_fixture_pkg(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).unwrap_or_else(|e| {
        panic!("failed to open fixture {}: {e}", path.display());
    });

    WorkbookPackage::open(file).unwrap_or_else(|e| {
        panic!("failed to parse fixture {}: {e}", path.display());
    })
}

pub fn open_fixture_workbook(name: &str) -> Workbook {
    open_fixture_pkg(name).workbook
}

pub fn diff_fixture_pkgs(a: &str, b: &str, config: &DiffConfig) -> DiffReport {
    let pkg_a = open_fixture_pkg(a);
    let pkg_b = open_fixture_pkg(b);
    pkg_a.diff(&pkg_b, config)
}
```

**Why this matters:** Once this exists, updating ~20+ test files becomes mostly “replace two lines”.

---

## 2) Provide a constructor for “in-memory” tests (optional but makes migrations cleaner)

In `core/src/package.rs`, add:

```rust
use crate::workbook::Workbook;

impl WorkbookPackage {
    pub fn from_workbook(workbook: Workbook) -> Self {
        Self {
            workbook,
            data_mashup: None,
        }
    }
}

impl From<Workbook> for WorkbookPackage {
    fn from(workbook: Workbook) -> Self {
        Self::from_workbook(workbook)
    }
}
```

This lets tests replace:

```rust
diff_workbooks(&old, &new, &config)
```

with:

```rust
WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &config)
```

…without touching the workbook builders.

---

## 3) Migrate test files off deprecated APIs (bulk mechanical edits)

### 3.1 Replace `open_workbook(...)` usage

**Before** (pattern seen across tests) :

```rust
use excel_diff::open_workbook;

let wb_a = open_workbook(fixture_path("a.xlsx")).expect("open A");
```

**After**:

```rust
mod common;
use common::open_fixture_workbook;

let wb_a = open_fixture_workbook("a.xlsx");
```

Or if you need the mashup too:

```rust
mod common;
use common::open_fixture_pkg;

let pkg_a = open_fixture_pkg("a.xlsx");
```

### 3.2 Replace `diff_workbooks(&wb_a, &wb_b, config)` usage

**Before** :

```rust
use excel_diff::diff_workbooks;

let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
```

**After**:

```rust
use excel_diff::DiffConfig;

let pkg_old = excel_diff::WorkbookPackage::from_workbook(old);
let pkg_new = excel_diff::WorkbookPackage::from_workbook(new);

let report = pkg_old.diff(&pkg_new, &DiffConfig::default());
```

Or if both are fixtures:

```rust
mod common;
use common::diff_fixture_pkgs;

let report = diff_fixture_pkgs("old.xlsx", "new.xlsx", &excel_diff::DiffConfig::default());
```

### 3.3 Migrate `ExcelOpenError` usages in tests → `PackageError`

Your run log shows multiple warnings where tests construct/match `ExcelOpenError::SerializationError(...)`. 

**Before**:

```rust
use excel_diff::ExcelOpenError;

let wrapped = ExcelOpenError::SerializationError(err.to_string());
```

**After**:

```rust
use excel_diff::PackageError;

let wrapped = PackageError::SerializationError(err.to_string());
```

And update matches accordingly:

```rust
match wrapped {
    PackageError::SerializationError(msg) => { /* ... */ }
    other => panic!("unexpected error: {other:?}"),
}
```

### 3.4 Suggested “triage” list (start with biggest offenders)

From the warnings summary, these are high-churn files to update first (counts approximate from the warning log):

* `pg6_object_vs_grid_tests.rs` 
* `pg5_grid_diff_tests.rs` 
* `g5_g7_grid_workbook_tests.rs`, `g1_g2_grid_workbook_tests.rs`, `g8_row_alignment_grid_workbook_tests.rs` 
* `output_tests.rs` (ExcelOpenError alias usage) 
* `amr_multi_gap_tests.rs` etc. 

---

## 4) Remove/resolve the remaining non-deprecation warnings

### 4.1 Remove the unused `Cell` import

In `core/tests/engine_tests.rs`, remove `Cell` from the import list (or use it if it’s meant to be used). Your run log explicitly calls out that unused import. 

### 4.2 Delete or annotate the unused test helper `attach_strings`

In `core/tests/pg4_diffop_tests.rs`, either:

* delete `fn attach_strings(...)` if it’s not needed, or
* mark it:

```rust
#[allow(dead_code)]
fn attach_strings(mut report: DiffReport) -> DiffReport {
    // ...
}
```

The warning is currently showing it’s never used. 

### 4.3 Delete or annotate the unused `diff_grids` helper in `engine.rs`

Since `diff_grids` is currently unused, either delete it outright or mark it:

```rust
#[allow(dead_code)]
fn diff_grids<S: DiffSink>( /* ... */ ) -> Result<(), DiffError> {
    // ...
}
```

The warning is coming from the crate build.  

---

## 5) Close the “public API surface reduced” criterion

Branch 5’s acceptance criteria explicitly expects the public surface to shrink/clean up. 
Right now `lib.rs` still exports a lot. 

Here are two approaches—pick based on how strict you want to be:

### Option A (non-breaking): hide internals from docs

Keep API compatibility, but make docs present the clean unified model:

```rust
// in core/src/lib.rs

#[doc(hidden)]
pub mod engine;

#[doc(hidden)]
pub mod grid_parser;

// ...repeat for other internal-ish modules...
```

Also consider `#[doc(hidden)]` on “expert” re-exports.

### Option B (stricter, may be breaking): make internals private or feature-gated

Make internal modules private by default, and only expose them behind a feature:

```rust
// always compile the module
mod engine;

// only re-export it publicly when feature is enabled
#[cfg(feature = "internals")]
pub use crate::engine as engine;
```

Repeat for other internal modules.

This makes the “happy path” API obviously:

* `WorkbookPackage`
* `DiffConfig`
* `DiffOp`, `DiffReport`, `DiffSummary`
* `PackageError`
* output helpers

…and pushes everything else behind an opt-in.

---

## 6) Verification checklist (what to expect after patching)

After applying the above:

1. `cargo test` should pass **with dramatically fewer warnings** (ideally zero).
2. `cycle_summary.txt` should no longer show deprecated warnings for `open_workbook`, `diff_workbooks`, or `ExcelOpenError`.  
3. You should still be able to keep **one** small “deprecated API smoke test” (optional) if you want to ensure the wrappers still behave—just add `#![allow(deprecated)]` at the top of that one file.

---

If you want, I can also provide a **search/replace cookbook** (regex-style) for the three main migrations (`open_workbook`, `diff_workbooks`, `ExcelOpenError`) to make this a 10-minute mechanical sweep.
