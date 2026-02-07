# Perf Cycle Delta Summary

Cycle: `2026-02-07_210651`
Aggregation: `median` over **3 run(s)**
Pre: `c4d57470f97c` (main) at 2026-02-07T21:10:22.549237+00:00
Post: `7444a8c52fa7` (main) at 2026-02-07T21:17:43.254438+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 37 | 30 | -7 ms (-18.9%) |
| `perf_50k_adversarial_repetitive` | 22 | 31 | +9 ms (+40.9%) |
| `perf_50k_alignment_block_move` | 351 | 374 | +23 ms (+6.6%) |
| `perf_50k_completely_different` | 252 | 269 | +17 ms (+6.7%) |
| `perf_50k_dense_single_edit` | 48 | 51 | +3 ms (+6.2%) |
| `perf_50k_identical` | 20 | 17 | -3 ms (-15.0%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2828 | 3321 | +493 ms (+17.4%) | 2820 | 3310 | +490 ms (+17.4%) | 13 | 12 | -1 ms (-7.7%) |
| `e2e_p2_noise` | 1110 | 1392 | +282 ms (+25.4%) | 1092 | 1377 | +285 ms (+26.1%) | 15 | 13 | -2 ms (-13.3%) |
| `e2e_p3_repetitive` | 2691 | 3221 | +530 ms (+19.7%) | 2665 | 3188 | +523 ms (+19.6%) | 26 | 33 | +7 ms (+26.9%) |
| `e2e_p4_sparse` | 102 | 130 | +28 ms (+27.5%) | 73 | 93 | +20 ms (+27.4%) | 33 | 45 | +12 ms (+36.4%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 202 | 233 | +31 ms (+15.3%) | 202 | 233 | +31 ms (+15.3%) | 0 | 0 | +0 ms |

## CLI JSONL Emit (total/op_emit time)
| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |
| --- | --- | --- | --- | --- | --- | --- |
| `cli_perf_jsonl_emit` | 47 | 44 | -3 ms (-6.4%) | 47 | 44 | -3 ms (-6.4%) |
