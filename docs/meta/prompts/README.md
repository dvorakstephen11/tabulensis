# Prompt Library (`docs/meta/prompts/`)

This directory is the source of truth for reusable prompts used to operate Tabulensis.

## Copying Prompts Into ChatGPT

Use the repo helper to copy a prompt to clipboard with date placeholders filled:

```bash
python3 scripts/deep_research_prompt.py
```

Examples:

```bash
# List available short names.
python3 scripts/deep_research_prompt.py --list

# Copy by short name (maps to deep_research_<name>.md under docs/meta/prompts/).
python3 scripts/deep_research_prompt.py --prompt market_analysis

# Copy a specific prompt (accepts either a repo-relative path or a filename in docs/meta/prompts/).
python3 scripts/deep_research_prompt.py --prompt deep_research_ops_dashboard_apis.md

# Print instead of copying (useful for piping / redirecting).
python3 scripts/deep_research_prompt.py --print --prompt deep_research_market_analysis.md
```

Notes:
- `{{RUN_DATE}}` is replaced with the current date (ISO format).
- `{{RUN_DATETIME}}` is replaced with the current local datetime (ISO format, seconds).

## Where Results Go

Save deep research outputs (including links/citations) under:
- `docs/meta/results/**`

Prefer timestamp-prefixed, append-only files (for example: `docs/meta/results/YYYY-MM-DD_HHMMSS_<topic>.md`).

## Prompt Index

| File | Intended use | Cadence |
| --- | --- | --- |
| [`daily_plan.md`](daily_plan.md) | Turn SOPs + open checklists + recent logs into a small, timeboxed "today plan" (dependency-aware, with stop conditions) ready to paste into `docs/meta/today.md`. | Daily (during "Daily open"). |
| [`pro_context_payload.md`](pro_context_payload.md) | Spec for the "Ultimate Context Payload" generator (required sections, size limits, deterministic ordering). | As needed (before major planning/review sessions; when updating payload tooling). |
| [`deep_research_market_analysis.md`](deep_research_market_analysis.md) | Market/demand signals, competitor/adjacent watch, acquisition/partnership targets, and a high-level distribution sweep; includes an append-only research log entry for pasting. | Weekly (or before setting weekly priorities / outreach); run ad hoc when conditions change. |
| [`deep_research_acquisition_targets.md`](deep_research_acquisition_targets.md) | Focused acquisition/strategic-partner scan: plausible acquirers, concrete outreach paths (who/where/how), and a messaging + sprint plan, with citations. | Monthly/quarterly and before any acquisition/partnership outreach sprint. |
| [`deep_research_competitor_watch.md`](deep_research_competitor_watch.md) | Focused competitor + substitute watch: pricing, packaging, and notable recent changes; outputs a watchlist, change log, and ranked actions. | Weekly (or before pricing/positioning/messaging changes). |
| [`deep_research_security_watch.md`](deep_research_security_watch.md) | Security news + supply chain incidents + CVEs relevant to the Tabulensis stack; outputs prioritized mitigations and a low-maintenance monitoring plan. | Weekly (and immediately after major upstream incidents). |
| [`deep_research_distribution_experiments.md`](deep_research_distribution_experiments.md) | High-ROI distribution channels and a ranked backlog of 2-hour micro-experiments, with metrics + measurement plan. | Weekly while actively iterating on growth/distribution; otherwise monthly or ad hoc before an outreach sprint. |
| [`deep_research_ops_dashboard_apis.md`](deep_research_ops_dashboard_apis.md) | What operator-relevant data is accessible via Fastmail/Cloudflare/Resend/Stripe APIs, and how to structure an internal dashboard (MVP widgets + security posture). | As needed while building the dashboard; revisit quarterly or when vendor plans/APIs change. |
| [`deep_research_synthesis.md`](deep_research_synthesis.md) | Synthesize two deep research outputs (A/B) into a single "Today’s Checklist" with 30/60/120-minute actions (plus a post-synthesis operator checklist). | After running deep research in two chats (or two runs) and before updating `docs/meta/today.md`. |

## Other Prompt-Related Tools

- `generate_review_context.py`: collates planning/review bundles and generates context markdown for model-assisted planning/review. Cadence: per major implementation cycle or before a design review.
