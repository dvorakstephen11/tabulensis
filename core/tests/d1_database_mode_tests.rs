use excel_diff::{
    DiffOp, Grid, Workbook, diff_grids_database_mode, diff_workbooks, open_workbook,
};

mod common;
use common::{fixture_path, grid_from_numbers};

fn data_grid(workbook: &Workbook) -> &Grid {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == "Data")
        .map(|s| &s.grid)
        .expect("Data sheet present")
}

#[test]
fn d1_equal_ordered_database_mode_empty_diff() {
    let workbook = open_workbook(fixture_path("db_equal_ordered_a.xlsx")).expect("fixture A opens");
    let grid = data_grid(&workbook);

    let report = diff_grids_database_mode(grid, grid, &[0]);
    assert!(
        report.ops.is_empty(),
        "database mode should ignore row order when keyed rows are identical"
    );
}

#[test]
fn d1_equal_reordered_database_mode_empty_diff() {
    let wb_a = open_workbook(fixture_path("db_equal_ordered_a.xlsx")).expect("fixture A opens");
    let wb_b = open_workbook(fixture_path("db_equal_ordered_b.xlsx")).expect("fixture B opens");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_grids_database_mode(grid_a, grid_b, &[0]);
    assert!(
        report.ops.is_empty(),
        "keyed alignment should match rows by key and ignore reordering"
    );
}

#[test]
fn d1_spreadsheet_mode_sees_reorder_as_changes() {
    let wb_a = open_workbook(fixture_path("db_equal_ordered_a.xlsx")).expect("fixture A opens");
    let wb_b = open_workbook(fixture_path("db_equal_ordered_b.xlsx")).expect("fixture B opens");

    let report = diff_workbooks(&wb_a, &wb_b);

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

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

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

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

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

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

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

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

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

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

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
