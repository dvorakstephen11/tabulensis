# Perf Cycle Delta Summary

Cycle: `2026-02-06_224104`
Aggregation: `median` over **3 run(s)**
Pre: `55dc8616ae6c` (20260206_ui_improvements_milestone_3) at 2026-02-06T22:45:06.966804+00:00
Post: `9e292f59ca04` (20260206_ui_improvements_milestone_3) at 2026-02-06T22:54:10.950480+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 30 | 27 | -3 ms (-10.0%) |
| `perf_50k_adversarial_repetitive` | 19 | 18 | -1 ms (-5.3%) |
| `perf_50k_alignment_block_move` | 269 | 248 | -21 ms (-7.8%) |
| `perf_50k_completely_different` | 212 | 212 | +0 ms (+0.0%) |
| `perf_50k_dense_single_edit` | 39 | 39 | +0 ms (+0.0%) |
| `perf_50k_identical` | 14 | 13 | -1 ms (-7.1%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2060 | 2065 | +5 ms (+0.2%) | 2052 | 2056 | +4 ms (+0.2%) | 8 | 9 | +1 ms (+12.5%) |
| `e2e_p2_noise` | 867 | 855 | -12 ms (-1.4%) | 858 | 847 | -11 ms (-1.3%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p3_repetitive` | 2052 | 2033 | -19 ms (-0.9%) | 2034 | 2013 | -21 ms (-1.0%) | 18 | 20 | +2 ms (+11.1%) |
| `e2e_p4_sparse` | 84 | 81 | -3 ms (-3.6%) | 60 | 54 | -6 ms (-10.0%) | 23 | 25 | +2 ms (+8.7%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
