use serde_json::Value;

use crate::excel_open_xml::PackageError;
use crate::model::{Measure, Model};
use crate::string_pool::StringPool;

#[derive(Debug, Clone, Default)]
pub(crate) struct RawTabularModel {
    pub measures: Vec<RawMeasure>,
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
        if let Some(measures) = t.get("measures").and_then(|m| m.as_array()) {
            for m in measures {
                if let Some(rm) = parse_measure_obj(m, table_name) {
                    out.measures.push(rm);
                }
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
        .sort_by(|a, b| a.full_name.cmp(&b.full_name));
    out.measures
        .dedup_by(|a, b| a.full_name == b.full_name && a.expression == b.expression);
}

pub(crate) fn build_model(raw: &RawTabularModel, pool: &mut StringPool) -> Model {
    let mut m = Model::default();
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
