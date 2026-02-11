# Overnight Operator Agent Plan

This document is the full implementation plan for a long-running, non-interactive Codex-powered process that operates Tabulensis overnight (roughly 7PM–5AM), continuously pulling work from the repo, implementing it in isolated branches, running the appropriate validation (tests and perf), updating documentation, and producing an "Executive Summary" you can review in the morning.

Primary references:
- Idea sketch: `long_running_codex_agent.md`
- Operating intent: `meta_methodology.md`
- Repository guardrails/policies: `AGENTS.md`

Non-goals:
- This document does not implement the agent. It defines the intended design, invariants, configuration schema, and a concrete repo-specific wiring plan.

## Design Principles (Non-Negotiable Invariants)

1. Make it boring and auditable:
   - Every action is logged, every output is written to a file, and every change is committed on an isolated branch.
2. Make it restart-safe:
   - Crashes must resume without guessing what happened.
3. Make rollback trivial:
   - One substantial work item equals one branch (recommended).
   - The whole overnight run happens in a dedicated session worktree.
4. Make heavy validation conditional:
   - Do not run full perf cycles unless policy requires it (and enforce retention rules).
5. Make doc sprawl hard:
   - Any new doc must be indexed from `docs/index.md` (or it does not get created).
6. Never do dangerous operations:
   - No deploys.
   - No secret rotation.
   - No destructive git operations (`reset --hard`, force push, etc.).

## System Overview

Two components:
1. Watchdog (this repo: `scripts/overnight_agent.py`)
   - Single-instance lock (ensures only one supervisor is active).
   - Creates/ensures:
     - a dedicated *session worktree* (so the primary working tree stays untouched)
     - an *ops journal worktree* (so logs can be committed without conflicts)
   - Writes a durable session state file with the deadline (`tmp/overnight_agent/session.json`).
   - Starts a single Codex session via `codex exec` (pinned model).
   - If the Codex process exits early, resumes the same Codex thread via `codex exec resume --last`.
2. Codex session (the actual “agent”)
   - Self-directed: doc audit -> strategy -> execution loop until time budget expires.
   - Uses branches + commits for rollback.
   - Appends operator-facing summaries to the ops journal throughout the run.

Recommended implementation language: Python (pragmatic, cross-platform, aligns with existing `scripts/*.py`).

## Filesystem Layout (Proposed)

Local-only runtime (not committed):
- `tmp/overnight_agent/`
  - `session.json` (durable session state: deadline, worktree paths, restart counters)
  - `codex_home/` (Codex CLI sessions/history/auth; local-only)
  - `runs/<run_id>/` (watchdog logs: Codex stdout/stderr, restart history)

Repo-committed outputs (sanitized; no secrets):
- `docs/meta/logs/ops/`
  - `executive_summary.log` (append-only)
  - `<run_id>_strategy.md` (recommended per-run plan; written by the agent)
  - `<date>_questions_for_operator.md` (blocked decisions)

Operational note:
- To avoid merge conflicts (many per-task branches all appending to the same journal), keep these log/report commits on a dedicated ops journal branch (for example `overnight/ops-journal`) with its own worktree.

Configuration:
- `docs/meta/automation/overnight_agent.yaml` (repo-specific config; portable schema)

## Core Operational Loop

The watchdog starts a single Codex session and then mostly stays out of the way.

Codex session boot sequence (always first):
1. Documentation audit:
   - Scan for waste/contradictions (start at `docs/index.md` and `meta_methodology.md`).
   - Prefer low-risk cleanup: fix broken links, consolidate duplicates only when clearly safe.
   - Run `python3 scripts/docs_integrity.py` (and `--check-links` if fast) and fix issues.
2. Strategy plan:
   - Read `product_roadmap.md`, `docs/index.md`, and any relevant operating docs.
   - Decide the next 1-3 most valuable tasks for the available time budget.
   - Write a short plan artifact to `docs/meta/logs/ops/<run_id>_strategy.md` (commit on ops journal branch).

Codex session execution loop (repeat until time budget is up):
- Pick the next most profitable task.
- Create a new `overnight/*` branch and commit changes (rollback-first).
- Run appropriate validation (tests/perf) per `AGENTS.md`.
- Update docs where needed (and keep docs sprawl low).
- Append an executive summary line and any operator questions to the ops journal.

Wrap-up near the deadline:
- Stop starting new risky work; finish in-flight tasks or rollback cleanly.
- Ensure logs are readable and that questions for the operator are written down.

## Durable Session State (Restart Safe)

The watchdog is intentionally simple and only needs a small amount of durable state:

