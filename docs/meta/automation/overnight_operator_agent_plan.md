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
   - One iteration equals one branch equals one worktree.
4. Make heavy validation conditional:
   - Do not run full perf cycles unless policy requires it (and enforce retention rules).
5. Make doc sprawl hard:
   - Any new doc must be indexed from `docs/index.md` (or it does not get created).
6. Never do dangerous operations:
   - No deploys.
   - No secret rotation.
   - No destructive git operations (`reset --hard`, force push, etc.).

## System Overview

Two processes:
1. Supervisor
   - Single-instance lock
   - Starts Runner
   - Restarts Runner on crash (with backoff)
   - Enforces time budget/window
   - Rotates logs
2. Runner
   - Deterministic state machine with durable state
   - Creates and uses worktrees and branches
   - Selects tasks, plans, implements, validates, commits, reports

Recommended implementation language: Python (pragmatic, cross-platform, aligns with existing `scripts/*.py`).

## Filesystem Layout (Proposed)

Local-only runtime (not committed):
- `tmp/overnight_agent/`
  - `state.sqlite3` (durable state store)
  - `state.json` (human-readable mirror)
  - `runs/<run_id>/` (raw prompts/responses/logs, stdout/stderr, timing)

Repo-committed outputs (sanitized; no secrets):
- `docs/meta/logs/ops/`
  - `executive_summary.log` (append-only)
  - `<run_id>_report.md` (per iteration)
  - `<date>_questions_for_operator.md` (blocked decisions)

Operational note:
- To avoid merge conflicts (many per-task branches all appending to the same journal), keep these log/report commits on a dedicated ops journal branch (for example `overnight/ops-journal`) with its own worktree.

Configuration:
- `docs/meta/automation/overnight_agent.yaml` (repo-specific config; portable schema)

## Core Operational Loop

Each iteration is a transaction:
1. Choose a task (deterministically).
2. Create isolated worktree + branch.
3. Plan the change (LLM plan artifact).
4. Implement (patch-by-patch).
5. Run targeted formatting.
6. Run the right validation suites (tests/perf) based on triggers.
7. Refresh docs/index surfaces and checklist indexes.
8. Run guardrails (line endings, change scope, perf retention).
9. Commit changes (structured commits).
10. Write a report + executive summary line.
11. Repeat until time budget exhausted.

The "brainstorming loop" becomes its own iteration type that produces *documents and queues* (no code changes unless explicitly permitted in config).

## Durable State Machine (Restart Safe)

Runner is a state machine that persists state after every phase. On crash/restart, it loads the last state and resumes the next idempotent step.

Recommended phases:
1. `IDLE` (waiting / checking time window)
2. `ACQUIRE_TASK` (load and score tasks)
3. `PLAN` (LLM planning + predicted touched paths)
4. `WORKTREE_CREATE` (git worktree + branch)
5. `PRE_VALIDATE` (only for major-perf-risk: perf pre baseline)
6. `IMPLEMENT` (apply patches, run micro-checks as needed)
7. `FORMAT` (targeted formatting only)
8. `TEST` (suite selection via triggers)
9. `PERF_POST` (only if pre baseline ran)
10. `DOCS_REFRESH` (update doc indexes)
11. `GUARDRAILS` (scope/line endings/perf cycle scope)
12. `COMMIT` (structured commits)
13. `REPORT` (exec summary + per-run report)
14. `CLEANUP` (optional: remove worktree / keep for inspection)
15. `DONE`

State persistence:
- SQLite `iterations` table tracks (run_id, task_id, phase, timestamps, worktree_path, branch, base_commit, last_good_commit, artifacts).
- `state.json` mirrors the active run (for quick inspection without SQLite tooling).

## Git Strategy (Rollback First)

Hard rules:
- The agent never mutates your primary working tree.
- All changes happen in a per-iteration git worktree.
- One iteration produces exactly one branch.

Branch naming:
- `overnight/YYYY-MM-DD_HHMM_<slug>`

Commit structure (default):
1. `feat|perf|fix: <task summary>` (code/config changes)
2. `docs: refresh + update operating docs` (docs-only changes)
3. `perf: perf cycle artifacts` (only if required by policy)

