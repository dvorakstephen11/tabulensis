use excel_diff::{CellValue, Grid, with_default_session};

#[test]
fn sparse_grid_empty_has_zero_cells() {
    let grid = Grid::new(1000, 1000);
    assert_eq!(grid.cell_count(), 0);
    assert!(grid.is_empty());
    assert_eq!(grid.nrows, 1000);
    assert_eq!(grid.ncols, 1000);
}

#[test]
fn sparse_grid_insert_and_retrieve() {
    let mut grid = Grid::new(100, 100);
    grid.insert_cell(50, 50, Some(CellValue::Number(42.0)), None);
    assert_eq!(grid.cell_count(), 1);
    let retrieved = grid.get(50, 50).expect("cell should exist");
    assert_eq!(retrieved.value, Some(CellValue::Number(42.0)));
    assert!(grid.get(0, 0).is_none());
}

#[test]
fn sparse_grid_iter_cells_only_populated() {
    let mut grid = Grid::new(1000, 1000);
    for i in 0..10 {
        grid.insert_cell(i * 100, i * 100, Some(CellValue::Number(i as f64)), None);
    }
    let cells: Vec<_> = grid.iter_cells().collect();
    assert_eq!(cells.len(), 10);
}

#[test]
fn sparse_grid_memory_efficiency() {
    let grid = Grid::new(10_000, 1_000);
    assert!(std::mem::size_of_val(&grid) < 1024);
}

#[test]
fn rows_iter_covers_all_rows() {
    let grid = Grid::new(3, 5);
    let rows: Vec<u32> = grid.rows_iter().collect();
    assert_eq!(rows, vec![0, 1, 2]);
}

#[test]
fn cols_iter_covers_all_cols() {
    let grid = Grid::new(4, 2);
    let cols: Vec<u32> = grid.cols_iter().collect();
    assert_eq!(cols, vec![0, 1]);
}

#[test]
fn rows_iter_and_get_are_consistent() {
    let mut grid = Grid::new(2, 2);
    grid.insert_cell(1, 1, Some(CellValue::Number(1.0)), None);

    for r in grid.rows_iter() {
        for c in grid.cols_iter() {
            let _ = grid.get(r, c);
        }
    }
}

#[test]
fn sparse_grid_all_empty_rows_have_zero_signatures() {
    let mut grid = Grid::new(2, 3);

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(row_sigs.len(), 2);
    assert_eq!(col_sigs.len(), 3);
    let empty_row_hash = row_sigs[0].hash;
    let empty_col_hash = col_sigs[0].hash;
    assert!(row_sigs.iter().all(|sig| sig.hash == empty_row_hash));
    assert!(col_sigs.iter().all(|sig| sig.hash == empty_col_hash));
}

#[test]
fn compute_signatures_on_sparse_grid_produces_hashes() {
    let mut grid = Grid::new(4, 4);
    with_default_session(|session| {
        let text_id = session.strings.intern("value");
        let formula_id = session.strings.intern("=A1");
        grid.insert_cell(1, 3, Some(CellValue::Text(text_id)), Some(formula_id));
    });

    grid.compute_all_signatures();

    let row_hash = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist")[1]
        .hash;
    let col_hash = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist")[3]
        .hash;

    assert_ne!(row_hash, 0);
    assert_ne!(col_hash, 0);
}

#[test]
fn compute_all_signatures_matches_direct_computation() {
    let mut grid = Grid::new(3, 3);
    with_default_session(|session| {
        let formula_a = session.strings.intern("=5+5");
        let text_id = session.strings.intern("x");
        let formula_b = session.strings.intern("=A1");
        grid.insert_cell(0, 1, Some(CellValue::Number(10.0)), Some(formula_a));
        grid.insert_cell(1, 0, Some(CellValue::Text(text_id)), None);
        grid.insert_cell(2, 2, Some(CellValue::Bool(false)), Some(formula_b));
    });

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(grid.compute_row_signature(0).hash, row_sigs[0].hash);
    assert_eq!(grid.compute_row_signature(2).hash, row_sigs[2].hash);
    assert_eq!(grid.compute_col_signature(0).hash, col_sigs[0].hash);
    assert_eq!(grid.compute_col_signature(2).hash, col_sigs[2].hash);
}
