# Meta Methodology Implementation Checklist

This is the execution-level implementation plan for `meta_methodology.md`.

Goals:
- Convert the vision into atomic, checkable actions (docs, scripts, routines, operator habits).
- Preserve the self-referential nature: completing early items should reduce friction for completing later items.
- Keep everything discoverable via `docs/index.md` and agent-facing pointers (`AGENTS.md`, `.codex_skills/**`).

Conventions:
- Use `- [ ]` / `- [x]` only (so `docs/index.md` auto-index can count open/done).
- When a checkbox count changes in any checklist, refresh index:
  - `python3 scripts/update_docs_index_checklists.py`
- Prefer append-only logs; prefer checklists for work-in-progress.

Implementation order (dependency notes):
- Section 1 is prerequisite scaffolding for everything else (logs/templates/locations).
- Sections 2-5 turn daily work into durable artifacts (today plan, research results, logs, context bundles).
- Section 6 (automation) should not be attempted until guardrails and output locations exist.
- Sections 7-13 are the recurring workstreams the automation and daily planning should feed.
- Sections 14-17 make the whole system self-correcting (weekly review, DoD, runbook, docs refresh).

## 0) Establish Canonical Surfaces (Docs + Agent Guidance)

- [x] Make `meta_methodology.md` stable and structured (headings + sections).
- [x] Link `meta_methodology.md` from `docs/index.md` (Operating docs).
- [x] Link `meta_methodology.md` from `AGENTS.md` (so agents read it when asked to prioritize).
- [x] Teach the local docs skill about `meta_methodology.md` (`.codex_skills/tabulensis-docs/SKILL.md`).

- [x] Add `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` to `docs/index.md` under "Operating docs".
- [x] Add `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` to `AGENTS.md` in the "Documentation Index" section.
- [x] Add `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` to `.codex_skills/tabulensis-docs/SKILL.md` "Task Map".
- [x] Add an "Implementation plan checklist" link near the top of `meta_methodology.md` pointing to `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`.
- [x] Run `python3 scripts/update_docs_index_checklists.py` and confirm this checklist appears in the auto-index block in `docs/index.md`.

## 1) Create A Minimal "Operating System" Layout (Files + Directories)

- [x] Decide whether `docs/meta/logs/**` and `docs/meta/results/**` are committed to git or kept local-only (record the decision in `docs/meta/README.md`). (Decision: `docs/meta/results/decision_register.md` `DR-0001`)
- [x] If local-only: add `docs/meta/logs/` (and any other sensitive vendor dump directories) to `.gitignore` and explain the exclusion in `docs/meta/README.md`. (N/A; `DR-0001` chose committed)
- [x] If committed: add a short "Privacy + redaction" warning to `docs/meta/README.md` describing what must never be committed.

- [x] Create file `docs/meta/README.md` describing:
  - what belongs in `docs/meta/prompts/`
  - what belongs in `docs/meta/results/`
  - what belongs in `docs/meta/logs/`
  - what belongs in `docs/meta/automation/`
  - the update rule for `docs/index.md` (run `scripts/update_docs_index_checklists.py`)

- [x] Create directory `docs/meta/logs/`.
- [x] Create directory `docs/meta/logs/daily/`.
- [x] Create directory `docs/meta/logs/research/`.
- [x] Create directory `docs/meta/logs/ops/`.
- [x] Create directory `docs/meta/logs/weekly/`.
- [x] Create directory `docs/meta/logs/marketing/`.
- [x] Create file `docs/meta/logs/README.md` describing:
  - naming convention for daily logs and research logs
  - append-only rules
  - how logs link to checklists and code changes
- [x] Create file `docs/meta/logs/daily/_TEMPLATE.md` with sections:
  - "Date"
  - "Top goals (today)"
  - "Outputs produced (files/links)"
  - "Decisions made"
  - "Risks noticed"
  - "Next actions"
- [x] Create file `docs/meta/logs/research/_TEMPLATE.md` with sections:
  - "Query"
  - "Sources"
  - "Findings"
  - "Actionable experiments"
  - "Append-only log entry"
- [x] Create file `docs/meta/logs/ops/_TEMPLATE.md` with sections:
  - "Runbook executed"
  - "Incident/issue"
  - "Fix"
  - "Follow-ups"
- [x] Create file `docs/meta/logs/weekly/_TEMPLATE.md` with sections:
  - "Wins"
  - "Misses"
  - "What to stop doing"
  - "What to automate next"
  - "Next week priorities (max 3)"

## 2) Define One Place For "Today’s Checklist"

- [x] Create file `docs/meta/today.md` (single-day scratchpad, overwritten daily).
- [x] Decide whether `docs/meta/today.md` is committed to git or ignored (record the decision in `docs/meta/README.md`). (Decision: `docs/meta/results/decision_register.md` `DR-0002`)
- [x] If ignored: add `docs/meta/today.md` to `.gitignore` and add `docs/meta/today.example.md` as a committed template.
- [x] If committed: add a warning section to `docs/meta/today.md` describing what must never be included (secrets, personal data). (N/A; `DR-0002` chose ignored)
- [x] Add top-of-file warning to `docs/meta/today.md`: "This file is overwritten; do not store durable history here."
- [x] Add a short section to `docs/meta/today.md` explaining:
  - it is generated by automation (eventually)
  - it is derived from `meta_methodology.md` + open checklists
