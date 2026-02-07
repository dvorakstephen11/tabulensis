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

use excel_diff::{CellValue, WorkbookPackage};

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
    let runner = DiffRunner::new(
        paths.store_db_path.clone(),
        "test".to_string(),
        "test".to_string(),
    );
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

    // Export is a user-visible artifact: validate schema shape, not byte-for-byte determinism.
    let export_pkg = WorkbookPackage::open(fs::File::open(&export_path).expect("open export"))
        .expect("audit export should be a valid workbook");
    let sheet_names = excel_diff::with_default_session(|session| {
        export_pkg
            .workbook
            .sheets
            .iter()
            .map(|sheet| session.strings.resolve(sheet.name).to_string())
            .collect::<Vec<_>>()
    });
    for expected in [
        "Summary",
        "Warnings",
        "Cells",
        "Structure",
        "PowerQuery",
        "Model",
        "OtherOps",
    ] {
        assert!(
            sheet_names.iter().any(|name| name == expected),
            "export should include sheet '{expected}', got {sheet_names:?}"
        );
    }

    excel_diff::with_default_session(|session| {
        let cells_sheet = export_pkg
            .workbook
            .sheets
            .iter()
            .find(|sheet| session.strings.resolve(sheet.name) == "Cells")
            .expect("Cells sheet should exist");
        let a1 = cells_sheet
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref())
            .expect("Cells!A1 should exist");
        match a1 {
            CellValue::Text(id) => assert_eq!(session.strings.resolve(*id), "Sheet"),
            other => panic!("expected Cells!A1 to be text 'Sheet', got {other:?}"),
        }
    });

    let results = backend
        .search_diff_ops(&diff_id, "2", 20)
        .unwrap_or_else(|err| panic!("search diff ops failed: {}", err.message));
    assert!(
        results.iter().any(|r| {
            r.kind == "cell"
                && r.sheet.as_deref() == Some("Sheet1")
                && r.address.as_deref() == Some("C3")
        }),
        "expected to find the C3 cell change in search results; got {results:?}"
    );
}

#[test]
fn build_and_search_index_smoke() {
    let temp = TempDir::new("backend-index");
    let backend = build_backend(&temp);
    let fixtures = fixtures_dir();
    let workbook = fixtures.join("minimal.xlsx");

    let index = backend
        .build_search_index(&workbook, "old")
        .unwrap_or_else(|err| panic!("build index failed: {}", err.message));
    let results = backend
        .search_workbook_index(&index.index_id, "R1C1", 20)
        .unwrap_or_else(|err| panic!("search index failed: {}", err.message));
    assert!(
        results
            .iter()
            .any(|r| r.kind == "value" && r.sheet == "Sheet1" && r.address == "A1"),
        "expected to find Sheet1!A1 value hit for R1C1; got {results:?}"
    );
}

#[test]
fn search_index_finds_formulas() {
    let temp = TempDir::new("backend-index-formula");
    let backend = build_backend(&temp);
    let fixtures = fixtures_dir();
    let workbook = fixtures.join("pg3_value_and_formula_cells.xlsx");

    let index = backend
        .build_search_index(&workbook, "old")
        .unwrap_or_else(|err| panic!("build index failed: {}", err.message));
    let results = backend
        .search_workbook_index(&index.index_id, "world", 50)
        .unwrap_or_else(|err| panic!("search index failed: {}", err.message));
    assert!(
        results.iter().any(|r| r.kind == "formula"),
        "expected at least one formula hit for pg3 workbook; got {results:?}"
    );
}

#[test]
fn search_index_finds_power_query_text() {
    let temp = TempDir::new("backend-index-query");
    let backend = build_backend(&temp);
    let fixtures = fixtures_dir();
    let workbook = fixtures.join("m_embedded_change_a.xlsx");

    let index = backend
        .build_search_index(&workbook, "old")
        .unwrap_or_else(|err| panic!("build index failed: {}", err.message));
    let results = backend
        .search_workbook_index(&index.index_id, "EmbeddedQuery", 50)
        .unwrap_or_else(|err| panic!("search index failed: {}", err.message));
    assert!(
        results.iter().any(|r| r.kind == "query"),
        "expected at least one query hit; got {results:?}"
    );
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
