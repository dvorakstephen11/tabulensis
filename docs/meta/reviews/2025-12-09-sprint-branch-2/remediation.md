# Verification Report: 2025-12-09-sprint-branch-2

## Summary

This branch lands the new AMR-based row alignment pipeline (row metadata, anchor discovery, LIS-based anchor chain, gap strategies, RLE fast path) and wires it into the main diff engine. The implementation broadly follows the unified grid diff specification and the Branch 2 mini-plan, and all existing regression tests plus new unit tests for the alignment phases are passing. However, several important aspects of the plan are not fully realized: limit-handling “partial result” behavior is incorrect, performance/metrics infrastructure is only partially wired, multi-gap alignment and limit behaviors lack the tests called out in the plan, and some AMR phases (global move candidate extraction and validation) are simplified relative to the spec with no documented justification. Because one of these issues can cause the engine to silently return an empty diff when limits are exceeded under certain configurations, this branch should **not** be released as-is.

## Recommendation

[ ] Proceed to release  
[X] Remediation required

---

## Findings

### 1. LimitBehavior::ReturnPartialResult silently returns an empty diff

- **Severity**: Critical  
- **Category**: Bug / Spec Deviation  
- **Description**:  
  When row/column limits are exceeded, `diff_grids_with_config` checks `config.max_align_rows` / `max_align_cols` and branches on `config.on_limit_exceeded`. For `LimitBehavior::ReturnPartialResult`, it simply returns early without emitting any operations or setting any flags on the result. :contentReference[oaicite:0]{index=0}  
  At the same time, `DiffReport` has no completeness/partial-result flags, so callers cannot distinguish this “partial result” from a genuine “no differences” result. 
- **Evidence**:  
  ```rust
  let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
      || old.ncols.max(new.ncols) > config.max_align_cols;
  if exceeds_limits {
      #[cfg(feature = "perf-metrics")]
      if let Some(m) = metrics.as_mut() {
          m.end_phase(Phase::MoveDetection);
      }
      match config.on_limit_exceeded {
          LimitBehavior::FallbackToPositional => {
              positional_diff(sheet_id, old, new, ops);
          }
          LimitBehavior::ReturnPartialResult => {}
          LimitBehavior::ReturnError => {
              panic!(
                  "alignment limits exceeded (rows={}, cols={})",
                  old.nrows.max(new.nrows),
                  old.ncols.max(new.ncols)
              );
          }
      }
      return;
  }
  ``` :contentReference[oaicite:2]{index=2}  
  `DiffReport` only holds `version`, `ops`, and optional `metrics` (behind a feature flag), with no completeness/truncation flags. 
- **Impact**:  
  - For any caller that sets `on_limit_exceeded = ReturnPartialResult`, if the input exceeds row/column limits, they will receive a `DiffReport` with an empty `ops` vector and no indication that the diff is incomplete.  
  - Downstream consumers will treat “partial and empty” as “complete and equal,” which violates the behavioral contract described in the unified spec (Section 24.7 / 24.8: explicit partial result/error reporting).   
  - This is especially dangerous because `max_align_rows` is now relatively high (500_000), and the configuration is externally configurable via `DiffConfig`. :contentReference[oaicite:5]{index=5}  

---

### 2. AMR limit handling duplicated and internally inconsistent

- **Severity**: Moderate  
- **Category**: Bug / Spec Deviation  
- **Description**:  
  `align_rows_amr` itself also performs a limit check and returns `None`, `Some(RowAlignment::default())`, or panics based on `LimitBehavior`. :contentReference[oaicite:6]{index=6}  
  However, `diff_grids_with_config` already enforces the same limits before calling `align_rows_amr`, so the internal limit branch in `align_rows_amr` is effectively unreachable in normal use. :contentReference[oaicite:7]{index=7}  
- **Evidence**:
  ```rust
  pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
      if old.nrows.max(new.nrows) > config.max_align_rows
          || old.ncols.max(new.ncols) > config.max_align_cols
      {
          return match config.on_limit_exceeded {
              LimitBehavior::FallbackToPositional => None,
              LimitBehavior::ReturnPartialResult => Some(RowAlignment::default()),
              LimitBehavior::ReturnError => panic!( ... ),
          };
      }
      // ...
  }
  ``` :contentReference[oaicite:8]{index=8}
