const CELL_KEY_STRIDE = 16384;

const DEFAULT_OPTS = {
  contextRows: 1,
  contextCols: 1,
  maxCellsPerRegion: 200,
  maxVisualCells: 5000,
  mergeGap: 1,
  ignoreBlankToBlank: true
};

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
  const sheets = {
    oldSheets: normalizeSheetList(rawSheets?.old),
    newSheets: normalizeSheetList(rawSheets?.new)
  };
  return { report, sheets, alignments };
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

function categorizeOps(report) {
  const ops = Array.isArray(report?.ops) ? report.ops : [];
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
    if (ignoreBlank && !fromValue && !toValue && !fromFormula && !toFormula) {
      continue;
    }
    editMap.set(key, { fromValue, toValue, fromFormula, toFormula });
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
    "SheetRemoved"
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
    status?.kind === "ok" &&
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

function buildSheetViewModel({ report, sheetName, ops, oldSheet, newSheet, alignment, opts }) {
  const oldRows = oldSheet?.nrows || 0;
  const oldCols = oldSheet?.ncols || 0;
  const newRows = newSheet?.nrows || 0;
  const newCols = newSheet?.ncols || 0;

  const rowEntries = Array.isArray(alignment?.rows) ? alignment.rows : [];
  const colEntries = Array.isArray(alignment?.cols) ? alignment.cols : [];
  const rowsVm = makeAxisVm(rowEntries, oldRows, newRows);
  const colsVm = makeAxisVm(colEntries, oldCols, newCols);

  const oldCells = makeCellMap(oldSheet);
  const newCells = makeCellMap(newSheet);
  const editMap = makeEditMap(report, ops, rowsVm, colsVm, opts);

  const baseRegions = buildRegions({ ops, rowsVm, colsVm, editMap, opts });
  for (const region of baseRegions) {
    region.label = labelRegion(region, rowsVm, colsVm);
  }

  const items = buildChangeItems({ report, ops, rowsVm, colsVm, alignment, regions: baseRegions });

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

  let status = { kind: "ok" };
  if (!alignment || alignment.skipped) {
    status = alignment?.skipped
      ? { kind: "skipped", message: "Grid preview skipped because the aligned view is too large or inconsistent." }
      : { kind: "missing", message: "Alignment data is missing for this sheet." };
  } else if (rowsCount === 0 || colsCount === 0) {
    status = { kind: "missing", message: "Sheet snapshots are missing for this sheet." };
  }

  const anchors = buildChangeAnchors({ sheetName, status, items, regions: baseRegions, rowsVm, colsVm });
  attachNavTargets(items, anchors);

  const regionsToRender = status.kind === "ok" ? baseRegions.map(region => region.id) : [];

  return {
    name: sheetName,
    axis: { rows: rowsVm, cols: colsVm },
    cellAt: (viewRow, viewCol) => buildCellVm(viewRow, viewCol, rowsVm, colsVm, oldCells, newCells, editMap),
    changes: { items, regions: baseRegions, anchors },
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
    if (kind.startsWith("Query")) {
      label = `Query: ${name}`;
      if (op.semantic_detail?.step_diffs?.length) {
        detail = "Step diffs";
      }
    } else if (kind.startsWith("Measure")) {
      label = `Measure: ${name}`;
    } else if (kind.startsWith("NamedRange")) {
      label = `Named Range: ${name}`;
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
      label,
      detail
    });
  }
  return items;
}

export function buildWorkbookViewModel(payloadOrReport, opts = {}) {
  const { report, sheets, alignments } = normalizePayload(payloadOrReport);
  const options = { ...DEFAULT_OPTS, ...opts };
  const { sheetOps, vbaOps, namedRangeOps, chartOps, queryOps, measureOps, counts } = categorizeOps(report);

  const oldLookup = buildSheetLookup(sheets.oldSheets);
  const newLookup = buildSheetLookup(sheets.newSheets);
  const alignmentLookup = buildAlignmentLookup(alignments);

  const sheetVms = [];
  for (const [sheetName, ops] of sheetOps.entries()) {
    const sheetVm = buildSheetViewModel({
      report,
      sheetName,
      ops,
      oldSheet: oldLookup.get(sheetName) || null,
      newSheet: newLookup.get(sheetName) || null,
      alignment: alignmentLookup.get(sheetName) || null,
      opts: options
    });
    sheetVms.push(sheetVm);
  }

  sheetVms.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));

  return {
    report,
    warnings: Array.isArray(report?.warnings) ? report.warnings : [],
    counts,
    sheets: sheetVms,
    other: {
      vba: buildOtherItems(report, vbaOps, "Vba"),
      namedRanges: buildOtherItems(report, namedRangeOps, "NamedRange"),
      charts: buildOtherItems(report, chartOps, "Chart"),
      queries: buildOtherItems(report, queryOps, "Query"),
      measures: buildOtherItems(report, measureOps, "Measure")
    }
  };
}
