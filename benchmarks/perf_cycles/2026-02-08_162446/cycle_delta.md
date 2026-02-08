# Perf Cycle Delta Summary

Cycle: `2026-02-08_162446`
Aggregation: `median` over **3 run(s)**
Pre: `b830c72cefe1` (main) at 2026-02-08T16:28:10.639609+00:00
Post: `b830c72cefe1` (main) at 2026-02-08T16:49:11.196073+00:00

## Full-scale (total_time_ms)
| Test | Pre | Post | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 41 | 32 | -9 ms (-22.0%) |
| `perf_50k_adversarial_repetitive` | 24 | 23 | -1 ms (-4.2%) |
| `perf_50k_alignment_block_move` | 344 | 288 | -56 ms (-16.3%) |
| `perf_50k_completely_different` | 249 | 225 | -24 ms (-9.6%) |
| `perf_50k_dense_single_edit` | 44 | 50 | +6 ms (+13.6%) |
| `perf_50k_identical` | 18 | 18 | +0 ms (+0.0%) |

## E2E (total/parse/diff time)
| Test | Pre Total | Post Total | Delta | Pre Parse | Post Parse | Delta | Pre Diff | Post Diff | Delta |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `e2e_p1_dense` | 2351 | 1720 | -631 ms (-26.8%) | 2341 | 1713 | -628 ms (-26.8%) | 10 | 8 | -2 ms (-20.0%) |
| `e2e_p2_noise` | 717 | 606 | -111 ms (-15.5%) | 706 | 598 | -108 ms (-15.3%) | 10 | 8 | -2 ms (-20.0%) |
| `e2e_p3_repetitive` | 1560 | 1254 | -306 ms (-19.6%) | 1536 | 1234 | -302 ms (-19.7%) | 26 | 20 | -6 ms (-23.1%) |
| `e2e_p4_sparse` | 78 | 65 | -13 ms (-16.7%) | 48 | 38 | -10 ms (-20.8%) | 29 | 23 | -6 ms (-20.7%) |
| `e2e_p5_identical` | 0 | 0 | +0 ms | 0 | 0 | +0 ms | 0 | 0 | +0 ms |
| `e2e_p6_sharedstrings_changed_numeric_only` | 189 | 154 | -35 ms (-18.5%) | 189 | 154 | -35 ms (-18.5%) | 0 | 0 | +0 ms |

## CLI JSONL Emit (total/op_emit time)
| Test | Pre Total | Post Total | Delta | Pre Op Emit | Post Op Emit | Delta |
| --- | --- | --- | --- | --- | --- | --- |
| `cli_perf_jsonl_emit` | 41 | 33 | -8 ms (-19.5%) | 41 | 33 | -8 ms (-19.5%) |
