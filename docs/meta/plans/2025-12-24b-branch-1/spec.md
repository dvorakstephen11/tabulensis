Below is a concrete, file-level implementation plan to close **Section 1** of `next_sprint_outline.md` (the four remaining gaps), using the current architecture described in `codebase_context.md`. 

---

## Gap A: Web demo shows semantic Power Query changes (per-query cards + step diffs) and PBIX/PBIT measure diffs

This corresponds to the missing “web viewer semantic presentation” and the “PBIX/PBIT measure diffs in web demo” items. 

### A1) Add “diff any file type” support to the WASM bindings

Right now WASM only opens `WorkbookPackage` and exposes `diff_workbooks_json(old_bytes, new_bytes)` (so `.pbix/.pbit` cannot work). 
Meanwhile the CLI already selects host kind by extension (`xlsx/xlsm/...` vs `pbix/pbit`) and opens `PbixPackage` accordingly. 

**Plan**

1. Add a new WASM-exported function that accepts both bytes *and* filenames (or extensions), so we can select the host kind in Rust (same decision logic as CLI).
2. Keep `diff_workbooks_json` for backwards compatibility, but have the web demo call the new function.
3. Update `wasm/Cargo.toml` to enable `model-diff` (needed for measure ops to serialize and for PBIX “no DataMashup” fallback).

#### Code change: `wasm/Cargo.toml` enable `model-diff`

Replace this:

```toml
excel_diff = { path = "./core", default-features = false, features = ["excel-open-xml"] }
```

With this:

```toml
excel_diff = { path = "./core", default-features = false, features = ["excel-open-xml", "model-diff"] }
```



#### Code change: `wasm/src/lib.rs` add `diff_files_json(...)`

Replace this (existing WASM entrypoint):

```rust
#[wasm_bindgen]
pub fn diff_workbooks_json(old_bytes: &[u8], new_bytes: &[u8]) -> Result<String, JsValue> {
    let old_cursor = Cursor::new(old_bytes.to_vec());
    let new_cursor = Cursor::new(new_bytes.to_vec());

    let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open old file: {}", e)))?;
    let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open new file: {}", e)))?;

    let report = pkg_old.diff(&pkg_new, &excel_diff::DiffConfig::default());

    excel_diff::serialize_diff_report(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}
```

With this (keep `diff_workbooks_json`, but implement a general entrypoint and delegate):

```rust
use wasm_bindgen::prelude::*;
use std::io::Cursor;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HostKind {
    Workbook,
    Pbix,
}

fn host_kind_from_name(name: &str) -> Option<HostKind> {
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "xlsx" | "xlsm" | "xltx" | "xltm" => Some(HostKind::Workbook),
        "pbix" | "pbit" => Some(HostKind::Pbix),
        _ => None,
    }
}

#[wasm_bindgen]
pub fn diff_files_json(
    old_bytes: &[u8],
    new_bytes: &[u8],
    old_name: &str,
    new_name: &str,
) -> Result<String, JsValue> {
    let kind_old = host_kind_from_name(old_name)
        .ok_or_else(|| JsValue::from_str("Unsupported old file extension"))?;
    let kind_new = host_kind_from_name(new_name)
        .ok_or_else(|| JsValue::from_str("Unsupported new file extension"))?;

    if kind_old != kind_new {
        return Err(JsValue::from_str("Old/new files must be the same type"));
    }

    let old_cursor = Cursor::new(old_bytes.to_vec());
    let new_cursor = Cursor::new(new_bytes.to_vec());

    let cfg = excel_diff::DiffConfig::default();

    let report = match kind_old {
        HostKind::Workbook => {
            let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old workbook: {}", e)))?;
            let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new workbook: {}", e)))?;
            pkg_old.diff(&pkg_new, &cfg)
        }
        HostKind::Pbix => {
            let pkg_old = excel_diff::PbixPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old PBIX/PBIT: {}", e)))?;
            let pkg_new = excel_diff::PbixPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new PBIX/PBIT: {}", e)))?;
            pkg_old.diff(&pkg_new, &cfg)
        }
    };

    excel_diff::serialize_diff_report(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}

#[wasm_bindgen]
pub fn diff_workbooks_json(old_bytes: &[u8], new_bytes: &[u8]) -> Result<String, JsValue> {
    diff_files_json(old_bytes, new_bytes, "old.xlsx", "new.xlsx")
}
```



