use excel_diff::{Cell, CellAddress, CellValue, Grid};

#[test]
fn identical_rows_have_same_signature() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        let cell = Cell {
            row: 0,
            col: c,
            address: CellAddress::from_indices(0, c),
            value: Some(CellValue::Number(c as f64)),
            formula: None,
        };
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
        grid1.insert(Cell {
            row: 0,
            col: c,
            address: CellAddress::from_indices(0, c),
            value: Some(CellValue::Number(c as f64)),
            formula: None,
        });
        grid2.insert(Cell {
            row: 0,
            col: c,
            address: CellAddress::from_indices(0, c),
            value: Some(CellValue::Number((c + 1) as f64)),
            formula: None,
        });
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1, sig2);
}

#[test]
fn compute_all_signatures_populates_fields() {
    let mut grid = Grid::new(5, 5);
    grid.insert(Cell {
        row: 2,
        col: 2,
        address: CellAddress::from_indices(2, 2),
        value: Some(CellValue::Text("center".into())),
        formula: None,
    });
    assert!(grid.row_signatures.is_none());
    assert!(grid.col_signatures.is_none());
    grid.compute_all_signatures();
    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert_eq!(grid.row_signatures.as_ref().unwrap().len(), 5);
    assert_eq!(grid.col_signatures.as_ref().unwrap().len(), 5);
}
