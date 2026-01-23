## 1. Market Sizing (TAM / SAM / SOM)

### 1.1 Observed market today

From the competitor revenue triangulation, the “visible” Excel comparison / spreadsheet risk market is small but real. The seven best‑profiled commercial players have the following estimated annual revenue: 

| Vendor         | Segment focus                      | Est. annual revenue (USD) |
| -------------- | ---------------------------------- | ------------------------- |
| Ablebits       | Productivity & operations          | $3–5M                     |
| Beyond Compare | Developer utilities (generic diff) | $1.5–3M                   |
| PerfectXL      | Governance & audit (SRM)           | $1–3M                     |
| xltrail        | Governance + developer / Git       | $0.5–1.5M                 |
| Synkronizer    | Productivity / legacy Excel diff   | $0.5–0.8M                 |
| xlCompare      | Developer / prosumer               | $0.3–0.6M                 |
| DiffEngineX    | Legacy utility                     | $0.1–0.2M                 |

Low–high sum: **$6.9–14.1M**, midpoint ≈ **$10.5M**. 

But:

* Only a fraction of **Ablebits** and **Beyond Compare** revenue is actually tied to Excel diff use cases (Ultimate Suite has 70+ tools; BC is a generic file diff).
* The broader ecosystem includes additional governance tools (ExcelAnalyzer, 4 Tops, CIMCON XLAudit, TeamMate Analytics, Power BI Sentinel, etc.) with partial overlap but not fully sized in the table.

A reasonable adjustment:

* “Core Excel diff + spreadsheet governance” revenue of these 7 vendors after stripping out non‑diff features ≈ **$5–8M** (midpoint ≈ $5.6M).
* Adding the rest of the ecosystem (ExcelAnalyzer, 4Tops, XLAudit/EUC, Power BI tools like Sentinel, ALM Toolkit, Tabular Editor licenses used for diffing) plausibly adds another **$5–12M**.

**Conclusion: current realized market size:**
**~$10–20M/year** globally for tools whose primary value proposition is Excel / Power BI comparison, versioning, and spreadsheet risk management.

This is the *proven* spend today, not the potential.

---

### 1.2 Top‑down potential from the Microsoft ecosystem

Relevant macro datapoints:

* Excel has an estimated **1.1–1.5B users worldwide**.
* Microsoft 365 has **~345M paid commercial seats** and >3.7M business customers.
* Power BI is used by **97% of the Fortune 500**, with >100k organizations and tens of millions of monthly active users, in a BI market projected to exceed **$50B by 2032**.

We care about *authors*, not viewers:

**Assumption ladder (conservative):**

1. Only **5%** of Microsoft 365 commercial seats are “heavy Excel/Power BI model authors” ⇒
   345M × 5% ≈ **17M authors**.
2. Only **1–2%** of those authors will ever pay (directly or via their org) for specialized diff / governance tools ⇒
   170k–340k potential paying “power seats”.
3. Realistic long‑term ARPU for this category at the **seat level** (dev tools + governance dashboards) is **$150–250/year** (consistent with xltrail, PerfectXL, ExcelAnalyzer pricing).

That gives a *theoretical* annual revenue ceiling of roughly:

* 170k seats × $150 = $25.5M
* 340k seats × $250 = $85M

So a **reasonable long‑run TAM band is ~$40–80M/year**, which lines up with “current $10–20M market + growth and better penetration” rather than a huge step change.

---

### 1.3 Segmenting the market

The competitor intelligence already split the landscape into three sectors:

1. **Governance & Audit (SRM/EUC)**

   * PerfectXL, ExcelAnalyzer, CIMCON XLAudit, TeamMate Analytics, portions of xltrail.
   * High ACV (5–6 figures), low seat count, deep integration with audit/compliance.

2. **Developer & Automation**

   * xltrail (Git), xlCompare, Beyond Compare, Tabular Editor, ALM Toolkit.
   * Focus on Git, CI/CD, headless automation, dev tooling.

