# Perf Cycle Signal Report

Generated: `2026-02-08T15:36:26.889324+00:00`
Cycle: `2026-02-08_123202`
Commits: pre `d06024b3bee2` -> post `d06024b3bee2`
Aggregation: `median` over `3` run(s)

Confidence model:
- Effect score = `abs(median_delta) / max(pre_iqr, post_iqr, 1)`
- `high` >= 8, `medium` >= 3, `low` < 3
- Use confidence to separate likely signal from runtime noise

## High-Confidence Summary (`total_time_ms`)

- High-confidence improvements: **0**
- High-confidence regressions: **0**

## cli-jsonl `op_emit_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 32 | 36 | +4 | +12.5% | 2.5 | 1.0 | 1.60 | low |

## cli-jsonl `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 32 | 36 | +4 | +12.5% | 2.5 | 1.0 | 1.60 | low |

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 8 | 8 | +0 | +0.0% | 0.0 | 0.0 | 0.00 | low |
| `e2e_p2_noise` | 9 | 8 | -1 | -11.1% | 0.5 | 1.0 | 1.00 | low |
| `e2e_p3_repetitive` | 19 | 22 | +3 | +15.8% | 1.0 | 0.5 | 3.00 | medium |
| `e2e_p4_sparse` | 21 | 32 | +11 | +52.4% | 1.0 | 5.5 | 2.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 1662 | 1996 | +334 | +20.1% | 54.5 | 106.0 | 3.15 | medium |
| `e2e_p2_noise` | 588 | 610 | +22 | +3.7% | 8.0 | 51.5 | 0.43 | low |
| `e2e_p3_repetitive` | 1179 | 1373 | +194 | +16.5% | 5.5 | 79.5 | 2.44 | low |
| `e2e_p4_sparse` | 36 | 43 | +7 | +19.4% | 1.5 | 2.5 | 2.80 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 150 | 183 | +33 | +22.0% | 1.0 | 10.5 | 3.14 | medium |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 1670 | 2004 | +334 | +20.0% | 54.5 | 106.0 | 3.15 | medium |
| `e2e_p2_noise` | 597 | 618 | +21 | +3.5% | 8.5 | 52.5 | 0.40 | low |
| `e2e_p3_repetitive` | 1197 | 1395 | +198 | +16.5% | 6.0 | 79.0 | 2.51 | low |
| `e2e_p4_sparse` | 57 | 75 | +18 | +31.6% | 2.5 | 8.0 | 2.25 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 150 | 183 | +33 | +22.0% | 1.0 | 10.5 | 3.14 | medium |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 29 | 30 | +1 | +3.4% | 0.5 | 6.5 | 0.15 | low |
| `perf_50k_adversarial_repetitive` | 21 | 23 | +2 | +9.5% | 1.0 | 1.5 | 1.33 | low |
| `perf_50k_alignment_block_move` | 280 | 277 | -3 | -1.1% | 21.0 | 39.5 | 0.08 | low |
| `perf_50k_completely_different` | 215 | 235 | +20 | +9.3% | 8.0 | 63.0 | 0.32 | low |
| `perf_50k_dense_single_edit` | 42 | 46 | +4 | +9.5% | 1.5 | 6.0 | 0.67 | low |
| `perf_50k_identical` | 17 | 18 | +1 | +5.9% | 0.5 | 2.0 | 0.50 | low |
