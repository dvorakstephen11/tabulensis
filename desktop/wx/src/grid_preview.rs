use std::collections::{HashMap, HashSet};

use excel_diff::{DiffOp, DiffReport};

#[derive(Default)]
struct SheetGridMarks {
    edited_cells: HashSet<(u32, u32)>,
    rect_replaced: Vec<GridRect>,
    old_row_removed: HashSet<u32>,
    new_row_added: HashSet<u32>,
    row_replaced: HashSet<u32>,
    old_row_cluster: HashSet<u32>,
    new_row_cluster: HashSet<u32>,
    old_col_removed: HashSet<u32>,
    new_col_added: HashSet<u32>,
    old_move_src_rows: HashSet<u32>,
    new_move_dst_rows: HashSet<u32>,
    old_move_src_cols: HashSet<u32>,
    new_move_dst_cols: HashSet<u32>,
    old_move_src_rects: Vec<GridRect>,
    new_move_dst_rects: Vec<GridRect>,
}

#[derive(Clone, Copy)]
struct GridRect {
    row_start: u32,
    row_end: u32,
    col_start: u32,
    col_end: u32,
}

impl GridRect {
    fn contains(&self, row: u32, col: u32) -> bool {
        row >= self.row_start && row <= self.row_end && col >= self.col_start && col <= self.col_end
    }
}

