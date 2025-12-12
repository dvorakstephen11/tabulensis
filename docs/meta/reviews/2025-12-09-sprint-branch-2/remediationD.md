Short version: the developer got almost all of Branch 2 done. The AMR alignment pipeline, RLE, limit handling, metrics, perf fixtures, and CI wiring are in place and match the updated specs. What’s still missing is mostly around **perf threshold enforcement** and **documentation (the Activity Log)**, plus a couple of tiny “hardening” tests.

I’ll break it down.

---

## 1. What Branch 2 was supposed to do (mental model)

Branch 2 had two big responsibilities:

1. **Scalability / AMR algorithm**

   * Row metadata + frequency classification.
   * Anchor discovery + LIS-based anchor chain.
   * Multi-gap alignment with gap strategies (Myers / RLE / bail-out / recursive).
   * Localized move detection from gaps.
   * Optional run-length encoding (RLE) for repetitive rows.

2. **Perf & limits**

   * Replace hard-coded size limits with configurable `DiffConfig` limits + `LimitBehavior`.
   * Provide partial-result semantics (`complete` flag + `warnings`).
   * `DiffMetrics` with timing and counts behind a `perf-metrics` feature.
   * Perf fixtures (P1–P5), perf tests, and a CI perf workflow with a threshold script.

---

## 2. What **is** implemented from Branch 2

### 2.1 AMR alignment pipeline

**Implemented and wired into the engine.**

* **Alignment module structure**

  * `core/src/alignment/mod.rs` exposes AMR as `align_rows_amr`, with submodules:

    * `row_metadata.rs`, `anchor_discovery.rs`, `anchor_chain.rs`, `gap_strategy.rs`, `assembly.rs`, `move_extraction.rs`, `runs.rs`. 
  * Module docs explicitly map each piece to the unified spec sections and call out intentional deviations (no global move-candidate phase, no explicit move validation phase, RLE fast-path).

* **Row metadata & frequency classification**

  * `RowMeta` + `FrequencyClass` and `classify_row_frequencies` implemented in `row_metadata.rs`. It classifies rows as `Unique`, `Rare`, `Common`, `LowInfo` based on thresholds. 
  * `DiffConfig` contains `rare_threshold` and `low_info_threshold`, with sane defaults and tests. 
  * `GridView::from_grid_with_config` populates `row_meta` using this logic; tests confirm unique/rare/common classification behavior.

* **Anchor discovery & anchor chain**

  * `anchor_discovery.rs` finds anchors from `RowMeta` using unique rows only.
  * `anchor_chain.rs` implements LIS-based chain construction (`lis_by_key`) to ensure monotonic anchors, plus tests for monotonicity and LIS correctness.

* **Multi-gap alignment & gap strategies**

  * `gap_strategy.rs` defines `GapStrategy` and `select_gap_strategy`, choosing between `Empty`, `InsertAll`, `DeleteAll`, `SmallEdit`, `MoveCandidate`, `RecursiveAlign`.
  * `assembly.rs`:

    * `align_rows_amr` orchestrates: collect metadata → anchors → gaps → per-gap strategies → final `RowAlignment`. 
    * `fill_gap` applies the selected strategy.
    * Tests exercise:

      * Two disjoint insertion regions.
      * Insert + delete in different regions.
      * Gaps containing moved blocks.
      * Recursive alignment for large gaps.

* **Localized move detection**

  * `move_extraction.rs` implements `find_block_move` and `moves_from_matched_pairs`, with explicit doc comments that this is a *localized* gap-based move detector (a simplification of the global move-candidate + validation phases in the spec).
  * Tests cover block move detection and conversion of matched-pair sequences into `RowBlockMove`s.

* **RLE (run-length encoding) integration**

  * `runs.rs` defines `RowRun` and `compress_to_runs`, plus tests for:

    * Compression of 10k identical rows (into a single run).
    * Alternating pattern AB (little/no compression). 
  * `assembly.rs` uses `compress_to_runs` and a run-aware `align_runs_stable` path for highly repetitive grids, matching the “RLE fast path” described in both the sprint plan and the spec deviations docs.

