use excel_diff::{DiffConfig, DiffOp, QueryChangeKind, WorkbookPackage};
use std::fs::File;

mod common;
use common::fixture_path;

fn load_pkg(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).expect("fixture file should open");
    WorkbookPackage::open(file).expect("fixture should parse")
}

fn m_ops<'a>(ops: &'a [DiffOp]) -> Vec<&'a DiffOp> {
    ops.iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::QueryAdded { .. }
                    | DiffOp::QueryRemoved { .. }
                    | DiffOp::QueryRenamed { .. }
                    | DiffOp::QueryDefinitionChanged { .. }
            )
        })
        .collect()
}

#[test]
fn record_reorder_is_masked_by_semantic_canonicalization() {
    let a = load_pkg("m_record_equiv_a.xlsx");
    let b = load_pkg("m_record_equiv_b.xlsx");

    let mut cfg = DiffConfig::default();
    cfg.semantic.enable_m_semantic_diff = true;
    let diff = a.diff(&b, &cfg);

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}

#[test]
fn list_formatting_only_is_masked() {
    let a = load_pkg("m_list_formatting_a.xlsx");
    let b = load_pkg("m_list_formatting_b.xlsx");

    let mut cfg = DiffConfig::default();
    cfg.semantic.enable_m_semantic_diff = true;
    let diff = a.diff(&b, &cfg);

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}

#[test]
fn call_formatting_only_is_masked() {
    let a = load_pkg("m_call_formatting_a.xlsx");
    let b = load_pkg("m_call_formatting_b.xlsx");

    let mut cfg = DiffConfig::default();
    cfg.semantic.enable_m_semantic_diff = true;
    let diff = a.diff(&b, &cfg);

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}

#[test]
fn primitive_formatting_only_is_masked() {
    let a = load_pkg("m_primitive_formatting_a.xlsx");
    let b = load_pkg("m_primitive_formatting_b.xlsx");

    let mut cfg = DiffConfig::default();
    cfg.semantic.enable_m_semantic_diff = true;
    let diff = a.diff(&b, &cfg);

    let ops = m_ops(&diff.ops);
    assert_eq!(ops.len(), 1);

    match ops[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(*change_kind, QueryChangeKind::FormattingOnly);
            assert_eq!(old_hash, new_hash);
        }
        _ => panic!("expected QueryDefinitionChanged"),
    }
}
