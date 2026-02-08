import { renderWorkbookVm } from "./render.js";
import { buildWorkbookViewModel } from "./view_model.js";
import { mountSheetGridViewer } from "./grid_viewer.js";
import { downloadReportJson, downloadHtmlReport, downloadJsonl } from "./export.js";
import {
  createAppDiffClient,
  isDesktop,
  openFileDialog,
  openFolderDialog,
  loadRecents,
  saveRecent,
  loadDiffSummary,
  loadSheetPayload,
  exportAuditXlsx,
  openPath,
  runBatchCompare,
  loadBatchSummary,
  searchDiffOps,
  buildSearchIndex,
  searchWorkbookIndex
} from "./platform.js";

function byId(id) {
  const el = document.getElementById(id);
  if (!el) throw new Error("Missing element: " + id);
  return el;
}

function setStatus(msg, type = "") {
  const status = byId("status");
  status.textContent = msg;
  status.className = type;
}

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function baseName(path) {
  if (!path) return "";
  const parts = String(path).split(/[\\/]/);
  return parts[parts.length - 1] || "";
}

function fileDisplayName(file) {
  if (!file) return "";
  if (file.name) return file.name;
  if (file.path) return baseName(file.path);
  return "";
}

function buildDesktopSelection(path, name) {
  if (!path) return null;
  return {
    path,
    name: name || baseName(path)
  };
}

const BUILTIN_PROFILES = [
  {
    id: "builtin_default_balanced",
    name: "Default (Balanced)",
    builtIn: true,
    preset: "balanced",
    trusted: false,
    noiseFilters: {
      hideMFormattingOnly: false,
      hideDaxFormattingOnly: false,
      hideFormulaFormattingOnly: false,
      collapseMoves: false
    },
    enableMSemanticDiff: true,
    enableFormulaSemanticDiff: false,
    enableDaxSemanticDiff: false,
    semanticNoisePolicy: "report_formatting_only",
    limits: null
  },
  {
    id: "builtin_finance_model_review",
    name: "Finance model review",
    builtIn: true,
    preset: "most_precise",
    trusted: false,
    noiseFilters: {
      hideMFormattingOnly: true,
      hideDaxFormattingOnly: true,
      hideFormulaFormattingOnly: true,
      collapseMoves: true
    },
    enableMSemanticDiff: true,
    enableFormulaSemanticDiff: true,
    enableDaxSemanticDiff: true,
    semanticNoisePolicy: "suppress_formatting_only",
    limits: null
  },
  {
    id: "builtin_data_pipeline_workbook",
    name: "Data pipeline workbook",
    builtIn: true,
    preset: "balanced",
    trusted: false,
    noiseFilters: {
      hideMFormattingOnly: true,
      hideDaxFormattingOnly: false,
      hideFormulaFormattingOnly: false,
      collapseMoves: true
    },
    enableMSemanticDiff: true,
    enableFormulaSemanticDiff: false,
    enableDaxSemanticDiff: false,
    semanticNoisePolicy: "suppress_formatting_only",
    limits: {
      maxMemoryMb: 512,
      timeoutMs: 60_000,
      maxOps: 200_000,
      onLimitExceeded: "return_partial_result"
    }
  },
  {
    id: "builtin_power_bi_model_review",
    name: "Power BI model review",
    builtIn: true,
    preset: "most_precise",
    trusted: false,
    noiseFilters: {
      hideMFormattingOnly: true,
      hideDaxFormattingOnly: true,
      hideFormulaFormattingOnly: false,
      collapseMoves: false
    },
    enableMSemanticDiff: true,
    enableFormulaSemanticDiff: false,
    enableDaxSemanticDiff: true,
    semanticNoisePolicy: "suppress_formatting_only",
    limits: null
  }
];

function safeLocalStorageGet(key) {
  try {
    return localStorage.getItem(key);
  } catch (_) {
    return null;
  }
}

function safeLocalStorageSet(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch (_) {}
}

function loadUserProfilesFromStorage() {
  const raw = safeLocalStorageGet(PROFILE_STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed;
    if (parsed && typeof parsed === "object" && Array.isArray(parsed.profiles)) {
      return parsed.profiles;
    }
  } catch (_) {}
  return [];
}

function saveUserProfilesToStorage(profiles) {
  const payload = { version: 1, profiles: Array.isArray(profiles) ? profiles : [] };
  safeLocalStorageSet(PROFILE_STORAGE_KEY, JSON.stringify(payload));
}

function allProfiles() {
  return [...BUILTIN_PROFILES, ...(userProfiles || [])];
}

function getProfileById(id) {
  if (!id) return null;
  return allProfiles().find(p => p && p.id === id) || null;
}

function ensureSelectedProfile() {
  if (selectedProfileId && getProfileById(selectedProfileId)) return;
  selectedProfileId = safeLocalStorageGet(PROFILE_SELECTED_KEY) || "builtin_default_balanced";
  if (!getProfileById(selectedProfileId)) {
    selectedProfileId = "builtin_default_balanced";
  }
}

function makeProfileId(name) {
  const now = Date.now();
  const slug = String(name || "profile")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40) || "profile";
  return `user_${now}_${slug}`;
}

function rebuildProfileSelectOptions() {
  if (!profileSelect) return;
  profileSelect.innerHTML = "";
  for (const profile of allProfiles()) {
    const opt = document.createElement("option");
    opt.value = profile.id;
    opt.textContent = profile.name;
    profileSelect.append(opt);
  }
  profileSelect.value = selectedProfileId || "builtin_default_balanced";
  updateProfileActionButtons();
}

function updateProfileActionButtons() {
  const profile = getProfileById(selectedProfileId);
  const isBuiltin = Boolean(profile && profile.builtIn);
  if (profileRenameBtn) profileRenameBtn.disabled = isBuiltin;
  if (profileDeleteBtn) profileDeleteBtn.disabled = isBuiltin;
}

function currentProfile() {
  return getProfileById(selectedProfileId) || BUILTIN_PROFILES[0];
}

function engineOptionsFromProfile(profile) {
  const p = profile || {};
  const out = {};
  if (p.preset) out.preset = p.preset;
  if (p.limits) out.limits = p.limits;
  if (p.trusted !== undefined) out.trusted = Boolean(p.trusted);
  if (p.enableMSemanticDiff !== undefined) out.enableMSemanticDiff = Boolean(p.enableMSemanticDiff);
  if (p.enableFormulaSemanticDiff !== undefined) out.enableFormulaSemanticDiff = Boolean(p.enableFormulaSemanticDiff);
  if (p.enableDaxSemanticDiff !== undefined) out.enableDaxSemanticDiff = Boolean(p.enableDaxSemanticDiff);
  if (p.semanticNoisePolicy) out.semanticNoisePolicy = p.semanticNoisePolicy;
  return out;
}

function viewOptionsFromProfile(profile, { ignoreBlankToBlank = true } = {}) {
  const p = profile || {};
  const noiseFilters = p.noiseFilters && typeof p.noiseFilters === "object" ? p.noiseFilters : {};
  return { ignoreBlankToBlank, noiseFilters };
}

function persistSelectedProfileId() {
  if (selectedProfileId) {
    safeLocalStorageSet(PROFILE_SELECTED_KEY, selectedProfileId);
  }
}

function applySelectedProfileToView({ rerender = true } = {}) {
  const profile = currentProfile();
  const ignoreBlank = lastViewOptions ? lastViewOptions.ignoreBlankToBlank !== false : true;
  const nextView = viewOptionsFromProfile(profile, { ignoreBlankToBlank: ignoreBlank });
  if (rerender && lastPayload) {
    const snapshot = reviewController?.captureState ? reviewController.captureState() : {};
    renderResults(lastPayload, nextView, snapshot);
  }
}

function initProfilesUi() {
  userProfiles = loadUserProfilesFromStorage()
    .filter(p => p && !p.builtIn && typeof p.id === "string" && typeof p.name === "string")
    .map(p => ({ ...p, builtIn: false }));
  ensureSelectedProfile();
  rebuildProfileSelectOptions();
  persistSelectedProfileId();

  if (profileSelect) {
    profileSelect.addEventListener("change", () => {
      selectedProfileId = profileSelect.value;
      ensureSelectedProfile();
      persistSelectedProfileId();
      rebuildProfileSelectOptions();
      applySelectedProfileToView({ rerender: true });
    });
  }

  if (profileSaveBtn) {
    profileSaveBtn.addEventListener("click", () => {
      const base = currentProfile();
      const name = window.prompt("Name the new profile:", base?.name ? `${base.name} copy` : "New profile");
      const trimmed = String(name || "").trim();
      if (!trimmed) return;
      const noiseFilters = lastViewOptions?.noiseFilters || base.noiseFilters || {};
      const next = {
        ...base,
        id: makeProfileId(trimmed),
        name: trimmed,
        builtIn: false,
        noiseFilters: { ...noiseFilters }
      };
      userProfiles = [...userProfiles, next];
      saveUserProfilesToStorage(userProfiles);
      selectedProfileId = next.id;
      persistSelectedProfileId();
      rebuildProfileSelectOptions();
      applySelectedProfileToView({ rerender: true });
      setStatus(`Profile saved: ${trimmed}`, "");
    });
  }

  if (profileRenameBtn) {
    profileRenameBtn.addEventListener("click", () => {
      const profile = getProfileById(selectedProfileId);
      if (!profile || profile.builtIn) {
        setStatus("Built-in profiles cannot be renamed.", "error");
        return;
      }
      const name = window.prompt("Rename profile:", profile.name || "");
      const trimmed = String(name || "").trim();
      if (!trimmed) return;
      userProfiles = userProfiles.map(p => (p.id === profile.id ? { ...p, name: trimmed } : p));
      saveUserProfilesToStorage(userProfiles);
      rebuildProfileSelectOptions();
      setStatus(`Profile renamed: ${trimmed}`, "");
    });
  }

  if (profileDeleteBtn) {
    profileDeleteBtn.addEventListener("click", () => {
      const profile = getProfileById(selectedProfileId);
      if (!profile || profile.builtIn) {
        setStatus("Built-in profiles cannot be deleted.", "error");
        return;
      }
      const ok = window.confirm(`Delete profile '${profile.name}'?`);
      if (!ok) return;
      userProfiles = userProfiles.filter(p => p.id !== profile.id);
      saveUserProfilesToStorage(userProfiles);
      selectedProfileId = "builtin_default_balanced";
      persistSelectedProfileId();
      rebuildProfileSelectOptions();
      applySelectedProfileToView({ rerender: true });
      setStatus("Profile deleted.", "");
    });
  }
}

