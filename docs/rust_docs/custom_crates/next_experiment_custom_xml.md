# Next Custom Crate Experiment: `custom-xml` (Quick XML Replacement Slice 1)

## Recommendation

Run `custom-xml` as the next experiment, starting with a narrow first slice:
- replace only the `sharedStrings.xml` parse path behind a `custom-xml` feature flag,
- keep `quick-xml` as baseline default for all other XML parsing paths.

Status update (February 6, 2026): this slice is now active and implemented behind a feature flag; initial A/B measurements are recorded below.

## Why This Candidate Is Next

- Parse dominates workbook-open e2e runtime in recent perf data.
  - Example: `benchmarks/perf_cycles/2026-02-05_224626/pre_e2e.json` parse share is ~99.57% of total.
- Candidate priority in `docs/rust_docs/custom_crates/custom_crate_code_experiment.md` ranks XML parsing first.
- This path is high-impact and can be isolated safely with feature gating and parity tests.

## Scope (Slice 1)

Target function:
- `core/src/grid_parser.rs` shared strings parsing path (`parse_shared_strings`).

Non-goals for slice 1:
- No changes to sheet-cell parser event loops.
- No changes to workbook relationships or chart parsing.

## Experiment Matrix

- A: Baseline (`quick-xml` path, default features).
- B: Custom (`custom-xml` feature enabled for shared strings).
- C: Parity (test-only dual-path comparison, optional but recommended).

## Iteration 1 Log (February 6, 2026)

### Implementation completed

- Feature flags wired:
  - `core/Cargo.toml`: `custom-xml`
  - `desktop/backend/Cargo.toml`: `custom-xml = ["excel_diff/custom-xml"]`
  - `desktop/wx/Cargo.toml`: `backend-custom-xml = ["desktop_backend/custom-xml"]`
- `core/src/grid_parser.rs`:
  - `parse_shared_strings` now dispatches by feature flag.
  - Added `parse_shared_strings_custom` for manual shared-string XML scanning.
  - Added parity tests to compare custom parser behavior against quick-xml behavior.

### Correctness and compile checks run

- `cargo test -p excel_diff parse_shared_strings_rich_text_flattens_runs --lib`
- `cargo test -p excel_diff --features custom-xml parse_shared_strings_ --lib`
- `cargo check -p desktop_backend --features custom-xml --target-dir /tmp/excel_diff_target_backend`
- `cargo check -p desktop_wx --features backend-custom-xml --target-dir /tmp/excel_diff_target_wx`

### A/B perf sample (median-of-3, same target/settings)

Command A (baseline):
- `cargo test -p excel_diff --release --features perf-metrics --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1`

Command B (`custom-xml`):
- `cargo test -p excel_diff --release --features "perf-metrics custom-xml" --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1`

Median results:

| Test | Baseline Total (ms) | Custom Total (ms) | Delta | Baseline Parse (ms) | Custom Parse (ms) | Delta |
| --- | ---:| ---:| ---:| ---:| ---:| ---:|
| `e2e_p1_dense` | 2534 | 2416 | -4.66% | 2524 | 2406 | -4.68% |
| `e2e_p2_noise` | 942 | 978 | +3.82% | 932 | 970 | +4.08% |
| `e2e_p3_repetitive` | 2346 | 2372 | +1.11% | 2324 | 2352 | +1.20% |
| `e2e_p4_sparse` | 88 | 91 | +3.41% | 63 | 65 | +3.17% |
| `e2e_p5_identical` | 15016 | 14238 | -5.18% | 14999 | 14225 | -5.16% |

Aggregate (sum of all 5 tests per run, then median):
- Baseline total median: `20928 ms`
- Custom total median: `20078 ms`
- Delta: `-850 ms` (`-4.06%`)
- Baseline parse median: `20839 ms`
- Custom parse median: `19996 ms`
- Delta: `-843 ms` (`-4.05%`)

Observed caveat:
- One custom run was a large outlier (`23436 ms` aggregate total), so 5-run confirmation is required before promotion.

### 5-run confirmation (same command matrix, February 6, 2026)

Median-of-5 results (all improvements):

