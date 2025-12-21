## Branch 1 target behavior

Branch 1 (`2025-12-22-pbix-host-support`) is about making `.pbix` / `.pbit` first-class “hosts” alongside `.xlsx` by: (1) introducing a ZIP container abstraction that **does not** require OPC, (2) adding a PBIX package reader that can load `DataMashup` from the ZIP root, (3) wiring PBIX into the diff pipeline so it produces **Power Query (M) + metadata** ops, and (4) routing the CLI based on file extension and adding fixtures/tests. 

To anchor what’s changing:

* Today, the container layer is built around **OPC** (Open Packaging Conventions): the ZIP-based format used by Office files where `[Content_Types].xml` is required at the ZIP root. The existing `OpcContainer` enforces that by failing if `[Content_Types].xml` is absent. 
* PBIX/PBIT are ZIP containers too, but they are **not OPC**, and the `DataMashup` is typically stored as a **root-level file** named `DataMashup` (not base64 inside `customXml/item*.xml` like Excel).
* The “M diff” functionality already exists and is already integrated into workbook diffs via `m_diff::diff_m_ops_for_packages(...)`. 
* The CLI currently opens everything via `WorkbookPackage::open(...)`, so PBIX will currently fail at the container layer. 

The plan below implements branch 1 in a way that keeps existing Excel behavior stable, reuses the existing query-diff machinery, and avoids any “PBIX-specific special cases” leaking into the workbook parsing paths.

---

## Key concepts (so the design is obvious)

### OPC vs “plain ZIP container”

* **ZIP container**: any valid `.zip` file with entries (files) inside it.
* **OPC package**: a ZIP container that follows the Office packaging rules. The telltale marker is `[Content_Types].xml` at the ZIP root, which describes MIME-like content types for parts.

Right now `OpcContainer` is doing *both jobs*:

1. “Is it a ZIP?” and enforce decompression safety limits
2. “Is it an OPC package?” (requires `[Content_Types].xml`)

PBIX needs only (1), not (2). That’s why Branch 1 starts by splitting the responsibilities.

### DataMashup storage differences

* In Excel: `DataMashup` is encoded as base64 inside `customXml/item*.xml` parts; the current extractor scans those parts. 
* In PBIX: `DataMashup` is a file at ZIP root named `DataMashup`. Branch 1 adds a reader for that. 

---

## Implementation plan

### Step 1 — Introduce `ZipContainer` and refactor `OpcContainer` to wrap it

**Goal:** create a “generic ZIP” container that has the same safety properties (size limits, zip-bomb defense) as `OpcContainer`, and then make `OpcContainer` a small wrapper that only adds the OPC validation requirement.

#### 1.1 Design constraints

* Preserve the existing decompression hardening:

  * `max_entries`
  * `max_part_uncompressed_bytes`
  * `max_total_uncompressed_bytes`
    These are already enforced in `read_file_checked`. 
* Preserve existing error codes and semantics for OPC paths:

  * Non-OPC ZIP should still produce `NotOpcPackage` when opened as OPC. 
* Avoid leaking internal ZIP archive access (`container.archive`) across the crate: direct archive access makes it harder to wrap/compose later.

#### 1.2 Container API shape

Add:

* `pub struct ZipContainer { ... }` with:

  * `open_from_reader`
  * `open_from_reader_with_limits`
  * `read_file_checked`
  * `read_file_optional_checked`
  * `file_names()` iterator
  * `len()`

Refactor existing:

* `pub struct OpcContainer { inner: ZipContainer }`
* `OpcContainer::open_from_reader_with_limits`:

  * constructs `ZipContainer`
  * then enforces `[Content_Types].xml` exists and is not oversized

This aligns exactly with branch 1’s “ZipContainer + thin OpcContainer wrapper” requirement. 

#### 1.3 `core/src/container.rs` change (replace the `OpcContainer` definition + open methods)

**Code to replace (excerpt):** 

