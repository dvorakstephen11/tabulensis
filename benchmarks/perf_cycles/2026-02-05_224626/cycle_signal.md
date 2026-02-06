# Perf Cycle Signal Report

Generated: `2026-02-05T22:53:40.071122+00:00`
Cycle: `2026-02-05_224626`
Commits: pre `a3a19be67121` -> post `a3a19be67121`
Aggregation: `median` over `5` run(s)

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
| `e2e_p1_dense` | 9 | 9 | +0 | +0.0% | 1.0 | 1.0 | 0.00 | low |
| `e2e_p2_noise` | 9 | 9 | +0 | +0.0% | 0.0 | 0.0 | 0.00 | low |
| `e2e_p3_repetitive` | 20 | 19 | -1 | -5.0% | 2.0 | 1.0 | 0.50 | low |
| `e2e_p4_sparse` | 25 | 26 | +1 | +4.0% | 1.0 | 2.0 | 0.50 | low |
| `e2e_p5_identical` | 17 | 15 | -2 | -11.8% | 2.0 | 4.0 | 0.50 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2260 | 2245 | -15 | -0.7% | 47.0 | 178.0 | 0.08 | low |
| `e2e_p2_noise` | 902 | 915 | +13 | +1.4% | 31.0 | 31.0 | 0.42 | low |
| `e2e_p3_repetitive` | 2132 | 2302 | +170 | +8.0% | 57.0 | 130.0 | 1.31 | low |
| `e2e_p4_sparse` | 64 | 64 | +0 | +0.0% | 5.0 | 6.0 | 0.00 | low |
| `e2e_p5_identical` | 13095 | 13517 | +422 | +3.2% | 531.0 | 410.0 | 0.79 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2268 | 2253 | -15 | -0.7% | 48.0 | 178.0 | 0.08 | low |
| `e2e_p2_noise` | 911 | 924 | +13 | +1.4% | 30.0 | 30.0 | 0.43 | low |
| `e2e_p3_repetitive` | 2152 | 2322 | +170 | +7.9% | 58.0 | 129.0 | 1.32 | low |
| `e2e_p4_sparse` | 89 | 88 | -1 | -1.1% | 6.0 | 8.0 | 0.12 | low |
| `e2e_p5_identical` | 13113 | 13535 | +422 | +3.2% | 533.0 | 413.0 | 0.79 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 31 | 30 | -1 | -3.2% | 1.0 | 6.0 | 0.17 | low |
| `perf_50k_adversarial_repetitive` | 20 | 22 | +2 | +10.0% | 0.0 | 2.0 | 1.00 | low |
| `perf_50k_alignment_block_move` | 260 | 264 | +4 | +1.5% | 10.0 | 32.0 | 0.12 | low |
| `perf_50k_completely_different` | 213 | 222 | +9 | +4.2% | 9.0 | 6.0 | 1.00 | low |
| `perf_50k_dense_single_edit` | 40 | 39 | -1 | -2.5% | 2.0 | 1.0 | 0.50 | low |
| `perf_50k_identical` | 14 | 12 | -2 | -14.3% | 1.0 | 1.0 | 2.00 | low |
