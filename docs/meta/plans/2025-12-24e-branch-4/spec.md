## What the next sprint asks for that the codebase still lacks

After cross-checking the sprint outline against the current codebase context, most of the heavy lifting (PBIX/PBIT host support in core/wasm, semantic M parsing + semantic detail payloads, parallelism + determinism tests, enhanced-metadata PBIX fallback) already exists.

The pieces that are still *missing* (or not yet wired end-to-end) are:

1. **Web demo: semantic presentation** (per-query cards + step diffs, plus a PBIX/PBIT-friendly UI). The outline explicitly calls this out as still undone.  
   The codebase already has the data model needed for this (`QuerySemanticDetail`, `StepDiff`, etc.), but the web UI doesn’t yet surface it. 

2. **Release-readiness documentation** (coverage doc for the semantic M parser and a release checklist doc). 

3. **CLI `info`: PBIX/PBIT support + embedded queries, and a shared “open host” helper**. The outline asks for `info` to become host-aware. 
   Today `info` opens only `WorkbookPackage`. 
   `diff` already has host detection + `open_host`, but it’s local to that command instead of being shared. 

Below is an implementation plan to add those missing pieces.

---

## Feature 1: Make `excel-diff info` host-aware (workbook + PBIX/PBIT) and include embedded queries

### Goal

* `excel-diff info <file>` works for `.xlsx/.xlsm/.xltx/.xltm` **and** `.pbix/.pbit`. 
* `--queries` prints:

  * top-level Power Query queries (the usual DataMashup section)
  * **embedded** queries (queries living inside embedded package parts)

### Design principles

* A “host” is simply “what container format are we opening”: workbook vs PBIX/PBIT.
* Keep detection logic extension-based (already used in `diff`), but move it to a shared module so commands stay consistent. 
* Keep output stable and sorted.

### Step 1: Add a shared CLI host helper module

**Add new file**: `cli/src/commands/host.rs`

```rust
use anyhow::{Context, Result};
use excel_diff::{PbixPackage, WorkbookPackage};
use std::fs::File;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostKind {
    Workbook,
    Pbix,
}

pub(crate) enum Host {
    Workbook(WorkbookPackage),
    Pbix(PbixPackage),
}

pub(crate) fn host_kind_from_path(path: &Path) -> Option<HostKind> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" | "xlsm" | "xltx" | "xltm" => Some(HostKind::Workbook),
        "pbix" | "pbit" => Some(HostKind::Pbix),
        _ => None,
    }
}

pub(crate) fn open_host(path: &Path, kind: HostKind, label: &str) -> Result<Host> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open {} file: {}", label, path.display()))?;

    let host = match kind {
        HostKind::Workbook => Host::Workbook(
            WorkbookPackage::open(file)
                .with_context(|| format!("Failed to parse {} workbook: {}", label, path.display()))?,
        ),
        HostKind::Pbix => Host::Pbix(
            PbixPackage::open(file)
                .with_context(|| format!("Failed to parse {} PBIX/PBIT: {}", label, path.display()))?,
        ),
    };

    Ok(host)
}
```

### Step 2: Export the module from `cli/src/commands/mod.rs`

**Code to replace** (current):

```rust
pub mod diff;
pub mod info;
```

**New code**:

```rust
pub mod diff;
pub mod host;
pub mod info;
```

### Step 3: Refactor `diff` command to use the shared host helper

Today `diff.rs` defines `HostKind`, `host_kind_from_path`, `Host`, and `open_host` locally. 
Replace that block with an import of the shared helper.

**Code to replace** (from `cli/src/commands/diff.rs`): 

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostKind {
    Workbook,
    Pbix,
}

