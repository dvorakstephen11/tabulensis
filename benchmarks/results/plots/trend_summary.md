# Benchmark Trend Summary

Generated: 2025-12-13 11:43:32

## Overview

- Total benchmark runs: 9
- Quick-scale runs: 7
- Full-scale runs: 2
- Unique tests: 10
- Date range: 2025-12-12 to 2025-12-13

## Quick Tests Performance

- First run total: 6,309ms (2025-12-12_163759.json)
- Latest run total: 430ms (2025-12-13_000410.json)
- Overall change: -93.2% (faster)

### Per-Test Trends

| Test | First (ms) | Latest (ms) | Change |
|:-----|----------:|------------:|-------:|
| perf_p4_99_percent_blank | 24 | 20 | -16.7% |
| perf_p1_large_dense | 359 | 51 | -85.8% |
| perf_p2_large_noise | 372 | 124 | -66.7% |
| perf_p3_adversarial_repetitive | 1,236 | 195 | -84.2% |
| perf_p5_identical | 4,318 | 40 | -99.1% |


## Full-Scale Tests Performance

- First run total: 25,357ms (2025-12-13_155200_fullscale.json)
- Latest run total: 38,541ms (2025-12-13_165236_fullscale.json)
- Overall change: +52.0% (slower)

### Per-Test Trends

| Test | First (ms) | Latest (ms) | Change |
|:-----|----------:|------------:|-------:|
| perf_50k_99_percent_blank | 451 | 546 | +21.1% |
| perf_50k_identical | 2,076 | 2,870 | +38.2% |
| perf_50k_adversarial_repetitive | 7,016 | 10,846 | +54.6% |
| perf_50k_completely_different | 7,744 | 12,022 | +55.2% |
| perf_50k_dense_single_edit | 8,070 | 12,257 | +51.9% |

