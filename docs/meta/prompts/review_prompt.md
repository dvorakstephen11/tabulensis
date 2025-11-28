# Codebase Context for Review

## Directory Structure

```text
/
  README.md
  core/
    Cargo.lock
    Cargo.toml
    src/
      addressing.rs
      excel_open_xml.rs
      lib.rs
      main.rs
      workbook.rs
      output/
        json.rs
        mod.rs
    tests/
      addressing_pg2_tests.rs
      data_mashup_tests.rs
      excel_open_xml_tests.rs
      integration_test.rs
      output_tests.rs
      pg1_ir_tests.rs
      pg3_snapshot_tests.rs
      common/
        mod.rs
  fixtures/
    manifest.yaml
    pyproject.toml
    README.md
    requirements.txt
    generated/
      corrupt_base64.xlsx
      db_equal_ordered_a.xlsx
      db_equal_ordered_b.xlsx
      db_row_added_b.xlsx
      duplicate_datamashup_elements.xlsx
      duplicate_datamashup_parts.xlsx
      grid_large_dense.xlsx
      grid_large_noise.xlsx
      json_diff_bool_a.xlsx
      json_diff_bool_b.xlsx
      json_diff_single_cell_a.xlsx
      json_diff_single_cell_b.xlsx
      mashup_base64_whitespace.xlsx
      mashup_utf16_be.xlsx
      mashup_utf16_le.xlsx
      minimal.xlsx
      m_change_literal_b.xlsx
      not_a_zip.txt
      no_content_types.xlsx
      pg1_basic_two_sheets.xlsx
      pg1_empty_and_mixed_sheets.xlsx
      pg1_sparse_used_range.xlsx
      pg2_addressing_matrix.xlsx
      pg3_value_and_formula_cells.xlsx
      random_zip.zip
    src/
      generate.py
      __init__.py
      generators/
        base.py
        corrupt.py
        database.py
        grid.py
        mashup.py
        perf.py
        __init__.py
    templates/
      base_query.xlsx
  logs/
```

## File Contents

### File: `.gitignore`

```
# Rust
target/
**/target/
**/*.rs.bk

# Python
__pycache__/
**/__pycache__/
.venv/
*.pyc
*.egg-info/

# Shared Generated Data
fixtures/generated/*.xlsx
fixtures/generated/*.pbix
fixtures/generated/*.zip
fixtures/generated/*.csv


# Docs
docs/meta/completion_estimates/
```

---

### File: `core\Cargo.toml`

```yaml
[package]
name = "excel_diff"
version = "0.1.0"
edition = "2024"

[features]
default = ["excel-open-xml"]
excel-open-xml = ["dep:zip", "dep:quick-xml"]

[dependencies]
quick-xml = { version = "0.32", optional = true }
thiserror = "1.0"
zip = { version = "0.6", default-features = false, features = ["deflate"], optional = true }
base64 = "0.22"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[dev-dependencies]
```

---

### File: `core\src\addressing.rs`

```rust
/// Convert zero-based (row, col) indices to an Excel A1 address string.
pub fn index_to_address(row: u32, col: u32) -> String {
    let mut col_index = col;
    let mut col_label = String::new();

    loop {
        let rem = (col_index % 26) as u8;
        col_label.push((b'A' + rem) as char);
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }

    col_label.chars().rev().collect::<String>() + &(row + 1).to_string()
}

/// Parse an A1 address into zero-based (row, col) indices.
/// Returns `None` for malformed addresses.
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    if a1.is_empty() {
        return None;
    }

    let mut col: u32 = 0;
    let mut row: u32 = 0;
    let mut saw_letter = false;
    let mut saw_digit = false;

    for ch in a1.chars() {
        if ch.is_ascii_alphabetic() {
            saw_letter = true;
            if saw_digit {
                // Letters after digits are not allowed.
                return None;
            }
            let upper = ch.to_ascii_uppercase() as u8;
            if !upper.is_ascii_uppercase() {
                return None;
            }
            col = col
                .checked_mul(26)?
                .checked_add((upper - b'A' + 1) as u32)?;
        } else if ch.is_ascii_digit() {
            saw_digit = true;
            row = row.checked_mul(10)?.checked_add((ch as u8 - b'0') as u32)?;
        } else {
            return None;
        }
    }

    if !saw_letter || !saw_digit || row == 0 || col == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_address_examples() {
        assert_eq!(index_to_address(0, 0), "A1");
        assert_eq!(index_to_address(0, 25), "Z1");
        assert_eq!(index_to_address(0, 26), "AA1");
        assert_eq!(index_to_address(0, 27), "AB1");
        assert_eq!(index_to_address(0, 51), "AZ1");
        assert_eq!(index_to_address(0, 52), "BA1");
    }

    #[test]
    fn round_trip_addresses() {
        let addresses = [
            "A1", "B2", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1",
        ];
        for addr in addresses {
            let (r, c) = address_to_index(addr).expect("address should parse");
            assert_eq!(index_to_address(r, c), addr);
        }
    }

    #[test]
    fn invalid_addresses_rejected() {
        let invalid = ["", "1A", "A0", "A", "AA0", "A-1", "A1A"];
        for addr in invalid {
            assert!(address_to_index(addr).is_none(), "{addr} should be invalid");
        }
    }
}
```

---

### File: `core\src\excel_open_xml.rs`

