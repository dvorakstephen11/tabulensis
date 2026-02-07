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