```rust
pub struct OpcContainer {
    pub(crate) archive: ZipArchive<Box<dyn ReadSeek>>,
    limits: ContainerLimits,
    total_read: u64,
}

impl OpcContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(reader: R) -> Result<Self, ContainerError> {
        Self::open_from_reader_with_limits(reader, ContainerLimits::default())
    }

    pub fn open_from_reader_with_limits<R: Read + Seek + 'static>(
        reader: R,
        limits: ContainerLimits,
    ) -> Result<Self, ContainerError> {
        let reader: Box<dyn ReadSeek> = Box::new(reader);
        let mut archive = ZipArchive::new(reader).map_err(ContainerError::Zip)?;

        if archive.len() > limits.max_entries {
            return Err(ContainerError::TooManyEntries { count: archive.len() });
        }

        // Require OPC content types
        let ct = archive
            .by_name("[Content_Types].xml")
            .map_err(|_| ContainerError::NotOpcPackage)?;
        if ct.size() > limits.max_part_uncompressed_bytes {
            return Err(ContainerError::PartTooLarge {
                name: "[Content_Types].xml".to_string(),
                size: ct.size(),
            });
        }

        Ok(Self {
            archive,
            limits,
            total_read: 0,
        })
    }
}
```

**New code to replace it with (excerpt):**

```rust
pub struct ZipContainer {
    archive: ZipArchive<Box<dyn ReadSeek>>,
    limits: ContainerLimits,
    total_read: u64,
}

impl ZipContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(reader: R) -> Result<Self, ContainerError> {
        Self::open_from_reader_with_limits(reader, ContainerLimits::default())
    }

    pub fn open_from_reader_with_limits<R: Read + Seek + 'static>(
        reader: R,
        limits: ContainerLimits,
    ) -> Result<Self, ContainerError> {
        let reader: Box<dyn ReadSeek> = Box::new(reader);
        let archive = ZipArchive::new(reader).map_err(ContainerError::Zip)?;

        if archive.len() > limits.max_entries {
            return Err(ContainerError::TooManyEntries { count: archive.len() });
        }

        Ok(Self {
            archive,
            limits,
            total_read: 0,
        })
    }

    pub fn limits(&self) -> &ContainerLimits {
        &self.limits
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.archive.file_names()
    }

    pub fn read_file_checked(&mut self, name: &str) -> Result<Vec<u8>, ContainerError> {
        let mut file = self
            .archive
            .by_name(name)
            .map_err(|_| ContainerError::FileNotFound {
                name: name.to_string(),
            })?;

        let size = file.size();
        if size > self.limits.max_part_uncompressed_bytes {
            return Err(ContainerError::PartTooLarge {
                name: name.to_string(),
                size,
            });
        }

        let new_total = self.total_read.saturating_add(size);
        if new_total > self.limits.max_total_uncompressed_bytes {
            return Err(ContainerError::TotalUncompressedTooLarge {
                total: new_total,
                max: self.limits.max_total_uncompressed_bytes,
            });
        }

        let mut buf = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buf).map_err(ContainerError::Io)?;
        self.total_read = new_total;
        Ok(buf)
    }

    pub fn read_file_optional_checked(
        &mut self,
        name: &str,
    ) -> Result<Option<Vec<u8>>, ContainerError> {
        match self.archive.by_name(name) {
            Ok(_) => Ok(Some(self.read_file_checked(name)?)),
            Err(_) => Ok(None),
        }
    }
}

pub struct OpcContainer {
    inner: ZipContainer,
}

impl OpcContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(reader: R) -> Result<Self, ContainerError> {
        Self::open_from_reader_with_limits(reader, ContainerLimits::default())
    }

    pub fn open_from_reader_with_limits<R: Read + Seek + 'static>(
        reader: R,
        limits: ContainerLimits,
    ) -> Result<Self, ContainerError> {
        let mut inner = ZipContainer::open_from_reader_with_limits(reader, limits)?;

        let ct = inner
            .archive
            .by_name("[Content_Types].xml")
            .map_err(|_| ContainerError::NotOpcPackage)?;
        if ct.size() > inner.limits.max_part_uncompressed_bytes {
            return Err(ContainerError::PartTooLarge {
                name: "[Content_Types].xml".to_string(),
                size: ct.size(),
            });
        }

        Ok(Self { inner })
    }

    pub fn limits(&self) -> &ContainerLimits {
        self.inner.limits()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.inner.file_names()
    }

    pub fn read_file_checked(&mut self, name: &str) -> Result<Vec<u8>, ContainerError> {
        self.inner.read_file_checked(name)
    }

    pub fn read_file_optional_checked(
        &mut self,
        name: &str,
    ) -> Result<Option<Vec<u8>>, ContainerError> {
        self.inner.read_file_optional_checked(name)
    }
}
```