```rust
use crate::addressing::address_to_index;
use crate::workbook::{Cell, CellAddress, CellValue, Grid, Row, Sheet, SheetKind, Workbook};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use zip::ZipArchive;
use zip::result::ZipError;

#[derive(Debug, Error)]
pub enum ExcelOpenError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a ZIP container")]
    NotZipContainer,
    #[error("not an Excel Open XML package")]
    NotExcelOpenXml,
    #[error("workbook.xml missing or unreadable")]
    WorkbookXmlMissing,
    #[error("worksheet XML missing for sheet {sheet_name}")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("XML parse error: {0}")]
    XmlParseError(String),
    #[error("DataMashup base64 invalid")]
    DataMashupBase64Invalid,
    #[error("DataMashup unsupported version {version}")]
    DataMashupUnsupportedVersion { version: u32 },
    #[error("DataMashup framing invalid")]
    DataMashupFramingInvalid,
}

struct SheetDescriptor {
    name: String,
    rel_id: Option<String>,
    sheet_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDataMashup {
    pub version: u32,
    pub package_parts: Vec<u8>,
    pub permissions: Vec<u8>,
    pub metadata: Vec<u8>,
    pub permission_bindings: Vec<u8>,
}

pub fn open_workbook(path: impl AsRef<Path>) -> Result<Workbook, ExcelOpenError> {
    let mut archive = open_zip(path.as_ref())?;

    if archive.by_name("[Content_Types].xml").is_err() {
        return Err(ExcelOpenError::NotExcelOpenXml);
    }

    let shared_strings = match read_zip_file_optional(&mut archive, "xl/sharedStrings.xml") {
        Ok(Some(bytes)) => parse_shared_strings(&bytes)?,
        Ok(None) => Vec::new(),
        Err(e) => return Err(e),
    };

    let workbook_bytes = match read_zip_file(&mut archive, "xl/workbook.xml") {
        Ok(bytes) => bytes,
        Err(ZipError::FileNotFound) => return Err(ExcelOpenError::WorkbookXmlMissing),
        Err(ZipError::Io(e)) => return Err(ExcelOpenError::Io(e)),
        Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
    };

    let sheets = parse_workbook_xml(&workbook_bytes)?;

    let relationships = match read_zip_file_optional(&mut archive, "xl/_rels/workbook.xml.rels") {
        Ok(Some(bytes)) => parse_relationships(&bytes)?,
        Ok(None) => HashMap::new(),
        Err(e) => return Err(e),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes = match read_zip_file(&mut archive, &target) {
            Ok(bytes) => bytes,
            Err(ZipError::FileNotFound) => {
                return Err(ExcelOpenError::WorksheetXmlMissing {
                    sheet_name: sheet.name.clone(),
                });
            }
            Err(ZipError::Io(e)) => return Err(ExcelOpenError::Io(e)),
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
        };
        let grid = parse_sheet_xml(&sheet_bytes, &shared_strings)?;
        sheet_ir.push(Sheet {
            name: sheet.name.clone(),
            kind: SheetKind::Worksheet,
            grid,
        });
    }

    Ok(Workbook { sheets: sheet_ir })
}

pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, ExcelOpenError> {
    let mut archive = open_zip(path.as_ref())?;

    if archive.by_name("[Content_Types].xml").is_err() {
        return Err(ExcelOpenError::NotExcelOpenXml);
    }

    let dm_bytes = match extract_datamashup_bytes_from_excel(&mut archive)? {
        Some(bytes) => bytes,
        None => return Ok(None),
    };

    parse_data_mashup(&dm_bytes).map(Some)
}

pub(crate) fn parse_data_mashup(bytes: &[u8]) -> Result<RawDataMashup, ExcelOpenError> {
    const MIN_SIZE: usize = 4 + 4 * 4;
    if bytes.len() < MIN_SIZE {
        return Err(ExcelOpenError::DataMashupFramingInvalid);
    }

    let mut offset: usize = 0;
    let version = read_u32_at(bytes, offset).ok_or(ExcelOpenError::DataMashupFramingInvalid)?;
    offset += 4;

    if version != 0 {
        return Err(ExcelOpenError::DataMashupUnsupportedVersion { version });
    }

    let package_parts_len = read_length(bytes, offset)?;
    offset += 4;
    let package_parts = take_segment(bytes, &mut offset, package_parts_len)?;

    let permissions_len = read_length(bytes, offset)?;
    offset += 4;
    let permissions = take_segment(bytes, &mut offset, permissions_len)?;

    let metadata_len = read_length(bytes, offset)?;
    offset += 4;
    let metadata = take_segment(bytes, &mut offset, metadata_len)?;

    let permission_bindings_len = read_length(bytes, offset)?;
    offset += 4;
    let permission_bindings = take_segment(bytes, &mut offset, permission_bindings_len)?;

    if offset != bytes.len() {
        return Err(ExcelOpenError::DataMashupFramingInvalid);
    }

    Ok(RawDataMashup {
        version,
        package_parts,
        permissions,
        metadata,
        permission_bindings,
    })
}

fn extract_datamashup_bytes_from_excel(
    archive: &mut ZipArchive<File>,
) -> Result<Option<Vec<u8>>, ExcelOpenError> {
    let mut found: Option<Vec<u8>> = None;

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(ZipError::Io(e)) => return Err(ExcelOpenError::Io(e)),
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
        };
        let name = file.name().to_string();
        if !name.starts_with("customXml/") || !name.ends_with(".xml") {
            continue;
        }

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(ExcelOpenError::Io)?;

        if let Some(text) = read_datamashup_text(&buf)? {
            let decoded = decode_datamashup_base64(&text)?;
            if found.is_some() {
                return Err(ExcelOpenError::DataMashupFramingInvalid);
            }
            found = Some(decoded);
        }
    }

    Ok(found)
}

fn read_datamashup_text(xml: &[u8]) -> Result<Option<String>, ExcelOpenError> {
    let utf8_xml = decode_datamashup_xml(xml)?;

    let mut reader = Reader::from_reader(utf8_xml.as_deref().unwrap_or(xml));
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut found_content: Option<String> = None;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup || found_content.is_some() {
                    return Err(ExcelOpenError::DataMashupFramingInvalid);
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t
                    .unescape()
                    .map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?
                    .into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                let data = t.into_inner();
                content.push_str(&String::from_utf8_lossy(&data));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return Err(ExcelOpenError::DataMashupFramingInvalid);
                }
                in_datamashup = false;
                found_content = Some(content.clone());
            }
            Ok(Event::Eof) if in_datamashup => {
                return Err(ExcelOpenError::DataMashupFramingInvalid);
            }
            Ok(Event::Eof) => return Ok(found_content),
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
}

fn decode_datamashup_base64(text: &str) -> Result<Vec<u8>, ExcelOpenError> {
    let cleaned: String = text.split_whitespace().collect();
    STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| ExcelOpenError::DataMashupBase64Invalid)
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn decode_datamashup_xml(xml: &[u8]) -> Result<Option<Vec<u8>>, ExcelOpenError> {
    if xml.starts_with(&[0xFF, 0xFE]) {
        return Ok(Some(decode_utf16_xml(xml, true, true)?));
    }
    if xml.starts_with(&[0xFE, 0xFF]) {
        return Ok(Some(decode_utf16_xml(xml, false, true)?));
    }

    decode_declared_utf16_without_bom(xml)
}

fn decode_declared_utf16_without_bom(xml: &[u8]) -> Result<Option<Vec<u8>>, ExcelOpenError> {
    let attempt_decode = |little_endian| -> Result<Option<Vec<u8>>, ExcelOpenError> {
        if !looks_like_utf16(xml, little_endian) {
            return Ok(None);
        }
        let decoded = decode_utf16_xml(xml, little_endian, false)?;
        let lower = String::from_utf8_lossy(&decoded).to_ascii_lowercase();
        if lower.contains("encoding=\"utf-16\"") || lower.contains("encoding='utf-16'") {
            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    };

    if let Some(decoded) = attempt_decode(true)? {
        return Ok(Some(decoded));
    }
    attempt_decode(false)
}

fn looks_like_utf16(xml: &[u8], little_endian: bool) -> bool {
    if xml.len() < 4 {
        return false;
    }

    if little_endian {
        xml[0] == b'<' && xml[1] == 0 && xml[2] == b'?' && xml[3] == 0
    } else {
        xml[0] == 0 && xml[1] == b'<' && xml[2] == 0 && xml[3] == b'?'
    }
}

fn decode_utf16_xml(
    xml: &[u8],
    little_endian: bool,
    has_bom: bool,
) -> Result<Vec<u8>, ExcelOpenError> {
    let start = if has_bom { 2 } else { 0 };
    let body = xml
        .get(start..)
        .ok_or_else(|| ExcelOpenError::XmlParseError("invalid UTF-16 XML".into()))?;
    if body.len() % 2 != 0 {
        return Err(ExcelOpenError::XmlParseError(
            "invalid UTF-16 byte length".into(),
        ));
    }

    let mut code_units = Vec::with_capacity(body.len() / 2);
    for chunk in body.chunks_exact(2) {
        let unit = if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        code_units.push(unit);
    }

    let utf8 = String::from_utf16(&code_units)
        .map_err(|_| ExcelOpenError::XmlParseError("invalid UTF-16 XML".into()))?;
    Ok(utf8.into_bytes())
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    let array: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_le_bytes(array))
}

fn read_length(bytes: &[u8], offset: usize) -> Result<usize, ExcelOpenError> {
    let len = read_u32_at(bytes, offset).ok_or(ExcelOpenError::DataMashupFramingInvalid)?;
    usize::try_from(len).map_err(|_| ExcelOpenError::DataMashupFramingInvalid)
}

fn take_segment(bytes: &[u8], offset: &mut usize, len: usize) -> Result<Vec<u8>, ExcelOpenError> {
    let start = *offset;
    let end = start
        .checked_add(len)
        .ok_or(ExcelOpenError::DataMashupFramingInvalid)?;
    if end > bytes.len() {
        return Err(ExcelOpenError::DataMashupFramingInvalid);
    }

    let segment = bytes[start..end].to_vec();
    *offset = end;
    Ok(segment)
}

fn open_zip(path: &Path) -> Result<ZipArchive<File>, ExcelOpenError> {
    let file = File::open(path)?;
    ZipArchive::new(file).map_err(|err| match err {
        ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
            ExcelOpenError::NotZipContainer
        }
        ZipError::Io(e) => ExcelOpenError::Io(e),
        other => ExcelOpenError::XmlParseError(other.to_string()),
    })
}

fn read_zip_file(
    archive: &mut ZipArchive<File>,
    name: &str,
) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut file = archive.by_name(name)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn read_zip_file_optional(
    archive: &mut ZipArchive<File>,
    name: &str,
) -> Result<Option<Vec<u8>>, ExcelOpenError> {
    match read_zip_file(archive, name) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(ZipError::FileNotFound) => Ok(None),
        Err(ZipError::Io(e)) => Err(ExcelOpenError::Io(e)),
        Err(e) => Err(ExcelOpenError::XmlParseError(e.to_string())),
    }
}

fn parse_shared_strings(xml: &[u8]) -> Result<Vec<String>, ExcelOpenError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_si = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"si" => {
                current.clear();
                in_si = true;
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" && in_si => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?
                    .into_owned();
                current.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"si" => {
                strings.push(current.clone());
                in_si = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(strings)
}

fn parse_workbook_xml(xml: &[u8]) -> Result<Vec<SheetDescriptor>, ExcelOpenError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut sheets = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"sheet" => {
                let mut name = None;
                let mut rel_id = None;
                let mut sheet_id = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?;
                    match attr.key.as_ref() {
                        b"name" => {
                            name = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        b"sheetId" => {
                            let parsed = attr.unescape_value().map_err(to_xml_err)?;
                            sheet_id = parsed.into_owned().parse::<u32>().ok();
                        }
                        b"r:id" => {
                            rel_id = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        _ => {}
                    }
                }
                if let Some(name) = name {
                    sheets.push(SheetDescriptor {
                        name,
                        rel_id,
                        sheet_id,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(sheets)
}

fn parse_relationships(xml: &[u8]) -> Result<HashMap<String, String>, ExcelOpenError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut map = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"Relationship" => {
                let mut id = None;
                let mut target = None;
                let mut rel_type = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?;
                    match attr.key.as_ref() {
                        b"Id" => id = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned()),
                        b"Target" => {
                            target = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        b"Type" => {
                            rel_type = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        _ => {}
                    }
                }

                if let (Some(id), Some(target), Some(rel_type)) = (id, target, rel_type)
                    && rel_type.contains("worksheet")
                {
                    map.insert(id, target);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(map)
}

fn resolve_sheet_target(
    sheet: &SheetDescriptor,
    relationships: &HashMap<String, String>,
    index: usize,
) -> String {
    if let Some(rel_id) = &sheet.rel_id
        && let Some(target) = relationships.get(rel_id)
    {
        return normalize_target(target);
    }

    let guessed = sheet
        .sheet_id
        .map(|id| format!("xl/worksheets/sheet{id}.xml"))
        .unwrap_or_else(|| format!("xl/worksheets/sheet{}.xml", index + 1));
    normalize_target(&guessed)
}

fn normalize_target(target: &str) -> String {
    let trimmed = target.trim_start_matches('/');
    if trimmed.starts_with("xl/") {
        trimmed.to_string()
    } else {
        format!("xl/{trimmed}")
    }
}

fn parse_sheet_xml(xml: &[u8], shared_strings: &[String]) -> Result<Grid, ExcelOpenError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"dimension" => {
                if let Some(r) = get_attr_value(&e, b"ref")? {
                    dimension_hint = dimension_from_ref(&r);
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"c" => {
                let cell = parse_cell(&mut reader, e, shared_strings)?;
                max_row = Some(max_row.map_or(cell.row, |r| r.max(cell.row)));
                max_col = Some(max_col.map_or(cell.col, |c| c.max(cell.col)));
                parsed_cells.push(cell);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    if parsed_cells.is_empty() {
        return Ok(Grid {
            nrows: 0,
            ncols: 0,
            rows: Vec::new(),
        });
    }

    let mut nrows = dimension_hint.map(|(r, _)| r).unwrap_or(0);
    let mut ncols = dimension_hint.map(|(_, c)| c).unwrap_or(0);

    if let Some(max_r) = max_row {
        nrows = nrows.max(max_r + 1);
    }
    if let Some(max_c) = max_col {
        ncols = ncols.max(max_c + 1);
    }

    build_grid(nrows, ncols, parsed_cells)
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
    start: BytesStart,
    shared_strings: &[String],
) -> Result<ParsedCell, ExcelOpenError> {
    let address_raw = get_attr_value(&start, b"r")?
        .ok_or_else(|| ExcelOpenError::XmlParseError("cell missing address".into()))?;
    let (row, col) = address_to_index(&address_raw).ok_or_else(|| {
        ExcelOpenError::XmlParseError(format!("invalid cell address {address_raw}"))
    })?;

    let cell_type = get_attr_value(&start, b"t")?;

    let mut value_text: Option<String> = None;
    let mut formula_text: Option<String> = None;
    let mut inline_text: Option<String> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?
                    .into_owned();
                value_text = Some(text);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?
                    .into_owned();
                formula_text = Some(text);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                inline_text = Some(read_inline_string(reader)?);
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(ExcelOpenError::XmlParseError(
                    "unexpected EOF inside cell".into(),
                ));
            }
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let value = match inline_text {
        Some(text) => Some(CellValue::Text(text)),
        None => convert_value(value_text.as_deref(), cell_type.as_deref(), shared_strings)?,
    };

    Ok(ParsedCell {
        row,
        col,
        value,
        formula: formula_text,
    })
}

fn read_inline_string(reader: &mut Reader<&[u8]>) -> Result<String, ExcelOpenError> {
    let mut buf = Vec::new();
    let mut value = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?
                    .into_owned();
                value.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"is" => break,
            Ok(Event::Eof) => {
                return Err(ExcelOpenError::XmlParseError(
                    "unexpected EOF inside inline string".into(),
                ));
            }
            Err(e) => return Err(ExcelOpenError::XmlParseError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(value)
}

fn convert_value(
    value_text: Option<&str>,
    cell_type: Option<&str>,
    shared_strings: &[String],
) -> Result<Option<CellValue>, ExcelOpenError> {
    let raw = match value_text {
        Some(t) => t,
        None => return Ok(None),
    };

    let trimmed = raw.trim();
    if raw.is_empty() || trimmed.is_empty() {
        return Ok(Some(CellValue::Text(String::new())));
    }

    match cell_type {
        Some("s") => {
            let idx = trimmed
                .parse::<usize>()
                .map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?;
            let text = shared_strings.get(idx).ok_or_else(|| {
                ExcelOpenError::XmlParseError(format!("shared string index {idx} out of bounds"))
            })?;
            Ok(Some(CellValue::Text(text.clone())))
        }
        Some("b") => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some("str") | Some("inlineStr") => Ok(Some(CellValue::Text(raw.to_string()))),
        _ => {
            if let Ok(n) = trimmed.parse::<f64>() {
                Ok(Some(CellValue::Number(n)))
            } else {
                Ok(Some(CellValue::Text(trimmed.to_string())))
            }
        }
    }
}

fn dimension_from_ref(reference: &str) -> Option<(u32, u32)> {
    let mut parts = reference.split(':');
    let start = parts.next()?;
    let end = parts.next().unwrap_or(start);
    let (start_row, start_col) = address_to_index(start)?;
    let (end_row, end_col) = address_to_index(end)?;
    let height = end_row.checked_sub(start_row)?.checked_add(1)?;
    let width = end_col.checked_sub(start_col)?.checked_add(1)?;
    Some((height, width))
}

fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, ExcelOpenError> {
    if nrows == 0 || ncols == 0 {
        return Ok(Grid {
            nrows: 0,
            ncols: 0,
            rows: Vec::new(),
        });
    }

    let mut rows = Vec::with_capacity(nrows as usize);
    for r in 0..nrows {
        let mut row_cells = Vec::with_capacity(ncols as usize);
        for c in 0..ncols {
            row_cells.push(Cell {
                row: r,
                col: c,
                address: CellAddress::from_indices(r, c),
                value: None,
                formula: None,
            });
        }
        rows.push(Row {
            index: r,
            cells: row_cells,
        });
    }

    for parsed in cells {
        if let Some(row) = rows.get_mut(parsed.row as usize)
            && let Some(cell) = row.cells.get_mut(parsed.col as usize)
        {
            cell.value = parsed.value;
            cell.formula = parsed.formula;
        }
    }

    Ok(Grid { nrows, ncols, rows })
}

fn get_attr_value(element: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, ExcelOpenError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))?;
        if attr.key.as_ref() == key {
            return Ok(Some(
                attr.unescape_value().map_err(to_xml_err)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

fn to_xml_err(err: quick_xml::Error) -> ExcelOpenError {
    ExcelOpenError::XmlParseError(err.to_string())
}

struct ParsedCell {
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        ExcelOpenError, RawDataMashup, convert_value, parse_data_mashup, parse_shared_strings,
        read_datamashup_text, read_inline_string,
    };
    use crate::workbook::CellValue;
    use quick_xml::Reader;

    fn build_dm_bytes(
        version: u32,
        package_parts: &[u8],
        permissions: &[u8],
        metadata: &[u8],
        permission_bindings: &[u8],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&(package_parts.len() as u32).to_le_bytes());
        bytes.extend_from_slice(package_parts);
        bytes.extend_from_slice(&(permissions.len() as u32).to_le_bytes());
        bytes.extend_from_slice(permissions);
        bytes.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
        bytes.extend_from_slice(metadata);
        bytes.extend_from_slice(&(permission_bindings.len() as u32).to_le_bytes());
        bytes.extend_from_slice(permission_bindings);
        bytes
    }

    #[test]
    fn parse_zero_length_stream_succeeds() {
        let bytes = build_dm_bytes(0, b"", b"", b"", b"");
        let parsed = parse_data_mashup(&bytes).expect("zero-length sections should parse");
        assert_eq!(
            parsed,
            RawDataMashup {
                version: 0,
                package_parts: Vec::new(),
                permissions: Vec::new(),
                metadata: Vec::new(),
                permission_bindings: Vec::new(),
            }
        );
    }

    #[test]
    fn parse_basic_non_zero_lengths() {
        let bytes = build_dm_bytes(0, b"AAAA", b"BBBB", b"CCCC", b"DDDD");
        let parsed = parse_data_mashup(&bytes).expect("non-zero lengths should parse");
        assert_eq!(parsed.version, 0);
        assert_eq!(parsed.package_parts, b"AAAA");
        assert_eq!(parsed.permissions, b"BBBB");
        assert_eq!(parsed.metadata, b"CCCC");
        assert_eq!(parsed.permission_bindings, b"DDDD");
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let bytes = build_dm_bytes(1, b"AAAA", b"BBBB", b"CCCC", b"DDDD");
        let err = parse_data_mashup(&bytes).expect_err("version 1 should be unsupported");
        assert!(matches!(
            err,
            ExcelOpenError::DataMashupUnsupportedVersion { version: 1 }
        ));
    }

    #[test]
    fn truncated_stream_errors() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&100u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        let err = parse_data_mashup(&bytes).expect_err("length overflows buffer");
        assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
    }

    #[test]
    fn trailing_bytes_are_invalid() {
        let mut bytes = build_dm_bytes(0, b"", b"", b"", b"");
        bytes.push(0xFF);
        let err = parse_data_mashup(&bytes).expect_err("trailing bytes should fail");
        assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
    }

    #[test]
    fn too_short_stream_is_framing_invalid() {
        let bytes = vec![0u8; 8];
        let err =
            parse_data_mashup(&bytes).expect_err("buffer shorter than header must be invalid");
        assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
    }

    #[test]
    fn parse_shared_strings_rich_text_flattens_runs() {
        let xml = br#"<?xml version="1.0"?>
<sst>
  <si>
    <r><t>Hello</t></r>
    <r><t xml:space="preserve"> World</t></r>
  </si>
</sst>"#;
        let strings = parse_shared_strings(xml).expect("shared strings should parse");
        assert_eq!(strings.first(), Some(&"Hello World".to_string()));
    }

    #[test]
    fn read_inline_string_preserves_xml_space_preserve() {
        let xml = br#"<is><t xml:space="preserve"> hello</t></is>"#;
        let mut reader = Reader::from_reader(xml.as_ref());
        reader.config_mut().trim_text(false);
        let value = read_inline_string(&mut reader).expect("inline string should parse");
        assert_eq!(value, " hello");

        let converted = convert_value(Some(value.as_str()), Some("inlineStr"), &[])
            .expect("inlineStr conversion should succeed");
        assert_eq!(converted, Some(CellValue::Text(" hello".into())));
    }

    #[test]
    fn convert_value_bool_0_1_and_other() {
        let false_val =
            convert_value(Some("0"), Some("b"), &[]).expect("bool cell conversion should succeed");
        assert_eq!(false_val, Some(CellValue::Bool(false)));

        let true_val =
            convert_value(Some("1"), Some("b"), &[]).expect("bool cell conversion should succeed");
        assert_eq!(true_val, Some(CellValue::Bool(true)));

        let none_val = convert_value(Some("2"), Some("b"), &[])
            .expect("unexpected bool tokens should still parse");
        assert!(none_val.is_none());
    }

    #[test]
    fn convert_value_shared_string_index_out_of_bounds_errors() {
        let err = convert_value(Some("5"), Some("s"), &["only".into()])
            .expect_err("invalid shared string index should error");
        assert!(matches!(
            err,
            ExcelOpenError::XmlParseError(msg) if msg.contains("shared string index 5")
        ));
    }

    #[test]
    fn convert_value_error_cell_as_text() {
        let value =
            convert_value(Some("#DIV/0!"), Some("e"), &[]).expect("error cell should convert");
        assert_eq!(value, Some(CellValue::Text("#DIV/0!".into())));
    }

    #[test]
    fn utf16_datamashup_xml_decodes_correctly() {
        let xml_text = r#"<?xml version="1.0" encoding="utf-16"?><root xmlns:dm="http://schemas.microsoft.com/DataMashup"><dm:DataMashup>QQ==</dm:DataMashup></root>"#;
        let mut xml_bytes = Vec::with_capacity(2 + xml_text.len() * 2);
        xml_bytes.extend_from_slice(&[0xFF, 0xFE]);
        for unit in xml_text.encode_utf16() {
            xml_bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let text = read_datamashup_text(&xml_bytes)
            .expect("UTF-16 XML should parse")
            .expect("DataMashup element should be found");
        assert_eq!(text.trim(), "QQ==");
    }

    #[test]
    fn utf16_without_bom_with_declared_encoding_parses() {
        let xml_text = r#"<?xml version="1.0" encoding="utf-16"?><root xmlns:dm="http://schemas.microsoft.com/DataMashup"><dm:DataMashup>QQ==</dm:DataMashup></root>"#;
        for &little_endian in &[true, false] {
            let mut xml_bytes = Vec::with_capacity(xml_text.len() * 2);
            for unit in xml_text.encode_utf16() {
                let bytes = if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                xml_bytes.extend_from_slice(&bytes);
            }

            let text = read_datamashup_text(&xml_bytes)
                .expect("UTF-16 XML without BOM should parse when declared")
                .expect("DataMashup element should be found");
            assert_eq!(text.trim(), "QQ==");
        }
    }

    #[test]
    fn elements_with_datamashup_suffix_are_ignored() {
        let xml = br#"<?xml version="1.0"?><root><FooDataMashup>QQ==</FooDataMashup></root>"#;
        let result = read_datamashup_text(xml).expect("parsing should succeed");
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_sibling_datamashup_elements_error() {
        let xml = br#"<?xml version="1.0"?>
<root xmlns:dm="http://schemas.microsoft.com/DataMashup">
  <dm:DataMashup>QQ==</dm:DataMashup>
  <dm:DataMashup>QQ==</dm:DataMashup>
</root>"#;
        let err = read_datamashup_text(xml).expect_err("duplicate DataMashup elements should fail");
        assert!(matches!(err, ExcelOpenError::DataMashupFramingInvalid));
    }

    #[test]
    fn fuzz_style_never_panics() {
        for seed in 0u64..32 {
            let len = (seed as usize * 7 % 48) + (seed as usize % 5);
            let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut bytes = Vec::with_capacity(len);
            for _ in 0..len {
                state = state
                    .wrapping_mul(2862933555777941757)
                    .wrapping_add(3037000493);
                bytes.push((state >> 32) as u8);
            }
            let _ = parse_data_mashup(&bytes);
        }
    }
}
```

