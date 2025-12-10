#![cfg(feature = "perf-metrics")]

mod common;

use common::single_sheet_workbook;
use excel_diff::config::DiffConfig;
use excel_diff::diff::DiffOp;
use excel_diff::engine::diff_workbooks_with_config;
use excel_diff::{Cell, CellAddress, CellValue, Grid};

fn create_large_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert(Cell {
                row,
                col,
                address: CellAddress::from_indices(row, col),
                value: Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                formula: None,
            });
        }
    }
    grid
}

fn create_repetitive_grid(nrows: u32, ncols: u32, pattern_length: u32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        let pattern_idx = row % pattern_length;
        for col in 0..ncols {
            grid.insert(Cell {
                row,
                col,
                address: CellAddress::from_indices(row, col),
                value: Some(CellValue::Number((pattern_idx * 1000 + col) as f64)),
                formula: None,
            });
        }
    }
    grid
}

fn create_sparse_grid(nrows: u32, ncols: u32, fill_percent: u32, seed: u64) -> Grid {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            let mut hasher = DefaultHasher::new();
            (row, col, seed).hash(&mut hasher);
            let hash = hasher.finish();
            if (hash % 100) < fill_percent as u64 {
                grid.insert(Cell {
                    row,
                    col,
                    address: CellAddress::from_indices(row, col),
                    value: Some(CellValue::Number((row * 1000 + col) as f64)),
                    formula: None,
                });
            }
        }
    }
    grid
}

#[test]
fn perf_p1_large_dense() {
    let grid_a = create_large_grid(1000, 20, 0);
    let mut grid_b = create_large_grid(1000, 20, 0);
    grid_b.insert(Cell {
        row: 500,
        col: 10,
        address: CellAddress::from_indices(500, 10),
        value: Some(CellValue::Number(999999.0)),
        formula: None,
    });

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks_with_config(&wb_a, &wb_b, &config);

    assert!(report.complete, "P1 dense grid should complete successfully");
    assert!(report.warnings.is_empty(), "P1 should have no warnings");
    assert!(
        report.ops.iter().any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "P1 should detect the cell edit"
    );
    assert!(
        report.metrics.is_some(),
        "P1 should have metrics when perf-metrics enabled"
    );
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P1 should process rows");
    assert!(metrics.cells_compared > 0, "P1 should compare cells");
}

#[test]
fn perf_p2_large_noise() {
    let grid_a = create_large_grid(1000, 20, 0);
    let grid_b = create_large_grid(1000, 20, 1);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks_with_config(&wb_a, &wb_b, &config);

    assert!(report.complete, "P2 noise grid should complete successfully");
    assert!(report.metrics.is_some(), "P2 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P2 should process rows");
}

#[test]
fn perf_p3_adversarial_repetitive() {
    let grid_a = create_repetitive_grid(1000, 50, 100);
    let mut grid_b = create_repetitive_grid(1000, 50, 100);
    grid_b.insert(Cell {
        row: 500,
        col: 25,
        address: CellAddress::from_indices(500, 25),
        value: Some(CellValue::Number(999999.0)),
        formula: None,
    });

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks_with_config(&wb_a, &wb_b, &config);

    assert!(report.complete, "P3 repetitive grid should complete");
    assert!(report.metrics.is_some(), "P3 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P3 should process rows");
}

#[test]
fn perf_p4_99_percent_blank() {
    let grid_a = create_sparse_grid(1000, 100, 1, 12345);
    let mut grid_b = create_sparse_grid(1000, 100, 1, 12345);
    grid_b.insert(Cell {
        row: 500,
        col: 50,
        address: CellAddress::from_indices(500, 50),
        value: Some(CellValue::Number(999999.0)),
        formula: None,
    });

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks_with_config(&wb_a, &wb_b, &config);

    assert!(report.complete, "P4 sparse grid should complete");
    assert!(report.metrics.is_some(), "P4 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P4 should process rows");
}

#[test]
fn perf_p5_identical() {
    let grid_a = create_large_grid(1000, 100, 0);
    let grid_b = create_large_grid(1000, 100, 0);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks_with_config(&wb_a, &wb_b, &config);

    assert!(report.complete, "P5 identical grid should complete");
    assert!(report.ops.is_empty(), "P5 identical grids should produce no ops");
    assert!(report.metrics.is_some(), "P5 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P5 should process rows");
}

