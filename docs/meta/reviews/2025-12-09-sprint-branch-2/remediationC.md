Here’s a concrete “do-this-exactly” plan you can hand to an AI coding agent to finish Branch 2 (AMR + perf infra) and close the remaining gaps.

I’ll organize it into work packets the agent can tackle one by one.

---

## 0. Orientation for the Coding Agent

You’re working in the `excel_diff` repo, Rust project, core crate `excel_diff` (the one that `cargo test` just ran). The design and expectations for Branch 2 are in:

* `docs/rust_docs/next_sprint_plan.md` – Branch 2: Grid Algorithm Scalability (AMR) + Performance Infrastructure. 
* `docs/rust_docs/unified_grid_diff_algorithm_specification.md` – authoritative AMR algorithm spec. 
* `docs/rust_docs/excel_diff_specification.md` – DiffReport / JSON / error contracts. 
* `docs/meta/prompts/review_prompt.md` (aliased as `codebase_context.md`) – describes how the current code is structured. 

The verification report + earlier remediation ideas are captured in `combined_remediations.md`. 

The latest test run (in `cycle_summary.txt`) confirms a lot of fixes and tests are already in place: multi‑gap AMR tests, RLE tests, limit behavior tests, JSON partial-result tests, large-grid tests, etc. 

You **do not** need to re‑implement those; treat them as guardrails. Your job is to finish what’s still missing:

1. Make limit handling internally consistent and fully spec‑aligned (clean up duplication).
2. Finish and harden performance metrics instrumentation.
3. Build the perf fixtures and perf regression harness + CI workflow.
4. Finalize metrics / partial‑result JSON behavior and tests.
5. Document intentional spec deviations and alignment modules.
6. Clean up legacy alignment code and unused paths.

---

## Work Packet A – Limit Handling & Partial Result Semantics

### A1. Unify limit checks between `diff_grids_with_config` and `align_rows_amr`

**Goal:** There should be one coherent place where row/column limits and `LimitBehavior` are enforced and all callers behave consistently with the spec.

**Steps:**

1. **Locate limit logic in the engine:**

   * Open `core/src/engine.rs` and find `diff_grids_with_config`. There is a limit guard of the form:

     ```rust
     let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
         || old.ncols.max(new.ncols) > config.max_align_cols;
     ```

     followed by a `match config.on_limit_exceeded` branch. 

   * Open `core/src/alignment/assembly.rs` (or wherever `align_rows_amr` lives) and find `align_rows_amr`. It has its own limit check and a `match LimitBehavior` that returns `None`, `Some(RowAlignment::default())`, or panics. 

2. **Decide the single source of truth:**

   * Use `diff_grids_with_config` as the **only** place that decides what to do when `max_align_rows` / `max_align_cols` are exceeded.
   * `align_rows_amr` should assume its inputs are within limits (except potentially in tests that call it directly).

3. **Refactor `align_rows_amr`:**

   * Remove or gate the internal limit check:

     * If the internal branch is unused in production code, delete it entirely.
     * If unit tests rely on it, add a small wrapper function used only in tests (e.g. `align_rows_amr_with_limits_for_test`) that performs the check and delegates to a pure `align_rows_amr_inner` which assumes limits are ok.
   * Ensure `align_rows_amr` returns `RowAlignment` or `Option<RowAlignment>` based purely on AMR success/fallback semantics, not on `LimitBehavior`.

4. **Refine `diff_grids_with_config` limit behavior (now that tests exist):**

   * Inspect the existing behavior using the tests in `tests/limit_behavior_tests.rs`, especially:

     * `limit_exceeded_return_partial_result`
     * `limit_exceeded_return_error_returns_structured_error`
     * `limit_exceeded_fallback_to_positional`
     * `large_grid_50k_rows_completes_within_default_limits`
     * `wide_grid_500_cols_completes_within_default_limits` 
   * Your refactor must keep these tests passing. That means:

     * `LimitBehavior::FallbackToPositional` still triggers the positional diff path.
     * `LimitBehavior::ReturnError` returns the structured error type (not a panic) via the public API, but may still panic through legacy API paths where tests explicitly expect a panic (see `limit_exceeded_return_error_panics_via_legacy_api`).
     * `LimitBehavior::ReturnPartialResult` returns a `DiffReport` with:

       * `complete == false`
       * An appropriate `warnings` entry, including sheet name in multi-sheet scenarios (see `multiple_sheets_limit_warning_includes_sheet_name`).

