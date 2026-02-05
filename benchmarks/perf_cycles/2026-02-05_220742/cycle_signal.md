# Perf Cycle Signal Report

Generated: `2026-02-05T22:12:53.904613+00:00`
Cycle: `2026-02-05_220742`
Commits: pre `4be308bcbaa6` -> post `4be308bcbaa6`
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
| `e2e_p1_dense` | 9 | 10 | +1 | +11.1% | 1.0 | 0.5 | 1.00 | low |
| `e2e_p2_noise` | 8 | 9 | +1 | +12.5% | 2.0 | 0.5 | 0.50 | low |
| `e2e_p3_repetitive` | 20 | 20 | +0 | +0.0% | 1.5 | 0.0 | 0.00 | low |
| `e2e_p4_sparse` | 25 | 26 | +1 | +4.0% | 2.0 | 3.0 | 0.33 | low |
| `e2e_p5_identical` | 13 | 14 | +1 | +7.7% | 0.5 | 1.0 | 1.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2299 | 2347 | +48 | +2.1% | 140.0 | 86.5 | 0.34 | low |
| `e2e_p2_noise` | 970 | 933 | -37 | -3.8% | 55.0 | 24.0 | 0.67 | low |
| `e2e_p3_repetitive` | 2336 | 2150 | -186 | -8.0% | 230.0 | 27.0 | 0.81 | low |
| `e2e_p4_sparse` | 61 | 62 | +1 | +1.6% | 4.5 | 1.0 | 0.22 | low |
| `e2e_p5_identical` | 14244 | 14150 | -94 | -0.7% | 275.5 | 500.0 | 0.19 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2308 | 2357 | +49 | +2.1% | 139.0 | 86.0 | 0.35 | low |
| `e2e_p2_noise` | 978 | 943 | -35 | -3.6% | 57.0 | 24.0 | 0.61 | low |
| `e2e_p3_repetitive` | 2355 | 2170 | -185 | -7.9% | 229.0 | 27.0 | 0.81 | low |
| `e2e_p4_sparse` | 86 | 87 | +1 | +1.2% | 6.5 | 3.5 | 0.15 | low |
| `e2e_p5_identical` | 14256 | 14166 | -90 | -0.6% | 275.5 | 500.0 | 0.18 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 33 | 33 | +0 | +0.0% | 1.5 | 1.0 | 0.00 | low |
| `perf_50k_adversarial_repetitive` | 25 | 23 | -2 | -8.0% | 1.0 | 3.0 | 0.67 | low |
| `perf_50k_alignment_block_move` | 299 | 266 | -33 | -11.0% | 23.5 | 10.0 | 1.40 | low |
| `perf_50k_completely_different` | 232 | 222 | -10 | -4.3% | 48.0 | 5.5 | 0.21 | low |
| `perf_50k_dense_single_edit` | 51 | 42 | -9 | -17.6% | 5.0 | 1.5 | 1.80 | low |
| `perf_50k_identical` | 20 | 18 | -2 | -10.0% | 0.5 | 1.5 | 1.33 | low |
