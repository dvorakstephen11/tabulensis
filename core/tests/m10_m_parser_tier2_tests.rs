use excel_diff::{MAstKind, ast_semantically_equal, canonicalize_m_ast, parse_m_expression};

fn kind(expr: &str) -> MAstKind {
    let mut ast = parse_m_expression(expr).expect("parse should succeed");
    canonicalize_m_ast(&mut ast);
    ast.root_kind_for_testing()
}

fn canon(expr: &str) -> excel_diff::MModuleAst {
    let mut ast = parse_m_expression(expr).expect("parse should succeed");
    canonicalize_m_ast(&mut ast);
    ast
}

#[test]
fn parse_function_literals() {
    assert_eq!(
        kind("(x) => x"),
        MAstKind::FunctionLiteral { param_count: 1 }
    );
    assert_eq!(
        kind("(x, y) => x + y"),
        MAstKind::FunctionLiteral { param_count: 2 }
    );
}

#[test]
fn parse_unary_ops() {
    assert_eq!(kind("not true"), MAstKind::UnaryOp);
    assert_eq!(kind("-1"), MAstKind::Primitive);
    assert_eq!(kind("-(1)"), MAstKind::UnaryOp);
}

#[test]
fn precedence_mul_binds_tighter_than_add() {
    let a = canon("1 + 2 * 3");
    let b = canon("1 + (2 * 3)");
    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn precedence_and_binds_tighter_than_or() {
    let a = canon("a or b and c");
    let b = canon("a or (b and c)");
    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn formatting_only_function_literal_equal() {
    let a = canon("(x)=>x");
    let b = canon("( x ) => x");
    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn formatting_only_type_ascription_equal() {
    let a = canon("x as Number");
    let b = canon("x as number");
    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn formatting_only_precedence_equal() {
    let a = canon("1+(2*3)");
    let b = canon("1 + 2 * 3");
    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn parse_try_otherwise() {
    assert_eq!(kind("try 1 otherwise 0"), MAstKind::TryOtherwise);
    let a = canon("try 1 otherwise 0");
    let b = canon("try (1) otherwise (0)");
    assert!(ast_semantically_equal(&a, &b));
}
