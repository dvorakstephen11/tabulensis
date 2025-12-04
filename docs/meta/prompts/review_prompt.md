# Codebase Context for Review

## Directory Structure

```text
/
  Cargo.lock
  Cargo.toml
  README.md
  core/
    Cargo.lock
    Cargo.toml
    src/
      addressing.rs
      container.rs
      datamashup.rs
      datamashup_framing.rs
      datamashup_package.rs
      diff.rs
      engine.rs
      excel_open_xml.rs
      grid_parser.rs
      lib.rs
      main.rs
      m_section.rs
      workbook.rs
      output/
        json.rs
        mod.rs
    tests/
      addressing_pg2_tests.rs
      data_mashup_tests.rs
      engine_tests.rs
      excel_open_xml_tests.rs
      integration_test.rs
      m4_package_parts_tests.rs
      m4_permissions_metadata_tests.rs
      m_section_splitting_tests.rs
      output_tests.rs
      pg1_ir_tests.rs
      pg3_snapshot_tests.rs
      pg4_diffop_tests.rs
      pg5_grid_diff_tests.rs
      pg6_object_vs_grid_tests.rs
      signature_tests.rs
      sparse_grid_tests.rs
      common/
        mod.rs
  fixtures/
    manifest.yaml
    pyproject.toml
    README.md
    requirements.txt
    generated/
      corrupt_base64.xlsx
      db_equal_ordered_a.xlsx
      db_equal_ordered_b.xlsx
      db_row_added_b.xlsx
      duplicate_datamashup_elements.xlsx
      duplicate_datamashup_parts.xlsx
      grid_large_dense.xlsx
      grid_large_noise.xlsx
      json_diff_bool_a.xlsx
      json_diff_bool_b.xlsx
      json_diff_single_cell_a.xlsx
      json_diff_single_cell_b.xlsx
      json_diff_value_to_empty_a.xlsx
      json_diff_value_to_empty_b.xlsx
      mashup_base64_whitespace.xlsx
      mashup_utf16_be.xlsx
      mashup_utf16_le.xlsx
      metadata_hidden_queries.xlsx
      metadata_query_groups.xlsx
      metadata_simple.xlsx
      minimal.xlsx
      multi_query_with_embedded.xlsx
      m_change_literal_b.xlsx
      not_a_zip.txt
      no_content_types.xlsx
      one_query.xlsx
      permissions_defaults.xlsx
      permissions_firewall_off.xlsx
      pg1_basic_two_sheets.xlsx
      pg1_empty_and_mixed_sheets.xlsx
      pg1_sparse_used_range.xlsx
      pg2_addressing_matrix.xlsx
      pg3_value_and_formula_cells.xlsx
      pg6_sheet_added_a.xlsx
      pg6_sheet_added_b.xlsx
      pg6_sheet_and_grid_change_a.xlsx
      pg6_sheet_and_grid_change_b.xlsx
      pg6_sheet_removed_a.xlsx
      pg6_sheet_removed_b.xlsx
      pg6_sheet_renamed_a.xlsx
      pg6_sheet_renamed_b.xlsx
      random_zip.zip
      sheet_case_only_rename_a.xlsx
      sheet_case_only_rename_b.xlsx
      sheet_case_only_rename_edit_a.xlsx
      sheet_case_only_rename_edit_b.xlsx
    src/
      generate.py
      __init__.py
      generators/
        base.py
        corrupt.py
        database.py
        grid.py
        mashup.py
        perf.py
        __init__.py
    templates/
      base_query.xlsx
  logs/
    2025-11-28b-diffop-pg4/
      activity_log.txt
```

## File Contents

### File: `.gitignore`

```
# Rust
target/
**/target/
**/*.rs.bk

# Python
__pycache__/
**/__pycache__/
.venv/
*.pyc
*.egg-info/

# Shared Generated Data
fixtures/generated/*.xlsx
fixtures/generated/*.pbix
fixtures/generated/*.zip
fixtures/generated/*.csv


# Docs
docs/meta/completion_estimates/
```

---

### File: `Cargo.toml`

```yaml
[workspace]
members = ["core"]
resolver = "2"
```

---

### File: `core\Cargo.toml`

```yaml
[package]
name = "excel_diff"
version = "0.1.0"
edition = "2024"

[features]
default = ["excel-open-xml"]
excel-open-xml = []

[dependencies]
quick-xml = "0.32"
thiserror = "1.0"
zip = { version = "0.6", default-features = false, features = ["deflate"] }
base64 = "0.22"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
xxhash-rust = { version = "0.8", features = ["xxh64"] }

[dev-dependencies]
```

---

### File: `core\src\addressing.rs`

```rust
/// Convert zero-based (row, col) indices to an Excel A1 address string.
pub fn index_to_address(row: u32, col: u32) -> String {
    let mut col_index = col;
    let mut col_label = String::new();

    loop {
        let rem = (col_index % 26) as u8;
        col_label.push((b'A' + rem) as char);
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }

    col_label.chars().rev().collect::<String>() + &(row + 1).to_string()
}

/// Parse an A1 address into zero-based (row, col) indices.
/// Returns `None` for malformed addresses.
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    if a1.is_empty() {
        return None;
    }

    let mut col: u32 = 0;
    let mut row: u32 = 0;
    let mut saw_letter = false;
    let mut saw_digit = false;

    for ch in a1.chars() {
        if ch.is_ascii_alphabetic() {
            saw_letter = true;
            if saw_digit {
                // Letters after digits are not allowed.
                return None;
            }
            let upper = ch.to_ascii_uppercase() as u8;
            if !upper.is_ascii_uppercase() {
                return None;
            }
            col = col
                .checked_mul(26)?
                .checked_add((upper - b'A' + 1) as u32)?;
        } else if ch.is_ascii_digit() {
            saw_digit = true;
            row = row.checked_mul(10)?.checked_add((ch as u8 - b'0') as u32)?;
        } else {
            return None;
        }
    }

    if !saw_letter || !saw_digit || row == 0 || col == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_address_examples() {
        assert_eq!(index_to_address(0, 0), "A1");
        assert_eq!(index_to_address(0, 25), "Z1");
        assert_eq!(index_to_address(0, 26), "AA1");
        assert_eq!(index_to_address(0, 27), "AB1");
        assert_eq!(index_to_address(0, 51), "AZ1");
        assert_eq!(index_to_address(0, 52), "BA1");
    }

    #[test]
    fn round_trip_addresses() {
        let addresses = [
            "A1", "B2", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1",
        ];
        for addr in addresses {
            let (r, c) = address_to_index(addr).expect("address should parse");
            assert_eq!(index_to_address(r, c), addr);
        }
    }

    #[test]
    fn invalid_addresses_rejected() {
        let invalid = ["", "1A", "A0", "A", "AA0", "A-1", "A1A"];
        for addr in invalid {
            assert!(address_to_index(addr).is_none(), "{addr} should be invalid");
        }
    }
}
```

---

### File: `core\src\container.rs`

```rust
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use zip::ZipArchive;
use zip::result::ZipError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContainerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a ZIP container")]
    NotZipContainer,
    #[error("not an OPC package (missing [Content_Types].xml)")]
    NotOpcPackage,
}

pub struct OpcContainer {
    pub(crate) archive: ZipArchive<File>,
}

impl OpcContainer {
    pub fn open(path: impl AsRef<Path>) -> Result<OpcContainer, ContainerError> {
        let file = File::open(path)?;
        let archive = ZipArchive::new(file).map_err(|err| match err {
            ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
                ContainerError::NotZipContainer
            }
            ZipError::Io(e) => ContainerError::Io(e),
            other => ContainerError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                other.to_string(),
            )),
        })?;

        let mut container = OpcContainer { archive };
        if container.read_file("[Content_Types].xml").is_err() {
            return Err(ContainerError::NotOpcPackage);
        }

        Ok(container)
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, ZipError> {
        let mut file = self.archive.by_name(name)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn read_file_optional(&mut self, name: &str) -> Result<Option<Vec<u8>>, std::io::Error> {
        match self.read_file(name) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(ZipError::FileNotFound) => Ok(None),
            Err(ZipError::Io(e)) => Err(e),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        }
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.archive.file_names()
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```

---

### File: `core\src\datamashup.rs`

```rust
use crate::datamashup_framing::{DataMashupError, RawDataMashup};
use crate::datamashup_package::{PackageParts, parse_package_parts};
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
```

---

### File: `core\src\datamashup_framing.rs`

```rust
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use quick_xml::Reader;
use quick_xml::events::Event;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DataMashupError {
    #[error("base64 decoding failed")]
    Base64Invalid,
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u32),
    #[error("invalid framing structure")]
    FramingInvalid,
    #[error("XML parse error: {0}")]
    XmlError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDataMashup {
    pub version: u32,
    pub package_parts: Vec<u8>,
    pub permissions: Vec<u8>,
    pub metadata: Vec<u8>,
    pub permission_bindings: Vec<u8>,
}

pub fn parse_data_mashup(bytes: &[u8]) -> Result<RawDataMashup, DataMashupError> {
    const MIN_SIZE: usize = 4 + 4 * 4;
    if bytes.len() < MIN_SIZE {
        return Err(DataMashupError::FramingInvalid);
    }

    let mut offset: usize = 0;
    let version = read_u32_at(bytes, offset).ok_or(DataMashupError::FramingInvalid)?;
    offset += 4;

    if version != 0 {
        return Err(DataMashupError::UnsupportedVersion(version));
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
        return Err(DataMashupError::FramingInvalid);
    }

    Ok(RawDataMashup {
        version,
        package_parts,
        permissions,
        metadata,
        permission_bindings,
    })
}

pub fn read_datamashup_text(xml: &[u8]) -> Result<Option<String>, DataMashupError> {
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
                    return Err(DataMashupError::FramingInvalid);
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t
                    .unescape()
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?
                    .into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                let data = t.into_inner();
                content.push_str(&String::from_utf8_lossy(&data));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return Err(DataMashupError::FramingInvalid);
                }
                in_datamashup = false;
                found_content = Some(content.clone());
            }
            Ok(Event::Eof) if in_datamashup => {
                return Err(DataMashupError::FramingInvalid);
            }
            Ok(Event::Eof) => return Ok(found_content),
            Err(e) => return Err(DataMashupError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
}

pub fn decode_datamashup_base64(text: &str) -> Result<Vec<u8>, DataMashupError> {
    let cleaned: String = text.split_whitespace().collect();
    STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| DataMashupError::Base64Invalid)
}

pub(crate) fn decode_datamashup_xml(xml: &[u8]) -> Result<Option<Vec<u8>>, DataMashupError> {
    if xml.starts_with(&[0xFF, 0xFE]) {
        return Ok(Some(decode_utf16_xml(xml, true, true)?));
    }
    if xml.starts_with(&[0xFE, 0xFF]) {
        return Ok(Some(decode_utf16_xml(xml, false, true)?));
    }

    decode_declared_utf16_without_bom(xml)
}

fn decode_declared_utf16_without_bom(xml: &[u8]) -> Result<Option<Vec<u8>>, DataMashupError> {
    let attempt_decode = |little_endian| -> Result<Option<Vec<u8>>, DataMashupError> {
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
) -> Result<Vec<u8>, DataMashupError> {
    let start = if has_bom { 2 } else { 0 };
    let body = xml
        .get(start..)
        .ok_or_else(|| DataMashupError::XmlError("invalid UTF-16 XML".into()))?;
    if body.len() % 2 != 0 {
        return Err(DataMashupError::XmlError(
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
        .map_err(|_| DataMashupError::XmlError("invalid UTF-16 XML".into()))?;
    Ok(utf8.into_bytes())
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    let array: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_le_bytes(array))
}

fn read_length(bytes: &[u8], offset: usize) -> Result<usize, DataMashupError> {
    let len = read_u32_at(bytes, offset).ok_or(DataMashupError::FramingInvalid)?;
    usize::try_from(len).map_err(|_| DataMashupError::FramingInvalid)
}

fn take_segment(bytes: &[u8], offset: &mut usize, len: usize) -> Result<Vec<u8>, DataMashupError> {
    let start = *offset;
    let end = start
        .checked_add(len)
        .ok_or(DataMashupError::FramingInvalid)?;
    if end > bytes.len() {
        return Err(DataMashupError::FramingInvalid);
    }

    let segment = bytes[start..end].to_vec();
    *offset = end;
    Ok(segment)
}

#[cfg(test)]
mod tests {
    use super::{
        DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup,
        read_datamashup_text,
    };

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
        assert!(matches!(err, DataMashupError::UnsupportedVersion(1)));
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
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn trailing_bytes_are_invalid() {
        let mut bytes = build_dm_bytes(0, b"", b"", b"", b"");
        bytes.push(0xFF);
        let err = parse_data_mashup(&bytes).expect_err("trailing bytes should fail");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn too_short_stream_is_framing_invalid() {
        let bytes = vec![0u8; 8];
        let err =
            parse_data_mashup(&bytes).expect_err("buffer shorter than header must be invalid");
        assert!(matches!(err, DataMashupError::FramingInvalid));
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
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn decode_datamashup_base64_rejects_invalid() {
        let err = decode_datamashup_base64("!!!").expect_err("invalid base64 should fail");
        assert!(matches!(err, DataMashupError::Base64Invalid));
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
```

---

### File: `core\src\datamashup_package.rs`

```rust
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
```

---

### File: `core\src\diff.rs`

```rust
use crate::workbook::{CellAddress, CellSnapshot, ColSignature, RowSignature};

pub type SheetId = String;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum DiffOp {
    SheetAdded {
        sheet: SheetId,
    },
    SheetRemoved {
        sheet: SheetId,
    },
    RowAdded {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    RowRemoved {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    ColumnAdded {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    ColumnRemoved {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    BlockMovedRows {
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    BlockMovedColumns {
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    /// Logical change to a single cell.
    ///
    /// Invariants (maintained by producers and tests, not by the type system):
    /// - `addr` is the canonical location for the edit.
    /// - `from.addr` and `to.addr` must both equal `addr`.
    /// - `CellSnapshot` equality intentionally ignores `addr` and compares only
    ///   `(value, formula)`, so `DiffOp::CellEdited` equality does not by itself
    ///   enforce the address invariants; callers must respect them when
    ///   constructing ops.
    CellEdited {
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffReport {
    pub version: String,
    pub ops: Vec<DiffOp>,
}

impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            ops,
        }
    }
}

impl DiffOp {
    pub fn cell_edited(
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    ) -> DiffOp {
        debug_assert_eq!(from.addr, addr, "from.addr must match canonical addr");
        debug_assert_eq!(to.addr, addr, "to.addr must match canonical addr");
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        }
    }

    pub fn row_added(sheet: SheetId, row_idx: u32, row_signature: Option<RowSignature>) -> DiffOp {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn row_removed(
        sheet: SheetId,
        row_idx: u32,
        row_signature: Option<RowSignature>,
    ) -> DiffOp {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn column_added(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        }
    }

    pub fn column_removed(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        }
    }

    pub fn block_moved_rows(
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        }
    }

    pub fn block_moved_columns(
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        }
    }
}
```

---

### File: `core\src\engine.rs`

```rust
use crate::diff::{DiffOp, DiffReport, SheetId};
use crate::workbook::{CellAddress, CellSnapshot, Grid, Sheet, SheetKind, Workbook};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

fn make_sheet_key(sheet: &Sheet) -> SheetKey {
    SheetKey {
        name_lower: sheet.name.to_lowercase(),
        kind: sheet.kind.clone(),
    }
}

fn sheet_kind_order(kind: &SheetKind) -> u8 {
    match kind {
        SheetKind::Worksheet => 0,
        SheetKind::Chart => 1,
        SheetKind::Macro => 2,
        SheetKind::Other => 3,
    }
}

pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport {
    let mut ops = Vec::new();

    let mut old_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        let key = make_sheet_key(sheet);
        let was_unique = old_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in old workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut new_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        let key = make_sheet_key(sheet);
        let was_unique = new_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in new workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut all_keys: Vec<SheetKey> = old_sheets
        .keys()
        .chain(new_sheets.keys())
        .cloned()
        .collect();
    all_keys.sort_by(|a, b| match a.name_lower.cmp(&b.name_lower) {
        std::cmp::Ordering::Equal => sheet_kind_order(&a.kind).cmp(&sheet_kind_order(&b.kind)),
        other => other,
    });
    all_keys.dedup();

    for key in all_keys {
        match (old_sheets.get(&key), new_sheets.get(&key)) {
            (None, Some(new_sheet)) => {
                ops.push(DiffOp::SheetAdded {
                    sheet: new_sheet.name.clone(),
                });
            }
            (Some(old_sheet), None) => {
                ops.push(DiffOp::SheetRemoved {
                    sheet: old_sheet.name.clone(),
                });
            }
            (Some(old_sheet), Some(new_sheet)) => {
                let sheet_id: SheetId = old_sheet.name.clone();
                diff_grids(&sheet_id, &old_sheet.grid, &new_sheet.grid, &mut ops);
            }
            (None, None) => unreachable!(),
        }
    }

    DiffReport::new(ops)
}

fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    for row in 0..overlap_rows {
        for col in 0..overlap_cols {
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            let old_snapshot = old_cell.map(CellSnapshot::from_cell);
            let new_snapshot = new_cell.map(CellSnapshot::from_cell);

            if old_snapshot != new_snapshot {
                let addr = CellAddress::from_indices(row, col);
                let from = old_snapshot.unwrap_or_else(|| CellSnapshot::empty(addr));
                let to = new_snapshot.unwrap_or_else(|| CellSnapshot::empty(addr));

                ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
            }
        }
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            ops.push(DiffOp::row_added(sheet_id.clone(), row_idx, None));
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            ops.push(DiffOp::row_removed(sheet_id.clone(), row_idx, None));
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            ops.push(DiffOp::column_added(sheet_id.clone(), col_idx, None));
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            ops.push(DiffOp::column_removed(sheet_id.clone(), col_idx, None));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_kind_order_ranking_includes_macro_and_other() {
        assert!(
            sheet_kind_order(&SheetKind::Worksheet) < sheet_kind_order(&SheetKind::Chart),
            "Worksheet should rank before Chart"
        );
        assert!(
            sheet_kind_order(&SheetKind::Chart) < sheet_kind_order(&SheetKind::Macro),
            "Chart should rank before Macro"
        );
        assert!(
            sheet_kind_order(&SheetKind::Macro) < sheet_kind_order(&SheetKind::Other),
            "Macro should rank before Other"
        );
    }
}
```

---

### File: `core\src\excel_open_xml.rs`

```rust
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
                .map_err(|e| ContainerError::Io(std::io::Error::other(e.to_string())))?;

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
```

---

### File: `core\src\grid_parser.rs`

```rust
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
```

---

### File: `core\src\lib.rs`

```rust
pub mod addressing;
pub mod container;
pub mod datamashup;
pub mod datamashup_framing;
pub mod datamashup_package;
pub mod diff;
pub mod engine;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod grid_parser;
pub mod m_section;
pub mod output;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
pub use container::{ContainerError, OpcContainer};
pub use datamashup::{DataMashup, Metadata, Permissions, QueryMetadata, build_data_mashup};
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use datamashup_package::{
    EmbeddedContent, PackageParts, PackageXml, SectionDocument, parse_package_parts,
};
pub use diff::{DiffOp, DiffReport, SheetId};
pub use engine::diff_workbooks;
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, open_data_mashup, open_workbook};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[cfg(feature = "excel-open-xml")]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ColSignature, Grid, RowSignature, Sheet, SheetKind,
    Workbook,
};
```

---

### File: `core\src\main.rs`

```rust
fn main() {
    println!("Hello, world!");
}
```

---

### File: `core\src\m_section.rs`