- **Impact**:  
  - The duplicated limit handling is confusing and violates the “single source of truth” principle for resource limits.  
  - Future refactors or new call sites of `align_rows_amr` could accidentally depend on these semantics in ways that diverge from `diff_grids_with_config`, making behavior unpredictable.  
  - This inconsistency likely contributed to the `ReturnPartialResult` bug above and will make fixing it harder if not cleaned up.

---

### 3. Performance/metrics infrastructure is only partially implemented

- **Severity**: Moderate  
- **Category**: Gap / Spec Deviation / Missing Test  
- **Description**:  
  Branch 2’s plan calls for a `perf-metrics` feature flag, a `DiffMetrics` struct with phase timings and counts, and a CI perf workflow with regression thresholds and large fixtures (P1–P5). :contentReference[oaicite:9]{index=9}  

  The current implementation includes:
  - `DiffMetrics` with fields for phase timings, row/cell counts, anchor/move counts, and peak memory. :contentReference[oaicite:10]{index=10}  
  - A `perf-metrics` feature flag in `core/Cargo.toml` and `#[cfg(feature = "perf-metrics")]` usage in `engine.rs` and `DiffReport`.   

  But several aspects are missing or incomplete:
  - `parse_time_ms`, `cells_compared`, `anchors_found`, and `peak_memory_bytes` are never updated anywhere. Only `rows_processed`, some phase timers, and `moves_detected` (for rect moves) are touched.   
  - There is no memory tracking integration at all; `peak_memory_bytes` remains at its default.  
  - No dedicated perf regression tests, scripts (`check_perf_thresholds.py`), or CI workflows (`.github/workflows/perf.yml`) are present in the codebase snapshot. (Searches for these paths and names return nothing.)   
  - The large perf fixtures described in the plan (P3–P5: adversarial repetitive, 99% blank, deeply nested tables) are not present in the fixture manifest, and only P1 and P2 are defined.   
- **Impact**:  
  - There is no automated guardrail against performance regressions, despite AMR being a complex, performance-sensitive change.  
  - Metrics surfaced to consumers will be misleading (e.g., parse_time_ms and cells_compared remain zero), making them unusable for monitoring.  
  - Memory budgets from the unified spec (Section 25) are not enforced or observable, which is particularly risky in WASM environments.   

---

### 4. Planned multi-gap AMR tests are missing

- **Severity**: Moderate  
- **Category**: Missing Test / Gap  
- **Description**:  
  The Branch 2 mini-spec calls out explicit tests to validate multi-gap handling in AMR:  
  - “two disjoint insertion regions”  
  - “insertion + deletion in different regions”  
  - “gap contains moved block” :contentReference[oaicite:16]{index=16}  

  The new AMR assembly code (`assemble_from_meta`, `fill_gap`, `GapStrategy`) supports arbitrary gaps and strategies (InsertAll, DeleteAll, SmallEdit, MoveCandidate, RecursiveAlign).   

  However:
  - The only direct tests in `alignment::assembly` focus on the run-length encoded fast path (`aligns_compressed_runs_with_insert_and_delete`, etc.), not on multi-gap anchor traversal and recursive gap-filling.   
  - There are no unit or integration tests that explicitly construct multiple, disjoint gaps and validate AMR’s behavior against expected row-move/insert/delete sets.  
  - Existing higher-level grid tests (g1–g15) predate AMR and do not specifically target the new gap strategy/recursion logic.   
- **Impact**:  
  - Multi-gap behavior (a core justification for AMR in the spec) is minimally exercised by the current test suite.  
  - Subtle bugs in gap partitioning, recursion depth limits, or combined move+edit scenarios could slip through into production, especially on large, messy workbooks with multiple independent edit regions.  

---

### 5. Limit behavior and large-grid tests from the plan are missing

- **Severity**: Moderate  
- **Category**: Missing Test / Gap  
- **Description**:  
  The Branch 2 plan requires tests for: :contentReference[oaicite:20]{index=20}  
  - Successful alignment of a 50K-row grid.  
  - Successful alignment of a 500-column grid.  
  - Explicit validation of `LimitBehavior` variants (fallback to positional, partial result, error).  

  The current test suite (114 unit tests reported in the latest cycle) has no tests that construct such large grids or exercise `max_align_rows`, `max_align_cols`, or `on_limit_exceeded`. Searches for 50_000, 500_000, or direct references to `LimitBehavior` in tests return no results.   
