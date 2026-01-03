use excel_diff::{MAstAccessKind, MAstKind, canonicalize_m_ast, parse_m_expression};

fn kind(expr: &str) -> MAstKind {
    let mut ast = parse_m_expression(expr).expect("parse should succeed");
    canonicalize_m_ast(&mut ast);
    ast.root_kind_for_testing()
}

#[test]
fn parse_ident_ref() {
    assert_eq!(
        kind("Source"),
        MAstKind::Ident {
            name: "Source".to_string()
        }
    );

    assert_eq!(
        kind("#\"Previous Step\""),
        MAstKind::Ident {
            name: "Previous Step".to_string()
        }
    );
}

#[test]
fn parse_field_access() {
    assert_eq!(
        kind("Source[Field]"),
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 1
        }
    );
}

#[test]
fn parse_item_access() {
    assert_eq!(
        kind("Source{0}"),
        MAstKind::Access {
            kind: MAstAccessKind::Item,
            chain_len: 1
        }
    );
}

#[test]
fn parse_access_chain() {
    assert_eq!(
        kind("Source{0}[Content]"),
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 2
        }
    );
}

#[test]
fn parse_if_then_else() {
    assert_eq!(kind("if true then 1 else 0"), MAstKind::If);
}

#[test]
fn parse_each_expr() {
    assert_eq!(kind("each _ + 1"), MAstKind::Each);
}

#[test]
fn quoted_identifier_named_then_does_not_confuse_if_parser() {
    let expr = r##"if #"then" then 1 else 0"##;
    assert_eq!(kind(expr), MAstKind::If);
}
