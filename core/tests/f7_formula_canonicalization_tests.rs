use excel_diff::parse_formula;

#[test]
fn canonicalizes_commutative_binary_ops() {
    let a = parse_formula("A1+B1").unwrap().canonicalize();
    let b = parse_formula("B1+A1").unwrap().canonicalize();
    assert_eq!(a, b);

    let a = parse_formula("A1*B1").unwrap().canonicalize();
    let b = parse_formula("B1*A1").unwrap().canonicalize();
    assert_eq!(a, b);
}

#[test]
fn canonicalizes_commutative_functions_by_sorting_args() {
    let a = parse_formula("SUM(A1,B1)").unwrap().canonicalize();
    let b = parse_formula("SUM(B1,A1)").unwrap().canonicalize();
    assert_eq!(a, b);

    let a = parse_formula("AND(TRUE,FALSE)").unwrap().canonicalize();
    let b = parse_formula("AND(FALSE,TRUE)").unwrap().canonicalize();
    assert_eq!(a, b);
}

#[test]
fn does_not_canonicalize_non_commutative_ops() {
    let a = parse_formula("A1-B1").unwrap().canonicalize();
    let b = parse_formula("B1-A1").unwrap().canonicalize();
    assert_ne!(a, b);
}

#[test]
fn canonicalizes_range_endpoints() {
    let a = parse_formula("B2:A1").unwrap().canonicalize();
    let b = parse_formula("A1:B2").unwrap().canonicalize();
    assert_eq!(a, b);
}

#[test]
fn structured_refs_parse_and_canonicalize() {
    let a = parse_formula("Table1[Column1]").unwrap().canonicalize();
    let b = parse_formula("TABLE1[COLUMN1]").unwrap().canonicalize();
    assert_eq!(a, b);
}