| Test | Baseline Total (ms) | Custom Total (ms) | Delta | Baseline Parse (ms) | Custom Parse (ms) | Delta |
| --- | ---:| ---:| ---:| ---:| ---:| ---:|
| `e2e_p1_dense` | 2613 | 2378 | -8.99% | 2603 | 2370 | -8.95% |
| `e2e_p2_noise` | 1061 | 982 | -7.45% | 1051 | 972 | -7.52% |
| `e2e_p3_repetitive` | 2663 | 2376 | -10.78% | 2639 | 2356 | -10.72% |
| `e2e_p4_sparse` | 101 | 85 | -15.84% | 71 | 61 | -14.08% |
| `e2e_p5_identical` | 15907 | 14768 | -7.16% | 15890 | 14754 | -7.15% |

Aggregate (sum of all 5 tests per run, then median):
- Baseline total median: `22505 ms`
- Custom total median: `20811 ms`
- Delta: `-1694 ms` (`-7.53%`)
- Baseline parse median: `22409 ms`
- Custom parse median: `20734 ms`
- Delta: `-1675 ms` (`-7.48%`)

Conclusion:
- Slice 1 is a clear win on this machine under controlled conditions.
- Proceed to Slice 2 (sheet cell/value scan loop in `core/src/grid_parser.rs`) behind the same `custom-xml` flag.

### Slice 2 (worksheet cell/value scan loop) implementation + perf sample (February 6, 2026)

Implementation notes:
- `core/src/grid_parser.rs`:
  - `parse_sheet_xml_with_drawing_rids` now dispatches by feature flag.
  - Added `parse_sheet_xml_internal_custom` (manual scanner) behind `custom-xml`.
  - Added a parity test comparing custom vs quick-xml parsing for common cell types.
- Correctness fix applied for both parsers:
  - Treat `<v></v>` as an explicit empty cached value (`CellValue::Text("")`) rather than `None`.

Median-of-3 A/B results (same test target/settings as Slice 1):

| Test | Baseline Total (ms) | Custom Total (ms) | Delta | Baseline Parse (ms) | Custom Parse (ms) | Delta |
| --- | ---:| ---:| ---:| ---:| ---:| ---:|
| `e2e_p1_dense` | 2342 | 1934 | -17.42% | 2333 | 1924 | -17.53% |
| `e2e_p2_noise` | 1001 | 684 | -31.67% | 992 | 675 | -31.96% |
| `e2e_p3_repetitive` | 2378 | 1340 | -43.65% | 2355 | 1321 | -43.91% |
| `e2e_p4_sparse` | 94 | 66 | -29.79% | 65 | 40 | -38.46% |
| `e2e_p5_identical` | 14413 | 11801 | -18.12% | 14398 | 11789 | -18.12% |

Aggregate (sum of all 5 tests per run, then median):
- Baseline total median: `20113 ms`
- Custom total median: `15918 ms`
- Delta: `-4195 ms` (`-20.86%`)
- Baseline parse median: `20012 ms`
- Custom parse median: `15833 ms`
- Delta: `-4179 ms` (`-20.88%`)

Next measurement gate:
- Run a 5-run confirmation A/B (same matrix) to validate stability, then consider promoting this experiment to default-on (or broadening Slice 3) if it remains a consistent win.

Rerun after the `<v></v>` cached-value fix landed (same command matrix, median-of-3):

| Test | Baseline Total (ms) | Custom Total (ms) | Delta | Baseline Parse (ms) | Custom Parse (ms) | Delta |
| --- | ---:| ---:| ---:| ---:| ---:| ---:|
| `e2e_p1_dense` | 2456 | 1984 | -19.22% | 2444 | 1974 | -19.23% |
| `e2e_p2_noise` | 985 | 670 | -31.98% | 976 | 660 | -32.38% |
| `e2e_p3_repetitive` | 2408 | 1363 | -43.40% | 2387 | 1337 | -43.99% |
| `e2e_p4_sparse` | 104 | 65 | -37.50% | 71 | 40 | -43.66% |
| `e2e_p5_identical` | 15057 | 12162 | -19.23% | 15036 | 12147 | -19.21% |

Aggregate (sum of all 5 tests per run, then median):
- Baseline total median: `21075 ms`
- Custom total median: `16272 ms`
- Delta: `-4803 ms` (`-22.79%`)
- Baseline parse median: `20973 ms`
- Custom parse median: `16181 ms`
- Delta: `-4792 ms` (`-22.85%`)

5-run confirmation A/B (alternating baseline/custom, February 6, 2026):

