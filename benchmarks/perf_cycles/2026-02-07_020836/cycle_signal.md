# Perf Cycle Signal Report

Generated: `2026-02-07T02:41:26.576555+00:00`
Cycle: `2026-02-07_020836`
Commits: pre `9e6e69bd29ac` -> post `9e6e69bd29ac`
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
| `e2e_p1_dense` | 8 | 8 | +0 | +0.0% | 1.0 | 0.5 | 0.00 | low |
| `e2e_p2_noise` | 9 | 8 | -1 | -11.1% | 2.0 | 0.5 | 0.50 | low |
| `e2e_p3_repetitive` | 19 | 19 | +0 | +0.0% | 0.5 | 1.0 | 0.00 | low |
| `e2e_p4_sparse` | 23 | 23 | +0 | +0.0% | 5.0 | 1.5 | 0.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2147 | 2115 | -32 | -1.5% | 10.5 | 2.5 | 3.05 | medium |
| `e2e_p2_noise` | 867 | 864 | -3 | -0.3% | 10.0 | 30.5 | 0.10 | low |
| `e2e_p3_repetitive` | 2110 | 2054 | -56 | -2.7% | 7.5 | 34.0 | 1.65 | low |
| `e2e_p4_sparse` | 57 | 59 | +2 | +3.5% | 2.0 | 5.0 | 0.40 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 153 | 151 | -2 | -1.3% | 6.5 | 6.0 | 0.31 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2155 | 2124 | -31 | -1.4% | 9.5 | 2.5 | 3.26 | medium |
| `e2e_p2_noise` | 879 | 872 | -7 | -0.8% | 10.5 | 30.0 | 0.23 | low |
| `e2e_p3_repetitive` | 2128 | 2074 | -54 | -2.5% | 7.5 | 33.5 | 1.61 | low |
| `e2e_p4_sparse` | 82 | 84 | +2 | +2.4% | 4.0 | 5.5 | 0.36 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 153 | 151 | -2 | -1.3% | 6.5 | 6.0 | 0.31 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 27 | 29 | +2 | +7.4% | 1.5 | 0.5 | 1.33 | low |
| `perf_50k_adversarial_repetitive` | 20 | 23 | +3 | +15.0% | 1.0 | 5.0 | 0.60 | low |
| `perf_50k_alignment_block_move` | 252 | 255 | +3 | +1.2% | 7.0 | 4.0 | 0.43 | low |
| `perf_50k_completely_different` | 224 | 214 | -10 | -4.5% | 12.0 | 3.0 | 0.83 | low |
| `perf_50k_dense_single_edit` | 40 | 45 | +5 | +12.5% | 1.0 | 4.0 | 1.25 | low |
| `perf_50k_identical` | 13 | 18 | +5 | +38.5% | 1.0 | 0.5 | 5.00 | medium |
