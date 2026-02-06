# Perf Cycle Signal Report

Generated: `2026-02-06T01:58:03.597061+00:00`
Cycle: `2026-02-06_014829`
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
| `e2e_p1_dense` | 11 | 10 | -1 | -9.1% | 0.5 | 1.0 | 1.00 | low |
| `e2e_p2_noise` | 10 | 12 | +2 | +20.0% | 0.0 | 0.5 | 2.00 | low |
| `e2e_p3_repetitive` | 24 | 22 | -2 | -8.3% | 0.5 | 1.5 | 1.33 | low |
| `e2e_p4_sparse` | 28 | 28 | +0 | +0.0% | 0.5 | 1.0 | 0.00 | low |
| `e2e_p5_identical` | 19 | 17 | -2 | -10.5% | 0.5 | 2.0 | 1.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2446 | 2420 | -26 | -1.1% | 49.0 | 5.0 | 0.53 | low |
| `e2e_p2_noise` | 943 | 966 | +23 | +2.4% | 13.5 | 23.5 | 0.98 | low |
| `e2e_p3_repetitive` | 2326 | 2393 | +67 | +2.9% | 44.5 | 56.5 | 1.19 | low |
| `e2e_p4_sparse` | 66 | 69 | +3 | +4.5% | 2.5 | 5.0 | 0.60 | low |
| `e2e_p5_identical` | 15055 | 15238 | +183 | +1.2% | 284.0 | 1536.5 | 0.12 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2456 | 2432 | -24 | -1.0% | 49.0 | 5.0 | 0.49 | low |
| `e2e_p2_noise` | 953 | 978 | +25 | +2.6% | 13.5 | 24.0 | 1.04 | low |
| `e2e_p3_repetitive` | 2350 | 2415 | +65 | +2.8% | 44.0 | 58.0 | 1.12 | low |
| `e2e_p4_sparse` | 94 | 97 | +3 | +3.2% | 3.0 | 6.0 | 0.50 | low |
| `e2e_p5_identical` | 15075 | 15255 | +180 | +1.2% | 284.0 | 1538.5 | 0.12 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 33 | 33 | +0 | +0.0% | 1.5 | 2.5 | 0.00 | low |
| `perf_50k_adversarial_repetitive` | 22 | 22 | +0 | +0.0% | 1.0 | 1.0 | 0.00 | low |
| `perf_50k_alignment_block_move` | 298 | 357 | +59 | +19.8% | 16.5 | 41.5 | 1.42 | low |
| `perf_50k_completely_different` | 238 | 243 | +5 | +2.1% | 5.5 | 10.5 | 0.48 | low |
| `perf_50k_dense_single_edit` | 49 | 45 | -4 | -8.2% | 3.0 | 1.5 | 1.33 | low |
| `perf_50k_identical` | 19 | 17 | -2 | -10.5% | 1.5 | 0.5 | 1.33 | low |
