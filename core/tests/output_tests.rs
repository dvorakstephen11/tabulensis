use excel_diff::output::json::{CellDiff, diff_workbooks_to_json, serialize_cell_diffs};
use serde_json::Value;

mod common;
use common::fixture_path;

#[test]
fn test_json_format() {
    let diffs = vec![CellDiff {
        coords: "A1".into(),
        value_file1: Some("100".into()),
        value_file2: Some("200".into()),
    }];

    let json = serialize_cell_diffs(&diffs).expect("serialization should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    assert!(value.is_array(), "expected an array of cell diffs");
    let first = value
        .as_array()
        .and_then(|arr| arr.first())
        .expect("array should contain one element");
    assert_eq!(first["coords"], Value::String("A1".into()));
    assert_eq!(first["value_file1"], Value::String("100".into()));
    assert_eq!(first["value_file2"], Value::String("200".into()));
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
