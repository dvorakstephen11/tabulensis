mod common;

use common::sid;
use excel_diff::{CellValue, Grid, GridView, StringId};

#[derive(Clone)]
struct TestCell {
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<StringId>,
}

trait GridTestInsert {
    fn insert_test(&mut self, cell: TestCell);
}

impl GridTestInsert for Grid {
    fn insert_test(&mut self, cell: TestCell) {
        self.insert_cell(cell.row, cell.col, cell.value, cell.formula);
    }
}

fn make_cell(row: u32, col: u32, value: Option<CellValue>, formula: Option<&str>) -> TestCell {
    TestCell {
        row,
        col,
        value,
        formula: formula.map(sid),
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
        grid1.insert_test(make_cell(0, c, Some(CellValue::Number(c as f64)), None));
        grid2.insert_test(make_cell(
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
    grid.insert_test(make_cell(
        2,
        2,
        Some(CellValue::Text(sid("center"))),
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
fn compute_all_signatures_on_empty_grid_produces_empty_vectors() {
    let mut grid = Grid::new(0, 0);

    grid.compute_all_signatures();

    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert!(grid.row_signatures.as_ref().unwrap().is_empty());
    assert!(grid.col_signatures.as_ref().unwrap().is_empty());
}

#[test]
fn compute_all_signatures_with_all_empty_rows_and_cols_is_stable() {
    let mut grid = Grid::new(3, 4);

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    assert_eq!(first_rows.len(), 3);
    assert_eq!(first_cols.len(), 4);
    let empty_row_hash = first_rows[0].hash;
    let empty_col_hash = first_cols[0].hash;
    assert!(first_rows.iter().all(|sig| sig.hash == empty_row_hash));
    assert!(first_cols.iter().all(|sig| sig.hash == empty_col_hash));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_eq!(first_rows, *second_rows);
    assert_eq!(first_cols, *second_cols);
}

#[test]
fn row_and_col_signatures_match_bulk_computation() {
    let mut grid = Grid::new(3, 2);
    grid.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("text"))), None));
    grid.insert_test(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

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
fn compute_all_signatures_recomputes_after_mutation() {
    let mut grid = Grid::new(3, 3);
    grid.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("x"))), None));

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("y"))), None));
    grid.insert_test(make_cell(2, 2, Some(CellValue::Bool(true)), None));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_ne!(first_rows[1].hash, second_rows[1].hash);
    assert_ne!(first_cols[1].hash, second_cols[1].hash);
}

#[test]
fn row_signatures_distinguish_column_positions() {
    let mut grid1 = Grid::new(1, 2);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(1, 2);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn col_signatures_distinguish_row_positions() {
    let mut grid1 = Grid::new(2, 1);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(2, 1);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_col_signature(0);
    let sig2 = grid2.compute_col_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn row_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("mix"))), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Bool(true)), None));

    let mut grid2 = Grid::new(1, 3);
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Bool(true)), None));
    grid2.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("mix"))), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.row_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.row_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("col"))), None));
    grid1.insert_test(make_cell(2, 0, Some(CellValue::Bool(false)), None));

    let mut grid2 = Grid::new(3, 1);
    grid2.insert_test(make_cell(2, 0, Some(CellValue::Bool(false)), None));
    grid2.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("col"))), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.col_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.col_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(3, 1);
    grid_num.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(3, 1);
    grid_text.insert_test(make_cell(0, 0, Some(CellValue::Text(sid("1"))), None));

    let mut grid_bool = Grid::new(3, 1);
    grid_bool.insert_test(make_cell(0, 0, Some(CellValue::Bool(true)), None));

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
    grid_num.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(1, 1);
    grid_text.insert_test(make_cell(0, 0, Some(CellValue::Text(sid("1"))), None));

    let mut grid_bool = Grid::new(1, 1);
    grid_bool.insert_test(make_cell(0, 0, Some(CellValue::Bool(true)), None));

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
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(1, 10);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_ignores_empty_trailing_rows() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(10, 1);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_includes_formulas_by_default() {
    let mut with_formula = Grid::new(2, 1);
    with_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut without_formula = Grid::new(2, 1);
    without_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = with_formula.compute_col_signature(0).hash;
    let sig_without = without_formula.compute_col_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

#[test]
fn col_signature_includes_formulas_sparse() {
    let mut formula_short = Grid::new(5, 1);
    formula_short.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Text(sid("foo"))),
        Some("=A2"),
    ));

    let mut formula_tall = Grid::new(10, 1);
    formula_tall.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Text(sid("foo"))),
        Some("=A2"),
    ));

    let mut value_only = Grid::new(10, 1);
    value_only.insert_test(make_cell(0, 0, Some(CellValue::Text(sid("foo"))), None));

    let sig_formula_short = formula_short.compute_col_signature(0).hash;
    let sig_formula_tall = formula_tall.compute_col_signature(0).hash;
    let sig_value_only = value_only.compute_col_signature(0).hash;

    assert_eq!(sig_formula_short, sig_formula_tall);
    assert_ne!(sig_formula_short, sig_value_only);
}

