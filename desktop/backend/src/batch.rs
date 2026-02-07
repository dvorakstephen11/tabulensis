use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use globset::{Glob, GlobSet, GlobSetBuilder};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::events::{ProgressEvent, ProgressTx};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::diff_runner::{DiffErrorPayload, DiffRequest, DiffRunner};
use ui_payload::DiffOptions;
use crate::store::{OpStore, StoreError};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOutcome {
    pub batch_id: String,
    pub status: String,
    pub item_count: usize,
    pub completed_count: usize,
    pub items: Vec<BatchItemResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemResult {
    pub item_id: usize,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: String,
    pub diff_id: Option<String>,
    pub op_count: Option<u64>,
    pub warnings_count: Option<usize>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub old_root: String,
    pub new_root: String,
    pub strategy: String,
    pub include_globs: Option<Vec<String>>,
    pub exclude_globs: Option<Vec<String>>,
    pub trusted: bool,
}

pub fn run_batch_compare(
    runner: DiffRunner,
    store_path: &Path,
    request: BatchRequest,
    progress: ProgressTx,
) -> Result<BatchOutcome, DiffErrorPayload> {
    let old_root = PathBuf::from(&request.old_root);
    let new_root = PathBuf::from(&request.new_root);

    let include = build_globset(&request.include_globs)?;
    let exclude = build_globset(&request.exclude_globs)?;
    let include_all = request
        .include_globs
        .as_ref()
        .map(|list| list.is_empty())
        .unwrap_or(true);

    let old_files = collect_files(&old_root, &include, &exclude, include_all)?;
    let new_files = collect_files(&new_root, &include, &exclude, include_all)?;

    let strategy = request.strategy.to_lowercase();
    let batch_id = Uuid::new_v4().to_string();

    let mut pairs = pair_files(&old_root, &new_root, &old_files, &new_files, &strategy);
    pairs.sort_by(|a, b| a.key.cmp(&b.key));

    let store = OpStore::open(store_path).map_err(store_error)?;
    let conn = store.connection();
    insert_batch_run(conn, &batch_id, &request, pairs.len())?;

    let mut items = Vec::new();
    let mut completed = 0;

    for (idx, pair) in pairs.iter().enumerate() {
        let item_id = idx;
        let mut result = BatchItemResult {
            item_id,
            old_path: pair.old.as_ref().map(|p| p.display().to_string()),
            new_path: pair.new.as_ref().map(|p| p.display().to_string()),
            status: pair.status.clone(),
            diff_id: None,
            op_count: None,
            warnings_count: None,
            error: pair.error.clone(),
        };

        insert_batch_item(conn, &batch_id, &result)?;

        if pair.status != "pending" {
            items.push(result);
            continue;
        }

        emit_batch_progress(&progress, format!("Comparing {}", pair.key));
        let cancel = Arc::new(AtomicBool::new(false));
        let diff_request = DiffRequest {
            old_path: pair.old.as_ref().unwrap().display().to_string(),
            new_path: pair.new.as_ref().unwrap().display().to_string(),
            run_id: 0,
            options: DiffOptions {
                trusted: Some(request.trusted),
                ..DiffOptions::default()
            },
            cancel,
            progress: progress.clone(),
        };

        match runner.diff(diff_request) {
            Ok(outcome) => {
                result.status = "complete".to_string();
                result.diff_id = Some(outcome.diff_id.clone());
                if let Some(summary) = outcome.summary {
                    result.op_count = Some(summary.op_count);
                    result.warnings_count = Some(summary.warnings.len());
                }
                update_batch_item(conn, &batch_id, &result)?;
            }
            Err(err) => {
                result.status = "failed".to_string();
                result.error = Some(err.message);
                update_batch_item(conn, &batch_id, &result)?;
            }
        }

        completed += 1;
        update_batch_progress(conn, &batch_id, completed)?;
        items.push(result);
    }

    let status = "complete".to_string();
    finish_batch_run(conn, &batch_id, &status, completed)?;
    emit_batch_progress(&progress, format!("Batch complete: {completed}/{total}", total = pairs.len()));

    Ok(BatchOutcome {
        batch_id,
        status,
        item_count: pairs.len(),
        completed_count: completed,
        items,
    })
}

pub fn load_batch_summary(store_path: &Path, batch_id: &str) -> Result<BatchOutcome, DiffErrorPayload> {
    let store = OpStore::open(store_path).map_err(store_error)?;
    let conn = store.connection();

    let (status, item_count, completed_count) = conn
        .query_row(
            "SELECT status, item_count, completed_count FROM batch_runs WHERE batch_id = ?1",
            params![batch_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
        )
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

    let mut stmt = conn
        .prepare(
            "SELECT item_id, old_path, new_path, status, diff_id, op_count, warnings_count, error FROM batch_items WHERE batch_id = ?1 ORDER BY item_id",
        )
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    let rows = stmt
        .query_map(params![batch_id], |row| {
            Ok(BatchItemResult {
                item_id: row.get::<_, i64>(0)? as usize,
                old_path: row.get::<_, Option<String>>(1)?,
                new_path: row.get::<_, Option<String>>(2)?,
                status: row.get::<_, String>(3)?,
                diff_id: row.get::<_, Option<String>>(4)?,
                op_count: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                warnings_count: row.get::<_, Option<i64>>(6)?.map(|v| v as usize),
                error: row.get::<_, Option<String>>(7)?,
            })
        })
        .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?);
    }

    Ok(BatchOutcome {
        batch_id: batch_id.to_string(),
        status,
        item_count: item_count as usize,
        completed_count: completed_count as usize,
        items,
    })
}

