updated_at: 2026-01-15
last_task_id: 2026-01-15__011228__excel_diff__plan_next

# Repo summary
Excel Diff is a Rust workspace that compares Excel workbooks and Power BI packages and emits structured diffs via CLI, library APIs, and a WebAssembly-powered web demo.

# How to run
## Setup
- Install Rust 1.85+ (see `README.md`).
- Optional for scripts: Python 3.

## Dev
- CLI (dev run): `cargo run -p excel_diff_cli -- diff old.xlsx new.xlsx`
- Web demo (static): `python -m http.server 5179 --directory web`
  - Dev port: 5179

## Test
- Recommended (CI-like): `python scripts/dev_test.py`
- Core tests: `cargo test`

## Build
- CLI binary: `cargo build -p excel_diff_cli`
- Install locally: `cargo install --locked --path cli`

# Current state
- Core diff engine, CLI, and WASM/web demo are present and documented in `README.md`.
- Desktop (Tauri) project exists but lacks a quick-start in repo-level docs.
- No `APP_INTENT.md` found; repo intent inferred from `README.md`.

# Risks / debt
- Missing `APP_INTENT.md` makes product scope and MVP goals implicit rather than explicit.
- Web demo/dev workflow lacks a documented dev server and build steps (static only).
- Large test surface may be slow; no lightweight smoke test documented.

# Next best tasks
1. [P0] Add `APP_INTENT.md` describing the MVP scope, primary user flows (CLI, web demo), and non-goals.
2. [P1] Document web demo workflow (serve/build/wasm steps) and confirm dev port 5179 in `README.md` or `docs/index.md`.
3. [P1] Add a lightweight smoke test script (e.g., diff two fixtures) and document it alongside `scripts/dev_test.py`.
4. [P2] Add quick-start notes for the desktop (Tauri) app in `desktop/src-tauri/FEATURES.md` or top-level docs.
5. [P2] Capture known limitations (PBIX tabular-only, XLSB unsupported) in a concise “Support boundaries” doc section for user clarity.

# Tradeoffs
- Planning-only pass used repo docs and structure; no `APP_INTENT.md` available to ground intent.
