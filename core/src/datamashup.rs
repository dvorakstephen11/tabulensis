use std::collections::HashMap;

use crate::datamashup_framing::{DataMashupError, RawDataMashup};
use crate::datamashup_package::{PackageParts, parse_package_parts};
use crate::m_section::{SectionParseError, parse_section_members};
use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataMashup {
    pub version: u32,
    pub package_parts: PackageParts,
    pub permissions: Permissions,
    pub metadata: Metadata,
    pub permission_bindings_raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permissions {
    pub can_evaluate_future_packages: bool,
    pub firewall_enabled: bool,
    pub workbook_group_type: Option<String>,
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions {
            can_evaluate_future_packages: false,
            firewall_enabled: true,
            workbook_group_type: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub formulas: Vec<QueryMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMetadata {
    pub item_path: String,
    pub section_name: String,
    pub formula_name: String,
    pub load_to_sheet: bool,
    pub load_to_model: bool,
    pub is_connection_only: bool,
    pub group_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub name: String,
    pub section_member: String,
    pub expression_m: String,
    pub metadata: QueryMetadata,
}

pub fn build_data_mashup(raw: &RawDataMashup) -> Result<DataMashup, DataMashupError> {
    let package_parts = parse_package_parts(&raw.package_parts)?;
    let permissions = parse_permissions(&raw.permissions);
    let metadata = parse_metadata(&raw.metadata)?;

    Ok(DataMashup {
        version: raw.version,
        package_parts,
        permissions,
        metadata,
        permission_bindings_raw: raw.permission_bindings.clone(),
    })
}

pub fn build_queries(dm: &DataMashup) -> Result<Vec<Query>, SectionParseError> {
    let members = parse_section_members(&dm.package_parts.main_section.source)?;

    let mut metadata_index: HashMap<(String, String), QueryMetadata> = HashMap::new();
    for meta in &dm.metadata.formulas {
        metadata_index.insert(
            (meta.section_name.clone(), meta.formula_name.clone()),
            meta.clone(),
        );
    }

    let mut positions: HashMap<String, usize> = HashMap::new();
    let mut queries = Vec::new();

    for member in members {
        let section_name = member.section_name.clone();
        let member_name = member.member_name.clone();
        let key = (section_name.clone(), member_name.clone());
        let metadata = metadata_index
            .get(&key)
            .cloned()
            .unwrap_or_else(|| QueryMetadata {
                item_path: format!("{}/{}", section_name, member_name),
                section_name: section_name.clone(),
                formula_name: member_name.clone(),
                load_to_sheet: false,
                load_to_model: false,
                is_connection_only: true,
                group_path: None,
            });

        let name = format!("{}/{}", section_name, member_name);
        let query = Query {
            name: name.clone(),
            section_member: member.member_name,
            expression_m: member.expression_m,
            metadata,
        };

        if let Some(idx) = positions.get(&name) {
            debug_assert!(false, "duplicate query name {}", name);
            queries[*idx] = query;
        } else {
            positions.insert(name, queries.len());
            queries.push(query);
        }
    }

    Ok(queries)
}

pub fn parse_permissions(xml_bytes: &[u8]) -> Permissions {
    if xml_bytes.is_empty() {
        return Permissions::default();
    }

    let Ok(mut text) = String::from_utf8(xml_bytes.to_vec()) else {
        return Permissions::default();
    };
    if let Some(stripped) = text.strip_prefix('\u{FEFF}') {
        text = stripped.to_string();
    }

    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current_tag: Option<String> = None;
    let mut permissions = Permissions::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag =
                    Some(String::from_utf8_lossy(local_name(e.name().as_ref())).to_string());
            }
            Ok(Event::Text(t)) => {
                if let Some(tag) = current_tag.as_deref() {
                    let value = match t.unescape() {
                        Ok(v) => v.into_owned(),
                        Err(_) => {
                            // Any unescape failure means the permissions payload is unusable; fall back to defaults.
                            return Permissions::default();
                        }
                    };
                    match tag {
                        "CanEvaluateFuturePackages" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.can_evaluate_future_packages = v;
                            }
                        }
                        "FirewallEnabled" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.firewall_enabled = v;
                            }
                        }
                        "WorkbookGroupType" => {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() {
                                permissions.workbook_group_type = Some(trimmed.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(tag) = current_tag.as_deref() {
                    let value = String::from_utf8_lossy(&t.into_inner()).to_string();
                    match tag {
                        "CanEvaluateFuturePackages" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.can_evaluate_future_packages = v;
                            }
                        }
                        "FirewallEnabled" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.firewall_enabled = v;
                            }
                        }
                        "WorkbookGroupType" => {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() {
                                permissions.workbook_group_type = Some(trimmed.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => current_tag = None,
            Ok(Event::Eof) => break,
            Err(_) => return Permissions::default(),
            _ => {}
        }
        buf.clear();
    }

    permissions
}

pub fn parse_metadata(metadata_bytes: &[u8]) -> Result<Metadata, DataMashupError> {
    if metadata_bytes.is_empty() {
        return Ok(Metadata {
            formulas: Vec::new(),
        });
    }

    let xml_bytes = metadata_xml_bytes(metadata_bytes)?;
    let mut text = String::from_utf8(xml_bytes)
        .map_err(|_| DataMashupError::XmlError("metadata is not valid UTF-8".into()))?;
    if let Some(stripped) = text.strip_prefix('\u{FEFF}') {
        text = stripped.to_string();
    }

    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut element_stack: Vec<String> = Vec::new();
    let mut item_type: Option<String> = None;
    let mut item_path: Option<String> = None;
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut formulas: Vec<QueryMetadata> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(local_name(e.name().as_ref())).to_string();
                if name == "Entry"
                    && let Some((typ, val)) = parse_entry_attributes(&e)?
                {
                    entries.push((typ, val));
                }
            }
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(local_name(e.name().as_ref())).to_string();
                if name == "Item" {
                    item_type = None;
                    item_path = None;
                    entries.clear();
                }
                if name == "Entry"
                    && let Some((typ, val)) = parse_entry_attributes(&e)?
                {
                    entries.push((typ, val));
                }
                element_stack.push(name);
            }
            Ok(Event::Text(t)) => {
                if let Some(tag) = element_stack.last() {
                    let value = t
                        .unescape()
                        .map_err(|e| DataMashupError::XmlError(e.to_string()))?
                        .into_owned();
                    match tag.as_str() {
                        "ItemType" => {
                            item_type = Some(value.trim().to_string());
                        }
                        "ItemPath" => {
                            item_path = Some(value.trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(tag) = element_stack.last() {
                    let value = String::from_utf8_lossy(&t.into_inner()).to_string();
                    match tag.as_str() {
                        "ItemType" => {
                            item_type = Some(value.trim().to_string());
                        }
                        "ItemPath" => {
                            item_path = Some(value.trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name_bytes = local_name(e.name().as_ref()).to_vec();
                if name_bytes.as_slice() == b"Item" && item_type.as_deref() == Some("Formula") {
                    let raw_path = item_path.clone().ok_or_else(|| {
                        DataMashupError::XmlError("Formula item missing ItemPath".into())
                    })?;
                    let decoded_path = decode_item_path(&raw_path)?;
                    let (section_name, formula_name) = split_item_path(&decoded_path)?;
                    let load_to_sheet =
                        entry_bool(&entries, &["FillEnabled", "LoadEnabled"]).unwrap_or(false);
                    let load_to_model = entry_bool(
                        &entries,
                        &[
                            "FillToDataModelEnabled",
                            "AddedToDataModel",
                            "LoadToDataModel",
                        ],
                    )
                    .unwrap_or(false);
                    // Group paths are derived solely from per-formula entries for now; the AllFormulas tree is not parsed yet.
                    let group_path = entry_string(
                        &entries,
                        &[
                            "QueryGroupId",
                            "QueryGroupID",
                            "QueryGroupPath",
                            "QueryGroup",
                        ],
                    );

                    let metadata = QueryMetadata {
                        item_path: decoded_path.clone(),
                        section_name,
                        formula_name,
                        load_to_sheet,
                        load_to_model,
                        is_connection_only: !(load_to_sheet || load_to_model),
                        group_path,
                    };
                    formulas.push(metadata);
                }

                if let Some(last) = element_stack.last()
                    && last.as_bytes() == name_bytes.as_slice()
                {
                    element_stack.pop();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DataMashupError::XmlError(e.to_string())),
            _ => {}
        }

        buf.clear();
    }

    Ok(Metadata { formulas })
}

fn metadata_xml_bytes(metadata_bytes: &[u8]) -> Result<Vec<u8>, DataMashupError> {
    if looks_like_xml(metadata_bytes) {
        return Ok(metadata_bytes.to_vec());
    }

    if metadata_bytes.len() >= 8 {
        let content_len = u32::from_le_bytes(metadata_bytes[0..4].try_into().unwrap()) as usize;
        let xml_len = u32::from_le_bytes(metadata_bytes[4..8].try_into().unwrap()) as usize;
        let start = 8usize
            .checked_add(content_len)
            .ok_or_else(|| DataMashupError::XmlError("metadata length overflow".into()))?;
        let end = start
            .checked_add(xml_len)
            .ok_or_else(|| DataMashupError::XmlError("metadata length overflow".into()))?;
        if end <= metadata_bytes.len() {
            return Ok(metadata_bytes[start..end].to_vec());
        }
        return Err(DataMashupError::XmlError(
            "metadata length prefix invalid".into(),
        ));
    }

    Err(DataMashupError::XmlError("metadata XML not found".into()))
}

fn looks_like_xml(bytes: &[u8]) -> bool {
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    if idx >= bytes.len() {
        return false;
    }

    let slice = &bytes[idx..];
    slice.starts_with(b"<")
        || slice.starts_with(&[0xEF, 0xBB, 0xBF])
        || slice.starts_with(&[0xFE, 0xFF])
        || slice.starts_with(&[0xFF, 0xFE])
}

fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..).unwrap_or(name),
        None => name,
    }
}

fn parse_bool(text: &str) -> Option<bool> {
    let trimmed = text.trim();
    let payload = trimmed
        .strip_prefix(|c| c == 'l' || c == 'L')
        .unwrap_or(trimmed);
    let lowered = payload.to_ascii_lowercase();
    match lowered.as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn parse_entry_attributes(
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<Option<(String, String)>, DataMashupError> {
    let mut typ: Option<String> = None;
    let mut value: Option<String> = None;

    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|e| DataMashupError::XmlError(e.to_string()))?;
        let key = local_name(attr.key.as_ref());
        if key == b"Type" {
            typ = Some(
                String::from_utf8(attr.value.as_ref().to_vec())
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?,
            );
        } else if key == b"Value" {
            value = Some(
                String::from_utf8(attr.value.as_ref().to_vec())
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?,
            );
        }
    }

    match (typ, value) {
        (Some(t), Some(v)) => Ok(Some((t, v))),
        _ => Ok(None),
    }
}

fn entry_bool(entries: &[(String, String)], keys: &[&str]) -> Option<bool> {
    for (key, val) in entries {
        if keys.iter().any(|k| k.eq_ignore_ascii_case(key))
            && let Some(b) = parse_bool(val)
        {
            return Some(b);
        }
    }
    None
}

fn entry_string(entries: &[(String, String)], keys: &[&str]) -> Option<String> {
    for (key, val) in entries {
        if keys.iter().any(|k| k.eq_ignore_ascii_case(key)) {
            let trimmed = val.trim();
            let without_prefix = trimmed
                .strip_prefix('s')
                .or_else(|| trimmed.strip_prefix('S'))
                .unwrap_or(trimmed);
            if without_prefix.is_empty() {
                return None;
            }
            return Some(without_prefix.to_string());
        }
    }
    None
}

fn decode_item_path(path: &str) -> Result<String, DataMashupError> {
    let mut decoded = Vec::with_capacity(path.len());
    let bytes = path.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b == b'%' {
            if idx + 2 >= bytes.len() {
                return Err(DataMashupError::XmlError(
                    "invalid percent-encoding in ItemPath".into(),
                ));
            }
            let hi = hex_value(bytes[idx + 1]).ok_or_else(|| {
                DataMashupError::XmlError("invalid percent-encoding in ItemPath".into())
            })?;
            let lo = hex_value(bytes[idx + 2]).ok_or_else(|| {
                DataMashupError::XmlError("invalid percent-encoding in ItemPath".into())
            })?;
            decoded.push(hi << 4 | lo);
            idx += 3;
            continue;
        }
        decoded.push(b);
        idx += 1;
    }
    String::from_utf8(decoded)
        .map_err(|_| DataMashupError::XmlError("invalid UTF-8 in ItemPath".into()))
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + b - b'a'),
        b'A'..=b'F' => Some(10 + b - b'A'),
        _ => None,
    }
}

fn split_item_path(path: &str) -> Result<(String, String), DataMashupError> {
    let mut parts = path.split('/');
    let section = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    if section.is_empty() || rest.is_empty() {
        return Err(DataMashupError::XmlError(
            "invalid ItemPath in metadata".into(),
        ));
    }
    let formula = rest.join("/");
    Ok((section.to_string(), formula))
}
