use excel_diff::{
    BinaryOperator, CellReference, ColRef, ExcelError, FormulaExpr, RangeReference, RowRef,
    UnaryOperator, parse_formula,
};

fn cell(sheet: Option<&str>, row: RowRef, col: ColRef) -> FormulaExpr {
    FormulaExpr::CellRef(CellReference {
        sheet: sheet.map(|s| s.to_string()),
        row,
        col,
        spill: false,
    })
}

#[test]
fn parses_expected_ast_shapes() {
    let cases = vec![
        ("1", FormulaExpr::Number(1.0)),
        ("\"x\"", FormulaExpr::Text("x".to_string())),
        ("TRUE", FormulaExpr::Boolean(true)),
        ("#DIV/0!", FormulaExpr::Error(ExcelError::Div0)),
        ("A1", cell(None, RowRef::Relative(1), ColRef::Relative(1))),
        ("$B$2", cell(None, RowRef::Absolute(2), ColRef::Absolute(2))),
        (
            "R[1]C[-1]",
            cell(None, RowRef::Offset(1), ColRef::Offset(-1)),
        ),
        (
            "SUM(A1,B1)",
            FormulaExpr::FunctionCall {
                name: "SUM".into(),
                args: vec![
                    cell(None, RowRef::Relative(1), ColRef::Relative(1)),
                    cell(None, RowRef::Relative(1), ColRef::Relative(2)),
                ],
            },
        ),
        (
            "{1,2;3,4}",
            FormulaExpr::Array(vec![
                vec![FormulaExpr::Number(1.0), FormulaExpr::Number(2.0)],
                vec![FormulaExpr::Number(3.0), FormulaExpr::Number(4.0)],
            ]),
        ),
        (
            "A1:B2",
            FormulaExpr::RangeRef(RangeReference {
                sheet: None,
                start: CellReference {
                    sheet: None,
                    row: RowRef::Relative(1),
                    col: ColRef::Relative(1),
                    spill: false,
                },
                end: CellReference {
                    sheet: None,
                    row: RowRef::Relative(2),
                    col: ColRef::Relative(2),
                    spill: false,
                },
            }),
        ),
        (
            "A1^-1",
            FormulaExpr::BinaryOp {
                op: BinaryOperator::Pow,
                left: Box::new(cell(None, RowRef::Relative(1), ColRef::Relative(1))),
                right: Box::new(FormulaExpr::UnaryOp {
                    op: UnaryOperator::Minus,
                    operand: Box::new(FormulaExpr::Number(1.0)),
                }),
            },
        ),
    ];

    for (text, expected) in cases {
        let parsed = parse_formula(text).expect("formula should parse");
        assert_eq!(parsed, expected, "mismatched AST for '{text}'");
    }
}

#[test]
fn parses_varied_syntaxes() {
    let samples = [
        "=sum( A1 , B1 )",
        "'My Sheet'!$A$1",
        "[Book1.xlsx]Sheet1!A1",
        "{1,2;3,4}",
        "Table1[Column1]",
        "R1C1",
        "R[2]C[-3]",
    ];

    for text in samples {
        parse_formula(text).unwrap_or_else(|e| panic!("failed to parse {text}: {e}"));
    }
}