fn host_kind_from_path(path: &Path) -> Option<HostKind> {
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

fn open_host(path: &Path, kind: HostKind, label: &str) -> Result<Host> {
    let file =
        File::open(path).with_context(|| format!("Failed to open {} file: {}", label, path.display()))?;
    let host = match kind {
        HostKind::Workbook => Host::Workbook(
            WorkbookPackage::open(file)
                .with_context(|| format!("Failed to parse {} workbook: {}", label, path.display()))?,
        ),
        HostKind::Pbix => Host::Pbix(
            PbixPackage::open(file)
                .with_context(|| format!("Failed to parse {} PBIX/PBIT: {}", label, path.display()))?,
        ),
    };
    Ok(host)
}
```

**New code** (place near the other `use ...` statements in the same file):

```rust
use crate::commands::host::{host_kind_from_path, open_host, Host, HostKind};
```

Then update references from the old local `HostKind/Host` to the imported ones. This is mechanically the same types/functions, just relocated.

### Step 4: Replace `info.rs` with a host-aware implementation

`info.rs` currently opens only `WorkbookPackage`. 
We’ll replace it so it:

* detects host kind by extension (same logic as `diff`)
* opens with `open_host`
* prints workbook sheet list for workbooks
* prints Power Query info for both (when `--queries`)
* includes embedded queries by calling `build_embedded_queries`

**Code to replace** (`cli/src/commands/info.rs`, current): 

```rust
use anyhow::{Context, Result};
use excel_diff::{build_queries, SheetKind, WorkbookPackage};
use std::fs::File;
use std::io::{self, Write};
use std::process::ExitCode;

pub fn run(path: &str, show_queries: bool) -> Result<ExitCode> {
    let file = File::open(path).with_context(|| format!("Failed to open file: {}", path))?;
    let pkg = WorkbookPackage::open(file).with_context(|| format!("Failed to open workbook: {}", path))?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path);

    writeln!(handle, "Workbook: {}", filename)?;
    writeln!(handle, "Sheets: {}", pkg.workbook.sheets.len())?;
    for sheet in &pkg.workbook.sheets {
        let sheet_name = excel_diff::with_default_session(|session| session.strings.resolve(sheet.name).to_string());
        let kind_str = match sheet.kind {
            SheetKind::Worksheet => "worksheet",
            SheetKind::Chartsheet => "chartsheet",
            SheetKind::Dialogsheet => "dialog",
            SheetKind::Macrosheet => "macro",
            SheetKind::Modulesheet => "module",
        };
        writeln!(handle, "- {} ({})", sheet_name, kind_str)?;
    }

    if show_queries {
        writeln!(handle)?;
        match &pkg.data_mashup {
            None => {
                writeln!(handle, "Power Query: none")?;
            }
            Some(dm) => {
                let queries = build_queries(dm).context("Failed to parse Power Query DataMashup")?;
                writeln!(handle, "Power Query: {} queries", queries.len())?;
                for q in queries {
                    let load_flags = format_load_flags(&q.metadata);
                    let group_path = q
                        .metadata
                        .group_path
                        .as_ref()
                        .map(|v| v.join("/"))
                        .unwrap_or_else(|| "(none)".to_string());

                    if load_flags.is_empty() {
                        writeln!(handle, "- {}  [group={}]", q.name, group_path)?;
                    } else {
                        writeln!(handle, "- {}  [{}]  [group={}]", q.name, load_flags, group_path)?;
                    }
                }
            }
        }
    }

    Ok(ExitCode::from(0))
}

fn format_load_flags(meta: &excel_diff::QueryMetadata) -> String {
    let mut flags = Vec::new();
    if meta.load_enabled {
        flags.push("load");
    }
    if meta.include_in_refresh_all {
        flags.push("refresh_all");
    }
    if meta.fill_enabled {
        flags.push("fill");
    }
    flags.join(",")
}
```

**New code**:

```rust
use anyhow::{Context, Result};
use excel_diff::{build_embedded_queries, build_queries, DataMashup, SheetKind};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use crate::commands::host::{host_kind_from_path, open_host, Host};