---

### File: `core\src\lib.rs`

```rust
pub mod addressing;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod output;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, RawDataMashup, open_data_mashup, open_workbook};
pub use output::json::{CellDiff, serialize_cell_diffs};
#[cfg(feature = "excel-open-xml")]
pub use output::json::{diff_workbooks, diff_workbooks_to_json};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, Grid, Row, Sheet, SheetKind, Workbook,
};
```

---

### File: `core\src\main.rs`

```rust
fn main() {
    println!("Hello, world!");
}
```

---

### File: `core\src\workbook.rs`

```rust
use crate::addressing::{address_to_index, index_to_address};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

/// A snapshot of a cell's logical content (address, value, formula).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CellSnapshot {
    pub addr: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

impl CellSnapshot {
    pub fn from_cell(cell: &Cell) -> CellSnapshot {
        CellSnapshot {
            addr: cell.address,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub kind: SheetKind,
    pub grid: Grid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetKind {
    Worksheet,
    Chart,
    Macro,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub nrows: u32,
    pub ncols: u32,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub index: u32,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    pub row: u32,
    pub col: u32,
}

impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> CellAddress {
        CellAddress { row, col }
    }

    pub fn to_a1(&self) -> String {
        index_to_address(self.row, self.col)
    }
}

impl std::str::FromStr for CellAddress {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (row, col) = address_to_index(s).ok_or(())?;
        Ok(CellAddress { row, col })
    }
}

impl std::fmt::Display for CellAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

impl Serialize for CellAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_a1())
    }
}

impl<'de> Deserialize<'de> for CellAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let a1 = String::deserialize(deserializer)?;
        CellAddress::from_str(&a1)
            .map_err(|_| DeError::custom(format!("invalid cell address: {a1}")))
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Number(f64),
    Text(String),
    Bool(bool),
}

impl PartialEq for CellSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.formula == other.formula
    }
}

impl Eq for CellSnapshot {}

impl CellValue {
    pub fn as_text(&self) -> Option<&str> {
        if let CellValue::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if let CellValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let CellValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(a1: &str) -> CellAddress {
        a1.parse().expect("address should parse")
    }

    fn make_cell(address: &str, value: Option<CellValue>, formula: Option<&str>) -> Cell {
        let (row, col) = address_to_index(address).expect("address should parse");
        Cell {
            row,
            col,
            address: CellAddress::from_indices(row, col),
            value,
            formula: formula.map(|s| s.to_string()),
        }
    }

    #[test]
    fn snapshot_from_number_cell() {
        let cell = make_cell("A1", Some(CellValue::Number(42.0)), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "A1");
        assert_eq!(snap.value, Some(CellValue::Number(42.0)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_text_cell() {
        let cell = make_cell("B2", Some(CellValue::Text("hello".into())), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "B2");
        assert_eq!(snap.value, Some(CellValue::Text("hello".into())));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_bool_cell() {
        let cell = make_cell("C3", Some(CellValue::Bool(true)), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "C3");
        assert_eq!(snap.value, Some(CellValue::Bool(true)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_empty_cell() {
        let cell = make_cell("D4", None, None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "D4");
        assert!(snap.value.is_none());
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_equality_same_value_and_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(1.0)),
            formula: Some("A1+1".into()),
        };
        let snap2 = CellSnapshot {
            addr: addr("B2"),
            value: Some(CellValue::Number(1.0)),
            formula: Some("A1+1".into()),
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_different_value_same_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(43.0)),
            formula: Some("A1+1".into()),
        };
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(44.0)),
            formula: Some("A1+1".into()),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_value_vs_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: None,
        };
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: Some("A1+1".into()),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_equality_ignores_address() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Text("hello".into())),
            formula: None,
        };
        let snap2 = CellSnapshot {
            addr: addr("Z9"),
            value: Some(CellValue::Text("hello".into())),
            formula: None,
        };
        assert_eq!(snap1, snap2);
    }
}
```