5. **Preserve partial result semantics across layers:**

   * In the JSON helper (`diff_workbooks_to_json` or equivalent) make sure the serialized report includes:

     * `"complete": false` when the limit behavior produced a partial result.
     * A non-empty `"warnings"` array with a clear limit message.
   * The test `serialize_partial_diff_report_includes_complete_false_and_warnings` in `tests/output_tests.rs` already asserts this; keep it passing.

6. **Re-run tests and ensure nothing regressed:**

   * `cargo test limit_behavior_tests`
   * `cargo test output_tests`
   * `cargo test engine_tests` (for integration coverage)
   * `cargo test pg4_diffop_tests` (JSON schema tests)

**Done when:**

* All limit handling tests pass unchanged.
* There is only one production call site enforcing row/column limits.
* The public API path never silently returns an empty diff for `ReturnPartialResult`; it always carries explicit completeness/warnings information.

---

## Work Packet B – Complete Perf / Metrics Instrumentation

Branch 2 promised usable `DiffMetrics` behind a `perf-metrics` feature, with alignment/move/cell timings and a small set of counts.

Many pieces exist, but some fields still appear unused or misleading (e.g., `cells_compared`, `anchors_found`, potential `parse_time_ms` / `peak_memory_bytes`). 

### B1. Confirm the intended metrics shape

1. Open `docs/rust_docs/next_sprint_plan.md` and find the Branch 2 performance section (`2.7 Performance Infrastructure`) that describes `DiffMetrics`. 

2. Align that with the **current** `DiffMetrics` Rust struct in `core/src/perf.rs` (or wherever metrics sit).

3. Ensure the near-term contract matches the plan:

   * Required fields now:

     * `alignment_time_ms`, `move_detection_time_ms`, `cell_diff_time_ms`, `total_time_ms`
     * `rows_processed`, `cells_compared`, `anchors_found`, `moves_detected`
   * `parse_time_ms` and `peak_memory_bytes` are **either**:

     * Completely removed from the struct and from the spec (preferred short‑term), **or**
     * Present but explicitly documented as “not yet populated” and hidden behind a future feature.

4. If the spec and code disagree, update the **spec** (Branch 2 plan + `excel_diff_specification.md`) to match the actual, intended near‑term metrics surface and remove references to metrics you are not going to implement now.

### B2. Wire up all counts and timers

Assuming the agreed fields are:

```rust
#[cfg(feature = "perf-metrics")]
pub struct DiffMetrics {
    pub alignment_time_ms: u64,
    pub move_detection_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
}
```

**Steps:**

1. **Find metrics start/end helpers:**

   * Locate the `DiffMetrics` impl in core (likely an enum `Phase` with `start_phase`/`end_phase` methods). 
   * Verify that:

     * `alignment_time_ms` is updated around row/column alignment.
     * `move_detection_time_ms` is updated around move detection.
     * `cell_diff_time_ms` is updated around the cell comparison loop.
     * `total_time_ms` brackets the entire diff call.

2. **Increment `rows_processed`:**

   * In `diff_grids_with_config` (or the top-level grid diff path), after computing the `GridView` or row metadata, set `rows_processed` to something like `max(old.nrows, new.nrows)` or the actual number of rows the AMR pipeline processes.

3. **Increment `cells_compared`:**

   * Find the cell-to-cell comparison loop (Phase 5 in the unified spec – cell‑level diff). 
   * Every time you compare a pair of cells (even if equal), increment `metrics.cells_compared` by 1 under the `perf-metrics` feature.
   * Do **not** increment for cells that are *structurally* added/removed and never actually compared.

