use excel_diff::{Grid, Workbook, diff_grids_database_mode, open_workbook};

mod common;
use common::fixture_path;

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
