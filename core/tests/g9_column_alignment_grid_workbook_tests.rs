use excel_diff::{CellValue, DiffOp, Workbook, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn g9_col_insert_middle_emits_one_columnadded_and_no_noise() {
    let wb_a = open_workbook(fixture_path("col_insert_middle_a.xlsx"))
        .expect("failed to open fixture: col_insert_middle_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_insert_middle_b.xlsx"))
        .expect("failed to open fixture: col_insert_middle_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());

    let cols_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Data");
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
    let wb_a = open_workbook(fixture_path("col_delete_middle_a.xlsx"))
        .expect("failed to open fixture: col_delete_middle_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_delete_middle_b.xlsx"))
        .expect("failed to open fixture: col_delete_middle_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());

    let cols_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Data");
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
    let wb_a = open_workbook(fixture_path("col_insert_with_edit_a.xlsx"))
        .expect("failed to open fixture: col_insert_with_edit_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_insert_with_edit_b.xlsx"))
        .expect("failed to open fixture: col_insert_with_edit_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());
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
    workbook
        .sheets
        .iter()
        .flat_map(|sheet| sheet.grid.cells.iter())
        .find_map(|((row, col), cell)| match &cell.value {
            Some(CellValue::Text(text)) if *row == 0 && text == header => Some(*col),
            _ => None,
        })
        .expect("header column should exist in fixture")
}
