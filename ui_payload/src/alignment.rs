use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::{SheetPairSnapshot, SheetSnapshot};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisKind {
    Match,
    Insert,
    Delete,
    MoveSrc,
    MoveDst,
}

#[derive(Serialize)]
pub struct AxisEntry {
    old: Option<u32>,
    new: Option<u32>,
    kind: AxisKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    move_id: Option<String>,
}

#[derive(Serialize)]
pub struct MoveGroup {
    id: String,
    axis: String,
    src_start: u32,
    dst_start: u32,
    count: u32,
}

#[derive(Serialize)]
pub struct SheetAlignment {
    sheet: String,
    rows: Vec<AxisEntry>,
    cols: Vec<AxisEntry>,
    moves: Vec<MoveGroup>,
    skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_reason: Option<String>,
}

// Guardrail: keep the HTML grid from exploding on large sheets.
const MAX_VIEW_ROWS: u32 = 10_000;
const MAX_VIEW_COLS: u32 = 200;

pub fn build_alignments(
    report: &excel_diff::DiffReport,
    sheets: &SheetPairSnapshot,
) -> Vec<SheetAlignment> {
    let ops_by_sheet = group_ops_by_sheet(report);
    let rename_map = build_rename_map(report);
    let renamed_old: HashSet<String> = rename_map.values().cloned().collect();
    let old_lookup = build_sheet_lookup(&sheets.old.sheets);
    let new_lookup = build_sheet_lookup(&sheets.new.sheets);

    let mut names = HashSet::new();
    names.extend(ops_by_sheet.keys().cloned());
    names.extend(new_lookup.keys().cloned());
    for name in old_lookup.keys() {
        if renamed_old.contains(name) {
            continue;
        }
        names.insert(name.clone());
    }

    let mut names: Vec<String> = names.into_iter().collect();
    names.sort();

    let empty_ops: Vec<&excel_diff::DiffOp> = Vec::new();
    let mut alignments = Vec::with_capacity(names.len());

    for sheet in names {
        let ops = ops_by_sheet
            .get(&sheet)
            .map(Vec::as_slice)
            .unwrap_or(empty_ops.as_slice());
        let old_sheet = old_lookup
            .get(&sheet)
            .copied()
            .or_else(|| rename_map.get(&sheet).and_then(|old| old_lookup.get(old).copied()));
        let new_sheet = new_lookup.get(&sheet).copied();
        alignments.push(build_sheet_alignment(&sheet, old_sheet, new_sheet, ops));
    }

    alignments
}

fn build_sheet_lookup<'a>(sheets: &'a [SheetSnapshot]) -> HashMap<String, &'a SheetSnapshot> {
    let mut map = HashMap::new();
    for sheet in sheets {
        map.insert(sheet.name.clone(), sheet);
    }
    map
}

fn group_ops_by_sheet<'a>(
    report: &'a excel_diff::DiffReport,
) -> HashMap<String, Vec<&'a excel_diff::DiffOp>> {
    let mut map = HashMap::new();
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

fn build_rename_map(report: &excel_diff::DiffReport) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for op in &report.ops {
        let excel_diff::DiffOp::SheetRenamed { sheet, from, .. } = op else {
            continue;
        };
        let new_name = report.resolve(*sheet).unwrap_or("<unknown>");
        let old_name = report.resolve(*from).unwrap_or("<unknown>");
        map.insert(new_name.to_string(), old_name.to_string());
    }
    map
}

