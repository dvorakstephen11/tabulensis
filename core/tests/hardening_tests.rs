mod common;

use common::single_sheet_workbook;
use excel_diff::{CellValue, DiffConfig, DiffOp, Grid, ProgressCallback, WorkbookPackage};
use std::sync::Mutex;

fn create_simple_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                None,
            );
        }
    }
    grid
}

#[test]
fn memory_budget_forces_positional_fallback_and_warning() {
    let grid_a = create_simple_grid(10, 3, 0);
    let mut grid_b = create_simple_grid(10, 3, 0);
    grid_b.insert_cell(5, 1, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        max_memory_mb: Some(0),
        ..Default::default()
    };

    let report = WorkbookPackage::from(wb_a).diff(&WorkbookPackage::from(wb_b), &config);

    assert!(!report.complete, "memory fallback should mark report incomplete");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("memory")),
        "expected a memory warning: {:?}",
        report.warnings
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("positional")),
        "expected warning to mention positional diff: {:?}",
        report.warnings
    );
    assert!(
        report.ops.iter().any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "should still emit ops via positional diff"
    );
}

#[test]
fn timeout_yields_partial_report_and_warning() {
    let grid_a = create_simple_grid(10, 3, 0);
    let mut grid_b = create_simple_grid(10, 3, 0);
    grid_b.insert_cell(5, 1, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig {
        timeout_seconds: Some(0),
        ..Default::default()
    };

    let report = WorkbookPackage::from(wb_a).diff(&WorkbookPackage::from(wb_b), &config);

    assert!(!report.complete, "timeout should mark report incomplete");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("timeout")),
        "expected a timeout warning: {:?}",
        report.warnings
    );
}

#[derive(Default)]
struct CollectProgress {
    events: Mutex<Vec<(String, f32)>>,
}

impl ProgressCallback for CollectProgress {
    fn on_progress(&self, phase: &str, percent: f32) {
        let mut events = match self.events.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        events.push((phase.to_string(), percent));
    }
}

#[test]
fn progress_callback_fires_for_cell_diff() {
    let grid_a = create_simple_grid(512, 10, 0);
    let mut grid_b = create_simple_grid(512, 10, 0);
    grid_b.insert_cell(500, 5, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let config = DiffConfig::default();
    let progress = CollectProgress::default();

    let _report =
        WorkbookPackage::from(wb_a).diff_with_progress(&WorkbookPackage::from(wb_b), &config, &progress);

    let events = match progress.events.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    assert!(
        events.iter().any(|(phase, _)| phase == "cell_diff"),
        "expected at least one cell_diff progress event: {:?}",
        *events
    );
    assert!(
        events.iter().all(|(_, pct)| *pct >= 0.0 && *pct <= 1.0),
        "percent should stay within [0.0, 1.0]: {:?}",
        *events
    );
    assert!(
        events.len() < 10_000,
        "progress callbacks should be throttled: got {} callbacks",
        events.len()
    );
}

