use excel_diff::{Cell, CellAddress, CellValue, Grid};

fn make_cell(row: u32, col: u32, value: Option<CellValue>, formula: Option<&str>) -> Cell {
    Cell {
        row,
        col,
        address: CellAddress::from_indices(row, col),
        value,
        formula: formula.map(|s| s.to_string()),
    }
}

#[test]
fn identical_rows_have_same_signature() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        let cell = make_cell(0, c, Some(CellValue::Number(c as f64)), None);
        grid1.insert(cell.clone());
        grid2.insert(cell);
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_eq!(sig1, sig2);
}

#[test]
fn different_rows_have_different_signatures() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        grid1.insert(make_cell(0, c, Some(CellValue::Number(c as f64)), None));
        grid2.insert(make_cell(
            0,
            c,
            Some(CellValue::Number((c + 1) as f64)),
            None,
        ));
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1, sig2);
}

#[test]
fn compute_all_signatures_populates_fields() {
    let mut grid = Grid::new(5, 5);
    grid.insert(make_cell(
        2,
        2,
        Some(CellValue::Text("center".into())),
        None,
    ));
    assert!(grid.row_signatures.is_none());
    assert!(grid.col_signatures.is_none());
    grid.compute_all_signatures();
    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert_eq!(grid.row_signatures.as_ref().unwrap().len(), 5);
    assert_eq!(grid.col_signatures.as_ref().unwrap().len(), 5);
    assert_ne!(grid.row_signatures.as_ref().unwrap()[2].hash, 0);
    assert_ne!(grid.col_signatures.as_ref().unwrap()[2].hash, 0);
}

#[test]
fn row_and_col_signatures_match_bulk_computation() {
    let mut grid = Grid::new(3, 2);
    grid.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert(make_cell(1, 1, Some(CellValue::Text("text".into())), None));
    grid.insert(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    for r in 0..3 {
        assert_eq!(
            grid.compute_row_signature(r).hash,
            row_sigs[r as usize].hash
        );
    }

    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");
    for c in 0..2 {
        assert_eq!(
            grid.compute_col_signature(c).hash,
            col_sigs[c as usize].hash
        );
    }
}

#[test]
fn row_signatures_distinguish_column_positions() {
    let mut grid1 = Grid::new(1, 2);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert(make_cell(0, 1, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(1, 2);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert(make_cell(0, 1, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn col_signatures_distinguish_row_positions() {
    let mut grid1 = Grid::new(2, 1);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert(make_cell(1, 0, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(2, 1);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert(make_cell(1, 0, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_col_signature(0);
    let sig2 = grid2.compute_col_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn col_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(3, 1);
    grid_num.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(3, 1);
    grid_text.insert(make_cell(0, 0, Some(CellValue::Text("1".into())), None));

    let mut grid_bool = Grid::new(3, 1);
    grid_bool.insert(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_col_signature(0).hash;
    let txt = grid_text.compute_col_signature(0).hash;
    let boo = grid_bool.compute_col_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(1, 1);
    grid_num.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(1, 1);
    grid_text.insert(make_cell(0, 0, Some(CellValue::Text("1".into())), None));

    let mut grid_bool = Grid::new(1, 1);
    grid_bool.insert(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_row_signature(0).hash;
    let txt = grid_text.compute_row_signature(0).hash;
    let boo = grid_bool.compute_row_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_ignores_empty_trailing_cells() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(1, 10);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_ignores_empty_trailing_rows() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(10, 1);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_includes_formulas_by_default() {
    let mut with_formula = Grid::new(2, 1);
    with_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut without_formula = Grid::new(2, 1);
    without_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = with_formula.compute_col_signature(0).hash;
    let sig_without = without_formula.compute_col_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

#[test]
fn col_signature_includes_formulas_sparse() {
    let mut formula_short = Grid::new(5, 1);
    formula_short.insert(make_cell(
        0,
        0,
        Some(CellValue::Text("foo".into())),
        Some("=A2"),
    ));

    let mut formula_tall = Grid::new(10, 1);
    formula_tall.insert(make_cell(
        0,
        0,
        Some(CellValue::Text("foo".into())),
        Some("=A2"),
    ));

    let mut value_only = Grid::new(10, 1);
    value_only.insert(make_cell(0, 0, Some(CellValue::Text("foo".into())), None));

    let sig_formula_short = formula_short.compute_col_signature(0).hash;
    let sig_formula_tall = formula_tall.compute_col_signature(0).hash;
    let sig_value_only = value_only.compute_col_signature(0).hash;

    assert_eq!(sig_formula_short, sig_formula_tall);
    assert_ne!(sig_formula_short, sig_value_only);
}

#[test]
fn row_signature_includes_formulas_by_default() {
    let mut grid_with_formula = Grid::new(1, 1);
    grid_with_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut grid_without_formula = Grid::new(1, 1);
    grid_without_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = grid_with_formula.compute_row_signature(0).hash;
    let sig_without = grid_without_formula.compute_row_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

const ROW_SIGNATURE_GOLDEN: u64 = 13_315_384_008_147_106_509;
const ROW_SIGNATURE_WITH_FORMULA_GOLDEN: u64 = 3_920_348_561_402_334_617;

#[test]
fn row_signature_golden_constant_small_grid() {
    let mut grid = Grid::new(1, 3);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert(make_cell(0, 1, Some(CellValue::Text("x".into())), None));
    grid.insert(make_cell(0, 2, Some(CellValue::Bool(false)), None));

    let sig = grid.compute_row_signature(0);
    assert_eq!(sig.hash, ROW_SIGNATURE_GOLDEN);
}

#[test]
fn row_signature_golden_constant_with_formula() {
    let mut grid = Grid::new(1, 2);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));
    grid.insert(make_cell(0, 1, Some(CellValue::Text("bar".into())), None));

    let sig = grid.compute_row_signature(0);
    assert_eq!(sig.hash, ROW_SIGNATURE_WITH_FORMULA_GOLDEN);
}
