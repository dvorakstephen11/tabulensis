 ## Ultimate roadmap: from your MVP to “the everything product” for Excel + Power BI + DAX + Tabular

This is a **product + engineering + packaging + marketing** roadmap you can iterate on repeatedly. I’m basing the “current MVP” on what your codebase already supports (Rust core, CLI, desktop GUI, web/WASM demo, Excel Open XML + PBIX/PBIT parsing, semantic diff machinery, object diffs, batch + search + audit export).

Your biggest “win condition” is explicitly aligned with the market gaps your research identified:

* **Platform ubiquity (Mac + web)** is a huge unmet need.
* **Semantic depth (“Modern Excel”: Power Query + DAX / models)** is a blind spot for traditional Excel diff tools.
* **Workflow modernization (version control, 3‑way merge, collaborative review)** is broadly missing.
* **Performance & reliability** are pain points in incumbent tools; your architecture is already pointed at fixing that.
* **Freemium + SaaS packaging** is a big adoption lever (vs restrictive trials). 

---

## 1) Your MVP baseline (what you already have “on the board”)

This matters because your roadmap should *amplify* your existing differentiators, not rebuild them.

### Core compare engine (already strong)

You already have a unified diff model (`DiffReport` + `DiffOp`) that covers:

* Grid diffs: cell edits, row/col operations, moved blocks, etc.
* **Keyed “database mode”** comparisons (table-like sheets), plus auto key suggestion patterns.
* **Power Query (M) diffs** (queries added/removed/renamed, semantic vs formatting-only changes, metadata changes) with room for step-level details.
* **Tabular model diffs** (tables/columns/types/properties, calculated columns, relationships, measures) with semantic-noise suppression options.
* Object diffs: named ranges, charts (hash-based change detection), VBA modules (normalized text compare).

### Front-ends & workflows (already multi-surface)

* CLI supports multiple outputs (JSON/JSONL/text/payload/outcome) and a `git_diff`-style output.
* Desktop GUI exists (wxDragon). 
* Web/WASM demo exists and emphasizes local execution (“files are not uploaded”).
* Batch compare and “deep search” index patterns already exist in your UI layer.
* You already have licensing/trial infrastructure and a local-first privacy posture.

**Implication:** you are not “starting from MVP” in the typical sense—your core is already a credible moat. Your roadmap should primarily (1) expand coverage to parity, (2) ship integrations where users live, and (3) add the “platform features” (merge, history, collaboration, monitoring) that incumbents lack.

---

## 2) North-star product definition

### What you’re building (one sentence)

**“GitHub PRs for business logic”—for Excel workbooks *and* Power BI/Tabular artifacts (data model + queries + report definition), with diff, merge, history, collaboration, monitoring, and governance.**

### The platform promise (what “ultimate” means in practice)

1. **Compare anything**: workbooks, PBIX/PBIT, PBIP projects, report definitions, semantic models, M, DAX, VBA, charts, pivots, connections, etc.
2. **Explain differences**: semantic vs noise, root-cause chains, impact scoring.
3. **Merge safely**: 2‑way and 3‑way merges with conflict resolution (tables + models + code). 
4. **Operationalize change**: version history, shareable reviews, approvals, alerts, audit logs, policies.

---

## 3) Feature universe (everything you’ll ultimately want)

This is the “does everything other products do” inventory, plus the pieces that make yours uniquely valuable.

### A. Excel surface area coverage

**Workbook structure**

* Sheet add/remove/rename, reordering, hidden/very hidden, protection settings
* Defined names & scope (you already diff named ranges)
* External links, workbook connections, Power Pivot / data model presence flags

**Grid content**

* Values, formulas (with semantic normalization), errors, types, blanks
* Large-sheet performance, streaming + limits + determinism guarantees

**Formatting & “semantic formatting”**

* Cell styles, number formats, fonts, fills, borders
* Conditional formatting
* Table styles
  (Incumbents often ignore formatting; but some users need it. )

**Objects**

* Charts (you already detect add/remove/change; can deepen to metadata diff)
* Pivot tables/pivot caches, slicers, timelines
* Data validation rules
* Shapes, images, SmartArt, sparklines
* Comments/notes/threaded comments

**Code & advanced features**

* VBA modules (you already detect changes; expand to textual diff views + export)
* Office Scripts (for web/modern)
* Add-in custom functions metadata (where accessible)

### B. “Modern Excel” + BI semantic layer

This is where you can be first-to-market in a meaningful way for Excel users.

