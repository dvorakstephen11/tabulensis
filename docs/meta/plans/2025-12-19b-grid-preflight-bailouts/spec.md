
# Mini-Spec: Grid preflight bailouts for P1/P2 (move detection + AMR short-circuit)

## 0. Summary

Add a **cheap, deterministic preflight** in the grid diff hot path that can **short-circuit** to positional diff (skip masked move detection and AMR/row alignment) when:

1. grids are **nearly identical in-order** (small number of row-signature mismatches), or
2. grids are **extremely dissimilar** (low signature overlap / similarity), making alignment and move detection predictably low-value.

This is Phase 5 performance milestone progress for P1/P2. 

## 1. Scope

### In-scope modules / types

* `core/src/engine/grid_diff.rs`: introduce preflight decision point before move detection begins. 
* `core/src/engine/move_mask.rs` (`SheetGridDiffer`): expose any small helper accessors if needed (e.g., row signature slices / counts). 
* `core/src/config.rs`: add explicit config knobs for bailout thresholds (no hard-coded constants). 
* `core/tests/perf_large_grid_tests.rs`: extend existing perf tests and add deterministic “preflight triggers” tests. 

### Out of scope (explicitly not this cycle)

* Changing `DiffOp` schema to add “sheet replaced” summary ops.
* Reworking `Grid` storage (HashMap vs dense) or introducing chunked cell diff.
* Making the preflight depend on wall-clock timing assertions.

## 2. Behavioral Contract

### 2.1 Near-identical dense sheet (P1-like)

Given two same-shape dense grids where only a few cells change:

* Output contains only the expected `CellEdited` ops (no row/column structural ops, no move ops).
* Engine **skips** masked move detection and AMR alignment.
* With `perf-metrics`, `move_detection_time_ms == 0` and `alignment_time_ms == 0`. (Cell diff may still take time; that’s expected.)

Example:

* A: 1000×50 deterministic numbers
* B: identical except one cell changed
* Expect: exactly one `CellEdited` (plus existing formula-diff semantics if applicable), and move/alignment phases not entered.

### 2.2 Completely different same-shape sheet (P2-like)

Given two same-shape dense grids with different pseudo-random content:

* Output is primarily `CellEdited` (as today).
* Engine skips masked move detection and alignment (they are predictably low-value when similarity is below threshold).
* With `perf-metrics`, `move_detection_time_ms == 0` and `alignment_time_ms == 0`.

### 2.3 Exact move cases must NOT be skipped

Given a case where content is the same multiset but reordered (e.g., an exact row-block move):

* Preflight must not short-circuit.
* Existing behavior (emitting `BlockMovedRows` / `BlockMovedColumns` / rect moves where applicable) must remain intact.

## 3. Constraints / Invariants

* **Determinism**: preflight decision must depend only on grid content-derived hashes/signatures and config, not HashMap iteration order. 
* **Complexity**:

  * Preflight computation must be `O(R)` to `O(R log R)` in number of rows (and/or columns), with bounded memory.
  * Must prevent accidentally adding any `O(R²)` path before bail-out triggers.
* **No behavior regression**: For cases where moves/alignment actually matter (multiset-equal reorders, structural changes), the preflight must not disable the existing pipeline.
* **Placement**: preflight must occur **before** `Phase::MoveDetection` begins so perf metrics reflect “skipped” as `0ms` rather than “fast but nonzero”.

## 4. Interfaces

### 4.1 DiffConfig additions (public API)

Add explicit fields (names can be adjusted to match existing config style):

* `bailout_similarity_threshold: f64` (default `0.05` per unified spec config guidance) 
* `preflight_in_order_mismatch_max: u32` (default `32` or `64`)
* `preflight_in_order_match_ratio_min: f64` (default e.g. `0.995`)
* `preflight_min_rows: u32` (default e.g. `5000` to avoid spending effort on tiny grids)

These fields must be:

* validated by the existing config validator (non-negative, sane ranges),
* supported in the builder API,
* backward compatible via serde defaults.

### 4.2 Internal helper (no public surface)

Introduce an internal function (location flexible) such as:

