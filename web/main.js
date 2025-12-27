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
    const workbookVm = buildWorkbookViewModel(payload);

    const resultsEl = byId("results");
    resultsEl.innerHTML = renderWorkbookVm(workbookVm);
    resultsEl.classList.add("visible");
    byId("raw").textContent = JSON.stringify(report, null, 2);

    const opCount = report.ops ? report.ops.length : 0;
    if (opCount === 0) {
      setStatus("✓ Files are identical", "");
    } else {
      setStatus(`Found ${opCount} difference${opCount !== 1 ? "s" : ""}`, "");
    }
    
    const firstSheet = resultsEl.querySelector(".sheet-section");
    if (firstSheet) {
      firstSheet.classList.add("expanded");
    }
    cleanupHandler = hydrateGridViewers(resultsEl, workbookVm);
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

let cleanupHandler = null;

function cleanupViewers() {
  if (cleanupHandler) {
    cleanupHandler();
    cleanupHandler = null;
  }
}

function hydrateGridViewers(rootEl, workbookVm) {
  const sheetMap = new Map(workbookVm.sheets.map(sheet => [sheet.name, sheet]));
  const viewers = new Map();

  function mountForSection(section) {
    if (!section) return;
    const mount = section.querySelector(".grid-viewer-mount");
    if (!mount || mount.dataset.mounted) return;
    const sheetName = section.dataset.sheet || mount.dataset.sheet;
    const sheetVm = sheetMap.get(sheetName);
    if (!sheetVm) return;
    const initialMode = mount.dataset.initialMode || "side_by_side";
    const initialAnchor = mount.dataset.initialAnchor ? Number(mount.dataset.initialAnchor) : 0;
    const viewer = mountSheetGridViewer({
      mountEl: mount,
      sheetVm,
      opts: { initialMode, initialAnchor }
    });
    mount.dataset.mounted = "true";
    viewers.set(mount, viewer);
  }

  function onHeaderClick(event) {
    const header = event.target.closest(".sheet-header");
    if (!header || !rootEl.contains(header)) return;
    const section = header.closest(".sheet-section");
    if (!section) return;
    section.classList.toggle("expanded");
    if (section.classList.contains("expanded")) {
      mountForSection(section);
    }
  }

  rootEl.addEventListener("click", onHeaderClick);

  const expanded = rootEl.querySelectorAll(".sheet-section.expanded");
  for (const section of expanded) {
    mountForSection(section);
  }

  return () => {
    rootEl.removeEventListener("click", onHeaderClick);
    for (const viewer of viewers.values()) {
      viewer.destroy();
    }
    viewers.clear();
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
