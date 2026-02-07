# Perf Cycle Signal Report

Generated: `2026-02-07T21:55:47.384837+00:00`
Cycle: `2026-02-07_213724`
Commits: pre `66ae09f5f6e2` -> post `186561630fcb`
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
| `cli_perf_jsonl_emit` | 43 | 39 | -4 | -9.3% | 23.0 | 3.5 | 0.17 | low |

## cli-jsonl `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 43 | 39 | -4 | -9.3% | 23.0 | 3.5 | 0.17 | low |

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 11 | 12 | +1 | +9.1% | 0.5 | 1.5 | 0.67 | low |
| `e2e_p2_noise` | 13 | 9 | -4 | -30.8% | 7.0 | 0.5 | 0.57 | low |
| `e2e_p3_repetitive` | 27 | 21 | -6 | -22.2% | 1.5 | 1.0 | 4.00 | medium |
| `e2e_p4_sparse` | 31 | 27 | -4 | -12.9% | 1.5 | 1.5 | 2.67 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2685 | 2680 | -5 | -0.2% | 113.0 | 327.0 | 0.02 | low |
| `e2e_p2_noise` | 1119 | 953 | -166 | -14.8% | 50.5 | 47.0 | 3.29 | medium |
| `e2e_p3_repetitive` | 2703 | 2393 | -310 | -11.5% | 250.0 | 68.5 | 1.24 | low |
| `e2e_p4_sparse` | 79 | 66 | -13 | -16.5% | 4.0 | 4.0 | 3.25 | medium |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 197 | 172 | -25 | -12.7% | 8.5 | 3.5 | 2.94 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2696 | 2693 | -3 | -0.1% | 112.5 | 326.0 | 0.01 | low |
| `e2e_p2_noise` | 1145 | 962 | -183 | -16.0% | 51.0 | 46.5 | 3.59 | medium |
| `e2e_p3_repetitive` | 2728 | 2414 | -314 | -11.5% | 250.5 | 67.5 | 1.25 | low |
| `e2e_p4_sparse` | 110 | 91 | -19 | -17.3% | 5.5 | 4.5 | 3.45 | medium |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 197 | 172 | -25 | -12.7% | 8.5 | 3.5 | 2.94 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 43 | 38 | -5 | -11.6% | 6.5 | 6.0 | 0.77 | low |
| `perf_50k_adversarial_repetitive` | 27 | 25 | -2 | -7.4% | 2.5 | 0.5 | 0.80 | low |
| `perf_50k_alignment_block_move` | 379 | 302 | -77 | -20.3% | 65.5 | 33.0 | 1.18 | low |
| `perf_50k_completely_different` | 260 | 241 | -19 | -7.3% | 13.5 | 2.0 | 1.41 | low |
| `perf_50k_dense_single_edit` | 49 | 51 | +2 | +4.1% | 1.5 | 3.0 | 0.67 | low |
| `perf_50k_identical` | 21 | 18 | -3 | -14.3% | 1.5 | 3.0 | 1.00 | low |
