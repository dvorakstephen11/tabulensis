#![cfg(feature = "perf-metrics")]

mod common;

use common::single_sheet_workbook;
use excel_diff::perf::DiffMetrics;
use excel_diff::{
    CellValue, DiffConfig, DiffConfigBuilder, DiffOp, DiffReport, Grid, Workbook, WorkbookPackage,
};

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

fn create_large_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
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

fn create_repetitive_grid(nrows: u32, ncols: u32, pattern_length: u32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        let pattern_idx = row % pattern_length;
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number((pattern_idx * 1000 + col) as f64)),
                None,
            );
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
                grid.insert_cell(
                    row,
                    col,
                    Some(CellValue::Number((row * 1000 + col) as f64)),
                    None,
                );
            }
        }
    }
    grid
}

fn log_perf_metric(name: &str, metrics: &DiffMetrics, tail: &str) {
    println!(
        "PERF_METRIC {name} total_time_ms={} move_detection_time_ms={} alignment_time_ms={} cell_diff_time_ms={} rows_processed={} cells_compared={} anchors_found={} moves_detected={}{}",
        metrics.total_time_ms,
        metrics.move_detection_time_ms,
        metrics.alignment_time_ms,
        metrics.cell_diff_time_ms,
        metrics.rows_processed,
        metrics.cells_compared,
        metrics.anchors_found,
        metrics.moves_detected,
        tail
    );
}

#[test]
fn perf_p1_large_dense() {
    let grid_a = create_large_grid(1000, 20, 0);
    let mut grid_b = create_large_grid(1000, 20, 0);
    grid_b.insert_cell(500, 10, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "P1 dense grid should complete successfully"
    );
    assert!(report.warnings.is_empty(), "P1 should have no warnings");
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "P1 should detect the cell edit"
    );
    assert!(
        report.metrics.is_some(),
        "P1 should have metrics when perf-metrics enabled"
    );
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P1 should process rows");
    assert!(metrics.cells_compared > 0, "P1 should compare cells");
    log_perf_metric("perf_p1_large_dense", &metrics, "");
}

#[test]
fn perf_p2_large_noise() {
    let grid_a = create_large_grid(1000, 20, 0);
    let grid_b = create_large_grid(1000, 20, 1);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "P2 noise grid should complete successfully"
    );
    assert!(report.metrics.is_some(), "P2 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P2 should process rows");
    log_perf_metric("perf_p2_large_noise", &metrics, "");
}

#[test]
fn perf_p3_adversarial_repetitive() {
    let grid_a = create_repetitive_grid(1000, 50, 100);
    let mut grid_b = create_repetitive_grid(1000, 50, 100);
    grid_b.insert_cell(500, 25, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "P3 repetitive grid should complete");
    assert!(report.metrics.is_some(), "P3 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P3 should process rows");
    log_perf_metric("perf_p3_adversarial_repetitive", &metrics, "");
}

#[test]
fn perf_p4_99_percent_blank() {
    let grid_a = create_sparse_grid(1000, 100, 1, 12345);
    let mut grid_b = create_sparse_grid(1000, 100, 1, 12345);
    grid_b.insert_cell(500, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "P4 sparse grid should complete");
    assert!(report.metrics.is_some(), "P4 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P4 should process rows");
    log_perf_metric("perf_p4_99_percent_blank", &metrics, "");
}

