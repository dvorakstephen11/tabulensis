mod common;

use common::{grid_from_numbers, open_fixture_workbook, sid};
use excel_diff::{
    CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage,
    diff_grids_database_mode, with_default_session,
};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

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

fn grid_from_float_rows(rows: &[&[f64]]) -> Grid {
    let nrows = rows.len() as u32;
    let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
    let mut grid = Grid::new(nrows, ncols);

    for (r_idx, row_vals) in rows.iter().enumerate() {
        for (c_idx, value) in row_vals.iter().enumerate() {
            grid.insert_cell(
                r_idx as u32,
                c_idx as u32,
                Some(CellValue::Number(*value)),
                None,
            );
        }
    }

    grid
}

#[test]
fn d1_equal_ordered_database_mode_empty_diff() {
    let workbook = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let grid = data_grid(&workbook);

    let report = diff_db(grid, grid, &[0]);
    assert!(
        report.ops.is_empty(),
        "database mode should ignore row order when keyed rows are identical"
    );
}

#[test]
fn d1_equal_reordered_database_mode_empty_diff() {
    let wb_a = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let wb_b = open_fixture_workbook("db_equal_ordered_b.xlsx");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_db(grid_a, grid_b, &[0]);
    assert!(
        report.ops.is_empty(),
        "keyed alignment should match rows by key and ignore reordering"
    );
}

#[test]
fn d1_spreadsheet_mode_sees_reorder_as_changes() {
    let wb_a = open_fixture_workbook("db_equal_ordered_a.xlsx");
    let wb_b = open_fixture_workbook("db_equal_ordered_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "Spreadsheet Mode should see structural changes when rows are reordered, \
         demonstrating the semantic difference from Database Mode"
    );
}

#[test]
fn d1_duplicate_keys_fallback_to_spreadsheet_mode() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[1, 99]]);
    let grid_b = grid_from_numbers(&[&[1, 10]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    assert!(
        !report.ops.is_empty(),
        "duplicate keys cause fallback to spreadsheet mode which should detect differences"
    );

    let has_row_removed = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::RowRemoved { .. }));
    assert!(
        has_row_removed,
        "spreadsheet mode fallback should emit RowRemoved for the missing row"
    );
}

#[test]
fn d1_database_mode_row_added() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20]]);
    let grid_b = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "database mode should emit one RowAdded for key 3"
    );
}

#[test]
fn d1_database_mode_row_removed() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);
    let grid_b = grid_from_numbers(&[&[1, 10], &[2, 20]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "database mode should emit one RowRemoved for key 3"
    );
}

#[test]
fn d1_database_mode_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20]]);
    let grid_b = grid_from_numbers(&[&[1, 99], &[2, 20]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "database mode should emit one CellEdited for the changed non-key cell"
    );
}

#[test]
fn d1_database_mode_cell_edited_with_reorder() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);
    let grid_b = grid_from_numbers(&[&[3, 30], &[2, 99], &[1, 10]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "database mode should ignore reordering and find only the cell edit for key 2"
    );
}

#[test]
fn d1_database_mode_treats_small_float_key_noise_as_equal() {
    let grid_a = grid_from_float_rows(&[&[1.0, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);
    let grid_b = grid_from_float_rows(&[&[1.0000000000000002, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);
    assert!(
        report.ops.is_empty(),
        "ULP-level noise in key column should not break row alignment"
    );
}

#[test]
fn d1_database_mode_detects_meaningful_float_key_change() {
    let grid_a = grid_from_float_rows(&[&[1.0, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);
    let grid_b = grid_from_float_rows(&[&[1.0001, 10.0], &[2.0, 20.0], &[3.0, 30.0]]);

    let report = diff_db(&grid_a, &grid_b, &[0]);

    let row_removed = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    let row_added = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();

    assert_eq!(
        row_removed, 1,
        "meaningful key drift should remove the original keyed row"
    );
    assert_eq!(
        row_added, 1,
        "meaningful key drift should add the new keyed row"
    );
}

#[test]
fn d5_composite_key_equal_reordered_database_mode_empty_diff() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 20, 200], &[2, 10, 300]]);
    let grid_b = grid_from_numbers(&[&[2, 10, 300], &[1, 10, 100], &[1, 20, 200]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);
    assert!(
        report.ops.is_empty(),
        "composite keyed alignment should ignore row order differences"
    );
}

#[test]
fn d5_composite_key_row_added_and_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 20, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 150], &[1, 20, 200], &[2, 30, 300]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "new composite key should produce exactly one RowAdded"
    );

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 0,
        "no rows should be removed when only a new composite key is introduced"
    );

    let mut cell_edited_iter = report.ops.iter().filter_map(|op| {
        if let DiffOp::CellEdited { addr, .. } = op {
            Some(addr)
        } else {
            None
        }
    });

    let edited_addr = cell_edited_iter
        .next()
        .expect("one cell edit for changed non-key value");
    assert!(
        cell_edited_iter.next().is_none(),
        "only one CellEdited should be present"
    );
    assert_eq!(edited_addr.col, 2, "only non-key column should be edited");
    assert_eq!(
        edited_addr.row, 0,
        "cell edit should reference the row of key (1,10) in the new grid"
    );
}