---

### File: `core\src\output\json.rs`

```rust
#[cfg(feature = "excel-open-xml")]
use crate::addressing::index_to_address;
#[cfg(feature = "excel-open-xml")]
use crate::workbook::{Cell, CellValue, Sheet, Workbook};
use serde::Serialize;
#[cfg(feature = "excel-open-xml")]
use std::collections::HashMap;
#[cfg(feature = "excel-open-xml")]
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellDiff {
    #[serde(rename = "coords")]
    pub coords: String,
    #[serde(rename = "value_file1")]
    pub value_file1: Option<String>,
    #[serde(rename = "value_file2")]
    pub value_file2: Option<String>,
}

pub fn serialize_cell_diffs(diffs: &[CellDiff]) -> serde_json::Result<String> {
    serde_json::to_string(diffs)
}

#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{ExcelOpenError, open_workbook};

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<Vec<CellDiff>, ExcelOpenError> {
    let wb_a = open_workbook(path_a)?;
    let wb_b = open_workbook(path_b)?;
    Ok(compute_cell_diffs(&wb_a, &wb_b))
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<String, ExcelOpenError> {
    let diffs = diff_workbooks(path_a, path_b)?;
    serialize_cell_diffs(&diffs).map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))
}

#[cfg(feature = "excel-open-xml")]
fn compute_cell_diffs(a: &Workbook, b: &Workbook) -> Vec<CellDiff> {
    let map_a = sheet_map(a);
    let map_b = sheet_map(b);

    let mut names: Vec<&str> = map_a.keys().chain(map_b.keys()).copied().collect();
    names.sort_unstable();
    names.dedup();

    let mut diffs = Vec::new();
    for name in names {
        let sheet_a = map_a.get(name);
        let sheet_b = map_b.get(name);
        let dims = sheet_dims(sheet_a);
        let other_dims = sheet_dims(sheet_b);
        let nrows = dims.0.max(other_dims.0);
        let ncols = dims.1.max(other_dims.1);

        for r in 0..nrows {
            for c in 0..ncols {
                let cell_a = sheet_a
                    .and_then(|s| s.grid.rows.get(r as usize))
                    .and_then(|row| row.cells.get(c as usize));
                let cell_b = sheet_b
                    .and_then(|s| s.grid.rows.get(r as usize))
                    .and_then(|row| row.cells.get(c as usize));

                let value_a = cell_a.and_then(render_cell_value);
                let value_b = cell_b.and_then(render_cell_value);

                if value_a != value_b {
                    let coords = index_to_address(r, c);
                    diffs.push(CellDiff {
                        coords,
                        value_file1: value_a,
                        value_file2: value_b,
                    });
                }
            }
        }
    }

    diffs
}

#[cfg(feature = "excel-open-xml")]
fn sheet_map(workbook: &Workbook) -> HashMap<&str, &Sheet> {
    workbook
        .sheets
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect()
}

#[cfg(feature = "excel-open-xml")]
fn sheet_dims(sheet: Option<&&Sheet>) -> (u32, u32) {
    sheet
        .map(|s| (s.grid.nrows, s.grid.ncols))
        .unwrap_or((0, 0))
}

#[cfg(feature = "excel-open-xml")]
fn render_cell_value(cell: &Cell) -> Option<String> {
    match &cell.value {
        Some(CellValue::Number(n)) => Some(n.to_string()),
        Some(CellValue::Text(s)) => Some(s.clone()),
        Some(CellValue::Bool(b)) => Some(b.to_string()),
        None => None,
    }
}
```

---

### File: `core\src\output\mod.rs`

```rust
pub mod json;
```

---

### File: `core\tests\addressing_pg2_tests.rs`

```rust
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
```

---

### File: `core\tests\data_mashup_tests.rs`

```rust
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
```

---

### File: `core\tests\excel_open_xml_tests.rs`

```rust
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
```

---

### File: `core\tests\integration_test.rs`

```rust
use std::path::PathBuf;

fn get_fixture_path(filename: &str) -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up one level from 'core', then into 'fixtures/generated'
    d.push("../fixtures/generated");
    d.push(filename);
    d
}

#[test]
fn test_locate_fixture() {
    let path = get_fixture_path("minimal.xlsx");
    // This test confirms that the Rust code can locate the Python-generated fixtures
    // using the relative path strategy from the monorepo root.
    assert!(
        path.exists(),
        "Fixture minimal.xlsx should exist at {:?}",
        path
    );
}
```

---

### File: `core\tests\output_tests.rs`

