#![cfg(feature = "perf-metrics")]

use excel_diff::{
    CellAddress, CellSnapshot, CellValue, DiffOp, DiffSink, FormulaDiffResult, JsonLinesSink,
    StringPool,
};
use std::time::Instant;

fn log_perf_metric(name: &str, op_count: usize, emit_time_ms: u64, tail: &str) {
    println!(
        "PERF_METRIC {name} total_time_ms={} parse_time_ms=0 diff_time_ms={} signature_build_time_ms=0 move_detection_time_ms=0 alignment_time_ms=0 cell_diff_time_ms=0 op_emit_time_ms={} report_serialize_time_ms=0 peak_memory_bytes=0 grid_storage_bytes=0 string_pool_bytes=0 op_buffer_bytes=0 alignment_buffer_bytes=0 rows_processed=0 cells_compared=0 anchors_found=0 moves_detected=0 hash_lookups_est=0 allocations_est=0 op_count={}{}",
        emit_time_ms,
        emit_time_ms,
        emit_time_ms,
        op_count,
        tail,
    );
}

#[test]
#[ignore = "Perf test: run with `cargo test -p tabulensis-cli --release --features perf-metrics --test perf_cli_jsonl_emit -- --ignored --nocapture --test-threads=1`"]
fn perf_cli_jsonl_emit() {
    let nrows = 4000u32;
    let ncols = 50u32;
    let ops_to_emit = (nrows as u64).saturating_mul(ncols as u64) as usize;

    let mut pool = StringPool::new();
    let sheet = pool.intern("Sheet1");

    let mut sink = JsonLinesSink::new(std::io::sink());
    sink.begin(&pool).expect("begin should succeed");

    let start = Instant::now();
    for row in 0..nrows {
        for col in 0..ncols {
            let addr = CellAddress::from_indices(row, col);
            let from = CellSnapshot {
                addr,
                value: Some(CellValue::Number((row * 1000 + col) as f64)),
                formula: None,
            };
            let to = CellSnapshot {
                addr,
                value: Some(CellValue::Number((row * 1000 + col + 1) as f64)),
                formula: None,
            };
            sink.emit(DiffOp::CellEdited {
                sheet,
                addr,
                from,
                to,
                formula_diff: FormulaDiffResult::Unknown,
            })
            .expect("emit should succeed");
        }
    }
    sink.finish().expect("finish should succeed");

    let emit_time_ms = start.elapsed().as_millis() as u64;
    log_perf_metric(
        "cli_perf_jsonl_emit",
        ops_to_emit,
        emit_time_ms,
        &format!(" nrows={} ncols={}", nrows, ncols),
    );
}