- [x] Add a "Timebox" header to `docs/meta/today.md` with explicit fields:
  - available minutes
  - non-negotiable commitments (e.g., Sourcewell)
  - energy level (low/med/high)
- [x] Add a "Stop when" header to `docs/meta/today.md` (explicit cutoff conditions to prevent overwork).
- [x] Add link to `docs/meta/today.md` in `meta_methodology.md` under "Daily Schedule".

## 3) Deep Research: Prompt, Capture, Storage, Synthesis, Action

### 3.1 Prompt Library (Source Of Truth)

- [x] Create market-analysis deep research prompt at `docs/meta/prompts/deep_research_market_analysis.md`.
- [x] Create `scripts/deep_research_prompt.py` to copy the prompt to clipboard (with date injection).
- [x] Add the command reference in `meta_methodology.md` (under the deep research schedule block).

- [x] Create file `docs/meta/prompts/README.md` that:
  - lists every prompt file in `docs/meta/prompts/`
  - states the intended use + cadence for each prompt
  - documents the copy command (`python3 scripts/deep_research_prompt.py ...`)
  - states where results should be saved (`docs/meta/results/**`)
- [x] Create prompt `docs/meta/prompts/deep_research_competitor_watch.md` (competitors + substitutes + pricing + recent changes).
- [x] Create prompt `docs/meta/prompts/deep_research_acquisition_targets.md` (acquirers + outreach paths + rationale).
- [x] Create prompt `docs/meta/prompts/deep_research_distribution_experiments.md` (channels + 2-hour actions + metrics).
- [x] Create prompt `docs/meta/prompts/deep_research_security_watch.md` (security news + supply chain + CVEs relevant to stack).
- [x] Add a 1-paragraph "When to use which prompt" header at top of each deep research prompt file.
- [x] Ensure every deep research prompt includes a `{{RUN_DATE}}` placeholder (filled by script) and a "citation requirements" section.

### 3.2 Friction Removal For Running Deep Research

- [x] Add `--prompt <name>` support to `scripts/deep_research_prompt.py` so it can copy different prompt files.
- [x] Add `--list` support to `scripts/deep_research_prompt.py` to print available prompt names.
- [x] Add `--open` support to `scripts/deep_research_prompt.py` to open ChatGPT in the default browser (best-effort).
- [x] Add `--new-result` support to `scripts/deep_research_prompt.py` that creates a new timestamped file in `docs/meta/results/` and prints the path.
- [x] Add `--copy-result-path` support to copy the intended result file path (so you paste results into the right file).
- [x] Add `--new-a-b` support to `scripts/deep_research_prompt.py` that creates:
  - `..._<prompt>_a.md`
  - `..._<prompt>_b.md`
  - `..._<prompt>_synthesis.md`
  and prints all three paths.
- [x] Add `--open-result` support to `scripts/deep_research_prompt.py` that opens the generated result files in the default editor (best-effort).
- [x] Add `--audio` placeholder support to `scripts/deep_research_prompt.py` that prints a future audio output path (even before TTS exists).

### 3.3 Capturing Results (Two Chat Outputs + Synthesis)

- [x] Decide whether deep research results live in: (Decision: `docs/meta/results/decision_register.md` `DR-0003`)
  - `docs/meta/results/` (flat), or
  - `docs/meta/results/deep_research/` (grouped).
  Record the decision in `docs/meta/results/README.md`.
- [x] Decide a canonical filename scheme for deep research captures (document in `docs/meta/logs/README.md`): (Decision: `docs/meta/results/decision_register.md` `DR-0004`)
  - timestamp prefix: `YYYY-MM-DD_HHMMSS`
  - suffix describes prompt and/or chat: `market_analysis_a`, `market_analysis_b`, `synthesis`
- [x] Create file `docs/meta/results/README.md` with:
  - naming scheme
  - recommended structure inside each result file (executive brief, findings, sources, actions)
  - "append-only log entry" convention
- [x] Add an explicit "Where to paste" rule to `docs/meta/results/README.md`:
  - paste raw model output first
  - do not edit for style before capturing the raw output
  - do edits (formatting, pruning) only after raw capture
- [x] Add a "How to capture" section to `meta_methodology.md` describing:
  - run prompt A and prompt B (or same prompt in two chats)
  - paste outputs into two separate files under `docs/meta/results/`
  - run a synthesis step
- [x] Create synthesis prompt file `docs/meta/prompts/deep_research_synthesis.md` that takes:
  - the two outputs as input
  - produces a merged "Today’s Checklist" list with 30/60/120-minute actions
- [ ] Add a new script `scripts/deep_research_synthesize.py` that:
  - reads the two result files
  - prints a copy/pastable synthesis prompt
  - copies the synthesis prompt to clipboard
- [x] Add a "post-synthesis action" checklist to `docs/meta/prompts/deep_research_synthesis.md`:
  - update `docs/meta/today.md`
  - append 5-10 bullets to the daily log
  - create 1 marketing experiment task
  - create 1 product friction task
  - create 1 perf/UI/security task

### 3.4 Audio For Deep Research (So You Can Listen While Exercising)

- [x] Decide a TTS path (document in `docs/meta/README.md`): (Decision: `docs/meta/results/decision_register.md` `DR-0005`)
  - cloud TTS (which provider), or
  - local TTS (which tool).
- [x] Create directory `docs/meta/audio/` (or pick a different directory; record the decision).
- [x] Create script `scripts/tts_generate.py` that:
  - takes an input markdown/text file
  - writes an `.mp3` or `.wav` output
  - prints the output path
