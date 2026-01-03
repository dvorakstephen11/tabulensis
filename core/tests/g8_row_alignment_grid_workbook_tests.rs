mod common;

use common::diff_fixture_pkgs;
use excel_diff::{DiffConfig, DiffOp};

#[test]
fn single_row_insert_middle_produces_one_row_added() {
    let report = diff_fixture_pkgs(
        "row_insert_middle_a.xlsx",
        "row_insert_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let strings = &report.strings;

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(
                    strings.get(sheet.0 as usize).map(String::as_str),
                    Some("Sheet1")
                );
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(rows_added, vec![5], "expected single RowAdded at index 5");

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "no rows should be removed for middle insert"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned insert should not emit CellEdited noise"
    );
}

#[test]
fn single_row_delete_middle_produces_one_row_removed() {
    let report = diff_fixture_pkgs(
        "row_delete_middle_a.xlsx",
        "row_delete_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let strings = &report.strings;

    let rows_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(
                    strings.get(sheet.0 as usize).map(String::as_str),
                    Some("Sheet1")
                );
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_removed,
        vec![5],
        "expected single RowRemoved at index 5"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "no rows should be added for middle delete"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned delete should not emit CellEdited noise"
    );
}

#[test]
fn alignment_bails_out_when_additional_edits_present() {
    let report = diff_fixture_pkgs(
        "row_insert_with_edit_a.xlsx",
        "row_insert_with_edit_b.xlsx",
        &DiffConfig::default(),
    );

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded { row_idx, .. } => Some(*row_idx),
            _ => None,
        })
        .collect();

    assert!(
        rows_added.contains(&10),
        "fallback positional diff should add the tail row"
    );
    assert!(
        !rows_added.contains(&5),
        "mid-sheet RowAdded at 5 would indicate the alignment path was taken"
    );

    let edited_rows: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.row),
            _ => None,
        })
        .collect();

    assert!(
        !edited_rows.is_empty(),
        "fallback positional diff should surface cell edits after the inserted row"
    );
    assert!(
        edited_rows.iter().any(|row| *row >= 5),
        "cell edits should include rows at or below the insertion point"
    );
}
