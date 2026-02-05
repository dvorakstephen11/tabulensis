//! XML parsing for Excel worksheet grids.
//!
//! Handles parsing of worksheet XML, shared strings, workbook structure, and
//! relationship files to construct [`Grid`] representations of sheet data.

use crate::addressing::address_to_index;
use crate::error_codes;
use crate::string_pool::{StringId, StringPool};
use crate::workbook::{CellValue, Grid, NamedRange};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GridParseError {
    #[error("[EXDIFF_GRID_001] XML parse error: {0}. Suggestion: re-save the file in Excel or verify it is valid XML.")]
    XmlError(String),
    #[error("[EXDIFF_GRID_001] XML parse error at line {line}, column {column}: {message}. Suggestion: re-save the file in Excel or verify it is valid XML.")]
    XmlErrorAt {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("[EXDIFF_GRID_002] invalid cell address: {0}. Suggestion: the workbook may be corrupt.")]
    InvalidAddress(String),
    #[error("[EXDIFF_GRID_003] shared string index {0} out of bounds. Suggestion: the workbook may be corrupt.")]
    SharedStringOutOfBounds(usize),
}

impl GridParseError {
    pub fn code(&self) -> &'static str {
        match self {
            GridParseError::XmlError(_) => error_codes::GRID_XML_ERROR,
            GridParseError::XmlErrorAt { .. } => error_codes::GRID_XML_ERROR,
            GridParseError::InvalidAddress(_) => error_codes::GRID_INVALID_ADDRESS,
            GridParseError::SharedStringOutOfBounds(_) => error_codes::GRID_SHARED_STRING_OOB,
        }
    }
}

pub struct SheetDescriptor {
    pub name: String,
    pub rel_id: Option<String>,
    pub sheet_id: Option<u32>,
}

pub fn parse_shared_strings(
    xml: &[u8],
    pool: &mut StringPool,
) -> Result<Vec<StringId>, GridParseError> {
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
                    .map_err(|e| xml_err(&reader, xml, e))?
                    .into_owned();
                current.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"si" => {
                let id = pool.intern(&current);
                strings.push(id);
                in_si = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(strings)
}

pub fn parse_workbook_xml(xml: &[u8]) -> Result<Vec<SheetDescriptor>, GridParseError> {
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
                    let attr = attr.map_err(|e| xml_msg_err(&reader, xml, e.to_string()))?;
                    match attr.key.as_ref() {
                        b"name" => {
                            name = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, xml, e))?
                                    .into_owned(),
                            )
                        }
                        b"sheetId" => {
                            let parsed = attr
                                .unescape_value()
                                .map_err(|e| xml_err(&reader, xml, e))?;
                            sheet_id = parsed.into_owned().parse::<u32>().ok();
                        }
                        b"r:id" => {
                            rel_id = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, xml, e))?
                                    .into_owned(),
                            )
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
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(sheets)
}