* **Engine wiring**

  * The main diff engine uses AMR by default (calling `align_rows_amr`); `row_alignment.rs` is now explicitly documented as legacy/fallback.

**Conclusion: all algorithmic pieces 2.1–2.4 + 2.6 are implemented and well-tested.**

---

### 2.2 Configurable limits & `LimitBehavior` + partial results

**Implemented and matches the updated spec.**

* **DiffConfig and LimitBehavior**

  * `DiffConfig` includes `max_align_rows`, `max_align_cols`, `max_recursion_depth`, and `on_limit_exceeded: LimitBehavior`. Defaults: 500,000 rows, 16,384 columns, depth 10.
  * `LimitBehavior` enum exactly matches spec:

    * `FallbackToPositional`
    * `ReturnPartialResult`
    * `ReturnError`

* **Single limit enforcement point**

  * `diff_grids_with_config` is the *only* production call site that checks row/column limits and branches on `LimitBehavior`. AMR’s `align_rows_amr` no longer has its own limit branch.

* **Semantics when limits are exceeded**

  * `FallbackToPositional`: falls back to simple positional diff, flags the report as complete, and emits no warnings.
  * `ReturnPartialResult`: performs positional diff, sets `complete = false`, and pushes a sheet-specific warning message into `warnings`.
  * `ReturnError`: returns `DiffError::LimitsExceeded` from `try_diff_workbooks_with_config`, while the legacy non-`try_` API still panics in this mode (there’s a dedicated `limit_exceeded_return_error_panics_via_legacy_api` test confirming this).

* **DiffReport shape & JSON**

  * `DiffReport` now has:

    * `version`
    * `ops`
    * `complete: bool`
    * `warnings: Vec<String>`
    * `metrics: Option<DiffMetrics>` behind `perf-metrics` feature.
  * JSON helpers serialize `complete` and `warnings`. Tests such as:

    * `serialize_partial_diff_report_includes_complete_false_and_warnings`
    * `serialize_full_diff_report_has_complete_true_and_no_warnings`
    * `serialize_diff_report_with_metrics_includes_metrics_object`
      validate this behavior.

* **Limit behavior tests**

  * `tests/limit_behavior_tests.rs` covers:

    * Row/column limits not hit for large-but-within-limit inputs.
    * All three `LimitBehavior` variants, including structured error and partial result semantics.
    * Multi-sheet warnings including sheet name.
  * Large-grid and wide-grid tests use scaled-down sizes (5k rows instead of 50k, 500 columns) specifically to keep CI fast, which is consistent with the remediation guidance.

**Conclusion: 2.5 (“Remove/Raise Hard Caps & LimitBehavior”) is implemented, and the remediation’s correctness concerns are addressed.**

---

### 2.3 Perf metrics, fixtures, and CI

**Implemented, but the threshold script is only partially enforcing what the plan promised.**

* **DiffMetrics structure**

  * `core/src/perf.rs` defines `DiffMetrics` with:

    * `alignment_time_ms`, `move_detection_time_ms`, `cell_diff_time_ms`, `total_time_ms`
    * `rows_processed`, `cells_compared`, `anchors_found`, `moves_detected`
  * `parse_time_ms` and `peak_memory_bytes` were explicitly *dropped* from both code and spec and now appear as “Deferred metrics” in the spec, as intended by remediation B1.

* **Instrumentation**

  * A `Phase` enum and `start_phase` / `end_phase` helpers wrap the main phases: alignment, move detection, cell diff, total. The engine calls them in the right places.
  * Counts are wired:

    * `rows_processed` updated from the grid sizes.
    * `cells_compared` incremented in cell diff paths.
    * `anchors_found` and `moves_detected` updated from the AMR/rect-move paths.
  * Perf/perf-JSON tests assert that metrics are present and non-zero for perf fixtures, and that serialized JSON includes a `metrics` object when `perf-metrics` is enabled.

* **Perf fixtures**

  * `fixtures/manifest.yaml` defines P1–P5, matching the Branch 2 plan (dense, noise, adversarial repetitive, 99% blank, identical).
  * `fixtures/src/generators/perf.py` implements `LargeGridGenerator`, capable of generating these patterns, with configurable rows/cols/mode/pattern_length/fill_percent. 