* Power Query (M): parse, diff, step graph diff, metadata (load to sheet/model), grouping paths (you already have query metadata ops)
* Power Pivot / tabular model inside Excel (where extractable): measures, calc columns, relationships, formatting strings, hierarchies, roles
* DAX semantic diff (you already have this machinery)

### C. Power BI / Tabular / project formats (where the ecosystem is going *now*)

Power BI is shifting to developer-friendly formats:

* **PBIP** (Power BI project) stores the report and model as folders of files for source control. ([Microsoft Learn][1])
* **PBIR** (enhanced report format) breaks report definitions into a folder structure with JSON artifacts. ([Microsoft Learn][2])
* Microsoft has been moving toward PBIR being the **default** for new reports (service and later Desktop), which increases the demand for text-native diffs and merges. ([Power BI][3])
* **TMDL** (Tabular Model Definition Language) / model-as-code workflows are increasingly central in semantic model version control. ([Microsoft Learn][4])

**Your opportunity:** become the “diff/merge/insight engine” not only for PBIX/PBIT binaries, but also for PBIP/PBIR/TMDL repositories.

### D. Workflow & platform features (the “pull request for spreadsheets” layer)

* 2‑way merge (“apply these diffs”)
* 3‑way merge (base + two diverged versions)—unique among Excel comparators today per your research
* Version history + rollback (Excel + BI)
* Collaboration: shareable diffs, comments, approvals, assignments, notifications
* Automation: CLI + CI/CD hooks; APIs; difftool integrations (Git)
* Monitoring & alerts (watch folders / SharePoint / Power BI workspaces) logs, permissions, backups, retention
* Intelligent insights: noise suppression, risk scoring, change impact

---

## 4) Difficulty scale (relative engineering cost)

* **1 — Low:**
* **2 — Medium:**
* **3 — High:**
* **4 — Very high:**
* **5 — Extreme:**

---

## 5) The iteration roadmap (feature progression + technical notes + difficulty + pricing + marketing)

Each iteration is an increment you can ship, market, and monetize independently.

---

# Iteration 0 — Baseline (what you can market **now**)

**Theme:** “Fast, local, cross-platform diff engine for Excel + PBIX/PBIT—with semantic M/DAX awareness.”

### Features (ship/position)

* Grid diff + database mode + batch compare + deep search + audit export
* Query diffs (M) + model diffs (tabular) + named ranges + charts + VBA detection
* CLI + desktop + web/WASM “runs locally” demo

### Technical notes

* Your differentiator is architectural: external, memory-safe, streaming-friendly engine vs in-Excel COM add-ins that hang on large files.
* You already have semantic-noise policy hooks (DAX, formula semantic diff toggles).

### Difficulty: **(already done)**

### Pricing/package

* Keep current. If you do one change: add a **truly useful free tier** to accelerate adoption (see Iteration 2 pricing model).

### Marketing focus

* Positioning: “**Modern diff for Modern Excel**—includes query + model diffs, not just cells.”
* Proof content: publish 3 “benchmark stories” (large workbook, M change that doesn’t show in cells, DAX measure change causing pivot shift). The PDF explicitly calls out these pain points and opportunities.

---

# I upgrades (make it a daily driver)

**Theme:** “Reduce time-to-answer: summarize changes, filter noise, deep-link to root cause.”

### Features

1. **Change Summary panel** (top-level counts by sheet + category + severity)
2. **Noise controls** exposed as first-class UI: ignore formatting, ignore whitespace-only, collapse moved blocks, semantic-only mode for DAX/M
3. **“Why did this number change?” view**

   * Link a changed pivot output cell → upstream changed query step / DAX measure / input table cell (even if it’s best-effort at first)
4. **Saved comparison presets** (e.g., Finance model, Data pipeline workbook, Power BI model)
5. **Performance instrumentation**: time, memory, ops count; show when limits triggered (config already supports caps/limits).

### Technical notes

* Mostly *wiring and presentation* of data you already compute.
* Implement “root cause linking” as a graph of references you can progressively improve:

  * Basic: sheet/table name heuristics + query load metadata (you already diff query metadata like LoadToSheet/LoadToModel).
  * Later: build a dependency map (Power Query → tables → pivot caches; DAX measure used in pivots).

### Difficulty: **2**

(High product impact; moderate engineering because core primitives exist.)

### Pricing/package

* No change.

### Marketing focus

* “Fast and stable, *and* explains changes.” This is the bridge from “diff utility” to “decision-support tool,” which your market analysis calls out as a differentiator.
* Launch assets: 90‑second demo video, interactive web demo walkthrough, “diff preset” templates for common scenarios.

