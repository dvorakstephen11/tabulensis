function esc(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function resolveString(report, id) {
  if (typeof id !== "number") return String(id);
  if (!report || !Array.isArray(report.strings)) return "<missing strings>";
  return report.strings[id] != null ? report.strings[id] : "<unknown>";
}

function isQueryOp(kind) {
  return (
    kind === "QueryAdded" ||
    kind === "QueryRemoved" ||
    kind === "QueryRenamed" ||
    kind === "QueryDefinitionChanged" ||
    kind === "QueryMetadataChanged"
  );
}

function isMeasureOp(kind) {
  return (
    kind === "MeasureAdded" ||
    kind === "MeasureRemoved" ||
    kind === "MeasureDefinitionChanged"
  );
}

function formatStepType(stepType) {
  if (!stepType) return "unknown";
  if (stepType === "other") return "Other";
  const parts = String(stepType).split("_");
  if (parts.length === 1) return parts[0];
  const head = parts[0].charAt(0).toUpperCase() + parts[0].slice(1);
  const tail = parts
    .slice(1)
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join("");
  return head + "." + tail;
}

function renderStepDiff(report, d) {
  const k = d.kind;
  if (k === "step_added") {
    const name = resolveString(report, d.step.name);
    const typ = formatStepType(d.step.step_type);
    return "+ " + esc(name) + " (" + esc(typ) + ")";
  }
  if (k === "step_removed") {
    const name = resolveString(report, d.step.name);
    const typ = formatStepType(d.step.step_type);
    return "- " + esc(name) + " (" + esc(typ) + ")";
  }
  if (k === "step_reordered") {
    const name = resolveString(report, d.name);
    return "r " + esc(name) + " [" + d.from_index + " -> " + d.to_index + "]";
  }
  if (k === "step_modified") {
    const beforeName = resolveString(report, d.before.name);
    const afterName = resolveString(report, d.after.name);
    const parts = [];
    for (const c of d.changes || []) {
      if (c.kind === "renamed") {
        parts.push(
          "renamed(" +
            esc(resolveString(report, c.from)) +
            " -> " +
            esc(resolveString(report, c.to)) +
            ")"
        );
      } else if (c.kind === "source_refs_changed") {
        const rem = (c.removed || []).length;
        const add = (c.added || []).length;
        parts.push("source_refs(" + rem + " removed, " + add + " added)");
      } else if (c.kind === "params_changed") {
        parts.push("params_changed");
      } else {
        parts.push(String(c.kind || "unknown"));
      }
    }
    const changeTxt = parts.length ? " [" + parts.join(", ") + "]" : "";
    if (beforeName === afterName) {
      return "~ " + esc(beforeName) + changeTxt;
    }
    return "~ " + esc(beforeName) + " -> " + esc(afterName) + changeTxt;
  }
  return "? " + esc(JSON.stringify(d));
}

function renderQueryCard(report, queryName, ops) {
  const header = "Query: " + esc(queryName);
  let body = "";

  for (const op of ops) {
    if (op.kind === "QueryAdded") {
      body += "<div class=\"op\">+ Added</div>";
    } else if (op.kind === "QueryRemoved") {
      body += "<div class=\"op\">- Removed</div>";
    } else if (op.kind === "QueryRenamed") {
      body +=
        "<div class=\"op\">r Renamed: " +
        esc(resolveString(report, op.from)) +
        " -> " +
        esc(resolveString(report, op.to)) +
        "</div>";
    } else if (op.kind === "QueryMetadataChanged") {
      body +=
        "<div class=\"op\">~ Metadata: " +
        esc(op.field) +
        " (" +
        esc(resolveString(report, op.old)) +
        " -> " +
        esc(resolveString(report, op.new)) +
        ")</div>";
    } else if (op.kind === "QueryDefinitionChanged") {
      body += "<div class=\"op\">~ Definition changed (" + esc(op.change_kind) + ")</div>";

      const sd = op.semantic_detail;
      if (sd && Array.isArray(sd.step_diffs) && sd.step_diffs.length) {
        body += "<div class=\"subhead\">Step diffs</div><ul class=\"steps\">";
        for (const d of sd.step_diffs) {
          body += "<li>" + renderStepDiff(report, d) + "</li>";
        }
        body += "</ul>";
      } else if (sd && sd.ast_summary) {
        const a = sd.ast_summary;
        body +=
          "<div class=\"subhead\">AST summary</div>" +
          "<div class=\"ast\">" +
          "mode=" +
          esc(a.mode) +
          " inserted=" +
          a.inserted +
          " deleted=" +
          a.deleted +
          " updated=" +
          a.updated +
          " moved=" +
          a.moved +
          "</div>";
      }
    } else {
      body += "<div class=\"op\">? " + esc(op.kind) + "</div>";
    }
  }

  return (
    "<details class=\"card\">" +
    "<summary>" +
    header +
    "</summary>" +
    "<div class=\"card-body\">" +
    body +
    "</div>" +
    "</details>"
  );
}

function renderMeasureCard(report, name, ops) {
  const header = "Measure: " + esc(name);
  let body = "";
  for (const op of ops) {
    if (op.kind === "MeasureAdded") body += "<div class=\"op\">+ Added</div>";
    else if (op.kind === "MeasureRemoved") body += "<div class=\"op\">- Removed</div>";
    else if (op.kind === "MeasureDefinitionChanged") body += "<div class=\"op\">~ Definition changed</div>";
    else body += "<div class=\"op\">? " + esc(op.kind) + "</div>";
  }
  return (
    "<details class=\"card\">" +
    "<summary>" +
    header +
    "</summary>" +
    "<div class=\"card-body\">" +
    body +
    "</div>" +
    "</details>"
  );
}

export function renderReportHtml(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  const warnings = Array.isArray(report.warnings) ? report.warnings : [];

  const queryMap = new Map();
  const measureMap = new Map();
  let otherCount = 0;

  for (const op of ops) {
    const kind = op.kind;
    if (isQueryOp(kind)) {
      let key = "";
      if (kind === "QueryRenamed") key = resolveString(report, op.to);
      else key = resolveString(report, op.name);
      if (!queryMap.has(key)) queryMap.set(key, []);
      queryMap.get(key).push(op);
    } else if (isMeasureOp(kind)) {
      const key = resolveString(report, op.name);
      if (!measureMap.has(key)) measureMap.set(key, []);
      measureMap.get(key).push(op);
    } else {
      otherCount += 1;
    }
  }

  const queryKeys = Array.from(queryMap.keys()).sort();
  const measureKeys = Array.from(measureMap.keys()).sort();

  let html = "";
  html += "<div class=\"summary\">";
  html += "<div>schema_version=" + esc(report.version) + "</div>";
  html += "<div>complete=" + esc(report.complete) + "</div>";
  html += "<div>ops=" + ops.length + " queries=" + queryKeys.length + " measures=" + measureKeys.length + " other=" + otherCount + "</div>";
  html += "</div>";

  if (warnings.length) {
    html += "<div class=\"warnings\"><div class=\"subhead\">Warnings</div><ul>";
    for (const w of warnings) html += "<li>" + esc(w) + "</li>";
    html += "</ul></div>";
  }

  if (queryKeys.length) {
    html += "<h2>Power Query</h2>";
    for (const k of queryKeys) {
      html += renderQueryCard(report, k, queryMap.get(k));
    }
  }

  if (measureKeys.length) {
    html += "<h2>Measures</h2>";
    for (const k of measureKeys) {
      html += renderMeasureCard(report, k, measureMap.get(k));
    }
  }

  if (!queryKeys.length && !measureKeys.length && otherCount) {
    html += "<div class=\"note\">No query or measure ops found. Use Raw JSON to inspect details.</div>";
  }

  return html;
}
