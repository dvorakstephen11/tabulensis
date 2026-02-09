# Windows Task Scheduler Example (Midnight Automation)

This doc shows one concrete way to schedule the overnight automation on **Windows** while executing the repo tooling inside **WSL**.

Decision:
- Scheduler: Windows Task Scheduler (`docs/meta/results/decision_register.md` `DR-0011`)
- Execution environment: local workstation only (`docs/meta/results/decision_register.md` `DR-0010`)

## Prereqs

- WSL is installed and your preferred distro is available (example below uses `Ubuntu`).
- Repo is available inside WSL at (example): `/home/dvorak/repo/agent_hub_repos/excel_diff`
- Repo tooling works when run manually in WSL:

```bash
cd /home/dvorak/repo/agent_hub_repos/excel_diff
python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml doctor
```

## Recommended Command (Action)

In Task Scheduler, configure:
- Program/script: `wsl.exe`
- Add arguments (edit distro name + path if needed):

```text
-d Ubuntu -- bash -lc "cd /home/dvorak/repo/agent_hub_repos/excel_diff && mkdir -p tmp/scheduler && ts=$(date +%Y-%m-%d_%H%M%S) && python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml supervise --hours 10 >> tmp/scheduler/${ts}_overnight.log 2>&1"
```

Notes:
- `bash -lc` runs a login shell so PATH and env vars from your login profile are available.
- If you use OpenAI HTTP API mode, ensure `OPENAI_API_KEY` is set in WSL login startup (not just in an interactive terminal).
- The redirect writes a scheduler-level log to `tmp/scheduler/` (local-only; gitignored). The agent's canonical outputs are separate (see below).

## Trigger Settings (Example)

In Task Scheduler:
- Trigger: Daily
- Start: `12:05:00 AM` (pick a few minutes after midnight to avoid "midnight DST weirdness")
- Enabled: Yes

## Working Directory Guidance

Leave Task Scheduler's "Start in" empty; the command `cd`s into the repo inside WSL.

If you must change the repo location, only edit the path inside the `cd ...` segment.

## Recommended Task Scheduler Settings

These reduce footguns:
- Settings:
  - If the task is already running, then: **Do not start a new instance**
  - Stop the task if it runs longer than: **12 hours**
  - If the task fails, restart every: **10 minutes** (up to 3 times)
- Conditions:
  - Optional: **Wake the computer to run this task**
  - Optional: Start only if on AC power (your call)

## Where Outputs Go

This scheduled run will produce:
- Raw runtime state (local-only): `tmp/overnight_agent/`
- Ops journal (committed on its branch): `docs/meta/logs/ops/` on branch `overnight/ops-journal`

See:
- `docs/meta/automation/overnight_agent_runbook.md` ("Where Outputs Go")
- `docs/meta/automation/README.md` ("Output Locations (Canonical)")

## Quick Test (Before Scheduling)

Run the exact command once from Windows to validate quoting:

```powershell
wsl.exe -d Ubuntu -- bash -lc "cd /home/dvorak/repo/agent_hub_repos/excel_diff && python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml run-once --dry-run"
```

Then validate:
- Exit code is 0.
- A log file was created under `tmp/scheduler/` if you used the logged version.
