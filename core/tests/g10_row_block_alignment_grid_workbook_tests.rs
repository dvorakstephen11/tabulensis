mod common;

use common::{diff_fixture_pkgs, sid};
use excel_diff::{DiffConfig, DiffOp};

#[test]
fn g10_row_block_insert_middle_emits_four_rowadded_and_no_noise() {
    let report = diff_fixture_pkgs(
        "row_block_insert_a.xlsx",
        "row_block_insert_b.xlsx",
        &DiffConfig::default(),
    );

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(*sheet, sid("Sheet1"));
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_added,
        vec![3, 4, 5, 6],
        "expected four RowAdded ops for the inserted block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "no rows should be removed for block insert"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned block insert should not emit CellEdited noise"
    );
}

#[test]
fn g10_row_block_delete_middle_emits_four_rowremoved_and_no_noise() {
    let report = diff_fixture_pkgs(
        "row_block_delete_a.xlsx",
        "row_block_delete_b.xlsx",
        &DiffConfig::default(),
    );

    let rows_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(*sheet, sid("Sheet1"));
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_removed,
        vec![3, 4, 5, 6],
        "expected four RowRemoved ops for the deleted block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "no rows should be added for block delete"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned block delete should not emit CellEdited noise"
    );
}
