Short version: if this Rust/WASM diff engine lands roughly as specced, I’d frame “realistic but ambitious” revenue as **~$2–4.5M ARR by year 5**, depending on which go‑to‑market path you lean into:

* **Dev/CI-first strategy:** ~**$2.3M ARR** by year 5
* **Governance/Audit-first strategy:** ~**$2.9M ARR** by year 5
* **Mac/Web freemium strategy:** ~**$4.4M ARR** by year 5

Below is how I get there and what each strategy actually looks like.

---

## 1. Where you’re starting from (market + product)

From your docs, the product is:

* A **Rust core** that runs on **Win/Mac CLI, desktop, and WebAssembly** with a non‑destructive UI.
* Deep semantics: Power Query M + DataMashup, later DAX/data model, plus a serious 2D grid diff (database and spreadsheet modes, row/col moves, block detection).
* Designed to stay fast and memory‑safe even on ~100MB workbooks via streaming parsing and near‑linear alignment algorithms.

The commercial landscape from your research, roughly:

* **Ablebits** (Excel add-in “Ultimate Suite” mass market): **~$3–5M** revenue.
* **Beyond Compare** (general file diff): **~$1.5–3M**.
* **PerfectXL** (governance/audit): **~$1–3M**.
* **xltrail** (Git+Excel VCS): **~$0.5–1.5M**.
* **Synkronizer, xlCompare, DiffEngineX**: each **sub‑$1M**.

The key insight: this is a **small but very real niche**. The “winners” are mid‑single‑digit millions, not tens of millions, but they’re also lean, high‑margin businesses.

Your proposed product is “blue ocean” mainly because:

* Nobody does **Modern Excel** (M + data model/DAX) semantics properly.
* Nobody has a serious **Mac + zero‑install web** offering that stays private (WASM, no uploads).
* Incumbents are mostly **Windows COM add‑ins or old-school utilities**.

So we’re not trying to build a $100M company in 5 years; we’re trying to beat current category leaders and maybe unify several micro‑segments.

---

## 2. Pricing & segmentation assumptions

To keep projections concrete, I’ll pick a simple, consistent pricing skeleton and then vary *who* buys it.

### 2.1. Segments

Three main revenue pools:

1. **Developers & technical modelers**

   * Quants, FP&A power users, data engineers who live in Git and like CLIs/CI.
   * Currently served by xlCompare, xltrail, Beyond Compare.

2. **Governance / Risk / Audit**

   * Internal model risk teams, external auditors, regulated industries.
   * Currently served by PerfectXL, xltrail (self‑hosted), niche consulting.

3. **Productivity / Mac/Web crowd**

   * Analysts, PMs, finance teams, lots of MacBooks, minimal IT help.
   * Currently under‑served; Windows add‑ins and utilities mostly ignore Mac.

### 2.2. Baseline pricing model

Anchored to current competitors (PerfectXL ~€69/mo seat, xltrail $35/mo seat, Ablebits ~$99 one‑off).

Assumptions:

* **Free tier**

  * Browser‑only, small file limits, capped diff runs/day.
  * Designed as SEO / viral funnel (“compare Excel on Mac in browser”).

* **Pro Individual:** **$15/user/month** (~$180/year)

  * Full diff (Excel + M), Git integration, local CLI.

* **Team Workspace:** **$150/month** (~$1,800/year)

  * Up to ~20 editors, unlimited viewers, shared history, comments.

* **Enterprise:** avg **$20–75k/year**

  * SSO, on‑prem / VPC deploy, audit logging, SLAs; ACV varies by strategy.

These numbers are intentionally lower than xltrail/PerfectXL on a pure per‑seat basis, betting you make it back on **Mac/Web coverage and collaboration features** rather than raw per‑seat pricing.

---

## 3. Strategy A – Dev/CI-first

### 3.1. Strategy shape

**Positioning**

> “Git-native Excel & Power BI diff engine – cross-platform, semantic, and fast enough for CI.”

* Lead with **Rust core + WASM** story, Git difftool/mergetool integration, and semantic diffs for M/Power Query.
* Make it trivial to wire into `git diff` on `.xlsx/.pbix` and into CI pipelines (GitHub Actions, Azure DevOps).

**Product & marketing moves**

* Ship **CLI + Git integration** first, then Mac GUI/Web viewer.

* Heavy content & community:

  * “How to diff Excel in Git properly”, “Semantic diffs for Power Query”.
  * Show benchmarks vs xlCompare, Beyond Compare, Synkronizer on big files.