**Acceptance checks**

* Web demo can diff `.xlsx/.xlsm` and `.pbix/.pbit`.
* WASM size budgets still enforced by existing workflows (10MB for web demo wasm; 5MB for `wasm_smoke`). 

---

### A2) Update the web UI to render semantic query changes as per-query cards

The diff output already carries semantic detail at the `DiffOp::QueryDefinitionChanged` level via `semantic_detail: Option<QuerySemanticDetail>`, including `step_diffs` and `ast_summary`. 
The CLI already formats these step diffs, which is a good blueprint for the web viewer. 

**Plan**

1. Parse the JSON `DiffReport` (strings table + ops).
2. Build a “Power Query” view:

   * Group ops by query key (top-level query name, or `section/name` for embedded queries).
   * Render each group as a collapsible card.
   * For definition changes:

     * Show `change_kind` (semantic vs formatting_only vs renamed).
     * If `semantic_detail.step_diffs` exists: show a list of step diffs with step type labels.
     * Else if `semantic_detail.ast_summary` exists: show the AST summary counts and mode.
3. Build a “Measures” view (measure ops from PBIX/PBIT extraction; see Gap D).
4. Keep a “Raw ops” fallback section for everything else.

#### Suggested web implementation approach (minimal dependencies)

Since `pages.yml` deploys everything under `web/` as static content, keep it plain `index.html` + `main.js` and load `web/wasm/excel_diff_wasm.js` produced by wasm-pack. 

##### New `web/main.js` (drop-in template)

Create/replace `web/main.js` with:

