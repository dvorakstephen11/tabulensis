# Perf Cycle Delta Summary

Cycle: `2026-02-06_022721`
Aggregation: `median` over **3 run(s)**
Pre: `c11cdc83888c` (20260203_perf_improvement) at 2026-02-06T02:29:22.527161+00:00
Post: `baac1dcb1912` (20260203_perf_improvement) at 2026-02-06T03:17:15.168960+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 39 | 32 | -7 ms (-17.9%) |
| `perf_50k_adversarial_repetitive` | 23 | 25 | +2 ms (+8.7%) |
| `perf_50k_alignment_block_move` | 300 | 294 | -6 ms (-2.0%) |
| `perf_50k_completely_different` | 248 | 224 | -24 ms (-9.7%) |
| `perf_50k_dense_single_edit` | 46 | 42 | -4 ms (-8.7%) |
| `perf_50k_identical` | 17 | 14 | -3 ms (-17.6%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2674 | 2436 | -238 ms (-8.9%) | 2664 | 2428 | -236 ms (-8.9%) | 10 | 8 | -2 ms (-20.0%) |
| `e2e_p2_noise` | 1029 | 954 | -75 ms (-7.3%) | 1019 | 946 | -73 ms (-7.2%) | 10 | 9 | -1 ms (-10.0%) |
| `e2e_p3_repetitive` | 2454 | 2465 | +11 ms (+0.4%) | 2433 | 2444 | +11 ms (+0.5%) | 21 | 22 | +1 ms (+4.8%) |
| `e2e_p4_sparse` | 103 | 88 | -15 ms (-14.6%) | 71 | 63 | -8 ms (-11.3%) | 32 | 27 | -5 ms (-15.6%) |
| `e2e_p5_identical` | 14965 | 0 | -14965 ms (-100.0%) | 14949 | 0 | -14949 ms (-100.0%) | 18 | 0 | -18 ms (-100.0%) |
