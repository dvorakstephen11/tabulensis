import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildWorkbookViewModel } from "./view_model.js";
import { renderReportHtml } from "./render.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const payloadPath =
  process.env.WEB_PAYLOAD_PATH || path.join(__dirname, "testdata", "sample_payload.json");
const outcomePath =
  process.env.WEB_OUTCOME_PATH || path.join(__dirname, "testdata", "sample_outcome.json");

const payload = JSON.parse(fs.readFileSync(payloadPath, "utf8"));
const outcome = JSON.parse(fs.readFileSync(outcomePath, "utf8"));

const payloadVm = buildWorkbookViewModel(payload);
assert.ok(payloadVm.sheets.length > 0, "payload should include at least one sheet");
const payloadHtml = renderReportHtml(payload);
assert.ok(payloadHtml.includes("summary-cards"), "payload render should include summary cards");

assert.equal(outcome.mode, "payload", "outcome fixture should be payload mode");
assert.ok(outcome.payload, "outcome fixture should include payload");
const outcomeVm = buildWorkbookViewModel(outcome.payload);
assert.ok(outcomeVm.sheets.length > 0, "outcome payload should include sheets");
const outcomeHtml = renderReportHtml(outcome.payload);
assert.ok(outcomeHtml.includes("summary-cards"), "outcome render should include summary cards");

console.log("ok");
