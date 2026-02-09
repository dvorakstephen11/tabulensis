# Strategy Questions + Answers (2026-02-09)

Run date: 2026-02-09

This is the question bank for `docs/meta/strategy_audit_plan.md`.

Each entry must include:
- ID (stable slug)
- Source pointer (doc path + section)
- Category (`decision` / `evidence` / `research` / `code-verify` / `vendor` / `process`)
- Priority (`P0` blocks strategy, `P1` blocks execution, `P2` nice-to-clarify)
- Proposed method (docs / code check / deep research capture / operator decision)
- Status (open / answered / decision-required / deferred)
- Answer (if answered): evidence + confidence

---

## QA-0001: Are `docs/meta/logs/**` and `docs/meta/results/**` committed or local-only?

- Source: `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate)
- Category: decision
- Priority: P0
- Method: operator decision (record in decision register)
- Status: answered

Notes:
- Tracked as `DR-0001` in `docs/meta/results/decision_register.md`.

Answer:
- Commit `docs/meta/logs/**` and `docs/meta/results/**` to git (with strict redaction discipline).

Evidence:
- Decision recorded: `docs/meta/results/decision_register.md` (`DR-0001`).
- Overnight agent runbook assumes committed ops journal outputs under `docs/meta/logs/ops/`.

Confidence: high.

## QA-0002: Is `docs/meta/today.md` committed, and what is its exact intended structure?

- Source: `meta_methodology.md` ("Daily scratchpad"); `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (decision gate)
- Category: decision + process
- Priority: P0
- Method: operator decision + doc reconciliation
- Status: answered

Notes:
- Tracked as `DR-0002` in `docs/meta/results/decision_register.md`.

Answer:
- `docs/meta/today.md` is local-only (gitignored).
- Template is committed at `docs/meta/today.example.md`.
- Intended structure:
  - Timebox (minutes/commitments/energy)
  - Stop conditions
  - Top priorities (max 3) as checkboxes with source pointers
  - Optional “If time remains”

Evidence:
- Decision recorded: `docs/meta/results/decision_register.md` (`DR-0002`).
- Template: `docs/meta/today.example.md`.

Confidence: high.

## QA-0003: What is the canonical "daily open" command/tooling, if any?

- Source: `meta_methodology.md` (daily open rules); `docs/meta/prompts/daily_plan.md` + scripts
- Category: code-verify
- Priority: P1
- Method: repo reality: check existence of helper scripts + expected outputs
- Status: answered

Answer:
- Canonical daily-open workflow is manual but supported by helper scripts:
  - Plan/scratchpad: `docs/meta/today.md` (fill using daily-plan prompt output).
  - Daily log: `python3 scripts/new_daily_log.py --open`
  - Planning context payload: `python3 scripts/generate_daily_plan_context.py --copy`
  - Daily-plan prompt: `python3 scripts/deep_research_prompt.py --prompt docs/meta/prompts/daily_plan.md`
- Evidence:
  - `meta_methodology.md` (daily open rules + helper commands).
  - `scripts/new_daily_log.py`, `scripts/generate_daily_plan_context.py`, `scripts/deep_research_prompt.py`.

Confidence: high (docs + repo reality).

## QA-0004: What is the canonical "deep research capture" workflow and naming scheme?

- Source: `docs/meta/logs/README.md`; `docs/meta/results/README.md`; `docs/meta/prompts/README.md`
- Category: process
- Priority: P1
- Method: docs reconciliation (align docs); verify `scripts/deep_research_prompt.py --help`
- Status: answered

Answer (workflow):
- Workflow as implemented:
  - Use `scripts/deep_research_prompt.py --prompt <name>` to copy a prompt (or `--print` to stdout).
  - Create result files under `docs/meta/results/` via `--new-result` or `--new-a-b` (A/B/synthesis).
  - Store audio outputs (placeholder paths) under `docs/meta/audio/` (via `--audio`).
- Evidence:
  - `docs/meta/prompts/README.md` (tooling usage).
  - `scripts/deep_research_prompt.py --help` (flags and output paths).

Naming scheme (chosen):
- Canonical filenames are timestamp-prefixed and collision-safe:
  - `YYYY-MM-DD_HHMMSS_<topic>.md`
  - For A/B + synthesis: `..._a.md`, `..._b.md`, `..._synthesis.md`
- Evidence:
  - `scripts/deep_research_prompt.py` implements this naming.
  - Decision recorded: `docs/meta/results/decision_register.md` (`DR-0004`).

Confidence: high (repo reality + docs).

## QA-0005: What is the overnight agent lifecycle and minimum "healthy daily operation"?

- Source: `docs/meta/automation/overnight_agent_runbook.md`, `docs/meta/automation/overnight_agent.yaml`, `scripts/overnight_agent.py`
- Category: code-verify
- Priority: P1
- Method: run `doctor` + `list-tasks`; verify config/schema alignment
- Status: answered

Answer:
- Minimum healthy operation (per runbook + validated tooling):
  - Run `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml doctor` to validate deps/config.
  - Run `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml list-tasks --limit 30` to confirm task ingestion.
  - Use `supervise` for an overnight run; use `run-once --dry-run` for a no-LLM/no-change sanity check.
- Where outputs go:
  - Branch/worktrees under `overnight/*` and `../excel_diff_worktrees/overnight/`.
  - Local-only runtime under `tmp/overnight_agent/`.
  - Committed ops journal on `overnight/ops-journal` under `docs/meta/logs/ops/`.
- Evidence:
  - `docs/meta/automation/overnight_agent_runbook.md` (contract).
  - `docs/meta/automation/overnight_agent.yaml` (config).
  - Tool validation: `doctor` output reported PyYAML/requests OK and config OK.

Confidence: high (docs + repo reality).

## QA-0006: What deep research filename scheme is canonical (date-only vs timestamp)?

- Source: `docs/meta/logs/README.md`; `docs/meta/results/README.md`; `scripts/deep_research_prompt.py`
- Category: decision + code-verify
- Priority: P1
- Method: repo reality + doc reconciliation (decide and align docs + tooling)
- Status: answered

Answer:
- Canonical scheme is timestamp-prefixed, matching tooling:
  - `YYYY-MM-DD_HHMMSS_<topic>.md`
  - `..._a.md`, `..._b.md`, `..._synthesis.md` for A/B/synthesis

Evidence:
- `scripts/deep_research_prompt.py` uses `%Y-%m-%d_%H%M%S` prefixes for `--new-result` and `--new-a-b`.
- Decision recorded: `docs/meta/results/decision_register.md` (`DR-0004`).

Confidence: high (repo reality).

## QA-0007: Should meta checklist references to missing docs be fixed by creating the docs or by updating the checklist?

- Source: `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` references:
  - `docs/meta/automation/README.md`
  - `docs/meta/marketing_templates/README.md`
  - `docs/meta/automation/model_accounts.md`
  - `docs/meta/automation/guardrails.md`
- Category: process + evidence
- Priority: P1
- Method: repo reality (confirm missing), then operator decision: create scaffolding vs update references
- Status: answered

Answer (so far):
- Create the missing scaffolding docs when they are referenced as canonical recording locations and help reduce friction for operators/automation.
  - Created:
    - `docs/meta/automation/README.md`
    - `docs/meta/automation/guardrails.md`
    - `docs/meta/automation/model_accounts.md`

Still open:
- Marketing templates scaffolding was created:
  - `docs/meta/marketing_templates/README.md`
  - `docs/meta/marketing_templates/post_short_demo.md`
  - `docs/meta/marketing_templates/post_before_after.md`
  - `docs/meta/marketing_templates/post_user_story.md`

Status update:
- Remaining missing pieces are now “content/workflow decisions” (UTM scheme, metrics file, posting cadence), not missing docs.

## QA-0008: What is the intended “daily open” UX: manual steps only, or scripted helpers?

- Source: `meta_methodology.md` (rules); `scripts/new_daily_log.py`; `scripts/generate_daily_plan_context.py`; `docs/meta/prompts/daily_plan.md`
- Category: process + code-verify
- Priority: P1
- Method: docs + repo reality; reconcile into a single canonical daily-open procedure
- Status: answered

Answer:
- Manual steps are the source of truth (`meta_methodology.md`), with scripted helpers to reduce friction:
  - daily log creation (`scripts/new_daily_log.py`)
  - context bundle generation (`scripts/generate_daily_plan_context.py`)
  - prompt copying (`scripts/deep_research_prompt.py`)
- This keeps the “operator loop” runnable even if automation/LLMs fail.

Confidence: high.

## QA-0009: What is the canonical release pipeline (CI workflows, signing/notarization wiring)?

- Source: `docs/release_checklist.md`, `docs/release_signing.md`, `README.md`
- Category: code-verify + process
- Priority: P1
- Method: repo reality (locate CI workflows + scripts), then reconcile docs if missing
- Status: answered

Answer:
- Canonical release pipeline is GitHub Actions: `.github/workflows/release.yml`.
  - Trigger: tag pushes matching `v*` (and `workflow_dispatch`).
  - Gates:
    - version validation: `python3 scripts/verify_release_versions.py`
    - perf gate: full-scale perf suite (`python scripts/check_perf_thresholds.py --suite full-scale ...`)
  - Builds/publishes:
    - Windows: builds `tabulensis-cli`, uploads `.exe` and `.zip` plus SHA256 files (including stable `tabulensis-latest-*` asset names).
    - macOS: builds per-arch tarballs and a universal tarball (via `lipo`), plus a stable `tabulensis-latest-macos-universal.tar.gz`.
    - Linux: builds an x86_64 tarball, plus a stable `tabulensis-latest-linux-x86_64.tar.gz`.
    - Smoke test: runs basic CLI smoke checks from the universal tarball.
    - Generates Scoop + Homebrew manifests and attaches them to the GitHub Release.
- Signing/notarization status (repo reality):
  - macOS universal binary is ad-hoc signed in the workflow (`codesign -s - tabulensis`), not notarized.
  - No Windows signing step exists in `.github/workflows/release.yml` yet (planned: Azure Artifact Signing / Trusted Signing; see `docs/release_signing.md` + `docs/meta/results/decision_register.md` `DR-0020`).

Evidence:
- `.github/workflows/release.yml`
- `docs/release_signing.md` describes intended future state; it is not yet wired into CI.

Confidence: high (repo reality).

## QA-0010: Is desktop packaging + signing integrated into releases, or CLI-only today?

- Source: `README.md`, `docs/release_signing.md`, `scripts/package_desktop_appimage.py`, `docs/installer_ux.md`
- Category: evidence + code-verify
- Priority: P1
- Method: repo reality (inspect packaging scripts + release workflows); update release checklist if needed
- Status: answered

Answer:
- Release CI (`.github/workflows/release.yml`) is currently CLI-only (ships `tabulensis` artifacts).
- Desktop packaging exists as a local script for Linux AppImage:
  - `scripts/package_desktop_appimage.py` (packages `desktop_wx` into an AppImage)
- Desktop packaging/signing is not integrated into the release workflow.

Evidence:
- `.github/workflows/release.yml` does not reference `desktop_wx` or desktop packaging scripts.
- `scripts/package_desktop_appimage.py` exists but is not invoked by CI.

Confidence: high (repo reality).

## QA-0011: What is the production licensing backend source-of-truth (Worker + D1) and what is the minimum “ops dashboard” to keep it healthy?

- Source: `docs/licensing_service.md`, `docs/operations.md`, `STRIPE_WORKER_NEXT_STEPS.md`
- Category: process + vendor
- Priority: P1
- Method: docs + repo reality; extract required env vars, failure modes, and routine checks
- Status: partially-answered

Answer (source-of-truth):
- Production licensing backend is the Cloudflare Worker + D1 implementation under `tabulensis-api/`.
- The Rust `license_service/` is a reference/local server for dev.

Answer (minimum healthy ops checks, as documented):
- Store Stripe secrets + webhook signing secret in a secrets manager.
- Monitor licensing logs for webhook failures and activation errors.
- Keep an admin “reset activations” path available (`POST /license/reset` with `LICENSE_ADMIN_TOKEN`) for abuse/chargebacks.

Evidence:
- `docs/licensing_service.md` (Worker vs Rust backends, env vars, endpoints).
- `docs/operations.md` (ops essentials + abuse/chargeback action).

Confidence: medium (docs clear on components, but no single consolidated “ops dashboard” SOP exists yet).

## QA-0012: What is the canonical support/abuse response procedure (chargebacks, device reset, cancel)?

- Source: `docs/operations.md`, `docs/licensing_service.md`
- Category: process
- Priority: P1
- Method: docs-only (write/extend SOP if missing)
- Status: answered

Answer:
- Abuse/chargeback response (as documented):
  - set license status to canceled
  - reset activations via `POST /license/reset` (requires `LICENSE_ADMIN_TOKEN`)
- Support intake + SLA:
  - support@tabulensis.com, 1 business day SLA

Evidence:
- `docs/operations.md` (operator essentials).
- `docs/licensing_service.md` (`/license/reset` + admin token).

Confidence: high (docs explicit).

## QA-0013: What are the minimum security recurring checks (dependency audit, secret scanning, SAST), and which tools are chosen?

- Source: `meta_methodology.md`, `docs/meta/prompts/deep_research_security_watch.md`, `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`
- Category: decision + process
- Priority: P1
- Method: operator decision + deep research capture + repo reality (tooling integration)
- Status: answered

Answer:
- Minimum recurring checks (weekly or pre-release):
  - Rust dependency audit: `cargo audit` (RustSec)
  - npm dependency audit: `npm audit --omit=dev` (under `tabulensis-api/`)
  - Secret scanning: `gitleaks detect`
  - Minimal SAST: `semgrep --config auto`
- Operator entrypoint:
  - `SECURITY_DAILY_CHECKLIST.md`
- Wrapper report script:
  - `bash scripts/security_audit.sh` (writes/updates `docs/meta/logs/ops/YYYY-MM-DD_security_audit.md`)

Evidence:
- Decision recorded: `docs/meta/results/decision_register.md` (`DR-0016`).
- Guardrails updated: `docs/meta/automation/guardrails.md`.
- Checklist: `SECURITY_DAILY_CHECKLIST.md`.
- Script: `scripts/security_audit.sh`.

Confidence: medium-high (tools chosen + documented; actual tool availability depends on local install/CI wiring).

## QA-0014: What is the minimal marketing measurement system (UTM scheme + metrics file) and where is it recorded?

- Source: `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (Section 8.4), `docs/meta/marketing_templates/README.md`
- Category: decision + process
- Priority: P1
- Method: operator decision; implement minimal file + script
- Status: answered

Answer:
- Canonical UTM scheme (recorded in `docs/meta/marketing_templates/README.md`):
  - Required: `utm_source`, `utm_medium`, `utm_campaign`
  - Optional: `utm_content`
  - Values: ASCII `[A-Za-z0-9_-]` (no spaces), short + stable
- UTM link generator:
  - `python3 scripts/utm.py --source <...> --medium <...> --campaign <...> --content <...> [--copy]`
- Metrics file:
  - `docs/meta/logs/marketing/metrics.csv` (append-only; one row per post/outreach event)

Evidence:
- Decision recorded: `docs/meta/results/decision_register.md` (`DR-0012`).
- UTM scheme doc: `docs/meta/marketing_templates/README.md`.
- Script: `scripts/utm.py`.
- Metrics file: `docs/meta/logs/marketing/metrics.csv`.

Confidence: high.

---

## Phase 4 Coverage: Per-Doc Question Generation (Skim-Level)

These entries exist to ensure the question bank “covers every doc” linked from `docs/index.md` (plus a few top-level guardrail docs).

Default status for skim-generated questions is `open` (answer later; prioritize P0/P1 first).

## QA-0015: Does `README.md` match the current shipped binaries, workflows, and naming?

- Source: `README.md` (top-level)
- Category: evidence + code-verify
- Priority: P1
- Method: docs + repo reality (`--help`, release workflow)
- Status: open

## QA-0016: Are `AGENTS.md` guardrails complete, and do they match actual repo tooling/scripts?

- Source: `AGENTS.md` (top-level)
- Category: process + evidence
- Priority: P1
- Method: docs + repo reality (scripts + workflows)
- Status: open

## QA-0017: Does `docs/cli.md` match the current CLI surface area and examples?

- Source: `docs/cli.md`
- Category: code-verify
- Priority: P1
- Method: repo reality (`excel-diff --help` / CLI entrypoints) + doc reconciliation
- Status: open

## QA-0018: Does `docs/config.md` match the current `DiffConfig` API and defaults?

- Source: `docs/config.md`
- Category: evidence + code-verify
- Priority: P2
- Method: repo reality (Rust types + CLI flags) + doc reconciliation
- Status: open

## QA-0019: What is the intended migration path in `docs/migration.md`, and is it still relevant?

- Source: `docs/migration.md`
- Category: evidence
- Priority: P2
- Method: docs-only (and check for referenced legacy APIs/paths)
- Status: open

## QA-0020: Does `docs/git.md` describe Git integration that actually exists today?

- Source: `docs/git.md`
- Category: code-verify
- Priority: P2
- Method: repo reality (CLI flags/commands; tests) + doc reconciliation
- Status: open

## QA-0021: Does `docs/desktop.md` reflect the canonical desktop build/run commands and env vars?

- Source: `docs/desktop.md`
- Category: code-verify
- Priority: P1
- Method: repo reality (cargo commands; scenario scripts) + doc reconciliation
- Status: open

## QA-0022: Is `product_roadmap.md` still the canonical roadmap, and what is the next measurable milestone?

- Source: `product_roadmap.md`
- Category: process
- Priority: P1
- Method: docs + operator decision (align on next milestone + signals)
- Status: open

## QA-0023: What parts of `meta_methodology.md` are aspirational vs implemented, and what is the minimum “daily loop”?

- Source: `meta_methodology.md`
- Category: process + evidence
- Priority: P1
- Method: docs + repo reality (scripts and paths) + reconciliation
- Status: open

## QA-0024: Which remaining items in `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` are true blockers (P0/P1) vs backlog?

- Source: `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`
- Category: process
- Priority: P1
- Method: checklist triage + dependency mapping
- Status: open

## QA-0025: Is `docs/meta/README.md` the canonical index for meta-docs, and does it capture privacy/redaction rules clearly?

- Source: `docs/meta/README.md`
- Category: process
- Priority: P1
- Method: docs-only + reconcile with decision register/logs readmes
- Status: open

## QA-0026: What is the intended rerun cadence for `docs/meta/strategy_audit_plan.md`, and what is the minimal repeatable automation?

- Source: `docs/meta/strategy_audit_plan.md`
- Category: process
- Priority: P2
- Method: operator decision + Phase 8 script wiring
- Status: open

## QA-0027: Does `docs/meta/prompts/README.md` correctly describe how prompts/results are created, stored, and reused?

- Source: `docs/meta/prompts/README.md`
- Category: process + code-verify
- Priority: P1
- Method: docs + repo reality (`scripts/deep_research_prompt.py`)
- Status: open

## QA-0028: Is the Pro payload spec in `docs/meta/prompts/pro_context_payload.md` implementable, and what remains to ship a generator?

- Source: `docs/meta/prompts/pro_context_payload.md`
- Category: code-verify + decision
- Priority: P1
- Method: repo reality (`docs/meta/prompts/generate_review_context.py`) + decision register (`DR-0007`)
- Status: open

## QA-0029: Should audio outputs under `docs/meta/audio/` be committed or always local-only, and what should be gitignored?

- Source: `docs/meta/audio/README.md`
- Category: decision + process
- Priority: P2
- Method: operator decision + `.gitignore` hygiene
- Status: open

## QA-0030: Is `docs/meta/today.example.md` sufficient to drive “daily open”, and is the redaction warning visible enough?

- Source: `docs/meta/today.example.md`
- Category: process
- Priority: P2
- Method: docs-only + operator feedback
- Status: open

## QA-0031: Are marketing templates + measurement in `docs/meta/marketing_templates/README.md` being used consistently?

- Source: `docs/meta/marketing_templates/README.md`
- Category: process
- Priority: P2
- Method: docs + logs reality (`docs/meta/logs/marketing/metrics.csv`)
- Status: open

## QA-0032: Do overnight-agent invariants in `docs/meta/automation/overnight_operator_agent_plan.md` match the implementation safety model?

- Source: `docs/meta/automation/overnight_operator_agent_plan.md`
- Category: code-verify
- Priority: P1
- Method: docs + repo reality (`scripts/overnight_agent.py`)
- Status: open

## QA-0033: Does `docs/meta/automation/overnight_agent_runbook.md` match the current CLI/config and output locations?

- Source: `docs/meta/automation/overnight_agent_runbook.md`
- Category: code-verify
- Priority: P1
- Method: repo reality (run `doctor`, `list-tasks`, `run-once --dry-run`)
- Status: open

## QA-0034: Is `docs/meta/automation/README.md` sufficient for a new operator to run automation safely and interpret outputs?

- Source: `docs/meta/automation/README.md`
- Category: process
- Priority: P2
- Method: docs-only + operator walkthrough
- Status: open

## QA-0035: Do the guardrails in `docs/meta/automation/guardrails.md` cover all high-risk actions (release, licensing, vendors)?

- Source: `docs/meta/automation/guardrails.md`
- Category: process
- Priority: P1
- Method: docs + repo reality (audit scripts/workflows for risky commands)
- Status: open

## QA-0036: What is the model-account strategy in `docs/meta/automation/model_accounts.md`, and what triggers creating a second account?

- Source: `docs/meta/automation/model_accounts.md`
- Category: decision
- Priority: P2
- Method: operator decision (`DR-0015`)
- Status: open

## QA-0037: Should `SECURITY_DAILY_CHECKLIST.md` be run in CI (scheduled) and how are findings tracked?

- Source: `SECURITY_DAILY_CHECKLIST.md`
- Category: process + automation
- Priority: P2
- Method: operator decision + automation wiring (`docs/meta/automation/tasks.yaml` or GitHub Actions)
- Status: open

## QA-0038: What is the minimum “ops dashboard” view implied by `docs/operations.md` and vendor docs?

- Source: `docs/operations.md`
- Category: process
- Priority: P1
- Method: docs + deep research capture (vendor APIs) + operator decision
- Status: open

## QA-0039: Is `docs/licensing_service.md` complete enough for production ops (deploy, monitor, debug, reset)?

- Source: `docs/licensing_service.md`
- Category: process + code-verify
- Priority: P1
- Method: docs + repo reality (`tabulensis-api/` + endpoints) + reconcile into SOP
- Status: open

## QA-0040: Which decisions in `STRIPE_WORKER_NEXT_STEPS.md` are now obsolete vs still blocking production readiness?

- Source: `STRIPE_WORKER_NEXT_STEPS.md`
- Category: decision + process
- Priority: P2
- Method: docs triage + repo reality (current Worker implementation)
- Status: open

## QA-0041: Does `RESEND_SETUP_CHECKLIST.md` match the current Resend integration paths and tests?

- Source: `RESEND_SETUP_CHECKLIST.md`
- Category: code-verify + vendor
- Priority: P1
- Method: repo reality (`tabulensis-api/` + endpoints) + vendor verification
- Status: open

## QA-0042: Is `docs/release_checklist.md` sufficient given the actual `.github/workflows/release.yml` pipeline?

- Source: `docs/release_checklist.md`
- Category: process + code-verify
- Priority: P1
- Method: compare checklist vs CI workflow; reconcile gaps
- Status: open

## QA-0043: Does `docs/release_readiness.md` match the real acceptance tests and determinism guarantees?

- Source: `docs/release_readiness.md`
- Category: process
- Priority: P2
- Method: docs + test coverage verification
- Status: open

## QA-0044: What is the minimal signing/notarization target for MVP, and what work is required to wire it into CI?

- Source: `docs/release_signing.md`
- Category: decision + process
- Priority: P1
- Method: operator decision + CI workflow design
- Status: open

## QA-0045: What updater mechanism is assumed in `docs/auto_update_strategy.md`, and what are the prerequisites (signing, channels)?

- Source: `docs/auto_update_strategy.md`
- Category: decision + process
- Priority: P2
- Method: docs + repo reality + operator decision
- Status: open

## QA-0046: What is the minimum installer UX per OS per `docs/installer_ux.md`, and what is shipping now?

- Source: `docs/installer_ux.md`
- Category: evidence + process
- Priority: P2
- Method: docs + repo reality (packaging scripts + release workflow)
- Status: open

## QA-0047: Are PR checklist items in `.github/pull_request_template.md` reflected by CI enforcement (required checks)?

- Source: `.github/pull_request_template.md`
- Category: process + code-verify
- Priority: P2
- Method: compare checklist to `.github/workflows/*` and branch protection
- Status: open

## QA-0048: How will `docs/desktop_tab_switch_latency_plan.md` be measured in CI or nightly runs, and what is the target p95?

- Source: `docs/desktop_tab_switch_latency_plan.md`
- Category: process + evidence
- Priority: P2
- Method: repo reality (UI scenarios/snapshots) + operator decision on metrics
- Status: open

## QA-0049: Is `docs/maintainers/architecture_debt_post_iteration_1.md` still accurate, and what is its critical path?

- Source: `docs/maintainers/architecture_debt_post_iteration_1.md`
- Category: process
- Priority: P2
- Method: docs-only + repo reality check for addressed items
- Status: open

## QA-0050: Does `docs/meta/prompts/daily_plan.md` reliably produce a small, dependency-aware “today plan”?

- Source: `docs/meta/prompts/daily_plan.md`
- Category: process
- Priority: P2
- Method: run prompt on current context and evaluate output quality
- Status: open

## QA-0051: Does `docs/meta/prompts/deep_research_synthesis.md` reliably produce actionable synthesis without hallucinated steps?

- Source: `docs/meta/prompts/deep_research_synthesis.md`
- Category: process
- Priority: P2
- Method: run prompt on current A/B outputs and review against evidence
- Status: open

## QA-0052: What parts of `docs/product_iterations/iteration_1_daily_driver_plan.md` are already done and what remains?

- Source: `docs/product_iterations/iteration_1_daily_driver_plan.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (grep for completed features/tests)
- Status: open

## QA-0053: Are Iteration 2 tasks in `docs/product_iterations/iteration_2_implementation_plan.md` aligned with current repo state and test gates?

- Source: `docs/product_iterations/iteration_2_implementation_plan.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (open items vs existing code)
- Status: open

## QA-0054: Which open decisions in `docs/product_iterations/iteration_2_readiness.md` block starting Iteration 2?

- Source: `docs/product_iterations/iteration_2_readiness.md`
- Category: decision
- Priority: P2
- Method: extract “Decide …” items + record in decision register if truly gating
- Status: open

## QA-0055: What is the minimal “real-world datasets” program in `docs/real_world_datasets_perf_plan.md`, and what is the next acquisition step?

- Source: `docs/real_world_datasets_perf_plan.md`
- Category: process
- Priority: P2
- Method: docs + operator decision (licensing/ethics; fixture manifests)
- Status: open

## QA-0056: What “commercialization plan” decisions in `docs/rust_docs/mvp_checklist.md` are still open, and where should they live canonically?

- Source: `docs/rust_docs/mvp_checklist.md`
- Category: decision
- Priority: P1
- Method: extract/triage + record top decisions in decision register
- Status: open

## QA-0057: Does `docs/rust_docs/mvp_fulfillment_log.md` reflect the real state of MVP readiness, and what are the next 10 closure items?

- Source: `docs/rust_docs/mvp_fulfillment_log.md`
- Category: process
- Priority: P2
- Method: docs-only + reconcile with current release pipeline and product state
- Status: open

## QA-0058: Which items in `tabulensis_launch_to_dos_from_our_chat.md` are now obsolete vs still blocking launch?

- Source: `tabulensis_launch_to_dos_from_our_chat.md`
- Category: process + decision
- Priority: P2
- Method: triage + mark N/A/complete where appropriate
- Status: open

## QA-0059: Do `docs/maintainers/entrypoints.md` and code entrypoints still match the repo layout and workflows?

- Source: `docs/maintainers/entrypoints.md`
- Category: code-verify
- Priority: P2
- Method: repo reality (file existence; main entrypoints)
- Status: open

## QA-0060: Does `docs/maintainers/fixtures.md` fully cover fixture generation, manifests, and `--clean` pitfalls?

- Source: `docs/maintainers/fixtures.md`
- Category: process + code-verify
- Priority: P2
- Method: docs + repo reality (`generate-fixtures` flags; manifests)
- Status: open

## QA-0061: Does `docs/maintainers/architecture.md` match the current module boundaries and responsibilities?

- Source: `docs/maintainers/architecture.md`
- Category: evidence
- Priority: P2
- Method: docs + repo reality (code tree)
- Status: open

## QA-0062: Does `fixtures/README.md` match current fixture tooling and manifests used in CI/perf?

- Source: `fixtures/README.md`
- Category: process + code-verify
- Priority: P2
- Method: docs + CI workflow reality
- Status: open

## QA-0063: Does `benchmarks/README.md` match the current perf harness commands and baselines?

- Source: `benchmarks/README.md`
- Category: process + code-verify
- Priority: P2
- Method: docs + repo reality (scripts under `scripts/`)
- Status: open

## QA-0064: Are desktop UI scenarios in `desktop/ui_scenarios/README.md` the canonical smoke set, and are they automated?

- Source: `desktop/ui_scenarios/README.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (scenario runner scripts)
- Status: open

## QA-0065: Does `desktop/ui_snapshots/README.md` describe a deterministic visual-regression workflow that can be run headless?

- Source: `desktop/ui_snapshots/README.md`
- Category: process + code-verify
- Priority: P2
- Method: docs + repo reality (`scripts/ui_capture.sh`, snapshot summary scripts)
- Status: open

## QA-0066: What is `docs/database_mode.md`’s intended use case, and what data-safety guarantees exist?

- Source: `docs/database_mode.md`
- Category: process + evidence
- Priority: P2
- Method: docs + repo reality (where DB files live; cleanup)
- Status: open

## QA-0067: Is `docs/faq.md` current and aligned with actual feature support and limitations?

- Source: `docs/faq.md`
- Category: evidence
- Priority: P2
- Method: docs + repo reality spot-check
- Status: open

## QA-0068: Does `docs/architecture.md` reflect the current system architecture, or is it aspirational?

- Source: `docs/architecture.md`
- Category: evidence
- Priority: P2
- Method: docs + repo reality (code layout)
- Status: open

## QA-0069: Do the perf policies in `docs/perf_playbook.md` match CI gates and local workflows?

- Source: `docs/perf_playbook.md`
- Category: process + code-verify
- Priority: P1
- Method: docs + repo reality (CI workflows; scripts)
- Status: open

## QA-0070: What are the top 10 test gaps in `docs/test_suite_excellence_plan.md`, and which are P0 for MVP confidence?

- Source: `docs/test_suite_excellence_plan.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (existing tests; failure history)
- Status: open

## QA-0071: How is OpenXML coverage tracked per `docs/openxml_coverage.md`, and what’s the next coverage milestone?

- Source: `docs/openxml_coverage.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (fixtures/tests)
- Status: open

## QA-0072: How is formula coverage tracked per `docs/formula_coverage.md`, and what’s the next coverage milestone?

- Source: `docs/formula_coverage.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (fixtures/tests)
- Status: open

## QA-0073: How is diff-op coverage tracked per `docs/diff_op_coverage.md`, and what’s the next coverage milestone?

- Source: `docs/diff_op_coverage.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (fixtures/tests)
- Status: open

## QA-0074: How is M parser coverage tracked per `docs/m_parser_coverage.md`, and what’s the next coverage milestone?

- Source: `docs/m_parser_coverage.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (fixtures/tests)
- Status: open

## QA-0075: Are custom-crate experiment docs in `docs/rust_docs/custom_crates/` being maintained, and what is the current recommendation?

- Source: `docs/rust_docs/custom_crates/README.md`
- Category: process
- Priority: P2
- Method: docs-only + reconcile with perf artifacts
- Status: open

## QA-0076: Does `docs/rust_docs/custom_crates/agentic_experiment_playbook.md` match current perf-cycle guardrails and tooling?

- Source: `docs/rust_docs/custom_crates/agentic_experiment_playbook.md`
- Category: process
- Priority: P2
- Method: docs-only + reconcile with `AGENTS.md`
- Status: open

## QA-0077: What is the intended milestone ordering in `docs/desktop_native_ui_ux_roadmap.md`, and what is measurable “done”?

- Source: `docs/desktop_native_ui_ux_roadmap.md`
- Category: process
- Priority: P2
- Method: docs + operator decision (metrics and acceptance)
- Status: open

## QA-0078: What are the top “perf UX” targets in `docs/desktop_perf_ux_improvement_plan.md`, and how will they be validated?

- Source: `docs/desktop_perf_ux_improvement_plan.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (UI scenarios; perf harness)
- Status: open

## QA-0079: Is `docs/ui_visual_regression.md` consistent with actual snapshot tooling and storage paths?

- Source: `docs/ui_visual_regression.md`
- Category: code-verify
- Priority: P2
- Method: docs + repo reality (scripts + snapshot dirs)
- Status: open

## QA-0080: Does `docs/ui_visual_regression_manual.md` match the current manual QA flow and target scenarios?

- Source: `docs/ui_visual_regression_manual.md`
- Category: process
- Priority: P2
- Method: docs-only + operator walkthrough
- Status: open

## QA-0081: Is `docs/ui_visual_regression_plan.md` still the canonical plan, and which items are now obsolete/complete?

- Source: `docs/ui_visual_regression_plan.md`
- Category: process
- Priority: P2
- Method: docs triage + repo reality (existing tooling)
- Status: open

## QA-0082: Are `docs/errors.md` and the actual error codes aligned (complete, stable, and discoverable)?

- Source: `docs/errors.md`
- Category: code-verify
- Priority: P2
- Method: repo reality (search error enum/constants) + doc reconciliation
- Status: open

## QA-0083: Are `docs/ui_guidelines.md` guidelines actionable and enforced (or referenced by checklists/tests)?

- Source: `docs/ui_guidelines.md`
- Category: process
- Priority: P2
- Method: docs + repo reality (lint/tests or checklist enforcement)
- Status: open
