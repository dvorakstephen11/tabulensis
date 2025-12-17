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

use std::cell::RefCell;

mod addressing;
pub(crate) mod alignment;
pub(crate) mod column_alignment;
mod config;
mod container;
pub(crate) mod database_alignment;
mod datamashup;
mod datamashup_framing;
mod datamashup_package;
mod diff;
mod engine;
#[cfg(feature = "excel-open-xml")]
mod excel_open_xml;
mod grid_parser;
mod grid_view;
pub(crate) mod hashing;
mod m_ast;
mod m_diff;
mod m_section;
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

#[cfg(feature = "excel-open-xml")]
#[deprecated(note = "use WorkbookPackage::open")]
#[allow(deprecated)]
#[doc(hidden)]
pub fn open_workbook(path: impl AsRef<std::path::Path>) -> Result<Workbook, ExcelOpenError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        excel_open_xml::open_workbook(path, &mut session.strings)
    })
}

pub use addressing::{AddressParseError, address_to_index, index_to_address};
pub use config::{DiffConfig, LimitBehavior};
pub use container::{ContainerError, OpcContainer};
#[doc(hidden)]
pub use datamashup::parse_metadata;
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries,
};
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use datamashup_package::{
    EmbeddedContent, PackageParts, PackageXml, SectionDocument, parse_package_parts,
};
pub use diff::{
    DiffError, DiffOp, DiffReport, DiffSummary, QueryChangeKind, QueryMetadataField, SheetId,
};
#[doc(hidden)]
pub use engine::{
    diff_grids_database_mode, diff_workbooks as diff_workbooks_with_pool, diff_workbooks_streaming,
    try_diff_workbooks as try_diff_workbooks_with_pool, try_diff_workbooks_streaming,
};
#[cfg(feature = "excel-open-xml")]
#[allow(deprecated)]
#[doc(hidden)]
pub use excel_open_xml::{
    ExcelOpenError, PackageError, open_data_mashup, open_workbook as open_workbook_with_pool,
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
#[cfg(feature = "excel-open-xml")]
#[doc(hidden)]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use output::json_lines::JsonLinesSink;
pub use package::WorkbookPackage;
pub use session::DiffSession;
pub use sink::{CallbackSink, DiffSink, VecSink};
pub use string_pool::{StringId, StringPool};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ColSignature, Grid, RowSignature, Sheet, SheetKind,
    Workbook,
};
