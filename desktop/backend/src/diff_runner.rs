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
use serde::Serialize;

use crate::events::{ProgressEvent, ProgressTx};

#[cfg(test)]
use std::collections::hash_map::Entry;
#[cfg(test)]
use std::collections::HashMap;

#[cfg(all(not(feature = "custom-lru"), not(feature = "lru-crate")))]
compile_error!("Enable feature `lru-crate` or `custom-lru` for desktop_backend.");

#[cfg(feature = "custom-lru")]
use crate::tiny_lru::TinyLruCache as LruCache;
#[cfg(all(not(feature = "custom-lru"), feature = "lru-crate"))]
use lru::LruCache;

use crate::export::export_audit_xlsx_from_store;
use crate::store::{
    resolve_sheet_stats, DiffMode, DiffRunSummary, OpStore, OpStoreSink, RunStatus, SheetStats,
    StoreError,
};
use ui_payload::{build_payload_from_pbix_report, limits_from_config, DiffOptions, DiffOutcomeConfig, DiffPreset};
const WORKBOOK_CACHE_CAPACITY: usize = 4;
const PBIX_CACHE_CAPACITY: usize = 2;
const DIFF_KEY_CACHE_CAPACITY: usize = 16;
const AUTO_STREAM_BYTES_THRESHOLD: u64 = 2_000_000;

#[cfg(feature = "arc-cache")]
type WorkbookHandle = Arc<WorkbookPackage>;
#[cfg(not(feature = "arc-cache"))]
type WorkbookHandle = WorkbookPackage;

#[cfg(feature = "arc-cache")]
type PbixHandle = Arc<PbixPackage>;
#[cfg(not(feature = "arc-cache"))]
type PbixHandle = PbixPackage;

#[cfg(feature = "arc-cache")]
fn wrap_workbook(pkg: WorkbookPackage) -> WorkbookHandle {
    Arc::new(pkg)
}

#[cfg(not(feature = "arc-cache"))]
fn wrap_workbook(pkg: WorkbookPackage) -> WorkbookHandle {
    pkg
}

#[cfg(feature = "arc-cache")]
fn wrap_pbix(pkg: PbixPackage) -> PbixHandle {
    Arc::new(pkg)
}

#[cfg(not(feature = "arc-cache"))]
fn wrap_pbix(pkg: PbixPackage) -> PbixHandle {
    pkg
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CacheStats {
    pub workbook_hits: u64,
    pub workbook_misses: u64,
    pub pbix_hits: u64,
    pub pbix_misses: u64,
}

#[derive(Debug, Clone)]
pub struct DiffRequest {
    pub old_path: String,
    pub new_path: String,
    pub run_id: u64,
    pub options: DiffOptions,
    pub cancel: Arc<AtomicBool>,
    pub progress: ProgressTx,
}

#[derive(Debug, Clone)]
pub struct SheetPayloadRequest {
    pub diff_id: String,
    pub sheet_name: String,
    pub cancel: Arc<AtomicBool>,
    pub progress: ProgressTx,
}

#[derive(Debug, Clone)]
pub struct SheetMetaRequest {
    pub diff_id: String,
    pub sheet_name: String,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Default)]
