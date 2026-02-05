# Perf Cycle Signal Report

Generated: `2026-02-05T22:24:07.505703+00:00`
Cycle: `2026-02-05_221815`
Commits: pre `114dc862602c` -> post `114dc862602c`
Aggregation: `median` over `3` run(s)

Confidence model:
- Effect score = `abs(median_delta) / max(pre_iqr, post_iqr, 1)`
- `high` >= 8, `medium` >= 3, `low` < 3
- Use confidence to separate likely signal from runtime noise

## High-Confidence Summary (`total_time_ms`)

- High-confidence improvements: **0**
- High-confidence regressions: **0**

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 9 | 9 | +0 | +0.0% | 0.5 | 0.5 | 0.00 | low |
| `e2e_p2_noise` | 10 | 10 | +0 | +0.0% | 0.5 | 1.0 | 0.00 | low |
| `e2e_p3_repetitive` | 21 | 22 | +1 | +4.8% | 0.5 | 1.5 | 0.67 | low |
| `e2e_p4_sparse` | 27 | 25 | -2 | -7.4% | 1.5 | 2.5 | 0.80 | low |
| `e2e_p5_identical` | 13 | 17 | +4 | +30.8% | 1.0 | 1.0 | 4.00 | medium |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2387 | 2345 | -42 | -1.8% | 167.5 | 8.5 | 0.25 | low |
| `e2e_p2_noise` | 957 | 916 | -41 | -4.3% | 61.0 | 8.5 | 0.67 | low |
| `e2e_p3_repetitive` | 2264 | 2257 | -7 | -0.3% | 85.5 | 34.5 | 0.08 | low |
| `e2e_p4_sparse` | 67 | 64 | -3 | -4.5% | 4.0 | 1.5 | 0.75 | low |
| `e2e_p5_identical` | 14289 | 13907 | -382 | -2.7% | 195.5 | 62.5 | 1.95 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2396 | 2354 | -42 | -1.8% | 168.0 | 8.0 | 0.25 | low |
| `e2e_p2_noise` | 967 | 927 | -40 | -4.1% | 61.5 | 8.0 | 0.65 | low |
| `e2e_p3_repetitive` | 2284 | 2280 | -4 | -0.2% | 85.5 | 33.5 | 0.05 | low |
| `e2e_p4_sparse` | 96 | 89 | -7 | -7.3% | 3.5 | 4.0 | 1.75 | low |
| `e2e_p5_identical` | 14302 | 13924 | -378 | -2.6% | 194.5 | 61.5 | 1.94 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 35 | 32 | -3 | -8.6% | 5.0 | 0.5 | 0.60 | low |
| `perf_50k_adversarial_repetitive` | 21 | 22 | +1 | +4.8% | 2.0 | 2.0 | 0.50 | low |
| `perf_50k_alignment_block_move` | 266 | 274 | +8 | +3.0% | 4.0 | 5.0 | 1.60 | low |
| `perf_50k_completely_different` | 219 | 223 | +4 | +1.8% | 5.0 | 6.5 | 0.62 | low |
| `perf_50k_dense_single_edit` | 45 | 41 | -4 | -8.9% | 1.5 | 2.0 | 2.00 | low |
| `perf_50k_identical` | 14 | 15 | +1 | +7.1% | 1.0 | 1.0 | 1.00 | low |
