# Automation Guardrails

This file exists to make automation failure-safe and boring.

Source of truth:
- `AGENTS.md`
- `docs/meta/automation/overnight_operator_agent_plan.md`

## Forbidden Actions

- Deploys (example: `wrangler deploy`).
- Secret rotation (example: `wrangler secret put`).
- Destructive git operations:
  - `git reset --hard`
  - `git push --force` / `--force-with-lease`

## Required Safety Properties

- Primary working tree is never mutated by automation.
  - All code changes happen in isolated git worktrees/branches.
- Every automation run produces an operator-readable report:
  - short summary in `docs/meta/logs/ops/`
  - links to any branches/worktrees/artifacts
- Validation is conditional and policy-driven:
  - Follow perf and formatting scope rules in `AGENTS.md`.

## Do Not Proceed If

- Worktree is dirty with unrelated changes (risk of accidental capture or scope explosion).
- Required local tools are missing (Codex CLI / Python deps / Rust toolchain).
- Baseline tests fail (unless the run is explicitly scoped to diagnose/fix the failure).

## Redaction Rules

- Never print or commit secrets, tokens, API keys, or customer data.
- If vendor snapshots are captured, commit only sanitized summaries unless explicitly intended.

## Security Tooling (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0016`).

Minimum recurring checks (run weekly or before releases):
- Rust dependency audit: `cargo audit` (RustSec)
- npm dependency audit: `npm audit --omit=dev` (under `tabulensis-api/`)
- Secret scanning: `gitleaks detect`
- Minimal SAST: `semgrep --config auto`

Recommended wrapper:
- `bash scripts/security_audit.sh` (writes/updates `docs/meta/logs/ops/YYYY-MM-DD_security_audit.md`)
