# Strategy Reconstruction (As-Is) (2026-02-09)

Run date: 2026-02-09

Goal: reconstruct Tabulensis strategy + operating system from primary docs + repo reality, without injecting "vibes".

Primary inputs (starting set):
- `APP_INTENT.md`
- `README.md`
- `product_roadmap.md`
- `AGENTS.md`
- `meta_methodology.md`
- `docs/index.md`

## Repo state (for provenance)

- Git branch: `2026-02-09_strategy_doc_audit` (audit branch; created during Phase 0)
- Git commit: `9039c85728e12de747535a024849528610bf5e4f`
- Worktree status: dirty (pre-existing local edits/untracked files).

## Win Conditions (stated)

From `APP_INTENT.md` (skim):
- MVP: downloadable executable; CLI + desktop UI; fast readable diffs; git integration; runs on Windows/macOS/Linux.
- End state: adds PBIX parsing and automation, logical error detection, and business target: $500k+ ARR or sale for $3M-$5M.

## Core Product Thesis (stated)

From `README.md` (skim):
- Product: structured diff for Excel workbooks and Power BI packages.
- Primary interfaces: `tabulensis` CLI, Rust library (`excel_diff` crate), desktop UI, and a WASM web demo as funnel.
- Differentiators hinted: readable diffs, performance on large files, semantic M diff, hardening limits.

## Roadmap (stated)

From `product_roadmap.md` (skim):
- Iteration framing: Iteration 0 (baseline you can market now), Iteration 1 ("daily driver" change summarization/noise controls/root-cause linking), Iteration 2 (PBIP/PBIR/TMDL support), Iteration 3 (Excel parity: formatting/pivots/validations), Iteration 4 (2-way merge for table-like sheets), Iteration 5 (3-way merge), Iteration 6 (version history), Iteration 7 (insights), Iteration 8 (monitoring/alerts), Iteration 9 (shareable diffs + collaborative review), Iteration 10 (enterprise governance), Iteration 11 (SDK/plugins/API), Iteration 12 (Power BI Desktop integration).
- Product positioning: “Modern diff for Modern Excel” (includes Power Query + tabular model diffs), with a privacy-first “runs locally” web demo as funnel.

## Constraints + Guardrails (stated)

From `AGENTS.md` / `docs/meta/README.md` (skim):
- `docs/index.md` is canonical docs entrypoint; checklist index must be maintained.
- Perf policy: run full perf cycle only for major perf-risk Rust changes; otherwise quick/gate suites.
- Avoid wide formatting churn; prefer targeted formatting.
- Fixture generation guardrails: avoid narrow-manifest `--clean` foot-guns.
- Safety: no destructive git operations; no deploys/secret rotation.

## Operating Cadence (stated)

From `meta_methodology.md` (skim):
- Daily open:
  - create/open `docs/meta/today.md`
  - create/open the daily log
  - pick top 3 tasks (max)
- Daily close:
  - update daily log under `docs/meta/logs/daily/YYYY-MM-DD.md`
  - update at least one checklist checkbox
  - refresh `docs/index.md` checklist index if counts changed
- Overnight automation envisioned: multiple agents at midnight + morning rounds; includes perf work, UI/UX suggestions, and meta-doc improvements.

## Ops Map (daily loop + automation wiring)

Daily loop (as documented):
- Daily open:
  - create/open `docs/meta/today.md`
  - create/open the daily log
  - pick top 3 tasks (max)
- Daily close:
  - update `docs/meta/logs/daily/YYYY-MM-DD.md`
  - update at least one checklist checkbox
  - refresh `docs/index.md` checklist index if counts changed

Overnight agent (as documented):
- Runbook: `docs/meta/automation/overnight_agent_runbook.md`
- Script: `scripts/overnight_agent.py`
- Config: `docs/meta/automation/overnight_agent.yaml`
- Safety model:
  - no deploys, no secret rotation, no destructive git operations
  - all changes occur in git worktrees; primary worktree is not modified
- Outputs:
  - per-task branches + worktrees under `overnight/*` and `../excel_diff_worktrees/overnight/`
  - local-only runtime under `tmp/overnight_agent/`
  - ops journal committed on dedicated branch `overnight/ops-journal` under `docs/meta/logs/ops/`

Prompt tooling (as documented):
- Prompt library index: `docs/meta/prompts/README.md`
- Deep research prompt copier: `scripts/deep_research_prompt.py` (validated: `--help`, `--list`)
- Results conventions: `docs/meta/results/README.md` and `docs/meta/logs/README.md`

Daily planning helpers (repo reality):
- Create/open daily log: `python3 scripts/new_daily_log.py --open`
- Generate bounded planning context payload: `python3 scripts/generate_daily_plan_context.py --copy`
- Copy daily-plan prompt: `python3 scripts/deep_research_prompt.py --prompt docs/meta/prompts/daily_plan.md`
- Deep research result file creation (A/B/synthesis): `python3 scripts/deep_research_prompt.py --new-a-b --prompt <name>`
- TTS audio from a synthesis file: `python3 scripts/tts_generate.py docs/meta/results/<result>_synthesis.md`

## Domain Summaries (as-is)

### Product UI/UX and desktop experience

- Desktop UI exists (wxDragon), with both native (XRC) UI and an optional WebView UI toggle:
  - Default: native (XRC)
  - Force WebView UI: `EXCEL_DIFF_USE_WEBVIEW=1 cargo run -p desktop_wx --bin desktop_wx` (see `docs/desktop.md`)