* Integrations & distribution:

  * VS Code extensions, JetBrains plugin, GitHub Marketplace action.
  * “Plug-and-play replacement” for Beyond Compare/xlCompare in `.gitconfig`.

**Sales motion**

* Mostly **bottom‑up, product‑led**:

  * Individuals swipe a card for Pro.
  * Teams convert when they want shared history & permissions.
* Enterprise only when dev tooling or model risk teams come inbound.

### 3.2. Revenue projection (mid-case)

Assumed paying customer counts:

* Year 1 → Year 5: growing roughly 30–40%/year from a small base.

| Year | Pro individuals | Team workspaces | Enterprise customers |
| ---- | --------------- | --------------- | -------------------- |
| 1    | 600             | 40              | 1                    |
| 2    | 1,500           | 120             | 3                    |
| 3    | 3,000           | 250             | 6                    |
| 4    | 4,500           | 400             | 8                    |
| 5    | 6,000           | 550             | 10                   |

Revenue = Pro * $15/mo + Teams * $150/mo + Enterprise * $20k/year:

| Year | Approx. ARR |
| ---- | ----------- |
| 1    | **$0.20M**  |
| 2    | **$0.55M**  |
| 3    | **$1.11M**  |
| 4    | **$1.69M**  |
| 5    | **$2.27M**  |

This puts you by year 5 in the same ballpark as **Beyond Compare** and the upper end of **xltrail**, with more upside if you later add a GRC tier.

**Pros**

* Fastest initial traction; developers adopt quickly if the CLI is good.
* Clear product‑led growth loop via Git/GitHub.
* Keeps org small and margin high.

**Risks**

* You may get “stuck” as a dev utility doing ~$2–3M ARR if you don’t bridge into governance.
* Needs very strong developer DX and docs to stand out vs Beyond Compare/xlCompare.

---

## 4. Strategy B – Governance/Audit-first

### 4.1. Strategy shape

**Positioning**

> “Spreadsheet risk control for modern Excel & Power BI – semantic diffs your auditors actually trust.”

Here you lean more into **PerfectXL/xltrail territory**, but with:

* **Semantic visibility into M, DAX, and data models**, not just grids.
* Cross‑platform agents running in CI and on desktops.
* Strong story about reproducible, tested diff algorithms (you already have a heavy testing blueprint + meta dev process).

**Product & marketing moves**

* Feature emphasis:

  * Workbook‑level “change stories”: which queries changed, which joins/filters changed, which measures changed—mapped to business concepts.
  * Non‑destructive, shareable diff reports suitable for audit evidence (vs Synkronizer’s “paint the workbook yellow” approach).

* Package a **“GRC Edition”**:

  * SSO, on‑prem, audit logs, data residency.
  * Templated reports that plug into model risk policies.

* Go to **conferences, audit forums, solvers competitions**, and partner with mid‑tier consultancies that embed you into their spreadsheet review methodology.

**Sales motion**

* **Top‑down & consultative:**

  * Multi‑month sales cycles into risk, finance, internal audit.
  * POCs, paid pilots, and services (helping teams codify diff policies).

### 4.2. Revenue projection (mid-case)

Here revenue is dominated by *fewer, bigger* enterprise deals. Assumptions:

* Enterprise ACV **$75k** (typical when your tool is positioned as risk mitigation vs mere productivity).
* Smaller self‑serve segment; most usage happens inside enterprise licenses.

Counts:

| Year | Pro individuals | Team workspaces | Enterprise customers |
| ---- | --------------- | --------------- | -------------------- |
| 1    | 50              | 5               | 1                    |
| 2    | 100             | 20              | 3                    |
| 3    | 200             | 40              | 10                   |
| 4    | 300             | 80              | 20                   |
| 5    | 400             | 120             | 35                   |

Revenue with Pro=$15/mo, Team=$150/mo, Enterprise=$75k/year:

| Year | Approx. ARR |
| ---- | ----------- |
| 1    | **$0.09M**  |
| 2    | **$0.28M**  |
| 3    | **$0.86M**  |
| 4    | **$1.70M**  |
| 5    | **$2.91M**  |

By year 5 you’re now clearly in **PerfectXL‑scale territory (~$1–3M)**, with most of it concentrated in 25–35 high‑value accounts.

**Pros**

* Higher ACV, more defensible moat (you’re part of risk processes, not just tooling).
* Realistic path to acquisitions by GRC platforms (Workiva, Diligent, etc.).

**Risks**

* Slower early revenue; you won’t have big numbers for 2–3 years.
* Heavy investment in compliance, documentation, implementation support.
* Technically, you *must* land the hardest parts (M semantic diff, robustness to malformed files) early; your own difficulty/risk analysis flags grid diff + streaming parsing + M AST as the “hard cluster.”

