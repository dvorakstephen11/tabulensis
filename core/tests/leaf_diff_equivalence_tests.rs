use excel_diff::advanced::{diff_grids_with_pool, diff_sheets_with_pool, diff_workbooks_with_pool};
use excel_diff::{DiffConfig, Grid, Sheet, SheetKind, StringPool, Workbook};

fn make_grid(values: &[f64]) -> Grid {
    let mut grid = Grid::new(values.len() as u32, 1);
    for (idx, val) in values.iter().enumerate() {
        grid.insert_cell(idx as u32, 0, Some(excel_diff::CellValue::Number(*val)), None);
    }
    grid
}

#[test]
fn grid_leaf_diff_matches_single_sheet_workbook() {
    let mut pool = StringPool::new();
    let sheet_id = pool.intern("<grid>");

    let grid_a = make_grid(&[1.0, 2.0]);
    let grid_b = make_grid(&[1.0, 3.0]);

    let wb_a = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid: grid_a.clone(),
        }],
        ..Default::default()
    };
    let wb_b = Workbook {
        sheets: vec![Sheet {
            name: sheet_id,
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid: grid_b.clone(),
        }],
        ..Default::default()
    };

    let config = DiffConfig::default();
    let leaf_report = diff_grids_with_pool(&grid_a, &grid_b, &mut pool, &config);
    let wb_report = diff_workbooks_with_pool(&wb_a, &wb_b, &mut pool, &config);

    assert_eq!(leaf_report.ops, wb_report.ops);
    assert_eq!(leaf_report.complete, wb_report.complete);
    assert_eq!(leaf_report.warnings, wb_report.warnings);
}

#[test]
fn sheet_leaf_diff_matches_single_sheet_workbook() {
    let mut pool = StringPool::new();
    let sheet_id = pool.intern("Sheet1");

    let grid_a = make_grid(&[1.0, 2.0]);
    let grid_b = make_grid(&[1.0, 3.0]);

    let sheet_a = Sheet {
        name: sheet_id,
        workbook_sheet_id: None,
        kind: SheetKind::Worksheet,
        grid: grid_a.clone(),
    };
    let sheet_b = Sheet {
        name: sheet_id,
        workbook_sheet_id: None,
        kind: SheetKind::Worksheet,
        grid: grid_b.clone(),
    };

    let wb_a = Workbook {
        sheets: vec![sheet_a.clone()],
        ..Default::default()
    };
    let wb_b = Workbook {
        sheets: vec![sheet_b.clone()],
        ..Default::default()
    };

    let config = DiffConfig::default();
    let leaf_report = diff_sheets_with_pool(&sheet_a, &sheet_b, &mut pool, &config);
    let wb_report = diff_workbooks_with_pool(&wb_a, &wb_b, &mut pool, &config);

    assert_eq!(leaf_report.ops, wb_report.ops);
    assert_eq!(leaf_report.complete, wb_report.complete);
    assert_eq!(leaf_report.warnings, wb_report.warnings);
}
