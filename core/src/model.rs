use crate::string_pool::StringId;

/// Minimal tabular model IR for future DAX/model diffing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Model {
    pub measures: Vec<Measure>,
    pub tables: Vec<ModelTable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measure {
    pub name: StringId,
    pub expression: StringId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTable {
    pub name: StringId,
    pub columns: Vec<ModelColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelColumn {
    pub name: StringId,
    pub data_type: Option<StringId>,
}
