#![cfg(feature = "perf-metrics")]

use excel_diff::perf::{DiffMetrics, Phase};

#[test]
fn metrics_starts_with_zero_counts() {
    let metrics = DiffMetrics::default();
    assert_eq!(metrics.rows_processed, 0);
    assert_eq!(metrics.cells_compared, 0);
    assert_eq!(metrics.anchors_found, 0);
    assert_eq!(metrics.moves_detected, 0);
    assert_eq!(metrics.parse_time_ms, 0);
    assert_eq!(metrics.alignment_time_ms, 0);
    assert_eq!(metrics.move_detection_time_ms, 0);
    assert_eq!(metrics.cell_diff_time_ms, 0);
    assert_eq!(metrics.total_time_ms, 0);
    assert_eq!(metrics.diff_time_ms, 0);
    assert_eq!(metrics.peak_memory_bytes, 0);
}

#[test]
fn metrics_add_cells_compared_accumulates() {
    let mut metrics = DiffMetrics::default();
    metrics.add_cells_compared(100);
    assert_eq!(metrics.cells_compared, 100);
    metrics.add_cells_compared(50);
    assert_eq!(metrics.cells_compared, 150);
    metrics.add_cells_compared(1000);
    assert_eq!(metrics.cells_compared, 1150);
}

#[test]
fn metrics_add_cells_compared_saturates() {
    let mut metrics = DiffMetrics::default();
    metrics.cells_compared = u64::MAX - 10;
    metrics.add_cells_compared(100);
    assert_eq!(metrics.cells_compared, u64::MAX);
}

#[test]
fn metrics_phase_timing_accumulates() {
    let mut metrics = DiffMetrics::default();

    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Alignment);

    assert!(
        metrics.alignment_time_ms > 0,
        "alignment_time_ms should be non-zero after timed phase"
    );

    let first_alignment = metrics.alignment_time_ms;

    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Alignment);

    assert!(
        metrics.alignment_time_ms > first_alignment,
        "alignment_time_ms should accumulate across multiple phases"
    );
}

#[test]
fn metrics_different_phases_tracked_separately() {
    let mut metrics = DiffMetrics::default();

    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(5));
    metrics.end_phase(Phase::Alignment);

    metrics.start_phase(Phase::MoveDetection);
    std::thread::sleep(std::time::Duration::from_millis(5));
    metrics.end_phase(Phase::MoveDetection);

    metrics.start_phase(Phase::CellDiff);
    std::thread::sleep(std::time::Duration::from_millis(5));
    metrics.end_phase(Phase::CellDiff);

    assert!(metrics.alignment_time_ms > 0, "alignment should be tracked");
    assert!(
        metrics.move_detection_time_ms > 0,
        "move detection should be tracked"
    );
    assert!(metrics.cell_diff_time_ms > 0, "cell diff should be tracked");
}

#[test]
fn metrics_total_phase_separate_from_components() {
    let mut metrics = DiffMetrics::default();

    metrics.start_phase(Phase::Total);
    metrics.start_phase(Phase::Alignment);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Alignment);
    metrics.end_phase(Phase::Total);

    assert!(metrics.alignment_time_ms > 0);
    assert!(metrics.total_time_ms > 0);
    assert!(
        metrics.total_time_ms >= metrics.alignment_time_ms,
        "total should be >= alignment since it wraps alignment"
    );
}

#[test]
fn metrics_end_phase_without_start_is_safe() {
    let mut metrics = DiffMetrics::default();
    metrics.end_phase(Phase::Alignment);
    assert_eq!(metrics.alignment_time_ms, 0);
}

#[test]
fn metrics_parse_phase_tracks_time() {
    let mut metrics = DiffMetrics::default();
    metrics.start_phase(Phase::Parse);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Parse);
    assert!(metrics.parse_time_ms > 0, "parse_time_ms should be non-zero");
}

#[test]
fn metrics_diff_time_derived_from_total_minus_parse() {
    let mut metrics = DiffMetrics::default();
    metrics.start_phase(Phase::Total);
    metrics.start_phase(Phase::Parse);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Parse);
    std::thread::sleep(std::time::Duration::from_millis(10));
    metrics.end_phase(Phase::Total);

    assert!(
        metrics.total_time_ms >= metrics.parse_time_ms,
        "total should include parse time"
    );
    assert_eq!(
        metrics.diff_time_ms,
        metrics.total_time_ms.saturating_sub(metrics.parse_time_ms)
    );
}

#[test]
fn metrics_rows_processed_can_be_set_directly() {
    let mut metrics = DiffMetrics::default();
    metrics.rows_processed = 5000;
    assert_eq!(metrics.rows_processed, 5000);
    metrics.rows_processed = metrics.rows_processed.saturating_add(3000);
    assert_eq!(metrics.rows_processed, 8000);
}

#[test]
fn metrics_anchors_and_moves_can_be_set() {
    let mut metrics = DiffMetrics::default();
    metrics.anchors_found = 150;
    metrics.moves_detected = 3;
    assert_eq!(metrics.anchors_found, 150);
    assert_eq!(metrics.moves_detected, 3);
}

#[test]
fn metrics_clone_creates_independent_copy() {
    let mut metrics = DiffMetrics::default();
    metrics.rows_processed = 1000;
    metrics.cells_compared = 500;

    let cloned = metrics.clone();
    metrics.rows_processed = 2000;

    assert_eq!(cloned.rows_processed, 1000);
    assert_eq!(metrics.rows_processed, 2000);
}

#[test]
fn metrics_default_equality() {
    let m1 = DiffMetrics::default();
    let m2 = DiffMetrics::default();
    assert_eq!(m1, m2);
}
