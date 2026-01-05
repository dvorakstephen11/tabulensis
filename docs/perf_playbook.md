# Perf Playbook

This is the shortest path to reproduce the perf CI suites locally and keep baselines sane.

## Quick suite (CI default)

```bash
python scripts/check_perf_thresholds.py --suite quick --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv
```

## Gate suite (50k smoke)

```bash
python scripts/check_perf_thresholds.py --suite gate --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests
```

## Full-scale suite (ignored long-runs)

```bash
python scripts/check_perf_thresholds.py --suite full-scale --baseline benchmarks/baselines/full-scale.json --export-json benchmarks/latest_fullscale.json --export-csv benchmarks/latest_fullscale.csv
```

## E2E metrics (open + diff)

```bash
python scripts/export_e2e_metrics.py --baseline benchmarks/baselines/e2e.json
```

## Baseline updates

Update baselines only when an algorithm change or dependency shift consistently moves results.

```bash
python scripts/update_baselines.py --suite quick
python scripts/update_baselines.py --suite gate --test-target perf_large_grid_tests
python scripts/update_baselines.py --suite full-scale
```

If a pinned baseline is missing, the perf scripts fall back to the newest JSON in
`benchmarks/results`. Keep the latest JSON artifacts (`benchmarks/latest_*.json`)
alongside baseline updates in your PR for review.
