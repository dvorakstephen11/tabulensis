pub mod addressing;
pub mod diff;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod output;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
pub use diff::{ColSignature, DiffOp, DiffReport, RowSignature, SheetId};
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, RawDataMashup, open_data_mashup, open_workbook};
pub use output::json::{CellDiff, serialize_cell_diffs};
#[cfg(feature = "excel-open-xml")]
pub use output::json::{diff_workbooks, diff_workbooks_to_json};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, Grid, Row, Sheet, SheetKind, Workbook,
};
