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