| Test | Baseline Total (ms) | Custom Total (ms) | Delta | Baseline Parse (ms) | Custom Parse (ms) | Delta |
| --- | ---:| ---:| ---:| ---:| ---:| ---:|
| `e2e_p1_dense` | 2397 | 1884 | -21.40% | 2388 | 1874 | -21.52% |
| `e2e_p2_noise` | 959 | 668 | -30.34% | 949 | 660 | -30.45% |
| `e2e_p3_repetitive` | 2360 | 1339 | -43.26% | 2339 | 1317 | -43.69% |
| `e2e_p4_sparse` | 92 | 67 | -27.17% | 64 | 41 | -35.94% |
| `e2e_p5_identical` | 14427 | 12049 | -16.48% | 14409 | 12034 | -16.48% |

Aggregate (sum of all 5 tests per run, then median):
- Baseline total median: `20235 ms`
- Custom total median: `15990 ms`
- Delta: `-4245 ms` (`-20.98%`)
- Baseline parse median: `20148 ms`
- Custom parse median: `15905 ms`
- Delta: `-4243 ms` (`-21.06%`)

Interpretation:
- Slice 2 is now confirmed with a 5-run median improvement and consistent wins across all scenarios in this suite.

## Step 1 Expansion: Broader Perf Suite Validation (February 5, 2026)

Because there is no real-world workbook corpus available for this iteration, we expand coverage using the built-in perf suites in `core/tests/`.

### `perf_large_grid_tests` (quick suite, not ignored)

This suite exercises the diff pipeline on in-memory synthetic grids. It does not meaningfully hit XML parsing, so `custom-xml` should be effectively neutral here (expect tiny differences from compilation/link differences and run-to-run noise).

Single-run A/B:
- Baseline: `cargo test -p excel_diff --release --features perf-metrics --test perf_large_grid_tests perf_ -- --nocapture --test-threads=1`
- Custom: `cargo test -p excel_diff --release --features "perf-metrics custom-xml" --test perf_large_grid_tests perf_ -- --nocapture --test-threads=1`

Results (total_time_ms, single run):

| Test | Baseline (ms) | Custom (ms) | Delta |
| --- | ---:| ---:| ---:|
| `perf_p1_large_dense` | 9 | 12 | +33.33% |
| `perf_p2_large_noise` | 86 | 87 | +1.16% |
| `perf_p3_adversarial_repetitive` | 30 | 32 | +6.67% |
| `perf_p4_99_percent_blank` | 4 | 4 | +0.00% |
| `perf_p5_identical` | 0 | 0 | n/a |
| `perf_preflight_low_similarity` | 140 | 150 | +7.14% |

Peak memory was identical across A/B in this run.

### `perf_large_grid_tests` (`perf_50k_*`, ignored long-running suite)

This suite is the “big” version of the in-memory diff pipeline tests. Like the quick suite, it should be effectively neutral to `custom-xml`.

Single-run A/B:
- Baseline: `cargo test -p excel_diff --release --features perf-metrics --test perf_large_grid_tests perf_50k_ -- --ignored --nocapture --test-threads=1`
- Custom: `cargo test -p excel_diff --release --features "perf-metrics custom-xml" --test perf_large_grid_tests perf_50k_ -- --ignored --nocapture --test-threads=1`

Results (total_time_ms, single run):

| Test | Baseline (ms) | Custom (ms) | Delta |
| --- | ---:| ---:| ---:|
| `perf_50k_99_percent_blank` | 32 | 33 | +3.12% |
| `perf_50k_adversarial_repetitive` | 21 | 22 | +4.76% |
| `perf_50k_alignment_block_move` | 512 | 505 | -1.37% |
| `perf_50k_completely_different` | 226 | 235 | +3.98% |
| `perf_50k_dense_single_edit` | 46 | 51 | +10.87% |
| `perf_50k_identical` | 18 | 16 | -11.11% |

Aggregate (sum of the 6 tests in the suite, single run):
- Baseline: `855 ms`
- Custom: `862 ms`
- Delta: `+0.82%`

Peak memory was identical across A/B in this run.

### Datamashup e2e perf tests (ignored)

These targets are XML-adjacent but do not currently use the `custom-xml` parser path, so any A/B difference should be treated as noise unless repeated.

Median-of-3 A/B (alternating A/B/A/B/A/B) results:

