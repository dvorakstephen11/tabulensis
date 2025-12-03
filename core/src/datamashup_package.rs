use crate::datamashup_framing::DataMashupError;
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;

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
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|_| DataMashupError::FramingInvalid)?;

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

        let raw_name = file.name().to_string();
        let name = normalize_path(&raw_name);
        if package_xml.is_none() && name == "Config/Package.xml" {
            let text = read_file_to_string(&mut file)?;
            package_xml = Some(PackageXml { raw_xml: text });
            continue;
        }
        if main_section.is_none() && name == "Formulas/Section1.m" {
            let text = strip_leading_bom(read_file_to_string(&mut file)?);
            main_section = Some(SectionDocument { source: text });
            continue;
        }
        if name.starts_with("Content/") {
            let mut content_bytes = Vec::new();
            if file.read_to_end(&mut content_bytes).is_err() {
                continue;
            }

            if let Some(section) = extract_embedded_section(&content_bytes) {
                embedded_contents.push(EmbeddedContent {
                    name: normalize_path(&raw_name).to_string(),
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

fn normalize_path(name: &str) -> &str {
    name.trim_start_matches('/')
}

fn read_file_to_string(file: &mut zip::read::ZipFile<'_>) -> Result<String, DataMashupError> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|_| DataMashupError::FramingInvalid)?;
    String::from_utf8(buf).map_err(|_| DataMashupError::FramingInvalid)
}

fn extract_embedded_section(bytes: &[u8]) -> Option<String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).ok()?;
    find_section_document(&mut archive)
}

fn find_section_document<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    for idx in 0..archive.len() {
        let mut file = match archive.by_index(idx) {
            Ok(file) => file,
            Err(_) => continue,
        };
        if file.is_dir() {
            continue;
        }

        if normalize_path(file.name()) == "Formulas/Section1.m" {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                let text = String::from_utf8(buf).ok()?;
                return Some(strip_leading_bom(text));
            }
        }
    }
    None
}

fn strip_leading_bom(text: String) -> String {
    text.strip_prefix('\u{FEFF}')
        .map(|s| s.to_string())
        .unwrap_or(text)
}
