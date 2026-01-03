mod common;

use common::{open_fixture_workbook, sid};
use excel_diff::{
    CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, diff_grids_database_mode,
    with_default_session,
};

fn diff_db(grid_a: &Grid, grid_b: &Grid, keys: &[u32]) -> DiffReport {
    with_default_session(|session| {
        diff_grids_database_mode(
            grid_a,
            grid_b,
            keys,
            &mut session.strings,
            &DiffConfig::default(),
        )
    })
}

fn data_grid(workbook: &Workbook) -> &Grid {
    let data_id = sid("Data");
    workbook
        .sheets
        .iter()
        .find(|s| s.name == data_id)
        .map(|s| &s.grid)
        .expect("Data sheet present")
}

fn find_row_by_id(grid: &Grid, target_id: i64) -> Option<u32> {
    for row in 0..grid.nrows {
        if let Some(cell) = grid.get(row, 0) {
            if let Some(CellValue::Number(n)) = cell.value {
                if (n as i64) == target_id {
                    return Some(row);
                }
            }
        }
    }
    None
}

fn count_ops(report: &DiffReport) -> (usize, usize, usize) {
    let row_added = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    let row_removed = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    let cell_edited = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    (row_added, row_removed, cell_edited)
}

#[test]
fn d2_row_added_emits_row_added_only() {
    let wb_a = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let wb_b = open_fixture_workbook("db_row_added_b.xlsx");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_db(grid_a, grid_b, &[0]);
    let (row_added, row_removed, cell_edited) = count_ops(&report);

    assert_eq!(row_added, 1, "should emit exactly 1 RowAdded");
    assert_eq!(row_removed, 0, "should emit 0 RowRemoved");
    assert_eq!(cell_edited, 0, "should emit 0 CellEdited");

    let row_b = find_row_by_id(grid_b, 1001).expect("new row with ID 1001 should exist in B");

    let added_row_idx = report
        .ops
        .iter()
        .find_map(|op| {
            if let DiffOp::RowAdded { row_idx, .. } = op {
                Some(*row_idx)
            } else {
                None
            }
        })
        .expect("RowAdded op should exist");

    assert_eq!(
        added_row_idx, row_b,
        "RowAdded row_idx should match the row of ID 1001 in grid B"
    );
}

#[test]
fn d2_row_removed_emits_row_removed_only() {
    let wb_a = open_fixture_workbook("db_row_added_b.xlsx");
    let wb_b = open_fixture_workbook("db_equal_ordered_a.xlsx");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_db(grid_a, grid_b, &[0]);
    let (row_added, row_removed, cell_edited) = count_ops(&report);

    assert_eq!(row_removed, 1, "should emit exactly 1 RowRemoved");
    assert_eq!(row_added, 0, "should emit 0 RowAdded");
    assert_eq!(cell_edited, 0, "should emit 0 CellEdited");

    let row_a = find_row_by_id(grid_a, 1001).expect("removed row with ID 1001 should exist in A");

    let removed_row_idx = report
        .ops
        .iter()
        .find_map(|op| {
            if let DiffOp::RowRemoved { row_idx, .. } = op {
                Some(*row_idx)
            } else {
                None
            }
        })
        .expect("RowRemoved op should exist");

    assert_eq!(
        removed_row_idx, row_a,
        "RowRemoved row_idx should match the row of ID 1001 in grid A"
    );
}

#[test]
fn d3_row_update_emits_cell_edited_only() {
    let wb_a = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let wb_b = open_fixture_workbook("db_row_update_b.xlsx");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_db(grid_a, grid_b, &[0]);
    let (row_added, row_removed, cell_edited) = count_ops(&report);

    assert_eq!(cell_edited, 1, "should emit exactly 1 CellEdited");
    assert_eq!(row_added, 0, "should emit 0 RowAdded");
    assert_eq!(row_removed, 0, "should emit 0 RowRemoved");

    let row_b = find_row_by_id(grid_b, 7).expect("row with ID 7 should exist in B");

    let edit = report
        .ops
        .iter()
        .find_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                Some((addr, from, to))
            } else {
                None
            }
        })
        .expect("CellEdited op should exist");

    let (addr, from, to) = edit;
    assert_eq!(addr.row, row_b, "CellEdited should target row of ID 7 in B");
    assert_eq!(
        addr.col, 2,
        "CellEdited should target Amount column (col 2)"
    );

    let baseline_amount = 7.0 * 10.5;
    match &from.value {
        Some(CellValue::Number(n)) => {
            assert!(
                (*n - baseline_amount).abs() < 0.001,
                "from.value should be baseline Amount for ID 7 ({baseline_amount}), got {n}"
            );
        }
        other => panic!("from.value should be Number, got {:?}", other),
    }

    match &to.value {
        Some(CellValue::Number(n)) => {
            assert!(
                (*n - 120.0).abs() < 0.001,
                "to.value should be 120.0, got {n}"
            );
        }
        other => panic!("to.value should be Number, got {:?}", other),
    }

    assert!(from.formula.is_none(), "from.formula should be None");
    assert!(to.formula.is_none(), "to.formula should be None");
}

#[test]
fn d4_reorder_and_change_emits_cell_edited_only() {
    let wb_a = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let wb_b = open_fixture_workbook("db_reorder_and_change_b.xlsx");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_db(grid_a, grid_b, &[0]);
    let (row_added, row_removed, cell_edited) = count_ops(&report);

    assert_eq!(cell_edited, 1, "should emit exactly 1 CellEdited");
    assert_eq!(row_added, 0, "should emit 0 RowAdded (reorder is ignored)");
    assert_eq!(
        row_removed, 0,
        "should emit 0 RowRemoved (reorder is ignored)"
    );

    let row_b = find_row_by_id(grid_b, 7).expect("row with ID 7 should exist in shuffled B");

    let edit = report
        .ops
        .iter()
        .find_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                Some((addr, from, to))
            } else {
                None
            }
        })
        .expect("CellEdited op should exist");

    let (addr, from, to) = edit;
    assert_eq!(
        addr.row, row_b,
        "CellEdited should target row of ID 7 in shuffled B"
    );
    assert_eq!(
        addr.col, 2,
        "CellEdited should target Amount column (col 2)"
    );

    let baseline_amount = 7.0 * 10.5;
    match &from.value {
        Some(CellValue::Number(n)) => {
            assert!(
                (*n - baseline_amount).abs() < 0.001,
                "from.value should be baseline Amount for ID 7 ({baseline_amount}), got {n}"
            );
        }
        other => panic!("from.value should be Number, got {:?}", other),
    }

    match &to.value {
        Some(CellValue::Number(n)) => {
            assert!(
                (*n - 120.0).abs() < 0.001,
                "to.value should be 120.0, got {n}"
            );
        }
        other => panic!("to.value should be Number, got {:?}", other),
    }
}
