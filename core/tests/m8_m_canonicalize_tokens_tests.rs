use excel_diff::{ast_semantically_equal, canonicalize_m_ast, parse_m_expression};

#[test]
fn opaque_boolean_literal_case_is_canonicalized() {
    let a = "if TRUE then 1 else 0";
    let b = "if true then 1 else 0";

    let mut ast_a = parse_m_expression(a).expect("a should parse");
    let mut ast_b = parse_m_expression(b).expect("b should parse");

    canonicalize_m_ast(&mut ast_a);
    canonicalize_m_ast(&mut ast_b);

    assert!(ast_semantically_equal(&ast_a, &ast_b));
}

#[test]
fn opaque_null_literal_case_is_canonicalized() {
    let a = "if NULL then 1 else 0";
    let b = "if null then 1 else 0";

    let mut ast_a = parse_m_expression(a).expect("a should parse");
    let mut ast_b = parse_m_expression(b).expect("b should parse");

    canonicalize_m_ast(&mut ast_a);
    canonicalize_m_ast(&mut ast_b);

    assert!(ast_semantically_equal(&ast_a, &ast_b));
}
