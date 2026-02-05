# Perf Cycle Delta Summary

Cycle: `2026-02-05_172907`
Pre: `c370707ec9f3` (20260203_perf_improvement) at 2026-02-05T17:29:07.168248+00:00
Post: `c370707ec9f3` (20260203_perf_improvement) at 2026-02-05T17:47:09.838658+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 30 | 44 | +14 ms (+46.7%) |
| `perf_50k_adversarial_repetitive` | 19 | 25 | +6 ms (+31.6%) |
| `perf_50k_alignment_block_move` | 326 | 360 | +34 ms (+10.4%) |
| `perf_50k_completely_different` | 244 | 233 | -11 ms (-4.5%) |
| `perf_50k_dense_single_edit` | 43 | 51 | +8 ms (+18.6%) |
| `perf_50k_identical` | 16 | 21 | +5 ms (+31.2%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 3358 | 3539 | +181 ms (+5.4%) | 3349 | 3528 | +179 ms (+5.3%) | 9 | 11 | +2 ms (+22.2%) |
| `e2e_p2_noise` | 1370 | 1481 | +111 ms (+8.1%) | 1362 | 1472 | +110 ms (+8.1%) | 8 | 9 | +1 ms (+12.5%) |
| `e2e_p3_repetitive` | 4218 | 4487 | +269 ms (+6.4%) | 4199 | 4458 | +259 ms (+6.2%) | 19 | 29 | +10 ms (+52.6%) |
| `e2e_p4_sparse` | 145 | 175 | +30 ms (+20.7%) | 115 | 122 | +7 ms (+6.1%) | 30 | 53 | +23 ms (+76.7%) |
| `e2e_p5_identical` | 18484 | 25202 | +6718 ms (+36.3%) | 18468 | 25184 | +6716 ms (+36.4%) | 16 | 18 | +2 ms (+12.5%) |
