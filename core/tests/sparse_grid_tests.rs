use excel_diff::{Cell, CellAddress, CellValue, Grid};

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
    let cell = Cell {
        row: 50,
        col: 50,
        address: CellAddress::from_indices(50, 50),
        value: Some(CellValue::Number(42.0)),
        formula: None,
    };
    grid.insert(cell);
    assert_eq!(grid.cell_count(), 1);
    let retrieved = grid.get(50, 50).expect("cell should exist");
    assert_eq!(retrieved.value, Some(CellValue::Number(42.0)));
    assert!(grid.get(0, 0).is_none());
}

#[test]
fn sparse_grid_iter_cells_only_populated() {
    let mut grid = Grid::new(1000, 1000);
    for i in 0..10 {
        let cell = Cell {
            row: i * 100,
            col: i * 100,
            address: CellAddress::from_indices(i * 100, i * 100),
            value: Some(CellValue::Number(i as f64)),
            formula: None,
        };
        grid.insert(cell);
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
    grid.insert(Cell {
        row: 1,
        col: 1,
        address: CellAddress::from_indices(1, 1),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    for r in grid.rows_iter() {
        for c in grid.cols_iter() {
            let _ = grid.get(r, c);
        }
    }
}

#[test]
fn compute_signatures_on_sparse_grid_produces_hashes() {
    let mut grid = Grid::new(4, 4);
    grid.insert(Cell {
        row: 1,
        col: 3,
        address: CellAddress::from_indices(1, 3),
        value: Some(CellValue::Text("value".into())),
        formula: Some("=A1".into()),
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
