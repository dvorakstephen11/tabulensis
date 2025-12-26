import init, { diff_files_with_sheets_json, get_version } from "./wasm/excel_diff_wasm.js";
import { renderReportHtml } from "./render.js";

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
    const sheets = payload.sheets || null;

    byId("results").innerHTML = renderReportHtml(report, sheets);
    byId("results").classList.add("visible");
    byId("raw").textContent = JSON.stringify(report, null, 2);

    const opCount = report.ops ? report.ops.length : 0;
    if (opCount === 0) {
      setStatus("✓ Files are identical", "");
    } else {
      setStatus(`Found ${opCount} difference${opCount !== 1 ? "s" : ""}`, "");
    }
    
    const firstSheet = document.querySelector(".sheet-section");
    if (firstSheet) {
      firstSheet.classList.add("expanded");
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
