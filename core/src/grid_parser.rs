//! XML parsing for Excel worksheet grids.
//!
//! Handles parsing of worksheet XML, shared strings, workbook structure, and
//! relationship files to construct [`Grid`] representations of sheet data.

use crate::addressing::address_to_index;
use crate::error_codes;
use crate::string_pool::{StringId, StringPool};
use crate::workbook::{CellContent, CellValue, Grid, GridStorage, NamedRange};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GridParseError {
    #[error(
        "[EXDIFF_GRID_001] XML parse error: {0}. Suggestion: re-save the file in Excel or verify it is valid XML."
    )]
    XmlError(String),
    #[error(
        "[EXDIFF_GRID_001] XML parse error at line {line}, column {column}: {message}. Suggestion: re-save the file in Excel or verify it is valid XML."
    )]
    XmlErrorAt {
        line: usize,
        column: usize,
        message: String,
    },
    #[error(
        "[EXDIFF_GRID_002] invalid cell address: {0}. Suggestion: the workbook may be corrupt."
    )]
    InvalidAddress(String),
    #[error(
        "[EXDIFF_GRID_003] shared string index {0} out of bounds. Suggestion: the workbook may be corrupt."
    )]
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

#[derive(Debug)]
pub struct ParsedSheetXml {
    pub grid: Grid,
    pub drawing_rids: Vec<String>,
}

const STREAM_CELL_BUFFER_LIMIT: usize = 4096;
const STREAM_DENSE_REEVAL_INTERVAL: usize = 4096;
const STREAM_DENSE_COVERAGE_DIVISOR: u64 = 20;
const STREAM_DENSE_COVERAGE_MAX_CELLS: u64 = 250_000;

pub fn parse_shared_strings(
    xml: &[u8],
    pool: &mut StringPool,
) -> Result<Vec<StringId>, GridParseError> {
    #[cfg(feature = "custom-xml")]
    {
        parse_shared_strings_custom(xml, pool)
    }
    #[cfg(not(feature = "custom-xml"))]
    {
        parse_shared_strings_quick_xml(xml, pool)
    }
}

