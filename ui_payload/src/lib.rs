use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;

mod alignment;
mod analysis;
mod capabilities;
mod options;
mod outcome;

pub use alignment::SheetAlignment;
pub use analysis::{
    analyze_report, CategoryBreakdownRow, CategoryCounts, DiffAnalysis, NoiseFilters, OpCategory,
    OpNoiseClass, OpSeverity, SeverityCounts, SheetBreakdown,
};
pub use capabilities::{HostCapabilities, HostDefaults};
pub use options::{limits_from_config, DiffLimits, DiffOptions, DiffPreset};
pub use outcome::{
    summarize_report, ChangeCounts, DiffOutcome, DiffOutcomeConfig, DiffOutcomeMode,
    DiffOutcomeSummary, SheetSummary, SummaryMeta, SummarySink,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKind {
    Workbook,
    Pbix,
}

#[derive(Serialize)]
pub struct SheetCell {
    pub row: u32,
    pub col: u32,
    pub value: Option<String>,
    pub formula: Option<String>,
}

#[derive(Serialize)]
pub struct SheetSnapshot {
    pub name: String,
    pub nrows: u32,
    pub ncols: u32,
    pub cells: Vec<SheetCell>,
    pub truncated: bool,
    pub included_cells: u32,
    pub total_non_empty_cells: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestRect {
    pub id: String,
    pub kind: String,
    pub side: String,
    pub row_start: u32,
    pub row_end: u32,
    pub col_start: u32,
    pub col_end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetInterestRects {
    pub sheet: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rects: Vec<InterestRect>,
}

#[derive(Serialize)]
pub struct WorkbookSnapshot {
    pub sheets: Vec<SheetSnapshot>,
}

#[derive(Serialize)]
pub struct SheetPairSnapshot {
    pub old: WorkbookSnapshot,
    pub new: WorkbookSnapshot,
}

#[derive(Serialize)]
pub struct DiffWithSheets {
    pub report: excel_diff::DiffReport,
    pub sheets: SheetPairSnapshot,
    pub alignments: Vec<SheetAlignment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interest_rects: Vec<SheetInterestRects>,
}

pub const MAX_SNAPSHOT_CELLS_PER_SHEET: usize = 50_000;
pub const MAX_SNAPSHOT_CELLS_TOTAL: usize = 200_000;
pub const STRUCTURAL_PREVIEW_MAX_ROWS: u32 = 200;
pub const STRUCTURAL_PREVIEW_MAX_COLS: u32 = 80;
pub const SNAPSHOT_CONTEXT_ROWS: u32 = 1;
pub const SNAPSHOT_CONTEXT_COLS: u32 = 1;

struct SnapshotCaps {
    per_sheet: usize,
    max_rows: u32,
    max_cols: u32,
    context_rows: u32,
    context_cols: u32,
}

#[derive(Clone, Copy)]
struct Rect {
    row_start: u32,
    row_end: u32,
    col_start: u32,
    col_end: u32,
}

pub fn host_kind_from_name(name: &str) -> Option<HostKind> {
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "xlsx" | "xlsm" | "xltx" | "xltm" | "xlsb" => Some(HostKind::Workbook),
        "pbix" | "pbit" => Some(HostKind::Pbix),
        _ => None,
    }
}

pub fn host_kind_from_path(path: &Path) -> Option<HostKind> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" | "xlsm" | "xltx" | "xltm" | "xlsb" => Some(HostKind::Workbook),
        "pbix" | "pbit" => Some(HostKind::Pbix),
        _ => None,
    }
}

pub fn build_payload_from_workbooks(
    old_pkg: &excel_diff::WorkbookPackage,
    new_pkg: &excel_diff::WorkbookPackage,
    cfg: &excel_diff::DiffConfig,
) -> DiffWithSheets {
    let report = old_pkg.diff(new_pkg, cfg);
    build_payload_from_workbook_report(report, old_pkg, new_pkg)
}

pub fn build_payload_from_workbooks_with_progress(
    old_pkg: &excel_diff::WorkbookPackage,
    new_pkg: &excel_diff::WorkbookPackage,
    cfg: &excel_diff::DiffConfig,
    progress: &dyn excel_diff::ProgressCallback,
) -> DiffWithSheets {
    let report = old_pkg.diff_with_progress(new_pkg, cfg, progress);
    build_payload_from_workbook_report(report, old_pkg, new_pkg)
}

