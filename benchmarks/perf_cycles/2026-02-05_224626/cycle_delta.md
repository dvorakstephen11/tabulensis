# Perf Cycle Delta Summary

Cycle: `2026-02-05_224626`
Aggregation: `median` over **5 run(s)**
Pre: `a3a19be67121` (20260203_perf_improvement) at 2026-02-05T22:49:05.956605+00:00
Post: `a3a19be67121` (20260203_perf_improvement) at 2026-02-05T22:53:40.028168+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 31 | 30 | -1 ms (-3.2%) |
| `perf_50k_adversarial_repetitive` | 20 | 22 | +2 ms (+10.0%) |
| `perf_50k_alignment_block_move` | 260 | 264 | +4 ms (+1.5%) |
| `perf_50k_completely_different` | 213 | 222 | +9 ms (+4.2%) |
| `perf_50k_dense_single_edit` | 40 | 39 | -1 ms (-2.5%) |
| `perf_50k_identical` | 14 | 12 | -2 ms (-14.3%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2268 | 2253 | -15 ms (-0.7%) | 2260 | 2245 | -15 ms (-0.7%) | 9 | 9 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 911 | 924 | +13 ms (+1.4%) | 902 | 915 | +13 ms (+1.4%) | 9 | 9 | +0 ms (+0.0%) |
| `e2e_p3_repetitive` | 2152 | 2322 | +170 ms (+7.9%) | 2132 | 2302 | +170 ms (+8.0%) | 20 | 19 | -1 ms (-5.0%) |
| `e2e_p4_sparse` | 89 | 88 | -1 ms (-1.1%) | 64 | 64 | +0 ms (+0.0%) | 25 | 26 | +1 ms (+4.0%) |
| `e2e_p5_identical` | 13113 | 13535 | +422 ms (+3.2%) | 13095 | 13517 | +422 ms (+3.2%) | 17 | 15 | -2 ms (-11.8%) |
