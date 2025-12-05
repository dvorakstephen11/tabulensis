//! Excel Open XML file parsing.
//!
//! Provides functions for opening `.xlsx` files and parsing their contents into
//! the internal representation used for diffing.

use crate::container::{ContainerError, OpcContainer};
use crate::datamashup_framing::{
    DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup,
    read_datamashup_text,
};
use crate::grid_parser::{
    GridParseError, parse_relationships, parse_shared_strings, parse_sheet_xml, parse_workbook_xml,
    resolve_sheet_target,
};
use crate::workbook::{Sheet, SheetKind, Workbook};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExcelOpenError {
    #[error("container error: {0}")]
    Container(#[from] ContainerError),
    #[error("grid parse error: {0}")]
    GridParse(#[from] GridParseError),
    #[error("DataMashup error: {0}")]
    DataMashup(#[from] DataMashupError),
    #[error("workbook.xml missing or unreadable")]
    WorkbookXmlMissing,
    #[error("worksheet XML missing for sheet {sheet_name}")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("serialization error: {0}")]
    SerializationError(String),
}

pub fn open_workbook(path: impl AsRef<Path>) -> Result<Workbook, ExcelOpenError> {
    let mut container = OpcContainer::open(path.as_ref())?;

    let shared_strings = match container
        .read_file_optional("xl/sharedStrings.xml")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_shared_strings(&bytes)?,
        None => Vec::new(),
    };

    let workbook_bytes = container
        .read_file("xl/workbook.xml")
        .map_err(|_| ExcelOpenError::WorkbookXmlMissing)?;

    let sheets = parse_workbook_xml(&workbook_bytes)?;

    let relationships = match container
        .read_file_optional("xl/_rels/workbook.xml.rels")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_relationships(&bytes)?,
        None => HashMap::new(),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes =
            container
                .read_file(&target)
                .map_err(|_| ExcelOpenError::WorksheetXmlMissing {
                    sheet_name: sheet.name.clone(),
                })?;
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
    let mut container = OpcContainer::open(path.as_ref())?;
    let mut found: Option<RawDataMashup> = None;

    for i in 0..container.len() {
        let name = {
            let file = container.archive.by_index(i).ok();
            file.map(|f| f.name().to_string())
        };

        if let Some(name) = name {
            if !name.starts_with("customXml/") || !name.ends_with(".xml") {
                continue;
            }

            let bytes = container
                .read_file(&name)
                .map_err(|e| ContainerError::Zip(e.to_string()))?;

            if let Some(text) = read_datamashup_text(&bytes)? {
                let decoded = decode_datamashup_base64(&text)?;
                let parsed = parse_data_mashup(&decoded)?;
                if found.is_some() {
                    return Err(DataMashupError::FramingInvalid.into());
                }
                found = Some(parsed);
            }
        }
    }

    Ok(found)
}
