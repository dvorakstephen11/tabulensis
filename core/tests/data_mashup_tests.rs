use std::io::ErrorKind;

use excel_diff::{ExcelOpenError, RawDataMashup, open_data_mashup};

mod common;
use common::fixture_path;

#[test]
fn workbook_without_datamashup_returns_none() {
    let path = fixture_path("minimal.xlsx");
    let result = open_data_mashup(&path).expect("minimal workbook should load");
    assert!(result.is_none());
}

#[test]
fn workbook_with_valid_datamashup_parses() {
    let path = fixture_path("m_change_literal_b.xlsx");
    let raw = open_data_mashup(&path)
        .expect("valid mashup should load")
        .expect("mashup should be present");

    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
    assert!(!raw.metadata.is_empty());

    let assembled = assemble_top_level_bytes(&raw);
    let expected_len = 4 * 5
        + raw.package_parts.len()
        + raw.permissions.len()
        + raw.metadata.len()
        + raw.permission_bindings.len();
    assert_eq!(assembled.len(), expected_len);
}

#[test]
fn corrupt_base64_returns_error() {
    let path = fixture_path("corrupt_base64.xlsx");
    let err = open_data_mashup(&path).expect_err("corrupt base64 should fail");
    assert!(matches!(err, ExcelOpenError::DataMashupBase64Invalid));
}

#[test]
fn nonexistent_file_returns_io() {
    let path = fixture_path("missing_mashup.xlsx");
    let err = open_data_mashup(&path).expect_err("missing file should error");
    match err {
        ExcelOpenError::Io(e) => assert_eq!(e.kind(), ErrorKind::NotFound),
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn non_excel_container_returns_not_excel_error() {
    let path = fixture_path("random_zip.zip");
    let err = open_data_mashup(&path).expect_err("random zip should not parse");
    assert!(matches!(err, ExcelOpenError::NotExcelOpenXml));
}

fn assemble_top_level_bytes(raw: &RawDataMashup) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&raw.version.to_le_bytes());
    bytes.extend_from_slice(&(raw.package_parts.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.package_parts);
    bytes.extend_from_slice(&(raw.permissions.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permissions);
    bytes.extend_from_slice(&(raw.metadata.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.metadata);
    bytes.extend_from_slice(&(raw.permission_bindings.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permission_bindings);
    bytes
}
