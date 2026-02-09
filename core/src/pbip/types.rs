use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbipDocType {
    Pbir,
    Tmdl,
    Other,
}

impl PbipDocType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pbir => "pbir",
            Self::Tmdl => "tmdl",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbipChangeKind {
    Added,
    Removed,
    Modified,
    Unchanged,
}

impl PbipChangeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
            Self::Unchanged => "unchanged",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PbipNormalizationProfile {
    Strict,
    Balanced,
    Aggressive,
}

impl Default for PbipNormalizationProfile {
    fn default() -> Self {
        Self::Balanced
    }
}

impl PbipNormalizationProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PbipDocSnapshot {
    pub doc_type: PbipDocType,
    pub normalized_text: String,
    pub hash: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalization_applied: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PbipProjectSnapshot {
    /// Relative-path keyed snapshot of normalized documents.
    pub docs: Vec<PbipDocRecord>,
    pub profile: PbipNormalizationProfile,
    pub profile_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PbipDocRecord {
    pub path: String,
    pub doc_type: PbipDocType,
    pub snapshot: PbipDocSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PbipDocDiff {
    pub path: String,
    pub doc_type: PbipDocType,
    pub change_kind: PbipChangeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<PbipDocSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<PbipDocSnapshot>,
}
