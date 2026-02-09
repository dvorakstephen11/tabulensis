<!--
Paste this into ChatGPT "Deep research" mode.
Shortcut: `python3 scripts/deep_research_prompt.py` (copies to clipboard).
-->

## When to Use Which Prompt

Use `deep_research_market_analysis.md` for the broad weekly scan (market/demand signals, competitor/adjacent watch, acquisition/partnership targets, and a high-level distribution sweep); use `deep_research_competitor_watch.md` for focused competitor/substitute monitoring (pricing/packaging/recent changes); use `deep_research_distribution_experiments.md` for a ranked backlog of 2-hour distribution micro-experiments with metrics; use `deep_research_security_watch.md` for stack-relevant security/CVE/supply-chain monitoring; and use `deep_research_ops_dashboard_apis.md` as needed while building an internal ops dashboard (Fastmail/Cloudflare/Resend/Stripe API feasibility).

Today is {{RUN_DATE}}.

You are doing deep research to support Tabulensis (https://tabulensis.com): a desktop app + CLI that compares Excel workbooks (.xlsx/.xlsm) and Power BI packages (.pbix/.pbit) and produces a structured diff (including Power Query / M changes).

Primary objective: maximize odds of acquisition and/or durable revenue growth by discovering concrete opportunities and risks, with citations.

Constraints:
- Assume a tiny team (often 1 operator). Prefer high-ROI actions.
- Do not guess. If you cannot find a fact, say so and propose how to verify.
- Focus on information from the last 30 days when possible; include older context only when it materially informs decisions.

Research tasks:
1. Market and demand signals (last 30 days):
   - Evidence of demand for spreadsheet/workbook diff, Power BI governance, and version-control workflows for analytics artifacts.
   - Any notable product launches, community threads, or enterprise pain points relevant to Tabulensis.
2. Competitor + adjacent tool watch:
   - Identify direct competitors and close substitutes (Excel compare tools, BI governance, data diff/versioning, git-for-data tools).
   - For each: positioning, pricing tier hints, distribution channels, and any recent changes/news.
3. Acquisition / partnership targets:
   - List 10-20 plausible acquirers or strategic partners.
   - For each: why the fit is real (product adjacency), signals they buy/build in this area, and the best first outreach path.
4. Distribution and marketing opportunities:
   - List 10-15 concrete channels/plays (directories, communities, newsletters, partnerships, marketplaces, integrations).
   - For each: why it fits, what to do in <= 2 hours, and how to measure success.
5. Risks and anti-goals:
   - Identify the top risks (platform shifts by Microsoft, commoditization, policy/IT constraints, trust/security concerns).
   - Recommend mitigations and what to avoid this week.

Output format (Markdown):
- ## Executive Brief (10 bullets max)
- ## Key Findings (with citations)
- ## Competitor/Adjacent Landscape (table preferred)
- ## Acquisition/Partnership Shortlist (ranked)
- ## High-ROI Distribution Experiments (ranked)
- ## Risks / Watchouts
- ## Today’s Checklist (30-60 minutes)
- ## Append-Only Research Log Entry (ready to paste)
  - Include: date, 5-10 bullets, and a compact "links" list.

## Citation Requirements

- Provide direct links for non-obvious claims.
- For any pricing or market-size numbers: cite the source and include the publication date.
- When summarizing long sources, include the link and a 1-line reason it matters.