Notes on the new code:

* `ZipContainer` becomes the shared “zip hardening + reading” abstraction.
* `OpcContainer` becomes a wrapper that only performs the `[Content_Types].xml` existence/size check.

#### 1.4 Refactor any code that reaches into `container.archive`

Right now, Excel’s DataMashup extractor iterates entries by index and uses `container.archive.by_index(i)`. 
Once `OpcContainer` is wrapped, you should remove direct archive access and use `file_names()` instead.

**Code to replace (excerpt):** 

```rust
for i in 0..container.len() {
    let file = match container.archive.by_index(i) {
        Ok(f) => f,
        Err(_) => continue,
    };
    let name = file.name().to_string();
    if !(name.starts_with("customXml/") && name.ends_with(".xml") && name.contains("item")) {
        continue;
    }
    let bytes = match container.read_file_checked(&name) {
        Ok(b) => b,
        Err(_) => continue,
    };
    // parse XML, find DataMashup element...
}
```

**New code to replace it with (excerpt):**

```rust
let names: Vec<String> = container.file_names().map(|s| s.to_string()).collect();

for name in names {
    if !(name.starts_with("customXml/") && name.ends_with(".xml") && name.contains("item")) {
        continue;
    }

    let bytes = match container.read_file_checked(&name) {
        Ok(b) => b,
        Err(_) => continue,
    };

    // parse XML, find DataMashup element...
}
```

This avoids lifetime/borrow conflicts: you collect names with an immutable borrow, then read each entry with a mutable borrow.

#### 1.5 Tests for Step 1

Add unit tests to container module:

* `zip_container_opens_non_opc_zip`: create a simple zip without `[Content_Types].xml` and verify `ZipContainer::open_from_reader` succeeds.
* Ensure existing tests that expect `OpcContainer` to reject non-OPC continue to pass unchanged.

---

### Step 2 — Add `PbixPackage` (load DataMashup from ZIP root)

**Goal:** implement a package abstraction that represents a PBIX/PBIT “host” and can extract/parse `DataMashup` from the ZIP root. 

#### 2.1 Placement and API

Add a new type in `core/src/package.rs` alongside `WorkbookPackage`:

* `pub struct PbixPackage { data_mashup: Option<DataMashup> }`
* `pub fn open<R: Read + Seek + 'static>(reader: R) -> Result<Self, PackageError>`
* Optionally `pub fn data_mashup(&self) -> Option<&DataMashup>`

Using `Option<DataMashup>` is deliberate: it keeps compatibility with `m_diff::diff_m_ops_for_packages(&Option<DataMashup>, ...)` as used by workbooks today. 

#### 2.2 PBIX parsing algorithm

1. Open ZIP using `ZipContainer` (NOT `OpcContainer`).
2. Attempt to read `DataMashup` with `read_file_optional_checked("DataMashup")`.
3. If present:

   * `parse_data_mashup(&bytes)`
   * `build_data_mashup(&raw)`
4. If absent:

   * If PBIX-like markers are present, return `NoDataMashupUseTabularModel` (Step 3). 
   * Else return `UnsupportedFormat { message: ... }`

Marker heuristic: look for common PBIX files like:

* `Report/Layout`
* `Report/Version`
* `DataModelSchema`
* `DataModel`
* `Connections`
* `DiagramLayout`

This keeps the error specific to “this looks like a real PBIX but it does not contain M mashup.”

#### 2.3 New code: `PbixPackage` (additive)

**New code to add to `core/src/package.rs` (new block):**

