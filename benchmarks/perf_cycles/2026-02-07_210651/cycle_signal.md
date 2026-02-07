# Perf Cycle Signal Report

Generated: `2026-02-07T21:17:43.315514+00:00`
Cycle: `2026-02-07_210651`
Commits: pre `c4d57470f97c` -> post `7444a8c52fa7`
Aggregation: `median` over `3` run(s)

Confidence model:
- Effect score = `abs(median_delta) / max(pre_iqr, post_iqr, 1)`
- `high` >= 8, `medium` >= 3, `low` < 3
- Use confidence to separate likely signal from runtime noise

## High-Confidence Summary (`total_time_ms`)

- High-confidence improvements: **0**
- High-confidence regressions: **0**

## cli-jsonl `op_emit_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 47 | 44 | -3 | -6.4% | 13.5 | 2.0 | 0.22 | low |

## cli-jsonl `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 47 | 44 | -3 | -6.4% | 13.5 | 2.0 | 0.22 | low |

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 13 | 12 | -1 | -7.7% | 3.0 | 1.0 | 0.33 | low |
| `e2e_p2_noise` | 15 | 13 | -2 | -13.3% | 4.0 | 1.0 | 0.50 | low |
| `e2e_p3_repetitive` | 26 | 33 | +7 | +26.9% | 2.5 | 5.5 | 1.27 | low |
| `e2e_p4_sparse` | 33 | 45 | +12 | +36.4% | 5.5 | 7.5 | 1.60 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2820 | 3310 | +490 | +17.4% | 119.0 | 105.5 | 4.12 | medium |
| `e2e_p2_noise` | 1092 | 1377 | +285 | +26.1% | 65.5 | 65.5 | 4.35 | medium |
| `e2e_p3_repetitive` | 2665 | 3188 | +523 | +19.6% | 101.0 | 320.5 | 1.63 | low |
| `e2e_p4_sparse` | 73 | 93 | +20 | +27.4% | 7.0 | 19.5 | 1.03 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 202 | 233 | +31 | +15.3% | 3.5 | 12.5 | 2.48 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2828 | 3321 | +493 | +17.4% | 119.5 | 106.0 | 4.13 | medium |
| `e2e_p2_noise` | 1110 | 1392 | +282 | +25.4% | 63.0 | 65.5 | 4.31 | medium |
| `e2e_p3_repetitive` | 2691 | 3221 | +530 | +19.7% | 98.5 | 326.0 | 1.63 | low |
| `e2e_p4_sparse` | 102 | 130 | +28 | +27.5% | 10.5 | 23.0 | 1.22 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 202 | 233 | +31 | +15.3% | 3.5 | 12.5 | 2.48 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 37 | 30 | -7 | -18.9% | 5.5 | 11.5 | 0.61 | low |
| `perf_50k_adversarial_repetitive` | 22 | 31 | +9 | +40.9% | 9.0 | 6.0 | 1.00 | low |
| `perf_50k_alignment_block_move` | 351 | 374 | +23 | +6.6% | 11.5 | 54.5 | 0.42 | low |
| `perf_50k_completely_different` | 252 | 269 | +17 | +6.7% | 13.5 | 19.5 | 0.87 | low |
| `perf_50k_dense_single_edit` | 48 | 51 | +3 | +6.2% | 3.0 | 3.0 | 1.00 | low |
| `perf_50k_identical` | 20 | 17 | -3 | -15.0% | 2.5 | 1.5 | 1.20 | low |
