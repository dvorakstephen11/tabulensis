use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{PbipDocType, PbipNormalizationProfile};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizationApplied {
    pub profile: PbipNormalizationProfile,
    pub doc_type: PbipDocType,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct NormalizationError {
    pub message: String,
}

impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NormalizationError {}

pub fn normalize_doc_text(
    doc_type: PbipDocType,
    raw: &str,
    profile: PbipNormalizationProfile,
) -> Result<(String, NormalizationApplied), NormalizationError> {
    match doc_type {
        PbipDocType::Pbir => normalize_pbir(raw, profile),
        PbipDocType::Tmdl => normalize_tmdl(raw, profile),
        PbipDocType::Other => Ok((
            normalize_text_minimal(raw),
            NormalizationApplied {
                profile,
                doc_type,
                summary: format!("Other: text normalization only (profile={})", profile.as_str()),
            },
        )),
    }
}

fn normalize_pbir(raw: &str, profile: PbipNormalizationProfile) -> Result<(String, NormalizationApplied), NormalizationError> {
    if profile == PbipNormalizationProfile::Strict {
        return Ok((
            normalize_text_minimal(raw),
            NormalizationApplied {
                profile,
                doc_type: PbipDocType::Pbir,
                summary: "PBIR strict: text normalization only (no JSON parsing)".to_string(),
            },
        ));
    }

    let parsed: Value = serde_json::from_str(raw).map_err(|e| NormalizationError {
        message: format!("PBIR JSON parse error: {e}"),
    })?;

    let mut guid_map: HashMap<String, String> = HashMap::new();
    let canonical = canonicalize_json_value(&parsed, None, profile, &mut guid_map);
    let mut out = serde_json::to_string_pretty(&canonical).map_err(|e| NormalizationError {
        message: format!("PBIR JSON serialize error: {e}"),
    })?;
    if !out.ends_with('\n') {
        out.push('\n');
    }

    let guid_mode = match profile {
        PbipNormalizationProfile::Balanced => "id-like keys only",
        PbipNormalizationProfile::Aggressive => "all GUID-like strings",
        PbipNormalizationProfile::Strict => "off",
    };

    Ok((
        out,
        NormalizationApplied {
            profile,
            doc_type: PbipDocType::Pbir,
            summary: format!("PBIR: canonical JSON (sorted keys), GUID normalization: {guid_mode}"),
        },
    ))
}

fn normalize_tmdl(raw: &str, profile: PbipNormalizationProfile) -> Result<(String, NormalizationApplied), NormalizationError> {
    // For MVP we keep TMDL normalization conservative and purely lexical.
    // (Future: parse blocks and normalize ordering where safe.)
    let normalized = normalize_text_minimal(raw);
    Ok((
        normalized,
        NormalizationApplied {
            profile,
            doc_type: PbipDocType::Tmdl,
            summary: format!(
                "TMDL: lexical normalization (line endings + trailing whitespace), profile={}",
                profile.as_str()
            ),
        },
    ))
}

fn normalize_text_minimal(raw: &str) -> String {
    // Normalize line endings and trim trailing whitespace, keeping content otherwise intact.
    let raw = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(raw.len() + 1);
    for (idx, line) in raw.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        // Strip trailing whitespace but preserve leading indentation.
        out.push_str(line.trim_end_matches(|c: char| c == ' ' || c == '\t'));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn canonicalize_json_value(
    value: &Value,
    key_hint: Option<&str>,
    profile: PbipNormalizationProfile,
    guid_map: &mut HashMap<String, String>,
) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(v) => Value::Bool(*v),
        Value::Number(v) => Value::Number(v.clone()),
        Value::String(s) => {
            if should_normalize_guid(key_hint, profile) && looks_like_guid(s) {
                let next_idx = guid_map.len() + 1;
                let placeholder = guid_map
                    .entry(s.clone())
                    .or_insert_with(|| format!("GUID_{next_idx:04}"))
                    .clone();
                Value::String(placeholder)
            } else {
                Value::String(s.clone())
            }
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| canonicalize_json_value(v, key_hint, profile, guid_map))
                .collect(),
        ),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::with_capacity(map.len());
            for key in keys {
                let child = map.get(key).unwrap_or(&Value::Null);
                out.insert(
                    key.clone(),
                    canonicalize_json_value(child, Some(key.as_str()), profile, guid_map),
                );
            }
            Value::Object(out)
        }
    }
}

fn should_normalize_guid(key_hint: Option<&str>, profile: PbipNormalizationProfile) -> bool {
    match profile {
        PbipNormalizationProfile::Strict => false,
        PbipNormalizationProfile::Aggressive => true,
        PbipNormalizationProfile::Balanced => key_hint
            .map(|k| {
                let k = k.trim();
                k.eq_ignore_ascii_case("id")
                    || k.eq_ignore_ascii_case("guid")
                    || k.eq_ignore_ascii_case("objectId")
                    || k.ends_with("Id")
                    || k.ends_with("ID")
                    || k.to_ascii_lowercase().contains("guid")
            })
            .unwrap_or(false),
    }
}

fn looks_like_guid(value: &str) -> bool {
    // Conservative GUID detector: 36 chars like 8-4-4-4-12 hex.
    // We intentionally avoid regex to keep core deps minimal and to remain deterministic.
    let b = value.as_bytes();
    if b.len() != 36 {
        return false;
    }
    for (idx, &c) in b.iter().enumerate() {
        let is_dash = matches!(idx, 8 | 13 | 18 | 23);
        if is_dash {
            if c != b'-' {
                return false;
            }
            continue;
        }
        let is_hex = matches!(c, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F');
        if !is_hex {
            return false;
        }
    }
    true
}
