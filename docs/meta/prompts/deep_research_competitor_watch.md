<!--
Paste this into ChatGPT "Deep research" mode.
Shortcut: `python3 scripts/deep_research_prompt.py --prompt deep_research_competitor_watch.md`
-->

## When to Use Which Prompt

Use `deep_research_competitor_watch.md` when you want a high-signal competitor + substitute watch for Tabulensis (positioning, pricing, packaging, and notable recent changes); run weekly or before making pricing/messaging decisions. For a broader market + demand + acquisition/partnership scan, use `deep_research_market_analysis.md`; for actionable distribution micro-experiments, use `deep_research_distribution_experiments.md`; for stack-relevant security/CVE monitoring, use `deep_research_security_watch.md`; and for ops dashboard API feasibility, use `deep_research_ops_dashboard_apis.md`.

Today is {{RUN_DATE}} (local) / {{RUN_DATETIME}}.

You are an AI research agent with web browsing.

## Product Context

This research supports Tabulensis (https://tabulensis.com): a desktop app + CLI that compares Excel workbooks (.xlsx/.xlsm) and Power BI packages (.pbix/.pbit) and produces a structured diff (including Power Query / M changes).

Primary objective: maintain a current, citation-backed view of direct competitors and close substitutes so we can make better product, pricing, and positioning decisions.

Constraints:
- Assume a tiny team (often 1 operator). Prefer high-ROI insights and actions.
- Do not guess. If you cannot find a fact, say so and propose how to verify.
- Prefer information from the last 30 days; include older context only when it materially affects conclusions.
- Treat pricing as time-sensitive: always include "as of" dates and link the exact pricing source.

## Research Tasks (What You Must Produce)

1. **Competitor + substitute universe**
   - Identify direct competitors and close substitutes for:
     - Excel workbook compare/diff
     - Power BI / analytics artifact versioning and comparison
     - "Git for data/analytics" style workflows (adjacent substitutes)
   - Output a prioritized list (top 10-20), grouped by category.

2. **Pricing + packaging intelligence (as of {{RUN_DATE}})**
   - For each prioritized tool, capture:
     - Price points and tiers (with billing period and currency)
     - Licensing model (per user, per device, per org, per file, etc.)
     - Trial/free tier limitations
     - Any enterprise/custom pricing signals
   - If pricing is not public, say so and provide the best-available proxy (for example marketplace listing, docs, FAQ, or sales page language), with citations.

3. **Recent changes and signals (last 30-90 days)**
   - For each of the top 10 (at minimum), find and summarize:
     - Product updates (release notes, changelogs, docs changes)
     - Pricing changes, packaging changes, or new SKUs
     - Distribution changes (new marketplaces, partnerships, notable content/SEO pushes)
     - Company signals (funding, acquisitions, major hires/job posts, roadmap hints)
   - Include dates and direct links for every "recent change" claim.

4. **Implications for Tabulensis**
   - What are the 3-7 biggest threats and 3-7 biggest opportunities?
   - Where is Tabulensis clearly differentiated, and where is it at risk of being commoditized?
   - Recommend 5-10 concrete next steps, with at least:
     - 3 actions that can be done in <= 2 hours
     - 2 medium actions (1-3 days)
     - 1 longer bet (1-4 weeks)

5. **Ongoing monitoring plan (low-maintenance)**
   - Propose a lightweight watch system for the top competitors:
     - Which URLs/feeds to watch (pricing page, changelog, GitHub releases, blog)
     - How to check it (RSS, email alerts, GitHub "releases only", monthly manual pass)
   - Prefer approaches that require minimal tooling and avoid brittle scraping.

## Output Format (Markdown)

- ## Executive Brief (10 bullets max)
- ## Competitor + Substitute Watchlist (table)
  - Columns: Product | Category | Target user | Packaging | Pricing (as of date) | Notes | Sources
- ## Pricing Snapshot (summary + notable anomalies)
- ## Recent Changes (last 30-90 days)
  - Group by product; include dates + links.
- ## Threats + Opportunities (ranked)
- ## Recommended Actions (ranked; include 2-hour actions)
- ## Monitoring Plan (concrete watch URLs + cadence)
- ## Append-Only Research Log Entry (ready to paste)
  - Include: date, 5-10 bullets, and a compact "links" list.

## Citation Requirements

- Prefer official sources (pricing pages, docs, changelogs, GitHub releases) over third-party summaries.
- Every non-obvious claim must have a direct link.
- For pricing: include the source link, the "as of" date, and note any region/billing assumptions.
- When summarizing long sources, include the link and a 1-line reason it matters.
