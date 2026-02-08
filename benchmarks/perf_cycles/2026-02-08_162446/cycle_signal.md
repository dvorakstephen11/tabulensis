# Perf Cycle Signal Report

Generated: `2026-02-08T16:49:11.235100+00:00`
Cycle: `2026-02-08_162446`
Commits: pre `b830c72cefe1` -> post `b830c72cefe1`
Aggregation: `median` over `3` run(s)

Confidence model:
- Effect score = `abs(median_delta) / max(pre_iqr, post_iqr, 1)`
- `high` >= 8, `medium` >= 3, `low` < 3
- Use confidence to separate likely signal from runtime noise

## High-Confidence Summary (`total_time_ms`)

- High-confidence improvements: **1**
- High-confidence regressions: **0**

Top high-confidence improvements:
- `e2e/e2e_p2_noise`: 717 -> 606 (-111, -15.5%, effect=8.22)

## cli-jsonl `op_emit_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 41 | 33 | -8 | -19.5% | 21.0 | 0.5 | 0.38 | low |

## cli-jsonl `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 41 | 33 | -8 | -19.5% | 21.0 | 0.5 | 0.38 | low |

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 10 | 8 | -2 | -20.0% | 1.0 | 0.5 | 2.00 | low |
| `e2e_p2_noise` | 10 | 8 | -2 | -20.0% | 0.5 | 0.5 | 2.00 | low |
| `e2e_p3_repetitive` | 26 | 20 | -6 | -23.1% | 2.5 | 3.5 | 1.71 | low |
| `e2e_p4_sparse` | 29 | 23 | -6 | -20.7% | 1.5 | 5.5 | 1.09 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.5 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2341 | 1713 | -628 | -26.8% | 153.5 | 20.0 | 4.09 | medium |
| `e2e_p2_noise` | 706 | 598 | -108 | -15.3% | 12.5 | 13.0 | 8.31 | high |
| `e2e_p3_repetitive` | 1536 | 1234 | -302 | -19.7% | 52.0 | 63.0 | 4.79 | medium |
| `e2e_p4_sparse` | 48 | 38 | -10 | -20.8% | 0.5 | 2.0 | 5.00 | medium |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 189 | 154 | -35 | -18.5% | 6.0 | 4.0 | 5.83 | medium |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2351 | 1720 | -631 | -26.8% | 154.5 | 20.0 | 4.08 | medium |
| `e2e_p2_noise` | 717 | 606 | -111 | -15.5% | 12.5 | 13.5 | 8.22 | high |
| `e2e_p3_repetitive` | 1560 | 1254 | -306 | -19.6% | 50.5 | 59.5 | 5.14 | medium |
| `e2e_p4_sparse` | 78 | 65 | -13 | -16.7% | 1.5 | 5.5 | 2.36 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 189 | 154 | -35 | -18.5% | 5.5 | 4.0 | 6.36 | medium |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 41 | 32 | -9 | -22.0% | 5.0 | 1.5 | 1.80 | low |
| `perf_50k_adversarial_repetitive` | 24 | 23 | -1 | -4.2% | 2.5 | 1.0 | 0.40 | low |
| `perf_50k_alignment_block_move` | 344 | 288 | -56 | -16.3% | 15.0 | 35.0 | 1.60 | low |
| `perf_50k_completely_different` | 249 | 225 | -24 | -9.6% | 49.0 | 4.0 | 0.49 | low |
| `perf_50k_dense_single_edit` | 44 | 50 | +6 | +13.6% | 6.5 | 16.5 | 0.36 | low |
| `perf_50k_identical` | 18 | 18 | +0 | +0.0% | 6.0 | 2.5 | 0.00 | low |
