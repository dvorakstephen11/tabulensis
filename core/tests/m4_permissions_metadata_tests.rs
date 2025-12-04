use excel_diff::{
    DataMashupError, Permissions, RawDataMashup, build_data_mashup, datamashup::parse_metadata,
    open_data_mashup, parse_package_parts, parse_section_members,
};

mod common;
use common::fixture_path;

fn load_datamashup(path: &str) -> excel_diff::DataMashup {
    let raw = open_data_mashup(fixture_path(path))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

#[test]
fn permissions_parsed_flags_default_vs_firewall_off() {
    let defaults = load_datamashup("permissions_defaults.xlsx");
    let firewall_off = load_datamashup("permissions_firewall_off.xlsx");

    assert_eq!(defaults.version, 0);
    assert_eq!(firewall_off.version, 0);

    assert!(defaults.permissions.firewall_enabled);
    assert!(!defaults.permissions.can_evaluate_future_packages);
    assert!(!firewall_off.permissions.firewall_enabled);
    assert_eq!(
        defaults.permissions.workbook_group_type,
        firewall_off.permissions.workbook_group_type
    );
}

#[test]
fn permissions_missing_or_malformed_yields_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut missing = base_raw.clone();
    missing.permissions = Vec::new();
    missing.permission_bindings = Vec::new();
    let dm = build_data_mashup(&missing).expect("missing permissions should default");
    assert_eq!(dm.permissions, Permissions::default());

    let mut malformed = base_raw.clone();
    malformed.permissions = b"<not-xml".to_vec();
    let dm = build_data_mashup(&malformed).expect("malformed permissions should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn permissions_invalid_entities_yield_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let invalid_permissions = br#"
        <Permissions>
            <CanEvaluateFuturePackages>&bad;</CanEvaluateFuturePackages>
            <FirewallEnabled>true</FirewallEnabled>
        </Permissions>
    "#;
    let mut raw = base_raw.clone();
    raw.permissions = invalid_permissions.to_vec();

    let dm = build_data_mashup(&raw).expect("invalid permissions entities should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn metadata_empty_bytes_returns_empty_struct() {
    let metadata = parse_metadata(&[]).expect("empty metadata should parse");
    assert!(metadata.formulas.is_empty());
}

#[test]
fn metadata_invalid_header_too_short_errors() {
    let err = parse_metadata(&[0x01]).expect_err("short metadata should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata XML not found"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_length_prefix_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&100u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 10]);

    let err = parse_metadata(&bytes).expect_err("invalid length prefix should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata length prefix invalid"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_utf8_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&[0xFF, 0xFF]);

    let err = parse_metadata(&bytes).expect_err("invalid utf-8 should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata is not valid UTF-8"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_malformed_xml_errors() {
    let xml = b"<LocalPackageMetadataFile><foo";
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    bytes.extend_from_slice(xml);

    let err = parse_metadata(&bytes).expect_err("malformed xml should error");
    match err {
        DataMashupError::XmlError(_) => {}
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_formulas_match_section_members() {
    let raw = open_data_mashup(fixture_path("metadata_simple.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    let package = parse_package_parts(&raw.package_parts).expect("package parts should parse");
    let metadata = parse_metadata(&raw.metadata).expect("metadata should parse");
    let members =
        parse_section_members(&package.main_section.source).expect("section members should parse");

    let section1_formulas: Vec<_> = metadata
        .formulas
        .iter()
        .filter(|m| m.section_name == "Section1" && !m.is_connection_only)
        .collect();

    assert_eq!(section1_formulas.len(), members.len());
    for meta in section1_formulas {
        assert!(!meta.formula_name.is_empty());
    }
}

#[test]
fn metadata_load_destinations_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let load_to_sheet = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToSheet")
        .expect("LoadToSheet metadata missing");
    assert!(load_to_sheet.load_to_sheet);
    assert!(!load_to_sheet.load_to_model);
    assert!(!load_to_sheet.is_connection_only);

    let load_to_model = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToModel")
        .expect("LoadToModel metadata missing");
    assert!(!load_to_model.load_to_sheet);
    assert!(load_to_model.load_to_model);
    assert!(!load_to_model.is_connection_only);
}

#[test]
fn metadata_groups_basic_hierarchy() {
    let dm = load_datamashup("metadata_query_groups.xlsx");
    let grouped = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/GroupedFoo")
        .expect("GroupedFoo metadata missing");
    assert_eq!(grouped.group_path.as_deref(), Some("Inputs/DimTables"));

    let root = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/RootQuery")
        .expect("RootQuery metadata missing");
    assert!(root.group_path.is_none());
}

#[test]
fn metadata_hidden_queries_connection_only() {
    let dm = load_datamashup("metadata_hidden_queries.xlsx");
    let has_connection_only = dm
        .metadata
        .formulas
        .iter()
        .any(|m| !m.load_to_sheet && !m.load_to_model && m.is_connection_only);
    assert!(has_connection_only);
}

#[test]
fn metadata_itempath_decodes_percent_encoded_utf8() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%C3%A9</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar\u{00e9}");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar\u{00e9}");
    assert!(item.load_to_sheet);
    assert!(!item.is_connection_only);
}

#[test]
fn metadata_itempath_decodes_space_and_slash() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%2FInner</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar/Inner");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar/Inner");
}

#[test]
fn permission_bindings_present_flag() {
    let dm = load_datamashup("permissions_defaults.xlsx");
    assert!(!dm.permission_bindings_raw.is_empty());
}

#[test]
fn permission_bindings_missing_ok() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut synthetic = RawDataMashup {
        permission_bindings: Vec::new(),
        ..base_raw.clone()
    };
    synthetic.permissions = Vec::new();
    synthetic.metadata = Vec::new();

    let dm = build_data_mashup(&synthetic).expect("empty bindings should build");
    assert!(dm.permission_bindings_raw.is_empty());
    assert_eq!(dm.permissions, Permissions::default());
}
