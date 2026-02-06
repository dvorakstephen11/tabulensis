# Perf Cycle Delta Summary

Cycle: `2026-02-05_223828`
Aggregation: `median` over **3 run(s)**
Pre: `a3a19be67121` (20260203_perf_improvement) at 2026-02-05T22:39:45.113429+00:00
Post: `a3a19be67121` (20260203_perf_improvement) at 2026-02-05T22:42:07.988268+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 36 | 32 | -4 ms (-11.1%) |
| `perf_50k_adversarial_repetitive` | 26 | 20 | -6 ms (-23.1%) |
| `perf_50k_alignment_block_move` | 331 | 277 | -54 ms (-16.3%) |
| `perf_50k_completely_different` | 291 | 234 | -57 ms (-19.6%) |
| `perf_50k_dense_single_edit` | 44 | 48 | +4 ms (+9.1%) |
| `perf_50k_identical` | 16 | 15 | -1 ms (-6.2%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2610 | 2301 | -309 ms (-11.8%) | 2600 | 2290 | -310 ms (-11.9%) | 9 | 11 | +2 ms (+22.2%) |
| `e2e_p2_noise` | 974 | 925 | -49 ms (-5.0%) | 964 | 915 | -49 ms (-5.1%) | 9 | 10 | +1 ms (+11.1%) |
| `e2e_p3_repetitive` | 2383 | 2208 | -175 ms (-7.3%) | 2361 | 2189 | -172 ms (-7.3%) | 22 | 19 | -3 ms (-13.6%) |
| `e2e_p4_sparse` | 94 | 90 | -4 ms (-4.3%) | 66 | 63 | -3 ms (-4.5%) | 28 | 26 | -2 ms (-7.1%) |
| `e2e_p5_identical` | 14755 | 13510 | -1245 ms (-8.4%) | 14737 | 13498 | -1239 ms (-8.4%) | 18 | 12 | -6 ms (-33.3%) |
