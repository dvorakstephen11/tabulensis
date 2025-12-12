# Algorithm Efficiency Fix - Related Files

**Date**: 2025-12-11  
**Reference**: [algorithm_efficiency.md](./algorithm_efficiency.md)

---

## Core Algorithm Files (Must Modify)

These files contain the O(n²) bottleneck and must be modified to fix the performance issue:

### Primary Changes

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `core/src/alignment/assembly.rs` | Main AMR orchestration, `align_rows_amr()`, `fill_gap()`, **`align_small_gap()` (THE BOTTLENECK)** | Add identical grid fast path, add `hash_based_gap_alignment()`, add `positional_gap_alignment()`, fix gap filling dispatch |
| `core/src/alignment/gap_strategy.rs` | Gap strategy selection, `select_gap_strategy()` | Add hard cap constant, add `PositionalFallback` variant, fix fallback logic |

### Secondary Changes

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `core/src/config.rs` | `DiffConfig` with thresholds | Possibly add `max_lcs_gap_size` config (or keep as hard constant) |
| `core/src/alignment/mod.rs` | Module exports | Export new types/functions if needed |

---

## Supporting Algorithm Files (May Need Minor Updates)

These files support the alignment algorithm and may need adjustments:

| File | Purpose | Potential Changes |
|------|---------|-------------------|
| `core/src/alignment/anchor_discovery.rs` | `discover_anchors_from_meta()` - finds unique matching rows | None expected |
| `core/src/alignment/anchor_chain.rs` | `build_anchor_chain()` - LIS algorithm for anchor selection | None expected |
| `core/src/alignment/row_metadata.rs` | `RowMeta`, `FrequencyClass`, `classify_row_frequencies()` | None expected |
| `core/src/alignment/runs.rs` | `compress_to_runs()`, `RowRun` - RLE compression | None expected |
| `core/src/alignment/move_extraction.rs` | `find_block_move()`, `moves_from_matched_pairs()` | None expected |

---

## Integration Files (May Need Updates)

These files integrate the alignment algorithm with the rest of the engine:

| File | Purpose | Potential Changes |
|------|---------|-------------------|
| `core/src/engine.rs` | Main `diff_grids_internal()` - calls `align_rows_amr()` | May need early-exit checks for identical grids |
| `core/src/grid_view.rs` | `GridView::from_grid_with_config()` - builds row metadata | None expected |
| `core/src/row_alignment.rs` | Legacy alignment (fallback) | None expected - used only when AMR returns None |

---

## Test Files (Must Add/Update)

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `core/tests/perf_large_grid_tests.rs` | 50k row performance tests | Already has tests; verify they pass after fix |
| `core/tests/amr_multi_gap_tests.rs` | AMR algorithm tests | Add tests for new strategies |
| `core/tests/limit_behavior_tests.rs` | Limit handling tests | Verify tests still pass |

---

## Full File List

### Must Modify (Alphabetical)

```
core/src/alignment/assembly.rs
core/src/alignment/gap_strategy.rs
```

### Should Review (Alphabetical)

```
core/src/alignment/anchor_chain.rs
core/src/alignment/anchor_discovery.rs
core/src/alignment/mod.rs
core/src/alignment/move_extraction.rs
core/src/alignment/row_metadata.rs
core/src/alignment/runs.rs
core/src/config.rs
core/src/engine.rs
core/src/grid_view.rs
core/src/row_alignment.rs
```

### Tests to Update

```
core/tests/amr_multi_gap_tests.rs
core/tests/limit_behavior_tests.rs
core/tests/perf_large_grid_tests.rs
```

---

## Dependency Graph

```
engine.rs
    │
    └── align_rows_amr() ─────────────────────────────────────┐
              │                                                │
              ▼                                                │
        assembly.rs                                            │
              │                                                │
    ┌─────────┼─────────┬─────────────┐                       │
    ▼         ▼         ▼             ▼                       │
anchor_   anchor_   gap_         runs.rs                      │
discovery chain     strategy                                  │
    │         │         │                                     │
    └────┬────┘         │                                     │
         │              │                                     │
         ▼              ▼                                     │
    row_metadata.rs   fill_gap()                              │
                        │                                     │
            ┌───────────┼───────────┐                        │
            ▼           ▼           ▼                        │
       align_small_   move_      positional_                 │
       gap() ← O(n²)  extraction  gap_alignment() ← NEW O(n) │
            │                                                │
            └────────────────────────────────────────────────┘
```

The fix targets `gap_strategy.rs` (prevent calling O(n²) path) and `assembly.rs` (add O(n) alternatives).

---

## Lines of Code Estimate

| File | Current LOC | Estimated Change |
|------|-------------|------------------|
| `assembly.rs` | 527 | +80 (new functions) |
| `gap_strategy.rs` | 77 | +20 (new strategy, hard cap) |
| **Total** | | **~100 lines added** |

The fix is surgical - we're adding new fast paths, not rewriting existing logic.

