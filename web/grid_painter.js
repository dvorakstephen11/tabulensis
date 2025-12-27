function colToLetter(col) {
  let result = "";
  let c = col;
  while (c >= 0) {
    result = String.fromCharCode((c % 26) + 65) + result;
    c = Math.floor(c / 26) - 1;
  }
  return result;
}

function safeText(value) {
  if (value === null || value === undefined) return "";
  return String(value);
}

function resolveCellParts(cell, side) {
  if (!cell) return { value: "", formula: "" };
  if (cell.edit) {
    const value = side === "old" ? cell.edit.fromValue : cell.edit.toValue;
    const formula = side === "old" ? cell.edit.fromFormula : cell.edit.toFormula;
    if (value || formula) return { value: safeText(value), formula: safeText(formula) };
  }
  const payload = side === "old" ? cell.old : cell.new;
  if (!payload || !payload.cell) return { value: "", formula: "" };
  return { value: safeText(payload.cell.value || ""), formula: safeText(payload.cell.formula || "") };
}

function resolveCellText(cell, side, contentMode) {
  const parts = resolveCellParts(cell, side);
  if (contentMode === "formulas") {
    return parts.formula || parts.value || "";
  }
  return parts.value || parts.formula || "";
}

function resolveUnifiedText(cell, contentMode) {
  if (!cell) return "";
  if (cell.diffKind === "added") return resolveCellText(cell, "new", contentMode);
  if (cell.diffKind === "removed") return resolveCellText(cell, "old", contentMode);
  if (cell.diffKind === "moved") {
    return cell.moveRole === "src"
      ? resolveCellText(cell, "old", contentMode)
      : resolveCellText(cell, "new", contentMode);
  }
  return resolveCellText(cell, "new", contentMode) || resolveCellText(cell, "old", contentMode);
}

function truncateText(ctx, text, maxWidth) {
  const value = safeText(text);
  if (!value) return "";
  if (ctx.measureText(value).width <= maxWidth) return value;
  let end = value.length;
  while (end > 0) {
    const candidate = value.slice(0, end) + "...";
    if (ctx.measureText(candidate).width <= maxWidth) return candidate;
    end -= 1;
  }
  return "";
}

function drawStrike(ctx, x, y, width, color) {
  if (!width) return;
  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + width, y);
  ctx.stroke();
  ctx.restore();
}

function axisStyle(entry, axis, theme) {
  if (!entry) {
    return { bg: theme.bgTertiary, text: theme.textSecondary, marker: "" };
  }
  if (entry.kind === "insert") {
    return { bg: theme.diffAddBg, text: theme.accentGreen, marker: "+" };
  }
  if (entry.kind === "delete") {
    return { bg: theme.diffRemoveBg, text: theme.accentRed, marker: "-" };
  }
  if (entry.kind === "move_src") {
    return { bg: theme.diffMoveBg, text: theme.accentPurple, marker: "M" };
  }
  if (entry.kind === "move_dst") {
    return { bg: theme.diffMoveDstBg || theme.diffMoveBg, text: theme.accentPurple, marker: "M" };
  }
  return { bg: theme.bgTertiary, text: theme.textSecondary, marker: "" };
}

function cellStyle(cell, theme) {
  if (!cell) return { bg: theme.bgPrimary, text: theme.textPrimary };
  if (cell.diffKind === "edited") {
    return { bg: theme.diffModifyBg, text: theme.textPrimary };
  }
  if (cell.diffKind === "added") {
    return { bg: theme.diffAddBg, text: theme.textPrimary };
  }
  if (cell.diffKind === "removed") {
    return { bg: theme.diffRemoveBg, text: theme.textPrimary };
  }
  if (cell.diffKind === "moved") {
    const bg = cell.moveRole === "dst" ? theme.diffMoveDstBg : theme.diffMoveBg;
    return { bg, text: theme.textPrimary };
  }
  if (cell.diffKind === "unchanged") {
    return { bg: theme.bgPrimary, text: theme.textPrimary };
  }
  return { bg: theme.bgPrimary, text: theme.textMuted };
}

