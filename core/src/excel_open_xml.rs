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
    GridParseError, parse_defined_names, parse_relationships, parse_relationships_all,
    parse_shared_strings, parse_sheet_xml, parse_workbook_xml, resolve_sheet_target,
};
#[cfg(feature = "vba")]
use crate::vba::VbaModuleType;
use crate::string_pool::StringId;
use crate::string_pool::StringPool;
use crate::vba::VbaModule;
use crate::workbook::{ChartInfo, ChartObject, Sheet, SheetKind, Workbook};
use std::collections::HashMap;
#[cfg(feature = "std-fs")]
use std::path::Path;
use thiserror::Error;
use xxhash_rust::xxh3::Xxh3;

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
    #[error("[EXDIFF_PKG_009] serialization error: {0}. Suggestion: verify the workbook is a standard .xlsx saved by Excel.")]
    SerializationError(String),

    #[error("[EXDIFF_PKG_001] not a valid ZIP file: {message}. Suggestion: verify the input is a ZIP-based file and not corrupt.")]
    NotAZip { message: String },

    #[error("[EXDIFF_PKG_010] PBIX/PBIT does not contain DataMashup.\nSuggestion: if this is an enhanced-metadata PBIX, export as PBIT to expose DataModelSchema. If this is a legacy PBIX, ensure DataMashup is present.")]
    NoDataMashupUseTabularModel,

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
            PackageError::SerializationError(_) => error_codes::PKG_UNSUPPORTED_FORMAT,
            PackageError::NotAZip { .. } => error_codes::PKG_NOT_ZIP,
            PackageError::NoDataMashupUseTabularModel => {
                error_codes::PKG_NO_DATAMASHUP_USE_TABULAR_MODEL
            }
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

    let named_ranges = parse_defined_names(&workbook_bytes, &sheets, pool)
        .map_err(|e| wrap_grid_parse_error(e, "xl/workbook.xml"))?;

    let relationships = match container.read_file_optional_checked("xl/_rels/workbook.xml.rels")? {
        Some(bytes) => parse_relationships(&bytes)
            .map_err(|e| wrap_grid_parse_error(e, "xl/_rels/workbook.xml.rels"))?,
        None => HashMap::new(),
    };

    let mut charts: Vec<ChartObject> = Vec::new();
    let mut chart_parts: HashMap<String, ChartPartCacheEntry> = HashMap::new();

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

        let sheet_name_id = pool.intern(&sheet.name);

        let grid = parse_sheet_xml(&sheet_bytes, &shared_strings, pool)
            .map_err(|e| wrap_grid_parse_error(e, &target))?;
        sheet_ir.push(Sheet {
            name: sheet_name_id,
            workbook_sheet_id: sheet.sheet_id,
            kind: SheetKind::Worksheet,
            grid,
        });

        let drawing_rids = match parse_worksheet_drawing_rids(&sheet_bytes) {
            Ok(rids) => rids,
            Err(_) => continue,
        };
        for drawing_rid in drawing_rids {
            let sheet_rels_path = rels_part_path(&target);
            let sheet_rels_bytes = match read_optional_part(container, &sheet_rels_path)? {
                Some(bytes) => bytes,
                None => continue,
            };
            let sheet_rels = parse_relationships_all(&sheet_rels_bytes)
                .map_err(|e| wrap_grid_parse_error(e, &sheet_rels_path))?;
            let Some(drawing_target) = sheet_rels.get(&drawing_rid) else {
                continue;
            };
            let drawing_part_path = resolve_target_against_part(&target, drawing_target);

            let drawing_bytes = match read_optional_part(container, &drawing_part_path)? {
                Some(bytes) => bytes,
                None => continue,
            };
            let drawing_chart_refs = parse_drawing_chart_refs(&drawing_bytes)
                .map_err(|e| wrap_grid_parse_error(e, &drawing_part_path))?;
            if drawing_chart_refs.is_empty() {
                continue;
            }

            let drawing_rels_path = rels_part_path(&drawing_part_path);
            let drawing_rels_bytes = match read_optional_part(container, &drawing_rels_path)? {
                Some(bytes) => bytes,
                None => continue,
            };
            let drawing_rels = parse_relationships_all(&drawing_rels_bytes)
                .map_err(|e| wrap_grid_parse_error(e, &drawing_rels_path))?;

            for chart_ref in drawing_chart_refs {
                let Some(chart_target) = drawing_rels.get(&chart_ref.rel_id) else {
                    continue;
                };
                let chart_part_path = resolve_target_against_part(&drawing_part_path, chart_target);
                let chart_bytes = match read_optional_part(container, &chart_part_path)? {
                    Some(bytes) => bytes,
                    None => continue,
                };

                let entry = match chart_parts.get(&chart_part_path) {
                    Some(entry) => entry.clone(),
                    None => {
                        let xml_hash = hash_xml_part(&chart_bytes);
                        let (chart_type, data_range) =
                            parse_chart_part_metadata(&chart_bytes, pool)
                                .map_err(|e| wrap_grid_parse_error(e, &chart_part_path))?;
                        let entry = ChartPartCacheEntry {
                            xml_hash,
                            chart_type,
                            data_range,
                        };
                        chart_parts.insert(chart_part_path.clone(), entry.clone());
                        entry
                    }
                };

                let name = match chart_ref.name {
                    Some(name) => name,
                    None => fallback_chart_name_from_path(&chart_part_path),
                };

                charts.push(ChartObject {
                    sheet: sheet_name_id,
                    workbook_sheet_id: sheet.sheet_id,
                    info: ChartInfo {
                        name: pool.intern(&name),
                        chart_type: entry.chart_type,
                        data_range: entry.data_range,
                    },
                    xml_hash: entry.xml_hash,
                });
            }
        }
    }

    Ok(Workbook {
        sheets: sheet_ir,
        named_ranges,
        charts,
    })
}

