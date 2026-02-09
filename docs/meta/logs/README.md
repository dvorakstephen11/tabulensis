# Logs (`docs/meta/logs/`)

This directory is the canonical home for operator logs: what happened, what we
decided, and pointers to outputs (reports, artifacts, code changes).

Logs should be easy to scan, easy to append to, and safe to commit (avoid
secrets, tokens, or personal data).

## Subdirectories

- `daily/`: One file per calendar day; the operator's "journal + receipts".
- `research/`: Shorter research session logs (questions, sources, findings,
  next experiments).
- `ops/`: Operational logs and automated agent journal outputs (run reports,
  incident notes, blocked decisions).
- `weekly/`: Weekly review logs (wins/misses/next-week priorities).
- `marketing/`: Marketing activity logs (metrics, outreach, experiments).

## Naming Conventions

Daily logs:

- `docs/meta/logs/daily/YYYY-MM-DD.md`

Research logs:

- `docs/meta/logs/research/YYYY-MM-DD_<slug>.md`

Ops logs:

- `docs/meta/logs/ops/YYYY-MM-DD_<slug>.md`
- Overnight agent journal (append-only / per-run artifacts):
  - `docs/meta/logs/ops/executive_summary.log`
  - `docs/meta/logs/ops/<run_id>_report.md`
  - `docs/meta/logs/ops/<YYYY-MM-DD>_questions_for_operator.md`

Weekly review logs:

- `docs/meta/logs/weekly/YYYY-WW.md` (ISO week number, `WW` is 01-53)

`<slug>` rules:

- Use ASCII `lower_snake_case` (example: `acquisition_targets`, `stripe_debug`).
- Keep it short and specific; avoid "misc".
- If a topic evolves, create a new file and cross-link (avoid renames).

Deep research captures (results, not logs):

- Store large pasted model outputs under `docs/meta/results/`.
- Canonical filenames (prompt + multi-chat + synthesis):
  - `docs/meta/results/YYYY-MM-DD_HHMMSS_<prompt>_a.md`
  - `docs/meta/results/YYYY-MM-DD_HHMMSS_<prompt>_b.md`
  - `docs/meta/results/YYYY-MM-DD_HHMMSS_<prompt>_synthesis.md`

Note: `scripts/deep_research_prompt.py --new-a-b` creates these files with a timestamp prefix to avoid collisions.

## Append-Only Rules

Logs are an audit trail. Prefer appending to the end over rewriting history.

Allowed edits:

- Fix obvious typos and broken links (no meaning change).
- Redact accidental secrets, tokens, or personal data (treat as an incident).

Corrections:

- If you need to change a decision or correct facts, append a new "Correction"
  entry with a timestamp and a pointer to the original text.

## Linking Rules (Checklists + Code)

Every meaningful log entry should contain pointers so it can be audited later:

- Checklist/task references: `path:line` (or `path#Lnn`) plus the checkbox text.
  - Example: `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md:55`.
- Code change references: branch name + commit SHA (or PR number).
- Output artifact references: repo-relative file paths (reports, benchmarks,
  screenshots, etc.).

Rule of thumb: if you could not re-derive the work in 30 minutes from the log,
it needs more pointers.

## Canonical Inputs To Daily Planning

When generating a daily plan (or updating `docs/meta/today.md`), prefer these
inputs (rough order):

- `meta_methodology.md`
- `docs/index.md` "Unfinished checklists" auto-index block
- `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md` (open items)
- `todo.md`
- `product_roadmap.md`
- Most recent `docs/meta/logs/daily/*.md`
