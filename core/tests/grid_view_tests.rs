use excel_diff::{CellValue, DiffConfig, Grid, GridView, with_default_session};

mod common;
use common::grid_from_numbers;

fn insert_cell(
    grid: &mut Grid,
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<&str>,
) {
    let formula_id = formula.map(|s| with_default_session(|session| session.strings.intern(s)));
    grid.insert_cell(row, col, value, formula_id);
}

fn text(value: &str) -> CellValue {
    with_default_session(|session| CellValue::Text(session.strings.intern(value)))
}

#[test]
fn gridview_dense_3x3_layout_and_metadata() {
    let grid = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);

    let view = GridView::from_grid(&grid);

    assert_eq!(view.rows.len(), 3);
    assert_eq!(view.row_meta.len(), 3);
    assert_eq!(view.col_meta.len(), 3);

    for (row_idx, row_view) in view.rows.iter().enumerate() {
        assert_eq!(row_view.cells.len(), 3);
        for (col_idx, (col, _cell)) in row_view.cells.iter().enumerate() {
            assert_eq!(*col as usize, col_idx);
        }

        let meta = &view.row_meta[row_idx];
        assert_eq!(meta.non_blank_count, 3);
        assert_eq!(meta.first_non_blank_col, 0);
        assert!(!meta.is_low_info);
    }

    for (col_idx, col_meta) in view.col_meta.iter().enumerate() {
        assert_eq!(col_meta.non_blank_count, 3);
        assert_eq!(col_meta.first_non_blank_row, 0);
        assert_eq!(col_meta.col_idx as usize, col_idx);
    }
}

#[test]
fn gridview_sparse_rows_low_info_classification() {
    let mut grid = Grid::new(4, 4);
    insert_cell(&mut grid, 0, 0, Some(text("Header")), None);
    insert_cell(&mut grid, 2, 2, Some(CellValue::Number(10.0)), None);
    insert_cell(&mut grid, 3, 1, Some(text("   ")), None);

    let view = GridView::from_grid(&grid);

    assert_eq!(view.row_meta[0].non_blank_count, 1);
    assert!(view.row_meta[0].is_low_info);
    assert_eq!(view.row_meta[0].first_non_blank_col, 0);

    assert_eq!(view.row_meta[1].non_blank_count, 0);
    assert!(view.row_meta[1].is_low_info);
    assert_eq!(view.row_meta[1].first_non_blank_col, 0);

    assert_eq!(view.row_meta[2].non_blank_count, 1);
    assert!(view.row_meta[2].is_low_info);
    assert_eq!(view.row_meta[2].first_non_blank_col, 2);

    assert_eq!(view.row_meta[3].non_blank_count, 1);
    assert!(view.row_meta[3].is_low_info);
    assert_eq!(view.row_meta[3].first_non_blank_col, 1);
}

#[allow(clippy::field_reassign_with_default)]
#[test]
fn gridview_formula_only_row_respects_threshold() {
    let mut grid = Grid::new(2, 2);
    insert_cell(&mut grid, 0, 0, None, Some("=A1+1"));

    let view_default = GridView::from_grid(&grid);
    assert_eq!(view_default.row_meta[0].non_blank_count, 1);
    assert!(view_default.row_meta[0].is_low_info);

    let mut config = DiffConfig::default();
    config.low_info_threshold = 1;
    let view_tuned = GridView::from_grid_with_config(&grid, &config);
    assert_eq!(view_tuned.row_meta[0].non_blank_count, 1);
    assert!(!view_tuned.row_meta[0].is_low_info);
}

#[test]
fn gridview_column_metadata_matches_signatures() {
    let mut grid = Grid::new(4, 4);
    insert_cell(&mut grid, 0, 1, Some(text("a")), Some("=B1"));
    insert_cell(&mut grid, 1, 3, Some(CellValue::Number(2.0)), Some("=1+1"));
    insert_cell(&mut grid, 2, 0, Some(CellValue::Bool(true)), None);
    insert_cell(&mut grid, 2, 2, Some(text("mid")), None);
    insert_cell(&mut grid, 3, 0, None, Some("=A1"));

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

    for (idx, meta) in view.col_meta.iter().enumerate() {
        assert_eq!(meta.hash, col_signatures[idx]);
    }

    for (idx, meta) in view.row_meta.iter().enumerate() {
        assert_eq!(meta.signature, row_signatures[idx]);
    }

    assert_eq!(view.col_meta[0].non_blank_count, 2);
    assert_eq!(view.col_meta[0].first_non_blank_row, 2);
    assert_eq!(view.col_meta[1].non_blank_count, 1);
    assert_eq!(view.col_meta[1].first_non_blank_row, 0);
    assert_eq!(view.col_meta[2].non_blank_count, 1);
    assert_eq!(view.col_meta[2].first_non_blank_row, 2);
    assert_eq!(view.col_meta[3].non_blank_count, 1);
    assert_eq!(view.col_meta[3].first_non_blank_row, 1);
}

#[test]
fn gridview_empty_grid_is_stable() {
    let grid = Grid::new(0, 0);

    let view = GridView::from_grid(&grid);

    assert!(view.rows.is_empty());
    assert!(view.row_meta.is_empty());
    assert!(view.col_meta.is_empty());
}

#[test]
fn gridview_large_sparse_grid_constructs_without_panic() {
    let nrows = 10_000;
    let ncols = 10;
    let mut grid = Grid::new(nrows, ncols);

    for r in (0..nrows).step_by(100) {
        let col = (r / 100) % ncols;
        insert_cell(
            &mut grid,
            r,
            col,
            Some(CellValue::Number((r / 100) as f64)),
            None,
        );
    }

    let view = GridView::from_grid(&grid);

    assert_eq!(view.rows.len(), nrows as usize);
    assert_eq!(view.col_meta.len(), ncols as usize);

    assert_eq!(view.row_meta[1].non_blank_count, 0);
    assert_eq!(view.row_meta[100].non_blank_count, 1);
    assert_eq!(view.row_meta[100].first_non_blank_col, 1);

    assert!(
        view.col_meta
            .iter()
            .any(|meta| meta.non_blank_count > 0 && meta.first_non_blank_row == 0)
    );
}

#[test]
fn gridview_row_hashes_ignore_small_float_drift() {
    let mut grid_a = Grid::new(1, 1);
    insert_cell(&mut grid_a, 0, 0, Some(CellValue::Number(1.0)), None);

    let mut grid_b = Grid::new(1, 1);
    insert_cell(
        &mut grid_b,
        0,
        0,
        Some(CellValue::Number(1.0000000000000002)),
        None,
    );

    let view_a = GridView::from_grid(&grid_a);
    let view_b = GridView::from_grid(&grid_b);

    assert_eq!(
        view_a.row_meta[0].signature, view_b.row_meta[0].signature,
        "row signatures should be stable under ULP-level float differences"
    );
}
