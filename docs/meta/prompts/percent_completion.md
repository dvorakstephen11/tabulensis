You are a senior software engineering analyst and delivery planner.

Your job is to review my codebase and project artifacts, then estimate **what percentage of the project is complete** along multiple dimensions. You should base your analysis on actual code, structure, and history — not just file counts or LOC.

--------------------
1. Context & inputs you should use
--------------------

You will have access to some or all of the following:

**Core Codebase**
- The full **codebase** (current main branch).
- Any **TODOs / FIXMEs / comments** in the code that indicate missing or incomplete work.

**Technical Documentation** (in `docs/rust_docs/`)
- `excel_diff_meta_programming.md` — The authoritative development process guide.
- `excel_diff_technical_document.md` — Architecture and IR design.
- `excel_diff_testing_plan.md` — Phased testing milestones (Phase 0–6) and MVP readiness matrix.
- `excel_diff_difficulty_analysis.md` — Analysis of technical challenges.
- `excel_diff_m_query_parse.md` — M-code parsing design.
- `excel_diff_product_differentiation_plan.md` — Product roadmap and competitive positioning.

**Cycle Artifacts** (in `docs/meta/`)
- `plans/[branch-name].md` — Mini-specs defining scope, behavioral contracts, and test plans for each cycle.
- `plans/[branch-name].yaml` — Decision records explaining why work was chosen.
- `logs/[branch-name]/activity_log.txt` — Implementer's log of changes and decisions during a cycle.
- `results/[branch-name].txt` — Test output from the cycle.
- `retrospectives/` — Post-cycle learnings and process improvements.

**Business Context** (in `docs/`)
- `projections/` — Revenue projections and market analysis.
- `competitor_profiles/` — Analysis of competing products (Synkronizer, xlCompare, etc.).

**Version Control**
- **Git history** (commits, branches, tags, messages).
- Branch names follow the pattern `YYYY-MM-DD-description` for traceability.

**Review Context**
- The `generate_review_context.py --collate` workflow packages all relevant artifacts for review sessions, including codebase snapshots, mini-specs, decision records, activity logs, and test results.

Use as many of these signals as are available to build your estimates.

--------------------
2. Main outputs: completion percentages
--------------------

Estimate completion along at least these three axes, each as a percentage from 0–100%, with reasoning:

1. **Percent of total difficulty overcome**
   - Interpret "difficulty" as the combination of:
     - Core architectural challenges
     - Algorithmic complexity
     - Integration points and risky dependencies
   - The **hardest technical slices** for this project are documented in `excel_diff_difficulty_analysis.md` and include:
     - Container parsing (XLSX/PBIX OPC packages)
     - Grid alignment (database mode vs spreadsheet mode algorithms)
     - DataMashup binary stream extraction and M-code parsing
     - Hierarchical diff engine (workbook → object → semantic → grid levels)
     - Memory-efficient streaming for 100MB+ files
     - WASM compilation and cross-platform deployment
     - DAX/data model parsing (post-MVP)
   - For each hard slice, judge whether it is:
     - Not started,
     - Partially implemented,
     - Mostly implemented,
     - Essentially done and hardened (tested, refactored).
   - Weight these slices by their relative difficulty and compute an overall percentage:
     - 0% = architecture still mostly on paper.
     - 100% = all major hard problems solved and only low-risk cleanup / polish remains.

2. **Percent of total code written**
   - Do **not** just use raw LOC.
   - Instead:
     - Identify the planned modules / components from `excel_diff_technical_document.md` and the testing plan.
     - For each module, estimate:
       - Planned surface area (APIs, responsibilities).
       - What is implemented vs stubbed vs missing.
     - Include internal plumbing, not just user-visible features.
   - Provide:
     - A **weighted estimate** of "code written vs code still to be written" based on modules/features.
     - A brief breakdown per major subsystem:
       - **Container layer**: XLSX/XLSB/PBIX OPC package handling
       - **Grid IR**: Cell representation, sparse/dense storage, type coercion
       - **M-code parser**: DataMashup extraction, tokenizer, AST builder
       - **Diff engine**: Hierarchical comparison, alignment algorithms
       - **CLI**: Command-line interface for local usage
       - **Web viewer**: WASM-based browser diff viewer
       - **Integrations**: Git diff driver, CI/CD hooks

