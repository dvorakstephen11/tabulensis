# Perf Cycle Signal Report

Generated: `2026-02-06T22:35:45.048264+00:00`
Cycle: `2026-02-06_221034`
Commits: pre `86cd17e8c016` -> post `5d6cf6101a9d`
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
| `e2e_p1_dense` | 8 | 9 | +1 | +12.5% | 2.0 | 1.5 | 0.50 | low |
| `e2e_p2_noise` | 9 | 8 | -1 | -11.1% | 0.5 | 1.0 | 1.00 | low |
| `e2e_p3_repetitive` | 21 | 19 | -2 | -9.5% | 1.0 | 1.5 | 1.33 | low |
| `e2e_p4_sparse` | 27 | 26 | -1 | -3.7% | 3.0 | 4.5 | 0.22 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2141 | 2443 | +302 | +14.1% | 116.0 | 110.5 | 2.60 | low |
| `e2e_p2_noise` | 876 | 955 | +79 | +9.0% | 17.0 | 20.5 | 3.85 | medium |
| `e2e_p3_repetitive` | 2089 | 2272 | +183 | +8.8% | 36.5 | 94.5 | 1.94 | low |
| `e2e_p4_sparse` | 62 | 60 | -2 | -3.2% | 4.0 | 6.0 | 0.33 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2153 | 2452 | +299 | +13.9% | 116.0 | 112.0 | 2.58 | low |
| `e2e_p2_noise` | 885 | 965 | +80 | +9.0% | 17.5 | 20.5 | 3.90 | medium |
| `e2e_p3_repetitive` | 2110 | 2291 | +181 | +8.6% | 37.5 | 96.0 | 1.89 | low |
| `e2e_p4_sparse` | 89 | 85 | -4 | -4.5% | 7.0 | 10.0 | 0.40 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 41 | 28 | -13 | -31.7% | 4.5 | 5.0 | 2.60 | low |
| `perf_50k_adversarial_repetitive` | 22 | 21 | -1 | -4.5% | 1.5 | 4.0 | 0.25 | low |
| `perf_50k_alignment_block_move` | 262 | 306 | +44 | +16.8% | 29.5 | 33.5 | 1.31 | low |
| `perf_50k_completely_different` | 212 | 212 | +0 | +0.0% | 54.0 | 14.0 | 0.00 | low |
| `perf_50k_dense_single_edit` | 49 | 37 | -12 | -24.5% | 21.5 | 3.5 | 0.56 | low |
| `perf_50k_identical` | 14 | 13 | -1 | -7.1% | 2.5 | 3.0 | 0.33 | low |
