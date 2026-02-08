const CELL_KEY_STRIDE = 16384;

const DEFAULT_OPTS = {
  contextRows: 1,
  contextCols: 1,
  maxCellsPerRegion: 200,
  maxVisualCells: 5000,
  mergeGap: 1,
  ignoreBlankToBlank: true,
  previewRows: 200,
  previewCols: 80
};

const DEFAULT_NOISE_FILTERS = {
  hideMFormattingOnly: false,
  hideDaxFormattingOnly: false,
  hideFormulaFormattingOnly: false,
  collapseMoves: false
};

function normalizeNoiseFilters(raw) {
  const input = raw && typeof raw === "object" ? raw : {};
  return {
    hideMFormattingOnly: Boolean(input.hideMFormattingOnly ?? input.hide_m_formatting_only),
    hideDaxFormattingOnly: Boolean(input.hideDaxFormattingOnly ?? input.hide_dax_formatting_only),
    hideFormulaFormattingOnly: Boolean(
      input.hideFormulaFormattingOnly ?? input.hide_formula_formatting_only
    ),
    collapseMoves: Boolean(input.collapseMoves ?? input.collapse_moves)
  };
}

function moveIdForOp(op) {
  if (!op || typeof op !== "object") return "";
  const kind = op.kind || "";
  const sheet = op.sheet != null ? `s${op.sheet}:` : "";
  const hash = op.block_hash != null ? `#${op.block_hash}` : "";
  if (kind === "BlockMovedRows") {
    return `${sheet}r:${op.src_start_row}+${op.row_count}->${op.dst_start_row}${hash}`;
  }
  if (kind === "BlockMovedColumns") {
    return `${sheet}c:${op.src_start_col}+${op.col_count}->${op.dst_start_col}${hash}`;
  }
  if (kind === "BlockMovedRect") {
    return `${sheet}x:${op.src_start_row},${op.src_start_col}+${op.src_row_count}x${op.src_col_count}->${op.dst_start_row},${op.dst_start_col}${hash}`;
  }
  return "";
}

function applyNoiseFiltersToOps(rawOps, rawFilters) {
  const ops = Array.isArray(rawOps) ? rawOps : [];
  const filters = { ...DEFAULT_NOISE_FILTERS, ...normalizeNoiseFilters(rawFilters) };
  if (
    !filters.hideMFormattingOnly &&
    !filters.hideDaxFormattingOnly &&
    !filters.hideFormulaFormattingOnly &&
    !filters.collapseMoves
  ) {
    return ops;
  }

  const collapsedMoves = filters.collapseMoves ? new Set() : null;
  const out = [];
  for (const op of ops) {
    const kind = op?.kind || "";

    if (filters.hideMFormattingOnly && kind === "QueryDefinitionChanged") {
      if ((op.change_kind || op.changeKind) === "formatting_only") continue;
    }

    if (filters.hideDaxFormattingOnly && (kind === "MeasureDefinitionChanged" || kind === "CalculatedColumnDefinitionChanged")) {
      if ((op.change_kind || op.changeKind) === "formatting_only") continue;
    }

    if (filters.hideFormulaFormattingOnly && kind === "CellEdited") {
      const diff = op.formula_diff || op.formulaDiff || "";
      if (diff === "formatting_only") continue;
    }

    if (collapsedMoves && kind.startsWith("BlockMoved")) {
      const moveId = moveIdForOp(op);
      if (moveId) {
        if (collapsedMoves.has(moveId)) continue;
        collapsedMoves.add(moveId);
      }
    }

    out.push(op);
  }

  return out;
}

function resolveString(report, id) {
  if (typeof id !== "number") return String(id);
  if (!report || !Array.isArray(report.strings)) return "<unknown>";
  return report.strings[id] != null ? report.strings[id] : "<unknown>";
}

function formatChangeKind(kind) {
  if (!kind) return "";
  return String(kind).replace(/_/g, " ");
}

function formatFieldLabel(field) {
  if (!field) return "";
  return String(field).replace(/_/g, " ");
}

function formatColumnRef(report, tableId, columnId) {
  const table = resolveString(report, tableId);
  const column = resolveString(report, columnId);
  return `${table}.${column}`;
}

function formatRelationshipRef(report, op) {
  const fromTable = resolveString(report, op.from_table);
  const fromColumn = resolveString(report, op.from_column);
  const toTable = resolveString(report, op.to_table);
  const toColumn = resolveString(report, op.to_column);
  return `${fromTable}[${fromColumn}] -> ${toTable}[${toColumn}]`;
}

