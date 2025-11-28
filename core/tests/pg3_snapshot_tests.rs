use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, Sheet, Workbook, address_to_index, open_workbook,
};

mod common;
use common::fixture_path;

fn sheet_by_name<'a>(workbook: &'a Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == name)
        .expect("sheet should exist")
}

fn find_cell<'a>(sheet: &'a Sheet, addr: &str) -> Option<&'a Cell> {
    let (row, col) = address_to_index(addr).expect("address should parse");
    sheet
        .grid
        .rows
        .get(row as usize)
        .and_then(|r| r.cells.get(col as usize))
}

fn snapshot(sheet: &Sheet, addr: &str) -> CellSnapshot {
    if let Some(cell) = find_cell(sheet, addr) {
        CellSnapshot::from_cell(cell)
    } else {
        let (row, col) = address_to_index(addr).expect("address should parse");
        CellSnapshot {
            addr: CellAddress::from_indices(row, col),
            value: None,
            formula: None,
        }
    }
}

#[test]
fn pg3_value_and_formula_cells_snapshot_from_excel() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let a1 = snapshot(sheet, "A1");
    assert_eq!(a1.addr.to_string(), "A1");
    assert_eq!(a1.value, Some(CellValue::Number(42.0)));
    assert!(a1.formula.is_none());

    let a2 = snapshot(sheet, "A2");
    assert_eq!(a2.value, Some(CellValue::Text("hello".into())));
    assert!(a2.formula.is_none());

    let a3 = snapshot(sheet, "A3");
    assert_eq!(a3.value, Some(CellValue::Bool(true)));
    assert!(a3.formula.is_none());

    let a4 = snapshot(sheet, "A4");
    assert!(a4.value.is_none());
    assert!(a4.formula.is_none());

    let b1 = snapshot(sheet, "B1");
    assert!(matches!(
        b1.value,
        Some(CellValue::Number(n)) if (n - 43.0).abs() < 1e-6
    ));
    assert_eq!(b1.addr.to_string(), "B1");
    let b1_formula = b1.formula.as_deref().expect("B1 should have a formula");
    assert!(b1_formula.contains("A1+1"));

    let b2 = snapshot(sheet, "B2");
    assert_eq!(b2.value, Some(CellValue::Text("hello world".into())));
    assert_eq!(b2.addr.to_string(), "B2");
    let b2_formula = b2.formula.as_deref().expect("B2 should have a formula");
    assert!(b2_formula.contains("hello"));
    assert!(b2_formula.contains("world"));

    let b3 = snapshot(sheet, "B3");
    assert_eq!(b3.value, Some(CellValue::Bool(true)));
    assert_eq!(b3.addr.to_string(), "B3");
    let b3_formula = b3.formula.as_deref().expect("B3 should have a formula");
    assert!(
        b3_formula.contains(">0"),
        "B3 formula should include comparison: {b3_formula:?}"
    );
}

#[test]
fn snapshot_json_roundtrip() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let snapshots = vec![
        snapshot(sheet, "A1"),
        snapshot(sheet, "A2"),
        snapshot(sheet, "B1"),
        snapshot(sheet, "B2"),
        snapshot(sheet, "B3"),
    ];

    for snap in snapshots {
        let addr = snap.addr.to_string();
        let json = serde_json::to_string(&snap).expect("snapshot should serialize");
        let as_value: serde_json::Value =
            serde_json::from_str(&json).expect("snapshot JSON should parse to value");
        assert_eq!(as_value["addr"], serde_json::Value::String(addr));
        let snap_back: CellSnapshot = serde_json::from_str(&json).expect("snapshot should parse");
        assert_eq!(snap.addr, snap_back.addr);
        assert_eq!(snap, snap_back);
    }
}

#[test]
fn snapshot_json_roundtrip_detects_tampered_addr() {
    let snap = CellSnapshot {
        addr: "Z9".parse().expect("address should parse"),
        value: Some(CellValue::Number(1.0)),
        formula: Some("A1+1".into()),
    };

    let mut value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&snap).expect("serialize should work"))
            .expect("serialized JSON should parse");
    value["addr"] = serde_json::Value::String("A1".into());

    let tampered_json = serde_json::to_string(&value).expect("tampered JSON should serialize");
    let tampered: CellSnapshot =
        serde_json::from_str(&tampered_json).expect("tampered JSON should parse");

    assert_ne!(snap.addr, tampered.addr);
    assert_eq!(snap, tampered, "value/formula equality ignores addr");
}
