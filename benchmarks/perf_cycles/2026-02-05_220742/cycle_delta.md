# Perf Cycle Delta Summary

Cycle: `2026-02-05_220742`
Aggregation: `median` over **3 run(s)**
Pre: `4be308bcbaa6` (20260203_perf_improvement) at 2026-02-05T22:08:55.652121+00:00
Post: `4be308bcbaa6` (20260203_perf_improvement) at 2026-02-05T22:12:53.850675+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 33 | 33 | +0 ms (+0.0%) |
| `perf_50k_adversarial_repetitive` | 25 | 23 | -2 ms (-8.0%) |
| `perf_50k_alignment_block_move` | 299 | 266 | -33 ms (-11.0%) |
| `perf_50k_completely_different` | 232 | 222 | -10 ms (-4.3%) |
| `perf_50k_dense_single_edit` | 51 | 42 | -9 ms (-17.6%) |
| `perf_50k_identical` | 20 | 18 | -2 ms (-10.0%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2308 | 2357 | +49 ms (+2.1%) | 2299 | 2347 | +48 ms (+2.1%) | 9 | 10 | +1 ms (+11.1%) |
| `e2e_p2_noise` | 978 | 943 | -35 ms (-3.6%) | 970 | 933 | -37 ms (-3.8%) | 8 | 9 | +1 ms (+12.5%) |
| `e2e_p3_repetitive` | 2355 | 2170 | -185 ms (-7.9%) | 2336 | 2150 | -186 ms (-8.0%) | 20 | 20 | +0 ms (+0.0%) |
| `e2e_p4_sparse` | 86 | 87 | +1 ms (+1.2%) | 61 | 62 | +1 ms (+1.6%) | 25 | 26 | +1 ms (+4.0%) |
| `e2e_p5_identical` | 14256 | 14166 | -90 ms (-0.6%) | 14244 | 14150 | -94 ms (-0.7%) | 13 | 14 | +1 ms (+7.7%) |
