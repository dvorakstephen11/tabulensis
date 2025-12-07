use excel_diff::{
    DataMashup, QueryChangeKind, build_data_mashup, diff_m_queries, open_data_mashup,
};

mod common;
use common::fixture_path;

fn load_datamashup(name: &str) -> DataMashup {
    let raw = open_data_mashup(fixture_path(name))
        .expect("fixture should open")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

fn datamashup_with_expression(expression: &str) -> DataMashup {
    let mut dm = load_datamashup("one_query.xlsx");
    dm.package_parts.main_section.source =
        format!("section Section1;\n\nshared Foo = {expression};\n");
    dm
}

#[test]
fn formatting_only_diff_produces_no_diffs() {
    let dm_a = load_datamashup("m_formatting_only_a.xlsx");
    let dm_b = load_datamashup("m_formatting_only_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert!(
        diffs.is_empty(),
        "formatting-only changes should be ignored"
    );
}

#[test]
fn formatting_variant_with_real_change_still_reports_definitionchanged() {
    let dm_b = load_datamashup("m_formatting_only_b.xlsx");
    let dm_variant = load_datamashup("m_formatting_only_b_variant.xlsx");

    let diffs = diff_m_queries(&dm_b, &dm_variant).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for semantic change");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/FormatTest");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn semantic_gate_does_not_mask_metadata_only_or_definition_plus_metadata_changes() {
    let dm_meta_a = load_datamashup("m_metadata_only_change_a.xlsx");
    let dm_meta_b = load_datamashup("m_metadata_only_change_b.xlsx");

    let meta_diffs = diff_m_queries(&dm_meta_a, &dm_meta_b).expect("diff should succeed");
    assert_eq!(
        meta_diffs.len(),
        1,
        "metadata-only change should still be reported"
    );
    let meta_diff = &meta_diffs[0];
    assert_eq!(meta_diff.name, "Section1/Foo");
    assert_eq!(meta_diff.kind, QueryChangeKind::MetadataChangedOnly);

    let dm_both_a = load_datamashup("m_def_and_metadata_change_a.xlsx");
    let dm_both_b = load_datamashup("m_def_and_metadata_change_b.xlsx");

    let both_diffs = diff_m_queries(&dm_both_a, &dm_both_b).expect("diff should succeed");
    assert_eq!(
        both_diffs.len(),
        1,
        "definition+metadata change should prefer DefinitionChanged"
    );
    let diff = &both_diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn semantic_gate_falls_back_on_ast_parse_failure() {
    let dm_invalid = datamashup_with_expression("let Source = 1");
    let dm_valid = datamashup_with_expression("let Source = 1 in Source");

    let diffs = diff_m_queries(&dm_invalid, &dm_valid)
        .expect("diff should fall back to textual path when AST parse fails");

    assert_eq!(diffs.len(), 1, "fallback should still surface a diff");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}
