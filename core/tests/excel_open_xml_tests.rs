use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::time::SystemTime;

use excel_diff::{ContainerError, ExcelOpenError, SheetKind, open_workbook};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

mod common;
use common::fixture_path;

fn temp_xlsx_path(prefix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    path.push(format!("excel_diff_{prefix}_{nanos}.xlsx"));
    path
}

fn write_zip(entries: &[(&str, &str)], path: &Path) {
    let file = fs::File::create(path).expect("create temp zip");
    let mut writer = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, contents) in entries {
        writer.start_file(*name, options).expect("start zip entry");
        writer
            .write_all(contents.as_bytes())
            .expect("write zip entry");
    }

    writer.finish().expect("finish zip");
}

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

    let cell = sheet.grid.get(0, 0).expect("A1 should be present");
    assert_eq!(cell.address.to_a1(), "A1");
    assert!(cell.value.is_some());
}

#[test]
fn open_nonexistent_file_returns_io_error() {
    let path = fixture_path("definitely_missing.xlsx");
    let err = open_workbook(&path).expect_err("missing file should error");
    match err {
        ExcelOpenError::Container(ContainerError::Io(e)) => {
            assert_eq!(e.kind(), ErrorKind::NotFound)
        }
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn random_zip_is_not_excel() {
    let path = fixture_path("random_zip.zip");
    let err = open_workbook(&path).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn no_content_types_is_not_excel() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_workbook(&path).expect_err("missing content types should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn not_zip_container_returns_error() {
    let path = std::env::temp_dir().join("excel_diff_not_zip.txt");
    fs::write(&path, "this is not a zip container").expect("write temp file");
    let err = open_workbook(&path).expect_err("non-zip should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotZipContainer)
    ));
    let _ = fs::remove_file(&path);
}

#[test]
fn missing_workbook_xml_returns_workbookxmlmissing() {
    let path = temp_xlsx_path("missing_workbook_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    write_zip(&[("[Content_Types].xml", content_types)], &path);

    let err = open_workbook(&path).expect_err("missing workbook xml should error");
    assert!(matches!(err, ExcelOpenError::WorkbookXmlMissing));

    let _ = fs::remove_file(&path);
}

#[test]
fn missing_worksheet_xml_returns_worksheetxmlmissing() {
    let path = temp_xlsx_path("missing_worksheet_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    let workbook_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;

    let relationships = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
                Target="worksheets/sheet1.xml"/>
</Relationships>"#;

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("xl/workbook.xml", workbook_xml),
            ("xl/_rels/workbook.xml.rels", relationships),
        ],
        &path,
    );

    let err = open_workbook(&path).expect_err("missing worksheet xml should error");
    match err {
        ExcelOpenError::WorksheetXmlMissing { sheet_name } => {
            assert_eq!(sheet_name, "Sheet1");
        }
        other => panic!("expected WorksheetXmlMissing, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}