3. **Productivity & Operations**

   * Ablebits, Synkronizer, 4 Tops, DiffEngineX, legacy utilities.
   * Focus on list reconciliation, business operations, high‑volume low‑ARPU.

Using competitor revenues plus qualitative estimates:

| Segment                   | Examples                                         | Current realized spend (est.) | 5–7 year *segment TAM* |
| ------------------------- | ------------------------------------------------ | ----------------------------- | ---------------------- |
| Governance & Audit        | PerfectXL, ExcelAnalyzer, XLAudit, xltrail (gov) | $7–12M                        | **$20–25M**            |
| Developer & Automation    | xltrail (Git), xlCompare, BC (Excel share), TE3  | $3–6M                         | **$15–20M**            |
| Productivity & Operations | Ablebits (diff slice), Synkronizer, 4Tops, misc  | $4–8M                         | **$15–25M**            |

**Total 5–7 year category TAM:** ≈ **$50–70M/year**
This matches the earlier top‑down bound.

---

### 1.4 SAM – what this product can realistically serve

The planned product is explicitly optimized for:

* **Platform ubiquity:** Rust core, native on Win/macOS/Linux plus Web via WASM; not tied to COM.
* **Semantic depth:** Power Query M extraction and AST‑level diff now; DAX/data model diff later.
* **Workflow modernization:** CLI and Git/CI integration as first‑class; shareable web diffs; Mac‑first launch.

That maps primarily to:

* Most of the **Developer & Automation** segment.
* A meaningful slice of **Governance & Audit** where teams are moving toward “Modern Excel / Power BI + CI” instead of pure desktop SRM.
* Only a *small* sliver of Productivity/Operations (the product isn’t trying to be Ablebits).

**Serviceable Addressable Market (SAM):**

* ~80–100% of Developer & Automation TAM: **$12–20M**.
* ~30–40% of Governance & Audit TAM (modern / cross‑platform buyers): **$6–10M**.
* Plus maybe ~$1–3M from power‑user productivity buyers that care about Git/semantic diff.

So **SAM ≈ $18–25M/year**, centred on dev tools plus “modern” governance.

---

### 1.5 SOM – realistic 5‑year share

Given:

* Current leading specialists (PerfectXL, Ablebits) sit around **$3–5M** each.
* xltrail’s niche, high‑ARPU SaaS sits around **$0.5–1.5M**.
* The new product is *one vendor* entering a fragmented market with incumbents who have 10–20 years of SEO and customer relationships.

A **realistic 5‑year SOM band**:

* **Conservative:** ~$2.5M ARR by Year 5 (~10–14% of SAM).
* **Base:** ~$4.5M ARR by Year 5 (~18–25% of SAM).
* **Optimistic:** ~$8M ARR by Year 5 (~30–40% of SAM).

Those are the revenue targets used in the projections below.

---

## 2. Competitive Positioning Assessment

### 2.1 Segments most likely to be disrupted

**Most disruption potential**

1. **Developer & Automation sector**

   * Current state: xlCompare and Beyond Compare give developers *something* for Excel, but lack deep semantics and cross‑platform support.
   * xltrail offers Git integration and a web diff, but is focused on Windows/Excel and doesn’t deeply understand PBIX, DAX, or non‑Windows workflows.
   * Your engine: Rust/WASM, deterministic cross‑platform JSON diff, CLI, and CI‑ready APIs directly target this gap.

   **Expect:** fastest adoption here, especially among:

   * Finance/analytics teams already using GitHub / Azure DevOps.
   * Power BI teams struggling to diff PBIX models.

2. **Modern governance / mid‑market SRM**

   * PerfectXL and ExcelAnalyzer focus on static analysis with deep semantics but are Windows/COM‑bound and split into multiple tools.
   * A cross‑platform semantic engine that unifies diff, lineage graph, and risk views into a single UI can siphon off teams that:

     * Don’t need Big Four‑level audit sign‑off.
     * Do need Git/CI integration and BI‑aware semantics.

**Less disruption potential**