pub fn run(path: &str, show_queries: bool) -> Result<ExitCode> {
    let path = Path::new(path);
    let kind = host_kind_from_path(path)
        .with_context(|| format!("Unsupported input extension: {}", path.display()))?;

    let host = open_host(path, kind, "input")?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    match host {
        Host::Workbook(pkg) => {
            writeln!(handle, "Workbook: {}", filename)?;
            writeln!(handle, "Sheets: {}", pkg.workbook.sheets.len())?;
            for sheet in &pkg.workbook.sheets {
                let sheet_name =
                    excel_diff::with_default_session(|session| session.strings.resolve(sheet.name).to_string());
                let kind_str = match sheet.kind {
                    SheetKind::Worksheet => "worksheet",
                    SheetKind::Chartsheet => "chartsheet",
                    SheetKind::Dialogsheet => "dialog",
                    SheetKind::Macrosheet => "macro",
                    SheetKind::Modulesheet => "module",
                };
                writeln!(handle, "- {} ({})", sheet_name, kind_str)?;
            }

            if show_queries {
                write_power_query_section(&mut handle, pkg.data_mashup.as_ref())?;
            }
        }
        Host::Pbix(pkg) => {
            writeln!(handle, "PBIX/PBIT: {}", filename)?;
            if show_queries {
                write_power_query_section(&mut handle, pkg.data_mashup())?;
            }
        }
    }

    Ok(ExitCode::from(0))
}

fn write_power_query_section<W: Write>(w: &mut W, dm_opt: Option<&DataMashup>) -> Result<()> {
    writeln!(w)?;

    let Some(dm) = dm_opt else {
        writeln!(w, "Power Query: none")?;
        return Ok(());
    };

    writeln!(w, "Power Query:")?;

    match build_queries(dm) {
        Ok(mut top) => {
            top.sort_by(|a, b| a.name.cmp(&b.name));
            writeln!(w, "  Top-level: {}", top.len())?;
            for q in top {
                write_query_line(w, &q)?;
            }
        }
        Err(e) => {
            writeln!(w, "  Top-level: error parsing queries: {}", e)?;
        }
    }

    let mut embedded = build_embedded_queries(dm);
    embedded.sort_by(|a, b| a.name.cmp(&b.name));
    writeln!(w, "  Embedded: {}", embedded.len())?;
    for q in embedded {
        write_query_line(w, &q)?;
    }

    Ok(())
}

fn write_query_line<W: Write>(w: &mut W, q: &excel_diff::Query) -> Result<()> {
    let load_flags = format_load_flags(&q.metadata);
    let group_path = q
        .metadata
        .group_path
        .as_ref()
        .map(|v| v.join("/"))
        .unwrap_or_else(|| "(none)".to_string());

    if load_flags.is_empty() {
        writeln!(w, "  - {}  [group={}]", q.name, group_path)?;
    } else {
        writeln!(w, "  - {}  [{}]  [group={}]", q.name, load_flags, group_path)?;
    }

    Ok(())
}

