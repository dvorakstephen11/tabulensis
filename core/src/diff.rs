use crate::workbook::{CellAddress, CellSnapshot, ColSignature, RowSignature};

pub type SheetId = String;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
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
    /// Logical change to a single cell.
    ///
    /// Invariants (maintained by producers and tests, not by the type system):
    /// - `addr` is the canonical location for the edit.
    /// - `from.addr` and `to.addr` must both equal `addr`.
    /// - `CellSnapshot` equality intentionally ignores `addr` and compares only
    ///   `(value, formula)`, so `DiffOp::CellEdited` equality does not by itself
    ///   enforce the address invariants; callers must respect them when
    ///   constructing ops.
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

impl DiffOp {
    pub fn cell_edited(
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    ) -> DiffOp {
        debug_assert_eq!(from.addr, addr, "from.addr must match canonical addr");
        debug_assert_eq!(to.addr, addr, "to.addr must match canonical addr");
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        }
    }

    pub fn row_added(sheet: SheetId, row_idx: u32, row_signature: Option<RowSignature>) -> DiffOp {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn row_removed(
        sheet: SheetId,
        row_idx: u32,
        row_signature: Option<RowSignature>,
    ) -> DiffOp {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn column_added(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        }
    }

    pub fn column_removed(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        }
    }

    pub fn block_moved_rows(
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        }
    }

    pub fn block_moved_columns(
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        }
    }
}
