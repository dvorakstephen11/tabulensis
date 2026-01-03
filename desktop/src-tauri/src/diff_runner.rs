use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

use excel_diff::{
    should_use_large_mode, ContainerError, ContainerLimits, DiffConfig, DiffError, DiffReport,
    DiffSink, DiffSummary, PbixPackage, ProgressCallback, WorkbookPackage,
};
use lru::LruCache;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::export::export_audit_xlsx_from_store;
use crate::store::{
    resolve_sheet_stats, DiffMode, DiffRunSummary, OpStore, OpStoreSink, RunStatus, SheetStats,
    StoreError,
};
use ui_payload::{build_payload_from_pbix_report, limits_from_config, DiffOptions, DiffOutcomeConfig, DiffPreset};
const WORKBOOK_CACHE_CAPACITY: usize = 4;
const PBIX_CACHE_CAPACITY: usize = 2;

#[derive(Debug, Clone)]
pub struct DiffRequest {
    pub old_path: String,
    pub new_path: String,
    pub run_id: u64,
    pub options: DiffOptions,
    pub cancel: Arc<AtomicBool>,
    pub app: AppHandle,
}

#[derive(Debug, Clone)]
pub struct SheetPayloadRequest {
    pub diff_id: String,
    pub sheet_name: String,
    pub cancel: Arc<AtomicBool>,
    pub app: AppHandle,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffOutcome {
    pub diff_id: String,
    pub mode: DiffMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<ui_payload::DiffWithSheets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<DiffRunSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<DiffOutcomeConfig>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub trusted_retry: bool,
}

impl DiffErrorPayload {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>, trusted_retry: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            trusted_retry,
        }
    }
}

#[derive(Debug, Clone)]
struct CacheKey {
    path: String,
    mtime: u64,
    size: u64,
    trusted: bool,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.mtime == other.mtime
            && self.size == other.size
            && self.trusted == other.trusted
    }
}

impl Eq for CacheKey {}

impl std::hash::Hash for CacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.mtime.hash(state);
        self.size.hash(state);
        self.trusted.hash(state);
    }
}

#[derive(Clone)]
pub struct DiffRunner {
    tx: Sender<EngineCommand>,
}

impl DiffRunner {
    pub fn new(store_path: PathBuf, app_version: String, engine_version: String) -> Self {
        let (tx, rx) = mpsc::channel();
        let engine = EngineState::new(store_path, app_version, engine_version, rx);
        thread::spawn(move || engine.run());
        Self { tx }
    }

    pub fn diff(&self, request: DiffRequest) -> Result<DiffOutcome, DiffErrorPayload> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::Diff {
                request,
                respond_to: reply_tx,
            })
            .map_err(|e| {
                DiffErrorPayload::new("engine_down", e.to_string(), false)
            })?;
        reply_rx.recv().map_err(|e| {
            DiffErrorPayload::new("engine_down", e.to_string(), false)
        })?
    }

    pub fn load_sheet_payload(
        &self,
        request: SheetPayloadRequest,
    ) -> Result<ui_payload::DiffWithSheets, DiffErrorPayload> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::LoadSheet {
                request,
                respond_to: reply_tx,
            })
            .map_err(|e| DiffErrorPayload::new("engine_down", e.to_string(), false))?;
        reply_rx.recv().map_err(|e| {
            DiffErrorPayload::new("engine_down", e.to_string(), false)
        })?
    }
}

enum EngineCommand {
    Diff {
        request: DiffRequest,
        respond_to: Sender<Result<DiffOutcome, DiffErrorPayload>>,
    },
    LoadSheet {
        request: SheetPayloadRequest,
        respond_to: Sender<Result<ui_payload::DiffWithSheets, DiffErrorPayload>>,
    },
}

struct EngineState {
    store_path: PathBuf,
    app_version: String,
    engine_version: String,
    workbook_cache: LruCache<CacheKey, WorkbookPackage>,
    pbix_cache: LruCache<CacheKey, PbixPackage>,
    rx: mpsc::Receiver<EngineCommand>,
}

impl EngineState {
    fn new(
        store_path: PathBuf,
        app_version: String,
        engine_version: String,
        rx: mpsc::Receiver<EngineCommand>,
    ) -> Self {
        Self {
            store_path,
            app_version,
            engine_version,
            workbook_cache: LruCache::new(NonZeroUsize::new(WORKBOOK_CACHE_CAPACITY).unwrap()),
            pbix_cache: LruCache::new(NonZeroUsize::new(PBIX_CACHE_CAPACITY).unwrap()),
            rx,
        }
    }

