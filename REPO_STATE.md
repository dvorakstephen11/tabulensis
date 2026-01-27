updated_at: 2026-01-24
last_task_id: 2026-01-24__152144__excel_diff__plan_next

# Repo summary
Tabulensis is a Rust workspace for diffing Excel workbooks and Power BI packages, shipping a CLI, library API, and a WebAssembly-powered web demo intended as a top-of-funnel preview.

# How to run
## Setup
- Install Rust nightly `nightly-2025-02-20` (see `rust-toolchain.toml`).
- Optional for scripts: Python 3.

## Dev
- CLI (dev run): `cargo run -p tabulensis-cli -- diff old.xlsx new.xlsx`
- Web demo (static): serve `web/` with any static server (no build step noted).
  - Dev port: 5179

## Test
- Recommended (CI-like): `python scripts/dev_test.py`
- Core tests: `cargo test`

## Build
- CLI binary: `cargo build -p tabulensis-cli --profile release-cli`
- Install locally: `cargo install --locked --path cli`
- WASM build (for web demo): `wasm-pack build wasm --release --target web --out-dir ./web/wasm`

# Deployment
- Marketing/docs site: static pages deployed on Cloudflare (source: `public/`).
- Web demo: static site deployed on Cloudflare (source: `web/`).
- Support email: Fastmail manages `support@tabulensis.com`.

# Current state
- Core diff engine, CLI, and WASM/web demo are present; README covers CLI usage and testing.
- Web demo assets live in `web/` with a worker-based WASM client, but no explicit dev server guidance.
- Desktop app exists under `desktop/` (wxDragon) with limited top-level onboarding.
- `APP_INTENT.md` now defines MVP and end-state product intent.

# Risks / debt
- Web demo workflow lacks a documented build+serve loop (WASM build + static server).
- Desktop app onboarding/docs are fragmented, raising ramp-up cost for contributors.
- Test surface is large; no fast smoke test command is documented for quick verification.

# Next best tasks
1. [P0] Document the web demo workflow (WASM build + static server) and keep dev port 5179 consistent in top-level docs.
2. [P1] Add a lightweight smoke test command (fixture diff) and document it alongside `scripts/dev_test.py`.
3. [P1] Add a concise desktop quick-start for `desktop/` (build/run, status, known gaps).
4. [P2] Capture support boundaries (XLSB unsupported, PBIX limits) in a short “Support boundaries” section.
5. [P2] Create a short contributor note describing how to refresh `web/wasm` artifacts for local UI changes.

# Tradeoffs
- Planning-only pass; no changes to source or docs beyond this state file.
