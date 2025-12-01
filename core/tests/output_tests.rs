use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ContainerError, DiffOp, DiffReport, ExcelOpenError,
    diff_workbooks, open_workbook,
    output::json::{
        CellDiff, diff_report_to_cell_diffs, diff_workbooks_to_json, serialize_cell_diffs,
        serialize_diff_report,
    },
};
use serde_json::Value;

mod common;
use common::fixture_path;

fn render_value(value: &Option<excel_diff::CellValue>) -> Option<String> {
    match value {
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(s)) => Some(s.clone()),
        Some(excel_diff::CellValue::Bool(b)) => Some(b.to_string()),
        None => None,
    }
}

fn make_cell_snapshot(addr: CellAddress, value: Option<CellValue>) -> CellSnapshot {
    CellSnapshot {
        addr,
        value,
        formula: None,
    }
}

#[test]
fn diff_report_to_cell_diffs_filters_non_cell_ops() {
    let addr1 = CellAddress::from_indices(0, 0);
    let addr2 = CellAddress::from_indices(1, 1);

    let report = DiffReport::new(vec![
        DiffOp::SheetAdded {
            sheet: "SheetAdded".into(),
        },
        DiffOp::cell_edited(
            "Sheet1".into(),
            addr1,
            make_cell_snapshot(addr1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr1, Some(CellValue::Number(2.0))),
        ),
        DiffOp::RowAdded {
            sheet: "Sheet1".into(),
            row_idx: 5,
            row_signature: None,
        },
        DiffOp::cell_edited(
            "Sheet2".into(),
            addr2,
            make_cell_snapshot(addr2, Some(CellValue::Text("old".into()))),
            make_cell_snapshot(addr2, Some(CellValue::Text("new".into()))),
        ),
        DiffOp::SheetRemoved {
            sheet: "OldSheet".into(),
        },
    ]);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(
        cell_diffs.len(),
        2,
        "only CellEdited ops should be projected"
    );

    assert_eq!(cell_diffs[0].coords, addr1.to_a1());
    assert_eq!(cell_diffs[0].value_file1, Some("1".into()));
    assert_eq!(cell_diffs[0].value_file2, Some("2".into()));

    assert_eq!(cell_diffs[1].coords, addr2.to_a1());
    assert_eq!(cell_diffs[1].value_file1, Some("old".into()));
    assert_eq!(cell_diffs[1].value_file2, Some("new".into()));
}

#[test]
fn diff_report_to_cell_diffs_maps_values_correctly() {
    let addr_num = CellAddress::from_indices(2, 2); // C3
    let addr_bool = CellAddress::from_indices(3, 3); // D4

    let report = DiffReport::new(vec![
        DiffOp::cell_edited(
            "SheetX".into(),
            addr_num,
            make_cell_snapshot(addr_num, Some(CellValue::Number(42.5))),
            make_cell_snapshot(addr_num, Some(CellValue::Number(43.5))),
        ),
        DiffOp::cell_edited(
            "SheetX".into(),
            addr_bool,
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(true))),
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(false))),
        ),
    ]);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(cell_diffs.len(), 2);

    let number_diff = &cell_diffs[0];
    assert_eq!(number_diff.coords, addr_num.to_a1());
    assert_eq!(number_diff.value_file1, Some("42.5".into()));
    assert_eq!(number_diff.value_file2, Some("43.5".into()));

    let bool_diff = &cell_diffs[1];
    assert_eq!(bool_diff.coords, addr_bool.to_a1());
    assert_eq!(bool_diff.value_file1, Some("true".into()));
    assert_eq!(bool_diff.value_file2, Some("false".into()));
}

#[test]
fn test_json_format() {
    let diffs = vec![
        CellDiff {
            coords: "A1".into(),
            value_file1: Some("100".into()),
            value_file2: Some("200".into()),
        },
        CellDiff {
            coords: "B2".into(),
            value_file1: Some("true".into()),
            value_file2: Some("false".into()),
        },
        CellDiff {
            coords: "C3".into(),
            value_file1: Some("#DIV/0!".into()),
            value_file2: None,
        },
    ];

    let json = serialize_cell_diffs(&diffs).expect("serialization should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    assert!(value.is_array(), "expected an array of cell diffs");
    let arr = value
        .as_array()
        .expect("top-level json should be an array of cell diffs");
    assert_eq!(arr.len(), 3);

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("A1".into()));
    assert_eq!(first["value_file1"], Value::String("100".into()));
    assert_eq!(first["value_file2"], Value::String("200".into()));

    let second = &arr[1];
    assert_eq!(second["coords"], Value::String("B2".into()));
    assert_eq!(second["value_file1"], Value::String("true".into()));
    assert_eq!(second["value_file2"], Value::String("false".into()));

    let third = &arr[2];
    assert_eq!(third["coords"], Value::String("C3".into()));
    assert_eq!(third["value_file1"], Value::String("#DIV/0!".into()));
    assert_eq!(third["value_file2"], Value::Null);
}

#[test]
fn test_json_empty_diff() {
    let fixture = fixture_path("pg1_basic_two_sheets.xlsx");
    let json =
        diff_workbooks_to_json(&fixture, &fixture).expect("diffing identical files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "identical files should produce no diff ops"
    );
}

#[test]
fn test_json_non_empty_diff() {
    let a = fixture_path("json_diff_single_cell_a.xlsx");
    let b = fixture_path("json_diff_single_cell_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_non_empty_diff_bool() {
    let a = fixture_path("json_diff_bool_a.xlsx");
    let b = fixture_path("json_diff_bool_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("true".into()));
            assert_eq!(render_value(&to.value), Some("false".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_diff_value_to_empty() {
    let a = fixture_path("json_diff_value_to_empty_a.xlsx");
    let b = fixture_path("json_diff_value_to_empty_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), None);
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn json_diff_case_only_sheet_name_no_changes() {
    let a = fixture_path("sheet_case_only_rename_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_b.xlsx");

    let old = open_workbook(&a).expect("fixture A should open");
    let new = open_workbook(&b).expect("fixture B should open");

    let report = diff_workbooks(&old, &new);
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should produce no diff ops"
    );
}

#[test]
fn json_diff_case_only_sheet_name_cell_edit() {
    let a = fixture_path("sheet_case_only_rename_edit_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_edit_b.xlsx");

    let old = open_workbook(&a).expect("fixture A should open");
    let new = open_workbook(&b).expect("fixture B should open");

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_diff_workbooks_to_json_reports_invalid_zip() {
    let path = fixture_path("not_a_zip.txt");
    let err = diff_workbooks_to_json(&path, &path)
        .expect_err("diffing invalid containers should return an error");

    assert!(
        matches!(
            err,
            ExcelOpenError::Container(ContainerError::NotZipContainer)
        ),
        "expected container error, got {err}"
    );
}

#[test]
fn serialize_diff_report_nan_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::NAN))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("NaN should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());

    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("nan"),
                "error message should mention NaN for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::INFINITY))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("Infinity should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());
    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_neg_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::NEG_INFINITY))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("NEG_INFINITY should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());
    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_with_finite_numbers_succeeds() {
    let addr = CellAddress::from_indices(1, 1);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(2.5))),
        make_cell_snapshot(addr, Some(CellValue::Number(3.5))),
    )]);

    let json = serialize_diff_report(&report).expect("finite values should serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(parsed.ops.len(), 1);
}
