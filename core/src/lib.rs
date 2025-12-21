//! Excel Diff: A library for comparing Excel workbooks.
//!
//! This crate provides functionality for:
//! - Opening and parsing Excel workbooks (`.xlsx` files)
//! - Computing structural and cell-level differences between workbooks
//! - Serializing diff reports to JSON
//! - Parsing Power Query (M) code from DataMashup sections
//!
//! # Quick Start
//!
//! ```ignore
//! use excel_diff::WorkbookPackage;
//!
//! let pkg_a = WorkbookPackage::open(std::fs::File::open("file_a.xlsx")?)?;
//! let pkg_b = WorkbookPackage::open(std::fs::File::open("file_b.xlsx")?)?;
//! let report = pkg_a.diff(&pkg_b, &excel_diff::DiffConfig::default());
//!
//! for op in &report.ops {
//!     println!("{:?}", op);
//! }
//! ```

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

use std::cell::RefCell;

mod addressing;
pub(crate) mod alignment;
mod alignment_types;
pub(crate) mod column_alignment;
mod config;
mod container;
mod database_alignment;
mod datamashup;
mod datamashup_framing;
mod datamashup_package;
pub mod error_codes;
mod diff;
mod engine;
#[cfg(feature = "excel-open-xml")]
mod excel_open_xml;
mod formula;
mod formula_diff;
mod grid_metadata;
mod grid_parser;
mod grid_view;
pub(crate) mod hashing;
mod m_ast;
mod m_diff;
mod m_section;
mod object_diff;
mod output;
mod package;
#[cfg(feature = "perf-metrics")]
#[doc(hidden)]
pub mod perf;
pub(crate) mod rect_block_move;
pub(crate) mod region_mask;
pub(crate) mod row_alignment;
mod session;
mod sink;
mod string_pool;
mod workbook;

thread_local! {
    static DEFAULT_SESSION: RefCell<DiffSession> = RefCell::new(DiffSession::new());
}

#[doc(hidden)]
pub fn with_default_session<T>(f: impl FnOnce(&mut DiffSession) -> T) -> T {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        f(&mut session)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn try_diff_workbooks(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::try_diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
#[deprecated(note = "use WorkbookPackage::open")]
#[allow(deprecated)]
#[doc(hidden)]
pub fn open_workbook(path: impl AsRef<std::path::Path>) -> Result<Workbook, ExcelOpenError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        excel_open_xml::open_workbook(path, &mut session.strings)
    })
}

/// Advanced APIs for power users.
///
/// The recommended entry point for most callers is [`WorkbookPackage`]. This module exposes
/// lower-level functions and types for callers who want to manage their own sessions/pools or
/// stream ops directly.
pub mod advanced {
    pub use crate::engine::{
        diff_grids_database_mode, diff_workbooks as diff_workbooks_with_pool,
        diff_workbooks_streaming, try_diff_workbooks as try_diff_workbooks_with_pool,
        try_diff_workbooks_streaming,
    };
    pub use crate::session::DiffSession;
    pub use crate::sink::{CallbackSink, DiffSink, VecSink};
    pub use crate::string_pool::{StringId, StringPool};
}

pub use addressing::{AddressParseError, address_to_index, index_to_address};
pub use config::{DiffConfig, DiffConfigBuilder, LimitBehavior};
pub use container::{ContainerError, ContainerLimits, OpcContainer};
#[doc(hidden)]
pub use datamashup::parse_metadata;
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries,
};
pub use datamashup_framing::{DataMashupError, RawDataMashup, parse_data_mashup};
pub use datamashup_package::{
    DataMashupLimits, EmbeddedContent, PackageParts, PackageXml, SectionDocument,
    parse_package_parts, parse_package_parts_with_limits,
};
pub use diff::{
    DiffError, DiffOp, DiffReport, DiffSummary, FormulaDiffResult, QueryChangeKind,
    QueryMetadataField, SheetId,
};
#[doc(hidden)]
pub use engine::{
    diff_grids_database_mode, diff_workbooks as diff_workbooks_with_pool, diff_workbooks_streaming,
    try_diff_workbooks as try_diff_workbooks_with_pool, try_diff_workbooks_streaming,
};
#[cfg(feature = "excel-open-xml")]
#[allow(deprecated)]
#[doc(hidden)]
pub use excel_open_xml::{ExcelOpenError, PackageError};
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
#[allow(deprecated)]
#[doc(hidden)]
pub use excel_open_xml::{open_data_mashup, open_workbook as open_workbook_with_pool};
pub use formula::{
    BinaryOperator, CellReference, ColRef, ExcelError, FormulaExpr, FormulaParseError,
    RangeReference, RowRef, UnaryOperator, formulas_equivalent_modulo_shift, parse_formula,
};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use grid_view::{
    ColHash, ColMeta, FrequencyClass, GridView, HashStats, RowHash, RowMeta, RowView,
};
#[doc(hidden)]
pub use m_ast::{MAstKind, MTokenDebug, tokenize_for_testing};
pub use m_ast::{
    MModuleAst, MParseError, ast_semantically_equal, canonicalize_m_ast, parse_m_expression,
};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[doc(hidden)]
pub use output::json::diff_report_to_cell_diffs;
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
#[doc(hidden)]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use output::json_lines::JsonLinesSink;
pub use package::{VbaModule, VbaModuleType, WorkbookPackage};
pub use session::DiffSession;
pub use sink::{CallbackSink, DiffSink, VecSink};
pub use string_pool::{StringId, StringPool};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ChartInfo, ChartObject, ColSignature, Grid,
    NamedRange, RowSignature, Sheet, SheetKind, Workbook,
};
pub use database_alignment::suggest_key_columns;