```rust
use excel_diff::output::json::{CellDiff, diff_workbooks_to_json, serialize_cell_diffs};
use serde_json::Value;

mod common;
use common::fixture_path;

#[test]
fn test_json_format() {
    let diffs = vec![
        CellDiff {
            coords: "A1".into(),
            value_file1: Some("100".into()),
            value_file2: Some("200".into()),
        },
        CellDiff {
            coords: "B2".into(),
            value_file1: Some("true".into()),
            value_file2: Some("false".into()),
        },
        CellDiff {
            coords: "C3".into(),
            value_file1: Some("#DIV/0!".into()),
            value_file2: None,
        },
    ];

    let json = serialize_cell_diffs(&diffs).expect("serialization should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    assert!(value.is_array(), "expected an array of cell diffs");
    let arr = value
        .as_array()
        .expect("top-level json should be an array of cell diffs");
    assert_eq!(arr.len(), 3);

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("A1".into()));
    assert_eq!(first["value_file1"], Value::String("100".into()));
    assert_eq!(first["value_file2"], Value::String("200".into()));

    let second = &arr[1];
    assert_eq!(second["coords"], Value::String("B2".into()));
    assert_eq!(second["value_file1"], Value::String("true".into()));
    assert_eq!(second["value_file2"], Value::String("false".into()));

    let third = &arr[2];
    assert_eq!(third["coords"], Value::String("C3".into()));
    assert_eq!(third["value_file1"], Value::String("#DIV/0!".into()));
    assert_eq!(third["value_file2"], Value::Null);
}

#[test]
fn test_json_empty_diff() {
    let fixture = fixture_path("pg1_basic_two_sheets.xlsx");
    let json =
        diff_workbooks_to_json(&fixture, &fixture).expect("diffing identical files should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    let arr = value
        .as_array()
        .expect("top-level json should be an array of cell diffs");
    assert!(
        arr.is_empty(),
        "identical files should produce no cell diffs"
    );
}

#[test]
fn test_json_non_empty_diff() {
    let a = fixture_path("json_diff_single_cell_a.xlsx");
    let b = fixture_path("json_diff_single_cell_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    let arr = value
        .as_array()
        .expect("top-level should be an array of cell diffs");
    assert_eq!(arr.len(), 1, "expected a single cell difference");

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("C3".into()));
    assert_eq!(first["value_file1"], Value::String("1".into()));
    assert_eq!(first["value_file2"], Value::String("2".into()));
}

#[test]
fn test_json_non_empty_diff_bool() {
    let a = fixture_path("json_diff_bool_a.xlsx");
    let b = fixture_path("json_diff_bool_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    let arr = value
        .as_array()
        .expect("top-level should be an array of cell diffs");
    assert_eq!(arr.len(), 1, "expected a single cell difference");

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("C3".into()));
    assert_eq!(first["value_file1"], Value::String("true".into()));
    assert_eq!(first["value_file2"], Value::String("false".into()));
}
```

---

### File: `core\tests\pg1_ir_tests.rs`

```rust
use excel_diff::{CellValue, Sheet, SheetKind, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg1_basic_two_sheets_structure() {
    let workbook = open_workbook(fixture_path("pg1_basic_two_sheets.xlsx"))
        .expect("pg1 basic fixture should open");
    assert_eq!(workbook.sheets.len(), 2);
    assert_eq!(workbook.sheets[0].name, "Sheet1");
    assert_eq!(workbook.sheets[1].name, "Sheet2");
    assert!(matches!(workbook.sheets[0].kind, SheetKind::Worksheet));
    assert!(matches!(workbook.sheets[1].kind, SheetKind::Worksheet));

    let sheet1 = &workbook.sheets[0];
    assert_eq!(sheet1.grid.nrows, 3);
    assert_eq!(sheet1.grid.ncols, 3);
    assert_eq!(
        sheet1.grid.rows[0].cells[0]
            .value
            .as_ref()
            .and_then(CellValue::as_text),
        Some("R1C1")
    );

    let sheet2 = &workbook.sheets[1];
    assert_eq!(sheet2.grid.nrows, 5);
    assert_eq!(sheet2.grid.ncols, 2);
    assert_eq!(
        sheet2.grid.rows[0].cells[0]
            .value
            .as_ref()
            .and_then(CellValue::as_text),
        Some("S2_R1C1")
    );
}

#[test]
fn pg1_sparse_used_range_extents() {
    let workbook =
        open_workbook(fixture_path("pg1_sparse_used_range.xlsx")).expect("sparse fixture opens");
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == "Sparse")
        .expect("Sparse sheet present");

    assert_eq!(sheet.grid.nrows, 10);
    assert_eq!(sheet.grid.ncols, 7);

    assert_cell_text(sheet, 0, 0, "A1");
    assert_cell_text(sheet, 1, 1, "B2");
    assert_cell_text(sheet, 9, 6, "G10");

    for row in &sheet.grid.rows {
        assert_eq!(row.cells.len() as u32, sheet.grid.ncols);
    }
}

#[test]
fn pg1_empty_and_mixed_sheets() {
    let workbook = open_workbook(fixture_path("pg1_empty_and_mixed_sheets.xlsx"))
        .expect("mixed sheets fixture opens");

    let empty = sheet_by_name(&workbook, "Empty");
    assert_eq!(empty.grid.nrows, 0);
    assert_eq!(empty.grid.ncols, 0);
    assert!(empty.grid.rows.is_empty());

    let values_only = sheet_by_name(&workbook, "ValuesOnly");
    assert_eq!(values_only.grid.nrows, 10);
    assert_eq!(values_only.grid.ncols, 10);
    assert!(
        values_only
            .grid
            .rows
            .iter()
            .flat_map(|r| &r.cells)
            .all(|c| c.value.is_some() && c.formula.is_none()),
        "ValuesOnly cells should have values and no formulas"
    );
    assert_eq!(
        values_only.grid.rows[0].cells[0]
            .value
            .as_ref()
            .and_then(CellValue::as_number),
        Some(1.0)
    );

    let formulas = sheet_by_name(&workbook, "FormulasOnly");
    assert_eq!(formulas.grid.nrows, 10);
    assert_eq!(formulas.grid.ncols, 10);
    let first = &formulas.grid.rows[0].cells[0];
    assert_eq!(first.formula.as_deref(), Some("ValuesOnly!A1"));
    assert!(
        first.value.is_some(),
        "Formulas should surface cached values when present"
    );
    assert!(
        formulas
            .grid
            .rows
            .iter()
            .flat_map(|r| &r.cells)
            .all(|c| c.formula.is_some()),
        "All cells should carry formulas in FormulasOnly"
    );
}

fn sheet_by_name<'a>(workbook: &'a excel_diff::Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("sheet {name} not found"))
}

fn assert_cell_text(sheet: &Sheet, row: u32, col: u32, expected: &str) {
    let cell = &sheet.grid.rows[row as usize].cells[col as usize];
    assert_eq!(cell.address.to_a1(), expected);
    assert_eq!(
        cell.value
            .as_ref()
            .and_then(CellValue::as_text)
            .unwrap_or(""),
        expected
    );
}
```

---

### File: `core\tests\pg3_snapshot_tests.rs`

```rust
use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, Sheet, Workbook, address_to_index, open_workbook,
};

mod common;
use common::fixture_path;

fn sheet_by_name<'a>(workbook: &'a Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == name)
        .expect("sheet should exist")
}

fn find_cell<'a>(sheet: &'a Sheet, addr: &str) -> Option<&'a Cell> {
    let (row, col) = address_to_index(addr).expect("address should parse");
    sheet
        .grid
        .rows
        .get(row as usize)
        .and_then(|r| r.cells.get(col as usize))
}

fn snapshot(sheet: &Sheet, addr: &str) -> CellSnapshot {
    if let Some(cell) = find_cell(sheet, addr) {
        CellSnapshot::from_cell(cell)
    } else {
        let (row, col) = address_to_index(addr).expect("address should parse");
        CellSnapshot {
            addr: CellAddress::from_indices(row, col),
            value: None,
            formula: None,
        }
    }
}

#[test]
fn pg3_value_and_formula_cells_snapshot_from_excel() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let a1 = snapshot(sheet, "A1");
    assert_eq!(a1.addr.to_string(), "A1");
    assert_eq!(a1.value, Some(CellValue::Number(42.0)));
    assert!(a1.formula.is_none());

    let a2 = snapshot(sheet, "A2");
    assert_eq!(a2.value, Some(CellValue::Text("hello".into())));
    assert!(a2.formula.is_none());

    let a3 = snapshot(sheet, "A3");
    assert_eq!(a3.value, Some(CellValue::Bool(true)));
    assert!(a3.formula.is_none());

    let a4 = snapshot(sheet, "A4");
    assert!(a4.value.is_none());
    assert!(a4.formula.is_none());

    let b1 = snapshot(sheet, "B1");
    assert!(matches!(
        b1.value,
        Some(CellValue::Number(n)) if (n - 43.0).abs() < 1e-6
    ));
    assert_eq!(b1.addr.to_string(), "B1");
    let b1_formula = b1.formula.as_deref().expect("B1 should have a formula");
    assert!(b1_formula.contains("A1+1"));

    let b2 = snapshot(sheet, "B2");
    assert_eq!(b2.value, Some(CellValue::Text("hello world".into())));
    assert_eq!(b2.addr.to_string(), "B2");
    let b2_formula = b2.formula.as_deref().expect("B2 should have a formula");
    assert!(b2_formula.contains("hello"));
    assert!(b2_formula.contains("world"));

    let b3 = snapshot(sheet, "B3");
    assert_eq!(b3.value, Some(CellValue::Bool(true)));
    assert_eq!(b3.addr.to_string(), "B3");
    let b3_formula = b3.formula.as_deref().expect("B3 should have a formula");
    assert!(
        b3_formula.contains(">0"),
        "B3 formula should include comparison: {b3_formula:?}"
    );
}

#[test]
fn snapshot_json_roundtrip() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let snapshots = vec![
        snapshot(sheet, "A1"),
        snapshot(sheet, "A2"),
        snapshot(sheet, "B1"),
        snapshot(sheet, "B2"),
        snapshot(sheet, "B3"),
    ];

    for snap in snapshots {
        let addr = snap.addr.to_string();
        let json = serde_json::to_string(&snap).expect("snapshot should serialize");
        let as_value: serde_json::Value =
            serde_json::from_str(&json).expect("snapshot JSON should parse to value");
        assert_eq!(as_value["addr"], serde_json::Value::String(addr));
        let snap_back: CellSnapshot = serde_json::from_str(&json).expect("snapshot should parse");
        assert_eq!(snap.addr, snap_back.addr);
        assert_eq!(snap, snap_back);
    }
}

#[test]
fn snapshot_json_roundtrip_detects_tampered_addr() {
    let snap = CellSnapshot {
        addr: "Z9".parse().expect("address should parse"),
        value: Some(CellValue::Number(1.0)),
        formula: Some("A1+1".into()),
    };

    let mut value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&snap).expect("serialize should work"))
            .expect("serialized JSON should parse");
    value["addr"] = serde_json::Value::String("A1".into());

    let tampered_json = serde_json::to_string(&value).expect("tampered JSON should serialize");
    let tampered: CellSnapshot =
        serde_json::from_str(&tampered_json).expect("tampered JSON should parse");

    assert_ne!(snap.addr, tampered.addr);
    assert_eq!(snap, tampered, "value/formula equality ignores addr");
}
```

