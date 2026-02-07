# Perf Cycle Signal Report

Generated: `2026-02-07T20:00:58.017039+00:00`
Cycle: `2026-02-07_194721`
Commits: pre `c54ad1d8b8f6` -> post `0f86267b935e`
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
| `cli_perf_jsonl_emit` | 34 | 33 | -1 | -2.9% | 2.0 | 3.0 | 0.33 | low |

## cli-jsonl `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 34 | 33 | -1 | -2.9% | 2.0 | 3.0 | 0.33 | low |

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 9 | 9 | +0 | +0.0% | 1.0 | 0.5 | 0.00 | low |
| `e2e_p2_noise` | 9 | 9 | +0 | +0.0% | 0.0 | 0.0 | 0.00 | low |
| `e2e_p3_repetitive` | 20 | 21 | +1 | +5.0% | 0.0 | 1.0 | 1.00 | low |
| `e2e_p4_sparse` | 25 | 24 | -1 | -4.0% | 1.0 | 0.5 | 1.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2149 | 2252 | +103 | +4.8% | 10.0 | 32.5 | 3.17 | medium |
| `e2e_p2_noise` | 888 | 901 | +13 | +1.5% | 6.5 | 18.5 | 0.70 | low |
| `e2e_p3_repetitive` | 2084 | 2126 | +42 | +2.0% | 13.5 | 13.5 | 3.11 | medium |
| `e2e_p4_sparse` | 60 | 60 | +0 | +0.0% | 0.5 | 1.5 | 0.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 158 | 161 | +3 | +1.9% | 7.0 | 5.0 | 0.43 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2158 | 2262 | +104 | +4.8% | 11.0 | 32.5 | 3.20 | medium |
| `e2e_p2_noise` | 897 | 910 | +13 | +1.4% | 6.5 | 18.5 | 0.70 | low |
| `e2e_p3_repetitive` | 2104 | 2147 | +43 | +2.0% | 13.5 | 14.5 | 2.97 | low |
| `e2e_p4_sparse` | 84 | 83 | -1 | -1.2% | 1.0 | 1.5 | 0.67 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 158 | 161 | +3 | +1.9% | 7.0 | 5.0 | 0.43 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 30 | 29 | -1 | -3.3% | 0.5 | 2.5 | 0.40 | low |
| `perf_50k_adversarial_repetitive` | 23 | 24 | +1 | +4.3% | 0.5 | 1.0 | 1.00 | low |
| `perf_50k_alignment_block_move` | 264 | 287 | +23 | +8.7% | 3.5 | 21.0 | 1.10 | low |
| `perf_50k_completely_different` | 222 | 230 | +8 | +3.6% | 11.5 | 54.5 | 0.15 | low |
| `perf_50k_dense_single_edit` | 44 | 44 | +0 | +0.0% | 2.0 | 8.5 | 0.00 | low |
| `perf_50k_identical` | 18 | 20 | +2 | +11.1% | 0.5 | 1.0 | 2.00 | low |
