use crate::addressing::address_to_index;
use crate::workbook::{Cell, CellAddress, CellValue, Grid};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GridParseError {
    #[error("XML parse error: {0}")]
    XmlError(String),
    #[error("invalid cell address: {0}")]
    InvalidAddress(String),
    #[error("shared string index {0} out of bounds")]
    SharedStringOutOfBounds(usize),
}

pub struct SheetDescriptor {
    pub name: String,
    pub rel_id: Option<String>,
    pub sheet_id: Option<u32>,
}

pub fn parse_shared_strings(xml: &[u8]) -> Result<Vec<String>, GridParseError> {
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
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                current.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"si" => {
                strings.push(current.clone());
                in_si = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
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
                    let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
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
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(sheets)
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
                    let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
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
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
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

pub fn parse_sheet_xml(xml: &[u8], shared_strings: &[String]) -> Result<Grid, GridParseError> {
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
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    if parsed_cells.is_empty() {
        return Ok(Grid::new(0, 0));
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
) -> Result<ParsedCell, GridParseError> {
    let address_raw = get_attr_value(&start, b"r")?
        .ok_or_else(|| GridParseError::XmlError("cell missing address".into()))?;
    let (row, col) = address_to_index(&address_raw)
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.clone()))?;

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
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                value_text = Some(text);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                let unescaped = quick_xml::escape::unescape(&text)
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                formula_text = Some(unescaped);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                inline_text = Some(read_inline_string(reader)?);
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(GridParseError::XmlError(
                    "unexpected EOF inside cell".into(),
                ));
            }
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
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

fn read_inline_string(reader: &mut Reader<&[u8]>) -> Result<String, GridParseError> {
    let mut buf = Vec::new();
    let mut value = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                value.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"is" => break,
            Ok(Event::Eof) => {
                return Err(GridParseError::XmlError(
                    "unexpected EOF inside inline string".into(),
                ));
            }
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
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
) -> Result<Option<CellValue>, GridParseError> {
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
                .map_err(|e| GridParseError::XmlError(e.to_string()))?;
            let text = shared_strings
                .get(idx)
                .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
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

fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, GridParseError> {
    let mut grid = Grid::new(nrows, ncols);

    for parsed in cells {
        let cell = Cell {
            row: parsed.row,
            col: parsed.col,
            address: CellAddress::from_indices(parsed.row, parsed.col),
            value: parsed.value,
            formula: parsed.formula,
        };
        grid.insert(cell);
    }

    Ok(grid)
}

fn get_attr_value(element: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, GridParseError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
        if attr.key.as_ref() == key {
            return Ok(Some(
                attr.unescape_value().map_err(to_xml_err)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

fn to_xml_err(err: quick_xml::Error) -> GridParseError {
    GridParseError::XmlError(err.to_string())
}

struct ParsedCell {
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{GridParseError, convert_value, parse_shared_strings, read_inline_string};
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
        assert!(matches!(err, GridParseError::SharedStringOutOfBounds(5)));
    }

    #[test]
    fn convert_value_error_cell_as_text() {
        let value =
            convert_value(Some("#DIV/0!"), Some("e"), &[]).expect("error cell should convert");
        assert_eq!(value, Some(CellValue::Text("#DIV/0!".into())));
    }
}