---

# Iteration 2 — PBIP/PBIR/TMDL support (ride the ecosystem wave)

**Theme:** “First-class diff/merge for source-controlled Power BI projects.”

### Features

1. **PBIP import + compare** (folder-to-folder diff) ([Microsoft Learn][1])
2. **PBIR diff viewer**

   * Page/visual-level diffs
   * Bookmark/selection pane diffs
   * Theme diffs
   * Robust JSON semantic diff (ignore ordering, normalize GUID noise, etc.) ([Microsoft Learn][2])
3. **TMDL diff viewer** (semantic model metadata-as-code) ([Microsoft Learn][4])
4. **Git UX kit**

   * Provide repo templates: `.gitattributes`, recommended difftool config, CI recipes
   * “Explain diffs in PR language” output mode

### Technical notes

* PBIP/PBIR/TMDL are “text-native,” so you can implement a *much more powerful* diff quickly:

  * AST-based or normalized JSON diff
  * Stable identifiers for visuals/measures/objects
* This also sets you up for merge features on the BI side earlier than Excel merge.

### Difficulty: **2–3**

(Parsing + normalization + UI: manageable; many edge cases but less brutal than Open XML.)

### Pricing/package (important)

* Introduce a **Product-Led Growth** structure:

  * **Free:** unlimited viewing + limited comparisons/month OR file size cap (but *not* “100 cells” style crippling)
  * **Pro (individual):** unlimited local compare + CLI + advanced exports
  * **Team:** collaboration features later
  * **Enterprise:** governance later

### Marketing focus

* Message: “Power BI is moving to PBIR/TMDL; we’re the diff engine that makes it usable.” ([Power BI][3])
* Distribution channels:

  * Dev-focused communities (Power BI / Tabular / Fabric)
  * Content targeted to “source control for Power BI projects” keywords

---

# Iteration 3 — Power BI Desktop integration (be where users click)

**Theme:** “One-click compare from inside Power BI Desktop.”

### Features

1. Ship a **Power BI External Tool** entry that can:

   * Diff the current PBIX/PBIT or PBected baseline
   * Diff “current” vs “last exported snapshot”
   * Open a focused view on model/query/report changes
     Power BI Desktop supports external tools and provides launch context. ([Microsoft Learn][5])

2. “Export project artifacts” helper

   * If the user is still in PBIX, offer a guided “export to PBIP/PBIR/TMDL” flow (where feasible and permitted)

### Technical notes

* External tool integration is mostly packaging + robust path detection + UX. ([Microsoft Learn][5])
* Add a “workspace” concept in your desktop app:

  * remembers last baseline
  * stores snapshots
  * supports quick compare

### Difficulty: **2**

### Pricing/package

* Pro includes external tool integration; free tier can include it but limit usage.

### Marketing focus

* “It shows up in the External Tools ribbon.” That’s a concrete adoption step that reduces friction.
* Make a short “install + first compare in 60 seconds” video and a 1-page PDF cheat sheet.

---

# Iteration 4 — Excel parity expansion (formatting + pivots + validations)

**Theme:** “Match and exceed the feature completeness of classic Excel diff tools.”

### Features

1. **Formatting diff (optional / noise-filtered)**

   * cell style changes
   * conditional formatting changes
   * number format changes
     Your research notes formatting diff is rare but valued for auditing.

2. **Pivot diff (v1)**

   * pivot table definitions, pivot cache changes, slicers/timelines
   * basic “pivot output changed” → “source changed” hints

3. **Data validation + named styles + tables**

4. **External links & connections diff**

5. **Chart diff (v2)**: beyond hash → show chart type + data range deltas (you already extract chart metadata fields).

### Technical notes

* This is deep Excel Open XML surface area:

  * `styles.xml`, `sharedStrings.xml`, `pivotTable*.xml`, `pivotCache*.xml`, drawing parts, comments parts, etc.
* Key engineering principle: keep “formatting diff” in a separate category and default it off for most presets to avoid noisy output.

### Difficulty: **3**

(Parsing breadth; lots of weird files.)

### Pricing/package

* No change unless you want a “Pro+ Audit” add-on later.

### Marketing focus

* Competitive positioning: “We now cover what legacy tools do (including merge-ready alignment), while also covering M + DAX.”

---

# Iteration 5 — Merge v1 (2‑way apply) for table-like sheets

**Theme:** “Make the diff actionable.”