function drawHeaderCell(ctx, x, y, width, height, label, style, theme) {
  ctx.fillStyle = style.bg;
  ctx.fillRect(x, y, width, height);
  ctx.strokeStyle = theme.borderSecondary;
  ctx.lineWidth = 1;
  ctx.strokeRect(x, y, width, height);
  ctx.fillStyle = style.text;
  ctx.font = "11px 'JetBrains Mono', monospace";
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";
  const text = style.marker ? `${label} ${style.marker}` : label;
  ctx.fillText(text, x + width / 2, y + height / 2);
}

function drawRowHeader(ctx, x, y, width, height, label, style, theme) {
  ctx.fillStyle = style.bg;
  ctx.fillRect(x, y, width, height);
  ctx.strokeStyle = theme.borderSecondary;
  ctx.lineWidth = 1;
  ctx.strokeRect(x, y, width, height);
  ctx.fillStyle = style.text;
  ctx.font = "11px 'JetBrains Mono', monospace";
  ctx.textBaseline = "middle";
  ctx.textAlign = "right";
  const text = style.marker ? `${label} ${style.marker}` : label;
  ctx.fillText(text, x + width - 6, y + height / 2);
}

function drawCellText(ctx, x, y, width, height, text, color) {
  const paddingX = 8;
  const maxWidth = Math.max(0, width - paddingX * 2);
  ctx.fillStyle = color;
  ctx.font = "12px 'JetBrains Mono', monospace";
  ctx.textBaseline = "middle";
  ctx.textAlign = "left";
  const value = truncateText(ctx, text, maxWidth);
  ctx.fillText(value, x + paddingX, y + height / 2);
}

function drawCellTextLines(ctx, x, y, width, height, primary, secondary, primaryColor, secondaryColor) {
  const paddingX = 8;
  const maxWidth = Math.max(0, width - paddingX * 2);
  ctx.textAlign = "left";
  ctx.textBaseline = "top";
  ctx.font = "11px 'JetBrains Mono', monospace";
  ctx.fillStyle = primaryColor;
  const primaryText = truncateText(ctx, primary, maxWidth);
  ctx.fillText(primaryText, x + paddingX, y + 6);
  ctx.font = "10px 'JetBrains Mono', monospace";
  ctx.fillStyle = secondaryColor;
  const secondaryText = truncateText(ctx, secondary, maxWidth);
  ctx.fillText(secondaryText, x + paddingX, y + height / 2);
}

function drawEditedUnified(ctx, x, y, width, height, cell, theme, contentMode) {
  const paddingX = 8;
  const paddingY = 6;
  const maxWidth = Math.max(0, width - paddingX * 2);
  const oldText = resolveCellText(cell, "old", contentMode);
  const newText = resolveCellText(cell, "new", contentMode);

  ctx.font = "10px 'JetBrains Mono', monospace";
  ctx.textBaseline = "top";
  ctx.textAlign = "left";
  ctx.fillStyle = theme.accentRed;
  const oldDisplay = truncateText(ctx, oldText, maxWidth);
  ctx.fillText(oldDisplay, x + paddingX, y + paddingY);
  const oldWidth = ctx.measureText(oldDisplay).width;
  drawStrike(ctx, x + paddingX, y + paddingY + 6, oldWidth, theme.accentRed);

  ctx.font = "12px 'JetBrains Mono', monospace";
  ctx.fillStyle = theme.accentGreen;
  const newDisplay = truncateText(ctx, newText, maxWidth);
  ctx.fillText(newDisplay, x + paddingX, y + paddingY + 14);
}

