# Perf Cycle Signal Report

Generated: `2026-02-06T03:17:15.220676+00:00`
Cycle: `2026-02-06_022721`
Commits: pre `c11cdc83888c` -> post `baac1dcb1912`
Aggregation: `median` over `3` run(s)

Confidence model:
- Effect score = `abs(median_delta) / max(pre_iqr, post_iqr, 1)`
- `high` >= 8, `medium` >= 3, `low` < 3
- Use confidence to separate likely signal from runtime noise

## High-Confidence Summary (`total_time_ms`)

- High-confidence improvements: **1**
- High-confidence regressions: **0**

Top high-confidence improvements:
- `e2e/e2e_p5_identical`: 14965 -> 0 (-14965, -100.0%, effect=39.54)

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 10 | 8 | -2 | -20.0% | 2.0 | 1.0 | 1.00 | low |
| `e2e_p2_noise` | 10 | 9 | -1 | -10.0% | 0.0 | 0.5 | 1.00 | low |
| `e2e_p3_repetitive` | 21 | 22 | +1 | +4.8% | 1.5 | 1.0 | 0.67 | low |
| `e2e_p4_sparse` | 32 | 27 | -5 | -15.6% | 2.0 | 1.5 | 2.50 | low |
| `e2e_p5_identical` | 18 | 0 | -18 | -100.0% | 1.5 | 0.0 | 12.00 | high |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2664 | 2428 | -236 | -8.9% | 44.0 | 31.0 | 5.36 | medium |
| `e2e_p2_noise` | 1019 | 946 | -73 | -7.2% | 40.5 | 15.0 | 1.80 | low |
| `e2e_p3_repetitive` | 2433 | 2444 | +11 | +0.5% | 85.5 | 22.5 | 0.13 | low |
| `e2e_p4_sparse` | 71 | 63 | -8 | -11.3% | 6.5 | 6.0 | 1.23 | low |
| `e2e_p5_identical` | 14949 | 0 | -14949 | -100.0% | 379.0 | 0.0 | 39.44 | high |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2674 | 2436 | -238 | -8.9% | 46.0 | 30.0 | 5.17 | medium |
| `e2e_p2_noise` | 1029 | 954 | -75 | -7.3% | 40.5 | 15.0 | 1.85 | low |
| `e2e_p3_repetitive` | 2454 | 2465 | +11 | +0.4% | 87.0 | 23.0 | 0.13 | low |
| `e2e_p4_sparse` | 103 | 88 | -15 | -14.6% | 8.5 | 6.5 | 1.76 | low |
| `e2e_p5_identical` | 14965 | 0 | -14965 | -100.0% | 378.5 | 0.0 | 39.54 | high |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 39 | 32 | -7 | -17.9% | 4.0 | 3.5 | 1.75 | low |
| `perf_50k_adversarial_repetitive` | 23 | 25 | +2 | +8.7% | 2.5 | 2.5 | 0.80 | low |
| `perf_50k_alignment_block_move` | 300 | 294 | -6 | -2.0% | 35.0 | 13.0 | 0.17 | low |
| `perf_50k_completely_different` | 248 | 224 | -24 | -9.7% | 33.0 | 3.5 | 0.73 | low |
| `perf_50k_dense_single_edit` | 46 | 42 | -4 | -8.7% | 3.5 | 4.5 | 0.89 | low |
| `perf_50k_identical` | 17 | 14 | -3 | -17.6% | 2.0 | 1.0 | 1.50 | low |
