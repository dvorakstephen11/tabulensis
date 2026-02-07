---
name: tabulensis-docs
description: Navigate, audit, and maintain documentation in the Tabulensis (excel_diff) repo. Use when you need to find the right operating doc/runbook/checklist, update docs/index.md to reference new operating docs, refresh the auto-indexed checklist section, or normalize checklist formatting so unfinished work is discoverable.
---

# Tabulensis Docs

## Quick Start

1. Start at `docs/index.md` (doc map: operating docs, workflows, checklists).
2. If you add/rename a checkbox-style checklist, refresh the auto-indexed list in `docs/index.md`:
   - `python3 scripts/update_docs_index_checklists.py`
3. For code entry points, use `docs/maintainers/entrypoints.md`.

## Find The Right Doc (Task Map)

- Desktop app (from source): `docs/desktop.md` (also mirrored in `README.md`).
- Licensing backend (Worker + Rust service): `docs/licensing_service.md`.
- Stripe/Worker deployment and wiring: `STRIPE_WORKER_NEXT_STEPS.md` and `tabulensis-api/wrangler.jsonc`.
- License email delivery (Resend): `RESEND_SETUP_CHECKLIST.md`.
- Release process: `docs/release_checklist.md`, `docs/release_readiness.md`, `docs/release_signing.md`.
- Perf validation: `docs/perf_playbook.md` and `AGENTS.md` (major vs minor perf policy).
- Fixtures: `fixtures/README.md` and `docs/maintainers/fixtures.md`.
- UI visual regression: `docs/ui_visual_regression*.md` plus `scripts/ui_capture.sh` and `scripts/ui_snapshot_summary.py`.
- “Where is the code for X”: `docs/maintainers/entrypoints.md` first.

## Search Commands (No ripgrep assumed)

Search Markdown docs by keyword (exclude vendored/build outputs):

```bash
grep -RIn \
  --exclude-dir=target --exclude-dir=vendor --exclude-dir=tmp --exclude-dir=node_modules --exclude-dir=.git \
  --include='*.md' \
  'your keyword' .
```

List checkbox checklists + counts (and keep `docs/index.md` in sync):

```bash
python3 scripts/update_docs_index_checklists.py --print
python3 scripts/update_docs_index_checklists.py
```

## Maintaining `docs/index.md`

- If you add a new operating doc (SOP/runbook/checklist), add a link under the appropriate section in `docs/index.md`.
- If you add or heavily modify checkbox checklists, run `python3 scripts/update_docs_index_checklists.py` so the “Unfinished checklists” section stays accurate.
- Prefer consistent checkbox formatting: `- [ ]` for open, `- [x]` for done.

## When Editing Docs As An Agent

- Default to small, targeted doc edits.
- If you change workflows (perf cycle, fixture manifests, release steps), update both:
  - the workflow doc (e.g., `docs/perf_playbook.md`, `docs/maintainers/fixtures.md`)
  - the pointer surface (`docs/index.md`, and optionally `AGENTS.md` if it affects agent guardrails)