    fn run(mut self) {
        while let Ok(cmd) = self.rx.recv() {
            match cmd {
                EngineCommand::Diff { request, respond_to } => {
                    let result = self.handle_diff(request);
                    let _ = respond_to.send(result);
                }
                EngineCommand::LoadSheet { request, respond_to } => {
                    let result = self.handle_load_sheet(request);
                    let _ = respond_to.send(result);
                }
            }
        }
    }

    fn handle_diff(&mut self, request: DiffRequest) -> Result<DiffOutcome, DiffErrorPayload> {
        emit_progress(&request.app, request.run_id, "read", "Reading files...");

        let old_path = PathBuf::from(&request.old_path);
        let new_path = PathBuf::from(&request.new_path);

        let old_kind = ui_payload::host_kind_from_path(&old_path)
            .ok_or_else(|| DiffErrorPayload::new("unsupported", "Unsupported old file extension", false))?;
        let new_kind = ui_payload::host_kind_from_path(&new_path)
            .ok_or_else(|| DiffErrorPayload::new("unsupported", "Unsupported new file extension", false))?;

        if old_kind != new_kind {
            return Err(DiffErrorPayload::new(
                "mismatch",
                "Old/new files must be the same type",
                false,
            ));
        }

        if request.cancel.load(Ordering::Relaxed) {
            return Err(DiffErrorPayload::new("canceled", "Diff canceled.", false));
        }

        let store = OpStore::open(&self.store_path).map_err(map_store_error)?;
        let options = request.options.clone();
        let trusted = options.trusted.unwrap_or(false);
        let config = options
            .effective_config(DiffConfig::balanced())
            .map_err(|e| DiffErrorPayload::new("config", e, false))?;
        let config_json = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
        let outcome_config = outcome_config_from_options(&options, &config);

        match old_kind {
            ui_payload::HostKind::Workbook => {
                let old_pkg = self.open_workbook_cached(&old_path, trusted)?;
                let new_pkg = self.open_workbook_cached(&new_path, trusted)?;

                let estimated_cells = estimate_diff_cell_volume(&old_pkg.workbook, &new_pkg.workbook);
                let mode = if should_use_large_mode(estimated_cells, &config) {
                    DiffMode::Large
                } else {
                    DiffMode::Payload
                };

                let diff_id = store
                    .start_run(
                        &request.old_path,
                        &request.new_path,
                        &config_json,
                        &self.engine_version,
                        &self.app_version,
                        mode,
                        trusted,
                    )
                    .map_err(map_store_error)?;

                match mode {
                    DiffMode::Payload => {
                        emit_progress(&request.app, request.run_id, "diff", "Diffing workbooks...");
                        let progress = EngineProgress::new(request.app.clone(), request.run_id, request.cancel.clone());
                        let report = match run_diff_with_progress(
                            || old_pkg.diff_with_progress(&new_pkg, &config, &progress),
                            &request.cancel,
                        ) {
                            Ok(report) => report,
                            Err(err) => {
                                let _ = store.fail_run(&diff_id, status_for_error(&err), &err.message);
                                return Err(err);
                            }
                        };

                        emit_progress(&request.app, request.run_id, "snapshot", "Building previews...");
                        let (counts, sheet_stats) = store
                            .insert_ops_from_report(&diff_id, &report)
                            .map_err(map_store_error)?;
                        let resolved = resolve_sheet_stats(&report.strings, &sheet_stats).map_err(map_store_error)?;
                        let summary = report_to_summary(&report);
                        store
                            .finish_run(&diff_id, &summary, &report.strings, &counts, &resolved, RunStatus::Complete)
                            .map_err(map_store_error)?;

                        let payload = ui_payload::build_payload_from_workbook_report(report, &old_pkg, &new_pkg);
                        let summary_record = store.load_summary(&diff_id).map_err(map_store_error)?;
                        Ok(DiffOutcome {
                            diff_id,
                            mode,
                            payload: Some(payload),
                            summary: Some(summary_record),
                            config: Some(outcome_config.clone()),
                        })
                    }
                    DiffMode::Large => {
                        emit_progress(&request.app, request.run_id, "diff", "Streaming diff to disk...");
                        let progress = EngineProgress::new(request.app.clone(), request.run_id, request.cancel.clone());
                        let sink_store = OpStore::open(&self.store_path).map_err(map_store_error)?;
                        let conn = sink_store.into_connection();
                        let mut sink = OpStoreSink::new(conn, diff_id.clone())
                            .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

                        let summary = match run_diff_with_progress(
                            || old_pkg.diff_streaming_with_progress(&new_pkg, &config, &mut sink, &progress),
                            &request.cancel,
                        ) {
                            Ok(result) => result.map_err(diff_error_from_diff),
                            Err(err) => Err(err),
                        };

                        let summary = match summary {
                            Ok(summary) => summary,
                            Err(err) => {
                                let _ = sink.finish();
                                let _ = store.fail_run(&diff_id, status_for_error(&err), &err.message);
                                return Err(err);
                            }
                        };

                        sink.finish().map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
                        let (_, counts, stats, _) = sink.into_parts();
                        let strings = current_strings();
                        let mut stats: Vec<SheetStats> = stats.into_values().collect();
                        stats.sort_by_key(|entry| entry.sheet_id);
                        let resolved = resolve_sheet_stats(&strings, &stats).map_err(map_store_error)?;
                        store
                            .finish_run(&diff_id, &summary, &strings, &counts, &resolved, RunStatus::Complete)
                            .map_err(map_store_error)?;

                        let summary_record = store.load_summary(&diff_id).map_err(map_store_error)?;
                        Ok(DiffOutcome {
                            diff_id,
                            mode,
                            payload: None,
                            summary: Some(summary_record),
                            config: Some(outcome_config.clone()),
                        })
                    }
                }
            }
            ui_payload::HostKind::Pbix => {
                let old_pkg = self.open_pbix_cached(&old_path, trusted)?;
                let new_pkg = self.open_pbix_cached(&new_path, trusted)?;

                let diff_id = store
                    .start_run(
                        &request.old_path,
                        &request.new_path,
                        &config_json,
                        &self.engine_version,
                        &self.app_version,
                        DiffMode::Payload,
                        trusted,
                    )
                    .map_err(map_store_error)?;

                emit_progress(&request.app, request.run_id, "diff", "Streaming PBIX diff to disk...");
                let progress = EngineProgress::new(request.app.clone(), request.run_id, request.cancel.clone());
                let sink_store = OpStore::open(&self.store_path).map_err(map_store_error)?;
                let conn = sink_store.into_connection();
                let mut sink = OpStoreSink::new(conn, diff_id.clone())
                    .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

                let summary = match run_diff_with_progress(
                    || old_pkg.diff_streaming_with_progress(&new_pkg, &config, &mut sink, &progress),
                    &request.cancel,
                ) {
                    Ok(result) => result.map_err(diff_error_from_diff),
                    Err(err) => Err(err),
                };

                let summary = match summary {
                    Ok(summary) => summary,
                    Err(err) => {
                        let _ = sink.finish();
                        let _ = store.fail_run(&diff_id, status_for_error(&err), &err.message);
                        return Err(err);
                    }
                };

                sink.finish().map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;
                let (_, counts, stats, _) = sink.into_parts();
                let strings = current_strings();
                let mut stats: Vec<SheetStats> = stats.into_values().collect();
                stats.sort_by_key(|entry| entry.sheet_id);
                let resolved = resolve_sheet_stats(&strings, &stats).map_err(map_store_error)?;
                let use_large_mode = should_use_large_mode(summary.op_count as u64, &config);
                if use_large_mode {
                    store.set_mode(&diff_id, DiffMode::Large).map_err(map_store_error)?;
                }
                store
                    .finish_run(&diff_id, &summary, &strings, &counts, &resolved, RunStatus::Complete)
                    .map_err(map_store_error)?;

                let summary_record = store.load_summary(&diff_id).map_err(map_store_error)?;
                if use_large_mode {
                    Ok(DiffOutcome {
                        diff_id,
                        mode: DiffMode::Large,
                        payload: None,
                        summary: Some(summary_record),
                        config: Some(outcome_config.clone()),
                    })
                } else {
                    let report = store.load_report(&diff_id).map_err(map_store_error)?;
                    let payload = build_payload_from_pbix_report(report);
                    Ok(DiffOutcome {
                        diff_id,
                        mode: DiffMode::Payload,
                        payload: Some(payload),
                        summary: Some(summary_record),
                        config: Some(outcome_config.clone()),
                    })
                }
            }
        }
    }

