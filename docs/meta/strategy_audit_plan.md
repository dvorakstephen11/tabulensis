# Strategy + Documentation Audit Plan

This document is a step-by-step plan to:

1. Exhaustively read the repo's *primary documentation*, including all checklists.
2. Reconstruct the underlying strategy and operating system (as-is, not as remembered).
3. Systematically surface "questions you haven't asked" that are implied by the docs.
4. Answer those questions using an evidence-driven workflow.
5. Reconcile answers back into durable documentation, checklists, and automation inputs.

This plan intentionally does **not** enumerate the questions in advance. It instead defines a repeatable process that forces question discovery and evidence-backed answers.

## Scope And Definitions

### Primary Documentation (Operational Definition)

Treat the following as the *primary* corpus for this audit:

1. `docs/index.md` and everything it links under:
   - "Operating docs (SOPs, runbooks, checklists)"
   - "Developer workflows (how to work in this repo)"
   - "Guides"
   - "Other references" (when referenced by operating docs or checklists)
2. Every file listed in `docs/index.md` under "Unfinished checklists (auto-indexed)".
3. Any additional doc/checklist that is:
   - Referenced by `meta_methodology.md`, `AGENTS.md`, `README.md`, or `APP_INTENT.md`, or
   - Referenced by a checklist item that is currently open (unchecked), or
   - Required to execute a runbook/SOP (including scripts/config referenced by the runbook).

### Evidence Standard

When "answering" a surfaced question, attach evidence and record confidence:

1. **Primary docs evidence:** explicit statements in the primary corpus (preferred).
2. **Repo reality evidence:** code/config behavior, CLI `--help`, script flags, file existence.
3. **Captured research evidence:** deep research outputs stored under `docs/meta/results/` (A/B + synthesis) with a log pointer.
4. **Operator decision:** if it is truly a choice, record as a decision with options and tradeoffs.

### Non-Goals

- Do not rewrite large docs while still learning what they say. First extract, then reconcile.
- Do not "answer" strategy questions based on vibes. Answer only with evidence or explicit decisions.
- Do not introduce new sources-of-truth. Prefer *one* canonical doc per topic.

## Outputs (What You Produce As You Execute This Plan)

Create these artifacts during execution. Use these exact paths unless you have a strong reason to deviate.

1. Investigation log (append-only):
   - `docs/meta/logs/research/YYYY-MM-DD_strategy_doc_audit.md`
2. Corpus coverage tracker ("docs map"):
   - `docs/meta/results/YYYY-MM-DD_strategy_audit_docs_map.md`
3. Strategy reconstruction memo (as-is, evidence-backed):
   - `docs/meta/results/YYYY-MM-DD_strategy_reconstruction.md`
4. Question bank + answers (with evidence pointers + confidence):
   - `docs/meta/results/YYYY-MM-DD_strategy_questions_and_answers.md`
5. Decision register (only items that require operator judgment):
   - `docs/meta/results/decision_register.md`
6. Checklist triage summary (themes, dependencies, critical path):
   - `docs/meta/results/YYYY-MM-DD_checklist_triage.md`
7. Docs reconciliation plan (exact files/sections to update, minimal churn):
   - `docs/meta/results/YYYY-MM-DD_docs_reconciliation_plan.md`

For any deep research captures, follow `docs/meta/logs/README.md`:

- `docs/meta/results/YYYY-MM-DD_<prompt>_a.md`
- `docs/meta/results/YYYY-MM-DD_<prompt>_b.md`
- `docs/meta/results/YYYY-MM-DD_<prompt>_synthesis.md`

## Phase 0: Setup (Make The Audit Safe And Auditable)

1. Create an audit branch/worktree dedicated to this investigation.
2. Refresh the checklist index so you start from current counts:
   - `python3 scripts/update_docs_index_checklists.py`
3. Create the investigation log:
   - `docs/meta/logs/research/YYYY-MM-DD_strategy_doc_audit.md`
4. In the investigation log, write:
   - The exact primary-corpus definition you are using (copy from this file).
   - Your time budget and what "done" means for this audit run.
   - The "evidence standard" you will follow for answers.
5. Create the docs map file:
   - `docs/meta/results/YYYY-MM-DD_strategy_audit_docs_map.md`
6. Seed the decision register if missing:
   - `docs/meta/results/decision_register.md`
7. Confirm the repo guardrails you must follow during doc edits:
   - Read `AGENTS.md` and summarize only the rules that affect this audit (docs indexing, perf policy, formatting limits).

Exit criteria:

- You have an audit log file, a docs map file, and a decision register file.
- `docs/index.md` checklist index is current at the start of the audit.

## Phase 1: Build The Corpus Inventory (Exhaustive)

