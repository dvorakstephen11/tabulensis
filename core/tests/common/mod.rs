//! Common test utilities shared across integration tests.

#![allow(dead_code)]

use excel_diff::{
    CellValue, DiffConfig, DiffReport, Grid, Sheet, SheetKind, StringId, Workbook,
    WorkbookPackage, with_default_session,
};
use std::fs::File;
use std::path::PathBuf;

pub fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path.push(filename);
    path
}

pub fn open_fixture_pkg(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).unwrap_or_else(|e| {
        panic!("failed to open fixture {}: {e}", path.display());
    });
    WorkbookPackage::open(file).unwrap_or_else(|e| {
        panic!("failed to parse fixture {}: {e}", path.display());
    })
}

pub fn open_fixture_workbook(name: &str) -> Workbook {
    open_fixture_pkg(name).workbook
}

pub fn diff_fixture_pkgs(a: &str, b: &str, config: &DiffConfig) -> DiffReport {
    let pkg_a = open_fixture_pkg(a);
    let pkg_b = open_fixture_pkg(b);
    pkg_a.diff(&pkg_b, config)
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

pub fn sid(s: &str) -> StringId {
    with_default_session(|session| session.strings.intern(s))
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
