# Results (`docs/meta/results/`)

This directory is for captured outputs (for example: deep research runs, perf
reports, and other long-form artifacts) that are too large to live inline in
operator logs.

Related:
- Canonical docs index: `docs/index.md`
- Logs: `docs/meta/logs/`
- Prompts: `docs/meta/prompts/`

## Naming Scheme

Default pattern:

- `YYYY-MM-DD_HHMMSS_<slug>.md`

Deep research (two independent chats + synthesis):

- `YYYY-MM-DD_HHMMSS_<prompt>_a.md`
- `YYYY-MM-DD_HHMMSS_<prompt>_b.md`
- `YYYY-MM-DD_HHMMSS_<prompt>_synthesis.md`

Note: `scripts/deep_research_prompt.py --new-result` / `--new-a-b` creates these files with a timestamp prefix to avoid collisions.

Directory layout:
- Keep deep research captures flat under `docs/meta/results/` (no `deep_research/` subdirectory).

Where:
- `<slug>` / `<prompt>` are ASCII `lower_snake_case` and should be short and
  specific (example: `market_analysis`, `security_watch`, `perf_baseline`).
- If the same topic is run multiple times in a day, prefer creating a new file
  with a disambiguator (example: `..._v2`, `..._pm`) over rewriting history.

## Recommended Structure (Inside Each Result File)

Use a simple, scan-friendly structure so results can turn into actions quickly:

1. Executive brief
2. Findings (details)
3. Sources (links/citations)
4. Actions (concrete next steps)

Suggested template:

```markdown
# <Title>

Run date: YYYY-MM-DD
Prompt: <prompt name or file path>

## Executive brief
- ...

## Findings
### <Theme 1>
...

## Sources
- <url> (accessed YYYY-MM-DD)

## Actions
### 30 minutes
- ...
### 60 minutes
- ...
### 120 minutes
- ...
```

## Append-Only Convention

Treat result files as an audit trail.

Rules:
- Capture raw output first (verbatim). Do not rewrite it.
- If you need to add structure, summaries, pruning, or follow-up notes, append
  them *after* the raw capture (for example under a `## Notes` or `## Actions`
  section).
- Allowed edits are limited to fixing obvious typos/broken links (no meaning
  change) and redacting accidental secrets/tokens/personal data.

## Where To Paste (Raw Capture First)

Rule:
- Paste raw model output into the result file first (verbatim).
- Do not edit for style/clarity before capturing the raw output.
- Any pruning/formatting/summaries should be appended after the raw capture.

## Append-Only Log Entry Convention (Daily/Research Logs)

Whenever you capture a new result, append a short pointer entry to the relevant
log (usually the daily log, sometimes a research log). This makes results
discoverable later without re-reading the whole artifact.

Suggested template:

```markdown
## Result captured: <topic> (YYYY-MM-DD)

Artifacts:
- docs/meta/results/YYYY-MM-DD_HHMMSS_<slug>.md

Summary:
- <1-3 bullets>

Follow-ups:
- TODO: <next action>
```
