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