This is where you start matching the “merge changes” selling point that users rely on in tools like Synkronizer/xlCompare/Ablebits.

### Features

1. **Apply selected diffs** into a target workbook (produce a new merged workbook)
2. **Table-aware merge**:

   * key-based row alignment
   * allow mapping “key columns” + fuzzy key matching
3. **Conflict detection** (2-way): highlight ambiguous merges, require user resolution
4. **Merge report**: what was applied, skipped, conflicted

### Technical notes

* You need a “patch” representation:

  * `DiffOp` → patch actions (set cell, insert row, update range, etc.)
* Hardest parts:

  * writing Open XML safely without corrupting formulas/styles
  * preserving references when rows/cols move
* Start narrow: **merge only on sheets that are in database mode** (predictable). Then expand.

### Difficulty: **4**

(Write-back correctness is harder than diff.)

### Pricing/package

* Introduce **Pro Merge** as a paid differentiator:

  * Free = compare/report
  * Pro = apply/merge/export
    This aligns with value-based pricing and avoids gating basic compare.

### Marketing focus

* “Compare → merge → ship.”
* Showcase: “monthly forecast consolidation” and “reconciling two diverged versions of a table.”

---

# Iteration 6 — 3‑way merge (the “xlCompare killer” feature)

**Theme:** “Real version-control workflows for spreadsheets and models.”

Your research notes that **3‑way merge is uniquely native to one Excel-focused tool today**.

### Features

1. Base + Mine + Theirs merge for:

   * table-like sheets (first)
   * then general grid (later)
2. Conflict UI:

   * side-by-side resolution
   * accept mine/theirs/manual edit
3. Output a merged workbook + conflict log

### Technical notes

* You need stable identity mapping:

  * For table mode: keys = identity
  * For general grid: identity is hard (moves, inserts, copy/paste). Use heuristics:

    * block hashes (you already have moved block ops)
    * locality + similarity scoring
* Make merge deterministic; users must trust the output.

### Difficulty: **5**

(This is one of the hardest things you can build in this space.)

### Pricing/package

* “Team” or “Pro+” tier:

  * 3-way merge is a flagship premium feature.

### Marketing focus

* This is the moment you can credibly claim: “We’re not just a comparator; we’re *change management for spreadsheets*.”

---

# Iteration 7 — Built-in version history & rollback (local + Git-backed)

**Theme:** “Never lose context; every change has a timeline.”

Version history and rollback is a major gap in classic compare tools; specialized tools exist for this.

### Features

1. “Project” concept:

   * track versions of a workbook/model
   * show diffs between any two points
   * rollback/export any version
2. Storage backends:

   * **Local** repository (simple)
   * Optional **Git backend** for teams already using Git
3. Snapshot triggers:

   * manual snapshot
   * watchrate with CI

### Technical notes

* For binaries (xlsx/pbix): store whole files + compute diffs on demand, or store deltas if you later build patch storage.
* For PBIP/PBIR/TMDL: store text files; diffs are instant and Git-native.

### Difficulty: **3–4**

(Depends on backend scope; start local-first.)

### Pricing/package

* Version history:

  * Pro includes local history
  * Team includes shared history

### Marketing focus

* “Spreadsheet time machine.”
* Ideal for compliance, finance model governance, and BI change tracking.

---

# Iteration 8 — Shareable diffs + collaborative review (PR workflow)

**Theme:** “Google Docs suggestion mode / GitHub PR review, but for Excel + BI.”

Your research explicitly calls out the gap: no shareable read-only diffs via URL, no commenting, no review workflows.

### Features

1. Shareable read-only diff links (hosted)
2. Comment threads on changes (cell/range/query/measure)
3. Approvals + reviewers + change requests
4. Notifications:

   * email
   * integrations with collaboration tools (later expansion)

### Technical notes

* You’ll need a cloud service that stores:

  * diff payloads (or encrypted blobs)
  * access control
  * audit trails
* Consider a “zero-knowledge” option:

  * store encrypted diffs client-side; server only hosts ciphertext
  * aligns with your privacy posture.

### Difficulty: **4**

(New platform surface + auth + storage.)

### Pricing/package

* This is where **Team** becomes a real SKU:

  * per-seat pricing
  * limits by storage or number of shared diffs

### Marketing focus

* “Stop screenshotting diffs into Slack.”
* Launch with a “pull request for spreadsheets” landing page and a guided demo.

---

# Iteration 9 — Monitoring & alerts (Excel + Power BI governance-lite)

**Theme:** “Be the Sentinel for spreadsheets.”

