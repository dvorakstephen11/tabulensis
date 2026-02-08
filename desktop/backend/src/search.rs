use std::path::Path;

use excel_diff::{CellValue, WorkbookPackage};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use uuid::Uuid;

use crate::diff_runner::DiffErrorPayload;
use crate::store::{OpStore, StoreError};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub kind: String,
    pub sheet: Option<String>,
    pub address: Option<String>,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexResult {
    pub sheet: String,
    pub address: String,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexSummary {
    pub index_id: String,
    pub path: String,
    pub side: String,
    pub created_at: String,
}

pub fn search_diff_ops(
    store_path: &Path,
    diff_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, DiffErrorPayload> {
    let store = OpStore::open(store_path).map_err(store_error)?;
    let strings = store.load_strings(diff_id).map_err(store_error)?;
    let conn = store.into_connection();

    let pattern = format!("%{}%", query.to_lowercase());
    let mut stmt = conn
        .prepare(
            "SELECT payload_json FROM diff_ops WHERE diff_id = ?1 AND lower(payload_json) LIKE ?2 LIMIT ?3",
        )
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    let rows = stmt
        .query_map(params![diff_id, pattern, limit as i64], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

    let mut results = Vec::new();
    for row in rows {
        let payload = row.map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
        let op: excel_diff::DiffOp = serde_json::from_str(&payload)
            .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
        if let Some(result) = match_op(&op, &strings, query) {
            results.push(result);
        }
    }

    Ok(results)
}

pub fn build_search_index(
    store_path: &Path,
    path: &Path,
    side: &str,
) -> Result<SearchIndexSummary, DiffErrorPayload> {
    let store = OpStore::open(store_path).map_err(store_error)?;
    let conn = store.connection();
    let path_str = path.display().to_string();

    let meta =
        std::fs::metadata(path).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;
    let size = meta.len() as i64;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    if let Some(existing) = find_existing_index(conn, &path_str, mtime, size, side)? {
        return Ok(existing);
    }

    let index_id = Uuid::new_v4().to_string();
    let created_at = now_iso();

    conn.execute(
        "INSERT INTO workbook_indexes (index_id, path, mtime, size, side, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![index_id, path_str, mtime, size, side, created_at],
    )
    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

    conn.execute(
        "DELETE FROM cell_docs WHERE index_id NOT IN (SELECT index_id FROM workbook_indexes)",
        [],
    )
    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

    index_workbook(conn, path, &index_id)
        .map_err(|e| DiffErrorPayload::new("index", e.to_string(), false))?;

    Ok(SearchIndexSummary {
        index_id,
        path: path.display().to_string(),
        side: side.to_string(),
        created_at: now_iso(),
    })
}

pub fn search_workbook_index(
    store_path: &Path,
    index_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchIndexResult>, DiffErrorPayload> {
    let store = OpStore::open(store_path).map_err(store_error)?;
    let conn = store.connection();

    let mut stmt = conn
        .prepare(
            "SELECT sheet, addr, kind, text FROM cell_docs WHERE index_id = ?1 AND cell_docs MATCH ?2 LIMIT ?3",
        )
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    let query_text = format!("{}*", query);
    let rows = stmt
        .query_map(params![index_id, query_text, limit as i64], |row| {
            Ok(SearchIndexResult {
                sheet: row.get(0)?,
                address: row.get(1)?,
                kind: row.get(2)?,
                text: row.get(3)?,
            })
        })
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?);
    }
    Ok(results)
}

fn match_op(op: &excel_diff::DiffOp, strings: &[String], query: &str) -> Option<SearchResult> {
    let query_lower = query.to_lowercase();
    match op {
        excel_diff::DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            let sheet_name = resolve_string(strings, *sheet).to_string();
            let old_value = render_cell_value(strings, &from.value);
            let new_value = render_cell_value(strings, &to.value);
            let old_formula = render_formula(strings, from.formula);
            let new_formula = render_formula(strings, to.formula);
            let text = format!(
                "{} {} {} {}",
                old_value, new_value, old_formula, new_formula
            )
            .to_lowercase();
            if text.contains(&query_lower) {
                return Some(SearchResult {
                    kind: "cell".to_string(),
                    sheet: Some(sheet_name),
                    address: Some(addr.to_a1()),
                    label: "Cell change".to_string(),
                    detail: Some(format!("{} -> {}", old_value, new_value)),
                });
            }
        }
        excel_diff::DiffOp::QueryAdded { name }
        | excel_diff::DiffOp::QueryRemoved { name }
        | excel_diff::DiffOp::QueryRenamed { from: name, .. }
        | excel_diff::DiffOp::QueryDefinitionChanged { name, .. }
        | excel_diff::DiffOp::QueryMetadataChanged { name, .. } => {
            let query_name = resolve_string(strings, *name);
            if query_name.to_lowercase().contains(&query_lower) {
                return Some(SearchResult {
                    kind: "query".to_string(),
                    sheet: None,
                    address: None,
                    label: format!("Query: {query_name}"),
                    detail: Some(op_kind(op).to_string()),
                });
            }
        }
        excel_diff::DiffOp::DuplicateKeyCluster {
            sheet,
            key,
            left_rows,
            right_rows,
        } => {
            let sheet_name = resolve_string(strings, *sheet).to_string();
            let key_text = key
                .iter()
                .map(|value| render_cell_value(strings, value))
                .collect::<Vec<_>>()
                .join(" ");
            let row_text = format!("left {} right {}", left_rows.len(), right_rows.len());
            let text = format!("{key_text} {row_text}").to_lowercase();
            if text.contains(&query_lower) {
                return Some(SearchResult {
                    kind: "duplicate_key_cluster".to_string(),
                    sheet: Some(sheet_name),
                    address: None,
                    label: "Duplicate key cluster".to_string(),
                    detail: Some(row_text),
                });
            }
        }
        _ => {}
    }

    None
}

fn op_kind(op: &excel_diff::DiffOp) -> &'static str {
    match op {
        excel_diff::DiffOp::CellEdited { .. } => "CellEdited",
        excel_diff::DiffOp::QueryAdded { .. } => "QueryAdded",
        excel_diff::DiffOp::QueryRemoved { .. } => "QueryRemoved",
        excel_diff::DiffOp::QueryRenamed { .. } => "QueryRenamed",
        excel_diff::DiffOp::QueryDefinitionChanged { .. } => "QueryDefinitionChanged",
        excel_diff::DiffOp::QueryMetadataChanged { .. } => "QueryMetadataChanged",
        excel_diff::DiffOp::DuplicateKeyCluster { .. } => "DuplicateKeyCluster",
        _ => "Other",
    }
}