let diffClient = null;
let reviewController = null;
let activeViewerManager = null;
let engineVersion = "";
let isBusy = false;
let activeRunId = 0;
let lastReport = null;
let lastMeta = null;
let lastDiffId = null;
let lastSummary = null;
let lastMode = "payload";
let lastEngineOptions = null;
let lastPayload = null;
let lastViewOptions = null;
let lastAuditPath = null;
let isDesktopApp = false;
let selectedOld = null;
let selectedNew = null;
let recentComparisons = [];
let recentsSection = null;
let recentsList = null;
let recentsEmpty = null;
let largeModeNav = null;
let selectedOldFolder = null;
let selectedNewFolder = null;
let batchSection = null;
let batchResults = null;
let batchOldLabel = null;
let batchNewLabel = null;
let batchRunBtn = null;
let searchSection = null;
let searchResults = null;
let searchIndexCache = {
  old: { id: null, path: null },
  new: { id: null, path: null }
};
let largeSummaryCleanup = null;

let profileSelect = null;
let profileSaveBtn = null;
let profileRenameBtn = null;
let profileDeleteBtn = null;

let userProfiles = [];
let selectedProfileId = null;

const PROFILE_STORAGE_KEY = "tabulensis_compare_profiles_v1";
const PROFILE_SELECTED_KEY = "tabulensis_selected_profile_v1";

const FILE_SIDES = {
  old: { dropId: "dropOld", inputId: "fileOld", nameId: "nameOld" },
  new: { dropId: "dropNew", inputId: "fileNew", nameId: "nameNew" }
};

function setBusy(state) {
  isBusy = state;
  byId("run").disabled = state;
  const cancelBtn = byId("cancel");
  if (cancelBtn) cancelBtn.disabled = !state;
}

function setExportsEnabled({ json = false, html = false, audit = false } = {}) {
  const jsonBtn = byId("exportJson");
  const htmlBtn = byId("exportHtml");
  const auditBtn = document.getElementById("exportAudit");
  const openBtn = document.getElementById("openAudit");
  const revealBtn = document.getElementById("revealAudit");
  if (jsonBtn) jsonBtn.disabled = !json;
  if (htmlBtn) htmlBtn.disabled = !html;
  if (auditBtn) auditBtn.disabled = !audit;
  if (openBtn) openBtn.disabled = !lastAuditPath || !isDesktopApp;
  if (revealBtn) revealBtn.disabled = !lastAuditPath || !isDesktopApp;
}

function clearResults() {
  byId("results").innerHTML = "";
  byId("results").classList.remove("visible");
  byId("raw").textContent = "";
  byId("rawJsonContent").classList.remove("visible");
  lastReport = null;
  lastMeta = null;
  lastDiffId = null;
  lastSummary = null;
  lastMode = "payload";
  lastAuditPath = null;
  lastPayload = null;
  lastViewOptions = null;
  if (largeModeNav) {
    largeModeNav.innerHTML = "";
    largeModeNav.classList.remove("visible");
  }
  if (largeSummaryCleanup) {
    largeSummaryCleanup();
    largeSummaryCleanup = null;
  }
}

function updateDropDisplay(side, file) {
  const config = FILE_SIDES[side];
  const drop = byId(config.dropId);
  const nameEl = byId(config.nameId);
  const label = fileDisplayName(file);
  if (label) {
    nameEl.textContent = label;
    drop.classList.add("has-file");
  } else {
    nameEl.textContent = "";
    drop.classList.remove("has-file");
  }
}

function setSelectedFile(side, file) {
  if (side === "old") {
    selectedOld = file;
  } else {
    selectedNew = file;
  }
  updateDropDisplay(side, file);
}

function toDesktopSelection(file) {
  if (!file) return null;
  const path = file.path || "";
  if (!path) return null;
  return buildDesktopSelection(path, file.name);
}

function setupFileDrop(side) {
  const config = FILE_SIDES[side];
  const drop = byId(config.dropId);
  const input = byId(config.inputId);

  function updateDisplay(file) {
    setSelectedFile(side, file);
  }

  if (isDesktopApp) {
    input.disabled = true;
    input.tabIndex = -1;
    input.style.display = "none";
    drop.addEventListener("click", async () => {
      try {
        const path = await openFileDialog();
        if (!path) return;
        updateDisplay(buildDesktopSelection(path));
      } catch (err) {
        setStatus(`Error: ${err.message || err}`, "error");
      }
    });
  } else {
    input.addEventListener("change", () => {
      updateDisplay(input.files[0]);
    });
  }

  drop.addEventListener("dragover", (e) => {
    e.preventDefault();
    drop.classList.add("dragover");
  });

  drop.addEventListener("dragleave", () => {
    drop.classList.remove("dragover");
  });

  drop.addEventListener("drop", (e) => {
    e.preventDefault();
    drop.classList.remove("dragover");
    const files = e.dataTransfer?.files || [];
    if (!files.length) return;

    if (isDesktopApp) {
      const selections = Array.from(files)
        .map(toDesktopSelection)
        .filter(Boolean);
      if (selections.length >= 2) {
        setSelectedFile("old", selections[0]);
        setSelectedFile("new", selections[1]);
        return;
      }
      if (selections[0]) {
        updateDisplay(selections[0]);
      }
      return;
    }

    input.files = files;
    updateDisplay(files[0]);
  });
}

const STAGE_LABELS = {
  init: "Initializing engine...",
  validate: "Validating inputs...",
  read: "Reading files...",
  transfer: "Transferring files to worker...",
  diff: "Diffing workbooks...",
  snapshot: "Building previews...",
  align: "Aligning sheets...",
  parse: "Parsing results...",
  render: "Rendering results...",
  hydrate: "Hydrating viewers..."
};

function showStage(stage, detail) {
  const text = detail || STAGE_LABELS[stage] || "";
  if (text) {
    setStatus(text, "loading");
  }
}

function handleWorkerStatus(status) {
  if (!isBusy) return;
  if (status && status.stage) {
    showStage(status.stage, status.detail);
  }
}

function setBatchFolder(side, path) {
  if (side === "old") {
    selectedOldFolder = path;
  } else {
    selectedNewFolder = path;
  }
  if (batchOldLabel) {
    batchOldLabel.textContent = selectedOldFolder || "";
  }
  if (batchNewLabel) {
    batchNewLabel.textContent = selectedNewFolder || "";
  }
  if (batchRunBtn) {
    batchRunBtn.disabled = !selectedOldFolder || !selectedNewFolder;
  }
}

function renderBatchResults(outcome) {
  if (!batchResults) return;
  const items = Array.isArray(outcome?.items) ? outcome.items : [];
  const rows = items
    .map(item => {
      const status = item.status || "";
      const diffId = item.diffId || "";
      const opCount = item.opCount != null ? item.opCount : "";
      const warnings = item.warningsCount != null ? item.warningsCount : "";
      const action = diffId
        ? `<button class="secondary-btn batch-open" data-diff-id="${diffId}">Open</button>`
        : "";
      return `
        <tr>
          <td>${status}</td>
          <td>${item.oldPath || ""}</td>
          <td>${item.newPath || ""}</td>
          <td>${opCount}</td>
          <td>${warnings}</td>
          <td>${action}</td>
        </tr>
      `;
    })
    .join("");

  batchResults.innerHTML = `
    <div class="batch-summary">
      <div><strong>${outcome.itemCount || items.length}</strong> items</div>
      <div><strong>${outcome.completedCount || 0}</strong> completed</div>
      <div>Status: ${outcome.status || ""}</div>
    </div>
    <div class="batch-table-wrap">
      <table class="batch-table">
        <thead>
          <tr>
            <th>Status</th>
            <th>Old path</th>
            <th>New path</th>
            <th>Ops</th>
            <th>Warnings</th>
            <th>Action</th>
          </tr>
        </thead>
        <tbody>
          ${rows || "<tr><td colspan=\"6\">No items.</td></tr>"}
        </tbody>
      </table>
    </div>
  `;
}