* **Mass productivity tools (Ablebits, Synkronizer)**
  These compete on Windows Excel convenience and a giant “Swiss Army knife” of utilities. Trying to out‑feature them at $99/license would be a losing battle and your differentiation plan explicitly warns against this.

---

### 2.2 Timeline to feature parity by segment

Anchoring to your phased roadmap: core engine months 1–6, Mac/Web viewer months 7–12, “Modern Excel” (M & DAX) months 13–18, governance/workflow months 19+.

Assuming work starts around now and proceeds more or less as planned:

1. **Developer/automation parity with xlCompare / Beyond Compare (for Excel grid)**

   * **MVP parity:** grid diff + structure diff + CLI on Win/macOS/Linux. Achievable within **12–18 months**, assuming H1 (grid diff) and H2 (streaming parsing) land on schedule.
   * **Superior offering:** once M diff and PBIX diff are stable (Phase 3–4), the entrant becomes clearly superior for “Excel as code” teams: ~**18–30 months**.

2. **Modern governance parity vs xltrail (Git, diff, basic SRM)**

   * **Versioning/Git parity:** CLI + JSON diff + Git-friendly wire formats should reach functional parity ~**12–24 months** in, constrained mainly by grid diff maturity and container robustness.
   * **Governance parity:** matching xltrail’s full Excel versioning UX (web UI, comments, approvals) is more of a product/UI effort; realistic timeline **24–36 months**.

3. **Semantic SRM parity vs PerfectXL / ExcelAnalyzer**

   * PerfectXL’s 39‑principle heuristic engine and visualizations represent ~10+ years of focused domain work.
   * Your plan to unify diff, graph, and risk views in a single semantic engine is sound but ambitious.
   * **Practical expectation:**

     * “Good enough” Modern Excel semantics (M + DAX + grid) that beat most office add‑ins: ~**3 years**.
     * Parity with PerfectXL’s full SRM feature set and trust level: **4–5+ years** if you invest consistently.

---

### 2.3 Sustainable competitive advantages from the architecture

1. **Multi‑platform, non‑COM core**

   * Rust engine compiled to native and WASM; streaming Open XML + DataMashup parsing; no dependency on Excel.exe or COM/VSTO.
   * This directly exploits the “platform gap” of Ablebits, Synkronizer, PerfectXL, ExcelAnalyzer, and most legacy utilities (Windows‑only).

2. **Performance for large models**

   * Design target: “Compare 100MB files in under ~2 seconds” leveraging streaming parsing and near‑linear grid algorithms.
   * H1 (grid diff) and H2 (streaming IR) are explicitly optimized to avoid O(n²) blowups and are heavily performance‑tested.
   * Incumbents built on COM and .NET often choke on large models or require Excel to be automated in‑process, which is fragile and slow.

3. **Semantic depth across grid, M, and DAX**

   * Engine parses Excel grid, Power Query M, and later DAX/tabular models into a common IR and uses AST diff to surface logical changes (e.g., measure changed from SUM to AVERAGE).
   * Competitors typically either:

     * Stay at grid level (Ablebits, Synkronizer, most utilities), or
     * Provide semantics only inside Excel or only for the data model (PerfectXL, DAX tools).

4. **Workflow‑native design (Git/CI + web viewer)**

   * CLI, Python SDK, and cross‑platform web viewer are part of the core plan, not bolt‑ons.
   * This positions the tool as infrastructure (like a linter or static analyzer) rather than only an analyst GUI, which is hard for COM‑add‑in incumbents to match.

5. **AI‑assisted development process**

   * The meta‑programming process (planner/implementer/reviewer agents) plus a detailed test plan is designed to systematically burn down the hardest risk areas and maintain high dev velocity with fewer humans.
   * This is not unique IP, but it increases the chance you can ship a high‑quality engine with a relatively small engineering team.

---

## 3. Revenue Projections (5‑Year)

### 3.1 Shared modeling assumptions

To keep the numbers concrete, define:

* **Year 1** = first full 12‑month commercial year after an MVP GA with:

  * Excel grid diff + DataMashup/M diff.
  * Native Win/macOS CLI and minimal web viewer.
* DAX/data‑model diff and richer governance UI land progressively in Years 2–3 (post‑MVP).

**Pricing anchors (USD):**

* Self‑serve **Pro / Dev seat**: list $20/user/month (≈$240/year); effective ARPU after discounts ≈ **$220/year**.
* Enterprise contracts (governance & CI integration): **$20–60k/year**, average around **$30–35k** by Year 5.
* Free tier: browser viewer with file size limits and no automation.

We’ll model revenue as:

* **Self‑serve seats** (individuals + team seats) at ~$220 ARPU.
* **Enterprise customers** at $30–35k ACV.

CAC and churn discussed per scenario.

---

### 3.2 Revenue projection table

All values are annual recurring revenue in USD millions.

| Scenario     | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
| ------------ | ------ | ------ | ------ | ------ | ------ |
| Conservative | 0.10   | 0.30   | 0.80   | 1.50   | 2.50   |
| Base         | 0.20   | 0.70   | 1.50   | 2.80   | 4.50   |
| Optimistic   | 0.30   | 1.00   | 2.50   | 5.00   | 8.00   |

Total 5‑year cumulative revenue:

* Conservative: **$5.2M**
* Base: **$9.7M**
* Optimistic: **$16.8M**

These are intentionally conservative relative to TAM/SAM and existing incumbents.

---

### 3.3 Base scenario – details

**Year 5 target:** ~$4.5M ARR (~18–25% of $18–25M SAM).

**Year‑5 volume breakdown (approximate):**

* Self‑serve:

  * ~13k seats × $220 ≈ **$2.9M**
* Enterprise:

  * ~50 customers × $32k ≈ **$1.6M**

**Unit trajectory (very approximate, cumulative paying seats):**

* Year 1: 500–800 seats, 5–10 enterprise customers (pilot + early adopters).
* Year 3: 4–6k seats, 20–30 enterprise customers.
* Year 5: ~13k seats, ~50 enterprise customers.

This scale is larger than xltrail or PerfectXL individually, but still well below the combined segment and uses PLG + cross‑platform Power BI semantics that incumbents lack.

**CAC assumptions (base):**

* Self‑serve seat CAC: **$80–120** (0.4–0.55× first‑year ARPU).

  * Driven by content/SEO, GitHub integrations, conference talks, and light paid spend.
* Enterprise customer CAC: **$12–18k** for ~30–40k initial ACV (payback ~12–18 months).

  * Sales motion: founder‑led + 1 AE with strong technical background.

**Churn assumptions (base):**

* Self‑serve annual gross churn: **10–15%**, offset by expansion (team upgrades), giving modest net churn or slight net retention.
* Enterprise gross churn: **5–8%** (governance tooling is sticky once integrated into CI and audit workflows).

**Key milestones that *keep* you in base scenario:**

* Strong MVP reception: at least **10–15 paying design‑partner teams** within 6–12 months of GA.
* “Instant diff” performance validated on real 50–100MB workbooks and public benchmarks.
* First 2–3 recognizable brand logos (regional banks, mid‑sized consulting firms).
* Meaningful Power BI story (PBIX with DataMashup & early DAX diff) by Year 2.

---

### 3.4 Conservative scenario – details

**Year 5 target:** ~$2.5M ARR (~10–14% of SAM).

**Year‑5 unit picture:**

* ~6.8k self‑serve seats (~$1.5M).
* ~30–35 enterprise customers (~$1.0M).

**Drivers:**

* Slower PLG due to weak SEO and limited marketing budget.
* Incumbents actively discount and add Power Query features (PerfectXL, ExcelAnalyzer, xltrail) to defend their install bases.
* DAX and PBIX semantics land later than planned, so Power BI governance buyers hesitate.

**CAC/churn (conservative):**

