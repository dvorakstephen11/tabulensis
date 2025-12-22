use excel_diff::{
    DiffConfig, DiffOp, DiffReport, QueryChangeKind, SemanticNoisePolicy, WorkbookPackage,
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
fn formatting_only_diff_produces_formatting_only_change() {
    let pkg_a = load_package("m_formatting_only_a.xlsx");
    let pkg_b = load_package("m_formatting_only_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(
        def_changed.len(),
        1,
        "formatting-only changes should produce QueryDefinitionChanged with FormattingOnly kind"
    );

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::FormattingOnly,
                "formatting-only diff should have FormattingOnly change kind"
            );
            assert_eq!(
                old_hash, new_hash,
                "formatting-only changes have equal canonical hashes"
            );
        }
        _ => unreachable!(),
    }
    assert_eq!(resolve_name(&report, def_changed[0]), "Section1/FormatTest");
}

#[test]
fn semantic_gate_disabled_produces_semantic_change() {
    let pkg_a = load_package("m_formatting_only_a.xlsx");
    let pkg_b = load_package("m_formatting_only_b.xlsx");

    let config = DiffConfig {
        enable_m_semantic_diff: false,
        ..DiffConfig::default()
    };

    let report = pkg_a.diff(&pkg_b, &config);
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(
        def_changed.len(),
        1,
        "disabling semantic gate should surface formatting-only differences as Semantic"
    );

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::Semantic,
                "with semantic diff disabled, changes are reported as Semantic"
            );
            assert_ne!(
                old_hash, new_hash,
                "textual hashes should differ when semantic diff is disabled"
            );
        }
        _ => unreachable!(),
    }
    assert_eq!(resolve_name(&report, def_changed[0]), "Section1/FormatTest");
}

#[test]
fn formatting_variant_with_real_change_still_reports_semantic() {
    let pkg_b = load_package("m_formatting_only_b.xlsx");
    let pkg_b_variant = load_package("m_formatting_only_b_variant.xlsx");

    let report = pkg_b.diff(&pkg_b_variant, &DiffConfig::default());
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert_eq!(
        def_changed.len(),
        1,
        "expected exactly one diff for semantic change"
    );

    match def_changed[0] {
        DiffOp::QueryDefinitionChanged {
            change_kind,
            old_hash,
            new_hash,
            ..
        } => {
            assert_eq!(
                *change_kind,
                QueryChangeKind::Semantic,
                "real change should be reported as Semantic"
            );
            assert_ne!(
                old_hash, new_hash,
                "semantic changes should have different hashes"
            );
        }
        _ => unreachable!(),
    }
    assert_eq!(resolve_name(&report, def_changed[0]), "Section1/FormatTest");
}

#[test]
fn formatting_only_can_be_suppressed_by_policy() {
    let pkg_a = load_package("m_formatting_only_a.xlsx");
    let pkg_b = load_package("m_formatting_only_b.xlsx");

    let config = DiffConfig {
        semantic_noise_policy: SemanticNoisePolicy::SuppressFormattingOnly,
        ..DiffConfig::default()
    };

    let report = pkg_a.diff(&pkg_b, &config);
    let ops = m_ops(&report);

    let def_changed: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }))
        .collect();

    assert!(
        def_changed.is_empty(),
        "formatting-only changes should be suppressed under SuppressFormattingOnly"
    );
    assert!(
        ops.is_empty(),
        "suppressed formatting-only diff should emit no M ops"
    );
}

#[test]
fn semantic_gate_does_not_mask_metadata_only_change() {
    let pkg_a = load_package("m_metadata_only_change_a.xlsx");
    let pkg_b = load_package("m_metadata_only_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let metadata_ops: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, DiffOp::QueryMetadataChanged { .. }))
        .collect();

    assert!(
        !metadata_ops.is_empty(),
        "expected metadata changes to be reported"
    );
    assert_eq!(resolve_name(&report, metadata_ops[0]), "Section1/Foo");
}

#[test]
fn semantic_gate_does_not_mask_definition_plus_metadata_change() {
    let pkg_a = load_package("m_def_and_metadata_change_a.xlsx");
    let pkg_b = load_package("m_def_and_metadata_change_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let ops = m_ops(&report);

    let has_def_change = ops
        .iter()
        .any(|op| matches!(op, DiffOp::QueryDefinitionChanged { .. }));

    assert!(
        has_def_change,
        "expected QueryDefinitionChanged for definition+metadata change"
    );
}
