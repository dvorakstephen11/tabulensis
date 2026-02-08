use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use excel_diff::SemanticNoisePolicy;
use serde::{Deserialize, Serialize};
use ui_payload::{DiffLimits, DiffPreset, NoiseFilters};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareProfile {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub built_in: bool,

    pub preset: DiffPreset,
    #[serde(default)]
    pub trusted_files: bool,

    #[serde(default)]
    pub noise_filters: NoiseFilters,

    // Engine-level semantic toggles and noise policy. These are persisted so that built-in and
    // imported profiles can drive engine behavior beyond the preset.
    #[serde(default = "default_enable_m_semantic_diff")]
    pub enable_m_semantic_diff: bool,
    #[serde(default)]
    pub enable_formula_semantic_diff: bool,
    #[serde(default)]
    pub enable_dax_semantic_diff: bool,
    #[serde(default = "default_semantic_noise_policy")]
    pub semantic_noise_policy: SemanticNoisePolicy,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<DiffLimits>,
}

fn default_enable_m_semantic_diff() -> bool {
    true
}

fn default_semantic_noise_policy() -> SemanticNoisePolicy {
    SemanticNoisePolicy::ReportFormattingOnly
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompareProfilesFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    profiles: Vec<CompareProfile>,
}

pub fn builtin_profiles() -> Vec<CompareProfile> {
    vec![
        CompareProfile {
            id: "builtin_default_balanced".to_string(),
            name: "Default (Balanced)".to_string(),
            built_in: true,
            preset: DiffPreset::Balanced,
            trusted_files: false,
            noise_filters: NoiseFilters::default(),
            enable_m_semantic_diff: true,
            enable_formula_semantic_diff: false,
            enable_dax_semantic_diff: false,
            semantic_noise_policy: SemanticNoisePolicy::ReportFormattingOnly,
            limits: None,
        },
        CompareProfile {
            id: "builtin_finance_model_review".to_string(),
            name: "Finance model review".to_string(),
            built_in: true,
            preset: DiffPreset::MostPrecise,
            trusted_files: false,
            noise_filters: NoiseFilters {
                hide_m_formatting_only: true,
                hide_dax_formatting_only: true,
                hide_formula_formatting_only: true,
                collapse_moves: true,
            },
            enable_m_semantic_diff: true,
            enable_formula_semantic_diff: true,
            enable_dax_semantic_diff: true,
            semantic_noise_policy: SemanticNoisePolicy::SuppressFormattingOnly,
            limits: None,
        },
        CompareProfile {
            id: "builtin_data_pipeline_workbook".to_string(),
            name: "Data pipeline workbook".to_string(),
            built_in: true,
            preset: DiffPreset::Balanced,
            trusted_files: false,
            noise_filters: NoiseFilters {
                hide_m_formatting_only: true,
                hide_dax_formatting_only: false,
                hide_formula_formatting_only: false,
                collapse_moves: true,
            },
            enable_m_semantic_diff: true,
            enable_formula_semantic_diff: false,
            enable_dax_semantic_diff: false,
            semantic_noise_policy: SemanticNoisePolicy::SuppressFormattingOnly,
            limits: Some(DiffLimits {
                max_memory_mb: Some(512),
                timeout_ms: Some(60_000),
                max_ops: Some(200_000),
                on_limit_exceeded: Some(excel_diff::LimitBehavior::ReturnPartialResult),
            }),
        },
        CompareProfile {
            id: "builtin_power_bi_model_review".to_string(),
            name: "Power BI model review".to_string(),
            built_in: true,
            preset: DiffPreset::MostPrecise,
            trusted_files: false,
            noise_filters: NoiseFilters {
                hide_m_formatting_only: true,
                hide_dax_formatting_only: true,
                hide_formula_formatting_only: false,
                collapse_moves: false,
            },
            enable_m_semantic_diff: true,
            enable_formula_semantic_diff: false,
            enable_dax_semantic_diff: true,
            semantic_noise_policy: SemanticNoisePolicy::SuppressFormattingOnly,
            limits: None,
        },
    ]
}

pub fn load_user_profiles(path: &Path) -> Vec<CompareProfile> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let parsed: CompareProfilesFile = serde_json::from_str(&contents).unwrap_or_default();
    parsed
        .profiles
        .into_iter()
        .filter(|p| !p.built_in)
        .collect()
}

pub fn save_user_profiles(path: &Path, profiles: &[CompareProfile]) {
    let payload = CompareProfilesFile {
        version: 1,
        profiles: profiles.iter().cloned().filter(|p| !p.built_in).collect(),
    };
    let Ok(json) = serde_json::to_string_pretty(&payload) else {
        return;
    };
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
    else {
        return;
    };
    let _ = file.write_all(json.as_bytes());
}

pub fn make_profile_id(name: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut slug = String::new();
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if c == ' ' || c == '_' || c == '-' {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
        if slug.len() >= 40 {
            break;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        slug = "profile".to_string();
    }
    format!("user_{now}_{slug}")
}
