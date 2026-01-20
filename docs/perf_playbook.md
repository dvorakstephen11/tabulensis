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
python scripts/check_perf_thresholds.py --suite full-scale --require-baseline --baseline benchmarks/baselines/full-scale.json --export-json benchmarks/latest_fullscale.json --export-csv benchmarks/latest_fullscale.csv
```

## E2E metrics (open + diff)

```bash
python scripts/export_e2e_metrics.py --baseline benchmarks/baselines/e2e.json
```

## Baseline updates

Update baselines only when an algorithm change or dependency shift consistently moves results.
Avoid bumping baselines to "green CI" after a one-off regression.

Baseline bumps are acceptable when:
- A change is intentional and repeatable (algorithm tradeoff, dependency upgrade).
- The new results are within the agreed budgets (time + memory caps), and the
  impact is documented in the PR.
Otherwise, treat the regression as a bug and fix it before updating baselines.

```bash
python scripts/update_baselines.py --suite quick
python scripts/update_baselines.py --suite gate --test-target perf_large_grid_tests
python scripts/update_baselines.py --suite full-scale
```

The update script runs the suite, compares against the previous baseline, and
copies `benchmarks/latest_*.json` into `benchmarks/baselines/*.json`.

If a pinned baseline is missing, the perf scripts fall back to the newest JSON in
`benchmarks/results`. Keep the latest JSON artifacts (`benchmarks/latest_*.json`)
alongside baseline updates in your PR for review, and include a short rationale
for any accepted regressions.
