use std::collections::HashSet;

use excel_diff::{build_data_mashup, build_queries, open_data_mashup, parse_section_members};

mod common;
use common::fixture_path;

fn load_datamashup(path: &str) -> excel_diff::DataMashup {
    let raw = open_data_mashup(fixture_path(path))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

#[test]
fn metadata_join_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 2);
    let names: HashSet<_> = queries.iter().map(|q| q.name.as_str()).collect();
    assert_eq!(
        names,
        HashSet::from(["Section1/LoadToSheet", "Section1/LoadToModel"])
    );

    let sheet = queries
        .iter()
        .find(|q| q.section_member == "LoadToSheet")
        .expect("LoadToSheet query missing");
    assert!(sheet.metadata.load_to_sheet);
    assert!(!sheet.metadata.load_to_model);

    let model = queries
        .iter()
        .find(|q| q.section_member == "LoadToModel")
        .expect("LoadToModel query missing");
    assert!(!model.metadata.load_to_sheet);
    assert!(model.metadata.load_to_model);
}

#[test]
fn metadata_join_url_encoding() {
    let dm = load_datamashup("metadata_url_encoding.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    let q = &queries[0];
    assert_eq!(q.name, "Section1/Query with space & #");
    assert_eq!(q.section_member, "Query with space & #");
    assert!(q.metadata.load_to_sheet || q.metadata.load_to_model);
}

#[test]
fn member_without_metadata_is_preserved() {
    let dm = load_datamashup("metadata_missing_entry.xlsx");
    assert!(dm.metadata.formulas.is_empty());
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    let q = &queries[0];
    assert_eq!(q.name, "Section1/MissingMetadata");
    assert_eq!(q.section_member, "MissingMetadata");
    assert_eq!(q.metadata.item_path, "Section1/MissingMetadata");
    assert!(!q.metadata.load_to_sheet);
    assert!(!q.metadata.load_to_model);
    assert!(q.metadata.is_connection_only);
    assert_eq!(q.metadata.group_path, None);
}

#[test]
fn query_names_unique() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    let mut seen = HashSet::new();
    for q in &queries {
        assert!(seen.insert(&q.name));
    }
}

#[test]
fn metadata_orphan_entries() {
    let dm = load_datamashup("metadata_orphan_entries.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "Section1/Foo");
    assert!(
        dm.metadata
            .formulas
            .iter()
            .any(|m| m.item_path == "Section1/Nonexistent")
    );
}

#[test]
fn queries_preserve_section_member_order() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let members = parse_section_members(&dm.package_parts.main_section.source)
        .expect("Section1 should parse");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(members.len(), queries.len());
    for (idx, (member, query)) in members.iter().zip(queries.iter()).enumerate() {
        assert_eq!(
            query.section_member, member.member_name,
            "query at position {} should match Section1 member order",
            idx
        );
    }
}