4. **Set `anchors_found`:**

   * In the AMR pipeline, right after building the anchor chain (LIS over discovered anchors), set `anchors_found` to the length of that chain. The unified spec describes this anchor chain explicitly.

5. **Increment `moves_detected` for **all** move types:**

   * Identify where row, column, and rectangle moves are finally accepted and turned into `DiffOp::RowBlockMoved`, `DiffOp::BlockMovedColumns`, `DiffOp::BlockMovedRect`, etc.
   * In the code that appends each move op to the result, bump `metrics.moves_detected += 1`.
   * Include both legacy block move detection and AMR `RowBlockMove` results.

6. **Handle optional/removed metrics:**

   * If you keep `parse_time_ms`, wrap workbook parse and `GridView` construction in a `Phase::Parse` timer and populate it.
   * If you drop it from the near-term contract, remove the field and any references entirely and update the spec / docs accordingly. Same for `peak_memory_bytes`.

### B3. Tests for metrics behavior

1. **Unit tests for `DiffMetrics` mechanics:**

   * Add tests in the module where `DiffMetrics` lives (or a dedicated `perf_tests.rs`) that:

     * Manually call `start_phase` / `end_phase` with a fixed fake clock (or stub time provider) to verify timing accumulation.
     * Check that calling `start_phase` twice in a row does something reasonable (ignore second, or restart; document whichever you choose).

2. **Integration test for non‑zero fields:**

   * Create a new test (e.g. `tests/metrics_smoke_tests.rs`, behind `#[cfg(feature = "perf-metrics")]`) that:

     * Constructs a small workbook pair with a few edits.
     * Calls the diff via the public API with `perf-metrics` enabled.
     * Asserts that:

       * `metrics` is `Some`.
       * `rows_processed >= 1`.
       * `cells_compared > 0`.
       * `anchors_found` is >= 0 (and > 0 if your fixture has anchors).
       * `moves_detected` is > 0 for a scenario with a known move.

3. **JSON shape test:**

   * Extend `pg4_diff_report_json_shape` (already present) so that under `perf-metrics` it also asserts that:

     * A `"metrics"` object exists.
     * It has exactly the fields you decided on; **no** `parse_time_ms` or `peak_memory_bytes` if you removed them.

**Done when:**

* All metrics‑related counts and timers are populated in realistic scenarios.
* You can see non‑zero values for `cells_compared`, `anchors_found`, `moves_detected` in integration tests.
* JSON schema tests verify metrics shape.

---

## Work Packet C – Perf Fixtures, Harness & CI Perf Regression

The Branch 2 plan calls for explicit perf fixtures P1–P5 and a CI job that runs them with thresholds. 

We see `tests/perf_large_grid_tests.rs` exists but defines 0 tests, and CI perf harness is missing.

### C1. Fixtures & generators

1. **Manifest entries:**

   * Open `fixtures/excel_diff_test_matrix.yaml` (or the equivalent manifest) and confirm P1 & P2 entries exist. Add entries for:

     * `p3_adversarial_repetitive` (50K×50, heavy repetition)
     * `p4_99_percent_blank` (50K×100, mostly blank, small noisy region)
     * `p5_identical` (50K×100, identical grids)

2. **Generator implementation:**

   * In `fixtures/src/generators/perf.py` (or your fixture generator binary):

     * Implement generator functions for each new fixture ID:

       * P3: create two grids with many identical rows and a deliberate block insert or move.
       * P4: create mostly blank rows with a small changed region.
       * P5: create two identical large grids.
   * Hook them into whatever CLI or build script the test harness uses (e.g. `python fixtures/src/generators/perf.py --generate p3_adversarial_repetitive`).

3. **Document generation commands:**

   * Add a short doc comment or README snippet in `fixtures/` describing how to regenerate P1–P5.

### C2. Perf harness tests in Rust

