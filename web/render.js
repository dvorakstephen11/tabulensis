import { buildWorkbookViewModel } from "./view_model.js";

function esc(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function domSafeId(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function resolveString(report, id) {
  if (id === null || id === undefined) return "";
  if (typeof id !== "number") return String(id);
  if (!report || !Array.isArray(report.strings)) return "<unknown>";
  return report.strings[id] != null ? report.strings[id] : "<unknown>";
}

function colToLetter(col) {
  let result = "";
  let c = col;
  while (c >= 0) {
    result = String.fromCharCode((c % 26) + 65) + result;
    c = Math.floor(c / 26) - 1;
  }
  return result;
}

function formatCellAddress(row, col) {
  return colToLetter(col) + (row + 1);
}

function parseCellAddress(addr) {
  if (!addr) return null;
  if (typeof addr === "object" && Number.isInteger(addr.row) && Number.isInteger(addr.col)) {
    return { row: addr.row, col: addr.col };
  }
  const match = /^([A-Z]+)(\d+)$/i.exec(String(addr).trim());
  if (!match) return null;
  const letters = match[1].toUpperCase();
  let col = 0;
  for (let i = 0; i < letters.length; i++) {
    col = col * 26 + (letters.charCodeAt(i) - 64);
  }
  const row = parseInt(match[2], 10) - 1;
  return { row, col: col - 1 };
}

function formatValue(report, val) {
  if (val === null || val === undefined) return "";
  if (val === "Blank") return "";
  if (typeof val === "object") {
    if (val.Number !== undefined) return String(val.Number);
    if (val.Text !== undefined) return resolveString(report, val.Text);
    if (val.Bool !== undefined) return val.Bool ? "TRUE" : "FALSE";
    if (val.Error !== undefined) return resolveString(report, val.Error);
    if (val.Formula !== undefined) return String(val.Formula);
    return JSON.stringify(val);
  }
  return String(val);
}

function truncateText(text, maxLen = 20) {
  const str = String(text ?? "");
  if (str.length > maxLen) return str.substring(0, maxLen - 3) + "...";
  return str;
}

function formatValueShort(report, val) {
  return truncateText(formatValue(report, val));
}

function resolveFormula(report, id) {
  if (id === null || id === undefined) return "";
  const text = resolveString(report, id);
  if (!text) return "";
  return text.startsWith("=") ? text : `=${text}`;
}

function buildCellMap(sheet) {
  const map = new Map();
  if (!sheet || !Array.isArray(sheet.cells)) return map;
  for (const cell of sheet.cells) {
    map.set(`${cell.row},${cell.col}`, cell);
  }
  return map;
}

function buildSheetLookup(sheets) {
  const map = new Map();
  if (!sheets) return map;
  const list = Array.isArray(sheets) ? sheets : sheets.sheets;
  if (!Array.isArray(list)) return map;
  for (const sheet of list) {
    map.set(sheet.name, sheet);
  }
  return map;
}

function buildAlignmentLookup(alignments) {
  const map = new Map();
  if (!Array.isArray(alignments)) return map;
  for (const alignment of alignments) {
    if (alignment && typeof alignment.sheet === "string") {
      map.set(alignment.sheet, alignment);
    }
  }
  return map;
}

function buildIndexMap(entries, key) {
  const map = new Map();
  if (!Array.isArray(entries)) return map;
  for (let i = 0; i < entries.length; i++) {
    const entry = entries[i];
    const value = entry ? entry[key] : null;
    if (value !== null && value !== undefined) {
      map.set(value, i);
    }
  }
  return map;
}

function formatAxisTitle(entry, axis) {
  if (!entry) return "";
  const hasOld = entry.old !== null && entry.old !== undefined;
  const hasNew = entry.new !== null && entry.new !== undefined;
  let oldLabel = "";
  let newLabel = "";
  if (axis === "row") {
    if (hasOld) oldLabel = `Old row ${entry.old + 1}`;
    if (hasNew) newLabel = `New row ${entry.new + 1}`;
  } else {
    if (hasOld) oldLabel = `Old col ${colToLetter(entry.old)}`;
    if (hasNew) newLabel = `New col ${colToLetter(entry.new)}`;
  }
  if (oldLabel && newLabel) return `${oldLabel} | ${newLabel}`;
  return oldLabel || newLabel || "";
}

function buildSheetGridData(report, ops, oldSheet, newSheet, alignment) {
  if (alignment) {
    return buildSheetGridDataAligned(report, ops, oldSheet, newSheet, alignment);
  }
  return buildSheetGridDataLegacy(report, ops, oldSheet, newSheet);
}

function buildSheetGridDataAligned(report, ops, oldSheet, newSheet, alignment) {
  const skipped = Boolean(alignment && alignment.skipped);
  if (skipped) {
    return { hasData: false, skipped: true, alignment };
  }

  const rows = Array.isArray(alignment?.rows) ? alignment.rows : [];
  const cols = Array.isArray(alignment?.cols) ? alignment.cols : [];

  const oldCells = buildCellMap(oldSheet);
  const newCells = buildCellMap(newSheet);

  const newRowToView = buildIndexMap(rows, "new");
  const newColToView = buildIndexMap(cols, "new");
  const cellEdits = new Map();

  for (const op of ops) {
    if (op.kind !== "CellEdited") continue;
    const addr = parseCellAddress(op.addr);
    if (!addr) continue;
    const viewRow = newRowToView.get(addr.row);
    const viewCol = newColToView.get(addr.col);
    if (viewRow === undefined || viewCol === undefined) continue;
    cellEdits.set(`${viewRow},${viewCol}`, {
      fromValue: op.from ? formatValue(report, op.from.value) : "",
      toValue: op.to ? formatValue(report, op.to.value) : "",
      fromFormula: resolveFormula(report, op.from?.formula),
      toFormula: resolveFormula(report, op.to?.formula)
    });
  }

  const hasData = rows.length > 0 && cols.length > 0;
  return {
    hasData,
    skipped: false,
    alignment,
    rows,
    cols,
    cellEdits,
    oldCells,
    newCells
  };
}

function buildSheetGridDataLegacy(report, ops, oldSheet, newSheet) {
  const cellEdits = new Map();
  const addedRows = new Set();
  const removedRows = new Set();
  const addedCols = new Set();
  const removedCols = new Set();

  let minRow = Infinity, maxRow = -1;
  let minCol = Infinity, maxCol = -1;
  let hasSheetOp = false;

  for (const op of ops) {
    if (op.kind === "CellEdited") {
      const addr = parseCellAddress(op.addr);
      if (!addr) continue;
      const r = addr.row;
      const c = addr.col;
      cellEdits.set(`${r},${c}`, {
        fromValue: op.from ? formatValue(report, op.from.value) : "",
        toValue: op.to ? formatValue(report, op.to.value) : "",
        fromFormula: resolveFormula(report, op.from?.formula),
        toFormula: resolveFormula(report, op.to?.formula)
      });
      minRow = Math.min(minRow, r);
      maxRow = Math.max(maxRow, r);
      minCol = Math.min(minCol, c);
      maxCol = Math.max(maxCol, c);
    } else if (op.kind === "RowAdded") {
      addedRows.add(op.row_idx);
      minRow = Math.min(minRow, op.row_idx);
      maxRow = Math.max(maxRow, op.row_idx);
    } else if (op.kind === "RowRemoved") {
      removedRows.add(op.row_idx);
      minRow = Math.min(minRow, op.row_idx);
      maxRow = Math.max(maxRow, op.row_idx);
    } else if (op.kind === "ColumnAdded") {
      addedCols.add(op.col_idx);
      minCol = Math.min(minCol, op.col_idx);
      maxCol = Math.max(maxCol, op.col_idx);
    } else if (op.kind === "ColumnRemoved") {
      removedCols.add(op.col_idx);
      minCol = Math.min(minCol, op.col_idx);
      maxCol = Math.max(maxCol, op.col_idx);
    } else if (op.kind === "RectReplaced") {
      minRow = Math.min(minRow, op.start_row);
      maxRow = Math.max(maxRow, op.start_row + op.row_count - 1);
      minCol = Math.min(minCol, op.start_col);
      maxCol = Math.max(maxCol, op.start_col + op.col_count - 1);
    } else if (op.kind === "BlockMovedRows") {
      minRow = Math.min(minRow, op.src_start_row, op.dst_start_row);
      maxRow = Math.max(maxRow, op.src_start_row + op.row_count - 1, op.dst_start_row + op.row_count - 1);
    } else if (op.kind === "BlockMovedColumns") {
      minCol = Math.min(minCol, op.src_start_col, op.dst_start_col);
      maxCol = Math.max(maxCol, op.src_start_col + op.col_count - 1, op.dst_start_col + op.col_count - 1);
    } else if (op.kind === "BlockMovedRect") {
      minRow = Math.min(minRow, op.src_start_row, op.dst_start_row);
      maxRow = Math.max(maxRow, op.src_start_row + op.src_row_count - 1, op.dst_start_row + op.src_row_count - 1);
      minCol = Math.min(minCol, op.src_start_col, op.dst_start_col);
      maxCol = Math.max(maxCol, op.src_start_col + op.src_col_count - 1, op.dst_start_col + op.src_col_count - 1);
    } else if (op.kind === "SheetAdded" || op.kind === "SheetRemoved" || op.kind === "SheetRenamed") {
      hasSheetOp = true;
    }
  }

  const hasChanges = hasSheetOp || cellEdits.size > 0 || addedRows.size > 0 || removedRows.size > 0 || addedCols.size > 0 || removedCols.size > 0;
  if (!hasChanges) {
    return { hasData: false };
  }

  const oldCells = buildCellMap(oldSheet);
  const newCells = buildCellMap(newSheet);

  const sheetRows = Math.max(oldSheet?.nrows || 0, newSheet?.nrows || 0);
  const sheetCols = Math.max(oldSheet?.ncols || 0, newSheet?.ncols || 0);
  if (sheetRows > 0 && sheetCols > 0) {
    return {
      cellEdits,
      addedRows,
      removedRows,
      addedCols,
      removedCols,
      oldCells,
      newCells,
      startRow: 0,
      endRow: sheetRows - 1,
      startCol: 0,
      endCol: sheetCols - 1,
      hasData: true
    };
  }

  if (minRow === Infinity) minRow = 0;
  if (maxRow === -1) maxRow = 0;
  if (minCol === Infinity) minCol = 0;
  if (maxCol === -1) maxCol = 0;

  const contextRows = 1;
  const contextCols = 1;
  const startRow = Math.max(0, minRow - contextRows);
  const endRow = maxRow + contextRows;
  const startCol = Math.max(0, minCol - contextCols);
  const endCol = maxCol + contextCols;

  return {
    cellEdits,
    addedRows,
    removedRows,
    addedCols,
    removedCols,
    oldCells,
    newCells,
    startRow,
    endRow,
    startCol,
    endCol,
    hasData: true
  };
}

function renderSheetGrid(report, gridData) {
  if (gridData.skipped) {
    return `
      <div class="grid-skip-warning">
        Grid preview skipped because the aligned view is too large or inconsistent.
      </div>
    `;
  }
  if (!gridData.hasData) return "";
  if (gridData.alignment) {
    return renderAlignedGrid(report, gridData);
  }
  return renderSheetGridLegacy(report, gridData);
}

function renderAlignedGrid(report, gridData) {
  const { rows, cols, cellEdits, oldCells, newCells } = gridData;
  if (!rows || !cols || rows.length === 0 || cols.length === 0) return "";

  const numCols = cols.length;

  function cellText(cell) {
    if (!cell) return "";
    const value = cell.value ?? "";
    const formula = cell.formula ?? "";
    return value || formula || "";
  }

  function cellTitle(label, value, formula) {
    const parts = [];
    if (value) parts.push(value);
    if (formula && formula != value) parts.push(formula);
    if (!parts.length) return "";
    return label ? `${label}: ${parts.join(" | ")}` : parts.join(" | ");
  }

  let html = `<div class="sheet-grid-container">
    <div class="sheet-grid" style="grid-template-columns: 50px repeat(${numCols}, minmax(100px, 1fr));">`;

  html += `<div class="grid-cell grid-corner"></div>`;
  for (let c = 0; c < cols.length; c++) {
    const colEntry = cols[c];
    const kind = colEntry?.kind;
    let cls = "grid-cell grid-col-header";
    if (kind === "insert") cls += " col-added";
    if (kind === "delete") cls += " col-removed";
    if (kind === "move_src") cls += " col-move-src";
    if (kind === "move_dst") cls += " col-move-dst";
    const title = formatAxisTitle(colEntry, "col");
    const moveAttr = colEntry?.move_id ? ` data-move-id="${esc(colEntry.move_id)}"` : "";
    html += `<div class="${cls}"${moveAttr} title="${esc(title)}">${colToLetter(c)}</div>`;
  }

  for (let r = 0; r < rows.length; r++) {
    const rowEntry = rows[r];
    const rowKind = rowEntry?.kind;
    let rowHeaderCls = "grid-cell grid-row-header";
    if (rowKind === "insert") rowHeaderCls += " row-added";
    if (rowKind === "delete") rowHeaderCls += " row-removed";
    if (rowKind === "move_src") rowHeaderCls += " row-move-src";
    if (rowKind === "move_dst") rowHeaderCls += " row-move-dst";
    const rowTitle = formatAxisTitle(rowEntry, "row");
    const rowMoveAttr = rowEntry?.move_id ? ` data-move-id="${esc(rowEntry.move_id)}"` : "";
    html += `<div class="${rowHeaderCls}"${rowMoveAttr} title="${esc(rowTitle)}">${r + 1}</div>`;

    for (let c = 0; c < cols.length; c++) {
      const colEntry = cols[c];
      const rowKind = rowEntry?.kind;
      const colKind = colEntry?.kind;
      const key = `${r},${c}`;
      const edit = cellEdits.get(key);
      const oldRow = rowEntry?.old;
      const newRow = rowEntry?.new;
      const oldCol = colEntry?.old;
      const newCol = colEntry?.new;

      const oldCell =
        oldRow !== null && oldRow !== undefined && oldCol !== null && oldCol !== undefined
          ? oldCells.get(`${oldRow},${oldCol}`)
          : null;
      const newCell =
        newRow !== null && newRow !== undefined && newCol !== null && newCol !== undefined
          ? newCells.get(`${newRow},${newCol}`)
          : null;

      const isMoveSrc = rowKind === "move_src" || colKind === "move_src";
      const isMoveDst = rowKind === "move_dst" || colKind === "move_dst";
      const isDelete = (rowKind === "delete" || colKind === "delete") && !isMoveSrc;
      const isInsert = (rowKind === "insert" || colKind === "insert") && !isMoveDst;

      let cls = "grid-cell";
      let content = "";
      let title = "";

      if (
        edit &&
        oldRow !== null && oldRow !== undefined &&
        newRow !== null && newRow !== undefined &&
        oldCol !== null && oldCol !== undefined &&
        newCol !== null && newCol !== undefined
      ) {
        cls += " cell-edited";
        const fromText = edit.fromValue || edit.fromFormula || "(empty)";
        const toText = edit.toValue || edit.toFormula || "(empty)";
        content = `<div class="cell-change"><span class="cell-old">${esc(truncateText(fromText))}</span><span class="cell-new">${esc(truncateText(toText))}</span></div>`;
        title = `Changed: ${fromText} -> ${toText}`;
      } else if (isMoveSrc && (cellText(oldCell) || oldCell?.formula)) {
        cls += " cell-move-src";
        const oldValue = oldCell?.value ?? "";
        const oldFormula = oldCell?.formula ?? "";
        const display = cellText(oldCell);
        content = esc(truncateText(display));
        title = cellTitle("Moved (from)", oldValue, oldFormula);
      } else if (isMoveDst && (cellText(newCell) || newCell?.formula)) {
        cls += " cell-move-dst";
        const newValue = newCell?.value ?? "";
        const newFormula = newCell?.formula ?? "";
        const display = cellText(newCell);
        content = esc(truncateText(display));
        title = cellTitle("Moved (to)", newValue, newFormula);
      } else if (isDelete && (cellText(oldCell) || oldCell?.formula)) {
        cls += " cell-removed";
        const oldValue = oldCell?.value ?? "";
        const oldFormula = oldCell?.formula ?? "";
        const display = cellText(oldCell);
        content = esc(truncateText(display));
        title = cellTitle("Removed", oldValue, oldFormula);
      } else if (isInsert && (cellText(newCell) || newCell?.formula)) {
        cls += " cell-added";
        const newValue = newCell?.value ?? "";
        const newFormula = newCell?.formula ?? "";
        const display = cellText(newCell);
        content = esc(truncateText(display));
        title = cellTitle("Added", newValue, newFormula);
      } else if (cellText(newCell) || cellText(oldCell)) {
        cls += " cell-unchanged";
        const newValue = newCell?.value ?? "";
        const newFormula = newCell?.formula ?? "";
        const oldValue = oldCell?.value ?? "";
        const oldFormula = oldCell?.formula ?? "";
        const display = cellText(newCell) || cellText(oldCell);
        const titleValue = newValue || oldValue;
        const titleFormula = newFormula || oldFormula;
        content = esc(truncateText(display));
        title = cellTitle("Value", titleValue, titleFormula);
      } else {
        cls += " cell-empty";
      }

      html += `<div class="${cls}" title="${esc(title)}">${content}</div>`;
    }
  }

  html += `</div></div>`;

  html += `<div class="grid-legend">
    <span class="legend-item"><span class="legend-box legend-edited"></span> Modified</span>
    <span class="legend-item"><span class="legend-box legend-added"></span> Added row/col</span>
    <span class="legend-item"><span class="legend-box legend-removed"></span> Removed row/col</span>
    <span class="legend-item"><span class="legend-box legend-moved"></span> Moved row/col</span>
  </div>`;

  return html;
}

function renderSheetGridLegacy(report, gridData) {
  if (!gridData.hasData) return "";

  const { cellEdits, addedRows, removedRows, addedCols, removedCols, startRow, endRow, startCol, endCol, oldCells, newCells } = gridData;

  const numCols = endCol - startCol + 1;

  function cellText(cell) {
    if (!cell) return "";
    const value = cell.value ?? "";
    const formula = cell.formula ?? "";
    return value || formula || "";
  }

  function cellTitle(label, value, formula) {
    const parts = [];
    if (value) parts.push(value);
    if (formula && formula != value) parts.push(formula);
    if (!parts.length) return "";
    return label ? `${label}: ${parts.join(" | ")}` : parts.join(" | ");
  }

  let html = `<div class="sheet-grid-container">
    <div class="sheet-grid" style="grid-template-columns: 50px repeat(${numCols}, minmax(100px, 1fr));">`;

  html += `<div class="grid-cell grid-corner"></div>`;
  for (let c = startCol; c <= endCol; c++) {
    const isAdded = addedCols.has(c);
    const isRemoved = removedCols.has(c);
    let cls = "grid-cell grid-col-header";
    if (isAdded) cls += " col-added";
    if (isRemoved) cls += " col-removed";
    html += `<div class="${cls}">${colToLetter(c)}${isAdded ? " ?os" : ""}${isRemoved ? " ?o" : ""}</div>`;
  }

  for (let r = startRow; r <= endRow; r++) {
    const rowAdded = addedRows.has(r);
    const rowRemoved = removedRows.has(r);

    let rowHeaderCls = "grid-cell grid-row-header";
    if (rowAdded) rowHeaderCls += " row-added";
    if (rowRemoved) rowHeaderCls += " row-removed";
    html += `<div class="${rowHeaderCls}">${r + 1}${rowAdded ? " ?os" : ""}${rowRemoved ? " ?o" : ""}</div>`;

    for (let c = startCol; c <= endCol; c++) {
      const key = `${r},${c}`;
      const edit = cellEdits.get(key);
      const colAdded = addedCols.has(c);
      const colRemoved = removedCols.has(c);
      const oldCell = oldCells.get(key);
      const newCell = newCells.get(key);

      let cls = "grid-cell";
      let content = "";
      let title = "";

      if (edit) {
        cls += " cell-edited";
        const fromText = edit.fromValue || edit.fromFormula || "(empty)";
        const toText = edit.toValue || edit.toFormula || "(empty)";
        content = `<div class="cell-change"><span class="cell-old">${esc(truncateText(fromText))}</span><span class="cell-new">${esc(truncateText(toText))}</span></div>`;
        title = `Changed: ${fromText} ?+' ${toText}`;
      } else if ((rowRemoved || colRemoved) && (cellText(oldCell) || oldCell?.formula)) {
        cls += " cell-removed";
        const oldValue = oldCell?.value ?? "";
        const oldFormula = oldCell?.formula ?? "";
        const display = cellText(oldCell);
        content = esc(truncateText(display));
        title = cellTitle("Removed", oldValue, oldFormula);
      } else if ((rowAdded || colAdded) && (cellText(newCell) || newCell?.formula)) {
        cls += " cell-added";
        const newValue = newCell?.value ?? "";
        const newFormula = newCell?.formula ?? "";
        const display = cellText(newCell);
        content = esc(truncateText(display));
        title = cellTitle("Added", newValue, newFormula);
      } else if (cellText(newCell) || cellText(oldCell)) {
        cls += " cell-unchanged";
        const newValue = newCell?.value ?? "";
        const newFormula = newCell?.formula ?? "";
        const oldValue = oldCell?.value ?? "";
        const oldFormula = oldCell?.formula ?? "";
        const display = cellText(newCell) || cellText(oldCell);
        const titleValue = newValue || oldValue;
        const titleFormula = newFormula || oldFormula;
        content = esc(truncateText(display));
        title = cellTitle("Value", titleValue, titleFormula);
      } else {
        cls += " cell-empty";
      }

      html += `<div class="${cls}" title="${esc(title)}">${content}</div>`;
    }
  }

  html += `</div></div>`;

  html += `<div class="grid-legend">
    <span class="legend-item"><span class="legend-box legend-edited"></span> Modified</span>
    <span class="legend-item"><span class="legend-box legend-added"></span> Added row/col</span>
    <span class="legend-item"><span class="legend-box legend-removed"></span> Removed row/col</span>
  </div>`;

  return html;
}

function categorizeOps(report) {
  const ops = Array.isArray(report.ops) ? report.ops : [];
  
  const sheetOps = new Map();
  const vbaOps = [];
  const namedRangeOps = [];
  const chartOps = [];
  const queryOps = [];
  const modelOps = [];
  
  let addedCount = 0;
  let removedCount = 0;
  let modifiedCount = 0;
  let movedCount = 0;
  
  for (const op of ops) {
    const kind = op.kind;
    
    if (kind === "SheetAdded" || kind === "SheetRemoved") {
      const sheetName = resolveString(report, op.sheet);
      if (!sheetOps.has(sheetName)) sheetOps.set(sheetName, []);
      sheetOps.get(sheetName).push(op);
      if (kind === "SheetAdded") addedCount++;
      else removedCount++;
    } else if (kind === "SheetRenamed") {
      const sheetName = resolveString(report, op.sheet ?? op.to);
      if (!sheetOps.has(sheetName)) sheetOps.set(sheetName, []);
      sheetOps.get(sheetName).push(op);
      modifiedCount++;
    } else if (kind.startsWith("Row") || kind.startsWith("Column") || kind.startsWith("Cell") || kind.startsWith("Block") || kind.startsWith("Rect")) {
      const sheetName = resolveString(report, op.sheet);
      if (!sheetOps.has(sheetName)) sheetOps.set(sheetName, []);
      sheetOps.get(sheetName).push(op);
      
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else if (kind.includes("Moved")) movedCount++;
      else if (kind.includes("Edited") || kind.includes("Changed") || kind.includes("Replaced")) modifiedCount++;
    } else if (kind.startsWith("Vba")) {
      vbaOps.push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    } else if (kind.startsWith("NamedRange")) {
      namedRangeOps.push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    } else if (kind.startsWith("Chart")) {
      chartOps.push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    } else if (kind.startsWith("Query")) {
      queryOps.push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    } else if (
      kind === "CalculatedColumnDefinitionChanged" ||
      kind.startsWith("Table") ||
      kind.startsWith("ModelColumn") ||
      kind.startsWith("Relationship") ||
      kind.startsWith("Measure")
    ) {
      modelOps.push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    }
  }
  
  return {
    sheetOps,
    vbaOps,
    namedRangeOps,
    chartOps,
    queryOps,
    modelOps,
    counts: { added: addedCount, removed: removedCount, modified: modifiedCount, moved: movedCount }
  };
}

function renderSummaryCards(counts) {
  const total = counts.added + counts.removed + counts.modified + counts.moved;
  if (total === 0) {
    return `
      <div class="no-changes">
        <div class="no-changes-icon">✓</div>
        <h2>No Differences Found</h2>
        <p>The two files are identical.</p>
      </div>
    `;
  }
  
  let html = '<div class="summary-cards">';
  
  if (counts.added > 0) {
    html += `
      <div class="summary-card added">
        <div class="count">${counts.added}</div>
        <div class="label">Added</div>
      </div>
    `;
  }
  
  if (counts.removed > 0) {
    html += `
      <div class="summary-card removed">
        <div class="count">${counts.removed}</div>
        <div class="label">Removed</div>
      </div>
    `;
  }
  
  if (counts.modified > 0) {
    html += `
      <div class="summary-card modified">
        <div class="count">${counts.modified}</div>
        <div class="label">Modified</div>
      </div>
    `;
  }
  
  if (counts.moved > 0) {
    html += `
      <div class="summary-card moved">
        <div class="count">${counts.moved}</div>
        <div class="label">Moved</div>
      </div>
    `;
  }
  
  html += '</div>';
  return html;
}

function renderSheetOp(report, op) {
  const kind = op.kind;
  
  if (kind === "SheetAdded") {
    return `
      <div class="change-item added">
        <div class="change-icon">+</div>
        <span>Sheet added</span>
      </div>
    `;
  }
  
  if (kind === "SheetRenamed") {
    const fromName = resolveString(report, op.from);
    const toName = resolveString(report, op.to ?? op.sheet);
    return `
      <div class="change-item modified">
        <div class="change-icon">~</div>
        <span>Sheet renamed: ${esc(fromName)} &rarr; ${esc(toName)}</span>
      </div>
    `;
  }

  if (kind === "SheetRemoved") {
    return `
      <div class="change-item removed">
        <div class="change-icon">−</div>
        <span>Sheet removed</span>
      </div>
    `;
  }
  
  if (kind === "RowAdded") {
    return `
      <div class="change-item added">
        <div class="change-icon">+</div>
        <span class="change-location">Row ${op.row_idx + 1}</span>
        <span class="change-detail">Row added</span>
      </div>
    `;
  }
  
  if (kind === "RowRemoved") {
    return `
      <div class="change-item removed">
        <div class="change-icon">−</div>
        <span class="change-location">Row ${op.row_idx + 1}</span>
        <span class="change-detail">Row removed</span>
      </div>
    `;
  }
  
  if (kind === "RowReplaced") {
    return `
      <div class="change-item modified">
        <div class="change-icon">~</div>
        <span class="change-location">Row ${op.row_idx + 1}</span>
        <span class="change-detail">Row replaced</span>
      </div>
    `;
  }
  
  if (kind === "ColumnAdded") {
    return `
      <div class="change-item added">
        <div class="change-icon">+</div>
        <span class="change-location">Column ${colToLetter(op.col_idx)}</span>
        <span class="change-detail">Column added</span>
      </div>
    `;
  }
  
  if (kind === "ColumnRemoved") {
    return `
      <div class="change-item removed">
        <div class="change-icon">−</div>
        <span class="change-location">Column ${colToLetter(op.col_idx)}</span>
        <span class="change-detail">Column removed</span>
      </div>
    `;
  }
  
  if (kind === "CellEdited") {
    const addr = typeof op.addr === "string" ? op.addr : formatCellAddress(op.addr.row, op.addr.col);
    const fromVal = op.from ? formatValue(report, op.from.value) : "<empty>";
    const toVal = op.to ? formatValue(report, op.to.value) : "<empty>";
    const fromFormula = op.from?.formula;
    const toFormula = op.to?.formula;
    
    let detail = "";
    if (fromFormula || toFormula) {
      const oldF = fromFormula ? esc(resolveFormula(report, fromFormula)) : "";
      const newF = toFormula ? esc(resolveFormula(report, toFormula)) : "";
      if (oldF && newF && oldF !== newF) {
        detail = `
          <span class="change-value old">${oldF}</span>
          <span class="change-arrow">→</span>
          <span class="change-value">${newF}</span>
        `;
      } else if (oldF && !newF) {
        detail = `<span class="change-value old">${oldF}</span> <span class="change-arrow">→</span> <span class="change-value">${esc(toVal)}</span>`;
      } else if (!oldF && newF) {
        detail = `<span class="change-value old">${esc(fromVal)}</span> <span class="change-arrow">→</span> <span class="change-value">${newF}</span>`;
      } else {
        detail = `<span class="change-value">${newF || esc(toVal)}</span>`;
      }
    } else {
      detail = `
        <span class="change-value old">${esc(fromVal)}</span>
        <span class="change-arrow">→</span>
        <span class="change-value">${esc(toVal)}</span>
      `;
    }
    
    return `
      <div class="change-item modified">
        <div class="change-icon">~</div>
        <span class="change-location">${addr}</span>
        <div class="change-detail">${detail}</div>
      </div>
    `;
  }
  
  if (kind === "BlockMovedRows") {
    const count = op.row_count;
    const from = op.src_start_row + 1;
    const to = op.dst_start_row + 1;
    return `
      <div class="change-item moved">
        <div class="change-icon">↕</div>
        <span class="change-location">Rows ${from}–${from + count - 1}</span>
        <div class="change-detail">
          Moved to row ${to}
        </div>
      </div>
    `;
  }
  
  if (kind === "BlockMovedColumns") {
    const count = op.col_count;
    const from = colToLetter(op.src_start_col);
    const to = colToLetter(op.dst_start_col);
    const fromEnd = colToLetter(op.src_start_col + count - 1);
    return `
      <div class="change-item moved">
        <div class="change-icon">↔</div>
        <span class="change-location">Columns ${from}–${fromEnd}</span>
        <div class="change-detail">
          Moved to column ${to}
        </div>
      </div>
    `;
  }
  
  if (kind === "RectReplaced") {
    const startAddr = formatCellAddress(op.start_row, op.start_col);
    const endAddr = formatCellAddress(op.start_row + op.row_count - 1, op.start_col + op.col_count - 1);
    return `
      <div class="change-item modified">
        <div class="change-icon">~</div>
        <span class="change-location">${startAddr}:${endAddr}</span>
        <div class="change-detail">Region replaced</div>
      </div>
    `;
  }
  
  return `
    <div class="change-item modified">
      <div class="change-icon">?</div>
      <span>${esc(kind)}</span>
    </div>
  `;
}

function renderSheetSection(report, sheetName, ops, sheetLookup, alignmentLookup) {
  const adds = ops.filter(o => o.kind.includes("Added")).length;
  const removes = ops.filter(o => o.kind.includes("Removed")).length;
  const mods = ops.filter(o => o.kind.includes("Edited") || o.kind.includes("Changed") || o.kind.includes("Replaced")).length;
  const moves = ops.filter(o => o.kind.includes("Moved")).length;
  
  let badge = `${ops.length} change${ops.length !== 1 ? "s" : ""}`;
  
  const rowOps = ops.filter(o => o.kind.startsWith("Row"));
  const colOps = ops.filter(o => o.kind.startsWith("Column"));
  const cellOps = ops.filter(o => o.kind === "CellEdited");
  const moveOps = ops.filter(o => o.kind.startsWith("Block"));
  const otherOps = ops.filter(o => !o.kind.startsWith("Row") && !o.kind.startsWith("Column") && o.kind !== "CellEdited" && !o.kind.startsWith("Block") && o.kind !== "SheetAdded" && o.kind !== "SheetRemoved" && o.kind !== "SheetRenamed");
  
  const oldSheet = sheetLookup ? sheetLookup.old.get(sheetName) : null;
  const newSheet = sheetLookup ? sheetLookup.new.get(sheetName) : null;
  const alignment = alignmentLookup ? alignmentLookup.get(sheetName) : null;
  const gridData = buildSheetGridData(report, ops, oldSheet, newSheet, alignment);
  const gridHtml = renderSheetGrid(report, gridData);
  
  let contentHtml = "";
  
  if (gridHtml) {
    contentHtml += `
      <div class="change-group">
        <div class="change-group-title">
          <span>📊</span>
          <span>Visual Diff</span>
        </div>
        ${gridHtml}
      </div>
    `;
  }
  
  let detailsHtml = "";
  
  if (rowOps.length > 0) {
    detailsHtml += `
      <div class="change-group">
        <div class="change-group-title">
          <span>📊</span>
          <span>Row Changes (${rowOps.length})</span>
        </div>
        <div class="change-list">
          ${rowOps.map(op => renderSheetOp(report, op)).join("")}
        </div>
      </div>
    `;
  }
  
  if (colOps.length > 0) {
    detailsHtml += `
      <div class="change-group">
        <div class="change-group-title">
          <span>📏</span>
          <span>Column Changes (${colOps.length})</span>
        </div>
        <div class="change-list">
          ${colOps.map(op => renderSheetOp(report, op)).join("")}
        </div>
      </div>
    `;
  }
  
  if (cellOps.length > 0) {
    detailsHtml += `
      <div class="change-group">
        <div class="change-group-title">
          <span>📝</span>
          <span>Cell Changes (${cellOps.length})</span>
        </div>
        <div class="change-list">
          ${cellOps.map(op => renderSheetOp(report, op)).join("")}
        </div>
      </div>
    `;
  }
  
  if (moveOps.length > 0) {
    detailsHtml += `
      <div class="change-group">
        <div class="change-group-title">
          <span>↕️</span>
          <span>Moved Blocks (${moveOps.length})</span>
        </div>
        <div class="change-list">
          ${moveOps.map(op => renderSheetOp(report, op)).join("")}
        </div>
      </div>
    `;
  }
  
  if (otherOps.length > 0) {
    detailsHtml += `
      <div class="change-group">
        <div class="change-group-title">
          <span>📋</span>
          <span>Other Changes (${otherOps.length})</span>
        </div>
        <div class="change-list">
          ${otherOps.map(op => renderSheetOp(report, op)).join("")}
        </div>
      </div>
    `;
  }
  
  if (detailsHtml) {
    contentHtml += `
      <details class="details-section" open>
        <summary class="details-toggle">Detailed Changes</summary>
        <div class="details-content">
          ${detailsHtml}
        </div>
      </details>
    `;
  }
  
  return `
    <section class="sheet-section" data-sheet="${esc(sheetName)}">
      <div class="sheet-header">
        <div class="sheet-title">
          <div class="sheet-icon">📋</div>
          <span class="sheet-name">${esc(sheetName)}</span>
          <span class="sheet-badge">${badge}</span>
        </div>
        <svg class="expand-icon" width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" />
        </svg>
      </div>
      <div class="sheet-content">
        ${contentHtml}
      </div>
    </section>
  `;
}

function renderOtherOp(report, op) {
  const kind = op.kind;
  const name = op.name !== undefined ? resolveString(report, op.name) : "";
  
  if (kind.includes("Added")) {
    return `
      <div class="change-item added">
        <div class="change-icon">+</div>
        <span>${esc(kind.replace("Added", ""))}: ${esc(name)}</span>
      </div>
    `;
  }
  
  if (kind.includes("Removed")) {
    return `
      <div class="change-item removed">
        <div class="change-icon">−</div>
        <span>${esc(kind.replace("Removed", ""))}: ${esc(name)}</span>
      </div>
    `;
  }
  
  if (kind.includes("Changed") || kind.includes("Renamed")) {
    return `
      <div class="change-item modified">
        <div class="change-icon">~</div>
        <span>${esc(kind.replace("Changed", "").replace("Renamed", ""))}: ${esc(name)}</span>
      </div>
    `;
  }
  
  return `
    <div class="change-item modified">
      <div class="change-icon">?</div>
      <span>${esc(kind)}: ${esc(name)}</span>
    </div>
  `;
}

function renderOtherChangesSection(report, title, icon, ops) {
  if (ops.length === 0) return "";
  
  return `
    <div class="other-changes">
      <div class="other-changes-title">
        <span class="icon">${icon}</span>
        <span>${esc(title)} (${ops.length})</span>
      </div>
      <div class="change-list">
        ${ops.map(op => renderOtherOp(report, op)).join("")}
      </div>
    </div>
  `;
}

function renderWarnings(warnings) {
  if (!warnings || warnings.length === 0) return "";
  
  return `
    <div class="warnings-section">
      <div class="warnings-title">
        <span>⚠️</span>
        <span>Warnings</span>
      </div>
      <ul class="warnings-list">
        ${warnings.map(w => `<li>${esc(w)}</li>`).join("")}
      </ul>
    </div>
  `;
}

function renderPreviewLimitations(vm) {
  const hasLimitations = vm.sheets?.some(sheet => sheet.renderPlan?.status?.kind && sheet.renderPlan.status.kind !== "ok");
  if (!hasLimitations) return "";
  return `
    <div class="preview-limitations">
      <div class="preview-limitations-title">Preview limitations</div>
      <p><strong>Partial</strong> means the preview is limited for performance; edited cells remain exact.</p>
      <p><strong>Skipped</strong> means the aligned view was too large or inconsistent to render. The change list is still valid.</p>
      <p><strong>Missing</strong> means snapshots or alignment data were not available for that sheet.</p>
    </div>
  `;
}

function renderReviewToolbar(vm) {
  if (!vm.sheets || vm.sheets.length === 0) return "";
  return `
    <div class="review-toolbar">
      <div class="review-toolbar-left">
        <div class="toolbar-field">
          <label class="toolbar-label" for="sheetSearch">Sheet search</label>
          <input type="search" id="sheetSearch" class="sheet-search" placeholder="Search sheets" />
        </div>
        <div class="toolbar-actions">
          <button type="button" class="review-nav-btn" data-review-nav="prev">Prev change</button>
          <button type="button" class="review-nav-btn" data-review-nav="next">Next change</button>
        </div>
      </div>
      <div class="review-toolbar-right">
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="focus-rows" />
          Show only changed rows
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="focus-cols" />
          Show only changed columns
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="only-structural" />
          Structural only
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="only-moved" />
          Moved only
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="only-limited" />
          Preview limited only
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="only-sheet-changes" />
          Sheet add/remove/rename
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="hide-m-formatting-only" />
          Hide formatting-only M
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="hide-dax-formatting-only" />
          Hide formatting-only DAX
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="hide-formula-formatting-only" />
          Hide formatting-only formulas
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="collapse-moves" />
          Collapse moved blocks
        </label>
        <label class="toolbar-toggle">
          <input type="checkbox" data-filter="ignore-blank" checked />
          Ignore blank-to-blank
        </label>
        <label class="toolbar-select">
          <span>Display</span>
          <select data-filter="content-mode">
            <option value="values">Values</option>
            <option value="formulas">Formulas</option>
            <option value="both">Both</option>
          </select>
        </label>
      </div>
    </div>
  `;
}

function renderSheetIndex(vm) {
  if (!vm.sheets || vm.sheets.length === 0) return "";
  const maxOps = Math.max(1, ...vm.sheets.map(sheet => sheet.opCount || 0));
  const maxAnchors = Math.max(1, ...vm.sheets.map(sheet => (sheet.changes?.anchors || []).length));
  const maxDensity = Math.max(maxOps, maxAnchors);

  let html = `
    <div class="sheet-index">
      <div class="sheet-index-title">Sheet Index</div>
      <div class="sheet-index-list">
  `;

  for (const sheet of vm.sheets) {
    const anchorCount = sheet.changes?.anchors ? sheet.changes.anchors.length : 0;
    const densityValue = Math.max(anchorCount, sheet.opCount || 0);
    const densityPct = Math.round((densityValue / maxDensity) * 100);
    const statusKind = sheet.renderPlan?.status?.kind || "ok";
    const statusLabel = statusKind === "ok" ? "OK" : statusKind.toUpperCase();
    const statusTitle = sheet.renderPlan?.status?.message ? ` title="${esc(sheet.renderPlan.status.message)}"` : "";
    const counts = sheet.counts || {};
    const sheetState = sheet.sheetState || "";
    const stateLabel =
      sheetState === "added" ? "Added" : sheetState === "removed" ? "Removed" : sheetState === "renamed" ? "Renamed" : "";
    const limitedFlag = statusKind !== "ok" ? "1" : "0";
    const structuralFlag = sheet.flags?.hasStructural ? "1" : "0";
    const movedFlag = counts.moved > 0 ? "1" : "0";

    html += `
      <button type="button" class="sheet-index-item" data-sheet="${esc(sheet.name)}" data-structural="${structuralFlag}" data-moved="${movedFlag}" data-limited="${limitedFlag}" data-sheet-state="${esc(sheetState)}">
        <div class="sheet-index-main">
          <span class="sheet-index-name">${esc(sheet.name)}</span>
          <span class="sheet-index-badges">
            <span class="sheet-index-badge">${sheet.opCount}</span>
            <span class="sheet-index-badge">${anchorCount}</span>
            ${stateLabel ? `<span class="sheet-index-state ${esc(sheetState)}">${esc(stateLabel)}</span>` : ""}
          </span>
        </div>
        <div class="sheet-index-counts">
          <span class="pill added">+${counts.added || 0}</span>
          <span class="pill removed">-${counts.removed || 0}</span>
          <span class="pill modified">~${counts.modified || 0}</span>
          <span class="pill moved">&gt;${counts.moved || 0}</span>
        </div>
        <div class="sheet-index-meta">
          <div class="density-bar"><span style="width: ${densityPct}%"></span></div>
          <span class="status-pill ${statusKind}"${statusTitle}>${statusLabel}</span>
        </div>
      </button>
    `;
  }

  html += `
      </div>
    </div>
  `;
  return html;
}


function renderChangeItemVm(item, sheetName) {
  const changeType = item.changeType || "modified";
  const cls = `change-item ${changeType}`;
  const icon = changeType === "added" ? "+" : changeType === "removed" ? "-" : changeType === "moved" ? ">" : "~";
  const detail = item.detail ? `<span class="change-detail">${esc(item.detail)}</span>` : "";
  const navTargets = Array.isArray(item.navTargets) ? item.navTargets : [];
  const actions = navTargets.length
    ? `<div class="change-actions">${navTargets
        .map(target => `<button type="button" class="change-jump" data-sheet="${esc(sheetName)}" data-anchor="${esc(target.anchorId)}">${esc(target.label || "Jump")}</button>`)
        .join("")}</div>`
    : "";
  const itemId = `change-${sheetName}-${item.id}`;
  return `
    <div class="${cls}" id="${esc(itemId)}">
      <div class="change-icon">${icon}</div>
      <span class="change-location">${esc(item.label || "")}</span>
      ${detail}
      ${actions}
    </div>
  `;
}

function renderChangeGroupVm(title, icon, items, sheetName) {
  if (!items || items.length === 0) return "";
  return `
    <div class="change-group">
      <div class="change-group-title">
        <span>${icon}</span>
        <span>${esc(title)} (${items.length})</span>
      </div>
      <div class="change-list">
        ${items.map(item => renderChangeItemVm(item, sheetName)).join("")}
      </div>
    </div>
  `;
}

function renderGridLegend() {
  return `
    <details class="grid-legend" open>
      <summary>Legend</summary>
      <div class="grid-legend-items">
        <span class="legend-item"><span class="legend-box legend-edited"></span> Modified</span>
        <span class="legend-item"><span class="legend-box legend-added"></span> Added row/col</span>
        <span class="legend-item"><span class="legend-box legend-removed"></span> Removed row/col</span>
        <span class="legend-item"><span class="legend-box legend-moved"></span> Moved row/col</span>
      </div>
    </details>
  `;
}

function renderPreviewBanner(sheetVm) {
  const status = sheetVm.renderPlan?.status;
  if (!status || status.kind === "ok") return "";
  const kind = status.kind;
  const title =
    kind === "partial"
      ? "Preview limited"
      : kind === "skipped"
        ? "Preview unavailable"
        : "Preview missing";
  const message = status.message || "Preview data is unavailable.";
  return `
    <div class="preview-banner ${esc(kind)}" data-sheet="${esc(sheetVm.name)}">
      <div class="preview-banner-title">${esc(title)}</div>
      <div class="preview-banner-text">${esc(message)}</div>
      <div class="preview-banner-actions">
        <button type="button" class="secondary-btn preview-action" data-action="show-hunks" data-sheet="${esc(sheetVm.name)}">Show change hunks</button>
        <button type="button" class="secondary-btn preview-action" data-action="export-audit" data-sheet="${esc(sheetVm.name)}">Export audit workbook</button>
        <button type="button" class="secondary-btn preview-action" data-action="open-audit" data-sheet="${esc(sheetVm.name)}">Open audit workbook</button>
      </div>
    </div>
  `;
}

function hunkBadge(kind) {
  if (!kind) return { label: "Change", cls: "modified" };
  if (kind.includes("added")) return { label: "Added", cls: "added" };
  if (kind.includes("removed")) return { label: "Removed", cls: "removed" };
  if (kind.includes("move")) return { label: "Moved", cls: "moved" };
  if (kind.includes("replaced")) return { label: "Replaced", cls: "modified" };
  if (kind.includes("cluster")) return { label: "Cluster", cls: "modified" };
  return { label: "Modified", cls: "modified" };
}

function rawCellClass(hunk, side) {
  if (!hunk) return "cell-unchanged";
  const kind = hunk.kind || "";
  if (kind.includes("added") && side === "new") return "cell-added";
  if (kind.includes("removed") && side === "old") return "cell-removed";
  if (kind.includes("move") && side === "old") return "cell-move-src";
  if (kind.includes("move") && side === "new") return "cell-move-dst";
  if (kind.includes("replaced") || kind.includes("cell")) return "cell-edited";
  if (kind.includes("cluster")) return "cell-edited";
  return "cell-unchanged";
}

function renderRawHunkGridSide(sheetVm, bounds, side, hunk) {
  if (!bounds) {
    return `<div class="hunk-grid-empty">No ${side} preview</div>`;
  }
  const numCols = bounds.right - bounds.left + 1;
  let html = `<div class="sheet-grid-container">
    <div class="sheet-grid" style="grid-template-columns: 50px repeat(${numCols}, minmax(100px, 1fr));">`;

  html += `<div class="grid-cell grid-corner"></div>`;
  for (let c = bounds.left; c <= bounds.right; c++) {
    html += `<div class="grid-cell grid-col-header">${colToLetter(c)}</div>`;
  }

  for (let r = bounds.top; r <= bounds.bottom; r++) {
    html += `<div class="grid-cell grid-row-header">${r + 1}</div>`;
    for (let c = bounds.left; c <= bounds.right; c++) {
      const cell = sheetVm.cellAtRaw(side, r, c);
      const value = cell?.value ?? cell?.formula ?? "";
      const cls = `grid-cell ${rawCellClass(hunk, side)}`;
      const title = value ? ` title="${esc(value)}"` : "";
      html += `<div class="${cls}"${title}>${esc(truncateText(String(value || "")))}</div>`;
    }
  }

  html += `</div></div>`;
  return html;
}

function renderSummaryBreakdowns(vm) {
  const analysis = vm && vm.analysis ? vm.analysis : null;
  if (!analysis) return "";

  const categories = Array.isArray(analysis.categories) ? analysis.categories : [];
  const severity = analysis.severity || { high: 0, medium: 0, low: 0 };
  const topSheets = Array.isArray(analysis.topSheets) ? analysis.topSheets : [];
  const topArtifacts = Array.isArray(analysis.topArtifacts) ? analysis.topArtifacts : [];

  const categoryRows = categories
    .map(row => {
      const counts = row.counts || {};
      const sev = row.severity || {};
      return `
        <div class="summary-row">
          <div class="summary-cell summary-name">${esc(row.category || "")}</div>
          <div class="summary-cell summary-num">${row.opCount || 0}</div>
          <div class="summary-cell summary-num sev-high">${sev.high || 0}</div>
          <div class="summary-cell summary-num sev-med">${sev.medium || 0}</div>
          <div class="summary-cell summary-num sev-low">${sev.low || 0}</div>
          <div class="summary-cell summary-mini">
            <span class="pill added">+${counts.added || 0}</span>
            <span class="pill removed">-${counts.removed || 0}</span>
            <span class="pill modified">~${counts.modified || 0}</span>
            <span class="pill moved">&gt;${counts.moved || 0}</span>
          </div>
        </div>
      `;
    })
    .join("");

  const runHealth = analysis.incomplete
    ? `<div class="run-health warn">Incomplete run. Some results may be missing.</div>`
    : `<div class="run-health ok">Run complete.</div>`;

  const topSheetRows = topSheets.length
    ? topSheets
        .map(sheet => {
          const counts = sheet.counts || {};
          const sev = String(sheet.severity || "low");
          return `
            <div class="top-row">
              <div class="top-name">${esc(sheet.name || "")}</div>
              <div class="top-meta">
                <span class="sev sev-${esc(sev)}">${esc(sev)}</span>
                <span class="top-count">${sheet.opCount || 0} ops</span>
                <span class="pill added">+${counts.added || 0}</span>
                <span class="pill removed">-${counts.removed || 0}</span>
                <span class="pill modified">~${counts.modified || 0}</span>
                <span class="pill moved">&gt;${counts.moved || 0}</span>
              </div>
            </div>
          `;
        })
        .join("")
    : `<div class="top-empty">No sheet-level changes.</div>`;

  const topArtifactRows = topArtifacts.length
    ? topArtifacts
        .map(item => {
          const sev = String(item.severity || "low");
          const label = item.label || "";
          return `
            <div class="top-row">
              <div class="top-name">${esc(label)}</div>
              <div class="top-meta">
                <span class="sev sev-${esc(sev)}">${esc(sev)}</span>
                <span class="top-kind">${esc(item.kind || "")}</span>
              </div>
            </div>
          `;
        })
        .join("")
    : `<div class="top-empty">No non-grid changes.</div>`;

  return `
    <div class="summary-breakdowns">
      ${runHealth}
      <div class="summary-breakdowns-grid">
        <div class="summary-panel">
          <div class="summary-panel-title">Categories</div>
          <div class="summary-table">
            <div class="summary-row summary-header">
              <div class="summary-cell">Category</div>
              <div class="summary-cell">Ops</div>
              <div class="summary-cell">High</div>
              <div class="summary-cell">Med</div>
              <div class="summary-cell">Low</div>
              <div class="summary-cell">Counts</div>
            </div>
            ${categoryRows}
          </div>
        </div>
        <div class="summary-panel">
          <div class="summary-panel-title">Severity</div>
          <div class="severity-cards">
            <div class="severity-card high"><div class="count">${severity.high || 0}</div><div class="label">High</div></div>
            <div class="severity-card medium"><div class="count">${severity.medium || 0}</div><div class="label">Medium</div></div>
            <div class="severity-card low"><div class="count">${severity.low || 0}</div><div class="label">Low</div></div>
          </div>
          <div class="summary-panel-title">Top Sheets</div>
          <div class="top-list">${topSheetRows}</div>
          <div class="summary-panel-title">Top Artifacts</div>
          <div class="top-list">${topArtifactRows}</div>
        </div>
      </div>
    </div>
  `;
}

function renderRawHunkGrid(sheetVm, hunk) {
  const oldGrid = renderRawHunkGridSide(sheetVm, hunk.renderOld, "old", hunk);
  const newGrid = renderRawHunkGridSide(sheetVm, hunk.renderNew, "new", hunk);
  return `
    <div class="hunk-grid-pair">
      <div class="hunk-grid-side">
        <div class="hunk-grid-label">Old</div>
        ${oldGrid}
      </div>
      <div class="hunk-grid-side">
        <div class="hunk-grid-label">New</div>
        ${newGrid}
      </div>
    </div>
  `;
}

function renderHunksVm(sheetVm) {
  const hunks = sheetVm.hunks || [];
  if (hunks.length === 0) {
    return `<div class="empty-state">No change hunks available.</div>`;
  }
  return `
    <div class="hunk-list">
      ${hunks
        .map(hunk => {
          const badge = hunkBadge(hunk.kind);
          const moveMeta = hunk.moveId ? `<span class="hunk-move-id">${esc(hunk.moveId)}</span>` : "";
          const openBtn = hunk.anchorId
            ? `<button type="button" class="secondary-btn hunk-open" data-sheet="${esc(sheetVm.name)}" data-anchor="${esc(hunk.anchorId)}">Open in grid</button>`
            : "";
          const gridHtml =
            sheetVm.hunkMode === "aligned" && hunk.viewBounds
              ? renderRegionGrid(sheetVm, hunk.viewBounds)
              : renderRawHunkGrid(sheetVm, hunk);
          return `
            <div class="hunk-card">
              <div class="hunk-header">
                <div class="hunk-title">${esc(hunk.label || "Change hunk")}</div>
                <div class="hunk-badges">
                  <span class="pill ${badge.cls}">${esc(badge.label)}</span>
                  ${moveMeta}
                </div>
                <div class="hunk-actions">
                  ${openBtn}
                </div>
              </div>
              ${gridHtml}
            </div>
          `;
        })
        .join("")}
    </div>
  `;
}

function renderOpsTableVm(sheetVm) {
  const ops = sheetVm.ops || [];
  if (!ops.length) {
    return `<div class="empty-state">No operations for this sheet.</div>`;
  }
  const rowsHtml = ops
    .map(op => {
      const navButtons = op.navTargets && op.navTargets.length
        ? op.navTargets
            .map(
              target =>
                `<button type="button" class="ops-jump" data-sheet="${esc(sheetVm.name)}" data-view-row="${target.viewRow}" data-view-col="${target.viewCol}">${esc(target.label || "Jump")}</button>`
            )
            .join("")
        : Number.isFinite(op.viewRow) && Number.isFinite(op.viewCol)
          ? `<button type="button" class="ops-jump" data-sheet="${esc(sheetVm.name)}" data-view-row="${op.viewRow}" data-view-col="${op.viewCol}">Jump</button>`
          : "";
      const filterText = `${op.kind} ${op.location || ""} ${op.detail || ""}`.toLowerCase();
      return `
        <div class="ops-row ${esc(op.changeType || "modified")}" data-op-text="${esc(filterText)}">
          <div class="ops-kind">${esc(op.kind)}</div>
          <div class="ops-location">${esc(op.location || "")}</div>
          <div class="ops-detail">${esc(op.detail || "")}</div>
          <div class="ops-actions">${navButtons}</div>
        </div>
      `;
    })
    .join("");
  return `
    <div class="ops-toolbar">
      <input type="search" class="ops-search" placeholder="Filter operations" data-sheet="${esc(sheetVm.name)}" />
    </div>
    <div class="ops-table">
      <div class="ops-row ops-header">
        <div class="ops-kind">Type</div>
        <div class="ops-location">Location</div>
        <div class="ops-detail">Detail</div>
        <div class="ops-actions">Jump</div>
      </div>
      ${rowsHtml}
    </div>
  `;
}

function renderNonGridOpsVm(sheetVm) {
  const items = sheetVm.nonGridOps || [];
  if (!items.length) {
    return `<div class="empty-state">No non-grid changes for this sheet.</div>`;
  }
  const rowsHtml = items
    .map(item => {
      const filterText = `${item.kind} ${item.location || ""} ${item.detail || ""}`.toLowerCase();
      return `
        <div class="ops-row ${esc(item.changeType || "modified")}" data-op-text="${esc(filterText)}">
          <div class="ops-kind">${esc(item.kind)}</div>
          <div class="ops-location">${esc(item.location || "")}</div>
          <div class="ops-detail">${esc(item.detail || "")}</div>
          <div class="ops-actions"></div>
        </div>
      `;
    })
    .join("");
  return `
    <div class="ops-table">
      <div class="ops-row ops-header">
        <div class="ops-kind">Type</div>
        <div class="ops-location">Item</div>
        <div class="ops-detail">Detail</div>
        <div class="ops-actions"></div>
      </div>
      ${rowsHtml}
    </div>
  `;
}

function renderRegionGrid(sheetVm, region) {
  const rows = sheetVm.axis.rows.entries;
  const cols = sheetVm.axis.cols.entries;
  const bounds = region.renderBounds || region;
  if (!bounds || rows.length === 0 || cols.length === 0) return "";

  const numCols = bounds.right - bounds.left + 1;

  let html = `<div class="sheet-grid-container">
    <div class="sheet-grid" style="grid-template-columns: 50px repeat(${numCols}, minmax(100px, 1fr));">`;

  html += `<div class="grid-cell grid-corner"></div>`;
  for (let c = bounds.left; c <= bounds.right; c++) {
    const colEntry = cols[c];
    const kind = colEntry?.kind;
    let cls = "grid-cell grid-col-header";
    if (kind === "insert") cls += " col-added";
    if (kind === "delete") cls += " col-removed";
    if (kind === "move_src") cls += " col-move-src";
    if (kind === "move_dst") cls += " col-move-dst";
    const title = formatAxisTitle(colEntry, "col");
    const moveAttr = colEntry?.move_id ? ` data-move-id="${esc(colEntry.move_id)}"` : "";
    html += `<div class="${cls}"${moveAttr} title="${esc(title)}">${colToLetter(c)}</div>`;
  }

  for (let r = bounds.top; r <= bounds.bottom; r++) {
    const rowEntry = rows[r];
    const rowKind = rowEntry?.kind;
    let rowHeaderCls = "grid-cell grid-row-header";
    if (rowKind === "insert") rowHeaderCls += " row-added";
    if (rowKind === "delete") rowHeaderCls += " row-removed";
    if (rowKind === "move_src") rowHeaderCls += " row-move-src";
    if (rowKind === "move_dst") rowHeaderCls += " row-move-dst";
    const rowTitle = formatAxisTitle(rowEntry, "row");
    const rowMoveAttr = rowEntry?.move_id ? ` data-move-id="${esc(rowEntry.move_id)}"` : "";
    html += `<div class="${rowHeaderCls}"${rowMoveAttr} title="${esc(rowTitle)}">${r + 1}</div>`;

    for (let c = bounds.left; c <= bounds.right; c++) {
      const cell = sheetVm.cellAt(r, c);
      let cls = "grid-cell";
      let content = "";
      let title = cell.display.tooltip || "";

      if (cell.diffKind === "edited") {
        cls += " cell-edited";
        const fromText = cell.edit?.fromValue || cell.edit?.fromFormula || "(empty)";
        const toText = cell.edit?.toValue || cell.edit?.toFormula || "(empty)";
        content = `<div class="cell-change"><span class="cell-old">${esc(truncateText(fromText))}</span><span class="cell-new">${esc(truncateText(toText))}</span></div>`;
        title = `Changed: ${fromText} -> ${toText}`;
      } else if (cell.diffKind === "added") {
        cls += " cell-added";
        content = esc(truncateText(cell.display.text || ""));
      } else if (cell.diffKind === "removed") {
        cls += " cell-removed";
        content = esc(truncateText(cell.display.text || ""));
      } else if (cell.diffKind === "moved") {
        cls += cell.moveRole === "src" ? " cell-move-src" : " cell-move-dst";
        content = esc(truncateText(cell.display.text || ""));
      } else if (cell.diffKind === "unchanged") {
        cls += " cell-unchanged";
        content = esc(truncateText(cell.display.text || ""));
      } else {
        cls += " cell-empty";
      }

      html += `<div class="${cls}" title="${esc(title)}">${content}</div>`;
    }
  }

  html += `</div></div>`;
  return html;
}

function renderSheetGridVm(sheetVm) {
  const status = sheetVm.renderPlan.status;
  if (status.kind === "missing" && sheetVm.changes.regions.length === 0) {
    return "";
  }
  if (status.kind === "skipped" || status.kind === "missing") {
    return `
      <div class="grid-skip-warning">
        ${esc(status.message || "Grid preview unavailable.")}
      </div>
    `;
  }

  const regionIds = sheetVm.renderPlan.regionsToRender || [];
  const hasGridAnchors = Array.isArray(sheetVm.changes?.anchors)
    ? sheetVm.changes.anchors.some(anchor => anchor.target?.kind === "grid")
    : false;
  if (regionIds.length === 0 && !hasGridAnchors) return "";

  const initialAnchor = Array.isArray(sheetVm.changes?.anchors)
    ? sheetVm.changes.anchors.find(anchor => anchor.target?.kind === "grid")
    : null;
  const initialAnchorId = initialAnchor ? initialAnchor.id : "0";

  return `
    <div class="grid-viewer-mount" data-sheet="${esc(sheetVm.name)}" data-initial-mode="side_by_side" data-initial-anchor="${esc(initialAnchorId)}"></div>
    ${renderGridLegend()}
  `;
}

function renderSheetVm(sheetVm) {
  const badge = `${sheetVm.opCount} change${sheetVm.opCount !== 1 ? "s" : ""}`;
  const anchorCount = sheetVm.changes?.anchors ? sheetVm.changes.anchors.length : 0;
  const anchorBadge = anchorCount > 0
    ? `<span class="sheet-badge anchor-badge">${anchorCount} anchor${anchorCount !== 1 ? "s" : ""}</span>`
    : "";
  const status = sheetVm.renderPlan.status || { kind: "ok" };
  const statusLabel = status.kind === "ok" ? "OK" : status.kind.toUpperCase();
  const statusTitle = status.message ? ` title="${esc(status.message)}"` : "";
  const statusPill = `<button type="button" class="status-pill ${status.kind}" data-sheet="${esc(sheetVm.name)}"${statusTitle}>${statusLabel}</button>`;
  const gridHtml = renderSheetGridVm(sheetVm);
  const hunksHtml = renderHunksVm(sheetVm);
  const nonGridHtml = renderNonGridOpsVm(sheetVm);
  const opsTableHtml = renderOpsTableVm(sheetVm);
  const previewBanner = renderPreviewBanner(sheetVm);

  const rowItems = sheetVm.changes.items.filter(item => item.group === "rows");
  const colItems = sheetVm.changes.items.filter(item => item.group === "cols");
  const cellItems = sheetVm.changes.items.filter(item => item.group === "cells");
  const moveItems = sheetVm.changes.items.filter(item => item.group === "moves");
  const otherItems = sheetVm.changes.items.filter(item => item.group === "other");

  let detailsHtml = "";
  detailsHtml += renderChangeGroupVm("Row Changes", "R", rowItems, sheetVm.name);
  detailsHtml += renderChangeGroupVm("Column Changes", "C", colItems, sheetVm.name);
  detailsHtml += renderChangeGroupVm("Cell Changes", "*", cellItems, sheetVm.name);
  detailsHtml += renderChangeGroupVm("Moved Blocks", ">", moveItems, sheetVm.name);
  detailsHtml += renderChangeGroupVm("Other Changes", "?", otherItems, sheetVm.name);

  const defaultTab =
    status.kind === "skipped" || status.kind === "missing"
      ? (sheetVm.hunks && sheetVm.hunks.length ? "hunks" : "ops")
      : "grid";
  const counts = sheetVm.counts || {};
  const sheetState = sheetVm.sheetState || "";
  const stateLabel =
    sheetState === "added" ? "Added" : sheetState === "removed" ? "Removed" : sheetState === "renamed" ? "Renamed" : "";
  const limitedFlag = status.kind !== "ok" ? "1" : "0";
  const structuralFlag = sheetVm.flags?.hasStructural ? "1" : "0";
  const movedFlag = counts.moved > 0 ? "1" : "0";

  return `
    <section class="sheet-section" data-sheet="${esc(sheetVm.name)}" data-structural="${structuralFlag}" data-moved="${movedFlag}" data-limited="${limitedFlag}" data-sheet-state="${esc(sheetState)}" data-default-tab="${esc(defaultTab)}">
      <div class="sheet-header">
        <div class="sheet-title">
          <div class="sheet-icon">#</div>
          <span class="sheet-name">${esc(sheetVm.name)}</span>
          <span class="sheet-badge">${badge}</span>
          ${anchorBadge}
          ${stateLabel ? `<span class="sheet-badge sheet-state ${esc(sheetState)}">${esc(stateLabel)}</span>` : ""}
          ${statusPill}
        </div>
        <div class="sheet-meta">
          <span class="pill added">+${counts.added || 0}</span>
          <span class="pill removed">-${counts.removed || 0}</span>
          <span class="pill modified">~${counts.modified || 0}</span>
          <span class="pill moved">&gt;${counts.moved || 0}</span>
        </div>
        <svg class="expand-icon" width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" />
        </svg>
      </div>
      <div class="sheet-content">
        <div class="sheet-tabs">
          <button type="button" class="sheet-tab ${defaultTab === "grid" ? "active" : ""}" data-tab="grid">Grid</button>
          <button type="button" class="sheet-tab ${defaultTab === "hunks" ? "active" : ""}" data-tab="hunks">Change hunks</button>
          <button type="button" class="sheet-tab ${defaultTab === "non-grid" ? "active" : ""}" data-tab="non-grid">Non-grid</button>
          <button type="button" class="sheet-tab ${defaultTab === "ops" ? "active" : ""}" data-tab="ops">Operations</button>
        </div>
        <div class="sheet-tab-content ${defaultTab === "grid" ? "active" : ""}" data-tab="grid">
          ${previewBanner}
          ${gridHtml || `<div class="empty-state">No grid preview available.</div>`}
        </div>
        <div class="sheet-tab-content ${defaultTab === "hunks" ? "active" : ""}" data-tab="hunks">
          ${hunksHtml}
        </div>
        <div class="sheet-tab-content ${defaultTab === "non-grid" ? "active" : ""}" data-tab="non-grid">
          ${nonGridHtml}
        </div>
        <div class="sheet-tab-content ${defaultTab === "ops" ? "active" : ""}" data-tab="ops">
          ${detailsHtml ? `
            <details class="details-section" open>
              <summary class="details-toggle">Grouped changes</summary>
              <div class="details-content">
                ${detailsHtml}
              </div>
            </details>
          ` : ""}
          ${opsTableHtml}
        </div>
      </div>
    </section>
  `;
}

function renderStepDiffs(report, item) {
  const diffs = item?.raw?.semantic_detail?.step_diffs || [];
  if (!diffs.length) return "";
  const lines = diffs.map(diff => {
    const kind = diff.kind || "";
    if (kind === "step_added") {
      const name = resolveString(report, diff.step?.name);
      return `Added step ${name || "<unknown>"}`;
    }
    if (kind === "step_removed") {
      const name = resolveString(report, diff.step?.name);
      return `Removed step ${name || "<unknown>"}`;
    }
    if (kind === "step_reordered") {
      const name = resolveString(report, diff.name);
      return `Reordered step ${name || "<unknown>"} (${diff.from_index} -> ${diff.to_index})`;
    }
    if (kind === "step_modified") {
      const beforeName = resolveString(report, diff.before?.name);
      const afterName = resolveString(report, diff.after?.name);
      const changes = (diff.changes || []).map(change => {
        if (change.kind === "renamed") {
          const from = resolveString(report, change.from);
          const to = resolveString(report, change.to);
          return `renamed ${from} -> ${to}`;
        }
        return change.kind || "change";
      });
      const changeText = changes.length ? ` (${changes.join(", ")})` : "";
      return `Modified step ${beforeName || "<unknown>"} -> ${afterName || "<unknown>"}${changeText}`;
    }
    return kind || "step";
  });
  return `
    <div class="step-diffs">
      <div class="step-diffs-title">Step diffs</div>
      <ul>
        ${lines.map(line => `<li>${esc(line)}</li>`).join("")}
      </ul>
    </div>
  `;
}

function renderOtherChangesVm(title, icon, items, report) {
  if (!items || items.length === 0) return "";
  const sectionId = `other-${domSafeId(title)}`;
  return `
    <div class="other-changes" id="${esc(sectionId)}">
      <div class="other-changes-title">
        <span class="icon">${icon}</span>
        <span>${esc(title)} (${items.length})</span>
      </div>
      <div class="other-table">
        <div class="other-row other-header">
          <div class="other-kind">Type</div>
          <div class="other-name">Item</div>
          <div class="other-detail">Detail</div>
          <div class="other-old">Old</div>
          <div class="other-new">New</div>
        </div>
        ${items
          .map(item => {
            const oldVal = item.oldValue || "";
            const newVal = item.newValue || "";
            const detail = item.detail || "";
            const stepDiffs = renderStepDiffs(report, item);
            const rowId = `${sectionId}-${domSafeId(item.id)}`;
            return `
              <div class="other-row ${esc(item.changeType || "modified")}" id="${esc(rowId)}">
                <div class="other-kind">${esc(item.kind || "")}</div>
                <div class="other-name">${esc(item.label || item.name || "")}</div>
                <div class="other-detail">${esc(detail)}${stepDiffs}</div>
                <div class="other-old">${esc(oldVal)}</div>
                <div class="other-new">${esc(newVal)}</div>
              </div>
            `;
          })
          .join("")}
      </div>
    </div>
  `;
}

export function renderWorkbookVm(vm) {
  let html = "";
  html += renderWarnings(vm.warnings);
  html += renderPreviewLimitations(vm);
  html += renderSummaryCards(vm.counts);
  html += renderSummaryBreakdowns(vm);

  const total = vm.counts.added + vm.counts.removed + vm.counts.modified + vm.counts.moved;
  if (total === 0) {
    return html;
  }

  html += renderReviewToolbar(vm);
  html += renderSheetIndex(vm);

  for (const sheetVm of vm.sheets) {
    html += renderSheetVm(sheetVm);
  }

  html += renderOtherChangesVm("VBA Modules", "V", vm.other.vba, vm.report);
  html += renderOtherChangesVm("Named Ranges", "N", vm.other.namedRanges, vm.report);
  html += renderOtherChangesVm("Charts", "C", vm.other.charts, vm.report);
  html += renderOtherChangesVm("Power Query", "Q", vm.other.queries, vm.report);
  html += renderOtherChangesVm("Model", "M", vm.other.model, vm.report);

  return html;
}

export function renderReportHtml(payloadOrReport) {
  const vm = buildWorkbookViewModel(payloadOrReport);
  return renderWorkbookVm(vm);
}