* **Perf tests**

  * `tests/perf_large_grid_tests.rs` contains:

    * `perf_p1_large_dense`
    * `perf_p2_large_noise`
    * `perf_p3_adversarial_repetitive`
    * `perf_p4_99_percent_blank`
    * `perf_p5_identical`
      all gated with `#[cfg(feature = "perf-metrics")]`.
  * Each test:

    * Loads the matching fixture.
    * Runs the diff via public APIs with `perf-metrics` enabled.
    * Asserts success, non-empty ops for non-identical cases, and `metrics.is_some()` with non-zero counts.

* **Perf workflow & script**

  * `.github/workflows/perf.yml` exists and runs:

    * `cargo test --release --features perf-metrics perf_`
    * `python scripts/check_perf_thresholds.py`
      on a `perf-regression` job.
  * `scripts/check_perf_thresholds.py`:

    * Defines per-test thresholds in a `THRESHOLDS` dict for `perf_p1…perf_p5`.
    * Invokes `cargo test --release --features perf-metrics perf_ -- --nocapture` with a global 120s timeout.
    * Checks that all expected perf tests ran.
    * **Does not yet parse per-test durations or enforce the individual `max_time_s` from `THRESHOLDS`.**

**Conclusion: core perf infra (DiffMetrics, fixtures, perf tests, workflow) is done; the only real gap is that the threshold script doesn’t actually enforce the per-fixture timing table.**

---

### 2.4 Documentation & spec alignment

* `excel_diff_specification.md` has an updated section 17 that:

  * Documents the exact `DiffMetrics` struct now in code.
  * Documents `DiffConfig` and `LimitBehavior` semantics, including `ReturnError` via `try_diff_workbooks_with_config`.
  * Describes perf regression testing and references P1–P5, the perf test command, the threshold script, and the CI workflow.

* `alignment/mod.rs` and `move_extraction.rs` explicitly document the simplified AMR phases and how they deviate from the full unified spec (no global move candidate extraction, no separate move validation phase, RLE fast path).

* However, the **Activity Log** mentioned in the spec (`docs/meta/logs/2025-12-09-sprint-branch-2/activity_log.md`) does **not** exist in the codebase snapshot, nor is there a `docs/` tree at all.

---

## 3. What’s **not** implemented / still incomplete

Here are the remaining gaps relative to the current Branch 2 spec + remediation docs:

1. **Perf threshold enforcement is only partial**

   * `check_perf_thresholds.py` defines thresholds but doesn’t measure per-test duration or compare against `THRESHOLDS`; it only:

     * Ensures `cargo test` finishes before a global timeout.
     * Verifies that all expected perf tests appear in the output.
   * The original mini-spec & remediation C3 expect the script to:

     * Measure P1–P5 timings individually.
     * Compare against per-fixture thresholds (the table in the docs).
     * Fail the job if any threshold is exceeded.

2. **Thresholds are not configurable**

   * Current script hardcodes `THRESHOLDS` and doesn’t read from config or environment variables, contrary to the remediation suggestion (“Make thresholds configurable”).

3. **Activity Log for Branch 2 is missing**

   * Spec references `docs/meta/logs/2025-12-09-sprint-branch-2/activity_log.md` for implementation details and intentional spec deviations, but the file and `docs/` tree are absent from the snapshot.

4. **Metrics unit tests are minimal**

   * There are integration-level checks (perf tests + JSON serialization) but no focused unit tests for `DiffMetrics::start_phase` / `end_phase` behavior with a deterministic clock, as envisioned in remediation B3. This is a *quality* gap rather than a functional bug.

5. **Naming / doc nits**

   * `large_grid_50k_rows_completes_within_default_limits` actually uses 5000 rows, not 50,000. This is intentional for CI speed, but the test name and/or doc could be clarified to reflect that it’s a scaled-down proxy for the spec’s 50k-case.

Everything else that Branch 2 promised is present: AMR phases, frequency classification, anchors + LIS, multi-gap alignment, RLE, limit behavior & partial results, metrics, fixtures, perf tests, and perf CI wiring.

