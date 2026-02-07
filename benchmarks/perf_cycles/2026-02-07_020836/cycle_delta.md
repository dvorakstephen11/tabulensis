# Perf Cycle Delta Summary

Cycle: `2026-02-07_020836`
Aggregation: `median` over **3 run(s)**
Pre: `9e6e69bd29ac` (20260206_ui_improvements_milestone_3) at 2026-02-07T02:12:17.876501+00:00
Post: `9e6e69bd29ac` (20260206_ui_improvements_milestone_3) at 2026-02-07T02:41:26.534817+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 27 | 29 | +2 ms (+7.4%) |
| `perf_50k_adversarial_repetitive` | 20 | 23 | +3 ms (+15.0%) |
| `perf_50k_alignment_block_move` | 252 | 255 | +3 ms (+1.2%) |
| `perf_50k_completely_different` | 224 | 214 | -10 ms (-4.5%) |
| `perf_50k_dense_single_edit` | 40 | 45 | +5 ms (+12.5%) |
| `perf_50k_identical` | 13 | 18 | +5 ms (+38.5%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2155 | 2124 | -31 ms (-1.4%) | 2147 | 2115 | -32 ms (-1.5%) | 8 | 8 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 879 | 872 | -7 ms (-0.8%) | 867 | 864 | -3 ms (-0.3%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p3_repetitive` | 2128 | 2074 | -54 ms (-2.5%) | 2110 | 2054 | -56 ms (-2.7%) | 19 | 19 | +0 ms (+0.0%) |
| `e2e_p4_sparse` | 82 | 84 | +2 ms (+2.4%) | 57 | 59 | +2 ms (+3.5%) | 23 | 23 | +0 ms (+0.0%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 153 | 151 | -2 ms (-1.3%) | 153 | 151 | -2 ms (-1.3%) | 0 | 0 | +0 ms |