1) `e2e_perf_datamashup_text_extract`
- Baseline parse/total median: `32 ms` (runs: `[31, 34, 32]`)
- Custom parse/total median: `38 ms` (runs: `[38, 35, 43]`)
- Delta: `+6 ms` (`+18.75%`)
- Peak memory: identical (`69778567 bytes`)

2) `e2e_perf_datamashup_decode`
- Baseline parse/total median: `92 ms` (runs: `[85, 97, 92]`)
- Custom parse/total median: `85 ms` (runs: `[85, 85, 77]`)
- Delta: `-7 ms` (`-7.61%`)
- Peak memory: identical (`43287994 bytes`)

Interpretation:
- No evidence of a meaningful regression outside workbook-open suites.
- If we later change datamashup parsing (or if we see stable regressions), re-run with a larger sample size and/or pin CPU governor for more stable comparisons.

## Minimum Measurement Plan

1. Baseline
- `cargo test -p excel_diff`
- `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture`
- `python3 scripts/perf_cycle.py pre --skip-fixtures --runs 5`

2. Post-change
- Same commands, same fixtures, same run count:
- `python3 scripts/perf_cycle.py post --cycle <cycle_id> --skip-fixtures --runs 5`

3. Candidate-specific checks
- Add parity tests in `core/src/grid_parser.rs` tests for shared strings outputs.
- Add targeted microbench (or targeted perf test) for shared strings parsing throughput.

## Promotion Criteria for Slice 1 (Met)

- No correctness regressions in parity tests.
- Repeatable improvement in parse-heavy metrics (median, 5-run) on workbook-open suites.
- No meaningful memory regression.
- Code complexity remains bounded and feature-gated.

Slice 1 outcome:
- Met on February 6, 2026 (5-run confirmation win).

## Promotion Criteria for Slice 2

- No correctness regressions (parity test + full `cargo test -p excel_diff --features custom-xml`).
- Repeatable improvement in parse-heavy metrics (median, 5-run) on workbook-open suites.
- No meaningful memory regression.
- Code complexity remains bounded and feature-gated.

## Next Action

Run a 5-run confirmation A/B for Slice 2 using the same command matrix and only consider default-on if:
- aggregate total and parse medians remain improved,
- per-test regressions stay bounded and explainable,
- variance is acceptable across runs.

## Fast-Diff World A/B (Median-of-5, February 6, 2026)

Context:
- The e2e harness now measures the OpenXML **fast streaming diff** path (ZIP fingerprint short-circuit) via `WorkbookPackage::diff_openxml_streaming_fast_with_limits(...)`.
- This makes `e2e_p5_identical` effectively `0ms` because we can skip parsing unchanged worksheets entirely.

Commit:
- `55bf699a5da6`

Commands:

Baseline (quick-xml):
- `cargo test -p excel_diff --release --features perf-metrics --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1`

Custom (`custom-xml`):
- `cargo test -p excel_diff --release --features "perf-metrics custom-xml" --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1`

Run protocol:
- Alternating A/B pairs, 5 pairs total.
- Reported numbers are **median-of-5** per test.

Results (total_time_ms / parse_time_ms):

| Test | Baseline Total (ms) | Custom Total (ms) | Delta | Baseline Parse (ms) | Custom Parse (ms) | Delta |
| --- | ---:| ---:| ---:| ---:| ---:| ---:|
| `e2e_p1_dense` | 2296 | 1864 | -18.8% | 2287 | 1855 | -18.9% |
| `e2e_p2_noise` | 927 | 679 | -26.8% | 917 | 670 | -26.9% |
| `e2e_p3_repetitive` | 2275 | 1435 | -36.9% | 2252 | 1417 | -37.1% |
| `e2e_p4_sparse` | 93 | 67 | -28.0% | 65 | 42 | -35.4% |
| `e2e_p5_identical` | 0 | 0 | n/a | 0 | 0 | n/a |

Aggregate (sum across the 5 tests per run, then median-of-5):
- Total: `5597 -> 4009 ms` (`-28.4%`)
- Parse: `5533 -> 3951 ms` (`-28.6%`)

Peak memory:
- No median change observed in this A/B: aggregate `peak_memory_bytes` median stayed the same.

Decision:
- **Enable `custom-xml` by default for the desktop app** (large win on the non-identical cases; identical case is already dominated by the fast-diff short-circuit).
- Keep `custom-xml` non-default for the core crate until broader real-world corpus / fuzz coverage is in place.
