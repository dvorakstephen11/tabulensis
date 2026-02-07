use excel_diff::{
    formulas_equivalent_modulo_shift, parse_formula, DiffConfig, DiffOp, FormulaDiffResult, Sheet,
    SheetKind, StringPool, Workbook,
};

#[test]
fn coverage_audit_canonicalize_commutative_ops_and_functions() {
    let a = parse_formula("=A1+B1")
        .expect("formula should parse")
        .canonicalize();
    let b = parse_formula("=b1 + a1")
        .expect("formula should parse")
        .canonicalize();
    assert_eq!(
        a, b,
        "commutative binary ops should canonicalize consistently"
    );

    let s1 = parse_formula("=sum(1,2,3)")
        .expect("formula should parse")
        .canonicalize();
    let s2 = parse_formula("=SUM(3, 1, 2)")
        .expect("formula should parse")
        .canonicalize();
    assert_eq!(
        s1, s2,
        "commutative functions should canonicalize arg order"
    );
}

#[test]
fn coverage_audit_range_endpoints_are_normalized() {
    let a = parse_formula("=B2:A1")
        .expect("formula should parse")
        .canonicalize();
    let b = parse_formula("=A1:B2")
        .expect("formula should parse")
        .canonicalize();
    assert_eq!(
        a, b,
        "range endpoints should be canonicalized into a stable order"
    );
}

#[test]
fn coverage_audit_fill_equivalence_is_detectable() {
    let a = parse_formula("=A1+B1").expect("formula should parse");
    let b = parse_formula("=A2+B2").expect("formula should parse");

    assert!(
        formulas_equivalent_modulo_shift(&a, &b, 1, 0),
        "expected shift-equivalence for a filled-down formula"
    );
    assert!(
        !formulas_equivalent_modulo_shift(&a, &b, 0, 0),
        "expected non-equivalence without a shift"
    );
}

#[test]
fn coverage_audit_semantic_formula_diff_classifies_formatting_only() {
    let mut pool = StringPool::new();
    let sheet_name = pool.intern("Sheet1");

    let mut old_grid = excel_diff::Grid::new(1, 1);
    let mut new_grid = excel_diff::Grid::new(1, 1);

    let old_formula = pool.intern("=sum(A1, 1)");
    let new_formula = pool.intern("=SUM( A1 ,1 )");

    old_grid.insert_cell(0, 0, None, Some(old_formula));
    new_grid.insert_cell(0, 0, None, Some(new_formula));

    let old = Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid: old_grid,
        }],
        ..Default::default()
    };
    let new = Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid: new_grid,
        }],
        ..Default::default()
    };

    let cfg = DiffConfig::builder()
        .enable_formula_semantic_diff(true)
        .build()
        .expect("valid config should build");

    let report = excel_diff::advanced::diff_workbooks_with_pool(&old, &new, &mut pool, &cfg);
    let edits: Vec<FormulaDiffResult> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { formula_diff, .. } => Some(*formula_diff),
            _ => None,
        })
        .collect();

    assert_eq!(edits.len(), 1, "expected exactly one CellEdited op");
    assert_eq!(edits[0], FormulaDiffResult::FormattingOnly);
}