```js
import init, { diff_files_json, get_version } from "./wasm/excel_diff_wasm.js";

function byId(id) {
  return document.getElementById(id);
}

function escapeHtml(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function resolveString(report, id) {
  if (typeof id !== "number") return "";
  const arr = report.strings || [];
  return id >= 0 && id < arr.length ? arr[id] : "";
}

function opKind(op) {
  return op && typeof op.kind === "string" ? op.kind : "Unknown";
}

function formatStepType(stepType) {
  switch (stepType) {
    case "table_select_rows": return "Table.SelectRows";
    case "table_remove_columns": return "Table.RemoveColumns";
    case "table_rename_columns": return "Table.RenameColumns";
    case "table_transform_column_types": return "Table.TransformColumnTypes";
    case "table_nested_join": return "Table.NestedJoin";
    case "table_join": return "Table.Join";
    default: return stepType || "Other";
  }
}

function queryKeyForOp(report, op) {
  const k = opKind(op);
  if (k.startsWith("EmbeddedQuery")) {
    const section = resolveString(report, op.section);
    if (k === "EmbeddedQueryRenamed") {
      const to = resolveString(report, op.to);
      return section ? section + "/" + to : to;
    }
    const name = resolveString(report, op.name);
    return section ? section + "/" + name : name;
  }

  if (k === "QueryRenamed") {
    return resolveString(report, op.to);
  }
  return resolveString(report, op.name);
}

function renderStepDiff(report, sd) {
  const kind = sd.kind || "unknown";
  if (kind === "step_added") {
    const step = sd.step || {};
    const name = escapeHtml(resolveString(report, step.name));
    const t = escapeHtml(formatStepType(step.step_type));
    return `<li>+ ${name} (${t})</li>`;
  }
  if (kind === "step_removed") {
    const step = sd.step || {};
    const name = escapeHtml(resolveString(report, step.name));
    const t = escapeHtml(formatStepType(step.step_type));
    return `<li>- ${name} (${t})</li>`;
  }
  if (kind === "step_reordered") {
    const name = escapeHtml(resolveString(report, sd.name));
    return `<li>~ reordered: ${name} (${sd.old_index} -> ${sd.new_index})</li>`;
  }
  if (kind === "step_modified") {
    const oldStep = sd.old_step || {};
    const newStep = sd.new_step || {};
    const name = escapeHtml(resolveString(report, newStep.name));
    const t = escapeHtml(formatStepType(newStep.step_type));
    const changes = Array.isArray(sd.changes) ? sd.changes : [];
    const changeBits = changes.map((c) => {
      if (!c || typeof c.kind !== "string") return "unknown";
      if (c.kind === "renamed") {
        const from = escapeHtml(resolveString(report, c.from));
        const to = escapeHtml(resolveString(report, c.to));
        return `renamed ${from} -> ${to}`;
      }
      if (c.kind === "source_refs_changed") {
        return "source refs changed";
      }
      if (c.kind === "params_changed") {
        return "params changed";
      }
      return c.kind;
    });
    return `<li>* ${name} (${t}) [${escapeHtml(changeBits.join(", "))}]</li>`;
  }
  return `<li>${escapeHtml(JSON.stringify(sd))}</li>`;
}

function renderAstSummary(summary) {
  if (!summary) return "";
  const mode = escapeHtml(summary.mode || "");
  const i = summary.inserted || 0;
  const d = summary.deleted || 0;
  const u = summary.updated || 0;
  const m = summary.moved || 0;
  return `<div class="ast-summary">AST diff (${mode}): +${i} -${d} ~${u} moved:${m}</div>`;
}

function renderQueryCard(report, key, ops) {
  let headerBits = [];
  let bodyBits = [];

  for (const op of ops) {
    const k = opKind(op);
    if (k === "QueryAdded" || k === "EmbeddedQueryAdded") headerBits.push("added");
    else if (k === "QueryRemoved" || k === "EmbeddedQueryRemoved") headerBits.push("removed");
    else if (k === "QueryRenamed" || k === "EmbeddedQueryRenamed") headerBits.push("renamed");
    else if (k === "QueryMetadataChanged" || k === "EmbeddedQueryMetadataChanged") headerBits.push("metadata");
    else if (k === "QueryDefinitionChanged" || k === "EmbeddedQueryDefinitionChanged") headerBits.push(op.change_kind || "changed");

    if (k === "QueryDefinitionChanged" || k === "EmbeddedQueryDefinitionChanged") {
      const kind = escapeHtml(op.change_kind || "");
      const detail = op.semantic_detail || null;

      if (detail && Array.isArray(detail.step_diffs) && detail.step_diffs.length > 0) {
        const items = detail.step_diffs.map((sd) => renderStepDiff(report, sd)).join("");
        bodyBits.push(`<div class="q-detail"><div>change: ${kind}</div><ul>${items}</ul></div>`);
      } else if (detail && detail.ast_summary) {
        bodyBits.push(`<div class="q-detail"><div>change: ${kind}</div>${renderAstSummary(detail.ast_summary)}</div>`);
      } else {
        bodyBits.push(`<div class="q-detail"><div>change: ${kind}</div><div class="muted">(no semantic details)</div></div>`);
      }
    }
  }

  const headerTag = headerBits.length ? ` <span class="tag">${escapeHtml(Array.from(new Set(headerBits)).join(", "))}</span>` : "";
  const title = escapeHtml(key || "(unnamed query)");
  const inner = bodyBits.join("");

  return `
    <details class="card">
      <summary><span class="title">${title}</span>${headerTag}</summary>
      <div class="body">${inner || "<div class='muted'>(no details)</div>"}</div>
    </details>
  `;
}

function renderMeasures(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  const measureOps = ops.filter((op) => {
    const k = opKind(op);
    return k === "MeasureAdded" || k === "MeasureRemoved" || k === "MeasureDefinitionChanged";
  });

  if (measureOps.length === 0) return "";

  const rows = measureOps.map((op) => {
    const k = opKind(op);
    const name = escapeHtml(resolveString(report, op.name));
    if (k === "MeasureAdded") return `<li>+ ${name}</li>`;
    if (k === "MeasureRemoved") return `<li>- ${name}</li>`;
    if (k === "MeasureDefinitionChanged") return `<li>* ${name} (definition changed)</li>`;
    return `<li>${escapeHtml(JSON.stringify(op))}</li>`;
  }).join("");

  return `
    <h2>Measures</h2>
    <ul>${rows}</ul>
  `;
}

function renderQueries(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  const queryOps = ops.filter((op) => {
    const k = opKind(op);
    return k.startsWith("Query") || k.startsWith("EmbeddedQuery");
  });

  if (queryOps.length === 0) return "";

  const groups = new Map();
  for (const op of queryOps) {
    const key = queryKeyForOp(report, op);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(op);
  }

  const keys = Array.from(groups.keys()).sort((a, b) => a.localeCompare(b));
  const cards = keys.map((k) => renderQueryCard(report, k, groups.get(k))).join("");

  return `
    <h2>Power Query</h2>
    <div class="cards">${cards}</div>
  `;
}

function renderSummary(report) {
  const complete = report.complete ? "true" : "false";
  const warnings = Array.isArray(report.warnings) ? report.warnings.length : 0;
  const ops = Array.isArray(report.ops) ? report.ops.length : 0;
  return `<div class="summary">complete=${complete} ops=${ops} warnings=${warnings}</div>`;
}

function renderReport(report) {
  return `
    ${renderSummary(report)}
    ${renderQueries(report)}
    ${renderMeasures(report)}
  `;
}

async function readFileBytes(file) {
  const buf = await file.arrayBuffer();
  return new Uint8Array(buf);
}

async function runDiff() {
  const oldFile = byId("oldFile").files[0];
  const newFile = byId("newFile").files[0];
  if (!oldFile || !newFile) {
    byId("out").innerHTML = "<div class='error'>Pick both files.</div>";
    return;
  }

  const oldBytes = await readFileBytes(oldFile);
  const newBytes = await readFileBytes(newFile);

  byId("out").innerHTML = "<div class='muted'>Diffing...</div>";

  try {
    const json = diff_files_json(oldBytes, newBytes, oldFile.name, newFile.name);
    const report = JSON.parse(json);
    byId("out").innerHTML = renderReport(report);
  } catch (e) {
    byId("out").innerHTML = "<pre class='error'>" + escapeHtml(String(e)) + "</pre>";
  }
}

async function main() {
  await init();
  byId("version").textContent = get_version();
  byId("diffBtn").addEventListener("click", runDiff);
}

main();
```

