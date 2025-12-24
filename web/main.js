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