#[test]
fn perf_p5_identical() {
    let grid_a = create_large_grid(1000, 100, 0);
    let grid_b = create_large_grid(1000, 100, 0);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "P5 identical grid should complete");
    assert!(
        report.ops.is_empty(),
        "P5 identical grids should produce no ops"
    );
    assert!(report.metrics.is_some(), "P5 should have metrics");
    let metrics = report.metrics.unwrap();
    assert!(metrics.rows_processed > 0, "P5 should process rows");
    log_perf_metric("perf_p5_identical", &metrics, "");
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_dense_single_edit() {
    let grid_a = create_large_grid(50000, 100, 0);
    let mut grid_b = create_large_grid(50000, 100, 0);
    grid_b.insert_cell(25000, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(
        report.complete,
        "50k dense grid should complete successfully"
    );
    assert!(
        report.warnings.is_empty(),
        "50k dense should have no warnings"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "50k dense should detect the cell edit"
    );
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric(
        "perf_50k_dense_single_edit",
        &metrics,
        " (enforced: <30s; target: <5s)",
    );
    assert!(
        metrics.total_time_ms < 30000,
        "50k dense grid should complete in <30s, took {}ms",
        metrics.total_time_ms
    );
    assert_eq!(
        metrics.move_detection_time_ms, 0,
        "50k dense single edit should skip move detection (preflight bailout), got {}ms",
        metrics.move_detection_time_ms
    );
    assert_eq!(
        metrics.alignment_time_ms, 0,
        "50k dense single edit should skip alignment (preflight bailout), got {}ms",
        metrics.alignment_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_completely_different() {
    let grid_a = create_large_grid(50000, 100, 0);
    let grid_b = create_large_grid(50000, 100, 1);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k different grids should complete");
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric(
        "perf_50k_completely_different",
        &metrics,
        " (enforced: <60s; target: <10s)",
    );
    assert!(
        metrics.total_time_ms < 60000,
        "50k completely different should complete in <60s, took {}ms",
        metrics.total_time_ms
    );
    assert_eq!(
        metrics.move_detection_time_ms, 0,
        "50k completely different should skip move detection (preflight bailout), got {}ms",
        metrics.move_detection_time_ms
    );
    assert_eq!(
        metrics.alignment_time_ms, 0,
        "50k completely different should skip alignment (preflight bailout), got {}ms",
        metrics.alignment_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_adversarial_repetitive() {
    let grid_a = create_repetitive_grid(50000, 50, 100);
    let mut grid_b = create_repetitive_grid(50000, 50, 100);
    grid_b.insert_cell(25000, 25, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k repetitive should complete");
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric(
        "perf_50k_adversarial_repetitive",
        &metrics,
        " (enforced: <120s; target: <15s)",
    );
    assert!(
        metrics.total_time_ms < 120000,
        "50k adversarial repetitive should complete in <120s, took {}ms",
        metrics.total_time_ms
    );
    assert_eq!(
        metrics.move_detection_time_ms, 0,
        "50k adversarial repetitive should skip move detection (preflight bailout), got {}ms",
        metrics.move_detection_time_ms
    );
    assert_eq!(
        metrics.alignment_time_ms, 0,
        "50k adversarial repetitive should skip alignment (preflight bailout), got {}ms",
        metrics.alignment_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_99_percent_blank() {
    let grid_a = create_sparse_grid(50000, 100, 1, 12345);
    let mut grid_b = create_sparse_grid(50000, 100, 1, 12345);
    grid_b.insert_cell(25000, 50, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k sparse should complete");
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric("perf_50k_99_percent_blank", &metrics, " (target: <2s)");
    assert!(
        metrics.total_time_ms < 30000,
        "50k 99% blank should complete in <30s, took {}ms",
        metrics.total_time_ms
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn perf_50k_identical() {
    let grid_a = create_large_grid(50000, 100, 0);
    let grid_b = create_large_grid(50000, 100, 0);

    let wb_a = single_sheet_workbook("Performance", grid_a);
    let wb_b = single_sheet_workbook("Performance", grid_b);

    let config = DiffConfig::default();
    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "50k identical should complete");
    assert!(
        report.ops.is_empty(),
        "50k identical grids should have no ops"
    );
    let metrics = report.metrics.expect("should have metrics");
    log_perf_metric("perf_50k_identical", &metrics, " (target: <1s)");
    assert!(
        metrics.total_time_ms < 15000,
        "50k identical should complete in <15s, took {}ms",
        metrics.total_time_ms
    );
}

#[test]
fn preflight_skips_move_and_alignment_for_single_cell_edit_same_shape() {
    let grid_a = create_large_grid(6000, 50, 0);
    let mut grid_b = create_large_grid(6000, 50, 0);
    grid_b.insert_cell(3000, 25, Some(CellValue::Number(999999.0)), None);

    let wb_a = single_sheet_workbook("Preflight", grid_a);
    let wb_b = single_sheet_workbook("Preflight", grid_b);

    let config = DiffConfig::builder()
        .preflight_min_rows(5000)
        .preflight_in_order_mismatch_max(32)
        .preflight_in_order_match_ratio_min(0.995)
        .bailout_similarity_threshold(0.05)
        .build()
        .expect("valid config");

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "report should complete");

    let cell_edits: Vec<_> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .collect();
    assert_eq!(
        cell_edits.len(),
        1,
        "should have exactly 1 CellEdited op, got {}",
        cell_edits.len()
    );

    let structural_ops = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. }
                    | DiffOp::RowRemoved { .. }
                    | DiffOp::ColumnAdded { .. }
                    | DiffOp::ColumnRemoved { .. }
                    | DiffOp::BlockMovedRows { .. }
                    | DiffOp::BlockMovedColumns { .. }
                    | DiffOp::BlockMovedRect { .. }
            )
        })
        .count();
    assert_eq!(structural_ops, 0, "should have no structural/move ops");

    let metrics = report.metrics.expect("should have metrics");
    assert_eq!(
        metrics.move_detection_time_ms, 0,
        "move_detection_time_ms should be 0 (skipped), got {}",
        metrics.move_detection_time_ms
    );
    assert_eq!(
        metrics.alignment_time_ms, 0,
        "alignment_time_ms should be 0 (skipped), got {}",
        metrics.alignment_time_ms
    );

    log_perf_metric(
        "preflight_single_cell_edit",
        &metrics,
        " (preflight bailout)",
    );
}

#[test]
fn preflight_skips_move_and_alignment_for_low_similarity_same_shape() {
    let grid_a = create_large_grid(6000, 50, 0);
    let grid_b = create_large_grid(6000, 50, 100_000_000);

    let wb_a = single_sheet_workbook("Preflight", grid_a);
    let wb_b = single_sheet_workbook("Preflight", grid_b);

    let config = DiffConfig::builder()
        .preflight_min_rows(5000)
        .bailout_similarity_threshold(0.05)
        .build()
        .expect("valid config");

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "report should complete");

    let has_cell_edit = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::CellEdited { .. }));
    assert!(has_cell_edit, "should have at least one CellEdited op");

    let move_ops = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::BlockMovedRows { .. }
                    | DiffOp::BlockMovedColumns { .. }
                    | DiffOp::BlockMovedRect { .. }
            )
        })
        .count();
    assert_eq!(move_ops, 0, "should have no move ops");

    let metrics = report.metrics.expect("should have metrics");
    assert_eq!(
        metrics.move_detection_time_ms, 0,
        "move_detection_time_ms should be 0 (skipped), got {}",
        metrics.move_detection_time_ms
    );
    assert_eq!(
        metrics.alignment_time_ms, 0,
        "alignment_time_ms should be 0 (skipped), got {}",
        metrics.alignment_time_ms
    );

    log_perf_metric(
        "preflight_low_similarity",
        &metrics,
        " (dissimilar bailout)",
    );
}