- [x] Add `scripts/tts_generate.py` `--voice` option (even if only 1 voice exists initially).
- [x] Add `scripts/tts_generate.py` `--rate` option (speed control).
- [x] Add `scripts/tts_generate.py` `--max-chars` option to avoid giant audio files.
- [x] Add a `docs/meta/audio/README.md` explaining:
  - how to generate audio
  - recommended inputs (synthesis file, not raw)
  - where outputs are stored

## 4) Make Daily Logging Non-Optional (But Low Friction)

- [x] Create script `scripts/new_daily_log.py` that:
  - creates `docs/meta/logs/daily/YYYY-MM-DD.md` from template
  - refuses to overwrite an existing file
  - prints the path to stdout
- [x] Add `scripts/new_daily_log.py` `--copy-path` option (copies the created path to clipboard for quick open/paste).
- [x] Add `scripts/new_daily_log.py` `--open` option (opens the created file in the default editor, best-effort).
- [x] Create script `scripts/new_ops_log.py` that:
  - creates `docs/meta/logs/ops/YYYY-MM-DD_<slug>.md` from template
  - requires an explicit `<slug>` argument (e.g., `stripe_webhook_debug`)
  - refuses to overwrite
- [x] Create script `scripts/new_research_log.py` that:
  - creates `docs/meta/logs/research/YYYY-MM-DD_<slug>.md` from template
  - requires an explicit `<slug>` argument (e.g., `acquisition_targets`)
  - refuses to overwrite
- [ ] Create script `scripts/open_today.py` that:
  - opens `docs/meta/today.md` in the default editor (best-effort), otherwise prints path
- [ ] Add `scripts/open_today.py` `--copy` option (copies the file path to clipboard).
- [ ] Add `scripts/open_today.py` `--reset` option that overwrites `docs/meta/today.md` with the committed template (or a built-in template) after confirming the decision in `docs/meta/README.md`.
- [x] Add to `meta_methodology.md` a single "Daily close-out" rule:
  - update `docs/meta/logs/daily/YYYY-MM-DD.md`
  - update at least one checklist checkbox
  - refresh `docs/index.md` checklist index if counts changed
- [x] Add to `meta_methodology.md` a single "Daily open" rule:
  - create/open `docs/meta/today.md`
  - create/open the daily log
  - pick the top 3 tasks (max)

## 5) Automate "What Should I Do Today?" (Without Magical Thinking)

### 5.1 Define Inputs

- [x] Decide canonical inputs to daily planning (document in `docs/meta/logs/README.md`): (Decision: `docs/meta/results/decision_register.md` `DR-0006`)
  - `meta_methodology.md`
  - `docs/index.md` unfinished checklist auto-index block
  - `todo.md` (ideas/backlog)
  - `product_roadmap.md` (direction)
  - most recent `docs/meta/logs/daily/*.md`
- [x] Create file `docs/meta/prompts/daily_plan.md` that:
  - instructs the model to select a small set of tasks
  - includes strict constraints (time budget, dependencies, stop conditions)
  - outputs an explicit checklist ready to paste into `docs/meta/today.md`

### 5.2 Build A Deterministic Generator (Local Script)

- [x] Create script `scripts/generate_daily_plan_context.py` that prints:
  - the above input file contents (or excerpts)
  - a directory tree of the repo (bounded)
  - the list of unfinished checklists from `python3 scripts/update_docs_index_checklists.py --print`
- [x] Ensure `scripts/generate_daily_plan_context.py` has a `--max-chars` option to avoid runaway output.
- [x] Ensure `scripts/generate_daily_plan_context.py` has a `--copy` option to copy output to clipboard (fallback to `tmp/` backup).
- [x] Ensure `scripts/generate_daily_plan_context.py` includes (at minimum) these file contents when present:
  - `docs/index.md`
  - `AGENTS.md`
  - `meta_methodology.md`
  - `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`
  - `product_roadmap.md`
  - `todo.md`
- [ ] Add `--include <path>` and `--exclude <path>` options to `scripts/generate_daily_plan_context.py` for one-off planning sessions.
- [x] Add a short usage block in `meta_methodology.md`:
  - `python3 scripts/generate_daily_plan_context.py --copy`
  - paste into ChatGPT
  - paste the resulting plan into `docs/meta/today.md`

### 5.3 "Ultimate Context Payload" For GPT-5.x-Pro

- [x] Create file `docs/meta/prompts/pro_context_payload.md` that defines:
  - what must be included (codebase + SOPs + vendor readings)
  - hard limits (max tokens/characters)
  - output format (one concatenated markdown file with sections)
- [x] Decide where context payload outputs live: (Decision: `docs/meta/results/decision_register.md` `DR-0007`)
  - `tmp/pro_context_payload.md` (local-only), or
  - `docs/meta/results/pro_context_payloads/` (committed).
  Record decision in `docs/meta/README.md`.
- [ ] Add a new command to `docs/meta/prompts/generate_review_context.py` (or a new script `scripts/generate_pro_context_payload.py`) that:
  - concatenates all SOP/runbook/checklist docs you actually want (start from `docs/index.md` "Operating docs")
  - includes the current unfinished checklist index output
  - includes the last N daily logs (bounded)
  - writes a single markdown payload file
  - optionally copies it to clipboard
