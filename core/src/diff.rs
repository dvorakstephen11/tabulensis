//! Diff operations and reports for workbook comparison.
//!
//! This module defines the types used to represent differences between two workbooks:
//! - [`DiffOp`]: Individual operations representing a single change (cell edit, row/column add/remove, etc.)
//! - [`DiffReport`]: A versioned collection of diff operations
//! - [`DiffError`]: Errors that can occur during the diff process

use crate::error_codes;
use crate::string_pool::StringId;
use crate::workbook::{CellAddress, CellSnapshot, ColSignature, RowSignature};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueryChangeKind {
    /// A semantic change (meaningfully different after canonicalization).
    Semantic,
    /// Only formatting changed (whitespace/comments); meaning is unchanged.
    FormattingOnly,
    /// The query was renamed (definition may be unchanged).
    Renamed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormulaDiffResult {
    /// Unknown or not computed.
    Unknown,
    /// Formula/value is unchanged.
    Unchanged,
    /// Formula/value was added.
    Added,
    /// Formula/value was removed.
    Removed,
    /// Only formatting changed (whitespace/casing), semantics unchanged.
    FormattingOnly,
    /// Filled down/across (shift-equivalent).
    Filled,
    /// Semantic change.
    SemanticChange,
    /// Textual change (different text but semantics not computed/unknown).
    TextChange,
}

impl Default for FormulaDiffResult {
    fn default() -> Self {
        FormulaDiffResult::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueryMetadataField {
    /// Whether the query loads to a sheet.
    LoadToSheet,
    /// Whether the query loads to the data model.
    LoadToModel,
    /// Query group path.
    GroupPath,
    /// Whether the query is connection-only.
    ConnectionOnly,
}

/// Errors produced by diffing APIs.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DiffError {
    #[error(
        "[EXDIFF_DIFF_001] alignment limits exceeded for sheet '{sheet}': rows={rows}, cols={cols} (limits: rows={max_rows}, cols={max_cols}). Suggestion: increase `max_align_rows`/`max_align_cols` or change `on_limit_exceeded`."
    )]
    LimitsExceeded {
        sheet: StringId,
        rows: u32,
        cols: u32,
        max_rows: u32,
        max_cols: u32,
    },

    #[error("[EXDIFF_DIFF_002] sink error: {message}. Suggestion: check the output destination and retry.")]
    SinkError { message: String },

    #[error("[EXDIFF_DIFF_003] sheet '{requested}' not found. Available sheets: {}. Suggestion: check the sheet name and casing.", available.join(", "))]
    SheetNotFound {
        requested: String,
        available: Vec<String>,
    },

    #[error("[EXDIFF_DIFF_004] internal error: {message}. Suggestion: report a bug with the input file if possible.")]
    InternalError { message: String },
}

impl DiffError {
    pub fn code(&self) -> &'static str {
        match self {
            DiffError::LimitsExceeded { .. } => error_codes::DIFF_LIMITS_EXCEEDED,
            DiffError::SinkError { .. } => error_codes::DIFF_SINK_ERROR,
            DiffError::SheetNotFound { .. } => error_codes::DIFF_SHEET_NOT_FOUND,
            DiffError::InternalError { .. } => error_codes::DIFF_INTERNAL_ERROR,
        }
    }
}

pub type SheetId = StringId;