    fn handle_load_sheet(
        &mut self,
        request: SheetPayloadRequest,
    ) -> Result<ui_payload::DiffWithSheets, DiffErrorPayload> {
        let store = OpStore::open(&self.store_path).map_err(map_store_error)?;
        let summary = store.load_summary(&request.diff_id).map_err(map_store_error)?;

        if request.cancel.load(Ordering::Relaxed) {
            return Err(DiffErrorPayload::new("canceled", "Diff canceled.", false));
        }

        let old_path = PathBuf::from(&summary.old_path);
        let new_path = PathBuf::from(&summary.new_path);
        let old_pkg = self.open_workbook_cached(&old_path, summary.trusted)?;
        let new_pkg = self.open_workbook_cached(&new_path, summary.trusted)?;

        let ops = store.load_sheet_ops(&request.diff_id, &request.sheet_name).map_err(map_store_error)?;
        let strings = store.load_strings(&request.diff_id).map_err(map_store_error)?;

        let mut report = DiffReport::new(ops);
        report.strings = strings;
        report.complete = summary.complete;
        report.warnings = summary.warnings.clone();

        emit_progress(&request.app, 0, "snapshot", "Building previews...");
        Ok(ui_payload::build_payload_from_workbook_report(report, &old_pkg, &new_pkg))
    }