- `tmp/overnight_agent/session.json`: deadline + worktree paths + restart counters.
- `tmp/overnight_agent/codex_home/`: Codex CLI’s persisted session/thread state.

Restart behavior:
- If the watchdog crashes and is restarted while the deadline has not passed, it reuses the same
  worktrees and resumes the most recent Codex `exec` thread (`codex exec resume --last`).
- If the deadline has passed (or the operator uses `--no-resume`), it starts a new session and
  writes a fresh `session.json`.

## Git Strategy (Rollback First)

Hard rules:
- The agent never mutates your primary working tree.
- All changes happen in a dedicated *session* git worktree for the whole run.
- For rollback, the agent should create one `overnight/*` branch per substantial work item (recommended),
  and commit as it goes.

Branch naming:
- Session branch (worktree base): `overnight/session_<run_id>`
- Work item branches: `overnight/YYYY-MM-DD_HHMM_<slug>`

Commit structure (default):
1. `feat|perf|fix: <task summary>` (code/config changes)
2. `docs: refresh + update operating docs` (docs-only changes)
3. `perf: perf cycle artifacts` (only if required by policy)

Cleanup:
- Default: keep the worktree for review (morning inspection).
- Optional: an explicit retention policy can remove old worktrees after N days, but only those created by the agent.

## Work Discovery (Self-Directed)

The overnight agent does **not** consume a deterministic task queue.

Instead, the Codex session should continuously discover the next most profitable work by scanning
repo reality and operator intent, in roughly this order:

- Documentation quality:
  - contradictions, redundancy, broken links, stale instructions
  - gaps in runbooks/guardrails that cause operator time waste
- Product leverage:
  - `product_roadmap.md` and iteration plans under `docs/product_iterations/`
  - UX/feature work that makes the product meaningfully better (small, shippable slices)
- Operational leverage:
  - `meta_methodology.md` and unfinished checklists indexed from `docs/index.md`
  - automation improvements that reduce future toil

Decision-heavy work:
- If progress requires operator judgment, the agent should stop that thread of work and write a
  concrete question to `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md` (ops journal branch),
  then move to the next best task.

## Planning

Planning happens inside the Codex session.

Recommended planning artifacts:
- At run start, write `docs/meta/logs/ops/<run_id>_strategy.md` on the ops journal branch with:
  - time budget + deadline
  - top 1-3 tasks for the session (with rationale)
  - explicit “stop conditions” (when to defer and ask the operator)
- Before each substantial work item, write a short plan (can be in the branch’s first commit message or a short markdown note).

If progress requires operator judgment, the agent should not guess:
- write the exact question(s) to `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md`
- then move to the next best task

## Implementation

Implementation happens inside the Codex session in the dedicated session worktree.

Guidelines:
- Keep changes small and auditable (prefer multiple small branches over one sprawling branch).
- Commit frequently with clear messages.
- Run validation appropriate to the touched code (follow `AGENTS.md` perf policy).
- Avoid wide-scope formatting/refactors overnight.
- Keep docs sprawl low: if you create a new operating doc, link it from `docs/index.md`.
- Never do forbidden operations (deploys, secret rotation, destructive git).

## Validation and Perf Policy (Repo-Aware)

This repo has explicit perf policy in `AGENTS.md`. The agent must follow it.

Validation is suite-driven with triggers:
- Always run:
  - targeted formatting (if Rust touched)
  - doc index refresh (`python3 scripts/update_docs_index_checklists.py`)
  - guardrails on staged set (line endings, change scope, perf cycle scope when relevant)

- Conditionally run (based on touched paths):
  - `cd tabulensis-api && npm test -- --run` when `tabulensis-api/**` changed
  - Rust tests (or `python scripts/dev_test.py` if you prefer) when Rust paths changed
  - perf quick/gate suites when perf-sensitive paths changed
  - full perf cycle only on major perf-risk triggers (see below)

### Repo-Specific Gotcha 1: Perf Cycle Retention

Problem:
- A long-running agent can accidentally generate multiple perf cycles and violate retention rules.

Design:
- Detect major perf-risk changes from *predicted touched paths before implementation*.
- If major perf-risk:
  - Run exactly one `perf_cycle.py pre`.
  - Implement.
  - Run exactly one matching `perf_cycle.py post --cycle <same_id>`.
  - Stage and commit exactly one cycle directory for the iteration.
- If a cycle directory already exists in that branch/worktree:
  - Fail fast and ask operator (do not create another cycle).

### Repo-Specific Gotcha 2: Formatting Churn

Problem:
- `cargo fmt --all` can create workspace-wide churn, which is explicitly discouraged.

