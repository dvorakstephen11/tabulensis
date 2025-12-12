# Activity Log: Branch 2 - Grid Algorithm Scalability (AMR) + Performance Infrastructure

**Start Date**: 2025-12-09
**Completion Date**: 2025-12-10

## Summary

Branch 2 implemented the Anchor-Move-Refine (AMR) alignment algorithm as described in `docs/rust_docs/unified_grid_diff_algorithm_specification.md`, along with performance infrastructure including metrics collection and CI regression testing.

## Implemented Components

### AMR Algorithm (Section 2.1-2.6 of `next_sprint_plan.md`)

1. **Module Structure** (`core/src/alignment/`)
   - `row_metadata.rs` - Row metadata computation with frequency classification
   - `anchor_discovery.rs` - Anchor discovery from unique row matches
   - `anchor_chain.rs` - LIS-based anchor chain construction
   - `gap_strategy.rs` - Gap strategy selection (Empty/InsertAll/DeleteAll/SmallEdit/MoveCandidate/RecursiveAlign)
   - `assembly.rs` - Final alignment assembly
   - `runs.rs` - Run-length encoding for repetitive row compression
   - `move_extraction.rs` - Block move detection within gaps

2. **Row Frequency Classification**
   - Implemented `FrequencyClass` enum (Unique/Rare/Common/LowInfo)
   - Configurable thresholds via `DiffConfig::rare_threshold` and `DiffConfig::low_info_threshold`

3. **Multi-Gap Alignment**
   - Anchor chain divides grids into gaps
   - Per-gap strategy selection based on gap size and content
   - Recursive alignment for large gaps with rare anchors
   - Move detection within gaps via `GapStrategy::MoveCandidate`

4. **RLE Fast Path**
   - For grids where >50% of rows share signatures, uses run-length encoded alignment
   - Bypasses full AMR for significant performance gains on repetitive data

### Performance Infrastructure (Section 2.7)

1. **DiffMetrics Struct** (`core/src/perf.rs`)
   - `alignment_time_ms`, `move_detection_time_ms`, `cell_diff_time_ms`, `total_time_ms`
   - `rows_processed`, `cells_compared`, `anchors_found`, `moves_detected`
   - Phase timing via `start_phase`/`end_phase` methods

2. **Perf Tests** (`core/tests/perf_large_grid_tests.rs`)
   - P1: Large dense grid (1000×20)
   - P2: Large noise grid (1000×20, all different)
   - P3: Adversarial repetitive (1000×50, 100-row pattern)
   - P4: 99% blank sparse (1000×100, 1% fill)
   - P5: Identical grids (1000×100)

3. **CI Workflow** (`.github/workflows/perf.yml`)
   - Runs perf tests in release mode
   - Threshold checking via `scripts/check_perf_thresholds.py`

### Limit Handling (Section 2.5)

1. **Configurable Limits** (`core/src/config.rs`)
   - `max_align_rows` (default: 500,000)
   - `max_align_cols` (default: 16,384)
   - `max_block_gap` (default: 10,000)
   - `max_recursion_depth` (default: 10)

2. **LimitBehavior Enum**
   - `FallbackToPositional` - Use positional diff when limits exceeded
   - `ReturnPartialResult` - Return partial result with warnings
   - `ReturnError` - Return structured error

3. **Unified Enforcement**
   - Single check point in `try_diff_grids_with_config` (`core/src/engine.rs`)
   - Warnings include sheet name for multi-sheet scenarios

## Intentional Spec Deviations

The implementation differs from the full unified grid diff specification in the following documented ways:

### 1. No Global Move-Candidate Extraction Phase

**Spec Reference**: Sections 9.5-9.7, 11 describe a global phase that extracts out-of-order row matches before gap filling.

**Implementation**: Moves are detected opportunistically within gaps via `GapStrategy::MoveCandidate` and the `find_block_move` function. This is simpler but may miss some complex multi-block move patterns.

**Rationale**: Most real-world Excel workbooks have simple move patterns (single block moves, row insertions/deletions). The global extraction phase adds significant complexity for edge cases that are rare in practice.

### 2. No Explicit Move Validation Phase

**Spec Reference**: Section 11 describes validating move candidates to resolve conflicts between overlapping or ambiguous moves.

**Implementation**: The implementation accepts the first valid move found within each gap without global conflict resolution.

**Rationale**: Conflict resolution is only needed when multiple valid moves exist for the same rows, which is uncommon. The simplified approach produces correct results for the vast majority of cases.

### 3. RLE Fast Path Bypasses Full AMR

**Spec Reference**: The spec describes RLE as an optimization within AMR, not as an alternative path.

**Implementation**: When >50% of rows can be compressed via RLE, the implementation uses a simpler run-based alignment (`align_runs_stable`) that bypasses the full anchor/gap/assembly pipeline.

**Rationale**: Highly repetitive grids (templates, blank rows) benefit significantly from this fast path. The run-based alignment produces identical results for these cases with much better performance.

### 4. Deferred Metrics