* Self‑serve CAC: **$120–180** (close to or above first‑year ARPU; payback ~18–24 months).
* Enterprise CAC: **$18–25k**; more outbound, longer sales cycles.
* Higher self‑serve churn (**15–20%**) if product remains “tooling for enthusiasts” rather than deeply embedded.

**Triggers into conservative track:**

* Repeated slippage on H1/H2 (grid diff and streaming parsing) causing negative early reviews.
* Microsoft ships better built‑in diffing / lineage visualization faster than expected, commoditizing basic comparisons.
* Limited bandwidth to build web UX; product perceived as “CLI‑only hacker tool”.

---

### 3.5 Optimistic scenario – details

**Year 5 target:** ~$8M ARR (~30–40% of SAM, still <~15% of full TAM).

**Year‑5 unit picture:**

* ~21–22k self‑serve seats (~$4.8M).
* ~90 enterprise customers (~$3.2M).

**What has to go right:**

* The product becomes the **de facto Git‑friendly diff for Modern Excel + Power BI**, referenced in docs, blog posts, and conference talks in the same way Beyond Compare is for code.
* Strong resonance with developer/data communities (GitHub stars, open‑core goodwill, conference ecosystem).
* A few flagship enterprises (Big Four firm, global bank, FAANG‑scale tech company analytics org) adopt it as their standard diff/governance layer, driving case studies and second‑order sales.

**CAC/churn (optimistic):**

* Self‑serve CAC: **$40–80** (PLG‑driven; strong organic inbound).
* Enterprise CAC: **$10–15k** due to strong brand pull; payback < 12 months.
* Self‑serve churn **7–10%**, enterprise churn **<5%**, with some net negative churn from seat expansion and add‑on features.

**Milestones that *unlock* optimistic trajectory:**

* Public benchmarks clearly showing 10× speed or robustness vs Synkronizer/xlCompare on large workbooks.
* Delightful Mac/Web UI that finds traction in fintech/startup ecosystems currently ignored by Windows‑only incumbents.
* Early, credible DAX/model diff leading to cross‑sell into Power BI governance budgets (alongside ALM Toolkit / Tabular Editor rather than against them).

---

## 4. Revenue Model Recommendations

### 4.1 Pricing tiers and structure

**Recommended tiering (USD, indicative):**

1. **Free / Viewer**

   * Web‑only, limited file size (e.g., 5–10MB).
   * No CLI or automation.
   * Purpose: top‑of‑funnel, “drag‑drop‑see‑diff” experience.

2. **Pro (Individual) – ~ $19/user/month (~$199/year)**

   * Full Excel + M diff, all platforms.
   * Local GUI + CLI + Git integration for single user.
   * Limited PBIX support (DataMashup‑based models initially).

3. **Team (5–50 users) – ~ $29/user/month (with volume discounts)**

   * Adds:

     * Centralized web UI with shared projects.
     * CI integrations (GitHub Actions, Azure DevOps).
     * Role‑based access and SSO for up to N users.
   * Effective ARPU ~ $220/year net of discounts in the model.

4. **Enterprise (50+ users / regulated industries) – custom ($20–60k+/year)**

   * Includes:

     * On‑prem / private‑cloud engine option.
     * Advanced governance features: approval workflows, audit trails, integration with GRC platforms.
     * Priority support, training, and annual model reviews.

This keeps you:

* Clearly more premium than $40 perpetual utilities (Formulasoft, DiffEngineX).
* Cheaper and simpler to buy than PerfectXL’s opaque pricing and ExcelAnalyzer’s ~€800/year seats.

### 4.2 Perpetual vs SaaS

Given:

* Rapid evolution of Excel/Power BI and the need to keep up with DataMashup, enhanced metadata, and Fabric changes.
* Performance and accuracy depending on continuous improvements to high‑complexity algorithms (grid diff, AST diff).

**Recommendation:**

* **SaaS‑first subscription model** for all regular customers.
* Offer **limited perpetual/term licenses only for the engine/CLI** in highly constrained environments (e.g., air‑gapped banks), priced at a premium with mandatory annual maintenance if they want updates.

