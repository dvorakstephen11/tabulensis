# Benchmark Trend Summary

Generated: 2026-01-27 15:32:19

## Overview

- Total benchmark runs: 40
- Quick-scale runs: 12
- Full-scale runs: 28
- Unique tests: 13
- Date range: 2025-12-12 to 2026-01-27

## Quick Tests Performance

- First run total: 6,309ms (2025-12-12_163759.json)
- Latest run total: 451ms (2025-12-23_215619.json)
- Overall change: -92.9% (faster)

### Per-Test Trends

| Test | First (ms) | Latest (ms) | Change |
|:-----|----------:|------------:|-------:|
| perf_p4_99_percent_blank | 24 | 2 | -91.7% |
| perf_p1_large_dense | 359 | 12 | -96.7% |
| perf_p2_large_noise | 372 | 74 | -80.1% |
| perf_p3_adversarial_repetitive | 1,236 | 36 | -97.1% |
| perf_p5_identical | 4,318 | 14 | -99.7% |
| preflight_low_similarity | 196 | - | N/A |
| preflight_single_cell_edit | 117 | - | N/A |


## Full-Scale Tests Performance

- First run total: 25,357ms (2025-12-13_155200_fullscale.json)
- Latest run total: 3,578ms (2026-01-27_212351_fullscale.json)
- Overall change: -85.9% (faster)

### Per-Test Trends

| Test | First (ms) | Latest (ms) | Change |
|:-----|----------:|------------:|-------:|
| perf_50k_completely_different | 7,744 | 1,189 | -84.6% |
| perf_50k_adversarial_repetitive | 7,016 | 301 | -95.7% |
| perf_50k_dense_single_edit | 8,070 | 807 | -90.0% |
| perf_50k_99_percent_blank | 451 | 137 | -69.6% |
| perf_50k_identical | 2,076 | 215 | -89.6% |
| perf_50k_alignment_block_move | 12,230 | 929 | -92.4% |

