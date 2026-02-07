# Perf Cycle Delta Summary

Cycle: `2026-02-07_194721`
Aggregation: `median` over **3 run(s)**
Pre: `c54ad1d8b8f6` (main) at 2026-02-07T19:51:15.924341+00:00
Post: `0f86267b935e` (main) at 2026-02-07T20:00:57.974310+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 30 | 29 | -1 ms (-3.3%) |
| `perf_50k_adversarial_repetitive` | 23 | 24 | +1 ms (+4.3%) |
| `perf_50k_alignment_block_move` | 264 | 287 | +23 ms (+8.7%) |
| `perf_50k_completely_different` | 222 | 230 | +8 ms (+3.6%) |
| `perf_50k_dense_single_edit` | 44 | 44 | +0 ms (+0.0%) |
| `perf_50k_identical` | 18 | 20 | +2 ms (+11.1%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2158 | 2262 | +104 ms (+4.8%) | 2149 | 2252 | +103 ms (+4.8%) | 9 | 9 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 897 | 910 | +13 ms (+1.4%) | 888 | 901 | +13 ms (+1.5%) | 9 | 9 | +0 ms (+0.0%) |
| `e2e_p3_repetitive` | 2104 | 2147 | +43 ms (+2.0%) | 2084 | 2126 | +42 ms (+2.0%) | 20 | 21 | +1 ms (+5.0%) |
| `e2e_p4_sparse` | 84 | 83 | -1 ms (-1.2%) | 60 | 60 | +0 ms (+0.0%) | 25 | 24 | -1 ms (-4.0%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 158 | 161 | +3 ms (+1.9%) | 158 | 161 | +3 ms (+1.9%) | 0 | 0 | +0 ms |

## CLI JSONL Emit (total/op_emit time)
| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |
| --- | --- | --- | --- | --- | --- | --- |
| `cli_perf_jsonl_emit` | 34 | 33 | -1 ms (-2.9%) | 34 | 33 | -1 ms (-2.9%) |