#[cfg(feature = "vba")]
pub(crate) fn open_vba_modules_from_container(
    container: &mut OpcContainer,
    pool: &mut StringPool,
) -> Result<Option<Vec<VbaModule>>, PackageError> {
    let bytes = match container.read_file_optional_checked("xl/vbaProject.bin")? {
        Some(bytes) => bytes,
        None => return Ok(None),
    };

    let project = ovba::open_project(bytes).map_err(|e| PackageError::UnsupportedFormat {
        message: format!("failed to parse xl/vbaProject.bin: {e}"),
    })?;

    let mut modules = Vec::with_capacity(project.modules.len());
    for module in &project.modules {
        let name = pool.intern(&module.name);
        let module_type = match module.module_type {
            ovba::ModuleType::Procedural => VbaModuleType::Standard,
            ovba::ModuleType::DocClsDesigner => VbaModuleType::Document,
        };

        let code = match project.module_source(&module.name) {
            Ok(code) => code,
            Err(_) => match project.module_source_raw(&module.name) {
                Ok(raw) => String::from_utf8_lossy(&raw).into_owned(),
                Err(_) => String::new(),
            },
        };

        modules.push(VbaModule {
            name,
            module_type,
            code,
        });
    }

    Ok(Some(modules))
}

#[cfg(not(feature = "vba"))]
pub(crate) fn open_vba_modules_from_container(
    _container: &mut OpcContainer,
    _pool: &mut StringPool,
) -> Result<Option<Vec<VbaModule>>, PackageError> {
    Ok(None)
}

#[derive(Debug, Clone)]
struct ChartPartCacheEntry {
    xml_hash: u128,
    chart_type: StringId,
    data_range: Option<StringId>,
}

#[derive(Debug, Clone)]
struct DrawingChartRef {
    rel_id: String,
    name: Option<String>,
}

fn read_optional_part(
    container: &mut OpcContainer,
    path: &str,
) -> Result<Option<Vec<u8>>, PackageError> {
    container
        .read_file_optional_checked(path)
        .map_err(|e| PackageError::ReadPartFailed {
            part: path.to_string(),
            message: e.to_string(),
        })
}

fn rels_part_path(part_path: &str) -> String {
    let part_path = part_path.trim_start_matches('/');
    let (dir, file) = match part_path.rsplit_once('/') {
        Some((dir, file)) => (dir, file),
        None => ("", part_path),
    };

    if dir.is_empty() {
        format!("_rels/{file}.rels")
    } else {
        format!("{dir}/_rels/{file}.rels")
    }
}

fn resolve_target_against_part(base_part: &str, target: &str) -> String {
    let target = target.trim();
    if let Some(rest) = target.strip_prefix('/') {
        return normalize_part_path(rest);
    }
    if target.starts_with("xl/") {
        return normalize_part_path(target);
    }

    let base_part = base_part.trim_start_matches('/');
    let base_dir = base_part.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    if base_dir.is_empty() {
        normalize_part_path(target)
    } else {
        normalize_part_path(&format!("{base_dir}/{target}"))
    }
}

fn normalize_part_path(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                let _ = stack.pop();
            }
            other => stack.push(other),
        }
    }
    stack.join("/")
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|&b| b == b':').next().unwrap_or(name)
}

