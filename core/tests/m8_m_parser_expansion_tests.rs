use excel_diff::{MAstKind, ast_semantically_equal, canonicalize_m_ast, parse_m_expression};

#[test]
fn record_literal_parses_as_record() {
    let ast = parse_m_expression("[Field1 = 1, Field2 = 2]").unwrap();
    assert_eq!(
        ast.root_kind_for_testing(),
        MAstKind::Record { field_count: 2 }
    );
}

#[test]
fn list_literal_parses_as_list() {
    let ast = parse_m_expression("{1,2,3}").unwrap();
    assert_eq!(
        ast.root_kind_for_testing(),
        MAstKind::List { item_count: 3 }
    );
}

#[test]
fn function_call_parses_as_call() {
    let ast = parse_m_expression("Table.FromRows(.)").unwrap();
    assert_eq!(
        ast.root_kind_for_testing(),
        MAstKind::FunctionCall {
            name: "Table.FromRows".to_string(),
            arg_count: 1
        }
    );
}

#[test]
fn primitive_string_parses() {
    let ast = parse_m_expression(r#""hello""#).unwrap();
    assert_eq!(ast.root_kind_for_testing(), MAstKind::Primitive);
}

#[test]
fn primitive_number_parses() {
    let ast = parse_m_expression("42").unwrap();
    assert_eq!(ast.root_kind_for_testing(), MAstKind::Primitive);
}

#[test]
fn record_field_order_is_semantically_equivalent() {
    let mut a = parse_m_expression("[B=2, A=1]").unwrap();
    let mut b = parse_m_expression("[A=1, B=2]").unwrap();

    canonicalize_m_ast(&mut a);
    canonicalize_m_ast(&mut b);

    assert!(ast_semantically_equal(&a, &b));
}

#[test]
fn list_order_is_not_semantically_equivalent() {
    let mut a = parse_m_expression("{1,2}").unwrap();
    let mut b = parse_m_expression("{2,1}").unwrap();

    canonicalize_m_ast(&mut a);
    canonicalize_m_ast(&mut b);

    assert!(!ast_semantically_equal(&a, &b));
}