fn build_sheet_alignment(
    sheet: &str,
    old_sheet: Option<&SheetSnapshot>,
    new_sheet: Option<&SheetSnapshot>,
    ops: &[&excel_diff::DiffOp],
) -> SheetAlignment {
    let old_nrows = old_sheet.map(|s| s.nrows).unwrap_or(0);
    let new_nrows = new_sheet.map(|s| s.nrows).unwrap_or(0);
    let old_ncols = old_sheet.map(|s| s.ncols).unwrap_or(0);
    let new_ncols = new_sheet.map(|s| s.ncols).unwrap_or(0);

    let mut added_rows = HashSet::new();
    let mut removed_rows = HashSet::new();
    let mut added_cols = HashSet::new();
    let mut removed_cols = HashSet::new();
    let mut move_src_rows = HashMap::new();
    let mut move_dst_rows = HashMap::new();
    let mut move_src_cols = HashMap::new();
    let mut move_dst_cols = HashMap::new();
    let mut moves = Vec::new();

    for op in ops {
        match op {
            excel_diff::DiffOp::RowAdded { row_idx, .. } => {
                added_rows.insert(*row_idx);
            }
            excel_diff::DiffOp::RowRemoved { row_idx, .. } => {
                removed_rows.insert(*row_idx);
            }
            excel_diff::DiffOp::ColumnAdded { col_idx, .. } => {
                added_cols.insert(*col_idx);
            }
            excel_diff::DiffOp::ColumnRemoved { col_idx, .. } => {
                removed_cols.insert(*col_idx);
            }
            excel_diff::DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                ..
            } => {
                if *row_count == 0 {
                    continue;
                }
                let move_id = format!("r:{}+{}->{}", src_start_row, row_count, dst_start_row);
                moves.push(MoveGroup {
                    id: move_id.clone(),
                    axis: "row".to_string(),
                    src_start: *src_start_row,
                    dst_start: *dst_start_row,
                    count: *row_count,
                });
                let src_end = src_start_row.saturating_add(*row_count);
                for row in *src_start_row..src_end {
                    move_src_rows.insert(row, move_id.clone());
                }
                let dst_end = dst_start_row.saturating_add(*row_count);
                for row in *dst_start_row..dst_end {
                    move_dst_rows.insert(row, move_id.clone());
                }
            }
            excel_diff::DiffOp::BlockMovedColumns {
                src_start_col,
                col_count,
                dst_start_col,
                ..
            } => {
                if *col_count == 0 {
                    continue;
                }
                let move_id = format!("c:{}+{}->{}", src_start_col, col_count, dst_start_col);
                moves.push(MoveGroup {
                    id: move_id.clone(),
                    axis: "col".to_string(),
                    src_start: *src_start_col,
                    dst_start: *dst_start_col,
                    count: *col_count,
                });
                let src_end = src_start_col.saturating_add(*col_count);
                for col in *src_start_col..src_end {
                    move_src_cols.insert(col, move_id.clone());
                }
                let dst_end = dst_start_col.saturating_add(*col_count);
                for col in *dst_start_col..dst_end {
                    move_dst_cols.insert(col, move_id.clone());
                }
            }
            _ => {}
        }
    }

    let row_summary = axis_summary(
        old_nrows,
        new_nrows,
        &added_rows,
        &removed_rows,
        &move_src_rows,
        &move_dst_rows,
    );
    let col_summary = axis_summary(
        old_ncols,
        new_ncols,
        &added_cols,
        &removed_cols,
        &move_src_cols,
        &move_dst_cols,
    );

    let mut skip_reason = None;
    if !row_summary.consistent || !col_summary.consistent {
        skip_reason = Some("Preview disabled: alignment inconsistent.".to_string());
    }
    if let Some(reason) = limit_reason(row_summary.view_len, col_summary.view_len) {
        skip_reason = Some(reason);
    }

    if skip_reason.is_some() {
        return SheetAlignment {
            sheet: sheet.to_string(),
            rows: Vec::new(),
            cols: Vec::new(),
            moves,
            skipped: true,
            skip_reason,
        };
    }

    let (mut rows, rows_consistent) = build_axis_entries(
        old_nrows,
        new_nrows,
        &added_rows,
        &removed_rows,
        &move_src_rows,
        &move_dst_rows,
    );
    let (mut cols, cols_consistent) = build_axis_entries(
        old_ncols,
        new_ncols,
        &added_cols,
        &removed_cols,
        &move_src_cols,
        &move_dst_cols,
    );

    let mut skip_reason = None;
    if !(rows_consistent && cols_consistent) {
        skip_reason = Some("Preview disabled: alignment inconsistent.".to_string());
    }
    if let Some(reason) = limit_reason(rows.len() as u32, cols.len() as u32) {
        skip_reason = Some(reason);
    }

    let skipped = skip_reason.is_some();
    if skipped {
        rows.clear();
        cols.clear();
    }

    SheetAlignment {
        sheet: sheet.to_string(),
        rows,
        cols,
        moves,
        skipped,
        skip_reason,
    }
}

