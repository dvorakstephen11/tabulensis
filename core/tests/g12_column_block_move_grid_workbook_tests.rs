use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers, single_sheet_workbook};

#[test]
fn g12_column_move_emits_single_blockmovedcolumns() {
    let wb_a = open_workbook(fixture_path("column_move_a.xlsx"))
        .expect("failed to open fixture: column_move_a.xlsx");
    let wb_b = open_workbook(fixture_path("column_move_b.xlsx"))
        .expect("failed to open fixture: column_move_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            assert_eq!(sheet, "Data");
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

    let report = diff_workbooks(&wb_a, &wb_b);

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
