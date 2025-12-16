mod common;

use common::sid;
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ColSignature, DiffOp, DiffReport, RowSignature,
    with_default_session,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn addr(a1: &str) -> CellAddress {
    a1.parse().expect("address should parse")
}

fn snapshot(a1: &str, value: Option<CellValue>, formula: Option<&str>) -> CellSnapshot {
    CellSnapshot {
        addr: addr(a1),
        value,
        formula: formula.map(|s| sid(s)),
    }
}

fn sample_cell_edited() -> DiffOp {
    DiffOp::CellEdited {
        sheet: sid("Sheet1"),
        addr: addr("C3"),
        from: snapshot("C3", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    }
}

// Enforces the invariant documented on DiffOp::CellEdited.
fn assert_cell_edited_invariants(op: &DiffOp, expected_sheet: &str, expected_addr: &str) {
    let expected_addr_parsed: CellAddress =
        expected_addr.parse().expect("expected_addr should parse");
    if let DiffOp::CellEdited {
        sheet,
        addr,
        from,
        to,
    } = op
    {
        assert_eq!(sheet, &sid(expected_sheet));
        assert_eq!(*addr, expected_addr_parsed);
        assert_eq!(from.addr, expected_addr_parsed);
        assert_eq!(to.addr, expected_addr_parsed);
    } else {
        panic!("expected CellEdited");
    }
}

fn op_kind(op: &DiffOp) -> &'static str {
    match op {
        DiffOp::SheetAdded { .. } => "SheetAdded",
        DiffOp::SheetRemoved { .. } => "SheetRemoved",
        DiffOp::RowAdded { .. } => "RowAdded",
        DiffOp::RowRemoved { .. } => "RowRemoved",
        DiffOp::ColumnAdded { .. } => "ColumnAdded",
        DiffOp::ColumnRemoved { .. } => "ColumnRemoved",
        DiffOp::BlockMovedRows { .. } => "BlockMovedRows",
        DiffOp::BlockMovedColumns { .. } => "BlockMovedColumns",
        DiffOp::BlockMovedRect { .. } => "BlockMovedRect",
        DiffOp::CellEdited { .. } => "CellEdited",
        _ => "Unknown",
    }
}

fn attach_strings(mut report: DiffReport) -> DiffReport {
    report.strings = with_default_session(|session| session.strings.strings().to_vec());
    report
}

fn json_keys(json: &Value) -> BTreeSet<String> {
    json.as_object()
        .expect("object json")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn pg4_construct_cell_edited_diffop() {
    let op = sample_cell_edited();

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
    if let DiffOp::CellEdited { from, to, .. } = &op {
        assert_ne!(from.value, to.value);
    }
}

#[test]
fn pg4_construct_row_and_column_diffops() {
    let row_added_with_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 8,
        row_signature: None,
    };
    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 0,
        col_signature: None,
    };

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_with_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 10);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0xDEADBEEF);
    } else {
        panic!("expected RowAdded with signature");
    }

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_without_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 11);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowAdded without signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_with_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 9);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0x1234);
    } else {
        panic!("expected RowRemoved with signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_without_sig
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*row_idx, 8);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowRemoved without signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_with_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 2);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0xABCDEF);
    } else {
        panic!("expected ColumnAdded with signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_without_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 3);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnAdded without signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_with_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 1);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0x123456);
    } else {
        panic!("expected ColumnRemoved with signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_without_sig
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*col_idx, 0);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnRemoved without signature");
    }

    assert_ne!(row_added_with_sig, row_added_without_sig);
    assert_ne!(row_removed_with_sig, row_removed_without_sig);
    assert_ne!(col_added_with_sig, col_added_without_sig);
    assert_ne!(col_removed_with_sig, col_removed_without_sig);
}