- Deterministic headless UI snapshot tooling is referenced in `AGENTS.md` (`scripts/ui_capture.sh`, `scripts/ui_snapshot_summary.py`).
- Strategy emphasis (via roadmap): reduce time-to-answer in the UI (summary, noise controls, root-cause linking) before deeper feature expansion.

### Core diff correctness + coverage

- The repo maintains multiple “coverage” workstreams under `docs/` (OpenXML, formulas, diff ops, M parser).
- Implied strategy: broaden construct coverage systematically while preserving determinism and readability of diffs.

### Performance and perf-validation policy

- Perf policy is explicit in `AGENTS.md`: full perf cycle only for major perf-risk changes; otherwise quick/gate suites.
- Supporting playbook exists under `docs/perf_playbook.md` and perf harness docs under `benchmarks/README.md`.
- Perf playbook details:
  - Full perf cycle: `python3 scripts/perf_cycle.py pre` / `post --cycle <id>` with retention guards.
  - Quick suite: `python scripts/check_perf_thresholds.py --suite quick ...`
  - Gate suite: `python scripts/check_perf_thresholds.py --suite gate ...`

### Distribution/release operations

- Dedicated SOPs exist: `docs/release_checklist.md`, `docs/release_readiness.md`, `docs/release_signing.md`, plus `docs/auto_update_strategy.md` and `docs/installer_ux.md`.
- Strategy emphasis: remove installation friction and make releases boring/repeatable.
- Repo reality: canonical release pipeline is GitHub Actions `.github/workflows/release.yml` (tag `v*`), and it currently ships CLI artifacts (Windows + macOS), not desktop app bundles.
- Release checklist explicitly calls out:
  - core test run (`cargo test --workspace`)
  - fuzz / WASM budgets / web demo deployment verification
  - manual smoke tests across Excel and PBIX edge cases (see `docs/release_checklist.md`)
- Signing doc expects CI-driven signing/notarization (macOS `notarytool` + Windows Azure Artifact Signing / Trusted Signing), with CI config supplied at runtime (see `docs/release_signing.md`).
- Auto-update is intentionally deferred until signing pipeline is stable (see `docs/auto_update_strategy.md`).

### Licensing and vendors (Stripe/Resend/Worker)

- Client-side licensing commands and env vars are documented in `README.md`.
- Licensing backend has two implementations (see `docs/licensing_service.md`):
  - Production: Cloudflare Worker + D1 (`tabulensis-api/`) with Stripe + optional Resend integration.
  - Local reference: Rust server (`license_service/`) with `LICENSE_MOCK_STRIPE=1` for simulated checkout completion.
- Vendor setup workstreams exist as operating docs/checklists: `docs/licensing_service.md`, `RESEND_SETUP_CHECKLIST.md`, `STRIPE_WORKER_NEXT_STEPS.md`.

### Automation and AI leverage (overnight + prompt tooling + payloads)

- Overnight agent is implemented and runbooked (`docs/meta/automation/*`, `scripts/overnight_agent.py`).
- Prompt library exists with deep research + synthesis prompts and a copier utility (`docs/meta/prompts/*`, `scripts/deep_research_prompt.py`).
- A deterministic “Pro context payload” specification exists (`docs/meta/prompts/pro_context_payload.md`), but the generator/output policy is still pending.

### Security posture and vulnerability discovery workflow

- `meta_methodology.md` calls out AI-driven vulnerability discovery.
- Prompt support exists (`docs/meta/prompts/deep_research_security_watch.md`), but the concrete security toolchain and recurring workflow are not yet fully specified.
- Minimal recurring checks were chosen and scaffolded during this audit:
  - `SECURITY_DAILY_CHECKLIST.md`
  - `bash scripts/security_audit.sh` (writes/updates `docs/meta/logs/ops/YYYY-MM-DD_security_audit.md`)
  - chosen tools recorded in `docs/meta/automation/guardrails.md` (see `docs/meta/results/decision_register.md` `DR-0016`)

### Marketing/research/acquisition strategy

- Deep research prompts exist for market analysis, distribution experiments, competitor watch, and acquisition targets (`docs/meta/prompts/deep_research_*.md`).
- There is a legacy backlog checklist (`tabulensis_launch_to_dos_from_our_chat.md`) with many vendor/business decisions.
- Minimal measurement primitives were chosen and scaffolded during this audit:
  - Canonical UTM scheme documented in `docs/meta/marketing_templates/README.md` (see `DR-0012`).
  - Metrics file: `docs/meta/logs/marketing/metrics.csv`.
  - UTM generator: `python3 scripts/utm.py --source ... --medium ... --campaign ... [--content ...]`.

## Conflicts / Gaps (initial)

- `docs/meta/today.md` is local-only (gitignored); the committed template `docs/meta/today.example.md` appears in the `docs/index.md` checklist auto-index (`DR-0002`).
- Decision gates existed in `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` for commit policy of logs/results/today; decisions are now recorded in `docs/meta/results/decision_register.md`.
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` referenced missing automation docs; created during this audit:
  - `docs/meta/automation/README.md`
  - `docs/meta/automation/guardrails.md`
  - `docs/meta/automation/model_accounts.md`
- Marketing templates scaffolding exists (`docs/meta/marketing_templates/**`), but the workstream still needs UTM + metrics wiring to be operational.
- Marketing templates + minimal measurement are now scaffolded:
  - UTM scheme: `docs/meta/marketing_templates/README.md` (`DR-0012`)
  - Metrics file: `docs/meta/logs/marketing/metrics.csv`
- Overnight agent runbook assumes some logs are committed (ops journal on `overnight/ops-journal`); this is now consistent with `DR-0001` (commit logs/results to git).