```rust
use crate::container::ZipContainer;
use crate::{build_data_mashup, parse_data_mashup, DataMashup};
use crate::excel_open_xml::PackageError;

pub struct PbixPackage {
    pub(crate) data_mashup: Option<DataMashup>,
}

impl PbixPackage {
    pub fn open<R: std::io::Read + std::io::Seek + 'static>(
        reader: R,
    ) -> Result<Self, PackageError> {
        let mut container = ZipContainer::open_from_reader(reader)?;

        let bytes = match container.read_file_optional_checked("DataMashup")? {
            Some(b) => b,
            None => {
                if looks_like_pbix(&container) {
                    return Err(PackageError::NoDataMashupUseTabularModel);
                }

                return Err(PackageError::UnsupportedFormat {
                    message: "missing DataMashup at ZIP root".to_string(),
                });
            }
        };

        let raw = parse_data_mashup(&bytes)?;
        let dm = build_data_mashup(&raw)?;

        Ok(Self {
            data_mashup: Some(dm),
        })
    }

    pub fn data_mashup(&self) -> Option<&DataMashup> {
        self.data_mashup.as_ref()
    }
}

fn looks_like_pbix(container: &ZipContainer) -> bool {
    container.file_names().any(|n| {
        n == "Report/Layout"
            || n == "Report/Version"
            || n == "DataModelSchema"
            || n == "DataModel"
            || n == "Connections"
            || n == "DiagramLayout"
    })
}
```

Notes:

* `looks_like_pbix` uses `ZipContainer::file_names()` which is immutable and cheap.
* `PbixPackage` returns `PackageError` to match the rest of the public package APIs.

---

### Step 3 — Add dedicated error: `NoDataMashupUseTabularModel`

**Goal:** when `DataMashup` is missing but the ZIP appears to be a PBIX, return a dedicated error: `NoDataMashupUseTabularModel`. 

This is crucial for UX: it tells users “this PBIX likely uses a tabular model, not Power Query mashup,” instead of a vague “missing part” error.

#### 3.1 Add a new error code constant

**Code to replace (`core/src/error_codes.rs` excerpt):** 

```rust
// Package-level errors
pub const PKG_NOT_ZIP: &str = "EXDIFF_PKG_001";
pub const PKG_NOT_OPC: &str = "EXDIFF_PKG_002";
pub const PKG_MISSING_PART: &str = "EXDIFF_PKG_003";
pub const PKG_INVALID_XML: &str = "EXDIFF_PKG_004";
pub const PKG_ZIP_PART_TOO_LARGE: &str = "EXDIFF_PKG_005";
pub const PKG_TOO_MANY_ENTRIES: &str = "EXDIFF_PKG_006";
pub const PKG_TOTAL_TOO_LARGE: &str = "EXDIFF_PKG_007";
pub const PKG_READ_FAILED: &str = "EXDIFF_PKG_008";
pub const PKG_UNSUPPORTED_FORMAT: &str = "EXDIFF_PKG_009";
```

**New code to replace it with:**

```rust
// Package-level errors
pub const PKG_NOT_ZIP: &str = "EXDIFF_PKG_001";
pub const PKG_NOT_OPC: &str = "EXDIFF_PKG_002";
pub const PKG_MISSING_PART: &str = "EXDIFF_PKG_003";
pub const PKG_INVALID_XML: &str = "EXDIFF_PKG_004";
pub const PKG_ZIP_PART_TOO_LARGE: &str = "EXDIFF_PKG_005";
pub const PKG_TOO_MANY_ENTRIES: &str = "EXDIFF_PKG_006";
pub const PKG_TOTAL_TOO_LARGE: &str = "EXDIFF_PKG_007";
pub const PKG_READ_FAILED: &str = "EXDIFF_PKG_008";
pub const PKG_UNSUPPORTED_FORMAT: &str = "EXDIFF_PKG_009";
pub const PKG_NO_DATAMASHUP_USE_TABULAR_MODEL: &str = "EXDIFF_PKG_010";
```

#### 3.2 Add `PackageError` variant and wire `code()`

**Code to replace (`core/src/excel_open_xml.rs` excerpt):** 

```rust
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("[{0}] not a valid ZIP file: {1}\nSuggestion: verify the input is a .xlsx workbook and not corrupt.")]
    NotAZip(&'static str, String),
    // ...
    #[error("[{0}] unsupported format: {message}")]
    UnsupportedFormat {
        #[source]
        message: String,
    },
    // ...
}

impl PackageError {
    pub fn code(&self) -> &'static str {
        match self {
            PackageError::NotAZip(..) => error_codes::PKG_NOT_ZIP,
            // ...
            PackageError::UnsupportedFormat { .. } => error_codes::PKG_UNSUPPORTED_FORMAT,
            // ...
        }
    }
}
```

