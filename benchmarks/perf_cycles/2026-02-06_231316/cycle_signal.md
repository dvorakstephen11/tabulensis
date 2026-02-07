# Perf Cycle Signal Report

Generated: `2026-02-06T23:25:04.169698+00:00`
Cycle: `2026-02-06_231316`
Commits: pre `a481f01a6a06` -> post `2e418c6b27b3`
Aggregation: `median` over `3` run(s)

Confidence model:
- Effect score = `abs(median_delta) / max(pre_iqr, post_iqr, 1)`
- `high` >= 8, `medium` >= 3, `low` < 3
- Use confidence to separate likely signal from runtime noise

## High-Confidence Summary (`total_time_ms`)

- High-confidence improvements: **1**
- High-confidence regressions: **0**

Top high-confidence improvements:
- `e2e/e2e_p6_sharedstrings_changed_numeric_only`: 879 -> 152 (-727, -82.7%, effect=14.40)

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 8 | 9 | +1 | +12.5% | 0.0 | 0.5 | 1.00 | low |
| `e2e_p2_noise` | 8 | 9 | +1 | +12.5% | 1.0 | 0.5 | 1.00 | low |
| `e2e_p3_repetitive` | 19 | 18 | -1 | -5.3% | 2.5 | 1.0 | 0.40 | low |
| `e2e_p4_sparse` | 24 | 24 | +0 | +0.0% | 4.5 | 1.5 | 0.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 3 | 0 | -3 | -100.0% | 0.5 | 0.0 | 3.00 | medium |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2061 | 2101 | +40 | +1.9% | 20.5 | 3.5 | 1.95 | low |
| `e2e_p2_noise` | 873 | 878 | +5 | +0.6% | 9.5 | 6.0 | 0.53 | low |
| `e2e_p3_repetitive` | 2057 | 2064 | +7 | +0.3% | 46.0 | 34.5 | 0.15 | low |
| `e2e_p4_sparse` | 60 | 56 | -4 | -6.7% | 6.0 | 5.0 | 0.67 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 876 | 152 | -724 | -82.6% | 50.0 | 8.5 | 14.48 | high |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2069 | 2110 | +41 | +2.0% | 20.5 | 3.0 | 2.00 | low |
| `e2e_p2_noise` | 882 | 886 | +4 | +0.5% | 10.0 | 6.0 | 0.40 | low |
| `e2e_p3_repetitive` | 2074 | 2081 | +7 | +0.3% | 47.5 | 35.0 | 0.15 | low |
| `e2e_p4_sparse` | 83 | 80 | -3 | -3.6% | 10.0 | 6.5 | 0.30 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 879 | 152 | -727 | -82.7% | 50.5 | 8.5 | 14.40 | high |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 28 | 30 | +2 | +7.1% | 0.5 | 4.5 | 0.44 | low |
| `perf_50k_adversarial_repetitive` | 20 | 21 | +1 | +5.0% | 1.5 | 2.0 | 0.50 | low |
| `perf_50k_alignment_block_move` | 251 | 255 | +4 | +1.6% | 9.0 | 4.5 | 0.44 | low |
| `perf_50k_completely_different` | 215 | 211 | -4 | -1.9% | 12.0 | 7.0 | 0.33 | low |
| `perf_50k_dense_single_edit` | 39 | 40 | +1 | +2.6% | 2.0 | 1.0 | 0.50 | low |
| `perf_50k_identical` | 13 | 14 | +1 | +7.7% | 0.5 | 2.0 | 0.50 | low |