pub fn parse_defined_names(
    workbook_xml: &[u8],
    sheets_in_order: &[SheetDescriptor],
    pool: &mut StringPool,
) -> Result<Vec<NamedRange>, GridParseError> {
    fn local_name(name: &[u8]) -> &[u8] {
        name.rsplit(|&b| b == b':').next().unwrap_or(name)
    }

    fn quote_sheet_name(sheet: &str) -> String {
        let needs_quotes = sheet
            .chars()
            .any(|c| matches!(c, ' ' | '\'' | '!' | ',' | ';' | '[' | ']' | '(' | ')'));
        if !needs_quotes {
            return sheet.to_string();
        }
        let escaped = sheet.replace('\'', "''");
        format!("'{escaped}'")
    }

    let mut reader = Reader::from_reader(workbook_xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut named_ranges = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if local_name(e.name().as_ref()) == b"definedName" => {
                let mut name = None;
                let mut local_sheet_id = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| xml_msg_err(&reader, workbook_xml, e.to_string()))?;
                    match attr.key.as_ref() {
                        b"name" => {
                            name = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, workbook_xml, e))?
                                    .into_owned(),
                            );
                        }
                        b"localSheetId" => {
                            let value = attr
                                .unescape_value()
                                .map_err(|e| xml_err(&reader, workbook_xml, e))?
                                .into_owned();
                            local_sheet_id = value.parse::<usize>().ok();
                        }
                        _ => {}
                    }
                }

                let name = match name {
                    Some(name) => name,
                    None => {
                        return Err(xml_msg_err(
                            &reader,
                            workbook_xml,
                            "definedName missing required 'name' attribute",
                        ));
                    }
                };

                let refers_to = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(&reader, workbook_xml, e))?
                    .into_owned();
                let refers_to = refers_to.trim();

                let (qualified_name, scope) = match local_sheet_id {
                    None => (name.clone(), None),
                    Some(idx) => {
                        let sheet_name = sheets_in_order.get(idx).map(|s| s.name.as_str());
                        let sheet_name = match sheet_name {
                            Some(sheet_name) => sheet_name,
                            None => {
                                return Err(xml_msg_err(
                                    &reader,
                                    workbook_xml,
                                    format!(
                                        "definedName localSheetId {idx} out of bounds (sheets={})",
                                        sheets_in_order.len()
                                    ),
                                ));
                            }
                        };
                        let sheet_name_id = pool.intern(sheet_name);
                        let qualified = format!("{}!{}", quote_sheet_name(sheet_name), name);
                        (qualified, Some(sheet_name_id))
                    }
                };

                named_ranges.push(NamedRange {
                    name: pool.intern(&qualified_name),
                    refers_to: pool.intern(refers_to),
                    scope,
                });
            }
            Ok(Event::Empty(e)) if local_name(e.name().as_ref()) == b"definedName" => {
                let mut name = None;
                let mut local_sheet_id = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| xml_msg_err(&reader, workbook_xml, e.to_string()))?;
                    match attr.key.as_ref() {
                        b"name" => {
                            name = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, workbook_xml, e))?
                                    .into_owned(),
                            );
                        }
                        b"localSheetId" => {
                            let value = attr
                                .unescape_value()
                                .map_err(|e| xml_err(&reader, workbook_xml, e))?
                                .into_owned();
                            local_sheet_id = value.parse::<usize>().ok();
                        }
                        _ => {}
                    }
                }

                let name = match name {
                    Some(name) => name,
                    None => {
                        return Err(xml_msg_err(
                            &reader,
                            workbook_xml,
                            "definedName missing required 'name' attribute",
                        ));
                    }
                };

                let (qualified_name, scope) = match local_sheet_id {
                    None => (name.clone(), None),
                    Some(idx) => {
                        let sheet_name = sheets_in_order.get(idx).map(|s| s.name.as_str());
                        let sheet_name = match sheet_name {
                            Some(sheet_name) => sheet_name,
                            None => {
                                return Err(xml_msg_err(
                                    &reader,
                                    workbook_xml,
                                    format!(
                                        "definedName localSheetId {idx} out of bounds (sheets={})",
                                        sheets_in_order.len()
                                    ),
                                ));
                            }
                        };
                        let sheet_name_id = pool.intern(sheet_name);
                        let qualified = format!("{}!{}", quote_sheet_name(sheet_name), name);
                        (qualified, Some(sheet_name_id))
                    }
                };

                named_ranges.push(NamedRange {
                    name: pool.intern(&qualified_name),
                    refers_to: pool.intern(""),
                    scope,
                });
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(xml_err(&reader, workbook_xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(named_ranges)
}

pub fn parse_relationships(xml: &[u8]) -> Result<HashMap<String, String>, GridParseError> {
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
                    let attr = attr.map_err(|e| xml_msg_err(&reader, xml, e.to_string()))?;
                    match attr.key.as_ref() {
                        b"Id" => {
                            id = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, xml, e))?
                                    .into_owned(),
                            )
                        }
                        b"Target" => {
                            target = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, xml, e))?
                                    .into_owned(),
                            )
                        }
                        b"Type" => {
                            rel_type = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, xml, e))?
                                    .into_owned(),
                            )
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
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(map)
}