Design:
- Formatting is always narrow:
  - Rust: `python3 scripts/safe_rustfmt.py --worktree` (changed-file scoped)
- If change scope guard flags wide changes:
  - Stop and ask operator; do not "fix" by formatting more widely.

### Repo-Specific Gotcha 3: Documentation Sprawl

Problem:
- Automation tends to create redundant docs.

Design:
- Before creating a new `.md`, run a similarity search (simple grep keywords + filename heuristics).
- If still needed:
  - Create doc only in a designated folder (e.g., `docs/meta/automation/`).
  - Add it to `docs/index.md` under the correct section.
  - Refresh `docs/index.md` checklist auto-index if checkboxes changed anywhere.

## Configuration: Portable YAML That Still Works

The watchdog config is intentionally small:
- The “task selection” logic is inside the long-running Codex session, not a deterministic queue.
- The YAML mostly defines *paths* and *Codex session settings* (worktrees, ops journal locations, CODEX_HOME).

### Example YAML (excel_diff wiring, schema pattern intended to be reusable)

```yaml
version: 2

repo:
  base_branch: main
  session_worktree_root: ../excel_diff_worktrees/overnight_session
  ops_journal_branch: overnight/ops-journal
  ops_journal_worktree: ../excel_diff_worktrees/overnight_ops_journal
  run_root: tmp/overnight_agent
  session_state: tmp/overnight_agent/session.json
  exec_summary_log: docs/meta/logs/ops/executive_summary.log
  reports_dir: docs/meta/logs/ops
  git_identity:
    name: Overnight Agent
    email: overnight-agent@localhost

codex:
  codex_bin: codex
  codex_home: tmp/overnight_agent/codex_home
  model: gpt-5.3-codex
  model_reasoning_effort: xhigh
  full_auto: true
  sandbox: workspace-write
```

Portability strategy:
- In other repos, update the `repo.*` paths and branch names to match local conventions.
- Keep the model pinned for consistency.

## Observability (So It Can Fail Safely)

Two output layers:
1. Human layer (morning review):
   - `docs/meta/logs/ops/executive_summary.log` (append-only lines)
   - `docs/meta/logs/ops/<run_id>_strategy.md` (per-run plan and checkpoints; recommended)
   - `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md` (blocked decisions)
2. Machine layer (for resuming and debugging):
   - `tmp/overnight_agent/session.json` (deadline + worktree paths)
   - `tmp/overnight_agent/runs/<run_id>/codex_start.log` / `codex_resume.log` (Codex stdout/stderr)
   - `tmp/overnight_agent/codex_home/` (Codex persisted session/thread state; local-only)

Executive summary line format (recommended):
- `<ISO_UTC_TIMESTAMP> <branch> <commit_sha_or_n/a> <1-3 sentence summary>`

## Failure Modes and Recovery

Crashes:
- Watchdog restarts the Codex process.
- Watchdog resumes the previous Codex `exec` thread (`codex exec resume --last`) when possible.
- Worktrees are reused and the deadline is read from `tmp/overnight_agent/session.json`.

Test failures:
- The agent should attempt reasonable fixes.
- If still failing, it should:
  - record what failed in the ops journal (command + error excerpt)
  - leave the branch/worktree intact for morning review
  - switch to the next best task

Perf cycle failures:
- Do not start another cycle on the same branch without explicitly calling it out.
- Record the failure and move on (or stop if it is blocking the run).

Doc/guardrail failures:
- Treat as high priority (they directly impact operator efficiency and repo hygiene).

## Security and Privacy Model

Rules:
- Never print secrets to stdout.
- Never write secrets into repo-committed logs.
- Any vendor "snapshot" automation must redact tokens/keys.
- LLM prompts must not include secrets (use placeholders; do not paste `.env`).

## Implementation Roadmap (Future Improvements)

The core design is intentionally minimal: a watchdog + one self-sustaining Codex session.

Potential follow-ups:
- Stronger signal handling (Ctrl+C should always terminate the Codex process cleanly).
- Resume fallback: if `codex exec resume --last` fails, start a fresh session with the same deadline.
- Optional “heartbeat” file that the agent updates periodically so the watchdog can detect hangs.
- Optional helper scripts to make ops-journal appends/commits deterministic.

Acceptance criteria for "v1 robust":
- Runs for N hours with **one** Codex session (restarted only if it exits early).
- Can crash/restart and resume the Codex thread when possible.
- Does not touch the primary working tree (uses the session worktree).
- Uses branches + commits for rollback.
- Appends operator-facing summaries throughout the run.
- Always uses `gpt-5.3-codex` with reasoning effort `xhigh`.
