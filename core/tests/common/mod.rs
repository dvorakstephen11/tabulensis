//! Common test utilities shared across integration tests.

#![allow(dead_code)]

use excel_diff::{CellValue, Grid, Sheet, SheetKind, Workbook, with_default_session};
use std::path::PathBuf;

pub fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path.push(filename);
    path
}

pub fn grid_from_numbers(values: &[&[i32]]) -> Grid {
    let nrows = values.len() as u32;
    let ncols = if nrows == 0 {
        0
    } else {
        values[0].len() as u32
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, row_vals) in values.iter().enumerate() {
        for (c, v) in row_vals.iter().enumerate() {
            grid.insert_cell(r as u32, c as u32, Some(CellValue::Number(*v as f64)), None);
        }
    }

    grid
}

pub fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook {
    with_default_session(|session| Workbook {
        sheets: vec![Sheet {
            name: session.strings.intern(name),
            kind: SheetKind::Worksheet,
            grid,
        }],
    })
}
