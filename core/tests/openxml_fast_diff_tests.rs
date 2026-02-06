use excel_diff::{ContainerLimits, DiffConfig, StringPool, VecSink, WorkbookPackage, with_default_session};
use std::io::Cursor;

fn make_minimal_xlsx(sheet_xml: &str) -> Vec<u8> {
    use std::io::Write;
    use zip::CompressionMethod;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file("[Content_Types].xml", options)
            .expect("start [Content_Types].xml");
        zip.write_all(b"<Types/>")
            .expect("write [Content_Types].xml");

        zip.start_file("xl/workbook.xml", options)
            .expect("start xl/workbook.xml");
        zip.write_all(
            br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        )
        .expect("write xl/workbook.xml");

        zip.start_file("xl/_rels/workbook.xml.rels", options)
            .expect("start xl/_rels/workbook.xml.rels");
        zip.write_all(
            br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        )
        .expect("write xl/_rels/workbook.xml.rels");

        zip.start_file("xl/worksheets/sheet1.xml", options)
            .expect("start xl/worksheets/sheet1.xml");
        zip.write_all(sheet_xml.as_bytes())
            .expect("write xl/worksheets/sheet1.xml");

        zip.finish().expect("finish zip");
    }
    buf
}

#[test]
fn openxml_fast_diff_matches_slow_path_for_identical_workbooks() {
    with_default_session(|session| session.strings = StringPool::new());

    let sheet_xml = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#;
    let old_bytes = make_minimal_xlsx(sheet_xml);
    let new_bytes = make_minimal_xlsx(sheet_xml);

    let limits = ContainerLimits {
        max_entries: 10_000,
        max_part_uncompressed_bytes: 128 * 1024 * 1024,
        max_total_uncompressed_bytes: 256 * 1024 * 1024,
    };

    let mut fast_sink = VecSink::new();
    let fast_summary = WorkbookPackage::diff_openxml_streaming_fast_with_limits(
        Cursor::new(old_bytes.clone()),
        Cursor::new(new_bytes.clone()),
        limits,
        &DiffConfig::default(),
        &mut fast_sink,
    )
    .expect("fast streaming diff should succeed");
    let fast_ops = fast_sink.into_ops();
    assert_eq!(fast_ops.len(), fast_summary.op_count);

    let old_pkg = WorkbookPackage::open_with_limits(Cursor::new(old_bytes), limits)
        .expect("open old workbook");
    let new_pkg = WorkbookPackage::open_with_limits(Cursor::new(new_bytes), limits)
        .expect("open new workbook");
    let mut slow_sink = VecSink::new();
    let slow_summary = old_pkg
        .diff_streaming(&new_pkg, &DiffConfig::default(), &mut slow_sink)
        .expect("slow streaming diff should succeed");
    let slow_ops = slow_sink.into_ops();
    assert_eq!(slow_ops.len(), slow_summary.op_count);

    assert_eq!(fast_ops, slow_ops);
}

#[test]
fn openxml_fast_diff_matches_slow_path_for_single_cell_edit() {
    with_default_session(|session| session.strings = StringPool::new());

    let old_sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#;
    let new_sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>2</v></c></row></sheetData></worksheet>"#;
    let old_bytes = make_minimal_xlsx(old_sheet);
    let new_bytes = make_minimal_xlsx(new_sheet);

    let limits = ContainerLimits {
        max_entries: 10_000,
        max_part_uncompressed_bytes: 128 * 1024 * 1024,
        max_total_uncompressed_bytes: 256 * 1024 * 1024,
    };

    let mut fast_sink = VecSink::new();
    let fast_summary = WorkbookPackage::diff_openxml_streaming_fast_with_limits(
        Cursor::new(old_bytes.clone()),
        Cursor::new(new_bytes.clone()),
        limits,
        &DiffConfig::default(),
        &mut fast_sink,
    )
    .expect("fast streaming diff should succeed");
    let fast_ops = fast_sink.into_ops();
    assert_eq!(fast_ops.len(), fast_summary.op_count);
    assert!(!fast_ops.is_empty(), "expected at least one op");

    let old_pkg = WorkbookPackage::open_with_limits(Cursor::new(old_bytes), limits)
        .expect("open old workbook");
    let new_pkg = WorkbookPackage::open_with_limits(Cursor::new(new_bytes), limits)
        .expect("open new workbook");
    let mut slow_sink = VecSink::new();
    let slow_summary = old_pkg
        .diff_streaming(&new_pkg, &DiffConfig::default(), &mut slow_sink)
        .expect("slow streaming diff should succeed");
    let slow_ops = slow_sink.into_ops();
    assert_eq!(slow_ops.len(), slow_summary.op_count);

    assert_eq!(fast_ops, slow_ops);
}
