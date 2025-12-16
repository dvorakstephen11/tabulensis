mod common;

use common::{diff_fixture_pkgs, grid_from_numbers, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, DiffReport, Workbook, WorkbookPackage};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g11_row_block_move_emits_single_blockmovedrows() {
    let report = diff_fixture_pkgs(
        "row_block_move_a.xlsx",
        "row_block_move_b.xlsx",
        &DiffConfig::default(),
    );

    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    let strings = &report.strings;

    match &report.ops[0] {
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        } => {
            assert_eq!(
                strings.get(sheet.0 as usize).map(String::as_str),
                Some("Sheet1")
            );
            assert_eq!(*src_start_row, 4);
            assert_eq!(*row_count, 4);
            assert_eq!(*dst_start_row, 12);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedRows op, got {:?}", other),
    }

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "pure move should not emit RowAdded"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "pure move should not emit RowRemoved"
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
fn g11_repeated_rows_do_not_emit_blockmove() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[1, 10], &[2, 20], &[2, 20]]);

    let grid_b = grid_from_numbers(&[&[2, 20], &[2, 20], &[1, 10], &[1, 10]]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "ambiguous repeated rows must not emit BlockMovedRows"
    );

    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "fallback path should emit positional CellEdited noise"
    );
}
