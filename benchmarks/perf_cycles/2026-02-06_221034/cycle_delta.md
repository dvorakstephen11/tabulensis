# Perf Cycle Delta Summary

Cycle: `2026-02-06_221034`
Aggregation: `median` over **3 run(s)**
Pre: `86cd17e8c016` (20260206_ui_improvements_milestone_3) at 2026-02-06T22:14:27.232906+00:00
Post: `5d6cf6101a9d` (20260206_ui_improvements_milestone_3) at 2026-02-06T22:35:45.010050+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 41 | 28 | -13 ms (-31.7%) |
| `perf_50k_adversarial_repetitive` | 22 | 21 | -1 ms (-4.5%) |
| `perf_50k_alignment_block_move` | 262 | 306 | +44 ms (+16.8%) |
| `perf_50k_completely_different` | 212 | 212 | +0 ms (+0.0%) |
| `perf_50k_dense_single_edit` | 49 | 37 | -12 ms (-24.5%) |
| `perf_50k_identical` | 14 | 13 | -1 ms (-7.1%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2153 | 2452 | +299 ms (+13.9%) | 2141 | 2443 | +302 ms (+14.1%) | 8 | 9 | +1 ms (+12.5%) |
| `e2e_p2_noise` | 885 | 965 | +80 ms (+9.0%) | 876 | 955 | +79 ms (+9.0%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p3_repetitive` | 2110 | 2291 | +181 ms (+8.6%) | 2089 | 2272 | +183 ms (+8.8%) | 21 | 19 | -2 ms (-9.5%) |
| `e2e_p4_sparse` | 89 | 85 | -4 ms (-4.5%) | 62 | 60 | -2 ms (-3.2%) | 27 | 26 | -1 ms (-3.7%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
