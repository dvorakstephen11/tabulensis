# Performance Benchmarks

This directory contains performance benchmarking infrastructure for excel_diff.

## Directory Structure

```
benchmarks/
ƒ"oƒ"?ƒ"? baselines/          # Pinned baseline JSONs used for perf gates
├── README.md           # This file
├── results/            # Timestamped JSON performance data
│   └── *.json          # Individual benchmark run results
└── benches/            # Criterion benchmark definitions (in core/benches/)
```

## Running Benchmarks

### Perf Validation Policy

Run the **full perf cycle** only for **major perf-risk changes**.

Major-change triggers:
- Core parse/open/container/alignment/diff behavior changes (`core/src/**` hot paths).
- Desktop backend runtime/storage or payload-shaping changes (`desktop/backend/src/**`, `ui_payload/src/**`).
- Rust dependency/toolchain/profile changes (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`).
- Intentional optimization work expected to move runtime or memory materially.

Full cycle commands:

```bash
python3 scripts/perf_cycle.py pre
# ...make Rust changes...
python3 scripts/perf_cycle.py post --cycle <cycle_id>
```

The delta summary is written to `benchmarks/perf_cycles/<cycle_id>/cycle_delta.md`.
If fixture generation fails in your environment, add `--skip-fixtures`.

For routine non-major changes:
- Run quick suite.
- Add gate suite when touching large-grid/streaming paths.
- Escalate to full cycle if quick/gate fails or behaves unexpectedly.

### Perf Gate Suites (scripts used by CI)

Quick suite (PR gate, small grids):

```bash
python scripts/check_perf_thresholds.py --suite quick --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json
```

Gate suite (PR gate, 50k sentinel):

```bash
python scripts/check_perf_thresholds.py --suite gate --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests --export-json benchmarks/latest_gate.json
```

Full-scale suite (scheduled coverage):

```bash
python scripts/check_perf_thresholds.py --suite full-scale --baseline benchmarks/baselines/full-scale.json --export-json benchmarks/latest_fullscale.json
```

E2E parse+diff suite (scheduled, xlsx fixtures):

```bash
python scripts/export_e2e_metrics.py --baseline benchmarks/baselines/e2e.json --export-csv benchmarks/latest_e2e.csv
```

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

### Baseline Update Workflow

1. Run the suite on a known-good commit (commands above).
2. Copy the latest JSON into `benchmarks/baselines/`:
   - `quick.json`, `gate.json`, `full-scale.json`, `e2e.json`
3. Re-run the suite with `--baseline` to confirm no regressions.
4. Use `python scripts/compare_perf_results.py` to summarize deltas in the PR.

## Size Tracking

Release artifact sizes (CLI binaries, desktop installers) are tracked separately from perf.

Generate a size report (CLI example):

```bash
cargo build -p tabulensis-cli --profile release-cli --locked
python scripts/size_report.py --label cli --path target/release-cli/tabulensis --zip --out target/size_reports/cli.json
```

Update baselines:

```bash
python scripts/update_size_baselines.py target/size_reports/cli.json
```

Check budgets (hard caps + baseline slack):

```bash
python scripts/check_size_budgets.py target/size_reports/cli.json
```

Budget config lives in `benchmarks/size_budgets.json` and can look like:

```json
{
  "cli": {
    "raw_bytes": { "hard_cap_bytes": 15000000, "slack_ratio": 0.02 },
    "zip_bytes": { "hard_cap_bytes": 6000000, "slack_ratio": 0.02 }
  }
}
```

### Triage When a Gate Fails

1. Re-run the failing suite locally with the same command.
2. Inspect `benchmarks/latest_gate.json` (or the suite's latest JSON) for:
   - `total_time_ms`, `parse_time_ms`, `diff_time_ms`, `peak_memory_bytes`
   - `move_detection_time_ms` and `alignment_time_ms` for bailouts
3. If regression is expected, update the baseline with a short rationale.
4. Otherwise, optimize/fix and re-run the gate.

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

