mod common;

use common::{sid, single_sheet_workbook};
use excel_diff::config::{DiffConfig, LimitBehavior};
use excel_diff::diff::{DiffError, DiffOp};
use excel_diff::{diff_workbooks, try_diff_workbooks};
use excel_diff::{CellValue, Grid};

fn create_simple_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                None,
            );
        }
    }
    grid
}

fn count_ops(ops: &[DiffOp], predicate: impl Fn(&DiffOp) -> bool) -> usize {
    ops.iter().filter(|op| predicate(op)).count()
}

#[test]
fn large_grid_completes_within_default_limits() {
    let grid_a = create_simple_grid(1000, 10, 0);
    let mut grid_b = create_simple_grid(1000, 10, 0);
    grid_b.insert_cell(500, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "1000-row grid should complete within default limits"
    );
    assert!(
        report.warnings.is_empty(),
        "should have no warnings for successful diff"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit"
    );
}

#[test]
fn limit_exceeded_fallback_to_positional() {
    let grid_a = create_simple_grid(100, 10, 0);
    let mut grid_b = create_simple_grid(100, 10, 0);
    grid_b.insert_cell(50, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::FallbackToPositional,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "FallbackToPositional should still produce a complete result"
    );
    assert!(
        report.warnings.is_empty(),
        "FallbackToPositional should not add warnings"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit via positional diff"
    );
}

#[test]
fn limit_exceeded_return_partial_result() {
    let grid_a = create_simple_grid(100, 10, 0);
    let mut grid_b = create_simple_grid(100, 10, 0);
    grid_b.insert_cell(50, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        !report.complete,
        "ReturnPartialResult should mark report as incomplete"
    );
    assert!(
        !report.warnings.is_empty(),
        "ReturnPartialResult should add a warning about limits exceeded"
    );
    assert!(
        report.warnings[0].contains("limits exceeded"),
        "warning should mention limits exceeded"
    );
    assert!(
        !report.ops.is_empty(),
        "ReturnPartialResult should still produce ops via positional diff"
    );
}

#[test]
fn limit_exceeded_return_error_returns_structured_error() {
    let grid_a = create_simple_grid(100, 10, 0);
    let grid_b = create_simple_grid(100, 10, 0);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnError,
        ..Default::default()
    };

    let result = try_diff_workbooks(&wb_a, &wb_b, &config);
    assert!(result.is_err(), "should return error when limits exceeded");

    let err = result.unwrap_err();
    match err {
        DiffError::LimitsExceeded {
            sheet,
            rows,
            cols,
            max_rows,
            max_cols,
        } => {
            assert_eq!(sheet, sid("Sheet1"));
            assert_eq!(rows, 100);
            assert_eq!(cols, 10);
            assert_eq!(max_rows, 50);
            assert_eq!(max_cols, 16384);
        }
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
#[should_panic(expected = "alignment limits exceeded")]
fn limit_exceeded_return_error_panics_via_legacy_api() {
    let grid_a = create_simple_grid(100, 10, 0);
    let grid_b = create_simple_grid(100, 10, 0);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnError,
        ..Default::default()
    };

    let _ = diff_workbooks(&wb_a, &wb_b, &config);
}

#[test]
fn column_limit_exceeded() {
    let grid_a = create_simple_grid(10, 100, 0);
    let mut grid_b = create_simple_grid(10, 100, 0);
    grid_b.insert_cell(5, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_cols: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        !report.complete,
        "should be marked incomplete when column limit exceeded"
    );
    assert!(
        !report.warnings.is_empty(),
        "should have warning about column limit"
    );
}

#[test]
fn within_limits_no_warning() {
    let grid_a = create_simple_grid(45, 10, 0);
    let mut grid_b = create_simple_grid(45, 10, 0);
    grid_b.insert_cell(20, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "should be complete when within limits");
    assert!(
        report.warnings.is_empty(),
        "should have no warnings when within limits"
    );
}

#[test]
fn multiple_sheets_limit_warning_includes_sheet_name() {
    let grid_small = create_simple_grid(10, 5, 0);
    let grid_large_a = create_simple_grid(100, 10, 1000);
    let grid_large_b = create_simple_grid(100, 10, 2000);

    let wb_a = excel_diff::Workbook {
        sheets: vec![
            excel_diff::Sheet {
                name: sid("SmallSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_small.clone(),
            },
            excel_diff::Sheet {
                name: sid("LargeSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_large_a,
            },
        ],
    };

    let wb_b = excel_diff::Workbook {
        sheets: vec![
            excel_diff::Sheet {
                name: sid("SmallSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_small,
            },
            excel_diff::Sheet {
                name: sid("LargeSheet"),
                kind: excel_diff::SheetKind::Worksheet,
                grid: grid_large_b,
            },
        ],
    };

    let config = DiffConfig {
        max_align_rows: 50,
        on_limit_exceeded: LimitBehavior::ReturnPartialResult,
        ..Default::default()
    };

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(!report.complete, "should be incomplete due to large sheet");
    assert!(
        report.warnings.iter().any(|w| w.contains("LargeSheet")),
        "warning should reference the sheet that exceeded limits"
    );
}

#[test]
fn large_grid_5k_rows_completes_within_default_limits() {
    let grid_a = create_simple_grid(5000, 10, 0);
    let mut grid_b = create_simple_grid(5000, 10, 0);
    grid_b.insert_cell(2500, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("LargeSheet", grid_a);
    let wb_b = single_sheet_workbook("LargeSheet", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "5000-row grid should complete within default limits (max_align_rows=500000)"
    );
    assert!(
        report.warnings.is_empty(),
        "should have no warnings for successful large grid diff"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit in large grid"
    );
}

#[test]
fn wide_grid_500_cols_completes_within_default_limits() {
    let grid_a = create_simple_grid(100, 500, 0);
    let mut grid_b = create_simple_grid(100, 500, 0);
    grid_b.insert_cell(50, 250, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("WideSheet", grid_a);
    let wb_b = single_sheet_workbook("WideSheet", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "500-column grid should complete within default limits (max_align_cols=16384)"
    );
    assert!(
        report.warnings.is_empty(),
        "should have no warnings for successful wide grid diff"
    );
    assert!(
        count_ops(&report.ops, |op| matches!(op, DiffOp::CellEdited { .. })) >= 1,
        "should detect the cell edit in wide grid"
    );
}