Cleanup:
- Default: keep the worktree for review (morning inspection).
- Optional: an explicit retention policy can remove old worktrees after N days, but only those created by the agent.

## Task Ingestion (Deterministic Inputs)

Task sources (ordered):
1. `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (checkbox tasks)
2. Unfinished checklists from `docs/index.md` auto-index block
3. `todo.md` (ideas/backlog; lowest priority)
4. Optional: `docs/meta/automation/manual_queue.md` (explicit operator queue)
5. Optional: GitHub issues (only if configured)

Task normalization:
- Record (source_path, line_number, raw_text, tags, created_at, last_attempted_at, status).
- De-duplicate identical tasks across sources.

Task scoring (configurable):
- Prefer tasks that unlock other tasks (scaffolding).
- Prefer low blast radius by default.
- Prefer tasks with explicit acceptance criteria.
- Penalize tasks requiring operator decisions late at night (route to questions queue).

## Planning (LLM) Phase

The planning phase must output a structured plan artifact (JSON or YAML), not prose only.

LLM invocation options (choose via config):
- Codex CLI non-interactive sessions (`codex exec`) for planning/patch generation (no `OPENAI_API_KEY` required by the runner).
- Direct OpenAI HTTP API calls for planning/patch generation (requires `OPENAI_API_KEY`).

Required plan fields:
- goal
- proposed changes (files + descriptions)
- predicted touched paths (glob/paths)
- risk class:
  - `docs_only`
  - `minor`
  - `major_perf_risk`
  - `wide_scope`
  - `security_risk`
  - `decision_required`
- validation plan (which suites and why)
- stop conditions (when to stop and ask operator)

If the plan is `decision_required`, the runner does not implement; it writes a question doc and moves to the next task.

## Implementation Phase (Patch-Based, Audited)

Mechanics:
- The runner requests patches from the LLM in small chunks.
- The runner applies patches and records:
  - patch text
  - files changed
  - command outputs for follow-up checks

Retries:
- For each failed phase (tests/perf/guardrails), allow N fix attempts with increasingly constrained prompts.

Safety:
- Enforce forbidden command patterns (configured).
- Enforce "no new doc without indexing" policy.

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

The YAML must balance portability with usefulness:
- Portability: runner ships with built-in "recipes" (cargo test, npm test, python script, shell).
- Usefulness: repo YAML binds those recipes to real commands, and declares triggers/policies.

Key concepts:
- `recipes`: typed command invocations (shell, python, cargo, npm)
- `suites`: ordered sets of recipe calls
- `triggers`: glob patterns -> suites
- `policy`: forbidden commands, change scope thresholds, perf retention rules
- `pipeline`: orchestration rules (always/when-change/when-major)

### Example YAML (excel_diff wiring, schema pattern intended to be reusable)

```yaml
version: 1

repo:
  name: excel_diff
  base_branch: main
  worktree_root: ../excel_diff_worktrees/overnight
  run_root: tmp/overnight_agent
  exec_summary_log: docs/meta/logs/ops/executive_summary.log

policy:
  forbid_commands_regex:
    - 'git\\s+reset\\s+--hard'
    - 'git\\s+push\\s+--force'
    - 'wrangler\\s+deploy'
  max_changed_files_soft: 40
  max_changed_files_hard: 120

docs:
  index_file: docs/index.md
  checklist_index_refresh:
    recipe: python
    cwd: .
    cmd: ["python3", "scripts/update_docs_index_checklists.py"]

suites:
  guardrails_staged:
    - { recipe: python, cmd: ["python3", "scripts/check_line_endings.py", "--staged"] }
    - { recipe: python, cmd: ["python3", "scripts/check_change_scope.py", "--staged"] }
    - { recipe: python, cmd: ["python3", "scripts/check_perf_cycle_scope.py", "--staged"], when: "perf_artifacts_staged" }

  fmt_rust:
    - { recipe: python, cmd: ["python3", "scripts/safe_rustfmt.py", "--worktree"] }

  tests_rust:
    - { recipe: shell, cmd: ["bash", "-lc", "cargo test"] }

  tests_worker:
    - { recipe: shell, cwd: tabulensis-api, cmd: ["bash", "-lc", "npm test -- --run"] }

  perf_quick:
    - { recipe: shell, cmd: ["bash", "-lc",
        "python3 scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv"
      ] }

  perf_cycle_full:
    pre:
      - { recipe: python, cmd: ["python3", "scripts/perf_cycle.py", "pre", "--cycle", "{{cycle_id}}"] }
    post:
      - { recipe: python, cmd: ["python3", "scripts/perf_cycle.py", "post", "--cycle", "{{cycle_id}}"] }

