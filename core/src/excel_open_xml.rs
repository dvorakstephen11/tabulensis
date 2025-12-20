//! Excel Open XML file parsing.
//!
//! Provides functions for opening `.xlsx` files and parsing their contents into
//! the internal representation used for diffing.

use crate::container::{ContainerError, OpcContainer};
use crate::datamashup_framing::{
    DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup,
    read_datamashup_text,
};
use crate::error_codes;
use crate::grid_parser::{
    GridParseError, parse_relationships, parse_shared_strings, parse_sheet_xml, parse_workbook_xml,
    resolve_sheet_target,
};
use crate::string_pool::StringPool;
use crate::workbook::{Sheet, SheetKind, Workbook};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PackageError {
    #[error("{0}")]
    Container(#[from] ContainerError),
    #[error("{0}")]
    GridParse(#[from] GridParseError),
    #[error("{0}")]
    DataMashup(#[from] DataMashupError),
    #[error("[EXDIFF_PKG_003] workbook.xml missing or unreadable. Suggestion: re-save the file in Excel or verify it is a valid .xlsx.")]
    WorkbookXmlMissing,
    #[error("[EXDIFF_PKG_003] worksheet XML missing for sheet {sheet_name}. Suggestion: re-save the file in Excel or verify it is a valid .xlsx.")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("{0}")]
    Diff(#[from] crate::diff::DiffError),
    #[error("[EXDIFF_PKG_009] serialization error: {0}. Suggestion: verify the workbook is a standard .xlsx saved by Excel.")]
    SerializationError(String),

    #[error("[EXDIFF_PKG_001] not a valid ZIP file: {message}. Suggestion: verify the input is a .xlsx workbook.")]
    NotAZip { message: String },

    #[error("[EXDIFF_PKG_003] missing required part: {path}. Suggestion: the workbook may be corrupt; re-save the file in Excel.")]
    MissingPart { path: String },

    #[error("[EXDIFF_PKG_004] invalid XML in '{part}' at line {line}, column {column}: {message}. Suggestion: re-save the file in Excel.")]
    InvalidXml {
        part: String,
        line: usize,
        column: usize,
        message: String,
    },

    #[error("[EXDIFF_PKG_009] unsupported format: {message}. Suggestion: verify the workbook is a standard .xlsx saved by Excel.")]
    UnsupportedFormat { message: String },

    #[error("[EXDIFF_PKG_008] failed to read part '{part}': {message}")]
    ReadPartFailed { part: String, message: String },

    #[error("{source} (in part '{part}')")]
    DataMashupPartError { part: String, source: DataMashupError },

    #[error("[{path}] {source}")]
    WithPath {
        path: String,
        #[source]
        source: Box<PackageError>,
    },
}

impl PackageError {
    pub fn code(&self) -> &'static str {
        match self {
            PackageError::Container(e) => e.code(),
            PackageError::GridParse(e) => e.code(),
            PackageError::DataMashup(e) => e.code(),
            PackageError::WorkbookXmlMissing => error_codes::PKG_MISSING_PART,
            PackageError::WorksheetXmlMissing { .. } => error_codes::PKG_MISSING_PART,
            PackageError::Diff(_) => error_codes::DIFF_INTERNAL_ERROR,
            PackageError::SerializationError(_) => error_codes::PKG_UNSUPPORTED_FORMAT,
            PackageError::NotAZip { .. } => error_codes::PKG_NOT_ZIP,
            PackageError::MissingPart { .. } => error_codes::PKG_MISSING_PART,
            PackageError::InvalidXml { .. } => error_codes::PKG_INVALID_XML,
            PackageError::UnsupportedFormat { .. } => error_codes::PKG_UNSUPPORTED_FORMAT,
            PackageError::ReadPartFailed { .. } => error_codes::PKG_ZIP_READ,
            PackageError::DataMashupPartError { source, .. } => source.code(),
            PackageError::WithPath { source, .. } => source.code(),
        }
    }

    pub fn with_path(self, path: impl Into<String>) -> Self {
        PackageError::WithPath {
            path: path.into(),
            source: Box::new(self),
        }
    }
}

#[deprecated(note = "use PackageError")]
pub type ExcelOpenError = PackageError;

pub(crate) fn open_workbook_from_container(
    container: &mut OpcContainer,
    pool: &mut StringPool,
) -> Result<Workbook, PackageError> {
    let shared_strings = match container.read_file_optional_checked("xl/sharedStrings.xml")? {
        Some(bytes) => parse_shared_strings(&bytes, pool).map_err(|e| {
            wrap_grid_parse_error(e, "xl/sharedStrings.xml")
        })?,
        None => Vec::new(),
    };

    let workbook_bytes = container
        .read_file_checked("xl/workbook.xml")
        .map_err(|e| match e {
            ContainerError::FileNotFound { .. } => PackageError::MissingPart {
                path: "xl/workbook.xml".to_string(),
            },
            other => PackageError::ReadPartFailed {
                part: "xl/workbook.xml".to_string(),
                message: other.to_string(),
            },
        })?;

    let sheets = parse_workbook_xml(&workbook_bytes)
        .map_err(|e| wrap_grid_parse_error(e, "xl/workbook.xml"))?;

    let relationships = match container.read_file_optional_checked("xl/_rels/workbook.xml.rels")? {
        Some(bytes) => parse_relationships(&bytes)
            .map_err(|e| wrap_grid_parse_error(e, "xl/_rels/workbook.xml.rels"))?,
        None => HashMap::new(),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes = container.read_file_checked(&target).map_err(|e| match e {
            ContainerError::FileNotFound { .. } => PackageError::MissingPart {
                path: target.clone(),
            },
            other => PackageError::ReadPartFailed {
                part: target.clone(),
                message: other.to_string(),
            },
        })?;
        let grid = parse_sheet_xml(&sheet_bytes, &shared_strings, pool)
            .map_err(|e| wrap_grid_parse_error(e, &target))?;
        sheet_ir.push(Sheet {
            name: pool.intern(&sheet.name),
            kind: SheetKind::Worksheet,
            grid,
        });
    }

    Ok(Workbook { sheets: sheet_ir })
}

fn wrap_grid_parse_error(err: GridParseError, part: &str) -> PackageError {
    match err {
        GridParseError::XmlErrorAt { line, column, message } => PackageError::InvalidXml {
            part: part.to_string(),
            line,
            column,
            message,
        },
        GridParseError::XmlError(msg) => PackageError::InvalidXml {
            part: part.to_string(),
            line: 0,
            column: 0,
            message: msg,
        },
        GridParseError::InvalidAddress(addr) => PackageError::UnsupportedFormat {
            message: format!("invalid cell address '{}' in {}", addr, part),
        },
        GridParseError::SharedStringOutOfBounds(idx) => PackageError::UnsupportedFormat {
            message: format!(
                "shared string index {} out of bounds while parsing {}",
                idx, part
            ),
        },
    }
}

#[allow(deprecated)]
pub fn open_workbook(
    path: impl AsRef<Path>,
    pool: &mut StringPool,
) -> Result<Workbook, PackageError> {
    let path_str = path.as_ref().display().to_string();
    let mut container = OpcContainer::open_from_path(path.as_ref())
        .map_err(|e| PackageError::from(e).with_path(&path_str))?;
    open_workbook_from_container(&mut container, pool)
        .map_err(|e| e.with_path(&path_str))
}

pub(crate) fn open_data_mashup_from_container(
    container: &mut OpcContainer,
) -> Result<Option<RawDataMashup>, PackageError> {
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
                .read_file_checked(&name)
                .map_err(|e| PackageError::ReadPartFailed {
                    part: name.clone(),
                    message: e.to_string(),
                })?;

            match read_datamashup_text(&bytes) {
                Ok(Some(text)) => {
                    let decoded = decode_datamashup_base64(&text).map_err(|e| {
                        PackageError::DataMashupPartError {
                            part: name.clone(),
                            source: e,
                        }
                    })?;
                    let parsed = parse_data_mashup(&decoded).map_err(|e| {
                        PackageError::DataMashupPartError {
                            part: name.clone(),
                            source: e,
                        }
                    })?;
                    if found.is_some() {
                        return Err(PackageError::DataMashupPartError {
                            part: name,
                            source: DataMashupError::FramingInvalid,
                        });
                    }
                    found = Some(parsed);
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(PackageError::DataMashupPartError {
                        part: name,
                        source: e,
                    });
                }
            }
        }
    }

    Ok(found)
}

#[allow(deprecated)]
pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, PackageError> {
    let path_str = path.as_ref().display().to_string();
    let mut container = OpcContainer::open_from_path(path.as_ref())
        .map_err(|e| PackageError::from(e).with_path(&path_str))?;
    open_data_mashup_from_container(&mut container)
        .map_err(|e| e.with_path(&path_str))
}
