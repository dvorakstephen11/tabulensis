# Agent Notes

## Common questions

- **Command to open the desktop app (from source):** `cargo run -p desktop_wx`
- **Optimized build:** `cargo run -p desktop_wx --profile release-desktop`
- **More detail:** see `docs/desktop.md` and the “Desktop App (from source)” section in `README.md`.

## Perf Validation Policy (Major vs Minor Changes)

Use the **full perf cycle** only for **major perf-risk changes**.

Run full cycle when any of these are true:
- You change parse/diff/alignment/open/container behavior in Rust (for example `core/src/**` paths involved in workbook open, XML/grid parse, diff engine, or alignment).
- You change desktop perf-sensitive orchestration/storage paths (for example `desktop/backend/src/diff_runner.rs`, `desktop/backend/src/store/**`, `ui_payload/src/**`).
- You change Rust dependencies/toolchain/profiles (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`).
- You make an intentional performance optimization or expect non-trivial runtime/memory/I/O impact.

Full perf cycle commands:
1. **Before edits:** `python3 scripts/perf_cycle.py pre`
2. **After edits:** `python3 scripts/perf_cycle.py post --cycle <cycle_id>`

This produces `benchmarks/perf_cycles/<cycle_id>/cycle_delta.md`.
If fixture generation fails in your environment, add `--skip-fixtures`.

For routine Rust changes (non-major), run lighter checks instead:
1. Quick suite:
   `python scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv`
2. Add gate suite when touching large-grid / streaming paths:
   `python scripts/check_perf_thresholds.py --suite gate --parallel --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests`

Escalation rule: if quick/gate fails or results are noisy/suspicious, run the full perf cycle before merging.