Goal: enumerate every document/checklist you must read so nothing critical is missed.

1. Parse `docs/index.md` into a "must read" list.
2. Add top-level strategy/guardrail docs:
   - `README.md`
   - `APP_INTENT.md`
   - `AGENTS.md`
   - `meta_methodology.md`
   - `product_roadmap.md`
   - `REPO_STATE.md`
   - `OPERATING_AGREEMENT.md`
3. Extract all checklists from `docs/index.md` auto-index block.
4. Expand the corpus by link-following:
   - For each operating doc/runbook, scan for referenced paths and add them to the list.
   - Include scripts/config referenced by runbooks (treat as primary when they define behavior).
5. Use repo-wide discovery to find "hidden primary" docs:
   - Find unchecked checkbox files that are not already indexed:
     - `rg -n \"^- \\[ \\]\" -S docs *.md`
   - Find decision-gated language hotspots:
     - `rg -n \"\\b(Decide|Choose|TBD|TODO|If local-only:|If committed:)\\b\" -S`
6. Write the docs map:
   - One row per doc file, with:
     - Path
     - Category (operating/runbook/checklist/guide/reference/log/results)
     - Source-of-truth candidate (yes/no/unknown)
     - Read status (pending / skimmed / close-read / reconciled)
     - Notes: "why it matters" (1 sentence)

Exit criteria:

- The docs map includes everything from `docs/index.md` plus any newly discovered required docs/checklists.
- You can point to a deterministic method for proving completeness (index list + discovery grep).

## Phase 2: Checklist Audit And Critical Path Extraction

Goal: understand what the system thinks is unfinished work, what is decision-gated, and what is critical path.

1. For each checklist file in the docs map:
   - Record open/done counts.
   - Identify whether it is:
     - Execution checklist (clear tasks)
     - Decision checklist (choices required)
     - Process compliance checklist (logs, hygiene, recurring steps)
     - Research checklist (exploration)
2. Extract "decision gates":
   - Items starting with `Decide`, `Choose`, `If local-only:`, `If committed:` are decision gates.
   - Record each gate in `docs/meta/results/decision_register.md` as "pending".
3. Build dependencies:
   - For each decision gate, list the checklist items that depend on it.
   - For each execution item, note prerequisites (files, scripts, vendors, prior steps).
4. Produce the checklist triage summary:
   - Group open items into themes:
     - Operator infrastructure (logs, today, prompts, capture rules)
     - Automation infrastructure (overnight agent, prompt tooling, vendor integrations)
     - Product shipping/distribution (release, signing, auto-update, installer UX)
     - Licensing/vendoring (Stripe/Resend/worker)
     - Core product engineering (perf, UX, correctness, coverage)
     - Research/marketing/acquisition
   - Identify a critical path for "strategy clarity":
     - The minimal set of decisions/docs that unlock everything else.

Exit criteria:

- You have a dependency-aware triage that explains why some open tasks should be done later.
- Decision gates are captured in the decision register (not silently worked around).

## Phase 3: Multi-Pass Reading To Reconstruct Strategy (As-Is)

Goal: reconstruct your strategy and operating system using only the evidence in docs and repo reality.

Use three passes to avoid mixing interpretation with extraction too early.

### Pass A: Orientation (Skim For Structure And Claims)

Read, in order:

1. `APP_INTENT.md`
2. `README.md`
3. `product_roadmap.md`
4. `AGENTS.md`
5. `meta_methodology.md`
6. `docs/index.md`

For each doc, extract into `docs/meta/results/YYYY-MM-DD_strategy_reconstruction.md`:

- Stated win conditions.
- Time horizon and milestones (if any).
- Explicit constraints and guardrails.
- Explicit operating cadence (daily/overnight/weekly).
- Explicit "must have" systems (logs, checklists, prompt library, automation).
- Implied gaps and questions (do not answer yet, just record).

### Pass B: Operating System And Automation (Close Read)

Read and extract concrete mechanics:

1. `docs/meta/README.md`
2. `docs/meta/logs/README.md`
3. Prompt library:
   - `docs/meta/prompts/README.md`
   - `docs/meta/prompts/pro_context_payload.md`
   - `docs/meta/prompts/daily_plan.md`
   - Deep research prompts and synthesis prompt
4. Overnight agent:
   - `docs/meta/automation/overnight_operator_agent_plan.md`
   - `docs/meta/automation/overnight_agent_runbook.md`
   - `docs/meta/automation/overnight_agent.yaml`
   - `scripts/overnight_agent.py`
5. "Today" workflow:
   - `docs/meta/today.md` and any related templates/policies

Extraction rubric (apply to each file):

