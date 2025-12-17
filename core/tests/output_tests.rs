mod common;

use common::{fixture_path, open_fixture_workbook};
use excel_diff::{
    CellAddress, CellDiff, CellSnapshot, CellValue, ContainerError, DiffConfig, DiffOp, DiffReport,
    FormulaDiffResult, PackageError, WorkbookPackage, diff_report_to_cell_diffs,
    diff_workbooks_to_json, serialize_cell_diffs, serialize_diff_report,
};
use serde_json::Value;
#[cfg(feature = "perf-metrics")]
use std::collections::BTreeSet;

fn sid_local(pool: &mut excel_diff::StringPool, value: &str) -> excel_diff::StringId {
    pool.intern(value)
}

fn attach_strings(mut report: DiffReport, pool: excel_diff::StringPool) -> DiffReport {
    report.strings = pool.into_strings();
    report
}

fn render_value(report: &DiffReport, value: &Option<excel_diff::CellValue>) -> Option<String> {
    match value {
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(id)) => report.strings.get(id.0 as usize).cloned(),
        Some(excel_diff::CellValue::Bool(b)) => Some(b.to_string()),
        Some(excel_diff::CellValue::Error(id)) => report.strings.get(id.0 as usize).cloned(),
        Some(excel_diff::CellValue::Blank) => Some(String::new()),
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

fn cell_edit(
    sheet: excel_diff::StringId,
    addr: CellAddress,
    from: CellSnapshot,
    to: CellSnapshot,
) -> DiffOp {
    DiffOp::cell_edited(sheet, addr, from, to, FormulaDiffResult::Unchanged)
}

fn numeric_report(addr: CellAddress, from: f64, to: f64) -> DiffReport {
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    attach_strings(
        DiffReport::new(vec![cell_edit(
            sheet,
            addr,
            make_cell_snapshot(addr, Some(CellValue::Number(from))),
            make_cell_snapshot(addr, Some(CellValue::Number(to))),
        )]),
        pool,
    )
}

#[test]
fn diff_report_to_cell_diffs_filters_non_cell_ops() {
    let mut pool = excel_diff::StringPool::new();
    let sheet_added = sid_local(&mut pool, "SheetAdded");
    let sheet1 = sid_local(&mut pool, "Sheet1");
    let sheet2 = sid_local(&mut pool, "Sheet2");
    let old_sheet = sid_local(&mut pool, "OldSheet");
    let old_text = sid_local(&mut pool, "old");
    let new_text = sid_local(&mut pool, "new");
    let addr1 = CellAddress::from_indices(0, 0);
    let addr2 = CellAddress::from_indices(1, 1);

    let report = attach_strings(
        DiffReport::new(vec![
            DiffOp::SheetAdded { sheet: sheet_added },
            cell_edit(
                sheet1,
                addr1,
                make_cell_snapshot(addr1, Some(CellValue::Number(1.0))),
                make_cell_snapshot(addr1, Some(CellValue::Number(2.0))),
            ),
            DiffOp::RowAdded {
                sheet: sheet1,
                row_idx: 5,
                row_signature: None,
            },
            cell_edit(
                sheet2,
                addr2,
                make_cell_snapshot(addr2, Some(CellValue::Text(old_text))),
                make_cell_snapshot(addr2, Some(CellValue::Text(new_text))),
            ),
            DiffOp::SheetRemoved { sheet: old_sheet },
        ]),
        pool,
    );

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
fn diff_report_to_cell_diffs_ignores_block_moved_rect() {
    let mut pool = excel_diff::StringPool::new();
    let sheet1 = sid_local(&mut pool, "Sheet1");
    let addr = CellAddress::from_indices(2, 2);

    let report = attach_strings(
        DiffReport::new(vec![
            DiffOp::block_moved_rect(sheet1, 2, 3, 1, 3, 9, 6, Some(0xCAFEBABE)),
            cell_edit(
                sheet1,
                addr,
                make_cell_snapshot(addr, Some(CellValue::Number(10.0))),
                make_cell_snapshot(addr, Some(CellValue::Number(20.0))),
            ),
            DiffOp::BlockMovedRows {
                sheet: sheet1,
                src_start_row: 0,
                row_count: 2,
                dst_start_row: 5,
                block_hash: None,
            },
            DiffOp::BlockMovedColumns {
                sheet: sheet1,
                src_start_col: 0,
                col_count: 2,
                dst_start_col: 5,
                block_hash: None,
            },
        ]),
        pool,
    );

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(
        cell_diffs.len(),
        1,
        "only CellEdited should be projected; BlockMovedRect and other block moves should be ignored"
    );

    assert_eq!(cell_diffs[0].coords, addr.to_a1());
    assert_eq!(cell_diffs[0].value_file1, Some("10".into()));
    assert_eq!(cell_diffs[0].value_file2, Some("20".into()));
}

#[test]
fn diff_report_to_cell_diffs_maps_values_correctly() {
    let mut pool = excel_diff::StringPool::new();
    let sheet_id = sid_local(&mut pool, "SheetX");
    let addr_num = CellAddress::from_indices(2, 2); // C3
    let addr_bool = CellAddress::from_indices(3, 3); // D4

    let report = attach_strings(
        DiffReport::new(vec![
            cell_edit(
                sheet_id,
                addr_num,
                make_cell_snapshot(addr_num, Some(CellValue::Number(42.5))),
                make_cell_snapshot(addr_num, Some(CellValue::Number(43.5))),
            ),
            cell_edit(
                sheet_id,
                addr_bool,
                make_cell_snapshot(addr_bool, Some(CellValue::Bool(true))),
                make_cell_snapshot(addr_bool, Some(CellValue::Bool(false))),
            ),
        ]),
        pool,
    );

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
fn diff_report_to_cell_diffs_filters_no_op_cell_edits() {
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    let addr_a1 = CellAddress::from_indices(0, 0);
    let addr_a2 = CellAddress::from_indices(1, 0);

    let report = attach_strings(
        DiffReport::new(vec![
            cell_edit(
                sheet,
                addr_a1,
                make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
                make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
            ),
            cell_edit(
                sheet,
                addr_a2,
                make_cell_snapshot(addr_a2, Some(CellValue::Number(1.0))),
                make_cell_snapshot(addr_a2, Some(CellValue::Number(2.0))),
            ),
        ]),
        pool,
    );

    let diffs = diff_report_to_cell_diffs(&report);

    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].coords, "A2");
    assert_eq!(diffs[0].value_file1, Some("1".to_string()));
    assert_eq!(diffs[0].value_file2, Some("2".to_string()));
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
    let json = diff_workbooks_to_json(&fixture, &fixture, &DiffConfig::default())
        .expect("diffing identical files should succeed");
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

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_non_empty_diff_bool() {
    let a = fixture_path("json_diff_bool_a.xlsx");
    let b = fixture_path("json_diff_bool_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&report, &from.value), Some("true".into()));
            assert_eq!(render_value(&report, &to.value), Some("false".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_diff_value_to_empty() {
    let a = fixture_path("json_diff_value_to_empty_a.xlsx");
    let b = fixture_path("json_diff_value_to_empty_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), None);
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn json_diff_case_only_sheet_name_no_changes() {
    let old = open_fixture_workbook("sheet_case_only_rename_a.xlsx");
    let new = open_fixture_workbook("sheet_case_only_rename_b.xlsx");

    let report =
        WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should produce no diff ops"
    );
}

#[test]
fn json_diff_case_only_sheet_name_cell_edit() {
    let old = open_fixture_workbook("sheet_case_only_rename_edit_a.xlsx");
    let new = open_fixture_workbook("sheet_case_only_rename_edit_b.xlsx");

    let report =
        WorkbookPackage::from(old).diff(&WorkbookPackage::from(new), &DiffConfig::default());
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(
                report.strings.get(sheet.0 as usize),
                Some(&"Sheet1".to_string())
            );
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_case_only_sheet_name_no_changes() {
    let a = fixture_path("sheet_case_only_rename_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing case-only sheet rename should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should serialize to no ops"
    );
}

#[test]
fn test_json_case_only_sheet_name_cell_edit_via_helper() {
    let a = fixture_path("sheet_case_only_rename_edit_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_edit_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b, &DiffConfig::default())
        .expect("diffing case-only sheet rename with cell edit should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(
                report.strings.get(sheet.0 as usize),
                Some(&"Sheet1".to_string())
            );
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&report, &from.value), Some("1".into()));
            assert_eq!(render_value(&report, &to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_diff_workbooks_to_json_reports_invalid_zip() {
    let path = fixture_path("not_a_zip.txt");
    let err = diff_workbooks_to_json(&path, &path, &DiffConfig::default())
        .expect_err("diffing invalid containers should return an error");

    assert!(
        matches!(
            err,
            PackageError::Container(ContainerError::NotZipContainer)
        ),
        "expected container error, got {err}"
    );
}

#[test]
fn serialize_diff_report_nan_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = numeric_report(addr, f64::NAN, 1.0);

    let err = serialize_diff_report(&report).expect_err("NaN should fail to serialize");
    let wrapped = PackageError::SerializationError(err.to_string());

    match wrapped {
        PackageError::SerializationError(msg) => {
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
    let report = numeric_report(addr, f64::INFINITY, 1.0);

    let err = serialize_diff_report(&report).expect_err("Infinity should fail to serialize");
    let wrapped = PackageError::SerializationError(err.to_string());
    match wrapped {
        PackageError::SerializationError(msg) => {
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
    let report = numeric_report(addr, f64::NEG_INFINITY, 1.0);

    let err = serialize_diff_report(&report).expect_err("NEG_INFINITY should fail to serialize");
    let wrapped = PackageError::SerializationError(err.to_string());
    match wrapped {
        PackageError::SerializationError(msg) => {
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
    let report = numeric_report(addr, 2.5, 3.5);

    let json = serialize_diff_report(&report).expect("finite values should serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(parsed.ops.len(), 1);
}

#[test]
fn serialize_full_diff_report_has_complete_true_and_no_warnings() {
    let addr = CellAddress::from_indices(0, 0);
    let report = numeric_report(addr, 1.0, 2.0);

    let json = serialize_diff_report(&report).expect("full report should serialize");
    let value: Value = serde_json::from_str(&json).expect("json should parse");
    let obj = value.as_object().expect("should be object");

    assert_eq!(
        obj.get("complete").and_then(Value::as_bool),
        Some(true),
        "full result should have complete=true"
    );

    let has_warnings = obj
        .get("warnings")
        .map(|v| v.as_array().map(|arr| !arr.is_empty()).unwrap_or(false))
        .unwrap_or(false);
    assert!(
        !has_warnings,
        "full result should have no warnings or empty warnings array"
    );
}

#[test]
fn serialize_partial_diff_report_includes_complete_false_and_warnings() {
    let addr = CellAddress::from_indices(0, 0);
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    let ops = vec![cell_edit(
        sheet,
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
        make_cell_snapshot(addr, Some(CellValue::Number(2.0))),
    )];
    let report = attach_strings(
        DiffReport::with_partial_result(
            ops,
            "Sheet 'LargeSheet': alignment limits exceeded".to_string(),
        ),
        pool,
    );

    let json = serialize_diff_report(&report).expect("partial report should serialize");
    let value: Value = serde_json::from_str(&json).expect("json should parse");
    let obj = value.as_object().expect("should be object");

    assert_eq!(
        obj.get("complete").and_then(Value::as_bool),
        Some(false),
        "partial result should have complete=false"
    );

    let warnings = obj
        .get("warnings")
        .and_then(Value::as_array)
        .expect("warnings should be present");
    assert!(!warnings.is_empty(), "warnings array should not be empty");
    assert!(
        warnings[0]
            .as_str()
            .unwrap_or("")
            .contains("limits exceeded"),
        "warning should mention limits exceeded"
    );
}

#[test]
#[cfg(feature = "perf-metrics")]
fn serialize_diff_report_with_metrics_includes_metrics_object() {
    use excel_diff::perf::DiffMetrics;

    let addr = CellAddress::from_indices(0, 0);
    let mut pool = excel_diff::StringPool::new();
    let sheet = sid_local(&mut pool, "Sheet1");
    let ops = vec![cell_edit(
        sheet,
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
        make_cell_snapshot(addr, Some(CellValue::Number(2.0))),
    )];

    let mut report = attach_strings(DiffReport::new(ops), pool);
    let mut metrics = DiffMetrics::default();
    metrics.move_detection_time_ms = 5;
    metrics.alignment_time_ms = 10;
    metrics.cell_diff_time_ms = 15;
    metrics.total_time_ms = 30;
    metrics.rows_processed = 500;
    metrics.cells_compared = 2500;
    metrics.anchors_found = 25;
    metrics.moves_detected = 1;
    report.metrics = Some(metrics);

    let json = serialize_diff_report(&report).expect("report with metrics should serialize");
    let value: Value = serde_json::from_str(&json).expect("json should parse");
    let obj = value.as_object().expect("should be object");

    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    assert!(
        keys.contains("metrics"),
        "serialized report should include metrics key"
    );

    let metrics_obj = obj
        .get("metrics")
        .and_then(Value::as_object)
        .expect("metrics should be an object");

    assert!(
        metrics_obj.contains_key("move_detection_time_ms"),
        "metrics should contain move_detection_time_ms"
    );
    assert!(
        metrics_obj.contains_key("alignment_time_ms"),
        "metrics should contain alignment_time_ms"
    );
    assert!(
        metrics_obj.contains_key("cell_diff_time_ms"),
        "metrics should contain cell_diff_time_ms"
    );
    assert!(
        metrics_obj.contains_key("total_time_ms"),
        "metrics should contain total_time_ms"
    );
    assert!(
        metrics_obj.contains_key("rows_processed"),
        "metrics should contain rows_processed"
    );
    assert!(
        metrics_obj.contains_key("cells_compared"),
        "metrics should contain cells_compared"
    );
    assert!(
        metrics_obj.contains_key("anchors_found"),
        "metrics should contain anchors_found"
    );
    assert!(
        metrics_obj.contains_key("moves_detected"),
        "metrics should contain moves_detected"
    );

    assert_eq!(
        metrics_obj.get("rows_processed").and_then(Value::as_u64),
        Some(500)
    );
    assert_eq!(
        metrics_obj.get("cells_compared").and_then(Value::as_u64),
        Some(2500)
    );
}
