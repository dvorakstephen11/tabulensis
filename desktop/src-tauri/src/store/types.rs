use std::collections::HashMap;

use excel_diff::{DiffOp, StringId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
    Moved,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCounts {
    pub added: u64,
    pub removed: u64,
    pub modified: u64,
    pub moved: u64,
}

impl ChangeCounts {
    pub fn apply(&mut self, kind: ChangeKind) {
        match kind {
            ChangeKind::Added => self.added = self.added.saturating_add(1),
            ChangeKind::Removed => self.removed = self.removed.saturating_add(1),
            ChangeKind::Modified => self.modified = self.modified.saturating_add(1),
            ChangeKind::Moved => self.moved = self.moved.saturating_add(1),
        }
    }

    pub fn add_op(&mut self, op: &DiffOp) {
        if let Some(kind) = classify_op(op) {
            self.apply(kind);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SheetStats {
    pub sheet_id: u32,
    pub counts: ChangeCounts,
    pub op_count: u64,
}

impl SheetStats {
    pub fn new(sheet_id: u32) -> Self {
        Self {
            sheet_id,
            counts: ChangeCounts::default(),
            op_count: 0,
        }
    }

    pub fn add_op(&mut self, op: &DiffOp) {
        self.op_count = self.op_count.saturating_add(1);
        self.counts.add_op(op);
    }
}

#[derive(Debug, Default, Clone)]
pub struct OpIndexFields {
    pub kind: String,
    pub sheet_id: Option<u32>,
    pub row: Option<u32>,
    pub col: Option<u32>,
    pub row_end: Option<u32>,
    pub col_end: Option<u32>,
    pub move_id: Option<String>,
}

pub fn op_sheet_id(op: &DiffOp) -> Option<StringId> {
    match op {
        DiffOp::SheetAdded { sheet }
        | DiffOp::SheetRemoved { sheet }
        | DiffOp::SheetRenamed { sheet, .. }
        | DiffOp::RowAdded { sheet, .. }
        | DiffOp::RowRemoved { sheet, .. }
        | DiffOp::RowReplaced { sheet, .. }
        | DiffOp::ColumnAdded { sheet, .. }
        | DiffOp::ColumnRemoved { sheet, .. }
        | DiffOp::BlockMovedRows { sheet, .. }
        | DiffOp::BlockMovedColumns { sheet, .. }
        | DiffOp::BlockMovedRect { sheet, .. }
        | DiffOp::RectReplaced { sheet, .. }
        | DiffOp::CellEdited { sheet, .. } => Some(*sheet),
        _ => None,
    }
}

pub fn diff_op_kind(op: &DiffOp) -> &'static str {
    match op {
        DiffOp::SheetAdded { .. } => "SheetAdded",
        DiffOp::SheetRemoved { .. } => "SheetRemoved",
        DiffOp::SheetRenamed { .. } => "SheetRenamed",
        DiffOp::RowAdded { .. } => "RowAdded",
        DiffOp::RowRemoved { .. } => "RowRemoved",
        DiffOp::RowReplaced { .. } => "RowReplaced",
        DiffOp::ColumnAdded { .. } => "ColumnAdded",
        DiffOp::ColumnRemoved { .. } => "ColumnRemoved",
        DiffOp::BlockMovedRows { .. } => "BlockMovedRows",
        DiffOp::BlockMovedColumns { .. } => "BlockMovedColumns",
        DiffOp::BlockMovedRect { .. } => "BlockMovedRect",
        DiffOp::RectReplaced { .. } => "RectReplaced",
        DiffOp::CellEdited { .. } => "CellEdited",
        DiffOp::VbaModuleAdded { .. } => "VbaModuleAdded",
        DiffOp::VbaModuleRemoved { .. } => "VbaModuleRemoved",
        DiffOp::VbaModuleChanged { .. } => "VbaModuleChanged",
        DiffOp::NamedRangeAdded { .. } => "NamedRangeAdded",
        DiffOp::NamedRangeRemoved { .. } => "NamedRangeRemoved",
        DiffOp::NamedRangeChanged { .. } => "NamedRangeChanged",
        DiffOp::ChartAdded { .. } => "ChartAdded",
        DiffOp::ChartRemoved { .. } => "ChartRemoved",
        DiffOp::ChartChanged { .. } => "ChartChanged",
        DiffOp::QueryAdded { .. } => "QueryAdded",
        DiffOp::QueryRemoved { .. } => "QueryRemoved",
        DiffOp::QueryRenamed { .. } => "QueryRenamed",
        DiffOp::QueryDefinitionChanged { .. } => "QueryDefinitionChanged",
        DiffOp::QueryMetadataChanged { .. } => "QueryMetadataChanged",
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureAdded { .. } => "MeasureAdded",
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureRemoved { .. } => "MeasureRemoved",
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureDefinitionChanged { .. } => "MeasureDefinitionChanged",
        _ => "Unknown",
    }
}

pub fn classify_op(op: &DiffOp) -> Option<ChangeKind> {
    match op {
        DiffOp::SheetAdded { .. }
        | DiffOp::RowAdded { .. }
        | DiffOp::ColumnAdded { .. }
        | DiffOp::NamedRangeAdded { .. }
        | DiffOp::ChartAdded { .. }
        | DiffOp::VbaModuleAdded { .. }
        | DiffOp::QueryAdded { .. }
        | DiffOp::QueryMetadataChanged {
            field: excel_diff::QueryMetadataField::LoadToSheet,
            ..
        } => Some(ChangeKind::Added),
        DiffOp::SheetRemoved { .. }
        | DiffOp::RowRemoved { .. }
        | DiffOp::ColumnRemoved { .. }
        | DiffOp::NamedRangeRemoved { .. }
        | DiffOp::ChartRemoved { .. }
        | DiffOp::VbaModuleRemoved { .. }
        | DiffOp::QueryRemoved { .. } => Some(ChangeKind::Removed),
        DiffOp::BlockMovedRows { .. }
        | DiffOp::BlockMovedColumns { .. }
        | DiffOp::BlockMovedRect { .. } => Some(ChangeKind::Moved),
        DiffOp::RowReplaced { .. }
        | DiffOp::RectReplaced { .. }
        | DiffOp::CellEdited { .. }
        | DiffOp::SheetRenamed { .. }
        | DiffOp::NamedRangeChanged { .. }
        | DiffOp::ChartChanged { .. }
        | DiffOp::VbaModuleChanged { .. }
        | DiffOp::QueryRenamed { .. }
        | DiffOp::QueryDefinitionChanged { .. }
        | DiffOp::QueryMetadataChanged { .. } => Some(ChangeKind::Modified),
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureAdded { .. } => Some(ChangeKind::Added),
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureRemoved { .. } => Some(ChangeKind::Removed),
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureDefinitionChanged { .. } => Some(ChangeKind::Modified),
        _ => None,
    }
}

pub fn op_index_fields(op: &DiffOp) -> OpIndexFields {
    let mut fields = OpIndexFields {
        kind: diff_op_kind(op).to_string(),
        ..OpIndexFields::default()
    };

    if let Some(sheet) = op_sheet_id(op) {
        fields.sheet_id = Some(sheet.0);
    }

    match op {
        DiffOp::RowAdded { row_idx, .. }
        | DiffOp::RowRemoved { row_idx, .. }
        | DiffOp::RowReplaced { row_idx, .. } => {
            fields.row = Some(*row_idx);
            fields.row_end = Some(*row_idx);
        }
        DiffOp::ColumnAdded { col_idx, .. } | DiffOp::ColumnRemoved { col_idx, .. } => {
            fields.col = Some(*col_idx);
            fields.col_end = Some(*col_idx);
        }
        DiffOp::BlockMovedRows {
            src_start_row,
            row_count,
            dst_start_row,
            ..
        } => {
            fields.row = Some(*src_start_row);
            fields.row_end = Some(src_start_row.saturating_add(*row_count).saturating_sub(1));
            fields.move_id = Some(format!("r:{}+{}->{}", src_start_row, row_count, dst_start_row));
        }
        DiffOp::BlockMovedColumns {
            src_start_col,
            col_count,
            dst_start_col,
            ..
        } => {
            fields.col = Some(*src_start_col);
            fields.col_end = Some(src_start_col.saturating_add(*col_count).saturating_sub(1));
            fields.move_id = Some(format!("c:{}+{}->{}", src_start_col, col_count, dst_start_col));
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
            fields.row = Some(*src_start_row);
            fields.col = Some(*src_start_col);
            fields.row_end = Some(src_start_row.saturating_add(*src_row_count).saturating_sub(1));
            fields.col_end = Some(src_start_col.saturating_add(*src_col_count).saturating_sub(1));
            fields.move_id = Some(format!(
                "rect:{},{}+{}x{}->{}", 
                src_start_row, src_start_col, src_row_count, src_col_count, dst_start_row
            ));
            if fields.move_id.is_some() {
                if let Some(id) = fields.move_id.as_mut() {
                    id.push_str(&format!(",{}", dst_start_col));
                }
            }
        }
        DiffOp::RectReplaced {
            start_row,
            row_count,
            start_col,
            col_count,
            ..
        } => {
            fields.row = Some(*start_row);
            fields.col = Some(*start_col);
            fields.row_end = Some(start_row.saturating_add(*row_count).saturating_sub(1));
            fields.col_end = Some(start_col.saturating_add(*col_count).saturating_sub(1));
        }
        DiffOp::CellEdited { addr, .. } => {
            fields.row = Some(addr.row);
            fields.col = Some(addr.col);
            fields.row_end = Some(addr.row);
            fields.col_end = Some(addr.col);
        }
        _ => {}
    }

    fields
}

pub fn accumulate_sheet_stats(
    stats: &mut HashMap<u32, SheetStats>,
    op: &DiffOp,
) {
    if let Some(sheet_id) = op_sheet_id(op).map(|id| id.0) {
        let entry = stats.entry(sheet_id).or_insert_with(|| SheetStats::new(sheet_id));
        entry.add_op(op);
    }
}