1. What is the exact step-by-step workflow described?
2. What artifacts must exist, where do they live, and are they committed or local-only?
3. What is automated vs manual?
4. What are the failure modes and how are they handled?
5. What is the minimal operator action required to keep the system healthy daily?

Write an "Ops Map" section in the strategy reconstruction memo with:

- The daily open/close loop and its required artifacts.
- The deep research capture workflow (A/B + synthesis) and its storage/logging rules.
- The overnight agent lifecycle (doctor, list-tasks, supervise, outputs).
- Any missing pieces required for the system to be usable without tribal knowledge.

### Pass C: Domain Deep Dives (Close Read By Theme)

Read by domain so strategy becomes coherent.

For each domain, read the relevant docs from the docs map, then write:

1. The domain's strategy (goal, rationale, constraints).
2. The domain's current state (what exists).
3. The domain's plan (what is next per docs/checklists).
4. The domain's unanswered questions (to be generated in Phase 4).
5. The domain's measurable signals (what you would check daily/weekly).

Domains to cover:

1. Product UI/UX and desktop experience.
2. Core diff correctness + coverage.
3. Performance and perf-validation policy (including how you avoid noise).
4. Distribution/release operations (signing, auto-update, installer UX).
5. Licensing and vendors (Stripe, Resend, worker/service).
6. Automation and AI leverage (overnight, prompt tooling, context payload).
7. Security posture and vulnerability discovery workflow.
8. Marketing/research/acquisition strategy.

Exit criteria:

- The strategy reconstruction memo is complete enough that a third party could explain your plan and constraints with citations.
- You have a clear separation between extracted claims and your own recommendations.

## Phase 4: Generate "Questions You Haven't Asked" (Systematically)

Goal: force discovery of the missing questions implied by the docs.

Mechanism: for every doc and every domain summary, perform an explicit question-generation pass before attempting answers.

### Per-Doc Question Generation

For each doc, generate questions by applying this rubric:

1. Purpose: what is the doc trying to accomplish?
2. Preconditions: what must be true for the doc's workflow to work?
3. Decisions assumed: what choices does it assume were already made?
4. Failure modes: what breaks if the doc is wrong/outdated?
5. Missing operator knowledge: what would a new operator need that isn't written?
6. Conflicts: where might this contradict other docs?
7. Metrics: what success criteria or signals are implied but not specified?
8. Automation hooks: what work is implied but not automated?
9. Privacy/security: what sensitive data is implied and what must never be committed?

### Per-Domain Question Generation

For each domain section in the strategy reconstruction memo, generate questions by applying:

1. Win condition: what is the next milestone, and why is it the right next one?
2. Bottlenecks: what is currently slowing you down, per evidence?
3. Feedback loops: how do you know progress is real (not just activity)?
4. Tradeoffs: what do you sacrifice to go faster (and is it acceptable)?
5. Default plan: what happens when automation fails or vendors change?
6. Budget leverage: how money/time can be traded for speed (tools, vendors, outsourcing).
7. AI collapse resilience: how the product remains valuable as building gets cheaper.

### Recording Format (Question Bank)

For each question you generate, create an entry in:

- `docs/meta/results/YYYY-MM-DD_strategy_questions_and_answers.md`

Each entry should include:

- ID (stable short slug)
- Source pointer (doc path and, if possible, a nearby quote or section heading)
- Category (decision / evidence / research / code-verify / vendor / process)
- Priority (P0 blocks strategy, P1 blocks execution, P2 nice-to-clarify)
- Proposed method to answer (docs / code check / deep research capture / operator decision)
- Status (open / answered / decision-required / deferred)

Exit criteria:

- The question bank covers every doc and every domain, not just the parts you personally find interesting.
- Each question has a defined method-of-answering (so it does not stay vague).

## Phase 5: Answer The Questions (Evidence Driven)

Goal: convert the question bank into answers with explicit evidence, then reconcile back into durable docs.

### Answering Workflow (Repeat Per Question)

1. Attempt to answer from primary docs, quoting minimally and pointing to the exact file/section.
2. If ambiguous, verify against repo reality:
   - Does the referenced script exist?
   - Does `--help` match the documented workflow?
   - Do referenced paths exist and are they correct relative paths?
   - Do runbooks match what the scripts/config actually do?
3. If external knowledge is required, do deep research capture:
   - Two independent runs (A and B) using the relevant prompt from `docs/meta/prompts/`.
   - Store outputs under `docs/meta/results/` using the naming convention.
   - Run synthesis and store the synthesis output.
   - Log the session summary in `docs/meta/logs/research/` with links to the result files.
