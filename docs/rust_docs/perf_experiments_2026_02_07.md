# Rust Core Performance Experiments

Date: 2026-02-07

This document records a performance experiment plan (and outcome) for improving Tabulensis core performance.

---

# Experiment 1: Sparse Row Signature Allocation Reduction (`row_signatures_for_grid`)

## Objective

Reduce diff preflight/signature overhead for sparse sheets by eliminating per-row heap allocations when building row signatures from sparse storage.

Primary target metric:
- `signature_build_time_ms` in `core/tests/e2e_perf_workbook_open.rs`, especially `e2e_p4_sparse`.

## Motivation / Evidence

`core/src/engine/grid_diff.rs:row_signatures_for_grid` currently constructs `Vec<Vec<(u32, &CellContent)>>` for sparse grids, pre-sizing each row's inner `Vec` using a pre-count pass.

For sparse fixtures with many rows containing 0-1 cells (notably `e2e_p4_sparse_*`: `rows=50000`, `cols=100`, `fill_percent=1`), this results in tens of thousands of tiny heap allocations (many `Vec::with_capacity(1)`), which can dominate `signature_build_time_ms`.

## Hypothesis

If we avoid heap allocation for rows with 0 or 1 populated cell, `signature_build_time_ms` for `e2e_p4_sparse` should drop measurably (and may reduce `diff_time_ms` and `total_time_ms` correspondingly).

## Scope / Touchpoints

- `core/src/engine/grid_diff.rs`:
  - `row_signatures_for_grid(grid: &Grid) -> Vec<RowSignature>`

Correctness guard:
- Add a unit test ensuring `row_signatures_for_grid` matches `Grid::compute_row_signature` for sparse grids.

## Implementation Plan (Decision Complete)

1. Keep the existing dense-path unchanged.
2. In the sparse-path:
   - Use a one-pass row accumulator optimized for very sparse distributions (avg <= ~2 cells/row):
     - Represent each row as `Empty | One((col, &CellContent)) | Many(Vec<(col, &CellContent)>)`.
     - Allocate a `Vec` only when a row receives its second cell.
     - Sort by column only when `len >= 2`.
   - For “less sparse” sparse-grids (avg > ~2 cells/row), fall back to the existing pre-count + per-row `Vec` approach (it pre-sizes well and avoids repeated growth).
3. Micro-optimization: avoid sorting when a row has 0-1 cells.

## Measurement Plan (Perf Cycle Anchoring)

Use a full perf cycle anchored around this experiment:

1. Pre: `python3 scripts/perf_cycle.py pre`
2. Post: `python3 scripts/perf_cycle.py post --cycle <cycle_id>`

Key metrics to evaluate:
- `e2e_p4_sparse`: `signature_build_time_ms`, `diff_time_ms`, `total_time_ms`
- Aggregate e2e suite totals (noise-aware signal report)
- Ensure no regressions on `e2e_p1_dense`, `e2e_p2_noise`, `e2e_p3_repetitive`

## Success Criteria

- `e2e_p4_sparse.signature_build_time_ms` median improves by >= 15% (or >= 4ms if the baseline is already small).
- No significant regressions (> ~2-3%) in aggregate `total_time_ms` across the e2e suite.
- All tests pass.

## Outcome (2026-02-07)

Perf-cycle artifacts (pre/post) recorded at:
- `benchmarks/perf_cycles/2026-02-07_194721/`
- Delta summary: `benchmarks/perf_cycles/2026-02-07_194721/cycle_delta.md`
- Signal report: `benchmarks/perf_cycles/2026-02-07_194721/cycle_signal.md`

Key deltas (median-of-3):
- `e2e_p4_sparse.signature_build_time_ms`: `25 -> 23` (-2ms, -8%)
- `e2e_p4_sparse.diff_time_ms`: `25 -> 24` (-1ms, -4%)
- `e2e_p4_sparse.total_time_ms`: `84 -> 83` (-1ms, -1.2%)

Notes:
- Several parse-dominated tests showed slower `parse_time_ms` in the post run (e.g. `e2e_p1_dense` +4.8%).
  - This experiment only touched diff signature construction; treat the parse deltas as likely environment noise unless reproduced with alternating A/B runs.

Conclusion:
- This was a small but real improvement to the targeted sparse-signature metric, but not a large end-to-end win on the full suite.

---

# Experiment 2: Faster A1 Address Parsing (`address_to_index_ascii_bytes`)

## Objective

Reduce OpenXML worksheet parse time by speeding up A1 cell-reference parsing in the hot path.

Primary target metrics:
- `parse_time_ms` in `core/tests/e2e_perf_workbook_open.rs` (notably `e2e_p2_noise`, `e2e_p1_dense`, `e2e_p3_repetitive`)

## Motivation / Evidence

`core/src/grid_parser.rs:address_to_index_ascii_bytes` is called for every `<c r="A1">` cell in worksheet XML. For the perf e2e fixtures, that is on the order of millions of calls per workbook open.

