# Remediation Plan: 2025-12-02-row-signatures-v1

## Overview

The current implementation is release-ready: all prior remediation findings are addressed, and the RS1 behavioral contract is well satisfied and well tested. The items below are **non-blocking hardening tasks** to reduce future confusion and lock in behavior for edge cases.

## Fixes Required

### Fix 1: Align unified spec text with RS1 commutative hashing model

- **Addresses Finding**: Finding 1 – Unified spec still describes the old “sorted sequence” hashing model
- **Changes**:
  - **Documentation only**; no code changes.
  - In `docs/rust_docs/unified_grid_diff_algorithm_specification.md`, Section 5.4 (“Dual Hash System” / Hash64 implementation):  
    - Update the narrative and pseudocode to make it clear that:
      - Per-cell contributions are computed from `(position, type, value, formula)` via XXHash64.   
      - The **combination step** for row and column hashes is a commutative reduction over these per-cell contributions (e.g., additive mix of a per-cell `mix_hash`), making the final hash independent of iteration order while still sensitive to which positions are present.
    - Add a short “Implementation Note” referencing the RS1 milestone:
      - Clarify that the sparse `Grid` layer uses a commutative combining function so hashing can stream over `HashMap`-backed cells without sorting.
      - Mention that higher-level `GridView` structures may still choose to materialize sorted row/column views for other algorithms, but **the fingerprint semantics are defined in terms of the commutative reduction**, not the iteration order.
  - Optionally, in `docs/rust_docs/2025-11-30-docs-vs-implementation.md`, add a brief bullet under Preprocessing/Hashing that explicitly points out:
    - “Implementation uses commutative mixing over per-cell XXHash64 contributions; sorted iteration is no longer required for determinism.”
- **Tests**:
  - No new runtime tests required.
  - As a light guardrail, consider adding a short “doc sanity” comment or checklist item in the RS1 or hashing section (e.g., a note in the spec’s revision history) so future reviewers can confirm that commutative mixing is intentional.

---

### Fix 2: Add explicit tests for degenerate/empty grids and all-empty rows/columns

- **Addresses Finding**: Finding 2 – No explicit tests for degenerate/empty grids and all-empty rows/columns signatures
- **Changes**:
  - In `core/tests/signature_tests.rs`:
    1. **Test: `compute_all_signatures_on_empty_grid_produces_empty_vectors`**  
       - Construct `let mut grid = Grid::new(0, 0);`  
       - Call `grid.compute_all_signatures();`  
       - Assert:
         - `grid.row_signatures.is_some()` and `grid.col_signatures.is_some()`.  
         - `grid.row_signatures.as_ref().unwrap().is_empty()` and ditto for columns.
    2. **Test: `compute_all_signatures_with_all_empty_rows_and_cols_is_stable`**  
       - Construct `let mut grid = Grid::new(3, 4);` with **no inserted cells**.  
       - Call `grid.compute_all_signatures();`  
       - Assert:
         - `row_signatures.len() == 3`, `col_signatures.len() == 4`.  
         - Every `RowSignature.hash` and `ColSignature.hash` is `0`.  
       - Optionally, call `compute_all_signatures()` a second time and assert the same vectors remain (idempotence).
  - In `core/tests/sparse_grid_tests.rs` (optional but helpful for documenting sparse semantics):
    - Add a small test like `sparse_grid_all_empty_rows_have_zero_signatures`:
      - Create a sparse grid with `nrows > 0` and `ncols > 0` but no cells.
      - Use `compute_all_signatures` and assert all row/col signature hashes are zero, reaffirming that empties are ignored.
- **Tests**:
  - Run the existing suite:
    - `cargo test` (all targets)  
    - Confirm the new tests pass and do not materially change timings for the existing suite.

## Constraints

- These changes are non-invasive:
  - No changes to the public API (`DiffOp`, `DiffReport`, `Grid`, etc.).
  - No changes to hashing behavior; tests only **codify** current behavior.
  - Docs change only clarifies what is already implemented and relied on by RS1.
- Work can be scheduled in a later hardening cycle and does not need to block this release.

## Expected Outcome

After remediation:

- The unified algorithm spec, RS1 mini-spec, and implementation will all consistently describe the commutative XXHash64-based row/column fingerprinting model, eliminating ambiguity for future contributors.
- Degenerate/empty grid behaviors will be explicitly covered by tests:
  - 0×0 grids produce empty signature vectors without panicking.
  - Grids with only empty rows/columns produce zeros for those signatures in a stable, repeatable way.
- Future refactors in the grid/signature layer will be less likely to accidentally regress edge-case behavior or reintroduce unnecessary sorting/cloning, while preserving the strong, well-tested fingerprint semantics needed for later alignment phases.
