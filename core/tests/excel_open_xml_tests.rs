mod common;

use common::{fixture_path, open_fixture_workbook, sid};
use excel_diff::{
    open_data_mashup, CellAddress, ContainerError, ContainerLimits, DataMashupError, OpcContainer,
    PackageError, SheetKind, WorkbookPackage,
};
use std::fs;
use std::io::{Cursor, ErrorKind, Write};
use std::path::Path;
use std::time::SystemTime;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

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
    let workbook = open_fixture_workbook("minimal.xlsx");
    assert_eq!(workbook.sheets.len(), 1);

    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.name, sid("Sheet1"));
    assert!(matches!(sheet.kind, SheetKind::Worksheet));
    assert_eq!(sheet.grid.nrows, 1);
    assert_eq!(sheet.grid.ncols, 1);

    let cell = sheet.grid.get(0, 0).expect("A1 should be present");
    assert_eq!(CellAddress::from_coords(0, 0).to_a1(), "A1");
    assert!(cell.value.is_some());
}

#[test]
fn open_nonexistent_file_returns_io_error() {
    let path = fixture_path("definitely_missing.xlsx");
    let file = std::fs::File::open(&path);
    assert!(file.is_err(), "missing file should error");
    let io_err = file.unwrap_err();
    assert_eq!(io_err.kind(), ErrorKind::NotFound);
}

