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

#[test]
fn basic_add_query_diff() {
    let dm_a = load_datamashup("m_add_query_a.xlsx");
    let dm_b = load_datamashup("m_add_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected exactly one diff for added query");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Bar");
    assert_eq!(diff.kind, QueryChangeKind::Added);
}

#[test]
fn basic_remove_query_diff() {
    let dm_a = load_datamashup("m_remove_query_a.xlsx");
    let dm_b = load_datamashup("m_remove_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

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

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for changed literal");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn metadata_change_produces_metadataonly() {
    let dm_a = load_datamashup("m_metadata_only_change_a.xlsx");
    let dm_b = load_datamashup("m_metadata_only_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for metadata-only change");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::MetadataChangedOnly);
}

#[test]
fn identical_workbooks_produce_no_diffs() {
    let dm = load_datamashup("one_query.xlsx");

    let diffs = diff_m_queries(&dm, &dm).expect("diff should succeed");

    assert!(
        diffs.is_empty(),
        "identical DataMashup should produce no diffs"
    );
}

#[test]
fn rename_reports_add_and_remove() {
    let dm_a = load_datamashup("m_rename_query_a.xlsx");
    let dm_b = load_datamashup("m_rename_query_b.xlsx");

    let mut diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");
    diffs.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(diffs.len(), 2, "expected add + remove for rename scenario");

    let names: Vec<_> = diffs.iter().map(|d| (&d.name, &d.kind)).collect();
    assert!(
        names.contains(&(&"Section1/Foo".to_string(), &QueryChangeKind::Removed)),
        "Foo should be reported as Removed"
    );
    assert!(
        names.contains(&(&"Section1/Bar".to_string(), &QueryChangeKind::Added)),
        "Bar should be reported as Added"
    );
}
