#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use desktop_backend::{
    BatchOutcome, BatchRequest, BackendConfig, DesktopBackend, DiffErrorPayload, DiffOutcome, DiffRequest,
    DiffRunSummary, ProgressRx, RecentComparison, SearchIndexResult, SearchIndexSummary, SearchResult,
    SheetPayloadRequest,
};
use tauri::{AppHandle, Emitter, Manager, State};
use ui_payload::{DiffOptions, HostCapabilities, HostDefaults};

struct ActiveDiff {
    run_id: u64,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct DiffState {
    current: Mutex<Option<ActiveDiff>>,
}

struct DesktopState {
    backend: DesktopBackend,
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
fn load_recents(desktop: State<'_, DesktopState>) -> Result<Vec<RecentComparison>, DiffErrorPayload> {
    desktop.backend.load_recents()
}

#[tauri::command]
fn save_recent(
    desktop: State<'_, DesktopState>,
    entry: RecentComparison,
) -> Result<Vec<RecentComparison>, DiffErrorPayload> {
    desktop.backend.save_recent(entry)
}

#[tauri::command]
fn pick_file() -> Option<String> {
    let path = rfd::FileDialog::new()
        .add_filter("Excel / PBIX", &["xlsx", "xlsm", "xltx", "xltm", "xlsb", "pbix", "pbit"])
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

    let runner = desktop.backend.runner.clone();
    let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
    spawn_progress_forwarder(app.clone(), progress_rx);

    let task = tauri::async_runtime::spawn_blocking(move || {
        let request = DiffRequest {
            old_path,
            new_path,
            run_id,
            options,
            cancel: cancel_flag,
            progress: progress_tx,
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
    desktop.backend.load_diff_summary(&diff_id)
}

#[tauri::command]
async fn load_sheet_payload(
    app: AppHandle,
    desktop: State<'_, DesktopState>,
    diff_id: String,
    sheet_name: String,
) -> Result<ui_payload::DiffWithSheets, DiffErrorPayload> {
    let runner = desktop.backend.runner.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
    spawn_progress_forwarder(app, progress_rx);

    let task = tauri::async_runtime::spawn_blocking(move || {
        runner.load_sheet_payload(SheetPayloadRequest {
            diff_id,
            sheet_name,
            cancel,
            progress: progress_tx,
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
    let summary = desktop.backend.load_diff_summary(&diff_id)?;
    let filename = DesktopBackend::default_export_name(&summary, "audit", "xlsx");

    let path = rfd::FileDialog::new()
        .set_file_name(&filename)
        .add_filter("Excel", &["xlsx"])
        .save_file()
        .ok_or_else(|| DiffErrorPayload::new("canceled", "Export canceled.", false))?;

    desktop
        .backend
        .export_audit_xlsx_to_path(&diff_id, &path)?;
    Ok(path.display().to_string())
}

#[tauri::command]
async fn run_batch_compare(
    app: AppHandle,
    desktop: State<'_, DesktopState>,
    request: BatchRequest,
) -> Result<BatchOutcome, DiffErrorPayload> {
    let backend = desktop.backend.clone();
    let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
    spawn_progress_forwarder(app, progress_rx);
    let task = tauri::async_runtime::spawn_blocking(move || backend.run_batch_compare(request, progress_tx));
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
    desktop.backend.load_batch_summary(&batch_id)
}

#[tauri::command]
fn search_diff_ops(
    desktop: State<'_, DesktopState>,
    diff_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, DiffErrorPayload> {
    let limit = limit.unwrap_or(100);
    desktop.backend.search_diff_ops(&diff_id, &query, limit)
}

#[tauri::command]
fn build_search_index(
    desktop: State<'_, DesktopState>,
    path: String,
    side: String,
) -> Result<SearchIndexSummary, DiffErrorPayload> {
    let path = PathBuf::from(path);
    desktop.backend.build_search_index(&path, &side)
}

#[tauri::command]
fn search_workbook_index(
    desktop: State<'_, DesktopState>,
    index_id: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchIndexResult>, DiffErrorPayload> {
    let limit = limit.unwrap_or(100);
    desktop.backend.search_workbook_index(&index_id, &query, limit)
}

fn spawn_progress_forwarder(app: AppHandle, rx: ProgressRx) {
    thread::spawn(move || {
        for event in rx.iter() {
            let _ = app.emit("diff-progress", event);
        }
    });
}

fn main() {
    tauri::Builder::default()
        .manage(DiffState::default())
        .setup(|app| {
            let app_version = env!("CARGO_PKG_VERSION").to_string();
            let backend = DesktopBackend::init(BackendConfig {
                app_name: "excel_diff".to_string(),
                app_version: app_version.clone(),
                engine_version: app_version,
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.message))?;
            app.manage(DesktopState { backend });
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
