# Perf Cycle Delta Summary

Cycle: `2026-02-06_014829`
Aggregation: `median` over **3 run(s)**
Pre: `a3a19be67121` (20260203_perf_improvement) at 2026-02-06T01:50:21.118525+00:00
Post: `a3a19be67121` (20260203_perf_improvement) at 2026-02-06T01:58:03.551882+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 33 | 33 | +0 ms (+0.0%) |
| `perf_50k_adversarial_repetitive` | 22 | 22 | +0 ms (+0.0%) |
| `perf_50k_alignment_block_move` | 298 | 357 | +59 ms (+19.8%) |
| `perf_50k_completely_different` | 238 | 243 | +5 ms (+2.1%) |
| `perf_50k_dense_single_edit` | 49 | 45 | -4 ms (-8.2%) |
| `perf_50k_identical` | 19 | 17 | -2 ms (-10.5%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2456 | 2432 | -24 ms (-1.0%) | 2446 | 2420 | -26 ms (-1.1%) | 11 | 10 | -1 ms (-9.1%) |
| `e2e_p2_noise` | 953 | 978 | +25 ms (+2.6%) | 943 | 966 | +23 ms (+2.4%) | 10 | 12 | +2 ms (+20.0%) |
| `e2e_p3_repetitive` | 2350 | 2415 | +65 ms (+2.8%) | 2326 | 2393 | +67 ms (+2.9%) | 24 | 22 | -2 ms (-8.3%) |
| `e2e_p4_sparse` | 94 | 97 | +3 ms (+3.2%) | 66 | 69 | +3 ms (+4.5%) | 28 | 28 | +0 ms (+0.0%) |
| `e2e_p5_identical` | 15075 | 15255 | +180 ms (+1.2%) | 15055 | 15238 | +183 ms (+1.2%) | 19 | 17 | -2 ms (-10.5%) |