pub fn build_payload_from_pbix(
    old_pkg: &excel_diff::PbixPackage,
    new_pkg: &excel_diff::PbixPackage,
    cfg: &excel_diff::DiffConfig,
) -> DiffWithSheets {
    let report = old_pkg.diff(new_pkg, cfg);
    build_payload_from_pbix_report(report)
}

pub fn build_payload_from_pbix_report(report: excel_diff::DiffReport) -> DiffWithSheets {
    let empty = WorkbookSnapshot { sheets: Vec::new() };
    DiffWithSheets {
        report,
        sheets: SheetPairSnapshot {
            old: empty,
            new: WorkbookSnapshot { sheets: Vec::new() },
        },
        alignments: Vec::new(),
        interest_rects: Vec::new(),
    }
}

pub fn build_payload_from_workbook_report(
    report: excel_diff::DiffReport,
    old_pkg: &excel_diff::WorkbookPackage,
    new_pkg: &excel_diff::WorkbookPackage,
) -> DiffWithSheets {
    let sheet_ids = collect_sheet_ids(&report.ops);
    let ops_by_sheet = group_ops_by_sheet(&report);
    let caps = SnapshotCaps {
        per_sheet: MAX_SNAPSHOT_CELLS_PER_SHEET,
        max_rows: STRUCTURAL_PREVIEW_MAX_ROWS,
        max_cols: STRUCTURAL_PREVIEW_MAX_COLS,
        context_rows: SNAPSHOT_CONTEXT_ROWS,
        context_cols: SNAPSHOT_CONTEXT_COLS,
    };

    let sheets = excel_diff::with_default_session(|session| {
        let mut remaining = MAX_SNAPSHOT_CELLS_TOTAL;
        SheetPairSnapshot {
            old: snapshot_workbook(
                &old_pkg.workbook,
                &sheet_ids,
                &session.strings,
                &ops_by_sheet,
                &caps,
                &mut remaining,
            ),
            new: snapshot_workbook(
                &new_pkg.workbook,
                &sheet_ids,
                &session.strings,
                &ops_by_sheet,
                &caps,
                &mut remaining,
            ),
        }
    });

    let alignments = alignment::build_alignments(&report, &sheets);
    let old_lookup: HashMap<String, &SheetSnapshot> = sheets
        .old
        .sheets
        .iter()
        .map(|s| (s.name.clone(), s))
        .collect();
    let new_lookup: HashMap<String, &SheetSnapshot> = sheets
        .new
        .sheets
        .iter()
        .map(|s| (s.name.clone(), s))
        .collect();
    let mut interest_rects = Vec::new();
    for (sheet_name, ops) in &ops_by_sheet {
        let old_sheet = old_lookup.get(sheet_name).copied();
        let new_sheet = new_lookup.get(sheet_name).copied();
        let rects = collect_interest_rects_payload(ops, old_sheet, new_sheet, &caps);
        if rects.is_empty() {
            continue;
        }
        interest_rects.push(SheetInterestRects {
            sheet: sheet_name.clone(),
            rects,
        });
    }
    DiffWithSheets {
        report,
        sheets,
        alignments,
        interest_rects,
    }
}

pub fn build_alignment_for_sheet(
    sheet: &str,
    old_dims: Option<(u32, u32)>,
    new_dims: Option<(u32, u32)>,
    ops: &[excel_diff::DiffOp],
) -> SheetAlignment {
    let old_sheet = old_dims.map(|(nrows, ncols)| SheetSnapshot {
        name: sheet.to_string(),
        nrows,
        ncols,
        cells: Vec::new(),
        truncated: false,
        included_cells: 0,
        total_non_empty_cells: 0,
        note: None,
    });
    let new_sheet = new_dims.map(|(nrows, ncols)| SheetSnapshot {
        name: sheet.to_string(),
        nrows,
        ncols,
        cells: Vec::new(),
        truncated: false,
        included_cells: 0,
        total_non_empty_cells: 0,
        note: None,
    });
    let mut op_refs = Vec::with_capacity(ops.len());
    for op in ops {
        op_refs.push(op);
    }
    alignment::build_sheet_alignment_for(sheet, old_sheet.as_ref(), new_sheet.as_ref(), &op_refs)
}