#[test]
fn random_zip_is_not_excel() {
    let path = fixture_path("random_zip.zip");
    let file = std::fs::File::open(&path).expect("random zip file exists");
    let err = WorkbookPackage::open(file).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn no_content_types_is_not_excel() {
    let path = fixture_path("no_content_types.xlsx");
    let file = std::fs::File::open(&path).expect("no content types file exists");
    let err = WorkbookPackage::open(file).expect_err("missing content types should fail");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn not_zip_container_returns_error() {
    let path = fixture_path("not_a_zip.txt");
    let file = std::fs::File::open(&path).expect("fixture should exist");
    let err = WorkbookPackage::open(file).expect_err("non-zip should fail");
    assert!(matches!(
        err,
        PackageError::Container(ContainerError::NotZipContainer)
    ));
}

#[test]
fn missing_workbook_xml_returns_missing_part() {
    let path = temp_xlsx_path("missing_workbook_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    write_zip(&[("[Content_Types].xml", content_types)], &path);

    let file = std::fs::File::open(&path).expect("temp file exists");
    let err = WorkbookPackage::open(file).expect_err("missing workbook xml should error");
    match err {
        PackageError::MissingPart { path } => {
            assert_eq!(path, "xl/workbook.xml");
        }
        other => panic!("expected MissingPart for xl/workbook.xml, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn missing_worksheet_xml_returns_missing_part() {
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

    let file = std::fs::File::open(&path).expect("temp file exists");
    let err = WorkbookPackage::open(file).expect_err("missing worksheet xml should error");
    match err {
        PackageError::MissingPart { path: part_path } => {
            assert!(
                part_path.contains("sheet1.xml"),
                "expected path to contain sheet1.xml, got {part_path}"
            );
        }
        other => panic!("expected MissingPart for worksheet, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn truncated_zip_never_panics() {
    let valid_zip_bytes = {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut writer = ZipWriter::new(cursor);
            let options = FileOptions::default().compression_method(CompressionMethod::Stored);
            writer.start_file("[Content_Types].xml", options).unwrap();
            writer.write_all(b"<Types/>").unwrap();
            writer.finish().unwrap();
        }
        buf
    };

    for truncate_at in [0, 4, 10, 20, valid_zip_bytes.len() / 2] {
        let truncated = &valid_zip_bytes[..truncate_at.min(valid_zip_bytes.len())];
        let cursor = Cursor::new(truncated.to_vec());
        let result = std::panic::catch_unwind(|| OpcContainer::open_from_reader(cursor));
        assert!(
            result.is_ok(),
            "truncated ZIP at byte {} should not panic",
            truncate_at
        );
        let inner = result.unwrap();
        assert!(inner.is_err(), "truncated ZIP should return error, not Ok");
    }
}

#[test]
fn zip_bomb_defense_rejects_oversized_part() {
    let path = temp_xlsx_path("oversized_part");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    let large_content = "x".repeat(1024);

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("large_file.xml", &large_content),
        ],
        &path,
    );

    let file = std::fs::File::open(&path).expect("temp file exists");
    let limits = ContainerLimits {
        max_entries: 100,
        max_part_uncompressed_bytes: 512,
        max_total_uncompressed_bytes: 10_000,
    };
    let mut container = OpcContainer::open_from_reader_with_limits(file, limits)
        .expect("container should open (content_types is small)");

    let err = container
        .read_file_checked("large_file.xml")
        .expect_err("oversized part should be rejected");
    assert!(
        matches!(err, ContainerError::PartTooLarge { .. }),
        "expected PartTooLarge error, got {err:?}"
    );

    let _ = fs::remove_file(&path);
}

#[test]
fn oversized_content_types_is_rejected_during_open() {
    let path = temp_xlsx_path("oversized_content_types");
    let huge_content_types = "x".repeat(1024);

    write_zip(&[("[Content_Types].xml", &huge_content_types)], &path);

    let file = std::fs::File::open(&path).expect("temp file exists");
    let limits = ContainerLimits {
        max_entries: 100,
        max_part_uncompressed_bytes: 100,
        max_total_uncompressed_bytes: 10_000,
    };

    let err = match OpcContainer::open_from_reader_with_limits(file, limits) {
        Ok(_) => panic!("expected oversized [Content_Types].xml to be rejected"),
        Err(e) => e,
    };

    match err {
        ContainerError::PartTooLarge { path, size, limit } => {
            assert_eq!(path, "[Content_Types].xml");
            assert_eq!(size, 1024);
            assert_eq!(limit, 100);
        }
        other => panic!("expected PartTooLarge, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn invalid_xml_in_workbook_does_not_panic() {
    let path = temp_xlsx_path("invalid_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    let malformed_workbook = "<workbook><sheets><sheet name='x' unclosed";

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("xl/workbook.xml", malformed_workbook),
        ],
        &path,
    );

    let file = std::fs::File::open(&path).expect("temp file exists");
    let result = std::panic::catch_unwind(|| WorkbookPackage::open(file));
    assert!(
        result.is_ok(),
        "malformed XML should not panic (catch_unwind succeeded)"
    );
    let err = result
        .unwrap()
        .expect_err("malformed XML should return error, not Ok");
    match err {
        PackageError::InvalidXml {
            part, line, column, ..
        } => {
            assert_eq!(part, "xl/workbook.xml");
            assert!(line > 0, "expected line > 0, got {line}");
            assert!(column > 0, "expected column > 0, got {column}");
        }
        other => panic!("expected InvalidXml, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn invalid_xml_in_relationships_includes_part_and_position() {
    let path = temp_xlsx_path("invalid_rels_xml");
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

    let malformed_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Target="worksheets/sheet1.xml""#;

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("xl/workbook.xml", workbook_xml),
            ("xl/_rels/workbook.xml.rels", malformed_rels),
        ],
        &path,
    );

    let file = std::fs::File::open(&path).expect("temp file exists");
    let result = std::panic::catch_unwind(|| WorkbookPackage::open(file));
    assert!(
        result.is_ok(),
        "malformed relationships XML should not panic"
    );
    let err = result
        .unwrap()
        .expect_err("malformed relationships XML should error");
    match err {
        PackageError::InvalidXml {
            part, line, column, ..
        } => {
            assert_eq!(part, "xl/_rels/workbook.xml.rels");
            assert!(line > 0, "expected line > 0, got {line}");
            assert!(column > 0, "expected column > 0, got {column}");
        }
        other => panic!("expected InvalidXml, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn invalid_xml_in_worksheet_includes_part_and_position() {
    let path = temp_xlsx_path("invalid_sheet_xml");
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

    let malformed_sheet = r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1"><v>1</v>
    </row>
  </sheetData>
</worksheet>"#;

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("xl/workbook.xml", workbook_xml),
            ("xl/_rels/workbook.xml.rels", relationships),
            ("xl/worksheets/sheet1.xml", malformed_sheet),
        ],
        &path,
    );

    let file = std::fs::File::open(&path).expect("temp file exists");
    let result = std::panic::catch_unwind(|| WorkbookPackage::open(file));
    assert!(result.is_ok(), "malformed worksheet XML should not panic");
    let err = result
        .unwrap()
        .expect_err("malformed worksheet XML should error");
    match err {
        PackageError::InvalidXml {
            part, line, column, ..
        } => {
            assert_eq!(part, "xl/worksheets/sheet1.xml");
            assert!(line > 0, "expected line > 0, got {line}");
            assert!(column > 0, "expected column > 0, got {column}");
        }
        other => panic!("expected InvalidXml, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn datamashup_base64_decodes_but_framing_invalid_includes_part() {
    let path = temp_xlsx_path("dm_framing_invalid");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    let dm_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns:dm="http://schemas.microsoft.com/DataMashup">
  <dm:DataMashup>AAAA</dm:DataMashup>
</root>"#;

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("customXml/item1.xml", dm_xml),
        ],
        &path,
    );

    let err = open_data_mashup(&path).expect_err("expected framing error");
    let err = match err {
        PackageError::WithPath { source, .. } => *source,
        other => other,
    };

    match err {
        PackageError::DataMashupPartError { part, source } => {
            assert_eq!(part, "customXml/item1.xml");
            assert!(matches!(source, DataMashupError::FramingInvalid));
        }
        other => panic!("expected DataMashupPartError, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn corrupt_inputs_never_panic() {
    let test_cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("garbage", vec![0xFF, 0xFE, 0x00, 0x01, 0x02, 0x03]),
        ("partial_zip_header", vec![0x50, 0x4B, 0x03, 0x04]),
        (
            "random_bytes",
            (0..100).map(|i| (i * 17 + 31) as u8).collect(),
        ),
    ];

    for (name, bytes) in test_cases {
        let cursor = Cursor::new(bytes);
        let result = std::panic::catch_unwind(|| WorkbookPackage::open(cursor));
        assert!(result.is_ok(), "corrupt input '{}' should not panic", name);
    }
}