Monitoring/alerts is strong in the BI realm and under-served in Excel, per your research.

### Features

1. Scheduled snapshots and diffs:

   * Watch OneDrive/SharePoint folders
   * Watch Power BI workspaces (datasets/reports)
2. Alert policies:

   * “Query changed”
   * “Measure changed”
   * “External connection changed”
   * “New hidden sheet added”
3. Change feed + audit export

### Technical notes

* This requires connectors + identity + secure storage:

  * Microsoft cloud APIs will add complexity and support load
* Start with “ageninstalled in org) to satisfy security teams; offer pure SaaS later.

### Difficulty: **4–5**

(Connectors, scale, security, product scope.)

### Pricing/package

* Team: limited monitors
* Enterprise: unlimited + retention + compliance

### Marketing focus

* Sell to managers/admins: “Know when critical business logic changes.”
* Case studies: SOX-like controls, finance planning models, BI governance.

---


# Iteration 10 — Intelligent insights (AI + analytics) that reduce noise & surface risk

**Theme:** “From diff → decision.”

Your research highlights “intelligent insights” as the path to leapfrogging incumbents.

### Features

1. **Impact scoring**

   * measure changes: used in which visuals?
   * query changes: affects which outputs?
   * high-riskrnal links, hidden sheets, macros
2. **Noise suppression**

   * automatically classify formatting-only or reorder-only changes
3. **Narrative summaries**

   * “What changed?” + “Why it matters?”
4. **Rules engine**

   * customizable policies per project/org

### Technical notes

* Start deterministic + rules-based; add AI as a layer, not as the only brain.
* If you add AI:

  * keep local-first options for sensitive users
  * keep a “no data leaves device” mode consistent with your posture.

### Difficulty: **3–4**

(If rules-first; **4–5** if heavy AI + lineage.)

### Pricing/package

* Pro includes basic insights
* Team/Enterprise includes policy packs, org-level analytics

### Marketing focus

* “Spend 10 minutes reviewing changes instead of 2 hours.”
* This is where you shift from “tool” to “platform.”

---



# Iteration 11 — Enterprise governance (permissions, logs, retention, on‑prem)

**Theme:** “Make it approvable by serious IT.”

Governance features are a differentiator in enterprise spreadsheet management tools and BI governance products.

### Features

* SSO, RBAC, SCIM
* Immutable audit logs
* Retention policies
* Central inventory of “critical spreadsheets and models”
* On-prem / private cloud deployments
* DLP-friendly architecture

### Technical notes

* Don’t build this before you have:

  * collaboration + monitoring primitives
  * an enterprise buyer pulling you into it
* Make governance modular and service-oriented so it doesn’t slow the core product.

### Difficulty: **5**

### Pricing/package

* Enterprise only.

### Marketing focus

* Move from product-led to sales-assisted:

  * security docs, SOC2 roadmap, procurement readiness
  * partner with governance consultants

---


# Iteration 12 — Ecosystem: SDK, plugins, API, marketplace

**Theme:** “Let the ecosystem build the long tail.”

### Features

* Plugin SDK for:

  * new artifact types (e.g., custom Excel objects)
  * custom diff rules / normalizers / severity scoring
* REST API:

  * submit artifacts
  * get diff payloads
  * post review status
* Integrations:

  * CI pipelines
  * ticketing systems
  * chat ops

### Technical notes

* Keep core diff engine stable; expose extension points at boundaries:

  * “extract → normalize → diff → classify → render” pipeline
* Treat plugins as sandboxed to preserve reliability/security.

### Difficulty: **4**

### Pricing/package

* Pro: local SDK
* Enterprise: private plugin registry + polieting focus
* “Build your own governance rules” / “Bring your own artifacts.”
* Partner channel: consultants and enterprise tooling vendors.

---

## 6) Packaging model (recommended “end state”)

To maximize adoption *and* revenue, align packaging to value inflection points your research identified (freemium + SaaS + enterprise governance).

### **Free**

* Local compare + view
* Limited comparisons/month or file-size cap
* Web demo stays usable and impressive (your best top-of-funnel)

### **Pro (individual)**

* Unlimited compares
* Advanced diff presets (semantic, formatting toggles)
* CLI + git-friendly output
* Export formats (HTML, audit workbook, JSON)

### **Pro+ Merge**

* Apply diffs / 2-way merge
* Eventually 3-way merge upsell

### **Team**

* Shareable diffs
* Review workflow (comments, approvals)
* Shared history

### **Enterprise**