This is intentionally “dumb but robust”: it only depends on the `DiffReport` schema that already exists (strings + ops), and it’s tolerant of unknown op kinds.

**Acceptance checks**

* Queries render as collapsible cards, grouped by query name.
* Definition changes show `change_kind` and either step diffs or AST summary. 
* PBIX/PBIT uploads show measure ops under “Measures” once Gap D ships. 

---

## Gap B: Release-readiness documentation (support boundaries, semantic diff, resource ceilings, checklist)

This corresponds to the “release-readiness documentation” item in Section 1. 

### B1) Add a single “Release Readiness” doc that answers the three questions users ask

**Plan: create `docs/release_readiness.md`** (or similar)
Include these sections (explicitly called out by `next_sprint_outline.md`): 

1. **Support boundaries**

   * Excel workbook (`.xlsx/.xlsm`) coverage (grid diffs, objects, VBA if enabled).
   * Power Query M diffs from `DataMashup` (legacy PBIX/PBIT and workbooks with queries).
   * PBIX/PBIT “enhanced metadata” limitation: when `DataMashup` is missing, we fall back to `DataModelSchema` and only emit *measure-level* diffs (see Gap D).
   * Explicitly: we do **not** diff report visuals/layout in PBIX.
2. **Semantic M diff**

   * Define “semantic change” vs “formatting-only” vs “renamed” (these are `QueryChangeKind`). 
   * Explain “step diffs”: the human-friendly “Applied Steps” model derived from the M AST.
   * Explain AST fallback summary (`AstDiffSummary`) if step extraction is incomplete.
3. **Resource ceilings**

   * Explain `DiffConfig.max_memory_mb`, `DiffConfig.timeout_seconds`, `DiffConfig.max_ops`, and limit behavior (`StopWithWarning`, etc.). 
   * Give recommended presets (“fast” vs “precise”) and how to reason about them.

### B2) Update `docs/m_parser_coverage.md` to match reality

Even if this doc already exists in-repo, the plan is to:

* Add a “Tier1 / Tier2” table of M syntax coverage (identifiers, access, if/then/else, `each`, operators, lambdas, try/otherwise, type-ascription) as described in `next_sprint_outline.md`. 
* Include 2–3 “known gaps” that cause step extraction to fall back to AST summary.

### B3) Add a practical release checklist (copy/paste)

Add `docs/release_checklist.md` with:

* Run `cargo test --workspace` (CI already does this). 
* Run fuzz workflow locally or verify scheduled fuzz run is green. 
* Verify WASM budgets (smoke + web demo). 
* Verify web demo deployment (pages workflow). 
* Quick manual smoke on:

  * `.xlsx` with M queries (should show step diffs)
  * `.pbix/.pbit` with and without DataMashup (should show either query diffs or measure diffs)