4. If operator judgment is required, treat it as a decision:
   - Record options, default recommendation, and tradeoffs.
   - Record what checklist items are blocked by the decision.
   - Do not proceed as if a decision were made unless explicitly decided.

### Confidence And Provenance

For each answer, record:

- Confidence level (high/medium/low).
- Evidence links (docs/code/results).
- What would change your mind (tests to run, data to collect, vendor check).

Exit criteria:

- Every answered question has evidence and a confidence level.
- Every decision-required question is in the decision register with clear options.

## Phase 6: Reconcile Back Into Docs, Checklists, And Automation Inputs

Goal: make the clarified strategy and answers durable, discoverable, and executable.

1. Update source-of-truth docs rather than leaving knowledge in the audit artifacts:
   - If it is a daily workflow rule, update `meta_methodology.md` or the relevant SOP.
   - If it is logging/capture format, update `docs/meta/logs/README.md` or related docs.
   - If it is an automation rule, update the relevant runbook/config/docs.
2. Update checklists to reflect reality:
   - Check off items that are now complete.
   - Add new checklist items when you discover missing work that is required.
   - Add explicit "blocked on decision X" notes where needed.
3. Keep docs discoverable:
   - If you add a new primary operating doc, add it to `docs/index.md`.
   - Refresh checklist index counts if checkbox states changed:
     - `python3 scripts/update_docs_index_checklists.py`
4. Produce a docs reconciliation plan:
   - List exact file paths and the precise minimal edits needed.
   - Call out any conflicts and nominate the canonical source-of-truth.

Exit criteria:

- Strategy and workflows are represented in primary docs, not only in audit artifacts.
- Checklists reflect true state (not "aspirational").
- `docs/index.md` remains the canonical entrypoint and includes all primary operating docs.

## Phase 7: Validation (Prove The System Works)

Goal: demonstrate that the documented system is consistent, runnable, and non-fragile.

1. Verify index and checklists:
   - Run `python3 scripts/update_docs_index_checklists.py`.
   - Confirm `docs/index.md` "Unfinished checklists" block matches reality.
2. Validate automation wiring:
   - Run `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml doctor`.
   - If a session is active, run `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml time-remaining`.
   - Optional end-to-end smoke (starts Codex): `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml supervise --hours 0.05 --no-resume`.
3. Validate prompt tooling:
   - Run `python3 scripts/deep_research_prompt.py --help` and ensure docs match reality.
4. Validate the daily operator loop:
   - Confirm the repo has an executable "daily open" and "daily close" flow as written.
5. Spot-check link integrity for newly edited docs:
   - Verify relative links resolve and are not stale.

Exit criteria:

- The runbooks and scripts agree (no documented flags that do not exist).
- The overnight agent can be run without thrashing on schema mismatches.
- A new operator could follow the docs without hidden steps.

## Phase 8: Operationalize (Make This Repeatable)

This phase is optional, but recommended if you want this audit process to become part of your operating system.

1. Create a recurring cadence for "docs integrity" checks:
   - Weekly: refresh checklist index, scan for stale links, scan for decision-gated items.
2. Automate the corpus inventory:
   - A script that enumerates primary docs from `docs/index.md`, discovers checkbox files, and reports deltas.
3. Automate decision-gate surfacing:
   - A script that collects `Decide/Choose/If committed/If local-only` items into a daily operator queue.
4. Add an overnight agent task that:
   - Runs doc integrity scans.
   - Suggests minimal doc patches with citations.
   - Never burns checklist tasks due to transient LLM failures.

## Appendix A: Templates

### A1) Investigation Log Skeleton

Create this in `docs/meta/logs/research/YYYY-MM-DD_strategy_doc_audit.md`:

1. Scope and corpus definition.
2. Time budget and completion criteria.
3. What changed since last audit (if applicable).
4. Findings (high signal only).
5. Conflicts discovered (doc vs doc, doc vs code).
6. Decisions required (with pointers to decision register entries).
7. Next actions (with pointers to checklists).

### A2) Doc Extraction Record (Per Document)

For each doc you read, record:

1. Path.
2. Purpose (1 sentence).
3. Canonical? (yes/no/unknown, and why).
4. Key claims (bullet list).
5. Implied assumptions (bullet list).
6. Dependencies (files/scripts/vendors).
7. Questions generated (IDs into the question bank).
8. Actions queued (checklist items or reconciliation plan entries).

### A3) Decision Register Entry Format

In `docs/meta/results/decision_register.md`:

1. Decision ID and title.
2. Context (what depends on it, with file pointers).
3. Options (A/B/C) with tradeoffs.
4. Default recommendation (if any) and rationale.
5. Chosen option, date, and evidence.
6. Follow-up tasks (checklist pointers).