struct AxisSummary {
    view_len: u32,
    consistent: bool,
}

fn axis_summary(
    old_len: u32,
    new_len: u32,
    added: &HashSet<u32>,
    removed: &HashSet<u32>,
    move_src: &HashMap<u32, String>,
    move_dst: &HashMap<u32, String>,
) -> AxisSummary {
    let added_total = union_count(added, move_dst.keys());
    let removed_total = union_count(removed, move_src.keys());

    let view_from_old = old_len.saturating_add(added_total);
    let view_from_new = new_len.saturating_add(removed_total);
    let view_len = view_from_old.max(view_from_new);

    let consistent = old_len >= removed_total
        && new_len >= added_total
        && (old_len - removed_total) == (new_len - added_total);

    AxisSummary { view_len, consistent }
}

fn union_count<'a>(
    base: &HashSet<u32>,
    extra: impl Iterator<Item = &'a u32>,
) -> u32 {
    let mut count = u32::try_from(base.len()).unwrap_or(u32::MAX);
    for value in extra {
        if !base.contains(value) {
            count = count.saturating_add(1);
        }
    }
    count
}

fn build_axis_entries(
    old_len: u32,
    new_len: u32,
    added: &HashSet<u32>,
    removed: &HashSet<u32>,
    move_src: &HashMap<u32, String>,
    move_dst: &HashMap<u32, String>,
) -> (Vec<AxisEntry>, bool) {
    let mut entries = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < old_len || j < new_len {
        if j < new_len {
            if let Some(move_id) = move_dst.get(&j) {
                entries.push(AxisEntry {
                    old: None,
                    new: Some(j),
                    kind: AxisKind::MoveDst,
                    move_id: Some(move_id.clone()),
                });
                j += 1;
                continue;
            }
            if added.contains(&j) {
                entries.push(AxisEntry {
                    old: None,
                    new: Some(j),
                    kind: AxisKind::Insert,
                    move_id: None,
                });
                j += 1;
                continue;
            }
        }

        if i < old_len {
            if let Some(move_id) = move_src.get(&i) {
                entries.push(AxisEntry {
                    old: Some(i),
                    new: None,
                    kind: AxisKind::MoveSrc,
                    move_id: Some(move_id.clone()),
                });
                i += 1;
                continue;
            }
            if removed.contains(&i) {
                entries.push(AxisEntry {
                    old: Some(i),
                    new: None,
                    kind: AxisKind::Delete,
                    move_id: None,
                });
                i += 1;
                continue;
            }
        }

        if i < old_len && j < new_len {
            entries.push(AxisEntry {
                old: Some(i),
                new: Some(j),
                kind: AxisKind::Match,
                move_id: None,
            });
            i += 1;
            j += 1;
            continue;
        }

        if i < old_len {
            entries.push(AxisEntry {
                old: Some(i),
                new: None,
                kind: AxisKind::Delete,
                move_id: None,
            });
            i += 1;
        } else if j < new_len {
            entries.push(AxisEntry {
                old: None,
                new: Some(j),
                kind: AxisKind::Insert,
                move_id: None,
            });
            j += 1;
        }
    }

    let summary = axis_summary(old_len, new_len, added, removed, move_src, move_dst);
    (entries, summary.consistent)
}

fn limit_reason(rows: u32, cols: u32) -> Option<String> {
    if rows > MAX_VIEW_ROWS {
        return Some(format!(
            "Preview disabled: sheet has {} rows (cap is {}).",
            rows, MAX_VIEW_ROWS
        ));
    }
    if cols > MAX_VIEW_COLS {
        return Some(format!(
            "Preview disabled: sheet has {} columns (cap is {}).",
            cols, MAX_VIEW_COLS
        ));
    }
    None
}