struct PairCandidate {
    key: String,
    old: Option<PathBuf>,
    new: Option<PathBuf>,
    status: String,
    error: Option<String>,
}

fn collect_files(
    root: &Path,
    include: &GlobSet,
    exclude: &GlobSet,
    include_all: bool,
) -> Result<Vec<PathBuf>, DiffErrorPayload> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_supported(path) {
            continue;
        }
        if include_all || include.is_match(path) {
            if !exclude.is_match(path) {
                files.push(path.to_path_buf());
            }
        }
    }
    Ok(files)
}

fn build_globset(globs: &Option<Vec<String>>) -> Result<GlobSet, DiffErrorPayload> {
    let mut builder = GlobSetBuilder::new();
    if let Some(globs) = globs {
        for glob in globs {
            let parsed = Glob::new(glob).map_err(|e| DiffErrorPayload::new("glob", e.to_string(), false))?;
            builder.add(parsed);
        }
    }
    builder.build().map_err(|e| DiffErrorPayload::new("glob", e.to_string(), false))
}

fn pair_files(
    old_root: &Path,
    new_root: &Path,
    old_files: &[PathBuf],
    new_files: &[PathBuf],
    strategy: &str,
) -> Vec<PairCandidate> {
    let mut old_map = group_files(old_root, old_files, strategy);
    let mut new_map = group_files(new_root, new_files, strategy);
    let mut keys = BTreeMap::new();

    for key in old_map.keys() {
        keys.insert(key.clone(), ());
    }
    for key in new_map.keys() {
        keys.insert(key.clone(), ());
    }

    let mut pairs = Vec::new();
    for key in keys.keys() {
        let old_list = old_map.remove(key).unwrap_or_default();
        let new_list = new_map.remove(key).unwrap_or_default();

        match (old_list.len(), new_list.len()) {
            (1, 1) => pairs.push(PairCandidate {
                key: key.clone(),
                old: Some(old_list[0].clone()),
                new: Some(new_list[0].clone()),
                status: "pending".to_string(),
                error: None,
            }),
            (0, 1) => pairs.push(PairCandidate {
                key: key.clone(),
                old: None,
                new: Some(new_list[0].clone()),
                status: "missing_old".to_string(),
                error: Some("Missing old file".to_string()),
            }),
            (1, 0) => pairs.push(PairCandidate {
                key: key.clone(),
                old: Some(old_list[0].clone()),
                new: None,
                status: "missing_new".to_string(),
                error: Some("Missing new file".to_string()),
            }),
            _ => pairs.push(PairCandidate {
                key: key.clone(),
                old: old_list.get(0).cloned(),
                new: new_list.get(0).cloned(),
                status: "duplicate".to_string(),
                error: Some("Duplicate match".to_string()),
            }),
        }
    }

    pairs
}

