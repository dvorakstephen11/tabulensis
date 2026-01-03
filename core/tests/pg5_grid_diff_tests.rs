mod common;

use common::{grid_from_numbers, single_sheet_workbook};
use excel_diff::{CellValue, DiffConfig, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage};
use std::collections::BTreeSet;

fn sheet_name<'a>(report: &'a DiffReport, id: &excel_diff::SheetId) -> &'a str {
    report.strings[id.0 as usize].as_str()
}

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn pg5_1_grid_diff_1x1_identical_empty_diff() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert!(report.ops.is_empty());
}

#[test]
fn pg5_2_grid_diff_1x1_value_change_single_cell_edited() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[2]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn pg5_3_grid_diff_row_appended_row_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowAdded, got {other:?}"),
    }
}

#[test]
fn pg5_4_grid_diff_column_appended_column_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnAdded, got {other:?}"),
    }
}

#[test]
fn pg5_5_grid_diff_same_shape_scattered_cell_edits() {
    let old = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]),
    );
    let new = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[10, 2, 3], &[4, 50, 6], &[7, 8, 90]]),
    );

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 3);
    assert!(
        report
            .ops
            .iter()
            .all(|op| matches!(op, DiffOp::CellEdited { .. }))
    );

    let edited_addrs: BTreeSet<String> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.to_a1()),
            _ => None,
        })
        .collect();
    let expected: BTreeSet<String> = ["A1", "B2", "C3"].into_iter().map(String::from).collect();
    assert_eq!(edited_addrs, expected);
}

#[test]
fn pg5_dense_row_replacement_emits_row_replaced() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2, 3, 4, 5]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[10, 20, 30, 40, 50]]));

    let config = DiffConfig::builder()
        .dense_row_replace_ratio(0.5)
        .dense_row_replace_min_cols(1)
        .dense_rect_replace_min_rows(0)
        .build()
        .expect("valid config should build");

    let report = diff_workbooks(&old, &new, &config);
    assert_eq!(report.ops.len(), 1);
    assert!(matches!(report.ops[0], DiffOp::RowReplaced { .. }));
}

#[test]
fn pg5_dense_rect_replacement_emits_rect_replaced() {
    let old = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6]]),
    );
    let new = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[10, 20, 30], &[40, 50, 60]]),
    );

    let config = DiffConfig::builder()
        .dense_row_replace_ratio(0.5)
        .dense_row_replace_min_cols(1)
        .dense_rect_replace_min_rows(2)
        .build()
        .expect("valid config should build");

    let report = diff_workbooks(&old, &new, &config);
    assert_eq!(report.ops.len(), 1);
    assert!(matches!(report.ops[0], DiffOp::RectReplaced { .. }));
}

#[test]
fn pg5_6_grid_diff_degenerate_grids() {
    let empty_old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let empty_new = single_sheet_workbook("Sheet1", Grid::new(0, 0));

    let empty_report = diff_workbooks(&empty_old, &empty_new, &DiffConfig::default());
    assert!(empty_report.ops.is_empty());

    let old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 2);

    let mut row_added = 0;
    let mut col_added = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*row_idx, 0);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*col_idx, 0);
                assert!(col_signature.is_none());
                col_added += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    assert_eq!(col_added, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_7_grid_diff_row_truncated_row_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_8_grid_diff_column_truncated_column_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet_name(&report, sheet), "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_9_grid_diff_row_and_column_truncated_structure_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 2);

    let mut rows_removed = 0;
    let mut cols_removed = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*row_idx, 1);
                assert!(row_signature.is_none());
                rows_removed += 1;
            }
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*col_idx, 1);
                assert!(col_signature.is_none());
                cols_removed += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(rows_removed, 1);
    assert_eq!(cols_removed, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_10_grid_diff_row_appended_with_overlap_cell_edits() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 20], &[30, 4], &[5, 6]]));

    let report = diff_workbooks(&old, &new, &DiffConfig::default());
    assert_eq!(report.ops.len(), 3);

    let mut row_added = 0;
    let mut cell_edits = BTreeSet::new();

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet_name(&report, sheet), "Sheet1");
                assert_eq!(*row_idx, 2);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::CellEdited { addr, .. } => {
                cell_edits.insert(addr.to_a1());
            }
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    let expected: BTreeSet<String> = ["B1", "A2"].into_iter().map(String::from).collect();
    assert_eq!(cell_edits, expected);
}