**New code to replace it with (excerpt):**

```rust
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("[{0}] not a valid ZIP file: {1}\nSuggestion: verify the input is a ZIP-based file and not corrupt.")]
    NotAZip(&'static str, String),

    #[error("[{0}] PBIX/PBIT does not contain DataMashup (likely tabular model)\nSuggestion: export or extract Power Query mashup from a legacy PBIX, or use a tabular-model extraction path.")]
    NoDataMashupUseTabularModel,

    #[error("[{0}] unsupported format: {message}")]
    UnsupportedFormat {
        #[source]
        message: String,
    },

    // existing variants...
}

impl PackageError {
    pub fn code(&self) -> &'static str {
        match self {
            PackageError::NotAZip(..) => error_codes::PKG_NOT_ZIP,
            PackageError::NoDataMashupUseTabularModel => {
                error_codes::PKG_NO_DATAMASHUP_USE_TABULAR_MODEL
            }
            PackageError::UnsupportedFormat { .. } => error_codes::PKG_UNSUPPORTED_FORMAT,
            // existing mappings...
        }
    }
}
```

Two intentional tweaks here:

* The `NotAZip` suggestion becomes “ZIP-based file” rather than “.xlsx workbook” so PBIX errors read correctly too.
* The new PBIX-specific error has a “what to do next” suggestion.

---

### Step 4 — Diff PBIX packages by reusing the existing M diff machinery

**Goal:** `excel-diff diff old.pbix new.pbix` should produce a `DiffReport` consisting of query ops (`QueryAdded`, `QueryRemoved`, `QueryDefinitionChanged`, `QueryMetadataChanged`, `QueryRenamed`) exactly like workbook M diffs.

#### 4.1 Implement `PbixPackage::diff(...)` and streaming variants

You already have a strong pattern in `WorkbookPackage`:

* non-streaming collects ops then attaches `pool.strings().to_vec()` to the report 
* streaming uses `DiffSink` and emits ops then calls `finish()` exactly once

We’ll mirror that but without grid/object ops.

**New code to add to `core/src/package.rs` (additive):**

```rust
use crate::diff::{DiffError, DiffReport, DiffSummary};
use crate::m_diff::diff_m_ops_for_packages;
use crate::sink::DiffSink;
use crate::string_pool::StringPool;
use crate::{DiffConfig, ProgressCallback};

impl PbixPackage {
    pub fn diff(&self, other: &Self, config: &DiffConfig) -> Result<DiffReport, PackageError> {
        Ok(crate::with_default_session(|session| {
            let ops = diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            let strings = session.strings.strings().to_vec();
            DiffReport::new(ops).with_strings(strings)
        }))
    }

    pub fn diff_streaming<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            self.diff_streaming_with_pool(other, &mut session.strings, config, sink)
        })
    }

    pub fn diff_streaming_with_pool<S: DiffSink>(
        &self,
        other: &Self,
        pool: &mut StringPool,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        sink.begin(pool)?;

        let m_ops = diff_m_ops_for_packages(&self.data_mashup, &other.data_mashup, pool, config);

        let mut op_count = 0usize;
        for op in m_ops {
            sink.emit(op)?;
            op_count = op_count.saturating_add(1);
        }

        sink.finish()?;

        Ok(DiffSummary {
            complete: true,
            warnings: Vec::new(),
            op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        })
    }

    pub fn diff_streaming_with_progress<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
        progress: &dyn ProgressCallback,
    ) -> Result<DiffSummary, DiffError> {
        progress.on_progress("m_diff", 0.0);
        let out = self.diff_streaming(other, config, sink);
        progress.on_progress("m_diff", 1.0);
        out
    }
}
```

You may need a tiny helper to attach strings to a report; if you don’t already have `with_strings`, then the plan is:

* either add `DiffReport::from_ops_and_strings(ops, strings)`
* or construct `DiffReport { strings, ops, ... }` directly, matching how workbook diff does it. 