---

## Gap C: “Productize” fuzz coverage (run the right targets in CI, remove duplicate harness)

Section 1 calls out that the important M fuzz target isn’t running in CI and that the fuzz harness is duplicated. 
Today, CI runs:

* core/fuzz: `fuzz_datamashup_parse`, `fuzz_diff_grids`
* top-level fuzz: `datamashup`, `diff_engine` 
  …but **does not** run `core/fuzz`’s `fuzz_m_section_and_ast`, even though it’s defined. 

### C1) Delete the redundant top-level `/fuzz` harness and standardize on `core/fuzz`

The top-level `fuzz/` is a second, separate cargo-fuzz package that duplicates coverage and runs a different set of fuzz targets. 
Keep `core/fuzz` as the canonical harness.

**Plan**

1. Remove `/fuzz` directory.
2. Update `.github/workflows/fuzz.yml` to only run `core/fuzz` targets.

### C2) Update `.github/workflows/fuzz.yml` to run the M fuzz target

Replace this portion:

```yaml
      - name: Fuzz core datamashup framing
        run: cargo fuzz run fuzz_datamashup_parse -max_total_time=60
        working-directory: core/fuzz

      - name: Fuzz core grid diff
        run: cargo fuzz run fuzz_diff_grids -max_total_time=60
        working-directory: core/fuzz

      - name: Fuzz datamashup framing (top-level)
        run: cargo fuzz run datamashup -max_total_time=60
        working-directory: fuzz

      - name: Fuzz diff engine (top-level)
        run: cargo fuzz run diff_engine -max_total_time=60
        working-directory: fuzz

      - name: Upload fuzz artifacts
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: fuzz-artifacts
          path: |
            core/fuzz/artifacts
            fuzz/artifacts
          if-no-files-found: ignore
```

With this:

```yaml
      - name: Fuzz core datamashup framing
        run: cargo fuzz run fuzz_datamashup_parse -max_total_time=60
        working-directory: core/fuzz

      - name: Fuzz core grid diff
        run: cargo fuzz run fuzz_diff_grids -max_total_time=60
        working-directory: core/fuzz

      - name: Fuzz M parser + AST diff
        run: cargo fuzz run fuzz_m_section_and_ast -max_total_time=60
        working-directory: core/fuzz

      - name: Upload fuzz artifacts
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: fuzz-artifacts
          path: |
            core/fuzz/artifacts
          if-no-files-found: ignore
```

**Acceptance checks**

* Scheduled fuzz workflow executes `fuzz_m_section_and_ast` at least 60 seconds/week.
* Only one fuzz harness exists (`core/fuzz`).

---

## Gap D: PBIX/PBIT “no DataMashup” path (fallback to DataModelSchema and emit measure ops)

Section 1 calls out the current limitation: PBIX/PBIT without `DataMashup` errors out today. 
Current behavior: `PbixPackage::open` reads `DataMashup` and if missing (but the container “looks like PBIX”), returns the dedicated `NoDataMashupUseTabularModel` error. 

There’s already a model diff layer that emits `MeasureAdded/Removed/MeasureDefinitionChanged` ops, but it needs:

* correct union iteration (current code can duplicate ops for changed measures),
* a parser for `DataModelSchema`,
* wiring into `PbixPackage` diff path,
* outputs and fixtures.

### D1) Fix `diff_models` to avoid duplicate ops for keys present in both models

Replace this loop:

```rust
for name in old_by_name.keys().chain(new_by_name.keys()) {
    match (old_by_name.get(name), new_by_name.get(name)) {
        (Some(_old_expr), None) => {
            ops.push(DiffOp::MeasureRemoved { name: *name });
        }
        (None, Some(_new_expr)) => {
            ops.push(DiffOp::MeasureAdded { name: *name });
        }
        (Some(old_expr), Some(new_expr)) => {
            if old_expr != new_expr {
                let old_hash = hash64(&pool.resolve(*old_expr));
                let new_hash = hash64(&pool.resolve(*new_expr));
                ops.push(DiffOp::MeasureDefinitionChanged {
                    name: *name,
                    old_hash,
                    new_hash,
                });
            }
        }
        (None, None) => {}
    }
}
```

