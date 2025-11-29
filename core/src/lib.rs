pub mod addressing;
#[cfg(feature = "excel-open-xml")]
pub mod container;
#[cfg(feature = "excel-open-xml")]
pub mod datamashup_framing;
pub mod diff;
pub mod engine;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
#[cfg(feature = "excel-open-xml")]
pub mod grid_parser;
pub mod output;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
#[cfg(feature = "excel-open-xml")]
pub use container::{ContainerError, OpcContainer};
#[cfg(feature = "excel-open-xml")]
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use diff::{DiffOp, DiffReport, SheetId};
pub use engine::diff_workbooks;
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, open_data_mashup, open_workbook};
#[cfg(feature = "excel-open-xml")]
pub use grid_parser::{GridParseError, SheetDescriptor};
#[cfg(feature = "excel-open-xml")]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ColSignature, Grid, RowSignature, Sheet, SheetKind,
    Workbook,
};