```rust
use std::str::Lines;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SectionParseError {
    #[error("missing section header")]
    MissingSectionHeader,
    #[error("invalid section header")]
    InvalidHeader,
    #[error("invalid member syntax")]
    InvalidMemberSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionMember {
    pub section_name: String,
    pub member_name: String,
    pub expression_m: String,
    pub is_shared: bool,
}

pub fn parse_section_members(source: &str) -> Result<Vec<SectionMember>, SectionParseError> {
    let source = strip_leading_bom(source);
    let mut lines = source.lines();
    let section_name = find_section_name(&mut lines)?;

    let mut members = Vec::new();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if !trimmed.starts_with("shared") {
            continue;
        }

        if let Some(member) = parse_shared_member(trimmed, &mut lines, &section_name) {
            members.push(member);
        }
    }

    Ok(members)
}

fn find_section_name(lines: &mut Lines<'_>) -> Result<String, SectionParseError> {
    for line in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        match try_parse_section_header(trimmed) {
            Ok(Some(name)) => return Ok(name),
            Ok(None) => continue,
            Err(err) => return Err(err),
        }
    }

    Err(SectionParseError::MissingSectionHeader)
}

fn try_parse_section_header(line: &str) -> Result<Option<String>, SectionParseError> {
    let Some(rest) = line.strip_prefix("section") else {
        return Ok(None);
    };

    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let header_body = rest.trim_start();
    if !header_body.ends_with(';') {
        return Err(SectionParseError::InvalidHeader);
    }

    let without_semicolon = &header_body[..header_body.len() - 1];
    let name_candidate = without_semicolon.trim();
    if name_candidate.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let mut parts = name_candidate.split_whitespace();
    let name = parts.next().ok_or(SectionParseError::InvalidHeader)?;
    if parts.next().is_some() {
        return Err(SectionParseError::InvalidHeader);
    }

    if !is_valid_identifier(name) {
        return Err(SectionParseError::InvalidHeader);
    }

    Ok(Some(name.to_string()))
}

fn parse_shared_member(
    line: &str,
    remaining_lines: &mut Lines<'_>,
    section_name: &str,
) -> Option<SectionMember> {
    let rest = line.strip_prefix("shared")?;
    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return None;
    }

    let body = rest.trim_start();
    if body.is_empty() {
        return None;
    }

    let (member_name, after_name) = split_identifier(body)?;
    if !is_valid_identifier(member_name) {
        return None;
    }

    let mut expression_source = after_name;
    let eq_index = expression_source.find('=')?;
    if !expression_source[..eq_index].trim().is_empty() {
        return None;
    }
    expression_source = &expression_source[eq_index + 1..];

    let mut expression = expression_source.to_string();
    if let Some(idx) = expression_source.find(';') {
        expression.truncate(idx);
    } else {
        let mut terminator_index = None;
        while terminator_index.is_none() {
            let Some(next_line) = remaining_lines.next() else {
                break;
            };

            expression.push('\n');
            let offset = expression.len();
            expression.push_str(next_line);
            if let Some(idx) = next_line.find(';') {
                terminator_index = Some(offset + idx);
            }
        }

        if let Some(idx) = terminator_index {
            expression.truncate(idx);
        } else {
            return None;
        }
    }

    let expression_m = expression.trim().to_string();

    Some(SectionMember {
        section_name: section_name.to_string(),
        member_name: member_name.to_string(),
        expression_m,
        is_shared: true,
    })
}

fn split_identifier(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    let mut end = 0;
    for ch in trimmed.chars() {
        if ch.is_whitespace() || ch == '=' {
            break;
        }
        end += ch.len_utf8();
    }

    if end == 0 {
        return None;
    }

    Some(trimmed.split_at(end))
}

fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn strip_leading_bom(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}
```

---

### File: `core\src\workbook.rs`

```rust
use crate::addressing::{address_to_index, index_to_address};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use xxhash_rust::xxh64::Xxh64;

/// A snapshot of a cell's logical content (address, value, formula).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CellSnapshot {
    pub addr: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

impl CellSnapshot {
    pub fn from_cell(cell: &Cell) -> CellSnapshot {
        CellSnapshot {
            addr: cell.address,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
        }
    }

    pub fn empty(addr: CellAddress) -> CellSnapshot {
        CellSnapshot {
            addr,
            value: None,
            formula: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub kind: SheetKind,
    pub grid: Grid,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SheetKind {
    Worksheet,
    Chart,
    Macro,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
/// Invariant: all cells stored in `cells` must satisfy `row < nrows` and `col < ncols`.
pub struct Grid {
    pub nrows: u32,
    pub ncols: u32,
    pub cells: HashMap<(u32, u32), Cell>,
    pub row_signatures: Option<Vec<RowSignature>>,
    pub col_signatures: Option<Vec<ColSignature>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    pub row: u32,
    pub col: u32,
}

impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> CellAddress {
        CellAddress { row, col }
    }

    pub fn to_a1(&self) -> String {
        index_to_address(self.row, self.col)
    }
}

impl std::str::FromStr for CellAddress {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (row, col) = address_to_index(s).ok_or(())?;
        Ok(CellAddress { row, col })
    }
}

impl std::fmt::Display for CellAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

impl Serialize for CellAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_a1())
    }
}

impl<'de> Deserialize<'de> for CellAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let a1 = String::deserialize(deserializer)?;
        CellAddress::from_str(&a1)
            .map_err(|_| DeError::custom(format!("invalid cell address: {a1}")))
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Number(f64),
    Text(String),
    Bool(bool),
}

impl Hash for CellValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CellValue::Number(n) => {
                0u8.hash(state);
                n.to_bits().hash(state);
            }
            CellValue::Text(s) => {
                1u8.hash(state);
                s.hash(state);
            }
            CellValue::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowSignature {
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ColSignature {
    pub hash: u64,
}

const XXH64_SEED: u64 = 0;
const HASH_MIX_CONSTANT: u64 = 0x9e3779b97f4a7c15;

impl Grid {
    pub fn new(nrows: u32, ncols: u32) -> Grid {
        Grid {
            nrows,
            ncols,
            cells: HashMap::new(),
            row_signatures: None,
            col_signatures: None,
        }
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells.get(&(row, col))
    }

    pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut Cell> {
        self.cells.get_mut(&(row, col))
    }

    pub fn insert(&mut self, cell: Cell) {
        debug_assert!(
            cell.row < self.nrows && cell.col < self.ncols,
            "cell coordinates must lie within the grid bounds"
        );
        self.cells.insert((cell.row, cell.col), cell);
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = &Cell> {
        self.cells.values()
    }

    pub fn rows_iter(&self) -> impl Iterator<Item = u32> + '_ {
        0..self.nrows
    }

    pub fn cols_iter(&self) -> impl Iterator<Item = u32> + '_ {
        0..self.ncols
    }

    pub fn compute_row_signature(&self, row: u32) -> RowSignature {
        let hash = self
            .cells
            .values()
            .filter(|cell| cell.row == row)
            .fold(0u64, |acc, cell| {
                combine_hashes(acc, hash_cell_contribution(cell.col, cell))
            });
        RowSignature { hash }
    }

    pub fn compute_col_signature(&self, col: u32) -> ColSignature {
        let hash = self
            .cells
            .values()
            .filter(|cell| cell.col == col)
            .fold(0u64, |acc, cell| {
                combine_hashes(acc, hash_cell_contribution(cell.row, cell))
            });
        ColSignature { hash }
    }

    pub fn compute_all_signatures(&mut self) {
        let mut row_hashes = vec![0u64; self.nrows as usize];
        let mut col_hashes = vec![0u64; self.ncols as usize];

        for cell in self.cells.values() {
            let row_idx = cell.row as usize;
            let col_idx = cell.col as usize;

            debug_assert!(
                row_idx < row_hashes.len() && col_idx < col_hashes.len(),
                "cell coordinates must lie within the grid bounds"
            );

            let row_contribution = hash_cell_contribution(cell.col, cell);
            row_hashes[row_idx] = combine_hashes(row_hashes[row_idx], row_contribution);

            let col_contribution = hash_cell_contribution(cell.row, cell);
            col_hashes[col_idx] = combine_hashes(col_hashes[col_idx], col_contribution);
        }

        self.row_signatures = Some(
            row_hashes
                .into_iter()
                .map(|hash| RowSignature { hash })
                .collect(),
        );

        self.col_signatures = Some(
            col_hashes
                .into_iter()
                .map(|hash| ColSignature { hash })
                .collect(),
        );
    }
}

fn hash_cell_contribution(position: u32, cell: &Cell) -> u64 {
    let mut hasher = Xxh64::new(XXH64_SEED);
    position.hash(&mut hasher);
    cell.value.hash(&mut hasher);
    cell.formula.hash(&mut hasher);
    hasher.finish()
}

fn mix_hash(hash: u64) -> u64 {
    hash.rotate_left(13) ^ HASH_MIX_CONSTANT
}

fn combine_hashes(current: u64, contribution: u64) -> u64 {
    current.wrapping_add(mix_hash(contribution))
}

impl PartialEq for CellSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.formula == other.formula
    }
}

impl Eq for CellSnapshot {}

impl CellValue {
    pub fn as_text(&self) -> Option<&str> {
        if let CellValue::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if let CellValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let CellValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(a1: &str) -> CellAddress {
        a1.parse().expect("address should parse")
    }

    fn make_cell(address: &str, value: Option<CellValue>, formula: Option<&str>) -> Cell {
        let (row, col) = address_to_index(address).expect("address should parse");
        Cell {
            row,
            col,
            address: CellAddress::from_indices(row, col),
            value,
            formula: formula.map(|s| s.to_string()),
        }
    }

    #[test]
    fn snapshot_from_number_cell() {
        let cell = make_cell("A1", Some(CellValue::Number(42.0)), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "A1");
        assert_eq!(snap.value, Some(CellValue::Number(42.0)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_text_cell() {
        let cell = make_cell("B2", Some(CellValue::Text("hello".into())), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "B2");
        assert_eq!(snap.value, Some(CellValue::Text("hello".into())));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_bool_cell() {
        let cell = make_cell("C3", Some(CellValue::Bool(true)), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "C3");
        assert_eq!(snap.value, Some(CellValue::Bool(true)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_empty_cell() {
        let cell = make_cell("D4", None, None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "D4");
        assert!(snap.value.is_none());
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_equality_same_value_and_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(1.0)),
            formula: Some("A1+1".into()),
        };
        let snap2 = CellSnapshot {
            addr: addr("B2"),
            value: Some(CellValue::Number(1.0)),
            formula: Some("A1+1".into()),
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_different_value_same_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(43.0)),
            formula: Some("A1+1".into()),
        };
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(44.0)),
            formula: Some("A1+1".into()),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_value_vs_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: None,
        };
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: Some("A1+1".into()),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_equality_ignores_address() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Text("hello".into())),
            formula: None,
        };
        let snap2 = CellSnapshot {
            addr: addr("Z9"),
            value: Some(CellValue::Text("hello".into())),
            formula: None,
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn cellvalue_as_text_number_bool_match_variants() {
        let text = CellValue::Text("abc".into());
        let number = CellValue::Number(5.0);
        let boolean = CellValue::Bool(true);

        assert_eq!(text.as_text(), Some("abc"));
        assert_eq!(text.as_number(), None);
        assert_eq!(text.as_bool(), None);

        assert_eq!(number.as_text(), None);
        assert_eq!(number.as_number(), Some(5.0));
        assert_eq!(number.as_bool(), None);

        assert_eq!(boolean.as_text(), None);
        assert_eq!(boolean.as_number(), None);
        assert_eq!(boolean.as_bool(), Some(true));
    }
}
```

---

### File: `core\src\output\json.rs`

```rust
use crate::diff::DiffReport;
#[cfg(feature = "excel-open-xml")]
use crate::engine::diff_workbooks as compute_diff;
#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{ExcelOpenError, open_workbook};
use serde::Serialize;
use serde::ser::Error as SerdeError;
#[cfg(feature = "excel-open-xml")]
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellDiff {
    #[serde(rename = "coords")]
    pub coords: String,
    #[serde(rename = "value_file1")]
    pub value_file1: Option<String>,
    #[serde(rename = "value_file2")]
    pub value_file2: Option<String>,
}

pub fn serialize_cell_diffs(diffs: &[CellDiff]) -> serde_json::Result<String> {
    serde_json::to_string(diffs)
}

pub fn serialize_diff_report(report: &DiffReport) -> serde_json::Result<String> {
    if contains_non_finite_numbers(report) {
        return Err(SerdeError::custom(
            "non-finite numbers (NaN or infinity) are not supported in DiffReport JSON output",
        ));
    }
    serde_json::to_string(report)
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<DiffReport, ExcelOpenError> {
    let wb_a = open_workbook(path_a)?;
    let wb_b = open_workbook(path_b)?;
    Ok(compute_diff(&wb_a, &wb_b))
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<String, ExcelOpenError> {
    let report = diff_workbooks(path_a, path_b)?;
    serialize_diff_report(&report).map_err(|e| ExcelOpenError::SerializationError(e.to_string()))
}

pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn render_value(value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(s)) => Some(s.clone()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            None => None,
        }
    }

    report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                Some(CellDiff {
                    coords: addr.to_a1(),
                    value_file1: render_value(&from.value),
                    value_file2: render_value(&to.value),
                })
            } else {
                None
            }
        })
        .collect()
}

fn contains_non_finite_numbers(report: &DiffReport) -> bool {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    report.ops.iter().any(|op| match op {
        DiffOp::CellEdited { from, to, .. } => {
            matches!(from.value, Some(CellValue::Number(n)) if !n.is_finite())
                || matches!(to.value, Some(CellValue::Number(n)) if !n.is_finite())
        }
        _ => false,
    })
}
```

---

### File: `core\src\output\mod.rs`

```rust
pub mod json;
```

---

### File: `core\tests\addressing_pg2_tests.rs`

```rust
use excel_diff::{CellValue, address_to_index, index_to_address, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg2_addressing_matrix_consistency() {
    let workbook =
        open_workbook(fixture_path("pg2_addressing_matrix.xlsx")).expect("address fixture opens");
    let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == "Addresses")
        .unwrap_or_else(|| panic!("Addresses sheet present; found {:?}", sheet_names));

    for cell in sheet.grid.iter_cells() {
        if let Some(CellValue::Text(text)) = &cell.value {
            assert_eq!(cell.address.to_a1(), text.as_str());
            let (r, c) = address_to_index(text).expect("address strings should parse to indices");
            assert_eq!((r, c), (cell.row, cell.col));
            assert_eq!(index_to_address(cell.row, cell.col), cell.address.to_a1());
        }
    }
}
```

---

### File: `core\tests\data_mashup_tests.rs`

```rust
use std::fs::File;
use std::io::{ErrorKind, Read};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use excel_diff::{
    ContainerError, DataMashupError, ExcelOpenError, RawDataMashup, build_data_mashup,
    open_data_mashup,
};
use quick_xml::{Reader, events::Event};
use zip::ZipArchive;

mod common;
use common::fixture_path;

#[test]
fn workbook_without_datamashup_returns_none() {
    let path = fixture_path("minimal.xlsx");
    let result = open_data_mashup(&path).expect("minimal workbook should load");
    assert!(result.is_none());
}

#[test]
fn workbook_with_valid_datamashup_parses() {
    let path = fixture_path("m_change_literal_b.xlsx");
    let raw = open_data_mashup(&path)
        .expect("valid mashup should load")
        .expect("mashup should be present");

    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
    assert!(!raw.metadata.is_empty());

    let assembled = assemble_top_level_bytes(&raw);
    let expected = datamashup_bytes_from_fixture(&path);
    assert_eq!(assembled, expected);
}

#[test]
fn datamashup_with_base64_whitespace_parses() {
    let path = fixture_path("mashup_base64_whitespace.xlsx");
    let raw = open_data_mashup(&path)
        .expect("whitespace in base64 payload should be tolerated")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_le_datamashup_parses() {
    let path = fixture_path("mashup_utf16_le.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16LE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_be_datamashup_parses() {
    let path = fixture_path("mashup_utf16_be.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16BE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn corrupt_base64_returns_error() {
    let path = fixture_path("corrupt_base64.xlsx");
    let err = open_data_mashup(&path).expect_err("corrupt base64 should fail");
    assert!(matches!(
        err,
        ExcelOpenError::DataMashup(DataMashupError::Base64Invalid)
    ));
}

#[test]
fn duplicate_datamashup_parts_are_rejected() {
    let path = fixture_path("duplicate_datamashup_parts.xlsx");
    let err = open_data_mashup(&path).expect_err("duplicate DataMashup parts should be rejected");
    assert!(matches!(
        err,
        ExcelOpenError::DataMashup(DataMashupError::FramingInvalid)
    ));
}

#[test]
fn duplicate_datamashup_elements_are_rejected() {
    let path = fixture_path("duplicate_datamashup_elements.xlsx");
    let err =
        open_data_mashup(&path).expect_err("duplicate DataMashup elements should be rejected");
    assert!(matches!(
        err,
        ExcelOpenError::DataMashup(DataMashupError::FramingInvalid)
    ));
}

#[test]
fn nonexistent_file_returns_io() {
    let path = fixture_path("missing_mashup.xlsx");
    let err = open_data_mashup(&path).expect_err("missing file should error");
    match err {
        ExcelOpenError::Container(ContainerError::Io(e)) => {
            assert_eq!(e.kind(), ErrorKind::NotFound)
        }
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn non_excel_container_returns_not_excel_error() {
    let path = fixture_path("random_zip.zip");
    let err = open_data_mashup(&path).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn missing_content_types_is_not_excel_error() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_data_mashup(&path).expect_err("missing [Content_Types].xml should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn non_zip_file_returns_not_zip_error() {
    let path = fixture_path("not_a_zip.txt");
    let err = open_data_mashup(&path).expect_err("non-zip input should not parse as Excel");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotZipContainer)
    ));
}

#[test]
fn build_data_mashup_smoke_from_fixture() {
    let raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    let dm = build_data_mashup(&raw).expect("build_data_mashup should succeed");

    assert_eq!(dm.version, 0);
    assert!(
        dm.package_parts
            .main_section
            .source
            .contains("section Section1;")
    );
    assert!(!dm.metadata.formulas.is_empty());

    let non_connection: Vec<_> = dm
        .metadata
        .formulas
        .iter()
        .filter(|m| m.section_name == "Section1" && !m.is_connection_only)
        .collect();
    assert_eq!(non_connection.len(), 1);
    let meta = non_connection[0];
    assert_eq!(
        meta.item_path,
        format!("{}/{}", meta.section_name, meta.formula_name)
    );
    assert_eq!(meta.item_path, "Section1/Query1");
    assert_eq!(meta.section_name, "Section1");
    assert_eq!(meta.formula_name, "Query1");
    assert!(meta.load_to_sheet || meta.load_to_model);
}

fn datamashup_bytes_from_fixture(path: &std::path::Path) -> Vec<u8> {
    let file = File::open(path).expect("fixture should be readable");
    let mut archive = ZipArchive::new(file).expect("fixture should be a zip container");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip entry should be readable");
        let name = file.name().to_string();
        if !name.starts_with("customXml/") || !name.ends_with(".xml") {
            continue;
        }

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("XML part should read");
        if let Some(text) = extract_datamashup_base64(&buf) {
            let cleaned: String = text.split_whitespace().collect();
            return STANDARD
                .decode(cleaned.as_bytes())
                .expect("DataMashup base64 should decode");
        }
    }

    panic!("DataMashup element not found in {}", path.display());
}

fn extract_datamashup_base64(xml: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup {
                    return None;
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t.unescape().ok()?.into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                content.push_str(&String::from_utf8_lossy(&t.into_inner()));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return None;
                }
                return Some(content.clone());
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn assemble_top_level_bytes(raw: &RawDataMashup) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&raw.version.to_le_bytes());
    bytes.extend_from_slice(&(raw.package_parts.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.package_parts);
    bytes.extend_from_slice(&(raw.permissions.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permissions);
    bytes.extend_from_slice(&(raw.metadata.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.metadata);
    bytes.extend_from_slice(&(raw.permission_bindings.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permission_bindings);
    bytes
}
```

---

### File: `core\tests\engine_tests.rs`

