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
mustInclude("Model");
mustInclude("Column: Sales.Amount");
mustInclude("Relationship: Sales[CustomerId] -&gt; Customers[Id]");
mustInclude("Calculated Column: Sales.Calc");
mustInclude("Measure: Measure1");

console.log("ok");