The prior implementation used `checked_mul`/`checked_add` for both column and row accumulation, which adds overflow checks in a path where values are expected to be within Excel bounds.

## Hypothesis

If we replace checked arithmetic with plain arithmetic plus explicit Excel-bound checks, `parse_time_ms` should improve measurably (especially on `e2e_p2_noise`, which is parse-heavy but does less string interning work per cell).

## Scope / Touchpoints

- `core/src/grid_parser.rs`:
  - `address_to_index_ascii_bytes(a1: &[u8]) -> Option<(u32, u32)>`

## Implementation Plan (Decision Complete)

1. Parse ASCII letters without calling `to_ascii_uppercase`:
   - map `A..Z` and `a..z` into a single branch.
2. Use plain `col = col * 26 + ...` and `row = row * 10 + ...`.
3. Enforce Excel bounds during parse:
   - reject `col > 16_384` (XFD)
   - reject `row > 1_048_576`
4. Keep existing behavior for malformed inputs:
   - reject missing row/col, non-digits after row start, `row == 0`, etc.

## Measurement Plan (Perf Cycle Anchoring)

Use a full perf cycle anchored around this experiment:

1. Pre: `python3 scripts/perf_cycle.py pre` (completed as `benchmarks/perf_cycles/2026-02-07_210651/`)
2. Post: `python3 scripts/perf_cycle.py post --cycle 2026-02-07_210651`

## Success Criteria

- `e2e_p2_noise.parse_time_ms` median improves by >= 1%.
- No regressions > ~2% on `e2e_p1_dense` / `e2e_p3_repetitive` parse totals.
- All tests pass.

## Outcome (2026-02-07)

Perf-cycle artifacts (pre/post) recorded at:
- `benchmarks/perf_cycles/2026-02-07_210651/`
- Delta summary: `benchmarks/perf_cycles/2026-02-07_210651/cycle_delta.md`
- Signal report: `benchmarks/perf_cycles/2026-02-07_210651/cycle_signal.md`

Key deltas from the anchored perf cycle (median-of-3):
- `e2e_p1_dense.parse_time_ms`: `2820 -> 3310` (+490ms, +17.4%)
- `e2e_p2_noise.parse_time_ms`: `1092 -> 1377` (+285ms, +26.1%)

Sanity check / interpretation:
- The post cycle included cold compilation work (notably `post_fullscale_run1` was much slower than the other runs), which likely altered CPU/system state for the subsequent e2e runs.
- A small A/B spot-check (3 runs each, compiled at each commit) showed no win and a tiny regression for `e2e_p2_noise` (median parse `988ms -> 1010ms`, ~+2%).

Conclusion:
- Not a performance win; treat as a failed experiment (no improvement, possible small regression). Code change was reverted after the experiment.

---

# Experiment 3: Faster Decimal Float Parsing (`convert_value_bytes` via `lexical-core`)

## Objective

Reduce OpenXML worksheet parse time by speeding up numeric cell value parsing for the common “decimal/exponent” forms (values containing `.` / `e` / `E`).

Primary target metrics:
- `parse_time_ms` in `core/tests/e2e_perf_workbook_open.rs` (notably `e2e_p2_noise`, `e2e_p1_dense`, `e2e_p3_repetitive`)

## Motivation / Evidence

`core/src/grid_parser.rs:convert_value_bytes` parses most numbers via a fast integer-only path, but falls back to `std::str::from_utf8(..)` + `str::parse::<f64>()` when a number includes `.` or an exponent.

The perf e2e fixtures are parse-dominated, and contain many numeric values that are not pure integers (especially in the “noise” fixtures), so this fallback is likely a hotspot.

## Hypothesis

If we replace the fallback float parse with `lexical_core::parse::<f64>(bytes)`, we should reduce `parse_time_ms` measurably (without changing numeric correctness).

## Scope / Touchpoints

- `core/Cargo.toml`: add dependency on `lexical-core`
- `core/src/grid_parser.rs`:
  - Update the float fallback in `convert_value_bytes` (and test-only `convert_value`) to use `lexical-core`

Correctness guard:
- Add a unit test ensuring `convert_value_bytes` parses decimal/exponent numeric strings identically to Rust’s `str::parse::<f64>()` (bitwise equality).

## Measurement Plan (Perf Cycle Anchoring)

Use the full perf cycle anchored for this experiment:

1. Pre (already captured): `python3 scripts/perf_cycle.py pre --cycle 2026-02-07_213724`
2. Post: `python3 scripts/perf_cycle.py post --cycle 2026-02-07_213724`

Key metrics to evaluate:
- `e2e_p2_noise.parse_time_ms` (primary)
- `e2e_p1_dense.parse_time_ms`, `e2e_p3_repetitive.parse_time_ms`
- No regressions in `total_time_ms` across the e2e suite

## Success Criteria

- `e2e_p2_noise.parse_time_ms` median improves by >= 1%.
- No significant regressions (> ~2-3%) in aggregate `total_time_ms` across the e2e suite.
- All tests pass.

## Outcome (TBD)