1. **Populate `tests/perf_large_grid_tests.rs`:**

   * Add tests (gated by `#[cfg(feature = "perf-metrics")]`) such as:

     * `perf_p1_large_dense`
     * `perf_p2_large_noise`
     * `perf_p3_adversarial_repetitive`
     * `perf_p4_99_percent_blank`
     * `perf_p5_identical`
   * Each test should:

     * Locate its fixture pair using the existing fixture helpers (e.g. `fixture_path("p1_large_dense_old")`).
     * Call the public diff API with `DiffConfig::default()` and `perf-metrics` enabled.
     * Assert:

       * Diff completes without error.

       * For non-identical fixtures, the ops vector is non-empty.

       * `metrics` is `Some` and `rows_processed` == expected row count for that fixture.

   > These tests do **not** need to assert time thresholds; they just validate correctness and metrics output on large inputs.

2. **Keep them reasonably fast:**

   * For CI, you can:

     * Decrease grid dimensions in tests (e.g., 10K instead of 50K) while retaining structure.
     * Or add a `#[ignore]` flag for full‑scale tests and run them only from the perf script (below).

### C3. Perf thresholds script

1. **Add `scripts/check_perf_thresholds.py`:**

   * Script responsibilities:

     * Run the perf tests in release mode with `perf-metrics` enabled:

       * e.g. `cargo test --release --features perf-metrics perf_ -- --nocapture`.
     * Parse either:

       * Test durations (using `time` output, or `cargo test` `--message-format=json`), **or**
       * Metrics emitted in test output (you can print `metrics.total_time_ms` from each perf test).
     * Compare them against thresholds defined in a table matching the Branch 2 plan:

       * P1: 50K×100 < 5s
       * P2: 50K×100 < 10s
       * P3: 50K×50 < 15s
       * P4: 50K×100 < 2s
       * P5: 50K×100 < 1s 
     * Exit with non‑zero status if any threshold is exceeded.

2. **Make thresholds configurable:**

   * Read thresholds from a small config file or environment variables so they can be tuned for CI hardware vs local machines.

### C4. CI perf workflow

1. **Add `.github/workflows/perf.yml`:**

   * Job `perf-regression` should:

     * Check out the repo.
     * Build with `--release --features perf-metrics`.
     * Run the perf tests (subset, or tagged with a `perf_` prefix).
     * Run `python scripts/check_perf_thresholds.py`.
   * Keep this job separate so it can be triggered on main, nightly, or manually rather than on every PR if that’s too heavy.

2. **Document this in the project docs:**

   * Add a short “Performance Regression Testing” section to `excel_diff_specification.md` or a dedicated `docs/perf.md` describing:

     * The P1–P5 fixtures.
     * How to run perf tests locally.
     * What thresholds are enforced in CI.

**Done when:**

* P1–P5 fixtures exist and can be regenerated.
* `tests/perf_large_grid_tests.rs` actually contains tests and passes in debug and release.
* `scripts/check_perf_thresholds.py` enforces thresholds.
* A CI workflow runs this script as a separate job.

---

## Work Packet D – JSON & Partial Results & Metrics at the Output Boundary

You already have tests like `serialize_partial_diff_report_includes_complete_false_and_warnings` and many JSON shape tests.

Now we want to make sure partial results + metrics are fully covered and spec‑aligned.

### D1. Ensure `DiffReport` carries completeness and warnings everywhere

1. **Inspect `DiffReport` struct:**

   * Confirm it has at least:

     * `version`
     * `ops`
     * `complete: bool`
     * `warnings: Vec<String>`
     * Optional `metrics` behind `#[cfg(feature = "perf-metrics")]`

2. **Check all construction sites:**

   * Wherever `DiffReport` is created:

     * For normal successful diffs: set `complete = true`, `warnings = []`.
     * For partial results (from Work Packet A): set `complete = false` and a suitable warning string.
     * For error paths that still return a `DiffReport` (if any) ensure the combination of `complete`/`warnings` is coherent.

### D2. JSON tests

1. **In `tests/output_tests.rs`:**

   * You already have `serialize_partial_diff_report_includes_complete_false_and_warnings`. Verify that it:

     * Asserts `complete: false`.
     * Checks at least one warning string mentions “limit” / “partial” so downstream consumers can distinguish this case.
   * Add a complementary test:

     * `serialize_full_diff_report_has_complete_true_and_no_warnings`:

       * Build a trivial report for identical workbooks.
       * Assert JSON shows `complete: true` and `warnings` is `[]` or omitted entirely (document whichever you choose).