```rust
use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, DiffOp, DiffReport, Grid, Sheet, SheetKind,
    Workbook, diff_workbooks,
};

type SheetSpec<'a> = (&'a str, Vec<(u32, u32, f64)>);

fn make_workbook(sheets: Vec<SheetSpec<'_>>) -> Workbook {
    let sheet_ir: Vec<Sheet> = sheets
        .into_iter()
        .map(|(name, cells)| {
            let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
            let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
            let mut grid = Grid::new(max_row + 1, max_col + 1);
            for (r, c, val) in cells {
                grid.insert(Cell {
                    row: r,
                    col: c,
                    address: CellAddress::from_indices(r, c),
                    value: Some(CellValue::Number(val)),
                    formula: None,
                });
            }
            Sheet {
                name: name.to_string(),
                kind: SheetKind::Worksheet,
                grid,
            }
        })
        .collect();
    Workbook { sheets: sheet_ir }
}

fn make_sheet_with_kind(name: &str, kind: SheetKind, cells: Vec<(u32, u32, f64)>) -> Sheet {
    let (nrows, ncols) = if cells.is_empty() {
        (0, 0)
    } else {
        let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
        let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
        (max_row + 1, max_col + 1)
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, c, val) in cells {
        grid.insert(Cell {
            row: r,
            col: c,
            address: CellAddress::from_indices(r, c),
            value: Some(CellValue::Number(val)),
            formula: None,
        });
    }

    Sheet {
        name: name.to_string(),
        kind,
        grid,
    }
}

#[test]
fn identical_workbooks_produce_empty_report() {
    let wb = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&wb, &wb);
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_added_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let report = diff_workbooks(&old, &new);
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetAdded { sheet } if sheet == "Sheet2"))
    );
}

#[test]
fn sheet_removed_detected() {
    let old = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&old, &new);
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetRemoved { sheet } if sheet == "Sheet2"))
    );
}

#[test]
fn cell_edited_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        _ => panic!("expected CellEdited"),
    }
}

#[test]
fn diff_report_json_round_trips() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new);
    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed);
}

#[test]
fn sheet_name_case_insensitive_no_changes() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 1.0)])]);

    let report = diff_workbooks(&old, &new);
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_name_case_insensitive_cell_edit() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 2.0)])]);

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn sheet_identity_includes_kind() {
    let mut grid = Grid::new(1, 1);
    grid.insert(Cell {
        row: 0,
        col: 0,
        address: CellAddress::from_indices(0, 0),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    let worksheet = Sheet {
        name: "Sheet1".to_string(),
        kind: SheetKind::Worksheet,
        grid: grid.clone(),
    };

    let chart = Sheet {
        name: "Sheet1".to_string(),
        kind: SheetKind::Chart,
        grid,
    };

    let old = Workbook {
        sheets: vec![worksheet],
    };
    let new = Workbook {
        sheets: vec![chart],
    };

    let report = diff_workbooks(&old, &new);

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Sheet1" => added += 1,
            DiffOp::SheetRemoved { sheet } if sheet == "Sheet1" => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Chart 'Sheet1'");
    assert_eq!(
        removed, 1,
        "expected one SheetRemoved for Worksheet 'Sheet1'"
    );
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[test]
fn deterministic_sheet_op_ordering() {
    let budget_old = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let budget_new = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 2.0)]);
    let sheet1_old = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 1, 5.0)]);
    let sheet1_chart = make_sheet_with_kind("sheet1", SheetKind::Chart, Vec::new());
    let summary_new = make_sheet_with_kind("Summary", SheetKind::Worksheet, vec![(0, 0, 3.0)]);

    let old = Workbook {
        sheets: vec![budget_old.clone(), sheet1_old],
    };
    let new = Workbook {
        sheets: vec![budget_new.clone(), sheet1_chart, summary_new],
    };

    let budget_addr = CellAddress::from_indices(0, 0);
    let expected = vec![
        DiffOp::cell_edited(
            "Budget".into(),
            budget_addr,
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(1.0)),
                formula: None,
            },
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(2.0)),
                formula: None,
            },
        ),
        DiffOp::SheetRemoved {
            sheet: "Sheet1".into(),
        },
        DiffOp::SheetAdded {
            sheet: "sheet1".into(),
        },
        DiffOp::SheetAdded {
            sheet: "Summary".into(),
        },
    ];

    let report = diff_workbooks(&old, &new);
    assert_eq!(
        report.ops, expected,
        "ops should be ordered by lowercase name then sheet kind"
    );
}

#[test]
fn sheet_identity_includes_kind_for_macro_and_other() {
    let mut grid = Grid::new(1, 1);
    grid.insert(Cell {
        row: 0,
        col: 0,
        address: CellAddress::from_indices(0, 0),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    let macro_sheet = Sheet {
        name: "Code".to_string(),
        kind: SheetKind::Macro,
        grid: grid.clone(),
    };

    let other_sheet = Sheet {
        name: "Code".to_string(),
        kind: SheetKind::Other,
        grid,
    };

    let old = Workbook {
        sheets: vec![macro_sheet],
    };
    let new = Workbook {
        sheets: vec![other_sheet],
    };

    let report = diff_workbooks(&old, &new);

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Code" => added += 1,
            DiffOp::SheetRemoved { sheet } if sheet == "Code" => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Other 'Code'");
    assert_eq!(removed, 1, "expected one SheetRemoved for Macro 'Code'");
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[cfg(not(debug_assertions))]
#[test]
fn duplicate_sheet_identity_last_writer_wins_release() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);

    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1, "expected last writer to win");

    match &report.ops[0] {
        DiffOp::SheetRemoved { sheet } => assert_eq!(
            sheet, "sheet1",
            "duplicate identity should prefer the last sheet in release builds"
        ),
        other => panic!("expected SheetRemoved, got {other:?}"),
    }
}

#[test]
fn duplicate_sheet_identity_panics_in_debug() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);
    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let result = std::panic::catch_unwind(|| diff_workbooks(&old, &new));
    if cfg!(debug_assertions) {
        assert!(
            result.is_err(),
            "duplicate sheet identities should trigger a debug assertion"
        );
    } else {
        assert!(result.is_ok(), "debug assertions disabled should not panic");
    }
}
```

---

### File: `core\tests\excel_open_xml_tests.rs`

```rust
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::time::SystemTime;

use excel_diff::{ContainerError, ExcelOpenError, SheetKind, open_workbook};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

mod common;
use common::fixture_path;

fn temp_xlsx_path(prefix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    path.push(format!("excel_diff_{prefix}_{nanos}.xlsx"));
    path
}

fn write_zip(entries: &[(&str, &str)], path: &Path) {
    let file = fs::File::create(path).expect("create temp zip");
    let mut writer = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, contents) in entries {
        writer.start_file(*name, options).expect("start zip entry");
        writer
            .write_all(contents.as_bytes())
            .expect("write zip entry");
    }

    writer.finish().expect("finish zip");
}

#[test]
fn open_minimal_workbook_succeeds() {
    let path = fixture_path("minimal.xlsx");
    let workbook = open_workbook(&path).expect("minimal workbook should open");
    assert_eq!(workbook.sheets.len(), 1);

    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.name, "Sheet1");
    assert!(matches!(sheet.kind, SheetKind::Worksheet));
    assert_eq!(sheet.grid.nrows, 1);
    assert_eq!(sheet.grid.ncols, 1);

    let cell = sheet.grid.get(0, 0).expect("A1 should be present");
    assert_eq!(cell.address.to_a1(), "A1");
    assert!(cell.value.is_some());
}

#[test]
fn open_nonexistent_file_returns_io_error() {
    let path = fixture_path("definitely_missing.xlsx");
    let err = open_workbook(&path).expect_err("missing file should error");
    match err {
        ExcelOpenError::Container(ContainerError::Io(e)) => {
            assert_eq!(e.kind(), ErrorKind::NotFound)
        }
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn random_zip_is_not_excel() {
    let path = fixture_path("random_zip.zip");
    let err = open_workbook(&path).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn no_content_types_is_not_excel() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_workbook(&path).expect_err("missing content types should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn not_zip_container_returns_error() {
    let path = std::env::temp_dir().join("excel_diff_not_zip.txt");
    fs::write(&path, "this is not a zip container").expect("write temp file");
    let err = open_workbook(&path).expect_err("non-zip should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotZipContainer)
    ));
    let _ = fs::remove_file(&path);
}

#[test]
fn missing_workbook_xml_returns_workbookxmlmissing() {
    let path = temp_xlsx_path("missing_workbook_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    write_zip(&[("[Content_Types].xml", content_types)], &path);

    let err = open_workbook(&path).expect_err("missing workbook xml should error");
    assert!(matches!(err, ExcelOpenError::WorkbookXmlMissing));

    let _ = fs::remove_file(&path);
}

#[test]
fn missing_worksheet_xml_returns_worksheetxmlmissing() {
    let path = temp_xlsx_path("missing_worksheet_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    let workbook_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;

    let relationships = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
                Target="worksheets/sheet1.xml"/>
</Relationships>"#;

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("xl/workbook.xml", workbook_xml),
            ("xl/_rels/workbook.xml.rels", relationships),
        ],
        &path,
    );

    let err = open_workbook(&path).expect_err("missing worksheet xml should error");
    match err {
        ExcelOpenError::WorksheetXmlMissing { sheet_name } => {
            assert_eq!(sheet_name, "Sheet1");
        }
        other => panic!("expected WorksheetXmlMissing, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}
```

---

### File: `core\tests\integration_test.rs`

```rust
use std::path::PathBuf;

fn get_fixture_path(filename: &str) -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up one level from 'core', then into 'fixtures/generated'
    d.push("../fixtures/generated");
    d.push(filename);
    d
}

#[test]
fn test_locate_fixture() {
    let path = get_fixture_path("minimal.xlsx");
    // This test confirms that the Rust code can locate the Python-generated fixtures
    // using the relative path strategy from the monorepo root.
    assert!(
        path.exists(),
        "Fixture minimal.xlsx should exist at {:?}",
        path
    );
}
```

---

### File: `core\tests\m4_package_parts_tests.rs`

```rust
use std::io::{Cursor, Write};

use excel_diff::{DataMashupError, open_data_mashup, parse_package_parts, parse_section_members};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

mod common;
use common::fixture_path;

const MIN_PACKAGE_XML: &str = "<Package></Package>";
const MIN_SECTION: &str = "section Section1;\nshared Foo = 1;";
const BOM_SECTION: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

#[test]
fn package_parts_contains_expected_entries() {
    let path = fixture_path("one_query.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(
        parts.main_section.source.contains("section Section1;"),
        "main Section1.m should be present"
    );
    assert!(
        parts.main_section.source.contains("shared"),
        "at least one shared query should be present"
    );
    assert!(
        parts.embedded_contents.is_empty(),
        "one_query.xlsx should not contain embedded contents"
    );
}

#[test]
fn embedded_content_detection() {
    let path = fixture_path("multi_query_with_embedded.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(
        !parts.embedded_contents.is_empty(),
        "multi_query_with_embedded.xlsx should expose at least one embedded content"
    );

    for embedded in &parts.embedded_contents {
        assert!(
            embedded.section.source.contains("section Section1"),
            "embedded Section1.m should be present for {}",
            embedded.name
        );
        assert!(
            embedded.section.source.contains("shared"),
            "embedded Section1.m should contain at least one shared member for {}",
            embedded.name
        );
    }
}

#[test]
fn parse_package_parts_rejects_non_zip() {
    let bogus = b"this is not a zip file";
    let err = parse_package_parts(bogus).expect_err("non-zip bytes should fail");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_config_package_xml_errors() {
    let bytes = build_zip(vec![(
        "Formulas/Section1.m",
        MIN_SECTION.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Config/Package.xml should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_section1_errors() {
    let bytes = build_zip(vec![(
        "Config/Package.xml",
        MIN_PACKAGE_XML.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Formulas/Section1.m should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_package_xml_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", vec![0xFF, 0xFF, 0xFF]),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
    ]);
    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Package.xml should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_section1_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", vec![0xFF, 0xFF]),
    ]);

    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Section1.m should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn embedded_content_invalid_zip_is_skipped() {
    let bytes =
        build_minimal_package_parts_with(vec![("Content/bogus.package", b"not a zip".to_vec())]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_missing_section1_is_skipped() {
    let nested = build_zip(vec![("Config/Formulas.xml", b"<Formulas/>".to_vec())]);
    let bytes = build_minimal_package_parts_with(vec![("Content/no_section1.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_invalid_utf8_is_skipped() {
    let nested = build_zip(vec![("Formulas/Section1.m", vec![0xFF, 0xFF])]);
    let bytes = build_minimal_package_parts_with(vec![("Content/bad_utf8.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_partial_failure_retains_valid_entries() {
    let good_nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![
        ("Content/good.package", good_nested),
        ("Content/bad.package", b"not a zip".to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    let embedded = &parts.embedded_contents[0];
    assert_eq!(embedded.name, "Content/good.package");
    assert!(embedded.section.source.contains("section Section1;"));
    assert!(embedded.section.source.contains("shared"));
}

#[test]
fn leading_slash_paths_are_accepted() {
    let embedded =
        build_embedded_section_zip("section Section1;\nshared Bar = 2;".as_bytes().to_vec());
    let bytes = build_zip(vec![
        (
            "/Config/Package.xml",
            br#"<Package from="leading"/>"#.to_vec(),
        ),
        ("/Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("/Content/abcd.package", embedded),
        (
            "Config/Package.xml",
            br#"<Package from="canonical"/>"#.to_vec(),
        ),
    ]);

    let parts = parse_package_parts(&bytes).expect("leading slash entries should parse");
    assert!(
        parts.package_xml.raw_xml.contains(r#"from="leading""#),
        "first encountered Package.xml should win"
    );
    assert!(parts.main_section.source.contains("shared Foo = 1;"));
    assert_eq!(parts.embedded_contents.len(), 1);
    assert!(
        parts.embedded_contents[0]
            .section
            .source
            .contains("shared Bar = 2;")
    );
}

#[test]
fn embedded_content_name_is_canonicalized() {
    let nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![("/Content/efgh.package", nested)]);

    let parts =
        parse_package_parts(&bytes).expect("embedded content with leading slash should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    assert_eq!(parts.embedded_contents[0].name, "Content/efgh.package");
}

#[test]
fn empty_content_directory_is_ignored() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/", Vec::new()),
    ]);

    let parts = parse_package_parts(&bytes).expect("package with empty Content/ directory parses");
    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(!parts.main_section.source.is_empty());
    assert!(
        parts.embedded_contents.is_empty(),
        "bare Content/ directory should not produce embedded contents"
    );
}

#[test]
fn parse_package_parts_never_panics_on_random_bytes() {
    for seed in 0u64..64 {
        let len = (seed as usize * 13 % 256) + (seed as usize % 7);
        let bytes = random_bytes(seed, len);
        let _ = parse_package_parts(&bytes);
    }
}

#[test]
fn package_parts_section1_with_bom_parses_via_parse_section_members() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", BOM_SECTION.as_bytes().to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("BOM-prefixed Section1.m should parse");
    assert!(
        !parts.main_section.source.starts_with('\u{FEFF}'),
        "PackageParts should strip a single leading BOM from Section1.m"
    );
    let members = parse_section_members(&parts.main_section.source)
        .expect("parse_section_members should accept BOM-prefixed Section1");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
}

#[test]
fn embedded_content_section1_with_bom_parses_via_parse_section_members() {
    let embedded = build_embedded_section_zip(BOM_SECTION.as_bytes().to_vec());
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/bom_embedded.package", embedded),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(
        !parts.embedded_contents.is_empty(),
        "embedded package should be detected"
    );

    let embedded = parts
        .embedded_contents
        .iter()
        .find(|entry| entry.name == "Content/bom_embedded.package")
        .expect("expected embedded package to round-trip name");

    assert!(
        !embedded.section.source.starts_with('\u{FEFF}'),
        "embedded Section1.m should strip leading BOM"
    );

    let members = parse_section_members(&embedded.section.source)
        .expect("parse_section_members should accept embedded BOM Section1");
    assert!(
        !members.is_empty(),
        "embedded Section1.m should contain members"
    );
    assert!(
        members.iter().any(|member| {
            member.section_name == "Section1"
                && member.member_name == "Foo"
                && member.expression_m == "1"
        }),
        "embedded Section1.m should parse shared Foo = 1"
    );
}

fn build_minimal_package_parts_with(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut all_entries = Vec::with_capacity(entries.len() + 2);
    all_entries.push(("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()));
    all_entries.push(("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()));
    all_entries.extend(entries);
    build_zip(all_entries)
}

fn build_embedded_section_zip(section_bytes: Vec<u8>) -> Vec<u8> {
    build_zip(vec![("Formulas/Section1.m", section_bytes)])
}

fn build_zip(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, bytes) in entries {
        if name.ends_with('/') {
            writer
                .add_directory(name, options)
                .expect("start zip directory");
        } else {
            writer.start_file(name, options).expect("start zip entry");
            writer.write_all(&bytes).expect("write zip entry");
        }
    }

    writer.finish().expect("finish zip").into_inner()
}

fn random_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(len);
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    for _ in 0..len {
        state = state
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        bytes.push((state >> 32) as u8);
    }
    bytes
}
```

---

### File: `core\tests\m4_permissions_metadata_tests.rs`

```rust
use excel_diff::{
    DataMashupError, Permissions, RawDataMashup, build_data_mashup, datamashup::parse_metadata,
    open_data_mashup, parse_package_parts, parse_section_members,
};

mod common;
use common::fixture_path;

fn load_datamashup(path: &str) -> excel_diff::DataMashup {
    let raw = open_data_mashup(fixture_path(path))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

#[test]
fn permissions_parsed_flags_default_vs_firewall_off() {
    let defaults = load_datamashup("permissions_defaults.xlsx");
    let firewall_off = load_datamashup("permissions_firewall_off.xlsx");

    assert_eq!(defaults.version, 0);
    assert_eq!(firewall_off.version, 0);

    assert!(defaults.permissions.firewall_enabled);
    assert!(!defaults.permissions.can_evaluate_future_packages);
    assert!(!firewall_off.permissions.firewall_enabled);
    assert_eq!(
        defaults.permissions.workbook_group_type,
        firewall_off.permissions.workbook_group_type
    );
}

#[test]
fn permissions_missing_or_malformed_yields_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut missing = base_raw.clone();
    missing.permissions = Vec::new();
    missing.permission_bindings = Vec::new();
    let dm = build_data_mashup(&missing).expect("missing permissions should default");
    assert_eq!(dm.permissions, Permissions::default());

    let mut malformed = base_raw.clone();
    malformed.permissions = b"<not-xml".to_vec();
    let dm = build_data_mashup(&malformed).expect("malformed permissions should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn permissions_invalid_entities_yield_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let invalid_permissions = br#"
        <Permissions>
            <CanEvaluateFuturePackages>&bad;</CanEvaluateFuturePackages>
            <FirewallEnabled>true</FirewallEnabled>
        </Permissions>
    "#;
    let mut raw = base_raw.clone();
    raw.permissions = invalid_permissions.to_vec();

    let dm = build_data_mashup(&raw).expect("invalid permissions entities should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn metadata_empty_bytes_returns_empty_struct() {
    let metadata = parse_metadata(&[]).expect("empty metadata should parse");
    assert!(metadata.formulas.is_empty());
}

#[test]
fn metadata_invalid_header_too_short_errors() {
    let err = parse_metadata(&[0x01]).expect_err("short metadata should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata XML not found"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_length_prefix_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&100u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 10]);

    let err = parse_metadata(&bytes).expect_err("invalid length prefix should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata length prefix invalid"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_utf8_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&[0xFF, 0xFF]);

    let err = parse_metadata(&bytes).expect_err("invalid utf-8 should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata is not valid UTF-8"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_malformed_xml_errors() {
    let xml = b"<LocalPackageMetadataFile><foo";
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    bytes.extend_from_slice(xml);

    let err = parse_metadata(&bytes).expect_err("malformed xml should error");
    match err {
        DataMashupError::XmlError(_) => {}
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_formulas_match_section_members() {
    let raw = open_data_mashup(fixture_path("metadata_simple.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    let package = parse_package_parts(&raw.package_parts).expect("package parts should parse");
    let metadata = parse_metadata(&raw.metadata).expect("metadata should parse");
    let members =
        parse_section_members(&package.main_section.source).expect("section members should parse");

    let section1_formulas: Vec<_> = metadata
        .formulas
        .iter()
        .filter(|m| m.section_name == "Section1" && !m.is_connection_only)
        .collect();

    assert_eq!(section1_formulas.len(), members.len());
    for meta in section1_formulas {
        assert!(!meta.formula_name.is_empty());
    }
}

#[test]
fn metadata_load_destinations_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let load_to_sheet = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToSheet")
        .expect("LoadToSheet metadata missing");
    assert!(load_to_sheet.load_to_sheet);
    assert!(!load_to_sheet.load_to_model);
    assert!(!load_to_sheet.is_connection_only);

    let load_to_model = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToModel")
        .expect("LoadToModel metadata missing");
    assert!(!load_to_model.load_to_sheet);
    assert!(load_to_model.load_to_model);
    assert!(!load_to_model.is_connection_only);
}

#[test]
fn metadata_groups_basic_hierarchy() {
    let dm = load_datamashup("metadata_query_groups.xlsx");
    let grouped = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/GroupedFoo")
        .expect("GroupedFoo metadata missing");
    assert_eq!(grouped.group_path.as_deref(), Some("Inputs/DimTables"));

    let root = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/RootQuery")
        .expect("RootQuery metadata missing");
    assert!(root.group_path.is_none());
}

#[test]
fn metadata_hidden_queries_connection_only() {
    let dm = load_datamashup("metadata_hidden_queries.xlsx");
    let has_connection_only = dm
        .metadata
        .formulas
        .iter()
        .any(|m| !m.load_to_sheet && !m.load_to_model && m.is_connection_only);
    assert!(has_connection_only);
}

#[test]
fn metadata_itempath_decodes_percent_encoded_utf8() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%C3%A9</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar\u{00e9}");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar\u{00e9}");
    assert!(item.load_to_sheet);
    assert!(!item.is_connection_only);
}

#[test]
fn metadata_itempath_decodes_space_and_slash() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%2FInner</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar/Inner");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar/Inner");
}

#[test]
fn permission_bindings_present_flag() {
    let dm = load_datamashup("permissions_defaults.xlsx");
    assert!(!dm.permission_bindings_raw.is_empty());
}

#[test]
fn permission_bindings_missing_ok() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut synthetic = RawDataMashup {
        permission_bindings: Vec::new(),
        ..base_raw.clone()
    };
    synthetic.permissions = Vec::new();
    synthetic.metadata = Vec::new();

    let dm = build_data_mashup(&synthetic).expect("empty bindings should build");
    assert!(dm.permission_bindings_raw.is_empty());
    assert_eq!(dm.permissions, Permissions::default());
}
```