* `fn should_short_circuit_to_positional(old_view: &GridView, new_view: &GridView, cfg: &DiffConfig) -> bool`

Inputs should use existing row/col signatures already computed in `GridView` (no per-cell allocations). 

## 5. Implementation Notes (non-binding, but guiding)

Preflight decision logic (suggested):

1. Only consider short-circuit when:

   * `old.nrows == new.nrows` and `old.ncols == new.ncols` (same-shape), and
   * `old.nrows >= cfg.preflight_min_rows`
2. Compute:

   * `in_order_matches`: count of rows where `old_row_sig[i] == new_row_sig[i]`
   * `in_order_mismatches = nrows - in_order_matches`
   * `in_order_match_ratio = in_order_matches / nrows`
3. Compute a cheap similarity/overlap estimate using row signature sets (or counts):

   * `jaccard = |intersection| / |union|`
4. Short-circuit to positional when either:

   * **Near-identical edits**: `in_order_mismatches <= cfg.preflight_in_order_mismatch_max`
     AND `in_order_match_ratio >= cfg.preflight_in_order_match_ratio_min`
     AND `NOT multiset_equal` (to avoid skipping true reorder/move cases)
   * **Very dissimilar**: `jaccard < cfg.bailout_similarity_threshold`

Where “multiset_equal” can be computed using row signature frequency counts (not full alignment). This matches the spec’s emphasis on early bail-outs and avoiding wasted work. 

## 6. Test Plan

### 6.1 New tests (deterministic, not timing-based)

Add to `core/tests/perf_large_grid_tests.rs`:

1. `preflight_skips_move_and_alignment_for_single_cell_edit_same_shape`

   * Build 1000×50 dense grid A and B with 1 cell edit.
   * Assert:

     * report complete
     * exactly one `CellEdited`
     * `metrics.move_detection_time_ms == 0`
     * `metrics.alignment_time_ms == 0`

2. `preflight_skips_move_and_alignment_for_low_similarity_same_shape`

   * Use a smaller grid (e.g., 80×50) to avoid huge op vectors.
   * A uses base_value 0, B uses base_value 1 (all cells differ).
   * Assert:

     * report complete
     * `metrics.move_detection_time_ms == 0`
     * `metrics.alignment_time_ms == 0`
     * ops contain at least one `CellEdited` and no structural/move ops

3. `preflight_does_not_skip_when_multiset_equal_but_order_differs`

   * Construct same-shape grid B by moving a contiguous row block (no internal edits).
   * Assert presence of `DiffOp::BlockMovedRows` (or whatever existing move op is expected for that scenario).
   * This test is the guardrail against “high in-order match ratio” accidentally masking real moves.

### 6.2 Extend existing perf tests (ignored 50k)

In `core/tests/perf_large_grid_tests.rs`, extend the ignored long-running tests to encode the new guarantee without timing flakiness:

* `perf_50k_dense_single_edit`: assert `metrics.move_detection_time_ms == 0` and `metrics.alignment_time_ms == 0` in addition to existing correctness checks.
* `perf_50k_completely_different`: same assertions.
* `perf_50k_adversarial_repetitive`: same assertions.

Rationale: these particular synthetic cases contain no moves by construction; ensuring the engine doesn’t spend time “hunting for moves” is exactly the point of this milestone.

### 6.3 Config surface tests

Update/extend existing config tests (wherever `DiffConfig` defaults/builders are currently covered):

* new fields round-trip with serde defaults
* builder setters work
* validation rejects nonsense (e.g., negative thresholds, mismatch_max == 0 if disallowed)

## 7. Acceptance Criteria

* The latest full-scale benchmark suite shows large improvements specifically by eliminating wasted move detection + alignment time on “no moves” cases (dense single edit, completely different, adversarial repetitive). 
* All unit/integration tests remain green; move detection fixture tests (G11/G12 etc.) remain correct.
* Preflight logic is:

  * deterministic,
  * configurable via `DiffConfig`,
  * cheap relative to the work it avoids,
  * placed before metrics phases so “skipped” registers as `0ms`.