function setupBatchSection() {
  batchSection = document.getElementById("batchSection");
  batchResults = document.getElementById("batchResults");
  batchOldLabel = document.getElementById("batchOldLabel");
  batchNewLabel = document.getElementById("batchNewLabel");
  batchRunBtn = document.getElementById("batchRun");
  if (!batchSection) return;
  batchSection.classList.toggle("visible", isDesktopApp);

  const pickOld = document.getElementById("batchPickOld");
  const pickNew = document.getElementById("batchPickNew");

  if (pickOld) {
    pickOld.addEventListener("click", async () => {
      const path = await openFolderDialog();
      if (path) setBatchFolder("old", path);
    });
  }
  if (pickNew) {
    pickNew.addEventListener("click", async () => {
      const path = await openFolderDialog();
      if (path) setBatchFolder("new", path);
    });
  }

  if (batchRunBtn) {
    batchRunBtn.addEventListener("click", async () => {
      if (!selectedOldFolder || !selectedNewFolder) return;
      setStatus("Running batch compare...", "loading");
      try {
        const outcome = await runBatchCompare({
          oldRoot: selectedOldFolder,
          newRoot: selectedNewFolder,
          strategy: "relative",
          trusted: false
        });
        renderBatchResults(outcome);
        setStatus("Batch complete.", "");
      } catch (err) {
        handleError(err);
      }
    });
  }

  if (batchResults) {
    batchResults.addEventListener("click", event => {
      const btn = event.target.closest(".batch-open");
      if (!btn) return;
      const diffId = btn.dataset.diffId;
      if (diffId) {
        openStoredDiff({ diffId });
      }
    });
  }
}

function renderSearchResults(items, title) {
  if (!searchResults) return;
  const rows = items
    .map(item => {
      const sheet = item.sheet ? `<div class="search-meta">${item.sheet}</div>` : "";
      const addr = item.address ? `<div class="search-meta">${item.address}</div>` : "";
      const detail = item.detail ? `<div class="search-detail">${item.detail}</div>` : "";
      return `
        <div class="search-item">
          <div class="search-title">${item.label || item.text || ""}</div>
          ${sheet}
          ${addr}
          ${detail}
        </div>
      `;
    })
    .join("");

  searchResults.innerHTML = `
    <div class="search-summary">${title}</div>
    ${rows || "<div class=\"empty-state\">No results.</div>"}
  `;
}

function getSearchPath(side) {
  if (lastSummary) {
    return side === "old" ? lastSummary.oldPath : lastSummary.newPath;
  }
  if (side === "old" && selectedOld?.path) return selectedOld.path;
  if (side === "new" && selectedNew?.path) return selectedNew.path;
  return null;
}

async function ensureSearchIndex(side) {
  const path = getSearchPath(side);
  if (!path) throw new Error("Select files before building an index.");
  const cached = searchIndexCache[side];
  if (cached?.id && cached.path === path) return cached.id;
  const summary = await buildSearchIndex(path, side);
  searchIndexCache[side] = { id: summary.indexId, path: summary.path || path };
  return summary.indexId;
}

function setupSearchSection() {
  searchSection = document.getElementById("searchSection");
  searchResults = document.getElementById("searchResults");
  if (!searchSection) return;
  searchSection.classList.toggle("visible", isDesktopApp);

  const searchInput = document.getElementById("searchInput");
  const searchScope = document.getElementById("searchScope");
  const searchBtn = document.getElementById("searchRun");
  const indexBtn = document.getElementById("searchIndex");

  if (searchBtn) {
    searchBtn.addEventListener("click", async () => {
      const query = searchInput?.value || "";
      const scope = searchScope?.value || "changes";
      if (!query.trim()) return;

      try {
        if (scope === "changes") {
          if (!lastDiffId) throw new Error("Run a diff before searching.");
          const results = await searchDiffOps(lastDiffId, query, 100);
          renderSearchResults(results, `Matches for \"${query}\" in changes`);
        } else if (scope === "old" || scope === "new") {
          const indexId = await ensureSearchIndex(scope);
          const results = await searchWorkbookIndex(indexId, query, 100);
          const mapped = results.map(item => ({
            label: item.text,
            sheet: item.sheet,
            address: item.address,
            detail: item.kind
          }));
          renderSearchResults(mapped, `Matches for \"${query}\" in ${scope} workbook`);
        }
      } catch (err) {
        handleError(err);
      }
    });
  }

  if (indexBtn) {
    indexBtn.addEventListener("click", async () => {
      const scope = searchScope?.value || "changes";
      if (scope !== "old" && scope !== "new") return;
      try {
        await ensureSearchIndex(scope);
        setStatus(`Index ready for ${scope} workbook.`, "");
      } catch (err) {
        handleError(err);
      }
    });
  }
}

function normalizeDiffOutcome(result) {
  if (result && result.mode) {
    return {
      mode: result.mode || "payload",
      diffId: result.diffId || null,
      payload: result.payload || null,
      summary: result.summary || null,
      config: result.config || null
    };
  }
  return {
    mode: "payload",
    diffId: null,
    payload: result,
    summary: null,
    config: null
  };
}

function buildMeta(oldFile, newFile) {
  return {
    version: engineVersion,
    oldName: fileDisplayName(oldFile) || "",
    newName: fileDisplayName(newFile) || "",
    createdAtIso: new Date().toISOString()
  };
}

function buildMetaFromSummary(summary) {
  const oldPath = summary?.oldPath || "";
  const newPath = summary?.newPath || "";
  const oldName = summary?.oldName || (oldPath ? baseName(oldPath) : "");
  const newName = summary?.newName || (newPath ? baseName(newPath) : "");
  return {
    version: engineVersion,
    oldName,
    newName,
    createdAtIso: summary?.finishedAt || summary?.startedAt || new Date().toISOString()
  };
}

function collectGridPreviews() {
  const previews = {};
  if (!activeViewerManager || typeof activeViewerManager.getMountedViewers !== "function") {
    return previews;
  }
  for (const [sheetName, viewer] of activeViewerManager.getMountedViewers()) {
    if (!viewer || typeof viewer.capturePng !== "function") continue;
    const dataUrl = viewer.capturePng();
    if (dataUrl) previews[sheetName] = dataUrl;
  }
  return previews;
}

function handleError(err) {
  const message = err && err.message ? err.message : String(err);
  setStatus(`Error: ${message}`, "error");
  byId("results").innerHTML = `
    <div class="warnings-section">
      <div class="warnings-title">
        <span>!</span>
        <span>Error</span>
      </div>
      <p style="color: var(--text-secondary); margin-top: 8px;">${String(message)}</p>
    </div>
  `;
  byId("results").classList.add("visible");
}

function formatRecentTimestamp(iso) {
  if (!iso) return "";
  const dt = new Date(iso);
  if (Number.isNaN(dt.getTime())) return iso;
  return dt.toLocaleString();
}

function applyRecentSelection(entry) {
  if (!entry) return;
  setSelectedFile("old", buildDesktopSelection(entry.oldPath, entry.oldName));
  setSelectedFile("new", buildDesktopSelection(entry.newPath, entry.newName));
}

function renderRecents() {
  if (!recentsSection || !recentsList || !recentsEmpty) return;
  if (!isDesktopApp) {
    recentsSection.classList.remove("visible");
    return;
  }
  recentsSection.classList.add("visible");
  recentsList.innerHTML = "";
  if (!recentComparisons.length) {
    recentsEmpty.hidden = false;
    return;
  }
  recentsEmpty.hidden = true;

  recentComparisons.forEach((entry, index) => {
    const item = document.createElement("div");
    item.className = "recent-item";

    const main = document.createElement("div");
    main.className = "recent-main";

    const names = document.createElement("div");
    names.className = "recent-names";

    const oldSpan = document.createElement("span");
    oldSpan.className = "recent-name";
    oldSpan.textContent = entry.oldName || baseName(entry.oldPath);

    const arrow = document.createElement("span");
    arrow.className = "recent-arrow";
    arrow.textContent = "->";

    const newSpan = document.createElement("span");
    newSpan.className = "recent-name";
    newSpan.textContent = entry.newName || baseName(entry.newPath);

    names.appendChild(oldSpan);
    names.appendChild(arrow);
    names.appendChild(newSpan);

    const meta = document.createElement("div");
    meta.className = "recent-meta";
    meta.textContent = formatRecentTimestamp(entry.lastRunIso);

    main.appendChild(names);
    main.appendChild(meta);

    const actions = document.createElement("div");
    actions.className = "recent-actions";

    const loadBtn = document.createElement("button");
    loadBtn.type = "button";
    loadBtn.className = "secondary-btn recent-action";
    loadBtn.dataset.recentAction = "load";
    loadBtn.dataset.recentIndex = String(index);
    loadBtn.textContent = "Load";

    const openBtn = document.createElement("button");
    openBtn.type = "button";
    openBtn.className = "secondary-btn recent-action";
    openBtn.dataset.recentAction = "open";
    openBtn.dataset.recentIndex = String(index);
    openBtn.textContent = "Open";
    if (!entry.diffId) {
      openBtn.disabled = true;
    }

    const rerunBtn = document.createElement("button");
    rerunBtn.type = "button";
    rerunBtn.className = "secondary-btn recent-action";
    rerunBtn.dataset.recentAction = "rerun";
    rerunBtn.dataset.recentIndex = String(index);
    rerunBtn.textContent = "Re-run";

    const swapBtn = document.createElement("button");
    swapBtn.type = "button";
    swapBtn.className = "secondary-btn recent-action";
    swapBtn.dataset.recentAction = "swap";
    swapBtn.dataset.recentIndex = String(index);
    swapBtn.textContent = "Swap";

    actions.appendChild(loadBtn);
    actions.appendChild(openBtn);
    actions.appendChild(rerunBtn);
    actions.appendChild(swapBtn);

    item.appendChild(main);
    item.appendChild(actions);
    recentsList.appendChild(item);
  });
}