function isModelKind(kind) {
  return (
    kind === "CalculatedColumnDefinitionChanged" ||
    kind.startsWith("Table") ||
    kind.startsWith("ModelColumn") ||
    kind.startsWith("Relationship") ||
    kind.startsWith("Measure")
  );
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

function resolveFormula(report, id) {
  if (id === null || id === undefined) return "";
  const text = resolveString(report, id);
  if (!text) return "";
  return text.startsWith("=") ? text : `=${text}`;
}

function normalizeSheetList(list) {
  if (!list) return [];
  if (Array.isArray(list)) return list;
  if (Array.isArray(list.sheets)) return list.sheets;
  return [];
}

function normalizePayload(payloadOrReport) {
  const payload = payloadOrReport && payloadOrReport.report ? payloadOrReport : { report: payloadOrReport };
  const report = payload.report || payloadOrReport || {};
  const rawSheets = payload.sheets || null;
  const alignments = Array.isArray(payload.alignments) ? payload.alignments : [];
  const interestRectsRaw = payload.interestRects || payload.interest_rects;
  const interestRects = Array.isArray(interestRectsRaw) ? interestRectsRaw : [];
  const sheets = {
    oldSheets: normalizeSheetList(rawSheets?.old),
    newSheets: normalizeSheetList(rawSheets?.new)
  };
  return { report, sheets, alignments, interestRects };
}

function buildSheetLookup(sheets) {
  const map = new Map();
  for (const sheet of sheets || []) {
    if (sheet && typeof sheet.name === "string") {
      map.set(sheet.name, sheet);
    }
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

function buildInterestRectLookup(interestRects) {
  const map = new Map();
  if (!Array.isArray(interestRects)) return map;
  for (const entry of interestRects) {
    if (entry && typeof entry.sheet === "string") {
      const rects = Array.isArray(entry.rects) ? entry.rects : [];
      map.set(entry.sheet, rects);
    }
  }
  return map;
}

function categorizeOps(report) {
  const ops = Array.isArray(report?.ops) ? report.ops : [];
  const sheetOps = new Map();
  const renameMap = new Map();
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
      const fromName = resolveString(report, op.from);
      if (!sheetOps.has(sheetName)) sheetOps.set(sheetName, []);
      sheetOps.get(sheetName).push(op);
      renameMap.set(sheetName, fromName);
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
      const sheetName = resolveString(report, op.sheet);
      if (!sheetOps.has(sheetName)) sheetOps.set(sheetName, []);
      sheetOps.get(sheetName).push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    } else if (kind.startsWith("Query")) {
      queryOps.push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    } else if (isModelKind(kind)) {
      modelOps.push(op);
      if (kind.includes("Added")) addedCount++;
      else if (kind.includes("Removed")) removedCount++;
      else modifiedCount++;
    }
  }

  return {
    sheetOps,
    renameMap,
    vbaOps,
    namedRangeOps,
    chartOps,
    queryOps,
    modelOps,
    counts: { added: addedCount, removed: removedCount, modified: modifiedCount, moved: movedCount }
  };
}

function isGridKind(kind) {
  return (
    kind === "CellEdited" ||
    kind.startsWith("Row") ||
    kind.startsWith("Column") ||
    kind.startsWith("Block") ||
    kind.startsWith("Rect") ||
    kind === "DuplicateKeyCluster"
  );
}

const CATEGORY_ORDER = ["Grid", "Power Query", "Model", "Objects", "Other"];

function opCategoryForKind(kind) {
  const k = kind || "";
  if (k.startsWith("Query")) return "Power Query";
  if (isModelKind(k)) return "Model";
  if (k.startsWith("Chart") || k.startsWith("NamedRange") || k.startsWith("Vba")) return "Objects";
  if (k.startsWith("Sheet") || isGridKind(k)) return "Grid";
  return "Other";
}

function severityRank(value) {
  if (value === "high") return 3;
  if (value === "medium") return 2;
  return 1;
}

function opSeverity(op) {
  const kind = op?.kind || "";

  if (kind === "DuplicateKeyCluster") return "high";

  if (kind.startsWith("Query")) {
    if (kind === "QueryDefinitionChanged") {
      const changeKind = (op.change_kind || op.changeKind || "").toLowerCase();
      if (changeKind === "semantic") return "high";
      if (changeKind === "formatting_only") return "low";
      if (changeKind === "renamed") return "low";
      return "medium";
    }
    if (kind === "QueryRenamed") return "low";
    if (kind === "QueryAdded" || kind === "QueryRemoved") return "high";
    if (kind === "QueryMetadataChanged") {
      const field = op.field || "";
      if (field === "LoadToSheet" || field === "LoadToModel") return "medium";
      return "low";
    }
    return "medium";
  }

  if (isModelKind(kind)) {
    if (kind === "MeasureDefinitionChanged" || kind === "CalculatedColumnDefinitionChanged") {
      const changeKind = (op.change_kind || op.changeKind || "").toLowerCase();
      if (changeKind === "semantic") return "high";
      if (changeKind === "formatting_only") return "low";
      return "medium";
    }
    if (kind.includes("Added") || kind.includes("Removed")) return "high";
    if (kind.includes("TypeChanged")) return "high";
    if (kind.startsWith("Relationship")) return "high";
    return "medium";
  }

  if (kind.startsWith("Vba")) {
    if (kind.includes("Changed") || kind.includes("Added") || kind.includes("Removed")) return "high";
    return "medium";
  }

  if (kind.startsWith("Chart") || kind.startsWith("NamedRange")) {
    return "medium";
  }

  if (kind.startsWith("Sheet")) {
    if (kind === "SheetRenamed") return "low";
    return "medium";
  }

  if (kind.startsWith("BlockMoved")) return "medium";

  if (kind.startsWith("Row") || kind.startsWith("Column") || kind === "RectReplaced") {
    return "medium";
  }

  if (kind === "CellEdited") {
    const diff = (op.formula_diff || op.formulaDiff || "").toLowerCase();
    if (diff === "semantic_change") return "high";
    if (diff === "formatting_only") return "low";
    if (diff === "added" || diff === "removed") return "high";
    if (diff === "filled") return "medium";
    if (diff === "text_change") return "medium";
    return "medium";
  }

  return "medium";
}

function maxSeverityForOps(ops) {
  let best = "low";
  for (const op of ops || []) {
    const sev = opSeverity(op);
    if (severityRank(sev) > severityRank(best)) best = sev;
    if (best === "high") return best;
  }
  return best;
}

function changeTypeForKind(kind) {
  if (!kind) return "modified";
  if (kind.includes("Added")) return "added";
  if (kind.includes("Removed")) return "removed";
  if (kind.includes("Moved")) return "moved";
  if (kind.includes("Renamed")) return "modified";
  if (kind.includes("Edited") || kind.includes("Changed") || kind.includes("Replaced")) return "modified";
  return "modified";
}

function groupForKind(kind) {
  if (!kind) return "other";
  if (kind.startsWith("Row")) return "rows";
  if (kind.startsWith("Column")) return "cols";
  if (kind === "CellEdited" || kind.startsWith("Rect")) return "cells";
  if (kind.startsWith("Block") || kind.includes("Moved")) return "moves";
  if (kind.startsWith("Chart") || kind.startsWith("NamedRange") || kind.startsWith("Vba") || kind.startsWith("Query") || isModelKind(kind)) {
    return "other";
  }
  return "other";
}

function buildSheetCounts(ops) {
  const counts = { added: 0, removed: 0, modified: 0, moved: 0 };
  for (const op of ops || []) {
    const kind = op.kind || "";
    const changeType = changeTypeForKind(kind);
    if (changeType === "added") counts.added += 1;
    else if (changeType === "removed") counts.removed += 1;
    else if (changeType === "moved") counts.moved += 1;
    else counts.modified += 1;
  }
  return counts;
}

function makeAxisVm(entries, oldLen, newLen) {
  const list = Array.isArray(entries) ? entries : [];
  const oldToView = new Array(oldLen || 0).fill(null);
  const newToView = new Array(newLen || 0).fill(null);
  for (let i = 0; i < list.length; i++) {
    const entry = list[i];
    if (!entry) continue;
    if (entry.old !== null && entry.old !== undefined && oldToView.length) {
      oldToView[entry.old] = i;
    }
    if (entry.new !== null && entry.new !== undefined && newToView.length) {
      newToView[entry.new] = i;
    }
  }
  return {
    entries: list,
    oldToView,
    newToView,
    oldLen: oldLen || 0,
    newLen: newLen || 0,
    count: list.length
  };
}

function makeCellMap(sheet) {
  const map = new Map();
  if (!sheet || !Array.isArray(sheet.cells)) return map;
  for (const cell of sheet.cells) {
    const key = cell.row * CELL_KEY_STRIDE + cell.col;
    map.set(key, cell);
  }
  return map;
}

function mapIndexToView(index, map) {
  if (index === null || index === undefined) return null;
  if (Array.isArray(map) && map.length > 0) {
    const mapped = map[index];
    if (mapped !== null && mapped !== undefined) return mapped;
  }
  return index;
}

function makeEditMap(report, sheetOps, rowsVm, colsVm, opts) {
  const editMap = new Map();
  const ignoreBlank = opts?.ignoreBlankToBlank !== false;
  for (const op of sheetOps) {
    if (op.kind !== "CellEdited") continue;
    const addr = parseCellAddress(op.addr);
    if (!addr) continue;
    const viewRow = mapIndexToView(addr.row, rowsVm.newToView);
    const viewCol = mapIndexToView(addr.col, colsVm.newToView);
    if (viewRow === null || viewCol === null) continue;
    const key = viewRow * CELL_KEY_STRIDE + viewCol;
    const fromValue = op.from ? formatValue(report, op.from.value) : "";
    const toValue = op.to ? formatValue(report, op.to.value) : "";
    const fromFormula = resolveFormula(report, op.from?.formula);
    const toFormula = resolveFormula(report, op.to?.formula);
    const formulaDiff = op.formula_diff || op.formulaDiff || "";
    if (ignoreBlank && !fromValue && !toValue && !fromFormula && !toFormula) {
      continue;
    }
    editMap.set(key, { fromValue, toValue, fromFormula, toFormula, formulaDiff });
  }
  return editMap;
}

function cellDisplayText(cell) {
  if (!cell) return "";
  return cell.value || cell.formula || "";
}

function cellTooltip(label, cell) {
  if (!cell) return "";
  const value = cell.value ?? "";
  const formula = cell.formula ?? "";
  if (!value && !formula) return "";
  if (value && formula && value !== formula) {
    return label ? `${label}: ${value} | ${formula}` : `${value} | ${formula}`;
  }
  const text = value || formula;
  return label ? `${label}: ${text}` : text;
}

function buildCellVm(viewRow, viewCol, rowsVm, colsVm, oldCells, newCells, editMap) {
  const rowEntry = rowsVm.entries[viewRow];
  const colEntry = colsVm.entries[viewCol];
  if (!rowEntry || !colEntry) {
    return {
      viewRow,
      viewCol,
      old: null,
      new: null,
      diffKind: "empty",
      display: { text: "", tooltip: "" }
    };
  }

  const oldRow = rowEntry.old;
  const oldCol = colEntry.old;
  const newRow = rowEntry.new;
  const newCol = colEntry.new;

  const oldCell =
    oldRow !== null && oldRow !== undefined && oldCol !== null && oldCol !== undefined
      ? oldCells.get(oldRow * CELL_KEY_STRIDE + oldCol) || null
      : null;
  const newCell =
    newRow !== null && newRow !== undefined && newCol !== null && newCol !== undefined
      ? newCells.get(newRow * CELL_KEY_STRIDE + newCol) || null
      : null;

  const viewKey = viewRow * CELL_KEY_STRIDE + viewCol;
  const edit = editMap.get(viewKey);

  const rowKind = rowEntry.kind;
  const colKind = colEntry.kind;
  const isInsert = rowKind === "insert" || colKind === "insert";
  const isDelete = rowKind === "delete" || colKind === "delete";
  const isMoveSrc = rowKind === "move_src" || colKind === "move_src";
  const isMoveDst = rowKind === "move_dst" || colKind === "move_dst";
  const moveRole = isMoveSrc ? "src" : isMoveDst ? "dst" : undefined;
  const moveId = rowEntry.move_id || colEntry.move_id || undefined;

  let diffKind = "empty";
  if (edit) diffKind = "edited";
  else if (isInsert) diffKind = "added";
  else if (isDelete) diffKind = "removed";
  else if (isMoveSrc || isMoveDst) diffKind = "moved";
  else if (oldCell || newCell) diffKind = "unchanged";

  let displayText = "";
  let tooltip = "";
  if (diffKind === "edited") {
    const fromText = edit.fromValue || edit.fromFormula || "";
    const toText = edit.toValue || edit.toFormula || "";
    displayText = toText || fromText;
    tooltip = fromText || toText ? `Changed: ${fromText || "(empty)"} -> ${toText || "(empty)"}` : "";
  } else if (diffKind === "added") {
    displayText = cellDisplayText(newCell);
    tooltip = cellTooltip("Added", newCell);
  } else if (diffKind === "removed") {
    displayText = cellDisplayText(oldCell);
    tooltip = cellTooltip("Removed", oldCell);
  } else if (diffKind === "moved") {
    const cell = moveRole === "src" ? oldCell : newCell;
    displayText = cellDisplayText(cell);
    tooltip = cellTooltip("Moved", cell);
  } else if (diffKind === "unchanged") {
    displayText = cellDisplayText(newCell || oldCell);
    tooltip = cellTooltip("Value", newCell || oldCell);
  }

  return {
    viewRow,
    viewCol,
    old: oldRow !== null && oldRow !== undefined && oldCol !== null && oldCol !== undefined ? { row: oldRow, col: oldCol, cell: oldCell } : null,
    new: newRow !== null && newRow !== undefined && newCol !== null && newCol !== undefined ? { row: newRow, col: newCol, cell: newCell } : null,
    diffKind,
    moveId,
    moveRole,
    edit: edit || undefined,
    display: { text: displayText, tooltip }
  };
}

function mapRangeToView(start, count, map) {
  if (start === null || start === undefined || count === null || count === undefined) return null;
  if (count <= 0) return null;
  let minView = null;
  let maxView = null;
  const end = start + count - 1;
  for (let idx = start; idx <= end; idx++) {
    const view = mapIndexToView(idx, map);
    if (view === null || view === undefined) continue;
    if (minView === null || view < minView) minView = view;
    if (maxView === null || view > maxView) maxView = view;
  }
  if (minView === null || maxView === null) return null;
  return { start: minView, end: maxView };
}

function viewStartForRange(axisVm, start, count, side) {
  if (!axisVm) return null;
  const map = side === "old" ? axisVm.oldToView : axisVm.newToView;
  const range = mapRangeToView(start, count, map);
  return range ? range.start : null;
}

function viewIndexForSide(axisVm, viewIndex, side) {
  if (axisVm?.entries?.length) {
    const entry = axisVm.entries[viewIndex];
    const value = entry ? entry[side] : null;
    if (value !== null && value !== undefined) return value;
  }
  return viewIndex;
}

function formatRowRange(axisVm, startView, endView, side, action) {
  const start = viewIndexForSide(axisVm, startView, side);
  const end = viewIndexForSide(axisVm, endView, side);
  if (start === end) return `Row ${start + 1} ${action}`;
  return `Rows ${start + 1}-${end + 1} ${action}`;
}

function formatColRange(axisVm, startView, endView, side, action) {
  const start = viewIndexForSide(axisVm, startView, side);
  const end = viewIndexForSide(axisVm, endView, side);
  if (start === end) return `Column ${colToLetter(start)} ${action}`;
  return `Columns ${colToLetter(start)}-${colToLetter(end)} ${action}`;
}

function groupConsecutive(indices) {
  const sorted = [...indices].sort((a, b) => a - b);
  const groups = [];
  let start = null;
  let prev = null;
  for (const idx of sorted) {
    if (start === null) {
      start = idx;
      prev = idx;
      continue;
    }
    if (idx === prev + 1) {
      prev = idx;
      continue;
    }
    groups.push({ start, end: prev, count: prev - start + 1 });
    start = idx;
    prev = idx;
  }
  if (start !== null) {
    groups.push({ start, end: prev, count: prev - start + 1 });
  }
  return groups;
}

function clusterEditsToRegions(editKeys, opts) {
  if (!editKeys || editKeys.length === 0) return [];
  const byRow = new Map();
  for (const key of editKeys) {
    const row = Math.floor(key / CELL_KEY_STRIDE);
    const col = key % CELL_KEY_STRIDE;
    if (!byRow.has(row)) byRow.set(row, []);
    byRow.get(row).push(col);
  }
  const rows = Array.from(byRow.keys()).sort((a, b) => a - b);
  const regions = [];
  const mergeGap = opts.mergeGap ?? 0;
  const maxCells = opts.maxCellsPerRegion ?? 200;
  let active = [];

  for (const row of rows) {
    const cols = byRow.get(row).sort((a, b) => a - b);
    const runs = [];
    let runStart = null;
    let runEnd = null;
    for (const col of cols) {
      if (runStart === null) {
        runStart = col;
        runEnd = col;
        continue;
      }
      if (col === runEnd + 1) {
        runEnd = col;
        continue;
      }
      runs.push({ start: runStart, end: runEnd, count: runEnd - runStart + 1 });
      runStart = col;
      runEnd = col;
    }
    if (runStart !== null) {
      runs.push({ start: runStart, end: runEnd, count: runEnd - runStart + 1 });
    }

    const expandedRuns = [];
    if (maxCells > 0) {
      for (const run of runs) {
        if (run.count <= maxCells) {
          expandedRuns.push(run);
          continue;
        }
        let segStart = run.start;
        while (segStart <= run.end) {
          const segEnd = Math.min(run.end, segStart + maxCells - 1);
          expandedRuns.push({ start: segStart, end: segEnd, count: segEnd - segStart + 1 });
          segStart = segEnd + 1;
        }
      }
    } else {
      expandedRuns.push(...runs);
    }

    const nextActive = [];
    const used = new Set();

    for (const run of expandedRuns) {
      let matched = null;
      let matchedIndex = -1;
      for (let i = 0; i < active.length; i++) {
        const region = active[i];
        if (region.rowEnd !== row - 1) continue;
        const overlaps =
          run.start <= region.colEnd + mergeGap &&
          run.end >= region.colStart - mergeGap;
        if (overlaps) {
          matched = region;
          matchedIndex = i;
          break;
        }
      }

      if (matched) {
        const nextCount = matched.cellCount + run.count;
        if (nextCount > maxCells) {
          regions.push(matched);
        } else {
          matched.rowEnd = row;
          matched.colStart = Math.min(matched.colStart, run.start);
          matched.colEnd = Math.max(matched.colEnd, run.end);
          matched.cellCount = nextCount;
          nextActive.push(matched);
          used.add(matchedIndex);
          continue;
        }
      }

      nextActive.push({
        rowStart: row,
        rowEnd: row,
        colStart: run.start,
        colEnd: run.end,
        cellCount: run.count
      });
    }

    for (let i = 0; i < active.length; i++) {
      if (!used.has(i)) {
        regions.push(active[i]);
      }
    }
    active = nextActive;
  }

  regions.push(...active);

  return regions.map((region, idx) => ({
    id: `cells-${idx + 1}`,
    kind: "cell",
    top: region.rowStart,
    bottom: region.rowEnd,
    left: region.colStart,
    right: region.colEnd,
    cellCount: region.cellCount
  }));
}

function capBounds(bounds, maxVisualCells) {
  if (!maxVisualCells || maxVisualCells <= 0) return bounds;
  let rows = bounds.bottom - bounds.top + 1;
  let cols = bounds.right - bounds.left + 1;
  if (rows * cols <= maxVisualCells) return bounds;
  const maxRows = Math.max(1, Math.floor(maxVisualCells / cols));
  if (maxRows < rows) {
    return { ...bounds, bottom: bounds.top + maxRows - 1 };
  }
  const maxCols = Math.max(1, Math.floor(maxVisualCells / rows));
  if (maxCols < cols) {
    return { ...bounds, right: bounds.left + maxCols - 1 };
  }
  return bounds;
}

function expandBounds(region, rowsCount, colsCount, opts) {
  const contextRows = opts.contextRows ?? 0;
  const contextCols = opts.contextCols ?? 0;
  const top = Math.max(0, region.top - contextRows);
  const left = Math.max(0, region.left - contextCols);
  const bottom = Math.min(rowsCount - 1, region.bottom + contextRows);
  const right = Math.min(colsCount - 1, region.right + contextCols);
  return capBounds({ top, left, bottom, right }, opts.maxVisualCells);
}

function buildChangeItems({ report, ops, rowsVm, colsVm, alignment, regions }) {
  const items = [];

  const rowAdds = [];
  const rowRemoves = [];
  const colAdds = [];
  const colRemoves = [];

  for (const op of ops) {
    if (op.kind === "RowAdded") {
      const viewIdx = mapIndexToView(op.row_idx, rowsVm.newToView);
      if (viewIdx !== null && viewIdx !== undefined) {
        rowAdds.push(viewIdx);
      }
    } else if (op.kind === "RowRemoved") {
      const viewIdx = mapIndexToView(op.row_idx, rowsVm.oldToView);
      if (viewIdx !== null && viewIdx !== undefined) {
        rowRemoves.push(viewIdx);
      }
    } else if (op.kind === "ColumnAdded") {
      const viewIdx = mapIndexToView(op.col_idx, colsVm.newToView);
      if (viewIdx !== null && viewIdx !== undefined) {
        colAdds.push(viewIdx);
      }
    } else if (op.kind === "ColumnRemoved") {
      const viewIdx = mapIndexToView(op.col_idx, colsVm.oldToView);
      if (viewIdx !== null && viewIdx !== undefined) {
        colRemoves.push(viewIdx);
      }
    } else if (op.kind === "RowReplaced") {
      const viewIdx = mapIndexToView(op.row_idx, rowsVm.newToView);
      items.push({
        id: `row-replaced-${op.row_idx}`,
        group: "rows",
        changeType: "modified",
        label: `Row ${viewIndexForSide(rowsVm, viewIdx, "new") + 1} replaced`,
        axis: "row",
        viewStart: viewIdx,
        viewEnd: viewIdx
      });
    }
  }

  for (const group of groupConsecutive(rowAdds)) {
    items.push({
      id: `row-added-${group.start}`,
      group: "rows",
      changeType: "added",
      label: formatRowRange(rowsVm, group.start, group.end, "new", "added"),
      axis: "row",
      viewStart: group.start,
      viewEnd: group.end
    });
  }
  for (const group of groupConsecutive(rowRemoves)) {
    items.push({
      id: `row-removed-${group.start}`,
      group: "rows",
      changeType: "removed",
      label: formatRowRange(rowsVm, group.start, group.end, "old", "removed"),
      axis: "row",
      viewStart: group.start,
      viewEnd: group.end
    });
  }
  for (const group of groupConsecutive(colAdds)) {
    items.push({
      id: `col-added-${group.start}`,
      group: "cols",
      changeType: "added",
      label: formatColRange(colsVm, group.start, group.end, "new", "added"),
      axis: "col",
      viewStart: group.start,
      viewEnd: group.end
    });
  }
  for (const group of groupConsecutive(colRemoves)) {
    items.push({
      id: `col-removed-${group.start}`,
      group: "cols",
      changeType: "removed",
      label: formatColRange(colsVm, group.start, group.end, "old", "removed"),
      axis: "col",
      viewStart: group.start,
      viewEnd: group.end
    });
  }

  const alignmentMoves = Array.isArray(alignment?.moves) ? alignment.moves : [];
  if (alignmentMoves.length > 0) {
    for (const move of alignmentMoves) {
      if (move.axis === "row") {
        const start = move.src_start + 1;
        const end = move.src_start + move.count;
        const label = start === end ? `Row ${start} moved` : `Rows ${start}-${end} moved`;
        items.push({
          id: `move-${move.id}`,
          group: "moves",
          changeType: "moved",
          label,
          detail: `to row ${move.dst_start + 1}`,
          moveId: move.id,
          axis: "row",
          srcStart: move.src_start,
          dstStart: move.dst_start,
          count: move.count
        });
      } else if (move.axis === "col") {
        const start = colToLetter(move.src_start);
        const end = colToLetter(move.src_start + move.count - 1);
        const label = start === end ? `Column ${start} moved` : `Columns ${start}-${end} moved`;
        items.push({
          id: `move-${move.id}`,
          group: "moves",
          changeType: "moved",
          label,
          detail: `to column ${colToLetter(move.dst_start)}`,
          moveId: move.id,
          axis: "col",
          srcStart: move.src_start,
          dstStart: move.dst_start,
          count: move.count
        });
      }
    }
  } else {
    for (const op of ops) {
      if (op.kind === "BlockMovedRows") {
        const start = op.src_start_row + 1;
        const end = op.src_start_row + op.row_count;
        const label = start === end ? `Row ${start} moved` : `Rows ${start}-${end} moved`;
        const moveId = `r:${op.src_start_row}+${op.row_count}->${op.dst_start_row}`;
        items.push({
          id: `move-rows-${op.src_start_row}-${op.dst_start_row}`,
          group: "moves",
          changeType: "moved",
          label,
          detail: `to row ${op.dst_start_row + 1}`,
          moveId,
          axis: "row",
          srcStart: op.src_start_row,
          dstStart: op.dst_start_row,
          count: op.row_count
        });
      } else if (op.kind === "BlockMovedColumns") {
        const start = colToLetter(op.src_start_col);
        const end = colToLetter(op.src_start_col + op.col_count - 1);
        const label = start === end ? `Column ${start} moved` : `Columns ${start}-${end} moved`;
        const moveId = `c:${op.src_start_col}+${op.col_count}->${op.dst_start_col}`;
        items.push({
          id: `move-cols-${op.src_start_col}-${op.dst_start_col}`,
          group: "moves",
          changeType: "moved",
          label,
          detail: `to column ${colToLetter(op.dst_start_col)}`,
          moveId,
          axis: "col",
          srcStart: op.src_start_col,
          dstStart: op.dst_start_col,
          count: op.col_count
        });
      }
    }
  }

  for (const op of ops) {
    if (op.kind === "BlockMovedRect") {
      const srcStart = formatCellAddress(op.src_start_row, op.src_start_col);
      const srcEnd = formatCellAddress(op.src_start_row + op.src_row_count - 1, op.src_start_col + op.src_col_count - 1);
      const dstStart = formatCellAddress(op.dst_start_row, op.dst_start_col);
      const dstEnd = formatCellAddress(op.dst_start_row + op.src_row_count - 1, op.dst_start_col + op.src_col_count - 1);
      const moveId = `rect:${op.src_start_row},${op.src_start_col}+${op.src_row_count}x${op.src_col_count}->${op.dst_start_row},${op.dst_start_col}`;
      items.push({
        id: `move-rect-${srcStart}-${dstStart}`,
        group: "moves",
        changeType: "moved",
        label: `Range ${srcStart}:${srcEnd} moved`,
        detail: `to ${dstStart}:${dstEnd}`,
        moveId,
        moveKind: "rect"
      });
    }
  }

  for (const region of regions) {
    if (region.kind === "cell") {
      const cellCount = region.cellCount || ((region.bottom - region.top + 1) * (region.right - region.left + 1));
      const detail = cellCount > 1 ? `${cellCount} cells` : "";
      items.push({
        id: `cell-region-${region.id}`,
        group: "cells",
        changeType: "modified",
        label: `${region.label} modified`,
        detail,
        regionId: region.id
      });
    } else if (region.kind === "rect") {
      items.push({
        id: `rect-region-${region.id}`,
        group: "cells",
        changeType: "modified",
        label: `${region.label} replaced`,
        regionId: region.id
      });
    }
  }

  for (const op of ops) {
    if (op.kind === "SheetRenamed") {
      const fromName = resolveString(report, op.from);
      const toName = resolveString(report, op.to ?? op.sheet);
      const fromId = op.from ?? "unknown";
      const toId = op.to ?? op.sheet ?? "unknown";
      items.push({
        id: `sheet-renamed-${fromId}-${toId}`,
        group: "other",
        changeType: "modified",
        label: "Sheet renamed",
        detail: `${fromName} -> ${toName}`
      });
    }
  }

  const handledKinds = new Set([
    "RowAdded",
    "RowRemoved",
    "RowReplaced",
    "ColumnAdded",
    "ColumnRemoved",
    "CellEdited",
    "RectReplaced",
    "BlockMovedRows",
    "BlockMovedColumns",
    "BlockMovedRect",
    "SheetAdded",
    "SheetRemoved",
    "SheetRenamed"
  ]);

  for (const op of ops) {
    if (handledKinds.has(op.kind)) continue;
    items.push({
      id: `other-${op.kind}`,
      group: "other",
      changeType: "modified",
      label: op.kind
    });
  }

  return items;
}

function anchorPriority(group, regionKind) {
  if (group === "moves") return 0;
  if (group === "rows" || group === "cols") return 1;
  if (regionKind === "rect") return 2;
  if (group === "cells") return 3;
  return 4;
}

function buildChangeAnchors({ sheetName, status, items, regions, rowsVm, colsVm }) {
  const entries = [];
  const regionById = new Map();
  const moveRegions = new Map();

  for (const region of regions || []) {
    regionById.set(region.id, region);
    if (region.moveId) {
      if (!moveRegions.has(region.moveId)) {
        moveRegions.set(region.moveId, {});
      }
      const record = moveRegions.get(region.moveId);
      if (region.kind === "move_src") record.src = region;
      if (region.kind === "move_dst") record.dst = region;
    }
  }

  const canGrid =
    (status?.kind === "ok" || status?.kind === "partial") &&
    Array.isArray(rowsVm?.entries) &&
    Array.isArray(colsVm?.entries) &&
    rowsVm.entries.length > 0 &&
    colsVm.entries.length > 0;

  function addAnchor({ id, group, changeType, label, detail, viewRow, viewCol, regionId, moveId, listElementId, regionKind }) {
    if (!id || !listElementId) return;
    const hasGridTarget = canGrid && Number.isFinite(viewRow) && Number.isFinite(viewCol);
    const target = hasGridTarget
      ? {
          kind: "grid",
          viewRow,
          viewCol,
          ...(regionId ? { regionId } : {}),
          ...(moveId ? { moveId } : {})
        }
      : { kind: "list", elementId: listElementId };
    const anchor = {
      id,
      group: group || "cells",
      changeType: changeType || "modified",
      label: label || id,
      target
    };
    if (detail) anchor.detail = detail;
    entries.push({
      anchor,
      sortRow: Number.isFinite(viewRow) ? viewRow : Number.MAX_SAFE_INTEGER,
      sortCol: Number.isFinite(viewCol) ? viewCol : Number.MAX_SAFE_INTEGER,
      priority: anchorPriority(group, regionKind)
    });
  }

  for (const item of items || []) {
    const listElementId = `change-${sheetName}-${item.id}`;

    if (item.regionId) {
      const region = regionById.get(item.regionId);
      if (region) {
        addAnchor({
          id: `region:${region.id}`,
          group: item.group,
          changeType: item.changeType,
          label: item.label,
          detail: item.detail,
          viewRow: region.top,
          viewCol: region.left,
          regionId: region.id,
          moveId: region.moveId,
          listElementId,
          regionKind: region.kind
        });
      }
      continue;
    }

    if (item.group === "rows") {
      addAnchor({
        id: `row:${item.changeType}:${item.viewStart}-${item.viewEnd}`,
        group: "rows",
        changeType: item.changeType,
        label: item.label,
        detail: item.detail,
        viewRow: item.viewStart,
        viewCol: 0,
        listElementId
      });
      continue;
    }

    if (item.group === "cols") {
      addAnchor({
        id: `col:${item.changeType}:${item.viewStart}-${item.viewEnd}`,
        group: "cols",
        changeType: item.changeType,
        label: item.label,
        detail: item.detail,
        viewRow: 0,
        viewCol: item.viewStart,
        listElementId
      });
      continue;
    }

    if (item.group === "moves" && item.moveId) {
      if (item.axis === "row" || item.axis === "col") {
        const axisVm = item.axis === "row" ? rowsVm : colsVm;
        const srcStart = viewStartForRange(axisVm, item.srcStart, item.count, "old");
        const dstStart = viewStartForRange(axisVm, item.dstStart, item.count, "new");
        if (srcStart !== null && srcStart !== undefined) {
          addAnchor({
            id: `move:${item.moveId}:src`,
            group: "moves",
            changeType: "moved",
            label: item.label,
            detail: item.detail,
            viewRow: item.axis === "row" ? srcStart : 0,
            viewCol: item.axis === "col" ? srcStart : 0,
            moveId: item.moveId,
            listElementId
          });
        }
        if (dstStart !== null && dstStart !== undefined) {
          addAnchor({
            id: `move:${item.moveId}:dst`,
            group: "moves",
            changeType: "moved",
            label: item.label,
            detail: item.detail,
            viewRow: item.axis === "row" ? dstStart : 0,
            viewCol: item.axis === "col" ? dstStart : 0,
            moveId: item.moveId,
            listElementId
          });
        }
        continue;
      }

      if (item.moveKind === "rect") {
        const moveRegion = moveRegions.get(item.moveId);
        if (moveRegion?.src) {
          addAnchor({
            id: `region:${moveRegion.src.id}`,
            group: "moves",
            changeType: "moved",
            label: item.label,
            detail: "From",
            viewRow: moveRegion.src.top,
            viewCol: moveRegion.src.left,
            regionId: moveRegion.src.id,
            moveId: item.moveId,
            listElementId,
            regionKind: moveRegion.src.kind
          });
        }
        if (moveRegion?.dst) {
          addAnchor({
            id: `region:${moveRegion.dst.id}`,
            group: "moves",
            changeType: "moved",
            label: item.label,
            detail: "To",
            viewRow: moveRegion.dst.top,
            viewCol: moveRegion.dst.left,
            regionId: moveRegion.dst.id,
            moveId: item.moveId,
            listElementId,
            regionKind: moveRegion.dst.kind
          });
        }
      }
    }
  }

  entries.sort((a, b) => {
    if (a.sortRow !== b.sortRow) return a.sortRow - b.sortRow;
    if (a.sortCol !== b.sortCol) return a.sortCol - b.sortCol;
    if (a.priority !== b.priority) return a.priority - b.priority;
    return String(a.anchor.id).localeCompare(String(b.anchor.id));
  });

  return entries.map(entry => entry.anchor);
}

function attachNavTargets(items, anchors) {
  const anchorIds = new Set((anchors || []).map(anchor => anchor.id));
  for (const item of items || []) {
    const targets = [];
    if (item.regionId) {
      const anchorId = `region:${item.regionId}`;
      if (anchorIds.has(anchorId)) targets.push({ anchorId });
    } else if (item.group === "rows") {
      const anchorId = `row:${item.changeType}:${item.viewStart}-${item.viewEnd}`;
      if (anchorIds.has(anchorId)) targets.push({ anchorId });
    } else if (item.group === "cols") {
      const anchorId = `col:${item.changeType}:${item.viewStart}-${item.viewEnd}`;
      if (anchorIds.has(anchorId)) targets.push({ anchorId });
    } else if (item.group === "moves" && item.moveId) {
      if (item.axis === "row" || item.axis === "col") {
        const srcId = `move:${item.moveId}:src`;
        const dstId = `move:${item.moveId}:dst`;
        if (anchorIds.has(srcId)) targets.push({ anchorId: srcId, label: "From" });
        if (anchorIds.has(dstId)) targets.push({ anchorId: dstId, label: "To" });
      } else if (item.moveKind === "rect") {
        const srcId = `region:move-src-${item.moveId}`;
        const dstId = `region:move-dst-${item.moveId}`;
        if (anchorIds.has(srcId)) targets.push({ anchorId: srcId, label: "From" });
        if (anchorIds.has(dstId)) targets.push({ anchorId: dstId, label: "To" });
      }
    }
    if (targets.length > 0) {
      item.navTargets = targets;
    }
  }
}

function buildRegions({ ops, rowsVm, colsVm, editMap, opts }) {
  const regions = [];
  const editKeys = Array.from(editMap.keys());
  regions.push(...clusterEditsToRegions(editKeys, opts));

  for (const op of ops) {
    if (op.kind === "RectReplaced") {
      const rowRange = mapRangeToView(op.start_row, op.row_count, rowsVm.newToView);
      const colRange = mapRangeToView(op.start_col, op.col_count, colsVm.newToView);
      if (!rowRange || !colRange) continue;
      regions.push({
        id: `rect-${op.start_row}-${op.start_col}`,
        kind: "rect",
        top: rowRange.start,
        bottom: rowRange.end,
        left: colRange.start,
        right: colRange.end,
        cellCount: (rowRange.end - rowRange.start + 1) * (colRange.end - colRange.start + 1)
      });
    } else if (op.kind === "BlockMovedRect") {
      const moveId = `rect:${op.src_start_row},${op.src_start_col}+${op.src_row_count}x${op.src_col_count}->${op.dst_start_row},${op.dst_start_col}`;
      const srcRows = mapRangeToView(op.src_start_row, op.src_row_count, rowsVm.oldToView);
      const srcCols = mapRangeToView(op.src_start_col, op.src_col_count, colsVm.oldToView);
      const dstRows = mapRangeToView(op.dst_start_row, op.src_row_count, rowsVm.newToView);
      const dstCols = mapRangeToView(op.dst_start_col, op.src_col_count, colsVm.newToView);
      if (srcRows && srcCols) {
        regions.push({
          id: `move-src-${moveId}`,
          kind: "move_src",
          moveId,
          top: srcRows.start,
          bottom: srcRows.end,
          left: srcCols.start,
          right: srcCols.end,
          cellCount: (srcRows.end - srcRows.start + 1) * (srcCols.end - srcCols.start + 1)
        });
      }
      if (dstRows && dstCols) {
        regions.push({
          id: `move-dst-${moveId}`,
          kind: "move_dst",
          moveId,
          top: dstRows.start,
          bottom: dstRows.end,
          left: dstCols.start,
          right: dstCols.end,
          cellCount: (dstRows.end - dstRows.start + 1) * (dstCols.end - dstCols.start + 1)
        });
      }
    }
  }

  return regions;
}

function normalizeInterestRect(rect) {
  if (!rect) return null;
  const rowStart = rect.rowStart ?? rect.row_start;
  const rowEnd = rect.rowEnd ?? rect.row_end ?? rowStart;
  const colStart = rect.colStart ?? rect.col_start;
  const colEnd = rect.colEnd ?? rect.col_end ?? colStart;
  return {
    id: rect.id || rect.rectId || rect.key || "",
    kind: rect.kind || "rect",
    side: rect.side || "both",
    moveId: rect.moveId || rect.move_id || "",
    rowStart: Number.isFinite(rowStart) ? rowStart : 0,
    rowEnd: Number.isFinite(rowEnd) ? rowEnd : 0,
    colStart: Number.isFinite(colStart) ? colStart : 0,
    colEnd: Number.isFinite(colEnd) ? colEnd : 0
  };
}

function buildAlignedHunks(regions, anchors) {
  const anchorIds = new Set((anchors || []).map(anchor => anchor.id));
  return (regions || []).map(region => ({
    id: region.id,
    kind: region.kind,
    label: region.label,
    moveId: region.moveId || "",
    viewBounds: region.renderBounds || region,
    anchorId: anchorIds.has(`region:${region.id}`) ? `region:${region.id}` : ""
  }));
}

function buildRawHunks({ rects, oldSheet, newSheet, opts }) {
  const hunks = [];
  if (!Array.isArray(rects) || rects.length === 0) return hunks;
  const oldRows = oldSheet?.nrows || 0;
  const oldCols = oldSheet?.ncols || 0;
  const newRows = newSheet?.nrows || 0;
  const newCols = newSheet?.ncols || 0;

  for (const rect of rects) {
    const normalized = normalizeInterestRect(rect);
    if (!normalized) continue;
    const bounds = {
      top: normalized.rowStart,
      bottom: normalized.rowEnd,
      left: normalized.colStart,
      right: normalized.colEnd
    };
    let oldBounds = null;
    let newBounds = null;
    if (normalized.side === "old") {
      oldBounds = bounds;
    } else if (normalized.side === "new") {
      newBounds = bounds;
    } else {
      oldBounds = bounds;
      newBounds = bounds;
    }

    const renderOld =
      oldBounds && oldRows > 0 && oldCols > 0
        ? expandBounds(oldBounds, oldRows, oldCols, opts)
        : null;
    const renderNew =
      newBounds && newRows > 0 && newCols > 0
        ? expandBounds(newBounds, newRows, newCols, opts)
        : null;

    const labelParts = [];
    if (oldBounds) {
      const start = formatCellAddress(oldBounds.top, oldBounds.left);
      const end = formatCellAddress(oldBounds.bottom, oldBounds.right);
      labelParts.push(oldBounds.top === oldBounds.bottom && oldBounds.left === oldBounds.right ? `Old ${start}` : `Old ${start}:${end}`);
    }
    if (newBounds) {
      const start = formatCellAddress(newBounds.top, newBounds.left);
      const end = formatCellAddress(newBounds.bottom, newBounds.right);
      labelParts.push(newBounds.top === newBounds.bottom && newBounds.left === newBounds.right ? `New ${start}` : `New ${start}:${end}`);
    }

    hunks.push({
      id: normalized.id || `${normalized.kind}-${normalized.rowStart}-${normalized.colStart}`,
      kind: normalized.kind,
      moveId: normalized.moveId,
      label: labelParts.join(" -> ") || "Change hunk",
      oldBounds,
      newBounds,
      renderOld,
      renderNew
    });
  }
  return hunks;
}

function buildInterestRectsFromOps({ ops, oldSheet, newSheet, opts }) {
  const rects = [];
  const previewRows = opts.previewRows ?? 200;
  const previewCols = opts.previewCols ?? 80;
  const oldRows = oldSheet?.nrows || 0;
  const oldCols = oldSheet?.ncols || 0;
  const newRows = newSheet?.nrows || 0;
  const newCols = newSheet?.ncols || 0;

  const previewColsOld = Math.min(previewCols, oldCols);
  const previewColsNew = Math.min(previewCols, newCols);
  const previewRowsOld = Math.min(previewRows, oldRows);
  const previewRowsNew = Math.min(previewRows, newRows);

  const pushRect = (kind, side, bounds, moveId = "") => {
    if (!bounds) return;
    rects.push({
      id: `${kind}-${side}-${bounds.top}-${bounds.left}-${bounds.bottom}-${bounds.right}`,
      kind,
      side,
      moveId,
      rowStart: bounds.top,
      rowEnd: bounds.bottom,
      colStart: bounds.left,
      colEnd: bounds.right
    });
  };

  for (const op of ops || []) {
    const kind = op.kind || "";
    if (kind === "CellEdited") {
      const addr = parseCellAddress(op.addr);
      if (!addr) continue;
      pushRect("cell", "both", { top: addr.row, bottom: addr.row, left: addr.col, right: addr.col });
    } else if (kind === "RectReplaced") {
      pushRect("rect_replaced", "both", {
        top: op.start_row,
        bottom: op.start_row + op.row_count - 1,
        left: op.start_col,
        right: op.start_col + op.col_count - 1
      });
    } else if (kind === "RowAdded") {
      if (previewColsNew > 0) {
        pushRect("row_added", "new", { top: op.row_idx, bottom: op.row_idx, left: 0, right: previewColsNew - 1 });
      }
    } else if (kind === "RowRemoved") {
      if (previewColsOld > 0) {
        pushRect("row_removed", "old", { top: op.row_idx, bottom: op.row_idx, left: 0, right: previewColsOld - 1 });
      }
    } else if (kind === "RowReplaced") {
      if (previewColsOld > 0) {
        pushRect("row_replaced", "old", { top: op.row_idx, bottom: op.row_idx, left: 0, right: previewColsOld - 1 });
      }
      if (previewColsNew > 0) {
        pushRect("row_replaced", "new", { top: op.row_idx, bottom: op.row_idx, left: 0, right: previewColsNew - 1 });
      }
    } else if (kind === "ColumnAdded") {
      if (previewRowsNew > 0) {
        pushRect("col_added", "new", { top: 0, bottom: previewRowsNew - 1, left: op.col_idx, right: op.col_idx });
      }
    } else if (kind === "ColumnRemoved") {
      if (previewRowsOld > 0) {
        pushRect("col_removed", "old", { top: 0, bottom: previewRowsOld - 1, left: op.col_idx, right: op.col_idx });
      }
    } else if (kind === "BlockMovedRows") {
      const moveId = `r:${op.src_start_row}+${op.row_count}->${op.dst_start_row}`;
      if (previewColsOld > 0) {
        pushRect("move_src", "old", {
          top: op.src_start_row,
          bottom: op.src_start_row + op.row_count - 1,
          left: 0,
          right: previewColsOld - 1
        }, moveId);
      }
      if (previewColsNew > 0) {
        pushRect("move_dst", "new", {
          top: op.dst_start_row,
          bottom: op.dst_start_row + op.row_count - 1,
          left: 0,
          right: previewColsNew - 1
        }, moveId);
      }
    } else if (kind === "BlockMovedColumns") {
      const moveId = `c:${op.src_start_col}+${op.col_count}->${op.dst_start_col}`;
      if (previewRowsOld > 0) {
        pushRect("move_src", "old", {
          top: 0,
          bottom: previewRowsOld - 1,
          left: op.src_start_col,
          right: op.src_start_col + op.col_count - 1
        }, moveId);
      }
      if (previewRowsNew > 0) {
        pushRect("move_dst", "new", {
          top: 0,
          bottom: previewRowsNew - 1,
          left: op.dst_start_col,
          right: op.dst_start_col + op.col_count - 1
        }, moveId);
      }
    } else if (kind === "BlockMovedRect") {
      const moveId = `rect:${op.src_start_row},${op.src_start_col}+${op.src_row_count}x${op.src_col_count}->${op.dst_start_row},${op.dst_start_col}`;
      pushRect("move_src", "old", {
        top: op.src_start_row,
        bottom: op.src_start_row + op.src_row_count - 1,
        left: op.src_start_col,
        right: op.src_start_col + op.src_col_count - 1
      }, moveId);
      pushRect("move_dst", "new", {
        top: op.dst_start_row,
        bottom: op.dst_start_row + op.src_row_count - 1,
        left: op.dst_start_col,
        right: op.dst_start_col + op.src_col_count - 1
      }, moveId);
    } else if (kind === "DuplicateKeyCluster") {
      if (previewColsOld > 0) {
        for (const rowIdx of op.left_rows || []) {
          pushRect("row_cluster", "old", { top: rowIdx, bottom: rowIdx, left: 0, right: previewColsOld - 1 });
        }
      }
      if (previewColsNew > 0) {
        for (const rowIdx of op.right_rows || []) {
          pushRect("row_cluster", "new", { top: rowIdx, bottom: rowIdx, left: 0, right: previewColsNew - 1 });
        }
      }
    }
  }

  return rects;
}

function buildMoveLookup({ report, alignment, ops }) {
  const moveMap = new Map();
  const moves = Array.isArray(alignment?.moves) ? alignment.moves : [];
  for (const move of moves) {
    if (!move?.id) continue;
    if (move.axis === "row") {
      const start = move.src_start + 1;
      const end = move.src_start + move.count;
      const dstStart = move.dst_start + 1;
      const dstEnd = move.dst_start + move.count;
      moveMap.set(move.id, {
        axis: "row",
        src: start === end ? `Row ${start}` : `Rows ${start}-${end}`,
        dst: dstStart === dstEnd ? `Row ${dstStart}` : `Rows ${dstStart}-${dstEnd}`
      });
    } else if (move.axis === "col") {
      const start = colToLetter(move.src_start);
      const end = colToLetter(move.src_start + move.count - 1);
      const dstStart = colToLetter(move.dst_start);
      const dstEnd = colToLetter(move.dst_start + move.count - 1);
      moveMap.set(move.id, {
        axis: "col",
        src: start === end ? `Column ${start}` : `Columns ${start}-${end}`,
        dst: dstStart === dstEnd ? `Column ${dstStart}` : `Columns ${dstStart}-${dstEnd}`
      });
    }
  }

  for (const op of ops || []) {
    if (op.kind !== "BlockMovedRect") continue;
    const moveId = `rect:${op.src_start_row},${op.src_start_col}+${op.src_row_count}x${op.src_col_count}->${op.dst_start_row},${op.dst_start_col}`;
    const srcStart = formatCellAddress(op.src_start_row, op.src_start_col);
    const srcEnd = formatCellAddress(op.src_start_row + op.src_row_count - 1, op.src_start_col + op.src_col_count - 1);
    const dstStart = formatCellAddress(op.dst_start_row, op.dst_start_col);
    const dstEnd = formatCellAddress(op.dst_start_row + op.src_row_count - 1, op.dst_start_col + op.src_col_count - 1);
    moveMap.set(moveId, {
      axis: "rect",
      src: `${srcStart}:${srcEnd}`,
      dst: `${dstStart}:${dstEnd}`
    });
  }

  return moveMap;
}

function buildOpRows({ report, ops, rowsVm, colsVm }) {
  const rows = [];
  let idx = 0;

  for (const op of ops || []) {
    const kind = op.kind || "Unknown";
    const changeType = changeTypeForKind(kind);
    const group = groupForKind(kind);
    const row = {
      id: `op-${idx++}`,
      kind,
      changeType,
      group,
      location: "",
      detail: "",
      viewRow: null,
      viewCol: null,
      navTargets: []
    };

    if (kind === "CellEdited") {
      const addr = parseCellAddress(op.addr);
      if (addr) {
        row.location = formatCellAddress(addr.row, addr.col);
        row.viewRow = mapIndexToView(addr.row, rowsVm.newToView);
        row.viewCol = mapIndexToView(addr.col, colsVm.newToView);
      }
      const fromVal = op.from ? formatValue(report, op.from.value) : "";
      const toVal = op.to ? formatValue(report, op.to.value) : "";
      if (fromVal || toVal) {
        row.detail = `${fromVal || "(empty)"} -> ${toVal || "(empty)"}`;
      }
    } else if (kind === "RowAdded" || kind === "RowRemoved" || kind === "RowReplaced") {
      const rowNum = op.row_idx + 1;
      row.location = `Row ${rowNum}`;
      const map = kind === "RowRemoved" ? rowsVm.oldToView : rowsVm.newToView;
      row.viewRow = mapIndexToView(op.row_idx, map);
      row.viewCol = 0;
    } else if (kind === "ColumnAdded" || kind === "ColumnRemoved") {
      const colLetter = colToLetter(op.col_idx);
      row.location = `Column ${colLetter}`;
      const map = kind === "ColumnRemoved" ? colsVm.oldToView : colsVm.newToView;
      row.viewCol = mapIndexToView(op.col_idx, map);
      row.viewRow = 0;
    } else if (kind === "BlockMovedRows") {
      const start = op.src_start_row + 1;
      const end = op.src_start_row + op.row_count;
      row.location = start === end ? `Row ${start}` : `Rows ${start}-${end}`;
      row.detail = `Moved to row ${op.dst_start_row + 1}`;
      const srcView = mapIndexToView(op.src_start_row, rowsVm.oldToView);
      const dstView = mapIndexToView(op.dst_start_row, rowsVm.newToView);
      if (srcView !== null && srcView !== undefined) {
        row.navTargets.push({ viewRow: srcView, viewCol: 0, label: "From" });
      }
      if (dstView !== null && dstView !== undefined) {
        row.navTargets.push({ viewRow: dstView, viewCol: 0, label: "To" });
      }
    } else if (kind === "BlockMovedColumns") {
      const start = colToLetter(op.src_start_col);
      const end = colToLetter(op.src_start_col + op.col_count - 1);
      row.location = start === end ? `Column ${start}` : `Columns ${start}-${end}`;
      row.detail = `Moved to column ${colToLetter(op.dst_start_col)}`;
      const srcView = mapIndexToView(op.src_start_col, colsVm.oldToView);
      const dstView = mapIndexToView(op.dst_start_col, colsVm.newToView);
      if (srcView !== null && srcView !== undefined) {
        row.navTargets.push({ viewRow: 0, viewCol: srcView, label: "From" });
      }
      if (dstView !== null && dstView !== undefined) {
        row.navTargets.push({ viewRow: 0, viewCol: dstView, label: "To" });
      }
    } else if (kind === "BlockMovedRect") {
      const srcStart = formatCellAddress(op.src_start_row, op.src_start_col);
      const srcEnd = formatCellAddress(op.src_start_row + op.src_row_count - 1, op.src_start_col + op.src_col_count - 1);
      const dstStart = formatCellAddress(op.dst_start_row, op.dst_start_col);
      const dstEnd = formatCellAddress(op.dst_start_row + op.src_row_count - 1, op.dst_start_col + op.src_col_count - 1);
      row.location = `${srcStart}:${srcEnd}`;
      row.detail = `Moved to ${dstStart}:${dstEnd}`;
      const srcRows = mapRangeToView(op.src_start_row, op.src_row_count, rowsVm.oldToView);
      const srcCols = mapRangeToView(op.src_start_col, op.src_col_count, colsVm.oldToView);
      const dstRows = mapRangeToView(op.dst_start_row, op.src_row_count, rowsVm.newToView);
      const dstCols = mapRangeToView(op.dst_start_col, op.src_col_count, colsVm.newToView);
      if (srcRows && srcCols) {
        row.navTargets.push({ viewRow: srcRows.start, viewCol: srcCols.start, label: "From" });
      }
      if (dstRows && dstCols) {
        row.navTargets.push({ viewRow: dstRows.start, viewCol: dstCols.start, label: "To" });
      }
    } else if (kind === "RectReplaced") {
      const start = formatCellAddress(op.start_row, op.start_col);
      const end = formatCellAddress(op.start_row + op.row_count - 1, op.start_col + op.col_count - 1);
      row.location = `${start}:${end}`;
      row.detail = "Region replaced";
      const viewRow = mapIndexToView(op.start_row, rowsVm.newToView);
      const viewCol = mapIndexToView(op.start_col, colsVm.newToView);
      row.viewRow = viewRow;
      row.viewCol = viewCol;
    } else if (kind.startsWith("Sheet")) {
      if (kind === "SheetRenamed") {
        const fromName = resolveString(report, op.from);
        const toName = resolveString(report, op.to ?? op.sheet);
        row.location = "Sheet renamed";
        row.detail = `${fromName} -> ${toName}`;
      } else {
        row.location = kind.replace(/([A-Z])/g, " $1").trim();
      }
    } else if (kind.startsWith("Chart")) {
      const name = resolveString(report, op.name);
      row.location = `Chart: ${name}`;
    } else if (kind.startsWith("NamedRange")) {
      const name = resolveString(report, op.name);
      row.location = `Named Range: ${name}`;
    } else if (kind.startsWith("Vba")) {
      const name = resolveString(report, op.name);
      row.location = `VBA Module: ${name}`;
    } else if (kind.startsWith("Query")) {
      const name = resolveString(report, op.name ?? op.from ?? op.to);
      row.location = `Query: ${name}`;
      if (op.semantic_detail?.step_diffs?.length) row.detail = "Step diffs";
    }

    rows.push(row);
  }

  return rows;
}

function labelRegion(region, rowsVm, colsVm) {
  const startRow = viewIndexForSide(rowsVm, region.top, "new");
  const endRow = viewIndexForSide(rowsVm, region.bottom, "new");
  const startCol = viewIndexForSide(colsVm, region.left, "new");
  const endCol = viewIndexForSide(colsVm, region.right, "new");
  const startAddr = formatCellAddress(startRow, startCol);
  const endAddr = formatCellAddress(endRow, endCol);
  if (region.kind === "rect") {
    return `Region ${startAddr}:${endAddr}`;
  }
  if (startAddr === endAddr) return `Cell ${startAddr}`;
  return `Cells ${startAddr}:${endAddr}`;
}

function buildSheetViewModel({ report, sheetName, ops, oldSheet, newSheet, alignment, interestRects, opts }) {
  const oldRows = oldSheet?.nrows || 0;
  const oldCols = oldSheet?.ncols || 0;
  const newRows = newSheet?.nrows || 0;
  const newCols = newSheet?.ncols || 0;

  const rowEntries = Array.isArray(alignment?.rows) ? alignment.rows : [];
  const colEntries = Array.isArray(alignment?.cols) ? alignment.cols : [];
  const rowsVm = makeAxisVm(rowEntries, oldRows, newRows);
  const colsVm = makeAxisVm(colEntries, oldCols, newCols);

  let oldCells = null;
  let newCells = null;

  function ensureCellMaps() {
    if (!oldCells) oldCells = makeCellMap(oldSheet);
    if (!newCells) newCells = makeCellMap(newSheet);
  }
  const editMap = makeEditMap(report, ops, rowsVm, colsVm, opts);

  const baseRegions = buildRegions({ ops, rowsVm, colsVm, editMap, opts });
  for (const region of baseRegions) {
    region.label = labelRegion(region, rowsVm, colsVm);
  }

  const items = buildChangeItems({ report, ops, rowsVm, colsVm, alignment, regions: baseRegions });
  const counts = buildSheetCounts(ops);
  const severity = maxSeverityForOps(ops);

  let sheetState = "";
  let renameFrom = "";
  for (const op of ops) {
    if (op.kind === "SheetRemoved") sheetState = "removed";
    else if (op.kind === "SheetAdded" && sheetState !== "removed") sheetState = "added";
    else if (op.kind === "SheetRenamed" && !sheetState) {
      sheetState = "renamed";
      renameFrom = resolveString(report, op.from);
    }
  }

  const hasStructural = ops.some(op => {
    const kind = op.kind || "";
    return kind.startsWith("Row") || kind.startsWith("Column") || kind.startsWith("Block") || kind === "RectReplaced";
  });
  const hasMoves = ops.some(op => (op.kind || "").includes("Moved"));
  const hasGridOps = ops.some(op => isGridKind(op.kind || ""));
  const moveLookup = buildMoveLookup({ report, alignment, ops });

  const kindOrder = { move_src: 0, move_dst: 0, rect: 1, cell: 2 };
  baseRegions.sort((a, b) => {
    if (a.top !== b.top) return a.top - b.top;
    if (a.left !== b.left) return a.left - b.left;
    const ak = kindOrder[a.kind] ?? 99;
    const bk = kindOrder[b.kind] ?? 99;
    if (ak !== bk) return ak - bk;
    return String(a.id).localeCompare(String(b.id));
  });

  const rowsCount = rowsVm.entries.length;
  const colsCount = colsVm.entries.length;
  for (const region of baseRegions) {
    if (rowsCount > 0 && colsCount > 0) {
      region.renderBounds = expandBounds(region, rowsCount, colsCount, opts);
    }
  }

  const preview = {
    truncatedOld: Boolean(oldSheet?.truncated),
    truncatedNew: Boolean(newSheet?.truncated)
  };
  if (oldSheet?.note || newSheet?.note) {
    preview.note = oldSheet?.note || newSheet?.note;
  }

  let status = { kind: "ok" };
  if (!alignment || alignment.skipped) {
    const message = alignment?.skip_reason
      ? alignment.skip_reason
      : "Grid preview skipped because the aligned view is too large or inconsistent.";
    status = alignment?.skipped
      ? { kind: "skipped", message }
      : { kind: "missing", message: "Alignment data is missing for this sheet." };
  } else if (rowsCount === 0 || colsCount === 0) {
    status = { kind: "missing", message: "Sheet snapshots are missing for this sheet." };
  } else if (preview.truncatedOld || preview.truncatedNew) {
    status = {
      kind: "partial",
      message: preview.note || "Preview limited for performance; edited cells remain exact."
    };
  }

  const anchors = buildChangeAnchors({ sheetName, status, items, regions: baseRegions, rowsVm, colsVm });
  attachNavTargets(items, anchors);

  const regionsToRender =
    status.kind === "ok" || status.kind === "partial"
      ? baseRegions.map(region => region.id)
      : [];

  const opRows = buildOpRows({ report, ops, rowsVm, colsVm });
  const nonGridOps = opRows.filter(row => row.group === "other");
  const interestList = Array.isArray(interestRects) && interestRects.length
    ? interestRects
    : buildInterestRectsFromOps({ ops, oldSheet, newSheet, opts });
  const rawHunks = buildRawHunks({ rects: interestList, oldSheet, newSheet, opts });
  const alignedHunks = buildAlignedHunks(baseRegions, anchors);
  const useAlignedHunks = status.kind === "ok" || status.kind === "partial";
  const hunks = useAlignedHunks ? alignedHunks : rawHunks;
  const hunkMode = useAlignedHunks ? "aligned" : "raw";

  return {
    name: sheetName,
    sheetState,
    renameFrom,
    counts,
    severity,
    flags: { hasStructural, hasMoves, hasGridOps },
    axis: { rows: rowsVm, cols: colsVm },
    preview,
    moveLookup,
    ensureCellIndex: () => ensureCellMaps(),
    cellAt: (viewRow, viewCol) => {
      ensureCellMaps();
      return buildCellVm(viewRow, viewCol, rowsVm, colsVm, oldCells, newCells, editMap);
    },
    cellAtRaw: (side, row, col) => {
      ensureCellMaps();
      const map = side === "old" ? oldCells : newCells;
      if (!map) return null;
      return map.get(row * CELL_KEY_STRIDE + col) || null;
    },
    changes: { items, regions: baseRegions, anchors },
    ops: opRows,
    nonGridOps,
    hunks,
    hunkMode,
    renderPlan: {
      regionsToRender,
      status,
      contextRows: opts.contextRows,
      contextCols: opts.contextCols,
      maxVisualCells: opts.maxVisualCells
    },
    opCount: ops.length
  };
}

function buildOtherItems(report, ops, prefix) {
  const items = [];
  for (const op of ops) {
    const kind = op.kind || prefix;
    const name = op.name !== undefined ? resolveString(report, op.name) : "";
    let label = "";
    let detail = "";
    let oldValue = "";
    let newValue = "";
    if (kind.startsWith("Query")) {
      label = `Query: ${name}`;
      if (op.semantic_detail?.step_diffs?.length) {
        detail = "Step diffs";
      }
      if (kind === "QueryRenamed") {
        const fromName = resolveString(report, op.from);
        const toName = resolveString(report, op.to);
        oldValue = fromName;
        newValue = toName;
      } else if (kind === "QueryMetadataChanged") {
        oldValue = op.old != null ? resolveString(report, op.old) : "<none>";
        newValue = op.new != null ? resolveString(report, op.new) : "<none>";
      }
    } else if (kind.startsWith("Table")) {
      label = `Table: ${name}`;
    } else if (kind.startsWith("ModelColumn") || kind === "CalculatedColumnDefinitionChanged") {
      const columnLabel = formatColumnRef(report, op.table, op.name);
      label =
        kind === "CalculatedColumnDefinitionChanged"
          ? `Calculated Column: ${columnLabel}`
          : `Column: ${columnLabel}`;
      if (kind === "ModelColumnTypeChanged") {
        const oldType = op.old_type != null ? resolveString(report, op.old_type) : "<none>";
        const newType = op.new_type != null ? resolveString(report, op.new_type) : "<none>";
        detail = `Type: ${oldType} -> ${newType}`;
        oldValue = oldType;
        newValue = newType;
      } else if (kind === "ModelColumnPropertyChanged") {
        const oldVal = op.old != null ? resolveString(report, op.old) : "<none>";
        const newVal = op.new != null ? resolveString(report, op.new) : "<none>";
        detail = `${formatFieldLabel(op.field)}: ${oldVal} -> ${newVal}`;
        oldValue = oldVal;
        newValue = newVal;
      } else if (kind === "ModelColumnAdded" && op.data_type != null) {
        detail = `Type: ${resolveString(report, op.data_type)}`;
      } else if (kind === "CalculatedColumnDefinitionChanged") {
        const kindLabel = formatChangeKind(op.change_kind);
        detail = kindLabel ? `Definition changed (${kindLabel})` : "Definition changed";
      }
    } else if (kind.startsWith("Relationship")) {
      label = `Relationship: ${formatRelationshipRef(report, op)}`;
      if (kind === "RelationshipPropertyChanged") {
        const oldVal = op.old != null ? resolveString(report, op.old) : "<none>";
        const newVal = op.new != null ? resolveString(report, op.new) : "<none>";
        detail = `${formatFieldLabel(op.field)}: ${oldVal} -> ${newVal}`;
        oldValue = oldVal;
        newValue = newVal;
      }
    } else if (kind.startsWith("Measure")) {
      label = `Measure: ${name}`;
      if (kind === "MeasureDefinitionChanged") {
        const kindLabel = formatChangeKind(op.change_kind);
        detail = kindLabel ? `Definition changed (${kindLabel})` : "Definition changed";
      }
    } else if (kind.startsWith("NamedRange")) {
      label = `Named Range: ${name}`;
      if (kind === "NamedRangeChanged") {
        oldValue = resolveString(report, op.old_ref);
        newValue = resolveString(report, op.new_ref);
      }
    } else if (kind.startsWith("Chart")) {
      label = `Chart: ${name}`;
    } else if (kind.startsWith("Vba")) {
      label = `VBA Module: ${name}`;
    } else {
      label = name ? `${kind}: ${name}` : String(kind);
    }
    const changeType = kind.includes("Added") ? "added" : kind.includes("Removed") ? "removed" : "modified";
    items.push({
      id: `${kind}-${name}`,
      changeType,
      severity: opSeverity(op),
      label,
      detail,
      kind,
      name,
      oldValue,
      newValue,
      raw: op
    });
  }
  return items;
}

function computeWorkbookAnalysis(report, sheetVms, other, noiseFilters) {
  const ops = Array.isArray(report?.ops) ? report.ops : [];
  const categories = new Map();
  const severity = { high: 0, medium: 0, low: 0 };

  for (const op of ops) {
    const kind = op?.kind || "";
    const category = opCategoryForKind(kind);
    const sev = opSeverity(op);
    const changeType = changeTypeForKind(kind);

    let entry = categories.get(category);
    if (!entry) {
      entry = {
        category,
        opCount: 0,
        counts: { added: 0, removed: 0, modified: 0, moved: 0 },
        severity: { high: 0, medium: 0, low: 0 }
      };
      categories.set(category, entry);
    }

    entry.opCount += 1;
    entry.counts[changeType] += 1;
    entry.severity[sev] += 1;
    severity[sev] += 1;
  }

  const categoryRows = CATEGORY_ORDER.map(category => {
    return (
      categories.get(category) || {
        category,
        opCount: 0,
        counts: { added: 0, removed: 0, modified: 0, moved: 0 },
        severity: { high: 0, medium: 0, low: 0 }
      }
    );
  });

  const warnings = Array.isArray(report?.warnings) ? report.warnings : [];
  const complete = report?.complete !== false;
  const warningCount = warnings.length;
  const incomplete = !complete || warningCount > 0;

  const topSheets = (Array.isArray(sheetVms) ? sheetVms : [])
    .slice(0, 6)
    .map(sheet => ({
      kind: "sheet",
      name: sheet.name,
      severity: sheet.severity || "low",
      opCount: sheet.opCount || 0,
      counts: sheet.counts || { added: 0, removed: 0, modified: 0, moved: 0 }
    }));

  const artifacts = [];
  const otherGroups = [
    ...(other?.queries || []),
    ...(other?.model || []),
    ...(other?.vba || []),
    ...(other?.namedRanges || []),
    ...(other?.charts || [])
  ];
  for (const item of otherGroups) {
    artifacts.push({
      kind: item.kind || "Other",
      label: item.label || item.name || "",
      severity: item.severity || opSeverity(item.raw),
      changeType: item.changeType || "modified"
    });
  }

  artifacts.sort((a, b) => {
    const sev = severityRank(b.severity) - severityRank(a.severity);
    if (sev) return sev;
    return String(a.label).localeCompare(String(b.label));
  });

  const topArtifacts = artifacts.slice(0, 6);

  return {
    opCount: ops.length,
    complete,
    warningCount,
    incomplete,
    noiseFilters: { ...DEFAULT_NOISE_FILTERS, ...normalizeNoiseFilters(noiseFilters) },
    severity,
    categories: categoryRows,
    topSheets,
    topArtifacts
  };
}

export function buildWorkbookViewModel(payloadOrReport, opts = {}) {
  const { report, sheets, alignments, interestRects } = normalizePayload(payloadOrReport);
  const options = { ...DEFAULT_OPTS, ...opts };
  const noiseFilters = normalizeNoiseFilters(options.noiseFilters || options.noise_filters);
  const filteredOps = applyNoiseFiltersToOps(report?.ops, noiseFilters);
  const viewReport = filteredOps === report?.ops ? report : { ...report, ops: filteredOps };
  const { sheetOps, renameMap, vbaOps, namedRangeOps, chartOps, queryOps, modelOps, counts } =
    categorizeOps(viewReport);

  const oldLookup = buildSheetLookup(sheets.oldSheets);
  const newLookup = buildSheetLookup(sheets.newSheets);
  const alignmentLookup = buildAlignmentLookup(alignments);
  const interestLookup = buildInterestRectLookup(interestRects);

  const sheetVms = [];
  for (const [sheetName, ops] of sheetOps.entries()) {
    const sheetVm = buildSheetViewModel({
      report: viewReport,
      sheetName,
      ops,
      oldSheet: oldLookup.get(sheetName) || oldLookup.get(renameMap.get(sheetName)) || null,
      newSheet: newLookup.get(sheetName) || null,
      alignment: alignmentLookup.get(sheetName) || alignmentLookup.get(renameMap.get(sheetName)) || null,
      interestRects: interestLookup.get(sheetName) || interestLookup.get(renameMap.get(sheetName)) || [],
      opts: options
    });
    sheetVms.push(sheetVm);
  }

  sheetVms.sort((a, b) => {
    const sev = severityRank(b.severity) - severityRank(a.severity);
    if (sev) return sev;
    if (b.opCount !== a.opCount) return b.opCount - a.opCount;
    return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
  });

  const other = {
    vba: buildOtherItems(viewReport, vbaOps, "Vba"),
    namedRanges: buildOtherItems(viewReport, namedRangeOps, "NamedRange"),
    charts: buildOtherItems(viewReport, chartOps, "Chart"),
    queries: buildOtherItems(viewReport, queryOps, "Query"),
    model: buildOtherItems(viewReport, modelOps, "Model")
  };

  const analysis = computeWorkbookAnalysis(viewReport, sheetVms, other, noiseFilters);

  return {
    report: viewReport,
    warnings: Array.isArray(viewReport?.warnings) ? viewReport.warnings : [],
    counts,
    sheets: sheetVms,
    other,
    analysis
  };
}
