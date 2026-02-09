<!--
Paste this into ChatGPT "Deep research" mode.
Shortcut: `python3 scripts/deep_research_prompt.py --prompt deep_research_distribution_experiments.md`
-->

## When to Use Which Prompt

Use `deep_research_distribution_experiments.md` when you want a practical, ranked backlog of distribution experiments with specific channels/placements (links), step-by-step actions executable in <= 2 hours, and clear success metrics + measurement; run weekly during active growth/distribution sprints, otherwise monthly or ad hoc before an outreach push. For a broader market + demand + acquisition/partnership scan, use `deep_research_market_analysis.md`; for competitor/pricing monitoring, use `deep_research_competitor_watch.md`; for stack-relevant security/CVE monitoring, use `deep_research_security_watch.md`; and for ops dashboard API feasibility, use `deep_research_ops_dashboard_apis.md`.

Today is {{RUN_DATE}} (local) / {{RUN_DATETIME}}.

You are an AI research agent with web browsing.

## Product Context

This research supports Tabulensis (https://tabulensis.com): a desktop app + CLI that compares Excel workbooks (.xlsx/.xlsm) and Power BI packages (.pbix/.pbit) and produces a structured diff (including Power Query / M changes).

Primary objective: identify high-ROI distribution channels and propose concrete micro-experiments we can run quickly (tiny team, often 1 operator), with citation-backed rationale.

Constraints:
- Assume minimal bandwidth and minimal tooling. Prefer experiments that do not require new infrastructure.
- Every proposed experiment must be launchable in <= 2 hours (execution time, not counting "wait for results").
- Do not guess. If you cannot find evidence, say so and propose how to verify quickly.
- Prefer signals from the last 30-90 days when possible; include older context only when it materially changes recommendations.

## Research Tasks (What You Must Produce)

1. **Channel map (specific, not generic)**
   - Identify 10-15 promising distribution channels for Tabulensis, spanning at least:
     - Directories/marketplaces/listings
     - Communities (forums, subreddits, Slack/Discord, professional groups)
     - Content/SEO (keywords, "jobs to be done" queries, comparison pages)
     - Partnerships (consultancies, training providers, agencies, Git tooling vendors)
     - Integrations and ecosystems (Git, Power BI governance, Excel tooling workflows)
     - Paid options (only where likely to work; include constraints)
   - For each channel, list 2-5 concrete placements (named sites/communities) with direct links and a sentence of evidence for why it is a fit (audience, topic match, activity level, prior similar posts).

2. **Ranked experiment backlog (2-hour setup/action only)**
   - Propose 12-20 experiments, each tied to one channel/placement.
   - For each experiment, include:
     - Hypothesis (1 sentence)
     - Target persona (who will convert and why)
     - Exact placement (URL + where/how to post or list)
     - **2-hour action plan** (step-by-step checklist; include any copy you recommend drafting)
     - Assets needed (screenshots, a 30s demo GIF, a comparison page, etc.) and the fastest way to produce each
     - Measurement plan (UTM scheme, unique landing page suggestion, coupon/promo code if relevant, and how to attribute)
     - Metrics (leading + lagging) and a time window for evaluation (for example 48h, 7d)
     - Success threshold (what would make us keep investing vs stop)
     - Risks/downsides (reputation/spam risk; platform rules; time sink)
     - Cost (time + expected cash spend if any)

3. **Metrics definition (make it measurable without a big analytics stack)**
   - Propose a minimal funnel we can measure for Tabulensis and define each metric clearly, including formulas:
     - Impression/Reach (if available)
     - Clicks to site (UTM-tagged)
     - Download intent (clicks on download)
     - Checkout start (if applicable)
     - Purchase (Stripe checkout completion)
     - Activation (if applicable)
   - Include recommended UTM conventions (source/medium/campaign/content) and examples.
   - If a metric cannot be measured reliably without adding tooling, say so and propose a lowest-effort proxy.

4. **"Do next" shortlist**
   - Pick the top 5 experiments to run next week (ranked), and for each:
     - Why it's top-5 (evidence + expected impact)
     - The exact 2-hour plan (condensed)
     - The metric to check first (leading indicator) and when

## Output Format (Markdown)

- ## Executive Brief (10 bullets max)
- ## Channel Map (with links)
  - Provide a table: Channel | Placements (links) | Why it fits | Notes/rules
- ## Ranked 2-Hour Experiments (table)
  - Columns: Rank | Experiment | Channel/Placement | 2-hour actions | Metrics | Success threshold | Est. cost | Risks | Sources
- ## Measurement Plan (UTMs + minimal funnel)
- ## Top 5 to Run Next Week (detailed checklists)
- ## Appendix: Links (grouped by channel)

## Citation Requirements

- Every placement/community/directory you name must include a direct link.
- Any claim about audience size, activity, pricing, or performance expectations must include a citation and the "as of" date.
- Prefer official sources and primary evidence (community rules pages, directory submission pages, platform docs, recent posts/threads).
- When summarizing long sources, include the link and a 1-line reason it matters.