function handleRecentsClick(event) {
  const button = event.target.closest(".recent-action");
  if (!button) return;
  const index = Number(button.dataset.recentIndex);
  const action = button.dataset.recentAction;
  const entry = recentComparisons[index];
  if (!entry) return;

  if (action === "swap") {
    const swapped = {
      oldPath: entry.newPath,
      newPath: entry.oldPath,
      oldName: entry.newName,
      newName: entry.oldName,
      lastRunIso: entry.lastRunIso
    };
    applyRecentSelection(swapped);
    return;
  }

  if (action === "open") {
    openStoredDiff(entry);
    return;
  }

  applyRecentSelection(entry);
  if (action === "rerun") {
    runDiff();
  }
}

async function persistRecentComparison(oldFile, newFile, lastRunIso) {
  if (!isDesktopApp || !oldFile?.path || !newFile?.path) return;
  const entry = {
    oldPath: oldFile.path,
    newPath: newFile.path,
    oldName: fileDisplayName(oldFile),
    newName: fileDisplayName(newFile),
    lastRunIso: lastRunIso || new Date().toISOString(),
    diffId: lastDiffId || undefined,
    mode: lastMode || undefined
  };
  try {
    const updated = await saveRecent(entry);
    if (Array.isArray(updated)) {
      recentComparisons = updated;
      renderRecents();
    }
  } catch (err) {
    console.warn("Failed to save recent comparison:", err);
  }
}

async function loadRecentComparisons() {
  if (!isDesktopApp) return;
  try {
    const items = await loadRecents();
    if (Array.isArray(items)) {
      recentComparisons = items;
    }
  } catch (err) {
    console.warn("Failed to load recents:", err);
  }
  renderRecents();
}

async function openStoredDiff(entry) {
  if (!isDesktopApp || !entry?.diffId) return;
  cleanupViewers();
  clearResults();
  setExportsEnabled({ json: false, html: false, audit: false });
  setStatus("Loading stored diff...", "loading");
  try {
    const summary = await loadDiffSummary(entry.diffId);
    lastDiffId = entry.diffId;
    lastSummary = summary;
    lastMode = summary?.mode || "payload";
    lastReport = null;
    lastMeta = buildMetaFromSummary(summary);
    renderLargeSummary(summary);
    setExportsEnabled({ json: false, html: true, audit: isDesktopApp });
    const opCount = summary?.opCount || 0;
    setStatus(`Loaded ${opCount} change${opCount !== 1 ? "s" : ""}.`, "");
  } catch (err) {
    handleError(err);
  }
}

function cancelDiff() {
  if (!isBusy) return;
  activeRunId += 1;
  diffClient.cancel();
  clearResults();
  setExportsEnabled({ json: false, html: false, audit: false });
  setBusy(false);
  setStatus("Canceled.", "");
}

async function runDiff() {
  const oldFile = selectedOld;
  const newFile = selectedNew;

  if (!oldFile || !newFile) {
    setStatus("Please select both files to compare.", "error");
    return;
  }
  if (isDesktopApp && (!oldFile.path || !newFile.path)) {
    setStatus("Please select valid files to compare.", "error");
    return;
  }

  cleanupViewers();
  clearResults();
  setExportsEnabled({ json: false, html: false, audit: false });

  activeRunId += 1;
  const runId = activeRunId;
  setBusy(true);
  showStage("validate");
  showStage("read");

  try {
    const profile = currentProfile();
    const viewOptions = viewOptionsFromProfile(profile, { ignoreBlankToBlank: true });
    const engineOptions = engineOptionsFromProfile(profile);
    lastEngineOptions = { ...engineOptions };
    let payload;

    if (isDesktopApp) {
      payload = await diffClient.diff(
        {
          oldName: fileDisplayName(oldFile),
          newName: fileDisplayName(newFile),
          oldPath: oldFile.path,
          newPath: newFile.path
        },
        engineOptions
      );
    } else {
      const oldBuffer = await oldFile.arrayBuffer();
      const newBuffer = await newFile.arrayBuffer();
      if (runId !== activeRunId) return;

      showStage("transfer");

      payload = await diffClient.diff(
        {
          oldName: oldFile.name,
          newName: newFile.name,
          oldBuffer,
          newBuffer
        },
        engineOptions
      );
    }
    if (runId !== activeRunId) return;

    showStage("render");
    await nextFrame();

    const outcome = normalizeDiffOutcome(payload);
    lastDiffId = outcome.diffId || null;
    lastSummary = outcome.summary || null;
    lastMode = outcome.mode || "payload";
    if (outcome.config) {
      lastEngineOptions = {
        ...(lastEngineOptions || {}),
        ...(outcome.config.preset ? { preset: outcome.config.preset } : {}),
        ...(outcome.config.limits ? { limits: outcome.config.limits } : {})
      };
    }

    if (outcome.mode === "payload" && outcome.payload) {
      const report = outcome.payload.report || outcome.payload;
      lastPayload = outcome.payload;
      lastViewOptions = { ...viewOptions };
      renderResults(outcome.payload, viewOptions);
      byId("raw").textContent = JSON.stringify(report, null, 2);

      const opCount = report.ops ? report.ops.length : 0;
      if (opCount === 0) {
        setStatus("Files are identical.", "");
      } else {
        setStatus(`Found ${opCount} difference${opCount !== 1 ? "s" : ""}.`, "");
      }

      lastReport = report;
      lastMeta = buildMeta(oldFile, newFile);
      setExportsEnabled({ json: true, html: true, audit: isDesktopApp });
    } else if (outcome.summary) {
      renderLargeSummary(outcome.summary);
      byId("raw").textContent = JSON.stringify(outcome.summary, null, 2);
      lastReport = null;
      lastPayload = null;
      lastViewOptions = null;
      lastMeta = buildMetaFromSummary(outcome.summary);

      const opCount = outcome.summary.opCount || 0;
      if (opCount === 0) {
        setStatus("Files are identical.", "");
      } else {
        setStatus(`Found ${opCount} difference${opCount !== 1 ? "s" : ""} (large mode).`, "");
      }

      setExportsEnabled({ json: false, html: true, audit: isDesktopApp });
    } else {
      throw new Error("Unexpected diff response.");
    }

    await persistRecentComparison(oldFile, newFile, lastMeta?.createdAtIso || "");
  } catch (e) {
    if (!isBusy && String(e).toLowerCase().includes("canceled")) {
      return;
    }
    handleError(e);
  } finally {
    if (runId === activeRunId) {
      setBusy(false);
    }
  }
}

function cleanupViewers() {
  if (reviewController) {
    reviewController.cleanup();
    reviewController = null;
  }
  activeViewerManager = null;
}

function renderResults(payload, options = {}, state = {}) {
  cleanupViewers();
  hideLargeModeNav();
  lastPayload = payload;
  lastViewOptions = {
    ...options,
    ...(options.noiseFilters ? { noiseFilters: { ...options.noiseFilters } } : {})
  };
  const workbookVm = buildWorkbookViewModel(payload, options);
  const resultsEl = byId("results");
  resultsEl.innerHTML = renderWorkbookVm(workbookVm);
  resultsEl.classList.add("visible");
  reviewController = setupReviewWorkflow(resultsEl, workbookVm, payload, options, state);
  activeViewerManager = reviewController.viewerManager || null;
  return workbookVm;
}

function renderSummaryCards(counts = {}) {
  const added = counts.added || 0;
  const removed = counts.removed || 0;
  const modified = counts.modified || 0;
  const moved = counts.moved || 0;
  return `
    <div class="summary-cards">
      <div class="summary-card added">
        <div class="count">${added}</div>
        <div class="label">Added</div>
      </div>
      <div class="summary-card removed">
        <div class="count">${removed}</div>
        <div class="label">Removed</div>
      </div>
      <div class="summary-card modified">
        <div class="count">${modified}</div>
        <div class="label">Modified</div>
      </div>
      <div class="summary-card moved">
        <div class="count">${moved}</div>
        <div class="label">Moved</div>
      </div>
    </div>
  `;
}

