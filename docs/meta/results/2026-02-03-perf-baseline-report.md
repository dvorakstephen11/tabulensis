**Perf Baseline Report (2026-02-03)**

**Context**
- Repo: `excel_diff`
- Branch: `20260203_perf_improvement`
- Commit: `60b1eaaa099d3d3497b6d27bbfdcb5713f7e3326`

**Runs (Initial)**
- `python3 scripts/check_perf_thresholds.py --suite full-scale --parallel --require-baseline --baseline benchmarks/baselines/full-scale.json --export-json benchmarks/latest_fullscale.json --export-csv benchmarks/latest_fullscale.csv`
- `python3 scripts/export_e2e_metrics.py --skip-fixtures --baseline benchmarks/baselines/e2e.json --export-csv benchmarks/latest_e2e.csv`

**Baseline Failures (Initial)**
- Full-scale: `perf_50k_identical` total_time_ms `187 > 156` (+19.9%, cap +15%).
- E2E: `e2e_p4_sparse` parse_time_ms `135 > 106` (+27.4%, cap +20%).

**Full-Scale Summary (total_time_ms)**
| Test | Baseline | Current | Delta | Status |
| --- | --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 77 | 34 | -43 ms (-55.8%) | improved |
| `perf_50k_adversarial_repetitive` | 188 | 151 | -37 ms (-19.7%) | improved |
| `perf_50k_alignment_block_move` | 5733 | 581 | -5152 ms (-89.9%) | improved |
| `perf_50k_completely_different` | 453 | 239 | -214 ms (-47.2%) | improved |
| `perf_50k_dense_single_edit` | 371 | 290 | -81 ms (-21.8%) | improved |
| `perf_50k_identical` | 156 | 187 | +31 ms (+19.9%) | baseline regression |

**E2E Summary (total_time_ms, parse_time_ms)**
| Test | Total (Baseline -> Current) | Parse (Baseline -> Current) |
| --- | --- | --- |
| `e2e_p1_dense` | 3669 -> 3385 (-7.7%) | 3618 -> 3375 (-6.7%) |
| `e2e_p2_noise` | 1537 -> 1674 (+8.9%) | 1431 -> 1597 (+11.6%) |
| `e2e_p3_repetitive` | 4136 -> 4599 (+11.2%) | 4040 -> 4578 (+13.3%) |
| `e2e_p4_sparse` | 176 -> 162 (-8.0%) | 106 -> 135 (+27.4%) |
| `e2e_p5_identical` | 22898 -> 19570 (-14.5%) | 22863 -> 19555 (-14.5%) |

**Notes**
- E2E run used `--skip-fixtures` due to permission errors installing `generate-fixtures`.
- Artifacts updated: `benchmarks/latest_fullscale.json`, `benchmarks/latest_fullscale.csv`, `benchmarks/latest_e2e.json`, `benchmarks/latest_e2e.csv`.

**Rerun Confirmation (2026-02-03)**
- Full-scale rerun passed baseline checks.
- E2E rerun passed baseline checks.

**Rerun Metrics For Previously Failing Tests**
- `perf_50k_identical` total_time_ms: 160 (baseline 156, within +15% cap).
- `e2e_p4_sparse` parse_time_ms: 115 (baseline 106, within +20% cap).

**Conclusion**
- The two earlier baseline regressions did not reproduce on rerun, so they appear to be run-to-run variance rather than stable regressions.

**Delta Summary vs Baseline (After Addressing Regressions Changes)**

**Full-scale (total_time_ms)**
| Test | Baseline | Current | Delta |
| --- | --- | --- | --- |
| `perf_50k_99_percent_blank` | 77 | 31 | -46 ms (-59.7%) |
| `perf_50k_adversarial_repetitive` | 188 | 22 | -166 ms (-88.3%) |
| `perf_50k_alignment_block_move` | 5733 | 502 | -5231 ms (-91.2%) |
| `perf_50k_completely_different` | 453 | 231 | -222 ms (-49.0%) |
| `perf_50k_dense_single_edit` | 371 | 49 | -322 ms (-86.8%) |
| `perf_50k_identical` | 156 | 18 | -138 ms (-88.5%) |

**E2E (total/parse/diff time)**
| Test | Total (Baseline -> Current) | Parse (Baseline -> Current) | Diff (Baseline -> Current) |
| --- | --- | --- | --- |
| `e2e_p1_dense` | 3669 -> 2839 (-22.6%) | 3618 -> 2830 (-21.8%) | 51 -> 9 (-82.4%) |
| `e2e_p2_noise` | 1537 -> 1364 (-11.3%) | 1431 -> 1355 (-5.3%) | 106 -> 9 (-91.5%) |
| `e2e_p3_repetitive` | 4136 -> 3875 (-6.3%) | 4040 -> 3855 (-4.6%) | 96 -> 20 (-79.2%) |
| `e2e_p4_sparse` | 176 -> 130 (-26.1%) | 106 -> 105 (-0.9%) | 70 -> 25 (-64.3%) |
| `e2e_p5_identical` | 22898 -> 16116 (-29.6%) | 22863 -> 16102 (-29.6%) | 35 -> 14 (-60.0%) |