#[cfg_attr(feature = "custom-xml", allow(dead_code))]
fn parse_shared_strings_quick_xml(
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

#[cfg(feature = "custom-xml")]
fn parse_shared_strings_custom(
    xml: &[u8],
    pool: &mut StringPool,
) -> Result<Vec<StringId>, GridParseError> {
    fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
        if needle.is_empty() || start >= haystack.len() || needle.len() > haystack.len() {
            return None;
        }
        haystack[start..]
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|idx| start + idx)
    }

    fn find_tag_end(xml: &[u8], mut cursor: usize) -> Option<usize> {
        let mut quote: Option<u8> = None;
        while cursor < xml.len() {
            let b = xml[cursor];
            if let Some(q) = quote {
                if b == q {
                    quote = None;
                }
            } else if b == b'"' || b == b'\'' {
                quote = Some(b);
            } else if b == b'>' {
                return Some(cursor);
            }
            cursor += 1;
        }
        None
    }

    fn xml_offset_err(xml: &[u8], offset: usize, message: impl Into<String>) -> GridParseError {
        let (line, column) = compute_line_col(xml, offset);
        GridParseError::XmlErrorAt {
            line,
            column,
            message: message.into(),
        }
    }

    fn push_utf8_text(raw: &[u8], out: &mut String) -> Result<(), String> {
        if raw.is_empty() {
            return Ok(());
        }
        let text = std::str::from_utf8(raw)
            .map_err(|e| format!("invalid UTF-8 in shared string text: {e}"))?;
        out.push_str(text);
        Ok(())
    }

    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_si = false;
    let mut cursor = 0usize;

    while cursor < xml.len() {
        if xml[cursor] != b'<' {
            cursor += 1;
            continue;
        }

        if xml[cursor..].starts_with(b"<!--") {
            let end = find_bytes(xml, b"-->", cursor + 4).ok_or_else(|| {
                xml_offset_err(xml, cursor, "unterminated XML comment in sharedStrings")
            })?;
            cursor = end + 3;
            continue;
        }

        if xml[cursor..].starts_with(b"<?") {
            let end = find_bytes(xml, b"?>", cursor + 2).ok_or_else(|| {
                xml_offset_err(
                    xml,
                    cursor,
                    "unterminated XML processing instruction in sharedStrings",
                )
            })?;
            cursor = end + 2;
            continue;
        }

        if xml[cursor..].starts_with(b"<![CDATA[") {
            let end = find_bytes(xml, b"]]>", cursor + 9).ok_or_else(|| {
                xml_offset_err(xml, cursor, "unterminated CDATA section in sharedStrings")
            })?;
            cursor = end + 3;
            continue;
        }

        if xml[cursor..].starts_with(b"<!") {
            let end = find_tag_end(xml, cursor + 2).ok_or_else(|| {
                xml_offset_err(xml, cursor, "unterminated declaration in sharedStrings")
            })?;
            cursor = end + 1;
            continue;
        }

        let mut name_cursor = cursor + 1;
        let is_end_tag = if name_cursor < xml.len() && xml[name_cursor] == b'/' {
            name_cursor += 1;
            true
        } else {
            false
        };

        while name_cursor < xml.len() && xml[name_cursor].is_ascii_whitespace() {
            name_cursor += 1;
        }
        let name_start = name_cursor;
        while name_cursor < xml.len() {
            let b = xml[name_cursor];
            if b.is_ascii_whitespace() || b == b'/' || b == b'>' {
                break;
            }
            name_cursor += 1;
        }
        if name_start == name_cursor {
            return Err(xml_offset_err(
                xml,
                cursor,
                "malformed XML tag in sharedStrings",
            ));
        }

        let tag_end = find_tag_end(xml, name_cursor)
            .ok_or_else(|| xml_offset_err(xml, cursor, "unterminated XML tag in sharedStrings"))?;
        let local = local_tag_name(&xml[name_start..name_cursor]);
        let self_closing = if is_end_tag {
            false
        } else {
            let mut probe = tag_end;
            while probe > cursor && xml[probe - 1].is_ascii_whitespace() {
                probe -= 1;
            }
            probe > cursor && xml[probe - 1] == b'/'
        };

        if !is_end_tag && !self_closing && local == b"t" && in_si {
            let content_start = tag_end + 1;
            let mut text_cursor = tag_end + 1;
            loop {
                if text_cursor >= xml.len() {
                    return Err(xml_offset_err(
                        xml,
                        text_cursor,
                        "unexpected EOF while reading <t> text",
                    ));
                }

                if xml[text_cursor] != b'<' {
                    while text_cursor < xml.len() && xml[text_cursor] != b'<' {
                        text_cursor += 1;
                    }
                    continue;
                }

                if xml[text_cursor..].starts_with(b"<![CDATA[") {
                    let cdata_end = find_bytes(xml, b"]]>", text_cursor + 9).ok_or_else(|| {
                        xml_offset_err(xml, text_cursor, "unterminated CDATA in <t> text")
                    })?;
                    text_cursor = cdata_end + 3;
                    continue;
                }

                if xml[text_cursor..].starts_with(b"</") {
                    let mut end_name_cursor = text_cursor + 2;
                    while end_name_cursor < xml.len() && xml[end_name_cursor].is_ascii_whitespace()
                    {
                        end_name_cursor += 1;
                    }
                    let end_name_start = end_name_cursor;
                    while end_name_cursor < xml.len() {
                        let b = xml[end_name_cursor];
                        if b.is_ascii_whitespace() || b == b'>' {
                            break;
                        }
                        end_name_cursor += 1;
                    }
                    if end_name_start == end_name_cursor {
                        return Err(xml_offset_err(
                            xml,
                            text_cursor,
                            "malformed closing tag in <t> text",
                        ));
                    }
                    let end_tag = local_tag_name(&xml[end_name_start..end_name_cursor]);
                    let close_end = find_tag_end(xml, end_name_cursor).ok_or_else(|| {
                        xml_offset_err(xml, text_cursor, "unterminated closing tag in <t> text")
                    })?;
                    if end_tag == b"t" {
                        push_utf8_text(&xml[content_start..text_cursor], &mut current)
                            .map_err(|e| xml_offset_err(xml, content_start, e))?;
                        cursor = close_end + 1;
                        break;
                    }
                    return Err(xml_offset_err(
                        xml,
                        text_cursor,
                        format!(
                            "unexpected closing tag '</{}>' inside <t>",
                            String::from_utf8_lossy(end_tag)
                        ),
                    ));
                }

                return Err(xml_offset_err(
                    xml,
                    text_cursor,
                    "unexpected nested markup inside <t> text",
                ));
            }
            continue;
        }

        if local == b"si" {
            if is_end_tag {
                if in_si {
                    strings.push(pool.intern(&current));
                    in_si = false;
                }
            } else if !self_closing {
                current.clear();
                in_si = true;
            }
        }

        cursor = tag_end + 1;
    }

    if in_si {
        return Err(xml_offset_err(
            xml,
            xml.len(),
            "unexpected EOF while reading <si>",
        ));
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
                    let attr =
                        attr.map_err(|e| xml_msg_err(&reader, workbook_xml, e.to_string()))?;
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
                    let attr =
                        attr.map_err(|e| xml_msg_err(&reader, workbook_xml, e.to_string()))?;
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

pub fn parse_relationship_targets_by_type_contains(
    xml: &[u8],
    needle: &str,
) -> Result<Vec<String>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut targets = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"Relationship" => {
                let mut target = None;
                let mut rel_type = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| xml_msg_err(&reader, xml, e.to_string()))?;
                    match attr.key.as_ref() {
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

                if let (Some(target), Some(rel_type)) = (target, rel_type)
                    && rel_type.contains(needle)
                {
                    targets.push(target);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(xml_err(&reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(targets)
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

pub fn parse_sheet_xml_with_drawing_rids(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<ParsedSheetXml, GridParseError> {
    #[cfg(feature = "custom-xml")]
    {
        parse_sheet_xml_internal_custom(xml, shared_strings, pool, true)
    }
    #[cfg(not(feature = "custom-xml"))]
    {
        parse_sheet_xml_internal_quick_xml(xml, shared_strings, pool, true)
    }
}

pub fn parse_sheet_xml(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
) -> Result<Grid, GridParseError> {
    #[cfg(feature = "custom-xml")]
    {
        Ok(parse_sheet_xml_internal_custom(xml, shared_strings, pool, false)?.grid)
    }
    #[cfg(not(feature = "custom-xml"))]
    {
        Ok(parse_sheet_xml_internal_quick_xml(xml, shared_strings, pool, false)?.grid)
    }
}

#[cfg_attr(feature = "custom-xml", allow(dead_code))]
fn parse_sheet_xml_internal_quick_xml(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
    collect_drawing_rids: bool,
) -> Result<ParsedSheetXml, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut cell_buf = Vec::new();
    let mut value_text_scratch = Vec::new();
    let mut inline_string_scratch = String::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut grid: Option<Grid> = None;
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;
    let mut drawing_rids = Vec::new();
    let mut stream_dense_recheck = 0usize;

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
                            observed_bounds(max_row, max_col),
                        )?;
                        grid = Some(new_grid);
                    }
                }
            }
            Ok(Event::Start(e)) | Ok(Event::Empty(e))
                if collect_drawing_rids && local_tag_name(e.name().as_ref()) == b"drawing" =>
            {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() != b"r:id" {
                        continue;
                    }
                    if let Ok(rid) = attr.unescape_value() {
                        drawing_rids.push(rid.into_owned());
                    }
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"c" => {
                let cell = parse_cell(
                    &mut reader,
                    xml,
                    e,
                    shared_strings,
                    pool,
                    &mut cell_buf,
                    &mut value_text_scratch,
                    &mut inline_string_scratch,
                )?;
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
                        let rebuilt = rebuild_grid(
                            grid.take().unwrap(),
                            nrows,
                            ncols,
                            observed_bounds(max_row, max_col),
                        );
                        grid = Some(rebuilt);
                    }
                    {
                        let existing = grid.as_mut().expect("grid should be initialized");
                        existing.cells.insert(
                            cell.row,
                            cell.col,
                            CellContent {
                                value: cell.value,
                                formula: cell.formula,
                            },
                        );
                    }

                    stream_dense_recheck = stream_dense_recheck.saturating_add(1);
                    if stream_dense_recheck >= STREAM_DENSE_REEVAL_INTERVAL {
                        stream_dense_recheck = 0;
                        let should_promote = {
                            let existing = grid.as_ref().expect("grid should be initialized");
                            matches!(existing.cells, GridStorage::Sparse(_))
                                && prefer_dense_storage(
                                    existing.nrows,
                                    existing.ncols,
                                    existing.cell_count(),
                                    observed_bounds(max_row, max_col),
                                )
                        };
                        if should_promote {
                            let (nrows, ncols) =
                                grid_bounds_from_hint(dimension_hint, max_row, max_col);
                            let rebuilt = rebuild_grid(
                                grid.take().unwrap(),
                                nrows,
                                ncols,
                                observed_bounds(max_row, max_col),
                            );
                            grid = Some(rebuilt);
                        }
                    }
                } else {
                    parsed_cells.push(cell);
                    if dimension_hint.is_some() && parsed_cells.len() >= STREAM_CELL_BUFFER_LIMIT {
                        let (nrows, ncols) =
                            grid_bounds_from_hint(dimension_hint, max_row, max_col);
                        let new_grid = build_grid(
                            nrows,
                            ncols,
                            std::mem::take(&mut parsed_cells),
                            observed_bounds(max_row, max_col),
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
                grid = rebuild_grid(grid, nrows, ncols, observed_bounds(max_row, max_col));
            }
            for cell in parsed_cells {
                grid.cells.insert(
                    cell.row,
                    cell.col,
                    CellContent {
                        value: cell.value,
                        formula: cell.formula,
                    },
                );
            }
            if matches!(grid.cells, GridStorage::Sparse(_))
                && prefer_dense_storage(
                    grid.nrows,
                    grid.ncols,
                    grid.cell_count(),
                    observed_bounds(max_row, max_col),
                )
            {
                let nrows = grid.nrows;
                let ncols = grid.ncols;
                grid = rebuild_grid(grid, nrows, ncols, observed_bounds(max_row, max_col));
            }
        } else {
            let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
            if nrows > grid.nrows || ncols > grid.ncols {
                grid = rebuild_grid(grid, nrows, ncols, observed_bounds(max_row, max_col));
            }
        }
        return Ok(ParsedSheetXml { grid, drawing_rids });
    }

    if parsed_cells.is_empty() {
        return Ok(ParsedSheetXml {
            grid: Grid::new(0, 0),
            drawing_rids,
        });
    }

    let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
    Ok(ParsedSheetXml {
        grid: build_grid(
            nrows,
            ncols,
            parsed_cells,
            observed_bounds(max_row, max_col),
        )?,
        drawing_rids,
    })
}

#[cfg(feature = "custom-xml")]
fn parse_sheet_xml_internal_custom(
    xml: &[u8],
    shared_strings: &[StringId],
    pool: &mut StringPool,
    collect_drawing_rids: bool,
) -> Result<ParsedSheetXml, GridParseError> {
    fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
        if needle.is_empty() || start >= haystack.len() || needle.len() > haystack.len() {
            return None;
        }
        haystack[start..]
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|idx| start + idx)
    }

    fn find_tag_end(xml: &[u8], mut cursor: usize) -> Option<usize> {
        let mut quote: Option<u8> = None;
        while cursor < xml.len() {
            let b = xml[cursor];
            if let Some(q) = quote {
                if b == q {
                    quote = None;
                }
            } else if b == b'"' || b == b'\'' {
                quote = Some(b);
            } else if b == b'>' {
                return Some(cursor);
            }
            cursor += 1;
        }
        None
    }

    fn xml_offset_err(xml: &[u8], offset: usize, message: impl Into<String>) -> GridParseError {
        let (line, column) = compute_line_col(xml, offset);
        GridParseError::XmlErrorAt {
            line,
            column,
            message: message.into(),
        }
    }

    fn find_attr_value<'a>(
        xml: &'a [u8],
        mut cursor: usize,
        end: usize,
        attr_name: &[u8],
    ) -> Option<&'a [u8]> {
        while cursor < end {
            while cursor < end && xml[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= end {
                break;
            }

            let key_start = cursor;
            while cursor < end {
                let b = xml[cursor];
                if b.is_ascii_whitespace() || b == b'=' || b == b'/' || b == b'>' {
                    break;
                }
                cursor += 1;
            }
            if key_start == cursor {
                break;
            }
            let key = &xml[key_start..cursor];

            while cursor < end && xml[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= end || xml[cursor] != b'=' {
                continue;
            }
            cursor += 1;

            while cursor < end && xml[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= end {
                break;
            }

            let quote = xml[cursor];
            if quote != b'"' && quote != b'\'' {
                break;
            }
            cursor += 1;
            let value_start = cursor;
            while cursor < end && xml[cursor] != quote {
                cursor += 1;
            }
            if cursor >= end {
                break;
            }
            let value = &xml[value_start..cursor];
            cursor += 1;

            if key == attr_name {
                return Some(value);
            }
        }
        None
    }

    fn push_utf8_text(
        xml: &[u8],
        offset: usize,
        raw: &[u8],
        out: &mut String,
    ) -> Result<(), GridParseError> {
        if raw.is_empty() {
            return Ok(());
        }
        let text = std::str::from_utf8(raw).map_err(|e| {
            xml_offset_err(
                xml,
                offset,
                format!("invalid UTF-8 in inline string text: {e}"),
            )
        })?;
        out.push_str(text);
        Ok(())
    }

    fn convert_value_bytes_custom(
        value_bytes: Option<&[u8]>,
        cell_type: Option<CellTypeTag>,
        shared_strings: &[StringId],
        pool: &mut StringPool,
        xml: &[u8],
        offset: usize,
    ) -> Result<Option<CellValue>, GridParseError> {
        fn parse_usize_decimal_bytes(raw: &[u8]) -> Option<usize> {
            if raw.is_empty() {
                return None;
            }
            let mut n: usize = 0;
            for &b in raw {
                if !b.is_ascii_digit() {
                    return None;
                }
                let d = (b - b'0') as usize;
                n = n.checked_mul(10)?.checked_add(d)?;
            }
            Some(n)
        }

        fn parse_f64_fast_bytes(raw: &[u8]) -> Option<f64> {
            if raw.is_empty() {
                return None;
            }
            let mut i = 0usize;
            let mut neg = false;
            match raw[0] {
                b'-' => {
                    neg = true;
                    i = 1;
                }
                b'+' => {
                    i = 1;
                }
                _ => {}
            }
            if i >= raw.len() {
                return None;
            }

            let mut int: u64 = 0;
            let mut saw_digit = false;
            while i < raw.len() {
                let b = raw[i];
                if b.is_ascii_digit() {
                    saw_digit = true;
                    let d = (b - b'0') as u64;
                    if int > (u64::MAX - d) / 10 {
                        let s = std::str::from_utf8(raw).ok()?;
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
            if i == raw.len() {
                let v = int as f64;
                return Some(if neg { -v } else { v });
            }
            match raw[i] {
                b'.' | b'e' | b'E' => {
                    let s = std::str::from_utf8(raw).ok()?;
                    s.parse::<f64>().ok()
                }
                _ => None,
            }
        }

        fn trim_ascii_bytes(raw: &[u8]) -> &[u8] {
            if raw.is_empty() {
                return raw;
            }
            let first = raw[0];
            let last = raw[raw.len() - 1];
            if !first.is_ascii_whitespace() && !last.is_ascii_whitespace() {
                return raw;
            }
            let mut start = 0usize;
            let mut end = raw.len();
            while start < end && raw[start].is_ascii_whitespace() {
                start += 1;
            }
            while end > start && raw[end - 1].is_ascii_whitespace() {
                end -= 1;
            }
            &raw[start..end]
        }

        fn utf8<'a>(raw: &'a [u8], xml: &[u8], offset: usize) -> Result<&'a str, GridParseError> {
            std::str::from_utf8(raw).map_err(|e| {
                xml_offset_err(xml, offset, format!("invalid UTF-8 in cell value: {e}"))
            })
        }

        let Some(raw_bytes) = value_bytes else {
            return Ok(None);
        };

        if raw_bytes.is_empty() {
            return Ok(Some(CellValue::Text(pool.intern(""))));
        }

        let trimmed_bytes = trim_ascii_bytes(raw_bytes);
        if trimmed_bytes.is_empty() {
            return Ok(Some(CellValue::Text(pool.intern(""))));
        }

        match cell_type {
            Some(CellTypeTag::SharedString) => {
                let idx = match parse_usize_decimal_bytes(trimmed_bytes) {
                    Some(v) => v,
                    None => {
                        let trimmed = utf8(trimmed_bytes, xml, offset)?;
                        trimmed
                            .parse::<usize>()
                            .map_err(|e| xml_offset_err(xml, offset, e.to_string()))?
                    }
                };
                let text_id = *shared_strings
                    .get(idx)
                    .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
                Ok(Some(CellValue::Text(text_id)))
            }
            Some(CellTypeTag::Bool) => Ok(match trimmed_bytes {
                b"1" => Some(CellValue::Bool(true)),
                b"0" => Some(CellValue::Bool(false)),
                _ => None,
            }),
            Some(CellTypeTag::Error) => {
                if trimmed_bytes.contains(&b'&') {
                    let raw = utf8(trimmed_bytes, xml, offset)?;
                    let unescaped = quick_xml::escape::unescape(raw)
                        .map_err(|e| xml_offset_err(xml, offset, e.to_string()))?;
                    Ok(Some(CellValue::Error(pool.intern(unescaped.as_ref()))))
                } else {
                    let raw = utf8(trimmed_bytes, xml, offset)?;
                    Ok(Some(CellValue::Error(pool.intern(raw))))
                }
            }
            Some(CellTypeTag::FormulaString) | Some(CellTypeTag::InlineString) => {
                if raw_bytes.contains(&b'&') {
                    let raw = utf8(raw_bytes, xml, offset)?;
                    let unescaped = quick_xml::escape::unescape(raw)
                        .map_err(|e| xml_offset_err(xml, offset, e.to_string()))?;
                    Ok(Some(CellValue::Text(pool.intern(unescaped.as_ref()))))
                } else {
                    let raw = utf8(raw_bytes, xml, offset)?;
                    Ok(Some(CellValue::Text(pool.intern(raw))))
                }
            }
            _ => {
                if let Some(n) = parse_f64_fast_bytes(trimmed_bytes) {
                    Ok(Some(CellValue::Number(n)))
                } else if trimmed_bytes.contains(&b'&') {
                    let raw = utf8(trimmed_bytes, xml, offset)?;
                    let unescaped = quick_xml::escape::unescape(raw)
                        .map_err(|e| xml_offset_err(xml, offset, e.to_string()))?;
                    Ok(Some(CellValue::Text(pool.intern(unescaped.as_ref()))))
                } else {
                    let raw = utf8(trimmed_bytes, xml, offset)?;
                    Ok(Some(CellValue::Text(pool.intern(raw))))
                }
            }
        }
    }

    fn read_element_text_bytes_custom<'a>(
        xml: &[u8],
        mut cursor: usize,
        end_tag: &[u8],
        scratch: &'a mut Vec<u8>,
    ) -> Result<(Option<&'a [u8]>, usize), GridParseError> {
        scratch.clear();
        let mut depth: usize = 0;

        loop {
            if cursor >= xml.len() {
                return Err(xml_offset_err(
                    xml,
                    xml.len(),
                    "unexpected EOF while reading element text",
                ));
            }

            let mut lt = cursor;
            while lt < xml.len() && xml[lt] != b'<' {
                lt += 1;
            }
            if lt >= xml.len() {
                return Err(xml_offset_err(
                    xml,
                    xml.len(),
                    "unexpected EOF while reading element text",
                ));
            }

            if lt > cursor {
                scratch.extend_from_slice(&xml[cursor..lt]);
            }

            if xml[lt..].starts_with(b"<![CDATA[") {
                let end = find_bytes(xml, b"]]>", lt + 9).ok_or_else(|| {
                    xml_offset_err(
                        xml,
                        lt,
                        "unterminated CDATA section while reading element text",
                    )
                })?;
                scratch.extend_from_slice(&xml[lt + 9..end]);
                cursor = end + 3;
                continue;
            }

            if xml[lt..].starts_with(b"<!--") {
                let end = find_bytes(xml, b"-->", lt + 4).ok_or_else(|| {
                    xml_offset_err(
                        xml,
                        lt,
                        "unterminated XML comment while reading element text",
                    )
                })?;
                cursor = end + 3;
                continue;
            }

            if xml[lt..].starts_with(b"<?") {
                let end = find_bytes(xml, b"?>", lt + 2).ok_or_else(|| {
                    xml_offset_err(
                        xml,
                        lt,
                        "unterminated XML processing instruction while reading element text",
                    )
                })?;
                cursor = end + 2;
                continue;
            }

            if xml[lt..].starts_with(b"<!") {
                let end = find_tag_end(xml, lt + 2).ok_or_else(|| {
                    xml_offset_err(
                        xml,
                        lt,
                        "unterminated declaration while reading element text",
                    )
                })?;
                cursor = end + 1;
                continue;
            }

            let mut name_cursor = lt + 1;
            let is_end_tag = if name_cursor < xml.len() && xml[name_cursor] == b'/' {
                name_cursor += 1;
                true
            } else {
                false
            };

            while name_cursor < xml.len() && xml[name_cursor].is_ascii_whitespace() {
                name_cursor += 1;
            }
            let name_start = name_cursor;
            while name_cursor < xml.len() {
                let b = xml[name_cursor];
                if b.is_ascii_whitespace() || b == b'/' || b == b'>' {
                    break;
                }
                name_cursor += 1;
            }
            if name_start == name_cursor {
                return Err(xml_offset_err(
                    xml,
                    lt,
                    "malformed XML tag while reading element text",
                ));
            }

            let tag_end_offset = find_tag_end(xml, name_cursor).ok_or_else(|| {
                xml_offset_err(xml, lt, "unterminated XML tag while reading element text")
            })?;
            let raw_name = &xml[name_start..name_cursor];
            let local = local_tag_name(raw_name);
            let self_closing = if is_end_tag {
                false
            } else {
                let mut probe = tag_end_offset;
                while probe > lt && xml[probe - 1].is_ascii_whitespace() {
                    probe -= 1;
                }
                probe > lt && xml[probe - 1] == b'/'
            };

            cursor = tag_end_offset + 1;

            if is_end_tag {
                if depth == 0 && local == end_tag && local == raw_name {
                    break;
                }
                if depth > 0 {
                    depth -= 1;
                } else {
                    return Err(xml_offset_err(
                        xml,
                        lt,
                        format!(
                            "unexpected closing tag '</{}>' while reading element text",
                            String::from_utf8_lossy(local)
                        ),
                    ));
                }
                continue;
            }

            if !self_closing {
                depth += 1;
            }
        }

        Ok((Some(scratch.as_slice()), cursor))
    }

    fn read_inline_string_custom(
        xml: &[u8],
        mut cursor: usize,
        value: &mut String,
    ) -> Result<usize, GridParseError> {
        value.clear();

        while cursor < xml.len() {
            if xml[cursor] != b'<' {
                cursor += 1;
                continue;
            }

            if xml[cursor..].starts_with(b"<!--") {
                let end = find_bytes(xml, b"-->", cursor + 4).ok_or_else(|| {
                    xml_offset_err(xml, cursor, "unterminated XML comment in inline string")
                })?;
                cursor = end + 3;
                continue;
            }

            if xml[cursor..].starts_with(b"<?") {
                let end = find_bytes(xml, b"?>", cursor + 2).ok_or_else(|| {
                    xml_offset_err(
                        xml,
                        cursor,
                        "unterminated XML processing instruction in inline string",
                    )
                })?;
                cursor = end + 2;
                continue;
            }

            if xml[cursor..].starts_with(b"<![CDATA[") {
                let end = find_bytes(xml, b"]]>", cursor + 9).ok_or_else(|| {
                    xml_offset_err(xml, cursor, "unterminated CDATA section in inline string")
                })?;
                cursor = end + 3;
                continue;
            }

            if xml[cursor..].starts_with(b"<!") {
                let end = find_tag_end(xml, cursor + 2).ok_or_else(|| {
                    xml_offset_err(xml, cursor, "unterminated declaration in inline string")
                })?;
                cursor = end + 1;
                continue;
            }

            let mut name_cursor = cursor + 1;
            let is_end_tag = if name_cursor < xml.len() && xml[name_cursor] == b'/' {
                name_cursor += 1;
                true
            } else {
                false
            };

            while name_cursor < xml.len() && xml[name_cursor].is_ascii_whitespace() {
                name_cursor += 1;
            }
            let name_start = name_cursor;
            while name_cursor < xml.len() {
                let b = xml[name_cursor];
                if b.is_ascii_whitespace() || b == b'/' || b == b'>' {
                    break;
                }
                name_cursor += 1;
            }
            if name_start == name_cursor {
                return Err(xml_offset_err(
                    xml,
                    cursor,
                    "malformed XML tag in inline string",
                ));
            }

            let tag_end_offset = find_tag_end(xml, name_cursor).ok_or_else(|| {
                xml_offset_err(xml, cursor, "unterminated XML tag in inline string")
            })?;
            let raw_name = &xml[name_start..name_cursor];
            let local = local_tag_name(raw_name);
            let self_closing = if is_end_tag {
                false
            } else {
                let mut probe = tag_end_offset;
                while probe > cursor && xml[probe - 1].is_ascii_whitespace() {
                    probe -= 1;
                }
                probe > cursor && xml[probe - 1] == b'/'
            };

            if is_end_tag && local == b"is" && local == raw_name {
                return Ok(tag_end_offset + 1);
            }

            if !is_end_tag && !self_closing && local == b"t" && local == raw_name {
                let content_start = tag_end_offset + 1;
                let mut text_cursor = tag_end_offset + 1;
                loop {
                    if text_cursor >= xml.len() {
                        return Err(xml_offset_err(
                            xml,
                            text_cursor,
                            "unexpected EOF while reading <t> text",
                        ));
                    }

                    if xml[text_cursor] != b'<' {
                        while text_cursor < xml.len() && xml[text_cursor] != b'<' {
                            text_cursor += 1;
                        }
                        continue;
                    }

                    if xml[text_cursor..].starts_with(b"<![CDATA[") {
                        let cdata_end =
                            find_bytes(xml, b"]]>", text_cursor + 9).ok_or_else(|| {
                                xml_offset_err(xml, text_cursor, "unterminated CDATA in <t> text")
                            })?;
                        text_cursor = cdata_end + 3;
                        continue;
                    }

                    if xml[text_cursor..].starts_with(b"</") {
                        let mut end_name_cursor = text_cursor + 2;
                        while end_name_cursor < xml.len()
                            && xml[end_name_cursor].is_ascii_whitespace()
                        {
                            end_name_cursor += 1;
                        }
                        let end_name_start = end_name_cursor;
                        while end_name_cursor < xml.len() {
                            let b = xml[end_name_cursor];
                            if b.is_ascii_whitespace() || b == b'>' {
                                break;
                            }
                            end_name_cursor += 1;
                        }
                        if end_name_start == end_name_cursor {
                            return Err(xml_offset_err(
                                xml,
                                text_cursor,
                                "malformed closing tag in <t> text",
                            ));
                        }
                        let end_raw = &xml[end_name_start..end_name_cursor];
                        let end_local = local_tag_name(end_raw);
                        let close_end = find_tag_end(xml, end_name_cursor).ok_or_else(|| {
                            xml_offset_err(xml, text_cursor, "unterminated closing tag in <t> text")
                        })?;
                        if end_local == b"t" && end_local == end_raw {
                            push_utf8_text(
                                xml,
                                content_start,
                                &xml[content_start..text_cursor],
                                value,
                            )?;
                            cursor = close_end + 1;
                            break;
                        }
                        return Err(xml_offset_err(
                            xml,
                            text_cursor,
                            format!(
                                "unexpected closing tag '</{}>' inside <t>",
                                String::from_utf8_lossy(end_local)
                            ),
                        ));
                    }

                    return Err(xml_offset_err(
                        xml,
                        text_cursor,
                        "unexpected nested markup inside <t> text",
                    ));
                }
                continue;
            }

            cursor = tag_end_offset + 1;
        }

        Err(xml_offset_err(
            xml,
            xml.len(),
            "unexpected EOF inside inline string",
        ))
    }

    fn parse_cell_custom(
        xml: &[u8],
        cell_tag_start: usize,
        name_end: usize,
        tag_end: usize,
        shared_strings: &[StringId],
        pool: &mut StringPool,
        value_text_scratch: &mut Vec<u8>,
        inline_string_scratch: &mut String,
    ) -> Result<(ParsedCell, usize), GridParseError> {
        let mut address: Option<(u32, u32)> = None;
        let mut cell_type: Option<CellTypeTag> = None;

        if let Some(raw) = find_attr_value(xml, name_end, tag_end, b"r") {
            if raw.contains(&b'&') {
                let raw_str = std::str::from_utf8(raw).map_err(|e| {
                    xml_offset_err(
                        xml,
                        cell_tag_start,
                        format!("invalid UTF-8 in cell address: {e}"),
                    )
                })?;
                let unescaped = quick_xml::escape::unescape(raw_str)
                    .map_err(|e| xml_offset_err(xml, cell_tag_start, e.to_string()))?;
                address = address_to_index(unescaped.as_ref());
                if address.is_none() {
                    return Err(GridParseError::InvalidAddress(unescaped.into_owned()));
                }
            } else {
                address = address_to_index_ascii_bytes(raw);
                if address.is_none() {
                    return Err(GridParseError::InvalidAddress(
                        String::from_utf8_lossy(raw).into_owned(),
                    ));
                }
            }
        }

        if let Some(raw) = find_attr_value(xml, name_end, tag_end, b"t") {
            cell_type = parse_cell_type_tag_bytes(raw);
            if cell_type.is_none() && raw.contains(&b'&') {
                let raw_str = std::str::from_utf8(raw).map_err(|e| {
                    xml_offset_err(
                        xml,
                        cell_tag_start,
                        format!("invalid UTF-8 in cell type: {e}"),
                    )
                })?;
                let unescaped = quick_xml::escape::unescape(raw_str)
                    .map_err(|e| xml_offset_err(xml, cell_tag_start, e.to_string()))?;
                cell_type = parse_cell_type_tag_str(unescaped.as_ref());
            }
        }

        let (row, col) =
            address.ok_or_else(|| xml_offset_err(xml, cell_tag_start, "cell missing address"))?;

        let mut value: Option<CellValue> = None;
        let mut formula: Option<StringId> = None;
        let mut cursor = tag_end + 1;

        loop {
            if cursor >= xml.len() {
                return Err(xml_offset_err(xml, xml.len(), "unexpected EOF inside cell"));
            }

            if xml[cursor] != b'<' {
                cursor += 1;
                continue;
            }

            if xml[cursor..].starts_with(b"<!--") {
                let end = find_bytes(xml, b"-->", cursor + 4).ok_or_else(|| {
                    xml_offset_err(xml, cursor, "unterminated XML comment inside cell")
                })?;
                cursor = end + 3;
                continue;
            }

            if xml[cursor..].starts_with(b"<?") {
                let end = find_bytes(xml, b"?>", cursor + 2).ok_or_else(|| {
                    xml_offset_err(
                        xml,
                        cursor,
                        "unterminated XML processing instruction inside cell",
                    )
                })?;
                cursor = end + 2;
                continue;
            }

            if xml[cursor..].starts_with(b"<![CDATA[") {
                let end = find_bytes(xml, b"]]>", cursor + 9)
                    .ok_or_else(|| xml_offset_err(xml, cursor, "unterminated CDATA inside cell"))?;
                cursor = end + 3;
                continue;
            }

            if xml[cursor..].starts_with(b"<!") {
                let end = find_tag_end(xml, cursor + 2).ok_or_else(|| {
                    xml_offset_err(xml, cursor, "unterminated declaration inside cell")
                })?;
                cursor = end + 1;
                continue;
            }

            let mut name_cursor = cursor + 1;
            let is_end_tag = if name_cursor < xml.len() && xml[name_cursor] == b'/' {
                name_cursor += 1;
                true
            } else {
                false
            };

            while name_cursor < xml.len() && xml[name_cursor].is_ascii_whitespace() {
                name_cursor += 1;
            }
            let name_start = name_cursor;
            while name_cursor < xml.len() {
                let b = xml[name_cursor];
                if b.is_ascii_whitespace() || b == b'/' || b == b'>' {
                    break;
                }
                name_cursor += 1;
            }
            if name_start == name_cursor {
                return Err(xml_offset_err(xml, cursor, "malformed XML tag inside cell"));
            }

            let tag_end_offset = find_tag_end(xml, name_cursor)
                .ok_or_else(|| xml_offset_err(xml, cursor, "unterminated XML tag inside cell"))?;
            let raw_name = &xml[name_start..name_cursor];
            let local = local_tag_name(raw_name);
            let self_closing = if is_end_tag {
                false
            } else {
                let mut probe = tag_end_offset;
                while probe > cursor && xml[probe - 1].is_ascii_whitespace() {
                    probe -= 1;
                }
                probe > cursor && xml[probe - 1] == b'/'
            };

            if is_end_tag && local == b"c" && local == raw_name {
                cursor = tag_end_offset + 1;
                break;
            }

            if !is_end_tag && !self_closing && local == b"v" && local == raw_name {
                let content_start = tag_end_offset + 1;
                let (raw_opt, next) =
                    read_element_text_bytes_custom(xml, content_start, b"v", value_text_scratch)?;
                value = convert_value_bytes_custom(
                    raw_opt,
                    cell_type,
                    shared_strings,
                    pool,
                    xml,
                    content_start,
                )?;
                cursor = next;
                continue;
            }

            if !is_end_tag && !self_closing && local == b"f" && local == raw_name {
                let content_start = tag_end_offset + 1;
                let (raw_opt, next) =
                    read_element_text_bytes_custom(xml, content_start, b"f", value_text_scratch)?;
                let raw_bytes = raw_opt.unwrap_or(&[]);
                let raw_str = std::str::from_utf8(raw_bytes).map_err(|e| {
                    xml_offset_err(xml, content_start, format!("invalid UTF-8 in formula: {e}"))
                })?;
                let unescaped = quick_xml::escape::unescape(raw_str)
                    .map_err(|e| xml_offset_err(xml, content_start, e.to_string()))?;
                formula = Some(pool.intern(unescaped.as_ref()));
                cursor = next;
                continue;
            }

            if !is_end_tag && !self_closing && local == b"is" && local == raw_name {
                let content_start = tag_end_offset + 1;
                let next = read_inline_string_custom(xml, content_start, inline_string_scratch)?;
                value = Some(CellValue::Text(pool.intern(inline_string_scratch.as_str())));
                cursor = next;
                continue;
            }

            cursor = tag_end_offset + 1;
        }

        Ok((
            ParsedCell {
                row,
                col,
                value,
                formula,
            },
            cursor,
        ))
    }

    let mut value_text_scratch = Vec::new();
    let mut inline_string_scratch = String::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut grid: Option<Grid> = None;
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;
    let mut drawing_rids = Vec::new();
    let mut stream_dense_recheck = 0usize;

    let mut cursor = 0usize;
    while cursor < xml.len() {
        if xml[cursor] != b'<' {
            cursor += 1;
            continue;
        }

        if xml[cursor..].starts_with(b"<!--") {
            let end = find_bytes(xml, b"-->", cursor + 4)
                .ok_or_else(|| xml_offset_err(xml, cursor, "unterminated XML comment"))?;
            cursor = end + 3;
            continue;
        }

        if xml[cursor..].starts_with(b"<?") {
            let end = find_bytes(xml, b"?>", cursor + 2).ok_or_else(|| {
                xml_offset_err(xml, cursor, "unterminated XML processing instruction")
            })?;
            cursor = end + 2;
            continue;
        }

        if xml[cursor..].starts_with(b"<![CDATA[") {
            let end = find_bytes(xml, b"]]>", cursor + 9)
                .ok_or_else(|| xml_offset_err(xml, cursor, "unterminated CDATA section"))?;
            cursor = end + 3;
            continue;
        }

        if xml[cursor..].starts_with(b"<!") {
            let end = find_tag_end(xml, cursor + 2)
                .ok_or_else(|| xml_offset_err(xml, cursor, "unterminated declaration"))?;
            cursor = end + 1;
            continue;
        }

        let mut name_cursor = cursor + 1;
        let is_end_tag = if name_cursor < xml.len() && xml[name_cursor] == b'/' {
            name_cursor += 1;
            true
        } else {
            false
        };

        while name_cursor < xml.len() && xml[name_cursor].is_ascii_whitespace() {
            name_cursor += 1;
        }
        let name_start = name_cursor;
        while name_cursor < xml.len() {
            let b = xml[name_cursor];
            if b.is_ascii_whitespace() || b == b'/' || b == b'>' {
                break;
            }
            name_cursor += 1;
        }
        if name_start == name_cursor {
            return Err(xml_offset_err(xml, cursor, "malformed XML tag"));
        }

        let tag_end_offset = find_tag_end(xml, name_cursor)
            .ok_or_else(|| xml_offset_err(xml, cursor, "unterminated XML tag"))?;
        let raw_name = &xml[name_start..name_cursor];
        let local = local_tag_name(raw_name);
        let self_closing = if is_end_tag {
            false
        } else {
            let mut probe = tag_end_offset;
            while probe > cursor && xml[probe - 1].is_ascii_whitespace() {
                probe -= 1;
            }
            probe > cursor && xml[probe - 1] == b'/'
        };

        if !is_end_tag && local == b"dimension" && local == raw_name {
            if let Some(raw) = find_attr_value(xml, name_cursor, tag_end_offset, b"ref") {
                let raw_str = std::str::from_utf8(raw).map_err(|e| {
                    xml_offset_err(xml, cursor, format!("invalid UTF-8 in dimension: {e}"))
                })?;
                let reference = if raw.contains(&b'&') {
                    let unescaped = quick_xml::escape::unescape(raw_str)
                        .map_err(|e| xml_offset_err(xml, cursor, e.to_string()))?;
                    unescaped.into_owned()
                } else {
                    raw_str.to_string()
                };

                dimension_hint = dimension_from_ref(reference.as_str());
                if grid.is_none()
                    && dimension_hint.is_some()
                    && parsed_cells.len() >= STREAM_CELL_BUFFER_LIMIT
                {
                    let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
                    let new_grid = build_grid(
                        nrows,
                        ncols,
                        std::mem::take(&mut parsed_cells),
                        observed_bounds(max_row, max_col),
                    )?;
                    grid = Some(new_grid);
                }
            }

            cursor = tag_end_offset + 1;
            continue;
        }

        if !is_end_tag && collect_drawing_rids && local == b"drawing" {
            if let Some(raw) = find_attr_value(xml, name_cursor, tag_end_offset, b"r:id") {
                let raw_str = std::str::from_utf8(raw).map_err(|e| {
                    xml_offset_err(xml, cursor, format!("invalid UTF-8 in r:id: {e}"))
                })?;
                let rid = if raw.contains(&b'&') {
                    let unescaped = quick_xml::escape::unescape(raw_str)
                        .map_err(|e| xml_offset_err(xml, cursor, e.to_string()))?;
                    unescaped.into_owned()
                } else {
                    raw_str.to_string()
                };
                drawing_rids.push(rid);
            }

            cursor = tag_end_offset + 1;
            continue;
        }

        if !is_end_tag && !self_closing && local == b"c" && local == raw_name {
            let (cell, next_cursor) = parse_cell_custom(
                xml,
                cursor,
                name_cursor,
                tag_end_offset,
                shared_strings,
                pool,
                &mut value_text_scratch,
                &mut inline_string_scratch,
            )?;

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
                    let rebuilt = rebuild_grid(
                        grid.take().unwrap(),
                        nrows,
                        ncols,
                        observed_bounds(max_row, max_col),
                    );
                    grid = Some(rebuilt);
                }
                {
                    let existing = grid.as_mut().expect("grid should be initialized");
                    existing.cells.insert(
                        cell.row,
                        cell.col,
                        CellContent {
                            value: cell.value,
                            formula: cell.formula,
                        },
                    );
                }

                stream_dense_recheck = stream_dense_recheck.saturating_add(1);
                if stream_dense_recheck >= STREAM_DENSE_REEVAL_INTERVAL {
                    stream_dense_recheck = 0;
                    let should_promote = {
                        let existing = grid.as_ref().expect("grid should be initialized");
                        matches!(existing.cells, GridStorage::Sparse(_))
                            && prefer_dense_storage(
                                existing.nrows,
                                existing.ncols,
                                existing.cell_count(),
                                observed_bounds(max_row, max_col),
                            )
                    };
                    if should_promote {
                        let (nrows, ncols) =
                            grid_bounds_from_hint(dimension_hint, max_row, max_col);
                        let rebuilt = rebuild_grid(
                            grid.take().unwrap(),
                            nrows,
                            ncols,
                            observed_bounds(max_row, max_col),
                        );
                        grid = Some(rebuilt);
                    }
                }
            } else {
                parsed_cells.push(cell);
                if dimension_hint.is_some() && parsed_cells.len() >= STREAM_CELL_BUFFER_LIMIT {
                    let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
                    let new_grid = build_grid(
                        nrows,
                        ncols,
                        std::mem::take(&mut parsed_cells),
                        observed_bounds(max_row, max_col),
                    )?;
                    grid = Some(new_grid);
                }
            }

            cursor = next_cursor;
            continue;
        }

        cursor = tag_end_offset + 1;
    }

    if let Some(mut grid) = grid {
        if !parsed_cells.is_empty() {
            let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
            if nrows > grid.nrows || ncols > grid.ncols {
                grid = rebuild_grid(grid, nrows, ncols, observed_bounds(max_row, max_col));
            }
            for cell in parsed_cells {
                grid.cells.insert(
                    cell.row,
                    cell.col,
                    CellContent {
                        value: cell.value,
                        formula: cell.formula,
                    },
                );
            }
            if matches!(grid.cells, GridStorage::Sparse(_))
                && prefer_dense_storage(
                    grid.nrows,
                    grid.ncols,
                    grid.cell_count(),
                    observed_bounds(max_row, max_col),
                )
            {
                let nrows = grid.nrows;
                let ncols = grid.ncols;
                grid = rebuild_grid(grid, nrows, ncols, observed_bounds(max_row, max_col));
            }
        } else {
            let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
            if nrows > grid.nrows || ncols > grid.ncols {
                grid = rebuild_grid(grid, nrows, ncols, observed_bounds(max_row, max_col));
            }
        }
        return Ok(ParsedSheetXml { grid, drawing_rids });
    }

    if parsed_cells.is_empty() {
        return Ok(ParsedSheetXml {
            grid: Grid::new(0, 0),
            drawing_rids,
        });
    }

    let (nrows, ncols) = grid_bounds_from_hint(dimension_hint, max_row, max_col);
    Ok(ParsedSheetXml {
        grid: build_grid(
            nrows,
            ncols,
            parsed_cells,
            observed_bounds(max_row, max_col),
        )?,
        drawing_rids,
    })
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    start: BytesStart,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    buf: &mut Vec<u8>,
    value_text_scratch: &mut Vec<u8>,
    inline_string_scratch: &mut String,
) -> Result<ParsedCell, GridParseError> {
    let mut address = None;
    let mut cell_type: Option<CellTypeTag> = None;

    for attr in start.attributes() {
        let attr = attr.map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
        match attr.key.as_ref() {
            b"r" => {
                let raw = attr.value.as_ref();
                if raw.contains(&b'&') {
                    let unescaped = attr.unescape_value().map_err(|e| xml_err(reader, xml, e))?;
                    address = address_to_index(unescaped.as_ref());
                    if address.is_none() {
                        return Err(GridParseError::InvalidAddress(unescaped.into_owned()));
                    }
                } else {
                    address = address_to_index_ascii_bytes(raw);
                    if address.is_none() {
                        return Err(GridParseError::InvalidAddress(
                            String::from_utf8_lossy(raw).into_owned(),
                        ));
                    }
                }
            }
            b"t" => {
                let raw = attr.value.as_ref();
                cell_type = parse_cell_type_tag_bytes(raw);
                if cell_type.is_none() && raw.contains(&b'&') {
                    let unescaped = attr.unescape_value().map_err(|e| xml_err(reader, xml, e))?;
                    cell_type = parse_cell_type_tag_str(unescaped.as_ref());
                }
            }
            _ => {}
        }
    }

    let (row, col) = address.ok_or_else(|| xml_msg_err(reader, xml, "cell missing address"))?;

    let mut value: Option<CellValue> = None;
    let mut formula: Option<StringId> = None;

    buf.clear();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let raw = read_element_text_bytes(reader, xml, b"v", buf, value_text_scratch)?;
                value = convert_value_bytes(raw, cell_type, shared_strings, pool, reader, xml)?;
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
                read_inline_string(reader, xml, buf, inline_string_scratch)?;
                value = Some(CellValue::Text(pool.intern(inline_string_scratch.as_str())));
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

fn read_inline_string(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    buf: &mut Vec<u8>,
    value: &mut String,
) -> Result<(), GridParseError> {
    value.clear();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| xml_err(reader, xml, e))?;
                value.push_str(text.as_ref());
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
    Ok(())
}