- [ ] Add a deterministic "SOP concatenation" order and record it in `docs/meta/prompts/pro_context_payload.md` (so the payload is stable between runs).
- [ ] Add a `--max-chars` option to the payload generator that truncates sections with an explicit "... truncated ..." marker.
- [ ] Add a `--vendor-snapshot` option that appends a vendor snapshot section (even if initially manual placeholders).

## 6) Multi-Agent Dispatch (Midnight Automation)

### 6.1 Make It Safe To Run

- [x] Create directory `docs/meta/automation/`.
- [x] Create file `docs/meta/automation/README.md` defining:
  - what counts as a "safe" automated action
  - what must be human-reviewed
  - how to record failures
- [x] Create file `docs/meta/automation/guardrails.md` defining:
  - never deploy without explicit operator action
  - never rotate secrets
  - never run destructive git commands (`reset --hard`, etc.)
  - always write outputs to `docs/meta/logs/ops/` or `docs/meta/results/`

### 6.2 Standardize Agent Outputs

- [x] Decide a single output directory for automated runs (document in `docs/meta/automation/README.md`). (Decision: `docs/meta/results/decision_register.md` `DR-0008`)
- [x] Decide a filename scheme: `YYYY-MM-DD_HHMMSS_<task>_<agent>.md`. (Decision: `docs/meta/results/decision_register.md` `DR-0009`)
- [ ] Create file `docs/meta/automation/_TEMPLATE_agent_output.md` with:
  - "Task"
  - "Inputs read"
  - "Actions taken"
  - "Files changed"
  - "Commands run"
  - "Results"
  - "Next steps"

### 6.3 Worktree Isolation

- [x] Decide whether automation runs in dedicated git worktrees (safer; chosen) vs current worktree (fast, risky). (Decision: `docs/meta/results/decision_register.md` `DR-0018`)
  - current worktree (fast, risky), or
  - dedicated git worktrees (safer)
- [x] If worktrees: create script `scripts/agent_worktree.sh` that:
  - creates `../excel_diff_worktrees/<name>` (outside repo) with `git worktree add`
  - prints the path
  - has a `--cleanup` command to remove old worktrees
- [x] Add documentation in `docs/meta/automation/README.md` for using `scripts/agent_worktree.sh`.

### 6.4 Dispatch Mechanism (Human-In-The-Loop First)

- [ ] Create file `docs/meta/automation/tasks.yaml` defining a list of tasks (one per agent).
- [ ] Create script `scripts/automation_dispatch.py` that:
  - reads `docs/meta/automation/tasks.yaml`
  - prints copy/pastable prompts for each task (one block per agent)
  - optionally copies each prompt to clipboard sequentially
- [ ] Add a `--dry-run` mode to `scripts/automation_dispatch.py` that prints without copying.
- [ ] Add a `--tag` option (e.g., `--tag 2026-02-08_midnight`) to namespace outputs.
- [ ] Add "operator checklist" at end of `scripts/automation_dispatch.py` output:
  - "create daily log"
  - "run deep research prompt"
  - "review agent outputs"
  - "update checklists"

### 6.5 Define The Midnight Agent Set (Matches `meta_methodology.md`)

- [ ] In `docs/meta/automation/tasks.yaml`, define these tasks explicitly (one per agent):
  - `perf_experiment`
  - `ui_ux_improvement_candidate`
  - `automation_and_friction_improvements`
  - `manual_todo_list_for_operator`
- [ ] Create prompt file `docs/meta/prompts/agents/perf_experiment.md` that instructs the agent to:
  - read perf policy in `AGENTS.md`
  - propose 1 scoped experiment
  - run the correct perf suite (quick/gate/full cycle) based on scope
  - write results to a timestamped report under `docs/meta/logs/ops/`
- [ ] Create prompt file `docs/meta/prompts/agents/ui_ux_improvement_candidate.md` that instructs the agent to:
  - read UI docs (`docs/ui_visual_regression*.md`, `docs/ui_guidelines.md`)
  - propose 1 scoped UI improvement with acceptance criteria
  - specify the exact file(s) likely to change
  - propose an update to a UI checklist (or create one if missing)
- [ ] Create prompt file `docs/meta/prompts/agents/automation_and_friction_improvements.md` that instructs the agent to:
  - read `meta_methodology.md` and this checklist
  - identify the top 3 friction points
  - propose a concrete script/doc change for each
  - output a patch-ready plan
- [ ] Create prompt file `docs/meta/prompts/agents/manual_todo_list_for_operator.md` that instructs the agent to:
  - read unfinished checklists
  - propose a 30/60/120-minute plan for the operator
  - include explicit stop conditions
  - output as `docs/meta/today.md` content
- [ ] Add a `docs/meta/prompts/agents/README.md` that:
  - explains what each agent prompt is for
  - specifies expected output locations

### 6.6 Scheduling The Automation

- [x] Decide the scheduler for the midnight run (record in `docs/meta/automation/README.md`). (Decision: `docs/meta/results/decision_register.md` `DR-0011`)
  - cron, or
  - systemd timer, or
  - Windows Task Scheduler, or
  - manual-only for now.
- [ ] If cron: create `docs/meta/automation/cron_example.md` with:
  - exact cron line
  - environment setup requirements
  - where logs go
- [ ] If systemd: create `docs/meta/automation/systemd_example.md` with:
  - `*.service` unit content
  - `*.timer` unit content
  - enable/start commands
- [x] If Task Scheduler: create `docs/meta/automation/task_scheduler_example.md` with:
  - trigger settings
  - action command line
  - working directory guidance

