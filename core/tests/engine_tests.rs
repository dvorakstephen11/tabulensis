use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, DiffOp, DiffReport, Grid, Sheet, SheetKind,
    Workbook, diff_workbooks,
};

type SheetSpec<'a> = (&'a str, Vec<(u32, u32, f64)>);

fn make_workbook(sheets: Vec<SheetSpec<'_>>) -> Workbook {
    let sheet_ir: Vec<Sheet> = sheets
        .into_iter()
        .map(|(name, cells)| {
            let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
            let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
            let mut grid = Grid::new(max_row + 1, max_col + 1);
            for (r, c, val) in cells {
                grid.insert(Cell {
                    row: r,
                    col: c,
                    address: CellAddress::from_indices(r, c),
                    value: Some(CellValue::Number(val)),
                    formula: None,
                });
            }
            Sheet {
                name: name.to_string(),
                kind: SheetKind::Worksheet,
                grid,
            }
        })
        .collect();
    Workbook { sheets: sheet_ir }
}

fn make_sheet_with_kind(name: &str, kind: SheetKind, cells: Vec<(u32, u32, f64)>) -> Sheet {
    let (nrows, ncols) = if cells.is_empty() {
        (0, 0)
    } else {
        let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
        let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
        (max_row + 1, max_col + 1)
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, c, val) in cells {
        grid.insert(Cell {
            row: r,
            col: c,
            address: CellAddress::from_indices(r, c),
            value: Some(CellValue::Number(val)),
            formula: None,
        });
    }

    Sheet {
        name: name.to_string(),
        kind,
        grid,
    }
}

#[test]
fn identical_workbooks_produce_empty_report() {
    let wb = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&wb, &wb, &excel_diff::DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_added_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetAdded { sheet } if sheet == "Sheet2"))
    );
}

#[test]
fn sheet_removed_detected() {
    let old = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetRemoved { sheet } if sheet == "Sheet2"))
    );
}

#[test]
fn cell_edited_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    assert_eq!(report.ops.len(), 1);
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        _ => panic!("expected CellEdited"),
    }
}

#[test]
fn diff_report_json_round_trips() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed);
}

#[test]
fn sheet_name_case_insensitive_no_changes() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 1.0)])]);

    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_name_case_insensitive_cell_edit() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 2.0)])]);

    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn sheet_identity_includes_kind() {
    let mut grid = Grid::new(1, 1);
    grid.insert(Cell {
        row: 0,
        col: 0,
        address: CellAddress::from_indices(0, 0),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    let worksheet = Sheet {
        name: "Sheet1".to_string(),
        kind: SheetKind::Worksheet,
        grid: grid.clone(),
    };

    let chart = Sheet {
        name: "Sheet1".to_string(),
        kind: SheetKind::Chart,
        grid,
    };

    let old = Workbook {
        sheets: vec![worksheet],
    };
    let new = Workbook {
        sheets: vec![chart],
    };

    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Sheet1" => added += 1,
            DiffOp::SheetRemoved { sheet } if sheet == "Sheet1" => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Chart 'Sheet1'");
    assert_eq!(
        removed, 1,
        "expected one SheetRemoved for Worksheet 'Sheet1'"
    );
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[test]
fn deterministic_sheet_op_ordering() {
    let budget_old = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let budget_new = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 2.0)]);
    let sheet1_old = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 1, 5.0)]);
    let sheet1_chart = make_sheet_with_kind("sheet1", SheetKind::Chart, Vec::new());
    let summary_new = make_sheet_with_kind("Summary", SheetKind::Worksheet, vec![(0, 0, 3.0)]);

    let old = Workbook {
        sheets: vec![budget_old.clone(), sheet1_old],
    };
    let new = Workbook {
        sheets: vec![budget_new.clone(), sheet1_chart, summary_new],
    };

    let budget_addr = CellAddress::from_indices(0, 0);
    let expected = vec![
        DiffOp::cell_edited(
            "Budget".into(),
            budget_addr,
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(1.0)),
                formula: None,
            },
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(2.0)),
                formula: None,
            },
        ),
        DiffOp::SheetRemoved {
            sheet: "Sheet1".into(),
        },
        DiffOp::SheetAdded {
            sheet: "sheet1".into(),
        },
        DiffOp::SheetAdded {
            sheet: "Summary".into(),
        },
    ];

    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    assert_eq!(
        report.ops, expected,
        "ops should be ordered by lowercase name then sheet kind"
    );
}

