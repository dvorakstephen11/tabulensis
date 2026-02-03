use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use desktop_backend::{
    BackendPaths, CellsRangeRequest, DesktopBackend, DiffRequest, DiffRunner, RangeBounds,
    SheetMetaRequest, SheetPayloadRequest,
};
use ui_payload::{DiffOptions, DiffPreset};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tabulensis-{prefix}-{stamp}"));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/generated")
}

fn build_backend(temp: &TempDir) -> DesktopBackend {
    let paths = BackendPaths {
        app_data_dir: temp.path.clone(),
        store_db_path: temp.path.join("diff_store.sqlite"),
        recents_json_path: temp.path.join("recents.json"),
    };
    let runner = DiffRunner::new(paths.store_db_path.clone(), "test".to_string(), "test".to_string());
    DesktopBackend { paths, runner }
}

#[test]
fn diff_export_and_search_smoke() {
    let temp = TempDir::new("backend");
    let backend = build_backend(&temp);
    let fixtures = fixtures_dir();
    let old_path = fixtures.join("single_cell_value_a.xlsx");
    let new_path = fixtures.join("single_cell_value_b.xlsx");

    let (progress_tx, _progress_rx) = DesktopBackend::new_progress_channel();
    let request = DiffRequest {
        old_path: old_path.display().to_string(),
        new_path: new_path.display().to_string(),
        run_id: 1,
        options: DiffOptions {
            preset: Some(DiffPreset::Balanced),
            trusted: Some(true),
            ..DiffOptions::default()
        },
        cancel: Arc::new(AtomicBool::new(false)),
        progress: progress_tx,
    };

    let outcome = backend
        .runner
        .diff(request)
        .unwrap_or_else(|err| panic!("diff failed: {}", err.message));
    let diff_id = outcome.diff_id.clone();
    let summary = outcome
        .summary
        .unwrap_or_else(|| panic!("diff summary missing for {}", diff_id));
    assert_eq!(summary.diff_id, diff_id);

    let loaded = backend
        .load_diff_summary(&diff_id)
        .unwrap_or_else(|err| panic!("load summary failed: {}", err.message));
    assert_eq!(loaded.diff_id, diff_id);

    let export_path = temp.path.join("audit.xlsx");
    backend
        .export_audit_xlsx_to_path(&diff_id, &export_path)
        .unwrap_or_else(|err| panic!("export failed: {}", err.message));
    assert!(export_path.is_file());

    let _ = backend
        .search_diff_ops(&diff_id, "sheet", 5)
        .unwrap_or_else(|err| panic!("search diff ops failed: {}", err.message));
}

#[test]
fn build_and_search_index_smoke() {
    let temp = TempDir::new("backend-index");
    let backend = build_backend(&temp);
    let fixtures = fixtures_dir();
    let workbook = fixtures.join("single_cell_value_a.xlsx");

    let index = backend
        .build_search_index(&workbook, "old")
        .unwrap_or_else(|err| panic!("build index failed: {}", err.message));
    let results = backend
        .search_workbook_index(&index.index_id, "A1", 5)
        .unwrap_or_else(|err| panic!("search index failed: {}", err.message));
    assert!(results.len() <= 5);
}

#[test]
#[ignore]
fn cache_hit_loop_smoke() {
    let temp = TempDir::new("backend-cache-hit");
    let backend = build_backend(&temp);
    let fixtures = fixtures_dir();
    let old_path = fixtures.join("single_cell_value_a.xlsx");
    let new_path = fixtures.join("single_cell_value_b.xlsx");

    let (progress_tx, _progress_rx) = DesktopBackend::new_progress_channel();
    let request = DiffRequest {
        old_path: old_path.display().to_string(),
        new_path: new_path.display().to_string(),
        run_id: 2,
        options: DiffOptions {
            preset: Some(DiffPreset::Balanced),
            trusted: Some(true),
            ..DiffOptions::default()
        },
        cancel: Arc::new(AtomicBool::new(false)),
        progress: progress_tx,
    };

    let outcome = backend
        .runner
        .diff(request)
        .unwrap_or_else(|err| panic!("diff failed: {}", err.message));
    let diff_id = outcome.diff_id.clone();
    let payload = outcome
        .payload
        .unwrap_or_else(|| panic!("diff payload missing for {}", diff_id));
    let sheet_name = payload
        .sheets
        .new
        .sheets
        .first()
        .or_else(|| payload.sheets.old.sheets.first())
        .map(|sheet| sheet.name.clone())
        .unwrap_or_else(|| "Sheet1".to_string());

    let stats_before = backend
        .runner
        .cache_stats()
        .unwrap_or_else(|err| panic!("cache stats failed: {}", err.message));

    let cancel = Arc::new(AtomicBool::new(false));
    let (progress_tx, _progress_rx) = DesktopBackend::new_progress_channel();
    let range = RangeBounds {
        row_start: Some(0),
        row_end: Some(20),
        col_start: Some(0),
        col_end: Some(20),
    };

    let iterations = 50u64;
    let start = Instant::now();
    for _ in 0..iterations {
        backend
            .runner
            .load_sheet_meta(SheetMetaRequest {
                diff_id: diff_id.clone(),
                sheet_name: sheet_name.clone(),
                cancel: cancel.clone(),
            })
            .unwrap_or_else(|err| panic!("load sheet meta failed: {}", err.message));

        backend
            .runner
            .load_cells_in_range(CellsRangeRequest {
                diff_id: diff_id.clone(),
                sheet_name: sheet_name.clone(),
                side: "old".to_string(),
                range: range.clone(),
            })
            .unwrap_or_else(|err| panic!("load cells range failed: {}", err.message));

        backend
            .runner
            .load_sheet_payload(SheetPayloadRequest {
                diff_id: diff_id.clone(),
                sheet_name: sheet_name.clone(),
                cancel: cancel.clone(),
                progress: progress_tx.clone(),
            })
            .unwrap_or_else(|err| panic!("load sheet payload failed: {}", err.message));
    }
    let elapsed = start.elapsed();

    let stats_after = backend
        .runner
        .cache_stats()
        .unwrap_or_else(|err| panic!("cache stats failed: {}", err.message));

    println!(
        "cache_hit_loop iterations={} elapsed_ms={} workbook_hits_delta={} workbook_misses_delta={} pbix_hits_delta={} pbix_misses_delta={}",
        iterations,
        elapsed.as_millis(),
        stats_after.workbook_hits - stats_before.workbook_hits,
        stats_after.workbook_misses - stats_before.workbook_misses,
        stats_after.pbix_hits - stats_before.pbix_hits,
        stats_after.pbix_misses - stats_before.pbix_misses
    );
    println!(
        "PERF_METRIC desktop_backend_cache_hit_loop_elapsed_ms={}",
        elapsed.as_millis()
    );
}