---

### File: `core\tests\m_section_splitting_tests.rs`

```rust
use excel_diff::{SectionParseError, parse_section_members};

const SECTION_SINGLE: &str = r#"
    section Section1;

    shared Foo = 1;
"#;

const SECTION_MULTI: &str = r#"
    section Section1;

    shared Foo = 1;
    shared Bar = 2;
    Baz = 3;
"#;

const SECTION_NOISY: &str = r#"

// Leading comment

section Section1;

// Comment before Foo
shared Foo = 1;

// Another comment

    shared   Bar   =    2    ;

"#;

const SECTION_WITH_BOM: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

#[test]
fn parse_single_member_section() {
    let members = parse_section_members(SECTION_SINGLE).expect("single member section parses");
    assert_eq!(members.len(), 1);

    let foo = &members[0];
    assert_eq!(foo.section_name, "Section1");
    assert_eq!(foo.member_name, "Foo");
    assert_eq!(foo.expression_m, "1");
    assert!(foo.is_shared);
}

#[test]
fn parse_multiple_members() {
    let members = parse_section_members(SECTION_MULTI).expect("multi-member section parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].section_name, "Section1");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
}

#[test]
fn tolerate_whitespace_comments() {
    let members = parse_section_members(SECTION_NOISY).expect("noisy section still parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);
    assert_eq!(members[0].section_name, "Section1");

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
    assert_eq!(members[1].section_name, "Section1");
}

#[test]
fn error_on_missing_section_header() {
    const NO_SECTION: &str = r#"
        shared Foo = 1;
    "#;

    let result = parse_section_members(NO_SECTION);
    assert_eq!(result, Err(SectionParseError::MissingSectionHeader));
}

#[test]
fn section_parsing_tolerates_utf8_bom() {
    let members =
        parse_section_members(SECTION_WITH_BOM).expect("BOM-prefixed section should parse");
    assert_eq!(members.len(), 1);

    let member = &members[0];
    assert_eq!(member.member_name, "Foo");
    assert_eq!(member.section_name, "Section1");
    assert_eq!(member.expression_m, "1");
    assert!(member.is_shared);
}
```

---

### File: `core\tests\output_tests.rs`

```rust
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ContainerError, DiffOp, DiffReport, ExcelOpenError,
    diff_workbooks, open_workbook,
    output::json::{
        CellDiff, diff_report_to_cell_diffs, diff_workbooks_to_json, serialize_cell_diffs,
        serialize_diff_report,
    },
};
use serde_json::Value;

mod common;
use common::fixture_path;

fn render_value(value: &Option<excel_diff::CellValue>) -> Option<String> {
    match value {
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(s)) => Some(s.clone()),
        Some(excel_diff::CellValue::Bool(b)) => Some(b.to_string()),
        None => None,
    }
}

fn make_cell_snapshot(addr: CellAddress, value: Option<CellValue>) -> CellSnapshot {
    CellSnapshot {
        addr,
        value,
        formula: None,
    }
}

#[test]
fn diff_report_to_cell_diffs_filters_non_cell_ops() {
    let addr1 = CellAddress::from_indices(0, 0);
    let addr2 = CellAddress::from_indices(1, 1);

    let report = DiffReport::new(vec![
        DiffOp::SheetAdded {
            sheet: "SheetAdded".into(),
        },
        DiffOp::cell_edited(
            "Sheet1".into(),
            addr1,
            make_cell_snapshot(addr1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr1, Some(CellValue::Number(2.0))),
        ),
        DiffOp::RowAdded {
            sheet: "Sheet1".into(),
            row_idx: 5,
            row_signature: None,
        },
        DiffOp::cell_edited(
            "Sheet2".into(),
            addr2,
            make_cell_snapshot(addr2, Some(CellValue::Text("old".into()))),
            make_cell_snapshot(addr2, Some(CellValue::Text("new".into()))),
        ),
        DiffOp::SheetRemoved {
            sheet: "OldSheet".into(),
        },
    ]);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(
        cell_diffs.len(),
        2,
        "only CellEdited ops should be projected"
    );

    assert_eq!(cell_diffs[0].coords, addr1.to_a1());
    assert_eq!(cell_diffs[0].value_file1, Some("1".into()));
    assert_eq!(cell_diffs[0].value_file2, Some("2".into()));

    assert_eq!(cell_diffs[1].coords, addr2.to_a1());
    assert_eq!(cell_diffs[1].value_file1, Some("old".into()));
    assert_eq!(cell_diffs[1].value_file2, Some("new".into()));
}

#[test]
fn diff_report_to_cell_diffs_maps_values_correctly() {
    let addr_num = CellAddress::from_indices(2, 2); // C3
    let addr_bool = CellAddress::from_indices(3, 3); // D4

    let report = DiffReport::new(vec![
        DiffOp::cell_edited(
            "SheetX".into(),
            addr_num,
            make_cell_snapshot(addr_num, Some(CellValue::Number(42.5))),
            make_cell_snapshot(addr_num, Some(CellValue::Number(43.5))),
        ),
        DiffOp::cell_edited(
            "SheetX".into(),
            addr_bool,
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(true))),
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(false))),
        ),
    ]);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(cell_diffs.len(), 2);

    let number_diff = &cell_diffs[0];
    assert_eq!(number_diff.coords, addr_num.to_a1());
    assert_eq!(number_diff.value_file1, Some("42.5".into()));
    assert_eq!(number_diff.value_file2, Some("43.5".into()));

    let bool_diff = &cell_diffs[1];
    assert_eq!(bool_diff.coords, addr_bool.to_a1());
    assert_eq!(bool_diff.value_file1, Some("true".into()));
    assert_eq!(bool_diff.value_file2, Some("false".into()));
}

#[test]
fn test_json_format() {
    let diffs = vec![
        CellDiff {
            coords: "A1".into(),
            value_file1: Some("100".into()),
            value_file2: Some("200".into()),
        },
        CellDiff {
            coords: "B2".into(),
            value_file1: Some("true".into()),
            value_file2: Some("false".into()),
        },
        CellDiff {
            coords: "C3".into(),
            value_file1: Some("#DIV/0!".into()),
            value_file2: None,
        },
    ];

    let json = serialize_cell_diffs(&diffs).expect("serialization should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    assert!(value.is_array(), "expected an array of cell diffs");
    let arr = value
        .as_array()
        .expect("top-level json should be an array of cell diffs");
    assert_eq!(arr.len(), 3);

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("A1".into()));
    assert_eq!(first["value_file1"], Value::String("100".into()));
    assert_eq!(first["value_file2"], Value::String("200".into()));

    let second = &arr[1];
    assert_eq!(second["coords"], Value::String("B2".into()));
    assert_eq!(second["value_file1"], Value::String("true".into()));
    assert_eq!(second["value_file2"], Value::String("false".into()));

    let third = &arr[2];
    assert_eq!(third["coords"], Value::String("C3".into()));
    assert_eq!(third["value_file1"], Value::String("#DIV/0!".into()));
    assert_eq!(third["value_file2"], Value::Null);
}

#[test]
fn test_json_empty_diff() {
    let fixture = fixture_path("pg1_basic_two_sheets.xlsx");
    let json =
        diff_workbooks_to_json(&fixture, &fixture).expect("diffing identical files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "identical files should produce no diff ops"
    );
}

#[test]
fn test_json_non_empty_diff() {
    let a = fixture_path("json_diff_single_cell_a.xlsx");
    let b = fixture_path("json_diff_single_cell_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_non_empty_diff_bool() {
    let a = fixture_path("json_diff_bool_a.xlsx");
    let b = fixture_path("json_diff_bool_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("true".into()));
            assert_eq!(render_value(&to.value), Some("false".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_diff_value_to_empty() {
    let a = fixture_path("json_diff_value_to_empty_a.xlsx");
    let b = fixture_path("json_diff_value_to_empty_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), None);
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn json_diff_case_only_sheet_name_no_changes() {
    let a = fixture_path("sheet_case_only_rename_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_b.xlsx");

    let old = open_workbook(&a).expect("fixture A should open");
    let new = open_workbook(&b).expect("fixture B should open");

    let report = diff_workbooks(&old, &new);
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should produce no diff ops"
    );
}

#[test]
fn json_diff_case_only_sheet_name_cell_edit() {
    let a = fixture_path("sheet_case_only_rename_edit_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_edit_b.xlsx");

    let old = open_workbook(&a).expect("fixture A should open");
    let new = open_workbook(&b).expect("fixture B should open");

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_case_only_sheet_name_no_changes() {
    let a = fixture_path("sheet_case_only_rename_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_b.xlsx");

    let json =
        diff_workbooks_to_json(&a, &b).expect("diffing case-only sheet rename should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should serialize to no ops"
    );
}

#[test]
fn test_json_case_only_sheet_name_cell_edit_via_helper() {
    let a = fixture_path("sheet_case_only_rename_edit_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_edit_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b)
        .expect("diffing case-only sheet rename with cell edit should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_diff_workbooks_to_json_reports_invalid_zip() {
    let path = fixture_path("not_a_zip.txt");
    let err = diff_workbooks_to_json(&path, &path)
        .expect_err("diffing invalid containers should return an error");

    assert!(
        matches!(
            err,
            ExcelOpenError::Container(ContainerError::NotZipContainer)
        ),
        "expected container error, got {err}"
    );
}

#[test]
fn serialize_diff_report_nan_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::NAN))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("NaN should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());

    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("nan"),
                "error message should mention NaN for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::INFINITY))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("Infinity should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());
    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_neg_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::NEG_INFINITY))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("NEG_INFINITY should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());
    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_with_finite_numbers_succeeds() {
    let addr = CellAddress::from_indices(1, 1);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(2.5))),
        make_cell_snapshot(addr, Some(CellValue::Number(3.5))),
    )]);

    let json = serialize_diff_report(&report).expect("finite values should serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(parsed.ops.len(), 1);
}
```

---

### File: `core\tests\pg1_ir_tests.rs`

```rust
use excel_diff::{CellValue, Sheet, SheetKind, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg1_basic_two_sheets_structure() {
    let workbook = open_workbook(fixture_path("pg1_basic_two_sheets.xlsx"))
        .expect("pg1 basic fixture should open");
    assert_eq!(workbook.sheets.len(), 2);
    assert_eq!(workbook.sheets[0].name, "Sheet1");
    assert_eq!(workbook.sheets[1].name, "Sheet2");
    assert!(matches!(workbook.sheets[0].kind, SheetKind::Worksheet));
    assert!(matches!(workbook.sheets[1].kind, SheetKind::Worksheet));

    let sheet1 = &workbook.sheets[0];
    assert_eq!(sheet1.grid.nrows, 3);
    assert_eq!(sheet1.grid.ncols, 3);
    assert_eq!(
        sheet1
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_text)),
        Some("R1C1")
    );

    let sheet2 = &workbook.sheets[1];
    assert_eq!(sheet2.grid.nrows, 5);
    assert_eq!(sheet2.grid.ncols, 2);
    assert_eq!(
        sheet2
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_text)),
        Some("S2_R1C1")
    );
}

#[test]
fn pg1_sparse_used_range_extents() {
    let workbook =
        open_workbook(fixture_path("pg1_sparse_used_range.xlsx")).expect("sparse fixture opens");
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == "Sparse")
        .expect("Sparse sheet present");

    assert_eq!(sheet.grid.nrows, 10);
    assert_eq!(sheet.grid.ncols, 7);

    assert_cell_text(sheet, 0, 0, "A1");
    assert_cell_text(sheet, 1, 1, "B2");
    assert_cell_text(sheet, 9, 6, "G10");
    assert_eq!(sheet.grid.cell_count(), 3);
}

#[test]
fn pg1_empty_and_mixed_sheets() {
    let workbook = open_workbook(fixture_path("pg1_empty_and_mixed_sheets.xlsx"))
        .expect("mixed sheets fixture opens");

    let empty = sheet_by_name(&workbook, "Empty");
    assert_eq!(empty.grid.nrows, 0);
    assert_eq!(empty.grid.ncols, 0);
    assert_eq!(empty.grid.cell_count(), 0);

    let values_only = sheet_by_name(&workbook, "ValuesOnly");
    assert_eq!(values_only.grid.nrows, 10);
    assert_eq!(values_only.grid.ncols, 10);
    let values: Vec<_> = values_only.grid.iter_cells().collect();
    assert!(
        values
            .iter()
            .all(|c| c.value.is_some() && c.formula.is_none()),
        "ValuesOnly cells should have values and no formulas"
    );
    assert_eq!(
        values_only
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_number)),
        Some(1.0)
    );

    let formulas = sheet_by_name(&workbook, "FormulasOnly");
    assert_eq!(formulas.grid.nrows, 10);
    assert_eq!(formulas.grid.ncols, 10);
    let first = formulas.grid.get(0, 0).expect("A1 should exist");
    assert_eq!(first.formula.as_deref(), Some("ValuesOnly!A1"));
    assert!(
        first.value.is_some(),
        "Formulas should surface cached values when present"
    );
    assert!(
        formulas.grid.iter_cells().all(|c| c.formula.is_some()),
        "All cells should carry formulas in FormulasOnly"
    );
}

fn sheet_by_name<'a>(workbook: &'a excel_diff::Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("sheet {name} not found"))
}

fn assert_cell_text(sheet: &Sheet, row: u32, col: u32, expected: &str) {
    let cell = sheet
        .grid
        .get(row, col)
        .unwrap_or_else(|| panic!("cell {expected} should exist"));
    assert_eq!(cell.address.to_a1(), expected);
    assert_eq!(
        cell.value
            .as_ref()
            .and_then(CellValue::as_text)
            .unwrap_or(""),
        expected
    );
}
```

---

### File: `core\tests\pg3_snapshot_tests.rs`

```rust
use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, Sheet, Workbook, address_to_index, open_workbook,
};

mod common;
use common::fixture_path;

fn sheet_by_name<'a>(workbook: &'a Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == name)
        .expect("sheet should exist")
}

fn find_cell<'a>(sheet: &'a Sheet, addr: &str) -> Option<&'a Cell> {
    let (row, col) = address_to_index(addr).expect("address should parse");
    sheet.grid.get(row, col)
}

fn snapshot(sheet: &Sheet, addr: &str) -> CellSnapshot {
    if let Some(cell) = find_cell(sheet, addr) {
        CellSnapshot::from_cell(cell)
    } else {
        let (row, col) = address_to_index(addr).expect("address should parse");
        CellSnapshot {
            addr: CellAddress::from_indices(row, col),
            value: None,
            formula: None,
        }
    }
}

#[test]
fn pg3_value_and_formula_cells_snapshot_from_excel() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let a1 = snapshot(sheet, "A1");
    assert_eq!(a1.addr.to_string(), "A1");
    assert_eq!(a1.value, Some(CellValue::Number(42.0)));
    assert!(a1.formula.is_none());

    let a2 = snapshot(sheet, "A2");
    assert_eq!(a2.value, Some(CellValue::Text("hello".into())));
    assert!(a2.formula.is_none());

    let a3 = snapshot(sheet, "A3");
    assert_eq!(a3.value, Some(CellValue::Bool(true)));
    assert!(a3.formula.is_none());

    let a4 = snapshot(sheet, "A4");
    assert!(a4.value.is_none());
    assert!(a4.formula.is_none());

    let b1 = snapshot(sheet, "B1");
    assert!(matches!(
        b1.value,
        Some(CellValue::Number(n)) if (n - 43.0).abs() < 1e-6
    ));
    assert_eq!(b1.addr.to_string(), "B1");
    let b1_formula = b1.formula.as_deref().expect("B1 should have a formula");
    assert!(b1_formula.contains("A1+1"));

    let b2 = snapshot(sheet, "B2");
    assert_eq!(b2.value, Some(CellValue::Text("hello world".into())));
    assert_eq!(b2.addr.to_string(), "B2");
    let b2_formula = b2.formula.as_deref().expect("B2 should have a formula");
    assert!(b2_formula.contains("hello"));
    assert!(b2_formula.contains("world"));

    let b3 = snapshot(sheet, "B3");
    assert_eq!(b3.value, Some(CellValue::Bool(true)));
    assert_eq!(b3.addr.to_string(), "B3");
    let b3_formula = b3.formula.as_deref().expect("B3 should have a formula");
    assert!(
        b3_formula.contains(">0"),
        "B3 formula should include comparison: {b3_formula:?}"
    );
}

#[test]
fn snapshot_json_roundtrip() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let snapshots = vec![
        snapshot(sheet, "A1"),
        snapshot(sheet, "A2"),
        snapshot(sheet, "B1"),
        snapshot(sheet, "B2"),
        snapshot(sheet, "B3"),
    ];

    for snap in snapshots {
        let addr = snap.addr.to_string();
        let json = serde_json::to_string(&snap).expect("snapshot should serialize");
        let as_value: serde_json::Value =
            serde_json::from_str(&json).expect("snapshot JSON should parse to value");
        assert_eq!(as_value["addr"], serde_json::Value::String(addr));
        let snap_back: CellSnapshot = serde_json::from_str(&json).expect("snapshot should parse");
        assert_eq!(snap.addr, snap_back.addr);
        assert_eq!(snap, snap_back);
    }
}

#[test]
fn snapshot_json_roundtrip_detects_tampered_addr() {
    let snap = CellSnapshot {
        addr: "Z9".parse().expect("address should parse"),
        value: Some(CellValue::Number(1.0)),
        formula: Some("A1+1".into()),
    };

    let mut value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&snap).expect("serialize should work"))
            .expect("serialized JSON should parse");
    value["addr"] = serde_json::Value::String("A1".into());

    let tampered_json = serde_json::to_string(&value).expect("tampered JSON should serialize");
    let tampered: CellSnapshot =
        serde_json::from_str(&tampered_json).expect("tampered JSON should parse");

    assert_ne!(snap.addr, tampered.addr);
    assert_eq!(snap, tampered, "value/formula equality ignores addr");
}

#[test]
fn snapshot_json_rejects_invalid_addr_1a() {
    let json = r#"{"addr":"1A","value":null,"formula":null}"#;
    let result: Result<CellSnapshot, _> = serde_json::from_str(json);
    let err = result
        .expect_err("invalid addr should fail to deserialize")
        .to_string();

    assert!(
        err.contains("invalid cell address"),
        "error should mention invalid cell address: {err}"
    );
    assert!(
        err.contains("1A"),
        "error should include the offending address: {err}"
    );
}

#[test]
fn snapshot_json_rejects_invalid_addr_a0() {
    let json = r#"{"addr":"A0","value":null,"formula":null}"#;
    let result: Result<CellSnapshot, _> = serde_json::from_str(json);
    let err = result
        .expect_err("invalid addr should fail to deserialize")
        .to_string();

    assert!(
        err.contains("invalid cell address"),
        "error should mention invalid cell address: {err}"
    );
    assert!(
        err.contains("A0"),
        "error should include the offending address: {err}"
    );
}
```

