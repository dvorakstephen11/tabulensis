# Perf Cycle Signal Report

Generated: `2026-02-05T22:42:08.030913+00:00`
Cycle: `2026-02-05_223828`
Commits: pre `a3a19be67121` -> post `a3a19be67121`
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
| `e2e_p1_dense` | 9 | 11 | +2 | +22.2% | 0.5 | 1.5 | 1.33 | low |
| `e2e_p2_noise` | 9 | 10 | +1 | +11.1% | 0.5 | 1.0 | 1.00 | low |
| `e2e_p3_repetitive` | 22 | 19 | -3 | -13.6% | 1.0 | 0.5 | 3.00 | medium |
| `e2e_p4_sparse` | 28 | 26 | -2 | -7.1% | 2.0 | 0.5 | 1.00 | low |
| `e2e_p5_identical` | 18 | 12 | -6 | -33.3% | 0.5 | 0.5 | 6.00 | medium |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2600 | 2290 | -310 | -11.9% | 155.0 | 79.5 | 2.00 | low |
| `e2e_p2_noise` | 964 | 915 | -49 | -5.1% | 148.0 | 2.5 | 0.33 | low |
| `e2e_p3_repetitive` | 2361 | 2189 | -172 | -7.3% | 170.5 | 12.5 | 1.01 | low |
| `e2e_p4_sparse` | 66 | 63 | -3 | -4.5% | 2.0 | 7.0 | 0.43 | low |
| `e2e_p5_identical` | 14737 | 13498 | -1239 | -8.4% | 1131.5 | 298.5 | 1.10 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2610 | 2301 | -309 | -11.8% | 155.0 | 81.0 | 1.99 | low |
| `e2e_p2_noise` | 974 | 925 | -49 | -5.0% | 148.0 | 3.5 | 0.33 | low |
| `e2e_p3_repetitive` | 2383 | 2208 | -175 | -7.3% | 171.5 | 13.0 | 1.02 | low |
| `e2e_p4_sparse` | 94 | 90 | -4 | -4.3% | 4.0 | 7.0 | 0.57 | low |
| `e2e_p5_identical` | 14755 | 13510 | -1245 | -8.4% | 1132.0 | 299.0 | 1.10 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 36 | 32 | -4 | -11.1% | 1.0 | 2.0 | 2.00 | low |
| `perf_50k_adversarial_repetitive` | 26 | 20 | -6 | -23.1% | 6.5 | 0.5 | 0.92 | low |
| `perf_50k_alignment_block_move` | 331 | 277 | -54 | -16.3% | 7.0 | 65.0 | 0.83 | low |
| `perf_50k_completely_different` | 291 | 234 | -57 | -19.6% | 48.0 | 15.0 | 1.19 | low |
| `perf_50k_dense_single_edit` | 44 | 48 | +4 | +9.1% | 0.5 | 5.0 | 0.80 | low |
| `perf_50k_identical` | 16 | 15 | -1 | -6.2% | 1.0 | 0.5 | 1.00 | low |
