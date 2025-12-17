use excel_diff::{canonicalize_m_ast, parse_m_expression, MAstKind};

fn assert_opaque(expr: &str) {
    let mut ast = parse_m_expression(expr).expect("expression should parse into an AST container");
    canonicalize_m_ast(&mut ast);
    match ast.root_kind_for_testing() {
        MAstKind::Opaque { token_count } => {
            assert!(token_count > 0, "opaque token_count must be > 0")
        }
        other => panic!("expected Opaque, got {:?}", other),
    }
}

#[test]
fn coverage_audit_unsupported_constructs_are_opaque() {
    let cases = [
        "Source",
        "#\"Previous Step\"",
        "if true then 1 else 0",
        "each _ + 1",
        "(x) => x",
        "1 + 2",
        "not true",
        "Source[Field]",
        "Source{0}",
        "Source{0}[Content]",
        "x as number",
    ];

    for expr in cases {
        assert_opaque(expr);
    }
}
