use std::fs::File;
use std::io::{ErrorKind, Read};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use excel_diff::{ExcelOpenError, RawDataMashup, open_data_mashup};
use quick_xml::{Reader, events::Event};
use zip::ZipArchive;

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
    let expected = datamashup_bytes_from_fixture(&path);
    assert_eq!(assembled, expected);
}

#[test]
fn datamashup_with_base64_whitespace_parses() {
    let path = fixture_path("mashup_base64_whitespace.xlsx");
    let raw = open_data_mashup(&path)
        .expect("whitespace in base64 payload should be tolerated")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_le_datamashup_parses() {
    let path = fixture_path("mashup_utf16_le.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16LE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_be_datamashup_parses() {
    let path = fixture_path("mashup_utf16_be.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16BE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn corrupt_base64_returns_error() {
    let path = fixture_path("corrupt_base64.xlsx");
    let err = open_data_mashup(&path).expect_err("corrupt base64 should fail");
    assert!(matches!(err, ExcelOpenError::DataMashupBase64Invalid));
}

#[test]
fn duplicate_datamashup_parts_are_rejected() {
    let path = fixture_path("duplicate_datamashup_parts.xlsx");
    let err = open_data_mashup(&path).expect_err("duplicate DataMashup parts should be rejected");
    assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
}

#[test]
fn duplicate_datamashup_elements_are_rejected() {
    let path = fixture_path("duplicate_datamashup_elements.xlsx");
    let err =
        open_data_mashup(&path).expect_err("duplicate DataMashup elements should be rejected");
    assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
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

#[test]
fn missing_content_types_is_not_excel_error() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_data_mashup(&path).expect_err("missing [Content_Types].xml should fail");
    assert!(matches!(err, ExcelOpenError::NotExcelOpenXml));
}

#[test]
fn non_zip_file_returns_not_zip_error() {
    let path = fixture_path("not_a_zip.txt");
    let err = open_data_mashup(&path).expect_err("non-zip input should not parse as Excel");
    assert!(matches!(err, ExcelOpenError::NotZipContainer));
}

fn datamashup_bytes_from_fixture(path: &std::path::Path) -> Vec<u8> {
    let file = File::open(path).expect("fixture should be readable");
    let mut archive = ZipArchive::new(file).expect("fixture should be a zip container");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip entry should be readable");
        let name = file.name().to_string();
        if !name.starts_with("customXml/") || !name.ends_with(".xml") {
            continue;
        }

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("XML part should read");
        if let Some(text) = extract_datamashup_base64(&buf) {
            let cleaned: String = text.split_whitespace().collect();
            return STANDARD
                .decode(cleaned.as_bytes())
                .expect("DataMashup base64 should decode");
        }
    }

    panic!("DataMashup element not found in {}", path.display());
}

fn extract_datamashup_base64(xml: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup {
                    return None;
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t.unescape().ok()?.into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                content.push_str(&String::from_utf8_lossy(&t.into_inner()));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return None;
                }
                return Some(content.clone());
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
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