* Monitoring + connectors + alerts
* Governance suite (SSO/RBAC/logs/retention/on-prem)

---

## 7) Marketing playbook by stage (how to maximize success as features roll out)

### Stage 1: Prove the “Modern Excel” wedge (right now → Iteration 2)

**Angle:** “Traditional tools only diff cells. We diff the logic behind the numbers.”
Tactics:

* Publish 5 “before/after” demos where cells barely change but M/DAX did
* SEO pillars:

  * “compare Power Query changes”
  * “diff DAX measures”
  * “Excel compare for Mac”

### Stage 2: Be everywhere users already work (Iterations 3–4)

**Angle:** “One click inside Power BI Desktop; works in Excel on Mac.” ([Microsoft Learn][5])
Tactics:

* External tools listing + tutorials
* Excel add-in listing + “no admin rights” deployment story (big for enterprises)

### Stage 3: Own merges + versioning (Iterations 6–8)

**Angle:** “Stop emailing files. Merge changes safely.”
Tactics:

* Case studies:

  * budgeting consolidation
  * forecasting models with multiple contributors
* Pricing story: merge is where you charge more (clear ROI)

### Stage 4: Become the platform (Iterations 9–11)

**Angle:** “Govern business logic like software.”
Tactics:

* Sell to managers: auditability, approvals, alerts
* Partner with governance-focused consultants
* “Security & privacy” pages that align with local-first and enterprise expectations

### Stage 5: Win with insight (Iteration 12+)

**Angle:** “We t
Tactics:

* Benchmark: time saved reviewing diffs
* “Top 10 high-risk changes this week” dashboards (for Team/Enterprise)

---

## 8) Information that would let me refine this roadmap even further (use this as your research checklist)

You asked for what would be useful to improve/expand suggestions—here’s the highest leverage info to gather.

### A. Product/market clarersona today (choose top 1–2):

* finance modelers, BI developers, internal audit/compliance, IT governance, consultants

2. Typical file profiles:

   * size ranges, sheet counts, table usage %, pivot usage %, Power Query usage %, Power Pivot usage %
3. Critical workflows:

   * compare-only vs merge, manual review vs approvals, how often comparisons happen

### B. Deployment constraints (this changes everything)

1. Must the product be **100% offline** for most customers?
2. Are customers okay with “upload to cloud for shareable diff,” or do you need encrypted/zero-knowledge?
3. Target environments:

   * regulated ia, banking) vs general SMB

### C. Integration priorities

1. Excel: Windows vs Mac vs webBI: PBIX vs PBIP adoption among your target users (PBIP/PBIR/TMDL are growing in importance). ([Microsoft Learn][1])
2. Version control norms:

   * do your users already use Git, or do they need “Git under the hood, invisible”?

### D. Commercial strategy

1. Your preferred pricing philosophy:

   * maximize adoption (PLG) vs maximize revenue per customer
2. Support model:

   * self-serve vs services vs enterprise support
3. Which capabilities are you willing to gate (merge, monitoring, collaboration)?

### E. Technical/engineering constraints

1. Do you want a single unified UI stack (web tech in desktop wrapper) or truly native UIs?
2. Your tolerance for maintaining many parsers (Excel Open XML breadth is huge)
3. Your appetite for “write back” features (merge) vs staying read-only

---

## If you want one “meta rule” to guide every roadmap revision

**Always prioritize features that (1) widen platform reach, (2) deepen semantic understanding 3) turn diffs into workflow outcomes (merge/history/review/alerts).** That’s exactly where the market gaps are largest—and where incumbents are slowest to respond.

[1]: https://learn.microsoft.com/en-us/power-bi/developer/projects/projects-overview?utm_source=chatgpt.com "Power BI Desktop projects (PBIP)"
[2]: https://learn.microsoft.com/en-us/power-bi/developer/embedded/projects-enhanced-report-format?utm_source=chatgpt.com "Create a Power BI report in enhanced report format"
[3]: https://powerbi.microsoft.com/en-us/blog/pbir-will-become-the-default-power-bi-report-format-get-ready-for-the-transition/?utm_source=chatgpt.com "PBIR will become the default Power BI Report Format"
[4]: https://learn.microsoft.com/en-us/power-bi/transform-model/desktop-tmdl-view?utm_source=chatgpt.com "Work with TMDL view in Power BI Desktop"
[5]: https://learn.microsoft.com/en-us/power-bi/transform-model/desktop-external-tools?utm_source=chatgpt.com "External Tools in Power BI Desktop"
