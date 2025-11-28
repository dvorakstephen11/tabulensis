use excel_diff::output::json::{CellDiff, diff_workbooks_to_json, serialize_cell_diffs};
use serde_json::Value;

mod common;
use common::fixture_path;

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
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    let arr = value
        .as_array()
        .expect("top-level json should be an array of cell diffs");
    assert!(
        arr.is_empty(),
        "identical files should produce no cell diffs"
    );
}

#[test]
fn test_json_non_empty_diff() {
    let a = fixture_path("json_diff_single_cell_a.xlsx");
    let b = fixture_path("json_diff_single_cell_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    let arr = value
        .as_array()
        .expect("top-level should be an array of cell diffs");
    assert_eq!(arr.len(), 1, "expected a single cell difference");

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("C3".into()));
    assert_eq!(first["value_file1"], Value::String("1".into()));
    assert_eq!(first["value_file2"], Value::String("2".into()));
}

#[test]
fn test_json_non_empty_diff_bool() {
    let a = fixture_path("json_diff_bool_a.xlsx");
    let b = fixture_path("json_diff_bool_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    let arr = value
        .as_array()
        .expect("top-level should be an array of cell diffs");
    assert_eq!(arr.len(), 1, "expected a single cell difference");

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("C3".into()));
    assert_eq!(first["value_file1"], Value::String("true".into()));
    assert_eq!(first["value_file2"], Value::String("false".into()));
}
