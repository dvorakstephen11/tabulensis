# Perf Cycle Delta Summary

Cycle: `2026-02-03_211840`
Pre: `c370707ec9f3` (20260203_perf_improvement) at 2026-02-03T21:19:31.604809+00:00
Post: `c370707ec9f3` (20260203_perf_improvement) at 2026-02-03T21:31:37.243306+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 46 | 64 | +18 ms (+39.1%) |
| `perf_50k_adversarial_repetitive` | 31 | 27 | -4 ms (-12.9%) |
| `perf_50k_alignment_block_move` | 361 | 432 | +71 ms (+19.7%) |
| `perf_50k_completely_different` | 366 | 328 | -38 ms (-10.4%) |
| `perf_50k_dense_single_edit` | 66 | 56 | -10 ms (-15.2%) |
| `perf_50k_identical` | 25 | 34 | +9 ms (+36.0%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 4655 | 4240 | -415 ms (-8.9%) | 4643 | 4228 | -415 ms (-8.9%) | 12 | 12 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 2146 | 1891 | -255 ms (-11.9%) | 2135 | 1880 | -255 ms (-11.9%) | 11 | 11 | +0 ms (+0.0%) |
| `e2e_p3_repetitive` | 5754 | 5319 | -435 ms (-7.6%) | 5729 | 5295 | -434 ms (-7.6%) | 25 | 24 | -1 ms (-4.0%) |
| `e2e_p4_sparse` | 186 | 176 | -10 ms (-5.4%) | 144 | 142 | -2 ms (-1.4%) | 42 | 34 | -8 ms (-19.0%) |
| `e2e_p5_identical` | 28397 | 28043 | -354 ms (-1.2%) | 28376 | 28017 | -359 ms (-1.3%) | 21 | 26 | +5 ms (+23.8%) |
