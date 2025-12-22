use excel_diff::{DiffConfig, DiffOp, DiffReport, QueryChangeKind, WorkbookPackage};
use std::fs::File;

mod common;
use common::fixture_path;

fn load_package(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).expect("fixture file should open");
    WorkbookPackage::open(file).expect("fixture should parse as WorkbookPackage")
}

fn m_ops(report: &DiffReport) -> Vec<&DiffOp> {
    report.m_ops().collect()
}

fn resolve_name<'a>(report: &'a DiffReport, op: &DiffOp) -> &'a str {
    let name_id = match op {
        DiffOp::QueryAdded { name } => *name,
        DiffOp::QueryRemoved { name } => *name,
        DiffOp::QueryRenamed { from, .. } => *from,
        DiffOp::QueryDefinitionChanged { name, .. } => *name,
        DiffOp::QueryMetadataChanged { name, .. } => *name,
        _ => panic!("not a query op"),
    };
    &report.strings[name_id.0 as usize]
}

#[test]
fn embedded_only_change_produces_embedded_definitionchanged() {
    let pkg_a = load_package("m_embedded_change_a.xlsx");
    let pkg_b = load_package("m_embedded_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert_eq!(ops.len(), 1, "expected exactly one diff for embedded change");

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(def_changed.len(), 1, "expected one definition change");

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged { change_kind, .. } => {
            assert_eq!(*change_kind, QueryChangeKind::Semantic);
        }
        _ => unreachable!(),
    }

    assert_eq!(
        resolve_name(&report, def_changed[0]),
        "Embedded/Content/efgh.package/Section1/Inner"
    );
}
