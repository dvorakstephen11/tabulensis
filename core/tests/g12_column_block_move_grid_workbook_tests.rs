mod common;

use common::{diff_fixture_pkgs, grid_from_numbers, sid, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g12_column_move_emits_single_blockmovedcolumns() {
    let report = diff_fixture_pkgs(
        "column_move_a.xlsx",
        "column_move_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            assert_eq!(sheet, &sid("Data"));
            assert_eq!(*src_start_col, 2);
            assert_eq!(*col_count, 1);
            assert_eq!(*dst_start_col, 5);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedColumns op, got {:?}", other),
    }

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::ColumnAdded { .. })),
        "pure move should not emit ColumnAdded"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::ColumnRemoved { .. })),
        "pure move should not emit ColumnRemoved"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. })),
        "pure move should not emit row operations"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "pure move should not emit CellEdited noise"
    );
}

#[test]
fn g12_repeated_columns_do_not_emit_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[1, 1, 2, 2], &[10, 10, 20, 20]]);
    let grid_b = grid_from_numbers(&[&[2, 2, 1, 1], &[20, 20, 10, 10]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. })),
        "ambiguous repeated columns must not emit BlockMovedColumns"
    );

    assert!(
        report.ops.iter().any(|op| matches!(
            op,
            DiffOp::CellEdited { .. } | DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. }
        )),
        "fallback path should emit some other diff operation"
    );
}

#[test]
fn g12_multi_column_block_move_emits_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[
        &[10, 20, 30, 40, 50, 60],
        &[11, 21, 31, 41, 51, 61],
        &[12, 22, 32, 42, 52, 62],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 40, 50, 20, 30, 60],
        &[11, 41, 51, 21, 31, 61],
        &[12, 42, 52, 22, 32, 62],
    ]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert_eq!(
        report.ops.len(),
        1,
        "expected a single diff op for multi-column move"
    );

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            assert_eq!(sheet, &sid("Data"));
            assert_eq!(*src_start_col, 3);
            assert_eq!(*col_count, 2, "should detect a 2-column block move");
            assert_eq!(*dst_start_col, 1);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedColumns op, got {:?}", other),
    }
}

#[test]
fn g12_two_independent_column_moves_do_not_emit_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[10, 20, 30, 40, 50, 60], &[11, 21, 31, 41, 51, 61]]);

    let grid_b = grid_from_numbers(&[&[20, 10, 30, 40, 60, 50], &[21, 11, 31, 41, 61, 51]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. })),
        "two independent column swaps must not emit BlockMovedColumns"
    );

    assert!(
        !report.ops.is_empty(),
        "fallback path should emit some diff operations"
    );
}

#[test]
fn g12_column_swap_emits_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

    let grid_b = grid_from_numbers(&[&[20, 10, 30, 40], &[21, 11, 31, 41]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert_eq!(
        report.ops.len(),
        1,
        "swap should produce single BlockMovedColumns op"
    );

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            col_count,
            src_start_col,
            dst_start_col,
            ..
        } => {
            assert_eq!(sheet, &sid("Data"));
            assert_eq!(*col_count, 1, "swap is represented as moving one column");
            assert!(
                (*src_start_col == 0 && *dst_start_col == 1)
                    || (*src_start_col == 1 && *dst_start_col == 0),
                "swap should move column 0 or 1 past the other"
            );
        }
        other => panic!("expected BlockMovedColumns, got {:?}", other),
    }
}
