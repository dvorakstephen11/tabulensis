# Perf Cycle Signal Report

Generated: `2026-02-08T21:45:46.034771+00:00`
Cycle: `2026-02-08_200429`
Commits: pre `aa32790b6ff3` -> post `aa32790b6ff3`
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
| `cli_perf_jsonl_emit` | 51 | 33 | -18 | -35.3% | 11.0 | 0.0 | 1.64 | low |

## cli-jsonl `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `cli_perf_jsonl_emit` | 51 | 33 | -18 | -35.3% | 11.0 | 0.0 | 1.64 | low |

## e2e `diff_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 11 | 9 | -2 | -18.2% | 1.0 | 0.0 | 2.00 | low |
| `e2e_p2_noise` | 10 | 9 | -1 | -10.0% | 2.0 | 0.5 | 0.50 | low |
| `e2e_p3_repetitive` | 20 | 20 | +0 | +0.0% | 1.0 | 0.0 | 0.00 | low |
| `e2e_p4_sparse` | 27 | 24 | -3 | -11.1% | 1.5 | 1.0 | 2.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |

## e2e `parse_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2175 | 1803 | -372 | -17.1% | 91.5 | 45.5 | 4.07 | medium |
| `e2e_p2_noise` | 666 | 642 | -24 | -3.6% | 67.5 | 8.5 | 0.36 | low |
| `e2e_p3_repetitive` | 1360 | 1304 | -56 | -4.1% | 143.5 | 14.0 | 0.39 | low |
| `e2e_p4_sparse` | 44 | 40 | -4 | -9.1% | 4.0 | 0.5 | 1.00 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 178 | 161 | -17 | -9.6% | 29.5 | 4.5 | 0.58 | low |

## e2e `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `e2e_p1_dense` | 2186 | 1812 | -374 | -17.1% | 92.5 | 45.5 | 4.04 | medium |
| `e2e_p2_noise` | 675 | 651 | -24 | -3.6% | 66.0 | 9.0 | 0.36 | low |
| `e2e_p3_repetitive` | 1382 | 1324 | -58 | -4.2% | 143.5 | 14.0 | 0.40 | low |
| `e2e_p4_sparse` | 72 | 63 | -9 | -12.5% | 5.0 | 1.0 | 1.80 | low |
| `e2e_p5_identical` | 0 | 0 | +0 | n/a | 0.0 | 0.0 | 0.00 | low |
| `e2e_p6_sharedstrings_changed_numeric_only` | 178 | 161 | -17 | -9.6% | 29.5 | 4.5 | 0.58 | low |

## full-scale `total_time_ms`

| Test | Pre Med | Post Med | Delta | Delta % | Pre IQR | Post IQR | Effect | Confidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `perf_50k_99_percent_blank` | 29 | 29 | +0 | +0.0% | 1.0 | 2.5 | 0.00 | low |
| `perf_50k_adversarial_repetitive` | 22 | 20 | -2 | -9.1% | 1.0 | 1.0 | 2.00 | low |
| `perf_50k_alignment_block_move` | 260 | 255 | -5 | -1.9% | 3.0 | 5.5 | 0.91 | low |
| `perf_50k_completely_different` | 217 | 220 | +3 | +1.4% | 8.0 | 5.5 | 0.38 | low |
| `perf_50k_dense_single_edit` | 44 | 43 | -1 | -2.3% | 3.5 | 6.5 | 0.15 | low |
| `perf_50k_identical` | 16 | 15 | -1 | -6.2% | 1.0 | 0.5 | 1.00 | low |