fn format_load_flags(meta: &excel_diff::QueryMetadata) -> String {
    let mut flags = Vec::new();
    if meta.load_enabled {
        flags.push("load");
    }
    if meta.include_in_refresh_all {
        flags.push("refresh_all");
    }
    if meta.fill_enabled {
        flags.push("fill");
    }
    flags.join(",")
}
```

### Step 5: Update CLI help text to stop claiming “workbooks only”

This is part of making PBIX/PBIT “first-class” in the CLI. 

**Code to replace** (from `cli/src/main.rs`): 

```rust
Diff {
    /// Path to the old/base workbook (.xlsx, .xlsm)
    old: String,
    /// Path to the new workbook (.xlsx, .xlsm)
    new: String,
    ...
},
Info {
    /// Path to the workbook (.xlsx, .xlsm)
    path: String,
    ...
},
```

**New code**:

```rust
Diff {
    /// Path to the old/base file (.xlsx, .xlsm, .xltx, .xltm, .pbix, .pbit)
    old: String,
    /// Path to the new file (.xlsx, .xlsm, .xltx, .xltm, .pbix, .pbit)
    new: String,
    ...
},
Info {
    /// Path to the file (.xlsx, .xlsm, .xltx, .xltm, .pbix, .pbit)
    path: String,
    ...
},
```

Also update the `about` strings:

**Code to replace**: 

```rust
Diff {
    about: "Compare two Excel workbooks",
    ...
},
Info {
    about: "Show information about a workbook",
    ...
},
```

**New code**:

```rust
Diff {
    about: "Compare two Excel workbooks or PBIX/PBIT packages",
    ...
},
Info {
    about: "Show information about a workbook or PBIX/PBIT package",
    ...
},
```

### Step 6: Add a PBIX fixture with embedded queries and a CLI integration test

You already have mashup fixtures that include multiple embedded packages (see `m_embedded_change_a.xlsx` / `m_embedded_change_b.xlsx`). 
Generate a PBIX from one of them (using the existing PBIX generator that copies DataMashup out of a workbook). 

**Add to** `fixtures/manifest_cli_tests.yaml` (append a new item):

```yaml
- id: "branch2_pbix_embedded_queries"
  generator: "pbix"
  args:
    mode: "from_xlsx"
    base_file: "generated/m_embedded_change_a.xlsx"
    output: "pbix_embedded_queries.pbix"
