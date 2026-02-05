# Perf Cycle Delta Summary

Cycle: `2026-02-05_221815`
Aggregation: `median` over **3 run(s)**
Pre: `114dc862602c` (20260203_perf_improvement) at 2026-02-05T22:20:32.532705+00:00
Post: `114dc862602c` (20260203_perf_improvement) at 2026-02-05T22:24:07.464897+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 35 | 32 | -3 ms (-8.6%) |
| `perf_50k_adversarial_repetitive` | 21 | 22 | +1 ms (+4.8%) |
| `perf_50k_alignment_block_move` | 266 | 274 | +8 ms (+3.0%) |
| `perf_50k_completely_different` | 219 | 223 | +4 ms (+1.8%) |
| `perf_50k_dense_single_edit` | 45 | 41 | -4 ms (-8.9%) |
| `perf_50k_identical` | 14 | 15 | +1 ms (+7.1%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2396 | 2354 | -42 ms (-1.8%) | 2387 | 2345 | -42 ms (-1.8%) | 9 | 9 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 967 | 927 | -40 ms (-4.1%) | 957 | 916 | -41 ms (-4.3%) | 10 | 10 | +0 ms (+0.0%) |
| `e2e_p3_repetitive` | 2284 | 2280 | -4 ms (-0.2%) | 2264 | 2257 | -7 ms (-0.3%) | 21 | 22 | +1 ms (+4.8%) |
| `e2e_p4_sparse` | 96 | 89 | -7 ms (-7.3%) | 67 | 64 | -3 ms (-4.5%) | 27 | 25 | -2 ms (-7.4%) |
| `e2e_p5_identical` | 14302 | 13924 | -378 ms (-2.6%) | 14289 | 13907 | -382 ms (-2.7%) | 13 | 17 | +4 ms (+30.8%) |
