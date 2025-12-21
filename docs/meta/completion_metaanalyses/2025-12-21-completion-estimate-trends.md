# Completion Estimate Trends (2025-11-26 to 2025-12-22)

Method: pulled the headline metrics from each `docs/meta/completion_estimates/*.md` summary table.

Normalization:

- If a cell included a central point estimate + a range (e.g., `~70% (68-74%)` or `~12% (range 8-15%)`), the point estimate is shown.
- If a cell was only a range (e.g., `15-20%`), the midpoint is shown.
- OpenAI `2025-12-18-branch-7` includes two dev-time scopes; the tables below use the first ("full roadmap") value for consistency.

Metrics are percentages.

## Gemini Trajectory

| Date | Report | Difficulty | Code | Dev Time | Risk-Adj |
| :--- | :----- | ---------: | ---: | -------: | -------: |
| 2025-11-26 | container-and-grid-ir | 13.7 | 17.3 | 8 | 13 |
| 2025-11-27 | datamashup-framing | 24 | 23.5 | 7 | 16.5 |
| 2025-11-28 | cell-snapshots-pg3 | 24 | 27 | 14 | 20 |
| 2025-12-03 | m4-packageparts | 32 | 29.5 | 22 | 26.5 |
| 2025-12-04c | m6-textual-m-diff | 33 | 45 | 35 | 30 |
| 2025-12-05d | g9-column-alignment | 36 | 45 | 35 | 30 |
| 2025-12-07 | m-ast-equality | 55 | 45 | 40 | 45 |
| 2025-12-18 | branch-7 | 78 | 60 | 65 | 70 |
| 2025-12-22 | branch-6 | 90 | 88 | 85 | 85 |

## OpenAI Trajectory

| Date | Report | Difficulty | Code | Dev Time | Risk-Adj |
| :--- | :----- | ---------: | ---: | -------: | -------: |
| 2025-11-26 | container-and-grid-ir | 12 | 17 | 10 | 11 |
| 2025-11-27 | datamashup-framing | 17.5 | 22.5 | 20 | 12.5 |
| 2025-11-28 | cell-snapshots-pg3 | 21 | 30 | 20 | 20 |
| 2025-12-04c | m6-textual-m-diff | 32.5 | 55 | 35 | 27.5 |
| 2025-12-07 | m-ast-equality | 45 | 60 | 40 | 45 |
| 2025-12-18 | branch-7 | 62.5 | 70 | 40 | 52.5 |
| 2025-12-22 | branch-6 | 70 | 79 | 60 | 62 |

## Model Differences On Shared Dates (Gemini - OpenAI)

| Date | Report | Difficulty (G-O) | Code (G-O) | Dev Time (G-O) | Risk-Adj (G-O) |
| :--- | :----- | ---------------: | ---------: | -------------: | -------------: |
| 2025-11-26 | container-and-grid-ir | +1.7 | +0.3 | -2 | +2 |
| 2025-11-27 | datamashup-framing | +6.5 | +1 | -13 | +4 |
| 2025-11-28 | cell-snapshots-pg3 | +3 | -3 | -6 | 0 |
| 2025-12-04c | m6-textual-m-diff | +0.5 | -10 | 0 | +2.5 |
| 2025-12-07 | m-ast-equality | +10 | -15 | 0 | 0 |
| 2025-12-18 | branch-7 | +15.5 | -10 | +25 | +17.5 |
| 2025-12-22 | branch-6 | +20 | +9 | +25 | +23 |

## Cross-Cutting Trends

- **Phase shift in what remains.**
  - **11-26 to 11-28:** both series describe a strong foundation (XLSX container parsing, grid IR, DataMashup framing), with the "main remaining work" being the diff engines (grid + M) and product surfaces.
  - **12-03 to 12-07:** focus moves to semantic structure and alignment primitives (PackageParts, textual M diffs, column alignment, M AST equality). By **12-07**, both models converge on **~45% risk-adjusted**, suggesting the core-engine plan is no longer speculative.
  - **12-18 to 12-22:** reports pivot to "shipping surfaces": CLI + output UX, web/WASM viewer, perf/RC guardrails, and PBIX host support. The biggest remaining unknowns are no longer "can we implement alignment" but "can we ship it reliably across real-world files and hosts".

- **Convergence then divergence.**
  - By **12-07**, both models match on **Risk-Adj = 45%** and **Dev Time = 40%**, even though they disagree on Code (Gemini 45 vs OpenAI 60).
  - After that, the gap widens: by **12-22**, Gemini is at **85% risk-adjusted** while OpenAI is at **62%** (G-O +23). The dev-time gap is similarly large (G-O +25), implying different denominators/scope assumptions.

- **Denominator effects increasingly dominate the headline numbers.**
  - Late reports explicitly split "Excel-first MVP" vs "full roadmap" (PBIX + DAX/model + deeper semantics). When those are counted in-scope, OpenAI's percentages stay materially lower; when treated as post-MVP, Gemini's climb faster.
  - Practically: the series is most comparable *within* a model over time; cross-model comparisons late in the timeline reflect scope/definition differences as much as code changes.

## What "Remaining Work" Looks Like In Late Reports (12-18 / 12-22)

Recurring themes (present in one or both models):

- **PBIX/PBIT host support (Phase 3.5)** remains the biggest missing integration slice.
- **DAX/tabular-model diff (Phase 6)** is unstarted and is the main reason "full roadmap" denominators stay low.
- **Web/WASM viewer** is still a thin/skeletal layer compared to a launchable UI.
- **Perf/memory guardrails** exist but still have open validation/tuning items (the exact bottleneck differs by report).
- **Diff semantics depth** still matters for diff quality/noise (step-aware M diffs, formula semantic diff gating, and database-mode edge cases like duplicates/clusters).

## Takeaways

- If "done" means an **Excel-first CLI MVP**, the late reports place the project in **Phase 5** and within striking distance of an RC; the remaining work is mostly productization and hardening.
- If "done" means the **full roadmap (including PBIX + DAX/model diff)**, OpenAI's series is a closer fit: **~70% difficulty / ~62% risk-adjusted**, with the remaining risk concentrated in PBIX hosting and deeper semantics.