### 6.7 Make It Observable (So It Can Fail Safely)

- [ ] Add a "run header" section to every automation output that includes:
  - run tag
  - start/end timestamps
  - git branch + commit
  - environment (OS/WSL)
- [ ] Add a "failure summary" section to automation outputs:
  - what failed
  - what was skipped
  - what needs human follow-up
- [ ] Add a "do not proceed if" section to automation guardrails:
  - dirty worktree with unrelated changes
  - missing secrets/vars
  - failing baseline tests

## 7) Remove Obstacles For Download/Install/Use (Methodology -> Concrete Workstreams)

### 7.1 Make The Work Visible

- [ ] Create checklist file `DOWNLOAD_INSTALL_FRICTION_CHECKLIST.md`.
- [ ] Add it to `docs/index.md` under Operating docs.
- [ ] In `DOWNLOAD_INSTALL_FRICTION_CHECKLIST.md`, add a "Scope" section that explicitly lists:
  - CLI install paths
  - desktop app install paths (if applicable)
  - web demo (if applicable)
- [ ] Add a section in `DOWNLOAD_INSTALL_FRICTION_CHECKLIST.md` for each platform:
  - Windows (exe/zip, path)
  - macOS (tar.gz, quarantine)
  - Linux (tar.gz/AppImage)
- [ ] Add a "smoke test script" section with exact commands to validate installs (one command block per platform).
- [ ] Add a "Known failure modes" section (Gatekeeper/quarantine, missing PATH, missing deps, permissions).

### 7.2 Make The Download Page Verifiably Correct

- [ ] In `DOWNLOAD_INSTALL_FRICTION_CHECKLIST.md`, add a "Download page contract" section:
  - what URLs must exist
  - what file naming pattern must be used
  - what checksums must be present
- [ ] Add checklist item(s) that explicitly reference existing scripts:
  - `scripts/generate_checksums.py`
  - `scripts/verify_release_versions.py`
  - `scripts/package_cli_windows.py`, `scripts/package_cli_macos.py`, `scripts/package_cli_linux.py`
- [ ] Add a "Release artifact inventory" table to `DOWNLOAD_INSTALL_FRICTION_CHECKLIST.md`:
  - artifact name
  - platform
  - how produced (script)
  - where hosted
  - verification command

### 7.3 Automate The Audit

- [ ] Create script `scripts/audit_download_links.py` that:
  - fetches `https://tabulensis.com/download` (or the local `public/download/index.html` source)
  - verifies links are non-placeholder and reachable
  - outputs a pass/fail report
- [ ] Add a CI-friendly mode to `scripts/audit_download_links.py` (exit non-zero on failures).
- [ ] Add documentation in `docs/operations.md` for running this audit weekly.

### 7.4 Automate A Local Install Smoke Test (One Command)

- [ ] Create script `scripts/install_smoke_test.sh` that:
  - downloads the current release artifacts (or uses local build outputs)
  - runs `tabulensis --version`
  - runs a minimal `tabulensis diff` on a tiny fixture
  - prints a clear pass/fail summary
- [ ] Create a Windows version `scripts/install_smoke_test.ps1` with equivalent behavior.

## 8) Marketing Avenues (Make It Concrete, Trackable)

### 8.1 Create The Checklist + Tracking

- [ ] Create file `MARKETING_DAILY_CHECKLIST.md` (checkboxes; daily repeatables).
- [ ] Create file `MARKETING_WEEKLY_REVIEW_CHECKLIST.md`.
- [ ] Create file `docs/meta/logs/marketing/README.md` describing how to record:
  - what was posted
  - what channels
  - metrics (impressions, clicks, signups, conversions)
- [ ] Verify `docs/meta/logs/marketing/` exists (created in Section 1) and is either committed or gitignored per the decision in `docs/meta/README.md`.
- [ ] Add links to these marketing checklists in `docs/index.md` under Operating docs.

### 8.2 Define "Basic Marketing Avenues" Precisely

- [ ] In `MARKETING_DAILY_CHECKLIST.md`, add explicit tasks for each channel you will actually use:
  - LinkedIn
  - Twitter/X
  - YouTube
  - Reddit (specific subreddits)
  - Hacker News (if/when appropriate)
  - newsletter/email list (if applicable)
- [ ] For each channel task, include:
  - exact posting frequency target
  - template types (demo clip, tip, before/after diff, feature highlight)
  - metric to record (and where)

### 8.3 Create Reusable Copy/Asset Templates (So Posting Is 5 Minutes)

- [x] Create directory `docs/meta/marketing_templates/`.
- [x] Create file `docs/meta/marketing_templates/README.md` describing:
  - what each template is for
  - where generated assets live
  - how to measure results
- [x] Create file `docs/meta/marketing_templates/post_short_demo.md` with:
  - hook line template
  - 3 bullet body template
  - CTA template (download link + docs link)
  - hashtag/tag guidance
- [x] Create file `docs/meta/marketing_templates/post_before_after.md` with:
  - "before" screenshot caption template
  - "after" screenshot caption template
  - CTA template
- [x] Create file `docs/meta/marketing_templates/post_user_story.md` with:
  - target persona
  - pain point
  - how Tabulensis helps
  - CTA

### 8.4 Tracking and Attribution (UTMs + Simple Dashboard)

