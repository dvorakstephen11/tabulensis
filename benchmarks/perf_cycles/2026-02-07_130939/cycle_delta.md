# Perf Cycle Delta Summary

Cycle: `2026-02-07_130939`
Aggregation: `median` over **3 run(s)**
Pre: `87ea95ea9fd9` (20260206_ui_improvements_milestone_3) at 2026-02-07T13:10:10.428078+00:00
Post: `2d255a0365ab` (20260206_ui_improvements_milestone_3) at 2026-02-07T13:12:08.299622+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 30 | 31 | +1 ms (+3.3%) |
| `perf_50k_adversarial_repetitive` | 22 | 24 | +2 ms (+9.1%) |
| `perf_50k_alignment_block_move` | 286 | 297 | +11 ms (+3.8%) |
| `perf_50k_completely_different` | 232 | 239 | +7 ms (+3.0%) |
| `perf_50k_dense_single_edit` | 41 | 44 | +3 ms (+7.3%) |
| `perf_50k_identical` | 18 | 19 | +1 ms (+5.6%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2287 | 2185 | -102 ms (-4.5%) | 2278 | 2178 | -100 ms (-4.4%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p2_noise` | 924 | 911 | -13 ms (-1.4%) | 916 | 902 | -14 ms (-1.5%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p3_repetitive` | 2211 | 2221 | +10 ms (+0.5%) | 2193 | 2201 | +8 ms (+0.4%) | 19 | 20 | +1 ms (+5.3%) |
| `e2e_p4_sparse` | 81 | 83 | +2 ms (+2.5%) | 58 | 58 | +0 ms (+0.0%) | 23 | 25 | +2 ms (+8.7%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 152 | 152 | +0 ms (+0.0%) | 152 | 152 | +0 ms (+0.0%) | 0 | 0 | +0 ms |

## CLI JSONL Emit (total/op_emit time)
| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |
| --- | --- | --- | --- | --- | --- | --- |
| `cli_perf_jsonl_emit` | 74 | 32 | -42 ms (-56.8%) | 74 | 32 | -42 ms (-56.8%) |
