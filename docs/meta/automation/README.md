# Automation (`docs/meta/automation/`)

This directory contains operator-facing documentation and configuration for
automation and agentic workflows (especially the overnight operator agent).

Canonical docs entrypoint: `docs/index.md`.

## What Lives Here

Overnight operator agent (implemented):
- Plan/design: `docs/meta/automation/overnight_operator_agent_plan.md`
- Runbook: `docs/meta/automation/overnight_agent_runbook.md`
- Config: `docs/meta/automation/overnight_agent.yaml`
- Script: `scripts/overnight_agent.py`

Other automation docs (scaffolding / future work):
- Guardrails: `docs/meta/automation/guardrails.md`
- Model accounts + capacity planning: `docs/meta/automation/model_accounts.md`
- Windows Task Scheduler example (chosen scheduler): `docs/meta/automation/task_scheduler_example.md`

## Safety Model (Non-Negotiable)

Source of truth:
- `AGENTS.md`
- `docs/meta/automation/overnight_operator_agent_plan.md`

Hard rules:
- No deploys (`wrangler deploy` forbidden).
- No secret rotation (`wrangler secret put` forbidden).
- No destructive git operations (`git reset --hard`, force push, etc.).
- Never mutate the primary working tree; automation must use isolated git worktrees/branches.

## Output Locations (Canonical)

Decision: see `docs/meta/results/decision_register.md` (`DR-0008`).

Defaults:
- Raw runtime state/logs: `tmp/<automation_name>/...` (local-only; gitignored via `tmp/`).
- Sanitized operator-facing reports/journals:
  - Ops logs: `docs/meta/logs/ops/`
  - Long-form artifacts: `docs/meta/results/`

Overnight agent specifics (see runbook):
- Raw runtime: `tmp/overnight_agent/`
- Ops journal branch: `overnight/ops-journal`
- Ops journal files:
  - `docs/meta/logs/ops/executive_summary.log`
  - `docs/meta/logs/ops/<run_id>_report.md`
  - `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md`

### Report Filenames (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0009`).

For new automation outputs that are intended for humans to scan quickly, prefer:
- `docs/meta/logs/ops/YYYY-MM-DD_HHMMSS_<task>_<agent>.md`

Rules:
- `<task>` / `<agent>` are ASCII `lower_snake_case`.
- Each report should include a short run header block inside the file:
  - run tag
  - start/end timestamps
  - git branch + commit
  - environment (OS/WSL)
  - internal `run_id` if the system has one

Note:
- The overnight agent’s current ops journal uses `run_id`-based report names (`<run_id>_report.md`). That is acceptable as long as the run header inside the file is complete.

## Worktrees (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0018`).

Automation (and manual agentic work) should run in dedicated git worktrees rather than the primary working tree.

Helper script:
- Create/ensure a worktree: `scripts/agent_worktree.sh <name>`
  - Creates `../excel_diff_worktrees/<name>` on branch `agent_wt/<name>` (default base branch: `main`).
  - Prints the worktree path.
- Cleanup old agent worktrees: `scripts/agent_worktree.sh --cleanup`
  - Only targets worktrees on branches under `agent_wt/*`.
  - Default retention: older than 30 days (override with `--older-than-days N`).
  - Skips dirty worktrees unless `--force` is provided.

## Execution Environment (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0010`).

For now, automation is intended to run on the operator's local workstation only.

Implications:
- Prefer WSL/Linux/macOS terminals where repo tooling already works.
- Keep secrets local (avoid duplicating keys across machines until the safety model is mature).
- When scheduling is implemented (`DR-0011`), the scheduler should be configured on the same local workstation.

## What Counts As "Safe" Automated Action

Safe by default:
- Read-only analysis (docs/code scanning, generating reports, proposing plans).
- Creating new logs under `docs/meta/logs/**` and results under `docs/meta/results/**` (no secrets).
- Creating isolated branches/worktrees and making small, scoped, reversible changes.
- Running tests/perf checks consistent with repo policy.

Requires human review:
- Any change that touches release/signing, license enforcement, billing, or production infra.
- Any change requiring new credentials or vendor secrets.
- Wide-scope formatting/refactors (blast-radius guardrails should block these by default).

## Failure Recording

When automation fails:
- Write a short ops log under `docs/meta/logs/ops/` describing:
  - what was attempted
  - what failed (exact command + error excerpt)
  - what was skipped
  - next manual step
- If the failure blocks progress due to a human decision, write/append to:
  - `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md` (overnight agent convention)
  - and record the decision in `docs/meta/results/decision_register.md`.

## Docs Integrity Checks (Repeatable)

Recommended weekly (and after doc changes):

```bash
python3 scripts/docs_integrity.py
python3 scripts/docs_integrity.py --check-links
python3 scripts/docs_integrity.py --decision-gates --gate-limit 200
```

This catches:
- broken local links in `docs/index.md` (and optionally the linked corpus)
- checklist-index drift (scan vs committed `docs/index.md` block)
- decision-gate hotspots to triage into the operator queue

## Scheduling (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0011`).

Chosen scheduler:
- Windows Task Scheduler (invoking WSL).

Example configuration:
- `docs/meta/automation/task_scheduler_example.md`

Note:
- Manual invocation is still supported and is the safest default when you are actively changing automation internals.
