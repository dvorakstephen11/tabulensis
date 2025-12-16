use excel_diff::{
    DiffConfig, DiffOp, DiffReport, QueryChangeKind, QueryMetadataField, WorkbookPackage,
};
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
fn basic_add_query_diff() {
    let pkg_a = load_package("m_add_query_a.xlsx");
    let pkg_b = load_package("m_add_query_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert_eq!(ops.len(), 1, "expected exactly one diff for added query");
    assert!(
        matches!(ops[0], DiffOp::QueryAdded { .. }),
        "expected QueryAdded"
    );
    assert_eq!(resolve_name(&report, ops[0]), "Section1/Bar");
}

#[test]
fn basic_remove_query_diff() {
    let pkg_a = load_package("m_remove_query_a.xlsx");
    let pkg_b = load_package("m_remove_query_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert_eq!(ops.len(), 1, "expected exactly one diff for removed query");
    assert!(
        matches!(ops[0], DiffOp::QueryRemoved { .. }),
        "expected QueryRemoved"
    );
    assert_eq!(resolve_name(&report, ops[0]), "Section1/Bar");
}

#[test]
fn literal_change_produces_definitionchanged() {
    let pkg_a = load_package("m_change_literal_a.xlsx");
    let pkg_b = load_package("m_change_literal_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert_eq!(ops.len(), 1, "expected one diff for changed literal");
    match ops[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::Semantic,
                "literal change is semantic"
            );
            assert_ne!(old_hash, new_hash, "hashes should differ for semantic change");
        }
        _ => panic!("expected QueryDefinitionChanged, got {:?}", ops[0]),
    }
    assert_eq!(resolve_name(&report, ops[0]), "Section1/Foo");
}

#[test]
fn metadata_change_produces_metadata_ops() {
    let pkg_a = load_package("m_metadata_only_change_a.xlsx");
    let pkg_b = load_package("m_metadata_only_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    assert!(
        !ops.is_empty(),
        "expected at least one diff for metadata change"
    );
    for op in &ops {
        match op {
            DiffOp::QueryMetadataChanged { field, .. } => {
                assert!(
                    matches!(
                        field,
                        QueryMetadataField::LoadToSheet
                            | QueryMetadataField::LoadToModel
                            | QueryMetadataField::GroupPath
                            | QueryMetadataField::ConnectionOnly
                    ),
                    "expected a recognized metadata field"
                );
            }
            _ => panic!("expected only QueryMetadataChanged ops, got {:?}", op),
        }
    }
}

#[test]
fn definition_and_metadata_change_produces_both() {
    let pkg_a = load_package("m_def_and_metadata_change_a.xlsx");
    let pkg_b = load_package("m_def_and_metadata_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let has_definition_change = ops
        .iter()
        .any(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }));
    assert!(
        has_definition_change,
        "expected QueryDefinitionChanged when definition changes"
    );
}

#[test]
fn identical_workbooks_produce_no_diffs() {
    let pkg = load_package("one_query.xlsx");

    let report = pkg.diff(&pkg, &DiffConfig::default());
    let ops = m_ops(&report);

    assert!(
        ops.is_empty(),
        "identical WorkbookPackage should produce no M diffs"
    );
}

#[test]
fn rename_produces_query_renamed() {
    let pkg_a = load_package("m_rename_query_a.xlsx");
    let pkg_b = load_package("m_rename_query_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let renamed_ops: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryRenamed { .. }))
        .collect();

    assert_eq!(
        renamed_ops.len(),
        1,
        "expected exactly one QueryRenamed op for rename scenario"
    );

    match renamed_ops[0] {
        DiffOp::QueryRenamed { from, to } => {
            let from_name = &report.strings[from.0 as usize];
            let to_name = &report.strings[to.0 as usize];
            assert_eq!(from_name, "Section1/Foo");
            assert_eq!(to_name, "Section1/Bar");
        }
        _ => unreachable!(),
    }
}

