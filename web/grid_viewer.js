import { GRID_METRICS } from "./grid_metrics.js";
import { paintGrid } from "./grid_painter.js";
import { readGridTheme } from "./grid_theme.js";

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
  return `${colToLetter(col)}${row + 1}`;
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function createEl(tag, className, text) {
  const el = document.createElement(tag);
  if (className) el.className = className;
  if (text !== undefined) el.textContent = text;
  return el;
}

function extractSideDetails(cell, side) {
  if (!cell) return { value: "", formula: "" };
  const edit = cell.edit;
  let value = side === "old" ? edit?.fromValue : edit?.toValue;
  let formula = side === "old" ? edit?.fromFormula : edit?.toFormula;
  if (!value && !formula) {
    const payload = side === "old" ? cell.old : cell.new;
    value = payload?.cell?.value ?? "";
    formula = payload?.cell?.formula ?? "";
  }
  return { value: String(value || ""), formula: String(formula || "") };
}

function buildCellSummary(cell, viewRow, viewCol) {
  const summary = {
    viewAddress: formatCellAddress(viewRow, viewCol),
    diffKind: cell?.diffKind || "empty",
    moveId: cell?.moveId || "",
    moveRole: cell?.moveRole || "",
    formulaDiff: cell?.edit?.formulaDiff || "",
    oldAddress: "",
    newAddress: "",
    old: extractSideDetails(cell, "old"),
    fresh: extractSideDetails(cell, "new")
  };
  if (cell?.old) {
    summary.oldAddress = formatCellAddress(cell.old.row, cell.old.col);
  }
  if (cell?.new) {
    summary.newAddress = formatCellAddress(cell.new.row, cell.new.col);
  }
  return summary;
}

