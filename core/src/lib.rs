pub mod addressing;
pub mod container;
pub mod datamashup;
pub mod datamashup_framing;
pub mod datamashup_package;
pub mod diff;
pub mod engine;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod grid_parser;
pub mod m_diff;
pub mod m_section;
pub mod output;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
pub use container::{ContainerError, OpcContainer};
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries,
};
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use datamashup_package::{
    EmbeddedContent, PackageParts, PackageXml, SectionDocument, parse_package_parts,
};
pub use diff::{DiffOp, DiffReport, SheetId};
pub use engine::diff_workbooks;
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, open_data_mashup, open_workbook};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use m_diff::{MQueryDiff, QueryChangeKind, diff_m_queries};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[cfg(feature = "excel-open-xml")]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ColSignature, Grid, RowSignature, Sheet, SheetKind,
    Workbook,
};
