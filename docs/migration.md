# Migration Guide

[Docs index](index.md)

The `excel_diff` crate still exposes a few deprecated legacy entry points for compatibility. New code should prefer `WorkbookPackage` and the higher-level APIs.

## Old API -> New API

### Open a workbook

Old:

```rust
// Deprecated (and feature-gated)
let wb = excel_diff::open_workbook("file.xlsx")?;
```

New:

```rust
use excel_diff::WorkbookPackage;
use std::fs::File;

let pkg = WorkbookPackage::open(File::open("file.xlsx")?)?;
```

### Diff two workbooks

Old:

```rust
let report = excel_diff::diff_workbooks(&old_wb, &new_wb, &cfg);
```

New:

```rust
let report = old_pkg.diff(&new_pkg, &cfg);
```

### Prefer fallible/streaming APIs

If you previously relied on `try_diff_workbooks(...)` for error handling, consider streaming:

```rust
use excel_diff::{DiffConfig, JsonLinesSink, WorkbookPackage};
use std::fs::File;
use std::io::{self, BufWriter};

let old = WorkbookPackage::open(File::open("old.xlsx")?)?;
let new = WorkbookPackage::open(File::open("new.xlsx")?)?;

let mut sink = JsonLinesSink::new(BufWriter::new(io::stdout().lock()));
let summary = old.diff_streaming(&new, &DiffConfig::default(), &mut sink)?;
```

## String pool / session notes

`WorkbookPackage` uses a default thread-local session internally. For power users who want to manage their own string pool and reuse allocations across many diffs, see:

- Rust: `excel_diff::advanced` (e.g., `DiffSession`, `diff_workbooks_with_pool`, streaming entry points)

## Database mode

Database mode is exposed on `WorkbookPackage`:

- `diff_database_mode(...)` and streaming variants

See [Database mode](database_mode.md).

## Feature flags

The crate uses feature flags for optional functionality. Defaults include:

- `excel-open-xml` (parsing `.xlsx`)
- `std-fs` (path-based helpers)
- `vba` (VBA module extraction)

WASM builds typically disable heavier features.
