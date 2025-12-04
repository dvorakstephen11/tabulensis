use excel_diff::{CellValue, DiffOp, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn g1_equal_sheet_produces_empty_diff() {
    let wb_a =
        open_workbook(fixture_path("equal_sheet_a.xlsx")).expect("equal_sheet_a.xlsx should open");
    let wb_b =
        open_workbook(fixture_path("equal_sheet_b.xlsx")).expect("equal_sheet_b.xlsx should open");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        report.ops.is_empty(),
        "identical 5x5 sheet should produce an empty diff"
    );
}

#[test]
fn g2_single_cell_literal_change_produces_one_celledited() {
    let wb_a = open_workbook(fixture_path("single_cell_value_a.xlsx"))
        .expect("single_cell_value_a.xlsx should open");
    let wb_b = open_workbook(fixture_path("single_cell_value_b.xlsx"))
        .expect("single_cell_value_b.xlsx should open");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        1,
        "expected exactly one diff op for a single edited cell"
    );

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
            assert_eq!(from.formula, to.formula, "no formula changes expected");
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
        )),
        "single cell change should not produce row/column structure ops"
    );
}
