<!--
Paste this into ChatGPT "Deep research" mode.
Shortcut: `python3 scripts/deep_research_prompt.py --prompt deep_research_acquisition_targets.md`
-->

## When to Use Which Prompt

Use `deep_research_acquisition_targets.md` when you want a focused, citation-backed shortlist of plausible acquirers (and acquisition-adjacent strategic partners) for Tabulensis, including concrete outreach paths (who to contact, how to reach them, and what to say first). Run monthly/quarterly, and always before an outreach sprint or any acquisition/partnership conversations. For a broader weekly scan (market/demand, competitors, and light acquisition/distribution coverage), use `deep_research_market_analysis.md`; for competitor/pricing monitoring, use `deep_research_competitor_watch.md`; for distribution micro-experiments, use `deep_research_distribution_experiments.md`; for stack-relevant security/CVE monitoring, use `deep_research_security_watch.md`; and for ops dashboard API feasibility, use `deep_research_ops_dashboard_apis.md`.

Today is {{RUN_DATE}} (local) / {{RUN_DATETIME}}.

You are an AI research agent with web browsing.

## Product Context

This research supports Tabulensis (https://tabulensis.com): a desktop app + CLI that compares Excel workbooks (.xlsx/.xlsm) and Power BI packages (.pbix/.pbit) and produces a structured diff (including Power Query / M changes).

Primary objective: produce a short, high-likelihood list of acquirers (or near-term strategic partners) and a practical outreach plan that a tiny team can execute, with citations for every non-obvious claim.

Constraints:
- Assume a tiny team (often 1 operator). Prefer high-ROI targets and actions.
- Do not guess. If you cannot find evidence, say so and propose a fast verification path.
- Prefer information from the last 12-24 months for M&A / partnership signals; include older context only if it materially changes the thesis.
- Treat acquisition appetite as time-sensitive: include "as of" dates for key signals (recent acquisitions, product launches, job posts).
- Privacy: do not invent or infer private contact information. Prefer publicly listed contact routes, corporate dev/partnership pages, and publicly visible profiles.

## Research Tasks (What You Must Produce)

1. **Acquisition thesis + scoring rubric**
   - Define 4-8 acquirer categories relevant to Tabulensis (examples: BI governance, data/analytics devtools, Excel/Power BI ecosystem vendors, data quality/observability, version control/collaboration, security/compliance, consultancies/platform partners).
   - Define a scoring rubric (0-5 each, with 1-line definitions) for:
     - Strategic fit (product adjacency and roadmap fit)
     - Distribution leverage (do they reach our buyers/users?)
     - Ability to buy (size, pricing model, plausible deal size)
     - Demonstrated appetite (recent acquisitions/partnerships in the area)
     - Integration plausibility (how Tabulensis could slot into their product)
     - Deal friction risk (competitive conflicts, platform risk, trust/security concerns)

2. **Longlist (30-60) and prioritized shortlist (15-25)**
   - Build a longlist across categories, then produce a ranked shortlist.
   - For each shortlisted acquirer, include:
     - 2-4 sentence rationale (why the fit is real)
     - The most relevant product line(s) they have
     - 1-3 concrete "signals" (acquisitions, launch, marketplace activity, job posts, blog posts) with dates and links
     - What they likely want (technology, distribution wedge, SKU expansion, enterprise features, etc.)
     - Top 1-2 risks (why they might not buy)

3. **Top-10 deep dives (actionable, outreach-ready)**
   - For the top 10 targets, expand into a compact dossier that includes:
     - A crisp "why them / why now" thesis (5 bullets max)
     - Integration story: where Tabulensis fits (SKU/add-on/bundled feature) and what the first 90 days could look like
     - Comparable moves: 2-5 similar acquisitions/partnerships they did (or peers did) that indicate appetite
     - A one-paragraph "operator narrative" that can be reused in outreach

4. **Outreach map (paths, roles, and best first message)**
   - For each of the top 10 targets, propose the best first outreach path, ranked:
     - Warm intro vectors (investors, advisors, mutual partners, marketplaces, communities)
     - Formal routes (corp dev, partnerships page, "contact sales", partner program)
     - Direct routes (LinkedIn DM to the right role; founder-to-founder where appropriate)
   - For each target, identify relevant role titles and (only if publicly available) specific people:
     - Corp dev / M&A / strategy
     - Partnerships / alliances
     - Product leader for the adjacent product line
     - GTM leader who owns the buyer persona
   - Include direct links to the public pages/profiles you used.

5. **Messaging kit**
   - Draft 3 short outreach templates (<= 150 words each):
     - Corp dev / strategy intro
     - Product/partnerships intro
     - Founder-to-founder intro
   - For each template, provide:
     - 3 subject line options
     - 3 personalization hooks (what to reference from the target's recent signals)
     - A 1-line call to action (15-minute chat, forward to corp dev, etc.)

6. **Two-week outreach sprint plan (tiny-team realistic)**
   - Produce a plan that fits ~5 hours/week:
     - Top 10 targets, in priority order
     - Day-by-day checklist with concrete deliverables (1-pager, demo clip, target-specific angle, outreach sent, follow-up cadence)
     - What to measure (leading indicators) and when to stop or pivot

7. **Monitoring plan (low-maintenance)**
   - Recommend a lightweight watch list for acquisition appetite shifts:
     - URLs/feeds to monitor per category (press releases, corp dev pages, partner announcements, job searches)
     - Cadence (weekly/monthly) and trigger thresholds (what should prompt renewed outreach)

## Output Format (Markdown)

- ## Executive Brief (10 bullets max)
- ## Scoring Rubric
- ## Prioritized Acquisition/Partner Shortlist (table)
  - Columns: Rank | Target | Category | Fit thesis | Key signals (with dates) | Outreach path (best first) | Risks | Sources
- ## Top 10 Dossiers (one subsection per target)
  - Include: why now, integration story, comparable moves, and outreach notes.
- ## Outreach Map (table)
  - Columns: Target | Best route | Role titles to contact | Public links | Notes
- ## Messaging Kit (templates)
- ## Two-Week Outreach Sprint Plan (checklist)
- ## Monitoring Plan (URLs + cadence)
- ## Appendix: Sources (grouped)

## Citation Requirements

- Prefer primary sources:
  - Target company press releases, blogs, partner announcements, docs
  - Official product pages and marketplace listings
  - SEC filings / investor materials when relevant
- For each key signal, include the date and a direct link.
- If you use third-party databases (Crunchbase, PitchBook, etc.), treat them as secondary; confirm key claims with primary sources when possible.
- When summarizing long sources, include the link and a 1-line reason it matters.

