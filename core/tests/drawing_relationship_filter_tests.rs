use std::io::{Cursor, Write};

use excel_diff::WorkbookPackage;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut writer = ZipWriter::new(cursor);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, contents) in entries {
            writer.start_file(*name, options).expect("start zip entry");
            writer
                .write_all(contents)
                .expect("write zip entry contents");
        }
        writer.finish().expect("finish zip");
    }
    buf
}

#[test]
fn open_skips_unreferenced_drawing_relationships() {
    let content_types = br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"></Types>
"#;

    let workbook_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>
"#;

    let workbook_rels = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
                Target="worksheets/sheet1.xml"/>
</Relationships>
"#;

    let sheet_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <drawing r:id="rId1"/>
</worksheet>
"#;

    // The sheet relationships file contains two drawing relationships, but only `rId1` is
    // referenced by the worksheet XML via `<drawing r:id="...">`.
    let sheet_rels = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing"
                Target="../drawings/drawing1.xml"/>
  <Relationship Id="rId2"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing"
                Target="../drawings/drawing2.xml"/>
</Relationships>
"#;

    // A well-formed drawing part with no charts; the parser should accept it and find no refs.
    let drawing1_xml = br#"<?xml version="1.0" encoding="UTF-8"?><wsDr></wsDr>"#;
    // An invalid XML drawing part that should be ignored because it is not referenced by the sheet.
    let drawing2_xml = br#"<broken"#;

    let bytes = make_zip(&[
        ("[Content_Types].xml", content_types),
        ("xl/workbook.xml", workbook_xml),
        ("xl/_rels/workbook.xml.rels", workbook_rels),
        ("xl/worksheets/sheet1.xml", sheet_xml),
        ("xl/worksheets/_rels/sheet1.xml.rels", sheet_rels),
        ("xl/drawings/drawing1.xml", drawing1_xml),
        ("xl/drawings/drawing2.xml", drawing2_xml),
    ]);

    let pkg = WorkbookPackage::open(Cursor::new(bytes));
    assert!(
        pkg.is_ok(),
        "open should ignore unreferenced drawing relationships and succeed: {pkg:?}"
    );
}