#[test]
fn pg4_construct_block_move_diffops() {
    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_with_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 10);
        assert_eq!(*row_count, 3);
        assert_eq!(*dst_start_row, 5);
        assert_eq!(block_hash.unwrap(), 0x12345678);
    } else {
        panic!("expected BlockMovedRows with hash");
    }

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_without_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 20);
        assert_eq!(*row_count, 2);
        assert_eq!(*dst_start_row, 0);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedRows without hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_with_hash
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*src_start_col, 7);
        assert_eq!(*col_count, 2);
        assert_eq!(*dst_start_col, 3);
        assert_eq!(block_hash.unwrap(), 0xCAFEBABE);
    } else {
        panic!("expected BlockMovedColumns with hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_without_hash
    {
        assert_eq!(sheet, &sid("Sheet2"));
        assert_eq!(*src_start_col, 4);
        assert_eq!(*col_count, 1);
        assert_eq!(*dst_start_col, 9);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedColumns without hash");
    }

    assert_ne!(block_rows_with_hash, block_rows_without_hash);
    assert_ne!(block_cols_with_hash, block_cols_without_hash);
}

#[test]
fn pg4_construct_block_rect_diffops() {
    let rect_with_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 5,
        src_row_count: 3,
        src_start_col: 2,
        src_col_count: 4,
        dst_start_row: 10,
        dst_start_col: 6,
        block_hash: Some(0xCAFEBABE),
    };
    let rect_without_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 0,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 1,
        dst_start_row: 20,
        dst_start_col: 10,
        block_hash: None,
    };

    if let DiffOp::BlockMovedRect {
        sheet,
        src_start_row,
        src_row_count,
        src_start_col,
        src_col_count,
        dst_start_row,
        dst_start_col,
        block_hash,
    } = &rect_with_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 5);
        assert_eq!(*src_row_count, 3);
        assert_eq!(*src_start_col, 2);
        assert_eq!(*src_col_count, 4);
        assert_eq!(*dst_start_row, 10);
        assert_eq!(*dst_start_col, 6);
        assert_eq!(block_hash.unwrap(), 0xCAFEBABE);
    } else {
        panic!("expected BlockMovedRect with hash");
    }

    if let DiffOp::BlockMovedRect {
        sheet,
        src_start_row,
        src_row_count,
        src_start_col,
        src_col_count,
        dst_start_row,
        dst_start_col,
        block_hash,
    } = &rect_without_hash
    {
        assert_eq!(sheet, &sid("Sheet1"));
        assert_eq!(*src_start_row, 0);
        assert_eq!(*src_row_count, 1);
        assert_eq!(*src_start_col, 0);
        assert_eq!(*src_col_count, 1);
        assert_eq!(*dst_start_row, 20);
        assert_eq!(*dst_start_col, 10);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedRect without hash");
    }

    assert_ne!(rect_with_hash, rect_without_hash);
}

#[test]
fn pg4_cell_edited_json_shape() {
    let op = sample_cell_edited();
    let json = serde_json::to_value(&op).expect("serialize");
    assert_cell_edited_invariants(&op, "Sheet1", "C3");

    assert_eq!(json["kind"], "CellEdited");
    assert_eq!(json["sheet"], "Sheet1");
    assert_eq!(json["addr"], "C3");
    assert_eq!(json["from"]["addr"], "C3");
    assert_eq!(json["to"]["addr"], "C3");

    let obj = json.as_object().expect("object json");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["addr", "from", "kind", "sheet", "to"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected);
}

#[test]
fn pg4_row_added_json_optional_signature() {
    let op_without_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: None,
    };
    let json_without = serde_json::to_value(&op_without_sig).expect("serialize without sig");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "RowAdded");
    assert_eq!(json_without["sheet"], "Sheet1");
    assert_eq!(json_without["row_idx"], 10);
    assert!(obj_without.get("row_signature").is_none());

    let op_with_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 123 }),
    };
    let json_with = serde_json::to_value(&op_with_sig).expect("serialize with sig");
    assert_eq!(
        json_with["row_signature"]["hash"],
        "0000000000000000000000000000007b"
    );
}