- **Impact**:  
  - The “happy path” for large but supported grids (e.g., 50K x 100) is not validated, so we have no evidence that AMR behaves as expected at the designed scale.  
  - The limit-handling bug in Finding 1 would likely have been caught by the planned tests; their absence directly contributed to it remaining undetected.  
  - Any future changes to `max_align_rows` / `max_align_cols` or LimitBehavior could regress silently.

---

### 6. AMR move-candidate phase simplified vs spec (no global phase 2 / 4)

- **Severity**: Moderate  
- **Category**: Spec Deviation  
- **Description**:  
  The unified spec describes AMR as having explicit phases:  
  1. Anchor discovery  
  2. Move candidate extraction (global out-of-order matches)  
  3. Move-aware gap filling  
  4. Move validation and emission.   

  The implementation instead:
  - Uses `discover_anchors_from_meta` and `build_anchor_chain` as specified for Phase 1.   
  - Does **not** implement a global move-candidate extraction step (no `unanchored_matches` or equivalent).  
  - Detects row block moves opportunistically inside gaps via `GapStrategy::MoveCandidate`, `align_small_gap`, and `moves_from_matched_pairs`/`find_block_move`.   

- **Impact**:  
  - Some patterns that the spec intends to detect via global out-of-order matches (especially competing move candidates) may not be recognized, or may only be seen as insert/delete rather than moves.  
  - This could reduce move-detection quality on complex reordering scenarios (e.g., multiple overlapping moved blocks), though existing move tests (g10–g14) still pass. :contentReference[oaicite:25]{index=25}  
  - Because there is no Activity Log / Intentional Spec Deviations section for this cycle, this simplification is undocumented and may confuse future maintainers who expect the four-phase AMR described in the spec.   

---

### 7. Performance fixtures and CI perf workflow from Branch 2 plan are missing

- **Severity**: Moderate  
- **Category**: Gap / Missing Test  
- **Description**:  
  Branch 2 explicitly calls for perf fixtures P1–P5 and a CI perf workflow that runs them and enforces thresholds (e.g., 50K x 100 grid under 5 seconds, adversarial repetitive case P3). :contentReference[oaicite:27]{index=27}  

  In the current codebase snapshot:
  - The fixture matrix includes only `p1_large_dense` and `p2_large_noise`. There are no entries for `p3_adversarial_repetitive`, `p4_99_percent_blank`, or `p5_nested_tables`.   
  - No scripts or CI workflows for perf regression are present. :contentReference[oaicite:29]{index=29}  
- **Impact**:  
  - The team has no machine-readable way to confirm that AMR meets its intended performance characteristics.  
  - Regressions in runtime or memory use on large or adversarial workloads will not be automatically caught.  
  - This weakens confidence in releasing a significantly more complex alignment algorithm.

---

### 8. Diff metrics are incomplete/inaccurate

- **Severity**: Minor  
- **Category**: Gap / Spec Deviation  
- **Description**:  
  `DiffMetrics` exposes a rich set of fields, but only a subset are actually maintained:  
  - `rows_processed` is incremented in `diff_grids_with_config`.   
  - `move_detection_time_ms`, `alignment_time_ms`, `cell_diff_time_ms`, and `total_time_ms` are updated via `start_phase`/`end_phase` calls for some phases.   
  - `parse_time_ms`, `cells_compared`, `anchors_found`, and `peak_memory_bytes` are never updated anywhere.  
  - `moves_detected` appears to only be incremented for rect block moves, not for AMR `RowBlockMove`s.   
- **Impact**:  
  - Consumers relying on these metrics will see misleading data (e.g., zero cells compared, zero anchors) even in nontrivial diffs.  
  - This reduces the value of the `perf-metrics` feature and makes automated threshold checking unreliable.

---

### 9. Activity log / intentional spec deviations are missing

