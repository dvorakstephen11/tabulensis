# Remediation Plan: 2025-12-02-row-signatures-v1

## Overview

The RS1 implementation is release-ready. The items below are non-blocking hardening steps aimed at keeping the hashing layer easy to reason about over time: tightening documentation around the chosen commutative reduction, adding a couple of targeted tests for iteration-order independence and recomputation behavior, and making the `Grid` bounds invariants more explicit for library consumers.

## Fixes Required

### Fix 1: Update docs/comments to match the commutative-reduction design

- **Addresses Finding**: Finding 2 – Doc/spec wording still assumes sorted row/col order
- **Changes**:
  - In `cycle_plan.md`:
    - §4.3 “Determinism and platform independence”:  
      - Replace the parenthetical “we already iterate by explicit row/col indices; we must keep that invariant” with wording that explicitly calls out the commutative reduction approach: e.g., “we use an order-independent commutative reduction over per-cell XXHash64 contributions, so `HashMap` iteration order does not matter.” 
    - §5.2 “Grid methods”:  
      - Adjust the `compute_all_signatures` comment from “uses the two above” to something like “shares the same per-cell hashing scheme as `compute_row_signature`/`compute_col_signature` and populates `row_signatures`/`col_signatures` in one pass.” 
  - If there is a separate “docs-vs-implementation” doc for preprocessing (mentioned in references), ensure it also describes the commutative reduction instead of sorted streaming, consistent with the current code and tests. 
- **Tests**:
  - None required; this is documentation-only.
  - Re-run the existing test suite (`cargo test`) to confirm no accidental code changes.

---

### Fix 2: Add tests to lock in iteration-order independence

- **Addresses Finding**: Finding 3 – No test guards against `HashMap` iteration order changes
- **Changes**:
  - In `core/tests/signature_tests.rs`, add two tests that construct logically identical grids with different insertion orders:

    **New test 1: `row_signature_independent_of_insertion_order`**

    - Build two 1×3 grids:
      - `grid1`: insert cells for row 0 in order `(0,0)`, `(0,1)`, `(0,2)`.
      - `grid2`: insert the same cells but in reverse or shuffled order, e.g. `(0,2)`, `(0,0)`, `(0,1)`.
      - Values and formulas are identical across the two grids.
    - Assert:
      - `grid1.compute_row_signature(0).hash == grid2.compute_row_signature(0).hash`.
      - After calling `compute_all_signatures()` on each grid, `grid1.row_signatures[0].hash == grid2.row_signatures[0].hash`.

    **New test 2: `col_signature_independent_of_insertion_order`**

    - Analogous, but for a 3×1 grid and `compute_col_signature(0)` / `col_signatures[0]`.
    - Insert cells for column 0 in different row orders in the two grids.

  - These tests should use a mix of types (number/text/bool and optionally a formula) to ensure the commutative reduction is exercised on realistic inputs.
- **Tests**:
  - Run `cargo test tests::signature_tests` (or full `cargo test`) and confirm:
    - New tests pass.
    - Golden constants (`ROW_SIGNATURE_GOLDEN`, `ROW_SIGNATURE_WITH_FORMULA_GOLDEN`) remain unchanged.   

  These tests will fail immediately if someone changes `combine_hashes` to a non-commutative operation in the future.

---

### Fix 3: Make `Grid` bounds invariants explicit

- **Addresses Finding**: Finding 4 – Grid bounds invariants are implicit and untested
- **Changes**:
  - In `core/src/workbook.rs`, clarify the intended invariant in a comment on `Grid` and/or `Grid::insert`, e.g.:

    - “Invariant: `cell.row < nrows` and `cell.col < ncols` for all cells in `cells`; the parser and all constructors must enforce this.”

  - Add defensive checks in debug builds while keeping release behavior unchanged:
    - Option A (minimal impact): use `debug_assert!(cell.row < self.nrows && cell.col < self.ncols);` inside `Grid::insert`. 
    - Optionally, add a similar `debug_assert!` in `compute_all_signatures` before indexing `row_hashes[row_idx]` and `col_hashes[col_idx]`.

  - If you prefer stronger guarantees for library consumers:
    - Introduce a helper constructor (e.g. `Grid::insert_checked`) that returns a `Result<(), GridError>` on out-of-range coordinates, and migrate internal code paths (parsing, tests) to use it.
    - Leave `insert` as-is for backward compatibility, but document that it assumes valid coordinates.
- **Tests**:
  - Optionally add a debug-only test that uses `#[should_panic]` to validate `Grid::insert` on out-of-range coordinates when compiled in debug mode.
  - For portability across build modes, it may be sufficient to rely on the debug assertions plus comments, given this is not a behavior change for normal inputs.

---

### Fix 4: Add a recomputation/idempotence test for `compute_all_signatures`

- **Addresses Finding**: Finding 5 – Repeated `compute_all_signatures` calls not test-backed
- **Changes**:
  - In `core/tests/signature_tests.rs` (or `sparse_grid_tests.rs`), add a new test like `compute_all_signatures_recomputes_after_mutation`:

    ```rust
    #[test]
    fn compute_all_signatures_recomputes_after_mutation() {
        let mut grid = Grid::new(3, 3);
        // Initial content
        grid.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
        grid.insert(make_cell(1, 1, Some(CellValue::Text("x".into())), None));

        grid.compute_all_signatures();
        let first_rows = grid.row_signatures.as_ref().unwrap().clone();
        let first_cols = grid.col_signatures.as_ref().unwrap().clone();

        // Mutate one cell and add another
        grid.insert(make_cell(1, 1, Some(CellValue::Text("y".into())), None));
        grid.insert(make_cell(2, 2, Some(CellValue::Bool(true)), None));

        grid.compute_all_signatures();
        let second_rows = grid.row_signatures.as_ref().unwrap();
        let second_cols = grid.col_signatures.as_ref().unwrap();

        assert_ne!(first_rows[1].hash, second_rows[1].hash);
        assert_ne!(first_cols[1].hash, second_cols[1].hash);
    }
    ```

    This explicitly verifies that:
    - Calling `compute_all_signatures` again does not reuse stale hashes.
    - Row/column signatures for affected indices actually change when the grid changes.
  - If you prefer to keep all signature-specific tests in one place, put this in `signature_tests.rs`; otherwise `sparse_grid_tests.rs` is also acceptable.
- **Tests**:
  - Run `cargo test` to verify the new test passes alongside existing ones.   

---

## Constraints

- Do not change the public shapes of `RowSignature`, `ColSignature`, or `DiffOp`; PG4/PG5 JSON and grid diff behavior must remain unchanged.   
- Do not change the hashing algorithm or golden constants unless intentionally updating the hashing surface; if hashing changes, regenerate and update golden constants and document the change.   
- Preserve the O(M) single-pass behavior and “no per-cell cloning” constraint for `compute_all_signatures`.   
- Any added assertions must not introduce overhead or panics for valid, in-spec grids in release builds.

## Expected Outcome

After these follow-up steps:

- Documentation clearly reflects the chosen XXHash64 + commutative-reduction design, minimizing confusion for future maintainers.
- Tests explicitly guard the most important invariants that future alignment phases depend on:
  - Independence from `HashMap` iteration order.
  - Correct recomputation behavior after grid mutations.
- Grid invariants around row/column bounds are explicit and harder to misuse for library consumers.
- The RS1 hashing layer remains stable, predictable, and well-defended against accidental regressions as the Excel diff engine evolves toward full alignment and AMR features.
