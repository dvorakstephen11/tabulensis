use excel_diff::{CellValue, address_to_index, index_to_address, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg2_addressing_matrix_consistency() {
    let workbook =
        open_workbook(fixture_path("pg2_addressing_matrix.xlsx")).expect("address fixture opens");
    let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == "Addresses")
        .unwrap_or_else(|| panic!("Addresses sheet present; found {:?}", sheet_names));

    for row in &sheet.grid.rows {
        for cell in &row.cells {
            if let Some(CellValue::Text(text)) = &cell.value {
                assert_eq!(cell.address.to_a1(), text.as_str());
                let (r, c) =
                    address_to_index(text).expect("address strings should parse to indices");
                assert_eq!((r, c), (cell.row, cell.col));
                assert_eq!(index_to_address(cell.row, cell.col), cell.address.to_a1());
            }
        }
    }
}