pub fn parse_relationships_all(xml: &[u8]) -> Result<HashMap<String, String>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut map = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"Relationship" => {
                let mut id = None;
                let mut target = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| xml_msg_err(&reader, xml, e.to_string()))?;
                    match attr.key.as_ref() {
                        b"Id" => {
                            id = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, xml, e))?
                                    .into_owned(),
                            )
                        }
                        b"Target" => {
                            target = Some(
                                attr.unescape_value()
                                    .map_err(|e| xml_err(&reader, xml, e))?
                                    .into_owned(),
                            )
                        }
                        _ => {}
                    }
                }

                if let (Some(id), Some(target)) = (id, target) {
                    map.insert(id, target);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(map)
}

pub fn resolve_sheet_target(
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

pub fn parse_sheet_xml(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<Grid, GridParseError> {
    const STREAM_CELL_BUFFER_LIMIT: usize = 4096;

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut cell_buf = Vec::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut grid: Option<Grid> = None;
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"dimension" => {
                if let Some(r) = get_attr_value(&reader, xml, &e, b"ref")? {
                    dimension_hint = dimension_from_ref(r.as_ref());
                    if grid.is_none()
                        && dimension_hint.is_some()
                        && parsed_cells.len() >= STREAM_CELL_BUFFER_LIMIT
                    {
                        let (nrows, ncols) =
                            grid_bounds_from_hint(dimension_hint, max_row, max_col);
                        let new_grid = build_grid(
                            nrows,
                            ncols,
                            std::mem::take(&mut parsed_cells),
                        )?;
                        grid = Some(new_grid);
                    }
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"c" => {
                let cell = parse_cell(&mut reader, xml, e, shared_strings, pool, &mut cell_buf)?;
                max_row = Some(max_row.map_or(cell.row, |r| r.max(cell.row)));
                max_col = Some(max_col.map_or(cell.col, |c| c.max(cell.col)));
                if grid.is_some() {
                    let needs_resize = grid
                        .as_ref()
                        .map(|existing| cell.row >= existing.nrows || cell.col >= existing.ncols)
                        .unwrap_or(false);
                    if needs_resize {
                        let (mut nrows, mut ncols) =
                            grid_bounds_from_hint(dimension_hint, max_row, max_col);
                        nrows = nrows.max(cell.row.saturating_add(1));
                        ncols = ncols.max(cell.col.saturating_add(1));
                        let rebuilt = rebuild_grid(grid.take().unwrap(), nrows, ncols);
                        grid = Some(rebuilt);
                    }
                    grid.as_mut()
                        .unwrap()
                        .insert_cell(cell.row, cell.col, cell.value, cell.formula);
                } else {
                    parsed_cells.push(cell);
                    if dimension_hint.is_some()
                        && parsed_cells.len() >= STREAM_CELL_BUFFER_LIMIT
                    {
                        let (nrows, ncols) =
                            grid_bounds_from_hint(dimension_hint, max_row, max_col);
                        let new_grid = build_grid(
                            nrows,
                            ncols,
                            std::mem::take(&mut parsed_cells),
                        )?;
                        grid = Some(new_grid);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    if let Some(mut grid) = grid {
        if !parsed_cells.is_empty() {
            let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
            if nrows > grid.nrows || ncols > grid.ncols {
                grid = rebuild_grid(grid, nrows, ncols);
            }
            for cell in parsed_cells {
                grid.insert_cell(cell.row, cell.col, cell.value, cell.formula);
            }
        } else {
            let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
            if nrows > grid.nrows || ncols > grid.ncols {
                grid = rebuild_grid(grid, nrows, ncols);
            }
        }
        return Ok(grid);
    }

    if parsed_cells.is_empty() {
        return Ok(Grid::new(0, 0));
    }

    let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
    build_grid(nrows, ncols, parsed_cells)
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    start: BytesStart,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    buf: &mut Vec<u8>,
) -> Result<ParsedCell, GridParseError> {
    let mut address_raw = None;
    let mut cell_type = None;

    for attr in start.attributes() {
        let attr = attr.map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
        match attr.key.as_ref() {
            b"r" => {
                address_raw = Some(attr.unescape_value().map_err(|e| xml_err(reader, xml, e))?);
            }
            b"t" => {
                cell_type = Some(attr.unescape_value().map_err(|e| xml_err(reader, xml, e))?);
            }
            _ => {}
        }
    }

    let address_raw = address_raw.ok_or_else(|| xml_msg_err(reader, xml, "cell missing address"))?;
    let (row, col) = address_to_index(address_raw.as_ref())
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.into_owned()))?;

    let mut value: Option<CellValue> = None;
    let mut formula: Option<StringId> = None;

    buf.clear();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                value = convert_value(
                    Some(text.as_ref()),
                    cell_type.as_deref(),
                    shared_strings,
                    pool,
                    reader,
                    xml,
                )?;
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                let unescaped = quick_xml::escape::unescape(text.as_ref())
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
                formula = Some(pool.intern(unescaped.as_ref()));
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                let inline = read_inline_string(reader, xml)?;
                value = Some(CellValue::Text(pool.intern(&inline)));
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(xml_msg_err(reader, xml, "unexpected EOF inside cell"));
            }
            Err(e) => return Err(xml_err(reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(ParsedCell {
        row,
        col,
        value,
        formula,
    })
}

fn read_inline_string(reader: &mut Reader<&[u8]>, xml: &[u8]) -> Result<String, GridParseError> {
    let mut buf = Vec::new();
    let mut value = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?
                    .into_owned();
                value.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"is" => break,
            Ok(Event::Eof) => {
                return Err(xml_msg_err(
                    reader,
                    xml,
                    "unexpected EOF inside inline string",
                ));
            }
            Err(e) => return Err(xml_err(reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }
    Ok(value)
}

fn convert_value(
    value_text: Option<&str>,
    cell_type: Option<&str>,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    reader: &Reader<&[u8]>,
    xml: &[u8],
) -> Result<Option<CellValue>, GridParseError> {
    fn parse_usize_decimal(s: &str) -> Option<usize> {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return None;
        }
        let mut n: usize = 0;
        for &b in bytes {
            if !b.is_ascii_digit() {
                return None;
            }
            let d = (b - b'0') as usize;
            n = n.checked_mul(10)?.checked_add(d)?;
        }
        Some(n)
    }

    fn parse_f64_fast(s: &str) -> Option<f64> {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return None;
        }

        let mut i = 0usize;
        let mut neg = false;
        match bytes[0] {
            b'-' => {
                neg = true;
                i = 1;
            }
            b'+' => {
                i = 1;
            }
            _ => {}
        }

        if i >= bytes.len() {
            return None;
        }

        let mut int: u64 = 0;
        let mut saw_digit = false;

        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_digit() {
                saw_digit = true;
                let d = (b - b'0') as u64;
                if int > (u64::MAX - d) / 10 {
                    return s.parse::<f64>().ok();
                }
                int = int * 10 + d;
                i += 1;
                continue;
            }
            break;
        }

        if !saw_digit {
            return None;
        }

        if i == bytes.len() {
            let v = int as f64;
            return Some(if neg { -v } else { v });
        }

        match bytes[i] {
            b'.' | b'e' | b'E' => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    let raw = match value_text {
        Some(t) => t,
        None => return Ok(None),
    };

    let trimmed = raw.trim();
    if raw.is_empty() || trimmed.is_empty() {
        return Ok(Some(CellValue::Text(pool.intern(""))));
    }

    match cell_type {
        Some("s") => {
            let idx = match parse_usize_decimal(trimmed) {
                Some(v) => v,
                None => trimmed
                    .parse::<usize>()
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?,
            };
            let text_id = *shared_strings
                .get(idx)
                .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
            Ok(Some(CellValue::Text(text_id)))
        }
        Some("b") => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some("e") => Ok(Some(CellValue::Error(pool.intern(trimmed)))),
        Some("str") | Some("inlineStr") => Ok(Some(CellValue::Text(pool.intern(raw)))),
        _ => {
            if let Some(n) = parse_f64_fast(trimmed) {
                Ok(Some(CellValue::Number(n)))
            } else {
                Ok(Some(CellValue::Text(pool.intern(trimmed))))
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

fn grid_bounds_from_hint(
    dimension_hint: Option<(u32, u32)>,
    max_row: Option<u32>,
    max_col: Option<u32>,
) -> (u32, u32) {
    let mut nrows = dimension_hint.map(|(r, _)| r).unwrap_or(0);
    let mut ncols = dimension_hint.map(|(_, c)| c).unwrap_or(0);

    if let Some(max_r) = max_row {
        nrows = nrows.max(max_r.saturating_add(1));
    }
    if let Some(max_c) = max_col {
        ncols = ncols.max(max_c.saturating_add(1));
    }

    (nrows, ncols)
}

fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, GridParseError> {
    let filled = cells.len();
    let mut grid = if Grid::should_use_dense(nrows, ncols, filled) {
        Grid::new_dense(nrows, ncols)
    } else {
        Grid::new(nrows, ncols)
    };

    for parsed in cells {
        if parsed.value.is_none() && parsed.formula.is_none() {
            continue;
        }

        debug_assert!(parsed.row < nrows && parsed.col < ncols);

        grid.cells.insert(
            parsed.row,
            parsed.col,
            crate::workbook::CellContent {
                value: parsed.value,
                formula: parsed.formula,
            },
        );
    }

    Ok(grid)
}

fn rebuild_grid(grid: Grid, nrows: u32, ncols: u32) -> Grid {
    if grid.nrows == nrows && grid.ncols == ncols {
        return grid;
    }

    let filled = grid.cell_count();
    let mut rebuilt = if Grid::should_use_dense(nrows, ncols, filled) {
        Grid::new_dense(nrows, ncols)
    } else {
        Grid::new(nrows, ncols)
    };

    for ((row, col), cell) in grid.iter_cells() {
        rebuilt.insert_cell(row, col, cell.value.clone(), cell.formula);
    }

    rebuilt
}

fn get_attr_value<'a>(
    reader: &Reader<&[u8]>,
    xml: &[u8],
    element: &'a BytesStart<'a>,
    key: &[u8],
) -> Result<Option<std::borrow::Cow<'a, str>>, GridParseError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
        if attr.key.as_ref() == key {
            let v = attr
                .unescape_value()
                .map_err(|e| xml_err(reader, xml, e))?;
            return Ok(Some(v));
        }
    }
    Ok(None)
}

fn xml_err(reader: &Reader<&[u8]>, xml: &[u8], err: quick_xml::Error) -> GridParseError {
    xml_error_with_position(err, xml, reader.buffer_position())
}

fn xml_msg_err(reader: &Reader<&[u8]>, xml: &[u8], message: impl Into<String>) -> GridParseError {
    let (line, column) = compute_line_col(xml, reader.buffer_position());
    GridParseError::XmlErrorAt {
        line,
        column,
        message: message.into(),
    }
}

fn xml_error_with_position(
    err: quick_xml::Error,
    xml: &[u8],
    byte_offset: usize,
) -> GridParseError {
    let (line, column) = compute_line_col(xml, byte_offset);
    GridParseError::XmlErrorAt {
        line,
        column,
        message: err.to_string(),
    }
}

fn compute_line_col(data: &[u8], offset: usize) -> (usize, usize) {
    let safe_offset = offset.min(data.len());
    let slice = &data[..safe_offset];
    let line = slice.iter().filter(|&&b| b == b'\n').count() + 1;
    let last_newline = slice.iter().rposition(|&b| b == b'\n');
    let column = match last_newline {
        Some(pos) => safe_offset - pos,
        None => safe_offset + 1,
    };
    (line, column)
}

struct ParsedCell {
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
}

#[cfg(test)]
mod tests {
    use super::{GridParseError, convert_value, parse_shared_strings, read_inline_string};
    use crate::string_pool::StringPool;
    use crate::workbook::CellValue;
    use quick_xml::Reader;

    #[test]
    fn parse_shared_strings_rich_text_flattens_runs() {
        let xml = br#"<?xml version="1.0"?>
<sst>
  <si>
    <r><t>Hello</t></r>
    <r><t xml:space="preserve"> World</t></r>
  </si>
</sst>"#;
        let mut pool = StringPool::new();
        let strings = parse_shared_strings(xml, &mut pool).expect("shared strings should parse");
        let first = strings.first().copied().unwrap();
        assert_eq!(pool.resolve(first), "Hello World");
    }

    #[test]
    fn read_inline_string_preserves_xml_space_preserve() {
        let xml = br#"<is><t xml:space="preserve"> hello</t></is>"#;
        let mut reader = Reader::from_reader(xml.as_ref());
        reader.config_mut().trim_text(false);
        let value = read_inline_string(&mut reader, xml).expect("inline string should parse");
        assert_eq!(value, " hello");

        let mut pool = StringPool::new();
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);
        let converted = convert_value(
            Some(value.as_str()),
            Some("inlineStr"),
            &[],
            &mut pool,
            &dummy_reader,
            dummy_xml,
        )
            .expect("inlineStr conversion should succeed");
        let text_id = converted
            .as_ref()
            .and_then(CellValue::as_text_id)
            .expect("text id");
        assert_eq!(pool.resolve(text_id), " hello");
    }

    #[test]
    fn convert_value_bool_0_1_and_other() {
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);

        let mut pool = StringPool::new();
        let false_val = convert_value(Some("0"), Some("b"), &[], &mut pool, &dummy_reader, dummy_xml)
            .expect("bool cell conversion should succeed");
        assert_eq!(false_val, Some(CellValue::Bool(false)));

        let mut pool = StringPool::new();
        let true_val = convert_value(Some("1"), Some("b"), &[], &mut pool, &dummy_reader, dummy_xml)
            .expect("bool cell conversion should succeed");
        assert_eq!(true_val, Some(CellValue::Bool(true)));

        let none_val = convert_value(Some("2"), Some("b"), &[], &mut pool, &dummy_reader, dummy_xml)
            .expect("unexpected bool tokens should still parse");
        assert!(none_val.is_none());
    }

    #[test]
    fn convert_value_shared_string_index_out_of_bounds_errors() {
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);

        let mut pool = StringPool::new();
        let only_id = pool.intern("only");
        let err = convert_value(Some("5"), Some("s"), &[only_id], &mut pool, &dummy_reader, dummy_xml)
            .expect_err("invalid shared string index should error");
        assert!(matches!(err, GridParseError::SharedStringOutOfBounds(5)));
    }

    #[test]
    fn convert_value_error_cell_as_text() {
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);

        let mut pool = StringPool::new();
        let value = convert_value(Some("#DIV/0!"), Some("e"), &[], &mut pool, &dummy_reader, dummy_xml)
            .expect("error cell should convert");
        let err_id = value
            .and_then(|v| {
                if let CellValue::Error(id) = v {
                    Some(id)
                } else {
                    None
                }
            })
            .expect("error id");
        assert_eq!(pool.resolve(err_id), "#DIV/0!");
    }
}
