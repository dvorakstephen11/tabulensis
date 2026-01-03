# Verification Report: 2025-12-02-row-signatures-v1

## Summary

The implementation successfully introduces `RowSignature` and `ColSignature` backed by a deterministic `XXHash64` implementation. The signatures correctly incorporate column/row position and value type, satisfying the core behavioral requirements of the milestone. The test suite has been expanded with the specified cases, including a golden hash test. However, a specific memory constraint from the mini-spec was violated in `Grid::compute_all_signatures`, which currently clones every cell in the grid (deep copy) rather than iterating references. This creates unnecessary heap allocations that will degrade performance on large datasets.

## Recommendation

[ ] Proceed to release
[x] Remediation required

## Findings

### [Constraint Violation: Unnecessary Allocations in `compute_all_signatures`]
- **Severity**: Moderate
- **Category**: Spec Deviation / Performance
- **Description**: The mini-spec explicitly stated: "No new per-cell allocations during hashing... Use existing Cell structures and iterate them." The implementation of `Grid::compute_all_signatures` performs `self.cells.values().cloned().collect()`. Since `Cell` contains `Option<String>` fields (text values and formulas), this triggers a deep copy of the entire grid's text content onto the heap during signature computation.
- **Evidence**: `core/src/workbook.rs`: `let mut cells: Vec<Cell> = self.cells.values().cloned().collect();`
- **Impact**: Doubles memory usage during hashing for text-heavy grids and increases GC/allocator pressure, threatening the "100MB instant diff" performance goal.

## Checklist Verification

- [x] All scope items from mini-spec addressed
- [x] All specified tests created
- [x] Behavioral contract satisfied (Logic is correct)
- [ ] No undocumented deviations from spec (Memory constraint violated)
- [x] Error handling adequate
- [x] No obvious performance regressions (Except memory spike noted above)

---

# Remediation Plan: 2025-12-02-row-signatures-v1

## Overview

Refactor `Grid::compute_all_signatures` to iterate over cell references (`&Cell`) instead of cloning the cells. This brings the implementation in line with the memory constraints defined in the mini-spec.

## Fixes Required

### Fix 1: Remove Cloning in `compute_all_signatures`
- **Addresses Finding**: Constraint Violation: Unnecessary Allocations in `compute_all_signatures`
- **Changes**: 
    - In `core/src/workbook.rs`, modify `compute_all_signatures`.
    - Change `let mut cells: Vec<Cell> = self.cells.values().cloned().collect();` to `let mut cells: Vec<&Cell> = self.cells.values().collect();`.
    - Ensure the sorting closures and `hash_cell_with_position` calls work with `&&Cell` or `&Cell` as appropriate (dereferencing may be needed for the sort comparison, though `hash_cell_with_position` already takes `&Cell`).
- **Tests**: 
    - Existing tests in `signature_tests.rs` and `sparse_grid_tests.rs` must continue to pass.
    - No new tests strictly required as this is a refactor of internals, but existing coverage guards the logic.

## Constraints

- Ensure the deterministic order of processing is maintained (sorting logic must remain identical).
- Do not change the logic of `compute_row_signature` or `compute_col_signature`, which already correctly use references.

## Expected Outcome

`compute_all_signatures` will execute with `O(1)` additional heap allocation for cell data (only the `Vec` of pointers is allocated), satisfying the milestone's memory efficiency constraints.