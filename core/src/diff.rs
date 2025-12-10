//! Diff operations and reports for workbook comparison.
//!
//! This module defines the types used to represent differences between two workbooks:
//! - [`DiffOp`]: Individual operations representing a single change (cell edit, row/column add/remove, etc.)
//! - [`DiffReport`]: A versioned collection of diff operations

use crate::workbook::{CellAddress, CellSnapshot, ColSignature, RowSignature};

pub type SheetId = String;

/// A single diff operation representing one logical change between workbooks.
///
/// Operations are emitted by the diff engine and collected into a [`DiffReport`].
/// The enum is marked `#[non_exhaustive]` to allow future additions.
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
    BlockMovedRect {
        sheet: SheetId,
        src_start_row: u32,
        src_row_count: u32,
        src_start_col: u32,
        src_col_count: u32,
        dst_start_row: u32,
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

/// A versioned collection of diff operations between two workbooks.
///
/// The `version` field indicates the schema version for forwards compatibility.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffReport {
    /// Schema version (currently "1").
    pub version: String,
    /// The list of diff operations.
    pub ops: Vec<DiffOp>,
    /// Whether the diff result is complete. When `false`, some operations may be missing
    /// due to resource limits being exceeded (e.g., row/column limits).
    #[serde(default = "default_complete")]
    pub complete: bool,
    /// Warnings generated during the diff process. Non-empty when limits were exceeded
    /// or other partial-result conditions occurred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[cfg(feature = "perf-metrics")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<crate::perf::DiffMetrics>,
}

fn default_complete() -> bool {
    true
}

impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            ops,
            complete: true,
            warnings: Vec::new(),
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn with_partial_result(ops: Vec<DiffOp>, warning: String) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            ops,
            complete: false,
            warnings: vec![warning],
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
        self.complete = false;
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

    #[allow(clippy::too_many_arguments)]
    pub fn block_moved_rect(
        sheet: SheetId,
        src_start_row: u32,
        src_row_count: u32,
        src_start_col: u32,
        src_col_count: u32,
        dst_start_row: u32,
        dst_start_col: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash,
        }
    }
}
