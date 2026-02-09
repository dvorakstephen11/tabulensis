use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffDomain {
    ExcelWorkbook,
    PbipProject,
}

impl DiffDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExcelWorkbook => "excel_workbook",
            Self::PbipProject => "pbip_project",
        }
    }
}

impl Default for DiffDomain {
    fn default() -> Self {
        Self::ExcelWorkbook
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionKind {
    Sheet,
    Cell,
    Range,
    Document,
    Entity,
    Location,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionTarget {
    pub domain: DiffDomain,
    pub kind: SelectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorRow {
    pub target: SelectionTarget,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorModel {
    pub columns: Vec<String>,
    pub rows: Vec<NavigatorRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailsPayload {
    pub target: SelectionTarget,
    pub language: String,
    pub header: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalization_applied: Option<String>,
}