#[test]
fn sheet_identity_includes_kind_for_macro_and_other() {
    let mut grid = Grid::new(1, 1);
    grid.insert(Cell {
        row: 0,
        col: 0,
        address: CellAddress::from_indices(0, 0),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    let macro_sheet = Sheet {
        name: "Code".to_string(),
        kind: SheetKind::Macro,
        grid: grid.clone(),
    };

    let other_sheet = Sheet {
        name: "Code".to_string(),
        kind: SheetKind::Other,
        grid,
    };

    let old = Workbook {
        sheets: vec![macro_sheet],
    };
    let new = Workbook {
        sheets: vec![other_sheet],
    };

    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Code" => added += 1,
            DiffOp::SheetRemoved { sheet } if sheet == "Code" => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Other 'Code'");
    assert_eq!(removed, 1, "expected one SheetRemoved for Macro 'Code'");
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[cfg(not(debug_assertions))]
#[test]
fn duplicate_sheet_identity_last_writer_wins_release() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);

    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let report = diff_workbooks(&old, &new, &excel_diff::DiffConfig::default());
    assert_eq!(report.ops.len(), 1, "expected last writer to win");

    match &report.ops[0] {
        DiffOp::SheetRemoved { sheet } => assert_eq!(
            sheet, "sheet1",
            "duplicate identity should prefer the last sheet in release builds"
        ),
        other => panic!("expected SheetRemoved, got {other:?}"),
    }
}

#[test]
fn move_detection_respects_column_gate() {
    let nrows: u32 = 4;
    let ncols: u32 = 300;
    let src_rows = 1..3;
    let src_cols = 2..7;
    let dst_start_col: u32 = 200;
    let dst_end_col = dst_start_col + (src_cols.end - src_cols.start);

    let mut grid_a = Grid::new(nrows, ncols);
    let mut grid_b = Grid::new(nrows, ncols);

    for r in 0..nrows {
        for c in 0..ncols {
            let base_value = Some(CellValue::Number((r * 1_000 + c) as f64));
            let addr = CellAddress::from_indices(r, c);

            grid_a.insert(Cell {
                row: r,
                col: c,
                address: addr,
                value: base_value.clone(),
                formula: None,
            });

            let in_src = src_rows.contains(&r) && src_cols.contains(&c);
            let in_dst = src_rows.contains(&r) && c >= dst_start_col && c < dst_end_col;

            if in_dst {
                let offset = c - dst_start_col;
                let src_c = src_cols.start + offset;
                let moved_value = Some(CellValue::Number((r * 1_000 + src_c) as f64));
                grid_b.insert(Cell {
                    row: r,
                    col: c,
                    address: addr,
                    value: moved_value,
                    formula: None,
                });
            } else if !in_src {
                grid_b.insert(Cell {
                    row: r,
                    col: c,
                    address: addr,
                    value: base_value,
                    formula: None,
                });
            }
        }
    }

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: "Sheet1".to_string(),
            kind: SheetKind::Worksheet,
            grid: grid_a,
        }],
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: "Sheet1".to_string(),
            kind: SheetKind::Worksheet,
            grid: grid_b,
        }],
    };

    let default_report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());
    assert!(
        !default_report.ops.is_empty(),
        "changes should be detected even when move detection is gated off"
    );
    assert!(
        !default_report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "default gate should skip block move detection on wide sheets"
    );

    let wide_gate = excel_diff::DiffConfig {
        max_move_detection_cols: 512,
        ..excel_diff::DiffConfig::default()
    };
    let wide_report = diff_workbooks(&wb_a, &wb_b, &wide_gate);
    assert!(
        !wide_report.ops.is_empty(),
        "expected diffs when move detection is enabled"
    );
    assert!(
        wide_report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "wider gate should allow block move detection on wide sheets"
    );
}

#[test]
fn duplicate_sheet_identity_panics_in_debug() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);
    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let result =
        std::panic::catch_unwind(|| diff_workbooks(&old, &new, &excel_diff::DiffConfig::default()));
    if cfg!(debug_assertions) {
        assert!(
            result.is_err(),
            "duplicate sheet identities should trigger a debug assertion"
        );
    } else {
        assert!(result.is_ok(), "debug assertions disabled should not panic");
    }
}
