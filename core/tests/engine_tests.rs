mod common;

use common::sid;
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, DiffConfig, DiffOp, DiffReport, FormulaDiffResult, Grid,
    Sheet, SheetKind, Workbook, WorkbookPackage,
};

type SheetSpec<'a> = (&'a str, Vec<(u32, u32, f64)>);

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn make_workbook(sheets: Vec<SheetSpec<'_>>) -> Workbook {
    let sheet_ir: Vec<Sheet> = sheets
        .into_iter()
        .map(|(name, cells)| {
            let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
            let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
            let mut grid = Grid::new(max_row + 1, max_col + 1);
            for (r, c, val) in cells {
                grid.insert_cell(r, c, Some(CellValue::Number(val)), None);
            }
            Sheet {
                name: sid(name),
                workbook_sheet_id: None,
                kind: SheetKind::Worksheet,
                grid,
            }
        })
        .collect();
    Workbook {
        sheets: sheet_ir,
        ..Default::default()
    }
}

fn make_sheet_with_kind(name: &str, kind: SheetKind, cells: Vec<(u32, u32, f64)>) -> Sheet {
    make_sheet_with_kind_and_id(name, kind, None, cells)
}

fn make_sheet_with_kind_and_id(
    name: &str,
    kind: SheetKind,
    workbook_sheet_id: Option<u32>,
    cells: Vec<(u32, u32, f64)>,
) -> Sheet {
    let (nrows, ncols) = if cells.is_empty() {
        (0, 0)
    } else {
        let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
        let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
        (max_row + 1, max_col + 1)
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, c, val) in cells {
        grid.insert_cell(r, c, Some(CellValue::Number(val)), None);
    }

    Sheet {
        name: sid(name),
        workbook_sheet_id,
        kind,
        grid,
    }
}

#[test]
fn identical_workbooks_produce_empty_report() {
    let wb = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&wb, &wb, &DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_added_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetAdded { sheet } if *sheet == sid("Sheet2")))
    );
}

#[test]
fn sheet_removed_detected() {
    let old = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetRemoved { sheet } if *sheet == sid("Sheet2")))
    );
}

#[test]
fn cell_edited_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(*sheet, sid("Sheet1"));
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
    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed);
}

#[test]
fn sheet_name_case_insensitive_no_changes() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 1.0)])]);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_name_case_insensitive_cell_edit() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 2.0)])]);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(*sheet, sid("Sheet1"));
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
    grid.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);

    let worksheet = Sheet {
        name: sid("Sheet1"),
        workbook_sheet_id: None,
        kind: SheetKind::Worksheet,
        grid: grid.clone(),
    };

    let chart = Sheet {
        name: sid("Sheet1"),
        workbook_sheet_id: None,
        kind: SheetKind::Chart,
        grid,
    };

    let old = Workbook {
        sheets: vec![worksheet],
        ..Default::default()
    };
    let new = Workbook {
        sheets: vec![chart],
        ..Default::default()
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if *sheet == sid("Sheet1") => added += 1,
            DiffOp::SheetRemoved { sheet } if *sheet == sid("Sheet1") => removed += 1,
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
        ..Default::default()
    };
    let new = Workbook {
        sheets: vec![budget_new.clone(), sheet1_chart, summary_new],
        ..Default::default()
    };

    let budget_addr = CellAddress::from_indices(0, 0);
    let expected = vec![
        DiffOp::cell_edited(
            sid("Budget"),
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
            FormulaDiffResult::Unchanged,
        ),
        DiffOp::SheetRemoved {
            sheet: sid("Sheet1"),
        },
        DiffOp::SheetAdded {
            sheet: sid("sheet1"),
        },
        DiffOp::SheetAdded {
            sheet: sid("Summary"),
        },
    ];

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(
        report.ops, expected,
        "ops should be ordered by lowercase name then sheet kind"
    );
}

