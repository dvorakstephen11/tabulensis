use excel_diff::{
    CellValue, DiffConfig, DiffOp, FormulaDiffResult, Grid, Sheet, SheetKind, StringPool, Workbook,
    diff_grids_database_mode, diff_workbooks_with_pool,
};

fn workbook_with_formula(
    pool: &mut StringPool,
    sheet: excel_diff::StringId,
    row: u32,
    col: u32,
    formula: &str,
) -> Workbook {
    let mut grid = Grid::new(row + 1, col + 1);
    let formula_id = pool.intern(formula);
    grid.insert_cell(row, col, None, Some(formula_id));

    Workbook {
        sheets: vec![Sheet {
            name: sheet,
            kind: SheetKind::Worksheet,
            grid,
        }],
        ..Default::default()
    }
}

fn cell_edit_op(report: &excel_diff::DiffReport) -> DiffOp {
    report
        .ops
        .iter()
        .find(|op| matches!(op, DiffOp::CellEdited { .. }))
        .cloned()
        .expect("expected a cell edit in the diff")
}

#[test]
fn formatting_only_vs_text_change_respects_flag() {
    let mut pool = StringPool::new();
    let sheet = pool.intern("Sheet1");
    let old = workbook_with_formula(&mut pool, sheet, 0, 0, "sum(a1,b1)");
    let new = workbook_with_formula(&mut pool, sheet, 0, 0, "SUM(A1,B1)");

    let mut enabled = DiffConfig::default();
    enabled.semantic.enable_formula_semantic_diff = true;
    let disabled = DiffConfig::default();

    let report_enabled = diff_workbooks_with_pool(&old, &new, &mut pool, &enabled);
    let report_disabled = diff_workbooks_with_pool(&old, &new, &mut pool, &disabled);

    match cell_edit_op(&report_enabled) {
        DiffOp::CellEdited { formula_diff, .. } => {
            assert_eq!(formula_diff, FormulaDiffResult::FormattingOnly);
        }
        _ => panic!("expected CellEdited op in enabled diff"),
    }

    match cell_edit_op(&report_disabled) {
        DiffOp::CellEdited { formula_diff, .. } => {
            assert_eq!(formula_diff, FormulaDiffResult::TextChange);
        }
        _ => panic!("expected CellEdited op in disabled diff"),
    }
}

#[test]
fn filled_down_formulas_detect_row_shift() {
    let mut pool = StringPool::new();

    let mut config = DiffConfig::default();
    config.semantic.enable_formula_semantic_diff = true;

    let mut old = Grid::new(1, 2);
    old.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
    old.insert_cell(0, 1, None, Some(pool.intern("A1+B1")));

    let mut new = Grid::new(2, 2);
    new.insert_cell(0, 0, Some(CellValue::Number(0.0)), None);
    new.insert_cell(1, 0, Some(CellValue::Number(1.0)), None);
    new.insert_cell(1, 1, None, Some(pool.intern("A2+B2")));

    let report = diff_grids_database_mode(&old, &new, &[0], &mut pool, &config);

    let cell_edit = cell_edit_op(&report);
    match cell_edit {
        DiffOp::CellEdited {
            addr, formula_diff, ..
        } => {
            assert_eq!(addr.row, 1);
            assert_eq!(addr.col, 1);
            assert_eq!(formula_diff, FormulaDiffResult::Filled);
        }
        _ => panic!("expected CellEdited op"),
    }

    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { row_idx, .. } if *row_idx == 0)),
        "expected a row insertion ahead of the filled-down formula",
    );
}