    fn open_workbook_cached(&mut self, path: &Path, trusted: bool) -> Result<WorkbookPackage, DiffErrorPayload> {
        let key = cache_key(path, trusted)?;
        if let Some(pkg) = self.workbook_cache.get(&key) {
            return Ok(pkg.clone());
        }
        let file = File::open(path).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;
        let pkg = if trusted {
            WorkbookPackage::open_with_limits(file, trusted_limits())
                .map_err(map_package_error)?
        } else {
            WorkbookPackage::open(file)
                .map_err(map_package_error)?
        };
        self.workbook_cache.put(key, pkg.clone());
        Ok(pkg)
    }

    fn open_pbix_cached(&mut self, path: &Path, trusted: bool) -> Result<PbixPackage, DiffErrorPayload> {
        let key = cache_key(path, trusted)?;
        if let Some(pkg) = self.pbix_cache.get(&key) {
            return Ok(pkg.clone());
        }
        let file = File::open(path).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;
        let pkg = if trusted {
            PbixPackage::open_with_limits(file, trusted_limits())
                .map_err(map_package_error)?
        } else {
            PbixPackage::open(file)
                .map_err(map_package_error)?
        };
        self.pbix_cache.put(key, pkg.clone());
        Ok(pkg)
    }
}

fn report_to_summary(report: &DiffReport) -> DiffSummary {
    DiffSummary {
        complete: report.complete,
        warnings: report.warnings.clone(),
        op_count: report.ops.len(),
        #[cfg(feature = "perf-metrics")]
        metrics: report.metrics.clone(),
    }
}

fn outcome_config_from_options(options: &DiffOptions, cfg: &DiffConfig) -> DiffOutcomeConfig {
    let preset = if options.config_json.as_ref().map(|v| v.trim()).unwrap_or("").is_empty() {
        Some(options.preset.unwrap_or(DiffPreset::Balanced))
    } else {
        None
    };
    DiffOutcomeConfig {
        preset,
        limits: Some(limits_from_config(cfg)),
    }
}

fn cache_key(path: &Path, trusted: bool) -> Result<CacheKey, DiffErrorPayload> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let normalized = canonical.to_string_lossy().to_lowercase();
    let meta = std::fs::metadata(&canonical).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;
    let size = meta.len();
    let mtime = meta.modified().ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|dur| dur.as_secs())
        .unwrap_or(0);
    Ok(CacheKey {
        path: normalized,
        mtime,
        size,
        trusted,
    })
}

fn trusted_limits() -> ContainerLimits {
    let base = ContainerLimits::default();
    ContainerLimits {
        max_entries: base.max_entries.saturating_mul(4),
        max_part_uncompressed_bytes: base.max_part_uncompressed_bytes.saturating_mul(4),
        max_total_uncompressed_bytes: base.max_total_uncompressed_bytes.saturating_mul(4),
    }
}

