# Perf Cycle Delta Summary

Cycle: `2026-02-05_213657`
Aggregation: `median` over **1 run(s)**
Pre: `3c90b94aec14` (20260203_perf_improvement) at 2026-02-05T21:37:30.954315+00:00
Post: `3c90b94aec14` (20260203_perf_improvement) at 2026-02-05T21:38:09.408747+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 42 | 61 | +19 ms (+45.2%) |
| `perf_50k_adversarial_repetitive` | 32 | 31 | -1 ms (-3.1%) |
| `perf_50k_alignment_block_move` | 416 | 430 | +14 ms (+3.4%) |
| `perf_50k_completely_different` | 358 | 301 | -57 ms (-15.9%) |
| `perf_50k_dense_single_edit` | 75 | 59 | -16 ms (-21.3%) |
| `perf_50k_identical` | 25 | 24 | -1 ms (-4.0%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 3383 | 3591 | +208 ms (+6.1%) | 3373 | 3579 | +206 ms (+6.1%) | 10 | 12 | +2 ms (+20.0%) |
| `e2e_p2_noise` | 1247 | 1193 | -54 ms (-4.3%) | 1236 | 1182 | -54 ms (-4.4%) | 11 | 11 | +0 ms (+0.0%) |
| `e2e_p3_repetitive` | 3305 | 3172 | -133 ms (-4.0%) | 3277 | 3136 | -141 ms (-4.3%) | 28 | 36 | +8 ms (+28.6%) |
| `e2e_p4_sparse` | 129 | 132 | +3 ms (+2.3%) | 93 | 99 | +6 ms (+6.5%) | 36 | 33 | -3 ms (-8.3%) |
| `e2e_p5_identical` | 19667 | 19558 | -109 ms (-0.6%) | 19641 | 19540 | -101 ms (-0.5%) | 26 | 18 | -8 ms (-30.8%) |