#[derive(Clone, Copy)]
struct GridViewport {
    row_start: u32,
    row_end: u32,
    col_start: u32,
    col_end: u32,
    cropped: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GridSide {
    Old,
    New,
}

pub(crate) fn build_sheet_grid_preview_html(
    sheet_name: &str,
    payload: &ui_payload::DiffWithSheets,
) -> String {
    use ui_payload::{STRUCTURAL_PREVIEW_MAX_COLS, STRUCTURAL_PREVIEW_MAX_ROWS};

    let report = &payload.report;
    let old_sheet_name =
        resolve_old_sheet_name(report, sheet_name).unwrap_or_else(|| sheet_name.to_string());
    let old_sheet = payload
        .sheets
        .old
        .sheets
        .iter()
        .find(|sheet| sheet.name.eq_ignore_ascii_case(&old_sheet_name));
    let new_sheet = payload
        .sheets
        .new
        .sheets
        .iter()
        .find(|sheet| sheet.name.eq_ignore_ascii_case(sheet_name));

    let old_dims = old_sheet.map(|s| (s.nrows, s.ncols)).unwrap_or((0, 0));
    let new_dims = new_sheet.map(|s| (s.nrows, s.ncols)).unwrap_or((0, 0));

    let marks = build_grid_marks(report, sheet_name);

    let old_preview_rows = old_dims.0.min(STRUCTURAL_PREVIEW_MAX_ROWS);
    let old_preview_cols = old_dims.1.min(STRUCTURAL_PREVIEW_MAX_COLS);
    let new_preview_rows = new_dims.0.min(STRUCTURAL_PREVIEW_MAX_ROWS);
    let new_preview_cols = new_dims.1.min(STRUCTURAL_PREVIEW_MAX_COLS);

    let old_vp = compute_viewport(
        &marks,
        old_dims,
        old_preview_rows,
        old_preview_cols,
        GridSide::Old,
    );
    let new_vp = compute_viewport(
        &marks,
        new_dims,
        new_preview_rows,
        new_preview_cols,
        GridSide::New,
    );

    let mut html = String::new();
    html.push_str("<!doctype html><html><head><meta charset=\"utf-8\" />");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\" />");
    html.push_str("<style>");
    html.push_str(
        r#":root{
  --bg:#0d1117;
  --panel:#161b22;
  --border:#30363d;
  --border2:#21262d;
  --text:#e6edf3;
  --muted:#8b949e;
  --mono:ui-monospace,"SFMono-Regular",Menlo,Consolas,"Liberation Mono",monospace;
  --sans:system-ui,-apple-system,"Segoe UI",Arial,sans-serif;
  --add-bg:rgba(46,160,67,.18);
  --add-br:rgba(46,160,67,.45);
  --rm-bg:rgba(248,81,73,.18);
  --rm-br:rgba(248,81,73,.45);
  --edit-bg:rgba(210,153,34,.22);
  --edit-br:rgba(210,153,34,.50);
  --move-bg:rgba(163,113,247,.22);
  --move-br:rgba(163,113,247,.55);
  --cluster-bg:rgba(88,166,255,.18);
  --cluster-br:rgba(88,166,255,.45);
}
body{margin:0;background:var(--bg);color:var(--text);font-family:var(--sans);}
.wrap{padding:12px;}
.top{display:flex;flex-wrap:wrap;gap:10px;align-items:center;justify-content:space-between;margin-bottom:10px;}
.title{font-size:13px;font-weight:600;}
.meta{font-family:var(--mono);font-size:12px;color:var(--muted);}
.cols{display:grid;grid-template-columns:1fr 1fr;gap:12px;}
@media (max-width:1100px){.cols{grid-template-columns:1fr;}}
.card{background:var(--panel);border:1px solid var(--border);border-radius:12px;padding:10px;min-width:0;}
.card h3{margin:0 0 6px 0;font-size:12px;color:var(--muted);font-weight:600;letter-spacing:.3px;text-transform:uppercase;}
.note{margin-top:6px;font-size:12px;color:var(--muted);white-space:pre-line;}
.gridwrap{overflow:auto;border:1px solid var(--border);border-radius:10px;background:#0b0f14;}
table{border-collapse:collapse;font-family:var(--mono);font-size:11px;}
th,td{border-right:1px solid var(--border2);border-bottom:1px solid var(--border2);padding:4px 6px;min-width:88px;max-width:240px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}
th.corner{min-width:42px;max-width:42px;background:#21262d;position:sticky;left:0;z-index:3;}
th.col{background:#21262d;color:var(--muted);position:sticky;top:0;z-index:2;text-align:center;}
th.row{min-width:42px;max-width:42px;background:#21262d;color:var(--muted);position:sticky;left:0;z-index:1;text-align:right;}
td{background:#0d1117;}
.row-edited,.col-edited{background:var(--edit-bg);color:var(--text);}
.row-added,.col-added{background:var(--add-bg);color:var(--text);}
.row-removed,.col-removed{background:var(--rm-bg);color:var(--text);}
.row-move,.col-move{background:var(--move-bg);color:var(--text);}
.row-cluster{background:var(--cluster-bg);color:var(--text);}
.cell-added{background:var(--add-bg);}
.cell-removed{background:var(--rm-bg);}
.cell-edited{background:var(--edit-bg);box-shadow:inset 0 0 0 1px var(--edit-br);}
.cell-move-src{background:var(--move-bg);box-shadow:inset 0 0 0 1px var(--move-br);}
.cell-move-dst{background:rgba(163,113,247,.32);box-shadow:inset 0 0 0 1px var(--move-br);}
.cell-cluster{background:var(--cluster-bg);box-shadow:inset 0 0 0 1px var(--cluster-br);}
.legend{display:flex;flex-wrap:wrap;gap:10px;margin-top:10px;font-size:12px;color:var(--muted);}
.legend .item{display:flex;gap:6px;align-items:center;}
.swatch{width:12px;height:12px;border-radius:3px;border:1px solid var(--border);}
.sw-add{background:var(--add-bg);border-color:var(--add-br);}
.sw-rm{background:var(--rm-bg);border-color:var(--rm-br);}
.sw-edit{background:var(--edit-bg);border-color:var(--edit-br);}
.sw-move{background:var(--move-bg);border-color:var(--move-br);}
.sw-cluster{background:var(--cluster-bg);border-color:var(--cluster-br);}
"#,
    );
    html.push_str("</style></head><body><div class=\"wrap\">");

    html.push_str("<div class=\"top\">");
    html.push_str(&format!(
        "<div class=\"title\">Sheet: {}</div>",
        escape_html(sheet_name)
    ));
    html.push_str(&format!(
        "<div class=\"meta\">old {}x{} | new {}x{}</div>",
        old_dims.0, old_dims.1, new_dims.0, new_dims.1
    ));
    html.push_str("</div>");

    html.push_str("<div class=\"cols\">");
    html.push_str(&render_side_grid_html(
        sheet_name,
        "Old",
        old_sheet,
        old_vp,
        &marks,
        GridSide::Old,
        old_preview_rows,
        old_preview_cols,
    ));
    html.push_str(&render_side_grid_html(
        sheet_name,
        "New",
        new_sheet,
        new_vp,
        &marks,
        GridSide::New,
        new_preview_rows,
        new_preview_cols,
    ));
    html.push_str("</div>");

    html.push_str(
        r#"<div class="legend">
  <div class="item"><span class="swatch sw-add"></span><span>added</span></div>
  <div class="item"><span class="swatch sw-rm"></span><span>removed</span></div>
  <div class="item"><span class="swatch sw-edit"></span><span>edited</span></div>
  <div class="item"><span class="swatch sw-move"></span><span>moved</span></div>
  <div class="item"><span class="swatch sw-cluster"></span><span>duplicate key cluster</span></div>
</div>"#,
    );

    if old_sheet.is_none() && new_sheet.is_some() {
        html.push_str("<div class=\"note\">Note: sheet is new (old side missing).</div>");
    } else if old_sheet.is_some() && new_sheet.is_none() {
        html.push_str("<div class=\"note\">Note: sheet removed (new side missing).</div>");
    } else if !old_sheet_name.eq_ignore_ascii_case(sheet_name) {
        html.push_str(&format!(
            "<div class=\"note\">Note: renamed from <span class=\"meta\">{}</span>.</div>",
            escape_html(&old_sheet_name)
        ));
    }

    html.push_str(
        r#"<script>
(function () {
  var post = window.__tabulensisPostMessage;
  if (typeof post !== "function") return;
  document.addEventListener("click", function (event) {
    var target = event.target;
    if (!target || !target.closest) return;
    var td = target.closest("td");
    if (!td) return;
    var wrap = td.closest(".gridwrap");
    if (!wrap) return;
    var row = parseInt(td.getAttribute("data-row") || "", 10);
    var col = parseInt(td.getAttribute("data-col") || "", 10);
    if (!isFinite(row) || !isFinite(col)) return;
    var sheetName = wrap.getAttribute("data-sheet") || "";
    var side = wrap.getAttribute("data-side") || "";
    try {
      post(JSON.stringify({ method: "gridCellClick", params: { sheetName: sheetName, side: side, row: row, col: col } }));
    } catch (err) {
      // No-op: keep preview rendering stable even if the bridge isn't available.
    }
  });
})();
</script>"#,
    );
    html.push_str("</div></body></html>");
    html
}

fn render_side_grid_html(
    sheet_name: &str,
    label: &str,
    sheet: Option<&ui_payload::SheetSnapshot>,
    vp: Option<GridViewport>,
    marks: &SheetGridMarks,
    side: GridSide,
    preview_rows: u32,
    preview_cols: u32,
) -> String {
    let mut out = String::new();
    out.push_str("<div class=\"card\">");
    out.push_str(&format!("<h3>{}</h3>", escape_html(label)));
    let Some(sheet) = sheet else {
        out.push_str("<div class=\"note\">Sheet not present on this side.</div>");
        out.push_str("</div>");
        return out;
    };
    let Some(vp) = vp else {
        out.push_str("<div class=\"note\">No grid preview available.</div>");
        out.push_str("</div>");
        return out;
    };

    let mut note_lines: Vec<String> = Vec::new();
    note_lines.push(format!(
        "Showing rows {}-{} and cols {}-{}{}",
        vp.row_start + 1,
        vp.row_end + 1,
        col_to_letter(vp.col_start),
        col_to_letter(vp.col_end),
        if vp.cropped { " (cropped)" } else { "" }
    ));
    if sheet.truncated {
        note_lines.push("Preview limited: snapshot truncated.".to_string());
    }
    if let Some(note) = sheet.note.as_deref() {
        note_lines.push(note.to_string());
    }
    if preview_rows < sheet.nrows || preview_cols < sheet.ncols {
        note_lines.push(format!(
            "Structural preview cap: {} rows x {} cols.",
            preview_rows, preview_cols
        ));
    }

    let cell_map = build_cell_lookup(&sheet.cells, &vp);
    out.push_str(&format!(
        "<div class=\"gridwrap\" data-sheet=\"{}\" data-side=\"{}\">",
        escape_html(sheet_name),
        if side == GridSide::Old { "old" } else { "new" }
    ));
    out.push_str("<table><thead><tr>");
    out.push_str("<th class=\"corner\"></th>");
    for col in vp.col_start..=vp.col_end {
        let class = match side {
            GridSide::Old => {
                if marks.old_col_removed.contains(&col) {
                    "col col-removed"
                } else if marks.old_move_src_cols.contains(&col)
                    || marks
                        .old_move_src_rects
                        .iter()
                        .any(|rect| col >= rect.col_start && col <= rect.col_end)
                {
                    "col col-move"
                } else {
                    "col"
                }
            }
            GridSide::New => {
                if marks.new_col_added.contains(&col) {
                    "col col-added"
                } else if marks.new_move_dst_cols.contains(&col)
                    || marks
                        .new_move_dst_rects
                        .iter()
                        .any(|rect| col >= rect.col_start && col <= rect.col_end)
                {
                    "col col-move"
                } else {
                    "col"
                }
            }
        };
        out.push_str(&format!(
            "<th class=\"{}\">{}</th>",
            class,
            escape_html(&col_to_letter(col))
        ));
    }
    out.push_str("</tr></thead><tbody>");

    for row in vp.row_start..=vp.row_end {
        out.push_str("<tr>");
        let row_class = match side {
            GridSide::Old => {
                if marks.old_row_removed.contains(&row) {
                    "row row-removed"
                } else if marks.old_move_src_rows.contains(&row)
                    || marks
                        .old_move_src_rects
                        .iter()
                        .any(|rect| row >= rect.row_start && row <= rect.row_end)
                {
                    "row row-move"
                } else if marks.row_replaced.contains(&row) {
                    "row row-edited"
                } else if marks.old_row_cluster.contains(&row) {
                    "row row-cluster"
                } else {
                    "row"
                }
            }
            GridSide::New => {
                if marks.new_row_added.contains(&row) {
                    "row row-added"
                } else if marks.new_move_dst_rows.contains(&row)
                    || marks
                        .new_move_dst_rects
                        .iter()
                        .any(|rect| row >= rect.row_start && row <= rect.row_end)
                {
                    "row row-move"
                } else if marks.row_replaced.contains(&row) {
                    "row row-edited"
                } else if marks.new_row_cluster.contains(&row) {
                    "row row-cluster"
                } else {
                    "row"
                }
            }
        };
        out.push_str(&format!("<th class=\"{}\">{}</th>", row_class, row + 1));

        for col in vp.col_start..=vp.col_end {
            let class = cell_class_for(marks, side, row, col);
            let key = cell_key(row, col);
            let cell = cell_map.get(&key).copied();
            let (display, title) = format_cell_display(cell);
            if title.is_empty() {
                out.push_str(&format!(
                    "<td class=\"{}\" data-row=\"{}\" data-col=\"{}\">{}</td>",
                    class, row, col, display
                ));
            } else {
                out.push_str(&format!(
                    "<td class=\"{}\" data-row=\"{}\" data-col=\"{}\" title=\"{}\">{}</td>",
                    class, row, col, title, display
                ));
            }
        }
        out.push_str("</tr>");
    }

    out.push_str("</tbody></table></div>");
    out.push_str("<div class=\"note\">");
    out.push_str(&escape_html(&note_lines.join("\n")));
    out.push_str("</div>");
    out.push_str("</div>");
    out
}

fn compute_viewport(
    marks: &SheetGridMarks,
    dims: (u32, u32),
    preview_rows: u32,
    preview_cols: u32,
    side: GridSide,
) -> Option<GridViewport> {
    use ui_payload::{
        SNAPSHOT_CONTEXT_COLS, SNAPSHOT_CONTEXT_ROWS, STRUCTURAL_PREVIEW_MAX_COLS,
        STRUCTURAL_PREVIEW_MAX_ROWS,
    };
    const MAX_RENDER_ROWS: u32 = STRUCTURAL_PREVIEW_MAX_ROWS;
    const MAX_RENDER_COLS: u32 = STRUCTURAL_PREVIEW_MAX_COLS;

    let (nrows, ncols) = dims;
    if nrows == 0 || ncols == 0 {
        return None;
    }

    let mut bounds: Option<GridRect> = None;
    let mut push_bounds = |rect: Option<GridRect>| {
        let Some(rect) = rect else {
            return;
        };
        bounds = Some(match bounds {
            None => rect,
            Some(existing) => GridRect {
                row_start: existing.row_start.min(rect.row_start),
                row_end: existing.row_end.max(rect.row_end),
                col_start: existing.col_start.min(rect.col_start),
                col_end: existing.col_end.max(rect.col_end),
            },
        });
    };

    let preview_rows = preview_rows.min(nrows);
    let preview_cols = preview_cols.min(ncols);

    for &(row, col) in &marks.edited_cells {
        push_bounds(rect_from_range(row, 1, col, 1, nrows, ncols));
    }
    for rect in &marks.rect_replaced {
        push_bounds(rect_from_rect(*rect, nrows, ncols));
    }

    match side {
        GridSide::Old => {
            for row in &marks.old_row_removed {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for row in &marks.row_replaced {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for row in &marks.old_row_cluster {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for row in &marks.old_move_src_rows {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for col in &marks.old_col_removed {
                push_bounds(rect_from_range(0, preview_rows, *col, 1, nrows, ncols));
            }
            for col in &marks.old_move_src_cols {
                push_bounds(rect_from_range(0, preview_rows, *col, 1, nrows, ncols));
            }
            for rect in &marks.old_move_src_rects {
                push_bounds(rect_from_rect(*rect, nrows, ncols));
            }
        }
        GridSide::New => {
            for row in &marks.new_row_added {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for row in &marks.row_replaced {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for row in &marks.new_row_cluster {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for row in &marks.new_move_dst_rows {
                push_bounds(rect_from_range(*row, 1, 0, preview_cols, nrows, ncols));
            }
            for col in &marks.new_col_added {
                push_bounds(rect_from_range(0, preview_rows, *col, 1, nrows, ncols));
            }
            for col in &marks.new_move_dst_cols {
                push_bounds(rect_from_range(0, preview_rows, *col, 1, nrows, ncols));
            }
            for rect in &marks.new_move_dst_rects {
                push_bounds(rect_from_rect(*rect, nrows, ncols));
            }
        }
    }

    let mut rect = bounds.unwrap_or_else(|| GridRect {
        row_start: 0,
        row_end: (MAX_RENDER_ROWS.saturating_sub(1)).min(nrows.saturating_sub(1)),
        col_start: 0,
        col_end: (MAX_RENDER_COLS.saturating_sub(1)).min(ncols.saturating_sub(1)),
    });

    rect.row_start = rect.row_start.saturating_sub(SNAPSHOT_CONTEXT_ROWS);
    rect.col_start = rect.col_start.saturating_sub(SNAPSHOT_CONTEXT_COLS);
    rect.row_end = rect
        .row_end
        .saturating_add(SNAPSHOT_CONTEXT_ROWS)
        .min(nrows.saturating_sub(1));
    rect.col_end = rect
        .col_end
        .saturating_add(SNAPSHOT_CONTEXT_COLS)
        .min(ncols.saturating_sub(1));

    let mut cropped = false;
    let span_rows = rect
        .row_end
        .saturating_sub(rect.row_start)
        .saturating_add(1);
    if span_rows > MAX_RENDER_ROWS {
        rect.row_end = rect
            .row_start
            .saturating_add(MAX_RENDER_ROWS)
            .saturating_sub(1);
        cropped = true;
    }
    let span_cols = rect
        .col_end
        .saturating_sub(rect.col_start)
        .saturating_add(1);
    if span_cols > MAX_RENDER_COLS {
        rect.col_end = rect
            .col_start
            .saturating_add(MAX_RENDER_COLS)
            .saturating_sub(1);
        cropped = true;
    }

    Some(GridViewport {
        row_start: rect.row_start,
        row_end: rect.row_end,
        col_start: rect.col_start,
        col_end: rect.col_end,
        cropped,
    })
}

fn rect_from_rect(rect: GridRect, nrows: u32, ncols: u32) -> Option<GridRect> {
    if nrows == 0 || ncols == 0 {
        return None;
    }
    Some(GridRect {
        row_start: rect.row_start.min(nrows.saturating_sub(1)),
        row_end: rect.row_end.min(nrows.saturating_sub(1)),
        col_start: rect.col_start.min(ncols.saturating_sub(1)),
        col_end: rect.col_end.min(ncols.saturating_sub(1)),
    })
}

fn rect_from_range(
    row_start: u32,
    row_count: u32,
    col_start: u32,
    col_count: u32,
    nrows: u32,
    ncols: u32,
) -> Option<GridRect> {
    if row_count == 0 || col_count == 0 || nrows == 0 || ncols == 0 {
        return None;
    }
    if row_start >= nrows || col_start >= ncols {
        return None;
    }
    let row_end = row_start
        .saturating_add(row_count)
        .saturating_sub(1)
        .min(nrows.saturating_sub(1));
    let col_end = col_start
        .saturating_add(col_count)
        .saturating_sub(1)
        .min(ncols.saturating_sub(1));
    Some(GridRect {
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

fn build_grid_marks(report: &DiffReport, sheet_name: &str) -> SheetGridMarks {
    let mut marks = SheetGridMarks::default();

    for op in &report.ops {
        let sheet_id = match op {
            DiffOp::SheetAdded { sheet }
            | DiffOp::SheetRemoved { sheet }
            | DiffOp::SheetRenamed { sheet, .. }
            | DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::RowReplaced { sheet, .. }
            | DiffOp::DuplicateKeyCluster { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::BlockMovedRows { sheet, .. }
            | DiffOp::BlockMovedColumns { sheet, .. }
            | DiffOp::BlockMovedRect { sheet, .. }
            | DiffOp::RectReplaced { sheet, .. }
            | DiffOp::CellEdited { sheet, .. } => Some(*sheet),
            _ => None,
        };
        let Some(sheet_id) = sheet_id else {
            continue;
        };
        let resolved = report.resolve(sheet_id).unwrap_or("");
        if !resolved.eq_ignore_ascii_case(sheet_name) {
            continue;
        }

        match op {
            DiffOp::RowAdded { row_idx, .. } => {
                marks.new_row_added.insert(*row_idx);
            }
            DiffOp::RowRemoved { row_idx, .. } => {
                marks.old_row_removed.insert(*row_idx);
            }
            DiffOp::RowReplaced { row_idx, .. } => {
                marks.row_replaced.insert(*row_idx);
            }
            DiffOp::DuplicateKeyCluster {
                left_rows,
                right_rows,
                ..
            } => {
                marks.old_row_cluster.extend(left_rows.iter().copied());
                marks.new_row_cluster.extend(right_rows.iter().copied());
            }
            DiffOp::ColumnAdded { col_idx, .. } => {
                marks.new_col_added.insert(*col_idx);
            }
            DiffOp::ColumnRemoved { col_idx, .. } => {
                marks.old_col_removed.insert(*col_idx);
            }
            DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                ..
            } => {
                let src_end = src_start_row.saturating_add(*row_count);
                for row in *src_start_row..src_end {
                    marks.old_move_src_rows.insert(row);
                }
                let dst_end = dst_start_row.saturating_add(*row_count);
                for row in *dst_start_row..dst_end {
                    marks.new_move_dst_rows.insert(row);
                }
            }
            DiffOp::BlockMovedColumns {
                src_start_col,
                col_count,
                dst_start_col,
                ..
            } => {
                let src_end = src_start_col.saturating_add(*col_count);
                for col in *src_start_col..src_end {
                    marks.old_move_src_cols.insert(col);
                }
                let dst_end = dst_start_col.saturating_add(*col_count);
                for col in *dst_start_col..dst_end {
                    marks.new_move_dst_cols.insert(col);
                }
            }
            DiffOp::BlockMovedRect {
                src_start_row,
                src_row_count,
                src_start_col,
                src_col_count,
                dst_start_row,
                dst_start_col,
                ..
            } => {
                if *src_row_count > 0 && *src_col_count > 0 {
                    marks.old_move_src_rects.push(GridRect {
                        row_start: *src_start_row,
                        row_end: src_start_row
                            .saturating_add(*src_row_count)
                            .saturating_sub(1),
                        col_start: *src_start_col,
                        col_end: src_start_col
                            .saturating_add(*src_col_count)
                            .saturating_sub(1),
                    });
                    marks.new_move_dst_rects.push(GridRect {
                        row_start: *dst_start_row,
                        row_end: dst_start_row
                            .saturating_add(*src_row_count)
                            .saturating_sub(1),
                        col_start: *dst_start_col,
                        col_end: dst_start_col
                            .saturating_add(*src_col_count)
                            .saturating_sub(1),
                    });
                }
            }
            DiffOp::RectReplaced {
                start_row,
                row_count,
                start_col,
                col_count,
                ..
            } => {
                if *row_count > 0 && *col_count > 0 {
                    marks.rect_replaced.push(GridRect {
                        row_start: *start_row,
                        row_end: start_row.saturating_add(*row_count).saturating_sub(1),
                        col_start: *start_col,
                        col_end: start_col.saturating_add(*col_count).saturating_sub(1),
                    });
                }
            }
            DiffOp::CellEdited { addr, .. } => {
                marks.edited_cells.insert((addr.row, addr.col));
            }
            _ => {}
        }
    }

    marks
}

fn resolve_old_sheet_name(report: &DiffReport, sheet_name: &str) -> Option<String> {
    for op in &report.ops {
        let DiffOp::SheetRenamed { sheet, from, .. } = op else {
            continue;
        };
        let new_name = report.resolve(*sheet).unwrap_or("");
        if new_name.eq_ignore_ascii_case(sheet_name) {
            return Some(report.resolve(*from).unwrap_or("").to_string());
        }
    }
    None
}

fn build_cell_lookup<'a>(
    cells: &'a [ui_payload::SheetCell],
    vp: &GridViewport,
) -> HashMap<u64, &'a ui_payload::SheetCell> {
    let mut map = HashMap::new();
    for cell in cells {
        if cell.row < vp.row_start
            || cell.row > vp.row_end
            || cell.col < vp.col_start
            || cell.col > vp.col_end
        {
            continue;
        }
        map.insert(cell_key(cell.row, cell.col), cell);
    }
    map
}

fn cell_key(row: u32, col: u32) -> u64 {
    ((row as u64) << 32) | (col as u64)
}

fn cell_class_for(marks: &SheetGridMarks, side: GridSide, row: u32, col: u32) -> &'static str {
    if marks.edited_cells.contains(&(row, col)) {
        return "cell-edited";
    }
    for rect in &marks.rect_replaced {
        if rect.contains(row, col) {
            return "cell-edited";
        }
    }
    match side {
        GridSide::Old => {
            for rect in &marks.old_move_src_rects {
                if rect.contains(row, col) {
                    return "cell-move-src";
                }
            }
            if marks.old_row_removed.contains(&row) || marks.old_col_removed.contains(&col) {
                return "cell-removed";
            }
            if marks.old_move_src_rows.contains(&row) || marks.old_move_src_cols.contains(&col) {
                return "cell-move-src";
            }
            if marks.row_replaced.contains(&row) {
                return "cell-edited";
            }
            if marks.old_row_cluster.contains(&row) {
                return "cell-cluster";
            }
        }
        GridSide::New => {
            for rect in &marks.new_move_dst_rects {
                if rect.contains(row, col) {
                    return "cell-move-dst";
                }
            }
            if marks.new_row_added.contains(&row) || marks.new_col_added.contains(&col) {
                return "cell-added";
            }
            if marks.new_move_dst_rows.contains(&row) || marks.new_move_dst_cols.contains(&col) {
                return "cell-move-dst";
            }
            if marks.row_replaced.contains(&row) {
                return "cell-edited";
            }
            if marks.new_row_cluster.contains(&row) {
                return "cell-cluster";
            }
        }
    }
    ""
}

fn format_cell_display(cell: Option<&ui_payload::SheetCell>) -> (String, String) {
    const MAX_LEN: usize = 28;
    let Some(cell) = cell else {
        return (String::new(), String::new());
    };
    let value = cell.value.as_deref().unwrap_or("");
    let formula = cell.formula.as_deref().unwrap_or("");
    let raw = if !value.is_empty() { value } else { formula };
    let mut display = raw.to_string();
    if display.len() > MAX_LEN {
        display.truncate(MAX_LEN.saturating_sub(3));
        display.push_str("...");
    }
    let display = escape_html(&display);

    let mut title = String::new();
    if !value.is_empty() {
        title.push_str(value);
    }
    if !formula.is_empty() {
        if !title.is_empty() {
            title.push_str("\n");
        }
        title.push_str(formula);
    }
    (display, escape_html(&title))
}

fn col_to_letter(col: u32) -> String {
    let mut c = col as i64;
    let mut out = String::new();
    while c >= 0 {
        let rem = (c % 26) as u8;
        out.insert(0, (b'A' + rem) as char);
        c = (c / 26) - 1;
    }
    out
}

pub(crate) fn escape_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