#[test]
fn sheet_rename_with_id_emits_rename_and_uses_new_name_for_grid_ops() {
    let old_sheet = make_sheet_with_kind_and_id(
        "OldName",
        SheetKind::Worksheet,
        Some(7),
        vec![(0, 0, 1.0)],
    );
    let new_sheet = make_sheet_with_kind_and_id(
        "NewName",
        SheetKind::Worksheet,
        Some(7),
        vec![(0, 0, 2.0)],
    );
    let old = Workbook {
        sheets: vec![old_sheet],
        ..Default::default()
    };
    let new = Workbook {
        sheets: vec![new_sheet],
        ..Default::default()
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 2, "expected rename plus one cell edit");

    let mut saw_rename = false;
    let mut saw_edit = false;
    for op in &report.ops {
        match op {
            DiffOp::SheetRenamed { sheet, from, to } => {
                assert_eq!(sheet, &sid("NewName"));
                assert_eq!(from, &sid("OldName"));
                assert_eq!(to, &sid("NewName"));
                saw_rename = true;
            }
            DiffOp::CellEdited { sheet, .. } => {
                assert_eq!(sheet, &sid("NewName"));
                saw_edit = true;
            }
            other => panic!("unexpected op: {other:?}"),
        }
    }
    assert!(saw_rename, "expected SheetRenamed op");
    assert!(saw_edit, "expected CellEdited op");
}

