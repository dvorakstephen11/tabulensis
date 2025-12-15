use excel_diff::{
    DataMashup, QueryChangeKind, SectionParseError, build_data_mashup, diff_m_queries,
    open_data_mashup,
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
fn basic_add_query_diff() {
    let dm_a = load_datamashup("m_add_query_a.xlsx");
    let dm_b = load_datamashup("m_add_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected exactly one diff for added query");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Bar");
    assert_eq!(diff.kind, QueryChangeKind::Added);
}

#[test]
fn basic_remove_query_diff() {
    let dm_a = load_datamashup("m_remove_query_a.xlsx");
    let dm_b = load_datamashup("m_remove_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "expected exactly one diff for removed query"
    );
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Bar");
    assert_eq!(diff.kind, QueryChangeKind::Removed);
}

#[test]
fn literal_change_produces_definitionchanged() {
    let dm_a = load_datamashup("m_change_literal_a.xlsx");
    let dm_b = load_datamashup("m_change_literal_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for changed literal");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn metadata_change_produces_metadataonly() {
    let dm_a = load_datamashup("m_metadata_only_change_a.xlsx");
    let dm_b = load_datamashup("m_metadata_only_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for metadata-only change");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::MetadataChangedOnly);
}

#[test]
fn definition_and_metadata_change_prefers_definitionchanged() {
    let dm_a = load_datamashup("m_def_and_metadata_change_a.xlsx");
    let dm_b = load_datamashup("m_def_and_metadata_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "expected one diff even when both definition and metadata change"
    );
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn identical_workbooks_produce_no_diffs() {
    let dm = load_datamashup("one_query.xlsx");

    let diffs =
        diff_m_queries(&dm, &dm, &excel_diff::DiffConfig::default()).expect("diff should succeed");

    assert!(
        diffs.is_empty(),
        "identical DataMashup should produce no diffs"
    );
}

#[test]
fn rename_reports_add_and_remove() {
    let dm_a = load_datamashup("m_rename_query_a.xlsx");
    let dm_b = load_datamashup("m_rename_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(diffs.len(), 2, "expected add + remove for rename scenario");

    assert_eq!(diffs[0].name, "Section1/Bar");
    assert_eq!(diffs[0].kind, QueryChangeKind::Added);
    assert_eq!(diffs[1].name, "Section1/Foo");
    assert_eq!(diffs[1].kind, QueryChangeKind::Removed);
}

#[test]
fn multiple_diffs_are_sorted_by_name() {
    let dm_a = datamashup_with_section(&["shared Zeta = 1;", "shared Bravo = 1;"]);
    let dm_b = datamashup_with_section(&["shared Alpha = 1;", "shared Delta = 1;"]);

    let diffs = diff_m_queries(&dm_a, &dm_b, &excel_diff::DiffConfig::default())
        .expect("diff should succeed");

    assert_eq!(diffs.len(), 4, "expected four diffs across both sides");
    assert!(
        diffs.windows(2).all(|w| w[0].name <= w[1].name),
        "diffs should already be sorted by name"
    );
    let names: Vec<_> = diffs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Section1/Alpha",
            "Section1/Bravo",
            "Section1/Delta",
            "Section1/Zeta"
        ],
        "lexicographic ordering should be preserved without resorting"
    );
}

#[test]
fn invalid_section_syntax_propagates_error() {
    let dm_invalid = datamashup_with_section(&["shared Broken // missing '=' and ';'"]);

    let result = diff_m_queries(&dm_invalid, &dm_invalid, &excel_diff::DiffConfig::default());

    assert!(matches!(
        result,
        Err(SectionParseError::InvalidMemberSyntax)
    ));
}
