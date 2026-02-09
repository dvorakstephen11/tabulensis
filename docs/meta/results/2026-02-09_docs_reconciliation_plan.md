# Docs Reconciliation Plan (2026-02-09)

Run date: 2026-02-09

Goal: list minimal, specific doc edits required to make the operating system consistent, discoverable, and executable (without wide churn).

Note: during this audit run, several “proposed” reconciliations were applied in
the working tree to remove obvious doc-vs-repo mismatches. Remaining items are
kept below as a forward plan.

## Reconciliations Applied (This Run)

1. `meta_methodology.md`
- Applied:
  - Created `docs/meta/today.md` template (now gitignored; see below).
  - Added concrete helper commands + deep research capture instructions.
- Remaining:
  - (none)

2. `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`
- Applied:
  - Added pointers to `docs/meta/results/decision_register.md` for key decision gates.
  - Checked off items that are already implemented in-repo (to reduce false “open” work).

3. `docs/meta/README.md`
- Applied:
  - Added a Decision Register pointer and surfaced high-impact pending decisions.
- Remaining:
  - (none; `DR-0001`/`DR-0002` now recorded)

4. `docs/index.md`
- Status:
  - `docs/meta/today.md` is local-only (gitignored); committed template is `docs/meta/today.example.md` (and appears in the auto-indexed checklist block).
  - Optional improvement: add an explicit Operating-docs link to `docs/meta/today.example.md`.

5. `docs/meta/logs/README.md` + `docs/meta/results/README.md`
- Applied:
  - Updated naming scheme docs to match tooling (timestamp-prefixed filenames).
  - Added an explicit “Where to paste (raw capture first)” rule to `docs/meta/results/README.md`.
  - Recorded the filename scheme decision in `docs/meta/results/decision_register.md` (`DR-0004`).

6. `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`
- Applied (partial):
  - Created missing automation scaffolding docs under `docs/meta/automation/`.
- Remaining:
  - (none; marketing templates scaffolding created)

7. `docs/meta/automation/` docs
- Applied:
  - Created `docs/meta/automation/README.md`, `docs/meta/automation/guardrails.md`, and `docs/meta/automation/model_accounts.md`.

8. Marketing measurement scaffolding
- Applied:
  - Chosen and documented a canonical UTM scheme:
    - `docs/meta/marketing_templates/README.md`
    - decision recorded: `docs/meta/results/decision_register.md` (`DR-0012`)
  - Implemented a UTM URL generator:
    - `scripts/utm.py`
  - Created minimal metrics file:
    - `docs/meta/logs/marketing/metrics.csv`
  - Checked off checklist items in `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (Section 8.4).

9. Security minimal recurring checks
- Applied:
  - Created `SECURITY_DAILY_CHECKLIST.md` and linked it from `docs/index.md`.
  - Implemented wrapper report script: `scripts/security_audit.sh`.
  - Recorded chosen tools in `docs/meta/automation/guardrails.md` and decision register (`DR-0016`).
  - Checked off checklist items in `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (Section 12).

10. Broken-link hygiene
- Applied:
  - `meta_methodology.md` now points to `docs/meta/today.example.md` (template) instead of linking to gitignored `docs/meta/today.md`.
  - `docs/meta/prompts/README.md` “Where results go” example updated to match timestamped filenames.
  - `docs/release_signing.md` updated with a “Current state” section to reflect CI reality.

11. Docs integrity tooling (Phase 8 partial)
- Applied:
  - Added `scripts/docs_integrity.py`:
    - validates `docs/index.md` local links and checklist-index drift
    - can surface decision-gate hotspots via `--decision-gates`
  - Documented usage in `docs/meta/automation/README.md` ("Docs Integrity Checks").

## Validation Steps (must re-run after edits)

- `python3 scripts/update_docs_index_checklists.py`
- `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml doctor`
- `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml list-tasks --limit 30`
- `python3 scripts/deep_research_prompt.py --help`

Validation executed in this run:
- `python3 scripts/update_docs_index_checklists.py`
- `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml doctor`
- `python3 scripts/overnight_agent.py --config docs/meta/automation/overnight_agent.yaml list-tasks --limit 30`
- `python3 scripts/deep_research_prompt.py --help`
- `python3 scripts/update_docs_index_checklists.py` (implementation updated to respect `.gitignore`)

## Remaining / Deferred (Next Iteration)

These items were surfaced during the audit but intentionally deferred to keep churn minimal:

- Pro context payload generator + output location decision:
  - `DR-0007` chosen: payload outputs default to `tmp/pro_context_payload.md` (local-only).
  - Implement generator (see `docs/meta/prompts/pro_context_payload.md`).
- Automation dispatch scaffolding:
  - `docs/meta/automation/tasks.yaml` + dispatch script + scheduler docs (see checklist Section 6/7).
- Release signing/notarization wiring:
  - `docs/release_signing.md` now reflects current state; next step is wiring secrets + CI steps.
- Security audit tooling installs (local machine):
  - `cargo-audit`, `gitleaks`, and `semgrep` are currently missing locally (see `docs/meta/logs/ops/2026-02-09_security_audit.md`).
