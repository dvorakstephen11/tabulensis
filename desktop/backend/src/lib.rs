mod batch;
mod diff_runner;
mod events;
mod export;
mod paths;
mod recents;
mod search;
mod store;
#[cfg(feature = "custom-lru")]
mod tiny_lru;

use std::path::Path;

pub use batch::{BatchOutcome, BatchRequest};
pub use diff_runner::CacheStats;
pub use diff_runner::{
    CellsRangeRequest, DiffErrorPayload, DiffOutcome, DiffRequest, DiffRunner, OpsRangeRequest,
    RangeBounds, SheetCellsPayload, SheetMeta, SheetMetaRequest, SheetPayloadRequest,
};
pub use events::{ProgressEvent, ProgressRx, ProgressTx};
pub use paths::BackendPaths;
pub use recents::RecentComparison;
pub use search::{SearchIndexResult, SearchIndexSummary, SearchResult};
pub use store::{resolve_sheet_stats, DiffMode, DiffRunSummary, OpStore, RunStatus, StoreError};
pub use ui_payload::{DetailsPayload, DiffAnalysis, NavigatorModel, NoiseFilters, SelectionTarget};

pub struct BackendConfig {
    pub app_name: String,
    pub app_version: String,
    pub engine_version: String,
}

#[derive(Clone)]
pub struct DesktopBackend {
    pub paths: BackendPaths,
    pub runner: DiffRunner,
}

impl DesktopBackend {
    pub fn init(cfg: BackendConfig) -> Result<Self, DiffErrorPayload> {
        let paths = paths::resolve_paths(&cfg.app_name)?;
        let runner = DiffRunner::new(
            paths.store_db_path.clone(),
            cfg.app_version,
            cfg.engine_version,
        );
        Ok(Self { paths, runner })
    }

    pub fn new_progress_channel() -> (ProgressTx, ProgressRx) {
        crossbeam_channel::unbounded()
    }

    pub fn load_recents(&self) -> Result<Vec<RecentComparison>, DiffErrorPayload> {
        recents::load_recents(&self.paths.recents_json_path)
    }

    pub fn save_recent(
        &self,
        entry: RecentComparison,
    ) -> Result<Vec<RecentComparison>, DiffErrorPayload> {
        recents::save_recent(&self.paths.recents_json_path, entry)
    }

    pub fn load_diff_summary(&self, diff_id: &str) -> Result<DiffRunSummary, DiffErrorPayload> {
        let store = OpStore::open(&self.paths.store_db_path).map_err(map_store_error)?;
        store.load_summary(diff_id).map_err(map_store_error)
    }

    pub fn load_diff_analysis(
        &self,
        diff_id: &str,
        filters: NoiseFilters,
    ) -> Result<DiffAnalysis, DiffErrorPayload> {
        let store = OpStore::open(&self.paths.store_db_path).map_err(map_store_error)?;
        store
            .load_diff_analysis(diff_id, filters)
            .map_err(map_store_error)
    }

    pub fn load_pbip_navigator(&self, diff_id: &str) -> Result<NavigatorModel, DiffErrorPayload> {
        let store = OpStore::open(&self.paths.store_db_path).map_err(map_store_error)?;
        store.load_pbip_navigator(diff_id).map_err(map_store_error)
    }

    pub fn load_pbip_details(
        &self,
        diff_id: &str,
        target: &SelectionTarget,
    ) -> Result<DetailsPayload, DiffErrorPayload> {
        let store = OpStore::open(&self.paths.store_db_path).map_err(map_store_error)?;
        store
            .load_pbip_details(diff_id, target)
            .map_err(map_store_error)
    }

    pub fn load_ops_by_kinds(
        &self,
        diff_id: &str,
        kinds: &[&str],
    ) -> Result<Vec<excel_diff::DiffOp>, DiffErrorPayload> {
        let store = OpStore::open(&self.paths.store_db_path).map_err(map_store_error)?;
        store
            .load_ops_by_kinds(diff_id, kinds)
            .map_err(map_store_error)
    }

    pub fn export_audit_xlsx_to_path(
        &self,
        diff_id: &str,
        path: &Path,
    ) -> Result<(), DiffErrorPayload> {
        let store = OpStore::open(&self.paths.store_db_path).map_err(map_store_error)?;
        export::export_audit_xlsx_from_store(&store, diff_id, path)
            .map_err(|e| DiffErrorPayload::new("export", e.to_string(), false))
    }

    pub fn default_export_name(summary: &DiffRunSummary, prefix: &str, ext: &str) -> String {
        let old = base_name(&summary.old_path);
        let new = base_name(&summary.new_path);
        let date = summary
            .finished_at
            .as_deref()
            .unwrap_or(&summary.started_at)
            .get(0..10)
            .unwrap_or("report");
        format!("tabulensis-{prefix}__{old}__{new}__{date}.{ext}")
    }

    pub fn search_diff_ops(
        &self,
        diff_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, DiffErrorPayload> {
        search::search_diff_ops(&self.paths.store_db_path, diff_id, query, limit)
    }

    pub fn build_search_index(
        &self,
        path: &Path,
        side: &str,
    ) -> Result<SearchIndexSummary, DiffErrorPayload> {
        search::build_search_index(&self.paths.store_db_path, path, side)
    }

    pub fn search_workbook_index(
        &self,
        index_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchIndexResult>, DiffErrorPayload> {
        search::search_workbook_index(&self.paths.store_db_path, index_id, query, limit)
    }

    pub fn run_batch_compare(
        &self,
        request: BatchRequest,
        progress: ProgressTx,
    ) -> Result<BatchOutcome, DiffErrorPayload> {
        batch::run_batch_compare(
            self.runner.clone(),
            &self.paths.store_db_path,
            request,
            progress,
        )
    }

    pub fn load_batch_summary(&self, batch_id: &str) -> Result<BatchOutcome, DiffErrorPayload> {
        batch::load_batch_summary(&self.paths.store_db_path, batch_id)
    }
}

fn base_name(path: &str) -> String {
    let parts: Vec<&str> = path.split(['\\', '/']).collect();
    parts.last().unwrap_or(&path).to_string()
}

fn map_store_error(err: StoreError) -> DiffErrorPayload {
    DiffErrorPayload::new("store", err.to_string(), false)
}
