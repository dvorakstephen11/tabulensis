# Overnight Agent Runbook

This repo includes a long-running “Overnight Operator Agent” implementation:

- Script: `scripts/overnight_agent.py`
- Config: `docs/meta/automation/overnight_agent.yaml`
- Design/architecture: `docs/meta/automation/overnight_operator_agent_plan.md`

## Safety Model (What It Will Not Do)

- It will not deploy (`wrangler deploy` is forbidden).
- It will not rotate secrets (`wrangler secret put` is forbidden).
- It will not run destructive git operations (`git reset --hard`, force push).
- It will not modify your primary working tree (all code changes happen in git worktrees).

## Prereqs

Execution environment:
- Intended to run on the operator's local workstation (decision: `docs/meta/results/decision_register.md` `DR-0010`).

Codex CLI:
- Install/configure Codex CLI so `codex exec` works non-interactively.
- The overnight agent pins the model to `gpt-5.3-codex` with reasoning effort `xhigh` (enforced in code + config).

## Quick Checks

```bash
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml doctor
```

If a session is active, check remaining time:

```bash
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml time-remaining
```

Ops summary helper (appends + commits 1 executive-summary line on the ops journal branch; useful for the agent itself too):

```bash
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml ops-log \
  --branch overnight/example_branch \
  --commit n/a \
  --message "Did X and Y; next step is Z."
```

## Start An Overnight Run

Run for 10 hours, resuming the previous Codex session after crashes/restarts when possible:

```bash
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml supervise --hours 10
```

Stop: `Ctrl+C` (the watchdog will terminate the Codex process).

## Scheduling

Scheduler decision: `docs/meta/results/decision_register.md` `DR-0011`.

Windows Task Scheduler (WSL invocation) example:
- `docs/meta/automation/task_scheduler_example.md`

## Where Outputs Go

1. Session branches + worktrees:
- Branches: `overnight/*`
- Session worktrees: `../excel_diff_worktrees/overnight_session/` (configurable)
- Ops journal worktree: `../excel_diff_worktrees/overnight_ops_journal/` (configurable)

2. Raw run artifacts (local-only):
- `tmp/overnight_agent/runs/<run_id>/**`
- `tmp/overnight_agent/session.json`
- `tmp/overnight_agent/codex_home/` (Codex CLI sessions/history/auth; needed for `resume --last`)
- `tmp/overnight_agent/runs/<run_id>/codex_start.log`
- `tmp/overnight_agent/runs/<run_id>/codex_resume.log`

3. Ops journal (committed on a dedicated branch):
- Branch: `overnight/ops-journal`
- Files:
  - `docs/meta/logs/ops/executive_summary.log`
  - `docs/meta/logs/ops/<run_id>_strategy.md` (recommended; written by the agent)
  - `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md`

To review the latest journal:

```bash
git switch overnight/ops-journal
sed -n '1,200p' docs/meta/logs/ops/executive_summary.log
ls docs/meta/logs/ops | tail
```
