# Activity Log: Branch 2 - Grid Algorithm Scalability (AMR) + Performance Infrastructure

**Date**: 2025-12-09
**Branch**: Branch 2 (Grid Scalability + Perf Infra)
**Status**: Complete

---

## Summary

Branch 2 implements the Anchor-Move-Refine (AMR) row alignment algorithm and establishes performance regression testing infrastructure. This log documents the implementation status relative to the unified grid diff specification and the sprint plan.

---

## Fully Implemented Components

### 1. AMR Algorithm Core (Spec Sections 9-12)

| Module | File | Description |
|--------|------|-------------|
| Row Metadata | `alignment/row_metadata.rs` | FrequencyClass enum (Unique/Rare/Common/LowInfo), row signature classification |
| Anchor Discovery | `alignment/anchor_discovery.rs` | Discovers rows unique in both grids with matching signatures |
| Anchor Chain | `alignment/anchor_chain.rs` | LIS-based anchor chain construction preserving relative order |
| Gap Strategy | `alignment/gap_strategy.rs` | Strategy selection: Empty, InsertAll, DeleteAll, SmallEdit, MoveCandidate, RecursiveAlign |
| Assembly | `alignment/assembly.rs` | Final alignment assembly from gaps and anchors |
| RLE Compression | `alignment/runs.rs` | Run-length encoding for repetitive row patterns |
| Move Extraction | `alignment/move_extraction.rs` | Block move detection within gaps |

### 2. Configuration Infrastructure (Branch 3 prerequisite)

- `DiffConfig` with all alignment thresholds
- `LimitBehavior` enum: FallbackToPositional, ReturnPartialResult, ReturnError
- Configurable limits: `max_align_rows` (500,000), `max_align_cols` (16,384)

### 3. Performance Metrics (`perf.rs`)

```rust
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

All fields are populated during diff operations when `perf-metrics` feature is enabled.

### 4. Performance Testing Infrastructure

- **Fixtures**: P1-P5 defined in `fixtures/manifest.yaml`
- **Tests**: `tests/perf_large_grid_tests.rs` with P1-P5 scenarios
- **CI**: `.github/workflows/perf.yml` runs perf tests
- **Thresholds**: `scripts/check_perf_thresholds.py` validates performance

### 5. Limit Handling

Single source of truth in `engine.rs`:
- Checks limits before calling alignment
- Handles all three `LimitBehavior` variants
- Produces appropriate warnings for `ReturnPartialResult`
- Returns structured `DiffError::LimitsExceeded` for `ReturnError`

### 6. JSON/Output Semantics

- `DiffReport.complete` field indicates full vs partial result
- `DiffReport.warnings` contains limit-exceeded messages
- All serialization tests pass for partial results and metrics

---

## Intentional Spec Deviations

### 1. No Global Move-Candidate Extraction Phase

**Spec Reference**: Sections 9.5-9.7, 11

**Full Spec Behavior**: The specification describes a global phase that extracts all out-of-order matches before gap filling, builds candidate moves from these matches, and validates them to resolve conflicts.

**Implementation**: Move detection is localized within gaps via `GapStrategy::MoveCandidate`. When a gap contains matching unique signatures, `find_block_move` scans for the largest contiguous block.

**Rationale**: The localized approach is simpler and handles most real-world Excel workbooks effectively. Complex multi-block reordering scenarios (e.g., A-B-C → C-A-B with all blocks having identical signatures in both halves) may not be optimally detected, but such cases are rare in practice.

### 2. No Explicit Move Validation Phase

**Spec Reference**: Section 11

**Full Spec Behavior**: After extracting move candidates, the spec describes a validation phase that resolves overlapping or conflicting moves.

**Implementation**: The first valid move found within each gap is accepted. Since gaps are processed sequentially between anchors, and each gap's move detection is independent, conflicts across gaps don't occur.

### 3. RLE Fast Path

**Spec Reference**: Section 2.6 (optional optimization)

**Implementation**: For highly repetitive grids (>50% compression ratio), `align_runs_stable` bypasses full AMR and aligns at the run level. This provides significant speedup for template-based workbooks with many identical rows.

### 4. Deferred Metrics: `parse_time_ms` and `peak_memory_bytes`

**Spec Reference**: Branch 2 plan Section 2.7

**Status**: These fields are not included in the current `DiffMetrics` struct.

**Rationale**: 
- `parse_time_ms` requires wrapping the parser infrastructure, which is outside the core diff engine
- `peak_memory_bytes` requires memory allocator integration (e.g., tikv-jemallocator)

Both are planned for future phases when the necessary infrastructure is ready.

---

## Test Coverage

| Test Category | File | Status |
|---------------|------|--------|
| Multi-gap AMR | `tests/amr_multi_gap_tests.rs` | ✓ Pass |
| RLE compression | `alignment/runs.rs` (unit tests) | ✓ Pass |
| Limit behavior | `tests/limit_behavior_tests.rs` | ✓ Pass |
| JSON/metrics output | `tests/output_tests.rs` | ✓ Pass |
| Performance | `tests/perf_large_grid_tests.rs` | ✓ Pass |
| Assembly | `alignment/assembly.rs` (unit tests) | ✓ Pass |

---

## Performance Fixture Summary

| Fixture | Rows | Cols | Description |
|---------|------|------|-------------|
| P1 | 50,000 | 100 | Large dense grid |
| P2 | 50,000 | 100 | Large noise grid (random floats) |
| P3 | 50,000 | 50 | Adversarial repetitive (100-row pattern) |
| P4 | 50,000 | 100 | 99% blank (sparse) |
| P5 | 50,000 | 100 | Identical grids (fast path) |

Note: Rust tests use smaller grids (1000 rows) for CI speed while still validating the algorithms.

---

## References

- `docs/rust_docs/next_sprint_plan.md` - Branch 2 specification
- `docs/rust_docs/unified_grid_diff_algorithm_specification.md` - Full AMR spec
- `docs/rust_docs/excel_diff_specification.md` - DiffReport, DiffMetrics, LimitBehavior docs

---

## Legacy Code Status

The `row_alignment.rs` module contains pre-AMR alignment code. It is retained for:
1. Fallback when AMR cannot produce useful alignment
2. Move detection helpers used by masked move detection
3. Test coverage

Functions are marked with `#[allow(dead_code)]` where not used in production paths.

