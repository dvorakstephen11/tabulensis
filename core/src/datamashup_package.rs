use crate::datamashup_framing::DataMashupError;
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;

#[derive(Debug, Clone, Copy)]
pub struct DataMashupLimits {
    pub max_inner_entries: usize,
    pub max_inner_part_bytes: u64,
    pub max_inner_total_bytes: u64,
}

impl Default for DataMashupLimits {
    fn default() -> Self {
        Self {
            max_inner_entries: 10_000,
            max_inner_part_bytes: 100 * 1024 * 1024,
            max_inner_total_bytes: 500 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageXml {
    pub raw_xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDocument {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedContent {
    /// Normalized PackageParts path for the embedded package (never starts with '/').
    pub name: String,
    pub section: SectionDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageParts {
    pub package_xml: PackageXml,
    pub main_section: SectionDocument,
    pub embedded_contents: Vec<EmbeddedContent>,
}

pub fn parse_package_parts(bytes: &[u8]) -> Result<PackageParts, DataMashupError> {
    parse_package_parts_with_limits(bytes, DataMashupLimits::default())
}

pub fn parse_package_parts_with_limits(
    bytes: &[u8],
    limits: DataMashupLimits,
) -> Result<PackageParts, DataMashupError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|_| DataMashupError::FramingInvalid)?;

    if archive.len() > limits.max_inner_entries {
        return Err(DataMashupError::InnerTooManyEntries {
            entries: archive.len(),
            max_entries: limits.max_inner_entries,
        });
    }

    let mut total_read: u64 = 0;
    let mut package_xml: Option<PackageXml> = None;
    let mut main_section: Option<SectionDocument> = None;
    let mut embedded_contents: Vec<EmbeddedContent> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|_| DataMashupError::FramingInvalid)?;
        if file.is_dir() {
            continue;
        }

        let name = normalize_path(file.name());
        if package_xml.is_none() && name == "Config/Package.xml" {
            reserve_inner_read_budget(&mut total_read, &name, file.size(), limits)?;
            let text = read_file_to_string(&mut file)?;
            package_xml = Some(PackageXml { raw_xml: text });
            continue;
        }
        if main_section.is_none() && name == "Formulas/Section1.m" {
            reserve_inner_read_budget(&mut total_read, &name, file.size(), limits)?;
            let text = strip_leading_bom(read_file_to_string(&mut file)?);
            main_section = Some(SectionDocument { source: text });
            continue;
        }
        if name.starts_with("Content/") {
            reserve_inner_read_budget(&mut total_read, &name, file.size(), limits)?;
            let mut content_bytes = Vec::new();
            if file.read_to_end(&mut content_bytes).is_err() {
                continue;
            }

            let unpacked_suffix = "/Formulas/Section1.m";
            if name.ends_with(unpacked_suffix) {
                if let Some(root) = name.strip_suffix(unpacked_suffix) {
                    if embedded_contents.iter().all(|e| e.name != root) {
                        if let Ok(text) = std::str::from_utf8(&content_bytes) {
                            let s = strip_leading_bom(text.to_string());
                            embedded_contents.push(EmbeddedContent {
                                name: root.to_string(),
                                section: SectionDocument { source: s },
                            });
                        }
                    }
                }
                continue;
            }

            if let Some(section) = extract_embedded_section(&content_bytes, limits, &name)? {
                embedded_contents.push(EmbeddedContent {
                    name: name.clone(),
                    section: SectionDocument { source: section },
                });
            }
        }
    }

    let package_xml = package_xml.ok_or(DataMashupError::FramingInvalid)?;
    let main_section = main_section.ok_or(DataMashupError::FramingInvalid)?;

    Ok(PackageParts {
        package_xml,
        main_section,
        embedded_contents,
    })
}

fn normalize_path(name: &str) -> String {
    let trimmed = name.trim_start_matches(|c| c == '/' || c == '\\');
    trimmed.replace('\\', "/")
}

fn read_file_to_string(file: &mut zip::read::ZipFile<'_>) -> Result<String, DataMashupError> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|_| DataMashupError::FramingInvalid)?;
    String::from_utf8(buf).map_err(|_| DataMashupError::FramingInvalid)
}

fn reserve_inner_read_budget(
    total_read: &mut u64,
    path: &str,
    size: u64,
    limits: DataMashupLimits,
) -> Result<(), DataMashupError> {
    if size > limits.max_inner_part_bytes {
        return Err(DataMashupError::InnerPartTooLarge {
            path: path.to_string(),
            size,
            limit: limits.max_inner_part_bytes,
        });
    }

    let new_total = total_read.saturating_add(size);
    if new_total > limits.max_inner_total_bytes {
        return Err(DataMashupError::InnerTotalTooLarge {
            limit: limits.max_inner_total_bytes,
        });
    }

    *total_read = new_total;
    Ok(())
}

fn extract_embedded_section(
    bytes: &[u8],
    limits: DataMashupLimits,
    outer_name: &str,
) -> Result<Option<String>, DataMashupError> {
    let cursor = Cursor::new(bytes);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(archive) => archive,
        Err(_) => return Ok(None),
    };

    if archive.len() > limits.max_inner_entries {
        return Err(DataMashupError::InnerTooManyEntries {
            entries: archive.len(),
            max_entries: limits.max_inner_entries,
        });
    }

    find_section_document(&mut archive, limits, outer_name)
}

fn find_section_document<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    limits: DataMashupLimits,
    outer_name: &str,
) -> Result<Option<String>, DataMashupError> {
    let mut total_read: u64 = 0;

    for idx in 0..archive.len() {
        let mut file = match archive.by_index(idx) {
            Ok(file) => file,
            Err(_) => continue,
        };
        if file.is_dir() {
            continue;
        }

        let inner_name = normalize_path(file.name());
        if inner_name == "Formulas/Section1.m" {
            let combined_name = format!("{outer_name}/{inner_name}");
            reserve_inner_read_budget(&mut total_read, &combined_name, file.size(), limits)?;
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                if let Ok(text) = String::from_utf8(buf) {
                    return Ok(Some(strip_leading_bom(text)));
                }
            }
            return Ok(None);
        }
    }
    Ok(None)
}

fn strip_leading_bom(text: String) -> String {
    text.strip_prefix('\u{FEFF}')
        .map(|s| s.to_string())
        .unwrap_or(text)
}