function drawCell(ctx, x, y, width, height, cell, theme, mode, side, contentMode) {
  const style = cellStyle(cell, theme);
  ctx.fillStyle = style.bg;
  ctx.fillRect(x, y, width, height);
  ctx.strokeStyle = theme.borderSecondary;
  ctx.lineWidth = 1;
  ctx.strokeRect(x, y, width, height);

  if (!cell || cell.diffKind === "empty") return;

  if (mode === "unified" && cell.diffKind === "edited") {
    drawEditedUnified(ctx, x, y, width, height, cell, theme, contentMode);
    return;
  }

  let text = "";
  if (mode === "side_by_side") {
    if (contentMode === "both") {
      const parts = resolveCellParts(cell, side);
      const primary = parts.value || parts.formula;
      const secondary = parts.value && parts.formula && parts.formula !== parts.value ? parts.formula : "";
      if (secondary && height >= 30) {
        const color =
          cell.diffKind === "added"
            ? theme.accentGreen
            : cell.diffKind === "removed"
              ? theme.accentRed
              : cell.diffKind === "moved"
                ? theme.accentPurple
                : style.text;
        drawCellTextLines(ctx, x, y, width, height, primary, secondary, color, theme.textMuted);
        return;
      }
      text = primary;
    } else {
      text = resolveCellText(cell, side, contentMode);
    }
  } else {
    text = resolveUnifiedText(cell, contentMode);
  }

  if (cell.diffKind === "removed" && mode === "unified") {
    ctx.save();
    ctx.fillStyle = theme.accentRed;
    drawCellText(ctx, x, y, width, height, text, theme.accentRed);
    const textWidth = ctx.measureText(truncateText(ctx, text, Math.max(0, width - 16))).width;
    drawStrike(ctx, x + 8, y + height / 2, textWidth, theme.accentRed);
    ctx.restore();
    return;
  }

  const color =
    cell.diffKind === "added"
      ? theme.accentGreen
      : cell.diffKind === "removed"
        ? theme.accentRed
        : cell.diffKind === "moved"
          ? theme.accentPurple
          : style.text;

  drawCellText(ctx, x, y, width, height, text, color);
}

