pub mod addressing;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, RawDataMashup, open_data_mashup, open_workbook};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, Grid, Row, Sheet, SheetKind, Workbook,
};
