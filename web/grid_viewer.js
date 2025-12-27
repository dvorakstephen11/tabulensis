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
  const state = {
    mode: opts.initialMode === "unified" ? "unified" : "side_by_side",
    selected: null,
    hover: null,
    hoverMoveId: null,
    anchorIndex: Number.isFinite(opts.initialAnchor) ? opts.initialAnchor : 0,
    pinned: false
  };

  const theme = readGridTheme(document.documentElement);
  const metrics = { ...GRID_METRICS };
  const anchors = (sheetVm.changes?.regions || []).map(region => ({
    row: region.top,
    col: region.left,
    id: region.id
  }));

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
  const inspectorTitle = createEl("div", "grid-inspector-title", "Inspector");
  const inspectorEmpty = createEl("div", "grid-inspector-empty", "Select a cell to inspect.");
  const inspectorContent = createEl("div", "grid-inspector-content");
  inspector.append(inspectorTitle, inspectorEmpty, inspectorContent);

  body.append(canvasWrap, inspector);
  root.append(toolbar, body);
  mountEl.append(root);

  const ctx = canvas.getContext("2d");
  let rafId = null;

  const contentWidth = metrics.rowHeaderWidth + sheetVm.axis.cols.entries.length * metrics.colWidth;
  const contentHeight = metrics.colHeaderHeight + sheetVm.axis.rows.entries.length * metrics.rowHeight;
  spacer.style.width = `${contentWidth}px`;
  spacer.style.height = `${contentHeight}px`;

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
    paintGrid(ctx, {
      sheetVm,
      mode: state.mode,
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
      const viewCol = Math.floor((scroll.scrollLeft + pane.localX) / metrics.colWidth);
      if (viewCol < 0 || viewCol >= sheetVm.axis.cols.entries.length) return null;
      return { type: "col-header", viewCol, pane: pane.index };
    }
    if (x < layout.rowHeaderWidth) {
      const viewRow = Math.floor((scroll.scrollTop + (y - layout.colHeaderHeight)) / metrics.rowHeight);
      if (viewRow < 0 || viewRow >= sheetVm.axis.rows.entries.length) return null;
      return { type: "row-header", viewRow };
    }
    const pane = resolvePane(x, layout);
    if (!pane) return null;
    const viewCol = Math.floor((scroll.scrollLeft + pane.localX) / metrics.colWidth);
    const viewRow = Math.floor((scroll.scrollTop + (y - layout.colHeaderHeight)) / metrics.rowHeight);
    if (viewRow < 0 || viewRow >= sheetVm.axis.rows.entries.length) return null;
    if (viewCol < 0 || viewCol >= sheetVm.axis.cols.entries.length) return null;
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

    if (summary.oldAddress || summary.old.value || summary.old.formula) {
      const section = createEl("div", "grid-tooltip-section");
      section.append(createEl("div", "grid-tooltip-label", summary.oldAddress ? `Old ${summary.oldAddress}` : "Old"));
      if (summary.old.value) section.append(createEl("div", "grid-tooltip-value", summary.old.value));
      if (summary.old.formula) section.append(createEl("div", "grid-tooltip-formula", summary.old.formula));
      tooltip.append(section);
    }

    if (summary.newAddress || summary.fresh.value || summary.fresh.formula) {
      const section = createEl("div", "grid-tooltip-section");
      section.append(createEl("div", "grid-tooltip-label", summary.newAddress ? `New ${summary.newAddress}` : "New"));
      if (summary.fresh.value) section.append(createEl("div", "grid-tooltip-value", summary.fresh.value));
      if (summary.fresh.formula) section.append(createEl("div", "grid-tooltip-formula", summary.fresh.formula));
      tooltip.append(section);
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
    addRow("Old", summary.oldAddress);
    if (summary.old.value) addRow("Old Value", summary.old.value);
    if (summary.old.formula) addRow("Old Formula", summary.old.formula);
    addRow("New", summary.newAddress);
    if (summary.fresh.value) addRow("New Value", summary.fresh.value);
    if (summary.fresh.formula) addRow("New Formula", summary.fresh.formula);

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
    const layout = getLayout();
    const cellAreaHeight = Math.max(0, layout.height - layout.colHeaderHeight);
    const cellX = viewCol * metrics.colWidth;
    const cellY = viewRow * metrics.rowHeight;
    const targetLeft = cellX - (layout.paneWidth - metrics.colWidth) / 2;
    const targetTop = cellY - (cellAreaHeight - metrics.rowHeight) / 2;
    setScroll(targetLeft, targetTop);
  }

  function scrollIntoView(viewRow, viewCol) {
    const layout = getLayout();
    const cellAreaHeight = Math.max(0, layout.height - layout.colHeaderHeight);
    const cellX = viewCol * metrics.colWidth;
    const cellY = viewRow * metrics.rowHeight;
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
    updateInspector();
    if (center) {
      centerOn(viewRow, viewCol);
    } else {
      scrollIntoView(viewRow, viewCol);
    }
    schedulePaint();
  }

  function jumpToRegion(index) {
    if (!anchors.length) return;
    const nextIndex = ((index % anchors.length) + anchors.length) % anchors.length;
    const anchor = anchors[nextIndex];
    state.anchorIndex = nextIndex;
    selectCell(anchor.row, anchor.col, { center: true });
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
      jumpToRegion(state.anchorIndex + 1);
      e.preventDefault();
      return;
    }
    if (e.key === "p" || e.key === "P") {
      jumpToRegion(state.anchorIndex - 1);
      e.preventDefault();
      return;
    }

    const rowCount = sheetVm.axis.rows.entries.length;
    const colCount = sheetVm.axis.cols.entries.length;
    let row = state.selected ? state.selected.viewRow : 0;
    let col = state.selected ? state.selected.viewCol : 0;
    if (e.key === "ArrowUp") row = clamp(row - 1, 0, rowCount - 1);
    if (e.key === "ArrowDown") row = clamp(row + 1, 0, rowCount - 1);
    if (e.key === "ArrowLeft") col = clamp(col - 1, 0, colCount - 1);
    if (e.key === "ArrowRight") col = clamp(col + 1, 0, colCount - 1);
    selectCell(row, col);
    e.preventDefault();
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

  toolbar.addEventListener("click", onModeClick);
  scroll.addEventListener("scroll", onScroll);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerleave", onPointerLeave);
  canvas.addEventListener("click", onClick);
  root.addEventListener("keydown", onKeyDown);

  const resizeObserver = new ResizeObserver(() => schedulePaint());
  resizeObserver.observe(scroll);

  updateModeButtons();
  schedulePaint();
  if (Number.isFinite(state.anchorIndex) && anchors.length > 0 && state.anchorIndex >= 0) {
    jumpToRegion(state.anchorIndex);
  }

  return {
    destroy() {
      if (rafId !== null) cancelAnimationFrame(rafId);
      resizeObserver.disconnect();
      toolbar.removeEventListener("click", onModeClick);
      scroll.removeEventListener("scroll", onScroll);
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      canvas.removeEventListener("click", onClick);
      root.removeEventListener("keydown", onKeyDown);
    },
    focus() {
      root.focus();
    },
    jumpTo(viewRow, viewCol) {
      selectCell(viewRow, viewCol, { center: true });
    },
    jumpToRegion(index) {
      jumpToRegion(index);
    }
  };
}