/// Summary metadata about a diff run emitted alongside streamed ops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSummary {
    /// Whether the diff completed without early aborts/fallbacks.
    pub complete: bool,
    /// Warnings explaining why results are incomplete (when `complete == false`).
    pub warnings: Vec<String>,
    /// Total number of ops emitted.
    pub op_count: usize,
    #[cfg(feature = "perf-metrics")]
    /// Optional performance metrics when the `perf-metrics` feature is enabled.
    pub metrics: Option<crate::perf::DiffMetrics>,
}

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
        #[serde(default)]
        formula_diff: FormulaDiffResult,
    },

    VbaModuleAdded {
        name: StringId,
    },
    VbaModuleRemoved {
        name: StringId,
    },
    VbaModuleChanged {
        name: StringId,
    },

    NamedRangeAdded {
        name: StringId,
    },
    NamedRangeRemoved {
        name: StringId,
    },
    NamedRangeChanged {
        name: StringId,
        old_ref: StringId,
        new_ref: StringId,
    },

    ChartAdded {
        sheet: StringId,
        name: StringId,
    },
    ChartRemoved {
        sheet: StringId,
        name: StringId,
    },
    ChartChanged {
        sheet: StringId,
        name: StringId,
    },

    QueryAdded {
        name: StringId,
    },
    QueryRemoved {
        name: StringId,
    },
    QueryRenamed {
        from: StringId,
        to: StringId,
    },
    QueryDefinitionChanged {
        name: StringId,
        change_kind: QueryChangeKind,
        old_hash: u64,
        new_hash: u64,
    },
    QueryMetadataChanged {
        name: StringId,
        field: QueryMetadataField,
        old: Option<StringId>,
        new: Option<StringId>,
    },
    // Future: DAX operations
    // MeasureAdded { name: StringId }
    // MeasureRemoved { name: StringId }
    // MeasureDefinitionChanged { name: StringId, change_kind: QueryChangeKind }
}

/// A versioned collection of diff operations between two workbooks.
///
/// The `version` field indicates the schema version for forwards compatibility.
///
/// # Incomplete results
///
/// Some safety rails and limit behaviors can produce partial results. In that case:
///
/// - `complete == false`
/// - `warnings` contains at least one human-readable explanation
///
/// The CLI prints warnings to stderr as `Warning: ...`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffReport {
    /// Schema version (currently "1").
    pub version: String,
    /// Interned string table used by ids referenced in this report.
    #[serde(default)]
    pub strings: Vec<String>,
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
            strings: Vec::new(),
            ops,
            complete: true,
            warnings: Vec::new(),
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        }
    }

    pub fn from_ops_and_summary(
        ops: Vec<DiffOp>,
        summary: DiffSummary,
        strings: Vec<String>,
    ) -> DiffReport {
        let mut report = DiffReport::new(ops);
        report.complete = summary.complete;
        report.warnings = summary.warnings;
        #[cfg(feature = "perf-metrics")]
        {
            report.metrics = summary.metrics;
        }
        report.strings = strings;
        report
    }

    pub fn with_partial_result(ops: Vec<DiffOp>, warning: String) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            strings: Vec::new(),
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

    /// Resolve an interned [`StringId`] into a string slice using this report's `strings` table.
    ///
    /// Many fields in [`DiffOp`] are `StringId`s (sheet names, query names, etc.). The returned
    /// string is owned by the report.
    ///
    /// ```
    /// # use excel_diff::{DiffReport, StringId};
    /// # fn demo(report: &DiffReport, id: StringId) {
    /// let text = report.resolve(id).unwrap_or("<unknown>");
    /// # let _ = text;
    /// # }
    /// ```
    pub fn resolve(&self, id: StringId) -> Option<&str> {
        self.strings.get(id.0 as usize).map(|s| s.as_str())
    }

    pub fn grid_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| !op.is_m_op())
    }

    pub fn m_ops(&self) -> impl Iterator<Item = &DiffOp> {
        self.ops.iter().filter(|op| op.is_m_op())
    }
}

impl DiffOp {
    pub fn is_m_op(&self) -> bool {
        matches!(
            self,
            DiffOp::QueryAdded { .. }
                | DiffOp::QueryRemoved { .. }
                | DiffOp::QueryRenamed { .. }
                | DiffOp::QueryDefinitionChanged { .. }
                | DiffOp::QueryMetadataChanged { .. }
        )
    }

    pub fn cell_edited(
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
        formula_diff: FormulaDiffResult,
    ) -> DiffOp {
        debug_assert_eq!(from.addr, addr, "from.addr must match canonical addr");
        debug_assert_eq!(to.addr, addr, "to.addr must match canonical addr");
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            formula_diff,
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
