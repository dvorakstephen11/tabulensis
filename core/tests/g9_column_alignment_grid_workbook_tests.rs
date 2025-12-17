mod common;

use common::{diff_fixture_pkgs, open_fixture_workbook, sid};
use excel_diff::{CellValue, DiffConfig, DiffOp, Workbook};

#[test]
fn g9_col_insert_middle_emits_one_columnadded_and_no_noise() {
    let report = diff_fixture_pkgs(
        "col_insert_middle_a.xlsx",
        "col_insert_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let cols_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, &sid("Data"));
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        cols_added,
        vec![3],
        "expected single ColumnAdded at inserted position"
    );

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnRemoved { .. } | DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }
        )),
        "column insert should not emit row ops or ColumnRemoved"
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
fn g9_col_delete_middle_emits_one_columnremoved_and_no_noise() {
    let report = diff_fixture_pkgs(
        "col_delete_middle_a.xlsx",
        "col_delete_middle_b.xlsx",
        &DiffConfig::default(),
    );

    let cols_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, &sid("Data"));
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        cols_removed,
        vec![3],
        "expected single ColumnRemoved at deleted position"
    );

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }
        )),
        "column delete should not emit ColumnAdded or row ops"
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
fn g9_alignment_bails_out_when_additional_edits_present() {
    let wb_b = open_fixture_workbook("col_insert_with_edit_b.xlsx");
    let report = diff_fixture_pkgs(
        "col_insert_with_edit_a.xlsx",
        "col_insert_with_edit_b.xlsx",
        &DiffConfig::default(),
    );
    let inserted_idx = find_header_col(&wb_b, "Inserted");

    let has_middle_column_add = report.ops.iter().any(|op| match op {
        DiffOp::ColumnAdded { col_idx, .. } => *col_idx == inserted_idx,
        _ => false,
    });
    assert!(
        !has_middle_column_add,
        "alignment should bail out; no ColumnAdded at the inserted index"
    );

    let edited_cols: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.col),
            _ => None,
        })
        .collect();

    assert!(
        !edited_cols.is_empty(),
        "fallback positional diff should emit CellEdited ops"
    );
    assert!(
        edited_cols.iter().any(|col| *col > inserted_idx),
        "CellEdited ops should appear in columns to the right of the insertion"
    );
}

fn find_header_col(workbook: &Workbook, header: &str) -> u32 {
    let header_id = sid(header);
    workbook
        .sheets
        .iter()
        .flat_map(|sheet| sheet.grid.cells.iter())
        .find_map(|((row, col), cell)| match &cell.value {
            Some(CellValue::Text(text)) if *row == 0 && *text == header_id => Some(*col),
            _ => None,
        })
        .expect("header column should exist in fixture")
}
