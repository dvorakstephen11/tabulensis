# Meta Docs (`docs/meta/`)

This directory contains operator-facing documentation and artifacts used to run
Tabulensis day-to-day, including references and inputs for automation (for
example the overnight operator agent).

## Start Here

- Canonical docs entrypoint: [`docs/index.md`](../index.md)
- Daily operator routine (source of truth): [`meta_methodology.md`](../../meta_methodology.md)
- Repo guardrails/policies: [`AGENTS.md`](../../AGENTS.md)
- If asked to produce a daily plan or prioritize work, read:
  - [`meta_methodology.md`](../../meta_methodology.md)
  - `docs/index.md` "Unfinished checklists" block

## What Lives Here

- `automation/`: overnight agent plan/runbook/config.
  - Plan: [`automation/overnight_operator_agent_plan.md`](automation/overnight_operator_agent_plan.md)
  - Runbook: [`automation/overnight_agent_runbook.md`](automation/overnight_agent_runbook.md)
  - Config: [`automation/overnight_agent.yaml`](automation/overnight_agent.yaml)
- Strategy + documentation audit plan: [`strategy_audit_plan.md`](strategy_audit_plan.md)
- `prompts/`: reusable prompt library and prompt tooling.
  - Index: [`prompts/README.md`](prompts/README.md)
- `logs/`: operator logs (append-only journal + receipts).
  - Conventions: [`logs/README.md`](logs/README.md)
- `results/`: captured outputs too large to live inline in logs.
  - Conventions: [`results/README.md`](results/README.md)
- `today.md`: local-only single-day scratchpad (overwritten frequently; not durable history).
  - Template: [`today.example.md`](today.example.md)
  - Local file (gitignored): `docs/meta/today.md`
- `audio/`: generated TTS audio for syntheses/notes (local-first).
  - Generate: `python3 scripts/tts_generate.py docs/meta/results/<result>.md`
  - Policy: local TTS only (no cloud TTS credentials in repo).

## Decision Register

Canonical decision log:
- `docs/meta/results/decision_register.md`

High-impact decisions:
- `docs/meta/logs/**` + `docs/meta/results/**` are committed to git (with redaction discipline). (`DR-0001`)
- `docs/meta/today.md` is local-only (gitignored); use `docs/meta/today.example.md` as template. (`DR-0002`)
- Pro context payload outputs default to `tmp/pro_context_payload.md` (local-only). (`DR-0007`)
- Canonical UTM scheme + metrics file live under meta marketing logs/templates. (`DR-0012`)
- Minimum security recurring checks + tools are recorded in automation guardrails. (`DR-0016`)

## Privacy + Redaction (Must Follow)

The files under `docs/meta/**` are intended to be safe to commit.

Never commit:
- API keys, tokens, credentials, or `.env` contents
- Vendor exports containing personal data (emails, addresses, payment details)
- Customer data or proprietary workbooks/PBIXs

If a secret is accidentally committed:
- Treat it as an incident and redact immediately (append a correction entry to the relevant log).

## Guardrails (Summary)

Authoritative source: [`AGENTS.md`](../../AGENTS.md) (this section is intentionally a brief
reminder; do not fork policy here).

- Documentation sprawl:
  - If you add/rename an operating doc or checklist, also add/update its link in
    [`docs/index.md`](../index.md).
  - If you add a checkbox-style checklist, prefer `- [ ]` / `- [x]` items so it
    shows up in the auto-index.
- Perf validation:
  - Run the full perf cycle only for major perf-risk changes (see triggers in
    [`AGENTS.md`](../../AGENTS.md)).
  - Full perf cycle commands:
    - `python3 scripts/perf_cycle.py pre`
    - `python3 scripts/perf_cycle.py post --cycle <cycle_id>`
  - Lighter checks for routine changes:
    - `python scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv`
    - Optional gate suite for large-grid/streaming paths:
      `python scripts/check_perf_thresholds.py --suite gate --parallel --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests`
- Formatting scope:
  - Avoid `cargo fmt --all` for targeted changes.
  - Prefer file/crate-scoped formatting or `python3 scripts/safe_rustfmt.py`.
- Fixtures:
  - Avoid `generate-fixtures --clean` unless intentionally resetting fixture
    sets; prefer manifest-specific generation for perf e2e fixtures.
- Before commit (blast-radius guards):
  - `python3 scripts/check_line_endings.py --staged`
  - `python3 scripts/check_change_scope.py --staged`
  - If committing perf artifacts: `python3 scripts/check_perf_cycle_scope.py --staged`
- Safety:
  - No deploys.
  - No secret rotation.
  - No destructive git operations (no `reset --hard`, no force push, etc.).

## Common Commands (From `AGENTS.md`)

- Open desktop app (from source): `cargo run -p desktop_wx`
- Optimized build: `cargo run -p desktop_wx --profile release-desktop`
- Capture + sanity-check legacy UI screenshots (headless, deterministic):
  - Capture: `./scripts/ui_capture.sh compare_grid_basic --tag <tag>`
  - Summarize/validate run metadata:
    `python3 scripts/ui_snapshot_summary.py desktop/ui_snapshots/compare_grid_basic/runs/<tag>.json`
  - Artifacts:
    - Screenshot: `desktop/ui_snapshots/<scenario>/runs/<tag>.png` (plus `current.png`)
    - Ready metadata: `desktop/ui_snapshots/<scenario>/runs/<tag>.ready.json` (plus `current_ready.json`)
    - Log: `desktop/ui_snapshots/<scenario>/runs/<tag>.log`

## Docs Index Maintenance

- [`docs/index.md`](../index.md) is the canonical entrypoint for Tabulensis docs.
- [`docs/index.md`](../index.md) includes an auto-indexed list of unfinished checkbox
  checklists; refresh it with:
  - `python3 scripts/update_docs_index_checklists.py`
