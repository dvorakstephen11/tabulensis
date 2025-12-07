use excel_diff::{
    DataMashup, MParseError, ast_semantically_equal, build_data_mashup, build_queries,
    canonicalize_m_ast, open_data_mashup, parse_m_expression,
};

mod common;
use common::fixture_path;

fn load_datamashup(name: &str) -> DataMashup {
    let raw = open_data_mashup(fixture_path(name))
        .expect("fixture should open")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

fn load_single_query_expression(workbook: &str) -> String {
    let dm = load_datamashup(workbook);
    let queries = build_queries(&dm).expect("queries should parse");
    queries
        .first()
        .expect("fixture should contain a query")
        .expression_m
        .clone()
}

fn load_query_expression(workbook: &str, query_name: &str) -> String {
    let dm = load_datamashup(workbook);
    let queries = build_queries(&dm).expect("queries should parse");
    queries
        .into_iter()
        .find(|q| q.name == query_name)
        .expect("expected query to exist")
        .expression_m
}

#[test]
fn parse_basic_let_query_succeeds() {
    let expr = load_single_query_expression("one_query.xlsx");

    let result = parse_m_expression(&expr);

    assert!(result.is_ok(), "expected parse to succeed");
}

#[test]
fn formatting_only_queries_semantically_equal() {
    let expr_a = load_query_expression("m_formatting_only_a.xlsx", "Section1/FormatTest");
    let expr_b = load_query_expression("m_formatting_only_b.xlsx", "Section1/FormatTest");

    let mut ast_a = parse_m_expression(&expr_a).expect("formatting-only A should parse");
    let mut ast_b = parse_m_expression(&expr_b).expect("formatting-only B should parse");

    canonicalize_m_ast(&mut ast_a);
    canonicalize_m_ast(&mut ast_b);

    assert!(
        ast_semantically_equal(&ast_a, &ast_b),
        "formatting-only variants should be equal after canonicalization"
    );
}

#[test]
fn formatting_only_variant_detects_semantic_change() {
    let expr_b = load_query_expression("m_formatting_only_b.xlsx", "Section1/FormatTest");
    let expr_variant =
        load_query_expression("m_formatting_only_b_variant.xlsx", "Section1/FormatTest");

    let mut ast_b = parse_m_expression(&expr_b).expect("formatting-only B should parse");
    let mut ast_variant =
        parse_m_expression(&expr_variant).expect("formatting-only B variant should parse");

    canonicalize_m_ast(&mut ast_b);
    canonicalize_m_ast(&mut ast_variant);

    assert!(
        !ast_semantically_equal(&ast_b, &ast_variant),
        "semantic change should be detected even after canonicalization"
    );
}

#[test]
fn malformed_query_yields_parse_error() {
    let malformed = "let\n    Source = 1\n// missing 'in' and expression";

    let result = parse_m_expression(malformed);

    assert!(
        matches!(
            result,
            Err(MParseError::MissingInClause | MParseError::InvalidLetBinding)
        ),
        "missing 'in' should produce a parse error"
    );
}

#[test]
fn canonicalization_is_idempotent() {
    let expr = load_query_expression("m_formatting_only_b.xlsx", "Section1/FormatTest");

    let mut ast_once = parse_m_expression(&expr).expect("formatting-only B should parse");
    let mut ast_twice = ast_once.clone();

    canonicalize_m_ast(&mut ast_once);
    canonicalize_m_ast(&mut ast_twice);
    canonicalize_m_ast(&mut ast_twice);

    assert_eq!(
        ast_once, ast_twice,
        "canonicalization should produce a stable AST"
    );
}
