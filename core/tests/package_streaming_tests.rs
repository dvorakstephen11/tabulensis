mod common;

use common::collect_string_ids;
use excel_diff::{
    DataMashup, DiffConfig, DiffError, DiffOp, DiffSink, Grid, JsonLinesSink, Metadata,
    PackageParts, PackageXml, Permissions, SectionDocument, Sheet, SheetKind, Workbook,
    WorkbookPackage,
};
#[cfg(feature = "perf-metrics")]
use excel_diff::{CallbackSink, CellValue};
use serde::Deserialize;

#[derive(Default)]
struct StrictSink {
    finished: bool,
    finish_calls: usize,
    ops: Vec<DiffOp>,
}

impl DiffSink for StrictSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        if self.finished {
            return Err(DiffError::SinkError {
                message: "emit called after finish".to_string(),
            });
        }
        self.ops.push(op);
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.finish_calls += 1;
        self.finished = true;
        Ok(())
    }
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
        metadata: Metadata {
            formulas: Vec::new(),
        },
        permission_bindings_raw: Vec::new(),
    }
}

fn make_workbook(sheet_name: &str) -> Workbook {
    let sheet_id = excel_diff::with_default_session(|session| session.strings.intern(sheet_name));

    Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: Grid::new(0, 0),
        }],
        ..Default::default()
    }
}

#[test]
fn package_diff_streaming_does_not_emit_after_finish_and_finishes_once() {
    let wb = make_workbook("Sheet1");

    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let pkg_a = WorkbookPackage {
        workbook: wb.clone(),
        data_mashup: Some(dm_a),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb,
        data_mashup: Some(dm_b),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };

    let mut sink = StrictSink::default();
    let summary = pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("diff_streaming should succeed");

    assert!(sink.finished, "sink should be finished at end");
    assert_eq!(
        sink.finish_calls, 1,
        "sink.finish() should be called exactly once"
    );

    assert!(
        sink.ops.iter().any(|op| op.is_m_op()),
        "expected at least one M diff op in streaming output"
    );

    assert_eq!(
        summary.op_count,
        sink.ops.len(),
        "summary.op_count should match ops actually emitted"
    );
}

#[test]
fn package_diff_streaming_finishes_on_error() {
    struct FailingSink {
        calls: usize,
        finish_called: bool,
    }

    impl DiffSink for FailingSink {
        fn emit(&mut self, _op: DiffOp) -> Result<(), DiffError> {
            self.calls += 1;
            if self.calls > 2 {
                return Err(DiffError::SinkError {
                    message: "intentional failure".to_string(),
                });
            }
            Ok(())
        }

        fn finish(&mut self) -> Result<(), DiffError> {
            self.finish_called = true;
            Ok(())
        }
    }

    let sheet_id = excel_diff::with_default_session(|session| session.strings.intern("Sheet1"));

    let mut grid_a = Grid::new(10, 1);
    let mut grid_b = Grid::new(10, 1);
    for i in 0..10 {
        grid_a.insert_cell(i, 0, Some(excel_diff::CellValue::Number(i as f64)), None);
        grid_b.insert_cell(
            i,
            0,
            Some(excel_diff::CellValue::Number((i + 100) as f64)),
            None,
        );
    }

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: grid_a,
        }],
        ..Default::default()
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: grid_b,
        }],
        ..Default::default()
    };

    let pkg_a = WorkbookPackage {
        workbook: wb_a,
        data_mashup: None,
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb_b,
        data_mashup: None,
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };

    let mut sink = FailingSink {
        calls: 0,
        finish_called: false,
    };

    let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);
    assert!(result.is_err(), "diff_streaming should return error");
    assert!(
        sink.finish_called,
        "sink.finish() should be called on error"
    );
}

#[test]
fn package_diff_streaming_finishes_on_m_emit_error() {
    struct FailOnMOpSink {
        finish_called: bool,
        finish_calls: usize,
    }

    impl DiffSink for FailOnMOpSink {
        fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
            if op.is_m_op() {
                return Err(DiffError::SinkError {
                    message: "fail on m op".to_string(),
                });
            }
            Ok(())
        }

        fn finish(&mut self) -> Result<(), DiffError> {
            self.finish_calls += 1;
            self.finish_called = true;
            Ok(())
        }
    }

    let wb = make_workbook("Sheet1");

    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let pkg_a = WorkbookPackage {
        workbook: wb.clone(),
        data_mashup: Some(dm_a),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb,
        data_mashup: Some(dm_b),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };

    let mut sink = FailOnMOpSink {
        finish_called: false,
        finish_calls: 0,
    };

    let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);

    assert!(result.is_err(), "expected sink error during M op emission");
    assert!(
        sink.finish_called,
        "sink.finish() should be called on M emit error"
    );
    assert_eq!(sink.finish_calls, 1, "finish should be called exactly once");
}

#[cfg(feature = "perf-metrics")]
#[test]
fn package_diff_streaming_includes_package_parse_time_in_total() {
    let mut grid_a = Grid::new(1, 1);
    let mut grid_b = Grid::new(1, 1);
    grid_a.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
    grid_b.insert_cell(0, 0, Some(CellValue::Number(2.0)), None);

    let sheet_id = excel_diff::with_default_session(|session| session.strings.intern("Sheet1"));

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: grid_a,
        }],
        ..Default::default()
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: grid_b,
        }],
        ..Default::default()
    };

    let pkg_a = WorkbookPackage {
        workbook: wb_a,
        data_mashup: None,
        vba_modules: None,
        parse_time_ms: 15,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb_b,
        data_mashup: None,
        vba_modules: None,
        parse_time_ms: 25,
    };

    let mut sink = CallbackSink::new(|_op| {});
    let summary = pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("diff_streaming should succeed");
    let metrics = summary.metrics.expect("expected perf metrics");

    let added = 15u64.saturating_add(25u64);
    assert!(
        metrics.parse_time_ms >= added,
        "parse_time_ms should include package parse time (>= {}), got {}",
        added,
        metrics.parse_time_ms
    );
    assert!(
        metrics.total_time_ms >= metrics.parse_time_ms,
        "total_time_ms should include parse_time_ms (total={}, parse={})",
        metrics.total_time_ms,
        metrics.parse_time_ms
    );
    assert_eq!(
        metrics.diff_time_ms,
        metrics.total_time_ms.saturating_sub(metrics.parse_time_ms)
    );
}

#[test]
fn package_streaming_json_lines_header_includes_m_strings() {
    #[derive(Deserialize)]
    struct Header {
        kind: String,
        strings: Vec<String>,
    }

    let wb = make_workbook("Sheet1");

    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let pkg_a = WorkbookPackage {
        workbook: wb.clone(),
        data_mashup: Some(dm_a),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb,
        data_mashup: Some(dm_b),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };

    let mut out = Vec::<u8>::new();
    let mut sink = JsonLinesSink::new(&mut out);

    let summary = pkg_a
        .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
        .expect("diff_streaming should succeed");

    let text = std::str::from_utf8(&out).expect("output should be valid UTF-8");
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header_line = lines.next().expect("expected a JSON Lines header line");
    let header: Header = serde_json::from_str(header_line).expect("header should parse");

    assert_eq!(header.kind, "Header");
    assert!(
        header.strings.iter().any(|s| s == "Section1/Foo"),
        "expected header string table to include query name Section1/Foo"
    );
    assert!(
        header.strings.iter().any(|s| s == "Section1/Bar"),
        "expected header string table to include query name Section1/Bar"
    );

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
        "summary op_count should match number of ops written after the header"
    );
}
