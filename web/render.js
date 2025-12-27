import { buildWorkbookViewModel } from "./view_model.js";

function esc(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function resolveString(report, id) {
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
    } else if (op.kind === "SheetAdded" || op.kind === "SheetRemoved") {
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
  const measureOps = [];
  
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
    } else if (kind.startsWith("Measure")) {
      measureOps.push(op);
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
    measureOps,
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
  const otherOps = ops.filter(o => !o.kind.startsWith("Row") && !o.kind.startsWith("Column") && o.kind !== "CellEdited" && !o.kind.startsWith("Block") && o.kind !== "SheetAdded" && o.kind !== "SheetRemoved");
  
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
    <section class="sheet-section">
      <div class="sheet-header" onclick="this.parentElement.classList.toggle('expanded')">
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


function renderChangeItemVm(item) {
  const changeType = item.changeType || "modified";
  const cls = `change-item ${changeType}`;
  const icon = changeType === "added" ? "+" : changeType === "removed" ? "-" : changeType === "moved" ? ">" : "~";
  const detail = item.detail ? `<span class="change-detail">${esc(item.detail)}</span>` : "";
  return `
    <div class="${cls}">
      <div class="change-icon">${icon}</div>
      <span class="change-location">${esc(item.label || "")}</span>
      ${detail}
    </div>
  `;
}

function renderChangeGroupVm(title, icon, items) {
  if (!items || items.length === 0) return "";
  return `
    <div class="change-group">
      <div class="change-group-title">
        <span>${icon}</span>
        <span>${esc(title)} (${items.length})</span>
      </div>
      <div class="change-list">
        ${items.map(item => renderChangeItemVm(item)).join("")}
      </div>
    </div>
  `;
}

function renderGridLegend() {
  return `
    <div class="grid-legend">
      <span class="legend-item"><span class="legend-box legend-edited"></span> Modified</span>
      <span class="legend-item"><span class="legend-box legend-added"></span> Added row/col</span>
      <span class="legend-item"><span class="legend-box legend-removed"></span> Removed row/col</span>
      <span class="legend-item"><span class="legend-box legend-moved"></span> Moved row/col</span>
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
  if (regionIds.length === 0) return "";

  const regionMap = new Map(sheetVm.changes.regions.map(region => [region.id, region]));
  let html = "";
  for (const id of regionIds) {
    const region = regionMap.get(id);
    if (!region) continue;
    html += renderRegionGrid(sheetVm, region);
  }
  html += renderGridLegend();
  return html;
}

function renderSheetVm(sheetVm) {
  const badge = `${sheetVm.opCount} change${sheetVm.opCount !== 1 ? "s" : ""}`;
  const gridHtml = renderSheetGridVm(sheetVm);

  let contentHtml = "";
  if (gridHtml) {
    contentHtml += `
      <div class="change-group">
        <div class="change-group-title">
          <span>*</span>
          <span>Visual Diff</span>
        </div>
        ${gridHtml}
      </div>
    `;
  }

  const rowItems = sheetVm.changes.items.filter(item => item.group === "rows");
  const colItems = sheetVm.changes.items.filter(item => item.group === "cols");
  const cellItems = sheetVm.changes.items.filter(item => item.group === "cells");
  const moveItems = sheetVm.changes.items.filter(item => item.group === "moves");
  const otherItems = sheetVm.changes.items.filter(item => item.group === "other");

  let detailsHtml = "";
  detailsHtml += renderChangeGroupVm("Row Changes", "R", rowItems);
  detailsHtml += renderChangeGroupVm("Column Changes", "C", colItems);
  detailsHtml += renderChangeGroupVm("Cell Changes", "*", cellItems);
  detailsHtml += renderChangeGroupVm("Moved Blocks", ">", moveItems);
  detailsHtml += renderChangeGroupVm("Other Changes", "?", otherItems);

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
    <section class="sheet-section">
      <div class="sheet-header" onclick="this.parentElement.classList.toggle('expanded')">
        <div class="sheet-title">
          <div class="sheet-icon">#</div>
          <span class="sheet-name">${esc(sheetVm.name)}</span>
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

function renderOtherChangesVm(title, icon, items) {
  if (!items || items.length === 0) return "";
  return `
    <div class="other-changes">
      <div class="other-changes-title">
        <span class="icon">${icon}</span>
        <span>${esc(title)} (${items.length})</span>
      </div>
      <div class="change-list">
        ${items.map(item => renderChangeItemVm(item)).join("")}
      </div>
    </div>
  `;
}

function renderWorkbookVm(vm) {
  let html = "";
  html += renderWarnings(vm.warnings);
  html += renderSummaryCards(vm.counts);

  const total = vm.counts.added + vm.counts.removed + vm.counts.modified + vm.counts.moved;
  if (total === 0) {
    return html;
  }

  for (const sheetVm of vm.sheets) {
    html += renderSheetVm(sheetVm);
  }

  html += renderOtherChangesVm("VBA Modules", "V", vm.other.vba);
  html += renderOtherChangesVm("Named Ranges", "N", vm.other.namedRanges);
  html += renderOtherChangesVm("Charts", "C", vm.other.charts);
  html += renderOtherChangesVm("Power Query", "Q", vm.other.queries);
  html += renderOtherChangesVm("Measures", "M", vm.other.measures);

  return html;
}

export function renderReportHtml(payloadOrReport) {
  const vm = buildWorkbookViewModel(payloadOrReport);
  return renderWorkbookVm(vm);
}
