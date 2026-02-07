# Perf Cycle Signal Report

Generated: `2026-02-07T13:12:08.341545+00:00`
Cycle: `2026-02-07_130939`
Commits: pre `87ea95ea9fd9` -> post `2d255a0365ab`
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
| `cli_perf_jsonl_emit` | 74 | 32 | -42 | -56.8% | 7.0 | 1.5 | 6.00 | medium |

## cli-jsonl `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 74 | 32 | -42 | -56.8% | 7.0 | 1.5 | 6.00 | medium |

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 9 | 8 | -1 | -11.1% | 0.5 | 1.0 | 1.00 | low |
| `e2e_p2_noise` | 9 | 8 | -1 | -11.1% | 1.0 | 0.5 | 1.00 | low |
| `e2e_p3_repetitive` | 19 | 20 | +1 | +5.3% | 1.0 | 1.0 | 1.00 | low |
| `e2e_p4_sparse` | 23 | 25 | +2 | +8.7% | 1.0 | 0.5 | 2.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2278 | 2178 | -100 | -4.4% | 101.5 | 71.0 | 0.99 | low |
| `e2e_p2_noise` | 916 | 902 | -14 | -1.5% | 8.0 | 30.0 | 0.47 | low |
| `e2e_p3_repetitive` | 2193 | 2201 | +8 | +0.4% | 29.0 | 18.5 | 0.28 | low |
| `e2e_p4_sparse` | 58 | 58 | +0 | +0.0% | 0.5 | 5.5 | 0.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 152 | 152 | +0 | +0.0% | 3.5 | 6.5 | 0.00 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2287 | 2185 | -102 | -4.5% | 101.0 | 71.5 | 1.01 | low |
| `e2e_p2_noise` | 924 | 911 | -13 | -1.4% | 8.5 | 30.0 | 0.43 | low |
| `e2e_p3_repetitive` | 2211 | 2221 | +10 | +0.5% | 29.5 | 17.5 | 0.34 | low |
| `e2e_p4_sparse` | 81 | 83 | +2 | +2.5% | 1.5 | 5.0 | 0.40 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 152 | 152 | +0 | +0.0% | 3.5 | 6.5 | 0.00 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 30 | 31 | +1 | +3.3% | 3.5 | 1.5 | 0.29 | low |
| `perf_50k_adversarial_repetitive` | 22 | 24 | +2 | +9.1% | 1.5 | 3.0 | 0.67 | low |
| `perf_50k_alignment_block_move` | 286 | 297 | +11 | +3.8% | 32.0 | 18.0 | 0.34 | low |
| `perf_50k_completely_different` | 232 | 239 | +7 | +3.0% | 9.5 | 18.5 | 0.38 | low |
| `perf_50k_dense_single_edit` | 41 | 44 | +3 | +7.3% | 0.5 | 8.5 | 0.35 | low |
| `perf_50k_identical` | 18 | 19 | +1 | +5.6% | 0.5 | 2.0 | 0.50 | low |