---

### File: `core\tests\common\mod.rs`

```rust
use std::path::PathBuf;

pub fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path.push(filename);
    path
}
```

---

### File: `fixtures\manifest.yaml`

```yaml
scenarios:
  # --- Phase 1.1: Basic File Opening ---
  - id: "smoke_minimal"
    generator: "basic_grid"
    args: { rows: 1, cols: 1 }
    output: "minimal.xlsx"

  # --- Phase 1.2: Is this a ZIP? ---
  - id: "container_random_zip"
    generator: "corrupt_container"
    args: { mode: "random_zip" }
    output: "random_zip.zip"
    
  - id: "container_no_content_types"
    generator: "corrupt_container"
    args: { mode: "no_content_types" }
    output: "no_content_types.xlsx"

  - id: "container_not_zip_text"
    generator: "corrupt_container"
    args: { mode: "not_zip_text" }
    output: "not_a_zip.txt"

  # --- PG1: Workbook -> Sheet -> Grid IR sanity ---
  - id: "pg1_basic_two_sheets"
    generator: "basic_grid"
    args: { rows: 3, cols: 3, two_sheets: true } # Sheet1 3x3, Sheet2 5x2 (logic in generator)
    output: "pg1_basic_two_sheets.xlsx"

  - id: "pg1_sparse"
    generator: "sparse_grid"
    output: "pg1_sparse_used_range.xlsx"

  - id: "pg1_mixed"
    generator: "edge_case"
    output: "pg1_empty_and_mixed_sheets.xlsx"

  # --- PG2: Addressing and index invariants ---
  - id: "pg2_addressing"
    generator: "address_sanity"
    args:
      targets: ["A1", "B2", "C3", "Z1", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1"]
    output: "pg2_addressing_matrix.xlsx"

  # --- PG3: Cell snapshots and comparison semantics ---
  - id: "pg3_types"
    generator: "value_formula"
    output: "pg3_value_and_formula_cells.xlsx"

  # --- JSON diff: simple non-empty change ---
  - id: "json_diff_single_cell"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: "1"
      value_b: "2"
    output:
      - "json_diff_single_cell_a.xlsx"
      - "json_diff_single_cell_b.xlsx"

  - id: "json_diff_single_bool"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: true
      value_b: false
    output:
      - "json_diff_bool_a.xlsx"
      - "json_diff_bool_b.xlsx"

  # --- Milestone 2.2: Base64 Correctness ---
  - id: "corrupt_base64"
    generator: "mashup_corrupt"
    args: 
      base_file: "templates/base_query.xlsx"
      mode: "byte_flip"
    output: "corrupt_base64.xlsx"

  - id: "duplicate_datamashup_parts"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
    output: "duplicate_datamashup_parts.xlsx"

  - id: "duplicate_datamashup_elements"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
      mode: "element"
    output: "duplicate_datamashup_elements.xlsx"

  - id: "mashup_utf16_le"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-le"
    output: "mashup_utf16_le.xlsx"

  - id: "mashup_utf16_be"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-be"
    output: "mashup_utf16_be.xlsx"

  - id: "mashup_base64_whitespace"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      whitespace: true
    output: "mashup_base64_whitespace.xlsx"

  # --- Milestone 6: Basic M Diffs ---
  - id: "m_change_literal"
    generator: "mashup_inject"
    args:
      base_file: "templates/base_query.xlsx"
      # This query adds a step, changing the definition
      m_code: |
        section Section1;
        shared Query1 = let
            Source = Csv.Document(File.Contents("C:\data.csv"),[Delimiter=",", Columns=2, Encoding=1252, QuoteStyle=QuoteStyle.None]),
            #"Changed Type" = Table.TransformColumnTypes(Source,{{"Column1", type text}, {"Column2", type text}}),
            #"Added Custom" = Table.AddColumn(#"Changed Type", "Custom", each 2)
        in
            #"Added Custom";
    output: "m_change_literal_b.xlsx"

  # --- P1: Large Dense Grid (Performance Baseline) ---
  - id: "p1_large_dense"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "dense" # Deterministic "R1C1" style data
    output: "grid_large_dense.xlsx"

  # --- P2: Large Noise Grid (Worst-case Alignment) ---
  - id: "p2_large_noise"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "noise" # Random float data
      seed: 12345
    output: "grid_large_noise.xlsx"

  # --- D1: Keyed Equality (Database Mode) ---
  # File A: Ordered IDs 1..1000
  - id: "db_equal_ordered_a"
    generator: "db_keyed"
    args: { count: 1000, shuffle: false, seed: 42 }
    output: "db_equal_ordered_a.xlsx"

  # File B: Same data, random order (Tests O(N) alignment)
  - id: "db_equal_ordered_b"
    generator: "db_keyed"
    args: { count: 1000, shuffle: true, seed: 42 }
    output: "db_equal_ordered_b.xlsx"

  # --- D2: Row Added (Database Mode) ---
  - id: "db_row_added_b"
    generator: "db_keyed"
    args: 
      count: 1000 
      seed: 42 
      # Inject a new ID at the end
      extra_rows: [{id: 1001, name: "New Row", amount: 999}]
    output: "db_row_added_b.xlsx"
```

---

### File: `fixtures\pyproject.toml`

```yaml
[project]
name = "excel-fixtures"
version = "0.1.0"
description = "Deterministic artifact generator for Excel Diff testing"
readme = "README.md"
requires-python = ">=3.9"
dependencies = [
    "openpyxl>=3.1.0",
    "lxml>=4.9.0",
    "jinja2>=3.1.0",
    "pyyaml>=6.0",
]

[project.scripts]
generate-fixtures = "src.generate:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src"]

```

---

### File: `fixtures\src\generate.py`

```python
import argparse
import yaml
import sys
from pathlib import Path
from typing import Dict, Any, List

# Import generators
from generators.grid import (
    BasicGridGenerator, 
    SparseGridGenerator, 
    EdgeCaseGenerator, 
    AddressSanityGenerator,
    ValueFormulaGenerator,
    SingleCellDiffGenerator,
)
from generators.corrupt import ContainerCorruptGenerator
from generators.mashup import (
    MashupCorruptGenerator,
    MashupDuplicateGenerator,
    MashupInjectGenerator,
    MashupEncodeGenerator,
)
from generators.perf import LargeGridGenerator
from generators.database import KeyedTableGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "single_cell_diff": SingleCellDiffGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_duplicate": MashupDuplicateGenerator,
    "mashup_inject": MashupInjectGenerator,
    "mashup_encode": MashupEncodeGenerator,
    "perf_large": LargeGridGenerator,
    "db_keyed": KeyedTableGenerator,
}

def load_manifest(manifest_path: Path) -> Dict[str, Any]:
    if not manifest_path.exists():
        print(f"Error: Manifest file not found at {manifest_path}")
        sys.exit(1)
    
    with open(manifest_path, 'r') as f:
        try:
            return yaml.safe_load(f)
        except yaml.YAMLError as e:
            print(f"Error parsing manifest: {e}")
            sys.exit(1)

def ensure_output_dir(output_dir: Path):
    output_dir.mkdir(parents=True, exist_ok=True)

def main():
    script_dir = Path(__file__).parent.resolve()
    fixtures_root = script_dir.parent
    
    default_manifest = fixtures_root / "manifest.yaml"
    default_output = fixtures_root / "generated"

    parser = argparse.ArgumentParser(description="Generate Excel fixtures based on a manifest.")
    parser.add_argument("--manifest", type=Path, default=default_manifest, help="Path to the manifest YAML file.")
    parser.add_argument("--output-dir", type=Path, default=default_output, help="Directory to output generated files.")
    parser.add_argument("--force", action="store_true", help="Force regeneration of existing files.")
    
    args = parser.parse_args()
    
    manifest = load_manifest(args.manifest)
    ensure_output_dir(args.output_dir)
    
    scenarios = manifest.get('scenarios', [])
    print(f"Found {len(scenarios)} scenarios in manifest.")
    
    for scenario in scenarios:
        scenario_id = scenario.get('id')
        generator_name = scenario.get('generator')
        generator_args = scenario.get('args', {})
        outputs = scenario.get('output')
        
        if not scenario_id or not generator_name or not outputs:
            print(f"Skipping invalid scenario: {scenario}")
            continue
            
        print(f"Processing scenario: {scenario_id} (Generator: {generator_name})")
        
        if generator_name not in GENERATORS:
            print(f"  Warning: Generator '{generator_name}' not implemented yet. Skipping.")
            continue
        
        try:
            generator_class = GENERATORS[generator_name]
            generator = generator_class(generator_args)
            generator.generate(args.output_dir, outputs)
            print(f"  Success: Generated {outputs}")
        except Exception as e:
            print(f"  Error generating scenario {scenario_id}: {e}")
            import traceback
            traceback.print_exc()

if __name__ == "__main__":
    main()
```

---

### File: `fixtures\src\__init__.py`

```python

```

---

### File: `fixtures\src\generators\base.py`

```python
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, Any, Union, List

class BaseGenerator(ABC):
    """
    Abstract base class for all fixture generators.
    """
    def __init__(self, args: Dict[str, Any]):
        self.args = args

    @abstractmethod
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        """
        Generates the fixture file(s).
        
        :param output_dir: The directory to save the file(s) in.
        :param output_names: The name(s) of the output file(s) as specified in the manifest.
        """
        pass

    def _post_process_injection(self, file_path: Path, injection_callback):
        """
        Implements the "Pass 2" architecture:
        1. Opens the generated xlsx (zip).
        2. Injects/Modifies streams (DataMashup, etc).
        3. Saves back.
        
        This is a crucial architectural decision to handle openpyxl stripping customXml.
        """
        pass

```

---

### File: `fixtures\src\generators\corrupt.py`