function renderLargeSummary(summary) {
  cleanupViewers();
  hideLargeModeNav();
  if (largeSummaryCleanup) {
    largeSummaryCleanup();
    largeSummaryCleanup = null;
  }

  const resultsEl = byId("results");
  const warnings = Array.isArray(summary?.warnings) ? summary.warnings : [];
  const warningsHtml = warnings.length
    ? `
      <div class="warnings-section">
        <div class="warnings-title">
          <span>!</span>
          <span>Warnings</span>
        </div>
        <ul class="warnings-list">
          ${warnings.map(w => `<li>${String(w)}</li>`).join("")}
        </ul>
      </div>
    `
    : "";

  const downloadButton = !isDesktopApp
    ? `<button class="secondary-btn" id="downloadJsonl">Download JSONL</button>`
    : "";

  const sheets = Array.isArray(summary?.sheets) ? summary.sheets : [];
  const sheetHtml = sheets
    .map(sheet => {
      const counts = sheet.counts || {};
      return `
        <div class="large-sheet-item" data-sheet="${sheet.sheetName}">
          <div class="large-sheet-main">
            <div class="large-sheet-name">${sheet.sheetName}</div>
            <div class="large-sheet-meta">${sheet.opCount || 0} changes</div>
          </div>
          <div class="large-sheet-counts">
            <span class="pill added">+${counts.added || 0}</span>
            <span class="pill removed">-${counts.removed || 0}</span>
            <span class="pill modified">~${counts.modified || 0}</span>
            <span class="pill moved">&gt;${counts.moved || 0}</span>
          </div>
          <button class="secondary-btn large-sheet-load" data-sheet="${sheet.sheetName}">Load details</button>
        </div>
      `;
    })
    .join("");

  resultsEl.innerHTML = `
    <div class="large-summary">
      <div class="large-summary-header">
        <div>
          <h2>Large Mode Summary</h2>
          <p>Sheet details load on demand to keep huge diffs responsive.</p>
        </div>
        <div class="large-summary-side">
          <div class="large-summary-meta">
            <span>${summary?.opCount || 0} ops</span>
            <span>${summary?.sheets?.length || 0} sheets</span>
          </div>
          ${downloadButton}
        </div>
      </div>
      ${warningsHtml}
      ${renderSummaryCards(summary?.counts || {})}
      <div class="large-sheet-list">
        ${sheetHtml || "<div class=\"empty-state\">No sheet-level changes recorded.</div>"}
      </div>
    </div>
  `;
  resultsEl.classList.add("visible");

  const onClick = event => {
    const download = event.target.closest("#downloadJsonl");
    if (download) {
      downloadLargeModeJsonl();
      return;
    }
    const button = event.target.closest(".large-sheet-load");
    if (!button) return;
    const sheetName = button.dataset.sheet;
    if (sheetName) {
      loadLargeSheet(sheetName);
    }
  };

  resultsEl.addEventListener("click", onClick);
  largeSummaryCleanup = () => resultsEl.removeEventListener("click", onClick);
}

async function downloadLargeModeJsonl() {
  if (isDesktopApp) return;
  if (isBusy) {
    setStatus("Wait for the current diff to finish before downloading JSONL.", "error");
    return;
  }
  if (!diffClient || typeof diffClient.downloadJsonl !== "function") {
    setStatus("JSONL download is unavailable.", "error");
    return;
  }
  const oldFile = selectedOld;
  const newFile = selectedNew;
  if (!oldFile || !newFile) {
    setStatus("Select files before downloading JSONL.", "error");
    return;
  }
  try {
    setStatus("Preparing JSONL download...", "loading");
    const oldBuffer = await oldFile.arrayBuffer();
    const newBuffer = await newFile.arrayBuffer();
    const blob = await diffClient.downloadJsonl(
      {
        oldName: fileDisplayName(oldFile),
        newName: fileDisplayName(newFile),
        oldBuffer,
        newBuffer
      },
      lastEngineOptions || { preset: "balanced" }
    );
    const meta = lastSummary ? buildMetaFromSummary(lastSummary) : buildMeta(oldFile, newFile);
    downloadJsonl({ blob, meta });
    setStatus("JSONL download ready.", "");
  } catch (err) {
    handleError(err);
  }
}

function showLargeModeNav(sheetName) {
  if (!largeModeNav) return;
  largeModeNav.innerHTML = `
    <button class="secondary-btn" id="largeBack">Back to summary</button>
    <span class="large-mode-title">${sheetName}</span>
  `;
  largeModeNav.classList.add("visible");
  const backBtn = largeModeNav.querySelector("#largeBack");
  if (backBtn) {
    backBtn.addEventListener("click", () => {
      if (lastSummary) {
        renderLargeSummary(lastSummary);
        setExportsEnabled({ json: false, html: true, audit: isDesktopApp });
      }
    });
  }
}

function hideLargeModeNav() {
  if (!largeModeNav) return;
  largeModeNav.classList.remove("visible");
  largeModeNav.innerHTML = "";
}

async function loadLargeSheet(sheetName) {
  if (!isDesktopApp || !lastDiffId) return;
  try {
    showStage("render", `Loading ${sheetName}...`);
    const payload = await loadSheetPayload(lastDiffId, sheetName);
    if (!payload) {
      throw new Error("No payload returned for sheet.");
    }
    renderResults(payload, { ignoreBlankToBlank: true });
    lastReport = payload.report || payload;
    lastMeta = buildMetaFromSummary(lastSummary || {});
    showLargeModeNav(sheetName);
    setExportsEnabled({ json: false, html: true, audit: isDesktopApp });
    setStatus(`Loaded ${sheetName}.`, "");
  } catch (err) {
    handleError(err);
  }
}

function buildReviewOrder(workbookVm) {
  const order = [];
  for (const sheet of workbookVm.sheets) {
    const anchors = sheet.changes?.anchors || [];
    for (const anchor of anchors) {
      order.push({ sheetName: sheet.name, anchorId: anchor.id });
    }
  }
  return order;
}

function resolveStringFromReport(report, id) {
  if (id === null || id === undefined) return "";
  if (typeof id !== "number") return String(id);
  if (!report || !Array.isArray(report.strings)) return "<unknown>";
  return report.strings[id] != null ? report.strings[id] : "<unknown>";
}