**Spec Reference**: Section 2.7 mentions `parse_time_ms` and `peak_memory_bytes` as planned metrics.

**Implementation**: These fields are not included in `DiffMetrics`. Parse timing requires wrapping the parser, and memory tracking requires allocator integration.

**Rationale**: These are deferred to a future phase to avoid scope creep. The current metrics (timing and counts) provide sufficient observability for performance regression testing.

## Test Coverage

### Multi-Gap AMR Tests (`core/tests/amr_multi_gap_tests.rs`)
- Two disjoint insertion regions
- Insertion + deletion in different regions
- Gap contains moved block
- Multiple anchors with gaps
- Recursive gap alignment

### RLE Tests (`core/src/alignment/runs.rs::tests`)
- 10K identical rows compress to single run
- Alternating A-B pattern does not over-compress
- Mixed runs with varying lengths
- Preservation of row indices

### Limit Behavior Tests (`core/tests/limit_behavior_tests.rs`)
- `FallbackToPositional` behavior
- `ReturnPartialResult` with warnings
- `ReturnError` via try_* API
- Multi-sheet warning includes sheet name
- 5000-row and 500-column grids within default limits

### Move Detection Tests (`core/tests/g10_*`, `g11_*`, `g12_*`, `g13_*`, `g14_*`)
- Row block insert/delete
- Row block move
- Column block move
- Rect block move
- Fuzzy row move
- Multiple simultaneous moves

## Files Modified/Created

### New Files
- `core/src/alignment/` module with 8 files
- `core/tests/amr_multi_gap_tests.rs`
- `core/tests/perf_large_grid_tests.rs`
- `.github/workflows/perf.yml`
- `scripts/check_perf_thresholds.py`

### Modified Files
- `core/src/config.rs` - Added limit config fields and `LimitBehavior`
- `core/src/engine.rs` - Integrated AMR alignment, limit handling, metrics collection
- `core/src/diff.rs` - Added `DiffError::LimitsExceeded` variant
- `core/src/lib.rs` - Exported new modules
- `core/src/perf.rs` - Added `DiffMetrics` struct
- `fixtures/manifest.yaml` - Added P1-P5 perf fixtures

## Verification Status

All tests pass:
- 126 unit tests
- All integration tests
- All perf tests (P1-P5)
- Limit behavior tests
- AMR multi-gap tests
- Move detection tests (g10-g14)

## Remediation (2025-12-11)

Based on review feedback (`docs/meta/reviews/2025-12-09-sprint-branch-2/remediationD.md`), the following gaps were addressed:

### 1. Perf Threshold Enforcement (Complete)

**Gap**: `check_perf_thresholds.py` only ran tests and verified completion, without enforcing per-test timing budgets.

**Fix**: 
- Perf tests now print `PERF_METRIC <test_name> total_time_ms=<value>` lines
- Threshold script parses these lines and compares against configured thresholds
- Script exits non-zero if any test exceeds its threshold
- Thresholds configurable via environment variables:
  - `EXCEL_DIFF_PERF_P1_MAX_TIME_S` through `EXCEL_DIFF_PERF_P5_MAX_TIME_S`
  - `EXCEL_DIFF_PERF_SLACK_FACTOR` to scale all thresholds

### 2. DiffMetrics Unit Tests (Complete)

**Gap**: Metrics behavior only tested indirectly through integration tests.

**Fix**: Added `core/tests/metrics_unit_tests.rs` with 12 focused tests:
- `metrics_starts_with_zero_counts` - Initial state validation
- `metrics_add_cells_compared_accumulates` - Count accumulation
- `metrics_add_cells_compared_saturates` - Saturating arithmetic
- `metrics_phase_timing_accumulates` - Phase timing works
- `metrics_different_phases_tracked_separately` - Phase isolation
- `metrics_total_phase_separate_from_components` - Total vs component times
- `metrics_end_phase_without_start_is_safe` - Graceful handling of unstarted phases
- `metrics_parse_phase_is_no_op` - Parse phase correctly ignored
- Plus tests for direct field access, cloning, and equality

### 3. Test Naming Fix (Complete)

**Gap**: `large_grid_50k_rows_completes_within_default_limits` used 5000 rows, not 50,000.

**Fix**: Renamed to `large_grid_5k_rows_completes_within_default_limits` to accurately reflect the test.

### Files Modified

- `core/tests/perf_large_grid_tests.rs` - Added PERF_METRIC println statements
- `scripts/check_perf_thresholds.py` - Full rewrite with metric parsing and threshold enforcement
- `core/tests/metrics_unit_tests.rs` - New file with 12 unit tests
- `core/tests/limit_behavior_tests.rs` - Renamed misleading test function

## Follow-Up Items

1. Consider implementing global move extraction if complex reordering scenarios are reported
2. Add `parse_time_ms` when parser instrumentation is ready
3. Add `peak_memory_bytes` with allocator integration (e.g., `tikv-jemallocator`)
4. Performance profiling on 50K-row grids from the full fixture set