fn collect_sheet_ids(ops: &[excel_diff::DiffOp]) -> HashSet<excel_diff::StringId> {
    let mut sheets = HashSet::new();
    for op in ops {
        if let excel_diff::DiffOp::SheetRenamed { sheet, from, .. } = op {
            sheets.insert(*sheet);
            sheets.insert(*from);
            continue;
        }
        let sheet = match op {
            excel_diff::DiffOp::SheetAdded { sheet }
            | excel_diff::DiffOp::SheetRemoved { sheet }
            | excel_diff::DiffOp::RowAdded { sheet, .. }
            | excel_diff::DiffOp::RowRemoved { sheet, .. }
            | excel_diff::DiffOp::RowReplaced { sheet, .. }
            | excel_diff::DiffOp::DuplicateKeyCluster { sheet, .. }
            | excel_diff::DiffOp::ColumnAdded { sheet, .. }
            | excel_diff::DiffOp::ColumnRemoved { sheet, .. }
            | excel_diff::DiffOp::BlockMovedRows { sheet, .. }
            | excel_diff::DiffOp::BlockMovedColumns { sheet, .. }
            | excel_diff::DiffOp::BlockMovedRect { sheet, .. }
            | excel_diff::DiffOp::RectReplaced { sheet, .. }
            | excel_diff::DiffOp::CellEdited { sheet, .. } => Some(*sheet),
            _ => None,
        };
        if let Some(sheet_id) = sheet {
            sheets.insert(sheet_id);
        }
    }
    sheets
}

fn group_ops_by_sheet(
    report: &excel_diff::DiffReport,
) -> HashMap<String, Vec<&excel_diff::DiffOp>> {
    let mut map: HashMap<String, Vec<&excel_diff::DiffOp>> = HashMap::new();
    for op in &report.ops {
        let sheet = match op {
            excel_diff::DiffOp::SheetAdded { sheet }
            | excel_diff::DiffOp::SheetRemoved { sheet }
            | excel_diff::DiffOp::SheetRenamed { sheet, .. }
            | excel_diff::DiffOp::RowAdded { sheet, .. }
            | excel_diff::DiffOp::RowRemoved { sheet, .. }
            | excel_diff::DiffOp::RowReplaced { sheet, .. }
            | excel_diff::DiffOp::DuplicateKeyCluster { sheet, .. }
            | excel_diff::DiffOp::ColumnAdded { sheet, .. }
            | excel_diff::DiffOp::ColumnRemoved { sheet, .. }
            | excel_diff::DiffOp::BlockMovedRows { sheet, .. }
            | excel_diff::DiffOp::BlockMovedColumns { sheet, .. }
            | excel_diff::DiffOp::BlockMovedRect { sheet, .. }
            | excel_diff::DiffOp::RectReplaced { sheet, .. }
            | excel_diff::DiffOp::CellEdited { sheet, .. } => Some(*sheet),
            _ => None,
        };
        let Some(sheet_id) = sheet else {
            continue;
        };
        let sheet_name = report.resolve(sheet_id).unwrap_or("<unknown>");
        map.entry(sheet_name.to_string())
            .or_insert_with(Vec::new)
            .push(op);
    }
    map
}

fn render_cell_value(
    pool: &excel_diff::StringPool,
    value: &Option<excel_diff::CellValue>,
) -> Option<String> {
    match value {
        None => None,
        Some(excel_diff::CellValue::Blank) => Some(String::new()),
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(id)) => Some(pool.resolve(*id).to_string()),
        Some(excel_diff::CellValue::Bool(b)) => Some(if *b {
            "TRUE".to_string()
        } else {
            "FALSE".to_string()
        }),
        Some(excel_diff::CellValue::Error(id)) => Some(pool.resolve(*id).to_string()),
    }
}