function domSafeId(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function makeExplainProvider(workbookVm) {
  const report = workbookVm?.report || {};
  const ops = Array.isArray(report.ops) ? report.ops : [];

  const queryIndex = new Map(); // queryLower -> { name, loadSheets:Set, groupPaths:Set, defChangeKind:string|null }
  const modelCounts = { semantic: 0, formattingOnly: 0, unknown: 0, other: 0 };

  for (const op of ops) {
    const kind = op?.kind || "";
    if (kind.startsWith("Query")) {
      const name = resolveStringFromReport(report, op.name);
      const key = String(name).toLowerCase();
      if (!queryIndex.has(key)) {
        queryIndex.set(key, {
          name,
          loadSheets: new Set(),
          groupPaths: new Set(),
          defChangeKind: null
        });
      }
      const entry = queryIndex.get(key);
      if (kind === "QueryMetadataChanged") {
        const field = op.field || "";
        if (field === "LoadToSheet") {
          const value = op.new != null ? resolveStringFromReport(report, op.new) : "";
          if (value) entry.loadSheets.add(String(value));
        }
        if (field === "GroupPath") {
          const value = op.new != null ? resolveStringFromReport(report, op.new) : "";
          if (value) entry.groupPaths.add(String(value));
        }
      }
      if (kind === "QueryDefinitionChanged") {
        entry.defChangeKind = op.change_kind || op.changeKind || entry.defChangeKind;
      }
      continue;
    }

    if (kind === "MeasureDefinitionChanged" || kind === "CalculatedColumnDefinitionChanged") {
      const ck = String(op.change_kind || op.changeKind || "unknown");
      if (ck === "semantic") modelCounts.semantic += 1;
      else if (ck === "formatting_only") modelCounts.formattingOnly += 1;
      else modelCounts.unknown += 1;
      continue;
    }
    if (kind.startsWith("Table") || kind.startsWith("ModelColumn") || kind.startsWith("Relationship") || kind.startsWith("Measure")) {
      modelCounts.other += 1;
    }
  }

  const otherQueryItems = Array.isArray(workbookVm?.other?.queries) ? workbookVm.other.queries : [];
  const otherModelItems = Array.isArray(workbookVm?.other?.model) ? workbookVm.other.model : [];

  return async function explainCell({ sheetVm, cellSummary }) {
    const sheetName = sheetVm?.name || "";
    const addr = cellSummary?.newAddress || cellSummary?.oldAddress || cellSummary?.viewAddress || "";
    const formulaDiff = cellSummary?.formulaDiff || "";
    const lines = [];
    const jumps = [];

    lines.push(`Cell: ${sheetName}!${addr}`);
    if (cellSummary?.old?.value || cellSummary?.fresh?.value) {
      lines.push(`Old value: ${cellSummary?.old?.value || ""}`);
      lines.push(`New value: ${cellSummary?.fresh?.value || ""}`);
    }
    if (cellSummary?.old?.formula || cellSummary?.fresh?.formula) {
      lines.push(`Old formula: ${cellSummary?.old?.formula || ""}`);
      lines.push(`New formula: ${cellSummary?.fresh?.formula || ""}`);
    }
    if (formulaDiff) {
      lines.push(`Formula diff: ${String(formulaDiff).replace(/_/g, " ")}`);
    }

    if (formulaDiff && String(formulaDiff).toLowerCase() !== "unchanged") {
      lines.push("");
      lines.push("Likely cause: formula changed.");
    }

    const sheetKey = String(sheetName).toLowerCase();
    const queryCandidates = [];
    for (const entry of queryIndex.values()) {
      let strong = false;
      for (const sheet of entry.loadSheets) {
        if (String(sheet).toLowerCase().includes(sheetKey) || sheetKey.includes(String(sheet).toLowerCase())) {
          strong = true;
          break;
        }
      }
      if (!strong) {
        for (const path of entry.groupPaths) {
          if (String(path).toLowerCase().includes(sheetKey)) {
            strong = true;
            break;
          }
        }
      }
      if (strong) queryCandidates.push(entry);
    }

    lines.push("");
    if (queryCandidates.length) {
      lines.push("Power Query candidates for this sheet:");
      for (const entry of queryCandidates.slice(0, 5)) {
        const ck = entry.defChangeKind ? String(entry.defChangeKind).replace(/_/g, " ") : "unknown";
        lines.push(`- ${entry.name} (${ck})`);
        const item = otherQueryItems.find(row => row && row.name && String(row.name).toLowerCase() === String(entry.name).toLowerCase());
        if (item) {
          jumps.push({
            label: `Jump: Query ${entry.name}`,
            elementId: `other-power-query-${domSafeId(item.id)}`
          });
        }
      }
      jumps.push({ label: "Jump: Power Query section", elementId: "other-power-query" });
    } else {
      lines.push("Power Query: no strong attribution found for this sheet.");
      if (otherQueryItems.length) {
        jumps.push({ label: "Jump: Power Query section", elementId: "other-power-query" });
      }
    }

    lines.push("");
    if (otherModelItems.length) {
      lines.push("Model changes present:");
      lines.push(`- semantic: ${modelCounts.semantic}`);
      lines.push(`- formatting-only: ${modelCounts.formattingOnly}`);
      lines.push(`- unknown: ${modelCounts.unknown}`);
      if (modelCounts.other) lines.push(`- other model ops: ${modelCounts.other}`);
      jumps.push({ label: "Jump: Model section", elementId: "other-model" });
    } else {
      lines.push("Model: no changes detected.");
    }

    if (!queryCandidates.length && !otherModelItems.length && (!formulaDiff || String(formulaDiff).toLowerCase() === "unchanged")) {
      lines.push("");
      lines.push("No strong attribution available. This may be caused by upstream sheet edits, data refresh, or external connections.");
    }

    return { text: lines.join("\n"), jumps };
  };
}

function setupReviewWorkflow(rootEl, workbookVm, payloadCache, options = {}, state = {}) {
  const anchorMap = new Map(
    workbookVm.sheets.map(sheet => [sheet.name, new Map((sheet.changes?.anchors || []).map(anchor => [anchor.id, anchor]))])
  );
  const displayOptions = {
    contentMode: state.contentMode || "values",
    focusRows: Boolean(state.focusRows),
    focusCols: Boolean(state.focusCols)
  };
  const explainProvider = makeExplainProvider(workbookVm);
  const viewerManager = hydrateGridViewers(
    rootEl,
    workbookVm,
    displayOptions,
    state.expandedSheets || null,
    explainProvider
  );
  const reviewOrder = buildReviewOrder(workbookVm);
  const reviewState = {
    activeSheetName: state.activeSheetName || null,
    activeAnchorId: state.activeAnchorId || null
  };

  const searchInput = rootEl.querySelector(".sheet-search");
  const focusRowsInput = rootEl.querySelector('input[data-filter="focus-rows"]');
  const focusColsInput = rootEl.querySelector('input[data-filter="focus-cols"]');
  const structuralInput = rootEl.querySelector('input[data-filter="only-structural"]');
  const movedInput = rootEl.querySelector('input[data-filter="only-moved"]');
  const limitedInput = rootEl.querySelector('input[data-filter="only-limited"]');
  const sheetChangeInput = rootEl.querySelector('input[data-filter="only-sheet-changes"]');
  const hideMInput = rootEl.querySelector('input[data-filter="hide-m-formatting-only"]');
  const hideDaxInput = rootEl.querySelector('input[data-filter="hide-dax-formatting-only"]');
  const hideFormulaInput = rootEl.querySelector('input[data-filter="hide-formula-formatting-only"]');
  const collapseMovesInput = rootEl.querySelector('input[data-filter="collapse-moves"]');
  const ignoreBlankInput = rootEl.querySelector('input[data-filter="ignore-blank"]');
  const contentModeSelect = rootEl.querySelector('select[data-filter="content-mode"]');

  if (focusRowsInput) focusRowsInput.checked = displayOptions.focusRows;
  if (focusColsInput) focusColsInput.checked = displayOptions.focusCols;
  if (structuralInput) structuralInput.checked = Boolean(state.onlyStructural);
  if (movedInput) movedInput.checked = Boolean(state.onlyMoved);
  if (limitedInput) limitedInput.checked = Boolean(state.onlyLimited);
  if (sheetChangeInput) sheetChangeInput.checked = Boolean(state.onlySheetChanges);
  const noiseFilters = options.noiseFilters && typeof options.noiseFilters === "object" ? options.noiseFilters : {};
  if (hideMInput) hideMInput.checked = Boolean(noiseFilters.hideMFormattingOnly);
  if (hideDaxInput) hideDaxInput.checked = Boolean(noiseFilters.hideDaxFormattingOnly);
  if (hideFormulaInput) hideFormulaInput.checked = Boolean(noiseFilters.hideFormulaFormattingOnly);
  if (collapseMovesInput) collapseMovesInput.checked = Boolean(noiseFilters.collapseMoves);
  if (ignoreBlankInput) ignoreBlankInput.checked = options.ignoreBlankToBlank !== false;
  if (contentModeSelect) contentModeSelect.value = displayOptions.contentMode;

  function setActiveSheet(sheetName) {
    const items = rootEl.querySelectorAll(".sheet-index-item");
    for (const item of items) {
      item.classList.toggle("active", item.dataset.sheet === sheetName);
    }
  }

  function applySheetFilter(value) {
    const term = String(value || "").trim().toLowerCase();
    const requireStructural = structuralInput?.checked;
    const requireMoved = movedInput?.checked;
    const requireLimited = limitedInput?.checked;
    const requireSheetChange = sheetChangeInput?.checked;
    const sections = rootEl.querySelectorAll(".sheet-section");
    const indexItems = rootEl.querySelectorAll(".sheet-index-item");
    for (const section of sections) {
      const name = (section.dataset.sheet || "").toLowerCase();
      const structuralOk = !requireStructural || section.dataset.structural === "1";
      const movedOk = !requireMoved || section.dataset.moved === "1";
      const limitedOk = !requireLimited || section.dataset.limited === "1";
      const sheetChangeOk = !requireSheetChange || Boolean(section.dataset.sheetState);
      const termOk = term ? name.includes(term) : true;
      section.hidden = !(termOk && structuralOk && movedOk && limitedOk && sheetChangeOk);
    }
    for (const item of indexItems) {
      const name = (item.dataset.sheet || "").toLowerCase();
      const structuralOk = !requireStructural || item.dataset.structural === "1";
      const movedOk = !requireMoved || item.dataset.moved === "1";
      const limitedOk = !requireLimited || item.dataset.limited === "1";
      const sheetChangeOk = !requireSheetChange || Boolean(item.dataset.sheetState);
      const termOk = term ? name.includes(term) : true;
      item.hidden = !(termOk && structuralOk && movedOk && limitedOk && sheetChangeOk);
    }
  }

  if (searchInput) {
    searchInput.value = state.sheetFilter || "";
    if (state.sheetFilter || state.onlyStructural || state.onlyMoved || state.onlyLimited || state.onlySheetChanges) {
      applySheetFilter(state.sheetFilter || "");
    }
  }

  const initialSections = rootEl.querySelectorAll(".sheet-section");
  for (const section of initialSections) {
    if (!section.dataset.activeTab) {
      const activeTab = section.querySelector(".sheet-tab.active")?.dataset.tab || section.dataset.defaultTab;
      if (activeTab) section.dataset.activeTab = activeTab;
    }
  }

  function getSheetSection(sheetName) {
    const sections = rootEl.querySelectorAll(".sheet-section");
    for (const section of sections) {
      if (section.dataset.sheet === sheetName) return section;
    }
    return null;
  }

  function expandSheet(sheetName) {
    const section = getSheetSection(sheetName);
    if (!section) return null;
    section.classList.add("expanded");
    return section;
  }

  function setActiveTab(section, tabId) {
    if (!section || !tabId) return;
    const tabs = section.querySelectorAll(".sheet-tab");
    const contents = section.querySelectorAll(".sheet-tab-content");
    for (const tab of tabs) {
      tab.classList.toggle("active", tab.dataset.tab === tabId);
    }
    for (const content of contents) {
      content.classList.toggle("active", content.dataset.tab === tabId);
    }
    section.dataset.activeTab = tabId;
  }

  function ensureViewer(sheetName) {
    return viewerManager.ensureViewer(sheetName);
  }

  function flashElement(el) {
    if (!el) return;
    el.classList.add("flash");
    window.setTimeout(() => {
      el.classList.remove("flash");
    }, 1200);
  }

  function navigateToAnchor(sheetName, anchorId) {
    const sheetAnchors = anchorMap.get(sheetName);
    const anchor = sheetAnchors ? sheetAnchors.get(anchorId) : null;
    if (!anchor) return false;
    const section = expandSheet(sheetName);
    if (anchor.target.kind === "grid") {
      const viewer = ensureViewer(sheetName);
      if (viewer) {
        viewer.jumpToAnchor(anchorId);
        viewer.flashAnchor(anchorId);
        viewer.focus();
      }
    } else if (anchor.target.kind === "list") {
      const target = document.getElementById(anchor.target.elementId);
      if (target) {
        target.scrollIntoView({ behavior: "smooth", block: "center" });
        flashElement(target);
      }
    }
    if (section && anchor.target.kind === "grid") {
      section.scrollIntoView({ behavior: "smooth", block: "start" });
    }
    reviewState.activeSheetName = sheetName;
    reviewState.activeAnchorId = anchorId;
    setActiveSheet(sheetName);
    return true;
  }

  function findReviewIndex() {
    if (!reviewState.activeSheetName || !reviewState.activeAnchorId) return -1;
    return reviewOrder.findIndex(
      entry => entry.sheetName === reviewState.activeSheetName && entry.anchorId === reviewState.activeAnchorId
    );
  }

  function moveReview(delta) {
    if (!reviewOrder.length) return;
    let idx = findReviewIndex();
    if (idx === -1) {
      idx = delta > 0 ? 0 : reviewOrder.length - 1;
    } else {
      idx += delta;
    }
    if (idx < 0 || idx >= reviewOrder.length) return;
    const next = reviewOrder[idx];
    navigateToAnchor(next.sheetName, next.anchorId);
  }

  function captureState() {
    const expandedSheets = new Set();
    const sections = rootEl.querySelectorAll(".sheet-section.expanded");
    for (const section of sections) {
      if (section.dataset.sheet) expandedSheets.add(section.dataset.sheet);
    }
    const sheetTabs = {};
    const allSections = rootEl.querySelectorAll(".sheet-section");
    for (const section of allSections) {
      const name = section.dataset.sheet;
      if (!name) continue;
      const activeTab = section.dataset.activeTab || section.querySelector(".sheet-tab.active")?.dataset.tab;
      if (activeTab) sheetTabs[name] = activeTab;
    }
    return {
      expandedSheets,
      activeSheetName: reviewState.activeSheetName,
      activeAnchorId: reviewState.activeAnchorId,
      contentMode: displayOptions.contentMode,
      focusRows: displayOptions.focusRows,
      focusCols: displayOptions.focusCols,
      sheetFilter: searchInput ? searchInput.value : "",
      onlyStructural: structuralInput?.checked || false,
      onlyMoved: movedInput?.checked || false,
      onlyLimited: limitedInput?.checked || false,
      onlySheetChanges: sheetChangeInput?.checked || false,
      sheetTabs
    };
  }

  function noiseFiltersFromUi() {
    return {
      hideMFormattingOnly: hideMInput?.checked || false,
      hideDaxFormattingOnly: hideDaxInput?.checked || false,
      hideFormulaFormattingOnly: hideFormulaInput?.checked || false,
      collapseMoves: collapseMovesInput?.checked || false
    };
  }

  function rebuildResults(delta = {}) {
    const nextState = captureState();
    const nextOptions = { ...options, ...delta };
    if (delta.noiseFilters) {
      nextOptions.noiseFilters = { ...delta.noiseFilters };
    }
    renderResults(payloadCache, nextOptions, nextState);
  }

  function onRootClick(event) {
    const tabBtn = event.target.closest(".sheet-tab");
    if (tabBtn) {
      event.preventDefault();
      const section = tabBtn.closest(".sheet-section");
      const tabId = tabBtn.dataset.tab;
      if (section && tabId) {
        setActiveTab(section, tabId);
        const sheetName = section.dataset.sheet;
        if (sheetName && tabId === "grid") {
          ensureViewer(sheetName);
        }
        if (sheetName) {
          reviewState.activeSheetName = sheetName;
          setActiveSheet(sheetName);
        }
      }
      return;
    }

    const previewAction = event.target.closest(".preview-action");
    if (previewAction) {
      event.preventDefault();
      const action = previewAction.dataset.action;
      const sheetName = previewAction.dataset.sheet;
      if (action === "show-hunks" && sheetName) {
        const section = getSheetSection(sheetName);
        if (section) {
          setActiveTab(section, "hunks");
          section.scrollIntoView({ behavior: "smooth", block: "start" });
        }
        return;
      }
      if (action === "export-audit") {
        if (lastDiffId) {
          exportAuditXlsx(lastDiffId).then(path => {
            if (path) {
              lastAuditPath = path;
              setExportsEnabled({ json: Boolean(lastReport), html: true, audit: isDesktopApp });
            }
          }).catch(handleError);
        }
        return;
      }
      if (action === "open-audit") {
        if (lastAuditPath) {
          openPath(lastAuditPath, false).catch(handleError);
        } else {
          setStatus("Export an audit workbook first.", "error");
        }
        return;
      }
    }

    const navBtn = event.target.closest(".review-nav-btn");
    if (navBtn) {
      event.preventDefault();
      const direction = navBtn.dataset.reviewNav;
      moveReview(direction === "prev" ? -1 : 1);
      return;
    }

    const jumpBtn = event.target.closest(".change-jump");
    if (jumpBtn) {
      event.preventDefault();
      event.stopPropagation();
      const sheetName = jumpBtn.dataset.sheet;
      const anchorId = jumpBtn.dataset.anchor;
      if (sheetName && anchorId) {
        navigateToAnchor(sheetName, anchorId);
      }
      return;
    }

    const hunkOpen = event.target.closest(".hunk-open");
    if (hunkOpen) {
      event.preventDefault();
      const sheetName = hunkOpen.dataset.sheet;
      const anchorId = hunkOpen.dataset.anchor;
      if (sheetName && anchorId) {
        navigateToAnchor(sheetName, anchorId);
      }
      return;
    }

    const opsJump = event.target.closest(".ops-jump");
    if (opsJump) {
      event.preventDefault();
      const sheetName = opsJump.dataset.sheet;
      const viewRow = Number(opsJump.dataset.viewRow);
      const viewCol = Number(opsJump.dataset.viewCol);
      if (sheetName && Number.isFinite(viewRow) && Number.isFinite(viewCol)) {
        const section = expandSheet(sheetName);
        if (section) {
          setActiveTab(section, "grid");
          const viewer = ensureViewer(sheetName);
          if (viewer) {
            viewer.jumpTo(viewRow, viewCol);
            viewer.focus();
          }
          section.scrollIntoView({ behavior: "smooth", block: "start" });
        }
      }
      return;
    }

    const statusBtn = event.target.closest(".status-pill");
    if (statusBtn && statusBtn.tagName === "BUTTON") {
      event.preventDefault();
      event.stopPropagation();
      const sheetName = statusBtn.dataset.sheet;
      if (sheetName) {
        const section = expandSheet(sheetName);
        if (section) {
          setActiveTab(section, "grid");
          const warning = section.querySelector(".grid-skip-warning");
          if (warning) {
            warning.scrollIntoView({ behavior: "smooth", block: "center" });
            flashElement(warning);
          } else {
            section.scrollIntoView({ behavior: "smooth", block: "start" });
          }
        }
      }
      return;
    }

    const indexBtn = event.target.closest(".sheet-index-item");
    if (indexBtn) {
      event.preventDefault();
      const sheetName = indexBtn.dataset.sheet;
      if (sheetName) {
        const section = expandSheet(sheetName);
        if (section) {
          const tabId = section.dataset.activeTab || section.dataset.defaultTab || "grid";
          setActiveTab(section, tabId);
          if (tabId === "grid") {
            ensureViewer(sheetName);
          }
          section.scrollIntoView({ behavior: "smooth", block: "start" });
          reviewState.activeSheetName = sheetName;
          reviewState.activeAnchorId = null;
          setActiveSheet(sheetName);
        }
      }
    }
  }

  function onRootInput(event) {
    if (event.target.classList.contains("sheet-search")) {
      applySheetFilter(event.target.value);
      return;
    }
    if (event.target.classList.contains("ops-search")) {
      const section = event.target.closest(".sheet-section");
      const tab = event.target.closest(".sheet-tab-content");
      if (!section) return;
      const term = String(event.target.value || "").trim().toLowerCase();
      const rows = tab ? tab.querySelectorAll(".ops-row") : section.querySelectorAll(".ops-row");
      for (const row of rows) {
        if (row.classList.contains("ops-header")) continue;
        const text = row.dataset.opText || "";
        row.hidden = term ? !text.includes(term) : false;
      }
    }
  }

  function onRootChange(event) {
    if (event.target === focusRowsInput) {
      displayOptions.focusRows = focusRowsInput.checked;
      viewerManager.setDisplayOptions(displayOptions);
    } else if (event.target === focusColsInput) {
      displayOptions.focusCols = focusColsInput.checked;
      viewerManager.setDisplayOptions(displayOptions);
    } else if (event.target === structuralInput || event.target === movedInput || event.target === limitedInput || event.target === sheetChangeInput) {
      applySheetFilter(searchInput ? searchInput.value : "");
    } else if (event.target === hideMInput || event.target === hideDaxInput || event.target === hideFormulaInput || event.target === collapseMovesInput) {
      rebuildResults({ noiseFilters: noiseFiltersFromUi() });
    } else if (event.target === ignoreBlankInput) {
      rebuildResults({ ignoreBlankToBlank: ignoreBlankInput.checked });
    } else if (event.target === contentModeSelect) {
      displayOptions.contentMode = contentModeSelect.value;
      viewerManager.setDisplayOptions(displayOptions);
    }
  }

  function onViewerFocus(event) {
    const mount = event.target.closest(".grid-viewer-mount");
    if (!mount) return;
    const sheetName = mount.dataset.sheet;
    if (!sheetName) return;
    reviewState.activeSheetName = sheetName;
    setActiveSheet(sheetName);
  }

  function onViewerAnchor(event) {
    const mount = event.target.closest(".grid-viewer-mount");
    if (!mount) return;
    const sheetName = mount.dataset.sheet;
    if (!sheetName) return;
    const anchorId = event.detail?.anchorId;
    if (!anchorId) return;
    reviewState.activeSheetName = sheetName;
    reviewState.activeAnchorId = anchorId;
    setActiveSheet(sheetName);
  }

  rootEl.addEventListener("click", onRootClick);
  rootEl.addEventListener("input", onRootInput);
  rootEl.addEventListener("change", onRootChange);
  rootEl.addEventListener("gridviewer:focus", onViewerFocus);
  rootEl.addEventListener("gridviewer:anchor", onViewerAnchor);

  viewerManager.setDisplayOptions(displayOptions);

  if (state.sheetTabs) {
    for (const [sheetName, tabId] of Object.entries(state.sheetTabs)) {
      const section = getSheetSection(sheetName);
      if (section) {
        setActiveTab(section, tabId);
        if (tabId === "grid") {
          ensureViewer(sheetName);
        }
      }
    }
  }

  if (reviewState.activeSheetName && reviewState.activeAnchorId) {
    const moved = navigateToAnchor(reviewState.activeSheetName, reviewState.activeAnchorId);
    if (!moved) {
      expandSheet(reviewState.activeSheetName);
      setActiveSheet(reviewState.activeSheetName);
    }
  }

  return {
    viewerManager,
    captureState,
    cleanup() {
      rootEl.removeEventListener("click", onRootClick);
      rootEl.removeEventListener("input", onRootInput);
      rootEl.removeEventListener("change", onRootChange);
      rootEl.removeEventListener("gridviewer:focus", onViewerFocus);
      rootEl.removeEventListener("gridviewer:anchor", onViewerAnchor);
      viewerManager.cleanup();
    }
  };
}

function hydrateGridViewers(rootEl, workbookVm, displayOptions = {}, expandedSheets = null, explainProvider = null) {
  const sheetMap = new Map(workbookVm.sheets.map(sheet => [sheet.name, sheet]));
  const viewers = new Map();
  let currentOptions = { ...displayOptions };

  function getSectionByName(sheetName) {
    const sections = rootEl.querySelectorAll(".sheet-section");
    for (const section of sections) {
      if (section.dataset.sheet === sheetName) return section;
    }
    return null;
  }

  function mountForSection(section) {
    if (!section) return;
    const gridTab = section.querySelector('.sheet-tab-content[data-tab="grid"]');
    if (gridTab && !gridTab.classList.contains("active")) {
      return;
    }
    const mount = section.querySelector(".grid-viewer-mount");
    if (!mount || mount.dataset.mounted) return;
    const sheetName = section.dataset.sheet || mount.dataset.sheet;
    const sheetVm = sheetMap.get(sheetName);
    if (!sheetVm) return;
    if (typeof sheetVm.ensureCellIndex === "function") {
      sheetVm.ensureCellIndex();
    }
    const initialMode = mount.dataset.initialMode || "side_by_side";
    const initialAnchor = mount.dataset.initialAnchor || "0";
    const viewer = mountSheetGridViewer({
      mountEl: mount,
      sheetVm,
      opts: { initialMode, initialAnchor, displayOptions: currentOptions, onExplain: explainProvider }
    });
    mount.dataset.mounted = "true";
    viewers.set(sheetName, viewer);
  }

  function ensureViewer(sheetName) {
    const existing = viewers.get(sheetName);
    if (existing) return existing;
    const section = getSectionByName(sheetName);
    if (!section) return null;
    section.classList.add("expanded");
    const gridTab = section.querySelector('.sheet-tab-content[data-tab="grid"]');
    if (gridTab && !gridTab.classList.contains("active")) {
      const tabs = section.querySelectorAll(".sheet-tab");
      const contents = section.querySelectorAll(".sheet-tab-content");
      for (const tab of tabs) {
        tab.classList.toggle("active", tab.dataset.tab === "grid");
      }
      for (const content of contents) {
        content.classList.toggle("active", content.dataset.tab === "grid");
      }
      section.dataset.activeTab = "grid";
    }
    mountForSection(section);
    return viewers.get(sheetName) || null;
  }

  function getViewer(sheetName) {
    return viewers.get(sheetName) || null;
  }

  function getMountedViewers() {
    return new Map(viewers);
  }

  function setDisplayOptions(nextOptions) {
    currentOptions = { ...currentOptions, ...nextOptions };
    for (const viewer of viewers.values()) {
      viewer.setDisplayOptions(currentOptions);
    }
  }

  function onHeaderClick(event) {
    const header = event.target.closest(".sheet-header");
    if (!header || !rootEl.contains(header)) return;
    if (event.target.closest("button")) return;
    const section = header.closest(".sheet-section");
    if (!section) return;
    section.classList.toggle("expanded");
    if (section.classList.contains("expanded")) {
      mountForSection(section);
    }
  }

  rootEl.addEventListener("click", onHeaderClick);

  const sections = rootEl.querySelectorAll(".sheet-section");
  if (expandedSheets && expandedSheets.size > 0) {
    for (const section of sections) {
      section.classList.toggle("expanded", expandedSheets.has(section.dataset.sheet));
    }
  } else {
    const anyExpanded = rootEl.querySelector(".sheet-section.expanded");
    if (!anyExpanded && sections.length > 0) {
      sections[0].classList.add("expanded");
    }
  }

  const expanded = rootEl.querySelectorAll(".sheet-section.expanded");
  for (const section of expanded) {
    mountForSection(section);
  }

  return {
    ensureViewer,
    getViewer,
    getMountedViewers,
    setDisplayOptions,
    cleanup() {
      rootEl.removeEventListener("click", onHeaderClick);
      for (const viewer of viewers.values()) {
        viewer.destroy();
      }
      viewers.clear();
    }
  };
}

async function main() {
  setStatus("Loading...", "loading");

  try {
    isDesktopApp = isDesktop();
    diffClient = createAppDiffClient({ onStatus: handleWorkerStatus });
    showStage("init");
    engineVersion = await diffClient.ready();
    byId("version").textContent = engineVersion;
    setStatus("");

    setupFileDrop("old");
    setupFileDrop("new");
    profileSelect = byId("profileSelect");
    profileSaveBtn = byId("profileSave");
    profileRenameBtn = byId("profileRename");
    profileDeleteBtn = byId("profileDelete");
    initProfilesUi();
    largeModeNav = byId("largeModeNav");
    setupBatchSection();
    setupSearchSection();

    recentsSection = byId("recentsSection");
    recentsList = byId("recentsList");
    recentsEmpty = byId("recentsEmpty");
    if (recentsList) {
      recentsList.addEventListener("click", handleRecentsClick);
    }
    await loadRecentComparisons();

    byId("run").addEventListener("click", runDiff);
    const cancelBtn = byId("cancel");
    if (cancelBtn) cancelBtn.addEventListener("click", cancelDiff);

    const exportJsonBtn = byId("exportJson");
    if (exportJsonBtn) {
      exportJsonBtn.addEventListener("click", () => {
        if (!lastReport || !lastMeta) return;
        downloadReportJson({ report: lastReport, meta: lastMeta });
      });
    }

    const exportHtmlBtn = byId("exportHtml");
    if (exportHtmlBtn) {
      exportHtmlBtn.addEventListener("click", () => {
        if (!lastMeta) return;
        const resultsHtml = byId("results").innerHTML;
        const cssText = document.querySelector("style")?.textContent || "";
        const reportJsonText = JSON.stringify(lastReport || lastSummary || {}, null, 2);
        const gridPreviews = collectGridPreviews();
        downloadHtmlReport({
          title: "Tabulensis Report",
          meta: lastMeta,
          renderedResultsHtml: resultsHtml,
          cssText,
          reportJsonText,
          gridPreviews
        });
      });
    }

    const exportAuditBtn = document.getElementById("exportAudit");
    if (exportAuditBtn) {
      exportAuditBtn.addEventListener("click", async () => {
        if (!lastDiffId) return;
        try {
          const path = await exportAuditXlsx(lastDiffId);
          if (path) {
            lastAuditPath = path;
            setExportsEnabled({ json: Boolean(lastReport), html: true, audit: isDesktopApp });
          }
        } catch (err) {
          handleError(err);
        }
      });
    }

    const openAuditBtn = document.getElementById("openAudit");
    if (openAuditBtn) {
      openAuditBtn.addEventListener("click", async () => {
        if (!lastAuditPath) return;
        try {
          await openPath(lastAuditPath, false);
        } catch (err) {
          handleError(err);
        }
      });
    }

    const revealAuditBtn = document.getElementById("revealAudit");
    if (revealAuditBtn) {
      revealAuditBtn.addEventListener("click", async () => {
        if (!lastAuditPath) return;
        try {
          await openPath(lastAuditPath, true);
        } catch (err) {
          handleError(err);
        }
      });
    }

    byId("toggleJson").addEventListener("click", () => {
      byId("rawJsonContent").classList.toggle("visible");
    });

    setBusy(false);
    setExportsEnabled({ json: false, html: false, audit: false });
  } catch (e) {
    setStatus("Failed to load: " + String(e), "error");
  }
}

main();
