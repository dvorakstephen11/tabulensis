
 **Prompt 1 — Excel diff market & buyer segmentation**

 Do a deep market and segmentation study for professional Excel comparison tools and adjacent workflows. Specifically:

 * Estimate the number of *practical* buyers in these segments:

   1. FP&A / corporate finance teams,
   2. audit / compliance / risk teams,
   3. consulting / transaction advisory,
   4. Excel/Power BI freelance modelers,
   5. developer‑adjacent analysts who already use Git.
 * For each segment, identify:

   * The most common current solutions (including built‑in Excel tools, Synkronizer, xlCompare, xltrail, generic diff tools, and homegrown scripts).
   * Typical budget and willingness to pay for Excel‑centric diff/merge (one‑time vs subscription; solo vs team).
   * Channels and ‘where they hang out’ (conferences, communities, newsletters, training courses).
 * Use available public data (tool download stats, pricing pages, job postings mentioning these tools, community discussions, etc.) to build a *rough but defensible* TAM/SAM/SOM model and 3 realistic adoption curves for years 1–3.
   Return: a structured report with segment definitions, evidence‑backed estimates, and concrete implications for pricing and initial targeting.


 **Prompt 2 — Funnel mechanics & conversion benchmarks**

 Research and propose a detailed funnel strategy where:

 * A free, browser‑based Excel diff viewer (WASM, no upload) is the top of funnel.
 * Paid products are a Mac/Windows desktop app and CLI with more power/features.
   Tasks:
 * Find benchmarks from analogous tools: code diff viewers, diagram viewers, PDF tools, or developer utilities that use ‘free viewer, paid editor/automation’ models (for example: Draw.io vs paid, online code formatters vs IDE plugins, diff/merge tools, etc.).
 * For each benchmark, capture:
   * Free vs paid feature boundaries,
   * Conversion rates from free to paid,
   * Typical upgrade triggers (file size limits, usage caps, collaboration, automation/CLI).
   * Design at least 3 candidate funnels for the Excel diff product, including:
   * Exact feature gating for web viewer vs desktop vs CLI,
   * When and how to ask users to sign up,
   * Which moments to surface upgrade prompts (e.g., file size, Modern Excel features, team sharing).
   * Conclude with a recommended funnel (MVP) and a table of key metrics to track in the first 12 months.
   Focus on concrete conversion numbers and real-world examples, not generic SaaS advice.


 **Prompt 3 — Partner-driven distribution (trainers, consultancies, ecosystems)**

 Investigate the best partnership and channel strategies for a niche B2B tool that targets Excel/Power BI professionals (especially on Mac) with advanced diff/merge capabilities (including Modern Excel).

 * Identify at least 10 potential partner types: Excel/financial modelling trainers, Power BI / Modern Excel course providers, boutique modelling consultancies, FP&A communities, audit/compliance training orgs, etc.
 * For each type, gather real examples of software/tools they already partner with (add‑ins, SaaS, utilities), the typical commercial arrangement (affiliate %, sponsorship, bundled licensing, white‑label, etc.), and any evidence of how successful those partnerships are.
 * From this, design 2–3 concrete partner programs for the Excel diff product (e.g., ‘Trainer bundle’, ‘Consultancy license + co‑marketing’, ‘Power BI community sponsor’), including target partner profile, offer structure, and expected impact on adoption.
 * Include a prioritised 6‑month outreach plan with sequencing and rough effort vs. payoff.
 Optimise for actionable partner ideas, not just a list of communities.


 **Prompt 4 — Security & privacy blueprint for Excel diff (desktop + WASM)**

 Construct a detailed security and privacy blueprint for an Excel comparison product with:

 * A Rust core,
 * macOS/Windows desktop app, and
 * A browser-based WASM viewer that *can* run entirely client-side.
   Tasks:
 * Build a threat model: enumerate realistic threats for individual users vs. regulated orgs (finance, healthcare, pharma), including local malware risks, supply-chain attacks (updates), and data exfiltration if any server-side processing is later introduced.
 * Define recommended data-handling modes:
   1. Purely local (no file ever leaves device),
   2. Optional cloud features (shared diffs, history) with server involvement.
 * For each mode, specify best practices for:
   * Handling temporary files, logs, and crash reports,
   * Encryption at rest/in transit where applicable,
   * Code signing, notarization, and update mechanisms on macOS and Windows,
   * Secure implementation patterns for WASM (e.g., avoiding dangerous APIs, sandbox considerations).
 * Summarise recommended defaults (what’s on/off by default) and how to document this for security reviewers in enterprises.
   Return: a concrete architecture + security checklist that could be turned into a short ‘Security Whitepaper’ for customers.


 **Prompt 5 — Enterprise readiness & compliance expectations**

 Research what mid-size and large companies (especially in finance/audit/compliance-heavy industries) expect when they evaluate a desktop + optional-cloud tool that touches sensitive Excel files.

 * Collect example security questionnaires, vendor due-diligence checklists, or RFP security sections relevant to desktop tools and browser-based tools.
 * Summarise the common requirements and questions around:
   * Data residency and data retention,
   * Logging/telemetry,
   * GDPR and other regional privacy regimes,
   * SSO/SAML, role-based access control, and audit logging (for any multi-user / cloud features),
   * Incident response and disclosure.
 * Based on this, draft:
   * Data residency and data retention,
   * Logging/telemetry,
   * GDPR and other regional privacy regimes,
   * SSO/SAML, role-based access control, and audit logging (for any multi-user / cloud features),
   * Incident response and disclosure.
   * A minimal ‘Enterprise Readiness’ feature set for v1/v2 of the product,
   * A sample 2–3 page ‘Security & Privacy Overview’ document tailored to this Excel diff tool, outlining how data is handled in both purely local and optional cloud modes.
   Focus on what a *small* team can realistically implement while still passing the most common objections.


 **Prompt 6 — Practical roadmap for Power Query (M) and DAX parsing**

 Design a practical, staged implementation plan for robust parsing and diffing of Power Query M scripts and DAX measures inside Excel workbooks.

 * Survey existing open-source and commercial tools that:
   * Parse M (Power Query) code,
   * Parse DAX and/or Tabular models (Power BI, SSAS Tabular, Excel data model),
   * Inspect or extract DataMashup and `xl/model` structures.
 * For each, document: license, maintained status, language, and whether parts can be reused (e.g., grammars, parsers, schema readers).
 * From available specs (MS-XLDM, Open XML docs, community reverse-engineering), detail the *actual* steps to:
   * Extract M code from the workbook’s DataMashup,
   * Extract model metadata and DAX from `xl/model/*`,
   * Represent each as ASTs suitable for diffing.
 * Propose a phase plan for the product:
   * Phase 1: minimal but robust M-only diff,
   * Phase 2: richer M (step-level diff, metadata),
   * Phase 3: DAX diff and model schema comparison.
 * Flag the riskiest unknowns and suggest mitigations (e.g., where specs are incomplete, where reverse-engineering is fragile).
   Return a technical design doc that could be implemented in Rust, but you may reference other languages for inspiration (e.g., existing grammars).


 **Prompt 7 — Algorithm selection & performance tradeoffs for 2D alignment**

 Given a requirement to diff large Excel workbooks efficiently (tens of MB, hundreds of thousands of cells), evaluate which alignment algorithms and heuristics are most appropriate for v1 vs later phases.

 * Compare practical performance and implementation complexity of:
   * Simple row/column hashing + LCS on row sequences,
   * Hunt–Szymanski vs Myers vs other LCS variants in this 2D context,
   * Block-based approaches (anchor rows + local alignments),
   * Hungarian algorithm vs greedy matching for within-block cell pairing,
   * Various move-detection heuristics.
 * Look for existing spreadsheet diff implementations or research papers that share real-world performance data or algorithm descriptions.
 * Propose 3 ‘algorithm bundles’:
   * Minimal v1 (simple but good-enough)
   * Intermediate (better handling of complex row inserts/moves)
   * Advanced
 * For each bundle, estimate:
   * Engineering effort,
   * Expected asymptotic and observed performance,
   * Edge cases it handles well vs poorly.
   End with a recommendation of which bundle to implement first and how to instrument the engine so you can later swap in more advanced components without breaking UI or tests.

