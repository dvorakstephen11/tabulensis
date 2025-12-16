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

pub mod addressing;
pub(crate) mod alignment;
pub(crate) mod column_alignment;
pub mod config;
pub mod container;
pub(crate) mod database_alignment;
pub mod datamashup;
pub mod datamashup_framing;
pub mod datamashup_package;
pub mod diff;
pub mod engine;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod grid_parser;
pub mod grid_view;
pub(crate) mod hashing;
pub mod m_ast;
pub mod m_diff;
pub mod m_section;
pub mod output;
pub mod package;
#[cfg(feature = "perf-metrics")]
pub mod perf;
pub(crate) mod rect_block_move;
pub(crate) mod region_mask;
pub(crate) mod row_alignment;
pub mod session;
pub mod sink;
pub mod string_pool;
pub mod workbook;

thread_local! {
    static DEFAULT_SESSION: RefCell<DiffSession> = RefCell::new(DiffSession::new());
}

pub fn with_default_session<T>(f: impl FnOnce(&mut DiffSession) -> T) -> T {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        f(&mut session)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[deprecated(note = "use WorkbookPackage::diff")]
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
pub fn open_workbook(path: impl AsRef<std::path::Path>) -> Result<Workbook, ExcelOpenError> {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        excel_open_xml::open_workbook(path, &mut session.strings)
    })
}

pub use addressing::{AddressParseError, address_to_index, index_to_address};
pub use config::DiffConfig;
pub use container::{ContainerError, OpcContainer};
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries,
};
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use datamashup_package::{
    EmbeddedContent, PackageParts, PackageXml, SectionDocument, parse_package_parts,
};
pub use diff::{DiffError, DiffOp, DiffReport, DiffSummary, QueryChangeKind, QueryMetadataField, SheetId};
pub use engine::{
    diff_grids_database_mode,
    diff_workbooks as diff_workbooks_with_pool,
    diff_workbooks_streaming,
    try_diff_workbooks as try_diff_workbooks_with_pool,
    try_diff_workbooks_streaming,
};
#[cfg(feature = "excel-open-xml")]
#[allow(deprecated)]
pub use excel_open_xml::{ExcelOpenError, PackageError, open_data_mashup, open_workbook as open_workbook_with_pool};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use grid_view::{ColHash, ColMeta, GridView, HashStats, RowHash, RowMeta, RowView};
pub use m_ast::{
    MModuleAst, MParseError, ast_semantically_equal, canonicalize_m_ast, parse_m_expression,
};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[cfg(feature = "excel-open-xml")]
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