pub struct RangeBounds {
    pub row_start: Option<u32>,
    pub row_end: Option<u32>,
    pub col_start: Option<u32>,
    pub col_end: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct OpsRangeRequest {
    pub diff_id: String,
    pub sheet_name: String,
    pub range: RangeBounds,
}

#[derive(Debug, Clone)]
pub struct CellsRangeRequest {
    pub diff_id: String,
    pub sheet_name: String,
    pub side: String,
    pub range: RangeBounds,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetPreviewMeta {
    pub truncated_old: bool,
    pub truncated_new: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetMeta {
    pub sheet_name: String,
    pub old_rows: u32,
    pub old_cols: u32,
    pub new_rows: u32,
    pub new_cols: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<ui_payload::SheetAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<SheetPreviewMeta>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetCellsPayload {
    pub sheet_name: String,
    pub side: String,
    pub cells: Vec<ui_payload::SheetCell>,
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
    pub fn new(code: impl Into<String>, message: impl Into<String>, trusted_retry: bool) -> Self {
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

    pub fn load_sheet_meta(
        &self,
        request: SheetMetaRequest,
    ) -> Result<SheetMeta, DiffErrorPayload> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::LoadSheetMeta {
                request,
                respond_to: reply_tx,
            })
            .map_err(|e| DiffErrorPayload::new("engine_down", e.to_string(), false))?;
        reply_rx.recv().map_err(|e| {
            DiffErrorPayload::new("engine_down", e.to_string(), false)
        })?
    }

    pub fn load_ops_in_range(
        &self,
        request: OpsRangeRequest,
    ) -> Result<DiffReport, DiffErrorPayload> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::LoadOpsRange {
                request,
                respond_to: reply_tx,
            })
            .map_err(|e| DiffErrorPayload::new("engine_down", e.to_string(), false))?;
        reply_rx.recv().map_err(|e| {
            DiffErrorPayload::new("engine_down", e.to_string(), false)
        })?
    }