- [x] Decide a canonical UTM scheme (document in `docs/meta/marketing_templates/README.md`): (Decision: `docs/meta/results/decision_register.md` `DR-0012`)
  - `utm_source`, `utm_medium`, `utm_campaign`, `utm_content`
- [x] Create script `scripts/utm.py` that:
  - takes source/medium/campaign/content
  - outputs a full URL with UTMs
  - optionally copies to clipboard
- [x] Create file `docs/meta/logs/marketing/metrics.csv` (or `.md`) with columns:
  - date
  - channel
  - post_id/link
  - impressions
  - clicks
  - conversions (if known)
  - notes
- [ ] Add a 5-minute daily ritual to `MARKETING_DAILY_CHECKLIST.md`:
  - record yesterday's metrics into the metrics file

### 8.5 Partnership / Outreach Pipeline

- [ ] Create file `PARTNERSHIPS_OUTREACH_CHECKLIST.md`.
- [ ] Add `PARTNERSHIPS_OUTREACH_CHECKLIST.md` to `docs/index.md` under Operating docs.
- [ ] Create file `docs/meta/logs/marketing/outreach.csv` (or `.md`) with columns:
  - date
  - target
  - channel (email/LinkedIn/etc.)
  - ask
  - status
  - follow-up date
- [ ] Add a weekly ritual to `MARKETING_WEEKLY_REVIEW_CHECKLIST.md`:
  - send N outreach messages
  - schedule N follow-ups
  - update outreach log

### 8.6 Demo Video Pipeline (Atomic, Repeatable)

- [x] Decide the minimum demo-video format: 30-60 seconds. Record decision in `docs/meta/marketing_templates/README.md`. (Decision: `docs/meta/results/decision_register.md` `DR-0013`)
- [ ] Create checklist `DEMO_VIDEO_CHECKLIST.md` with atomic steps:
  - script outline
  - record screen
  - export
  - upload
  - cross-post
- [ ] Add `DEMO_VIDEO_CHECKLIST.md` to `docs/index.md` under Operating docs.

## 9) Vendor Onboarding + Automation

### 9.1 Inventory Vendors

- [ ] Create file `docs/operations_vendors.md` listing:
  - vendor
  - purpose
  - credentials storage location (password manager name)
  - operator actions
  - automation hooks
- [ ] Add `docs/operations_vendors.md` to `docs/index.md` under Operating docs.

### 9.2 Fastmail Email Synthesis

- [x] Decide access method for Fastmail: manual export (for now). (Decision: `docs/meta/results/decision_register.md` `DR-0014`)
- [ ] Create file `docs/meta/automation/fastmail_synthesis.md` with:
  - scope of emails to read
  - privacy rules
  - output format
- [ ] Create script `scripts/fastmail_export_instructions.md` (even if manual) with exact steps.
- [ ] Create "first automation" milestone:
  - extract subject + sender + date + 1-paragraph summary for last N emails
  - write results to `docs/meta/logs/ops/YYYY-MM-DD_fastmail_summary.md`

### 9.3 Resend, Stripe, Cloudflare (Keep Runbooks Tight)

- [ ] Ensure each vendor has a checklist/runbook file with:
  - secrets/vars list
  - local dev steps
  - verification steps
  - common failure modes
- [ ] Add "failure mode: missing env-specific vars" note to `RESEND_SETUP_CHECKLIST.md` (re: Wrangler env inheritance).
- [ ] Add "wrangler dev env vars are not inherited" note to `docs/licensing_service.md`.

### 9.4 Vendor Snapshot (One Dashboard-Like Report)

- [ ] Create file `docs/meta/automation/vendor_snapshot.md` defining:
  - which vendors are included (Fastmail, Cloudflare, Stripe, Resend)
  - the exact fields to capture for each vendor
  - how often to capture (daily/weekly)
  - where the snapshot output is stored
- [ ] Create template `docs/meta/automation/_TEMPLATE_vendor_snapshot.md` with sections:
  - "Stripe"
  - "Cloudflare"
  - "Resend"
  - "Fastmail"
  - "Actions suggested"
- [ ] Create script `scripts/vendor_snapshot.py` (phase 1: manual placeholders) that:
  - creates `docs/meta/logs/ops/YYYY-MM-DD_vendor_snapshot.md`
  - fills in headings + placeholder fields
  - copies the file path to clipboard
- [ ] Add phase 2 plan: implement actual API reads (record required credentials and where stored).
- [ ] Add guardrail: `scripts/vendor_snapshot.py` must never print secrets to stdout.

### 9.5 OpenAI / Model Capacity Planning (Overnight Work)

- [x] Create file `docs/meta/automation/model_accounts.md` listing:
  - which model accounts exist
  - what each is used for
  - rate/usage limits observed
  - when to switch accounts
- [x] Decide whether to create a second OpenAI Pro account: defer until limits materially block progress. (Decision: `docs/meta/results/decision_register.md` `DR-0015`; recorded in `docs/meta/automation/model_accounts.md`)

## 10) UI/UX Improvement Loop (Repeatable, Not Random)

- [ ] Create file `UI_UX_DAILY_CHECKLIST.md` with:
  - one user-facing UI improvement per day (scoped)
  - one usability test (even self-run) per day
  - one "paper cut" fix per day
- [ ] Add it to `docs/index.md` under Operating docs.
- [ ] Add a rule to `UI_UX_DAILY_CHECKLIST.md`:
  - every UI change must include a before/after screenshot or a reproducible scenario
- [ ] Wire into existing tooling:
  - reference `scripts/ui_capture.sh`
  - reference `scripts/ui_snapshot_summary.py`