This keeps revenue recurring while still allowing you to sell into locked‑down institutions that dislike subscriptions.

### 4.3 Freemium / open‑source components

Given the dev‑heavy audience and the desire to become the “default diff,” participation in the open ecosystem matters.

Balanced approach:

* **Open‑source or very permissive:**

  * Low‑level parsers for DataMashup and maybe some Power Query/DAX AST utilities.
  * Small helper tools for exporting Excel/Power BI structure to JSON (goodwill + free marketing).
* **Closed‑source, commercial:**

  * High‑performance grid diff engine (H1).
  * Semantic diff heuristics and risk/lineage logic.
  * Web UI and governance features.

This:

* Encourages community validation and contributions to tricky binary/format parsing.
* Makes it harder for competitors to clone your “secret sauce” quickly.
* Helps you win mindshare with developers while preserving a strong monetization surface.

### 4.4 Enterprise vs individual licensing strategy

* Lean into **bottom‑up adoption** (individual/team subscriptions) with frictionless signup (credit card, no sales call).
* Layer on a **lightweight enterprise sales motion**:

  * Upgrade pathway when a domain hit threshold (e.g., 20+ active seats or multiple teams).
  * Offer SSO, centralized billing, and compliance features as the lever to move them to enterprise pricing.

This mirrors what works for tools like GitHub, Datadog, and Atlassian in dev tooling markets and fits your cross‑platform, dev‑centric positioning.

---

## 5. Critical Success Factors and Risk Analysis

### 5.1 Top 3–5 success factors

1. **Delivering a truly fast and robust grid diff (H1 + H2)**

   * High‑performance 2D alignment and streaming parsing scored 18/20 and 16/20 difficulty and sit in the absolute hot path.
   * If the tool hangs on 50–100MB workbooks or produces noisy, confusing diffs, no amount of semantic magic will save it.

2. **Achieving real semantic differentiation (M + DAX)**

   * Semantic M diff (H4) and M parser (H3) are also in the top difficulty cluster.
   * Without credible M and DAX understanding, you’re just another grid diff with nicer UI; with them, you’re the only modern Excel/Power BI tool that can show “why” a KPI changed rather than just “what cell turned red.”

3. **Nailing the Git/CI workflow story**

   * Competing differentiators (CLI, CI/CD automation) directly attack Ablebits’ and Synkronizer’s blind spots and match or exceed xltrail’s strengths.
   * If developers can’t trivially wire this into GitHub / Azure DevOps, they’ll fall back to Beyond Compare or xltrail.

4. **Compelling cross‑platform UX**

   * Mac‑first desktop launch and a smooth web viewer are a strategic pillar in the differentiation plan.
   * Winning Mac and browser‑centric teams gives you a wedge where incumbents literally cannot follow without rewrites.

5. **Credible governance features for mid‑market SRM**

   * You don’t need full PerfectXL parity, but you *do* need:

     * Clear risk summaries,
     * Audit‑friendly reports,
     * Some visual lineage / graph view.

### 5.2 Major risks

1. **Technical execution risk on hardest hurdles**

   * H1/H2/H4/H3/H9 form an “architecture‑defining” risk cluster. If they slip or produce fragile behavior, the entire product’s reputation suffers.

2. **Microsoft ecosystem shifts**

   * Microsoft Copilot and Fabric may gradually offer “good enough” diff, lineage, or risk diagnostics out of the box, squeezing standalone tools.

3. **SEO and distribution disadvantage**

   * Ablebits and Synkronizer have decades of SEO gravity; xltrail and PerfectXL are entrenched in certain verticals.
   * Without sustained content and devrel investment, discovery will be slow.

4. **Being stuck in the “dangerous middle”**

   * The analysis warns that the “middle” (neither cheap and ubiquitous nor deeply specialized and integrated) is where products die.