- **Severity**: Moderate  
- **Category**: Gap (Process / Documentation)  
- **Description**:  
  The cycle summary notes that the `Activity Log` file is missing and that the `Intentional Spec Deviations` section is therefore absent. :contentReference[oaicite:33]{index=33}  
  At the same time, there are several non-trivial deviations from the mini-spec and unified spec (e.g., simplified AMR phase structure, partially implemented perf metrics, partial result handling) with no recorded rationale.
- **Impact**:  
  - Future reviewers and maintainers have no canonical explanation for why AMR was implemented in this simplified form or why certain plan items were deferred.  
  - This increases the risk that later changes either reintroduce old bugs or break the intended contract.

---

### 10. Documentation expectations for new alignment modules are not met

- **Severity**: Minor  
- **Category**: Gap  
- **Description**:  
  The Branch 2 plan states that each `alignment/` submodule should have doc comments referencing the relevant spec sections. :contentReference[oaicite:34]{index=34}  
  While some modules have basic comments, there is no systematic, spec-referenced documentation across `row_metadata`, `anchor_discovery`, `anchor_chain`, `gap_strategy`, `assembly`, or `runs`.   
- **Impact**:  
  - Makes it harder to map implementation details back to the formal spec (notable for a complex algorithm like AMR).  
  - Increases onboarding cost and the likelihood of misaligned future changes.

---

### 11. Miscellaneous nits and warnings

- **Severity**: Minor  
- **Category**: Gap  
- **Description**:  
  - Several functions in `row_alignment.rs` and `rect_block_move.rs` are now unused after AMR integration (`detect_exact_row_block_move`, `detect_fuzzy_row_block_move`, `align_row_changes`, etc.), and the compiler emits warnings.   
  - This is expected given AMR’s new path, but the code hasn’t been cleaned up or explicitly marked as legacy.  
- **Impact**:  
  - Increases code noise and cognitive load.  
  - Risk that legacy paths become stale and diverge from the rest of the implementation if they are ever re-used.

---

## Checklist Verification

- [ ] All scope items from mini-spec addressed  
  - Perf infra & CI gating, explicit large-grid tests, and some AMR phase details (global move-candidate extraction, move validation) are not fully implemented.   
- [ ] All specified tests created  
  - Multi-gap tests, large-grid/limit behavior tests, and perf fixtures/tests P3–P5 are missing.   
- [ ] Behavioral contract satisfied  
  - The contract for `LimitBehavior::ReturnPartialResult` is not satisfied; the engine may silently return an empty diff with no completeness flag.   
- [ ] No undocumented deviations from spec (documented deviations with rationale are acceptable)  
  - AMR phase structure and perf infra simplifications are undocumented due to missing activity log.   
- [ ] Error handling adequate  
  - Limit behavior is partially implemented and inconsistent across layers; no explicit partial-result/error reporting in `DiffReport`.   
- [X] No obvious performance regressions  
  - The AMR design (anchors + LIS + RLE) is asymptotically better than the previous monolithic LCS-style approach, and there are no obvious new quadratic loops or unbounded allocations in the code paths. However, this is not empirically validated due to missing perf tests and CI gating (see Findings 3 and 7).   

---

# Remediation Plan: 2025-12-09-sprint-branch-2

## Overview

This remediation plan focuses on fixing the most serious behavioral bug around limit-handling and partial results, completing the performance/metrics infrastructure promised by Branch 2, and shoring up the test suite around multi-gap alignment and large-grid limit behavior. It also calls for documenting intentional spec deviations in AMR’s phase structure and cleaning up minor documentation and legacy-code issues.

---

## Fixes Required

### Fix 1: Correct LimitBehavior::ReturnPartialResult semantics

- **Addresses Finding**: 1, 2, 5  
- **Changes**:
  - **Engine limit handling**  
    - In `core/src/engine.rs`, update `diff_grids_with_config`’s `exceeds_limits` branch so that `ReturnPartialResult` does not silently return an empty `ops` list. :contentReference[oaicite:43]{index=43}  
    - Options (choose one and document in the mini-spec):  
      1. **Conservative behavior**: Treat `ReturnPartialResult` the same as `ReturnError` for now (panic or error), and remove the config variant from the public API until full partial-result support is ready.  
      2. **Partial + flag**: Extend `DiffReport` to include `complete: bool` and `warnings: Vec<String>` (as per unified spec Section 24.8), and set `complete = false` and an explicit warning when limits are exceeded. Continue emitting whatever operations have already been computed.   
    - Remove or simplify the redundant limit handling in `align_rows_amr` (or ensure it mirrors the outer behavior and is only used in tests). :contentReference[oaicite:45]{index=45}  
  - **API surface**  
    - If `ReturnPartialResult` semantics change, update `DiffConfig` documentation (and any external docs) to clearly describe the new behavior. :contentReference[oaicite:46]{index=46}  
