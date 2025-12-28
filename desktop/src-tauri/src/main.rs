#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

struct ActiveDiff {
    run_id: u64,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct DiffState {
    current: Mutex<Option<ActiveDiff>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    run_id: u64,
    stage: String,
    detail: String,
}

fn emit_progress(app: &AppHandle, run_id: u64, stage: &str, detail: &str) {
    let _ = app.emit(
        "diff-progress",
        ProgressEvent {
            run_id,
            stage: stage.to_string(),
            detail: detail.to_string(),
        },
    );
}

struct EngineProgress {
    app: AppHandle,
    run_id: u64,
    cancel: Arc<AtomicBool>,
    last_phase: Mutex<Option<String>>,
}

impl EngineProgress {
    fn new(app: AppHandle, run_id: u64, cancel: Arc<AtomicBool>) -> Self {
        Self {
            app,
            run_id,
            cancel,
            last_phase: Mutex::new(None),
        }
    }

    fn map_detail(phase: &str) -> &'static str {
        match phase {
            "parse" => "Parsing workbooks...",
            "alignment" => "Aligning rows and columns...",
            "cell_diff" => "Diffing cells...",
            "move_detection" => "Detecting moved blocks...",
            "m_diff" => "Diffing Power Query...",
            _ => "Diffing workbooks...",
        }
    }

    fn should_emit(&self, phase: &str) -> bool {
        let mut last = self.last_phase.lock().unwrap_or_else(|e| e.into_inner());
        if last.as_deref() == Some(phase) {
            return false;
        }
        *last = Some(phase.to_string());
        true
    }
}

impl excel_diff::ProgressCallback for EngineProgress {
    fn on_progress(&self, phase: &str, _percent: f32) {
        if self.cancel.load(Ordering::Relaxed) {
            panic!("diff canceled");
        }
        if self.should_emit(phase) {
            emit_progress(&self.app, self.run_id, "diff", Self::map_detail(phase));
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiffOptions {
    ignore_blank_to_blank: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RecentComparison {
    old_path: String,
    new_path: String,
    old_name: String,
    new_name: String,
    last_run_iso: String,
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
    old_path: String,
    new_path: String,
    run_id: u64,
    options: Option<DiffOptions>,
) -> Result<ui_payload::DiffWithSheets, String> {
    if let Some(opts) = options {
        let _ = opts.ignore_blank_to_blank;
    }
    let cancel_flag = {
        let mut current = match state.current.lock() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        if current.is_some() {
            return Err("Diff already in progress.".to_string());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *current = Some(ActiveDiff {
            run_id,
            cancel: cancel.clone(),
        });
        cancel
    };

    let app_handle = app.clone();
    let task = tauri::async_runtime::spawn_blocking(move || {
        emit_progress(&app_handle, run_id, "read", "Reading files...");

        let old_kind = ui_payload::host_kind_from_path(Path::new(&old_path))
            .ok_or_else(|| "Unsupported old file extension".to_string())?;
        let new_kind = ui_payload::host_kind_from_path(Path::new(&new_path))
            .ok_or_else(|| "Unsupported new file extension".to_string())?;

        if old_kind != new_kind {
            return Err("Old/new files must be the same type".to_string());
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return Err("Diff canceled.".to_string());
        }

        let cfg = excel_diff::DiffConfig::default();

        match old_kind {
            ui_payload::HostKind::Workbook => {
                let old_file = std::fs::File::open(&old_path)
                    .map_err(|e| format!("Failed to open old file: {e}"))?;
                let new_file = std::fs::File::open(&new_path)
                    .map_err(|e| format!("Failed to open new file: {e}"))?;

                let old_pkg = excel_diff::WorkbookPackage::open(old_file)
                    .map_err(|e| format!("Failed to parse old workbook: {e}"))?;
                let new_pkg = excel_diff::WorkbookPackage::open(new_file)
                    .map_err(|e| format!("Failed to parse new workbook: {e}"))?;

                emit_progress(&app_handle, run_id, "diff", "Diffing workbooks...");
                let progress = EngineProgress::new(app_handle.clone(), run_id, cancel_flag.clone());
                let report = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    old_pkg.diff_with_progress(&new_pkg, &cfg, &progress)
                })) {
                    Ok(report) => report,
                    Err(_) => {
                        if cancel_flag.load(Ordering::Relaxed) {
                            return Err("Diff canceled.".to_string());
                        }
                        return Err("Diff failed unexpectedly.".to_string());
                    }
                };

                if cancel_flag.load(Ordering::Relaxed) {
                    return Err("Diff canceled.".to_string());
                }

                emit_progress(&app_handle, run_id, "snapshot", "Building previews...");
                Ok(ui_payload::build_payload_from_workbook_report(
                    report, &old_pkg, &new_pkg,
                ))
            }
            ui_payload::HostKind::Pbix => {
                let old_file = std::fs::File::open(&old_path)
                    .map_err(|e| format!("Failed to open old file: {e}"))?;
                let new_file = std::fs::File::open(&new_path)
                    .map_err(|e| format!("Failed to open new file: {e}"))?;

                let old_pkg = excel_diff::PbixPackage::open(old_file)
                    .map_err(|e| format!("Failed to parse old PBIX/PBIT: {e}"))?;
                let new_pkg = excel_diff::PbixPackage::open(new_file)
                    .map_err(|e| format!("Failed to parse new PBIX/PBIT: {e}"))?;

                emit_progress(&app_handle, run_id, "diff", "Diffing PBIX metadata...");
                Ok(ui_payload::build_payload_from_pbix(&old_pkg, &new_pkg, &cfg))
            }
        }
    });

    let result = match task.await {
        Ok(result) => result,
        Err(e) => Err(format!("Diff task failed: {e}")),
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

fn main() {
    tauri::Builder::default()
        .manage(DiffState::default())
        .invoke_handler(tauri::generate_handler![
            get_version,
            load_recents,
            save_recent,
            pick_file,
            cancel_diff,
            diff_paths_with_sheets
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
