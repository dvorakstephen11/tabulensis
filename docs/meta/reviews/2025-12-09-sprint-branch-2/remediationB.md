# Remediation Plan: 2025-12-09-sprint-branch-2

## Overview

This remediation focuses on closing the remaining gaps relative to the Branch 2 mini-plan and unified spec: completing the performance infrastructure and CI perf suite, adding large-scale validation tests for limits and RLE, tightening error-handling semantics for `LimitBehavior::ReturnError`, hardening tests around new `DiffReport` flags and metrics, and creating a branch-level Activity Log documenting the intentional spec deviations.

## Fixes Required

### Fix 1: Build Out Performance Infrastructure & CI Perf Regression Suite

- **Changes**:
  - **Fixtures / manifest**  
    - Extend `fixtures/manifest.yaml` to include the remaining perf workloads:  
      - `p3_adversarial_repetitive` (50K×50)  
      - `p4_99_percent_blank` (50K×100)  
      - `p5_identical` (50K×100)   
    - Implement corresponding generators in `fixtures/src/generators/perf.py` and expose them via the CLI used by CI. :contentReference[oaicite:46]{index=46}  
  - **Perf harness in Rust**  
    - Add perf-focused tests under `core/tests/` (e.g., `perf_large_grid_tests.rs`) that:  
      - Load each perf fixture (P1–P5) using the existing `fixture_path` helper.   
      - Run `diff_workbooks_with_config` with `perf-metrics` enabled and **assert**:
        - The diff completes successfully within a conservative time budget (measured externally via the CI perf script, not via test assertions).
        - `DiffReport.metrics` is `Some`, with non-zero `rows_processed` and `cells_compared`.   
  - **Perf script and CI integration**  
    - Add `scripts/check_perf_thresholds.py` (or Rust equivalent) that:
      - Runs `cargo test --release --features perf-metrics perf_` and measures test durations.  
      - Compares timings against thresholds from the mini-spec table for P1–P5.   
    - Add `.github/workflows/perf.yml` with a `perf-regression` job that:
      - Builds with `--features perf-metrics`.  
      - Runs the perf test subset (tagged by a `perf_` prefix in test names).  
      - Invokes the perf script to enforce thresholds. :contentReference[oaicite:50]{index=50}  
- **Tests**:
  - New Rust perf tests (e.g., `perf_p1_large_dense`, `perf_p2_large_noise`, `perf_p3_adversarial_repetitive`, etc.) that:
    - Are gated behind `#[cfg(feature = "perf-metrics")]`.  
    - Assert basic correctness (no panics, no empty-op results for non-identical inputs).  
  - CI run of perf workflow must pass with all thresholds satisfied.

---

### Fix 2: Add Explicit Large-Grid and 500-Column Limit-Within Tests

- **Changes**:
  - Extend `core/tests/limit_behavior_tests.rs` with additional tests that **do not** hit limits but match the planned scale:
    - `large_grid_50k_rows_completes_within_default_limits`:
      - Build 50,000×100 grid pair using `single_sheet_workbook` or a lightweight in-memory constructor, ensuring content is mostly identical aside from a few edits.  
      - Use `DiffConfig::default()` and assert:
        - `report.complete == true`  
        - `report.warnings.is_empty()`  
        - At least one `CellEdited` operation is present to ensure non-trivial work.   
    - `wide_grid_500_cols_completes_within_default_limits`:
      - Build a smaller row-count but 500-column workbook (e.g., 100×500) with a small number of edits.  
      - Assert the same invariants.  
- **Tests**:
  - New tests in `limit_behavior_tests.rs` as above; they should be fast enough for normal CI, but still validate the code-paths under realistic scale.

---

### Fix 3: Clarify and Document Perf Metrics Spec Deviation

- **Changes**:
  - Update `excel_diff_specification.md` and/or `next_sprint_plan.md` to reflect the current `DiffMetrics` contents:
    - Remove `parse_time_ms` and `peak_memory_bytes` from the near-term contract.  
    - Add a note that memory tracking and parse timing are planned for a future branch/phase.   
  - Optionally:
    - If feasible, implement `parse_time_ms` by wrapping grid parsing calls in `Phase::Parse` timers and reintroduce the field; otherwise, keep it explicitly out of the spec for now.  
- **Tests**:
  - Extend `pg4_diff_report_json_shape` to:
    - Include `metrics` in the serialized JSON when `perf-metrics` is enabled and assert that the metrics object contains the expected fields (but not `parse_time_ms` or `peak_memory_bytes`).   

---

### Fix 4: Replace Panic in `LimitBehavior::ReturnError` With Structured Error