```python
import zipfile
import io
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class ContainerCorruptGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        mode = self.args.get('mode', 'no_content_types')
        
        for name in output_names:
            # Create a dummy zip
            out_path = output_dir / name
            
            if mode == 'random_zip':
                # Just a zip with a text file
                with zipfile.ZipFile(out_path, 'w') as z:
                    z.writestr("hello.txt", "This is not excel")
                    
            elif mode == 'no_content_types':
                # Create a valid excel in memory, then strip [Content_Types].xml
                buffer = io.BytesIO()
                import openpyxl
                wb = openpyxl.Workbook()
                # Add some content just so it's not totally empty
                wb.active['A1'] = 1
                wb.save(buffer)
                buffer.seek(0)
                
                with zipfile.ZipFile(buffer, 'r') as zin:
                    with zipfile.ZipFile(out_path, 'w') as zout:
                        for item in zin.infolist():
                            if item.filename != "[Content_Types].xml":
                                zout.writestr(item, zin.read(item.filename))
            elif mode == 'not_zip_text':
                out_path.write_text("This is not a zip container", encoding="utf-8")
            else:
                raise ValueError(f"Unsupported corrupt_container mode: {mode}")

```

---

### File: `fixtures\src\generators\database.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class KeyedTableGenerator(BaseGenerator):
    """
    Generates datasets with Primary Keys (ID columns).
    Capable of shuffling rows to test O(N) alignment (Database Mode).
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        count = self.args.get('count', 100)
        shuffle = self.args.get('shuffle', False)
        seed = self.args.get('seed', 42)
        extra_rows = self.args.get('extra_rows', [])

        # Use deterministic seed
        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Data"

            # 1. Define Base Data (List of Dicts)
            # Schema: [ID, Name, Amount, Category]
            data_rows = []
            for i in range(1, count + 1):
                data_rows.append({
                    'id': i,
                    'name': f"Customer_{i}",
                    'amount': i * 10.5,
                    'category': rng.choice(['A', 'B', 'C'])
                })

            # 2. Apply Mutations (Additions)
            # This allows us to inject specific "diffs" like D2 (Row Added)
            for row in extra_rows:
                data_rows.append(row)

            # 3. Apply Shuffle (The core D1 test)
            if shuffle:
                rng.shuffle(data_rows)

            # 4. Write to Sheet
            # Header
            headers = ['ID', 'Name', 'Amount', 'Category']
            ws.append(headers)

            for row in data_rows:
                # Ensure strictly ordered list matching headers
                ws.append([
                    row.get('id'),
                    row.get('name'),
                    row.get('amount'),
                    row.get('category')
                ])

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\grid.py`

```python
import openpyxl
import zipfile
import xml.etree.ElementTree as ET
from openpyxl.utils import get_column_letter
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class BasicGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        rows = self.args.get('rows', 5)
        cols = self.args.get('cols', 5)
        two_sheets = self.args.get('two_sheets', False)
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sheet1"
            
            # Fill grid
            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")
            
            # Check if we need a second sheet
            if two_sheets:
                ws2 = wb.create_sheet(title="Sheet2")
                # Different dimensions for Sheet2 (PG1 requirement: 5x2)
                # If args are customized we might need more logic, but for PG1 this is sufficient or we use defaults
                s2_rows = 5
                s2_cols = 2
                for r in range(1, s2_rows + 1):
                    for c in range(1, s2_cols + 1):
                         ws2.cell(row=r, column=c, value=f"S2_R{r}C{c}")

            wb.save(output_dir / name)

class SparseGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sparse"
            
            # Specifics for pg1_sparse_used_range
            ws['A1'] = "A1"
            ws['B2'] = "B2"
            ws['G10'] = "G10" # Forces extent
            # Row 5 and Col D are empty implicitly by not writing to them
            
            wb.save(output_dir / name)

class EdgeCaseGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
        
        for name in output_names:
            wb = openpyxl.Workbook()
            # Remove default sheet
            default_ws = wb.active
            wb.remove(default_ws)
            
            # Empty Sheet
            wb.create_sheet("Empty")
            
            # Values Only
            ws_val = wb.create_sheet("ValuesOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    ws_val.cell(row=r, column=c, value=r*c)
            
            # Formulas Only
            ws_form = wb.create_sheet("FormulasOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    # Reference ValuesOnly sheet
                    col_letter = get_column_letter(c)
                    ws_form.cell(row=r, column=c, value=f"=ValuesOnly!{col_letter}{r}")
            
            wb.save(output_dir / name)

class AddressSanityGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        targets = self.args.get('targets', ["A1", "B2", "Z10"])
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Addresses"
            
            for addr in targets:
                ws[addr] = addr
                
            wb.save(output_dir / name)

class ValueFormulaGenerator(BaseGenerator):
    """PG3: Types, formulas, values"""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Types"
            
            ws['A1'] = 42
            ws['A2'] = "hello"
            ws['A3'] = True
            # A4 empty
            
            ws['B1'] = "=A1+1"
            ws['B2'] = '="hello" & " world"'
            ws['B3'] = "=A1>0"
            
            output_path = output_dir / name
            wb.save(output_path)
            self._inject_formula_caches(output_path)

    def _inject_formula_caches(self, path: Path):
        ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
        with zipfile.ZipFile(path, "r") as zf:
            sheet_xml = zf.read("xl/worksheets/sheet1.xml")
            other_files = {
                info.filename: zf.read(info.filename)
                for info in zf.infolist()
                if info.filename != "xl/worksheets/sheet1.xml"
            }

        root = ET.fromstring(sheet_xml)

        def update_cell(ref: str, value: str, cell_type: str | None = None):
            cell = root.find(f".//{{{ns}}}c[@r='{ref}']")
            if cell is None:
                return
            if cell_type:
                cell.set("t", cell_type)
            v = cell.find(f"{{{ns}}}v")
            if v is None:
                v = ET.SubElement(cell, f"{{{ns}}}v")
            v.text = value

        update_cell("B1", "43")
        update_cell("B2", "hello world", "str")
        update_cell("B3", "1", "b")

        ET.register_namespace("", ns)
        updated_sheet = ET.tostring(root, encoding="utf-8", xml_declaration=False)
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("xl/worksheets/sheet1.xml", updated_sheet)
            for name, data in other_files.items():
                zf.writestr(name, data)

class SingleCellDiffGenerator(BaseGenerator):
    """Generates a tiny pair of workbooks with a single differing cell."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("single_cell_diff generator expects exactly two output filenames")

        rows = self.args.get('rows', 3)
        cols = self.args.get('cols', 3)
        sheet = self.args.get('sheet', "Sheet1")
        target_cell = self.args.get('target_cell', "C3")
        value_a = self.args.get('value_a', "1")
        value_b = self.args.get('value_b', "2")

        def create_workbook(value, name: str):
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = sheet

            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")

            ws[target_cell] = value
            wb.save(output_dir / name)

        create_workbook(value_a, output_names[0])
        create_workbook(value_b, output_names[1])

```

---

### File: `fixtures\src\generators\mashup.py`

```python
import base64
import copy
import io
import random
import re
import struct
import zipfile
from pathlib import Path
from typing import Callable, List, Optional, Union
from xml.etree import ElementTree as ET
from lxml import etree
from .base import BaseGenerator

# XML Namespaces
NS = {'dm': 'http://schemas.microsoft.com/DataMashup'}

class MashupBaseGenerator(BaseGenerator):
    """Base class for handling the outer Excel container and finding DataMashup."""
    
    def _get_mashup_element(self, tree):
        if tree.tag.endswith("DataMashup"):
            return tree
        return tree.find('.//dm:DataMashup', namespaces=NS)

    def _process_excel_container(
        self,
        base_path,
        output_path,
        callback,
        text_mutator: Optional[Callable[[str], str]] = None,
    ):
        """
        Generic wrapper to open xlsx, find customXml, apply a callback to the 
        DataMashup bytes, and save the result.
        """
        # Copy base file structure to output
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for item in zin.infolist():
                    buffer = zin.read(item.filename)
                    
                    # We only care about the item containing DataMashup
                    # Usually customXml/item1.xml, but we check content to be safe
                    has_marker = b"DataMashup" in buffer or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in buffer
                    if item.filename.startswith("customXml/item") and has_marker:
                        # Parse XML
                        root = etree.fromstring(buffer)
                        dm_node = self._get_mashup_element(root)
                        
                        if dm_node is not None:
                            # 1. Decode
                            # The text content might have whitespace/newlines, strip them
                            b64_text = dm_node.text.strip() if dm_node.text else ""
                            if b64_text:
                                raw_bytes = base64.b64decode(b64_text)
                                
                                # 2. Apply modification (The Callback)
                                new_bytes = callback(raw_bytes)
                                
                                # 3. Encode back
                                new_text = base64.b64encode(new_bytes).decode('utf-8')
                                if text_mutator is not None:
                                    new_text = text_mutator(new_text)
                                dm_node.text = new_text
                                buffer = etree.tostring(root, encoding='utf-8', xml_declaration=True)
                    
                    zout.writestr(item, buffer)

