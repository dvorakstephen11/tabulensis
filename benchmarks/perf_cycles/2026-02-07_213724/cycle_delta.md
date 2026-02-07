# Perf Cycle Delta Summary

Cycle: `2026-02-07_213724`
Aggregation: `median` over **3 run(s)**
Pre: `66ae09f5f6e2` (main) at 2026-02-07T21:47:15.927304+00:00
Post: `186561630fcb` (main) at 2026-02-07T21:55:47.338378+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 43 | 38 | -5 ms (-11.6%) |
| `perf_50k_adversarial_repetitive` | 27 | 25 | -2 ms (-7.4%) |
| `perf_50k_alignment_block_move` | 379 | 302 | -77 ms (-20.3%) |
| `perf_50k_completely_different` | 260 | 241 | -19 ms (-7.3%) |
| `perf_50k_dense_single_edit` | 49 | 51 | +2 ms (+4.1%) |
| `perf_50k_identical` | 21 | 18 | -3 ms (-14.3%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2696 | 2693 | -3 ms (-0.1%) | 2685 | 2680 | -5 ms (-0.2%) | 11 | 12 | +1 ms (+9.1%) |
| `e2e_p2_noise` | 1145 | 962 | -183 ms (-16.0%) | 1119 | 953 | -166 ms (-14.8%) | 13 | 9 | -4 ms (-30.8%) |
| `e2e_p3_repetitive` | 2728 | 2414 | -314 ms (-11.5%) | 2703 | 2393 | -310 ms (-11.5%) | 27 | 21 | -6 ms (-22.2%) |
| `e2e_p4_sparse` | 110 | 91 | -19 ms (-17.3%) | 79 | 66 | -13 ms (-16.5%) | 31 | 27 | -4 ms (-12.9%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 197 | 172 | -25 ms (-12.7%) | 197 | 172 | -25 ms (-12.7%) | 0 | 0 | +0 ms |

## CLI JSONL Emit (total/op_emit time)
| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |
| --- | --- | --- | --- | --- | --- | --- |
| `cli_perf_jsonl_emit` | 43 | 39 | -4 ms (-9.3%) | 43 | 39 | -4 ms (-9.3%) |
