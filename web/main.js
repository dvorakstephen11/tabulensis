import { renderWorkbookVm } from "./render.js";
import { buildWorkbookViewModel } from "./view_model.js";
import { mountSheetGridViewer } from "./grid_viewer.js";
import { downloadReportJson, downloadHtmlReport } from "./export.js";
import { createAppDiffClient, isDesktop, openFileDialog, loadRecents, saveRecent } from "./platform.js";

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

let diffClient = null;
let reviewController = null;
let activeViewerManager = null;
let engineVersion = "";
let isBusy = false;
let activeRunId = 0;
let lastReport = null;
let lastMeta = null;
let isDesktopApp = false;
let selectedOld = null;
let selectedNew = null;
let recentComparisons = [];
let recentsSection = null;
let recentsList = null;
let recentsEmpty = null;

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

function setExportsEnabled(enabled) {
  const jsonBtn = byId("exportJson");
  const htmlBtn = byId("exportHtml");
  if (jsonBtn) jsonBtn.disabled = !enabled;
  if (htmlBtn) htmlBtn.disabled = !enabled;
}

function clearResults() {
  byId("results").innerHTML = "";
  byId("results").classList.remove("visible");
  byId("raw").textContent = "";
  byId("rawJsonContent").classList.remove("visible");
  lastReport = null;
  lastMeta = null;
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

function buildMeta(oldFile, newFile) {
  return {
    version: engineVersion,
    oldName: fileDisplayName(oldFile) || "",
    newName: fileDisplayName(newFile) || "",
    createdAtIso: new Date().toISOString()
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
    lastRunIso: lastRunIso || new Date().toISOString()
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

function cancelDiff() {
  if (!isBusy) return;
  activeRunId += 1;
  diffClient.cancel();
  clearResults();
  setExportsEnabled(false);
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
  setExportsEnabled(false);

  activeRunId += 1;
  const runId = activeRunId;
  setBusy(true);
  showStage("validate");
  showStage("read");

  try {
    const options = { ignoreBlankToBlank: true };
    let payload;

    if (isDesktopApp) {
      payload = await diffClient.diff(
        {
          oldName: fileDisplayName(oldFile),
          newName: fileDisplayName(newFile),
          oldPath: oldFile.path,
          newPath: newFile.path
        },
        options
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
        options
      );
    }
    if (runId !== activeRunId) return;

    showStage("render");
    await nextFrame();

    const report = payload.report || payload;
    renderResults(payload, options);
    byId("raw").textContent = JSON.stringify(report, null, 2);

    const opCount = report.ops ? report.ops.length : 0;
    if (opCount === 0) {
      setStatus("Files are identical.", "");
    } else {
      setStatus(`Found ${opCount} difference${opCount !== 1 ? "s" : ""}.`, "");
    }

    lastReport = report;
    lastMeta = buildMeta(oldFile, newFile);
    setExportsEnabled(true);
    await persistRecentComparison(oldFile, newFile, lastMeta.createdAtIso);
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
  const workbookVm = buildWorkbookViewModel(payload, options);
  const resultsEl = byId("results");
  resultsEl.innerHTML = renderWorkbookVm(workbookVm);
  resultsEl.classList.add("visible");
  reviewController = setupReviewWorkflow(resultsEl, workbookVm, payload, options, state);
  activeViewerManager = reviewController.viewerManager || null;
  return workbookVm;
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

function setupReviewWorkflow(rootEl, workbookVm, payloadCache, options = {}, state = {}) {
  const anchorMap = new Map(
    workbookVm.sheets.map(sheet => [sheet.name, new Map((sheet.changes?.anchors || []).map(anchor => [anchor.id, anchor]))])
  );
  const displayOptions = {
    contentMode: state.contentMode || "values",
    focusRows: Boolean(state.focusRows),
    focusCols: Boolean(state.focusCols)
  };
  const viewerManager = hydrateGridViewers(rootEl, workbookVm, displayOptions, state.expandedSheets || null);
  const reviewOrder = buildReviewOrder(workbookVm);
  const reviewState = {
    activeSheetName: state.activeSheetName || null,
    activeAnchorId: state.activeAnchorId || null
  };

  const searchInput = rootEl.querySelector(".sheet-search");
  const focusRowsInput = rootEl.querySelector('input[data-filter="focus-rows"]');
  const focusColsInput = rootEl.querySelector('input[data-filter="focus-cols"]');
  const ignoreBlankInput = rootEl.querySelector('input[data-filter="ignore-blank"]');
  const contentModeSelect = rootEl.querySelector('select[data-filter="content-mode"]');

  if (focusRowsInput) focusRowsInput.checked = displayOptions.focusRows;
  if (focusColsInput) focusColsInput.checked = displayOptions.focusCols;
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
    const sections = rootEl.querySelectorAll(".sheet-section");
    const indexItems = rootEl.querySelectorAll(".sheet-index-item");
    for (const section of sections) {
      const name = (section.dataset.sheet || "").toLowerCase();
      section.hidden = term ? !name.includes(term) : false;
    }
    for (const item of indexItems) {
      const name = (item.dataset.sheet || "").toLowerCase();
      item.hidden = term ? !name.includes(term) : false;
    }
  }

  if (searchInput) {
    searchInput.value = state.sheetFilter || "";
    if (state.sheetFilter) applySheetFilter(state.sheetFilter);
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
    return {
      expandedSheets,
      activeSheetName: reviewState.activeSheetName,
      activeAnchorId: reviewState.activeAnchorId,
      contentMode: displayOptions.contentMode,
      focusRows: displayOptions.focusRows,
      focusCols: displayOptions.focusCols,
      sheetFilter: searchInput ? searchInput.value : ""
    };
  }

  function rebuildResults(ignoreBlankToBlank) {
    const nextState = captureState();
    const nextOptions = { ...options, ignoreBlankToBlank };
    renderResults(payloadCache, nextOptions, nextState);
  }

  function onRootClick(event) {
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

    const statusBtn = event.target.closest(".status-pill");
    if (statusBtn && statusBtn.tagName === "BUTTON") {
      event.preventDefault();
      event.stopPropagation();
      const sheetName = statusBtn.dataset.sheet;
      if (sheetName) {
        const section = expandSheet(sheetName);
        if (section) {
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
          ensureViewer(sheetName);
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
    }
  }

  function onRootChange(event) {
    if (event.target === focusRowsInput) {
      displayOptions.focusRows = focusRowsInput.checked;
      viewerManager.setDisplayOptions(displayOptions);
    } else if (event.target === focusColsInput) {
      displayOptions.focusCols = focusColsInput.checked;
      viewerManager.setDisplayOptions(displayOptions);
    } else if (event.target === ignoreBlankInput) {
      rebuildResults(ignoreBlankInput.checked);
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

  if (reviewState.activeSheetName && reviewState.activeAnchorId) {
    const moved = navigateToAnchor(reviewState.activeSheetName, reviewState.activeAnchorId);
    if (!moved) {
      expandSheet(reviewState.activeSheetName);
      setActiveSheet(reviewState.activeSheetName);
    }
  }

  return {
    viewerManager,
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

function hydrateGridViewers(rootEl, workbookVm, displayOptions = {}, expandedSheets = null) {
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
      opts: { initialMode, initialAnchor, displayOptions: currentOptions }
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
        if (!lastReport || !lastMeta) return;
        const resultsHtml = byId("results").innerHTML;
        const cssText = document.querySelector("style")?.textContent || "";
        const reportJsonText = JSON.stringify(lastReport, null, 2);
        const gridPreviews = collectGridPreviews();
        downloadHtmlReport({
          title: "Excel Diff Report",
          meta: lastMeta,
          renderedResultsHtml: resultsHtml,
          cssText,
          reportJsonText,
          gridPreviews
        });
      });
    }

    byId("toggleJson").addEventListener("click", () => {
      byId("rawJsonContent").classList.toggle("visible");
    });

    setBusy(false);
    setExportsEnabled(false);
  } catch (e) {
    setStatus("Failed to load: " + String(e), "error");
  }
}

main();