---

### File: `core\tests\pg4_diffop_tests.rs`

```rust
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ColSignature, DiffOp, DiffReport, RowSignature,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn addr(a1: &str) -> CellAddress {
    a1.parse().expect("address should parse")
}

fn snapshot(a1: &str, value: Option<CellValue>, formula: Option<&str>) -> CellSnapshot {
    CellSnapshot {
        addr: addr(a1),
        value,
        formula: formula.map(|s| s.to_string()),
    }
}

fn sample_cell_edited() -> DiffOp {
    DiffOp::CellEdited {
        sheet: "Sheet1".to_string(),
        addr: addr("C3"),
        from: snapshot("C3", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    }
}

// Enforces the invariant documented on DiffOp::CellEdited.
fn assert_cell_edited_invariants(op: &DiffOp, expected_sheet: &str, expected_addr: &str) {
    let expected_addr_parsed: CellAddress =
        expected_addr.parse().expect("expected_addr should parse");
    if let DiffOp::CellEdited {
        sheet,
        addr,
        from,
        to,
    } = op
    {
        assert_eq!(sheet, expected_sheet);
        assert_eq!(*addr, expected_addr_parsed);
        assert_eq!(from.addr, expected_addr_parsed);
        assert_eq!(to.addr, expected_addr_parsed);
    } else {
        panic!("expected CellEdited");
    }
}

fn op_kind(op: &DiffOp) -> &'static str {
    match op {
        DiffOp::SheetAdded { .. } => "SheetAdded",
        DiffOp::SheetRemoved { .. } => "SheetRemoved",
        DiffOp::RowAdded { .. } => "RowAdded",
        DiffOp::RowRemoved { .. } => "RowRemoved",
        DiffOp::ColumnAdded { .. } => "ColumnAdded",
        DiffOp::ColumnRemoved { .. } => "ColumnRemoved",
        DiffOp::BlockMovedRows { .. } => "BlockMovedRows",
        DiffOp::BlockMovedColumns { .. } => "BlockMovedColumns",
        DiffOp::CellEdited { .. } => "CellEdited",
        _ => "Unknown",
    }
}

fn json_keys(json: &Value) -> BTreeSet<String> {
    json.as_object()
        .expect("object json")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn pg4_construct_cell_edited_diffop() {
    let op = sample_cell_edited();

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
    if let DiffOp::CellEdited { from, to, .. } = &op {
        assert_ne!(from.value, to.value);
    }
}

#[test]
fn pg4_construct_row_and_column_diffops() {
    let row_added_with_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 8,
        row_signature: None,
    };
    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 0,
        col_signature: None,
    };

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_with_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 10);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0xDEADBEEF);
    } else {
        panic!("expected RowAdded with signature");
    }

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_without_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 11);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowAdded without signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_with_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 9);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0x1234);
    } else {
        panic!("expected RowRemoved with signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_without_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 8);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowRemoved without signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_with_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 2);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0xABCDEF);
    } else {
        panic!("expected ColumnAdded with signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_without_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 3);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnAdded without signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_with_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 1);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0x123456);
    } else {
        panic!("expected ColumnRemoved with signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_without_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 0);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnRemoved without signature");
    }

    assert_ne!(row_added_with_sig, row_added_without_sig);
    assert_ne!(row_removed_with_sig, row_removed_without_sig);
    assert_ne!(col_added_with_sig, col_added_without_sig);
    assert_ne!(col_removed_with_sig, col_removed_without_sig);
}

#[test]
fn pg4_construct_block_move_diffops() {
    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_with_hash
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*src_start_row, 10);
        assert_eq!(*row_count, 3);
        assert_eq!(*dst_start_row, 5);
        assert_eq!(block_hash.unwrap(), 0x12345678);
    } else {
        panic!("expected BlockMovedRows with hash");
    }

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_without_hash
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*src_start_row, 20);
        assert_eq!(*row_count, 2);
        assert_eq!(*dst_start_row, 0);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedRows without hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_with_hash
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*src_start_col, 7);
        assert_eq!(*col_count, 2);
        assert_eq!(*dst_start_col, 3);
        assert_eq!(block_hash.unwrap(), 0xCAFEBABE);
    } else {
        panic!("expected BlockMovedColumns with hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_without_hash
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*src_start_col, 4);
        assert_eq!(*col_count, 1);
        assert_eq!(*dst_start_col, 9);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedColumns without hash");
    }

    assert_ne!(block_rows_with_hash, block_rows_without_hash);
    assert_ne!(block_cols_with_hash, block_cols_without_hash);
}

#[test]
fn pg4_cell_edited_json_shape() {
    let op = sample_cell_edited();
    let json = serde_json::to_value(&op).expect("serialize");
    assert_cell_edited_invariants(&op, "Sheet1", "C3");

    assert_eq!(json["kind"], "CellEdited");
    assert_eq!(json["sheet"], "Sheet1");
    assert_eq!(json["addr"], "C3");
    assert_eq!(json["from"]["addr"], "C3");
    assert_eq!(json["to"]["addr"], "C3");

    let obj = json.as_object().expect("object json");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["addr", "from", "kind", "sheet", "to"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected);
}

#[test]
fn pg4_row_added_json_optional_signature() {
    let op_without_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: None,
    };
    let json_without = serde_json::to_value(&op_without_sig).expect("serialize without sig");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "RowAdded");
    assert_eq!(json_without["sheet"], "Sheet1");
    assert_eq!(json_without["row_idx"], 10);
    assert!(obj_without.get("row_signature").is_none());

    let op_with_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 123 }),
    };
    let json_with = serde_json::to_value(&op_with_sig).expect("serialize with sig");
    assert_eq!(json_with["row_signature"]["hash"], 123);
}

#[test]
fn pg4_column_added_json_optional_signature() {
    let added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet1".to_string(),
        col_idx: 5,
        col_signature: None,
    };
    let json_added_without = serde_json::to_value(&added_without_sig).expect("serialize no sig");
    let obj_added_without = json_added_without.as_object().expect("object json");
    assert_eq!(json_added_without["kind"], "ColumnAdded");
    assert_eq!(json_added_without["sheet"], "Sheet1");
    assert_eq!(json_added_without["col_idx"], 5);
    assert!(obj_added_without.get("col_signature").is_none());

    let added_with_sig = DiffOp::ColumnAdded {
        sheet: "Sheet1".to_string(),
        col_idx: 6,
        col_signature: Some(ColSignature { hash: 321 }),
    };
    let json_added_with = serde_json::to_value(&added_with_sig).expect("serialize with sig");
    assert_eq!(json_added_with["col_signature"]["hash"], 321);

    let removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: None,
    };
    let json_removed_without =
        serde_json::to_value(&removed_without_sig).expect("serialize removed no sig");
    let obj_removed_without = json_removed_without.as_object().expect("object json");
    assert_eq!(json_removed_without["kind"], "ColumnRemoved");
    assert!(obj_removed_without.get("col_signature").is_none());

    let removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 654 }),
    };
    let json_removed_with =
        serde_json::to_value(&removed_with_sig).expect("serialize removed with sig");
    assert_eq!(json_removed_with["col_signature"]["hash"], 654);
}

#[test]
fn pg4_block_moved_rows_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedRows");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: Some(777),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(777));
}

#[test]
fn pg4_block_moved_columns_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedColumns {
        sheet: "SheetX".to_string(),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedColumns");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedColumns {
        sheet: "SheetX".to_string(),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: Some(4242),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(4242));
}

#[test]
fn pg4_sheet_added_and_removed_json_shape() {
    let added = DiffOp::SheetAdded {
        sheet: "Sheet1".to_string(),
    };
    let added_json = serde_json::to_value(&added).expect("serialize sheet added");
    assert_eq!(added_json["kind"], "SheetAdded");
    assert_eq!(added_json["sheet"], "Sheet1");
    let added_keys = json_keys(&added_json);
    let expected_keys: BTreeSet<String> = ["kind", "sheet"].into_iter().map(String::from).collect();
    assert_eq!(added_keys, expected_keys);

    let removed = DiffOp::SheetRemoved {
        sheet: "SheetX".to_string(),
    };
    let removed_json = serde_json::to_value(&removed).expect("serialize sheet removed");
    assert_eq!(removed_json["kind"], "SheetRemoved");
    assert_eq!(removed_json["sheet"], "SheetX");
    let removed_keys = json_keys(&removed_json);
    assert_eq!(removed_keys, expected_keys);
}

#[test]
fn pg4_row_and_column_json_shape_keysets() {
    let expected_row_with_sig: BTreeSet<String> = ["kind", "row_idx", "row_signature", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_row_without_sig: BTreeSet<String> = ["kind", "row_idx", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_with_sig: BTreeSet<String> = ["col_idx", "col_signature", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_without_sig: BTreeSet<String> = ["col_idx", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();

    let row_added_with_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 8,
        row_signature: None,
    };

    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 0,
        col_signature: None,
    };

    let cases = vec![
        (
            row_added_with_sig,
            "RowAdded",
            expected_row_with_sig.clone(),
        ),
        (
            row_added_without_sig,
            "RowAdded",
            expected_row_without_sig.clone(),
        ),
        (
            row_removed_with_sig,
            "RowRemoved",
            expected_row_with_sig.clone(),
        ),
        (
            row_removed_without_sig,
            "RowRemoved",
            expected_row_without_sig.clone(),
        ),
        (
            col_added_with_sig,
            "ColumnAdded",
            expected_col_with_sig.clone(),
        ),
        (
            col_added_without_sig,
            "ColumnAdded",
            expected_col_without_sig.clone(),
        ),
        (
            col_removed_with_sig,
            "ColumnRemoved",
            expected_col_with_sig.clone(),
        ),
        (
            col_removed_without_sig,
            "ColumnRemoved",
            expected_col_without_sig.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_block_move_json_shape_keysets() {
    let expected_rows_with_hash: BTreeSet<String> = [
        "block_hash",
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rows_without_hash: BTreeSet<String> = [
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_with_hash: BTreeSet<String> = [
        "block_hash",
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_without_hash: BTreeSet<String> = [
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };

    let cases = vec![
        (
            block_rows_with_hash,
            "BlockMovedRows",
            expected_rows_with_hash.clone(),
        ),
        (
            block_rows_without_hash,
            "BlockMovedRows",
            expected_rows_without_hash.clone(),
        ),
        (
            block_cols_with_hash,
            "BlockMovedColumns",
            expected_cols_with_hash.clone(),
        ),
        (
            block_cols_without_hash,
            "BlockMovedColumns",
            expected_cols_without_hash.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_diffop_roundtrip_each_variant() {
    let ops = vec![
        DiffOp::SheetAdded {
            sheet: "SheetA".to_string(),
        },
        DiffOp::SheetRemoved {
            sheet: "SheetB".to_string(),
        },
        DiffOp::RowAdded {
            sheet: "Sheet1".to_string(),
            row_idx: 1,
            row_signature: Some(RowSignature { hash: 42 }),
        },
        DiffOp::RowRemoved {
            sheet: "Sheet1".to_string(),
            row_idx: 0,
            row_signature: None,
        },
        DiffOp::ColumnAdded {
            sheet: "Sheet1".to_string(),
            col_idx: 2,
            col_signature: None,
        },
        DiffOp::ColumnRemoved {
            sheet: "Sheet1".to_string(),
            col_idx: 3,
            col_signature: Some(ColSignature { hash: 99 }),
        },
        DiffOp::BlockMovedRows {
            sheet: "Sheet1".to_string(),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: Some(1234),
        },
        DiffOp::BlockMovedRows {
            sheet: "Sheet1".to_string(),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: None,
        },
        DiffOp::BlockMovedColumns {
            sheet: "Sheet2".to_string(),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: Some(888),
        },
        DiffOp::BlockMovedColumns {
            sheet: "Sheet2".to_string(),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: None,
        },
        sample_cell_edited(),
    ];

    for original in ops {
        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: DiffOp = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, original);

        if let DiffOp::CellEdited { .. } = &deserialized {
            assert_cell_edited_invariants(&deserialized, "Sheet1", "C3");
        }
    }
}

#[test]
fn pg4_cell_edited_roundtrip_preserves_snapshot_addrs() {
    let op = sample_cell_edited();
    let json = serde_json::to_string(&op).expect("serialize");
    let round_tripped: DiffOp = serde_json::from_str(&json).expect("deserialize");

    assert_cell_edited_invariants(&round_tripped, "Sheet1", "C3");
}

#[test]
fn pg4_diff_report_roundtrip_preserves_order() {
    let op1 = DiffOp::SheetAdded {
        sheet: "Sheet1".to_string(),
    };
    let op2 = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: None,
    };
    let op3 = sample_cell_edited();

    let ops = vec![op1, op2, op3];
    let report = DiffReport::new(ops.clone());
    assert_eq!(report.version, DiffReport::SCHEMA_VERSION);

    let serialized = serde_json::to_string(&report).expect("serialize report");
    let deserialized: DiffReport = serde_json::from_str(&serialized).expect("deserialize report");
    assert_eq!(deserialized.version, "1");
    assert_eq!(deserialized.ops, ops);

    let kinds: Vec<&str> = deserialized.ops.iter().map(op_kind).collect();
    assert_eq!(kinds, vec!["SheetAdded", "RowAdded", "CellEdited"]);
}

#[test]
fn pg4_diff_report_json_shape() {
    let ops = vec![
        DiffOp::SheetRemoved {
            sheet: "SheetX".to_string(),
        },
        DiffOp::RowRemoved {
            sheet: "SheetX".to_string(),
            row_idx: 3,
            row_signature: Some(RowSignature { hash: 7 }),
        },
    ];
    let report = DiffReport::new(ops);
    let json = serde_json::to_value(&report).expect("serialize to value");

    let obj = json.as_object().expect("report json object");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["ops", "version"].into_iter().map(String::from).collect();
    assert_eq!(keys, expected);
    assert_eq!(obj.get("version").and_then(Value::as_str), Some("1"));

    let ops_json = obj
        .get("ops")
        .and_then(Value::as_array)
        .expect("ops should be array");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "SheetRemoved");
    assert_eq!(ops_json[1]["kind"], "RowRemoved");
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_top_level_addr() {
    let json = r#"{
        "kind": "CellEdited",
        "sheet": "Sheet1",
        "addr": "1A",
        "from": { "addr": "C3", "value": null, "formula": null },
        "to":   { "addr": "C3", "value": null, "formula": null }
    }"#;

    let err = serde_json::from_str::<DiffOp>(json)
        .expect_err("invalid top-level addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_snapshot_addrs() {
    let json = r#"{
        "kind": "CellEdited",
        "sheet": "Sheet1",
        "addr": "C3",
        "from": { "addr": "A0", "value": null, "formula": null },
        "to":   { "addr": "C3", "value": null, "formula": null }
    }"#;

    let err = serde_json::from_str::<DiffOp>(json)
        .expect_err("invalid snapshot addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("A0"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diff_report_rejects_invalid_nested_addr() {
    let json = r#"{
        "version": "1",
        "ops": [{
            "kind": "CellEdited",
            "sheet": "Sheet1",
            "addr": "1A",
            "from": { "addr": "C3", "value": null, "formula": null },
            "to":   { "addr": "C3", "value": null, "formula": null }
        }]
    }"#;

    let err = serde_json::from_str::<DiffReport>(json)
        .expect_err("invalid nested addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should surface nested invalid address: {msg}",
    );
}

#[test]
#[should_panic]
fn pg4_cell_edited_invariant_helper_rejects_mismatched_snapshot_addr() {
    let op = DiffOp::CellEdited {
        sheet: "Sheet1".to_string(),
        addr: addr("C3"),
        from: snapshot("D4", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    };

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
}
```

---

### File: `core\tests\pg5_grid_diff_tests.rs`

```rust
use excel_diff::{
    Cell, CellAddress, CellValue, DiffOp, Grid, Sheet, SheetKind, Workbook, diff_workbooks,
};
use std::collections::BTreeSet;

fn grid_from_numbers(values: &[&[i32]]) -> Grid {
    let nrows = values.len() as u32;
    let ncols = if nrows == 0 {
        0
    } else {
        values[0].len() as u32
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, row_vals) in values.iter().enumerate() {
        for (c, v) in row_vals.iter().enumerate() {
            grid.insert(Cell {
                row: r as u32,
                col: c as u32,
                address: CellAddress::from_indices(r as u32, c as u32),
                value: Some(CellValue::Number(*v as f64)),
                formula: None,
            });
        }
    }

    grid
}

fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook {
    Workbook {
        sheets: vec![Sheet {
            name: name.to_string(),
            kind: SheetKind::Worksheet,
            grid,
        }],
    }
}

#[test]
fn pg5_1_grid_diff_1x1_identical_empty_diff() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert!(report.ops.is_empty());
}

#[test]
fn pg5_2_grid_diff_1x1_value_change_single_cell_edited() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[2]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn pg5_3_grid_diff_row_appended_row_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowAdded, got {other:?}"),
    }
}

#[test]
fn pg5_4_grid_diff_column_appended_column_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnAdded, got {other:?}"),
    }
}

#[test]
fn pg5_5_grid_diff_same_shape_scattered_cell_edits() {
    let old = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]),
    );
    let new = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[10, 2, 3], &[4, 50, 6], &[7, 8, 90]]),
    );

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 3);
    assert!(
        report
            .ops
            .iter()
            .all(|op| matches!(op, DiffOp::CellEdited { .. }))
    );

    let edited_addrs: BTreeSet<String> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.to_a1()),
            _ => None,
        })
        .collect();
    let expected: BTreeSet<String> = ["A1", "B2", "C3"].into_iter().map(String::from).collect();
    assert_eq!(edited_addrs, expected);
}

#[test]
fn pg5_6_grid_diff_degenerate_grids() {
    let empty_old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let empty_new = single_sheet_workbook("Sheet1", Grid::new(0, 0));

    let empty_report = diff_workbooks(&empty_old, &empty_new);
    assert!(empty_report.ops.is_empty());

    let old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 2);

    let mut row_added = 0;
    let mut col_added = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*row_idx, 0);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*col_idx, 0);
                assert!(col_signature.is_none());
                col_added += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    assert_eq!(col_added, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_7_grid_diff_row_truncated_row_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_8_grid_diff_column_truncated_column_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_9_grid_diff_row_and_column_truncated_structure_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 2);

    let mut rows_removed = 0;
    let mut cols_removed = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*row_idx, 1);
                assert!(row_signature.is_none());
                rows_removed += 1;
            }
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*col_idx, 1);
                assert!(col_signature.is_none());
                cols_removed += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(rows_removed, 1);
    assert_eq!(cols_removed, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_10_grid_diff_row_appended_with_overlap_cell_edits() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 20], &[30, 4], &[5, 6]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 3);

    let mut row_added = 0;
    let mut cell_edits = BTreeSet::new();

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*row_idx, 2);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::CellEdited { addr, .. } => {
                cell_edits.insert(addr.to_a1());
            }
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    let expected: BTreeSet<String> = ["B1", "A2"].into_iter().map(String::from).collect();
    assert_eq!(cell_edits, expected);
}
```

---

