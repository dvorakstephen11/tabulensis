mod common;

use common::{grid_from_numbers, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn count_ops(ops: &[DiffOp], predicate: impl Fn(&DiffOp) -> bool) -> usize {
    ops.iter().filter(|op| predicate(op)).count()
}

fn count_row_added(ops: &[DiffOp]) -> usize {
    count_ops(ops, |op| matches!(op, DiffOp::RowAdded { .. }))
}

fn count_row_removed(ops: &[DiffOp]) -> usize {
    count_ops(ops, |op| matches!(op, DiffOp::RowRemoved { .. }))
}

fn count_block_moved_rows(ops: &[DiffOp]) -> usize {
    count_ops(ops, |op| matches!(op, DiffOp::BlockMovedRows { .. }))
}

#[test]
fn amr_two_disjoint_insertion_regions() {
    let grid_a = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 11, 12],
        &[100, 101, 102],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[200, 201, 202],
        &[201, 202, 203],
        &[50, 51, 52],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        3,
        "should detect 3 inserted rows across 2 disjoint regions"
    );
    assert_eq!(
        count_row_removed(&report.ops),
        0,
        "should not detect any removed rows"
    );
}

#[test]
fn amr_insertion_and_deletion_in_different_regions() {
    let grid_a = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[90, 91, 92],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[100, 101, 102],
        &[50, 51, 52],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        1,
        "should detect 1 inserted row near the tail"
    );
    assert_eq!(
        count_row_removed(&report.ops),
        1,
        "should detect 1 deleted row in the middle"
    );
}

#[test]
fn amr_gap_contains_moved_block_scenario() {
    let grid_a = grid_from_numbers(&[
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
        &[60, 61, 62],
        &[70, 71, 72],
        &[80, 81, 82],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 11, 12],
        &[60, 61, 62],
        &[70, 71, 72],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
        &[80, 81, 82],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    let moves = count_block_moved_rows(&report.ops);
    assert!(
        moves >= 1,
        "should detect at least one block move (rows 60-70 moved up)"
    );
    assert_eq!(
        count_row_added(&report.ops),
        0,
        "should not report spurious insertions when move is detected"
    );
    assert_eq!(
        count_row_removed(&report.ops),
        0,
        "should not report spurious deletions when move is detected"
    );
}

#[test]
fn amr_multiple_anchors_with_gaps() {
    let grid_a = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 11, 12],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[50, 51, 52],
        &[60, 61, 62],
        &[70, 71, 72],
    ]);

    let grid_b = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 11, 12],
        &[100, 101, 102],
        &[20, 21, 22],
        &[30, 31, 32],
        &[40, 41, 42],
        &[200, 201, 202],
        &[50, 51, 52],
        &[60, 61, 62],
        &[70, 71, 72],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        2,
        "should detect both inserted rows in separate gaps between anchors"
    );
}

#[test]
fn amr_recursive_gap_alignment() {
    let values_a: Vec<&[i32]> = (1..=50i32)
        .map(|i| {
            let row: &[i32] = match i {
                1 => &[10, 11, 12],
                2 => &[20, 21, 22],
                3 => &[30, 31, 32],
                4 => &[40, 41, 42],
                5 => &[50, 51, 52],
                6 => &[60, 61, 62],
                7 => &[70, 71, 72],
                8 => &[80, 81, 82],
                9 => &[90, 91, 92],
                10 => &[100, 101, 102],
                11 => &[110, 111, 112],
                12 => &[120, 121, 122],
                13 => &[130, 131, 132],
                14 => &[140, 141, 142],
                15 => &[150, 151, 152],
                16 => &[160, 161, 162],
                17 => &[170, 171, 172],
                18 => &[180, 181, 182],
                19 => &[190, 191, 192],
                20 => &[200, 201, 202],
                21 => &[210, 211, 212],
                22 => &[220, 221, 222],
                23 => &[230, 231, 232],
                24 => &[240, 241, 242],
                25 => &[250, 251, 252],
                26 => &[260, 261, 262],
                27 => &[270, 271, 272],
                28 => &[280, 281, 282],
                29 => &[290, 291, 292],
                30 => &[300, 301, 302],
                31 => &[310, 311, 312],
                32 => &[320, 321, 322],
                33 => &[330, 331, 332],
                34 => &[340, 341, 342],
                35 => &[350, 351, 352],
                36 => &[360, 361, 362],
                37 => &[370, 371, 372],
                38 => &[380, 381, 382],
                39 => &[390, 391, 392],
                40 => &[400, 401, 402],
                41 => &[410, 411, 412],
                42 => &[420, 421, 422],
                43 => &[430, 431, 432],
                44 => &[440, 441, 442],
                45 => &[450, 451, 452],
                46 => &[460, 461, 462],
                47 => &[470, 471, 472],
                48 => &[480, 481, 482],
                49 => &[490, 491, 492],
                50 => &[500, 501, 502],
                _ => &[0, 0, 0],
            };
            row
        })
        .collect();
    let grid_a = grid_from_numbers(&values_a);

    let mut values_b: Vec<&[i32]> = values_a.clone();
    values_b.insert(10, &[1000, 1001, 1002]);
    values_b.insert(25, &[2000, 2001, 2002]);
    values_b.insert(40, &[3000, 3001, 3002]);

    let grid_b = grid_from_numbers(&values_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);
    let config = DiffConfig::default();

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "diff should be complete without hitting limits"
    );
    assert_eq!(
        count_row_added(&report.ops),
        3,
        "should detect all 3 inserted rows distributed across the grid"
    );
}
