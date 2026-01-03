use excel_diff::{MAstAccessKind, MAstKind, canonicalize_m_ast, parse_m_expression};

fn parse_kind(expr: &str) -> MAstKind {
    let mut ast = parse_m_expression(expr).expect("expression should parse into an AST container");
    canonicalize_m_ast(&mut ast);
    ast.root_kind_for_testing()
}

fn assert_kind(expr: &str, expected: MAstKind) {
    let got = parse_kind(expr);
    assert_eq!(got, expected);
}

#[test]
fn coverage_audit_tier1_cases_are_structured() {
    assert_kind(
        "Source",
        MAstKind::Ident {
            name: "Source".to_string(),
        },
    );
    assert_kind(
        "#\"Previous Step\"",
        MAstKind::Ident {
            name: "Previous Step".to_string(),
        },
    );

    assert_kind("if true then 1 else 0", MAstKind::If);
    assert_kind("each _ + 1", MAstKind::Each);

    assert_kind(
        "Source[Field]",
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 1,
        },
    );
    assert_kind(
        "Source{0}",
        MAstKind::Access {
            kind: MAstAccessKind::Item,
            chain_len: 1,
        },
    );
    assert_kind(
        "Source{0}[Content]",
        MAstKind::Access {
            kind: MAstAccessKind::Field,
            chain_len: 2,
        },
    );
}

#[test]
fn coverage_audit_tier2_cases_are_structured() {
    assert_kind(
        "(x) => x",
        MAstKind::FunctionLiteral { param_count: 1 },
    );
    assert_kind("1 + 2", MAstKind::BinaryOp);
    assert_kind("not true", MAstKind::UnaryOp);
    assert_kind("x as number", MAstKind::TypeAscription);
    assert_kind("try 1 otherwise 0", MAstKind::TryOtherwise);
}
