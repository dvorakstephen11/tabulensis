mod common;

use common::{open_fixture_workbook, sid};
use excel_diff::{CellAddress, CellValue, Sheet, with_default_session};

#[test]
fn pg1_basic_two_sheets_structure() {
    let workbook = open_fixture_workbook("pg1_basic_two_sheets.xlsx");
    assert_eq!(workbook.sheets.len(), 2);
    assert_eq!(workbook.sheets[0].name, sid("Sheet1"));
    assert_eq!(workbook.sheets[1].name, sid("Sheet2"));
    assert!(matches!(
        workbook.sheets[0].kind,
        excel_diff::SheetKind::Worksheet
    ));
    assert!(matches!(
        workbook.sheets[1].kind,
        excel_diff::SheetKind::Worksheet
    ));

    let sheet1 = &workbook.sheets[0];
    assert_eq!(sheet1.grid.nrows, 3);
    assert_eq!(sheet1.grid.ncols, 3);
    with_default_session(|session| {
        assert_eq!(
            sheet1.grid.get(0, 0).and_then(|cell| cell
                .value
                .as_ref()
                .and_then(|v| v.as_text(session.strings()))),
            Some("R1C1")
        );
    });

    let sheet2 = &workbook.sheets[1];
    assert_eq!(sheet2.grid.nrows, 5);
    assert_eq!(sheet2.grid.ncols, 2);
    with_default_session(|session| {
        assert_eq!(
            sheet2.grid.get(0, 0).and_then(|cell| {
                cell.value
                    .as_ref()
                    .and_then(|v| v.as_text(session.strings()))
            }),
            Some("S2_R1C1")
        );
    });
}

#[test]
fn pg1_sparse_used_range_extents() {
    let workbook = open_fixture_workbook("pg1_sparse_used_range.xlsx");
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == sid("Sparse"))
        .expect("Sparse sheet present");

    assert_eq!(sheet.grid.nrows, 10);
    assert_eq!(sheet.grid.ncols, 7);

    assert_cell_text(sheet, 0, 0, "A1");
    assert_cell_text(sheet, 1, 1, "B2");
    assert_cell_text(sheet, 9, 6, "G10");
    assert_eq!(sheet.grid.cell_count(), 3);
}

#[test]
fn pg1_empty_and_mixed_sheets() {
    let workbook = open_fixture_workbook("pg1_empty_and_mixed_sheets.xlsx");

    let empty = sheet_by_name(&workbook, "Empty");
    assert_eq!(empty.grid.nrows, 0);
    assert_eq!(empty.grid.ncols, 0);
    assert_eq!(empty.grid.cell_count(), 0);

    let values_only = sheet_by_name(&workbook, "ValuesOnly");
    assert_eq!(values_only.grid.nrows, 10);
    assert_eq!(values_only.grid.ncols, 10);
    let values: Vec<_> = values_only
        .grid
        .iter_cells()
        .map(|(_, cell)| cell)
        .collect();
    assert!(
        values
            .iter()
            .all(|c| c.value.is_some() && c.formula.is_none()),
        "ValuesOnly cells should have values and no formulas"
    );
    assert_eq!(
        values_only
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_number)),
        Some(1.0)
    );

    let formulas = sheet_by_name(&workbook, "FormulasOnly");
    assert_eq!(formulas.grid.nrows, 10);
    assert_eq!(formulas.grid.ncols, 10);
    let first = formulas.grid.get(0, 0).expect("A1 should exist");
    with_default_session(|session| {
        assert_eq!(
            first.formula.map(|id| session.strings.resolve(id)),
            Some("ValuesOnly!A1")
        );
    });
    assert!(
        first.value.is_some(),
        "Formulas should surface cached values when present"
    );
    assert!(
        formulas
            .grid
            .iter_cells()
            .all(|(_, cell)| cell.formula.is_some()),
        "All cells should carry formulas in FormulasOnly"
    );
}

fn sheet_by_name<'a>(workbook: &'a excel_diff::Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == sid(name))
        .unwrap_or_else(|| panic!("sheet {name} not found"))
}

fn assert_cell_text(sheet: &Sheet, row: u32, col: u32, expected: &str) {
    let cell = sheet
        .grid
        .get(row, col)
        .unwrap_or_else(|| panic!("cell {expected} should exist"));
    assert_eq!(CellAddress::from_coords(row, col).to_a1(), expected);
    with_default_session(|session| {
        assert_eq!(
            cell.value
                .as_ref()
                .and_then(|v| v.as_text(session.strings()))
                .unwrap_or(""),
            expected
        );
    });
}
