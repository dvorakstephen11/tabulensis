mod common;

use common::{diff_fixture_pkgs, sid};
use excel_diff::{
    CellValue, DiffConfig, DiffOp, DiffReport, Grid, Sheet, SheetKind, Workbook, WorkbookPackage,
};

fn workbook_with_number(value: f64) -> Workbook {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(value)), None);

    Workbook {
        sheets: vec![Sheet {
            name: sid("Sheet1"),
            kind: SheetKind::Worksheet,
            grid,
        }],
    }
}

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn g1_equal_sheet_produces_empty_diff() {
    let report = diff_fixture_pkgs(
        "equal_sheet_a.xlsx",
        "equal_sheet_b.xlsx",
        &DiffConfig::default(),
    );

    assert!(
        report.ops.is_empty(),
        "identical 5x5 sheet should produce an empty diff"
    );
}

#[test]
fn g2_single_cell_literal_change_produces_one_celledited() {
    let report = diff_fixture_pkgs(
        "single_cell_value_a.xlsx",
        "single_cell_value_b.xlsx",
        &DiffConfig::default(),
    );

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
            ..
        } => {
            assert_eq!(*sheet, sid("Sheet1"));
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

#[test]
fn g2_float_ulp_noise_is_ignored_in_diff() {
    let old = workbook_with_number(1.0);
    let new = workbook_with_number(1.0000000000000002);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    assert!(
        report.ops.is_empty(),
        "ULP-level float drift should not produce a diff op"
    );
}

#[test]
fn g2_meaningful_float_change_emits_cell_edit() {
    let old = workbook_with_number(1.0);
    let new = workbook_with_number(1.0001);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    assert_eq!(
        report.ops.len(),
        1,
        "meaningful float change should produce exactly one diff op"
    );

    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(1.0001)));
        }
        other => panic!("expected CellEdited diff op, got {other:?}"),
    }
}

#[test]
fn g2_nan_values_are_treated_as_equal() {
    let signaling_nan = f64::from_bits(0x7ff8_0000_0000_0000);
    let quiet_nan = f64::NAN;

    let old = workbook_with_number(signaling_nan);
    let new = workbook_with_number(quiet_nan);

    let report = diff_workbooks(&old, &new, &DiffConfig::default());

    assert!(
        report.ops.is_empty(),
        "different NaN bit patterns should be considered equal in diffing"
    );
}