impl Rect {
    fn area(&self) -> u64 {
        let rows = self
            .row_end
            .saturating_sub(self.row_start)
            .saturating_add(1) as u64;
        let cols = self
            .col_end
            .saturating_sub(self.col_start)
            .saturating_add(1) as u64;
        rows.saturating_mul(cols)
    }

    fn contains(&self, row: u32, col: u32) -> bool {
        row >= self.row_start && row <= self.row_end && col >= self.col_start && col <= self.col_end
    }
}

fn clamp_range(start: u32, count: u32, max: u32) -> Option<(u32, u32)> {
    if count == 0 || max == 0 {
        return None;
    }
    if start >= max {
        return None;
    }
    let end = start.saturating_add(count).saturating_sub(1);
    let end = end.min(max.saturating_sub(1));
    Some((start, end))
}

fn rect_from_range(
    row_start: u32,
    row_count: u32,
    col_start: u32,
    col_count: u32,
    nrows: u32,
    ncols: u32,
) -> Option<Rect> {
    let (row_start, row_end) = clamp_range(row_start, row_count, nrows)?;
    let (col_start, col_end) = clamp_range(col_start, col_count, ncols)?;
    Some(Rect {
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

fn expand_rect(rect: Rect, context_rows: u32, context_cols: u32, nrows: u32, ncols: u32) -> Rect {
    if nrows == 0 || ncols == 0 {
        return rect;
    }
    let row_start = rect.row_start.saturating_sub(context_rows);
    let col_start = rect.col_start.saturating_sub(context_cols);
    let row_end = rect
        .row_end
        .saturating_add(context_rows)
        .min(nrows.saturating_sub(1));
    let col_end = rect
        .col_end
        .saturating_add(context_cols)
        .min(ncols.saturating_sub(1));
    Rect {
        row_start,
        row_end,
        col_start,
        col_end,
    }
}

fn collect_interest_rects(
    nrows: u32,
    ncols: u32,
    ops: &[&excel_diff::DiffOp],
    caps: &SnapshotCaps,
) -> Vec<Rect> {
    let mut rects = Vec::new();
    if nrows == 0 || ncols == 0 {
        return rects;
    }

    let preview_cols = caps.max_cols.min(ncols);
    let preview_rows = caps.max_rows.min(nrows);

    for op in ops {
        match op {
            excel_diff::DiffOp::CellEdited { addr, .. } => {
                if addr.row < nrows && addr.col < ncols {
                    if let Some(rect) = rect_from_range(addr.row, 1, addr.col, 1, nrows, ncols) {
                        rects.push(rect);
                    }
                }
            }
            excel_diff::DiffOp::RectReplaced {
                start_row,
                row_count,
                start_col,
                col_count,
                ..
            } => {
                if let Some(rect) =
                    rect_from_range(*start_row, *row_count, *start_col, *col_count, nrows, ncols)
                {
                    rects.push(rect);
                }
            }
            excel_diff::DiffOp::BlockMovedRect {
                src_start_row,
                src_row_count,
                src_start_col,
                src_col_count,
                dst_start_row,
                dst_start_col,
                ..
            } => {
                if let Some(rect) = rect_from_range(
                    *src_start_row,
                    *src_row_count,
                    *src_start_col,
                    *src_col_count,
                    nrows,
                    ncols,
                ) {
                    rects.push(rect);
                }
                if let Some(rect) = rect_from_range(
                    *dst_start_row,
                    *src_row_count,
                    *dst_start_col,
                    *src_col_count,
                    nrows,
                    ncols,
                ) {
                    rects.push(rect);
                }
            }
            excel_diff::DiffOp::RowAdded { row_idx, .. }
            | excel_diff::DiffOp::RowRemoved { row_idx, .. }
            | excel_diff::DiffOp::RowReplaced { row_idx, .. } => {
                if preview_cols == 0 {
                    continue;
                }
                if let Some(rect) = rect_from_range(*row_idx, 1, 0, preview_cols, nrows, ncols) {
                    rects.push(rect);
                }
            }
            excel_diff::DiffOp::DuplicateKeyCluster {
                left_rows,
                right_rows,
                ..
            } => {
                if preview_cols == 0 {
                    continue;
                }
                for row_idx in left_rows.iter().chain(right_rows.iter()) {
                    if let Some(rect) = rect_from_range(*row_idx, 1, 0, preview_cols, nrows, ncols)
                    {
                        rects.push(rect);
                    }
                }
            }
            excel_diff::DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                ..
            } => {
                if preview_cols == 0 {
                    continue;
                }
                if let Some(rect) =
                    rect_from_range(*src_start_row, *row_count, 0, preview_cols, nrows, ncols)
                {
                    rects.push(rect);
                }
                if let Some(rect) =
                    rect_from_range(*dst_start_row, *row_count, 0, preview_cols, nrows, ncols)
                {
                    rects.push(rect);
                }
            }
            excel_diff::DiffOp::ColumnAdded { col_idx, .. }
            | excel_diff::DiffOp::ColumnRemoved { col_idx, .. } => {
                if preview_rows == 0 {
                    continue;
                }
                if let Some(rect) = rect_from_range(0, preview_rows, *col_idx, 1, nrows, ncols) {
                    rects.push(rect);
                }
            }
            excel_diff::DiffOp::BlockMovedColumns {
                src_start_col,
                col_count,
                dst_start_col,
                ..
            } => {
                if preview_rows == 0 {
                    continue;
                }
                if let Some(rect) =
                    rect_from_range(0, preview_rows, *src_start_col, *col_count, nrows, ncols)
                {
                    rects.push(rect);
                }
                if let Some(rect) =
                    rect_from_range(0, preview_rows, *dst_start_col, *col_count, nrows, ncols)
                {
                    rects.push(rect);
                }
            }
            _ => {}
        }
    }

    rects
        .into_iter()
        .map(|rect| expand_rect(rect, caps.context_rows, caps.context_cols, nrows, ncols))
        .collect()
}

fn rect_with_context(
    row_start: u32,
    row_count: u32,
    col_start: u32,
    col_count: u32,
    nrows: u32,
    ncols: u32,
    caps: &SnapshotCaps,
) -> Option<Rect> {
    let rect = rect_from_range(row_start, row_count, col_start, col_count, nrows, ncols)?;
    Some(expand_rect(
        rect,
        caps.context_rows,
        caps.context_cols,
        nrows,
        ncols,
    ))
}

fn push_interest_rect(
    rects: &mut Vec<InterestRect>,
    seen: &mut HashSet<String>,
    counter: &mut u32,
    kind: &str,
    side: &str,
    rect: Rect,
    move_id: Option<String>,
) {
    let key = format!(
        "{}:{}:{}:{}:{}:{}:{:?}",
        kind, side, rect.row_start, rect.row_end, rect.col_start, rect.col_end, move_id
    );
    if !seen.insert(key) {
        return;
    }
    let id = format!("ir-{}", *counter);
    *counter = counter.saturating_add(1);
    rects.push(InterestRect {
        id,
        kind: kind.to_string(),
        side: side.to_string(),
        row_start: rect.row_start,
        row_end: rect.row_end,
        col_start: rect.col_start,
        col_end: rect.col_end,
        move_id,
    });
}

fn collect_interest_rects_payload(
    ops: &[&excel_diff::DiffOp],
    old_sheet: Option<&SheetSnapshot>,
    new_sheet: Option<&SheetSnapshot>,
    caps: &SnapshotCaps,
) -> Vec<InterestRect> {
    let old_rows = old_sheet.map(|s| s.nrows).unwrap_or(0);
    let old_cols = old_sheet.map(|s| s.ncols).unwrap_or(0);
    let new_rows = new_sheet.map(|s| s.nrows).unwrap_or(0);
    let new_cols = new_sheet.map(|s| s.ncols).unwrap_or(0);
    let min_rows = old_rows.min(new_rows);
    let min_cols = old_cols.min(new_cols);

    let preview_cols_old = caps.max_cols.min(old_cols);
    let preview_cols_new = caps.max_cols.min(new_cols);
    let preview_rows_old = caps.max_rows.min(old_rows);
    let preview_rows_new = caps.max_rows.min(new_rows);

    let mut rects = Vec::new();
    let mut seen = HashSet::new();
    let mut counter: u32 = 1;

    for op in ops {
        match op {
            excel_diff::DiffOp::CellEdited { addr, .. } => {
                if let Some(rect) =
                    rect_with_context(addr.row, 1, addr.col, 1, min_rows, min_cols, caps)
                {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "cell",
                        "both",
                        rect,
                        None,
                    );
                }
            }
            excel_diff::DiffOp::RectReplaced {
                start_row,
                row_count,
                start_col,
                col_count,
                ..
            } => {
                if let Some(rect) = rect_with_context(
                    *start_row, *row_count, *start_col, *col_count, min_rows, min_cols, caps,
                ) {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "rect_replaced",
                        "both",
                        rect,
                        None,
                    );
                }
            }
            excel_diff::DiffOp::RowAdded { row_idx, .. } => {
                if preview_cols_new == 0 {
                    continue;
                }
                if let Some(rect) =
                    rect_with_context(*row_idx, 1, 0, preview_cols_new, new_rows, new_cols, caps)
                {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "row_added",
                        "new",
                        rect,
                        None,
                    );
                }
            }
            excel_diff::DiffOp::RowRemoved { row_idx, .. } => {
                if preview_cols_old == 0 {
                    continue;
                }
                if let Some(rect) =
                    rect_with_context(*row_idx, 1, 0, preview_cols_old, old_rows, old_cols, caps)
                {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "row_removed",
                        "old",
                        rect,
                        None,
                    );
                }
            }
            excel_diff::DiffOp::RowReplaced { row_idx, .. } => {
                if preview_cols_old > 0 {
                    if let Some(rect) = rect_with_context(
                        *row_idx,
                        1,
                        0,
                        preview_cols_old,
                        old_rows,
                        old_cols,
                        caps,
                    ) {
                        push_interest_rect(
                            &mut rects,
                            &mut seen,
                            &mut counter,
                            "row_replaced",
                            "old",
                            rect,
                            None,
                        );
                    }
                }
                if preview_cols_new > 0 {
                    if let Some(rect) = rect_with_context(
                        *row_idx,
                        1,
                        0,
                        preview_cols_new,
                        new_rows,
                        new_cols,
                        caps,
                    ) {
                        push_interest_rect(
                            &mut rects,
                            &mut seen,
                            &mut counter,
                            "row_replaced",
                            "new",
                            rect,
                            None,
                        );
                    }
                }
            }
            excel_diff::DiffOp::DuplicateKeyCluster {
                left_rows,
                right_rows,
                ..
            } => {
                if preview_cols_old > 0 {
                    for row_idx in left_rows {
                        if let Some(rect) = rect_with_context(
                            *row_idx,
                            1,
                            0,
                            preview_cols_old,
                            old_rows,
                            old_cols,
                            caps,
                        ) {
                            push_interest_rect(
                                &mut rects,
                                &mut seen,
                                &mut counter,
                                "row_cluster",
                                "old",
                                rect,
                                None,
                            );
                        }
                    }
                }
                if preview_cols_new > 0 {
                    for row_idx in right_rows {
                        if let Some(rect) = rect_with_context(
                            *row_idx,
                            1,
                            0,
                            preview_cols_new,
                            new_rows,
                            new_cols,
                            caps,
                        ) {
                            push_interest_rect(
                                &mut rects,
                                &mut seen,
                                &mut counter,
                                "row_cluster",
                                "new",
                                rect,
                                None,
                            );
                        }
                    }
                }
            }
            excel_diff::DiffOp::ColumnAdded { col_idx, .. } => {
                if preview_rows_new == 0 {
                    continue;
                }
                if let Some(rect) =
                    rect_with_context(0, preview_rows_new, *col_idx, 1, new_rows, new_cols, caps)
                {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "col_added",
                        "new",
                        rect,
                        None,
                    );
                }
            }
            excel_diff::DiffOp::ColumnRemoved { col_idx, .. } => {
                if preview_rows_old == 0 {
                    continue;
                }
                if let Some(rect) =
                    rect_with_context(0, preview_rows_old, *col_idx, 1, old_rows, old_cols, caps)
                {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "col_removed",
                        "old",
                        rect,
                        None,
                    );
                }
            }
            excel_diff::DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                ..
            } => {
                if preview_cols_old > 0 {
                    if let Some(rect) = rect_with_context(
                        *src_start_row,
                        *row_count,
                        0,
                        preview_cols_old,
                        old_rows,
                        old_cols,
                        caps,
                    ) {
                        let move_id =
                            format!("r:{}+{}->{}", src_start_row, row_count, dst_start_row);
                        push_interest_rect(
                            &mut rects,
                            &mut seen,
                            &mut counter,
                            "move_src",
                            "old",
                            rect,
                            Some(move_id.clone()),
                        );
                    }
                }
                if preview_cols_new > 0 {
                    if let Some(rect) = rect_with_context(
                        *dst_start_row,
                        *row_count,
                        0,
                        preview_cols_new,
                        new_rows,
                        new_cols,
                        caps,
                    ) {
                        let move_id =
                            format!("r:{}+{}->{}", src_start_row, row_count, dst_start_row);
                        push_interest_rect(
                            &mut rects,
                            &mut seen,
                            &mut counter,
                            "move_dst",
                            "new",
                            rect,
                            Some(move_id),
                        );
                    }
                }
            }
            excel_diff::DiffOp::BlockMovedColumns {
                src_start_col,
                col_count,
                dst_start_col,
                ..
            } => {
                if preview_rows_old > 0 {
                    if let Some(rect) = rect_with_context(
                        0,
                        preview_rows_old,
                        *src_start_col,
                        *col_count,
                        old_rows,
                        old_cols,
                        caps,
                    ) {
                        let move_id =
                            format!("c:{}+{}->{}", src_start_col, col_count, dst_start_col);
                        push_interest_rect(
                            &mut rects,
                            &mut seen,
                            &mut counter,
                            "move_src",
                            "old",
                            rect,
                            Some(move_id.clone()),
                        );
                    }
                }
                if preview_rows_new > 0 {
                    if let Some(rect) = rect_with_context(
                        0,
                        preview_rows_new,
                        *dst_start_col,
                        *col_count,
                        new_rows,
                        new_cols,
                        caps,
                    ) {
                        let move_id =
                            format!("c:{}+{}->{}", src_start_col, col_count, dst_start_col);
                        push_interest_rect(
                            &mut rects,
                            &mut seen,
                            &mut counter,
                            "move_dst",
                            "new",
                            rect,
                            Some(move_id),
                        );
                    }
                }
            }
            excel_diff::DiffOp::BlockMovedRect {
                src_start_row,
                src_row_count,
                src_start_col,
                src_col_count,
                dst_start_row,
                dst_start_col,
                ..
            } => {
                let move_id = format!(
                    "rect:{},{}+{}x{}->{},{}",
                    src_start_row,
                    src_start_col,
                    src_row_count,
                    src_col_count,
                    dst_start_row,
                    dst_start_col
                );
                if let Some(rect) = rect_with_context(
                    *src_start_row,
                    *src_row_count,
                    *src_start_col,
                    *src_col_count,
                    old_rows,
                    old_cols,
                    caps,
                ) {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "move_src",
                        "old",
                        rect,
                        Some(move_id.clone()),
                    );
                }
                if let Some(rect) = rect_with_context(
                    *dst_start_row,
                    *src_row_count,
                    *dst_start_col,
                    *src_col_count,
                    new_rows,
                    new_cols,
                    caps,
                ) {
                    push_interest_rect(
                        &mut rects,
                        &mut seen,
                        &mut counter,
                        "move_dst",
                        "new",
                        rect,
                        Some(move_id),
                    );
                }
            }
            _ => {}
        }
    }

    rects
}

