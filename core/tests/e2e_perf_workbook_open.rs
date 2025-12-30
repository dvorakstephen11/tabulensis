#![cfg(feature = "perf-metrics")]

mod common;

use common::fixture_path;
use excel_diff::perf::DiffMetrics;
use excel_diff::{CallbackSink, DiffConfig, WorkbookPackage};
use std::fs::File;

fn open_fixture_with_size(name: &str) -> (WorkbookPackage, u64) {
    let path = fixture_path(name);
    let bytes = std::fs::metadata(&path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let file = File::open(&path).unwrap_or_else(|e| {
        panic!("failed to open fixture {}: {e}", path.display());
    });
    let pkg = WorkbookPackage::open(file).unwrap_or_else(|e| {
        panic!("failed to parse fixture {}: {e}", path.display());
    });
    (pkg, bytes)
}

fn log_perf_metric(name: &str, metrics: &DiffMetrics, old_bytes: u64, new_bytes: u64) {
    let total_input_bytes = old_bytes.saturating_add(new_bytes);
    println!(
        "PERF_METRIC {name} total_time_ms={} parse_time_ms={} diff_time_ms={} signature_build_time_ms={} move_detection_time_ms={} alignment_time_ms={} cell_diff_time_ms={} op_emit_time_ms={} report_serialize_time_ms={} peak_memory_bytes={} grid_storage_bytes={} string_pool_bytes={} op_buffer_bytes={} alignment_buffer_bytes={} rows_processed={} cells_compared={} anchors_found={} moves_detected={} hash_lookups_est={} allocations_est={} old_bytes={} new_bytes={} total_input_bytes={}",
        metrics.total_time_ms,
        metrics.parse_time_ms,
        metrics.diff_time_ms,
        metrics.signature_build_time_ms,
        metrics.move_detection_time_ms,
        metrics.alignment_time_ms,
        metrics.cell_diff_time_ms,
        metrics.op_emit_time_ms,
        metrics.report_serialize_time_ms,
        metrics.peak_memory_bytes,
        metrics.grid_storage_bytes,
        metrics.string_pool_bytes,
        metrics.op_buffer_bytes,
        metrics.alignment_buffer_bytes,
        metrics.rows_processed,
        metrics.cells_compared,
        metrics.anchors_found,
        metrics.moves_detected,
        metrics.hash_lookups_est,
        metrics.allocations_est,
        old_bytes,
        new_bytes,
        total_input_bytes
    );
}

fn run_e2e_case(name: &str, old_name: &str, new_name: &str, expect_ops: bool) {
    let (old_pkg, old_bytes) = open_fixture_with_size(old_name);
    let (new_pkg, new_bytes) = open_fixture_with_size(new_name);

    let mut op_count = 0usize;
    let summary = {
        let mut sink = CallbackSink::new(|_op| op_count += 1);
        old_pkg
            .diff_streaming(&new_pkg, &DiffConfig::default(), &mut sink)
            .expect("diff_streaming should succeed")
    };

    assert!(summary.complete, "expected streaming diff to complete");
    assert_eq!(
        summary.op_count, op_count,
        "summary op_count should match sink-emitted ops"
    );

    if expect_ops {
        assert!(summary.op_count > 0, "expected at least one op");
    } else {
        assert_eq!(summary.op_count, 0, "expected no ops");
    }

    let metrics = summary.metrics.expect("expected perf metrics");
    assert!(
        metrics.parse_time_ms > 0,
        "parse_time_ms should be non-zero for e2e fixtures"
    );
    assert!(
        metrics.total_time_ms >= metrics.parse_time_ms,
        "total_time_ms should include parse_time_ms"
    );
    assert_eq!(
        metrics.diff_time_ms,
        metrics.total_time_ms.saturating_sub(metrics.parse_time_ms)
    );

    log_perf_metric(name, &metrics, old_bytes, new_bytes);
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn e2e_p1_dense_single_edit() {
    run_e2e_case(
        "e2e_p1_dense",
        "e2e_p1_dense_a.xlsx",
        "e2e_p1_dense_b.xlsx",
        true,
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn e2e_p2_noise_single_edit() {
    run_e2e_case(
        "e2e_p2_noise",
        "e2e_p2_noise_a.xlsx",
        "e2e_p2_noise_b.xlsx",
        true,
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn e2e_p3_repetitive_single_edit() {
    run_e2e_case(
        "e2e_p3_repetitive",
        "e2e_p3_repetitive_a.xlsx",
        "e2e_p3_repetitive_b.xlsx",
        true,
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn e2e_p4_sparse_single_edit() {
    run_e2e_case(
        "e2e_p4_sparse",
        "e2e_p4_sparse_a.xlsx",
        "e2e_p4_sparse_b.xlsx",
        true,
    );
}

#[test]
#[ignore = "Long-running test: run with `cargo test --features perf-metrics -- --ignored` to execute"]
fn e2e_p5_identical() {
    run_e2e_case(
        "e2e_p5_identical",
        "e2e_p5_identical_a.xlsx",
        "e2e_p5_identical_b.xlsx",
        false,
    );
}
