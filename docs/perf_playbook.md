# Perf Playbook

This is the shortest path to reproduce perf suites locally and keep baselines sane.

## Perf validation policy

Use the **full perf cycle** for **major perf-risk changes**.
Use **quick/gate suites** for routine changes.

Run the full perf cycle when any of these are true:
- You change parse/open/container/alignment/diff behavior in Rust.
- You change desktop backend runtime/storage orchestration (`desktop/backend/src/**`) or payload shaping (`ui_payload/src/**`).
- You change Rust dependencies/toolchain/profiles (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`).
- You make an intentional optimization or expect non-trivial runtime/memory/I/O impact.

For routine non-major Rust changes, run quick suite first, then gate suite if the touched code affects large-grid or streaming behavior.

Escalation rule:
- If quick/gate fails, regresses unexpectedly, or results are noisy/suspicious, run the full perf cycle before merging.

## Full perf cycle (major changes)

```bash
python3 scripts/perf_cycle.py pre
# ...make changes...
python3 scripts/perf_cycle.py post --cycle <cycle_id>
```

The delta summary is written to `benchmarks/perf_cycles/<cycle_id>/cycle_delta.md`.
If fixture generation fails in your environment, add `--skip-fixtures`.

## Quick suite (CI default)

```bash
python scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv
```

## Gate suite (50k smoke)

```bash
python scripts/check_perf_thresholds.py --suite gate --parallel --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests
```

## Full-scale suite (ignored long-runs)

```bash
python scripts/check_perf_thresholds.py --suite full-scale --parallel --require-baseline --baseline benchmarks/baselines/full-scale.json --export-json benchmarks/latest_fullscale.json --export-csv benchmarks/latest_fullscale.csv
```

## E2E metrics (open + diff)

```bash
# Ensure e2e fixtures exist (keeps other fixture sets intact)
generate-fixtures --manifest fixtures/manifest_perf_e2e.yaml --force

python scripts/export_e2e_metrics.py --baseline benchmarks/baselines/e2e.json
```

Note:
- Avoid `--clean` when generating `manifest_perf_e2e.yaml` unless you intentionally want only that subset in `fixtures/generated/`.

## Baseline updates

Update baselines only when an algorithm change or dependency shift consistently moves results.
Avoid bumping baselines to "green CI" after a one-off regression.

Baseline bumps are acceptable when:
- A change is intentional and repeatable (algorithm tradeoff, dependency upgrade).
- The new results are within the agreed budgets (time + memory caps), and the
  impact is documented in the PR.
Otherwise, treat the regression as a bug and fix it before updating baselines.

```bash
python scripts/update_baselines.py --suite quick --parallel
python scripts/update_baselines.py --suite gate --parallel --test-target perf_large_grid_tests
python scripts/update_baselines.py --suite full-scale --parallel
```

The update script runs the suite, compares against the previous baseline, and
copies `benchmarks/latest_*.json` into `benchmarks/baselines/*.json`.

If a pinned baseline is missing, the perf scripts fall back to the newest JSON in
`benchmarks/results`. Keep the latest JSON artifacts (`benchmarks/latest_*.json`)
alongside baseline updates in your PR for review, and include a short rationale
for any accepted regressions.
