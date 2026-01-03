use crate::string_pool::StringId;

/// Minimal tabular model IR for future DAX/model diffing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Model {
    pub measures: Vec<Measure>,
    pub tables: Vec<ModelTable>,
    pub relationships: Vec<ModelRelationship>,
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
    pub is_hidden: Option<bool>,
    pub format_string: Option<StringId>,
    pub sort_by: Option<StringId>,
    pub summarize_by: Option<StringId>,
    pub expression: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRelationship {
    pub from_table: StringId,
    pub from_column: StringId,
    pub to_table: StringId,
    pub to_column: StringId,
    pub cross_filtering_behavior: Option<StringId>,
    pub cardinality: Option<StringId>,
    pub is_active: Option<bool>,
    pub name: Option<StringId>,
}