### File: `core\tests\pg6_object_vs_grid_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg6_1_sheet_added_no_grid_ops_on_main() {
    let old = open_workbook(fixture_path("pg6_sheet_added_a.xlsx")).expect("open pg6 added A");
    let new = open_workbook(fixture_path("pg6_sheet_added_b.xlsx")).expect("open pg6 added B");

    let report = diff_workbooks(&old, &new);

    let mut sheet_added = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "NewSheet" => sheet_added += 1,
            DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::CellEdited { sheet, .. }
                if sheet == "Main" =>
            {
                panic!("unexpected grid op on Main: {op:?}");
            }
            DiffOp::SheetAdded { sheet } => {
                panic!("unexpected sheet added: {sheet}");
            }
            DiffOp::SheetRemoved { sheet } => {
                panic!("unexpected sheet removed: {sheet}");
            }
            DiffOp::BlockMovedRows { .. } | DiffOp::BlockMovedColumns { .. } => {
                panic!("block move ops are not expected in PG6.1: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(sheet_added, 1, "exactly one NewSheet addition expected");
    assert_eq!(report.ops.len(), 1, "no other operations expected");
}

#[test]
fn pg6_2_sheet_removed_no_grid_ops_on_main() {
    let old = open_workbook(fixture_path("pg6_sheet_removed_a.xlsx")).expect("open pg6 removed A");
    let new = open_workbook(fixture_path("pg6_sheet_removed_b.xlsx")).expect("open pg6 removed B");

    let report = diff_workbooks(&old, &new);

    let mut sheet_removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetRemoved { sheet } if sheet == "OldSheet" => sheet_removed += 1,
            DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::CellEdited { sheet, .. }
                if sheet == "Main" =>
            {
                panic!("unexpected grid op on Main: {op:?}");
            }
            DiffOp::SheetAdded { sheet } => {
                panic!("unexpected sheet added: {sheet}");
            }
            DiffOp::SheetRemoved { sheet } => {
                panic!("unexpected sheet removed: {sheet}");
            }
            DiffOp::BlockMovedRows { .. } | DiffOp::BlockMovedColumns { .. } => {
                panic!("block move ops are not expected in PG6.2: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(sheet_removed, 1, "exactly one OldSheet removal expected");
    assert_eq!(report.ops.len(), 1, "no other operations expected");
}

#[test]
fn pg6_3_rename_as_remove_plus_add_no_grid_ops() {
    let old = open_workbook(fixture_path("pg6_sheet_renamed_a.xlsx")).expect("open pg6 rename A");
    let new = open_workbook(fixture_path("pg6_sheet_renamed_b.xlsx")).expect("open pg6 rename B");

    let report = diff_workbooks(&old, &new);

    let mut added = 0;
    let mut removed = 0;

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "NewName" => added += 1,
            DiffOp::SheetRemoved { sheet } if sheet == "OldName" => removed += 1,
            DiffOp::SheetAdded { sheet } => panic!("unexpected sheet added: {sheet}"),
            DiffOp::SheetRemoved { sheet } => panic!("unexpected sheet removed: {sheet}"),
            DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. }
            | DiffOp::CellEdited { .. }
            | DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. } => {
                panic!("no grid-level ops expected for rename scenario: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(
        report.ops.len(),
        2,
        "rename should produce one add and one remove"
    );
    assert_eq!(added, 1, "expected one NewName addition");
    assert_eq!(removed, 1, "expected one OldName removal");
}

#[test]
fn pg6_4_sheet_and_grid_change_composed_cleanly() {
    let old =
        open_workbook(fixture_path("pg6_sheet_and_grid_change_a.xlsx")).expect("open pg6 4 A");
    let new =
        open_workbook(fixture_path("pg6_sheet_and_grid_change_b.xlsx")).expect("open pg6 4 B");

    let report = diff_workbooks(&old, &new);

    let mut scratch_added = 0;
    let mut main_cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Scratch" => scratch_added += 1,
            DiffOp::CellEdited { sheet, .. } => {
                assert_eq!(sheet, "Main", "only Main should have cell edits");
                main_cell_edits += 1;
            }
            DiffOp::SheetRemoved { .. } => {
                panic!("no sheets should be removed in PG6.4: {op:?}");
            }
            DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. }
            | DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. } => {
                panic!("no structural row/column ops expected in PG6.4: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(scratch_added, 1, "exactly one Scratch addition expected");
    assert!(
        main_cell_edits > 0,
        "Main should report at least one cell edit"
    );
}
```

---

### File: `core\tests\signature_tests.rs`

```rust
use excel_diff::{Cell, CellAddress, CellValue, Grid};

fn make_cell(row: u32, col: u32, value: Option<CellValue>, formula: Option<&str>) -> Cell {
    Cell {
        row,
        col,
        address: CellAddress::from_indices(row, col),
        value,
        formula: formula.map(|s| s.to_string()),
    }
}

#[test]
fn identical_rows_have_same_signature() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        let cell = make_cell(0, c, Some(CellValue::Number(c as f64)), None);
        grid1.insert(cell.clone());
        grid2.insert(cell);
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_eq!(sig1, sig2);
}

#[test]
fn different_rows_have_different_signatures() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        grid1.insert(make_cell(0, c, Some(CellValue::Number(c as f64)), None));
        grid2.insert(make_cell(
            0,
            c,
            Some(CellValue::Number((c + 1) as f64)),
            None,
        ));
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1, sig2);
}

#[test]
fn compute_all_signatures_populates_fields() {
    let mut grid = Grid::new(5, 5);
    grid.insert(make_cell(
        2,
        2,
        Some(CellValue::Text("center".into())),
        None,
    ));
    assert!(grid.row_signatures.is_none());
    assert!(grid.col_signatures.is_none());
    grid.compute_all_signatures();
    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert_eq!(grid.row_signatures.as_ref().unwrap().len(), 5);
    assert_eq!(grid.col_signatures.as_ref().unwrap().len(), 5);
    assert_ne!(grid.row_signatures.as_ref().unwrap()[2].hash, 0);
    assert_ne!(grid.col_signatures.as_ref().unwrap()[2].hash, 0);
}

#[test]
fn compute_all_signatures_on_empty_grid_produces_empty_vectors() {
    let mut grid = Grid::new(0, 0);

    grid.compute_all_signatures();

    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert!(grid.row_signatures.as_ref().unwrap().is_empty());
    assert!(grid.col_signatures.as_ref().unwrap().is_empty());
}

#[test]
fn compute_all_signatures_with_all_empty_rows_and_cols_is_stable() {
    let mut grid = Grid::new(3, 4);

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    assert_eq!(first_rows.len(), 3);
    assert_eq!(first_cols.len(), 4);
    assert!(first_rows.iter().all(|sig| sig.hash == 0));
    assert!(first_cols.iter().all(|sig| sig.hash == 0));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_eq!(first_rows, *second_rows);
    assert_eq!(first_cols, *second_cols);
}

