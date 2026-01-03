#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod diff_runner;
mod export;
mod store;
mod batch;
mod search;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use ui_payload::{DiffOptions, HostCapabilities, HostDefaults};

use crate::diff_runner::{
    DiffErrorPayload, DiffOutcome, DiffRequest, DiffRunner, SheetPayloadRequest,
};
use crate::store::{DiffRunSummary, OpStore, StoreError};
use crate::batch::{BatchOutcome, BatchRequest};
use crate::search::{SearchIndexResult, SearchIndexSummary, SearchResult};

struct ActiveDiff {
    run_id: u64,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct DiffState {
    current: Mutex<Option<ActiveDiff>>,
}

struct DesktopState {
    runner: DiffRunner,
    store_path: PathBuf,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RecentComparison {
    old_path: String,
    new_path: String,
    old_name: String,
    new_name: String,
    last_run_iso: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    diff_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
}

fn recents_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Unable to resolve app data directory: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create app data directory: {e}"))?;
    Ok(dir.join("recents.json"))
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Unable to resolve app data directory: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create app data directory: {e}"))?;
    Ok(dir.join("diff_store.sqlite"))
}

fn load_recents_from_disk(path: &Path) -> Vec<RecentComparison> {
    let data = std::fs::read_to_string(path).unwrap_or_default();
    if data.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_recents_to_disk(path: &Path, entries: &[RecentComparison]) -> Result<(), String> {
    let data = serde_json::to_string_pretty(entries)
        .map_err(|e| format!("Failed to serialize recents: {e}"))?;
    std::fs::write(path, data).map_err(|e| format!("Failed to write recents: {e}"))
}

fn update_recents(mut entries: Vec<RecentComparison>, entry: RecentComparison) -> Vec<RecentComparison> {
    entries.retain(|item| !(item.old_path == entry.old_path && item.new_path == entry.new_path));
    entries.insert(0, entry);
    entries.truncate(20);
    entries
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_capabilities() -> HostCapabilities {
    HostCapabilities::new(env!("CARGO_PKG_VERSION").to_string()).with_defaults(HostDefaults {
        max_memory_mb: None,
        large_mode_threshold: excel_diff::AUTO_STREAM_CELL_THRESHOLD,
    })
}

#[tauri::command]
fn load_recents(app: AppHandle) -> Result<Vec<RecentComparison>, String> {
    let path = recents_path(&app)?;
    Ok(load_recents_from_disk(&path))
}

#[tauri::command]
fn save_recent(app: AppHandle, entry: RecentComparison) -> Result<Vec<RecentComparison>, String> {
    let path = recents_path(&app)?;
    let current = load_recents_from_disk(&path);
    let updated = update_recents(current, entry);
    save_recents_to_disk(&path, &updated)?;
    Ok(updated)
}

#[tauri::command]
fn pick_file() -> Option<String> {
    let path = rfd::FileDialog::new()
        .add_filter("Excel / PBIX", &["xlsx", "xlsm", "xltx", "xltm", "pbix", "pbit"])
        .pick_file()?;
    Some(path.display().to_string())
}

#[tauri::command]
fn pick_folder() -> Option<String> {
    let path = rfd::FileDialog::new().pick_folder()?;
    Some(path.display().to_string())
}

#[tauri::command]
fn cancel_diff(state: State<'_, DiffState>, run_id: u64) -> bool {
    let current = match state.current.lock() {
        Ok(lock) => lock,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(active) = current.as_ref() {
        if active.run_id == run_id {
            active.cancel.store(true, Ordering::Relaxed);
            return true;
        }
    }
    false
}

#[tauri::command]
async fn diff_paths_with_sheets(
    app: AppHandle,
    state: State<'_, DiffState>,
    desktop: State<'_, DesktopState>,
    old_path: String,
    new_path: String,
    run_id: u64,
    options: Option<DiffOptions>,
) -> Result<DiffOutcome, DiffErrorPayload> {
    let options = options.unwrap_or_default();
    let cancel_flag = {
        let mut current = match state.current.lock() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        if current.is_some() {
            return Err(DiffErrorPayload::new("busy", "Diff already in progress.", false));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *current = Some(ActiveDiff {
            run_id,
            cancel: cancel.clone(),
        });
        cancel
    };

    let runner = desktop.runner.clone();
    let app_handle = app.clone();
    let task = tauri::async_runtime::spawn_blocking(move || {
        let request = DiffRequest {
            old_path,
            new_path,
            run_id,
            options,
            cancel: cancel_flag,
            app: app_handle,
        };
        runner.diff(request)
    });

    let result = match task.await {
        Ok(result) => result,
        Err(e) => Err(DiffErrorPayload::new("task", format!("Diff task failed: {e}"), false)),
    };

    let mut current = match state.current.lock() {
        Ok(lock) => lock,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(active) = current.as_ref() {
        if active.run_id == run_id {
            *current = None;
        }
    }

    result
}

#[tauri::command]
fn load_diff_summary(
    desktop: State<'_, DesktopState>,
    diff_id: String,
) -> Result<DiffRunSummary, DiffErrorPayload> {
    let store = OpStore::open(&desktop.store_path).map_err(map_store_error)?;
    store.load_summary(&diff_id).map_err(map_store_error)
}

#[tauri::command]
async fn load_sheet_payload(
    app: AppHandle,
    desktop: State<'_, DesktopState>,
    diff_id: String,
    sheet_name: String,
) -> Result<ui_payload::DiffWithSheets, DiffErrorPayload> {
    let runner = desktop.runner.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let task = tauri::async_runtime::spawn_blocking(move || {
        runner.load_sheet_payload(SheetPayloadRequest {
            diff_id,
            sheet_name,
            cancel,
            app,
        })
    });

    match task.await {
        Ok(result) => result,
        Err(e) => Err(DiffErrorPayload::new("task", format!("Load task failed: {e}"), false)),
    }
}

#[tauri::command]
fn export_audit_xlsx(
    _app: AppHandle,
    desktop: State<'_, DesktopState>,
    diff_id: String,
) -> Result<String, DiffErrorPayload> {
    let store = OpStore::open(&desktop.store_path).map_err(map_store_error)?;
    let summary = store.load_summary(&diff_id).map_err(map_store_error)?;
    let filename = default_export_name(&summary, "audit", "xlsx");

    let path = rfd::FileDialog::new()
        .set_file_name(&filename)
        .add_filter("Excel", &["xlsx"])
        .save_file()
        .ok_or_else(|| DiffErrorPayload::new("canceled", "Export canceled.", false))?;

    diff_runner::export_audit_xlsx(&diff_id, &desktop.store_path, &path)?;
    Ok(path.display().to_string())
}

#[tauri::command]
async fn run_batch_compare(
    app: AppHandle,
    desktop: State<'_, DesktopState>,
    request: BatchRequest,
) -> Result<BatchOutcome, DiffErrorPayload> {
    let runner = desktop.runner.clone();
    let store_path = desktop.store_path.clone();
    let task = tauri::async_runtime::spawn_blocking(move || {
        batch::run_batch_compare(app, runner, &store_path, request)
    });
    match task.await {
        Ok(result) => result,
        Err(e) => Err(DiffErrorPayload::new("task", format!("Batch task failed: {e}"), false)),
    }
}

#[tauri::command]
fn load_batch_summary(
    desktop: State<'_, DesktopState>,
    batch_id: String,
) -> Result<BatchOutcome, DiffErrorPayload> {
    batch::load_batch_summary(&desktop.store_path, &batch_id)
}

#[tauri::command]
fn search_diff_ops(
    desktop: State<'_, DesktopState>,
    diff_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, DiffErrorPayload> {
    let limit = limit.unwrap_or(100);
    search::search_diff_ops(&desktop.store_path, &diff_id, &query, limit)
}

#[tauri::command]
fn build_search_index(
    app: AppHandle,
    desktop: State<'_, DesktopState>,
    path: String,
    side: String,
) -> Result<SearchIndexSummary, DiffErrorPayload> {
    let path = PathBuf::from(path);
    search::build_search_index(app, &desktop.store_path, &path, &side)
}

#[tauri::command]
fn search_workbook_index(
    desktop: State<'_, DesktopState>,
    index_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchIndexResult>, DiffErrorPayload> {
    let limit = limit.unwrap_or(100);
    search::search_workbook_index(&desktop.store_path, &index_id, &query, limit)
}

fn default_export_name(summary: &DiffRunSummary, prefix: &str, ext: &str) -> String {
    let old = base_name(&summary.old_path);
    let new = base_name(&summary.new_path);
    let date = summary
        .finished_at
        .as_deref()
        .unwrap_or(&summary.started_at)
        .get(0..10)
        .unwrap_or("report");
    format!("excel-diff-{prefix}__{old}__{new}__{date}.{ext}")
}

fn base_name(path: &str) -> String {
    let parts: Vec<&str> = path.split(['\\', '/']).collect();
    parts.last().unwrap_or(&path).to_string()
}

fn map_store_error(err: StoreError) -> DiffErrorPayload {
    DiffErrorPayload::new("store", err.to_string(), false)
}

fn main() {
    tauri::Builder::default()
        .manage(DiffState::default())
        .setup(|app| {
            let store_path = store_path(&app.handle()).map_err(|e| e.to_string())?;
            let app_version = env!("CARGO_PKG_VERSION").to_string();
            let engine_version = app_version.clone();
            let runner = DiffRunner::new(store_path.clone(), app_version, engine_version);
            app.manage(DesktopState { runner, store_path });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_capabilities,
            load_recents,
            save_recent,
            pick_file,
            pick_folder,
            cancel_diff,
            diff_paths_with_sheets,
            load_diff_summary,
            load_sheet_payload,
            export_audit_xlsx,
            run_batch_compare,
            load_batch_summary,
            search_diff_ops,
            build_search_index,
            search_workbook_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
