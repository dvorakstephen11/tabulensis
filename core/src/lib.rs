//! Excel Diff: a library for comparing Excel workbooks.
//!
//! The main entry point is [`WorkbookPackage`], which can parse a workbook (when the
//! `excel-open-xml` feature is enabled) and then diff it against another workbook.
//!
//! The diff includes:
//! - sheet/grid ops (cell edits, row/column adds/removes, block moves)
//! - object ops (named ranges, charts, VBA modules)
//! - Power Query ops (M query add/remove/rename and definition/metadata changes)
//!
//! # Quick start
//!
//! ```no_run
//! use excel_diff::{DiffConfig, WorkbookPackage};
//! use std::fs::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let old_pkg = WorkbookPackage::open(File::open("old.xlsx")?)?;
//! let new_pkg = WorkbookPackage::open(File::open("new.xlsx")?)?;
//!
//! let report = old_pkg.diff(&new_pkg, &DiffConfig::default());
//! println!("ops={}", report.ops.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Streaming (JSON Lines)
//!
//! ```no_run
//! use excel_diff::{DiffConfig, JsonLinesSink, WorkbookPackage};
//! use std::fs::File;
//! use std::io::{self, BufWriter};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let old_pkg = WorkbookPackage::open(File::open("old.xlsx")?)?;
//! let new_pkg = WorkbookPackage::open(File::open("new.xlsx")?)?;
//!
//! let stdout = io::stdout();
//! let mut sink = JsonLinesSink::new(BufWriter::new(stdout.lock()));
//! let summary = old_pkg.diff_streaming(&new_pkg, &DiffConfig::default(), &mut sink)?;
//! eprintln!("complete={} ops={}", summary.complete, summary.op_count);
//! # Ok(())
//! # }
//! ```
//!
//! # Database mode (key-based diff)
//!
//! ```no_run
//! use excel_diff::{DiffConfig, WorkbookPackage};
//! use std::fs::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let old_pkg = WorkbookPackage::open(File::open("old.xlsx")?)?;
//! let new_pkg = WorkbookPackage::open(File::open("new.xlsx")?)?;
//!
//! // Key columns are 0-based indices (A=0, B=1, ...).
//! let keys: Vec<u32> = vec![0, 2]; // A,C
//! let report = old_pkg.diff_database_mode(&new_pkg, "Data", &keys, &DiffConfig::default())?;
//! println!("ops={}", report.ops.len());
//! # Ok(())
//! # }
//! ```

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

#[cfg(all(feature = "parallel", target_arch = "wasm32"))]
compile_error!("feature \"parallel\" is not supported on wasm32");

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
mod diffable;
mod engine;
#[cfg(feature = "excel-open-xml")]
mod excel_open_xml;
mod formula;
mod formula_diff;
mod grid_metadata;
mod grid_parser;
mod grid_view;
mod memory_estimate;
pub(crate) mod hashing;
mod matching;
mod m_ast;
mod m_ast_diff;
mod m_diff;
mod m_section;
mod m_semantic_detail;
#[cfg(feature = "model-diff")]
mod model;
#[cfg(feature = "model-diff")]
mod model_diff;
#[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
mod tabular_schema;
mod object_diff;
mod output;
mod package;
#[cfg(all(feature = "perf-metrics", not(target_arch = "wasm32")))]
mod memory_metrics;
mod progress;
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

#[cfg(all(feature = "perf-metrics", not(target_arch = "wasm32")))]
#[global_allocator]
static GLOBAL_ALLOC: memory_metrics::CountingAllocator<std::alloc::System> =
    memory_metrics::CountingAllocator::new(std::alloc::System);

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

#[cfg(feature = "legacy-api")]
#[deprecated(note = "use WorkbookPackage::diff")]
#[doc(hidden)]
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    DEFAULT_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        engine::diff_workbooks(old, new, &mut session.strings, config)
    })
}

#[cfg(feature = "legacy-api")]
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

#[cfg(all(feature = "legacy-api", feature = "excel-open-xml", feature = "std-fs"))]
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
        diff_workbooks_streaming, diff_workbooks_streaming_with_progress, diff_workbooks_with_progress,
        try_diff_grids_database_mode_streaming, try_diff_workbooks as try_diff_workbooks_with_pool,
        try_diff_workbooks_streaming,
        try_diff_workbooks_streaming_with_progress, try_diff_workbooks_with_progress,
    };
    pub use crate::session::DiffSession;
    pub use crate::sink::{CallbackSink, DiffSink, VecSink};
    pub use crate::string_pool::{StringId, StringPool};
}

pub use addressing::{AddressParseError, address_to_index, index_to_address};
pub use config::{DiffConfig, DiffConfigBuilder, LimitBehavior, SemanticNoisePolicy};
pub use container::{ContainerError, ContainerLimits, OpcContainer, ZipContainer};
#[doc(hidden)]
pub use datamashup::parse_metadata;
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup,
    build_embedded_queries, build_queries,
};
pub use datamashup_framing::{DataMashupError, RawDataMashup, parse_data_mashup};
pub use datamashup_package::{
    DataMashupLimits, EmbeddedContent, PackageParts, PackageXml, SectionDocument,
    parse_package_parts, parse_package_parts_with_limits,
};
pub use diff::{
    AstDiffMode, AstDiffSummary, AstMoveHint, ColumnTypeChange, DiffError, DiffOp, DiffReport,
    DiffSummary, ExtractedColumnTypeChanges, ExtractedRenamePairs, ExtractedString,
    ExtractedStringList, FormulaDiffResult, QueryChangeKind, QueryMetadataField,
    QuerySemanticDetail, RenamePair, SheetId, StepChange, StepDiff, StepParams, StepSnapshot,
    StepType,
};
pub use diffable::{DiffContext, Diffable};
#[doc(hidden)]
pub use engine::{
    diff_grids_database_mode, diff_workbooks as diff_workbooks_with_pool, diff_workbooks_streaming,
    diff_workbooks_streaming_with_progress, diff_workbooks_with_progress,
    try_diff_grids_database_mode_streaming, try_diff_workbooks as try_diff_workbooks_with_pool,
    try_diff_workbooks_streaming,
    try_diff_workbooks_streaming_with_progress, try_diff_workbooks_with_progress,
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
pub use m_ast::{MAstAccessKind, MAstKind, MTokenDebug, tokenize_for_testing};
pub use m_ast::{
    MModuleAst, MParseError, ast_semantically_equal, canonicalize_m_ast, parse_m_expression,
};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[cfg(feature = "model-diff")]
pub use model::{Measure, Model, ModelColumn, ModelTable};
#[cfg(feature = "model-diff")]
pub use model_diff::diff_models;
#[doc(hidden)]
pub use output::json::diff_report_to_cell_diffs;
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
#[doc(hidden)]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use output::json_lines::JsonLinesSink;
pub use package::{PbixPackage, VbaModule, VbaModuleType, WorkbookPackage};
pub use progress::{NoProgress, ProgressCallback};
pub use session::DiffSession;
pub use sink::{CallbackSink, DiffSink, VecSink};
pub use string_pool::{StringId, StringPool};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ChartInfo, ChartObject, ColSignature, Grid,
    NamedRange, RowSignature, Sheet, SheetKind, Workbook,
};
pub use database_alignment::suggest_key_columns;
