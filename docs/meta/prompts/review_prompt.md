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
    tests/
      addressing_pg2_tests.rs
      excel_open_xml_tests.rs
      integration_test.rs
      pg1_ir_tests.rs
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
      grid_large_dense.xlsx
      grid_large_noise.xlsx
      minimal.xlsx
      m_change_literal_b.xlsx
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
}

struct SheetDescriptor {
    name: String,
    rel_id: Option<String>,
    sheet_id: Option<u32>,
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
            Ok(shared_strings.get(idx).cloned().map(CellValue::Text))
        }
        Some("b") => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some("str") | Some("inlineStr") => Ok(Some(CellValue::Text(trimmed.to_string()))),
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
```

---

### File: `core\src\lib.rs`

```rust
pub mod addressing;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, open_workbook};
pub use workbook::{Cell, CellAddress, CellValue, Grid, Row, Sheet, SheetKind, Workbook};
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
use crate::addressing::index_to_address;

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

#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Number(f64),
    Text(String),
    Bool(bool),
}

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

  # --- Milestone 2.2: Base64 Correctness ---
  - id: "corrupt_base64"
    generator: "mashup_corrupt"
    args: 
      base_file: "templates/base_query.xlsx"
      mode: "byte_flip"
    output: "corrupt_base64.xlsx"

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
    ValueFormulaGenerator
)
from generators.corrupt import ContainerCorruptGenerator
from generators.mashup import MashupCorruptGenerator, MashupInjectGenerator
from generators.perf import LargeGridGenerator
from generators.database import KeyedTableGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_inject": MashupInjectGenerator,
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
            
            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\mashup.py`

```python
import base64
import struct
import zipfile
import io
import random
from pathlib import Path
from typing import Union, List
from lxml import etree
from .base import BaseGenerator

# XML Namespaces
NS = {'dm': 'http://schemas.microsoft.com/DataMashup'}

class MashupBaseGenerator(BaseGenerator):
    """Base class for handling the outer Excel container and finding DataMashup."""
    
    def _get_mashup_element(self, tree):
        return tree.find('//dm:DataMashup', namespaces=NS)

    def _process_excel_container(self, base_path, output_path, callback):
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
                    if item.filename.startswith("customXml/item") and b"DataMashup" in buffer:
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
                                dm_node.text = base64.b64encode(new_bytes).decode('utf-8')
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
            self._process_excel_container(base.resolve(), target_path, corruptor)


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