#[test]
fn pg4_column_added_json_optional_signature() {
    let added_without_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet1"),
        col_idx: 5,
        col_signature: None,
    };
    let json_added_without = serde_json::to_value(&added_without_sig).expect("serialize no sig");
    let obj_added_without = json_added_without.as_object().expect("object json");
    assert_eq!(json_added_without["kind"], "ColumnAdded");
    assert_eq!(json_added_without["sheet"], "Sheet1");
    assert_eq!(json_added_without["col_idx"], 5);
    assert!(obj_added_without.get("col_signature").is_none());

    let added_with_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet1"),
        col_idx: 6,
        col_signature: Some(ColSignature { hash: 321 }),
    };
    let json_added_with = serde_json::to_value(&added_with_sig).expect("serialize with sig");
    assert_eq!(
        json_added_with["col_signature"]["hash"],
        "00000000000000000000000000000141"
    );

    let removed_without_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 2,
        col_signature: None,
    };
    let json_removed_without =
        serde_json::to_value(&removed_without_sig).expect("serialize removed no sig");
    let obj_removed_without = json_removed_without.as_object().expect("object json");
    assert_eq!(json_removed_without["kind"], "ColumnRemoved");
    assert!(obj_removed_without.get("col_signature").is_none());

    let removed_with_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 654 }),
    };
    let json_removed_with =
        serde_json::to_value(&removed_with_sig).expect("serialize removed with sig");
    assert_eq!(
        json_removed_with["col_signature"]["hash"],
        "0000000000000000000000000000028e"
    );
}

#[test]
fn pg4_block_moved_rows_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedRows");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: Some(777),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(777));
}

#[test]
fn pg4_block_moved_columns_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedColumns {
        sheet: sid("SheetX"),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedColumns");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedColumns {
        sheet: sid("SheetX"),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: Some(4242),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(4242));
}

#[test]
fn pg4_sheet_added_and_removed_json_shape() {
    let added = DiffOp::SheetAdded {
        sheet: sid("Sheet1"),
    };
    let added_json = serde_json::to_value(&added).expect("serialize sheet added");
    assert_eq!(added_json["kind"], "SheetAdded");
    assert_eq!(added_json["sheet"], "Sheet1");
    let added_keys = json_keys(&added_json);
    let expected_keys: BTreeSet<String> = ["kind", "sheet"].into_iter().map(String::from).collect();
    assert_eq!(added_keys, expected_keys);

    let removed = DiffOp::SheetRemoved {
        sheet: sid("SheetX"),
    };
    let removed_json = serde_json::to_value(&removed).expect("serialize sheet removed");
    assert_eq!(removed_json["kind"], "SheetRemoved");
    assert_eq!(removed_json["sheet"], "SheetX");
    let removed_keys = json_keys(&removed_json);
    assert_eq!(removed_keys, expected_keys);
}