fn resolve_string(strings: &[String], id: excel_diff::StringId) -> &str {
    strings
        .get(id.0 as usize)
        .map(String::as_str)
        .unwrap_or("<unknown>")
}

fn render_cell_value(strings: &[String], value: &Option<CellValue>) -> String {
    match value {
        None => String::new(),
        Some(CellValue::Blank) => String::new(),
        Some(CellValue::Number(n)) => n.to_string(),
        Some(CellValue::Text(id)) => resolve_string(strings, *id).to_string(),
        Some(CellValue::Bool(b)) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Some(CellValue::Error(id)) => resolve_string(strings, *id).to_string(),
    }
}

fn render_formula(strings: &[String], formula: Option<excel_diff::StringId>) -> String {
    match formula {
        Some(id) => {
            let raw = resolve_string(strings, id);
            if raw.is_empty() {
                String::new()
            } else if raw.starts_with('=') {
                raw.to_string()
            } else {
                format!("={}", raw)
            }
        }
        None => String::new(),
    }
}

fn find_existing_index(
    conn: &Connection,
    path: &str,
    mtime: i64,
    size: i64,
    side: &str,
) -> Result<Option<SearchIndexSummary>, DiffErrorPayload> {
    let row = conn
        .query_row(
            "SELECT index_id, created_at FROM workbook_indexes WHERE path = ?1 AND mtime = ?2 AND size = ?3 AND side = ?4",
            params![path, mtime, size, side],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

    Ok(row.map(|(index_id, created_at)| SearchIndexSummary {
        index_id,
        path: path.to_string(),
        side: side.to_string(),
        created_at,
    }))
}

fn index_workbook(conn: &Connection, path: &Path, index_id: &str) -> Result<(), StoreError> {
    let file = std::fs::File::open(path).map_err(|e| StoreError::InvalidData(e.to_string()))?;
    let pkg = WorkbookPackage::open(file).map_err(|e| StoreError::InvalidData(e.to_string()))?;

    conn.execute_batch("BEGIN IMMEDIATE")?;

    excel_diff::with_default_session(|session| {
        for sheet in &pkg.workbook.sheets {
            let sheet_name = session.strings.resolve(sheet.name).to_string();
            for ((row, col), cell) in sheet.grid.iter_cells() {
                let addr = excel_diff::CellAddress::from_coords(row, col).to_a1();
                if let Some(value) = &cell.value {
                    let text = match value {
                        CellValue::Number(n) => n.to_string(),
                        CellValue::Text(id) => session.strings.resolve(*id).to_string(),
                        CellValue::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                        CellValue::Error(id) => session.strings.resolve(*id).to_string(),
                        CellValue::Blank => String::new(),
                    };
                    if !text.is_empty() {
                        let _ = conn.execute(
                            "INSERT INTO cell_docs (index_id, sheet, addr, kind, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![index_id, sheet_name, addr, "value", text],
                        );
                    }
                }
                if let Some(formula_id) = cell.formula {
                    let formula = session.strings.resolve(formula_id).to_string();
                    if !formula.is_empty() {
                        let _ = conn.execute(
                            "INSERT INTO cell_docs (index_id, sheet, addr, kind, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![index_id, sheet_name, addr, "formula", formula],
                        );
                    }
                }
            }
        }

        if let Some(dm) = &pkg.data_mashup {
            if let Ok(queries) = excel_diff::build_queries(dm) {
                for query in queries {
                    let text = query.expression_m;
                    let name = query.metadata.formula_name;
                    let _ = conn.execute(
                        "INSERT INTO cell_docs (index_id, sheet, addr, kind, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![index_id, name, "", "query", text],
                    );
                }
            }
        }

        if let Some(vba_modules) = &pkg.vba_modules {
            for module in vba_modules {
                let _ = conn.execute(
                    "INSERT INTO cell_docs (index_id, sheet, addr, kind, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![index_id, session.strings.resolve(module.name), "", "vba", module.code],
                );
            }
        }
    });

    conn.execute_batch("COMMIT")?;
    Ok(())
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "".to_string())
}

fn store_error(err: StoreError) -> DiffErrorPayload {
    DiffErrorPayload::new("store", err.to_string(), false)
}
