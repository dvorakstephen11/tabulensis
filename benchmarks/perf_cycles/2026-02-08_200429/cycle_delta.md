# Perf Cycle Delta Summary

Cycle: `2026-02-08_200429`
Aggregation: `median` over **3 run(s)**
Pre: `aa32790b6ff3` (20260208_iteration_2) at 2026-02-08T20:08:53.900462+00:00
Post: `aa32790b6ff3` (20260208_iteration_2) at 2026-02-08T21:45:45.976907+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 29 | 29 | +0 ms (+0.0%) |
| `perf_50k_adversarial_repetitive` | 22 | 20 | -2 ms (-9.1%) |
| `perf_50k_alignment_block_move` | 260 | 255 | -5 ms (-1.9%) |
| `perf_50k_completely_different` | 217 | 220 | +3 ms (+1.4%) |
| `perf_50k_dense_single_edit` | 44 | 43 | -1 ms (-2.3%) |
| `perf_50k_identical` | 16 | 15 | -1 ms (-6.2%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2186 | 1812 | -374 ms (-17.1%) | 2175 | 1803 | -372 ms (-17.1%) | 11 | 9 | -2 ms (-18.2%) |
| `e2e_p2_noise` | 675 | 651 | -24 ms (-3.6%) | 666 | 642 | -24 ms (-3.6%) | 10 | 9 | -1 ms (-10.0%) |
| `e2e_p3_repetitive` | 1382 | 1324 | -58 ms (-4.2%) | 1360 | 1304 | -56 ms (-4.1%) | 20 | 20 | +0 ms (+0.0%) |
| `e2e_p4_sparse` | 72 | 63 | -9 ms (-12.5%) | 44 | 40 | -4 ms (-9.1%) | 27 | 24 | -3 ms (-11.1%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 178 | 161 | -17 ms (-9.6%) | 178 | 161 | -17 ms (-9.6%) | 0 | 0 | +0 ms |

## CLI JSONL Emit (total/op_emit time)
| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |
| --- | --- | --- | --- | --- | --- | --- |
| `cli_perf_jsonl_emit` | 51 | 33 | -18 ms (-35.3%) | 51 | 33 | -18 ms (-35.3%) |