#[test]
fn row_and_col_signatures_match_bulk_computation() {
    let mut grid = Grid::new(3, 2);
    grid.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert(make_cell(1, 1, Some(CellValue::Text("text".into())), None));
    grid.insert(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    for r in 0..3 {
        assert_eq!(
            grid.compute_row_signature(r).hash,
            row_sigs[r as usize].hash
        );
    }

    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");
    for c in 0..2 {
        assert_eq!(
            grid.compute_col_signature(c).hash,
            col_sigs[c as usize].hash
        );
    }
}

#[test]
fn compute_all_signatures_recomputes_after_mutation() {
    let mut grid = Grid::new(3, 3);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert(make_cell(1, 1, Some(CellValue::Text("x".into())), None));

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    grid.insert(make_cell(1, 1, Some(CellValue::Text("y".into())), None));
    grid.insert(make_cell(2, 2, Some(CellValue::Bool(true)), None));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_ne!(first_rows[1].hash, second_rows[1].hash);
    assert_ne!(first_cols[1].hash, second_cols[1].hash);
}

#[test]
fn row_signatures_distinguish_column_positions() {
    let mut grid1 = Grid::new(1, 2);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert(make_cell(0, 1, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(1, 2);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert(make_cell(0, 1, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn col_signatures_distinguish_row_positions() {
    let mut grid1 = Grid::new(2, 1);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert(make_cell(1, 0, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(2, 1);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert(make_cell(1, 0, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_col_signature(0);
    let sig2 = grid2.compute_col_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn row_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid1.insert(make_cell(0, 1, Some(CellValue::Text("mix".into())), None));
    grid1.insert(make_cell(0, 2, Some(CellValue::Bool(true)), None));

    let mut grid2 = Grid::new(1, 3);
    grid2.insert(make_cell(0, 2, Some(CellValue::Bool(true)), None));
    grid2.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid2.insert(make_cell(0, 1, Some(CellValue::Text("mix".into())), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.row_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.row_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid1.insert(make_cell(1, 0, Some(CellValue::Text("col".into())), None));
    grid1.insert(make_cell(2, 0, Some(CellValue::Bool(false)), None));

    let mut grid2 = Grid::new(3, 1);
    grid2.insert(make_cell(2, 0, Some(CellValue::Bool(false)), None));
    grid2.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid2.insert(make_cell(1, 0, Some(CellValue::Text("col".into())), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.col_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.col_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(3, 1);
    grid_num.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(3, 1);
    grid_text.insert(make_cell(0, 0, Some(CellValue::Text("1".into())), None));

    let mut grid_bool = Grid::new(3, 1);
    grid_bool.insert(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_col_signature(0).hash;
    let txt = grid_text.compute_col_signature(0).hash;
    let boo = grid_bool.compute_col_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(1, 1);
    grid_num.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(1, 1);
    grid_text.insert(make_cell(0, 0, Some(CellValue::Text("1".into())), None));

    let mut grid_bool = Grid::new(1, 1);
    grid_bool.insert(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_row_signature(0).hash;
    let txt = grid_text.compute_row_signature(0).hash;
    let boo = grid_bool.compute_row_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_ignores_empty_trailing_cells() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(1, 10);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_ignores_empty_trailing_rows() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(10, 1);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_includes_formulas_by_default() {
    let mut with_formula = Grid::new(2, 1);
    with_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut without_formula = Grid::new(2, 1);
    without_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = with_formula.compute_col_signature(0).hash;
    let sig_without = without_formula.compute_col_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

#[test]
fn col_signature_includes_formulas_sparse() {
    let mut formula_short = Grid::new(5, 1);
    formula_short.insert(make_cell(
        0,
        0,
        Some(CellValue::Text("foo".into())),
        Some("=A2"),
    ));

    let mut formula_tall = Grid::new(10, 1);
    formula_tall.insert(make_cell(
        0,
        0,
        Some(CellValue::Text("foo".into())),
        Some("=A2"),
    ));

    let mut value_only = Grid::new(10, 1);
    value_only.insert(make_cell(0, 0, Some(CellValue::Text("foo".into())), None));

    let sig_formula_short = formula_short.compute_col_signature(0).hash;
    let sig_formula_tall = formula_tall.compute_col_signature(0).hash;
    let sig_value_only = value_only.compute_col_signature(0).hash;

    assert_eq!(sig_formula_short, sig_formula_tall);
    assert_ne!(sig_formula_short, sig_value_only);
}

#[test]
fn row_signature_includes_formulas_by_default() {
    let mut grid_with_formula = Grid::new(1, 1);
    grid_with_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut grid_without_formula = Grid::new(1, 1);
    grid_without_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = grid_with_formula.compute_row_signature(0).hash;
    let sig_without = grid_without_formula.compute_row_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

const ROW_SIGNATURE_GOLDEN: u64 = 13_315_384_008_147_106_509;
const ROW_SIGNATURE_WITH_FORMULA_GOLDEN: u64 = 3_920_348_561_402_334_617;

#[test]
fn row_signature_golden_constant_small_grid() {
    let mut grid = Grid::new(1, 3);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert(make_cell(0, 1, Some(CellValue::Text("x".into())), None));
    grid.insert(make_cell(0, 2, Some(CellValue::Bool(false)), None));

    let sig = grid.compute_row_signature(0);
    assert_eq!(sig.hash, ROW_SIGNATURE_GOLDEN);
}

#[test]
fn row_signature_golden_constant_with_formula() {
    let mut grid = Grid::new(1, 2);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));
    grid.insert(make_cell(0, 1, Some(CellValue::Text("bar".into())), None));

    let sig = grid.compute_row_signature(0);
    assert_eq!(sig.hash, ROW_SIGNATURE_WITH_FORMULA_GOLDEN);
}
```

---

### File: `core\tests\sparse_grid_tests.rs`

```rust
use excel_diff::{Cell, CellAddress, CellValue, Grid};

#[test]
fn sparse_grid_empty_has_zero_cells() {
    let grid = Grid::new(1000, 1000);
    assert_eq!(grid.cell_count(), 0);
    assert!(grid.is_empty());
    assert_eq!(grid.nrows, 1000);
    assert_eq!(grid.ncols, 1000);
}

#[test]
fn sparse_grid_insert_and_retrieve() {
    let mut grid = Grid::new(100, 100);
    let cell = Cell {
        row: 50,
        col: 50,
        address: CellAddress::from_indices(50, 50),
        value: Some(CellValue::Number(42.0)),
        formula: None,
    };
    grid.insert(cell);
    assert_eq!(grid.cell_count(), 1);
    let retrieved = grid.get(50, 50).expect("cell should exist");
    assert_eq!(retrieved.value, Some(CellValue::Number(42.0)));
    assert!(grid.get(0, 0).is_none());
}

#[test]
fn sparse_grid_iter_cells_only_populated() {
    let mut grid = Grid::new(1000, 1000);
    for i in 0..10 {
        let cell = Cell {
            row: i * 100,
            col: i * 100,
            address: CellAddress::from_indices(i * 100, i * 100),
            value: Some(CellValue::Number(i as f64)),
            formula: None,
        };
        grid.insert(cell);
    }
    let cells: Vec<_> = grid.iter_cells().collect();
    assert_eq!(cells.len(), 10);
}

#[test]
fn sparse_grid_memory_efficiency() {
    let grid = Grid::new(10_000, 1_000);
    assert!(std::mem::size_of_val(&grid) < 1024);
}

#[test]
fn rows_iter_covers_all_rows() {
    let grid = Grid::new(3, 5);
    let rows: Vec<u32> = grid.rows_iter().collect();
    assert_eq!(rows, vec![0, 1, 2]);
}

#[test]
fn cols_iter_covers_all_cols() {
    let grid = Grid::new(4, 2);
    let cols: Vec<u32> = grid.cols_iter().collect();
    assert_eq!(cols, vec![0, 1]);
}

#[test]
fn rows_iter_and_get_are_consistent() {
    let mut grid = Grid::new(2, 2);
    grid.insert(Cell {
        row: 1,
        col: 1,
        address: CellAddress::from_indices(1, 1),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    for r in grid.rows_iter() {
        for c in grid.cols_iter() {
            let _ = grid.get(r, c);
        }
    }
}

#[test]
fn sparse_grid_all_empty_rows_have_zero_signatures() {
    let mut grid = Grid::new(2, 3);

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(row_sigs.len(), 2);
    assert_eq!(col_sigs.len(), 3);
    assert!(row_sigs.iter().all(|sig| sig.hash == 0));
    assert!(col_sigs.iter().all(|sig| sig.hash == 0));
}

#[test]
fn compute_signatures_on_sparse_grid_produces_hashes() {
    let mut grid = Grid::new(4, 4);
    grid.insert(Cell {
        row: 1,
        col: 3,
        address: CellAddress::from_indices(1, 3),
        value: Some(CellValue::Text("value".into())),
        formula: Some("=A1".into()),
    });

    grid.compute_all_signatures();

    let row_hash = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist")[1]
        .hash;
    let col_hash = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist")[3]
        .hash;

    assert_ne!(row_hash, 0);
    assert_ne!(col_hash, 0);
}

#[test]
fn compute_all_signatures_matches_direct_computation() {
    let mut grid = Grid::new(3, 3);
    grid.insert(Cell {
        row: 0,
        col: 1,
        address: CellAddress::from_indices(0, 1),
        value: Some(CellValue::Number(10.0)),
        formula: Some("=5+5".into()),
    });
    grid.insert(Cell {
        row: 1,
        col: 0,
        address: CellAddress::from_indices(1, 0),
        value: Some(CellValue::Text("x".into())),
        formula: None,
    });
    grid.insert(Cell {
        row: 2,
        col: 2,
        address: CellAddress::from_indices(2, 2),
        value: Some(CellValue::Bool(false)),
        formula: Some("=A1".into()),
    });

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(grid.compute_row_signature(0).hash, row_sigs[0].hash);
    assert_eq!(grid.compute_row_signature(2).hash, row_sigs[2].hash);
    assert_eq!(grid.compute_col_signature(0).hash, col_sigs[0].hash);
    assert_eq!(grid.compute_col_signature(2).hash, col_sigs[2].hash);
}
```

---

### File: `core\tests\common\mod.rs`

```rust
use std::path::PathBuf;

pub fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path.push(filename);
    path
}
```

---

### File: `fixtures\manifest.yaml`

```yaml
scenarios:
  # --- Phase 1.1: Basic File Opening ---
  - id: "smoke_minimal"
    generator: "basic_grid"
    args: { rows: 1, cols: 1 }
    output: "minimal.xlsx"

  # --- Phase 1.2: Is this a ZIP? ---
  - id: "container_random_zip"
    generator: "corrupt_container"
    args: { mode: "random_zip" }
    output: "random_zip.zip"
    
  - id: "container_no_content_types"
    generator: "corrupt_container"
    args: { mode: "no_content_types" }
    output: "no_content_types.xlsx"

  - id: "container_not_zip_text"
    generator: "corrupt_container"
    args: { mode: "not_zip_text" }
    output: "not_a_zip.txt"

  # --- PG1: Workbook -> Sheet -> Grid IR sanity ---
  - id: "pg1_basic_two_sheets"
    generator: "basic_grid"
    args: { rows: 3, cols: 3, two_sheets: true } # Sheet1 3x3, Sheet2 5x2 (logic in generator)
    output: "pg1_basic_two_sheets.xlsx"

  - id: "pg1_sparse"
    generator: "sparse_grid"
    output: "pg1_sparse_used_range.xlsx"

  - id: "pg1_mixed"
    generator: "edge_case"
    output: "pg1_empty_and_mixed_sheets.xlsx"

  # --- PG2: Addressing and index invariants ---
  - id: "pg2_addressing"
    generator: "address_sanity"
    args:
      targets: ["A1", "B2", "C3", "Z1", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1"]
    output: "pg2_addressing_matrix.xlsx"

  # --- PG3: Cell snapshots and comparison semantics ---
  - id: "pg3_types"
    generator: "value_formula"
    output: "pg3_value_and_formula_cells.xlsx"

  # --- JSON diff: simple non-empty change ---
  - id: "json_diff_single_cell"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: "1"
      value_b: "2"
    output:
      - "json_diff_single_cell_a.xlsx"
      - "json_diff_single_cell_b.xlsx"

  - id: "json_diff_single_bool"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: true
      value_b: false
    output:
      - "json_diff_bool_a.xlsx"
      - "json_diff_bool_b.xlsx"

  - id: "json_diff_value_to_empty"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: "1"
      value_b: null
    output:
      - "json_diff_value_to_empty_a.xlsx"
      - "json_diff_value_to_empty_b.xlsx"

  # --- Sheet identity: case-only renames ---
  - id: "sheet_case_only_rename"
    generator: "sheet_case_rename"
    args:
      sheet_a: "Sheet1"
      sheet_b: "sheet1"
      cell: "A1"
      value_a: 1.0
      value_b: 1.0
    output:
      - "sheet_case_only_rename_a.xlsx"
      - "sheet_case_only_rename_b.xlsx"

  - id: "sheet_case_only_rename_cell_edit"
    generator: "sheet_case_rename"
    args:
      sheet_a: "Sheet1"
      sheet_b: "sheet1"
      cell: "A1"
      value_a: 1.0
      value_b: 2.0
    output:
      - "sheet_case_only_rename_edit_a.xlsx"
      - "sheet_case_only_rename_edit_b.xlsx"

  # --- PG6: Object graph vs grid responsibilities ---
  - id: "pg6_sheet_added"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_added"
    output:
      - "pg6_sheet_added_a.xlsx"
      - "pg6_sheet_added_b.xlsx"

  - id: "pg6_sheet_removed"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_removed"
    output:
      - "pg6_sheet_removed_a.xlsx"
      - "pg6_sheet_removed_b.xlsx"

  - id: "pg6_sheet_renamed"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_renamed"
    output:
      - "pg6_sheet_renamed_a.xlsx"
      - "pg6_sheet_renamed_b.xlsx"

  - id: "pg6_sheet_and_grid_change"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_and_grid_change"
    output:
      - "pg6_sheet_and_grid_change_a.xlsx"
      - "pg6_sheet_and_grid_change_b.xlsx"

  # --- Milestone 2.2: Base64 Correctness ---
  - id: "corrupt_base64"
    generator: "mashup_corrupt"
    args: 
      base_file: "templates/base_query.xlsx"
      mode: "byte_flip"
    output: "corrupt_base64.xlsx"

  - id: "duplicate_datamashup_parts"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
    output: "duplicate_datamashup_parts.xlsx"

  - id: "duplicate_datamashup_elements"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
      mode: "element"
    output: "duplicate_datamashup_elements.xlsx"

  - id: "mashup_utf16_le"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-le"
    output: "mashup_utf16_le.xlsx"

  - id: "mashup_utf16_be"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-be"
    output: "mashup_utf16_be.xlsx"

  - id: "mashup_base64_whitespace"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      whitespace: true
    output: "mashup_base64_whitespace.xlsx"

  # --- Milestone 4.1: PackageParts ---
  - id: "m4_packageparts_one_query"
    generator: "mashup:one_query"
    args:
      base_file: "templates/base_query.xlsx"
    output: "one_query.xlsx"

  - id: "m4_packageparts_multi_embedded"
    generator: "mashup:multi_query_with_embedded"
    args:
      base_file: "templates/base_query.xlsx"
    output: "multi_query_with_embedded.xlsx"

  # --- Milestone 4.2-4.4: Permissions / Metadata ---
  - id: "permissions_defaults"
    generator: "mashup:permissions_metadata"
    args:
      mode: "permissions_defaults"
      base_file: "templates/base_query.xlsx"
    output: "permissions_defaults.xlsx"

  - id: "permissions_firewall_off"
    generator: "mashup:permissions_metadata"
    args:
      mode: "permissions_firewall_off"
      base_file: "templates/base_query.xlsx"
    output: "permissions_firewall_off.xlsx"

  - id: "metadata_simple"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_simple"
      base_file: "templates/base_query.xlsx"
    output: "metadata_simple.xlsx"

  - id: "metadata_query_groups"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_query_groups"
      base_file: "templates/base_query.xlsx"
    output: "metadata_query_groups.xlsx"

  - id: "metadata_hidden_queries"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_hidden_queries"
      base_file: "templates/base_query.xlsx"
    output: "metadata_hidden_queries.xlsx"

  # --- Milestone 6: Basic M Diffs ---
  - id: "m_change_literal"
    generator: "mashup_inject"
    args:
      base_file: "templates/base_query.xlsx"
      # This query adds a step, changing the definition
      m_code: |
        section Section1;
        shared Query1 = let
            Source = Csv.Document(File.Contents("C:\data.csv"),[Delimiter=",", Columns=2, Encoding=1252, QuoteStyle=QuoteStyle.None]),
            #"Changed Type" = Table.TransformColumnTypes(Source,{{"Column1", type text}, {"Column2", type text}}),
            #"Added Custom" = Table.AddColumn(#"Changed Type", "Custom", each 2)
        in
            #"Added Custom";
    output: "m_change_literal_b.xlsx"

  # --- P1: Large Dense Grid (Performance Baseline) ---
  - id: "p1_large_dense"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "dense" # Deterministic "R1C1" style data
    output: "grid_large_dense.xlsx"

  # --- P2: Large Noise Grid (Worst-case Alignment) ---
  - id: "p2_large_noise"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "noise" # Random float data
      seed: 12345
    output: "grid_large_noise.xlsx"

  # --- D1: Keyed Equality (Database Mode) ---
  # File A: Ordered IDs 1..1000
  - id: "db_equal_ordered_a"
    generator: "db_keyed"
    args: { count: 1000, shuffle: false, seed: 42 }
    output: "db_equal_ordered_a.xlsx"

  # File B: Same data, random order (Tests O(N) alignment)
  - id: "db_equal_ordered_b"
    generator: "db_keyed"
    args: { count: 1000, shuffle: true, seed: 42 }
    output: "db_equal_ordered_b.xlsx"

  # --- D2: Row Added (Database Mode) ---
  - id: "db_row_added_b"
    generator: "db_keyed"
    args: 
      count: 1000 
      seed: 42 
      # Inject a new ID at the end
      extra_rows: [{id: 1001, name: "New Row", amount: 999}]
    output: "db_row_added_b.xlsx"
```

---

### File: `fixtures\pyproject.toml`

```yaml
[project]
name = "excel-fixtures"
version = "0.1.0"
description = "Deterministic artifact generator for Excel Diff testing"
readme = "README.md"
requires-python = ">=3.9"
dependencies = [
    "openpyxl>=3.1.0",
    "lxml>=4.9.0",
    "jinja2>=3.1.0",
    "pyyaml>=6.0",
]

[project.scripts]
generate-fixtures = "src.generate:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src"]

```

---

### File: `fixtures\src\generate.py`

```python
import argparse
import yaml
import sys
from pathlib import Path
from typing import Dict, Any, List

# Import generators
from generators.grid import (
    BasicGridGenerator, 
    SparseGridGenerator, 
    EdgeCaseGenerator, 
    AddressSanityGenerator,
    ValueFormulaGenerator,
    SingleCellDiffGenerator,
    SheetCaseRenameGenerator,
    Pg6SheetScenarioGenerator,
)
from generators.corrupt import ContainerCorruptGenerator
from generators.mashup import (
    MashupCorruptGenerator,
    MashupDuplicateGenerator,
    MashupInjectGenerator,
    MashupEncodeGenerator,
    MashupMultiEmbeddedGenerator,
    MashupOneQueryGenerator,
    MashupPermissionsMetadataGenerator,
)
from generators.perf import LargeGridGenerator
from generators.database import KeyedTableGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "single_cell_diff": SingleCellDiffGenerator,
    "sheet_case_rename": SheetCaseRenameGenerator,
    "pg6_sheet_scenario": Pg6SheetScenarioGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_duplicate": MashupDuplicateGenerator,
    "mashup_inject": MashupInjectGenerator,
    "mashup_encode": MashupEncodeGenerator,
    "mashup:one_query": MashupOneQueryGenerator,
    "mashup:multi_query_with_embedded": MashupMultiEmbeddedGenerator,
    "mashup:permissions_metadata": MashupPermissionsMetadataGenerator,
    "perf_large": LargeGridGenerator,
    "db_keyed": KeyedTableGenerator,
}

def load_manifest(manifest_path: Path) -> Dict[str, Any]:
    if not manifest_path.exists():
        print(f"Error: Manifest file not found at {manifest_path}")
        sys.exit(1)
    
    with open(manifest_path, 'r') as f:
        try:
            return yaml.safe_load(f)
        except yaml.YAMLError as e:
            print(f"Error parsing manifest: {e}")
            sys.exit(1)

def ensure_output_dir(output_dir: Path):
    output_dir.mkdir(parents=True, exist_ok=True)

def main():
    script_dir = Path(__file__).parent.resolve()
    fixtures_root = script_dir.parent
    
    default_manifest = fixtures_root / "manifest.yaml"
    default_output = fixtures_root / "generated"

    parser = argparse.ArgumentParser(description="Generate Excel fixtures based on a manifest.")
    parser.add_argument("--manifest", type=Path, default=default_manifest, help="Path to the manifest YAML file.")
    parser.add_argument("--output-dir", type=Path, default=default_output, help="Directory to output generated files.")
    parser.add_argument("--force", action="store_true", help="Force regeneration of existing files.")
    
    args = parser.parse_args()
    
    manifest = load_manifest(args.manifest)
    ensure_output_dir(args.output_dir)
    
    scenarios = manifest.get('scenarios', [])
    print(f"Found {len(scenarios)} scenarios in manifest.")
    
    for scenario in scenarios:
        scenario_id = scenario.get('id')
        generator_name = scenario.get('generator')
        generator_args = scenario.get('args', {})
        outputs = scenario.get('output')
        
        if not scenario_id or not generator_name or not outputs:
            print(f"Skipping invalid scenario: {scenario}")
            continue
            
        print(f"Processing scenario: {scenario_id} (Generator: {generator_name})")
        
        if generator_name not in GENERATORS:
            print(f"  Warning: Generator '{generator_name}' not implemented yet. Skipping.")
            continue
        
        try:
            generator_class = GENERATORS[generator_name]
            generator = generator_class(generator_args)
            generator.generate(args.output_dir, outputs)
            print(f"  Success: Generated {outputs}")
        except Exception as e:
            print(f"  Error generating scenario {scenario_id}: {e}")
            import traceback
            traceback.print_exc()

if __name__ == "__main__":
    main()
```

---

### File: `fixtures\src\__init__.py`

```python

```

---

### File: `fixtures\src\generators\base.py`

```python
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, Any, Union, List

class BaseGenerator(ABC):
    """
    Abstract base class for all fixture generators.
    """
    def __init__(self, args: Dict[str, Any]):
        self.args = args

    @abstractmethod
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        """
        Generates the fixture file(s).
        
        :param output_dir: The directory to save the file(s) in.
        :param output_names: The name(s) of the output file(s) as specified in the manifest.
        """
        pass

    def _post_process_injection(self, file_path: Path, injection_callback):
        """
        Implements the "Pass 2" architecture:
        1. Opens the generated xlsx (zip).
        2. Injects/Modifies streams (DataMashup, etc).
        3. Saves back.
        
        This is a crucial architectural decision to handle openpyxl stripping customXml.
        """
        pass

```

---

### File: `fixtures\src\generators\corrupt.py`

```python
import zipfile
import io
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class ContainerCorruptGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        mode = self.args.get('mode', 'no_content_types')
        
        for name in output_names:
            # Create a dummy zip
            out_path = output_dir / name
            
            if mode == 'random_zip':
                # Just a zip with a text file
                with zipfile.ZipFile(out_path, 'w') as z:
                    z.writestr("hello.txt", "This is not excel")
                    
            elif mode == 'no_content_types':
                # Create a valid excel in memory, then strip [Content_Types].xml
                buffer = io.BytesIO()
                import openpyxl
                wb = openpyxl.Workbook()
                # Add some content just so it's not totally empty
                wb.active['A1'] = 1
                wb.save(buffer)
                buffer.seek(0)
                
                with zipfile.ZipFile(buffer, 'r') as zin:
                    with zipfile.ZipFile(out_path, 'w') as zout:
                        for item in zin.infolist():
                            if item.filename != "[Content_Types].xml":
                                zout.writestr(item, zin.read(item.filename))
            elif mode == 'not_zip_text':
                out_path.write_text("This is not a zip container", encoding="utf-8")
            else:
                raise ValueError(f"Unsupported corrupt_container mode: {mode}")

```

---

### File: `fixtures\src\generators\database.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class KeyedTableGenerator(BaseGenerator):
    """
    Generates datasets with Primary Keys (ID columns).
    Capable of shuffling rows to test O(N) alignment (Database Mode).
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        count = self.args.get('count', 100)
        shuffle = self.args.get('shuffle', False)
        seed = self.args.get('seed', 42)
        extra_rows = self.args.get('extra_rows', [])

        # Use deterministic seed
        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Data"

            # 1. Define Base Data (List of Dicts)
            # Schema: [ID, Name, Amount, Category]
            data_rows = []
            for i in range(1, count + 1):
                data_rows.append({
                    'id': i,
                    'name': f"Customer_{i}",
                    'amount': i * 10.5,
                    'category': rng.choice(['A', 'B', 'C'])
                })

            # 2. Apply Mutations (Additions)
            # This allows us to inject specific "diffs" like D2 (Row Added)
            for row in extra_rows:
                data_rows.append(row)

            # 3. Apply Shuffle (The core D1 test)
            if shuffle:
                rng.shuffle(data_rows)

            # 4. Write to Sheet
            # Header
            headers = ['ID', 'Name', 'Amount', 'Category']
            ws.append(headers)

            for row in data_rows:
                # Ensure strictly ordered list matching headers
                ws.append([
                    row.get('id'),
                    row.get('name'),
                    row.get('amount'),
                    row.get('category')
                ])

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\grid.py`

```python
import openpyxl
import zipfile
import xml.etree.ElementTree as ET
from openpyxl.utils import get_column_letter
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class BasicGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        rows = self.args.get('rows', 5)
        cols = self.args.get('cols', 5)
        two_sheets = self.args.get('two_sheets', False)
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sheet1"
            
            # Fill grid
            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")
            
            # Check if we need a second sheet
            if two_sheets:
                ws2 = wb.create_sheet(title="Sheet2")
                # Different dimensions for Sheet2 (PG1 requirement: 5x2)
                # If args are customized we might need more logic, but for PG1 this is sufficient or we use defaults
                s2_rows = 5
                s2_cols = 2
                for r in range(1, s2_rows + 1):
                    for c in range(1, s2_cols + 1):
                         ws2.cell(row=r, column=c, value=f"S2_R{r}C{c}")

            wb.save(output_dir / name)

class SparseGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sparse"
            
            # Specifics for pg1_sparse_used_range
            ws['A1'] = "A1"
            ws['B2'] = "B2"
            ws['G10'] = "G10" # Forces extent
            # Row 5 and Col D are empty implicitly by not writing to them
            
            wb.save(output_dir / name)

class EdgeCaseGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
        
        for name in output_names:
            wb = openpyxl.Workbook()
            # Remove default sheet
            default_ws = wb.active
            wb.remove(default_ws)
            
            # Empty Sheet
            wb.create_sheet("Empty")
            
            # Values Only
            ws_val = wb.create_sheet("ValuesOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    ws_val.cell(row=r, column=c, value=r*c)
            
            # Formulas Only
            ws_form = wb.create_sheet("FormulasOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    # Reference ValuesOnly sheet
                    col_letter = get_column_letter(c)
                    ws_form.cell(row=r, column=c, value=f"=ValuesOnly!{col_letter}{r}")
            
            wb.save(output_dir / name)

class AddressSanityGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        targets = self.args.get('targets', ["A1", "B2", "Z10"])
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Addresses"
            
            for addr in targets:
                ws[addr] = addr
                
            wb.save(output_dir / name)

class ValueFormulaGenerator(BaseGenerator):
    """PG3: Types, formulas, values"""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Types"
            
            ws['A1'] = 42
            ws['A2'] = "hello"
            ws['A3'] = True
            # A4 empty
            
            ws['B1'] = "=A1+1"
            ws['B2'] = '="hello" & " world"'
            ws['B3'] = "=A1>0"
            
            output_path = output_dir / name
            wb.save(output_path)
            self._inject_formula_caches(output_path)

    def _inject_formula_caches(self, path: Path):
        ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
        with zipfile.ZipFile(path, "r") as zf:
            sheet_xml = zf.read("xl/worksheets/sheet1.xml")
            other_files = {
                info.filename: zf.read(info.filename)
                for info in zf.infolist()
                if info.filename != "xl/worksheets/sheet1.xml"
            }

        root = ET.fromstring(sheet_xml)

        def update_cell(ref: str, value: str, cell_type: str | None = None):
            cell = root.find(f".//{{{ns}}}c[@r='{ref}']")
            if cell is None:
                return
            if cell_type:
                cell.set("t", cell_type)
            v = cell.find(f"{{{ns}}}v")
            if v is None:
                v = ET.SubElement(cell, f"{{{ns}}}v")
            v.text = value

        update_cell("B1", "43")
        update_cell("B2", "hello world", "str")
        update_cell("B3", "1", "b")

        ET.register_namespace("", ns)
        updated_sheet = ET.tostring(root, encoding="utf-8", xml_declaration=False)
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("xl/worksheets/sheet1.xml", updated_sheet)
            for name, data in other_files.items():
                zf.writestr(name, data)

class SingleCellDiffGenerator(BaseGenerator):
    """Generates a tiny pair of workbooks with a single differing cell."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("single_cell_diff generator expects exactly two output filenames")

        rows = self.args.get('rows', 3)
        cols = self.args.get('cols', 3)
        sheet = self.args.get('sheet', "Sheet1")
        target_cell = self.args.get('target_cell', "C3")
        value_a = self.args.get('value_a', "1")
        value_b = self.args.get('value_b', "2")

        def create_workbook(value, name: str):
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = sheet

            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")

            ws[target_cell] = value
            wb.save(output_dir / name)

        create_workbook(value_a, output_names[0])
        create_workbook(value_b, output_names[1])

class SheetCaseRenameGenerator(BaseGenerator):
    """Generates a pair of workbooks that differ only by sheet name casing, with optional cell edit."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("sheet_case_rename generator expects exactly two output filenames")

        sheet_a = self.args.get("sheet_a", "Sheet1")
        sheet_b = self.args.get("sheet_b", "sheet1")
        cell = self.args.get("cell", "A1")
        value_a = self.args.get("value_a", 1.0)
        value_b = self.args.get("value_b", value_a)

        def create_workbook(sheet_name: str, value, output_name: str):
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = sheet_name
            ws[cell] = value
            wb.save(output_dir / output_name)

        create_workbook(sheet_a, value_a, output_names[0])
        create_workbook(sheet_b, value_b, output_names[1])

class Pg6SheetScenarioGenerator(BaseGenerator):
    """Generates workbook pairs for PG6 sheet add/remove/rename vs grid responsibilities."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("pg6_sheet_scenario generator expects exactly two output filenames")

        mode = self.args.get("mode")
        a_path = output_dir / output_names[0]
        b_path = output_dir / output_names[1]

        if mode == "sheet_added":
            self._gen_sheet_added(a_path, b_path)
        elif mode == "sheet_removed":
            self._gen_sheet_removed(a_path, b_path)
        elif mode == "sheet_renamed":
            self._gen_sheet_renamed(a_path, b_path)
        elif mode == "sheet_and_grid_change":
            self._gen_sheet_and_grid_change(a_path, b_path)
        else:
            raise ValueError(f"Unsupported PG6 mode: {mode}")

    def _fill_grid(self, worksheet, rows: int, cols: int, prefix: str = "R"):
        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                worksheet.cell(row=r, column=c, value=f"{prefix}{r}C{c}")

    def _gen_sheet_added(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, 5, 5)
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, 5, 5)
        ws_new = wb_b.create_sheet("NewSheet")
        self._fill_grid(ws_new, 3, 3, prefix="N")
        wb_b.save(b_path)

    def _gen_sheet_removed(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, 5, 5)
        ws_old = wb_a.create_sheet("OldSheet")
        self._fill_grid(ws_old, 3, 3, prefix="O")
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, 5, 5)
        wb_b.save(b_path)

    def _gen_sheet_renamed(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_old = wb_a.active
        ws_old.title = "OldName"
        self._fill_grid(ws_old, 3, 3)
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_new = wb_b.active
        ws_new.title = "NewName"
        self._fill_grid(ws_new, 3, 3)
        wb_b.save(b_path)

    def _gen_sheet_and_grid_change(self, a_path: Path, b_path: Path):
        base_rows = 5
        base_cols = 5

        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, base_rows, base_cols)
        ws_aux_a = wb_a.create_sheet("Aux")
        self._fill_grid(ws_aux_a, 3, 3, prefix="A")
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, base_rows, base_cols)
        ws_main_b["A1"] = "Main changed 1"
        ws_main_b["B2"] = "Main changed 2"
        ws_main_b["C3"] = "Main changed 3"

        ws_aux_b = wb_b.create_sheet("Aux")
        self._fill_grid(ws_aux_b, 3, 3, prefix="A")

        ws_scratch = wb_b.create_sheet("Scratch")
        self._fill_grid(ws_scratch, 2, 2, prefix="S")
        wb_b.save(b_path)
```

---

### File: `fixtures\src\generators\mashup.py`

```python
import base64
import copy
import io
import random
import re
import struct
import zipfile
from pathlib import Path
from typing import Callable, List, Optional, Union
from xml.etree import ElementTree as ET
from lxml import etree
from .base import BaseGenerator

# XML Namespaces
NS = {'dm': 'http://schemas.microsoft.com/DataMashup'}

class MashupBaseGenerator(BaseGenerator):
    """Base class for handling the outer Excel container and finding DataMashup."""
    
    def _get_mashup_element(self, tree):
        if tree.tag.endswith("DataMashup"):
            return tree
        return tree.find('.//dm:DataMashup', namespaces=NS)

    def _process_excel_container(
        self,
        base_path,
        output_path,
        callback,
        text_mutator: Optional[Callable[[str], str]] = None,
    ):
        """
        Generic wrapper to open xlsx, find customXml, apply a callback to the 
        DataMashup bytes, and save the result.
        """
        # Copy base file structure to output
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for item in zin.infolist():
                    buffer = zin.read(item.filename)
                    
                    # We only care about the item containing DataMashup
                    # Usually customXml/item1.xml, but we check content to be safe
                    has_marker = b"DataMashup" in buffer or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in buffer
                    if item.filename.startswith("customXml/item") and has_marker:
                        # Parse XML
                        root = etree.fromstring(buffer)
                        dm_node = self._get_mashup_element(root)
                        
                        if dm_node is not None:
                            # 1. Decode
                            # The text content might have whitespace/newlines, strip them
                            b64_text = dm_node.text.strip() if dm_node.text else ""
                            if b64_text:
                                raw_bytes = base64.b64decode(b64_text)
                                
                                # 2. Apply modification (The Callback)
                                new_bytes = callback(raw_bytes)
                                
                                # 3. Encode back
                                new_text = base64.b64encode(new_bytes).decode('utf-8')
                                if text_mutator is not None:
                                    new_text = text_mutator(new_text)
                                dm_node.text = new_text
                                buffer = etree.tostring(root, encoding='utf-8', xml_declaration=True)
                    
                    zout.writestr(item, buffer)

class MashupCorruptGenerator(MashupBaseGenerator):
    """Fuzzes the DataMashup bytes to test error handling."""
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        if not base_file_arg:
            raise ValueError("MashupCorruptGenerator requires 'base_file' argument")

        # Resolve base file relative to current working directory or fixtures/templates
        base = Path(base_file_arg)
        if not base.exists():
             # Try looking in fixtures/templates if a relative path was given
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        mode = self.args.get('mode', 'byte_flip')

        def corruptor(data):
            mutable = bytearray(data)
            if len(mutable) == 0:
                return bytes(mutable)

            if mode == 'byte_flip':
                # Flip a byte in the middle
                idx = len(mutable) // 2
                mutable[idx] = mutable[idx] ^ 0xFF
            elif mode == 'truncate':
                return mutable[:len(mutable)//2]
            return bytes(mutable)

        for name in output_names:
            # Convert Path objects to strings for resolve() to work correctly if there's a mix
            # Actually output_dir is a Path. name is str.
            # .resolve() resolves symlinks and relative paths to absolute
            target_path = (output_dir / name).resolve()
            text_mutator = self._garble_base64_text if mode == 'byte_flip' else None
            self._process_excel_container(
                base.resolve(),
                target_path,
                corruptor,
                text_mutator=text_mutator,
            )

    def _garble_base64_text(self, encoded: str) -> str:
        if not encoded:
            return "!!"
        chars = list(encoded)
        chars[0] = "!"
        return "".join(chars)


class MashupInjectGenerator(MashupBaseGenerator):
    """
    Peels the onion:
    1. Parses MS-QDEFF binary header.
    2. Unzips PackageParts.
    3. Injects new M-Code into Section1.m.
    4. Re-zips and fixes header lengths.
    """
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        new_m_code = self.args.get('m_code')

        if not base_file_arg:
             raise ValueError("MashupInjectGenerator requires 'base_file' argument")
        if new_m_code is None:
             raise ValueError("MashupInjectGenerator requires 'm_code' argument")

        base = Path(base_file_arg)
        if not base.exists():
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        def injector(raw_bytes):
            return self._inject_m_code(raw_bytes, new_m_code)

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, injector)

    def _inject_m_code(self, raw_bytes, m_code):
        # --- 1. Parse MS-QDEFF Header ---
        # Format: Version(4) + LenPP(4) + PackageParts(...) + LenPerm(4) + ...
        # We assume Version is 0 (first 4 bytes)
        
        if len(raw_bytes) < 8:
            return raw_bytes # Too short to handle

        offset = 4
        # Read PackageParts Length
        pp_len = struct.unpack('<I', raw_bytes[offset:offset+4])[0]
        offset += 4
        
        # Extract existing components
        pp_bytes = raw_bytes[offset : offset + pp_len]
        
        # Keep the rest of the stream (Permissions, Metadata, Bindings) intact
        # We just append it later
        remainder_bytes = raw_bytes[offset + pp_len :]

        # --- 2. Modify PackageParts (Inner ZIP) ---
        new_pp_bytes = self._replace_in_zip(pp_bytes, 'Formulas/Section1.m', m_code)

        # --- 3. Rebuild Stream ---
        # New Length for PackageParts
        new_pp_len = len(new_pp_bytes)
        
        # Reconstruct: Version(0) + NewLen + NewPP + Remainder
        header = raw_bytes[:4] # Version
        len_pack = struct.pack('<I', new_pp_len)
        
        return header + len_pack + new_pp_bytes + remainder_bytes

    def _replace_in_zip(self, zip_bytes, filename, new_content):
        """Opens a ZIP byte stream, replaces a file, returns new ZIP byte stream."""
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()
        
        try:
            with zipfile.ZipFile(in_buffer, 'r') as zin:
                with zipfile.ZipFile(out_buffer, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
                    for item in zin.infolist():
                        if item.filename == filename:
                            # Write the new M code
                            zout.writestr(filename, new_content.encode('utf-8'))
                        else:
                            # Copy others
                            zout.writestr(item, zin.read(item.filename))
        except zipfile.BadZipFile:
            # Fallback if inner stream isn't a valid zip (shouldn't happen on valid QDEFF)
            return zip_bytes
            
        return out_buffer.getvalue()


class MashupPackagePartsGenerator(MashupBaseGenerator):
    """
    Generates PackageParts-focused fixtures starting from a base workbook.
    """

    variant: str = "one_query"

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        if self.variant == "one_query":
            return raw_bytes

        version, package_parts, permissions, metadata, bindings = self._split_sections(raw_bytes)
        package_xml, main_section_text, content_types = self._extract_package_parts(package_parts)

        embedded_guid = self.args.get(
            "embedded_guid", "{11111111-2222-3333-4444-555555555555}"
        )
        embedded_section_text = self.args.get(
            "embedded_section",
            self._default_embedded_section(),
        )
        updated_main_section = self._extend_main_section(main_section_text, embedded_guid)
        embedded_bytes = self._build_embedded_package(embedded_section_text, content_types)
        updated_package_parts = self._build_package_parts(
            package_xml,
            updated_main_section,
            content_types,
            embedded_guid,
            embedded_bytes,
        )

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions,
            metadata,
            bindings,
        )

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts_end = offset + package_parts_len
        if package_parts_end > len(raw_bytes):
            raise ValueError("invalid PackageParts length")
        package_parts = raw_bytes[offset:package_parts_end]
        offset = package_parts_end

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions_end = offset + permissions_len
        if permissions_end > len(raw_bytes):
            raise ValueError("invalid permissions length")
        permissions = raw_bytes[offset:permissions_end]
        offset = permissions_end

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata_end = offset + metadata_len
        if metadata_end > len(raw_bytes):
            raise ValueError("invalid metadata length")
        metadata = raw_bytes[offset:metadata_end]
        offset = metadata_end

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings_end = offset + bindings_len
        if bindings_end > len(raw_bytes):
            raise ValueError("invalid bindings length")
        bindings = raw_bytes[offset:bindings_end]
        offset = bindings_end

        if offset != len(raw_bytes):
            raise ValueError("DataMashup trailing bytes mismatch")

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _extract_package_parts(self, package_parts: bytes):
        with zipfile.ZipFile(io.BytesIO(package_parts), "r") as z:
            package_xml = z.read("Config/Package.xml")
            content_types = z.read("[Content_Types].xml")
            main_section = z.read("Formulas/Section1.m")
        return package_xml, main_section.decode("utf-8", errors="ignore"), content_types

    def _extend_main_section(self, base_section: str, embedded_guid: str) -> str:
        stripped = base_section.rstrip()
        lines = [
            stripped,
            "",
            "shared EmbeddedQuery = let",
            f'    Source = Embedded.Value("Content/{embedded_guid}.package")',
            "in",
            "    Source;",
        ]
        return "\n".join(lines)

    def _build_embedded_package(self, section_text: str, content_types_template: bytes) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Formulas/Section1.m", section_text)
        return buffer.getvalue()

    def _build_package_parts(
        self,
        package_xml: bytes,
        main_section: str,
        content_types_template: bytes,
        embedded_guid: str,
        embedded_package: bytes,
    ) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Config/Package.xml", package_xml)
            z.writestr("Formulas/Section1.m", main_section)
            z.writestr(f"Content/{embedded_guid}.package", embedded_package)
        return buffer.getvalue()

    def _augment_content_types(self, content_types_bytes: bytes) -> str:
        text = content_types_bytes.decode("utf-8", errors="ignore")
        if "Extension=\"package\"" not in text and "Extension='package'" not in text:
            text = text.replace(
                "</Types>",
                '<Default Extension="package" ContentType="application/octet-stream" /></Types>',
                1,
            )
        return text

    def _default_embedded_section(self) -> str:
        return "\n".join(
            [
                "section Section1;",
                "",
                "shared Inner = let",
                "    Source = 1",
                "in",
                "    Source;",
            ]
        )


class MashupOneQueryGenerator(MashupPackagePartsGenerator):
    variant = "one_query"


class MashupMultiEmbeddedGenerator(MashupPackagePartsGenerator):
    variant = "multi_query_with_embedded"


class MashupDuplicateGenerator(MashupBaseGenerator):
    """
    Duplicates the customXml part that contains DataMashup to produce two
    DataMashup occurrences in a single workbook.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        mode = self.args.get('mode', 'part')
        if not base_file_arg:
            raise ValueError("MashupDuplicateGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            if mode == 'part':
                self._duplicate_datamashup_part(base.resolve(), target_path)
            elif mode == 'element':
                self._duplicate_datamashup_element(base.resolve(), target_path)
            else:
                raise ValueError(f"Unsupported duplicate mode: {mode}")

    def _duplicate_datamashup_part(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            try:
                item1_xml = zin.read("customXml/item1.xml")
                item_props1 = zin.read("customXml/itemProps1.xml")
                item1_rels = zin.read("customXml/_rels/item1.xml.rels")
                content_types = zin.read("[Content_Types].xml")
                workbook_rels = zin.read("xl/_rels/workbook.xml.rels")
            except KeyError as e:
                raise FileNotFoundError(f"Required DataMashup part missing: {e}") from e

            updated_content_types = self._add_itemprops_override(content_types)
            updated_workbook_rels = self._add_workbook_relationship(workbook_rels)
            item2_rels = item1_rels.replace(b"itemProps1.xml", b"itemProps2.xml")
            item_props2 = item_props1.replace(
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BE}",
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BF}",
            )

            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename == "[Content_Types].xml":
                        data = updated_content_types
                    elif info.filename == "xl/_rels/workbook.xml.rels":
                        data = updated_workbook_rels
                    zout.writestr(info, data)

                zout.writestr("customXml/item2.xml", item1_xml)
                zout.writestr("customXml/itemProps2.xml", item_props2)
                zout.writestr("customXml/_rels/item2.xml.rels", item2_rels)

    def _add_itemprops_override(self, content_types_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/content-types"
        root = ET.fromstring(content_types_bytes)
        override_tag = f"{{{ns}}}Override"
        if not any(
            elem.get("PartName") == "/customXml/itemProps2.xml"
            for elem in root.findall(override_tag)
        ):
            new_override = ET.SubElement(root, override_tag)
            new_override.set("PartName", "/customXml/itemProps2.xml")
            new_override.set(
                "ContentType",
                "application/vnd.openxmlformats-officedocument.customXmlProperties+xml",
            )
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _add_workbook_relationship(self, rels_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/relationships"
        root = ET.fromstring(rels_bytes)
        rel_tag = f"{{{ns}}}Relationship"
        existing_ids = {elem.get("Id") for elem in root.findall(rel_tag)}
        next_id = 1
        while f"rId{next_id}" in existing_ids:
            next_id += 1
        new_rel = ET.SubElement(root, rel_tag)
        new_rel.set("Id", f"rId{next_id}")
        new_rel.set(
            "Type",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
        )
        new_rel.set("Target", "../customXml/item2.xml")
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _duplicate_datamashup_element(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            root = etree.fromstring(data)
                            dm_node = self._get_mashup_element(root)
                            if dm_node is not None:
                                duplicate = copy.deepcopy(dm_node)
                                parent = dm_node.getparent()
                                if parent is not None:
                                    parent.append(duplicate)
                                    target_root = root
                                else:
                                    container = etree.Element("root", nsmap=root.nsmap)
                                    container.append(dm_node)
                                    container.append(duplicate)
                                    target_root = container
                                data = etree.tostring(
                                    target_root, encoding="utf-8", xml_declaration=True
                                )
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)


class MashupEncodeGenerator(MashupBaseGenerator):
    """
    Re-encodes the DataMashup customXml stream to a target encoding and optionally
    inserts whitespace into the base64 payload.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        encoding = self.args.get('encoding', 'utf-8')
        whitespace = bool(self.args.get('whitespace', False))
        if not base_file_arg:
            raise ValueError("MashupEncodeGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._rewrite_datamashup_xml(base.resolve(), target_path, encoding, whitespace)

    def _rewrite_datamashup_xml(
        self,
        base_path: Path,
        output_path: Path,
        encoding: str,
        whitespace: bool,
    ):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            data = self._process_datamashup_stream(data, encoding, whitespace)
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)

    def _process_datamashup_stream(
        self,
        xml_bytes: bytes,
        encoding: str,
        whitespace: bool,
    ) -> bytes:
        root = etree.fromstring(xml_bytes)
        dm_node = self._get_mashup_element(root)
        if dm_node is None:
            return xml_bytes

        if dm_node.text and whitespace:
            dm_node.text = self._with_whitespace(dm_node.text)

        xml_bytes = etree.tostring(root, encoding="utf-8", xml_declaration=True)
        return self._encode_bytes(xml_bytes, encoding)

    def _with_whitespace(self, text: str) -> str:
        cleaned = text.strip()
        if not cleaned:
            return text
        midpoint = max(1, len(cleaned) // 2)
        return f"\n  {cleaned[:midpoint]}\n  {cleaned[midpoint:]}\n"

    def _encode_bytes(self, xml_bytes: bytes, encoding: str) -> bytes:
        enc = encoding.lower()
        if enc == "utf-8":
            return xml_bytes
        if enc == "utf-16-le":
            return self._to_utf16(xml_bytes, little_endian=True)
        if enc == "utf-16-be":
            return self._to_utf16(xml_bytes, little_endian=False)
        raise ValueError(f"Unsupported encoding: {encoding}")

    def _to_utf16(self, xml_bytes: bytes, little_endian: bool) -> bytes:
        text = xml_bytes.decode("utf-8")
        text = self._rewrite_declaration(text)
        encoded = text.encode("utf-16-le" if little_endian else "utf-16-be")
        bom = b"\xff\xfe" if little_endian else b"\xfe\xff"
        return bom + encoded

    def _rewrite_declaration(self, text: str) -> str:
        pattern = r'encoding=["\'][^"\']+["\']'
        if re.search(pattern, text):
            return re.sub(pattern, 'encoding="UTF-16"', text, count=1)
        prefix = "<?xml version='1.0'?>"
        if text.startswith(prefix):
            return text.replace(prefix, "<?xml version='1.0' encoding='UTF-16'?>", 1)
        return text


class MashupPermissionsMetadataGenerator(MashupBaseGenerator):
    """
    Builds fixtures that exercise Permissions and Metadata parsing by rewriting
    the PackageParts Section1.m, Permissions XML, and Metadata XML inside
    the DataMashup stream.
    """

    def __init__(self, args):
        super().__init__(args)
        self.mode = args.get("mode")

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if not self.mode:
            raise ValueError("MashupPermissionsMetadataGenerator requires 'mode' argument")

        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        version, package_parts, _, _, bindings = self._split_sections(raw_bytes)
        scenario = self._scenario_definition()

        updated_package_parts = self._replace_section(
            package_parts,
            scenario["section_text"],
        )
        permissions_bytes = self._permissions_bytes(**scenario["permissions"])
        metadata_bytes = self._metadata_bytes(scenario["metadata_entries"])

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions_bytes,
            metadata_bytes,
            bindings,
        )

    def _scenario_definition(self):
        shared_section_simple = "\n".join(
            [
                "section Section1;",
                "",
                "shared LoadToSheet = 1;",
                "shared LoadToModel = 2;",
            ]
        )

        if self.mode in ("permissions_defaults", "permissions_firewall_off", "metadata_simple"):
            return {
                "section_text": shared_section_simple,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": self.mode != "permissions_firewall_off",
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/LoadToSheet",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/LoadToModel",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", True),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_query_groups":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared RootQuery = 1;",
                    "shared GroupedFoo = 2;",
                    "shared NestedBar = 3;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/RootQuery",
                        "entries": [("FillEnabled", True)],
                    },
                    {
                        "path": "Section1/GroupedFoo",
                        "entries": [
                            ("FillEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                    {
                        "path": "Section1/NestedBar",
                        "entries": [
                            ("FillToDataModelEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_hidden_queries":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared ConnectionOnly = 1;",
                    "shared VisibleLoad = 2;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/ConnectionOnly",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/VisibleLoad",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                ],
            }

        raise ValueError(f"Unsupported mode: {self.mode}")

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts = raw_bytes[offset : offset + package_parts_len]
        offset += package_parts_len

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions = raw_bytes[offset : offset + permissions_len]
        offset += permissions_len

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata = raw_bytes[offset : offset + metadata_len]
        offset += metadata_len

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings = raw_bytes[offset : offset + bindings_len]

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _replace_section(self, package_parts: bytes, section_text: str) -> bytes:
        return self._replace_in_zip(package_parts, "Formulas/Section1.m", section_text)

    def _replace_in_zip(self, zip_bytes: bytes, filename: str, new_content: str) -> bytes:
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()

        with zipfile.ZipFile(in_buffer, "r") as zin:
            with zipfile.ZipFile(out_buffer, "w", compression=zipfile.ZIP_DEFLATED) as zout:
                for item in zin.infolist():
                    if item.filename == filename:
                        zout.writestr(filename, new_content.encode("utf-8"))
                    else:
                        zout.writestr(item, zin.read(item.filename))
        return out_buffer.getvalue()

    def _permissions_bytes(self, can_eval: bool, firewall_enabled: bool, group_type: str) -> bytes:
        xml = (
            '<?xml version="1.0" encoding="utf-8"?>'
            "<PermissionList xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" "
            "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
            f"<CanEvaluateFuturePackages>{str(can_eval).lower()}</CanEvaluateFuturePackages>"
            f"<FirewallEnabled>{str(firewall_enabled).lower()}</FirewallEnabled>"
            f"<WorkbookGroupType>{group_type}</WorkbookGroupType>"
            "</PermissionList>"
        )
        return ("\ufeff" + xml).encode("utf-8")

    def _metadata_bytes(self, items: List[dict]) -> bytes:
        xml = self._metadata_xml(items)
        xml_bytes = ("\ufeff" + xml).encode("utf-8")
        header = struct.pack("<I", 0) + struct.pack("<I", len(xml_bytes))
        return header + xml_bytes

    def _metadata_xml(self, items: List[dict]) -> str:
        parts = [
            '<?xml version="1.0" encoding="utf-8"?>',
            '<LocalPackageMetadataFile xmlns:xsd="http://www.w3.org/2001/XMLSchema" '
            'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">',
            "<Items>",
            "<Item><ItemLocation><ItemType>AllFormulas</ItemType><ItemPath /></ItemLocation><StableEntries /></Item>",
        ]

        for item in items:
            parts.append("<Item>")
            parts.append("<ItemLocation>")
            parts.append("<ItemType>Formula</ItemType>")
            parts.append(f"<ItemPath>{item['path']}</ItemPath>")
            parts.append("</ItemLocation>")
            parts.append("<StableEntries>")
            for entry_name, entry_value in item.get("entries", []):
                value = self._format_entry_value(entry_value)
                parts.append(f'<Entry Type="{entry_name}" Value="{value}" />')
            parts.append("</StableEntries>")
            parts.append("</Item>")

        parts.append("</Items></LocalPackageMetadataFile>")
        return "".join(parts)

    def _format_entry_value(self, value):
        if isinstance(value, bool):
            return f"l{'1' if value else '0'}"
        return f"s{value}"

```

---

### File: `fixtures\src\generators\perf.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class LargeGridGenerator(BaseGenerator):
    """
    Generates massive grids using WriteOnly mode to save memory.
    Targeting P1/P2 milestones.
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        rows = self.args.get('rows', 1000)
        cols = self.args.get('cols', 10)
        mode = self.args.get('mode', 'dense')
        seed = self.args.get('seed', 0)

        # Use deterministic seed if provided, otherwise system time
        rng = random.Random(seed)

        for name in output_names:
            # WriteOnly mode is critical for 50k+ rows in Python
            wb = openpyxl.Workbook(write_only=True)
            ws = wb.create_sheet()
            ws.title = "Performance"

            # 1. Header
            header = [f"Col_{c}" for c in range(1, cols + 1)]
            ws.append(header)

            # 2. Data Stream
            for r in range(1, rows + 1):
                row_data = []
                if mode == 'dense':
                    # Deterministic pattern: "R{r}C{c}"
                    # Fast to generate, high compression ratio
                    row_data = [f"R{r}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'noise':
                    # Random floats: Harder to align, harder to compress
                    row_data = [rng.random() for _ in range(cols)]
                
                ws.append(row_data)

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\__init__.py`

```python
# Generators package

```

---