triggers:
  worker_changed:
    any_paths: ["tabulensis-api/**"]
  major_perf_risk:
    any_paths:
      - "core/src/**"
      - "desktop/backend/src/diff_runner.rs"
      - "desktop/backend/src/store/**"
      - "ui_payload/src/**"
      - "Cargo.toml"
      - "Cargo.lock"
      - "rust-toolchain.toml"
  perf_sensitive:
    any_paths: ["core/src/**", "ui_payload/src/**"]

pipeline:
  always:
    - docs.checklist_index_refresh
    - guardrails_staged
  on_change:
    - when: worker_changed
      run: [tests_worker]
    - when: perf_sensitive
      run: [perf_quick]
  on_major_perf_risk:
    run_full_perf_cycle: true
    run_suites: [perf_cycle_full]
```

Portability strategy:
- In other repos, only `suites` commands and `triggers` change.
- The runner should ship with a "starter generator" that can propose YAML by scanning for common markers:
  - `Cargo.toml` -> suggest `cargo test`, safe rustfmt
  - `package.json` -> suggest `npm test`
  - `pyproject.toml` -> suggest `pytest`
  - etc.

## Observability (So It Can Fail Safely)

Two output layers:
1. Human layer (morning review):
   - `docs/meta/logs/ops/executive_summary.log` (append-only lines)
   - `docs/meta/logs/ops/<run_id>_report.md` (full details)
2. Machine layer (for resuming and debugging):
   - `tmp/overnight_agent/runs/<run_id>/` (raw logs, prompts, outputs)

Executive summary line format (recommended):
- `<timestamp> <run_id> <branch> <task_slug> phase=<phase> result=<ok|failed|blocked> tests=<...> perf=<...> msg="<1-3 sentences>"`

## Failure Modes and Recovery

Crashes:
- Supervisor restarts Runner.
- Runner resumes based on persisted state and verifies invariants:
  - worktree exists and points at expected branch
  - branch exists
  - base commit still available

Test failures:
- Retry fix up to N times.
- If still failing:
  - write report
  - leave branch/worktree intact
  - proceed to next task (or stop if configured)

Perf cycle failures:
- Do not start another cycle in the same iteration.
- Write report and stop iteration.

Doc/guardrail failures:
- Treat as hard failures (because they break the methodology’s discoverability and churn discipline).

## Security and Privacy Model

Rules:
- Never print secrets to stdout.
- Never write secrets into repo-committed logs.
- Any vendor "snapshot" automation must redact tokens/keys.
- LLM prompts must not include secrets (use placeholders; do not paste `.env`).

## Implementation Roadmap (Full Version)

Suggested build order:
1. Config schema and loader (YAML + defaults)
2. Supervisor (lock, restarts, time window)
3. Runner skeleton + SQLite state store
4. Git/worktree manager
5. Task ingestion (checkbox + docs/index auto-index)
6. Task scoring/selection + decision routing
7. LLM planning + structured plan artifact
8. Patch application + change tracking
9. Validation runner (suites + triggers)
10. Perf cycle orchestration + retention enforcement
11. Docs refresh + anti-doc-sprawl contract
12. Reporting layer (exec summary + per-run markdown report)
13. Hardening and portability (starter YAML generator for other repos)

Acceptance criteria for "v1 robust":
- Can run for N hours and produce multiple independent branches without touching the main working tree.
- Can crash/restart and resume without duplicating side effects (idempotent state machine).
- Enforces perf policy and retention rules on major perf-risk changes.
- Avoids wide-scope formatting churn by default.
- Produces readable morning summaries and per-run reports with exact commands and artifact paths.