- [ ] In `UI_UX_DAILY_CHECKLIST.md`, add a required "Definition of done" section for a UI task:
  - exact UI state before (screenshot or reproduction)
  - exact UI state after
  - how to verify quickly (command(s))
- [ ] Create file `docs/meta/logs/ui_ux_backlog.md` with:
  - top 20 UX papercuts
  - for each: severity, evidence, and a 1-sentence fix hypothesis
- [ ] Create file `docs/meta/automation/ui_snapshot_nightly.md` describing:
  - the scenario(s) to capture nightly
  - the command(s) to run
  - where artifacts are stored
- [ ] Add a nightly task in `docs/meta/automation/tasks.yaml`:
  - run `./scripts/ui_capture.sh compare_grid_basic --tag <tag>`
  - run `python3 scripts/ui_snapshot_summary.py ...`
  - write a summary markdown to `docs/meta/logs/ops/`

## 11) Performance Improvement Experiments (Repeatable, Noise-Aware)

- [ ] Create file `PERF_EXPERIMENT_DAILY_CHECKLIST.md` that:
  - forces a hypothesis per experiment
  - forces pre/post symmetry (runs, fixtures)
  - forces recording results to `benchmarks/perf_cycles/` (or a dedicated log)
- [ ] Add it to `docs/index.md` under Operating docs.
- [ ] Add link to the perf policy in `AGENTS.md` and `docs/perf_playbook.md`.
- [ ] Add a "nightly perf experiment" automation task in `docs/meta/automation/tasks.yaml` (once dispatch exists).

- [ ] In `PERF_EXPERIMENT_DAILY_CHECKLIST.md`, add a "Which suite to run" decision table:
  - quick suite command
  - gate suite command
  - full perf cycle commands (pre/post)
- [ ] In `PERF_EXPERIMENT_DAILY_CHECKLIST.md`, add a "Noise discipline" section:
  - minimum runs for small deltas
  - how to interpret `cycle_signal.md`
- [ ] Create file `docs/meta/logs/perf_experiments.md` (append-only index) with entries:
  - date
  - hypothesis
  - command(s)
  - delta summary
  - link to cycle directory
- [ ] Add a guardrail to `docs/meta/automation/guardrails.md`:
  - perf automation must not delete existing perf cycle directories
  - perf automation must respect retention rule in `AGENTS.md`

## 12) Platform Security Improvements (Concrete, Trackable)

- [x] Create file `SECURITY_DAILY_CHECKLIST.md`.
- [x] Add it to `docs/index.md` under Operating docs.
- [x] Add explicit tasks:
  - dependency audit (Rust + npm)
  - secret scanning (pre-commit/CI)
  - minimal SAST (what tool, how often)
- [x] Create `scripts/security_audit.sh` that runs the chosen tools and writes a report file under `docs/meta/logs/ops/`.

- [x] In `SECURITY_DAILY_CHECKLIST.md`, list the exact commands to run for:
  - Rust dependency audit (choose tool)
  - npm dependency audit (choose tool)
  - secret scanning (choose tool)
  - SAST (choose tool)
- [x] Decide and record the chosen security tools in `docs/meta/automation/guardrails.md`. (Decision: `docs/meta/results/decision_register.md` `DR-0016`)
- [ ] Add a weekly task in `docs/meta/automation/tasks.yaml` to run `scripts/security_audit.sh`.
- [x] Add a "redaction rules" section to `SECURITY_DAILY_CHECKLIST.md`:
  - what outputs must never be committed
  - what can be committed (summaries only)

## 13) Internal Analytics System (Define Before Building)

- [ ] Create file `docs/analytics_plan.md` describing:
  - what to measure
  - privacy guarantees
  - opt-in/out UX
  - storage/retention
- [ ] Add `docs/analytics_plan.md` to `docs/index.md` under Guides or Operating docs (choose one and be consistent).
- [ ] Create `ANALYTICS_DAILY_CHECKLIST.md` with atomic steps for incremental implementation.

- [ ] In `docs/analytics_plan.md`, define a minimal event schema (names + fields) for:
  - app start
  - diff run (file sizes only, no content)
  - time-to-first-result
  - error codes
- [ ] In `docs/analytics_plan.md`, define an explicit "never collect" list (privacy boundary).
- [ ] In `docs/analytics_plan.md`, define storage options:
  - local-only analytics, or
  - remote telemetry (which provider).
  Record decision + rationale.
- [ ] In `ANALYTICS_DAILY_CHECKLIST.md`, add the first 10 atomic implementation tasks (no more than 1 hour each).

## 14) Improve The Meta-Methodology (Self-Referential Loop)

- [ ] Add a "Weekly review" block to `meta_methodology.md` with:
  - review last 7 daily logs
  - review top unfinished checklists
  - prune/merge checklists
  - decide next week’s 3 priorities
- [ ] Verify `docs/meta/logs/weekly/` and `docs/meta/logs/weekly/_TEMPLATE.md` exist (created in Section 1) and match the format you actually want.
- [ ] Create script `scripts/new_weekly_review.py` to generate `docs/meta/logs/weekly/YYYY-WW.md`.
- [ ] Add script `scripts/new_weekly_review.py` `--open` option (opens created file, best-effort).
- [ ] Add a weekly ritual to `MARKETING_WEEKLY_REVIEW_CHECKLIST.md`:
  - review last week's marketing metrics
  - pick next week's 3 experiments