- **Tests**:
  - Add a dedicated test module (e.g., `core/tests/limit_behavior_tests.rs`) that:  
    - Constructs a grid pair above `max_align_rows` or `max_align_cols`.  
    - Asserts behavior for each `LimitBehavior` variant:  
      - `FallbackToPositional`: diffs computed via positional path; ops non-empty when there are real differences.  
      - `ReturnPartialResult`: behavior matches the chosen implementation (panic vs partial + flag).  
      - `ReturnError`: diff call fails as expected (panic or explicit error).  
  - Add fixture-based tests that use `DiffReport` and verify completeness flags when partial results are returned (if that option is chosen).

---

### Fix 2: Complete perf/metrics instrumentation and CI perf workflow

- **Addresses Finding**: 3, 7, 8  
- **Changes**:
  - **Metrics implementation** (core/src/perf.rs & engine):   
    - Wire up `parse_time_ms` by timing workbook parsing or `GridView::from_grid_with_config` construction (depending on where “parse” is defined in the spec).  
    - Increment `cells_compared` where cell comparisons happen (inside the core cell-diff loop).   
    - Track `anchors_found` when building the anchor chain (size of the chain returned by `build_anchor_chain`).   
    - Track `moves_detected` for all block-move types (rect, row, column) including AMR `RowBlockMove`s.   
    - Decide either to implement real memory tracking for `peak_memory_bytes` or to remove that field until Branch 4’s memory tracker is available; document the decision in the mini-spec.   
  - **Feature flag & serialization**  
    - Ensure `DiffReport`’s `metrics` field remains behind `#[cfg(feature = "perf-metrics")]` and is omitted from JSON when not enabled.   
  - **Perf fixtures & CI**  
    - Extend `fixtures/excel_diff_test_matrix.yaml` to include the missing perf scenarios (`p3_adversarial_repetitive`, `p4_99_percent_blank`, `p5_nested_tables`) per the Branch 2 plan.   
    - Add a small perf harness (Rust binary or Python script) that:  
      - Loads these fixtures.  
      - Runs the diff with `perf-metrics` enabled.  
      - Asserts basic thresholds on `DiffMetrics` (e.g., runtime < X ms, rows_processed == expected, etc.).  
    - Add a `.github/workflows/perf.yml` or equivalent CI job that runs the perf harness on a reduced subset of fixtures (e.g., P1, P3, P4) with conservative thresholds to catch major regressions.

- **Tests**:
  - Unit tests for `DiffMetrics` that simulate `start_phase`/`end_phase` and verify fields update as expected.  
  - Integration tests that enable `perf-metrics`, run a small diff, and assert that metrics are present in serialized JSON and contain non-zero values for relevant fields.  

---

### Fix 3: Add explicit multi-gap AMR tests

- **Addresses Finding**: 4  
- **Changes**:
  - Add a new integration test module (e.g., `core/tests/amr_multi_gap_tests.rs`) that constructs synthetic grids with controlled row patterns to force specific AMR behavior:  
    1. **Two disjoint insertion regions**:  
       - Example: A has [Header, A, B, C, Footer], B has [Header, X, Y, A, B, C, Z, Footer].  
       - Validate that the diff reports two insert ranges and aligns the rest correctly.  
    2. **Insertion + deletion in different regions**:  
       - Example: A has extra rows in the middle, B has extra rows near the tail.  
       - Validate that both insert and delete gaps are correctly handled and anchors remain intact.  
    3. **Gap containing a moved block**:  
       - Example from the spec (Gamma/Delta/Epsilon moved). :contentReference[oaicite:54]{index=54}  
       - Verify that AMR identifies a `RowBlockMove` (or equivalent) rather than treating the moved block as delete+insert noise.  
  - Where possible, design the tests to hit the `RecursiveAlign` and `MoveCandidate` branches in `GapStrategy`/`fill_gap`.   
