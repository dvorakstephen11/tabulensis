mod common;

use common::{collect_string_ids, fixture_path};
use excel_diff::{
    CellValue, DataMashup, DiffConfig, DiffError, DiffOp, DiffSink, Grid, JsonLinesSink,
    LimitBehavior, Metadata, PackageParts, PackageXml, PbixPackage, Permissions, SectionDocument,
    Sheet, SheetKind, StringPool, VbaModule, VbaModuleType, Workbook, WorkbookPackage,
    try_diff_grids_database_mode_streaming, try_diff_workbooks_streaming,
};
use serde::Deserialize;
use std::fs::File;

#[derive(Default)]
struct StrictLifecycleSink {
    begin_seen: bool,
    finish_seen: bool,
    finish_calls: usize,
    emit_calls: usize,
}

impl DiffSink for StrictLifecycleSink {
    fn begin(&mut self, _pool: &StringPool) -> Result<(), DiffError> {
        if self.begin_seen {
            return Err(DiffError::SinkError {
                message: "begin called twice".to_string(),
            });
        }
        self.begin_seen = true;
        Ok(())
    }

    fn emit(&mut self, _op: DiffOp) -> Result<(), DiffError> {
        if !self.begin_seen {
            return Err(DiffError::SinkError {
                message: "emit before begin".to_string(),
            });
        }
        if self.finish_seen {
            return Err(DiffError::SinkError {
                message: "emit after finish".to_string(),
            });
        }
        self.emit_calls += 1;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.finish_calls += 1;
        self.finish_seen = true;
        Ok(())
    }
}

struct FailAfterNSink {
    fail_after: usize,
    emit_calls: usize,
    finish_calls: usize,
    finish_seen: bool,
}

impl FailAfterNSink {
    fn new(fail_after: usize) -> Self {
        Self {
            fail_after,
            emit_calls: 0,
            finish_calls: 0,
            finish_seen: false,
        }
    }
}

impl DiffSink for FailAfterNSink {
    fn emit(&mut self, _op: DiffOp) -> Result<(), DiffError> {
        self.emit_calls += 1;
        if self.emit_calls > self.fail_after {
            return Err(DiffError::SinkError {
                message: "intentional emit failure".to_string(),
            });
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.finish_calls += 1;
        self.finish_seen = true;
        Ok(())
    }
}

#[derive(Default)]
struct FrozenPoolSink {
    pool_ptr: Option<*const StringPool>,
    pool_len: Option<usize>,
    finish_calls: usize,
}

impl DiffSink for FrozenPoolSink {
    fn begin(&mut self, pool: &StringPool) -> Result<(), DiffError> {
        self.pool_ptr = Some(pool as *const StringPool);
        self.pool_len = Some(pool.len());
        Ok(())
    }

