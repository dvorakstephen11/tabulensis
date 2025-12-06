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
//! use excel_diff::{open_workbook, diff_workbooks};
//!
//! let wb_a = open_workbook("file_a.xlsx")?;
//! let wb_b = open_workbook("file_b.xlsx")?;
//! let report = diff_workbooks(&wb_a, &wb_b);
//!
//! for op in &report.ops {
//!     println!("{:?}", op);
//! }
//! ```

pub mod addressing;
pub(crate) mod column_alignment;
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
pub mod m_diff;
pub mod m_section;
pub mod output;
pub(crate) mod rect_block_move;
pub(crate) mod row_alignment;
pub mod workbook;

pub use addressing::{AddressParseError, address_to_index, index_to_address};
pub use container::{ContainerError, OpcContainer};
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries,
};
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use datamashup_package::{
    EmbeddedContent, PackageParts, PackageXml, SectionDocument, parse_package_parts,
};
pub use diff::{DiffOp, DiffReport, SheetId};
pub use engine::{diff_grids_database_mode, diff_workbooks};
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, open_data_mashup, open_workbook};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use grid_view::{ColHash, ColMeta, GridView, HashStats, RowHash, RowMeta, RowView};
pub use m_diff::{MQueryDiff, QueryChangeKind, diff_m_queries};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[cfg(feature = "excel-open-xml")]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ColSignature, Grid, RowSignature, Sheet, SheetKind,
    Workbook,
};
