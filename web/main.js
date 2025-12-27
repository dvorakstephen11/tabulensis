import init, { diff_files_with_sheets_json, get_version } from "./wasm/excel_diff_wasm.js";
import { renderWorkbookVm } from "./render.js";
import { buildWorkbookViewModel } from "./view_model.js";
import { mountSheetGridViewer } from "./grid_viewer.js";

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

async function readFileBytes(file) {
  return new Uint8Array(await file.arrayBuffer());
}

function setupFileDrop(dropId, inputId, nameId) {
  const drop = byId(dropId);
  const input = byId(inputId);
  const nameEl = byId(nameId);

  function updateDisplay(file) {
    if (file) {
      nameEl.textContent = file.name;
      drop.classList.add("has-file");
    } else {
      nameEl.textContent = "";
      drop.classList.remove("has-file");
    }
  }

  input.addEventListener("change", () => {
    updateDisplay(input.files[0]);
  });

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
    if (e.dataTransfer.files.length > 0) {
      input.files = e.dataTransfer.files;
      updateDisplay(e.dataTransfer.files[0]);
    }
  });
}

async function runDiff() {
  const oldFile = byId("fileOld").files[0];
  const newFile = byId("fileNew").files[0];

  if (!oldFile || !newFile) {
    setStatus("Please select both files to compare.", "error");
    return;
  }

  cleanupViewers();
  setStatus("Comparing files...", "loading");
  byId("results").innerHTML = "";
  byId("results").classList.remove("visible");
  byId("raw").textContent = "";
  byId("rawJsonContent").classList.remove("visible");

  try {
    const oldBytes = await readFileBytes(oldFile);
    const newBytes = await readFileBytes(newFile);

    const json = diff_files_with_sheets_json(oldBytes, newBytes, oldFile.name, newFile.name);
    const payload = JSON.parse(json);
    const report = payload.report || payload;
    const options = { ignoreBlankToBlank: true };
    renderResults(payload, options);

    byId("raw").textContent = JSON.stringify(report, null, 2);

    const opCount = report.ops ? report.ops.length : 0;
    if (opCount === 0) {
      setStatus("✓ Files are identical", "");
    } else {
      setStatus(`Found ${opCount} difference${opCount !== 1 ? "s" : ""}`, "");
    }
    
  } catch (e) {
    setStatus("Error: " + (e.message || String(e)), "error");
    byId("results").innerHTML = `
      <div class="warnings-section">
        <div class="warnings-title">
          <span>❌</span>
          <span>Error</span>
        </div>
        <p style="color: var(--text-secondary); margin-top: 8px;">${String(e.message || e)}</p>
      </div>
    `;
    byId("results").classList.add("visible");
  }
}

let reviewController = null;

function cleanupViewers() {
  if (reviewController) {
    reviewController.cleanup();
    reviewController = null;
  }
}

function renderResults(payload, options = {}, state = {}) {
  cleanupViewers();
  const workbookVm = buildWorkbookViewModel(payload, options);
  const resultsEl = byId("results");
  resultsEl.innerHTML = renderWorkbookVm(workbookVm);
  resultsEl.classList.add("visible");
  reviewController = setupReviewWorkflow(resultsEl, workbookVm, payload, options, state);
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
    await init();
    byId("version").textContent = get_version();
    setStatus("");
    
    setupFileDrop("dropOld", "fileOld", "nameOld");
    setupFileDrop("dropNew", "fileNew", "nameNew");
    
    byId("run").addEventListener("click", runDiff);
    
    byId("toggleJson").addEventListener("click", () => {
      byId("rawJsonContent").classList.toggle("visible");
    });
  } catch (e) {
    setStatus("Failed to load: " + String(e), "error");
  }
}

main();