#[test]
fn pg4_row_and_column_json_shape_keysets() {
    let expected_row_with_sig: BTreeSet<String> = ["kind", "row_idx", "row_signature", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_row_without_sig: BTreeSet<String> = ["kind", "row_idx", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_with_sig: BTreeSet<String> = ["col_idx", "col_signature", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_without_sig: BTreeSet<String> = ["col_idx", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();

    let row_added_with_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: sid("Sheet1"),
        row_idx: 8,
        row_signature: None,
    };

    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: sid("Sheet2"),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: sid("Sheet2"),
        col_idx: 0,
        col_signature: None,
    };

    let cases = vec![
        (
            row_added_with_sig,
            "RowAdded",
            expected_row_with_sig.clone(),
        ),
        (
            row_added_without_sig,
            "RowAdded",
            expected_row_without_sig.clone(),
        ),
        (
            row_removed_with_sig,
            "RowRemoved",
            expected_row_with_sig.clone(),
        ),
        (
            row_removed_without_sig,
            "RowRemoved",
            expected_row_without_sig.clone(),
        ),
        (
            col_added_with_sig,
            "ColumnAdded",
            expected_col_with_sig.clone(),
        ),
        (
            col_added_without_sig,
            "ColumnAdded",
            expected_col_without_sig.clone(),
        ),
        (
            col_removed_with_sig,
            "ColumnRemoved",
            expected_col_with_sig.clone(),
        ),
        (
            col_removed_without_sig,
            "ColumnRemoved",
            expected_col_without_sig.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_block_move_json_shape_keysets() {
    let expected_rows_with_hash: BTreeSet<String> = [
        "block_hash",
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rows_without_hash: BTreeSet<String> = [
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_with_hash: BTreeSet<String> = [
        "block_hash",
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_without_hash: BTreeSet<String> = [
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rect_with_hash: BTreeSet<String> = [
        "block_hash",
        "dst_start_col",
        "dst_start_row",
        "kind",
        "sheet",
        "src_col_count",
        "src_row_count",
        "src_start_col",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rect_without_hash: BTreeSet<String> = [
        "dst_start_col",
        "dst_start_row",
        "kind",
        "sheet",
        "src_col_count",
        "src_row_count",
        "src_start_col",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: sid("Sheet1"),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: sid("Sheet2"),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };
    let block_rect_with_hash = DiffOp::BlockMovedRect {
        sheet: sid("SheetZ"),
        src_start_row: 2,
        src_row_count: 2,
        src_start_col: 3,
        src_col_count: 4,
        dst_start_row: 8,
        dst_start_col: 1,
        block_hash: Some(0xAABBCCDD),
    };
    let block_rect_without_hash = DiffOp::BlockMovedRect {
        sheet: sid("SheetZ"),
        src_start_row: 5,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 2,
        dst_start_row: 10,
        dst_start_col: 4,
        block_hash: None,
    };

    let cases = vec![
        (
            block_rows_with_hash,
            "BlockMovedRows",
            expected_rows_with_hash.clone(),
        ),
        (
            block_rows_without_hash,
            "BlockMovedRows",
            expected_rows_without_hash.clone(),
        ),
        (
            block_cols_with_hash,
            "BlockMovedColumns",
            expected_cols_with_hash.clone(),
        ),
        (
            block_cols_without_hash,
            "BlockMovedColumns",
            expected_cols_without_hash.clone(),
        ),
        (
            block_rect_with_hash,
            "BlockMovedRect",
            expected_rect_with_hash.clone(),
        ),
        (
            block_rect_without_hash,
            "BlockMovedRect",
            expected_rect_without_hash.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_block_rect_json_shape_and_roundtrip() {
    let without_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 2,
        src_row_count: 3,
        src_start_col: 1,
        src_col_count: 2,
        dst_start_row: 10,
        dst_start_col: 5,
        block_hash: None,
    };
    let with_hash = DiffOp::BlockMovedRect {
        sheet: sid("Sheet1"),
        src_start_row: 4,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 1,
        dst_start_row: 20,
        dst_start_col: 7,
        block_hash: Some(0x55AA),
    };

    let report = DiffReport::new(vec![without_hash.clone(), with_hash.clone()]);
    let json = serde_json::to_value(&report).expect("serialize rect report");

    let ops_json = json["ops"]
        .as_array()
        .expect("ops should be array for report");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "BlockMovedRect");
    assert_eq!(ops_json[0]["sheet"], "Sheet1");
    assert_eq!(ops_json[0]["src_start_row"], 2);
    assert_eq!(ops_json[0]["src_row_count"], 3);
    assert_eq!(ops_json[0]["src_start_col"], 1);
    assert_eq!(ops_json[0]["src_col_count"], 2);
    assert_eq!(ops_json[0]["dst_start_row"], 10);
    assert_eq!(ops_json[0]["dst_start_col"], 5);
    assert!(
        ops_json[0].get("block_hash").is_none(),
        "block_hash should be omitted when None"
    );

    assert_eq!(ops_json[1]["kind"], "BlockMovedRect");
    assert_eq!(ops_json[1]["block_hash"], Value::from(0x55AA));

    let roundtrip: DiffReport =
        serde_json::from_value(json).expect("roundtrip deserialize rect report");
    assert_eq!(roundtrip.ops, vec![without_hash, with_hash]);
}

#[test]
fn pg4_diffop_roundtrip_each_variant() {
    let ops = vec![
        DiffOp::SheetAdded {
            sheet: sid("SheetA"),
        },
        DiffOp::SheetRemoved {
            sheet: sid("SheetB"),
        },
        DiffOp::RowAdded {
            sheet: sid("Sheet1"),
            row_idx: 1,
            row_signature: Some(RowSignature { hash: 42 }),
        },
        DiffOp::RowRemoved {
            sheet: sid("Sheet1"),
            row_idx: 0,
            row_signature: None,
        },
        DiffOp::ColumnAdded {
            sheet: sid("Sheet1"),
            col_idx: 2,
            col_signature: None,
        },
        DiffOp::ColumnRemoved {
            sheet: sid("Sheet1"),
            col_idx: 3,
            col_signature: Some(ColSignature { hash: 99 }),
        },
        DiffOp::BlockMovedRows {
            sheet: sid("Sheet1"),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: Some(1234),
        },
        DiffOp::BlockMovedRows {
            sheet: sid("Sheet1"),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: None,
        },
        DiffOp::BlockMovedColumns {
            sheet: sid("Sheet2"),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: Some(888),
        },
        DiffOp::BlockMovedColumns {
            sheet: sid("Sheet2"),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: None,
        },
        DiffOp::BlockMovedRect {
            sheet: sid("Sheet3"),
            src_start_row: 1,
            src_row_count: 2,
            src_start_col: 3,
            src_col_count: 4,
            dst_start_row: 10,
            dst_start_col: 20,
            block_hash: Some(0xABCD),
        },
        DiffOp::BlockMovedRect {
            sheet: sid("Sheet3"),
            src_start_row: 1,
            src_row_count: 2,
            src_start_col: 3,
            src_col_count: 4,
            dst_start_row: 10,
            dst_start_col: 20,
            block_hash: None,
        },
        sample_cell_edited(),
    ];

    for original in ops {
        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: DiffOp = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, original);

        if let DiffOp::CellEdited { .. } = &deserialized {
            assert_cell_edited_invariants(&deserialized, "Sheet1", "C3");
        }
    }
}

#[test]
fn pg4_cell_edited_roundtrip_preserves_snapshot_addrs() {
    let op = sample_cell_edited();
    let json = serde_json::to_string(&op).expect("serialize");
    let round_tripped: DiffOp = serde_json::from_str(&json).expect("deserialize");

    assert_cell_edited_invariants(&round_tripped, "Sheet1", "C3");
}

#[test]
fn pg4_diff_report_roundtrip_preserves_order() {
    let op1 = DiffOp::SheetAdded {
        sheet: sid("Sheet1"),
    };
    let op2 = DiffOp::RowAdded {
        sheet: sid("Sheet1"),
        row_idx: 10,
        row_signature: None,
    };
    let op3 = sample_cell_edited();

    let ops = vec![op1, op2, op3];
    let report = DiffReport::new(ops.clone());
    assert_eq!(report.version, DiffReport::SCHEMA_VERSION);

    let serialized = serde_json::to_string(&report).expect("serialize report");
    let deserialized: DiffReport = serde_json::from_str(&serialized).expect("deserialize report");
    assert_eq!(deserialized.version, "1");
    assert_eq!(deserialized.ops, ops);

    let kinds: Vec<&str> = deserialized.ops.iter().map(op_kind).collect();
    assert_eq!(kinds, vec!["SheetAdded", "RowAdded", "CellEdited"]);
}

#[test]
fn pg4_diff_report_json_shape() {
    let ops = vec![
        DiffOp::SheetRemoved {
            sheet: sid("SheetX"),
        },
        DiffOp::RowRemoved {
            sheet: sid("SheetX"),
            row_idx: 3,
            row_signature: Some(RowSignature { hash: 7 }),
        },
    ];
    let report = DiffReport::new(ops);
    let json = serde_json::to_value(&report).expect("serialize to value");

    let obj = json.as_object().expect("report json object");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["complete", "ops", "version"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected);
    assert_eq!(obj.get("version").and_then(Value::as_str), Some("1"));
    assert_eq!(obj.get("complete").and_then(Value::as_bool), Some(true));

    let ops_json = obj
        .get("ops")
        .and_then(Value::as_array)
        .expect("ops should be array");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "SheetRemoved");
    assert_eq!(ops_json[1]["kind"], "RowRemoved");
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_top_level_addr() {
    let json = r#"{
        "kind": "CellEdited",
        "sheet": "Sheet1",
        "addr": "1A",
        "from": { "addr": "C3", "value": null, "formula": null },
        "to":   { "addr": "C3", "value": null, "formula": null }
    }"#;

    let err = serde_json::from_str::<DiffOp>(json)
        .expect_err("invalid top-level addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_snapshot_addrs() {
    let json = r#"{
        "kind": "CellEdited",
        "sheet": "Sheet1",
        "addr": "C3",
        "from": { "addr": "A0", "value": null, "formula": null },
        "to":   { "addr": "C3", "value": null, "formula": null }
    }"#;

    let err = serde_json::from_str::<DiffOp>(json)
        .expect_err("invalid snapshot addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("A0"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diff_report_rejects_invalid_nested_addr() {
    let json = r#"{
        "version": "1",
        "ops": [{
            "kind": "CellEdited",
            "sheet": "Sheet1",
            "addr": "1A",
            "from": { "addr": "C3", "value": null, "formula": null },
            "to":   { "addr": "C3", "value": null, "formula": null }
        }]
    }"#;

    let err = serde_json::from_str::<DiffReport>(json)
        .expect_err("invalid nested addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should surface nested invalid address: {msg}",
    );
}

#[test]
#[should_panic]
fn pg4_cell_edited_invariant_helper_rejects_mismatched_snapshot_addr() {
    let op = DiffOp::CellEdited {
        sheet: sid("Sheet1"),
        addr: addr("C3"),
        from: snapshot("D4", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    };

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
}

#[test]
#[cfg(feature = "perf-metrics")]
fn pg4_diff_report_json_shape_with_metrics() {
    use excel_diff::perf::DiffMetrics;

    let ops = vec![DiffOp::SheetAdded {
        sheet: "NewSheet".to_string(),
    }];
    let mut report = DiffReport::new(ops);
    let mut metrics = DiffMetrics::default();
    metrics.move_detection_time_ms = 10;
    metrics.alignment_time_ms = 20;
    metrics.cell_diff_time_ms = 30;
    metrics.total_time_ms = 60;
    metrics.rows_processed = 1000;
    metrics.cells_compared = 5000;
    metrics.anchors_found = 50;
    metrics.moves_detected = 2;
    report.metrics = Some(metrics);

    let json = serde_json::to_value(&report).expect("serialize to value");
    let obj = json.as_object().expect("report json object");

    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["complete", "ops", "version", "metrics"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected, "report should include metrics key");

    let metrics_obj = obj
        .get("metrics")
        .and_then(Value::as_object)
        .expect("metrics object");

    assert!(metrics_obj.contains_key("move_detection_time_ms"));
    assert!(metrics_obj.contains_key("alignment_time_ms"));
    assert!(metrics_obj.contains_key("cell_diff_time_ms"));
    assert!(metrics_obj.contains_key("total_time_ms"));
    assert!(metrics_obj.contains_key("rows_processed"));
    assert!(metrics_obj.contains_key("cells_compared"));
    assert!(metrics_obj.contains_key("anchors_found"));
    assert!(metrics_obj.contains_key("moves_detected"));

    assert!(
        !metrics_obj.contains_key("parse_time_ms"),
        "parse_time_ms is planned for future phase"
    );
    assert!(
        !metrics_obj.contains_key("peak_memory_bytes"),
        "peak_memory_bytes is planned for future phase"
    );

    assert_eq!(
        metrics_obj.get("rows_processed").and_then(Value::as_u64),
        Some(1000)
    );
    assert_eq!(
        metrics_obj.get("cells_compared").and_then(Value::as_u64),
        Some(5000)
    );
}