With a proper union set:

```rust
use std::collections::{BTreeMap, BTreeSet};

let mut names: BTreeSet<StringId> = BTreeSet::new();
names.extend(old_by_name.keys().copied());
names.extend(new_by_name.keys().copied());

for name in names {
    match (old_by_name.get(&name), new_by_name.get(&name)) {
        (Some(_old_expr), None) => {
            ops.push(DiffOp::MeasureRemoved { name });
        }
        (None, Some(_new_expr)) => {
            ops.push(DiffOp::MeasureAdded { name });
        }
        (Some(old_expr), Some(new_expr)) => {
            if old_expr != new_expr {
                let old_hash = hash64(&pool.resolve(*old_expr));
                let new_hash = hash64(&pool.resolve(*new_expr));
                ops.push(DiffOp::MeasureDefinitionChanged {
                    name,
                    old_hash,
                    new_hash,
                });
            }
        }
        (None, None) => {}
    }
}
```



### D2) Implement a minimal `DataModelSchema` parser that extracts measures

**Key insight:** in this codebase, the diff report uses a central `StringPool` and `StringId`s for stable references. 
So we should parse `DataModelSchema` into a *raw* model with owned `String`s, then intern those strings during `diff(...)`.

**Plan**

1. Add a new module behind `feature = "model-diff"`: `core/src/tabular_schema.rs`.
2. Implement:

   * `parse_data_model_schema(bytes) -> RawTabularModel`
   * `build_model(raw, pool) -> Model` (interns strings into pool)
3. Use a best-effort JSON extraction:

   * try `model.tables[].measures[]`
   * fallback: search for any object containing a `measures` array.

#### New file: `core/src/tabular_schema.rs`

Create this file:

```rust
use serde_json::Value;

use crate::excel_open_xml::PackageError;
use crate::string_pool::StringPool;

use crate::model::{Measure, Model};

#[derive(Debug, Clone, Default)]
pub(crate) struct RawTabularModel {
    pub measures: Vec<RawMeasure>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawMeasure {
    pub full_name: String,
    pub expression: String,
}

fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{FEFF}').unwrap_or(s)
}

pub(crate) fn parse_data_model_schema(bytes: &[u8]) -> Result<RawTabularModel, PackageError> {
    let text = std::str::from_utf8(bytes).map_err(|e| PackageError::UnsupportedFormat {
        message: format!("DataModelSchema is not UTF-8: {}", e),
    })?;
    let text = strip_bom(text);

    let v: Value = serde_json::from_str(text).map_err(|e| PackageError::UnsupportedFormat {
        message: format!("DataModelSchema JSON parse error: {}", e),
    })?;

    let mut out = RawTabularModel::default();

    if try_collect_from_model_tables(&v, &mut out) {
        normalize(&mut out);
        return Ok(out);
    }

    collect_measures_anywhere(&v, "", &mut out);
    normalize(&mut out);
    Ok(out)
}

fn try_collect_from_model_tables(v: &Value, out: &mut RawTabularModel) -> bool {
    let model = match v.get("model") {
        Some(m) => m,
        None => return false,
    };
    let tables = match model.get("tables").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return false,
    };

    for t in tables {
        let table_name = t.get("name").and_then(|x| x.as_str()).unwrap_or("");
        if let Some(measures) = t.get("measures").and_then(|m| m.as_array()) {
            for m in measures {
                if let Some(rm) = parse_measure_obj(m, table_name) {
                    out.measures.push(rm);
                }
            }
        }
    }
    true
}

fn parse_measure_obj(v: &Value, table_name: &str) -> Option<RawMeasure> {
    let name = v.get("name").and_then(|x| x.as_str())?;
    let expr = v.get("expression").and_then(|x| x.as_str()).unwrap_or("");

    let full_name = if table_name.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", table_name, name)
    };

    Some(RawMeasure {
        full_name,
        expression: expr.to_string(),
    })
}

fn collect_measures_anywhere(v: &Value, table_name: &str, out: &mut RawTabularModel) {
    match v {
        Value::Object(map) => {
            if let Some(measures) = map.get("measures").and_then(|m| m.as_array()) {
                for m in measures {
                    if let Some(rm) = parse_measure_obj(m, table_name) {
                        out.measures.push(rm);
                    }
                }
            }

            let next_table = map.get("name").and_then(|x| x.as_str()).unwrap_or(table_name);

            for (_k, child) in map {
                collect_measures_anywhere(child, next_table, out);
            }
        }
        Value::Array(arr) => {
            for child in arr {
                collect_measures_anywhere(child, table_name, out);
            }
        }
        _ => {}
    }
}

fn normalize(out: &mut RawTabularModel) {
    out.measures.sort_by(|a, b| a.full_name.cmp(&b.full_name));
    out.measures.dedup_by(|a, b| a.full_name == b.full_name && a.expression == b.expression);
}

pub(crate) fn build_model(raw: &RawTabularModel, pool: &mut StringPool) -> Model {
    let mut m = Model::default();
    for rm in &raw.measures {
        let name = pool.intern(&rm.full_name);
        let expr = pool.intern(&rm.expression);
        m.measures.push(Measure { name, expression: expr });
    }
    m
}
```

