import init, { diff_files_json, get_version } from "./wasm/excel_diff_wasm.js";

let wasmReady = false;
let oldFileData = null;
let newFileData = null;
let oldFileName = "";
let newFileName = "";
let lastReport = null;

const allowedExtensions = [".xlsx", ".xlsm", ".xltx", ".xltm", ".pbix", ".pbit"];

function byId(id) {
  return document.getElementById(id);
}

function hasAllowedExtension(name) {
  const lower = String(name || "").toLowerCase();
  return allowedExtensions.some((ext) => lower.endsWith(ext));
}

function escapeHtml(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function showError(message) {
  const errorEl = byId("error");
  errorEl.textContent = message;
  errorEl.style.display = "block";
}

function hideError() {
  byId("error").style.display = "none";
}

function updateDiffButton() {
  const btn = byId("diffBtn");
  btn.disabled = !wasmReady || !oldFileData || !newFileData;
}

async function initWasm() {
  try {
    await init();
    wasmReady = true;
    const versionEl = byId("version");
    if (versionEl) {
      versionEl.textContent = get_version();
    }
  } catch (err) {
    showError("Failed to initialize WebAssembly module: " + err.message);
  }
}

function setupFileInput(inputId, boxId, nameId, setter) {
  const input = byId(inputId);
  const box = byId(boxId);
  const nameEl = byId(nameId);

  input.addEventListener("change", async (e) => {
    const file = e.target.files[0];
    if (!file) return;

    if (!hasAllowedExtension(file.name)) {
      showError("Unsupported file type. Use .xlsx, .xlsm, .pbix, or .pbit.");
      return;
    }

    try {
      const buffer = await file.arrayBuffer();
      setter(new Uint8Array(buffer), file.name);
      nameEl.textContent = file.name;
      box.classList.add("has-file");
      hideError();
      updateDiffButton();
    } catch (err) {
      showError("Failed to read file: " + err.message);
    }
  });

  box.addEventListener("dragover", (e) => {
    e.preventDefault();
    box.classList.add("dragover");
  });

  box.addEventListener("dragleave", () => {
    box.classList.remove("dragover");
  });

  box.addEventListener("drop", async (e) => {
    e.preventDefault();
    box.classList.remove("dragover");

    const file = e.dataTransfer.files[0];
    if (!file) return;

    if (!hasAllowedExtension(file.name)) {
      showError("Unsupported file type. Use .xlsx, .xlsm, .pbix, or .pbit.");
      return;
    }

    try {
      const buffer = await file.arrayBuffer();
      setter(new Uint8Array(buffer), file.name);
      nameEl.textContent = file.name;
      box.classList.add("has-file");
      hideError();
      updateDiffButton();
    } catch (err) {
      showError("Failed to read file: " + err.message);
    }
  });
}

function resolveString(report, id) {
  if (typeof id !== "number") return "";
  const arr = report.strings || [];
  return id >= 0 && id < arr.length ? arr[id] : "";
}

function opKind(op) {
  return op && typeof op.kind === "string" ? op.kind : "Unknown";
}

function isQueryOp(op) {
  const k = opKind(op);
  return k.startsWith("Query");
}

function isMeasureOp(op) {
  const k = opKind(op);
  return k.startsWith("Measure");
}

function formatStepType(stepType) {
  switch (stepType) {
    case "table_select_rows":
      return "Table.SelectRows";
    case "table_remove_columns":
      return "Table.RemoveColumns";
    case "table_rename_columns":
      return "Table.RenameColumns";
    case "table_transform_column_types":
      return "Table.TransformColumnTypes";
    case "table_nested_join":
      return "Table.NestedJoin";
    case "table_join":
      return "Table.Join";
    default:
      return stepType || "Other";
  }
}

function queryKeyForOp(report, op) {
  const k = opKind(op);
  if (k === "QueryRenamed") {
    return resolveString(report, op.to) || resolveString(report, op.from);
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
    return `<li>~ reordered: ${name} (${sd.from_index} -> ${sd.to_index})</li>`;
  }
  if (kind === "step_modified") {
    const after = sd.after || {};
    const name = escapeHtml(resolveString(report, after.name));
    const t = escapeHtml(formatStepType(after.step_type));
    const changes = Array.isArray(sd.changes) ? sd.changes : [];
    const changeBits = changes.map((c) => {
      if (!c || typeof c.kind !== "string") return "unknown";
      if (c.kind === "renamed") {
        const from = escapeHtml(resolveString(report, c.from));
        const to = escapeHtml(resolveString(report, c.to));
        return `renamed ${from} -> ${to}`;
      }
      if (c.kind === "source_refs_changed") {
        const removed = Array.isArray(c.removed) ? c.removed.length : 0;
        const added = Array.isArray(c.added) ? c.added.length : 0;
        return `source refs -${removed} +${added}`;
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
  const headerBits = [];
  const bodyBits = [];

  for (const op of ops) {
    const k = opKind(op);
    if (k === "QueryAdded") headerBits.push("added");
    else if (k === "QueryRemoved") headerBits.push("removed");
    else if (k === "QueryRenamed") headerBits.push("renamed");
    else if (k === "QueryMetadataChanged") headerBits.push("metadata");
    else if (k === "QueryDefinitionChanged") headerBits.push(op.change_kind || "changed");

    if (k === "QueryRenamed") {
      const from = escapeHtml(resolveString(report, op.from));
      const to = escapeHtml(resolveString(report, op.to));
      bodyBits.push(`<div class="q-detail">rename: ${from} -> ${to}</div>`);
    }

    if (k === "QueryMetadataChanged") {
      const field = escapeHtml(op.field || "metadata");
      const oldVal = escapeHtml(resolveString(report, op.old) || "(none)");
      const newVal = escapeHtml(resolveString(report, op.new) || "(none)");
      bodyBits.push(`<div class="q-detail">${field}: ${oldVal} -> ${newVal}</div>`);
    }

    if (k === "QueryDefinitionChanged") {
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

  const headerTag = headerBits.length
    ? ` <span class="tag">${escapeHtml(Array.from(new Set(headerBits)).join(", "))}</span>`
    : "";
  const title = escapeHtml(key || "(unnamed query)");
  const inner = bodyBits.join("");

  return `
    <details class="card">
      <summary><span class="title">${title}</span>${headerTag}</summary>
      <div class="body">${inner || "<div class='muted'>(no details)</div>"}</div>
    </details>
  `;
}

function renderQueries(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  const queryOps = ops.filter(isQueryOp);

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
    <section>
      <h2>Power Query</h2>
      <div class="cards">${cards}</div>
    </section>
  `;
}

function renderMeasures(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  const measureOps = ops.filter(isMeasureOp);

  if (measureOps.length === 0) return "";

  const rows = measureOps
    .map((op) => {
      const k = opKind(op);
      const name = escapeHtml(resolveString(report, op.name));
      if (k === "MeasureAdded") return `<li>+ ${name}</li>`;
      if (k === "MeasureRemoved") return `<li>- ${name}</li>`;
      if (k === "MeasureDefinitionChanged") return `<li>* ${name} (definition changed)</li>`;
      return `<li>${escapeHtml(JSON.stringify(op))}</li>`;
    })
    .join("");

  return `
    <section>
      <h2>Measures</h2>
      <ul>${rows}</ul>
    </section>
  `;
}

function renderWarnings(report) {
  const warnings = Array.isArray(report.warnings) ? report.warnings : [];
  if (warnings.length === 0) return "";

  const items = warnings.map((w) => `<li>${escapeHtml(w)}</li>`).join("");
  return `
    <section>
      <h2>Warnings</h2>
      <ul>${items}</ul>
    </section>
  `;
}

function renderRawOps(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  const rawOps = ops.filter((op) => !isQueryOp(op) && !isMeasureOp(op));
  const rawContainer = byId("rawOps");
  const rawBody = byId("rawOpsBody");

  if (rawOps.length === 0) {
    rawContainer.style.display = "none";
    rawBody.textContent = "";
    return;
  }

  const lines = rawOps.map((op) => JSON.stringify(op)).join("\n");
  rawBody.textContent = lines;
  rawContainer.style.display = "block";
}

function renderReport(report) {
  const parts = [];
  const warnings = renderWarnings(report);
  if (warnings) parts.push(warnings);

  const queries = renderQueries(report);
  if (queries) parts.push(queries);

  const measures = renderMeasures(report);
  if (measures) parts.push(measures);

  if (parts.length === 0) {
    parts.push("<div class='muted'>(no semantic ops)</div>");
  }

  byId("semanticOutput").innerHTML = parts.join("");
  renderRawOps(report);
}

function updateSummary(report) {
  const opCount = Array.isArray(report.ops) ? report.ops.length : 0;
  const warnings = Array.isArray(report.warnings) ? report.warnings.length : 0;
  const complete = report.complete ? "true" : "false";

  const opCountEl = byId("opCount");
  opCountEl.textContent = opCount;
  opCountEl.className =
    "summary-value" + (opCount > 100 ? " danger" : opCount > 0 ? " warning" : "");

  const warningEl = byId("warningCount");
  warningEl.textContent = warnings;
  warningEl.className = "summary-value" + (warnings > 0 ? " warning" : "");

  const completeEl = byId("completeStatus");
  completeEl.textContent = complete;
  completeEl.className = "summary-value" + (complete === "false" ? " danger" : "");
}

async function runDiff() {
  if (!wasmReady || !oldFileData || !newFileData) return;

  const btn = byId("diffBtn");
  const results = byId("results");
  const emptyState = byId("emptyState");

  btn.innerHTML = '<span class="loading-spinner"></span>Comparing...';
  btn.classList.add("loading");
  hideError();

  try {
    const json = diff_files_json(oldFileData, newFileData, oldFileName, newFileName);
    lastReport = json;

    const report = JSON.parse(json);
    updateSummary(report);
    renderReport(report);

    results.style.display = "block";
    emptyState.style.display = "none";
  } catch (err) {
    showError("Diff failed: " + err);
    results.style.display = "none";
    emptyState.style.display = "block";
  } finally {
    btn.innerHTML = "Compare Files";
    btn.classList.remove("loading");
  }
}

function downloadJson() {
  if (!lastReport) return;

  const blob = new Blob([lastReport], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "excel-diff-report.json";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

document.addEventListener("DOMContentLoaded", async () => {
  setupFileInput("oldFile", "oldFileBox", "oldFileName", (data, name) => {
    oldFileData = data;
    oldFileName = name;
  });
  setupFileInput("newFile", "newFileBox", "newFileName", (data, name) => {
    newFileData = data;
    newFileName = name;
  });

  byId("diffBtn").addEventListener("click", runDiff);
  byId("downloadBtn").addEventListener("click", downloadJson);

  await initWasm();
  updateDiffButton();
});
