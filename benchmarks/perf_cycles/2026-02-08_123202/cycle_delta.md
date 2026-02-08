# Perf Cycle Delta Summary

Cycle: `2026-02-08_123202`
Aggregation: `median` over **3 run(s)**
Pre: `d06024b3bee2` (main) at 2026-02-08T12:35:59.689744+00:00
Post: `d06024b3bee2` (main) at 2026-02-08T15:36:26.831993+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 29 | 30 | +1 ms (+3.4%) |
| `perf_50k_adversarial_repetitive` | 21 | 23 | +2 ms (+9.5%) |
| `perf_50k_alignment_block_move` | 280 | 277 | -3 ms (-1.1%) |
| `perf_50k_completely_different` | 215 | 235 | +20 ms (+9.3%) |
| `perf_50k_dense_single_edit` | 42 | 46 | +4 ms (+9.5%) |
| `perf_50k_identical` | 17 | 18 | +1 ms (+5.9%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 1670 | 2004 | +334 ms (+20.0%) | 1662 | 1996 | +334 ms (+20.1%) | 8 | 8 | +0 ms (+0.0%) |
| `e2e_p2_noise` | 597 | 618 | +21 ms (+3.5%) | 588 | 610 | +22 ms (+3.7%) | 9 | 8 | -1 ms (-11.1%) |
| `e2e_p3_repetitive` | 1197 | 1395 | +198 ms (+16.5%) | 1179 | 1373 | +194 ms (+16.5%) | 19 | 22 | +3 ms (+15.8%) |
| `e2e_p4_sparse` | 57 | 75 | +18 ms (+31.6%) | 36 | 43 | +7 ms (+19.4%) | 21 | 32 | +11 ms (+52.4%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 150 | 183 | +33 ms (+22.0%) | 150 | 183 | +33 ms (+22.0%) | 0 | 0 | +0 ms |

## CLI JSONL Emit (total/op_emit time)
| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |
| --- | --- | --- | --- | --- | --- | --- |
| `cli_perf_jsonl_emit` | 32 | 36 | +4 ms (+12.5%) | 32 | 36 | +4 ms (+12.5%) |
