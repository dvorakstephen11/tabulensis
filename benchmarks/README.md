# Performance Benchmarks

This directory contains performance benchmarking infrastructure for excel_diff.

## Directory Structure

```
benchmarks/
├── README.md           # This file
├── results/            # Timestamped JSON performance data
│   └── *.json          # Individual benchmark run results
└── benches/            # Criterion benchmark definitions (in core/benches/)
```

## Running Benchmarks

### Quick Performance Tests (CI-friendly, ~1K rows)

```bash
cd core
cargo test --release --features perf-metrics perf_ -- --nocapture
```

### Full-Scale Performance Tests (50K rows, ~2-5 minutes)

```bash
cd core
cargo test --release --features perf-metrics -- --ignored --nocapture
```

### Criterion Benchmarks (with statistical analysis)

```bash
cd core
cargo bench
```

To compare against a saved baseline:

```bash
cargo bench -- --baseline main
```

To save a new baseline:

```bash
cargo bench -- --save-baseline main
```

### Export Metrics to JSON

```bash
python scripts/export_perf_metrics.py
```

This runs the performance tests and saves timestamped results to `benchmarks/results/`.

## Benchmark Categories

| Category | Grid Size | Purpose |
|----------|-----------|---------|
| P1 Large Dense | 50K × 100 | Baseline dense numeric data |
| P2 Large Noise | 50K × 100 | Every row different (worst case) |
| P3 Adversarial Repetitive | 50K × 50 | High hash collision scenario |
| P4 99% Blank | 50K × 100 | Sparse data handling |
| P5 Identical | 50K × 100 | Fast-path (no changes) validation |

## Interpreting Results

Each JSON result file contains:

```json
{
  "timestamp": "2025-12-12T10:30:00Z",
  "git_commit": "abc123",
  "tests": {
    "perf_p1_large_dense": {
      "total_time_ms": 1234,
      "alignment_time_ms": 500,
      "move_detection_time_ms": 200,
      "cell_diff_time_ms": 534,
      "rows_processed": 50000,
      "cells_compared": 5000000,
      "anchors_found": 100,
      "moves_detected": 0
    }
  }
}
```

## Thresholds

Target performance thresholds (50K rows):

| Test | Max Time | Max Memory |
|------|----------|------------|
| P1 Large Dense | 5s | 500MB |
| P2 Large Noise | 10s | 600MB |
| P3 Adversarial Repetitive | 15s | 400MB |
| P4 99% Blank | 2s | 200MB |
| P5 Identical | 1s | 300MB |

Note: Memory tracking requires `tikv-jemallocator` integration (planned future work).

