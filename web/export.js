function escapeHtml(text) {
  return String(text ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function safeName(value, fallback) {
  const text = String(value || fallback || "file").trim();
  const cleaned = text.replace(/[^a-z0-9._-]+/gi, "_").replace(/^_+|_+$/g, "");
  return cleaned || fallback || "file";
}

function downloadBlob(filename, mime, textOrBytes) {
  const blob = textOrBytes instanceof Blob ? textOrBytes : new Blob([textOrBytes], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function downloadReportJson({ report, meta }) {
  const payload = { meta: meta || {}, report: report || {} };
  const json = JSON.stringify(payload, null, 2);
  const oldName = safeName(meta?.oldName, "old");
  const newName = safeName(meta?.newName, "new");
  const date = (meta?.createdAtIso || new Date().toISOString()).slice(0, 10);
  const filename = `excel-diff-report__${oldName}__${newName}__${date}.json`;
  downloadBlob(filename, "application/json", json);
}

export function downloadHtmlReport({
  title,
  meta,
  renderedResultsHtml,
  cssText,
  reportJsonText,
  gridPreviews
}) {
  const safeTitle = escapeHtml(title || "Excel Diff Report");
  const createdAt = escapeHtml(meta?.createdAtIso || new Date().toISOString());
  const oldName = escapeHtml(meta?.oldName || "Old file");
  const newName = escapeHtml(meta?.newName || "New file");
  const reportPre = escapeHtml(reportJsonText || "");
  const previews = gridPreviews || {};
  const previewHtml = Object.keys(previews).length
    ? `
      <section class="export-previews">
        <h2>Grid previews</h2>
        ${Object.entries(previews)
          .map(
            ([sheet, dataUrl]) => `
          <div class="export-preview">
            <h3>${escapeHtml(sheet)}</h3>
            <img src="${dataUrl}" alt="Grid preview for ${escapeHtml(sheet)}" />
          </div>`
          )
          .join("")}
      </section>
    `
    : "";

  const html = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${safeTitle}</title>
    <style>
${cssText || ""}
      .export-wrap { max-width: 1100px; margin: 32px auto; padding: 0 20px 40px; }
      .export-meta { color: var(--text-secondary); margin-bottom: 24px; }
      .export-previews img { max-width: 100%; border: 1px solid var(--border-primary); border-radius: 8px; background: var(--bg-primary); }
      .export-preview { margin-bottom: 24px; }
      pre { white-space: pre-wrap; word-break: break-word; background: var(--bg-primary); border: 1px solid var(--border-primary); color: var(--text-secondary); padding: 16px; border-radius: 8px; }
    </style>
  </head>
  <body>
    <div class="export-wrap">
      <header>
        <h1>${safeTitle}</h1>
        <div class="export-meta">
          <div><strong>Old:</strong> ${oldName}</div>
          <div><strong>New:</strong> ${newName}</div>
          <div><strong>Generated:</strong> ${createdAt}</div>
        </div>
      </header>
      <main>
        ${renderedResultsHtml || ""}
        ${previewHtml}
        <section class="export-report-json">
          <h2>Report JSON</h2>
          <pre>${reportPre}</pre>
        </section>
      </main>
    </div>
  </body>
</html>`;

  const oldSafe = safeName(meta?.oldName, "old");
  const newSafe = safeName(meta?.newName, "new");
  const date = (meta?.createdAtIso || new Date().toISOString()).slice(0, 10);
  const filename = `excel-diff-report__${oldSafe}__${newSafe}__${date}.html`;
  downloadBlob(filename, "text/html", html);
}