export function mountSheetGridViewer({ mountEl, sheetVm, opts = {} }) {
  const theme = readGridTheme(document.documentElement);
  const metrics = { ...GRID_METRICS };
  const regionLookup = new Map((sheetVm.changes?.regions || []).map(region => [region.id, region]));
  const anchorList = Array.isArray(sheetVm.changes?.anchors) ? sheetVm.changes.anchors : [];
  const gridAnchors = anchorList
    .filter(anchor => anchor?.target?.kind === "grid")
    .map(anchor => ({
      id: anchor.id,
      viewRow: anchor.target.viewRow,
      viewCol: anchor.target.viewCol,
      regionId: anchor.target.regionId,
      moveId: anchor.target.moveId,
      group: anchor.group
    }));
  const anchorIndexById = new Map(gridAnchors.map((anchor, idx) => [anchor.id, idx]));

  let initialAnchorIndex = 0;
  if (typeof opts.initialAnchor === "string") {
    const idx = anchorIndexById.get(opts.initialAnchor);
    if (Number.isFinite(idx)) initialAnchorIndex = idx;
  } else if (Number.isFinite(opts.initialAnchor)) {
    initialAnchorIndex = opts.initialAnchor;
  }
  if (gridAnchors.length === 0) {
    initialAnchorIndex = -1;
  }

  const state = {
    mode: opts.initialMode === "unified" ? "unified" : "side_by_side",
    selected: null,
    hover: null,
    hoverMoveId: null,
    anchorIndex: initialAnchorIndex,
    pinned: false,
    contentMode: opts.displayOptions?.contentMode || "values",
    focusRows: Boolean(opts.displayOptions?.focusRows),
    focusCols: Boolean(opts.displayOptions?.focusCols),
    rowMap: null,
    colMap: null,
    rowLookup: null,
    colLookup: null,
    rowCount: sheetVm.axis.rows.entries.length,
    colCount: sheetVm.axis.cols.entries.length,
    flash: null
  };

  mountEl.innerHTML = "";

  const root = createEl("div", "grid-viewer");
  root.tabIndex = 0;

  const toolbar = createEl("div", "grid-toolbar");
  const toolbarGroup = createEl("div", "grid-toolbar-group");
  const sideBtn = createEl("button", "grid-mode-btn", "Side-by-side");
  sideBtn.type = "button";
  sideBtn.dataset.mode = "side_by_side";
  const unifiedBtn = createEl("button", "grid-mode-btn", "Unified");
  unifiedBtn.type = "button";
  unifiedBtn.dataset.mode = "unified";
  toolbarGroup.append(sideBtn, unifiedBtn);
  const toolbarHint = createEl("div", "grid-toolbar-hint", "Arrow keys: move, N/P: next/prev change");
  toolbar.append(toolbarGroup, toolbarHint);

  const body = createEl("div", "grid-viewer-body");
  const canvasWrap = createEl("div", "grid-canvas-wrap");
  const scroll = createEl("div", "grid-scroll");
  const spacer = createEl("div", "grid-scroll-spacer");
  const canvas = createEl("canvas", "grid-canvas");
  canvas.setAttribute("role", "presentation");
  scroll.append(spacer, canvas);
  const tooltip = createEl("div", "grid-tooltip");
  canvasWrap.append(scroll, tooltip);

  const inspector = createEl("div", "grid-inspector");
  const inspectorHeader = createEl("div", "grid-inspector-header");
  const inspectorTitle = createEl("div", "grid-inspector-title", "Inspector");
  const inspectorToggle = createEl("button", "grid-inspector-toggle", "Collapse");
  inspectorToggle.type = "button";
  inspectorHeader.append(inspectorTitle, inspectorToggle);
  const inspectorEmpty = createEl("div", "grid-inspector-empty", "Select a cell to inspect.");
  const inspectorContent = createEl("div", "grid-inspector-content");
  inspector.append(inspectorHeader, inspectorEmpty, inspectorContent);

  body.append(canvasWrap, inspector);
  root.append(toolbar, body);
  mountEl.append(root);

  const ctx = canvas.getContext("2d");
  let rafId = null;
  const perfStart = performance.now();
  let firstPaint = true;

  let contentWidth = 0;
  let contentHeight = 0;

  function buildIndexLookup(map, total) {
    if (!Array.isArray(map)) return null;
    const lookup = new Array(total).fill(null);
    for (let i = 0; i < map.length; i++) {
      const viewIndex = map[i];
      if (Number.isFinite(viewIndex)) {
        lookup[viewIndex] = i;
      }
    }
    return lookup;
  }

  function computeFocusMap(axis, context) {
    const entries = axis === "row" ? sheetVm.axis.rows.entries : sheetVm.axis.cols.entries;
    const maxIndex = entries.length - 1;
    if (maxIndex < 0) return null;
    const changed = new Set();
    for (let i = 0; i < entries.length; i++) {
      const entry = entries[i];
      if (entry && entry.kind && entry.kind !== "match") {
        changed.add(i);
      }
    }
    for (const item of sheetVm.changes?.items || []) {
      if (axis === "row" && item.group === "rows" && Number.isFinite(item.viewStart) && Number.isFinite(item.viewEnd)) {
        for (let r = item.viewStart; r <= item.viewEnd; r++) {
          changed.add(r);
        }
      }
      if (axis === "col" && item.group === "cols" && Number.isFinite(item.viewStart) && Number.isFinite(item.viewEnd)) {
        for (let c = item.viewStart; c <= item.viewEnd; c++) {
          changed.add(c);
        }
      }
    }
    for (const region of sheetVm.changes?.regions || []) {
      if (axis === "row") {
        for (let r = region.top; r <= region.bottom; r++) {
          changed.add(r);
        }
      } else {
        for (let c = region.left; c <= region.right; c++) {
          changed.add(c);
        }
      }
    }
    if (changed.size === 0) return null;
    const expanded = new Set();
    const ctx = Math.max(0, context || 0);
    for (const idx of changed) {
      for (let offset = -ctx; offset <= ctx; offset++) {
        const next = idx + offset;
        if (next >= 0 && next <= maxIndex) {
          expanded.add(next);
        }
      }
    }
    if (expanded.size === 0) return null;
    return Array.from(expanded).sort((a, b) => a - b);
  }

  function updateContentSize() {
    contentWidth = metrics.rowHeaderWidth + state.colCount * metrics.colWidth;
    contentHeight = metrics.colHeaderHeight + state.rowCount * metrics.rowHeight;
    spacer.style.width = `${contentWidth}px`;
    spacer.style.height = `${contentHeight}px`;
  }

  function updateDisplayMaps() {
    const rows = sheetVm.axis.rows.entries.length;
    const cols = sheetVm.axis.cols.entries.length;
    if (state.focusRows) {
      const map = computeFocusMap("row", sheetVm.renderPlan?.contextRows || 0);
      state.rowMap = map && map.length ? map : null;
    } else {
      state.rowMap = null;
    }
    if (state.focusCols) {
      const map = computeFocusMap("col", sheetVm.renderPlan?.contextCols || 0);
      state.colMap = map && map.length ? map : null;
    } else {
      state.colMap = null;
    }
    state.rowLookup = buildIndexLookup(state.rowMap, rows);
    state.colLookup = buildIndexLookup(state.colMap, cols);
    state.rowCount = state.rowMap ? state.rowMap.length : rows;
    state.colCount = state.colMap ? state.colMap.length : cols;
    updateContentSize();
  }

  function visibleToViewRow(visibleRow) {
    if (state.rowMap) return state.rowMap[visibleRow];
    return visibleRow;
  }

  function visibleToViewCol(visibleCol) {
    if (state.colMap) return state.colMap[visibleCol];
    return visibleCol;
  }

  function viewToVisibleRow(viewRow) {
    if (state.rowLookup) return state.rowLookup[viewRow];
    return viewRow;
  }

  function viewToVisibleCol(viewCol) {
    if (state.colLookup) return state.colLookup[viewCol];
    return viewCol;
  }

  function ensureSelectionVisible() {
    if (!state.selected) return;
    const row = viewToVisibleRow(state.selected.viewRow);
    const col = viewToVisibleCol(state.selected.viewCol);
    if (row === null || row === undefined || col === null || col === undefined) {
      const fallbackRow = state.rowMap ? state.rowMap[0] : 0;
      const fallbackCol = state.colMap ? state.colMap[0] : 0;
      state.selected = { viewRow: fallbackRow, viewCol: fallbackCol };
    }
  }

  updateDisplayMaps();

  function updateModeButtons() {
    const active = state.mode;
    for (const btn of [sideBtn, unifiedBtn]) {
      btn.classList.toggle("active", btn.dataset.mode === active);
    }
  }

  function getLayout() {
    const width = scroll.clientWidth;
    const height = scroll.clientHeight;
    const paneCount = state.mode === "side_by_side" ? 2 : 1;
    const paneWidth =
      paneCount === 2
        ? Math.max(0, Math.floor((width - metrics.rowHeaderWidth - metrics.paneGap) / 2))
        : Math.max(0, width - metrics.rowHeaderWidth);
    return {
      width,
      height,
      paneCount,
      paneWidth,
      rowHeaderWidth: metrics.rowHeaderWidth,
      colHeaderHeight: metrics.colHeaderHeight,
      paneGap: metrics.paneGap
    };
  }

  function resizeCanvas() {
    const { width, height } = getLayout();
    const dpr = window.devicePixelRatio || 1;
    const targetWidth = Math.max(1, Math.floor(width));
    const targetHeight = Math.max(1, Math.floor(height));
    if (canvas.width !== targetWidth * dpr || canvas.height !== targetHeight * dpr) {
      canvas.width = targetWidth * dpr;
      canvas.height = targetHeight * dpr;
      canvas.style.width = `${targetWidth}px`;
      canvas.style.height = `${targetHeight}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }
    return { width: targetWidth, height: targetHeight };
  }

  function schedulePaint() {
    if (rafId !== null) return;
    rafId = requestAnimationFrame(() => {
      rafId = null;
      paint();
    });
  }

  function paint() {
    const size = resizeCanvas();
    const now = performance.now();
    let flashModel = null;
    if (state.flash) {
      const elapsed = now - state.flash.start;
      const duration = 900;
      if (elapsed >= duration) {
        state.flash = null;
      } else {
        flashModel = {
          ...state.flash,
          alpha: Math.max(0, 1 - elapsed / duration)
        };
      }
    }
    paintGrid(ctx, {
      sheetVm,
      mode: state.mode,
      contentMode: state.contentMode,
      rowMap: state.rowMap,
      colMap: state.colMap,
      rowLookup: state.rowLookup,
      colLookup: state.colLookup,
      flash: flashModel,
      viewport: {
        width: size.width,
        height: size.height,
        scrollTop: scroll.scrollTop,
        scrollLeft: scroll.scrollLeft
      },
      metrics,
      theme,
      selection: state.selected,
      hover: state.hover,
      hoverMoveId: state.hoverMoveId
    });
    if (firstPaint) {
      firstPaint = false;
      const duration = performance.now() - perfStart;
      root.dispatchEvent(new CustomEvent("gridviewer:rendered", { detail: { duration } }));
    }
    if (state.flash) {
      schedulePaint();
    }
  }

  function resolvePane(x, layout) {
    if (layout.paneWidth <= 0) return null;
    const leftStart = layout.rowHeaderWidth;
    if (layout.paneCount === 1) {
      if (x < leftStart) return null;
      return { index: 0, localX: x - leftStart };
    }
    const rightStart = layout.rowHeaderWidth + layout.paneWidth + layout.paneGap;
    if (x >= leftStart && x < leftStart + layout.paneWidth) {
      return { index: 0, localX: x - leftStart };
    }
    if (x >= rightStart && x < rightStart + layout.paneWidth) {
      return { index: 1, localX: x - rightStart };
    }
    return null;
  }

  function hitTest(clientX, clientY) {
    const rect = canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    const layout = getLayout();
    if (x < 0 || y < 0 || x > rect.width || y > rect.height) return null;
    if (x < layout.rowHeaderWidth && y < layout.colHeaderHeight) {
      return { type: "corner" };
    }
    if (y < layout.colHeaderHeight) {
      const pane = resolvePane(x, layout);
      if (!pane) return null;
      const visibleCol = Math.floor((scroll.scrollLeft + pane.localX) / metrics.colWidth);
      if (visibleCol < 0 || visibleCol >= state.colCount) return null;
      const viewCol = visibleToViewCol(visibleCol);
      if (!Number.isFinite(viewCol)) return null;
      return { type: "col-header", viewCol, pane: pane.index };
    }
    if (x < layout.rowHeaderWidth) {
      const visibleRow = Math.floor((scroll.scrollTop + (y - layout.colHeaderHeight)) / metrics.rowHeight);
      if (visibleRow < 0 || visibleRow >= state.rowCount) return null;
      const viewRow = visibleToViewRow(visibleRow);
      if (!Number.isFinite(viewRow)) return null;
      return { type: "row-header", viewRow };
    }
    const pane = resolvePane(x, layout);
    if (!pane) return null;
    const visibleCol = Math.floor((scroll.scrollLeft + pane.localX) / metrics.colWidth);
    const visibleRow = Math.floor((scroll.scrollTop + (y - layout.colHeaderHeight)) / metrics.rowHeight);
    if (visibleRow < 0 || visibleRow >= state.rowCount) return null;
    if (visibleCol < 0 || visibleCol >= state.colCount) return null;
    const viewRow = visibleToViewRow(visibleRow);
    const viewCol = visibleToViewCol(visibleCol);
    if (!Number.isFinite(viewRow) || !Number.isFinite(viewCol)) return null;
    return { type: "cell", viewRow, viewCol, pane: pane.index };
  }

  function updateTooltip(cell, viewRow, viewCol, clientX, clientY) {
    if (!cell) {
      tooltip.classList.remove("visible");
      return;
    }
    tooltip.innerHTML = "";
    const summary = buildCellSummary(cell, viewRow, viewCol);
    const title = createEl("div", "grid-tooltip-title", `View ${summary.viewAddress}`);
    const meta = createEl("div", "grid-tooltip-meta", `Diff: ${summary.diffKind}`);
    tooltip.append(title, meta);

    function appendTooltipSection(label, details) {
      if (!details.value && !details.formula) return;
      const section = createEl("div", "grid-tooltip-section");
      section.append(createEl("div", "grid-tooltip-label", label));
      if (state.contentMode === "formulas") {
        if (details.formula) {
          section.append(createEl("div", "grid-tooltip-formula", details.formula));
        } else if (details.value) {
          section.append(createEl("div", "grid-tooltip-value", details.value));
        }
      } else {
        if (details.value) section.append(createEl("div", "grid-tooltip-value", details.value));
        if (details.formula) section.append(createEl("div", "grid-tooltip-formula", details.formula));
      }
      tooltip.append(section);
    }

    if (summary.oldAddress || summary.old.value || summary.old.formula) {
      appendTooltipSection(summary.oldAddress ? `Old ${summary.oldAddress}` : "Old", summary.old);
    }

    if (summary.newAddress || summary.fresh.value || summary.fresh.formula) {
      appendTooltipSection(summary.newAddress ? `New ${summary.newAddress}` : "New", summary.fresh);
    }

    tooltip.classList.add("visible");

    const wrapRect = canvasWrap.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    let left = clientX - wrapRect.left + 12;
    let top = clientY - wrapRect.top + 12;
    if (left + tooltipRect.width > wrapRect.width) {
      left = Math.max(8, wrapRect.width - tooltipRect.width - 8);
    }
    if (top + tooltipRect.height > wrapRect.height) {
      top = Math.max(8, wrapRect.height - tooltipRect.height - 8);
    }
    tooltip.style.transform = `translate(${left}px, ${top}px)`;
  }

  function updateInspector() {
    inspectorContent.innerHTML = "";
    if (!state.selected) {
      inspectorEmpty.style.display = "block";
      return;
    }
    inspectorEmpty.style.display = "none";

    const cell = sheetVm.cellAt(state.selected.viewRow, state.selected.viewCol);
    const summary = buildCellSummary(cell, state.selected.viewRow, state.selected.viewCol);

    const addRow = (label, value) => {
      if (!value) return;
      const row = createEl("div", "grid-inspector-row");
      row.append(createEl("div", "grid-inspector-label", label));
      row.append(createEl("div", "grid-inspector-value", value));
      inspectorContent.append(row);
    };

    addRow("View", summary.viewAddress);
    addRow("Diff", summary.diffKind);
    if (summary.moveId) {
      addRow("Move", summary.moveRole ? `${summary.moveRole} ${summary.moveId}` : summary.moveId);
    }
    if (summary.formulaDiff) {
      addRow("Formula Diff", String(summary.formulaDiff).replace(/_/g, " "));
    }
    const moveInfo = summary.moveId && sheetVm.moveLookup ? sheetVm.moveLookup.get(summary.moveId) : null;
    if (moveInfo) {
      addRow("Move From", moveInfo.src);
      addRow("Move To", moveInfo.dst);
    }
    addRow("Old", summary.oldAddress);
    if (state.contentMode === "formulas") {
      if (summary.old.formula) addRow("Old Formula", summary.old.formula);
      else if (summary.old.value) addRow("Old Value", summary.old.value);
    } else {
      if (summary.old.value) addRow("Old Value", summary.old.value);
      if (summary.old.formula) addRow("Old Formula", summary.old.formula);
    }
    addRow("New", summary.newAddress);
    if (state.contentMode === "formulas") {
      if (summary.fresh.formula) addRow("New Formula", summary.fresh.formula);
      else if (summary.fresh.value) addRow("New Value", summary.fresh.value);
    } else {
      if (summary.fresh.value) addRow("New Value", summary.fresh.value);
      if (summary.fresh.formula) addRow("New Formula", summary.fresh.formula);
    }

    function copyText(value) {
      if (!value) return;
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(value).catch(() => {});
        return;
      }
      const textarea = createEl("textarea");
      textarea.value = value;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.append(textarea);
      textarea.select();
      try {
        document.execCommand("copy");
      } catch (_) {}
      textarea.remove();
    }

    const actionWrap = createEl("div", "grid-inspector-actions");
    const oldCopy = summary.old.value || summary.old.formula;
    const newCopy = summary.fresh.value || summary.fresh.formula;
    const addrCopy = summary.newAddress || summary.oldAddress || summary.viewAddress;
    if (oldCopy) {
      const btn = createEl("button", "grid-inspector-copy", "Copy old");
      btn.type = "button";
      btn.addEventListener("click", () => copyText(oldCopy));
      actionWrap.append(btn);
    }
    if (newCopy) {
      const btn = createEl("button", "grid-inspector-copy", "Copy new");
      btn.type = "button";
      btn.addEventListener("click", () => copyText(newCopy));
      actionWrap.append(btn);
    }
    if (addrCopy) {
      const btn = createEl("button", "grid-inspector-copy", "Copy address");
      btn.type = "button";
      btn.addEventListener("click", () => copyText(addrCopy));
      actionWrap.append(btn);
    }
    if (actionWrap.children.length > 0) {
      inspectorContent.append(actionWrap);
    }

    const jumpTarget = resolveMoveTarget(cell);
    if (jumpTarget) {
      const jumpBtn = createEl("button", "grid-inspector-jump", "Jump to other end");
      jumpBtn.type = "button";
      jumpBtn.addEventListener("click", () => {
        selectCell(jumpTarget.viewRow, jumpTarget.viewCol, { center: true });
      });
      inspectorContent.append(jumpBtn);
    }
  }

  function resolveMoveTarget(cell) {
    if (!cell || !cell.moveId) return null;
    const rowEntry = sheetVm.axis.rows.entries[cell.viewRow];
    if (rowEntry && rowEntry.move_id === cell.moveId) {
      const targetKind = rowEntry.kind === "move_src" ? "move_dst" : "move_src";
      const targetRow = sheetVm.axis.rows.entries.findIndex(entry => entry?.move_id === cell.moveId && entry?.kind === targetKind);
      if (targetRow >= 0) {
        return { viewRow: targetRow, viewCol: cell.viewCol };
      }
    }
    const colEntry = sheetVm.axis.cols.entries[cell.viewCol];
    if (colEntry && colEntry.move_id === cell.moveId) {
      const targetKind = colEntry.kind === "move_src" ? "move_dst" : "move_src";
      const targetCol = sheetVm.axis.cols.entries.findIndex(entry => entry?.move_id === cell.moveId && entry?.kind === targetKind);
      if (targetCol >= 0) {
        return { viewRow: cell.viewRow, viewCol: targetCol };
      }
    }
    return null;
  }

  function setScroll(left, top) {
    const layout = getLayout();
    const maxLeft = Math.max(0, contentWidth - layout.width);
    const maxTop = Math.max(0, contentHeight - layout.height);
    scroll.scrollLeft = clamp(left, 0, maxLeft);
    scroll.scrollTop = clamp(top, 0, maxTop);
  }

  function centerOn(viewRow, viewCol) {
    const visibleRow = viewToVisibleRow(viewRow);
    const visibleCol = viewToVisibleCol(viewCol);
    if (visibleRow === null || visibleRow === undefined || visibleCol === null || visibleCol === undefined) return;
    const layout = getLayout();
    const cellAreaHeight = Math.max(0, layout.height - layout.colHeaderHeight);
    const cellX = visibleCol * metrics.colWidth;
    const cellY = visibleRow * metrics.rowHeight;
    const targetLeft = cellX - (layout.paneWidth - metrics.colWidth) / 2;
    const targetTop = cellY - (cellAreaHeight - metrics.rowHeight) / 2;
    setScroll(targetLeft, targetTop);
  }

  function scrollIntoView(viewRow, viewCol) {
    const visibleRow = viewToVisibleRow(viewRow);
    const visibleCol = viewToVisibleCol(viewCol);
    if (visibleRow === null || visibleRow === undefined || visibleCol === null || visibleCol === undefined) return;
    const layout = getLayout();
    const cellAreaHeight = Math.max(0, layout.height - layout.colHeaderHeight);
    const cellX = visibleCol * metrics.colWidth;
    const cellY = visibleRow * metrics.rowHeight;
    const minLeft = scroll.scrollLeft;
    const maxLeft = scroll.scrollLeft + layout.paneWidth - metrics.colWidth;
    const minTop = scroll.scrollTop;
    const maxTop = scroll.scrollTop + cellAreaHeight - metrics.rowHeight;
    let nextLeft = scroll.scrollLeft;
    let nextTop = scroll.scrollTop;
    if (cellX < minLeft) nextLeft = cellX;
    else if (cellX > maxLeft) nextLeft = cellX - (layout.paneWidth - metrics.colWidth);
    if (cellY < minTop) nextTop = cellY;
    else if (cellY > maxTop) nextTop = cellY - (cellAreaHeight - metrics.rowHeight);
    setScroll(nextLeft, nextTop);
  }

  function selectCell(viewRow, viewCol, { center = false } = {}) {
    state.selected = { viewRow, viewCol };
    ensureSelectionVisible();
    updateInspector();
    const target = state.selected;
    if (center) {
      centerOn(target.viewRow, target.viewCol);
    } else {
      scrollIntoView(target.viewRow, target.viewCol);
    }
    schedulePaint();
  }

  function dispatchViewerEvent(name, detail) {
    mountEl.dispatchEvent(new CustomEvent(name, { detail }));
  }

  function resolveAnchorIndex(anchorIdOrIndex) {
    if (typeof anchorIdOrIndex === "string") {
      const idx = anchorIndexById.get(anchorIdOrIndex);
      return Number.isFinite(idx) ? idx : -1;
    }
    if (Number.isFinite(anchorIdOrIndex)) {
      return anchorIdOrIndex;
    }
    return -1;
  }

  function announceAnchor(anchor) {
    if (!anchor) return;
    dispatchViewerEvent("gridviewer:anchor", { anchorId: anchor.id });
  }

  function jumpToAnchor(anchorIdOrIndex) {
    if (!gridAnchors.length) return false;
    const nextIndex = resolveAnchorIndex(anchorIdOrIndex);
    if (nextIndex < 0 || nextIndex >= gridAnchors.length) return false;
    const anchor = gridAnchors[nextIndex];
    state.anchorIndex = nextIndex;
    selectCell(anchor.viewRow, anchor.viewCol, { center: true });
    announceAnchor(anchor);
    return true;
  }

  function flashAnchor(anchorIdOrIndex) {
    const idx = resolveAnchorIndex(anchorIdOrIndex);
    if (idx < 0 || idx >= gridAnchors.length) return false;
    const anchor = gridAnchors[idx];
    let flash = null;
    if (anchor.regionId) {
      const region = regionLookup.get(anchor.regionId);
      if (region) {
        flash = {
          kind: "region",
          bounds: { top: region.top, bottom: region.bottom, left: region.left, right: region.right },
          start: performance.now()
        };
      }
    }
    if (!flash && anchor.group === "rows") {
      flash = { kind: "row", viewRow: anchor.viewRow, start: performance.now() };
    }
    if (!flash && anchor.group === "cols") {
      flash = { kind: "col", viewCol: anchor.viewCol, start: performance.now() };
    }
    if (!flash && anchor.moveId) {
      if (anchor.moveId.startsWith("r:")) {
        flash = { kind: "row", viewRow: anchor.viewRow, start: performance.now() };
      } else if (anchor.moveId.startsWith("c:")) {
        flash = { kind: "col", viewCol: anchor.viewCol, start: performance.now() };
      }
    }
    if (!flash) {
      flash = { kind: "cell", viewRow: anchor.viewRow, viewCol: anchor.viewCol, start: performance.now() };
    }
    state.flash = flash;
    schedulePaint();
    return true;
  }

  function nextAnchor() {
    if (!gridAnchors.length) return false;
    const nextIndex = state.anchorIndex < 0 ? 0 : state.anchorIndex + 1;
    if (nextIndex >= gridAnchors.length) return false;
    return jumpToAnchor(nextIndex);
  }

  function prevAnchor() {
    if (!gridAnchors.length) return false;
    const nextIndex = state.anchorIndex < 0 ? gridAnchors.length - 1 : state.anchorIndex - 1;
    if (nextIndex < 0) return false;
    return jumpToAnchor(nextIndex);
  }

  function onScroll() {
    state.hover = null;
    state.hoverMoveId = null;
    tooltip.classList.remove("visible");
    schedulePaint();
  }

  function onPointerMove(e) {
    const hit = hitTest(e.clientX, e.clientY);
    if (!hit) {
      state.hover = null;
      state.hoverMoveId = null;
      tooltip.classList.remove("visible");
      schedulePaint();
      return;
    }
    if (hit.type === "cell") {
      state.hover = { viewRow: hit.viewRow, viewCol: hit.viewCol, pane: hit.pane };
      const cell = sheetVm.cellAt(hit.viewRow, hit.viewCol);
      state.hoverMoveId = cell?.moveId || null;
      updateTooltip(cell, hit.viewRow, hit.viewCol, e.clientX, e.clientY);
      schedulePaint();
      return;
    }
    if (hit.type === "row-header") {
      const entry = sheetVm.axis.rows.entries[hit.viewRow];
      state.hover = { viewRow: hit.viewRow, viewCol: null, pane: null };
      state.hoverMoveId = entry?.move_id || null;
    } else if (hit.type === "col-header") {
      const entry = sheetVm.axis.cols.entries[hit.viewCol];
      state.hover = { viewRow: null, viewCol: hit.viewCol, pane: null };
      state.hoverMoveId = entry?.move_id || null;
    } else {
      state.hover = null;
      state.hoverMoveId = null;
    }
    tooltip.classList.remove("visible");
    schedulePaint();
  }

  function onPointerLeave() {
    state.hover = null;
    state.hoverMoveId = null;
    tooltip.classList.remove("visible");
    schedulePaint();
  }

  function onClick(e) {
    const hit = hitTest(e.clientX, e.clientY);
    if (!hit) return;
    if (hit.type === "cell") {
      selectCell(hit.viewRow, hit.viewCol);
    } else if (hit.type === "row-header") {
      const targetCol = state.selected ? state.selected.viewCol : 0;
      selectCell(hit.viewRow, clamp(targetCol, 0, sheetVm.axis.cols.entries.length - 1));
    } else if (hit.type === "col-header") {
      const targetRow = state.selected ? state.selected.viewRow : 0;
      selectCell(clamp(targetRow, 0, sheetVm.axis.rows.entries.length - 1), hit.viewCol);
    }
    root.focus();
  }

  function onKeyDown(e) {
    if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", "n", "p", "N", "P"].includes(e.key)) {
      return;
    }
    if (e.key === "Enter") {
      state.pinned = !state.pinned;
      inspector.classList.toggle("pinned", state.pinned);
      e.preventDefault();
      return;
    }
    if (e.key === "n" || e.key === "N") {
      if (nextAnchor()) {
        flashAnchor(state.anchorIndex);
      }
      e.preventDefault();
      return;
    }
    if (e.key === "p" || e.key === "P") {
      if (prevAnchor()) {
        flashAnchor(state.anchorIndex);
      }
      e.preventDefault();
      return;
    }

    const maxVisibleRow = Math.max(0, state.rowCount - 1);
    const maxVisibleCol = Math.max(0, state.colCount - 1);
    let viewRow = state.selected ? state.selected.viewRow : 0;
    let viewCol = state.selected ? state.selected.viewCol : 0;
    let visibleRow = viewToVisibleRow(viewRow);
    let visibleCol = viewToVisibleCol(viewCol);
    if (visibleRow === null || visibleRow === undefined) visibleRow = 0;
    if (visibleCol === null || visibleCol === undefined) visibleCol = 0;
    if (e.key === "ArrowUp") visibleRow = clamp(visibleRow - 1, 0, maxVisibleRow);
    if (e.key === "ArrowDown") visibleRow = clamp(visibleRow + 1, 0, maxVisibleRow);
    if (e.key === "ArrowLeft") visibleCol = clamp(visibleCol - 1, 0, maxVisibleCol);
    if (e.key === "ArrowRight") visibleCol = clamp(visibleCol + 1, 0, maxVisibleCol);
    viewRow = visibleToViewRow(visibleRow);
    viewCol = visibleToViewCol(visibleCol);
    if (Number.isFinite(viewRow) && Number.isFinite(viewCol)) {
      selectCell(viewRow, viewCol);
    }
    e.preventDefault();
  }

  function setDisplayOptions(next = {}) {
    let updated = false;
    const allowedModes = new Set(["values", "formulas", "both"]);
    if (typeof next.contentMode === "string" && allowedModes.has(next.contentMode) && next.contentMode !== state.contentMode) {
      state.contentMode = next.contentMode;
      updated = true;
      updateInspector();
    }
    if (typeof next.focusRows === "boolean" && next.focusRows !== state.focusRows) {
      state.focusRows = next.focusRows;
      updated = true;
    }
    if (typeof next.focusCols === "boolean" && next.focusCols !== state.focusCols) {
      state.focusCols = next.focusCols;
      updated = true;
    }
    if (updated) {
      updateDisplayMaps();
      ensureSelectionVisible();
      setScroll(scroll.scrollLeft, scroll.scrollTop);
      if (state.selected) {
        scrollIntoView(state.selected.viewRow, state.selected.viewCol);
      }
      schedulePaint();
    }
  }

  function onFocus() {
    dispatchViewerEvent("gridviewer:focus", {});
  }

  function onModeClick(e) {
    const btn = e.target.closest(".grid-mode-btn");
    if (!btn) return;
    const nextMode = btn.dataset.mode === "unified" ? "unified" : "side_by_side";
    if (state.mode !== nextMode) {
      state.mode = nextMode;
      updateModeButtons();
      schedulePaint();
    }
  }

  function toggleInspector() {
    inspector.classList.toggle("collapsed");
    inspectorToggle.textContent = inspector.classList.contains("collapsed") ? "Expand" : "Collapse";
    root.classList.toggle("inspector-collapsed", inspector.classList.contains("collapsed"));
  }

  toolbar.addEventListener("click", onModeClick);
  inspectorToggle.addEventListener("click", toggleInspector);
  scroll.addEventListener("scroll", onScroll);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerleave", onPointerLeave);
  canvas.addEventListener("click", onClick);
  root.addEventListener("keydown", onKeyDown);
  root.addEventListener("focus", onFocus);

  const resizeObserver = new ResizeObserver(() => schedulePaint());
  resizeObserver.observe(scroll);

  updateModeButtons();
  schedulePaint();
  if (Number.isFinite(state.anchorIndex) && gridAnchors.length > 0 && state.anchorIndex >= 0) {
    jumpToAnchor(state.anchorIndex);
  }

  return {
    destroy() {
      if (rafId !== null) cancelAnimationFrame(rafId);
      resizeObserver.disconnect();
      toolbar.removeEventListener("click", onModeClick);
      inspectorToggle.removeEventListener("click", toggleInspector);
      scroll.removeEventListener("scroll", onScroll);
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      canvas.removeEventListener("click", onClick);
      root.removeEventListener("keydown", onKeyDown);
      root.removeEventListener("focus", onFocus);
    },
    focus() {
      root.focus();
    },
    jumpTo(viewRow, viewCol) {
      selectCell(viewRow, viewCol, { center: true });
    },
    jumpToAnchor(anchorIdOrIndex) {
      return jumpToAnchor(anchorIdOrIndex);
    },
    nextAnchor() {
      return nextAnchor();
    },
    prevAnchor() {
      return prevAnchor();
    },
    setDisplayOptions(options) {
      setDisplayOptions(options);
    },
    flashAnchor(anchorIdOrIndex) {
      return flashAnchor(anchorIdOrIndex);
    },
    capturePng() {
      paint();
      try {
        return canvas.toDataURL("image/png");
      } catch (err) {
        return "";
      }
    }
  };
}
