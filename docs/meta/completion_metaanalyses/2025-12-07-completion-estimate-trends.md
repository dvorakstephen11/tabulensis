# Completion Estimate Trends (Nov 26 - Dec 07, 2025)

Method: pulled the headline metrics from each `docs/meta/completion_estimates/*.md` summary table. For ranges, the midpoint is shown (e.g., 15-18% -> 16.5). Metrics are percentages.

## Gemini Trajectory

| Date | Report | Difficulty | Code | Dev Time | Risk-Adj |
| :--- | :----- | :--------- | :--- | :------- | :------- |
| 2025-11-26 | container-and-grid-ir | 13.7 | 17.3 | 8 | 13 |
| 2025-11-27 | datamashup-framing | 24 | 23.5 | 7 | 16.5 |
| 2025-11-28 | cell-snapshots-pg3 | 24 | 27 | 14 | 20 |
| 2025-12-03 | m4-packageparts | 32.0 | 29.5 | 22 | 26.5 |
| 2025-12-04c | m6-textual-m-diff | 33 | 45 | 35 | 30 |
| 2025-12-05d | g9-column-alignment | 36 | 45 | 35 | 30 |
| 2025-12-07 | m-ast-equality | 55 | 45 | 40 | 45 |

## OpenAI Trajectory

| Date | Report | Difficulty | Code | Dev Time | Risk-Adj |
| :--- | :----- | :--------- | :--- | :------- | :------- |
| 2025-11-26 | container-and-grid-ir | 12 | 17 | 10 | 11 |
| 2025-11-27 | datamashup-framing | 17.5 | 22.5 | 20 | 12.5 |
| 2025-11-28 | cell-snapshots-pg3 | 21 | 30 | 20 | 20 |
| 2025-12-04c | m6-textual-m-diff | 32.5 | 55 | 35 | 27.5 |
| 2025-12-07 | m-ast-equality | 45 | 60 | 40 | 45 |

## Cross-Cutting Trends

- Difficulty overcome climbed sharply in both series: Gemini from 13.7 -> 55 (+41.3) and OpenAI from 12 -> 45 (+33) over 11 days, with the steepest jumps after 2025-12-03 (alignment and M-AST focus).
- Code-written estimates stabilized: Gemini leveled at ~45% after 2025-12-04, while OpenAI rose to ~60% by 2025-12-07, implying differing views on how much of the remaining surface area (PBIX/DAX/UI) is counted.
- Dev-time elapsed kept pace with difficulty, rising from single digits to ~40%; both timelines now assume roughly 60% of effort remains.
- Risk-adjusted completion moved more slowly (mid-teens -> ~30 by 12-05), then spiked to 45% on 12-07, indicating perceived resolution of the largest algorithmic unknowns.
- Convergence: by 12-07 both models agree on similar risk-adjusted and dev-time values (~45% and ~40%), suggesting alignment on remaining scope (semantic M diff, memory/streaming, product shell).

## Takeaways

- The early period (11-26 to 11-28) shows rapid foundational progress with modest risk burn-down; most gains were plumbing/tests, not core intelligence.
- The mid period (12-03 to 12-05) adds alignment work and textual M diff; code % stabilizes while difficulty inches up, highlighting algorithmic heaviness.
- The latest report (12-07) credits major difficulty reduction without increased code %, implying breakthroughs in approach/design rather than volume.
- Remaining risk is concentrated in semantic M diff, memory/streaming, and productization (CLI/WASM/PBIX); both assess roughly half the journey still ahead despite the recent spike in solved difficulty.