fn estimate_diff_cell_volume(old: &excel_diff::Workbook, new: &excel_diff::Workbook) -> u64 {
    excel_diff::with_default_session(|session| {
        let mut max_counts: HashMap<(String, excel_diff::SheetKind), u64> = HashMap::new();
        for sheet in old.sheets.iter().chain(new.sheets.iter()) {
            let name_lower = session.strings.resolve(sheet.name).to_lowercase();
            let key = (name_lower, sheet.kind.clone());
            let cell_count = sheet.grid.cell_count() as u64;
            match max_counts.entry(key) {
                Entry::Occupied(mut entry) => {
                    if cell_count > *entry.get() {
                        entry.insert(cell_count);
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(cell_count);
                }
            }
        }
        max_counts.values().copied().sum()
    })
}

fn current_strings() -> Vec<String> {
    excel_diff::with_default_session(|session| session.strings.strings().to_vec())
}

fn map_store_error(err: StoreError) -> DiffErrorPayload {
    DiffErrorPayload::new("store", err.to_string(), false)
}

fn map_package_error(err: excel_diff::PackageError) -> DiffErrorPayload {
    let trusted_retry = matches!(
        err,
        excel_diff::PackageError::Container(ContainerError::TooManyEntries { .. })
            | excel_diff::PackageError::Container(ContainerError::PartTooLarge { .. })
            | excel_diff::PackageError::Container(ContainerError::TotalTooLarge { .. })
    );
    DiffErrorPayload::new(err.code(), err.to_string(), trusted_retry)
}

fn run_diff_with_progress<F, T>(f: F, cancel: &AtomicBool) -> Result<T, DiffErrorPayload>
where
    F: FnOnce() -> T,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => Ok(result),
        Err(_) => {
            if cancel.load(Ordering::Relaxed) {
                Err(DiffErrorPayload::new("canceled", "Diff canceled.", false))
            } else {
                Err(DiffErrorPayload::new("failed", "Diff failed unexpectedly.", false))
            }
        }
    }
}

struct EngineProgress {
    app: AppHandle,
    run_id: u64,
    cancel: Arc<AtomicBool>,
    last_phase: std::sync::Mutex<Option<String>>,
}

impl EngineProgress {
    fn new(app: AppHandle, run_id: u64, cancel: Arc<AtomicBool>) -> Self {
        Self {
            app,
            run_id,
            cancel,
            last_phase: std::sync::Mutex::new(None),
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

impl ProgressCallback for EngineProgress {
    fn on_progress(&self, phase: &str, _percent: f32) {
        if self.cancel.load(Ordering::Relaxed) {
            panic!("diff canceled");
        }
        if self.should_emit(phase) {
            emit_progress(&self.app, self.run_id, "diff", Self::map_detail(phase));
        }
    }
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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    run_id: u64,
    stage: String,
    detail: String,
}

pub fn export_audit_xlsx(diff_id: &str, store_path: &Path, output_path: &Path) -> Result<(), DiffErrorPayload> {
    let store = OpStore::open(store_path).map_err(map_store_error)?;
    export_audit_xlsx_from_store(&store, diff_id, output_path).map_err(|e| {
        DiffErrorPayload::new("export", e.to_string(), false)
    })
}

pub fn diff_error_from_diff(diff_error: DiffError) -> DiffErrorPayload {
    DiffErrorPayload::new(diff_error.code(), diff_error.to_string(), false)
}

fn status_for_error(err: &DiffErrorPayload) -> RunStatus {
    if err.code == "canceled" {
        RunStatus::Canceled
    } else {
        RunStatus::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::estimate_diff_cell_volume;
    use crate::store::DiffMode;
    use excel_diff::{
        CellValue, DiffConfig, Grid, Sheet, SheetKind, Workbook, AUTO_STREAM_CELL_THRESHOLD,
        should_use_large_mode, with_default_session,
    };

    fn create_dense_grid(nrows: u32, ncols: u32) -> Grid {
        let mut grid = Grid::new(nrows, ncols);
        for row in 0..nrows {
            for col in 0..ncols {
                let value = row as f64 * 1000.0 + col as f64;
                grid.insert_cell(row, col, Some(CellValue::Number(value)), None);
            }
        }
        grid
    }

    fn build_workbook(grid: Grid) -> Workbook {
        let name_id = with_default_session(|session| session.strings.intern("Sheet1"));
        Workbook {
            sheets: vec![Sheet {
                name: name_id,
                kind: SheetKind::Worksheet,
                grid,
            }],
            named_ranges: Vec::new(),
            charts: Vec::new(),
        }
    }

    #[test]
    fn large_mode_threshold_triggers_in_desktop() {
        let grid = create_dense_grid(1000, 1001);
        let old = build_workbook(grid.clone());
        let new = build_workbook(grid);
        let estimated = estimate_diff_cell_volume(&old, &new);
        assert!(estimated >= AUTO_STREAM_CELL_THRESHOLD);

        let config = DiffConfig::balanced();
        let mode = if should_use_large_mode(estimated, &config) {
            DiffMode::Large
        } else {
            DiffMode::Payload
        };
        assert_eq!(mode, DiffMode::Large);
    }
}
