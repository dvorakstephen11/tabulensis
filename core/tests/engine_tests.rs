use excel_diff::{
    Cell, CellAddress, CellValue, DiffOp, DiffReport, Grid, Sheet, SheetKind, Workbook,
    diff_workbooks,
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

#[test]
fn identical_workbooks_produce_empty_report() {
    let wb = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&wb, &wb);
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_added_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let report = diff_workbooks(&old, &new);
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
    let report = diff_workbooks(&old, &new);
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
    let report = diff_workbooks(&old, &new);
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
    let report = diff_workbooks(&old, &new);
    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed);
}

#[test]
fn sheet_name_case_insensitive_no_changes() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 1.0)])]);

    let report = diff_workbooks(&old, &new);
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_name_case_insensitive_cell_edit() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 2.0)])]);

    let report = diff_workbooks(&old, &new);
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

    let report = diff_workbooks(&old, &new);

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
