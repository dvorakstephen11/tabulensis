# Perf Cycle Signal Report

Generated: `2026-02-06T22:54:10.988437+00:00`
Cycle: `2026-02-06_224104`
Commits: pre `55dc8616ae6c` -> post `9e292f59ca04`
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
| `e2e_p1_dense` | 8 | 9 | +1 | +12.5% | 0.5 | 0.5 | 1.00 | low |
| `e2e_p2_noise` | 9 | 8 | -1 | -11.1% | 0.5 | 0.5 | 1.00 | low |
| `e2e_p3_repetitive` | 18 | 20 | +2 | +11.1% | 2.0 | 1.5 | 1.00 | low |
| `e2e_p4_sparse` | 23 | 25 | +2 | +8.7% | 0.5 | 2.0 | 1.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2052 | 2056 | +4 | +0.2% | 5.0 | 27.0 | 0.15 | low |
| `e2e_p2_noise` | 858 | 847 | -11 | -1.3% | 9.0 | 6.5 | 1.22 | low |
| `e2e_p3_repetitive` | 2034 | 2013 | -21 | -1.0% | 9.5 | 7.5 | 2.21 | low |
| `e2e_p4_sparse` | 60 | 54 | -6 | -10.0% | 2.5 | 1.5 | 2.40 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2060 | 2065 | +5 | +0.2% | 4.5 | 27.5 | 0.18 | low |
| `e2e_p2_noise` | 867 | 855 | -12 | -1.4% | 9.5 | 7.0 | 1.26 | low |
| `e2e_p3_repetitive` | 2052 | 2033 | -19 | -0.9% | 7.5 | 6.0 | 2.53 | low |
| `e2e_p4_sparse` | 84 | 81 | -3 | -3.6% | 2.5 | 2.5 | 1.20 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 30 | 27 | -3 | -10.0% | 1.5 | 2.5 | 1.20 | low |
| `perf_50k_adversarial_repetitive` | 19 | 18 | -1 | -5.3% | 3.0 | 0.5 | 0.33 | low |
| `perf_50k_alignment_block_move` | 269 | 248 | -21 | -7.8% | 36.5 | 8.5 | 0.58 | low |
| `perf_50k_completely_different` | 212 | 212 | +0 | +0.0% | 8.0 | 9.5 | 0.00 | low |
| `perf_50k_dense_single_edit` | 39 | 39 | +0 | +0.0% | 1.5 | 0.5 | 0.00 | low |
| `perf_50k_identical` | 14 | 13 | -1 | -7.1% | 1.0 | 1.5 | 0.67 | low |
