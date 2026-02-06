# Perf Cycle Delta Summary

Cycle: `2026-02-06_213047`
Aggregation: `median` over **3 run(s)**
Pre: `f3fc4347ac45` (20260206_ui_improvements_milestone_3) at 2026-02-06T21:34:15.875852+00:00
Post: `f3fc4347ac45` (20260206_ui_improvements_milestone_3) at 2026-02-06T21:48:35.658763+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 27 | 28 | +1 ms (+3.7%) |
| `perf_50k_adversarial_repetitive` | 20 | 21 | +1 ms (+5.0%) |
| `perf_50k_alignment_block_move` | 237 | 247 | +10 ms (+4.2%) |
| `perf_50k_completely_different` | 205 | 217 | +12 ms (+5.9%) |
| `perf_50k_dense_single_edit` | 39 | 40 | +1 ms (+2.6%) |
| `perf_50k_identical` | 13 | 13 | +0 ms (+0.0%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2119 | 2292 | +173 ms (+8.2%) | 2111 | 2283 | +172 ms (+8.1%) | 9 | 9 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 886 | 917 | +31 ms (+3.5%) | 878 | 909 | +31 ms (+3.5%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p3_repetitive` | 2234 | 2153 | -81 ms (-3.6%) | 2212 | 2134 | -78 ms (-3.5%) | 22 | 19 | -3 ms (-13.6%) |
| `e2e_p4_sparse` | 83 | 83 | +0 ms (+0.0%) | 56 | 59 | +3 ms (+5.4%) | 26 | 25 | -1 ms (-3.8%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