---

## 4. Implementation plan to finish Branch 2

Here’s a concrete plan for the remaining work.

### 4.1 Finish perf threshold enforcement

**Goal:** Make `check_perf_thresholds.py` actually enforce per-fixture performance budgets, while keeping it simple and robust.

**Steps:**

1. **Decide timing source**

   * Easiest option: have each perf test print its own `metrics.total_time_ms` (or a derived seconds value) via `println!`, in a machine-parsable format like:

     ```text
     PERF_METRIC perf_p1_large_dense total_time_ms=1234
     ```
   * This keeps the script decoupled from `cargo` internals and uses the metrics you already compute.

2. **Update perf tests**

   * For `tests/perf_large_grid_tests.rs`:

     * After asserting `metrics.is_some()`, also print the metrics line per test (only when `perf-metrics` feature is enabled).
     * Use a consistent prefix (`PERF_METRIC`) and include:

       * test name
       * total_time_ms
       * optionally rows_processed, cells_compared for debugging.

3. **Extend `check_perf_thresholds.py`**

   * After running `cargo test … perf_ -- --nocapture`:

     * Parse `result.stdout` line by line, find lines starting with `PERF_METRIC`.
     * Extract `test_name` and `total_time_ms`.
     * Convert to seconds and compare against `THRESHOLDS[test_name]["max_time_s"]`.
   * If any test is missing from the parsed metrics set or exceeds its threshold:

     * Print a clear error summary.
     * Exit with `sys.exit(1)`.
   * Keep the global `PERF_TEST_TIMEOUT_SECONDS` as a coarse guardrail.

4. **Make thresholds configurable**

   * Allow overriding default `THRESHOLDS` via env vars, e.g.:

     * `EXCEL_DIFF_PERF_P1_MAX_TIME_S`, etc.
   * Implementation:

     * For each test in `THRESHOLDS`, read `os.environ.get(env_name)`; if present, parse as float and override `max_time_s`.
   * Optional: allow a single “slack multiplier” env var like `EXCEL_DIFF_PERF_SLACK_FACTOR` to scale all thresholds.

5. **Update docs**

   * In `excel_diff_specification.md` section 17.3, clarify:

     * That per-fixture thresholds are enforced off the `PERF_METRIC` output.
     * How to adjust thresholds via env vars for different CI hardware.

**Done when:**

* Failing a per-fixture timing budget causes `scripts/check_perf_thresholds.py` to exit non-zero and fail the `perf-regression` job.
* You can locally bump thresholds via env vars to debug.

---

### 4.2 Add focused `DiffMetrics` unit tests

**Goal:** Prove that `start_phase` / `end_phase` and count fields behave as intended, independent of the full engine.

**Steps:**

1. **Introduce a deterministic clock abstraction**

   * In `core/src/perf.rs`, add an internal `Clock` trait (or simple injected time source) used by `DiffMetrics` in tests:

     * For production, keep using `std::time::Instant::now()`.
     * For tests, you can expose a feature or helper that uses a fake clock you control.

2. **Add `tests/metrics_unit_tests.rs`**

   * Tests could include:

     * `metrics_accumulates_phase_durations`:

       * Manually set timestamps for alignment, move detection, cell diff, and ensure correct accumulation into the `*_time_ms` fields.
     * `metrics_counts_accumulate_safely`:

       * Call helper methods to add `rows_processed`, `cells_compared`, `anchors_found`, `moves_detected` and assert the final values, including saturating behavior if present.
     * `metrics_total_time_ms_is_consistent_with_phases`:

       * Ensure `total_time_ms` is at least the sum of the principal phases or matches expected accounting.

3. **Wire these into CI**

   * They’re regular unit tests; no feature gate needed (unless you want to gate them on `perf-metrics` as well).

**Done when:**

* `DiffMetrics` semantics are validated by unit tests and not just indirectly via perf tests.

### 4.3 Large Grid Test
large_grid_50k_rows_completes_within_default_limits actually uses 5000 rows, not 50,000. If we only ever test against 5,000 rows, we'll never know how viable the code is for large grids/diffs. 