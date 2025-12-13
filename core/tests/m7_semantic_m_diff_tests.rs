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

fn datamashup_with_section(lines: &[&str]) -> DataMashup {
    let mut dm = load_datamashup("one_query.xlsx");
    let body = lines.join("\n");
    dm.package_parts.main_section.source = format!("section Section1;\n\n{body}\n");
    dm
}

#[test]
fn formatting_only_diff_produces_no_diffs() {
    let dm_a = load_datamashup("m_formatting_only_a.xlsx");
    let dm_b = load_datamashup("m_formatting_only_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert!(
        diffs.is_empty(),
        "formatting-only changes should be ignored, got {:?}",
        diffs
    );
}

#[test]
fn semantic_gate_can_be_disabled() {
    let dm_a = load_datamashup("m_formatting_only_a.xlsx");
    let dm_b = load_datamashup("m_formatting_only_b.xlsx");

    let mut config = excel_diff::DiffConfig::default();
    config.enable_m_semantic_diff = false;

    let diffs = diff_m_queries(&dm_a, &dm_b, &config).expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "disabling semantic gate should surface formatting-only differences"
    );
    assert_eq!(diffs[0].name, "Section1/FormatTest");
    assert_eq!(diffs[0].kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn formatting_variant_with_real_change_still_reports_definitionchanged() {
    let dm_b = load_datamashup("m_formatting_only_b.xlsx");
    let dm_b_variant = load_datamashup("m_formatting_only_b_variant.xlsx");

    let diffs = diff_m_queries(&dm_b, &dm_b_variant, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "expected exactly one diff for semantic change"
    );
    assert_eq!(diffs[0].name, "Section1/FormatTest");
    assert_eq!(diffs[0].kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn semantic_gate_does_not_mask_metadata_only_change() {
    let dm_a = load_datamashup("m_metadata_only_change_a.xlsx");
    let dm_b = load_datamashup("m_metadata_only_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "expected exactly one diff for metadata-only change"
    );
    assert_eq!(diffs[0].name, "Section1/Foo");
    assert_eq!(diffs[0].kind, QueryChangeKind::MetadataChangedOnly);
}

#[test]
fn semantic_gate_does_not_mask_definition_plus_metadata_change() {
    let dm_a = load_datamashup("m_def_and_metadata_change_a.xlsx");
    let dm_b = load_datamashup("m_def_and_metadata_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "expected exactly one diff for definition+metadata change"
    );
    assert_eq!(diffs[0].name, "Section1/Foo");
    assert_eq!(diffs[0].kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn semantic_gate_falls_back_on_ast_parse_failure() {
    let dm_a = datamashup_with_section(&["shared Foo = let Source = 1 in Source;"]);
    let dm_b = datamashup_with_section(&["shared Foo = let Source = (1;"]);

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed (not panic on AST failure)");

    assert_eq!(
        diffs.len(),
        1,
        "expected one diff when AST parse fails on one side"
    );
    assert_eq!(diffs[0].name, "Section1/Foo");
    assert_eq!(
        diffs[0].kind,
        QueryChangeKind::DefinitionChanged,
        "should fall back to textual diff when AST parse fails"
    );
}

#[test]
fn semantic_gate_falls_back_when_both_sides_malformed() {
    let dm_a = datamashup_with_section(&["shared Foo = let Source = (1;"]);
    let dm_b = datamashup_with_section(&["shared Foo = let Source = (2;"]);

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed (not panic on AST failure)");

    assert_eq!(
        diffs.len(),
        1,
        "expected one diff when AST parse fails on both sides"
    );
    assert_eq!(diffs[0].name, "Section1/Foo");
    assert_eq!(
        diffs[0].kind,
        QueryChangeKind::DefinitionChanged,
        "should fall back to textual diff when both sides fail AST parse"
    );
}
