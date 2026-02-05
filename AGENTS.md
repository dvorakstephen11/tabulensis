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
1. **Before edits:** `python3 scripts/perf_cycle.py pre` (defaults to median-of-3 runs).
2. **After edits:** `python3 scripts/perf_cycle.py post --cycle <cycle_id>` (same run count + aggregation).

This produces `benchmarks/perf_cycles/<cycle_id>/cycle_delta.md`.
If fixture generation fails in your environment, add `--skip-fixtures`.
Use `--runs <n>` only when you intentionally need a different run count.

For routine Rust changes (non-major), run lighter checks instead:
1. Quick suite:
   `python scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv`
2. Add gate suite when touching large-grid / streaming paths:
   `python scripts/check_perf_thresholds.py --suite gate --parallel --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests`

Escalation rule: if quick/gate fails or results are noisy/suspicious, run the full perf cycle before merging.

## Agent Guardrails (Formatting + Fixtures)

### Formatting scope

- Avoid `cargo fmt --all` for targeted changes; it can create workspace-wide churn.
- Prefer file- or crate-scoped formatting:
  - `rustfmt <path/to/file.rs>`
  - `cargo fmt -p <crate>`
- Prefer the repo wrapper for targeted formatting:
  - `python3 scripts/safe_rustfmt.py` (staged Rust files)
  - `python3 scripts/safe_rustfmt.py --worktree` (all changed Rust files)
- Run workspace-wide formatting only when the task explicitly requires a repo-wide formatting pass.
- Before commit, run blast-radius guard for the staged set:
  - `python3 scripts/check_change_scope.py --staged`
- If a wide-scope change is intentional, include `[allow-wide-scope]` in commit message and document why.

### Fixture manifests and `--clean`

- `generate-fixtures --clean` removes files not present in the selected manifest.
- Use manifest-specific generation for perf e2e fixtures without deleting unrelated fixtures:
  - `generate-fixtures --manifest fixtures/manifest_perf_e2e.yaml --force`
- Use `--clean` only when intentionally resetting to one manifest set (typically `manifest_cli_tests.yaml` for CI-like local runs).
- If you used `--clean` on a narrow manifest, regenerate required fixture sets before running other tests.

## Continuous Agent Improvement

When you find a repeatable way to improve speed, correctness, or operator clarity, proactively document it in the same change when practical.

Preferred update targets:
- `AGENTS.md` for repository-wide workflow rules and guardrails.
- Relevant skill docs (`SKILL.md`) when the improvement is skill-specific.
- `README.md` (or nearest user-facing doc) when the behavior affects normal developer usage.

Minimum standard for doc updates:
- Capture the concrete trigger/condition.
- Provide exact command(s) or file path(s).
- State common failure mode(s) and safe default behavior.

If a skill doc is outside writable scope, add the guidance to `AGENTS.md` and note that the external skill should be updated later.