#[test]
fn sheet_name_swap_prefers_id_matching() {
    let old_a = make_sheet_with_kind_and_id(
        "Alpha",
        SheetKind::Worksheet,
        Some(1),
        vec![(0, 0, 1.0)],
    );
    let old_b = make_sheet_with_kind_and_id(
        "Beta",
        SheetKind::Worksheet,
        Some(2),
        vec![(0, 0, 10.0)],
    );
    let new_a = make_sheet_with_kind_and_id(
        "Beta",
        SheetKind::Worksheet,
        Some(1),
        vec![(0, 0, 2.0)],
    );
    let new_b = make_sheet_with_kind_and_id(
        "Alpha",
        SheetKind::Worksheet,
        Some(2),
        vec![(0, 0, 11.0)],
    );

    let old = Workbook {
        sheets: vec![old_a, old_b],
        ..Default::default()
    };
    let new = Workbook {
        sheets: vec![new_a, new_b],
        ..Default::default()
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 4, "expected two renames and two edits");

    let mut renames = Vec::new();
    let mut edits = Vec::new();
    for op in &report.ops {
        match op {
            DiffOp::SheetRenamed { from, to, .. } => renames.push((*from, *to)),
            DiffOp::CellEdited { sheet, .. } => edits.push(*sheet),
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(renames.len(), 2);
    assert!(renames.contains(&(sid("Alpha"), sid("Beta"))));
    assert!(renames.contains(&(sid("Beta"), sid("Alpha"))));
    assert!(edits.contains(&sid("Alpha")));
    assert!(edits.contains(&sid("Beta")));
}

#[test]
fn sheet_identity_includes_kind_for_macro_and_other() {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);

    let macro_sheet = Sheet {
        name: sid("Code"),
        workbook_sheet_id: None,
        kind: SheetKind::Macro,
        grid: grid.clone(),
    };

    let other_sheet = Sheet {
        name: sid("Code"),
        workbook_sheet_id: None,
        kind: SheetKind::Other,
        grid,
    };

    let old = Workbook {
        sheets: vec![macro_sheet],
        ..Default::default()
    };
    let new = Workbook {
        sheets: vec![other_sheet],
        ..Default::default()
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if *sheet == sid("Code") => added += 1,
            DiffOp::SheetRemoved { sheet } if *sheet == sid("Code") => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Other 'Code'");
    assert_eq!(removed, 1, "expected one SheetRemoved for Macro 'Code'");
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[test]
fn sheet_id_matching_respects_kind() {
    let old_sheet = make_sheet_with_kind_and_id(
        "Sheet1",
        SheetKind::Worksheet,
        Some(42),
        vec![(0, 0, 1.0)],
    );
    let new_sheet = make_sheet_with_kind_and_id(
        "Sheet1",
        SheetKind::Chart,
        Some(42),
        Vec::new(),
    );

    let old = Workbook {
        sheets: vec![old_sheet],
        ..Default::default()
    };
    let new = Workbook {
        sheets: vec![new_sheet],
        ..Default::default()
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 2, "expected add/remove due to kind mismatch");
    assert!(report.ops.iter().any(|op| matches!(op, DiffOp::SheetRemoved { sheet } if *sheet == sid("Sheet1"))));
    assert!(report.ops.iter().any(|op| matches!(op, DiffOp::SheetAdded { sheet } if *sheet == sid("Sheet1"))));
    assert!(!report.ops.iter().any(|op| matches!(op, DiffOp::SheetRenamed { .. })));
}

#[cfg(not(debug_assertions))]
#[test]
fn duplicate_sheet_identity_last_writer_wins_release() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);

    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
        ..Default::default()
    };
    let new = Workbook {
        sheets: Vec::new(),
        ..Default::default()
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1, "expected last writer to win");

    match &report.ops[0] {
        DiffOp::SheetRemoved { sheet } => assert_eq!(
            *sheet,
            sid("sheet1"),
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

            grid_a.insert_cell(r, c, base_value.clone(), None);

            let in_src = src_rows.contains(&r) && src_cols.contains(&c);
            let in_dst = src_rows.contains(&r) && c >= dst_start_col && c < dst_end_col;

            if in_dst {
                let offset = c - dst_start_col;
                let src_c = src_cols.start + offset;
                let moved_value = Some(CellValue::Number((r * 1_000 + src_c) as f64));
                grid_b.insert_cell(r, c, moved_value, None);
            } else if !in_src {
                grid_b.insert_cell(r, c, base_value, None);
            }
        }
    }

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: sid("Sheet1"),
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid: grid_a,
        }],
        ..Default::default()
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sid("Sheet1"),
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid: grid_b,
        }],
        ..Default::default()
    };

    let default_report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());
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

    let mut wide_gate = DiffConfig::default();
    wide_gate.moves.max_move_detection_cols = 512;
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
fn duplicate_sheet_identity_emits_warning() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);
    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
        ..Default::default()
    };
    let new = Workbook {
        sheets: Vec::new(),
        ..Default::default()
    };

    let result = std::panic::catch_unwind(|| diff_workbooks(&old, &new, &DiffConfig::default()));
    assert!(result.is_ok(), "duplicate sheet identity should not panic");
    let report = result.unwrap();
    assert!(
        report.warnings.iter().any(|w| w.contains("duplicate sheet identity")),
        "should emit warning about duplicate sheet identity; warnings: {:?}",
        report.warnings
    );
}

#[test]
fn duplicate_workbook_sheet_id_falls_back_to_name_matching() {
    let first = make_sheet_with_kind_and_id(
        "First",
        SheetKind::Worksheet,
        Some(1),
        vec![(0, 0, 1.0)],
    );
    let second = make_sheet_with_kind_and_id(
        "Second",
        SheetKind::Worksheet,
        Some(1),
        vec![(0, 0, 2.0)],
    );
    let old = Workbook {
        sheets: vec![first, second],
        ..Default::default()
    };
    let new = Workbook {
        sheets: Vec::new(),
        ..Default::default()
    };

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(
        report.warnings.iter().any(|w| w.contains("duplicate workbook sheetId in old workbook: id=1")),
        "expected duplicate workbook sheetId warning, warnings: {:?}",
        report.warnings
    );

    let removed: Vec<_> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::SheetRemoved { sheet } => Some(*sheet),
            _ => None,
        })
        .collect();
    assert_eq!(removed.len(), 2, "expected both sheets removed via fallback");
    assert!(removed.contains(&sid("First")));
    assert!(removed.contains(&sid("Second")));
}
