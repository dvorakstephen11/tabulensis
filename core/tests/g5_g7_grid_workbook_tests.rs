use excel_diff::{CellValue, DiffOp, diff_workbooks, open_workbook};
use std::collections::BTreeSet;

mod common;
use common::fixture_path;

#[test]
fn g5_multi_cell_edits_produces_only_celledited_ops() {
    let wb_a = open_workbook(fixture_path("multi_cell_edits_a.xlsx"))
        .expect("multi_cell_edits_a.xlsx should open");
    let wb_b = open_workbook(fixture_path("multi_cell_edits_b.xlsx"))
        .expect("multi_cell_edits_b.xlsx should open");

    let report = diff_workbooks(&wb_a, &wb_b);

    let expected = vec![
        ("B2", CellValue::Number(1.0), CellValue::Number(42.0)),
        ("D5", CellValue::Number(2.0), CellValue::Number(99.0)),
        ("H7", CellValue::Number(3.0), CellValue::Number(3.5)),
        (
            "J10",
            CellValue::Text("x".into()),
            CellValue::Text("y".into()),
        ),
    ];

    assert_eq!(
        report.ops.len(),
        expected.len(),
        "expected one DiffOp per configured edit"
    );
    assert!(
        report
            .ops
            .iter()
            .all(|op| matches!(op, DiffOp::CellEdited { .. })),
        "multi-cell edits should produce only CellEdited ops"
    );

    for (addr, expected_from, expected_to) in expected {
        let (sheet, from, to) = report
            .ops
            .iter()
            .find_map(|op| match op {
                DiffOp::CellEdited {
                    sheet,
                    addr: a,
                    from,
                    to,
                } if a.to_a1() == addr => Some((sheet, from, to)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing CellEdited for {addr}"));

        assert_eq!(sheet, "Sheet1");
        assert_eq!(from.value, Some(expected_from));
        assert_eq!(to.value, Some(expected_to));
        assert_eq!(from.formula, to.formula, "no formula changes expected");
    }

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
        )),
        "multi-cell edits should not produce row/column structure ops"
    );
}

#[test]
fn g6_row_append_bottom_emits_two_rowadded_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("row_append_bottom_a.xlsx"))
        .expect("row_append_bottom_a.xlsx should open");
    let wb_b = open_workbook(fixture_path("row_append_bottom_b.xlsx"))
        .expect("row_append_bottom_b.xlsx should open");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two RowAdded ops for appended rows"
    );

    let rows_added: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [10u32, 11u32].into_iter().collect();
    assert_eq!(rows_added, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "row append should not emit removals, column ops, or cell edits"
    );
}

#[test]
fn g6_row_delete_bottom_emits_two_rowremoved_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("row_delete_bottom_a.xlsx"))
        .expect("row_delete_bottom_a.xlsx should open");
    let wb_b = open_workbook(fixture_path("row_delete_bottom_b.xlsx"))
        .expect("row_delete_bottom_b.xlsx should open");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two RowRemoved ops for deleted rows"
    );

    let rows_removed: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [10u32, 11u32].into_iter().collect();
    assert_eq!(rows_removed, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "row delete should not emit additions, column ops, or cell edits"
    );
}

#[test]
fn g7_col_append_right_emits_two_columnadded_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("col_append_right_a.xlsx"))
        .expect("col_append_right_a.xlsx should open");
    let wb_b = open_workbook(fixture_path("col_append_right_b.xlsx"))
        .expect("col_append_right_b.xlsx should open");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two ColumnAdded ops for appended columns"
    );

    let cols_added: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [4u32, 5u32].into_iter().collect();
    assert_eq!(cols_added, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnRemoved { .. }
                | DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "column append should not emit removals, row ops, or cell edits"
    );
}

#[test]
fn g7_col_delete_right_emits_two_columnremoved_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("col_delete_right_a.xlsx"))
        .expect("col_delete_right_a.xlsx should open");
    let wb_b = open_workbook(fixture_path("col_delete_right_b.xlsx"))
        .expect("col_delete_right_b.xlsx should open");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two ColumnRemoved ops for deleted columns"
    );

    let cols_removed: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [4u32, 5u32].into_iter().collect();
    assert_eq!(cols_removed, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnAdded { .. }
                | DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "column delete should not emit additions, row ops, or cell edits"
    );
}