### D3) Wire `DataModelSchema` fallback into `PbixPackage::open` and `PbixPackage::diff`

Current `PbixPackage` only stores `data_mashup: Option<DataMashup>` and errors when it’s missing. 

**Plan**

1. Add an optional `model_schema` field (behind `model-diff`).
2. In `open()`:

   * If `DataMashup` exists: behave as today.
   * Else if PBIX markers exist: try reading `DataModelSchema`; if present, parse into `RawTabularModel` and succeed.
   * Else: return dedicated error (but update message to mention “export as PBIT to expose DataModelSchema”).
3. In `diff()`:

   * Keep existing `diff_m_ops_for_packages(...)`.
   * If either side has a `model_schema`, build `Model` structs and append `diff_models(...)` ops.

#### Code change: `core/src/package.rs` PbixPackage struct and open()

Replace this:

```rust
pub struct PbixPackage {
    pub(crate) data_mashup: Option<DataMashup>,
}

impl PbixPackage {
    pub fn open<R: Read + Seek + 'static>(reader: R) -> Result<Self, PackageError> {
        let mut container = ZipContainer::open_from_reader(reader)?;

        let data_mashup_opt = container.read_file_optional_checked("DataMashup")?;

        let data_mashup = if let Some(bytes) = data_mashup_opt {
            let raw = parse_data_mashup(&bytes)?;
            Some(build_data_mashup(&raw))
        } else {
            None
        };

        if data_mashup.is_none() {
            let looks_like_pbix = looks_like_pbix(&mut container)?;
            if looks_like_pbix {
                return Err(PackageError::NoDataMashupUseTabularModel);
            }
        }

        Ok(Self { data_mashup })
    }
```

With this:

```rust
pub struct PbixPackage {
    pub(crate) data_mashup: Option<DataMashup>,
    #[cfg(feature = "model-diff")]
    pub(crate) model_schema: Option<crate::tabular_schema::RawTabularModel>,
}

impl PbixPackage {
    pub fn open<R: Read + Seek + 'static>(reader: R) -> Result<Self, PackageError> {
        let mut container = ZipContainer::open_from_reader(reader)?;

        let data_mashup_opt = container.read_file_optional_checked("DataMashup")?;

        let data_mashup = if let Some(bytes) = data_mashup_opt {
            let raw = parse_data_mashup(&bytes)?;
            Some(build_data_mashup(&raw))
        } else {
            None
        };

        #[cfg(feature = "model-diff")]
        let mut model_schema = None;

        if data_mashup.is_none() {
            let looks_like_pbix = looks_like_pbix(&mut container)?;
            if looks_like_pbix {
                #[cfg(feature = "model-diff")]
                {
                    if let Some(bytes) = container.read_file_optional_checked("DataModelSchema")? {
                        model_schema = Some(crate::tabular_schema::parse_data_model_schema(&bytes)?);
                        return Ok(Self { data_mashup, model_schema });
                    }
                }

                return Err(PackageError::NoDataMashupUseTabularModel);
            }
        }

        Ok(Self {
            data_mashup,
            #[cfg(feature = "model-diff")]
            model_schema,
        })
    }
```



#### Code change: `core/src/package.rs` PbixPackage diff()

Replace this (current pbix diff is mashup-only):