#[test]
fn d5_composite_key_partial_key_mismatch_yields_add_and_remove() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100]]);
    let grid_b = grid_from_numbers(&[&[1, 20, 100]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "changed composite key should remove the old tuple"
    );

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "changed composite key should add the new tuple"
    );

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 0,
        "partial key match must not be treated as a cell edit"
    );
}

#[test]
fn d5_composite_key_duplicate_keys_fallback_to_spreadsheet_mode() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 10, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 100]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1]);

    assert!(
        !report.ops.is_empty(),
        "duplicate composite keys should trigger spreadsheet-mode fallback"
    );

    let has_row_removed = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::RowRemoved { .. }));
    assert!(
        has_row_removed,
        "fallback should emit a RowRemoved reflecting duplicate handling"
    );
}

#[test]
fn d5_non_contiguous_key_columns_equal_reordered_empty_diff() {
    let grid_a = grid_from_numbers(&[&[1, 999, 10, 100], &[1, 888, 20, 200], &[2, 777, 10, 300]]);
    let grid_b = grid_from_numbers(&[&[2, 777, 10, 300], &[1, 999, 10, 100], &[1, 888, 20, 200]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 2]);
    assert!(
        report.ops.is_empty(),
        "non-contiguous key columns [0,2] should align correctly ignoring row order"
    );
}

#[test]
fn d5_non_contiguous_key_columns_detects_edits_in_skipped_column() {
    let grid_a = grid_from_numbers(&[&[1, 999, 10, 100], &[1, 888, 20, 200], &[2, 777, 10, 300]]);
    let grid_b = grid_from_numbers(&[&[2, 111, 10, 300], &[1, 222, 10, 100], &[1, 333, 20, 200]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 2]);

    let cell_edited_ops: Vec<_> = report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, .. } = op {
                Some(addr)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        cell_edited_ops.len(),
        3,
        "should detect 3 edits in skipped non-key column 1"
    );

    for addr in &cell_edited_ops {
        assert_eq!(
            addr.col, 1,
            "all edits should be in the skipped column 1, not key columns 0 or 2"
        );
    }

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(row_added_count, 0, "no rows should be added");

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(row_removed_count, 0, "no rows should be removed");
}

#[test]
fn d5_non_contiguous_key_columns_row_added_and_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 999, 10, 100], &[1, 888, 20, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 999, 10, 150], &[1, 888, 20, 200], &[2, 777, 30, 300]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 2]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "new non-contiguous composite key should produce exactly one RowAdded"
    );

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(row_removed_count, 0, "no rows should be removed");

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "changed non-key column should produce exactly one CellEdited"
    );
}

#[test]
fn d5_three_column_composite_key_equal_reordered_empty_diff() {
    let grid_a = grid_from_numbers(&[
        &[1, 10, 100, 1000],
        &[1, 10, 200, 2000],
        &[1, 20, 100, 3000],
        &[2, 10, 100, 4000],
    ]);
    let grid_b = grid_from_numbers(&[
        &[2, 10, 100, 4000],
        &[1, 20, 100, 3000],
        &[1, 10, 200, 2000],
        &[1, 10, 100, 1000],
    ]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1, 2]);
    assert!(
        report.ops.is_empty(),
        "three-column composite key should align correctly ignoring row order"
    );
}

#[test]
fn d5_three_column_composite_key_partial_match_yields_add_and_remove() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100, 1000]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 200, 1000]]);

    let report = diff_db(&grid_a, &grid_b, &[0, 1, 2]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "changed third key column should remove the old tuple"
    );

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "changed third key column should add the new tuple"
    );

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 0,
        "partial three-column key match must not be treated as a cell edit"
    );
}
