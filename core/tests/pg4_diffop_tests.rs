use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ColSignature, DiffOp, DiffReport, RowSignature,
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
        formula: formula.map(|s| s.to_string()),
    }
}

fn sample_cell_edited() -> DiffOp {
    DiffOp::CellEdited {
        sheet: "Sheet1".to_string(),
        addr: addr("C3"),
        from: snapshot("C3", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    }
}

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
        assert_eq!(sheet, expected_sheet);
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
        DiffOp::CellEdited { .. } => "CellEdited",
    }
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
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 8,
        row_signature: None,
    };
    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 0,
        col_signature: None,
    };

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_with_sig
    {
        assert_eq!(sheet, "Sheet1");
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
        assert_eq!(sheet, "Sheet1");
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
        assert_eq!(sheet, "Sheet1");
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
        assert_eq!(sheet, "Sheet1");
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
        assert_eq!(sheet, "Sheet2");
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
        assert_eq!(sheet, "Sheet2");
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
        assert_eq!(sheet, "Sheet2");
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
        assert_eq!(sheet, "Sheet2");
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
        sheet: "Sheet1".to_string(),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
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
        assert_eq!(sheet, "Sheet1");
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
        assert_eq!(sheet, "Sheet1");
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
        assert_eq!(sheet, "Sheet2");
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
        assert_eq!(sheet, "Sheet2");
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
        sheet: "Sheet1".to_string(),
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
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 123 }),
    };
    let json_with = serde_json::to_value(&op_with_sig).expect("serialize with sig");
    assert_eq!(json_with["row_signature"]["hash"], 123);
}

#[test]
fn pg4_column_added_json_optional_signature() {
    let added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet1".to_string(),
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
        sheet: "Sheet1".to_string(),
        col_idx: 6,
        col_signature: Some(ColSignature { hash: 321 }),
    };
    let json_added_with = serde_json::to_value(&added_with_sig).expect("serialize with sig");
    assert_eq!(json_added_with["col_signature"]["hash"], 321);

    let removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: None,
    };
    let json_removed_without =
        serde_json::to_value(&removed_without_sig).expect("serialize removed no sig");
    let obj_removed_without = json_removed_without.as_object().expect("object json");
    assert_eq!(json_removed_without["kind"], "ColumnRemoved");
    assert!(obj_removed_without.get("col_signature").is_none());

    let removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 654 }),
    };
    let json_removed_with =
        serde_json::to_value(&removed_with_sig).expect("serialize removed with sig");
    assert_eq!(json_removed_with["col_signature"]["hash"], 654);
}

#[test]
fn pg4_block_moved_rows_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
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
        sheet: "Sheet1".to_string(),
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
        sheet: "SheetX".to_string(),
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
        sheet: "SheetX".to_string(),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: Some(4242),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(4242));
}

#[test]
fn pg4_diffop_roundtrip_each_variant() {
    let ops = vec![
        DiffOp::SheetAdded {
            sheet: "SheetA".to_string(),
        },
        DiffOp::SheetRemoved {
            sheet: "SheetB".to_string(),
        },
        DiffOp::RowAdded {
            sheet: "Sheet1".to_string(),
            row_idx: 1,
            row_signature: Some(RowSignature { hash: 42 }),
        },
        DiffOp::RowRemoved {
            sheet: "Sheet1".to_string(),
            row_idx: 0,
            row_signature: None,
        },
        DiffOp::ColumnAdded {
            sheet: "Sheet1".to_string(),
            col_idx: 2,
            col_signature: None,
        },
        DiffOp::ColumnRemoved {
            sheet: "Sheet1".to_string(),
            col_idx: 3,
            col_signature: Some(ColSignature { hash: 99 }),
        },
        DiffOp::BlockMovedRows {
            sheet: "Sheet1".to_string(),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: Some(1234),
        },
        DiffOp::BlockMovedRows {
            sheet: "Sheet1".to_string(),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: None,
        },
        DiffOp::BlockMovedColumns {
            sheet: "Sheet2".to_string(),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: Some(888),
        },
        DiffOp::BlockMovedColumns {
            sheet: "Sheet2".to_string(),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
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
        sheet: "Sheet1".to_string(),
    };
    let op2 = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
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
            sheet: "SheetX".to_string(),
        },
        DiffOp::RowRemoved {
            sheet: "SheetX".to_string(),
            row_idx: 3,
            row_signature: Some(RowSignature { hash: 7 }),
        },
    ];
    let report = DiffReport::new(ops);
    let json = serde_json::to_value(&report).expect("serialize to value");

    let obj = json.as_object().expect("report json object");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["ops", "version"].into_iter().map(String::from).collect();
    assert_eq!(keys, expected);
    assert_eq!(obj.get("version").and_then(Value::as_str), Some("1"));

    let ops_json = obj
        .get("ops")
        .and_then(Value::as_array)
        .expect("ops should be array");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "SheetRemoved");
    assert_eq!(ops_json[1]["kind"], "RowRemoved");
}

#[test]
#[should_panic]
fn pg4_cell_edited_invariant_helper_rejects_mismatched_snapshot_addr() {
    let op = DiffOp::CellEdited {
        sheet: "Sheet1".to_string(),
        addr: addr("C3"),
        from: snapshot("D4", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    };

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
}