```

Then add a CLI test (append to `cli/tests/integration_tests.rs`):

```rust
#[test]
fn info_pbix_includes_embedded_queries() {
    let output = excel_diff_cmd()
        .args(["info", "--queries", &fixture_path("pbix_embedded_queries.pbix")])
        .output()
        .expect("failed to run excel-diff");

    assert!(
        output.status.success(),
        "info should succeed for pbix: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PBIX/PBIT:"),
        "expected pbix header, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Embedded/"),
        "expected embedded queries to be listed, got: {}",
        stdout
    );
}
```

That test is intentionally loose (it doesn’t depend on an exact embedded query name), but it still proves the feature works.

---

## Feature 2: Web demo semantic presentation (per-query cards, step diffs, PBIX/PBIT-friendly upload)

### Goal

* Web demo can compare:

  * workbook vs workbook (`.xlsx/.xlsm/...`)
  * pbix vs pbix (`.pbix/.pbit`)
* UI renders semantic M details for changed queries:

  * step-level diffs (`step_diffs`)
  * AST summary fallback
* UI also shows measure ops in a distinct section

The outline explicitly calls out that semantic rendering is the missing part.  
You already have:

* the semantic payload types (`QuerySemanticDetail`, `StepDiff`, etc.) 
* wasm entrypoint that supports host detection by filename and diffs PBIX/PBIT too (`diff_files_json`) 

So the work is “UI wiring + rendering,” not core diff logic.

### Step 1: Render from `DiffReport` JSON (string table + ops)

Key insight for the web UI:

* ops reference strings by integer id; you must resolve via `report.strings[id]` (same as the CLI does).

### Step 2: Implement a small pure renderer: `web/render.js`

**Add new file**: `web/render.js`

```js
function esc(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function resolveString(report, id) {
  if (typeof id !== "number") return String(id);
  if (!report || !Array.isArray(report.strings)) return "<missing strings>";
  return report.strings[id] != null ? report.strings[id] : "<unknown>";
}

function isQueryOp(kind) {
  return (
    kind === "QueryAdded" ||
    kind === "QueryRemoved" ||
    kind === "QueryRenamed" ||
    kind === "QueryDefinitionChanged" ||
    kind === "QueryMetadataChanged"
  );
}

function isMeasureOp(kind) {
  return (
    kind === "MeasureAdded" ||
    kind === "MeasureRemoved" ||
    kind === "MeasureDefinitionChanged"
  );
}

function formatStepType(stepType) {
  if (!stepType) return "unknown";
  if (stepType === "other") return "Other";
  const parts = String(stepType).split("_");
  if (parts.length === 1) return parts[0];
  const head = parts[0].charAt(0).toUpperCase() + parts[0].slice(1);
  const tail = parts
    .slice(1)
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join("");
  return head + "." + tail;
}

function renderStepDiff(report, d) {
  const k = d.kind;
  if (k === "step_added") {
    const name = resolveString(report, d.step.name);
    const typ = formatStepType(d.step.step_type);
    return "+ " + esc(name) + " (" + esc(typ) + ")";
  }
  if (k === "step_removed") {
    const name = resolveString(report, d.step.name);
    const typ = formatStepType(d.step.step_type);
    return "- " + esc(name) + " (" + esc(typ) + ")";
  }
  if (k === "step_reordered") {
    const name = resolveString(report, d.name);
    return "r " + esc(name) + " [" + d.from_index + " -> " + d.to_index + "]";
  }
  if (k === "step_modified") {
    const beforeName = resolveString(report, d.before.name);
    const afterName = resolveString(report, d.after.name);
    const parts = [];
    for (const c of d.changes || []) {
      if (c.kind === "renamed") {
        parts.push(
          "renamed(" +
            esc(resolveString(report, c.from)) +
            " -> " +
            esc(resolveString(report, c.to)) +
            ")"
        );
      } else if (c.kind === "source_refs_changed") {
        const rem = (c.removed || []).length;
        const add = (c.added || []).length;
        parts.push("source_refs(" + rem + " removed, " + add + " added)");
      } else if (c.kind === "params_changed") {
        parts.push("params_changed");
      } else {
        parts.push(String(c.kind || "unknown"));
      }
    }
    const changeTxt = parts.length ? " [" + parts.join(", ") + "]" : "";
    if (beforeName === afterName) {
      return "~ " + esc(beforeName) + changeTxt;
    }
    return "~ " + esc(beforeName) + " -> " + esc(afterName) + changeTxt;
  }
  return "? " + esc(JSON.stringify(d));
}

function renderQueryCard(report, queryName, ops) {
  const header = "Query: " + esc(queryName);
  let body = "";

  for (const op of ops) {
    if (op.kind === "QueryAdded") {
      body += "<div class=\"op\">+ Added</div>";
    } else if (op.kind === "QueryRemoved") {
      body += "<div class=\"op\">- Removed</div>";
    } else if (op.kind === "QueryRenamed") {
      body +=
        "<div class=\"op\">r Renamed: " +
        esc(resolveString(report, op.from)) +
        " -> " +
        esc(resolveString(report, op.to)) +
        "</div>";
    } else if (op.kind === "QueryMetadataChanged") {
      body +=
        "<div class=\"op\">~ Metadata: " +
        esc(op.field) +
        " (" +
        esc(resolveString(report, op.old)) +
        " -> " +
        esc(resolveString(report, op.new)) +
        ")</div>";
    } else if (op.kind === "QueryDefinitionChanged") {
      body += "<div class=\"op\">~ Definition changed (" + esc(op.change_kind) + ")</div>";

      const sd = op.semantic_detail;
      if (sd && Array.isArray(sd.step_diffs) && sd.step_diffs.length) {
        body += "<div class=\"subhead\">Step diffs</div><ul class=\"steps\">";
        for (const d of sd.step_diffs) {
          body += "<li>" + renderStepDiff(report, d) + "</li>";
        }
        body += "</ul>";
      } else if (sd && sd.ast_summary) {
        const a = sd.ast_summary;
        body +=
          "<div class=\"subhead\">AST summary</div>" +
          "<div class=\"ast\">" +
          "mode=" +
          esc(a.mode) +
          " inserted=" +
          a.inserted +
          " deleted=" +
          a.deleted +
          " updated=" +
          a.updated +
          " moved=" +
          a.moved +
          "</div>";
      }
    } else {
      body += "<div class=\"op\">? " + esc(op.kind) + "</div>";
    }
  }

  return (
    "<details class=\"card\">" +
    "<summary>" +
    header +
    "</summary>" +
    "<div class=\"card-body\">" +
    body +
    "</div>" +
    "</details>"
  );
}

function renderMeasureCard(report, name, ops) {
  const header = "Measure: " + esc(name);
  let body = "";
  for (const op of ops) {
    if (op.kind === "MeasureAdded") body += "<div class=\"op\">+ Added</div>";
    else if (op.kind === "MeasureRemoved") body += "<div class=\"op\">- Removed</div>";
    else if (op.kind === "MeasureDefinitionChanged") body += "<div class=\"op\">~ Definition changed</div>";
    else body += "<div class=\"op\">? " + esc(op.kind) + "</div>";
  }
  return (
    "<details class=\"card\">" +
    "<summary>" +
    header +
    "</summary>" +
    "<div class=\"card-body\">" +
    body +
    "</div>" +
    "</details>"
  );
}

export function renderReportHtml(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  const warnings = Array.isArray(report.warnings) ? report.warnings : [];

  const queryMap = new Map();
  const measureMap = new Map();
  let otherCount = 0;

  for (const op of ops) {
    const kind = op.kind;
    if (isQueryOp(kind)) {
      let key = "";
      if (kind === "QueryRenamed") key = resolveString(report, op.to);
      else key = resolveString(report, op.name);
      if (!queryMap.has(key)) queryMap.set(key, []);
      queryMap.get(key).push(op);
    } else if (isMeasureOp(kind)) {
      const key = resolveString(report, op.name);
      if (!measureMap.has(key)) measureMap.set(key, []);
      measureMap.get(key).push(op);
    } else {
      otherCount += 1;
    }
  }

  const queryKeys = Array.from(queryMap.keys()).sort();
  const measureKeys = Array.from(measureMap.keys()).sort();

  let html = "";
  html += "<div class=\"summary\">";
  html += "<div>schema_version=" + esc(report.version) + "</div>";
  html += "<div>complete=" + esc(report.complete) + "</div>";
  html += "<div>ops=" + ops.length + " queries=" + queryKeys.length + " measures=" + measureKeys.length + " other=" + otherCount + "</div>";
  html += "</div>";

  if (warnings.length) {
    html += "<div class=\"warnings\"><div class=\"subhead\">Warnings</div><ul>";
    for (const w of warnings) html += "<li>" + esc(w) + "</li>";
    html += "</ul></div>";
  }

  if (queryKeys.length) {
    html += "<h2>Power Query</h2>";
    for (const k of queryKeys) {
      html += renderQueryCard(report, k, queryMap.get(k));
    }
  }

  if (measureKeys.length) {
    html += "<h2>Measures</h2>";
    for (const k of measureKeys) {
      html += renderMeasureCard(report, k, measureMap.get(k));
    }
  }

  if (!queryKeys.length && !measureKeys.length && otherCount) {
    html += "<div class=\"note\">No query or measure ops found. Use Raw JSON to inspect details.</div>";
  }

  return html;
}
```

### Step 3: Update the web entrypoint to call `diff_files_json` and render

This is the key PBIX/PBIT enablement: wasm host detection needs filenames (it branches on `.pbix/.pbit` vs workbook). 

**Replace entire** `web/main.js` with:

```js
import init, { diff_files_json, get_version } from "./wasm/excel_diff_wasm.js";
import { renderReportHtml } from "./render.js";

function byId(id) {
  const el = document.getElementById(id);
  if (!el) throw new Error("Missing element: " + id);
  return el;
}

function setStatus(msg) {
  byId("status").textContent = msg;
}

async function readFileBytes(file) {
  return new Uint8Array(await file.arrayBuffer());
}

async function runDiff() {
  const oldFile = byId("fileOld").files[0];
  const newFile = byId("fileNew").files[0];

  if (!oldFile || !newFile) {
    setStatus("Select both files.");
    return;
  }

  setStatus("Diffing...");
  byId("report").innerHTML = "";
  byId("raw").textContent = "";

  try {
    const oldBytes = await readFileBytes(oldFile);
    const newBytes = await readFileBytes(newFile);

    const json = diff_files_json(oldBytes, newBytes, oldFile.name, newFile.name);
    const report = JSON.parse(json);

    byId("report").innerHTML = renderReportHtml(report);
    byId("raw").textContent = JSON.stringify(report, null, 2);

    setStatus("Done.");
  } catch (e) {
    setStatus("Error");
    byId("report").innerHTML = "<pre class=\"error\"></pre>";
    byId("report").querySelector("pre").textContent = String(e && e.message ? e.message : e);
  }
}

async function main() {
  setStatus("Loading...");
  await init();
  byId("version").textContent = get_version();
  setStatus("Ready.");
  byId("run").addEventListener("click", runDiff);
}

main().catch((e) => {
  setStatus("Failed to initialize: " + String(e));
});
```

### Step 4: Update `web/index.html` to support PBIX/PBIT uploads and show semantic sections

**Replace entire** `web/index.html` with:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>excel-diff web demo</title>
    <style>
      body { font-family: system-ui, sans-serif; margin: 24px; }
      .row { display: flex; gap: 16px; flex-wrap: wrap; align-items: end; }
      .row > div { min-width: 260px; }
      #status { margin: 12px 0; }
      .summary { padding: 10px; border: 1px solid #ddd; margin: 12px 0; }
      .warnings { padding: 10px; border: 1px solid #f0c; margin: 12px 0; }
      .card { border: 1px solid #ddd; border-radius: 6px; margin: 8px 0; padding: 6px 10px; }
      .card summary { cursor: pointer; font-weight: 600; }
      .card-body { margin-top: 8px; }
      .op { margin: 4px 0; }
      .subhead { margin-top: 10px; font-weight: 600; }
      .steps { margin: 6px 0 0 18px; }
      pre { white-space: pre-wrap; word-break: break-word; }
      .error { border: 1px solid #d33; padding: 10px; }
      details.raw { margin-top: 16px; }
      .muted { color: #666; }
    </style>
  </head>
  <body>
    <h1>excel-diff web demo</h1>
    <div class="muted">wasm version: <span id="version"></span></div>

    <div class="row">
      <div>
        <label for="fileOld">Old file</label><br />
        <input id="fileOld" type="file" accept=".xlsx,.xlsm,.xltx,.xltm,.pbix,.pbit" />
      </div>
      <div>
        <label for="fileNew">New file</label><br />
        <input id="fileNew" type="file" accept=".xlsx,.xlsm,.xltx,.xltm,.pbix,.pbit" />
      </div>
      <div>
        <button id="run">Diff</button>
      </div>
    </div>

    <div id="status"></div>

    <div id="report"></div>

    <details class="raw">
      <summary>Raw JSON</summary>
      <pre id="raw"></pre>
    </details>

    <script type="module" src="./main.js"></script>
  </body>
</html>
```

### Step 5: Add a tiny UI test (no browser required)

The outline asks for lightweight UI testing. 
Do it without DOM by testing the HTML renderer output.

**Add** `web/testdata/sample_report.json`:

```json
{
  "version": "1",
  "strings": ["Query1", "StepA", "StepB", "Measure1"],
  "ops": [
    {
      "kind": "QueryDefinitionChanged",
      "name": 0,
      "change_kind": "semantic",
      "semantic_detail": {
        "step_diffs": [
          { "kind": "step_added", "step": { "name": 1, "index": 0, "step_type": "other", "source_refs": [], "params": null, "signature": null } },
          {
            "kind": "step_modified",
            "before": { "name": 1, "index": 0, "step_type": "other", "source_refs": [], "params": null, "signature": null },
            "after": { "name": 2, "index": 1, "step_type": "other", "source_refs": [], "params": null, "signature": null },
            "changes": [{ "kind": "renamed", "from": 1, "to": 2 }]
          }
        ],
        "ast_summary": { "mode": "unknown", "node_count_old": 10, "node_count_new": 11, "inserted": 1, "deleted": 0, "updated": 0, "moved": 0, "move_hints": [] }
      }
    },
    { "kind": "MeasureDefinitionChanged", "name": 3 }
  ],
  "complete": true,
  "warnings": []
}
```

**Add** `web/test_render.js`:

```js
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { renderReportHtml } from "./render.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const p = path.join(__dirname, "testdata", "sample_report.json");
const report = JSON.parse(fs.readFileSync(p, "utf8"));
const html = renderReportHtml(report);

function mustInclude(s) {
  if (!html.includes(s)) {
    console.error("Missing:", s);
    process.exit(1);
  }
}

mustInclude("Power Query");
mustInclude("Query: Query1");
mustInclude("Step diffs");
mustInclude("Measure: Measure1");

console.log("ok");
```

Then add a simple workflow (new file) to run it.

**Add new file** `.github/workflows/web_ui_tests.yml`:

```yaml
name: Web UI Tests

on:
  pull_request:
  push:
    branches: [main, master]

jobs:
  web-ui:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "20"
      - name: Render test
        run: node web/test_render.js
```

This keeps the UI test stable and fast, and it won’t affect wasm size budgets because it only touches JS/HTML.

---

## Feature 3: Release-readiness documentation

The sprint outline explicitly calls for:

* updating `m_parser_coverage.md`
* adding a release readiness checklist doc 

### Step 1: Add `docs/m_parser_coverage.md`

If the file doesn’t exist in the repo, add it; if it exists, replace its contents with something like this:

```md
# Semantic M parser coverage

This doc describes which Power Query (M) constructs are parsed semantically versus treated as opaque text.

## Terms

- Semantic parse: the code builds an AST and can reason about structure (steps, references).
- Opaque: the code treats the query definition as a string blob and only reports text/format changes.

## Supported constructs

The semantic parser supports:
- Records, lists
- let/in expressions
- if/then/else expressions
- Binary operators (arithmetic, comparisons, logical)
- Function definitions (lambdas)
- try/otherwise
- Type ascription

## Not yet semantic

Anything not in the supported list is treated as opaque. In those cases:
- the diff can still detect that the query changed
- step-level diffs may be missing
- AST summary may be unknown or absent

## How to update this doc

Whenever new M syntax becomes semantic, add it to Supported constructs and add a fixture + test that exercises it.
```

### Step 2: Add `docs/release_readiness.md`

```md
# Release readiness checklist

## Host formats
- Workbooks: .xlsx, .xlsm, .xltx, .xltm
- Power BI: .pbix, .pbit

## PBIX boundaries
- If PBIX has DataMashup, Power Query diffs are available.
- If PBIX has no DataMashup but has DataModelSchema, model diffs (measures) are available.
- If PBIX has neither, the tool should return a clear error.

## Outputs
- Text output: all DiffOp variants should be represented (at least a fallback line).
- JSON output: schema_version is present and stable.

## Limits / knobs
- max memory, timeout, max ops: documented and tested.
- When limits hit: report.complete=false and warnings populated.

## Determinism
- Parallel runs (different thread counts) produce identical JSON and text outputs.

## CI gates
- Unit tests + integration tests pass.
- Fuzz targets run in CI or scheduled workflow.
- wasm build stays under size budgets.
```

### Step 3: Mark the sprint plan doc as completed

The outline mentions annotating `next_sprint_plan.md` as completed. 
If that file exists in the repo, add a short header at the top such as:

* “Status: Completed”
* Date + commit hash
* Link to the outline

(If it doesn’t exist, skip, or add `docs/next_sprint_plan.md` and note it’s historical.)

---

## Why this plan is low-risk

* It reuses existing core functionality instead of changing diff semantics.
* CLI changes are mostly wiring + one new module, and the new behavior is covered by a fixture-backed integration test.
* Web demo changes stay on the JS side; the wasm API already supports host detection by filename. 
* Release docs are additive; they don’t change runtime behavior.
