#![feature(let_chains)]
//! Tabulensis: a library for comparing Excel workbooks.
//!
//! The main entry point is [`WorkbookPackage`], which can parse a workbook (when the
//! `excel-open-xml` feature is enabled) and then diff it against another workbook.
//!
//! The diff includes:
//! - sheet/grid ops (cell edits, row/column adds/removes, block moves)
//! - object ops (named ranges, charts, VBA modules)
//! - Power Query ops (M query add/remove/rename and definition/metadata changes)
//!
//! # Architecture overview
//!
//! The pipeline is Parse -> IR -> Diff -> Output. For the detailed narrative and entry-point map,
//! see `docs/maintainers/architecture.md` and `docs/maintainers/entrypoints.md`.
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
mod capabilities;
pub(crate) mod column_alignment;
mod config;
mod container;
mod database_alignment;
mod datamashup;
mod datamashup_framing;
mod datamashup_package;
#[cfg(feature = "model-diff")]
mod dax;
mod diff;
mod diffable;
mod engine;
pub mod error_codes;
#[cfg(feature = "excel-open-xml")]
mod excel_open_xml;
mod formula;
mod formula_diff;
mod grid_metadata;
mod grid_parser;
mod grid_view;
pub(crate) mod hashing;
mod m_ast;
mod m_ast_diff;
mod m_diff;
mod m_section;
mod m_semantic_detail;
mod matching;
mod memory_estimate;
#[cfg(all(feature = "perf-metrics", not(target_arch = "wasm32")))]
mod memory_metrics;
#[cfg(feature = "model-diff")]
mod model;
#[cfg(feature = "model-diff")]
mod model_diff;
mod object_diff;
mod output;
mod package;
mod pbip;
#[cfg(feature = "perf-metrics")]
#[doc(hidden)]
pub mod perf;
mod permission_bindings;
mod policy;
mod progress;
pub(crate) mod rect_block_move;
pub(crate) mod region_mask;
pub(crate) mod row_alignment;
mod session;
mod sink;
mod string_pool;
#[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
mod tabular_schema;
mod vba;
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
///
/// ## Leaf diffs
///
/// Leaf diffs compare individual grids or sheets without workbook orchestration. Grid diffs
/// use a default sheet id of "<grid>".
///
/// ```no_run
/// use excel_diff::{DiffConfig, Grid, StringPool};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut pool = StringPool::new();
/// let old = Grid::new(1, 1);
/// let new = Grid::new(1, 1);
/// let report =
///     excel_diff::advanced::diff_grids_with_pool(&old, &new, &mut pool, &DiffConfig::default());
/// println!("ops={}", report.ops.len());
/// # Ok(())
/// # }
/// ```
///
/// ```no_run
/// use excel_diff::{DiffConfig, Grid, Sheet, SheetKind, StringPool};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut pool = StringPool::new();
/// let sheet_id = pool.intern("Sheet1");
/// let old = Sheet {
///     name: sheet_id,
///     workbook_sheet_id: None,
///     kind: SheetKind::Worksheet,
///     grid: Grid::new(1, 1),
/// };
/// let new = Sheet {
///     name: sheet_id,
///     workbook_sheet_id: None,
///     kind: SheetKind::Worksheet,
///     grid: Grid::new(1, 1),
/// };
/// let report =
///     excel_diff::advanced::diff_sheets_with_pool(&old, &new, &mut pool, &DiffConfig::default());
/// println!("ops={}", report.ops.len());
/// # Ok(())
/// # }
/// ```
///
/// When streaming leaf diffs, all strings referenced by emitted ops must be interned before
/// `begin()` is called. See `docs/streaming_contract.md` for the full contract.
pub mod advanced {
    pub use crate::engine::{
        diff_grids as diff_grids_with_pool, diff_grids_database_mode, diff_grids_streaming,
        diff_grids_streaming_with_progress, diff_sheets as diff_sheets_with_pool,
        diff_sheets_streaming, diff_sheets_streaming_with_progress,
        diff_workbooks as diff_workbooks_with_pool, diff_workbooks_streaming,
        diff_workbooks_streaming_with_progress, diff_workbooks_with_progress,
        try_diff_grids as try_diff_grids_with_pool, try_diff_grids_database_mode_streaming,
        try_diff_grids_streaming, try_diff_grids_streaming_with_progress,
        try_diff_sheets as try_diff_sheets_with_pool, try_diff_sheets_streaming,
        try_diff_sheets_streaming_with_progress,
        try_diff_workbooks as try_diff_workbooks_with_pool, try_diff_workbooks_streaming,
        try_diff_workbooks_streaming_with_progress, try_diff_workbooks_with_progress,
    };
    pub use crate::session::DiffSession;
    pub use crate::sink::{CallbackSink, DiffSink, VecSink};
    pub use crate::string_pool::{StringId, StringPool};
}