5. **Regulatory / procurement friction**

   * Governance buyers often require:

     * On‑prem deployment or strict data locality.
     * Lengthy vendor risk assessments.
   * If your hosting / data story is weak, those deals slip or vanish.

---

## 6. Investment Implications

### 6.1 Development resources justified by the opportunity

Base scenario cumulative revenue over 5 years is ~**$9.7M**; optimistic ~**$16.8M**.

Given that, it is rational to invest in a lean but serious product team.

**Engineering / product (annual, steady‑state):**

* 2–3 senior Rust/compilers/data‑structures engineers (grid diff, streaming parsing, semantics).
* 1–2 full‑stack / front‑end engineers (desktop + wxDragon UI, web viewer).
* 0.5–1 FTE dedicated to QA / test harness and fixtures (plus AI agents).

Assuming $175–200k fully loaded per FTE, an **engineering budget of ~$800k–1.2M/year** is justified, especially given the AI‑assisted development process should amplify output per human.

### 6.2 Marketing and sales investment for the base scenario

To reach the base trajectory (~$4.5M ARR by Year 5), suggest:

* Year 1–2:

  * 1 product‑oriented founder doing sales + evangelism.
  * 0.5–1 marketing generalist (SEO, docs, content).
  * Budget ~$150–250k/year for salaries + ~$50–100k/year for events, sponsorships, and limited paid acquisition.
* Year 3–5:

  * 1 dedicated devrel/advocate.
  * 1 AE / customer success hybrid for enterprise deals.
  * Marketing budget grows proportionally to ~8–12% of ARR.

Total GTM spend ramping toward **$400–600k/year** by Years 4–5 is consistent with ~20–25% operating margin at $4–5M ARR.

### 6.3 Break‑even / self‑sustaining point

Roughly:

* Early phase (Years 1–2):

  * Engineering + GTM + infra ≈ **$1.0–1.5M/year**.
  * Product likely runs at a loss; funded by parent org/investors.

* Once ARR surpasses **$1.5–2.0M**, the product can cover:

  * 3–4 person engineering core,
  * 1–2 GTM staff,
  * Hosting and overhead with modest positive margin.

In the **base scenario**, that break‑even is around **Year 3–4**.
In the **conservative scenario**, it may slip to **Year 4–5**.
In the **optimistic scenario**, it could be reached as early as **Year 2–3**.

---

## Executive Summary – Key Takeaways

1. **Market size is real but niche.**
   Today’s Excel comparison / governance category is only ~$10–20M/year, but realistic 5–7 year TAM is **$50–70M**, anchored in a small slice of the vast Excel/Power BI author base. Your natural SAM is **$18–25M/year**, mostly developer and modern governance buyers.

2. **This product is best positioned as a dev + modern governance tool, not a generic Excel add‑in.**
   Platform ubiquity (Win/macOS/Web), semantic depth (M and later DAX), and Git/CI workflows line up directly against the **Developer & Automation** and “modern SRM” gaps of incumbents like xltrail and PerfectXL.

3. **Conservative but meaningful revenue upside.**
   A new entrant with strong execution can reasonably aim for **$4–5M ARR by Year 5** (base) with a plausible upside case around **$8M**, while a downside but still viable path sits around **$2–3M** ARR.

4. **SaaS + PLG with a free viewer is the right monetization strategy.**
   Simple per‑seat pricing (~$20–30/user/month) plus higher‑ACV enterprise plans, anchored by a free web viewer, aligns with how dev tools and governance platforms are bought and avoids direct price fights with $99 perpetual utilities.

5. **Execution risk is concentrated in a few deep technical and GTM challenges.**
   Success hinges on landing high‑performance grid diff and streaming parsing, delivering credible M/DAX semantics, and winning the Git/CI & cross‑platform UX battle. On the commercial side, you must avoid getting stuck in the “middle” and instead own a clearly defined, high‑value niche.

If you’re deciding whether to double down, the numbers support building a focused, high‑leverage team (3–5 core engineers plus lean GTM) and aiming for the base scenario, with governance features and Power BI semantics as the main levers to push upside.