2. **Metrics serialization:**

   * Add `serialize_diff_report_with_metrics_includes_metrics_object` if it doesn’t already exist:

     * Enable `perf-metrics`.
     * Build a report with a populated `metrics`.
     * Serialize and assert the `metrics` object appears with expected field names and **no** extra fields.

3. **Error surface test:**

   * From remediationB: ensure that when `LimitBehavior::ReturnError` is used, the JSON helper returns a structured error, not a panic. Add an integration test that:

     * Forces a limit exceed.
     * Uses a config with `ReturnError`.
     * Asserts the top-level JSON diff function returns an error type or error JSON structure as designed (update `excel_diff_specification.md` to describe this).

---

## Work Packet E – Documentation: AMR Phase Simplifications & Module Docs

The implementation deviates slightly from the “four phase” AMR spec: global move-candidate extraction is simplified and move detection is localized inside gaps.

The plan and spec expect these deviations to be **documented**.

### E1. Activity Log and Intentional Spec Deviations

1. **Create an activity log file for this branch:**

   * Location: e.g. `docs/meta/activity_logs/2025-12-09-sprint-branch-2.md`.
   * Summarize:

     * Which parts of the unified grid spec are fully implemented.
     * Which parts are partially implemented or simplified, especially:

       * Global move-candidate extraction (spec phases 2 & 4) vs localized `GapStrategy::MoveCandidate`.
       * Perf metrics deferral of parse time and memory-usage fields (if you chose to defer them).
       * Any limit-handling semantics that differ slightly from the original mini-plan.

2. **Link from cycle summary / plan:**

   * Update `cycle_plan.md` or `next_sprint_plan.md` to reference this activity log, so future reviewers can find it.

### E2. Module-level doc comments in `alignment/`

1. For each module under `core/src/alignment/`:

   * `row_metadata.rs`
   * `anchor_discovery.rs`
   * `anchor_chain.rs`
   * `gap_strategy.rs`
   * `assembly.rs`
   * `runs.rs`
   * `move_extraction.rs` (if present)

   Add a `//!` module doc at the top that includes:

   * A brief description (what this module does in the AMR pipeline).
   * A pointer to the relevant spec sections (e.g., “See unified_grid_diff_algorithm_specification.md, Section 9.5–9.7 / 11 for global move-candidate extraction”).

2. Keep comments concise but explicit:

   * Note where behavior intentionally **differs** from the spec (e.g., “We only detect move candidates within gaps, not globally across the entire sheet, as described in the spec; see activity log”).

### E3. Metrics & perf docs

1. Update `excel_diff_specification.md` to:

   * Document the final `DiffMetrics` shape and semantics (see Work Packet B).
   * Clearly mark metrics that are deferred to a future branch.
   * Point to the CI perf workflow and fixtures, once created.

---

## Work Packet F – Legacy Row-Alignment Path Cleanup

Branch 1/early code left a “legacy” row alignment implementation (`row_alignment.rs`) and some unused rect-move helpers. These now produce warnings and confuse maintainers.

### F1. Identify unused functions

1. Run `cargo test` with warnings and inspect the warnings emitted for unused functions in:

   * `core/src/row_alignment.rs`
   * `core/src/rect_block_move.rs`
     These likely include helpers like:
   * `detect_exact_row_block_move`
   * `detect_fuzzy_row_block_move`
   * `align_row_changes`
     etc. 

2. Cross-check usage:

   * Use `rg "align_row_changes"` and similar searches to confirm which functions aren’t referenced from anywhere meaningful (other than tests).

### F2. Decide retention strategy

You have two options:

1. **Keep as legacy but clearly labelled:**

   * Add a module-level doc comment at the top of `row_alignment.rs`:

     * Explain that this is a legacy path preserved only for specific tests or future experiments.
   * Guard the entire legacy module or selected functions behind a feature flag, e.g. `legacy-row-align`, disabled by default.
   * Ensure tests that rely on the legacy code are explicitly tied to that feature or updated to exercise AMR instead.

