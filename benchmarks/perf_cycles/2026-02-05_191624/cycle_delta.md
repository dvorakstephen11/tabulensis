# Perf Cycle Delta Summary

Cycle: `2026-02-05_191624`
Pre: `7ed19c558ceb` (20260203_perf_improvement) at 2026-02-05T19:16:24.776226+00:00
Post: `7ed19c558ceb` (20260203_perf_improvement) at 2026-02-05T19:16:56.231944+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 28 | 29 | +1 ms (+3.6%) |
| `perf_50k_adversarial_repetitive` | 20 | 20 | +0 ms (+0.0%) |
| `perf_50k_alignment_block_move` | 284 | 283 | -1 ms (-0.4%) |
| `perf_50k_completely_different` | 207 | 214 | +7 ms (+3.4%) |
| `perf_50k_dense_single_edit` | 40 | 41 | +1 ms (+2.5%) |
| `perf_50k_identical` | 18 | 16 | -2 ms (-11.1%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2648 | 2767 | +119 ms (+4.5%) | 2639 | 2759 | +120 ms (+4.5%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p2_noise` | 1233 | 1280 | +47 ms (+3.8%) | 1225 | 1271 | +46 ms (+3.8%) | 8 | 9 | +1 ms (+12.5%) |
| `e2e_p3_repetitive` | 3553 | 3703 | +150 ms (+4.2%) | 3535 | 3683 | +148 ms (+4.2%) | 18 | 20 | +2 ms (+11.1%) |
| `e2e_p4_sparse` | 119 | 123 | +4 ms (+3.4%) | 94 | 98 | +4 ms (+4.3%) | 25 | 25 | +0 ms (+0.0%) |
| `e2e_p5_identical` | 15980 | 16023 | +43 ms (+0.3%) | 15964 | 16008 | +44 ms (+0.3%) | 16 | 15 | -1 ms (-6.2%) |
