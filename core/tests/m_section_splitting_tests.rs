use excel_diff::{SectionParseError, parse_section_members};

const SECTION_SINGLE: &str = r#"
    section Section1;

    shared Foo = 1;
"#;

const SECTION_MULTI: &str = r#"
    section Section1;

    shared Foo = 1;
    shared Bar = 2;
    Baz = 3;
"#;

const SECTION_NOISY: &str = r#"

// Leading comment

section Section1;

// Comment before Foo
shared Foo = 1;

// Another comment

    shared   Bar   =    2    ;

"#;

const SECTION_WITH_BOM: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

#[test]
fn parse_single_member_section() {
    let members = parse_section_members(SECTION_SINGLE).expect("single member section parses");
    assert_eq!(members.len(), 1);

    let foo = &members[0];
    assert_eq!(foo.section_name, "Section1");
    assert_eq!(foo.member_name, "Foo");
    assert_eq!(foo.expression_m, "1");
    assert!(foo.is_shared);
}

#[test]
fn parse_multiple_members() {
    let members = parse_section_members(SECTION_MULTI).expect("multi-member section parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].section_name, "Section1");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
}

#[test]
fn tolerate_whitespace_comments() {
    let members = parse_section_members(SECTION_NOISY).expect("noisy section still parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);
    assert_eq!(members[0].section_name, "Section1");

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
    assert_eq!(members[1].section_name, "Section1");
}

#[test]
fn error_on_missing_section_header() {
    const NO_SECTION: &str = r#"
        shared Foo = 1;
    "#;

    let result = parse_section_members(NO_SECTION);
    assert_eq!(result, Err(SectionParseError::MissingSectionHeader));
}

#[test]
fn section_parsing_tolerates_utf8_bom() {
    let members =
        parse_section_members(SECTION_WITH_BOM).expect("BOM-prefixed section should parse");
    assert_eq!(members.len(), 1);

    let member = &members[0];
    assert_eq!(member.member_name, "Foo");
    assert_eq!(member.section_name, "Section1");
    assert_eq!(member.expression_m, "1");
    assert!(member.is_shared);
}