- **Tests**:
  - Each scenario should assert on the final `RowAlignment` (matched pairs, inserted/deleted rows, and moves) rather than just on `DiffOp` counts, to directly validate AMR internals.

---

### Fix 4: Implement and test large-grid limit behavior

- **Addresses Finding**: 5  
- **Changes**:
  - Add helper functions in tests to construct large synthetic grids (e.g., 50K rows, 100 columns) with simple content patterns to avoid heavy formula/value overhead.  
  - Add tests that:  
    - Configure `max_align_rows` large enough (e.g., 100_000) and verify a 50K-row grid diff completes without hitting limits.  
    - Configure `max_align_rows` smaller (e.g., 10_000) and verify that limit behavior triggers as expected for each `LimitBehavior`.  
    - Similarly exercise `max_align_cols` with 500+ column grids.   
- **Tests**:
  - Integrate these into the same `limit_behavior_tests` module as Fix 1, to avoid duplication.

---

### Fix 5: Document AMR phase simplifications & align with spec

- **Addresses Finding**: 6, 9, 10  
- **Changes**:
  - Update the (currently missing) cycle-level Activity Log / Intentional Spec Deviations to explicitly document:   
    - That a full global move-candidate extraction phase (spec sections 9.5–9.7, 11) is not implemented; instead, move detection is localized to gap filling via `GapStrategy::MoveCandidate`.   
    - Why this is acceptable for now (e.g., complexity/time tradeoff) and what failure modes we accept (some multi-block moves modeled as insert/delete).  
  - Add module-level doc comments in each `alignment/` file referencing the corresponding spec sections (e.g., `row_metadata.rs` → Section 9.11; `anchor_discovery.rs`/`anchor_chain.rs` → Section 10; `gap_strategy.rs`/`assembly.rs` → Sections 9.6/12).   
  - Optionally, add TODOs at the top of `move_extraction.rs` describing the future work required to implement full global move-candidate extraction and validation matching the spec.

- **Tests**:
  - None required beyond correctness; this is primarily documentation/clarity work.

---

### Fix 6: Clean up legacy row-alignment paths and unused functions

- **Addresses Finding**: 11  
- **Changes**:
  - Explicitly mark `row_alignment.rs` as “legacy” and, if still needed for specific scenarios, document these scenarios; otherwise, delete unused functions (`detect_exact_row_block_move`, `detect_fuzzy_row_block_move`, `align_row_changes`, etc.) or gate them behind a feature flag.   
  - Remove or refactor any dead code in `rect_block_move.rs` that is no longer used.  
- **Tests**:
  - Ensure existing move-related tests (g10–g14) still pass after cleanup, verifying that AMR and the newer block-move paths are the ones used in core flows.

---

## Constraints

- **Backward compatibility**:  
  - Any change to `DiffReport`’s serialized shape (e.g., adding `complete` or `warnings` fields) must be backwards compatible with existing consumers, likely by adding optional fields with sensible defaults.   
- **Runtime limits**:  
  - Large-grid tests (50K rows) and perf fixtures must be designed carefully to avoid exceeding CI time limits. Consider scaled-down versions for CI and full-scale tests for local perf measurements.  
- **WASM environments**:  
  - Memory tracking (`peak_memory_bytes`) and perf tests should be mindful of WASM memory ceilings; heavy memory instrumentation may need to wait for Branch 4’s dedicated memory work.   

---

## Expected Outcome

After completing this remediation work:

- `DiffConfig::on_limit_exceeded` will have well-defined, correctly implemented semantics, with no silent “empty diff” behavior for `ReturnPartialResult`.  
- `DiffReport` (and/or an associated error/flag mechanism) will clearly indicate when results are partial, aligning with the unified spec’s result completeness model.   
- Perf metrics will be accurate enough to support regression detection, and a basic CI perf workflow will guard against major runtime/memory regressions on large and adversarial workloads.   
- AMR’s multi-gap behavior will be directly tested in scenarios outlined by the Branch 2 plan, increasing confidence in correctness for complex real-world diffs.   
- Intentional deviations from the full AMR spec will be documented, making the implementation easier to reason about and evolve in future branches.