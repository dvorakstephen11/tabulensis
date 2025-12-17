use excel_diff::{
    DataMashup, DiffConfig, DiffError, DiffOp, DiffSink, Grid, Metadata, PackageParts, PackageXml,
    Permissions, SectionDocument, Sheet, SheetKind, Workbook, WorkbookPackage,
};

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
    };
    let pkg_b = WorkbookPackage {
        workbook: wb,
        data_mashup: Some(dm_b),
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

    let sheet_id =
        excel_diff::with_default_session(|session| session.strings.intern("Sheet1"));

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
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            kind: SheetKind::Worksheet,
            grid: grid_b,
        }],
    };

    let pkg_a = WorkbookPackage {
        workbook: wb_a,
        data_mashup: None,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb_b,
        data_mashup: None,
    };

    let mut sink = FailingSink {
        calls: 0,
        finish_called: false,
    };

    let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);
    assert!(result.is_err(), "diff_streaming should return error");
    assert!(sink.finish_called, "sink.finish() should be called on error");
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
    };
    let pkg_b = WorkbookPackage {
        workbook: wb,
        data_mashup: Some(dm_b),
    };

    let mut sink = FailOnMOpSink {
        finish_called: false,
        finish_calls: 0,
    };

    let result = pkg_a.diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink);

    assert!(result.is_err(), "expected sink error during M op emission");
    assert!(sink.finish_called, "sink.finish() should be called on M emit error");
    assert_eq!(sink.finish_calls, 1, "finish should be called exactly once");
}