The essential part is: `PbixPackage::diff` must produce a `DiffReport` with its `strings` populated so the CLI’s text/json output can resolve query names. The JSONL format will work because `JsonLinesSink.begin()` writes the string table first, and we explicitly call `sink.begin(pool)` here.

---

### Step 5 — CLI host selection and routing

**Goal:** route `.pbix/.pbit` to PBIX pipeline and `.xlsx/...` to workbook pipeline, with clean errors on mismatches and on unsupported flags.

#### 5.1 CLI behavior rules

* Accept:

  * `.pbix` vs `.pbix`
  * `.pbit` vs `.pbit`
  * `.pbix` vs `.pbit` (treat both as PBIX host; same reader)
* Reject:

  * PBIX host vs Excel host (mismatch)
* For PBIX host:

  * disallow `--database` and any sheet/key-related settings (since there is no grid diff in PBIX branch 1)

#### 5.2 Minimal refactor approach: add a small enum in the CLI module

In `cli/src/commands/diff.rs`, define:

```rust
enum Host {
    Workbook(excel_diff::WorkbookPackage),
    Pbix(excel_diff::PbixPackage),
}
```

Then open based on extension, and match on the host for diffing.

#### 5.3 `cli/src/commands/diff.rs` change (replace the workbook-only open/diff logic)

**Code to replace (excerpt):** 

```rust
let old_pkg = WorkbookPackage::open(File::open(old_path)?)?;
let new_pkg = WorkbookPackage::open(File::open(new_path)?)?;

if format == OutputFormat::JsonLines {
    run_streaming(&old_pkg, &new_pkg, &config, progress)?;
    return Ok(ExitCode::from(0));
}

let report = if database {
    old_pkg.diff_database_mode(&new_pkg, sheet, &keys, &config)?
} else {
    old_pkg.diff(&new_pkg, &config)
};
```

**New code to replace it with (excerpt):**

```rust
use excel_diff::{PbixPackage, WorkbookPackage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostKind {
    Workbook,
    Pbix,
}

fn host_kind_from_path(path: &std::path::Path) -> Option<HostKind> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" | "xlsm" | "xltx" | "xltm" => Some(HostKind::Workbook),
        "pbix" | "pbit" => Some(HostKind::Pbix),
        _ => None,
    }
}

enum Host {
    Workbook(WorkbookPackage),
    Pbix(PbixPackage),
}

fn open_host(path: &std::path::Path) -> anyhow::Result<(HostKind, Host)> {
    let kind = host_kind_from_path(path)
        .ok_or_else(|| anyhow::anyhow!("unsupported input extension"))?;

    let file = std::fs::File::open(path)?;
    let host = match kind {
        HostKind::Workbook => Host::Workbook(WorkbookPackage::open(file)?),
        HostKind::Pbix => Host::Pbix(PbixPackage::open(file)?),
    };

    Ok((kind, host))
}

let (old_kind, old_host) = open_host(old_path)?;
let (new_kind, new_host) = open_host(new_path)?;

if old_kind != new_kind {
    anyhow::bail!("input host types must match");
}

if old_kind == HostKind::Pbix {
    if database || sheet.is_some() || keys.is_some() {
        anyhow::bail!("database mode and sheet/key options are not supported for PBIX/PBIT");
    }
}

if format == OutputFormat::JsonLines {
    run_streaming_host(&old_host, &new_host, &config, progress)?;
    return Ok(ExitCode::from(0));
}

let report = match (&old_host, &new_host) {
    (Host::Workbook(a), Host::Workbook(b)) => {
        if database {
            a.diff_database_mode(b, sheet.as_deref().unwrap(), &keys.unwrap(), &config)?
        } else {
            a.diff(b, &config)
        }
    }
    (Host::Pbix(a), Host::Pbix(b)) => a.diff(b, &config)?,
    _ => unreachable!(),
};
```

Then implement `run_streaming_host` to call either `WorkbookPackage::diff_streaming(_with_progress)` or `PbixPackage::diff_streaming(_with_progress)`.

This matches branch 1’s “extension-based selection” requirement and keeps the change localized to the CLI.

---

### Step 6 — Fixtures and tests