fn push_cell(
    cells: &mut Vec<SheetCell>,
    pool: &excel_diff::StringPool,
    row: u32,
    col: u32,
    cell: &excel_diff::Cell,
) {
    let value = render_cell_value(pool, &cell.value);
    let formula = cell.formula.map(|id| format!("={}", pool.resolve(id)));
    cells.push(SheetCell {
        row,
        col,
        value,
        formula,
    });
}

fn snapshot_sheet_limited(
    sheet: &excel_diff::Sheet,
    pool: &excel_diff::StringPool,
    ops: &[&excel_diff::DiffOp],
    caps: &SnapshotCaps,
    budget: &mut usize,
) -> SheetSnapshot {
    let name = pool.resolve(sheet.name).to_string();
    let nrows = sheet.grid.nrows;
    let ncols = sheet.grid.ncols;
    let total_non_empty = sheet.grid.cell_count();
    let total_non_empty_cells = u32::try_from(total_non_empty).unwrap_or(u32::MAX);

    if total_non_empty == 0 {
        return SheetSnapshot {
            name,
            nrows,
            ncols,
            cells: Vec::new(),
            truncated: false,
            included_cells: 0,
            total_non_empty_cells: 0,
            note: None,
        };
    }

    let per_sheet_limit = caps.per_sheet.min(*budget);
    let mut cells = Vec::new();

    if total_non_empty <= per_sheet_limit {
        cells.reserve(total_non_empty);
        for ((row, col), cell) in sheet.grid.iter_cells() {
            push_cell(&mut cells, pool, row, col, cell);
        }
        *budget = budget.saturating_sub(total_non_empty);
        return SheetSnapshot {
            name,
            nrows,
            ncols,
            cells,
            truncated: false,
            included_cells: total_non_empty_cells,
            total_non_empty_cells,
            note: None,
        };
    }

    if per_sheet_limit == 0 {
        return SheetSnapshot {
            name,
            nrows,
            ncols,
            cells: Vec::new(),
            truncated: true,
            included_cells: 0,
            total_non_empty_cells,
            note: Some("Preview limited: snapshot budget reached.".to_string()),
        };
    }

    let rects = collect_interest_rects(nrows, ncols, ops, caps);
    let total_rect_area: u64 = rects.iter().map(Rect::area).sum();
    let mut seen = HashSet::new();
    let mut remaining = per_sheet_limit;

    if rects.is_empty() || total_rect_area > total_non_empty as u64 {
        for ((row, col), cell) in sheet.grid.iter_cells() {
            if remaining == 0 || *budget == 0 {
                break;
            }
            if !rects.is_empty() && !rects.iter().any(|rect| rect.contains(row, col)) {
                continue;
            }
            if seen.insert((row, col)) {
                push_cell(&mut cells, pool, row, col, cell);
                remaining = remaining.saturating_sub(1);
                *budget = budget.saturating_sub(1);
            }
        }
    } else {
        'rects: for rect in &rects {
            for row in rect.row_start..=rect.row_end {
                for col in rect.col_start..=rect.col_end {
                    if remaining == 0 || *budget == 0 {
                        break 'rects;
                    }
                    if !seen.insert((row, col)) {
                        continue;
                    }
                    if let Some(cell) = sheet.grid.get(row, col) {
                        push_cell(&mut cells, pool, row, col, cell);
                        remaining = remaining.saturating_sub(1);
                        *budget = budget.saturating_sub(1);
                    }
                }
            }
        }
    }

    let included_cells = u32::try_from(cells.len()).unwrap_or(u32::MAX);
    let truncated = included_cells < total_non_empty_cells;
    let note = if truncated {
        Some(format!(
            "Preview limited: showing {} of {} non-empty cells.",
            included_cells, total_non_empty_cells
        ))
    } else {
        None
    };

    SheetSnapshot {
        name,
        nrows,
        ncols,
        cells,
        truncated,
        included_cells,
        total_non_empty_cells,
        note,
    }
}

fn snapshot_workbook(
    workbook: &excel_diff::Workbook,
    sheet_ids: &HashSet<excel_diff::StringId>,
    pool: &excel_diff::StringPool,
    ops_by_sheet: &HashMap<String, Vec<&excel_diff::DiffOp>>,
    caps: &SnapshotCaps,
    budget: &mut usize,
) -> WorkbookSnapshot {
    if sheet_ids.is_empty() {
        return WorkbookSnapshot { sheets: Vec::new() };
    }

    let mut sheets = Vec::new();
    for sheet in &workbook.sheets {
        if !sheet_ids.contains(&sheet.name) {
            continue;
        }
        let sheet_name = pool.resolve(sheet.name).to_string();
        let ops = ops_by_sheet
            .get(&sheet_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        sheets.push(snapshot_sheet_limited(sheet, pool, ops, caps, budget));
    }

    WorkbookSnapshot { sheets }
}