    pub fn load_cells_in_range(
        &self,
        request: CellsRangeRequest,
    ) -> Result<SheetCellsPayload, DiffErrorPayload> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::LoadCellsRange {
                request,
                respond_to: reply_tx,
            })
            .map_err(|e| DiffErrorPayload::new("engine_down", e.to_string(), false))?;
        reply_rx.recv().map_err(|e| {
            DiffErrorPayload::new("engine_down", e.to_string(), false)
        })?
    }

    pub fn cache_stats(&self) -> Result<CacheStats, DiffErrorPayload> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::CacheStats { respond_to: reply_tx })
            .map_err(|e| DiffErrorPayload::new("engine_down", e.to_string(), false))?;
        reply_rx.recv().map_err(|e| {
            DiffErrorPayload::new("engine_down", e.to_string(), false)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffCacheKind {
    Workbook,
    Pbix,
}

#[derive(Debug, Clone)]
struct DiffKeyEntry {
    kind: DiffCacheKind,
    old_key: CacheKey,
    new_key: CacheKey,
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
    LoadSheetMeta {
        request: SheetMetaRequest,
        respond_to: Sender<Result<SheetMeta, DiffErrorPayload>>,
    },
    LoadOpsRange {
        request: OpsRangeRequest,
        respond_to: Sender<Result<DiffReport, DiffErrorPayload>>,
    },
    LoadCellsRange {
        request: CellsRangeRequest,
        respond_to: Sender<Result<SheetCellsPayload, DiffErrorPayload>>,
    },
    CacheStats {
        respond_to: Sender<CacheStats>,
    },
}

struct EngineState {
    store_path: PathBuf,
    app_version: String,
    engine_version: String,
    workbook_cache: LruCache<CacheKey, WorkbookHandle>,
    pbix_cache: LruCache<CacheKey, PbixHandle>,
    diff_key_cache: LruCache<String, DiffKeyEntry>,
    workbook_cache_hits: u64,
    workbook_cache_misses: u64,
    pbix_cache_hits: u64,
    pbix_cache_misses: u64,
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
            diff_key_cache: LruCache::new(NonZeroUsize::new(DIFF_KEY_CACHE_CAPACITY).unwrap()),
            workbook_cache_hits: 0,
            workbook_cache_misses: 0,
            pbix_cache_hits: 0,
            pbix_cache_misses: 0,
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
                EngineCommand::LoadSheetMeta { request, respond_to } => {
                    let result = self.handle_load_sheet_meta(request);
                    let _ = respond_to.send(result);
                }
                EngineCommand::LoadOpsRange { request, respond_to } => {
                    let result = self.handle_load_ops_range(request);
                    let _ = respond_to.send(result);
                }
                EngineCommand::LoadCellsRange { request, respond_to } => {
                    let result = self.handle_load_cells_range(request);
                    let _ = respond_to.send(result);
                }
                EngineCommand::CacheStats { respond_to } => {
                    let _ = respond_to.send(self.cache_stats());
                }
            }
        }
    }

    fn cache_stats(&self) -> CacheStats {
        CacheStats {
            workbook_hits: self.workbook_cache_hits,
            workbook_misses: self.workbook_cache_misses,
            pbix_hits: self.pbix_cache_hits,
            pbix_misses: self.pbix_cache_misses,
        }
    }

    fn diff_key_entry(&mut self, diff_id: &String, kind: DiffCacheKind) -> Option<DiffKeyEntry> {
        self.diff_key_cache.get(diff_id).and_then(|entry| {
            if entry.kind == kind {
                Some(entry.clone())
            } else {
                None
            }
        })
    }

    fn store_diff_keys(
        &mut self,
        diff_id: &String,
        kind: DiffCacheKind,
        old_key: CacheKey,
        new_key: CacheKey,
    ) {
        self.diff_key_cache.put(
            diff_id.clone(),
            DiffKeyEntry {
                kind,
                old_key,
                new_key,
            },
        );
    }

    fn load_workbooks_for_diff(
        &mut self,
        diff_id: &String,
        old_path: &Path,
        new_path: &Path,
        trusted: bool,
    ) -> Result<(WorkbookHandle, WorkbookHandle), DiffErrorPayload> {
        if let Some(entry) = self.diff_key_entry(diff_id, DiffCacheKind::Workbook) {
            let old_pkg = self.get_or_open_workbook_by_key(&entry.old_key, old_path, trusted)?;
            let new_pkg = self.get_or_open_workbook_by_key(&entry.new_key, new_path, trusted)?;
            return Ok((old_pkg, new_pkg));
        }

        let (old_key, old_pkg) = self.open_workbook_cached_with_key(old_path, trusted)?;
        let (new_key, new_pkg) = self.open_workbook_cached_with_key(new_path, trusted)?;
        self.store_diff_keys(diff_id, DiffCacheKind::Workbook, old_key, new_key);
        Ok((old_pkg, new_pkg))
    }

    fn handle_diff(&mut self, request: DiffRequest) -> Result<DiffOutcome, DiffErrorPayload> {
        emit_progress(&request.progress, request.run_id, "read", "Reading files...");

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
                let old_key = cache_key(&old_path, trusted)?;
                let new_key = cache_key(&new_path, trusted)?;

                // Avoid paying full workbook parse cost just to decide mode.
                let mode = if old_key.size.max(new_key.size) >= AUTO_STREAM_BYTES_THRESHOLD {
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
                self.store_diff_keys(&diff_id, DiffCacheKind::Workbook, old_key.clone(), new_key.clone());

                match mode {
                    DiffMode::Payload => {
                        let old_pkg = self.get_or_open_workbook_by_key(&old_key, &old_path, trusted)?;
                        let new_pkg = self.get_or_open_workbook_by_key(&new_key, &new_path, trusted)?;

                        emit_progress(&request.progress, request.run_id, "diff", "Diffing workbooks...");
                        let progress = EngineProgress::new(request.progress.clone(), request.run_id, request.cancel.clone());
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

                        emit_progress(&request.progress, request.run_id, "snapshot", "Building previews...");
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
                        emit_progress(&request.progress, request.run_id, "diff", "Streaming diff to disk...");
                        let progress = EngineProgress::new(request.progress.clone(), request.run_id, request.cancel.clone());
                        let sink_store = OpStore::open(&self.store_path).map_err(map_store_error)?;
                        let conn = sink_store.into_connection();
                        let mut sink = OpStoreSink::new(conn, diff_id.clone())
                            .map_err(|e| DiffErrorPayload::new("store", e.to_string(), false))?;

                        let old_file = File::open(&old_path).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;
                        let new_file = File::open(&new_path).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;

                        let summary = match run_diff_with_progress(
                            || {
                                if trusted {
                                    WorkbookPackage::diff_openxml_streaming_fast_with_limits_and_progress(
                                        old_file,
                                        new_file,
                                        trusted_limits(),
                                        &config,
                                        &mut sink,
                                        &progress,
                                    )
                                } else {
                                    WorkbookPackage::diff_openxml_streaming_fast_with_progress(
                                        old_file,
                                        new_file,
                                        &config,
                                        &mut sink,
                                        &progress,
                                    )
                                }
                            },
                            &request.cancel,
                        ) {
                            Ok(result) => result.map_err(diff_error_from_openxml),
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
                let (old_key, old_pkg) = self.open_pbix_cached_with_key(&old_path, trusted)?;
                let (new_key, new_pkg) = self.open_pbix_cached_with_key(&new_path, trusted)?;

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
                self.store_diff_keys(&diff_id, DiffCacheKind::Pbix, old_key, new_key);

                emit_progress(&request.progress, request.run_id, "diff", "Streaming PBIX diff to disk...");
                let progress = EngineProgress::new(request.progress.clone(), request.run_id, request.cancel.clone());
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
        let (old_pkg, new_pkg) =
            self.load_workbooks_for_diff(&request.diff_id, &old_path, &new_path, summary.trusted)?;

        let ops = store.load_sheet_ops(&request.diff_id, &request.sheet_name).map_err(map_store_error)?;
        let strings = store.load_strings(&request.diff_id).map_err(map_store_error)?;

        let mut report = DiffReport::new(ops);
        report.strings = strings;
        report.complete = summary.complete;
        report.warnings = summary.warnings.clone();

        emit_progress(&request.progress, 0, "snapshot", "Building previews...");
        Ok(ui_payload::build_payload_from_workbook_report(report, &old_pkg, &new_pkg))
    }

    fn handle_load_sheet_meta(
        &mut self,
        request: SheetMetaRequest,
    ) -> Result<SheetMeta, DiffErrorPayload> {
        let store = OpStore::open(&self.store_path).map_err(map_store_error)?;
        let summary = store.load_summary(&request.diff_id).map_err(map_store_error)?;

        if request.cancel.load(Ordering::Relaxed) {
            return Err(DiffErrorPayload::new("canceled", "Diff canceled.", false));
        }

        let old_path = PathBuf::from(&summary.old_path);
        let new_path = PathBuf::from(&summary.new_path);
        let (old_pkg, new_pkg) =
            self.load_workbooks_for_diff(&request.diff_id, &old_path, &new_path, summary.trusted)?;

        let ops = store
            .load_sheet_ops(&request.diff_id, &request.sheet_name)
            .map_err(map_store_error)?;
        let strings = store.load_strings(&request.diff_id).map_err(map_store_error)?;
        let mut report = DiffReport::new(ops.clone());
        report.strings = strings;
        report.complete = summary.complete;
        report.warnings = summary.warnings.clone();

        let old_sheet_name = resolve_old_sheet_name(&report, &request.sheet_name)
            .unwrap_or_else(|| request.sheet_name.clone());

        let old_sheet = find_sheet_by_name(&old_pkg.workbook, &old_sheet_name);
        let new_sheet = find_sheet_by_name(&new_pkg.workbook, &request.sheet_name);
        let old_rows = old_sheet.map(|s| s.grid.nrows).unwrap_or(0);
        let old_cols = old_sheet.map(|s| s.grid.ncols).unwrap_or(0);
        let new_rows = new_sheet.map(|s| s.grid.nrows).unwrap_or(0);
        let new_cols = new_sheet.map(|s| s.grid.ncols).unwrap_or(0);

        let alignment = if ops.is_empty() {
            None
        } else {
            Some(ui_payload::build_alignment_for_sheet(
                &request.sheet_name,
                if old_rows > 0 && old_cols > 0 { Some((old_rows, old_cols)) } else { None },
                if new_rows > 0 && new_cols > 0 { Some((new_rows, new_cols)) } else { None },
                &ops,
            ))
        };

        let truncated_old = old_sheet
            .map(|s| s.grid.cell_count() > ui_payload::MAX_SNAPSHOT_CELLS_PER_SHEET)
            .unwrap_or(false);
        let truncated_new = new_sheet
            .map(|s| s.grid.cell_count() > ui_payload::MAX_SNAPSHOT_CELLS_PER_SHEET)
            .unwrap_or(false);
        let preview = if truncated_old || truncated_new {
            Some(SheetPreviewMeta {
                truncated_old,
                truncated_new,
                note: Some("Preview likely limited for large sheets.".to_string()),
            })
        } else {
            None
        };

        Ok(SheetMeta {
            sheet_name: request.sheet_name,
            old_rows,
            old_cols,
            new_rows,
            new_cols,
            alignment,
            preview,
        })
    }

    fn handle_load_ops_range(
        &mut self,
        request: OpsRangeRequest,
    ) -> Result<DiffReport, DiffErrorPayload> {
        let store = OpStore::open(&self.store_path).map_err(map_store_error)?;
        let summary = store.load_summary(&request.diff_id).map_err(map_store_error)?;
        let ops = store
            .load_ops_in_range(
                &request.diff_id,
                &request.sheet_name,
                request.range.row_start,
                request.range.row_end,
                request.range.col_start,
                request.range.col_end,
            )
            .map_err(map_store_error)?;
        let strings = store.load_strings(&request.diff_id).map_err(map_store_error)?;
        let mut report = DiffReport::new(ops);
        report.strings = strings;
        report.complete = summary.complete;
        report.warnings = summary.warnings.clone();
        Ok(report)
    }

    fn handle_load_cells_range(
        &mut self,
        request: CellsRangeRequest,
    ) -> Result<SheetCellsPayload, DiffErrorPayload> {
        let store = OpStore::open(&self.store_path).map_err(map_store_error)?;
        let summary = store.load_summary(&request.diff_id).map_err(map_store_error)?;

        let old_path = PathBuf::from(&summary.old_path);
        let new_path = PathBuf::from(&summary.new_path);
        let (old_pkg, new_pkg) =
            self.load_workbooks_for_diff(&request.diff_id, &old_path, &new_path, summary.trusted)?;

        let side = request.side.to_ascii_lowercase();
        let sheet = if side == "old" {
            find_sheet_by_name(&old_pkg.workbook, &request.sheet_name)
        } else {
            find_sheet_by_name(&new_pkg.workbook, &request.sheet_name)
        };

        let Some(sheet) = sheet else {
            return Ok(SheetCellsPayload {
                sheet_name: request.sheet_name,
                side,
                cells: Vec::new(),
            });
        };

        let Some((row_start, row_end, col_start, col_end)) =
            normalize_range(&request.range, sheet.grid.nrows, sheet.grid.ncols)
        else {
            return Ok(SheetCellsPayload {
                sheet_name: request.sheet_name,
                side,
                cells: Vec::new(),
            });
        };

        let cells = excel_diff::with_default_session(|session| {
            let mut out = Vec::new();
            for ((row, col), cell) in sheet.grid.iter_cells() {
                if row < row_start || row > row_end || col < col_start || col > col_end {
                    continue;
                }
                out.push(render_sheet_cell(&session.strings, row, col, cell));
            }
            out
        });

        Ok(SheetCellsPayload {
            sheet_name: request.sheet_name,
            side,
            cells,
        })
    }

    fn get_workbook_cached_by_key(&mut self, key: &CacheKey) -> Option<WorkbookHandle> {
        if let Some(pkg) = self.workbook_cache.get(key) {
            self.workbook_cache_hits += 1;
            return Some(pkg.clone());
        }
        self.workbook_cache_misses += 1;
        None
    }

    fn open_workbook_from_key(
        &mut self,
        key: CacheKey,
        path: &Path,
        trusted: bool,
    ) -> Result<WorkbookHandle, DiffErrorPayload> {
        let file = File::open(path).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;
        let pkg = if trusted {
            WorkbookPackage::open_with_limits(file, trusted_limits())
                .map_err(map_package_error)?
        } else {
            WorkbookPackage::open(file)
                .map_err(map_package_error)?
        };
        let pkg = wrap_workbook(pkg);
        self.workbook_cache.put(key, pkg.clone());
        Ok(pkg)
    }

    fn get_or_open_workbook_by_key(
        &mut self,
        key: &CacheKey,
        path: &Path,
        trusted: bool,
    ) -> Result<WorkbookHandle, DiffErrorPayload> {
        if let Some(pkg) = self.get_workbook_cached_by_key(key) {
            return Ok(pkg);
        }
        self.open_workbook_from_key(key.clone(), path, trusted)
    }

    fn open_workbook_cached_with_key(
        &mut self,
        path: &Path,
        trusted: bool,
    ) -> Result<(CacheKey, WorkbookHandle), DiffErrorPayload> {
        let key = cache_key(path, trusted)?;
        let pkg = self.get_or_open_workbook_by_key(&key, path, trusted)?;
        Ok((key, pkg))
    }

    fn get_pbix_cached_by_key(&mut self, key: &CacheKey) -> Option<PbixHandle> {
        if let Some(pkg) = self.pbix_cache.get(key) {
            self.pbix_cache_hits += 1;
            return Some(pkg.clone());
        }
        self.pbix_cache_misses += 1;
        None
    }

    fn open_pbix_from_key(
        &mut self,
        key: CacheKey,
        path: &Path,
        trusted: bool,
    ) -> Result<PbixHandle, DiffErrorPayload> {
        let file = File::open(path).map_err(|e| DiffErrorPayload::new("io", e.to_string(), false))?;
        let pkg = if trusted {
            PbixPackage::open_with_limits(file, trusted_limits())
                .map_err(map_package_error)?
        } else {
            PbixPackage::open(file)
                .map_err(map_package_error)?
        };
        let pkg = wrap_pbix(pkg);
        self.pbix_cache.put(key, pkg.clone());
        Ok(pkg)
    }

    fn get_or_open_pbix_by_key(
        &mut self,
        key: &CacheKey,
        path: &Path,
        trusted: bool,
    ) -> Result<PbixHandle, DiffErrorPayload> {
        if let Some(pkg) = self.get_pbix_cached_by_key(key) {
            return Ok(pkg);
        }
        self.open_pbix_from_key(key.clone(), path, trusted)
    }

    fn open_pbix_cached_with_key(
        &mut self,
        path: &Path,
        trusted: bool,
    ) -> Result<(CacheKey, PbixHandle), DiffErrorPayload> {
        let key = cache_key(path, trusted)?;
        let pkg = self.get_or_open_pbix_by_key(&key, path, trusted)?;
        Ok((key, pkg))
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

fn resolve_old_sheet_name(report: &DiffReport, sheet_name: &str) -> Option<String> {
    for op in &report.ops {
        let excel_diff::DiffOp::SheetRenamed { sheet, from, .. } = op else {
            continue;
        };
        let new_name = report.resolve(*sheet).unwrap_or("");
        if new_name.eq_ignore_ascii_case(sheet_name) {
            return Some(report.resolve(*from).unwrap_or("").to_string());
        }
    }
    None
}

fn find_sheet_by_name<'a>(
    workbook: &'a excel_diff::Workbook,
    sheet_name: &str,
) -> Option<&'a excel_diff::Sheet> {
    let idx = excel_diff::with_default_session(|session| {
        workbook
            .sheets
            .iter()
            .position(|sheet| session.strings.resolve(sheet.name).eq_ignore_ascii_case(sheet_name))
    });
    idx.and_then(|i| workbook.sheets.get(i))
}

fn normalize_range(
    range: &RangeBounds,
    nrows: u32,
    ncols: u32,
) -> Option<(u32, u32, u32, u32)> {
    if nrows == 0 || ncols == 0 {
        return None;
    }
    let mut row_start = range.row_start.unwrap_or(0);
    let mut row_end = range.row_end.unwrap_or(nrows.saturating_sub(1));
    let mut col_start = range.col_start.unwrap_or(0);
    let mut col_end = range.col_end.unwrap_or(ncols.saturating_sub(1));

    row_start = row_start.min(nrows.saturating_sub(1));
    row_end = row_end.min(nrows.saturating_sub(1));
    col_start = col_start.min(ncols.saturating_sub(1));
    col_end = col_end.min(ncols.saturating_sub(1));

    if row_end < row_start {
        std::mem::swap(&mut row_start, &mut row_end);
    }
    if col_end < col_start {
        std::mem::swap(&mut col_start, &mut col_end);
    }

    Some((row_start, row_end, col_start, col_end))
}

fn render_sheet_cell(
    pool: &excel_diff::StringPool,
    row: u32,
    col: u32,
    cell: &excel_diff::Cell,
) -> ui_payload::SheetCell {
    let value = match &cell.value {
        None => None,
        Some(excel_diff::CellValue::Blank) => Some(String::new()),
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(id)) => Some(pool.resolve(*id).to_string()),
        Some(excel_diff::CellValue::Bool(b)) => Some(if *b { "TRUE".to_string() } else { "FALSE".to_string() }),
        Some(excel_diff::CellValue::Error(id)) => Some(pool.resolve(*id).to_string()),
    };
    let formula = cell.formula.map(|id| format!("={}", pool.resolve(id)));
    ui_payload::SheetCell { row, col, value, formula }
}

#[cfg(test)]
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

fn diff_error_from_openxml(err: excel_diff::OpenXmlDiffError) -> DiffErrorPayload {
    match err {
        excel_diff::OpenXmlDiffError::Package(err) => map_package_error(err),
        excel_diff::OpenXmlDiffError::Diff(err) => diff_error_from_diff(err),
        other => DiffErrorPayload::new("openxml", other.to_string(), false),
    }
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
    progress: ProgressTx,
    run_id: u64,
    cancel: Arc<AtomicBool>,
    last_phase: std::sync::Mutex<Option<String>>,
}

impl EngineProgress {
    fn new(progress: ProgressTx, run_id: u64, cancel: Arc<AtomicBool>) -> Self {
        Self {
            progress,
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
            emit_progress(&self.progress, self.run_id, "diff", Self::map_detail(phase));
        }
    }
}

fn emit_progress(progress: &ProgressTx, run_id: u64, stage: &str, detail: &str) {
    let _ = progress.send(ProgressEvent {
        run_id,
        stage: stage.to_string(),
        detail: detail.to_string(),
    });
}

#[allow(dead_code)]
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
                workbook_sheet_id: None,
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

    #[cfg(feature = "arc-cache")]
    #[test]
    fn open_workbook_cached_reuses_arc_on_hit() {
        let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/generated");
        let path = fixtures.join("single_cell_value_a.xlsx");
        let (_tx, rx) = std::sync::mpsc::channel();
        let mut engine = super::EngineState::new(
            std::path::PathBuf::new(),
            "test".to_string(),
            "test".to_string(),
            rx,
        );

        let first = engine
            .open_workbook_cached_with_key(&path, true)
            .expect("open first workbook")
            .1;
        let second = engine
            .open_workbook_cached_with_key(&path, true)
            .expect("open second workbook")
            .1;

        assert!(std::sync::Arc::ptr_eq(&first, &second));
    }
}
