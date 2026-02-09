# Marketing Templates (`docs/meta/marketing_templates/`)

This directory contains reusable copy/asset templates so posting and outreach can
be done quickly and consistently.

Related:
- Marketing activity logs: `docs/meta/logs/marketing/`
- Deep research prompts (distribution/market analysis): `docs/meta/prompts/`

## What Each Template Is For

- `post_short_demo.md`: a short (30-60s) demo clip post (hook + bullets + CTA).
- `post_before_after.md`: a before/after “diff clarity” post (two images + captions).
- `post_user_story.md`: a persona-driven story post (pain -> “Tabulensis fixes” -> CTA).

## Where Generated Assets Live

Suggested conventions:
- Raw source video/screen recordings: local-only (outside repo) or `tmp/` (never commit).
- Committed artifacts:
  - Post copy drafts: append to `docs/meta/logs/marketing/YYYY-MM-DD_<slug>.md`
  - Metrics tracking: `docs/meta/logs/marketing/metrics.csv`

## Minimum Demo Video Format (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0013`).

Minimum repeatable format:
- 30-60 second "hook" screen recording.
- Captions on (burned-in or platform captions).
- Show one end-to-end workflow (start -> key moment -> result) without explaining every control.
- Publish targets: landing page asset + YouTube.

## How To Measure Results (Minimal)

For each post/outreach event, record:
- date
- channel (X/LinkedIn/Reddit/etc.)
- link/post_id
- impressions
- clicks
- conversions (if known)
- notes (what you changed)

### Canonical UTM Scheme (Chosen)

Decision: `docs/meta/results/decision_register.md` (`DR-0012`).

Parameters (always include the first 3):
- `utm_source`: platform/referrer (examples: `x`, `linkedin`, `reddit`, `hn`, `github`, `newsletter`)
- `utm_medium`: channel class (examples: `social`, `community`, `email`, `ad`, `referral`)
- `utm_campaign`: post/experiment family (examples: `demo_short`, `before_after`, `user_story`, `perf`)
- `utm_content`: optional variant id (examples: `clip_01`, `v2`, `image_a`)

Rules:
- Use ASCII `[A-Za-z0-9_-]` only (no spaces); keep values short and stable.
- Prefer consistent slugs over “clever” names so metrics stay comparable.

Generate a UTM link:

```bash
python3 scripts/utm.py --source x --medium social --campaign demo_short --content clip_01 --copy
```

### Minimal Metrics File

Append marketing outcomes to:
- `docs/meta/logs/marketing/metrics.csv`

This file is intentionally boring: one row per post/outreach event, updated daily/weekly.
