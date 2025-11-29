use crate::workbook::{CellAddress, CellSnapshot};

pub type SheetId = String;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowSignature {
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ColSignature {
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum DiffOp {
    SheetAdded {
        sheet: SheetId,
    },
    SheetRemoved {
        sheet: SheetId,
    },
    RowAdded {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    RowRemoved {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    ColumnAdded {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    ColumnRemoved {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    BlockMovedRows {
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    BlockMovedColumns {
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    CellEdited {
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffReport {
    pub version: String,
    pub ops: Vec<DiffOp>,
}

impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            ops,
        }
    }
}