Branch 1 explicitly calls out adding PBIX fixtures and tests. 

Because CI generates fixtures from `fixtures/manifest_cli_tests.yaml` before running tests, the cleanest approach is:

1. add a PBIX fixture generator to `fixtures/src/generate.py` registry, and
2. add new scenarios to `fixtures/manifest_cli_tests.yaml` that emit the pbix fixtures listed in branch 1.

#### 6.1 Add a new PBIX fixture generator (Python)

You already have a generator framework and multiple generator types registered in `fixtures/src/generate.py`. 
You also already have XML scanning + base64 decode logic in mashup generators. 

##### New file: `fixtures/src/generators/pbix.py`

**New code (new file):**

```python
import base64
import zipfile
from pathlib import Path

from lxml import etree

from .base import BaseGenerator


_NS = {"dm": "http://schemas.microsoft.com/DataMashup"}


def _find_datamashup_element(root):
    if root is None:
        return None
    if root.tag.endswith("DataMashup"):
        return root
    return root.find(".//dm:DataMashup", namespaces=_NS)


def _extract_datamashup_bytes_from_xlsx(path: Path) -> bytes:
    with zipfile.ZipFile(path, "r") as zin:
        for info in zin.infolist():
            name = info.filename
            if not (name.startswith("customXml/item") and name.endswith(".xml")):
                continue

            buf = zin.read(name)
            if b"DataMashup" not in buf and b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" not in buf:
                continue

            root = etree.fromstring(buf)
            node = _find_datamashup_element(root)
            if node is None or node.text is None:
                continue

            text = node.text.strip()
            if not text:
                continue

            return base64.b64decode(text)

    raise ValueError("DataMashup not found in xlsx")


class PbixGenerator(BaseGenerator):
    def generate(self, out_dir: Path, outputs):
        out_path = out_dir / outputs[0]

        mode = self.args.get("mode", "from_xlsx")
        base_file = self.args.get("base_file")

        include_datamashup = (mode == "from_xlsx")
        include_markers = True

        dm_bytes = b""
        if include_datamashup:
            if not base_file:
                raise ValueError("base_file is required for mode=from_xlsx")
            base_path = Path(base_file)
            if not base_path.exists():
                base_path = Path("fixtures") / base_path
            dm_bytes = _extract_datamashup_bytes_from_xlsx(base_path)

        with zipfile.ZipFile(out_path, "w", compression=zipfile.ZIP_DEFLATED) as zout:
            if include_datamashup:
                zout.writestr("DataMashup", dm_bytes)
            if include_markers:
                zout.writestr("Report/Layout", b"{}")
                zout.writestr("Report/Version", b"1")
                zout.writestr("DataModelSchema", b"{}")
```

This generator supports:

* `mode=from_xlsx` + `base_file=...` to create a “legacy mashup PBIX”
* `mode=no_datamashup` (by setting mode and omitting base_file) to create a PBIX-like file without `DataMashup` but with marker files.

#### 6.2 Register the generator in `fixtures/src/generate.py`

**Code to replace (excerpt):** 

```python
from generators.mashup import (
    MashupInjectGenerator,
    MashupPermissionsMetadataGenerator,
)
# ...

GENERATORS = {
    "mashup_inject": MashupInjectGenerator,
    "mashup_permissions_metadata": MashupPermissionsMetadataGenerator,
    # ...
}
```

**New code to replace it with (excerpt):**

```python
from generators.mashup import (
    MashupInjectGenerator,
    MashupPermissionsMetadataGenerator,
)
from generators.pbix import PbixGenerator

GENERATORS = {
    "mashup_inject": MashupInjectGenerator,
    "mashup_permissions_metadata": MashupPermissionsMetadataGenerator,
    "pbix": PbixGenerator,
}
```

#### 6.3 Update `fixtures/manifest_cli_tests.yaml` to generate PBIX fixtures

Branch 1’s requested PBIX fixtures are:

* `pbix_legacy_one_query_a.pbix`
* `pbix_legacy_one_query_b.pbix`
* `pbix_legacy_multi_query_a.pbix`
* `pbix_legacy_multi_query_b.pbix`
* `pbix_no_datamashup.pbix` 

