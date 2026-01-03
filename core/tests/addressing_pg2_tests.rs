mod common;

use common::{open_fixture_workbook, sid};
use excel_diff::{CellValue, address_to_index, index_to_address, with_default_session};

#[test]
fn pg2_addressing_matrix_consistency() {
    let workbook = open_fixture_workbook("pg2_addressing_matrix.xlsx");
    let sheet_names: Vec<String> = with_default_session(|session| {
        workbook
            .sheets
            .iter()
            .map(|s| session.strings.resolve(s.name).to_string())
            .collect()
    });
    let addresses_id = sid("Addresses");
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == addresses_id)
        .unwrap_or_else(|| panic!("Addresses sheet present; found {:?}", sheet_names));

    for cell in sheet.grid.iter_cell_refs() {
        if let Some(CellValue::Text(text_id)) = cell.value {
            let text =
                with_default_session(|session| session.strings.resolve(*text_id).to_string());
            assert_eq!(cell.address.to_a1(), text.as_str());
            let (r, c) = address_to_index(&text).expect("address strings should parse to indices");
            assert_eq!((r, c), (cell.row, cell.col));
            assert_eq!(index_to_address(cell.row, cell.col), cell.address.to_a1());
        }
    }
}