export function paintGrid(ctx, model) {
  const {
    sheetVm,
    mode,
    contentMode,
    rowMap,
    colMap,
    rowLookup,
    colLookup,
    flash,
    viewport,
    metrics,
    theme,
    selection,
    hover,
    hoverMoveId
  } = model;
  const { width, height, scrollTop, scrollLeft } = viewport;
  const rows = sheetVm.axis.rows.entries;
  const cols = sheetVm.axis.cols.entries;
  const rowCount = rowMap ? rowMap.length : rows.length;
  const colCount = colMap ? colMap.length : cols.length;

  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = theme.bgPrimary;
  ctx.fillRect(0, 0, width, height);

  if (!rowCount || !colCount || width <= 0 || height <= 0) return;

  const rowHeight = metrics.rowHeight;
  const colWidth = metrics.colWidth;
  const rowHeaderWidth = metrics.rowHeaderWidth;
  const colHeaderHeight = metrics.colHeaderHeight;
  const paneGap = metrics.paneGap;
  const paneCount = mode === "side_by_side" ? 2 : 1;
  const paneWidth =
    paneCount === 2
      ? Math.max(0, Math.floor((width - rowHeaderWidth - paneGap) / 2))
      : Math.max(0, width - rowHeaderWidth);
  const cellAreaHeight = Math.max(0, height - colHeaderHeight);

  const firstRow = Math.max(0, Math.floor(scrollTop / rowHeight));
  const visibleRowCount = cellAreaHeight > 0 ? Math.ceil(cellAreaHeight / rowHeight) + 1 : 0;
  const lastRow = Math.min(rowCount - 1, firstRow + visibleRowCount - 1);

  const firstCol = Math.max(0, Math.floor(scrollLeft / colWidth));
  const visibleColCount = paneWidth > 0 ? Math.ceil(paneWidth / colWidth) + 1 : 0;
  const lastCol = Math.min(colCount - 1, firstCol + visibleColCount - 1);

  const paneOffsets = [rowHeaderWidth];
  if (paneCount === 2) {
    paneOffsets.push(rowHeaderWidth + paneWidth + paneGap);
  }

  ctx.fillStyle = theme.bgTertiary;
  ctx.fillRect(0, 0, rowHeaderWidth, colHeaderHeight);

  for (let c = firstCol; c <= lastCol; c++) {
    const viewCol = colMap ? colMap[c] : c;
    const entry = cols[viewCol];
    const style = axisStyle(entry, "col", theme);
    const label = colToLetter(viewCol);
    for (let p = 0; p < paneCount; p++) {
      const x = paneOffsets[p] + (c * colWidth - scrollLeft);
      drawHeaderCell(ctx, x, 0, colWidth, colHeaderHeight, label, style, theme);
    }
  }

  for (let r = firstRow; r <= lastRow; r++) {
    const viewRow = rowMap ? rowMap[r] : r;
    const entry = rows[viewRow];
    const style = axisStyle(entry, "row", theme);
    const y = colHeaderHeight + (r * rowHeight - scrollTop);
    drawRowHeader(ctx, 0, y, rowHeaderWidth, rowHeight, String(viewRow + 1), style, theme);
  }

  for (let r = firstRow; r <= lastRow; r++) {
    const viewRow = rowMap ? rowMap[r] : r;
    const y = colHeaderHeight + (r * rowHeight - scrollTop);
    for (let c = firstCol; c <= lastCol; c++) {
      const viewCol = colMap ? colMap[c] : c;
      const cell = sheetVm.cellAt(viewRow, viewCol);
      for (let p = 0; p < paneCount; p++) {
        const x = paneOffsets[p] + (c * colWidth - scrollLeft);
        const side = p === 0 ? "old" : "new";
        drawCell(ctx, x, y, colWidth, rowHeight, cell, theme, mode, side, contentMode);
      }

      if (hoverMoveId) {
        const rowEntry = rows[viewRow];
        const colEntry = cols[viewCol];
        if ((rowEntry && rowEntry.move_id === hoverMoveId) || (colEntry && colEntry.move_id === hoverMoveId)) {
          ctx.save();
          ctx.strokeStyle = theme.diffMoveBorder || theme.accentPurple;
          ctx.lineWidth = 2;
          for (let p = 0; p < paneCount; p++) {
            const x = paneOffsets[p] + (c * colWidth - scrollLeft);
            ctx.strokeRect(x + 1, y + 1, colWidth - 2, rowHeight - 2);
          }
          ctx.restore();
        }
      }
    }
  }

  if (hover && hover.viewRow !== null && hover.viewCol !== null) {
    const row = rowLookup ? rowLookup[hover.viewRow] : hover.viewRow;
    const col = colLookup ? colLookup[hover.viewCol] : hover.viewCol;
    if (row !== null && row !== undefined && col !== null && col !== undefined &&
        row >= firstRow && row <= lastRow && col >= firstCol && col <= lastCol) {
      const y = colHeaderHeight + (row * rowHeight - scrollTop);
      for (let p = 0; p < paneCount; p++) {
        if (paneCount === 2 && hover.pane !== null && hover.pane !== undefined && hover.pane !== p) {
          continue;
        }
        const x = paneOffsets[p] + (col * colWidth - scrollLeft);
        ctx.save();
        ctx.strokeStyle = theme.accentBlue;
        ctx.lineWidth = 1;
        ctx.strokeRect(x + 1, y + 1, colWidth - 2, rowHeight - 2);
        ctx.restore();
      }
    }
  }

  if (selection && selection.viewRow !== null && selection.viewCol !== null) {
    const row = rowLookup ? rowLookup[selection.viewRow] : selection.viewRow;
    const col = colLookup ? colLookup[selection.viewCol] : selection.viewCol;
    if (row !== null && row !== undefined && col !== null && col !== undefined &&
        row >= firstRow && row <= lastRow && col >= firstCol && col <= lastCol) {
      const y = colHeaderHeight + (row * rowHeight - scrollTop);
      for (let p = 0; p < paneCount; p++) {
        const x = paneOffsets[p] + (col * colWidth - scrollLeft);
        ctx.save();
        ctx.strokeStyle = theme.accentBlue;
        ctx.lineWidth = 2;
        ctx.strokeRect(x + 1, y + 1, colWidth - 2, rowHeight - 2);
        ctx.restore();
      }
    }
  }

  if (flash && flash.alpha > 0) {
    const mapRange = (start, end, lookup) => {
      let min = null;
      let max = null;
      for (let i = start; i <= end; i++) {
        const vis = lookup ? lookup[i] : i;
        if (vis === null || vis === undefined) continue;
        if (min === null || vis < min) min = vis;
        if (max === null || vis > max) max = vis;
      }
      if (min === null || max === null) return null;
      return { start: min, end: max };
    };

    ctx.save();
    ctx.globalAlpha = flash.alpha;
    ctx.strokeStyle = theme.accentBlue;
    ctx.lineWidth = 2;

    if (flash.kind === "region" && flash.bounds) {
      const rowRange = mapRange(flash.bounds.top, flash.bounds.bottom, rowLookup);
      const colRange = mapRange(flash.bounds.left, flash.bounds.right, colLookup);
      if (rowRange && colRange) {
        const startRow = Math.max(firstRow, rowRange.start);
        const endRow = Math.min(lastRow, rowRange.end);
        const startCol = Math.max(firstCol, colRange.start);
        const endCol = Math.min(lastCol, colRange.end);
        if (startRow <= endRow && startCol <= endCol) {
          const y = colHeaderHeight + (startRow * rowHeight - scrollTop);
          const height = (endRow - startRow + 1) * rowHeight;
          for (let p = 0; p < paneCount; p++) {
            const x = paneOffsets[p] + (startCol * colWidth - scrollLeft);
            const width = (endCol - startCol + 1) * colWidth;
            ctx.strokeRect(x + 1, y + 1, width - 2, height - 2);
          }
        }
      }
    } else if (flash.kind === "row") {
      const row = rowLookup ? rowLookup[flash.viewRow] : flash.viewRow;
      if (row !== null && row !== undefined && row >= firstRow && row <= lastRow) {
        const y = colHeaderHeight + (row * rowHeight - scrollTop);
        ctx.strokeRect(0 + 1, y + 1, rowHeaderWidth - 2, rowHeight - 2);
      }
    } else if (flash.kind === "col") {
      const col = colLookup ? colLookup[flash.viewCol] : flash.viewCol;
      if (col !== null && col !== undefined && col >= firstCol && col <= lastCol) {
        for (let p = 0; p < paneCount; p++) {
          const x = paneOffsets[p] + (col * colWidth - scrollLeft);
          ctx.strokeRect(x + 1, 0 + 1, colWidth - 2, colHeaderHeight - 2);
        }
      }
    } else if (flash.kind === "cell") {
      const row = rowLookup ? rowLookup[flash.viewRow] : flash.viewRow;
      const col = colLookup ? colLookup[flash.viewCol] : flash.viewCol;
      if (row !== null && row !== undefined && col !== null && col !== undefined &&
          row >= firstRow && row <= lastRow && col >= firstCol && col <= lastCol) {
        const y = colHeaderHeight + (row * rowHeight - scrollTop);
        for (let p = 0; p < paneCount; p++) {
          const x = paneOffsets[p] + (col * colWidth - scrollLeft);
          ctx.strokeRect(x + 1, y + 1, colWidth - 2, rowHeight - 2);
        }
      }
    }

    ctx.restore();
  }
}