pub use addressing::{address_to_index, index_to_address, AddressParseError};
pub use capabilities::{engine_features, EngineFeatures};
pub use config::{DiffConfig, DiffConfigBuilder, LimitBehavior, SemanticNoisePolicy};
pub use container::{
    ContainerError, ContainerLimits, OpcContainer, ZipContainer, ZipEntryFingerprint,
};
#[doc(hidden)]
pub use datamashup::parse_metadata;
pub use datamashup::{
    build_data_mashup, build_data_mashup_with_decryptor, build_embedded_queries, build_queries,
    DataMashup, Metadata, Permissions, Query, QueryMetadata,
};
#[doc(hidden)]
pub use datamashup_framing::read_datamashup_text;
pub use datamashup_framing::{
    decode_datamashup_base64, parse_data_mashup, DataMashupError, RawDataMashup,
};
pub use datamashup_package::{
    parse_package_parts, parse_package_parts_with_limits, DataMashupLimits, EmbeddedContent,
    PackageParts, PackageXml, SectionDocument,
};
pub use diff::{
    AstDiffMode, AstDiffSummary, AstMoveHint, ColumnTypeChange, DiffError, DiffOp, DiffReport,
    DiffSummary, ExpressionChangeKind, ExtractedColumnTypeChanges, ExtractedRenamePairs,
    ExtractedString, ExtractedStringList, FormulaDiffResult, QueryChangeKind, QueryMetadataField,
    QuerySemanticDetail, RenamePair, SheetId, StepChange, StepDiff, StepParams, StepSnapshot,
    StepType,
};
#[cfg(feature = "model-diff")]
pub use diff::{ModelColumnProperty, RelationshipProperty};
pub use diffable::{DiffContext, Diffable};
#[doc(hidden)]
pub use engine::{
    diff_grids as diff_grids_with_pool, diff_grids_database_mode, diff_grids_streaming,
    diff_grids_streaming_with_progress, diff_sheets as diff_sheets_with_pool,
    diff_sheets_streaming, diff_sheets_streaming_with_progress,
    diff_workbooks as diff_workbooks_with_pool, diff_workbooks_streaming,
    diff_workbooks_streaming_with_progress, diff_workbooks_with_progress,
    try_diff_grids as try_diff_grids_with_pool, try_diff_grids_database_mode_streaming,
    try_diff_grids_streaming, try_diff_grids_streaming_with_progress,
    try_diff_sheets as try_diff_sheets_with_pool, try_diff_sheets_streaming,
    try_diff_sheets_streaming_with_progress, try_diff_workbooks as try_diff_workbooks_with_pool,
    try_diff_workbooks_streaming, try_diff_workbooks_streaming_with_progress,
    try_diff_workbooks_with_progress,
};
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
#[allow(deprecated)]
#[doc(hidden)]
pub use excel_open_xml::{open_data_mashup, open_workbook as open_workbook_with_pool};
#[cfg(feature = "excel-open-xml")]
#[allow(deprecated)]
#[doc(hidden)]
pub use excel_open_xml::{ExcelOpenError, PackageError};
pub use formula::{
    formulas_equivalent_modulo_shift, parse_formula, BinaryOperator, CellReference, ColRef,
    ExcelError, FormulaExpr, FormulaParseError, RangeReference, RowRef, UnaryOperator,
};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use grid_view::{
    ColHash, ColMeta, FrequencyClass, GridView, HashStats, RowHash, RowMeta, RowView,
};
pub use m_ast::{
    ast_semantically_equal, canonicalize_m_ast, parse_m_expression, MModuleAst, MParseError,
};
#[doc(hidden)]
pub use m_ast::{tokenize_for_testing, MAstAccessKind, MAstKind, MTokenDebug};
pub use m_section::{parse_section_members, SectionMember, SectionParseError};
#[cfg(feature = "model-diff")]
pub use model::{Measure, Model, ModelColumn, ModelRelationship, ModelTable};
#[cfg(feature = "model-diff")]
pub use model_diff::{diff_models, ModelDiffResult};
pub use permission_bindings::{
    DpapiDecryptError, DpapiDecryptor, PermissionBindingsKind, PermissionBindingsStatus,
};

#[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
#[doc(hidden)]
pub fn datamodel_schema_parse_counts(
    bytes: &[u8],
) -> Result<(usize, usize, usize), crate::excel_open_xml::PackageError> {
    let raw = tabular_schema::parse_data_model_schema(bytes)?;
    Ok((
        raw.tables.len(),
        raw.relationships.len(),
        raw.measures.len(),
    ))
}
pub use database_alignment::suggest_key_columns;
#[doc(hidden)]
pub use output::json::diff_report_to_cell_diffs;
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
#[doc(hidden)]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{serialize_cell_diffs, serialize_diff_report, CellDiff};
pub use output::json_lines::JsonLinesSink;
pub use package::{OpenXmlDiffError, PbixPackage, WorkbookPackage};
pub use pbip::{
    diff_snapshots as diff_pbip_snapshots, PbipChangeKind, PbipDiffReport, PbipDocDiff,
    PbipDocRecord, PbipDocSnapshot, PbipDocType, PbipEntityDiff, PbipEntityKind,
    PbipNormalizationProfile, PbipProjectSnapshot, NormalizationApplied, NormalizationError,
    normalize_doc_text,
};
#[cfg(feature = "std-fs")]
pub use pbip::{snapshot_project_from_fs as snapshot_pbip_project, PbipScanConfig, PbipScanError};
pub use policy::{should_use_large_mode, AUTO_STREAM_CELL_THRESHOLD};
pub use progress::{NoProgress, ProgressCallback};
pub use session::DiffSession;
pub use sink::{CallbackSink, DiffSink, VecSink};
pub use string_pool::{StringId, StringPool};
pub use vba::{VbaModule, VbaModuleType};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ChartInfo, ChartObject, ColSignature, Grid,
    NamedRange, RowSignature, Sheet, SheetKind, Workbook,
};
