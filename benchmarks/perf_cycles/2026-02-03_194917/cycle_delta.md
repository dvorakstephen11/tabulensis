# Perf Cycle Delta Summary

Cycle: `2026-02-03_194917`
Pre: `60b1eaaa099d` (20260203_perf_improvement) at 2026-02-03T19:49:17.042980+00:00
Post: `60b1eaaa099d` (20260203_perf_improvement) at 2026-02-03T19:50:13.195554+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 29 | 29 | +0 ms (+0.0%) |
| `perf_50k_adversarial_repetitive` | 20 | 21 | +1 ms (+5.0%) |
| `perf_50k_alignment_block_move` | 523 | 265 | -258 ms (-49.3%) |
| `perf_50k_completely_different` | 211 | 220 | +9 ms (+4.3%) |
| `perf_50k_dense_single_edit` | 39 | 42 | +3 ms (+7.7%) |
| `perf_50k_identical` | 15 | 18 | +3 ms (+20.0%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2707 | 3711 | +1004 ms (+37.1%) | 2698 | 3702 | +1004 ms (+37.2%) | 9 | 9 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 1299 | 1423 | +124 ms (+9.5%) | 1291 | 1413 | +122 ms (+9.5%) | 8 | 10 | +2 ms (+25.0%) |
| `e2e_p3_repetitive` | 3848 | 4221 | +373 ms (+9.7%) | 3828 | 4196 | +368 ms (+9.6%) | 20 | 25 | +5 ms (+25.0%) |
| `e2e_p4_sparse` | 134 | 146 | +12 ms (+9.0%) | 106 | 113 | +7 ms (+6.6%) | 28 | 33 | +5 ms (+17.9%) |
| `e2e_p5_identical` | 15898 | 19762 | +3864 ms (+24.3%) | 15884 | 19747 | +3863 ms (+24.3%) | 14 | 15 | +1 ms (+7.1%) |