fn parse_worksheet_drawing_rids(xml: &[u8]) -> Result<Vec<String>, GridParseError> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut rids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e))
            | Ok(quick_xml::events::Event::Empty(e))
                if local_name(e.name().as_ref()) == b"drawing" =>
            {
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
                    if attr.key.as_ref() == b"r:id" {
                        let rid = attr
                            .unescape_value()
                            .map_err(|e| GridParseError::XmlError(e.to_string()))?
                            .into_owned();
                        rids.push(rid);
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(rids)
}

fn parse_drawing_chart_refs(drawing_xml: &[u8]) -> Result<Vec<DrawingChartRef>, GridParseError> {
    let mut reader = quick_xml::Reader::from_reader(drawing_xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut refs = Vec::new();
    let mut pending_name: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e))
            | Ok(quick_xml::events::Event::Empty(e)) => {
                let name = e.name();
                let tag = local_name(name.as_ref());
                if tag == b"cNvPr" {
                    for attr in e.attributes() {
                        let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
                        if attr.key.as_ref() == b"name" {
                            pending_name = Some(
                                attr.unescape_value()
                                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                                    .into_owned(),
                            );
                        }
                    }
                } else if tag == b"chart" {
                    let mut rel_id = None;
                    for attr in e.attributes() {
                        let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
                        if attr.key.as_ref() == b"r:id" {
                            rel_id = Some(
                                attr.unescape_value()
                                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                                    .into_owned(),
                            );
                        }
                    }
                    if let Some(rel_id) = rel_id {
                        refs.push(DrawingChartRef {
                            rel_id,
                            name: pending_name.take(),
                        });
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(refs)
}

fn parse_chart_part_metadata(
    chart_xml: &[u8],
    pool: &mut StringPool,
) -> Result<(StringId, Option<StringId>), GridParseError> {
    let mut reader = quick_xml::Reader::from_reader(chart_xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut chart_type: Option<String> = None;
    let mut data_range: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e))
                if local_name(e.name().as_ref()) == b"f" && data_range.is_none() =>
            {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    data_range = Some(trimmed.to_string());
                }
            }
            Ok(quick_xml::events::Event::Start(e)) | Ok(quick_xml::events::Event::Empty(e)) => {
                let name = e.name();
                let tag = local_name(name.as_ref());
                if chart_type.is_none() && tag.ends_with(b"Chart") {
                    chart_type = Some(String::from_utf8_lossy(tag).to_string());
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let chart_type_id = pool.intern(chart_type.as_deref().unwrap_or("unknown"));
    let data_range_id = data_range.as_deref().map(|s| pool.intern(s));
    Ok((chart_type_id, data_range_id))
}

fn fallback_chart_name_from_path(chart_part_path: &str) -> String {
    let file = chart_part_path
        .rsplit('/')
        .next()
        .unwrap_or(chart_part_path);
    file.strip_suffix(".xml").unwrap_or(file).to_string()
}

fn hash_xml_part(xml: &[u8]) -> u128 {
    let mut hasher = Xxh3::new();
    hasher.update(xml);
    hasher.digest128()
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

#[cfg(feature = "std-fs")]
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

#[cfg(feature = "std-fs")]
#[allow(deprecated)]
pub fn open_vba_modules(
    path: impl AsRef<Path>,
    pool: &mut StringPool,
) -> Result<Option<Vec<VbaModule>>, PackageError> {
    let path_str = path.as_ref().display().to_string();
    let mut container = OpcContainer::open_from_path(path.as_ref())
        .map_err(|e| PackageError::from(e).with_path(&path_str))?;
    open_vba_modules_from_container(&mut container, pool).map_err(|e| e.with_path(&path_str))
}

pub(crate) fn open_data_mashup_from_container(
    container: &mut OpcContainer,
) -> Result<Option<RawDataMashup>, PackageError> {
    let mut found: Option<RawDataMashup> = None;
    let names: Vec<String> = container.file_names().map(|s| s.to_string()).collect();

    for name in names {
        if !(name.starts_with("customXml/") && name.ends_with(".xml") && name.contains("item")) {
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

    Ok(found)
}

#[cfg(feature = "std-fs")]
#[allow(deprecated)]
pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, PackageError> {
    let path_str = path.as_ref().display().to_string();
    let mut container = OpcContainer::open_from_path(path.as_ref())
        .map_err(|e| PackageError::from(e).with_path(&path_str))?;
    open_data_mashup_from_container(&mut container)
        .map_err(|e| e.with_path(&path_str))
}