- [ ] Create rule: any time you change the daily schedule, also update:
  - `meta_methodology.md`
  - this checklist
  - `docs/index.md` links if new operating docs were added

- [ ] Add a "Doc freshness" ritual:
  - run `python3 scripts/update_docs_index_checklists.py`
  - scan the auto-index for stale/high-open checklists
  - pick 1 to focus down (reduce open count)

## 15) "Definition Of Done" For Methodology Alignment

- [x] Create a short "Methodology DoD" section at the bottom of `meta_methodology.md` that is measurable (Implemented in `meta_methodology.md` "Methodology DoD (Measurable)").
  - daily log exists for at least 6 of the last 7 days
  - at least 1 checklist item checked per day
  - at least 1 deep research capture per week
  - at least 1 UI/UX improvement shipped per week
  - at least 1 perf experiment logged per week
- [ ] Add a self-audit script `scripts/methodology_audit.py` that:
  - checks for missing daily logs
  - checks for stale checklists (no changes in N days)
  - outputs a simple red/yellow/green report

- [x] Decide the "stale checklist" threshold (N days) and record it in `meta_methodology.md` DoD: N=30 days. (Decision: `docs/meta/results/decision_register.md` `DR-0017`)
- [ ] Add `scripts/methodology_audit.py` output to a timestamped file under `docs/meta/logs/ops/` (append-only snapshots).
- [ ] Add `scripts/methodology_audit.py` `--json` output mode (so it can be graphed later).
- [ ] Add `scripts/methodology_audit.py` `--copy` option (copy summary to clipboard for quick paste into a chat).
- [ ] Add a weekly scheduled task to run `scripts/methodology_audit.py` (once scheduling exists).

## 16) Materialize The Daily Schedule (Make It A Clickable Runbook)

- [ ] Create file `DAILY_OPERATOR_RUNBOOK.md` that mirrors the time blocks in `meta_methodology.md`:
  - Midnight
  - 5am-7am
  - 8:20am
  - 9am-12pm
  - 1pm-4:15pm/5:15pm
- [ ] Add `DAILY_OPERATOR_RUNBOOK.md` to `docs/index.md` under Operating docs.
- [ ] In `DAILY_OPERATOR_RUNBOOK.md`, for each time block, add:
  - "Objective"
  - "Inputs"
  - "Commands to run"
  - "Outputs to produce" (file paths)
  - "Stop conditions"
- [ ] In `DAILY_OPERATOR_RUNBOOK.md`, add explicit commands that already exist:
  - deep research prompt copy: `python3 scripts/deep_research_prompt.py`
  - docs checklist refresh: `python3 scripts/update_docs_index_checklists.py`
- [ ] Add a rule to `DAILY_OPERATOR_RUNBOOK.md`:
  - every automated run must write an output file under `docs/meta/logs/ops/` (or explain why not)

## 17) "Documentation Layer That Updates Itself"

- [ ] Create script `scripts/docs_refresh.py` that runs:
  - `python3 scripts/update_docs_index_checklists.py`
  - (optional) `python3 scripts/check_line_endings.py --staged` when in a git workflow
  - any other lightweight doc consistency checks you decide on
- [ ] Add a `--dry-run` option to `scripts/docs_refresh.py` that prints what it would do.
- [ ] Add a `--copy-summary` option to `scripts/docs_refresh.py` that copies a 5-line summary to clipboard.
- [ ] Add a "Docs refresh" step to `DAILY_OPERATOR_RUNBOOK.md` (where it belongs in the day).

## 18) Expand AI Tooling (Skills + Prompt Packs + Repeatable Routines)

- [ ] Create a repo-local Codex skill `.codex_skills/tabulensis-operator/SKILL.md` that teaches:
  - how to use `meta_methodology.md`
  - how to use `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`
  - how to run deep research prompts + capture results
  - how to write outputs to `docs/meta/logs/**`
  - what guardrails apply (no deploys, no secrets, no destructive git)
- [ ] Add `.codex_skills/tabulensis-operator/agents/openai.yaml` so it is selectable as an agent persona.
- [ ] Add a pointer to the new skill in `AGENTS.md` under "Documentation Index".
- [ ] Create prompt pack directory `docs/meta/prompts/packs/` and a `README.md` that describes:
  - what each pack is for
  - how to copy prompts to clipboard
  - expected output location
- [ ] Create prompt `docs/meta/prompts/packs/bug_triage.md` that instructs:
  - how to reproduce
  - how to minimize
  - where to log
  - how to add a regression test
- [ ] Create prompt `docs/meta/prompts/packs/marketing_post_generator.md` that outputs:
  - 3 variants of a post
  - explicit CTA + metrics to track
  - suggested UTM parameters

## 19) SOP Coverage Audit (Make Missing Docs Obvious)

- [ ] Add a "SOP coverage audit" section to `docs/index.md` (or `docs/operations.md`) describing how to check for missing runbooks.
- [ ] Create script `scripts/sop_audit.py` that:
  - parses `docs/index.md` Operating docs links
  - verifies each referenced file exists
  - reports missing or duplicated links
  - optionally checks for a "Last updated:" line in each SOP
- [ ] Add a `--fix-links` (dry-run first) option to `scripts/sop_audit.py` that suggests patches when links are broken.
- [ ] Add a weekly task in `DAILY_OPERATOR_RUNBOOK.md` to run `scripts/sop_audit.py`.
