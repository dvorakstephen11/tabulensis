use serde_json::Value;

use crate::excel_open_xml::PackageError;
use crate::model::{Measure, Model, ModelColumn, ModelRelationship, ModelTable};
use crate::string_pool::StringPool;

#[derive(Debug, Clone, Default)]
pub(crate) struct RawTabularModel {
    pub tables: Vec<RawTable>,
    pub relationships: Vec<RawRelationship>,
    pub measures: Vec<RawMeasure>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawTable {
    pub name: String,
    pub columns: Vec<RawColumn>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawColumn {
    pub name: String,
    pub data_type: Option<String>,
    pub is_hidden: Option<bool>,
    pub format_string: Option<String>,
    pub sort_by: Option<String>,
    pub summarize_by: Option<String>,
    pub expression: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawRelationship {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    pub cross_filtering_behavior: Option<String>,
    pub cardinality: Option<String>,
    pub is_active: Option<bool>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawMeasure {
    pub full_name: String,
    pub expression: String,
}

fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{FEFF}').unwrap_or(s)
}

pub(crate) fn parse_data_model_schema(bytes: &[u8]) -> Result<RawTabularModel, PackageError> {
    let text = std::str::from_utf8(bytes).map_err(|e| PackageError::UnsupportedFormat {
        message: format!("DataModelSchema is not UTF-8: {}", e),
    })?;
    let text = strip_bom(text);

    let v: Value = serde_json::from_str(text).map_err(|e| PackageError::UnsupportedFormat {
        message: format!("DataModelSchema JSON parse error: {}", e),
    })?;

    let mut out = RawTabularModel::default();

    if try_collect_from_model_tables(&v, &mut out) {
        normalize(&mut out);
        return Ok(out);
    }

    collect_measures_anywhere(&v, "", &mut out);
    normalize(&mut out);
    Ok(out)
}

fn try_collect_from_model_tables(v: &Value, out: &mut RawTabularModel) -> bool {
    let model = match v.get("model") {
        Some(m) => m,
        None => return false,
    };
    let tables = match model.get("tables").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return false,
    };

    for t in tables {
        let table_name = t.get("name").and_then(|x| x.as_str()).unwrap_or("");
        let mut raw_table = RawTable {
            name: table_name.to_string(),
            columns: Vec::new(),
        };

        if let Some(columns) = t.get("columns").and_then(|c| c.as_array()) {
            for c in columns {
                if let Some(col) = parse_column_obj(c) {
                    raw_table.columns.push(col);
                }
            }
        }

        if !raw_table.name.is_empty() {
            out.tables.push(raw_table);
        }

        if let Some(measures) = t.get("measures").and_then(|m| m.as_array()) {
            for m in measures {
                if let Some(rm) = parse_measure_obj(m, table_name) {
                    out.measures.push(rm);
                }
            }
        }
    }

    if let Some(relationships) = model.get("relationships").and_then(|r| r.as_array()) {
        for rel in relationships {
            if let Some(raw_rel) = parse_relationship_obj(rel) {
                out.relationships.push(raw_rel);
            }
        }
    }
    true
}

fn parse_measure_obj(v: &Value, table_name: &str) -> Option<RawMeasure> {
    let name = v.get("name").and_then(|x| x.as_str())?;
    let expr = v.get("expression").and_then(|x| x.as_str()).unwrap_or("");

    let full_name = if table_name.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", table_name, name)
    };

    Some(RawMeasure {
        full_name,
        expression: expr.to_string(),
    })
}

fn parse_column_obj(v: &Value) -> Option<RawColumn> {
    let name = v.get("name").and_then(|x| x.as_str())?;
    let data_type = opt_string_field(v, "dataType");
    let is_hidden = v.get("isHidden").and_then(|x| x.as_bool());
    let format_string = opt_string_field(v, "formatString");
    let sort_by = opt_string_field(v, "sortByColumn");
    let summarize_by = opt_string_field(v, "summarizeBy");
    let expression = opt_string_field(v, "expression");

    Some(RawColumn {
        name: name.to_string(),
        data_type,
        is_hidden,
        format_string,
        sort_by,
        summarize_by,
        expression,
    })
}

fn parse_relationship_obj(v: &Value) -> Option<RawRelationship> {
    let from_table = v.get("fromTable").and_then(|x| x.as_str())?;
    let from_column = v.get("fromColumn").and_then(|x| x.as_str())?;
    let to_table = v.get("toTable").and_then(|x| x.as_str())?;
    let to_column = v.get("toColumn").and_then(|x| x.as_str())?;

    Some(RawRelationship {
        from_table: from_table.to_string(),
        from_column: from_column.to_string(),
        to_table: to_table.to_string(),
        to_column: to_column.to_string(),
        cross_filtering_behavior: opt_string_field(v, "crossFilteringBehavior"),
        cardinality: opt_string_field(v, "cardinality"),
        is_active: v.get("isActive").and_then(|x| x.as_bool()),
        name: opt_string_field(v, "name"),
    })
}

fn collect_measures_anywhere(v: &Value, table_name: &str, out: &mut RawTabularModel) {
    match v {
        Value::Object(map) => {
            if let Some(measures) = map.get("measures").and_then(|m| m.as_array()) {
                for m in measures {
                    if let Some(rm) = parse_measure_obj(m, table_name) {
                        out.measures.push(rm);
                    }
                }
            }

            let next_table = map.get("name").and_then(|x| x.as_str()).unwrap_or(table_name);

            for (_k, child) in map {
                collect_measures_anywhere(child, next_table, out);
            }
        }
        Value::Array(arr) => {
            for child in arr {
                collect_measures_anywhere(child, table_name, out);
            }
        }
        _ => {}
    }
}

fn normalize(out: &mut RawTabularModel) {
    out.measures
        .sort_by(|a, b| cmp_case_insensitive(&a.full_name, &b.full_name));
    out.measures
        .dedup_by(|a, b| a.full_name == b.full_name && a.expression == b.expression);

    out.tables.sort_by(|a, b| cmp_case_insensitive(&a.name, &b.name));
    for table in &mut out.tables {
        table
            .columns
            .sort_by(|a, b| cmp_case_insensitive(&a.name, &b.name));
    }

    out.relationships.sort_by(|a, b| cmp_relationship_key(a, b));
}

pub(crate) fn build_model(raw: &RawTabularModel, pool: &mut StringPool) -> Model {
    let mut m = Model::default();

    for rt in &raw.tables {
        let name = pool.intern(&rt.name);
        let mut columns = Vec::with_capacity(rt.columns.len());
        for rc in &rt.columns {
            let col_name = pool.intern(&rc.name);
            let data_type = rc.data_type.as_deref().map(|s| pool.intern(s));
            let format_string = rc.format_string.as_deref().map(|s| pool.intern(s));
            let sort_by = rc.sort_by.as_deref().map(|s| pool.intern(s));
            let summarize_by = rc.summarize_by.as_deref().map(|s| pool.intern(s));
            let expression = rc.expression.as_deref().map(|s| pool.intern(s));
            columns.push(ModelColumn {
                name: col_name,
                data_type,
                is_hidden: rc.is_hidden,
                format_string,
                sort_by,
                summarize_by,
                expression,
            });
        }
        m.tables.push(ModelTable { name, columns });
    }

    for rr in &raw.relationships {
        let from_table = pool.intern(&rr.from_table);
        let from_column = pool.intern(&rr.from_column);
        let to_table = pool.intern(&rr.to_table);
        let to_column = pool.intern(&rr.to_column);
        let cross_filtering_behavior = rr
            .cross_filtering_behavior
            .as_deref()
            .map(|s| pool.intern(s));
        let cardinality = rr.cardinality.as_deref().map(|s| pool.intern(s));
        let name = rr.name.as_deref().map(|s| pool.intern(s));
        m.relationships.push(ModelRelationship {
            from_table,
            from_column,
            to_table,
            to_column,
            cross_filtering_behavior,
            cardinality,
            is_active: rr.is_active,
            name,
        });
    }

    for rm in &raw.measures {
        let name = pool.intern(&rm.full_name);
        let expr = pool.intern(&rm.expression);
        m.measures.push(Measure {
            name,
            expression: expr,
        });
    }
    m
}

fn opt_string_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn cmp_case_insensitive(a: &str, b: &str) -> std::cmp::Ordering {
    let al = a.to_lowercase();
    let bl = b.to_lowercase();
    let cmp = al.cmp(&bl);
    if cmp == std::cmp::Ordering::Equal {
        a.cmp(b)
    } else {
        cmp
    }
}

fn cmp_relationship_key(a: &RawRelationship, b: &RawRelationship) -> std::cmp::Ordering {
    let fields_a = [
        a.from_table.as_str(),
        a.from_column.as_str(),
        a.to_table.as_str(),
        a.to_column.as_str(),
    ];
    let fields_b = [
        b.from_table.as_str(),
        b.from_column.as_str(),
        b.to_table.as_str(),
        b.to_column.as_str(),
    ];

    for (av, bv) in fields_a.iter().zip(fields_b.iter()) {
        let cmp = cmp_case_insensitive(av, bv);
        if cmp != std::cmp::Ordering::Equal {
            return cmp;
        }
    }

    let extra_a = [
        a.cross_filtering_behavior.as_deref(),
        a.cardinality.as_deref(),
        a.name.as_deref(),
    ];
    let extra_b = [
        b.cross_filtering_behavior.as_deref(),
        b.cardinality.as_deref(),
        b.name.as_deref(),
    ];

    for (av, bv) in extra_a.iter().zip(extra_b.iter()) {
        let cmp = cmp_case_insensitive(av.unwrap_or(""), bv.unwrap_or(""));
        if cmp != std::cmp::Ordering::Equal {
            return cmp;
        }
    }

    a.is_active.cmp(&b.is_active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tables_columns_relationships() {
        let json = r#"{
            "model": {
                "tables": [
                    {
                        "name": "Sales",
                        "columns": [
                            {
                                "name": "Amount",
                                "dataType": "decimal",
                                "isHidden": true,
                                "formatString": "0.00",
                                "sortByColumn": "SortCol",
                                "summarizeBy": "sum",
                                "expression": "[Amount] * 2"
                            }
                        ],
                        "measures": [
                            {
                                "name": "Total",
                                "expression": "SUM(Sales[Amount])"
                            }
                        ]
                    }
                ],
                "relationships": [
                    {
                        "fromTable": "Sales",
                        "fromColumn": "CustomerId",
                        "toTable": "Customers",
                        "toColumn": "Id",
                        "crossFilteringBehavior": "oneDirection",
                        "cardinality": "ManyToOne",
                        "isActive": true,
                        "name": "SalesCustomers"
                    }
                ]
            }
        }"#;

        let raw = parse_data_model_schema(json.as_bytes()).expect("parse schema");
        assert_eq!(raw.tables.len(), 1);
        assert_eq!(raw.measures.len(), 1);
        assert_eq!(raw.relationships.len(), 1);

        let table = &raw.tables[0];
        assert_eq!(table.name, "Sales");
        assert_eq!(table.columns.len(), 1);
        let col = &table.columns[0];
        assert_eq!(col.name, "Amount");
        assert_eq!(col.data_type.as_deref(), Some("decimal"));
        assert_eq!(col.is_hidden, Some(true));
        assert_eq!(col.format_string.as_deref(), Some("0.00"));
        assert_eq!(col.sort_by.as_deref(), Some("SortCol"));
        assert_eq!(col.summarize_by.as_deref(), Some("sum"));
        assert_eq!(col.expression.as_deref(), Some("[Amount] * 2"));

        let measure = &raw.measures[0];
        assert_eq!(measure.full_name, "Sales/Total");
        assert_eq!(measure.expression, "SUM(Sales[Amount])");

        let rel = &raw.relationships[0];
        assert_eq!(rel.from_table, "Sales");
        assert_eq!(rel.from_column, "CustomerId");
        assert_eq!(rel.to_table, "Customers");
        assert_eq!(rel.to_column, "Id");
        assert_eq!(rel.cross_filtering_behavior.as_deref(), Some("oneDirection"));
        assert_eq!(rel.cardinality.as_deref(), Some("ManyToOne"));
        assert_eq!(rel.is_active, Some(true));
        assert_eq!(rel.name.as_deref(), Some("SalesCustomers"));

        let mut pool = StringPool::new();
        let model = build_model(&raw, &mut pool);
        assert_eq!(model.tables.len(), 1);
        assert_eq!(pool.resolve(model.tables[0].name), "Sales");
        assert_eq!(model.tables[0].columns.len(), 1);
        assert_eq!(
            pool.resolve(model.tables[0].columns[0].name),
            "Amount"
        );
        assert_eq!(model.relationships.len(), 1);
        assert_eq!(pool.resolve(model.relationships[0].from_table), "Sales");
        assert_eq!(model.measures.len(), 1);
        assert_eq!(pool.resolve(model.measures[0].name), "Sales/Total");
    }
}