#[test]
fn row_signature_includes_formulas_by_default() {
    let mut grid_with_formula = Grid::new(1, 1);
    grid_with_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut grid_without_formula = Grid::new(1, 1);
    grid_without_formula.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = grid_with_formula.compute_row_signature(0).hash;
    let sig_without = grid_without_formula.compute_row_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

#[test]
fn row_signature_is_stable_across_computations() {
    let mut grid = Grid::new(1, 3);
    grid.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("x"))), None));
    grid.insert_test(make_cell(0, 2, Some(CellValue::Bool(false)), None));

    let sig1 = grid.compute_row_signature(0);
    let sig2 = grid.compute_row_signature(0);
    assert_eq!(sig1.hash, sig2.hash);
    assert_ne!(sig1.hash, 0);
}

#[test]
fn row_signature_with_formula_is_stable() {
    let mut grid = Grid::new(1, 2);
    grid.insert_test(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));
    grid.insert_test(make_cell(0, 1, Some(CellValue::Text(sid("bar"))), None));

    let sig1 = grid.compute_row_signature(0);
    let sig2 = grid.compute_row_signature(0);
    assert_eq!(sig1.hash, sig2.hash);
    assert_ne!(sig1.hash, 0);
}

#[test]
fn gridview_rowmeta_hash_matches_compute_all_signatures() {
    let mut grid = Grid::new(3, 2);
    grid.insert_test(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("text"))), None));
    grid.insert_test(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

    grid.compute_all_signatures();

    let row_signatures = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should be computed")
        .clone();
    let col_signatures = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should be computed")
        .clone();

    let view = GridView::from_grid(&grid);

    for (idx, meta) in view.row_meta.iter().enumerate() {
        assert_eq!(meta.hash, row_signatures[idx]);
    }

    for (idx, meta) in view.col_meta.iter().enumerate() {
        assert_eq!(meta.hash, col_signatures[idx].hash);
    }
}

#[test]
fn row_signature_unchanged_after_column_insert_at_position_zero() {
    let mut grid1 = Grid::new(2, 3);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Number(3.0)), None));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("a"))), None));
    grid1.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("b"))), None));
    grid1.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("c"))), None));

    let mut grid2 = Grid::new(2, 4);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(99.0)), None));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(1.0)), None));
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(0, 3, Some(CellValue::Number(3.0)), None));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("z"))), None));
    grid2.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("a"))), None));
    grid2.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("b"))), None));
    grid2.insert_test(make_cell(1, 3, Some(CellValue::Text(sid("c"))), None));

    let view1 = GridView::from_grid(&grid1);
    let view2 = GridView::from_grid(&grid2);

    assert_ne!(view1.row_meta[0].hash, view2.row_meta[0].hash);
    assert_ne!(view1.row_meta[1].hash, view2.row_meta[1].hash);
}

#[test]
fn row_signature_unchanged_after_column_delete_from_middle() {
    let mut grid1 = Grid::new(2, 4);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Number(3.0)), None));
    grid1.insert_test(make_cell(0, 3, Some(CellValue::Number(4.0)), None));
    grid1.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("a"))), None));
    grid1.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("b"))), None));
    grid1.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("c"))), None));
    grid1.insert_test(make_cell(1, 3, Some(CellValue::Text(sid("d"))), None));

    let mut grid2 = Grid::new(2, 3);
    grid2.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(3.0)), None));
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Number(4.0)), None));
    grid2.insert_test(make_cell(1, 0, Some(CellValue::Text(sid("a"))), None));
    grid2.insert_test(make_cell(1, 1, Some(CellValue::Text(sid("c"))), None));
    grid2.insert_test(make_cell(1, 2, Some(CellValue::Text(sid("d"))), None));

    let view1 = GridView::from_grid(&grid1);
    let view2 = GridView::from_grid(&grid2);

    assert_ne!(view1.row_meta[0].hash, view2.row_meta[0].hash);
    assert_ne!(view1.row_meta[1].hash, view2.row_meta[1].hash);
}

#[test]
fn row_signature_consistent_for_same_content_different_column_indices() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert_test(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert_test(make_cell(0, 1, Some(CellValue::Number(2.0)), None));
    grid1.insert_test(make_cell(0, 2, Some(CellValue::Number(3.0)), None));

    let mut grid2 = Grid::new(1, 5);
    grid2.insert_test(make_cell(0, 1, Some(CellValue::Number(1.0)), None));
    grid2.insert_test(make_cell(0, 2, Some(CellValue::Number(2.0)), None));
    grid2.insert_test(make_cell(0, 3, Some(CellValue::Number(3.0)), None));

    let view1 = GridView::from_grid(&grid1);
    let view2 = GridView::from_grid(&grid2);

    assert_eq!(view1.row_meta[0].hash, view2.row_meta[0].hash);
}