fn group_files(root: &Path, files: &[PathBuf], strategy: &str) -> HashMap<String, Vec<PathBuf>> {
    let mut map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in files {
        let key = match strategy {
            "filename" => path.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase(),
            _ => path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
                .to_lowercase(),
        };
        map.entry(key).or_default().push(path.clone());
    }
    map
}

fn is_supported(path: &Path) -> bool {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    matches!(ext.as_str(), "xlsx" | "xlsm" | "xltx" | "xltm" | "xlsb" | "pbix" | "pbit")
}

fn insert_batch_run(conn: &Connection, batch_id: &str, req: &BatchRequest, item_count: usize) -> Result<(), DiffErrorPayload> {
    conn.execute(
        "INSERT INTO batch_runs (batch_id, old_root, new_root, strategy, started_at, status, item_count, completed_count)\
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            batch_id,
            req.old_root,
            req.new_root,
            req.strategy,
            now_iso(),
            "running",
            item_count as i64,
            0i64,
        ],
    )
    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    Ok(())
}

fn insert_batch_item(conn: &Connection, batch_id: &str, item: &BatchItemResult) -> Result<(), DiffErrorPayload> {
    conn.execute(
        "INSERT INTO batch_items (batch_id, item_id, old_path, new_path, status, diff_id, op_count, warnings_count, error)\
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            batch_id,
            item.item_id as i64,
            item.old_path,
            item.new_path,
            item.status,
            item.diff_id,
            item.op_count.map(|v| v as i64),
            item.warnings_count.map(|v| v as i64),
            item.error,
        ],
    )
    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    Ok(())
}

fn update_batch_item(conn: &Connection, batch_id: &str, item: &BatchItemResult) -> Result<(), DiffErrorPayload> {
    conn.execute(
        "UPDATE batch_items SET status = ?1, diff_id = ?2, op_count = ?3, warnings_count = ?4, error = ?5 WHERE batch_id = ?6 AND item_id = ?7",
        params![
            item.status,
            item.diff_id,
            item.op_count.map(|v| v as i64),
            item.warnings_count.map(|v| v as i64),
            item.error,
            batch_id,
            item.item_id as i64,
        ],
    )
    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    Ok(())
}

fn update_batch_progress(conn: &Connection, batch_id: &str, completed: usize) -> Result<(), DiffErrorPayload> {
    conn.execute(
        "UPDATE batch_runs SET completed_count = ?1 WHERE batch_id = ?2",
        params![completed as i64, batch_id],
    )
    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    Ok(())
}

fn finish_batch_run(conn: &Connection, batch_id: &str, status: &str, completed: usize) -> Result<(), DiffErrorPayload> {
    conn.execute(
        "UPDATE batch_runs SET status = ?1, completed_count = ?2, finished_at = ?3 WHERE batch_id = ?4",
        params![status, completed as i64, now_iso(), batch_id],
    )
    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
    Ok(())
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "".to_string())
}

fn emit_batch_progress(progress: &ProgressTx, detail: impl Into<String>) {
    let _ = progress.send(ProgressEvent {
        run_id: 0,
        stage: "batch".to_string(),
        detail: detail.into(),
    });
}

fn store_error(err: StoreError) -> DiffErrorPayload {
    DiffErrorPayload::new("store", err.to_string(), false)
}