3. **Percent of dev time in days (extrapolated)**
   - Use the following sources to estimate elapsed dev time:
     - **Activity logs** in `docs/meta/logs/[branch-name]/activity_log.txt` for each cycle.
     - **Decision records** in `docs/meta/plans/[branch-name].yaml` which include difficulty estimates.
     - **Git commit history** (branch names include dates: `YYYY-MM-DD-description`).
     - **Retrospectives** in `docs/meta/retrospectives/` for cycle duration insights.
   - Estimate:
     - **Elapsed dev time** so far (in effective calendar days of work).
     - **Remaining dev time**, based on:
       - Remaining features vs what's already implemented.
       - The distribution of difficulty (from item 1).
       - The phased testing plan (Phase 0–6) as a progress marker.
   - Produce an estimate like:
     - "We've likely used ~X–Y dev-days so far, with Z–W dev-days remaining."
   - Convert that into a **percentage of total dev time**:
     - Percent complete (time) ≈ (elapsed dev time) / (elapsed + remaining dev time) * 100.
   - State your assumptions (e.g., how many hours per dev-day, how many devs, productivity inferred from commit patterns).

--------------------
3. Additional completion metrics to compute
--------------------

Beyond those three, propose and compute **other useful measures of “percent complete”**, such as:

1. **Percent of planned features implemented**
   - From `excel_diff_testing_plan.md`, the MVP readiness matrix defines:
     - **Must work before MVP**: Excel grid diff, Excel DataMashup + M diff
     - **Can land just before release**: PBIX with DataMashup
     - **Post-MVP**: PBIX without DataMashup (tabular model), DAX/data model diff
   - From the phased testing plan (Phase 0–6):
     - Phase 0: Harness & fixtures
     - Phase 1: Containers, basic grid IR, WASM build guard
     - Phase 2: IR semantics, M-code snapshots, streaming budget
     - Phase 3: MVP diff slice, DataMashup fuzzing
     - Phase 3.5: PBIX host support
     - Phase 4: Advanced alignment, DB mode, adversarial grids
     - Phase 5: Polish, perf, metrics
     - Phase 6: DAX/model stubs (post-MVP)
   - Mark each phase/feature as: not started, in progress, done.
   - Compute:
     - A naïve percentage (features done / total features).
     - A **complexity-weighted percentage** (harder features have higher weight, using decision record difficulty estimates where available).