You can build these from existing xlsx fixtures used in CLI tests (you already generate `m_add_query_a.xlsx` and `m_add_query_b.xlsx` in the CLI manifest). 
If `m_change_literal_a/b.xlsx` are not currently in the CLI manifest, add them too (so “one query changed” exists).

**Manifest change strategy:**

* Ensure the xlsx sources exist *before* pbix generation entries.
* Add pbix entries at the end.

**Code to replace (conceptual excerpt near the end of `manifest_cli_tests.yaml`):** 

```yaml
# existing scenarios...
```

**New block to append (pbix scenarios):**

```yaml
- id: branch1_pbix_legacy_one_query_a
  generator: pbix
  args:
    mode: from_xlsx
    base_file: generated/m_change_literal_a.xlsx
  outputs:
    - pbix_legacy_one_query_a.pbix

- id: branch1_pbix_legacy_one_query_b
  generator: pbix
  args:
    mode: from_xlsx
    base_file: generated/m_change_literal_b.xlsx
  outputs:
    - pbix_legacy_one_query_b.pbix

- id: branch1_pbix_legacy_multi_query_a
  generator: pbix
  args:
    mode: from_xlsx
    base_file: generated/m_add_query_a.xlsx
  outputs:
    - pbix_legacy_multi_query_a.pbix

- id: branch1_pbix_legacy_multi_query_b
  generator: pbix
  args:
    mode: from_xlsx
    base_file: generated/m_add_query_b.xlsx
  outputs:
    - pbix_legacy_multi_query_b.pbix

- id: branch1_pbix_no_datamashup
  generator: pbix
  args:
    mode: no_datamashup
  outputs:
    - pbix_no_datamashup.pbix
```

If `m_change_literal_a/b.xlsx` aren’t currently generated by `manifest_cli_tests.yaml`, add those two scenarios earlier by copying them from the full manifest.

#### 6.4 Core tests to add

Add a new test module (e.g. `core/tests/pbix_host_support_tests.rs`):

1. **Open PBIX loads DataMashup**

* open `pbix_legacy_one_query_a.pbix` with `PbixPackage::open(...)`
* assert `data_mashup.is_some()`

2. **Diff PBIX emits query ops**

* diff `pbix_legacy_multi_query_a.pbix` vs `pbix_legacy_multi_query_b.pbix`
* assert there’s at least one `DiffOp::QueryAdded` (or other M op)

3. **No DataMashup yields dedicated error**

* open `pbix_no_datamashup.pbix`
* assert error matches `PackageError::NoDataMashupUseTabularModel`

#### 6.5 CLI integration tests to add

In `cli/tests/cli_integration_tests.rs`, mirror the existing Power Query test (which currently checks xlsx). 

Add:

* `diff_pbix_power_query_changes_detected`:

  * run `excel-diff diff pbix_legacy_multi_query_a.pbix pbix_legacy_multi_query_b.pbix --format json`
  * assert exit code indicates differences and output mentions query ops (or parse JSON and assert at least one `QueryAdded` op).

Also add a JSONL test:

* `diff_pbix_jsonl_writes_header_and_ops`:

  * run with `--format jsonl`
  * assert first line contains `"kind":"Header"` and string table includes at least one query name (same pattern as existing JSONL header tests in core).

---

## Suggested implementation order (to keep the codebase green at each step)

1. **Container split**

   * Add `ZipContainer`
   * Refactor `OpcContainer`
   * Update `open_data_mashup_from_container` to avoid archive access
     Run existing unit tests.

2. **PbixPackage open**

   * Add `PbixPackage::open` using `ZipContainer`
   * Add marker heuristic

3. **Error variant + code**

   * Add `NoDataMashupUseTabularModel` to `PackageError`
   * Add error code constant

4. **PbixPackage diff + streaming**

   * Implement `diff`, `diff_streaming`, `diff_streaming_with_progress`

5. **CLI routing**

   * Add host-kind detection and routing
   * Add disallow rules for PBIX + database mode
   * Add jsonl routing

6. **Fixtures + tests**

   * Add pbix generator + manifest scenarios
   * Add core + CLI tests

This matches branch 1’s deliverables while keeping each step reviewable and testable. 
