use excel_diff::{formulas_equivalent_modulo_shift, parse_formula};

#[test]
fn filled_down_formulas_match_under_row_shift() {
    let old = parse_formula("A1+B1").expect("old formula parses");
    let new = parse_formula("A2+B2").expect("new formula parses");

    assert!(
        formulas_equivalent_modulo_shift(&old, &new, 1, 0),
        "expected formulas to match after row shift",
    );
    assert!(
        !formulas_equivalent_modulo_shift(&old, &new, 0, 0),
        "without shift they should differ",
    );
}

#[test]
fn mismatched_refs_do_not_match_under_zero_shift() {
    let old = parse_formula("A1+B1").expect("old formula parses");
    let new = parse_formula("A1+B2").expect("new formula parses");

    assert!(
        !formulas_equivalent_modulo_shift(&old, &new, 0, 0),
        "different refs should not be equivalent without a shift",
    );
}