2. **Percent of modules implemented & integrated**
   - Build a list of core modules from the architecture:
     - **core/**: IR types, diff algorithms, M-code parser
     - **xlsx/**: XLSX container parsing and extraction
     - **xlsb/**: XLSB binary format support (if applicable)
     - **pbix/**: PBIX/PBIT Power BI container support
     - **cli/**: Command-line interface
     - **wasm/**: WebAssembly bindings for browser deployment
     - **tests/**: Test harness and fixtures (`fixtures/templates/`, `fixtures/generated/`)
   - Score each on:
     - Implementation completeness.
     - Integration with the rest of the system.
     - Presence of tests (unit, integration, property-based per testing plan).
   - Estimate:
     - "Standalone implementation complete %"
     - "Integrated into the full system %"

3. **Percent of test coverage and quality**
   - The testing plan uses priority tags:
     - `[G]` — Release-gating tests (must pass)
     - `[H]` — Hardening/nice-to-have tests
     - `[E]` — Exploratory/fuzz tests
     - `[RC]` — Resource-constrained guardrails (memory/time ceilings)
   - Look at:
     - Automated test coverage (if metrics exist).
     - Breadth of tests (unit, integration, property-based, performance, regression).
     - Test results in `docs/meta/results/[branch-name].txt` for each completed cycle.
   - Estimate:
     - What fraction of the planned test surface (by phase and priority tag) is actually in place.
     - How much "hardening" work remains (e.g., `[E]` fuzz tests, `[RC]` memory stress tests).
   - Express as a percentage of "testing maturity" relative to where it should be at launch.

4. **Percent of polish / production readiness**
   - Evaluate:
     - Logging, observability, error handling.
     - Configuration, documentation, packaging, build/release pipelines.
   - Distinguish between:
     - “Prototype complete” vs “production-ready.”
   - Give a percentage that reflects how close we are to **shipping something robust to external users**, not just “it runs on my machine.”

5. **Risk-adjusted completion percentage**
   - Identify key **remaining risks**:
     - Hard unsolved tech problems.
     - Third-party or platform dependencies.
     - Performance unknowns, security/compliance gaps.
   - Provide:
     - A “raw” completion percentage (based on features/code).
     - A **risk-adjusted** completion (e.g., “raw feature completion might be 70%, but given remaining high-risk unknowns, effective completion feels more like 50–55%”).

If you identify any other useful quantitative or qualitative metrics for “completion,” add them and justify why they matter.

--------------------
4. How to approach the analysis
--------------------

1. **Understand the intended scope**
   - Read `excel_diff_meta_programming.md` for the authoritative development process.
   - Read `excel_diff_testing_plan.md` for the phased milestones and MVP definition.
   - Read `excel_diff_product_differentiation_plan.md` for the product roadmap (Phase 1–3).
   - If the code clearly diverges from the specs, mention that and treat the specs as **directional**, not absolute.

2. **Map the architecture**
   - Use `excel_diff_technical_document.md` as the reference for intended architecture.
   - Sketch (internally) a mental map of the system:
     - Major modules, data flows, external interfaces.
   - For each area, determine:
     - Implemented vs stubbed vs missing.
     - Covered by tests vs untested.

3. **Use multiple signals**
   - Combine:
     - Code inspection,
     - File structure and naming,
     - TODO/FIXME comments,
     - Git commit history (branch names: `YYYY-MM-DD-description`),
     - Activity logs (`docs/meta/logs/[branch-name]/activity_log.txt`),
     - Decision records (`docs/meta/plans/[branch-name].yaml`),
     - Test results (`docs/meta/results/[branch-name].txt`).
   - Do not rely solely on any single metric like LOC or commit count.

4. **Be explicit about uncertainty**
   - For every percentage you provide, include:
     - A **confidence level** (low/medium/high).
     - Key assumptions (e.g., "assuming the testing plan is up to date", "assuming no large untracked work remains").

--------------------
5. Deliverable format
--------------------

Return a structured report with:

1. **Executive summary**
   - A short narrative answer to:
     - "Roughly what percentage complete is this project?"
     - "What's the main thing left to do?"
     - "Which testing plan phase are we currently in or completing?"
   - A compact table summarizing all completion metrics:
     - Percent of difficulty overcome
     - Percent of code written
     - Percent of dev time elapsed
     - Percent of planned features implemented (by testing phase)
     - Percent of modules integrated
     - Percent of testing maturity (by priority tag: `[G]`/`[H]`/`[E]`/`[RC]`)
     - Risk-adjusted completion

2. **Detailed sections**
   - One section per metric (Sections 2 and 3 above), with:
     - Your numeric estimates,
     - Reasoning,
     - Pointers to specific areas of the codebase or artifacts that informed your judgment.

3. **Phase progress**
   - Map the current state to the testing plan phases (0–6).
   - Indicate which phases are complete, in progress, or not started.
   - Note any deviations from the phased approach.

4. **Risk / unknowns**
   - List the biggest uncertainties that could significantly change the completion estimate (e.g., "we have not yet profiled performance on 100MB workbooks").
   - Reference specific risks from `excel_diff_difficulty_analysis.md` if applicable.

Your goal is not to be perfectly precise, but to provide **realistic, grounded ranges** and a clear explanation of how close this codebase is to a shippable, production-ready version of the intended product.

--------------------
6. Using the collate workflow
--------------------

To gather all relevant artifacts for this analysis, you can use:

```bash
python docs/meta/prompts/generate_review_context.py --collate [branch-name]
```

This packages the following into a single output directory (in Downloads):
- All `docs/rust_docs/*.md` files (technical blueprints)
- The mini-spec (`mini_spec_[branch-name].md`)
- The decision record (`decision_[branch-name].txt`)
- The codebase context (`codebase_context.md` — generated review snapshot)
- A combined `cycle_summary.txt` containing:
  - Activity log from the cycle
  - Test results from the cycle
  - Manifest of all included files

This collated package provides a comprehensive view of both the planned and actual work for the most recent development cycle.