    fn emit(&mut self, _op: DiffOp) -> Result<(), DiffError> {
        let Some(ptr) = self.pool_ptr else {
            return Err(DiffError::SinkError {
                message: "emit before begin".to_string(),
            });
        };
        let Some(len) = self.pool_len else {
            return Err(DiffError::SinkError {
                message: "missing pool length".to_string(),
            });
        };
        // Safety: the pool pointer is captured from begin and remains valid for the diff.
        let current = unsafe { (&*ptr).len() };
        assert_eq!(
            current, len,
            "string pool length changed after begin ({} -> {})",
            len, current
        );
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.finish_calls += 1;
        Ok(())
    }
}

fn make_workbook(pool: &mut StringPool, values: &[f64]) -> Workbook {
    let mut grid = Grid::new(values.len() as u32, 1);
    for (idx, val) in values.iter().enumerate() {
        grid.insert_cell(idx as u32, 0, Some(CellValue::Number(*val)), None);
    }

    Workbook {
        sheets: vec![Sheet {
            name: pool.intern("Sheet1"),
            kind: SheetKind::Worksheet,
            grid,
        }],
        ..Default::default()
    }
}

fn make_keyed_grid(keys: &[i32], values: &[i32]) -> Grid {
    let rows = keys.len().max(values.len());
    let mut grid = Grid::new(rows as u32, 2);
    for row in 0..rows {
        let key = keys.get(row).copied().unwrap_or_default() as f64;
        let value = values.get(row).copied().unwrap_or_default() as f64;
        grid.insert_cell(row as u32, 0, Some(CellValue::Number(key)), None);
        grid.insert_cell(row as u32, 1, Some(CellValue::Number(value)), None);
    }
    grid
}

fn make_dm(section_source: &str) -> DataMashup {
    DataMashup {
        version: 0,
        package_parts: PackageParts {
            package_xml: PackageXml {
                raw_xml: "<Package/>".to_string(),
            },
            main_section: SectionDocument {
                source: section_source.to_string(),
            },
            embedded_contents: Vec::new(),
        },
        permissions: Permissions::default(),
        metadata: Metadata { formulas: Vec::new() },
        permission_bindings_raw: Vec::new(),
    }
}

fn is_object_op(op: &DiffOp) -> bool {
    matches!(
        op,
        DiffOp::NamedRangeAdded { .. }
            | DiffOp::NamedRangeRemoved { .. }
            | DiffOp::NamedRangeChanged { .. }
            | DiffOp::ChartAdded { .. }
            | DiffOp::ChartRemoved { .. }
            | DiffOp::ChartChanged { .. }
            | DiffOp::VbaModuleAdded { .. }
            | DiffOp::VbaModuleRemoved { .. }
            | DiffOp::VbaModuleChanged { .. }
    )
}

#[test]
fn engine_workbook_streaming_calls_finish_once() {
    let mut pool = StringPool::new();
    let wb_a = make_workbook(&mut pool, &[1.0, 2.0]);
    let wb_b = make_workbook(&mut pool, &[1.0, 3.0]);

    let mut sink = StrictLifecycleSink::default();
    let summary =
        try_diff_workbooks_streaming(&wb_a, &wb_b, &mut pool, &DiffConfig::default(), &mut sink)
            .expect("streaming diff should succeed");

    assert!(sink.begin_seen, "begin should be called");
    assert_eq!(sink.finish_calls, 1, "finish should be called exactly once");
    assert!(sink.finish_seen, "finish should be seen");
    assert!(sink.emit_calls > 0, "expected at least one emit");
    assert_eq!(
        summary.op_count, sink.emit_calls,
        "summary op_count should match emitted ops"
    );
}

#[test]
fn engine_workbook_streaming_finishes_on_emit_error() {
    let mut pool = StringPool::new();
    let wb_a = make_workbook(&mut pool, &[1.0]);
    let wb_b = make_workbook(&mut pool, &[2.0]);

    let mut sink = FailAfterNSink::new(0);
    let result =
        try_diff_workbooks_streaming(&wb_a, &wb_b, &mut pool, &DiffConfig::default(), &mut sink);

    assert!(result.is_err(), "expected sink error");
    assert!(sink.finish_seen, "finish should be called on emit error");
    assert_eq!(sink.finish_calls, 1, "finish should be called once");
}

#[test]
fn engine_workbook_streaming_finishes_on_limit_error() {
    let mut pool = StringPool::new();
    let wb_a = make_workbook(&mut pool, &[1.0, 2.0]);
    let wb_b = make_workbook(&mut pool, &[1.0, 3.0]);

    let config = DiffConfig {
        max_align_rows: 1,
        max_align_cols: 1,
        on_limit_exceeded: LimitBehavior::ReturnError,
        ..DiffConfig::default()
    };

    let mut sink = StrictLifecycleSink::default();
    let result = try_diff_workbooks_streaming(&wb_a, &wb_b, &mut pool, &config, &mut sink);

    assert!(matches!(result, Err(DiffError::LimitsExceeded { .. })));
    assert!(sink.finish_seen, "finish should be called on error");
    assert_eq!(sink.finish_calls, 1, "finish should be called once");
}

#[test]
fn engine_database_streaming_calls_finish_once() {
    let mut pool = StringPool::new();
    let sheet_id = pool.intern("Data");

    let grid_a = make_keyed_grid(&[1, 2], &[10, 20]);
    let grid_b = make_keyed_grid(&[1, 2], &[10, 25]);

    let mut sink = StrictLifecycleSink::default();
    let mut op_count = 0usize;
    let summary = try_diff_grids_database_mode_streaming(
        sheet_id,
        &grid_a,
        &grid_b,
        &[0],
        &mut pool,
        &DiffConfig::default(),
        &mut sink,
        &mut op_count,
    )
    .expect("database streaming diff should succeed");

    assert!(sink.begin_seen, "begin should be called");
    assert_eq!(sink.finish_calls, 1, "finish should be called exactly once");
    assert!(summary.op_count > 0, "expected at least one op");
}

#[test]
fn engine_database_streaming_finishes_on_emit_error() {
    let mut pool = StringPool::new();
    let sheet_id = pool.intern("Data");

    let grid_a = make_keyed_grid(&[1, 2], &[10, 20]);
    let grid_b = make_keyed_grid(&[1, 2], &[10, 25]);

    let mut sink = FailAfterNSink::new(0);
    let mut op_count = 0usize;
    let result = try_diff_grids_database_mode_streaming(
        sheet_id,
        &grid_a,
        &grid_b,
        &[0],
        &mut pool,
        &DiffConfig::default(),
        &mut sink,
        &mut op_count,
    );

    assert!(result.is_err(), "expected sink error");
    assert!(sink.finish_seen, "finish should be called on emit error");
    assert_eq!(sink.finish_calls, 1, "finish should be called once");
}

#[test]
fn pbix_streaming_calls_finish_once() {
    let path_a = fixture_path("pbix_legacy_multi_query_a.pbix");
    let path_b = fixture_path("pbix_legacy_multi_query_b.pbix");
    let pkg_a = PbixPackage::open(File::open(&path_a).expect("fixture should exist"))
        .expect("pbix A should parse");
    let pkg_b = PbixPackage::open(File::open(&path_b).expect("fixture should exist"))
        .expect("pbix B should parse");

    let mut sink = StrictLifecycleSink::default();
    let summary = pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("pbix streaming should succeed");

    assert!(sink.begin_seen, "begin should be called");
    assert_eq!(sink.finish_calls, 1, "finish should be called exactly once");
    assert!(summary.op_count > 0, "expected at least one op");
}

#[test]
fn pbix_streaming_does_not_intern_after_begin() {
    let path_a = fixture_path("pbix_legacy_multi_query_a.pbix");
    let path_b = fixture_path("pbix_legacy_multi_query_b.pbix");
    let pkg_a = PbixPackage::open(File::open(&path_a).expect("fixture should exist"))
        .expect("pbix A should parse");
    let pkg_b = PbixPackage::open(File::open(&path_b).expect("fixture should exist"))
        .expect("pbix B should parse");

    let mut sink = FrozenPoolSink::default();
    pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("pbix streaming should succeed");
    assert_eq!(sink.finish_calls, 1, "finish should be called once");
}

#[test]
fn pbix_streaming_jsonl_header_includes_all_string_ids() {
    #[derive(Deserialize)]
    struct Header {
        kind: String,
        strings: Vec<String>,
    }

    let path_a = fixture_path("pbix_legacy_multi_query_a.pbix");
    let path_b = fixture_path("pbix_legacy_multi_query_b.pbix");
    let pkg_a = PbixPackage::open(File::open(&path_a).expect("fixture should exist"))
        .expect("pbix A should parse");
    let pkg_b = PbixPackage::open(File::open(&path_b).expect("fixture should exist"))
        .expect("pbix B should parse");

    let mut out = Vec::<u8>::new();
    let mut sink = JsonLinesSink::new(&mut out);
    let summary = pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("pbix streaming should succeed");

    let text = std::str::from_utf8(&out).expect("output should be UTF-8");
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header_line = lines.next().expect("expected a JSON Lines header line");
    let header: Header = serde_json::from_str(header_line).expect("header should parse");
    assert_eq!(header.kind, "Header");

    let mut op_lines = 0usize;
    for line in lines {
        let op: DiffOp = serde_json::from_str(line).expect("op line should parse as DiffOp");
        for id in collect_string_ids(&op) {
            assert!(
                (id.0 as usize) < header.strings.len(),
                "StringId {} out of range for header string table (len={})",
                id.0,
                header.strings.len()
            );
        }
        op_lines += 1;
    }

    assert!(op_lines > 0, "expected at least one op line after header");
    assert_eq!(
        summary.op_count, op_lines,
        "summary op_count should match ops written after the header"
    );
}

#[test]
fn workbook_package_streaming_orders_categories() {
    let mut pool = StringPool::new();

    let wb_a = make_workbook(&mut pool, &[1.0]);
    let wb_b = make_workbook(&mut pool, &[2.0]);

    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let vba_name = pool.intern("Module1");
    let vba_modules = Some(vec![VbaModule {
        name: vba_name,
        module_type: VbaModuleType::Standard,
        code: "Sub Foo()\nEnd Sub".to_string(),
    }]);

    let pkg_a = WorkbookPackage {
        workbook: wb_a,
        data_mashup: Some(dm_a),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb_b,
        data_mashup: Some(dm_b),
        vba_modules,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };

    let mut sink = excel_diff::VecSink::new();
    pkg_a
        .diff_streaming_with_pool(&pkg_b, &mut pool, &DiffConfig::default(), &mut sink)
        .expect("streaming diff should succeed");
    let ops = sink.into_ops();

    assert!(
        ops.iter().any(|op| !op.is_m_op() && !is_object_op(op)),
        "expected at least one grid op"
    );
    assert!(ops.iter().any(is_object_op), "expected at least one object op");
    assert!(ops.iter().any(DiffOp::is_m_op), "expected at least one M op");

    enum Stage {
        Grid,
        Object,
        M,
    }

    let mut stage = Stage::Grid;
    for op in &ops {
        if op.is_m_op() {
            stage = Stage::M;
            continue;
        }

        if is_object_op(op) {
            match stage {
                Stage::M => panic!("object op appeared after M ops"),
                Stage::Grid => stage = Stage::Object,
                Stage::Object => {}
            }
            continue;
        }

        match stage {
            Stage::Grid => {}
            Stage::Object => panic!("grid op appeared after object ops"),
            Stage::M => panic!("grid op appeared after M ops"),
        }
    }
}

#[test]
fn streaming_timeout_sets_complete_false_and_warns() {
    let mut pool = StringPool::new();
    let wb_a = make_workbook(&mut pool, &[1.0, 2.0]);
    let wb_b = make_workbook(&mut pool, &[1.0, 3.0]);

    let config = DiffConfig {
        timeout_seconds: Some(0),
        ..DiffConfig::default()
    };

    let mut sink = StrictLifecycleSink::default();
    let summary =
        try_diff_workbooks_streaming(&wb_a, &wb_b, &mut pool, &config, &mut sink)
            .expect("streaming diff should return summary on timeout");

    assert!(!summary.complete, "summary should be incomplete on timeout");
    assert!(
        summary
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("timeout")),
        "expected timeout warning"
    );
    assert_eq!(sink.finish_calls, 1, "finish should be called once");
}

#[test]
fn database_streaming_duplicate_key_fallback_warns_and_finishes() {
    let mut pool = StringPool::new();
    let sheet_id = pool.intern("Data");

    let grid_a = make_keyed_grid(&[1, 1, 2], &[10, 20, 30]);
    let grid_b = make_keyed_grid(&[1, 2, 3], &[10, 25, 35]);

    let mut sink = StrictLifecycleSink::default();
    let mut op_count = 0usize;
    let summary = try_diff_grids_database_mode_streaming(
        sheet_id,
        &grid_a,
        &grid_b,
        &[0],
        &mut pool,
        &DiffConfig::default(),
        &mut sink,
        &mut op_count,
    )
    .expect("streaming should fall back to spreadsheet mode");

    assert!(!summary.complete, "summary should be incomplete on fallback");
    assert_eq!(
        summary.warnings,
        vec![
            "database-mode: duplicate keys for requested columns; falling back to spreadsheet mode"
                .to_string()
        ],
        "warning should be deterministic"
    );
    assert_eq!(sink.finish_calls, 1, "finish should be called once");
}
