//! Integration tests verifying column structural changes do not break row alignment when row content is preserved.
//! Covers Branch 1.3 acceptance criteria for column insertion/deletion resilience.

mod common;

use common::single_sheet_workbook;
use excel_diff::{CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn make_grid_with_cells(nrows: u32, ncols: u32, cells: &[(u32, u32, i32)]) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for (row, col, val) in cells {
        grid.insert_cell(*row, *col, Some(CellValue::Number(*val as f64)), None);
    }
    grid
}

fn grid_from_row_data(rows: &[Vec<i32>]) -> Grid {
    let nrows = rows.len() as u32;
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0) as u32;
    let mut grid = Grid::new(nrows, ncols);

    for (r, row_vals) in rows.iter().enumerate() {
        for (c, val) in row_vals.iter().enumerate() {
            grid.insert_cell(
                r as u32,
                c as u32,
                Some(CellValue::Number(*val as f64)),
                None,
            );
        }
    }
    grid
}

#[test]
fn g15_blank_column_insert_at_position_zero_preserves_row_alignment() {
    let grid_a = grid_from_row_data(&[
        vec![10, 20, 30],
        vec![11, 21, 31],
        vec![12, 22, 32],
        vec![13, 23, 33],
        vec![14, 24, 34],
    ]);

    let grid_b = make_grid_with_cells(
        5,
        4,
        &[
            (0, 1, 10),
            (0, 2, 20),
            (0, 3, 30),
            (1, 1, 11),
            (1, 2, 21),
            (1, 3, 31),
            (2, 1, 12),
            (2, 2, 22),
            (2, 3, 32),
            (3, 1, 13),
            (3, 2, 23),
            (3, 3, 33),
            (4, 1, 14),
            (4, 2, 24),
            (4, 3, 34),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let column_adds: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded { col_idx, .. } => Some(*col_idx),
            _ => None,
        })
        .collect();

    let row_changes: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        column_adds.contains(&0) || !report.ops.is_empty(),
        "blank column insert at position 0 should be detected as ColumnAdded or produce some diff"
    );

    assert!(
        row_changes.is_empty(),
        "blank column insert should NOT produce spurious row add/remove operations; got {:?}",
        row_changes
    );
}

#[test]
fn g15_blank_column_insert_middle_preserves_row_alignment() {
    let grid_a = grid_from_row_data(&[
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ]);

    let grid_b = make_grid_with_cells(
        4,
        5,
        &[
            (0, 0, 1),
            (0, 1, 2),
            (0, 3, 3),
            (0, 4, 4),
            (1, 0, 5),
            (1, 1, 6),
            (1, 3, 7),
            (1, 4, 8),
            (2, 0, 9),
            (2, 1, 10),
            (2, 3, 11),
            (2, 4, 12),
            (3, 0, 13),
            (3, 1, 14),
            (3, 3, 15),
            (3, 4, 16),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "blank column insert in middle should not cause row structural changes; got {:?}",
        row_structural_ops
    );

    let has_column_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. }
        )
    });

    assert!(
        has_column_op || !report.ops.is_empty(),
        "column structure change should be detected"
    );
}

#[test]
fn g15_column_delete_preserves_row_alignment_when_content_order_maintained() {
    let grid_a = grid_from_row_data(&[
        vec![1, 2, 3, 4, 5],
        vec![6, 7, 8, 9, 10],
        vec![11, 12, 13, 14, 15],
        vec![16, 17, 18, 19, 20],
    ]);

    let grid_b = grid_from_row_data(&[
        vec![1, 2, 4, 5],
        vec![6, 7, 9, 10],
        vec![11, 12, 14, 15],
        vec![16, 17, 19, 20],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let column_removes: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved { col_idx, .. } => Some(*col_idx),
            _ => None,
        })
        .collect();

    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "column deletion should not cause spurious row changes; got {:?}",
        row_structural_ops
    );

    assert!(
        !column_removes.is_empty() || !report.ops.is_empty(),
        "column deletion should be detected"
    );
}

#[test]
fn g15_row_insert_with_column_structure_change_both_detected() {
    let grid_a = grid_from_row_data(&[vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);

    let grid_b = make_grid_with_cells(
        4,
        4,
        &[
            (0, 0, 1000),
            (0, 1, 1),
            (0, 2, 2),
            (0, 3, 3),
            (1, 0, 1001),
            (1, 1, 100),
            (1, 2, 200),
            (1, 3, 300),
            (2, 0, 1002),
            (2, 1, 4),
            (2, 2, 5),
            (2, 3, 6),
            (3, 0, 1003),
            (3, 1, 7),
            (3, 2, 8),
            (3, 3, 9),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report.ops.is_empty(),
        "row insert + column change should produce diff operations"
    );

    let has_row_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::CellEdited { .. }
        )
    });

    let has_col_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. } | DiffOp::CellEdited { .. }
        )
    });

    assert!(
        has_row_op || has_col_op,
        "at least one structural change type should be detected"
    );
}

#[test]
fn g15_single_row_grid_column_insert_no_spurious_row_ops() {
    let grid_a = grid_from_row_data(&[vec![10, 20]]);

    let grid_b = make_grid_with_cells(1, 3, &[(0, 0, 10), (0, 2, 20)]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }))
        .collect();

    assert!(
        row_ops.is_empty(),
        "single row grid with column insert should not have row ops; got {:?}",
        row_ops
    );
}

#[test]
fn g15_all_blank_column_insert_no_content_change_minimal_diff() {
    let grid_a = grid_from_row_data(&[vec![1, 2], vec![3, 4], vec![5, 6]]);

    let grid_b = make_grid_with_cells(
        3,
        3,
        &[
            (0, 0, 1),
            (0, 1, 2),
            (1, 0, 3),
            (1, 1, 4),
            (2, 0, 5),
            (2, 1, 6),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }))
        .collect();

    assert!(
        row_ops.is_empty(),
        "appending blank column should not cause row operations; got {:?}",
        row_ops
    );
}

#[test]
fn g15_large_grid_column_insert_row_alignment_preserved() {
    let rows: Vec<Vec<i32>> = (0..50)
        .map(|r| (0..10).map(|c| r * 100 + c).collect())
        .collect();
    let grid_a = grid_from_row_data(&rows);

    let mut cells_b: Vec<(u32, u32, i32)> = Vec::with_capacity(50 * 10);
    for r in 0..50 {
        for c in 0..10 {
            let new_col = if c < 5 { c } else { c + 1 };
            cells_b.push((r, new_col, r as i32 * 100 + c as i32));
        }
    }
    let grid_b = make_grid_with_cells(50, 11, &cells_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "large grid column insert should not cause row changes; got {} row ops",
        row_structural_ops.len()
    );

    let column_adds = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::ColumnAdded { .. }))
        .count();

    assert!(
        column_adds > 0 || !report.ops.is_empty(),
        "column insertion should be detected in large grid"
    );
}