fn read_element_text_bytes<'a>(
    reader: &mut Reader<&[u8]>,
    xml: &[u8],
    end_tag: &[u8],
    buf: &mut Vec<u8>,
    scratch: &'a mut Vec<u8>,
) -> Result<Option<&'a [u8]>, GridParseError> {
    scratch.clear();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Text(t)) => {
                scratch.extend_from_slice(t.as_ref());
            }
            Ok(Event::CData(t)) => {
                scratch.extend_from_slice(t.as_ref());
            }
            Ok(Event::End(e)) if e.name().as_ref() == end_tag => break,
            Ok(Event::Eof) => {
                return Err(xml_msg_err(
                    reader,
                    xml,
                    "unexpected EOF while reading element text",
                ));
            }
            Err(e) => return Err(xml_err(reader, xml, e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(Some(scratch.as_slice()))
}

fn address_to_index_ascii_bytes(a1: &[u8]) -> Option<(u32, u32)> {
    if a1.is_empty() {
        return None;
    }

    let mut i: usize = 0;
    let mut col: u32 = 0;
    while i < a1.len() {
        let b = a1[i];
        if !b.is_ascii_alphabetic() {
            break;
        }
        let upper = b.to_ascii_uppercase();
        col = col
            .checked_mul(26)?
            .checked_add((upper - b'A' + 1) as u32)?;

        i += 1;
    }

    if i == 0 || i >= a1.len() || col == 0 {
        return None;
    }

    let mut row: u32 = 0;
    while i < a1.len() {
        let b = a1[i];
        if !b.is_ascii_digit() {
            return None;
        }
        row = row.checked_mul(10)?.checked_add((b - b'0') as u32)?;
        i += 1;
    }

    if row == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CellTypeTag {
    SharedString,
    Bool,
    Error,
    FormulaString,
    InlineString,
}

fn parse_cell_type_tag_bytes(raw: &[u8]) -> Option<CellTypeTag> {
    match raw {
        b"s" => Some(CellTypeTag::SharedString),
        b"b" => Some(CellTypeTag::Bool),
        b"e" => Some(CellTypeTag::Error),
        b"str" => Some(CellTypeTag::FormulaString),
        b"inlineStr" => Some(CellTypeTag::InlineString),
        _ => None,
    }
}

fn parse_cell_type_tag_str(raw: &str) -> Option<CellTypeTag> {
    match raw {
        "s" => Some(CellTypeTag::SharedString),
        "b" => Some(CellTypeTag::Bool),
        "e" => Some(CellTypeTag::Error),
        "str" => Some(CellTypeTag::FormulaString),
        "inlineStr" => Some(CellTypeTag::InlineString),
        _ => None,
    }
}

#[cfg(test)]
fn trim_cell_text(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    if bytes.is_empty() {
        return raw;
    }

    let first = bytes[0];
    let last = bytes[bytes.len() - 1];

    if first >= 0x80 || last >= 0x80 {
        return raw.trim();
    }

    if !first.is_ascii_whitespace() && !last.is_ascii_whitespace() {
        return raw;
    }

    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    // SAFETY: start/end are advanced only across ASCII bytes, so UTF-8 boundaries are preserved.
    unsafe { std::str::from_utf8_unchecked(&bytes[start..end]) }
}

#[cfg(test)]
fn convert_value(
    value_text: Option<&str>,
    cell_type: Option<CellTypeTag>,
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
                    return lexical_core::parse::<f64>(s.as_bytes()).ok();
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
            b'.' | b'e' | b'E' => lexical_core::parse::<f64>(s.as_bytes()).ok(),
            _ => None,
        }
    }

    let raw = match value_text {
        Some(t) => t,
        None => return Ok(None),
    };

    let trimmed = trim_cell_text(raw);
    if raw.is_empty() || trimmed.is_empty() {
        return Ok(Some(CellValue::Text(pool.intern(""))));
    }

    match cell_type {
        Some(CellTypeTag::SharedString) => {
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
        Some(CellTypeTag::Bool) => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some(CellTypeTag::Error) => Ok(Some(CellValue::Error(pool.intern(trimmed)))),
        Some(CellTypeTag::FormulaString) | Some(CellTypeTag::InlineString) => {
            Ok(Some(CellValue::Text(pool.intern(raw))))
        }
        _ => {
            if let Some(n) = parse_f64_fast(trimmed) {
                Ok(Some(CellValue::Number(n)))
            } else {
                Ok(Some(CellValue::Text(pool.intern(trimmed))))
            }
        }
    }
}

fn convert_value_bytes(
    value_bytes: Option<&[u8]>,
    cell_type: Option<CellTypeTag>,
    shared_strings: &[StringId],
    pool: &mut StringPool,
    reader: &Reader<&[u8]>,
    xml: &[u8],
) -> Result<Option<CellValue>, GridParseError> {
    fn parse_usize_decimal_bytes(raw: &[u8]) -> Option<usize> {
        if raw.is_empty() {
            return None;
        }
        let mut n: usize = 0;
        for &b in raw {
            if !b.is_ascii_digit() {
                return None;
            }
            let d = (b - b'0') as usize;
            n = n.checked_mul(10)?.checked_add(d)?;
        }
        Some(n)
    }

    fn parse_f64_fast_bytes(raw: &[u8]) -> Option<f64> {
        if raw.is_empty() {
            return None;
        }
        let mut i = 0usize;
        let mut neg = false;
        match raw[0] {
            b'-' => {
                neg = true;
                i = 1;
            }
            b'+' => {
                i = 1;
            }
            _ => {}
        }
        if i >= raw.len() {
            return None;
        }

        let mut int: u64 = 0;
        let mut saw_digit = false;
        while i < raw.len() {
            let b = raw[i];
            if b.is_ascii_digit() {
                saw_digit = true;
                let d = (b - b'0') as u64;
                if int > (u64::MAX - d) / 10 {
                    return lexical_core::parse::<f64>(raw).ok();
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
        if i == raw.len() {
            let v = int as f64;
            return Some(if neg { -v } else { v });
        }
        match raw[i] {
            b'.' | b'e' | b'E' => lexical_core::parse::<f64>(raw).ok(),
            _ => None,
        }
    }

    fn trim_ascii_bytes(raw: &[u8]) -> &[u8] {
        if raw.is_empty() {
            return raw;
        }
        let first = raw[0];
        let last = raw[raw.len() - 1];
        if !first.is_ascii_whitespace() && !last.is_ascii_whitespace() {
            return raw;
        }
        let mut start = 0usize;
        let mut end = raw.len();
        while start < end && raw[start].is_ascii_whitespace() {
            start += 1;
        }
        while end > start && raw[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        &raw[start..end]
    }

    fn utf8<'a>(
        raw: &'a [u8],
        reader: &Reader<&[u8]>,
        xml: &[u8],
    ) -> Result<&'a str, GridParseError> {
        std::str::from_utf8(raw)
            .map_err(|e| xml_msg_err(reader, xml, format!("invalid UTF-8 in cell value: {e}")))
    }

    let Some(raw_bytes) = value_bytes else {
        return Ok(None);
    };

    if raw_bytes.is_empty() {
        return Ok(Some(CellValue::Text(pool.intern(""))));
    }

    let trimmed_bytes = trim_ascii_bytes(raw_bytes);
    if trimmed_bytes.is_empty() {
        return Ok(Some(CellValue::Text(pool.intern(""))));
    }

    match cell_type {
        Some(CellTypeTag::SharedString) => {
            let idx = match parse_usize_decimal_bytes(trimmed_bytes) {
                Some(v) => v,
                None => {
                    let trimmed = utf8(trimmed_bytes, reader, xml)?;
                    trimmed
                        .parse::<usize>()
                        .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?
                }
            };
            let text_id = *shared_strings
                .get(idx)
                .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
            Ok(Some(CellValue::Text(text_id)))
        }
        Some(CellTypeTag::Bool) => Ok(match trimmed_bytes {
            b"1" => Some(CellValue::Bool(true)),
            b"0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some(CellTypeTag::Error) => {
            if trimmed_bytes.contains(&b'&') {
                let raw = utf8(trimmed_bytes, reader, xml)?;
                let unescaped = quick_xml::escape::unescape(raw)
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
                Ok(Some(CellValue::Error(pool.intern(unescaped.as_ref()))))
            } else {
                let raw = utf8(trimmed_bytes, reader, xml)?;
                Ok(Some(CellValue::Error(pool.intern(raw))))
            }
        }
        Some(CellTypeTag::FormulaString) | Some(CellTypeTag::InlineString) => {
            if raw_bytes.contains(&b'&') {
                let raw = utf8(raw_bytes, reader, xml)?;
                let unescaped = quick_xml::escape::unescape(raw)
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
                Ok(Some(CellValue::Text(pool.intern(unescaped.as_ref()))))
            } else {
                let raw = utf8(raw_bytes, reader, xml)?;
                Ok(Some(CellValue::Text(pool.intern(raw))))
            }
        }
        _ => {
            if let Some(n) = parse_f64_fast_bytes(trimmed_bytes) {
                Ok(Some(CellValue::Number(n)))
            } else if trimmed_bytes.contains(&b'&') {
                let raw = utf8(trimmed_bytes, reader, xml)?;
                let unescaped = quick_xml::escape::unescape(raw)
                    .map_err(|e| xml_msg_err(reader, xml, e.to_string()))?;
                Ok(Some(CellValue::Text(pool.intern(unescaped.as_ref()))))
            } else {
                let raw = utf8(trimmed_bytes, reader, xml)?;
                Ok(Some(CellValue::Text(pool.intern(raw))))
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

fn observed_bounds(max_row: Option<u32>, max_col: Option<u32>) -> Option<(u32, u32)> {
    match (max_row, max_col) {
        (Some(row), Some(col)) => Some((row, col)),
        _ => None,
    }
}

fn dense_coverage_required(total_cells: u64) -> u64 {
    if total_cells == 0 {
        return 0;
    }
    let by_ratio = total_cells / STREAM_DENSE_COVERAGE_DIVISOR;
    let bounded = by_ratio.max(STREAM_CELL_BUFFER_LIMIT as u64);
    bounded
        .min(STREAM_DENSE_COVERAGE_MAX_CELLS)
        .min(total_cells)
}

fn prefer_dense_storage(
    nrows: u32,
    ncols: u32,
    filled_cells: usize,
    observed: Option<(u32, u32)>,
) -> bool {
    if Grid::should_use_dense(nrows, ncols, filled_cells) {
        return true;
    }

    let Some((max_row, max_col)) = observed else {
        return false;
    };
    let observed_rows = max_row.saturating_add(1);
    let observed_cols = max_col.saturating_add(1);
    let observed_cells = (observed_rows as u64).saturating_mul(observed_cols as u64);
    let total_cells = (nrows as u64).saturating_mul(ncols as u64);
    if total_cells == 0 {
        return false;
    }
    if observed_cells < dense_coverage_required(total_cells) {
        return false;
    }
    Grid::should_use_dense(observed_rows, observed_cols, filled_cells)
}

fn build_grid(
    nrows: u32,
    ncols: u32,
    cells: Vec<ParsedCell>,
    observed: Option<(u32, u32)>,
) -> Result<Grid, GridParseError> {
    let filled = cells.len();
    let mut grid = if prefer_dense_storage(nrows, ncols, filled, observed) {
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

fn local_tag_name(name: &[u8]) -> &[u8] {
    name.rsplit(|&b| b == b':').next().unwrap_or(name)
}

fn rebuild_grid(grid: Grid, nrows: u32, ncols: u32, observed: Option<(u32, u32)>) -> Grid {
    let filled = grid.cell_count();
    let target_dense = prefer_dense_storage(nrows, ncols, filled, observed);
    if grid.nrows == nrows && grid.ncols == ncols {
        let already_dense = matches!(grid.cells, GridStorage::Dense(_));
        if already_dense == target_dense {
            return grid;
        }
    }

    let mut rebuilt = if target_dense {
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
            let v = attr.unescape_value().map_err(|e| xml_err(reader, xml, e))?;
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
    use super::{
        address_to_index_ascii_bytes, convert_value, convert_value_bytes, dense_coverage_required,
        parse_shared_strings, parse_sheet_xml_with_drawing_rids, prefer_dense_storage,
        read_inline_string, CellTypeTag, GridParseError,
    };
    #[cfg(feature = "custom-xml")]
    use super::{
        parse_shared_strings_custom, parse_shared_strings_quick_xml,
        parse_sheet_xml_internal_custom, parse_sheet_xml_internal_quick_xml,
    };
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

    #[cfg(feature = "custom-xml")]
    #[test]
    fn parse_shared_strings_custom_matches_quick_xml_for_entities_and_cdata() {
        let xml = br#"<?xml version="1.0"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <si><t>plain</t></si>
  <si>
    <r><t>A&amp;B</t></r>
    <r><t>&#x20;C</t></r>
  </si>
  <si><t><![CDATA[<raw>]]></t></si>
</sst>"#;

        let mut quick_pool = StringPool::new();
        let quick = parse_shared_strings_quick_xml(xml, &mut quick_pool).expect("quick xml parser");
        let quick_values: Vec<String> = quick
            .iter()
            .map(|&id| quick_pool.resolve(id).to_string())
            .collect();

        let mut custom_pool = StringPool::new();
        let custom = parse_shared_strings_custom(xml, &mut custom_pool).expect("custom xml parser");
        let custom_values: Vec<String> = custom
            .iter()
            .map(|&id| custom_pool.resolve(id).to_string())
            .collect();

        assert_eq!(custom_values, quick_values);
    }

    #[cfg(feature = "custom-xml")]
    #[test]
    fn parse_shared_strings_custom_matches_quick_xml_on_invalid_entity_text() {
        let xml = br#"<sst><si><t>bad &bogus; entity</t></si></sst>"#;
        let mut quick_pool = StringPool::new();
        let quick = parse_shared_strings_quick_xml(xml, &mut quick_pool);

        let mut custom_pool = StringPool::new();
        let custom = parse_shared_strings_custom(xml, &mut custom_pool);

        match (quick, custom) {
            (Ok(quick_ids), Ok(custom_ids)) => {
                let quick_values: Vec<String> = quick_ids
                    .iter()
                    .map(|&id| quick_pool.resolve(id).to_string())
                    .collect();
                let custom_values: Vec<String> = custom_ids
                    .iter()
                    .map(|&id| custom_pool.resolve(id).to_string())
                    .collect();
                assert_eq!(custom_values, quick_values);
            }
            (Err(quick_err), Err(custom_err)) => {
                assert_eq!(quick_err.code(), custom_err.code());
            }
            (quick_state, custom_state) => {
                panic!(
                    "custom and quick parsers diverged on invalid entity input: quick={quick_state:?}, custom={custom_state:?}"
                );
            }
        }
    }

    #[test]
    fn address_to_index_ascii_bytes_parses_common_addresses() {
        assert_eq!(address_to_index_ascii_bytes(b"A1"), Some((0, 0)));
        assert_eq!(address_to_index_ascii_bytes(b"Z1"), Some((0, 25)));
        assert_eq!(address_to_index_ascii_bytes(b"AA10"), Some((9, 26)));
        assert_eq!(
            address_to_index_ascii_bytes(b"XFD1048576"),
            Some((1_048_575, 16_383))
        );
    }

    #[test]
    fn address_to_index_ascii_bytes_rejects_invalid_addresses() {
        assert_eq!(address_to_index_ascii_bytes(b""), None);
        assert_eq!(address_to_index_ascii_bytes(b"A"), None);
        assert_eq!(address_to_index_ascii_bytes(b"A0"), None);
        assert_eq!(address_to_index_ascii_bytes(b"1A"), None);
        assert_eq!(address_to_index_ascii_bytes(b"A1A"), None);
    }

    #[test]
    fn read_inline_string_preserves_xml_space_preserve() {
        let xml = br#"<is><t xml:space="preserve"> hello</t></is>"#;
        let mut reader = Reader::from_reader(xml.as_ref());
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut value = String::new();
        read_inline_string(&mut reader, xml, &mut buf, &mut value)
            .expect("inline string should parse");
        assert_eq!(value, " hello");

        let mut pool = StringPool::new();
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);
        let converted = convert_value(
            Some(value.as_str()),
            Some(CellTypeTag::InlineString),
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
        let false_val = convert_value(
            Some("0"),
            Some(CellTypeTag::Bool),
            &[],
            &mut pool,
            &dummy_reader,
            dummy_xml,
        )
        .expect("bool cell conversion should succeed");
        assert_eq!(false_val, Some(CellValue::Bool(false)));

        let mut pool = StringPool::new();
        let true_val = convert_value(
            Some("1"),
            Some(CellTypeTag::Bool),
            &[],
            &mut pool,
            &dummy_reader,
            dummy_xml,
        )
        .expect("bool cell conversion should succeed");
        assert_eq!(true_val, Some(CellValue::Bool(true)));

        let none_val = convert_value(
            Some("2"),
            Some(CellTypeTag::Bool),
            &[],
            &mut pool,
            &dummy_reader,
            dummy_xml,
        )
        .expect("unexpected bool tokens should still parse");
        assert!(none_val.is_none());
    }

    #[test]
    fn convert_value_shared_string_index_out_of_bounds_errors() {
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);

        let mut pool = StringPool::new();
        let only_id = pool.intern("only");
        let err = convert_value(
            Some("5"),
            Some(CellTypeTag::SharedString),
            &[only_id],
            &mut pool,
            &dummy_reader,
            dummy_xml,
        )
        .expect_err("invalid shared string index should error");
        assert!(matches!(err, GridParseError::SharedStringOutOfBounds(5)));
    }

    #[test]
    fn convert_value_error_cell_as_text() {
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);

        let mut pool = StringPool::new();
        let value = convert_value(
            Some("#DIV/0!"),
            Some(CellTypeTag::Error),
            &[],
            &mut pool,
            &dummy_reader,
            dummy_xml,
        )
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

    #[test]
    fn convert_value_bytes_parses_decimal_and_exponent_numbers() {
        let dummy_xml: &[u8] = b"";
        let dummy_reader = Reader::from_reader(dummy_xml);

        let cases: [&[u8]; 6] = [
            b"42",
            b"3.14159",
            b"-0.125",
            b"1e3",
            b"1.23e-4",
            // Triggers the integer fast-path overflow fallback.
            b"18446744073709551616",
        ];

        let mut pool = StringPool::new();
        for raw in cases {
            let raw_str = std::str::from_utf8(raw).expect("test cases should be UTF-8");
            let expected = raw_str
                .parse::<f64>()
                .expect("test cases should be valid f64");
            let value =
                convert_value_bytes(Some(raw), None, &[], &mut pool, &dummy_reader, dummy_xml)
                    .expect("conversion should succeed")
                    .expect("value should be present");

            match value {
                CellValue::Number(actual) => {
                    assert_eq!(
                        actual.to_bits(),
                        expected.to_bits(),
                        "float parse mismatch for {raw_str}"
                    );
                }
                other => panic!("expected number for {raw_str}, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_sheet_xml_with_drawing_rids_captures_rids_and_grid() {
        let xml = br#"<worksheet xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <dimension ref="A1"/>
  <drawing r:id="rId1"/>
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr">
        <is><t>hello</t></is>
      </c>
    </row>
  </sheetData>
  <drawing r:id="rId7"/>
</worksheet>"#;

        let mut pool = StringPool::new();
        let parsed =
            parse_sheet_xml_with_drawing_rids(xml, &[], &mut pool).expect("sheet xml should parse");

        assert_eq!(
            parsed.drawing_rids,
            vec!["rId1".to_string(), "rId7".to_string()]
        );
        let text_id = parsed
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref())
            .and_then(CellValue::as_text_id)
            .expect("A1 text value should be present");
        assert_eq!(pool.resolve(text_id), "hello");
    }

    #[cfg(feature = "custom-xml")]
    #[test]
    fn parse_sheet_xml_custom_matches_quick_xml_for_common_cell_types() {
        use std::collections::BTreeSet;

        #[derive(Debug, PartialEq)]
        enum NormVal {
            Blank,
            Number(u64),
            Bool(bool),
            Text(String),
            Error(String),
        }

        fn norm_value(value: &Option<CellValue>, pool: &StringPool) -> Option<NormVal> {
            match value {
                None => None,
                Some(CellValue::Blank) => Some(NormVal::Blank),
                Some(CellValue::Number(n)) => Some(NormVal::Number(n.to_bits())),
                Some(CellValue::Bool(b)) => Some(NormVal::Bool(*b)),
                Some(CellValue::Text(id)) => Some(NormVal::Text(pool.resolve(*id).to_string())),
                Some(CellValue::Error(id)) => Some(NormVal::Error(pool.resolve(*id).to_string())),
            }
        }

        fn norm_formula(
            formula: &Option<crate::string_pool::StringId>,
            pool: &StringPool,
        ) -> Option<String> {
            formula.map(|id| pool.resolve(id).to_string())
        }

        let xml = br#"<?xml version="1.0"?>
<worksheet xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
          xmlns:x="http://example.com">
  <dimension ref="A1:H1"/>
  <x:drawing r:id="rId9"/>
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
      <c r="B1"><v><![CDATA[42]]></v></c>
      <c r="C1" t="b"><v>1</v></c>
      <c r="D1" t="e"><v>#DIV/0!</v></c>
      <c r="E1" t="inlineStr"><is><t xml:space="preserve"> hi</t></is></c>
      <c r="F1"><f>"foo"&amp;"bar"</f><v>5</v></c>
      <c r="G1" t="inlineStr"><is><t><![CDATA[<raw>]]></t></is></c>
      <c r="H1"><v>A&amp;B</v></c>
    </row>
  </sheetData>
  <drawing r:id="rId1"/>
</worksheet>"#;

        let mut quick_pool = StringPool::new();
        let quick_shared_strings = vec![quick_pool.intern("Hello")];
        let quick =
            parse_sheet_xml_internal_quick_xml(xml, &quick_shared_strings, &mut quick_pool, true)
                .expect("quick sheet parser should succeed");

        let mut custom_pool = StringPool::new();
        let custom_shared_strings = vec![custom_pool.intern("Hello")];
        let custom =
            parse_sheet_xml_internal_custom(xml, &custom_shared_strings, &mut custom_pool, true)
                .expect("custom sheet parser should succeed");

        assert_eq!(custom.drawing_rids, quick.drawing_rids);
        assert_eq!(custom.grid.nrows, quick.grid.nrows);
        assert_eq!(custom.grid.ncols, quick.grid.ncols);

        let mut coords = BTreeSet::new();
        for ((row, col), _) in quick.grid.iter_cells() {
            coords.insert((row, col));
        }
        for ((row, col), _) in custom.grid.iter_cells() {
            coords.insert((row, col));
        }

        for (row, col) in coords {
            let quick_cell = quick.grid.get(row, col);
            let custom_cell = custom.grid.get(row, col);

            let quick_val = quick_cell.and_then(|cell| norm_value(&cell.value, &quick_pool));
            let custom_val = custom_cell.and_then(|cell| norm_value(&cell.value, &custom_pool));
            assert_eq!(
                custom_val, quick_val,
                "value mismatch at ({row},{col}): quick={quick_val:?} custom={custom_val:?}"
            );

            let quick_formula =
                quick_cell.and_then(|cell| norm_formula(&cell.formula, &quick_pool));
            let custom_formula =
                custom_cell.and_then(|cell| norm_formula(&cell.formula, &custom_pool));
            assert_eq!(
                custom_formula, quick_formula,
                "formula mismatch at ({row},{col}): quick={quick_formula:?} custom={custom_formula:?}"
            );
        }
    }

    #[test]
    fn dense_coverage_required_is_bounded() {
        assert_eq!(dense_coverage_required(0), 0);
        assert_eq!(dense_coverage_required(3_000), 3_000);
        assert_eq!(dense_coverage_required(5_000_000), 250_000);
    }

    #[test]
    fn prefer_dense_storage_promotes_large_dense_sheet_early() {
        let nrows = 50_000;
        let ncols = 100;
        let filled_cells = 250_000;
        let observed = Some((2_499, 99));
        assert!(
            prefer_dense_storage(nrows, ncols, filled_cells, observed),
            "observed dense window should trigger early dense storage"
        );
    }

    #[test]
    fn prefer_dense_storage_keeps_sparse_for_large_low_density_sheet() {
        let nrows = 50_000;
        let ncols = 100;
        let filled_cells = 50_000;
        let observed = Some((49_999, 99));
        assert!(
            !prefer_dense_storage(nrows, ncols, filled_cells, observed),
            "low observed density should remain sparse"
        );
    }
}