```rust
pub fn diff(&self, other: &Self, cfg: &DiffConfig) -> DiffReport {
    with_default_session(|session| {
        let mut report = DiffReport::new();
        let summary = diff_m_ops_for_packages(
            self.data_mashup.as_ref(),
            other.data_mashup.as_ref(),
            &mut session.strings,
            cfg,
            &mut report,
        );
        report.strings = session.strings.strings();
        report.complete = summary.complete;
        report.warnings = summary.warnings;
        report
    })
}
```

With this:

```rust
pub fn diff(&self, other: &Self, cfg: &DiffConfig) -> DiffReport {
    with_default_session(|session| {
        let mut report = DiffReport::new();

        let summary = diff_m_ops_for_packages(
            self.data_mashup.as_ref(),
            other.data_mashup.as_ref(),
            &mut session.strings,
            cfg,
            &mut report,
        );

        #[cfg(feature = "model-diff")]
        {
            let old_raw = self.model_schema.as_ref();
            let new_raw = other.model_schema.as_ref();

            if old_raw.is_some() || new_raw.is_some() {
                let old_model = old_raw
                    .map(|r| crate::tabular_schema::build_model(r, &mut session.strings))
                    .unwrap_or_default();

                let new_model = new_raw
                    .map(|r| crate::tabular_schema::build_model(r, &mut session.strings))
                    .unwrap_or_default();

                let mut model_ops = crate::model_diff::diff_models(&old_model, &new_model, &session.strings);
                report.ops.append(&mut model_ops);
            }
        }

        report.strings = session.strings.strings();
        report.complete = summary.complete;
        report.warnings = summary.warnings;
        report
    })
}
```

### D4) Make the error actionable when neither path works

The current dedicated error message exists, but Section 1 explicitly wants it to be actionable (ex: “export as PBIT to expose DataModelSchema”). 
This error is currently `PackageError::NoDataMashupUseTabularModel`. 

**Plan**

* Update the `#[error(...)]` string for that variant to explicitly say:

  * “If this is an enhanced-metadata PBIX, export as PBIT to expose DataModelSchema”
  * “If this is legacy, ensure DataMashup is present”

### D5) Enable measure ops in CLI output

CLI already opens PBIX/PBIT (host selection is implemented). 
But the CLI doesn’t enable `model-diff` on the dependency today. 

**Plan**

1. Update `cli/Cargo.toml` dependency to enable `excel_diff/model-diff`.
2. Add text rendering for `MeasureAdded`, `MeasureRemoved`, `MeasureDefinitionChanged` in `cli/src/output/text.rs` (and optionally in git-diff output).
3. Add a “Measures” section similar to the existing “Power Query” section split.

#### Code change: `cli/Cargo.toml` dependency features

Replace:

```toml
excel_diff = { path = "./core" }
```

With:

```toml
excel_diff = { path = "./core", features = ["model-diff"] }
```



### D6) Fixtures + tests

There is an existing PBIX “no_datamashup” fixture generator that writes `DataModelSchema` as `{}` today. 
There is also a core test that currently asserts opening that fixture returns `NoDataMashupUseTabularModel`. 

**Plan**

1. Update the existing test:

   * Opening `pbix_no_datamashup.pbix` should now succeed (it has DataModelSchema).
2. Add a new fixture and test for the true error path:

   * `pbix_no_datamashup_no_schema.pbix` (no `DataMashup`, no `DataModelSchema`) should still return the dedicated error.
3. Add two PBIT fixtures with a real `DataModelSchema` containing at least one measure:

   * A: contains measure `Sales/Total Sales` = `SUM(Sales[Amount])`
   * B: changes expression and/or adds/removes a measure
4. Add a CLI integration test that diffs those two PBITs and asserts measure ops are present in JSON output.

---

## Dependency / sequencing notes (so this lands cleanly)

* **Gap D (PBIX “no DataMashup”) must land before Gap A’s “Measures” section becomes meaningful**, because the web demo needs the measure ops to exist in the diff report JSON. 
* **Gap C (fuzz) is independent** and can land anytime; it just needs workflow edits and deletion of the top-level harness.
* **Gap B (docs) can land anytime**, but it should reference the final behavior of Gap D and the UI behavior of Gap A. 

---

If you want, I can also include a “Definition of Done” checklist per gap (files touched + commands to run) tailored to your existing CI workflows (`ci.yml`, `fuzz.yml`, `wasm.yml`, `pages.yml`).