- **Changes**:
  - Introduce a diff-level error type (or reuse an existing one, e.g., `ExcelOpenError` variant) that can represent “limits exceeded” in spreadsheet mode.  
  - Refactor `diff_grids_with_config` to:
    - Return a `Result<(), DiffError>` (or similar) internally when `ReturnError` is selected, rather than panicking.  
    - Bubble this up through `diff_workbooks_with_config` and `diff_workbooks` to the JSON entry point.   
  - Ensure that `ReturnPartialResult` and `FallbackToPositional` remain unchanged.  
- **Tests**:
  - Update `limit_behavior_tests`:
    - Replace the `#[should_panic]` test for `ReturnError` with a test that asserts an error result is returned, with an appropriate error kind or message. :contentReference[oaicite:55]{index=55}  
  - Add an output-level test that:
    - Invokes `diff_workbooks_to_json` with `ReturnError` on an over-limit input and asserts that it returns a structured error (not a panic).

---

### Fix 5: Strengthen Multi-Gap AMR Tests via RowAlignment & Recursive Coverage

- **Changes**:
  - Add unit tests in `core/src/alignment/assembly.rs` (or a new `alignment/tests` module) that:
    - Directly call `align_rows_amr` on synthetic `Grid`s designed to produce:
      - Two or more disjoint gaps with insertions/deletions.  
      - A gap large enough to trigger `GapStrategy::RecursiveAlign` (set `recursive_align_threshold` low in `DiffConfig` to force this path).   
    - Assert properties of `RowAlignment`:
      - Monotonicity of `matched` pairs.  
      - Correct contents of `inserted`/`deleted`.  
      - Correct extraction of `RowBlockMove` for move scenarios.  
- **Tests**:
  - New tests such as:
    - `amr_recursive_gap_alignment_returns_monotonic_alignment`  
    - `amr_multi_gap_move_detection_produces_expected_row_block_move`  

---

### Fix 6: Add JSON-Level Tests for Partial Results and Metrics

- **Changes**:
  - In `core/tests/output_tests.rs`, add tests that:
    - Construct a `DiffReport::with_partial_result` (or simulate limit-exceeded + `ReturnPartialResult` using a tiny synthetic grid and config) and call `serialize_diff_report`:
      - Assert JSON includes `"complete": false`.  
      - Assert `warnings` array is present and contains the expected message.   
    - When `perf-metrics` is enabled and metrics are attached:
      - Assert the metrics object is serialized with the expected fields.   
- **Tests**:
  - Example names:
    - `serialize_partial_diff_report_includes_complete_false_and_warnings`  
    - `serialize_diff_report_with_metrics_includes_metrics_object`  

---

### Fix 7: Add RLE-at-Scale Tests for Adversarial Repetitive Patterns

- **Changes**:
  - Extend `core/src/alignment/runs.rs` tests to include:
    - `compresses_10k_identical_rows_to_single_run`:
      - Build 10,000 `RowMeta` entries with the same hash and assert a single `RowRun` of count 10,000.   
    - `alternating_pattern_ab_does_not_overcompress`:
      - Construct 10,000 rows that alternate between two different signatures and assert that the number of runs ~ 10,000 / 1 (or at least >> 2), exercising the “no compression benefit” case.  
  - Optionally add a high-level RLE alignment test in `assembly.rs` to confirm that the RLE fast path is used and produces correct alignment for a 10K-identical-rows scenario. :contentReference[oaicite:63]{index=63}  
- **Tests**:
  - Ensure these tests run quickly (they operate on lightweight `RowMeta` objects, not full grids) to keep CI times reasonable.

---

## Constraints

- The perf CI workflow should be designed to run in a reasonable time window; time budgets in the mini-spec may need a safety margin to account for CI variability.   
- Any change to error types (Fix 4) must be coordinated with public API consumers, as it may alter exposed error variants.  
- Large-grid tests (Fix 2) and RLE-at-scale tests (Fix 8) must be written carefully to avoid excessive runtime in debug builds.

## Expected Outcome

After remediation:

- Limit behavior, partial-result semantics, and AMR multi-gap alignment are functionally correct **and** thoroughly tested, including at the JSON boundary.  
- Performance guarantees are backed by a concrete perf suite and CI workflow that run against the planned fixtures (P1–P5) with `perf-metrics` enabled.  
- Error-handling semantics for `LimitBehavior::ReturnError` and metrics scope are explicitly documented and aligned with the spec and public API expectations.  
- The branch has an Activity Log capturing intentional deviations and scope decisions, closing the process loop for future reviewers and maintainers.

