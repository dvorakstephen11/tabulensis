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

function resolveCellText(cell, side) {
  if (!cell) return "";
  if (cell.edit) {
    const value = side === "old" ? cell.edit.fromValue : cell.edit.toValue;
    const formula = side === "old" ? cell.edit.fromFormula : cell.edit.toFormula;
    if (value) return value;
    if (formula) return formula;
  }
  const payload = side === "old" ? cell.old : cell.new;
  if (!payload || !payload.cell) return "";
  return payload.cell.value || payload.cell.formula || "";
}

function resolveUnifiedText(cell) {
  if (!cell) return "";
  if (cell.diffKind === "added") return resolveCellText(cell, "new");
  if (cell.diffKind === "removed") return resolveCellText(cell, "old");
  if (cell.diffKind === "moved") {
    return cell.moveRole === "src" ? resolveCellText(cell, "old") : resolveCellText(cell, "new");
  }
  return resolveCellText(cell, "new") || resolveCellText(cell, "old");
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

function drawEditedUnified(ctx, x, y, width, height, cell, theme) {
  const paddingX = 8;
  const paddingY = 6;
  const maxWidth = Math.max(0, width - paddingX * 2);
  const oldText = resolveCellText(cell, "old");
  const newText = resolveCellText(cell, "new");

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

function drawCell(ctx, x, y, width, height, cell, theme, mode, side) {
  const style = cellStyle(cell, theme);
  ctx.fillStyle = style.bg;
  ctx.fillRect(x, y, width, height);
  ctx.strokeStyle = theme.borderSecondary;
  ctx.lineWidth = 1;
  ctx.strokeRect(x, y, width, height);

  if (!cell || cell.diffKind === "empty") return;

  if (mode === "unified" && cell.diffKind === "edited") {
    drawEditedUnified(ctx, x, y, width, height, cell, theme);
    return;
  }

  let text = "";
  if (mode === "side_by_side") {
    text = resolveCellText(cell, side);
  } else {
    text = resolveUnifiedText(cell);
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
  const { sheetVm, mode, viewport, metrics, theme, selection, hover, hoverMoveId } = model;
  const { width, height, scrollTop, scrollLeft } = viewport;
  const rows = sheetVm.axis.rows.entries;
  const cols = sheetVm.axis.cols.entries;

  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = theme.bgPrimary;
  ctx.fillRect(0, 0, width, height);

  if (!rows.length || !cols.length || width <= 0 || height <= 0) return;

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
  const lastRow = Math.min(rows.length - 1, firstRow + visibleRowCount - 1);

  const firstCol = Math.max(0, Math.floor(scrollLeft / colWidth));
  const visibleColCount = paneWidth > 0 ? Math.ceil(paneWidth / colWidth) + 1 : 0;
  const lastCol = Math.min(cols.length - 1, firstCol + visibleColCount - 1);

  const paneOffsets = [rowHeaderWidth];
  if (paneCount === 2) {
    paneOffsets.push(rowHeaderWidth + paneWidth + paneGap);
  }

  ctx.fillStyle = theme.bgTertiary;
  ctx.fillRect(0, 0, rowHeaderWidth, colHeaderHeight);

  for (let c = firstCol; c <= lastCol; c++) {
    const entry = cols[c];
    const style = axisStyle(entry, "col", theme);
    const label = colToLetter(c);
    for (let p = 0; p < paneCount; p++) {
      const x = paneOffsets[p] + (c * colWidth - scrollLeft);
      drawHeaderCell(ctx, x, 0, colWidth, colHeaderHeight, label, style, theme);
    }
  }

  for (let r = firstRow; r <= lastRow; r++) {
    const entry = rows[r];
    const style = axisStyle(entry, "row", theme);
    const y = colHeaderHeight + (r * rowHeight - scrollTop);
    drawRowHeader(ctx, 0, y, rowHeaderWidth, rowHeight, String(r + 1), style, theme);
  }

  for (let r = firstRow; r <= lastRow; r++) {
    const y = colHeaderHeight + (r * rowHeight - scrollTop);
    for (let c = firstCol; c <= lastCol; c++) {
      const cell = sheetVm.cellAt(r, c);
      for (let p = 0; p < paneCount; p++) {
        const x = paneOffsets[p] + (c * colWidth - scrollLeft);
        const side = p === 0 ? "old" : "new";
        drawCell(ctx, x, y, colWidth, rowHeight, cell, theme, mode, side);
      }

      if (hoverMoveId) {
        const rowEntry = rows[r];
        const colEntry = cols[c];
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
    const row = hover.viewRow;
    const col = hover.viewCol;
    if (row >= firstRow && row <= lastRow && col >= firstCol && col <= lastCol) {
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
    const row = selection.viewRow;
    const col = selection.viewCol;
    if (row >= firstRow && row <= lastRow && col >= firstCol && col <= lastCol) {
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
}
