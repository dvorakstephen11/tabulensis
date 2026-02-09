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

Pick one LLM mode:

1) Codex CLI mode (recommended; no `OPENAI_API_KEY` required by this script)

- Install/configure Codex CLI so `codex exec` works non-interactively.
- Configure `docs/meta/automation/overnight_agent.yaml`:
  - `llm.provider: codex_exec`
  - `llm.model: <your newest available codex model>`

2) OpenAI HTTP API mode (requires `OPENAI_API_KEY`)

- Configure `docs/meta/automation/overnight_agent.yaml`:
  - `llm.provider: openai_chat_completions`
- Set your OpenAI API key in the environment:

```bash
export OPENAI_API_KEY='...'
```

In both modes, you can update the model in `docs/meta/automation/overnight_agent.yaml` (`llm.model`).

## Quick Checks

```bash
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml doctor
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml list-tasks --limit 30
```

Dry run (no LLM calls; no code changes):

```bash
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml run-once --dry-run
```

## Start An Overnight Run

Run for 10 hours (default), resuming an in-progress iteration after crashes/restarts:

```bash
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml supervise --hours 10
```

Stop: `Ctrl+C`.

## Where Outputs Go

1. Task branches + worktrees:
- Branches: `overnight/*`
- Worktrees: `../excel_diff_worktrees/overnight/` (configurable)

2. Raw run artifacts (local-only):
- `tmp/overnight_agent/runs/<run_id>/**`
- `tmp/overnight_agent/state.sqlite3`
- `tmp/overnight_agent/state.json`

3. Ops journal (committed on a dedicated branch):
- Branch: `overnight/ops-journal`
- Files:
  - `docs/meta/logs/ops/executive_summary.log`
  - `docs/meta/logs/ops/<run_id>_report.md`
  - `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md`

To review the latest journal:

```bash
git switch overnight/ops-journal
sed -n '1,200p' docs/meta/logs/ops/executive_summary.log
ls docs/meta/logs/ops | tail
```