#[test]
fn preflight_does_not_skip_when_multiset_equal_but_order_differs() {
    let mut grid_a = Grid::new(6000, 10);
    for row in 0..6000u32 {
        for col in 0..10u32 {
            grid_a.insert_cell(
                row,
                col,
                Some(CellValue::Number((row * 100 + col) as f64)),
                None,
            );
        }
    }

    let mut grid_b = grid_a.clone();

    for col in 0..10u32 {
        let tmp_row_50 = grid_b.get(50, col).cloned();
        let tmp_row_51 = grid_b.get(51, col).cloned();
        let tmp_row_52 = grid_b.get(52, col).cloned();

        let row_80 = grid_b.get(80, col).cloned();
        let row_81 = grid_b.get(81, col).cloned();
        let row_82 = grid_b.get(82, col).cloned();

        if let Some(c) = row_80 {
            grid_b.insert_cell(50, col, c.value, c.formula);
        }
        if let Some(c) = row_81 {
            grid_b.insert_cell(51, col, c.value, c.formula);
        }
        if let Some(c) = row_82 {
            grid_b.insert_cell(52, col, c.value, c.formula);
        }

        if let Some(c) = tmp_row_50 {
            grid_b.insert_cell(80, col, c.value, c.formula);
        }
        if let Some(c) = tmp_row_51 {
            grid_b.insert_cell(81, col, c.value, c.formula);
        }
        if let Some(c) = tmp_row_52 {
            grid_b.insert_cell(82, col, c.value, c.formula);
        }
    }

    let wb_a = single_sheet_workbook("MoveTest", grid_a);
    let wb_b = single_sheet_workbook("MoveTest", grid_b);

    let config = DiffConfig::builder()
        .preflight_min_rows(5000)
        .preflight_in_order_mismatch_max(32)
        .preflight_in_order_match_ratio_min(0.995)
        .build()
        .expect("valid config");

    let report = diff_workbooks(&wb_a, &wb_b, &config);

    assert!(report.complete, "report should complete");

    let metrics = report.metrics.expect("should have metrics");

    assert!(
        metrics.alignment_time_ms > 0 || metrics.cell_diff_time_ms > 0,
        "preflight should NOT short-circuit when rows are reordered (multiset equal); expected pipeline to proceed, but alignment_time_ms={}, cell_diff_time_ms={}",
        metrics.alignment_time_ms,
        metrics.cell_diff_time_ms
    );
}
