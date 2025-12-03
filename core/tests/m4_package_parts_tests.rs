use std::io::{Cursor, Write};

use excel_diff::{DataMashupError, open_data_mashup, parse_package_parts, parse_section_members};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

mod common;
use common::fixture_path;

const MIN_PACKAGE_XML: &str = "<Package></Package>";
const MIN_SECTION: &str = "section Section1;\nshared Foo = 1;";
const BOM_SECTION: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

#[test]
fn package_parts_contains_expected_entries() {
    let path = fixture_path("one_query.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(
        parts.main_section.source.contains("section Section1;"),
        "main Section1.m should be present"
    );
    assert!(
        parts.main_section.source.contains("shared"),
        "at least one shared query should be present"
    );
    assert!(
        parts.embedded_contents.is_empty(),
        "one_query.xlsx should not contain embedded contents"
    );
}

#[test]
fn embedded_content_detection() {
    let path = fixture_path("multi_query_with_embedded.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(
        !parts.embedded_contents.is_empty(),
        "multi_query_with_embedded.xlsx should expose at least one embedded content"
    );

    for embedded in &parts.embedded_contents {
        assert!(
            embedded.section.source.contains("section Section1"),
            "embedded Section1.m should be present for {}",
            embedded.name
        );
        assert!(
            embedded.section.source.contains("shared"),
            "embedded Section1.m should contain at least one shared member for {}",
            embedded.name
        );
    }
}

#[test]
fn parse_package_parts_rejects_non_zip() {
    let bogus = b"this is not a zip file";
    let err = parse_package_parts(bogus).expect_err("non-zip bytes should fail");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_config_package_xml_errors() {
    let bytes = build_zip(vec![(
        "Formulas/Section1.m",
        MIN_SECTION.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Config/Package.xml should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_section1_errors() {
    let bytes = build_zip(vec![(
        "Config/Package.xml",
        MIN_PACKAGE_XML.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Formulas/Section1.m should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_package_xml_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", vec![0xFF, 0xFF, 0xFF]),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
    ]);
    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Package.xml should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_section1_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", vec![0xFF, 0xFF]),
    ]);

    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Section1.m should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn embedded_content_invalid_zip_is_skipped() {
    let bytes =
        build_minimal_package_parts_with(vec![("Content/bogus.package", b"not a zip".to_vec())]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_missing_section1_is_skipped() {
    let nested = build_zip(vec![("Config/Formulas.xml", b"<Formulas/>".to_vec())]);
    let bytes = build_minimal_package_parts_with(vec![("Content/no_section1.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_invalid_utf8_is_skipped() {
    let nested = build_zip(vec![("Formulas/Section1.m", vec![0xFF, 0xFF])]);
    let bytes = build_minimal_package_parts_with(vec![("Content/bad_utf8.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_partial_failure_retains_valid_entries() {
    let good_nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![
        ("Content/good.package", good_nested),
        ("Content/bad.package", b"not a zip".to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    let embedded = &parts.embedded_contents[0];
    assert_eq!(embedded.name, "Content/good.package");
    assert!(embedded.section.source.contains("section Section1;"));
    assert!(embedded.section.source.contains("shared"));
}

#[test]
fn leading_slash_paths_are_accepted() {
    let embedded =
        build_embedded_section_zip("section Section1;\nshared Bar = 2;".as_bytes().to_vec());
    let bytes = build_zip(vec![
        (
            "/Config/Package.xml",
            br#"<Package from="leading"/>"#.to_vec(),
        ),
        ("/Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("/Content/abcd.package", embedded),
        (
            "Config/Package.xml",
            br#"<Package from="canonical"/>"#.to_vec(),
        ),
    ]);

    let parts = parse_package_parts(&bytes).expect("leading slash entries should parse");
    assert!(
        parts.package_xml.raw_xml.contains(r#"from="leading""#),
        "first encountered Package.xml should win"
    );
    assert!(parts.main_section.source.contains("shared Foo = 1;"));
    assert_eq!(parts.embedded_contents.len(), 1);
    assert!(
        parts.embedded_contents[0]
            .section
            .source
            .contains("shared Bar = 2;")
    );
}

#[test]
fn embedded_content_name_is_canonicalized() {
    let nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![("/Content/efgh.package", nested)]);

    let parts =
        parse_package_parts(&bytes).expect("embedded content with leading slash should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    assert_eq!(parts.embedded_contents[0].name, "Content/efgh.package");
}

#[test]
fn empty_content_directory_is_ignored() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/", Vec::new()),
    ]);

    let parts = parse_package_parts(&bytes).expect("package with empty Content/ directory parses");
    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(!parts.main_section.source.is_empty());
    assert!(
        parts.embedded_contents.is_empty(),
        "bare Content/ directory should not produce embedded contents"
    );
}

#[test]
fn parse_package_parts_never_panics_on_random_bytes() {
    for seed in 0u64..64 {
        let len = (seed as usize * 13 % 256) + (seed as usize % 7);
        let bytes = random_bytes(seed, len);
        let _ = parse_package_parts(&bytes);
    }
}

#[test]
fn package_parts_section1_with_bom_parses_via_parse_section_members() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", BOM_SECTION.as_bytes().to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("BOM-prefixed Section1.m should parse");
    assert!(
        !parts.main_section.source.starts_with('\u{FEFF}'),
        "PackageParts should strip a single leading BOM from Section1.m"
    );
    let members = parse_section_members(&parts.main_section.source)
        .expect("parse_section_members should accept BOM-prefixed Section1");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
}

#[test]
fn embedded_content_section1_with_bom_parses_via_parse_section_members() {
    let embedded = build_embedded_section_zip(BOM_SECTION.as_bytes().to_vec());
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/bom_embedded.package", embedded),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(
        parts.embedded_contents.len() >= 1,
        "embedded package should be detected"
    );

    let embedded = parts
        .embedded_contents
        .iter()
        .find(|entry| entry.name == "Content/bom_embedded.package")
        .expect("expected embedded package to round-trip name");

    assert!(
        !embedded.section.source.starts_with('\u{FEFF}'),
        "embedded Section1.m should strip leading BOM"
    );

    let members = parse_section_members(&embedded.section.source)
        .expect("parse_section_members should accept embedded BOM Section1");
    assert!(
        !members.is_empty(),
        "embedded Section1.m should contain members"
    );
    assert!(
        members.iter().any(|member| {
            member.section_name == "Section1"
                && member.member_name == "Foo"
                && member.expression_m == "1"
        }),
        "embedded Section1.m should parse shared Foo = 1"
    );
}

fn build_minimal_package_parts_with(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut all_entries = Vec::with_capacity(entries.len() + 2);
    all_entries.push(("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()));
    all_entries.push(("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()));
    all_entries.extend(entries);
    build_zip(all_entries)
}

fn build_embedded_section_zip(section_bytes: Vec<u8>) -> Vec<u8> {
    build_zip(vec![("Formulas/Section1.m", section_bytes)])
}

fn build_zip(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, bytes) in entries {
        if name.ends_with('/') {
            writer
                .add_directory(name, options)
                .expect("start zip directory");
        } else {
            writer.start_file(name, options).expect("start zip entry");
            writer.write_all(&bytes).expect("write zip entry");
        }
    }

    writer.finish().expect("finish zip").into_inner()
}

fn random_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(len);
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    for _ in 0..len {
        state = state
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        bytes.push((state >> 32) as u8);
    }
    bytes
}
