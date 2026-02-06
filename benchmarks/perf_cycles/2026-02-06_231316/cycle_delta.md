# Perf Cycle Delta Summary

Cycle: `2026-02-06_231316`
Aggregation: `median` over **3 run(s)**
Pre: `a481f01a6a06` (20260206_ui_improvements_milestone_3) at 2026-02-06T23:16:58.751874+00:00
Post: `2e418c6b27b3` (20260206_ui_improvements_milestone_3) at 2026-02-06T23:25:04.132346+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 28 | 30 | +2 ms (+7.1%) |
| `perf_50k_adversarial_repetitive` | 20 | 21 | +1 ms (+5.0%) |
| `perf_50k_alignment_block_move` | 251 | 255 | +4 ms (+1.6%) |
| `perf_50k_completely_different` | 215 | 211 | -4 ms (-1.9%) |
| `perf_50k_dense_single_edit` | 39 | 40 | +1 ms (+2.6%) |
| `perf_50k_identical` | 13 | 14 | +1 ms (+7.7%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2069 | 2110 | +41 ms (+2.0%) | 2061 | 2101 | +40 ms (+1.9%) | 8 | 9 | +1 ms (+12.5%) |
| `e2e_p2_noise` | 882 | 886 | +4 ms (+0.5%) | 873 | 878 | +5 ms (+0.6%) | 8 | 9 | +1 ms (+12.5%) |
| `e2e_p3_repetitive` | 2074 | 2081 | +7 ms (+0.3%) | 2057 | 2064 | +7 ms (+0.3%) | 19 | 18 | -1 ms (-5.3%) |
| `e2e_p4_sparse` | 83 | 80 | -3 ms (-3.6%) | 60 | 56 | -4 ms (-6.7%) | 24 | 24 | +0 ms (+0.0%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 879 | 152 | -727 ms (-82.7%) | 876 | 152 | -724 ms (-82.6%) | 3 | 0 | -3 ms (-100.0%) |
