use std::fs;
use std::io::ErrorKind;

use excel_diff::{ExcelOpenError, SheetKind, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn open_minimal_workbook_succeeds() {
    let path = fixture_path("minimal.xlsx");
    let workbook = open_workbook(&path).expect("minimal workbook should open");
    assert_eq!(workbook.sheets.len(), 1);

    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.name, "Sheet1");
    assert!(matches!(sheet.kind, SheetKind::Worksheet));
    assert_eq!(sheet.grid.nrows, 1);
    assert_eq!(sheet.grid.ncols, 1);

    let cell = &sheet.grid.rows[0].cells[0];
    assert_eq!(cell.address.to_a1(), "A1");
    assert!(cell.value.is_some());
}

#[test]
fn open_nonexistent_file_returns_io_error() {
    let path = fixture_path("definitely_missing.xlsx");
    let err = open_workbook(&path).expect_err("missing file should error");
    match err {
        ExcelOpenError::Io(e) => assert_eq!(e.kind(), ErrorKind::NotFound),
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn random_zip_is_not_excel() {
    let path = fixture_path("random_zip.zip");
    let err = open_workbook(&path).expect_err("random zip should not parse");
    assert!(matches!(err, ExcelOpenError::NotExcelOpenXml));
}

#[test]
fn no_content_types_is_not_excel() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_workbook(&path).expect_err("missing content types should fail");
    assert!(matches!(err, ExcelOpenError::NotExcelOpenXml));
}

#[test]
fn not_zip_container_returns_error() {
    let path = std::env::temp_dir().join("excel_diff_not_zip.txt");
    fs::write(&path, "this is not a zip container").expect("write temp file");
    let err = open_workbook(&path).expect_err("non-zip should fail");
    assert!(matches!(err, ExcelOpenError::NotZipContainer));
    let _ = fs::remove_file(&path);
}