---

## 5. Strategy C – Mac/Web freemium (mass market within the niche)

### 5.1. Strategy shape

**Positioning**

> “The Excel & Power BI diff tool that actually works on Mac and in the browser.”

This leans hardest into the “blue ocean” angle from your competitive report:

* Incumbents are **Windows‑only** or COM‑bound add‑ins; Mac users are basically forced into workarounds today.
* Corporate IT doesn’t love installing COM add‑ins; browser‑only (WASM, local compute) is a major unlock.

**Product & marketing moves**

* **Mac‑first launch** with a polished desktop app (wxDragon UI) on top of the Rust engine.

* **Zero‑install web viewer** as the funnel:

  * Free diff for small files, shareable links to read‑only diffs.
  * Collect emails and nudge into Pro/Team for larger files, Git integration, and saved history.

* Ruthless SEO:

  * “compare two Excel files on Mac”, “how to diff Power Query M”, “compare PBIX files”, etc.—this is exactly how Ablebits built its funnel for generic Excel tasks.

* Built‑in collaboration:

  * Comments on diffs, lightweight review workflow (“who changed this query and why?”).

**Sales motion**

* Very **product‑led**, mid‑market focussed:

  * Teams put it on expense cards.
  * Enterprise only shows up in year 2–3 when Mac/Web deployment is a selling point vs internal tools.

### 5.2. Revenue projection (mid-case)

Here the bet is lots of smaller customers rather than big enterprise early on.

Assumed paying customers:

| Year | Pro individuals | Team workspaces | Enterprise customers |
| ---- | --------------- | --------------- | -------------------- |
| 1    | 800             | 60              | 0                    |
| 2    | 2,000           | 150             | 2                    |
| 3    | 4,000           | 350             | 6                    |
| 4    | 7,000           | 700             | 12                   |
| 5    | 10,000          | 1,200           | 20                   |

Revenue with same prices (Enterprise at $20k/year):

| Year | Approx. ARR |
| ---- | ----------- |
| 1    | **$0.25M**  |
| 2    | **$0.67M**  |
| 3    | **$1.47M**  |
| 4    | **$2.76M**  |
| 5    | **$4.36M**  |

By year 5 you’re in or above **Ablebits’ estimated revenue range (~$3–5M)**, which would effectively make you the most successful *diff‑focused* product in the market.

**Pros**

* Strong, obvious differentiation (Mac + Web + semantics).
* Free web viewer is a natural marketing engine and a nice “demo” for sales.
* Attractive acquisition target for dev platforms or collaboration tools that want spreadsheet diffing.

**Risks**

* You need real product polish & UX; dev‑tool‑style UI won’t cut it here.
* Support load can grow quickly with lots of small customers.
* Requires constant SEO/content investment to keep funnel full.

---

## 6. How “realistic” are these numbers?

Calibrated against your competitive intel:

* The entire **Excel comparison/risk** space today seems to support a handful of **$0.5–5M** businesses, each with small teams.
* Hitting **$2–3M ARR** by year 5 puts you alongside **PerfectXL / xltrail / Beyond Compare** in this niche.
* Hitting **$4M+ ARR** likely means you’ve successfully combined:

  * Mac/Web blue ocean,
  * developer workflows (Git/CLI),
  * and at least light governance features.

So I’d treat:

* **$2M ARR @ Year 5** as a *solid*, conservative success.
* **$3–4M ARR @ Year 5** as a *realistic upside* if you execute well on Mac/Web and semantic M.
* **$5M+ ARR @ Year 5** as stretching into best‑case territory (you’d basically be the Ablebits of diff tools).

All of this assumes you can actually ship the heavy technical pieces (H1–H4 in your difficulty ledger: grid alignment, streaming parsing, M AST + semantic diff) within ~18–24 months, which is aggressive but plausible given your meta‑development and testing process.

---

## 7. Picking a path (and a hybrid)

You don’t have to pick exactly one of these; the obvious blend is:

* **Year 0–2:** Lead with **Dev/CI + Mac/Web**, ship grid diff + textual M diffs early, then semantic M/PBIX.
* **Year 2–5:** Layer in a **Governance Edition** with enterprise features once the engine is hardened and you’ve accumulated a “museum of horrors” corpus from real files.

If you want, next step we can translate one of these strategies into a very concrete funnel model (traffic → free users → Pro/Team conversion → enterprise leads) and tie it to hiring needs and burn so you can see whether the engineering roadmap and revenue ramp actually line up.

---

Last updated: 2025-11-26 09:31:22
