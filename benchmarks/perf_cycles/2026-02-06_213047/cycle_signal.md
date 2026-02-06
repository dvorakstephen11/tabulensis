# Perf Cycle Signal Report

Generated: `2026-02-06T21:48:35.697374+00:00`
Cycle: `2026-02-06_213047`
Commits: pre `f3fc4347ac45` -> post `f3fc4347ac45`
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
| `e2e_p1_dense` | 9 | 9 | +0 | +0.0% | 2.0 | 0.5 | 0.00 | low |
| `e2e_p2_noise` | 9 | 8 | -1 | -11.1% | 0.5 | 0.5 | 1.00 | low |
| `e2e_p3_repetitive` | 22 | 19 | -3 | -13.6% | 3.0 | 1.5 | 1.00 | low |
| `e2e_p4_sparse` | 26 | 25 | -1 | -3.8% | 1.0 | 0.5 | 1.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2111 | 2283 | +172 | +8.1% | 43.5 | 69.5 | 2.47 | low |
| `e2e_p2_noise` | 878 | 909 | +31 | +3.5% | 15.0 | 10.0 | 2.07 | low |
| `e2e_p3_repetitive` | 2212 | 2134 | -78 | -3.5% | 106.5 | 9.0 | 0.73 | low |
| `e2e_p4_sparse` | 56 | 59 | +3 | +5.4% | 5.0 | 2.5 | 0.60 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2119 | 2292 | +173 | +8.2% | 45.0 | 69.0 | 2.51 | low |
| `e2e_p2_noise` | 886 | 917 | +31 | +3.5% | 15.0 | 10.5 | 2.07 | low |
| `e2e_p3_repetitive` | 2234 | 2153 | -81 | -3.6% | 109.5 | 10.5 | 0.74 | low |
| `e2e_p4_sparse` | 83 | 83 | +0 | +0.0% | 5.5 | 2.5 | 0.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 27 | 28 | +1 | +3.7% | 1.0 | 1.0 | 1.00 | low |
| `perf_50k_adversarial_repetitive` | 20 | 21 | +1 | +5.0% | 2.5 | 2.0 | 0.40 | low |
| `perf_50k_alignment_block_move` | 237 | 247 | +10 | +4.2% | 8.5 | 6.0 | 1.18 | low |
| `perf_50k_completely_different` | 205 | 217 | +12 | +5.9% | 7.0 | 4.0 | 1.71 | low |
| `perf_50k_dense_single_edit` | 39 | 40 | +1 | +2.6% | 3.5 | 4.0 | 0.25 | low |
| `perf_50k_identical` | 13 | 13 | +0 | +0.0% | 1.5 | 0.5 | 0.00 | low |