2. **Remove unused functions:**

   * If no tests / features truly depend on them, delete unused functions and their tests.
   * Keep only the minimal bits still needed by current code (if any).

### F3. Verify move detection tests still pass

After cleanup, run:

* `cargo test g10_row_block_alignment_grid_workbook_tests`
* `cargo test g11_row_block_move_grid_workbook_tests`
* `cargo test g12_column_block_move_grid_workbook_tests`
* `cargo test g12_rect_block_move_grid_workbook_tests`
* `cargo test g13_fuzzy_row_move_grid_workbook_tests`
* `cargo test g14_move_combination_tests` 

All of these must still pass; they confirm that the **new** AMR + move detection paths are exercised and legacy leftovers aren’t required for correctness.

---

## Work Packet G – Sanity: Multi-gap AMR & RLE Tests (Already Present but Guardrails)

This packet is mostly about **confirming** that the multi-gap and RLE tests that now exist fully align with the original plan, and adjusting only if something is missing. The heavy lifting appears done.

### G1. Multi-gap AMR tests

1. From `cycle_summary.txt` we know there are tests like: 

   * `alignment::assembly::tests::amr_disjoint_gaps_with_insertions_and_deletions`
   * `alignment::assembly::tests::amr_multi_gap_move_detection_produces_expected_row_block_move`
   * `tests/amr_multi_gap_tests.rs` with:

     * `amr_two_disjoint_insertion_regions`
     * `amr_insertion_and_deletion_in_different_regions`
     * `amr_gap_contains_moved_block_scenario`
     * `amr_recursive_gap_alignment`

2. Compare these against the Branch 2 plan’s required scenarios:

   * Two disjoint insertion regions
   * Insertion + deletion in different regions
   * Gap contains moved block
   * Recursive gap alignment behavior

3. If any scenario is missing at the **RowAlignment** level (not just final `DiffOp` level), add tests that assert:

   * Monotonicity of the matched pairs.
   * Correct classification of inserted / deleted rows.
   * Correct `RowBlockMove` structure where moves are expected.

### G2. RLE tests

1. We see tests in `alignment::runs::tests` like: 

   * `compresses_10k_identical_rows_to_single_run`
   * `alternating_pattern_ab_does_not_overcompress`

2. Confirm they are aligned with the spec’s RLE design (Part III/IV of the unified spec).

3. If needed, add one high-level test that uses the compressed run alignment path (not just the run compressor) and validates that a 10K-identical-rows scenario diff behaves correctly when there’s a block insert.

**No major new implementation required here**; you’re just verifying and topping up tests where necessary.

---

## Final Validation Checklist for the Agent

After implementing the above work packets:

1. **Unit & integration tests:**

   * `cargo test` at the crate root should still pass, including:

     * `limit_behavior_tests`
     * `output_tests`
     * `amr_multi_gap_tests`
     * `alignment::assembly` and `alignment::runs` tests
     * All `g10`–`g15` grid workbook tests. 

2. **Perf tests:**

   * Run perf tests in release mode with `perf-metrics`:

     * `cargo test --release --features perf-metrics perf_ -- --nocapture`
   * Confirm the new script `scripts/check_perf_thresholds.py` passes locally.

3. **Metrics sanity:**

   * Run a small diff and inspect JSON:

     * Check `complete`, `warnings`, and `metrics` are present and correctly populated for both:

       * Normal completion.
       * Limit-induced partial result.

4. **Documentation:**

   * Confirm the Activity Log for this branch exists and references key deviations.
   * Confirm module docs in `alignment/` mention the relevant spec sections.
   * Confirm `excel_diff_specification.md` documents the final `DiffMetrics` and limit behavior semantics.

If all of that holds, Branch 2 is functionally complete and aligned with both `next_sprint_plan.md` and the unified grid diff spec, and the perf/limit behavior surfaces are solid enough for release.