class MashupCorruptGenerator(MashupBaseGenerator):
    """Fuzzes the DataMashup bytes to test error handling."""
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        if not base_file_arg:
            raise ValueError("MashupCorruptGenerator requires 'base_file' argument")

        # Resolve base file relative to current working directory or fixtures/templates
        base = Path(base_file_arg)
        if not base.exists():
             # Try looking in fixtures/templates if a relative path was given
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        mode = self.args.get('mode', 'byte_flip')

        def corruptor(data):
            mutable = bytearray(data)
            if len(mutable) == 0:
                return bytes(mutable)

            if mode == 'byte_flip':
                # Flip a byte in the middle
                idx = len(mutable) // 2
                mutable[idx] = mutable[idx] ^ 0xFF
            elif mode == 'truncate':
                return mutable[:len(mutable)//2]
            return bytes(mutable)

        for name in output_names:
            # Convert Path objects to strings for resolve() to work correctly if there's a mix
            # Actually output_dir is a Path. name is str.
            # .resolve() resolves symlinks and relative paths to absolute
            target_path = (output_dir / name).resolve()
            text_mutator = self._garble_base64_text if mode == 'byte_flip' else None
            self._process_excel_container(
                base.resolve(),
                target_path,
                corruptor,
                text_mutator=text_mutator,
            )

    def _garble_base64_text(self, encoded: str) -> str:
        if not encoded:
            return "!!"
        chars = list(encoded)
        chars[0] = "!"
        return "".join(chars)


class MashupInjectGenerator(MashupBaseGenerator):
    """
    Peels the onion:
    1. Parses MS-QDEFF binary header.
    2. Unzips PackageParts.
    3. Injects new M-Code into Section1.m.
    4. Re-zips and fixes header lengths.
    """
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        new_m_code = self.args.get('m_code')

        if not base_file_arg:
             raise ValueError("MashupInjectGenerator requires 'base_file' argument")
        if new_m_code is None:
             raise ValueError("MashupInjectGenerator requires 'm_code' argument")

        base = Path(base_file_arg)
        if not base.exists():
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        def injector(raw_bytes):
            return self._inject_m_code(raw_bytes, new_m_code)

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, injector)

    def _inject_m_code(self, raw_bytes, m_code):
        # --- 1. Parse MS-QDEFF Header ---
        # Format: Version(4) + LenPP(4) + PackageParts(...) + LenPerm(4) + ...
        # We assume Version is 0 (first 4 bytes)
        
        if len(raw_bytes) < 8:
            return raw_bytes # Too short to handle

        offset = 4
        # Read PackageParts Length
        pp_len = struct.unpack('<I', raw_bytes[offset:offset+4])[0]
        offset += 4
        
        # Extract existing components
        pp_bytes = raw_bytes[offset : offset + pp_len]
        
        # Keep the rest of the stream (Permissions, Metadata, Bindings) intact
        # We just append it later
        remainder_bytes = raw_bytes[offset + pp_len :]

        # --- 2. Modify PackageParts (Inner ZIP) ---
        new_pp_bytes = self._replace_in_zip(pp_bytes, 'Formulas/Section1.m', m_code)

        # --- 3. Rebuild Stream ---
        # New Length for PackageParts
        new_pp_len = len(new_pp_bytes)
        
        # Reconstruct: Version(0) + NewLen + NewPP + Remainder
        header = raw_bytes[:4] # Version
        len_pack = struct.pack('<I', new_pp_len)
        
        return header + len_pack + new_pp_bytes + remainder_bytes

    def _replace_in_zip(self, zip_bytes, filename, new_content):
        """Opens a ZIP byte stream, replaces a file, returns new ZIP byte stream."""
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()
        
        try:
            with zipfile.ZipFile(in_buffer, 'r') as zin:
                with zipfile.ZipFile(out_buffer, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
                    for item in zin.infolist():
                        if item.filename == filename:
                            # Write the new M code
                            zout.writestr(filename, new_content.encode('utf-8'))
                        else:
                            # Copy others
                            zout.writestr(item, zin.read(item.filename))
        except zipfile.BadZipFile:
            # Fallback if inner stream isn't a valid zip (shouldn't happen on valid QDEFF)
            return zip_bytes
            
        return out_buffer.getvalue()


class MashupDuplicateGenerator(MashupBaseGenerator):
    """
    Duplicates the customXml part that contains DataMashup to produce two
    DataMashup occurrences in a single workbook.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        mode = self.args.get('mode', 'part')
        if not base_file_arg:
            raise ValueError("MashupDuplicateGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            if mode == 'part':
                self._duplicate_datamashup_part(base.resolve(), target_path)
            elif mode == 'element':
                self._duplicate_datamashup_element(base.resolve(), target_path)
            else:
                raise ValueError(f"Unsupported duplicate mode: {mode}")

    def _duplicate_datamashup_part(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            try:
                item1_xml = zin.read("customXml/item1.xml")
                item_props1 = zin.read("customXml/itemProps1.xml")
                item1_rels = zin.read("customXml/_rels/item1.xml.rels")
                content_types = zin.read("[Content_Types].xml")
                workbook_rels = zin.read("xl/_rels/workbook.xml.rels")
            except KeyError as e:
                raise FileNotFoundError(f"Required DataMashup part missing: {e}") from e

            updated_content_types = self._add_itemprops_override(content_types)
            updated_workbook_rels = self._add_workbook_relationship(workbook_rels)
            item2_rels = item1_rels.replace(b"itemProps1.xml", b"itemProps2.xml")
            item_props2 = item_props1.replace(
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BE}",
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BF}",
            )

            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename == "[Content_Types].xml":
                        data = updated_content_types
                    elif info.filename == "xl/_rels/workbook.xml.rels":
                        data = updated_workbook_rels
                    zout.writestr(info, data)

                zout.writestr("customXml/item2.xml", item1_xml)
                zout.writestr("customXml/itemProps2.xml", item_props2)
                zout.writestr("customXml/_rels/item2.xml.rels", item2_rels)

    def _add_itemprops_override(self, content_types_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/content-types"
        root = ET.fromstring(content_types_bytes)
        override_tag = f"{{{ns}}}Override"
        if not any(
            elem.get("PartName") == "/customXml/itemProps2.xml"
            for elem in root.findall(override_tag)
        ):
            new_override = ET.SubElement(root, override_tag)
            new_override.set("PartName", "/customXml/itemProps2.xml")
            new_override.set(
                "ContentType",
                "application/vnd.openxmlformats-officedocument.customXmlProperties+xml",
            )
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _add_workbook_relationship(self, rels_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/relationships"
        root = ET.fromstring(rels_bytes)
        rel_tag = f"{{{ns}}}Relationship"
        existing_ids = {elem.get("Id") for elem in root.findall(rel_tag)}
        next_id = 1
        while f"rId{next_id}" in existing_ids:
            next_id += 1
        new_rel = ET.SubElement(root, rel_tag)
        new_rel.set("Id", f"rId{next_id}")
        new_rel.set(
            "Type",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
        )
        new_rel.set("Target", "../customXml/item2.xml")
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _duplicate_datamashup_element(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            root = etree.fromstring(data)
                            dm_node = self._get_mashup_element(root)
                            if dm_node is not None:
                                duplicate = copy.deepcopy(dm_node)
                                parent = dm_node.getparent()
                                if parent is not None:
                                    parent.append(duplicate)
                                    target_root = root
                                else:
                                    container = etree.Element("root", nsmap=root.nsmap)
                                    container.append(dm_node)
                                    container.append(duplicate)
                                    target_root = container
                                data = etree.tostring(
                                    target_root, encoding="utf-8", xml_declaration=True
                                )
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)


class MashupEncodeGenerator(MashupBaseGenerator):
    """
    Re-encodes the DataMashup customXml stream to a target encoding and optionally
    inserts whitespace into the base64 payload.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        encoding = self.args.get('encoding', 'utf-8')
        whitespace = bool(self.args.get('whitespace', False))
        if not base_file_arg:
            raise ValueError("MashupEncodeGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._rewrite_datamashup_xml(base.resolve(), target_path, encoding, whitespace)

    def _rewrite_datamashup_xml(
        self,
        base_path: Path,
        output_path: Path,
        encoding: str,
        whitespace: bool,
    ):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            data = self._process_datamashup_stream(data, encoding, whitespace)
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)

    def _process_datamashup_stream(
        self,
        xml_bytes: bytes,
        encoding: str,
        whitespace: bool,
    ) -> bytes:
        root = etree.fromstring(xml_bytes)
        dm_node = self._get_mashup_element(root)
        if dm_node is None:
            return xml_bytes

        if dm_node.text and whitespace:
            dm_node.text = self._with_whitespace(dm_node.text)

        xml_bytes = etree.tostring(root, encoding="utf-8", xml_declaration=True)
        return self._encode_bytes(xml_bytes, encoding)

    def _with_whitespace(self, text: str) -> str:
        cleaned = text.strip()
        if not cleaned:
            return text
        midpoint = max(1, len(cleaned) // 2)
        return f"\n  {cleaned[:midpoint]}\n  {cleaned[midpoint:]}\n"

    def _encode_bytes(self, xml_bytes: bytes, encoding: str) -> bytes:
        enc = encoding.lower()
        if enc == "utf-8":
            return xml_bytes
        if enc == "utf-16-le":
            return self._to_utf16(xml_bytes, little_endian=True)
        if enc == "utf-16-be":
            return self._to_utf16(xml_bytes, little_endian=False)
        raise ValueError(f"Unsupported encoding: {encoding}")

    def _to_utf16(self, xml_bytes: bytes, little_endian: bool) -> bytes:
        text = xml_bytes.decode("utf-8")
        text = self._rewrite_declaration(text)
        encoded = text.encode("utf-16-le" if little_endian else "utf-16-be")
        bom = b"\xff\xfe" if little_endian else b"\xfe\xff"
        return bom + encoded

    def _rewrite_declaration(self, text: str) -> str:
        pattern = r'encoding=["\'][^"\']+["\']'
        if re.search(pattern, text):
            return re.sub(pattern, 'encoding="UTF-16"', text, count=1)
        prefix = "<?xml version='1.0'?>"
        if text.startswith(prefix):
            return text.replace(prefix, "<?xml version='1.0' encoding='UTF-16'?>", 1)
        return text

```

---

### File: `fixtures\src\generators\perf.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class LargeGridGenerator(BaseGenerator):
    """
    Generates massive grids using WriteOnly mode to save memory.
    Targeting P1/P2 milestones.
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        rows = self.args.get('rows', 1000)
        cols = self.args.get('cols', 10)
        mode = self.args.get('mode', 'dense')
        seed = self.args.get('seed', 0)

        # Use deterministic seed if provided, otherwise system time
        rng = random.Random(seed)

        for name in output_names:
            # WriteOnly mode is critical for 50k+ rows in Python
            wb = openpyxl.Workbook(write_only=True)
            ws = wb.create_sheet()
            ws.title = "Performance"

            # 1. Header
            header = [f"Col_{c}" for c in range(1, cols + 1)]
            ws.append(header)

            # 2. Data Stream
            for r in range(1, rows + 1):
                row_data = []
                if mode == 'dense':
                    # Deterministic pattern: "R{r}C{c}"
                    # Fast to generate, high compression ratio
                    row_data = [f"R{r}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'noise':
                    # Random floats: Harder to align, harder to compress
                    row_data = [rng.random() for _ in range(cols)]
                
                ws.append(row_data)

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\__init__.py`

```python
# Generators package

```

---

