# Verification Report: 2025-12-02-row-signatures-v1

## Summary

The RS1 milestone implementation delivers deterministic, XXHash64-based `RowSignature` and `ColSignature` that are position-sensitive, type-sensitive, and formula-aware, with `compute_all_signatures` now implemented as a single O(M) streaming pass over the sparse grid using a commutative reduction. The test suite matches and exceeds the mini-spec’s Test Plan, including golden constants, iteration-order independence, formula coverage, empty-grid behavior, and parity between bulk and per-row/col APIs. Documentation in the unified spec, mini-spec, and docs-vs-implementation has been updated to reflect the commutative hashing model and the intentional deferral of O(k) per-row/column iteration. All prior remediation findings are effectively addressed. Remaining issues are minor documentation/test hardening items and do not block release.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### Finding 1 – `compute_row_signature` / `compute_col_signature` index preconditions are implicit

- **Severity**: Minor  
- **Category**: Gap (API contract / documentation)  
- **Description**:  
  `Grid::compute_row_signature` and `Grid::compute_col_signature` scan `self.cells.values()` and filter by `cell.row == row` / `cell.col == col`, then fold contributions into a hash. They do not validate that the requested `row`/`col` is within `[0, nrows)` / `[0, ncols)`. For out-of-range indices, the filter yields no cells and returns `RowSignature { hash: 0 }` or `ColSignature { hash: 0 }`. :contentReference[oaicite:0]{index=0}  

  In contrast, `compute_all_signatures` allocates vectors sized to `nrows` / `ncols` and relies on the invariant that existing cells respect those bounds (with debug assertions for safety).  The behavioral contract in the mini-spec is written in terms of valid row/column indices. :contentReference[oaicite:2]{index=2}  

- **Evidence**:  
  - `Grid::compute_row_signature` and `Grid::compute_col_signature` implementations in `core/src/workbook.rs`. :contentReference[oaicite:3]{index=3}  
  - `compute_all_signatures` invariants and debug assertions in `core/src/workbook.rs`.   
  - Mini-spec behavioral contract (§3.1, §3.2) describing semantics for “a given row index r in a Grid” without explicitly stating preconditions for out-of-range indices. :contentReference[oaicite:5]{index=5}  

- **Impact**:  
  If callers accidentally pass out-of-range indices, they will get a “valid-looking” zero hash rather than an error or panic. Because all-empty rows/columns are intentionally represented by zero hashes (and tested as such),  it becomes impossible to distinguish “valid empty row/column” from “API misuse” via the return value alone. This is unlikely to surface as a bug in current usage (RS1 is not yet consumed by alignment), but once higher-level code starts using signatures, the lack of an explicit precondition in docs (or a debug assertion) could mask subtle mistakes.

---

### Finding 2 – Sparse/empty-cell invariant is relied on but only lightly documented

- **Severity**: Minor  
- **Category**: Gap (invariants / future-proofing)  
- **Description**:  
  The guarantee that row and column hashes “ignore empty cells” is implemented via sparsity: `Grid` stores only non-empty cells in its `HashMap`, and `hash_cell_contribution` always hashes the `Cell`’s `value` and `formula` Options into XXHash64.   

  The unified spec and mini-spec describe empties as “cells not present in the sparse map” and define row/column hashes in terms of non-empty cells.  Current tests validate that extra empty columns/rows do not affect hashes (`row_signature_ignores_empty_trailing_cells`, `col_signature_ignores_empty_trailing_rows`, empty-grid and all-empty-rows/cols tests).   

  However, the hashing helper will happily include a `Cell` whose `value` and `formula` are both `None`, because it always hashes whichever `Option` fields the `Cell` actually contains. The invariant that such “explicit empties” never exist is enforced by parser behavior and informal understanding, not a strongly advertised contract in the `Grid` docs.

- **Evidence**:  
  - `hash_cell_contribution(position, cell)` implementation in `core/src/workbook.rs`.   
  - Sparse grid tests and signature tests that assert zero hashes for empty rows/columns and correct handling of sparse grids.   
  - Unified spec and mini-spec description of “non-empty cells” as the unit of hashing.   

- **Impact**:  
  Today, all code that builds `Grid` respects the intended invariant (no explicit empty `Cell` entries), so behavior matches the spec and tests. But as the system evolves, another component could insert a `Cell` with `value: None` and `formula: None` to represent a forced empty, and that cell would silently affect the hash. Because empties are supposed to be ignored at the hashing layer, this would be a spec violation without an obvious local symptom. Making the “no explicit empty cell” invariant more explicit in `Grid`’s documentation (or enforcing it via construction APIs) would reduce this future risk.

---

### Finding 3 – No golden constant for column signatures

- **Severity**: Minor  
- **Category**: Missing Test / Hardening gap  
- **Description**:  
  The mini-spec requires at least one golden constant for a row hash to lock in the chosen XXHash64 configuration and encoding. This is implemented with two row-side constants: one for a value-only row, and one including a formula.   

  Column signatures have extensive behavioral tests: identical/different columns, position sensitivity (row reordering), type discrimination, sparsity invariance, formula inclusion (including a “sparse column with formulas” test), and insertion-order independence.  But there is no pinned `ColSignature` golden constant analogous to `ROW_SIGNATURE_GOLDEN` / `ROW_SIGNATURE_WITH_FORMULA_GOLDEN`. A previous remediation explicitly called out the lack of golden coverage for column behavior and formula inclusion as a potential future risk.   

- **Evidence**:  
  - `ROW_SIGNATURE_GOLDEN` and `ROW_SIGNATURE_WITH_FORMULA_GOLDEN` in `core/tests/signature_tests.rs`. :contentReference[oaicite:16]{index=16}  
  - Column behavior tests in `core/tests/signature_tests.rs` (position, types, sparsity, formula inclusion).   
  - Prior remediation notes on golden coverage. :contentReference[oaicite:18]{index=18}  

- **Impact**:  
  The absence of a column golden constant is not a correctness problem today; the behavioral tests are strong, and RS1’s contract is well covered. However, if the hash algorithm, seed, or mixing function were changed in the future in a way that only affected column hashing (or only affected formula inclusion), it’s possible that row-side goldens would stay green while column semantics changed. Adding a column golden constant would provide a cheap, high-signal guardrail against subtle regressions in column fingerprints.

---

## Prior Remediation Verification

All previously identified findings from earlier remediation rounds have been addressed:

- **Performance / allocations in `compute_all_signatures`**  
  - `compute_all_signatures` now streams directly over `self.cells.values()` with a single pass, maintaining per-row and per-column accumulators and combining per-cell hashes via a commutative reducer (`mix_hash` + `wrapping_add`). No per-cell cloning or sorting remains.   
  - This matches the mini-spec’s O(M) and “no new per-cell allocations” constraints.   

- **Formula hashing coverage**  
  - Tests compare value+formula vs value-only for both rows and columns (`row_signature_includes_formulas_by_default`, `col_signature_includes_formulas_by_default`, plus a sparse-formula column case).   
  - A second golden constant explicitly exercises formula inclusion on the row side (`ROW_SIGNATURE_WITH_FORMULA_GOLDEN`). :contentReference[oaicite:22]{index=22}  

- **Column invariants and sparsity**  
  - Column tests mirror row tests: position sensitivity, type discrimination, ignoring empty trailing rows, and sparse-column invariance.   

- **Bulk vs per-row/col parity**  
  - `row_and_col_signatures_match_bulk_computation` asserts that row/column signatures returned by `compute_row_signature`/`compute_col_signature` match the precomputed `row_signatures`/`col_signatures` filled in by `compute_all_signatures`. :contentReference[oaicite:24]{index=24}  

- **Iteration-order independence & commutative reduction**  
  - Tests confirm insertion-order independence for both rows and columns.   
  - The unified spec, mini-spec, and docs-vs-implementation docs now describe hashing as “per-cell XXHash64 contributions combined via a commutative reduction,” not as an ordered streaming hash with sorting.   

- **Degenerate/empty grid coverage & bounds invariants**  
  - New tests verify that `Grid::new(0,0)` produces empty but present signature vectors and that grids with all-empty rows/columns yield zero hashes in the expected positions.   
  - `compute_all_signatures` includes debug assertions that ensure cell row/col indices are within grid bounds during hashing.   

Collectively, these changes bring the implementation and documentation into alignment with the RS1 mini-spec and prior remediation plans.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - `RowSignature` / `ColSignature` implemented over XXHash64 with position, type, and formula inclusion; `Grid::compute_row_signature`, `Grid::compute_col_signature`, and `Grid::compute_all_signatures` implemented as specified.   

- [x] All specified tests created  
  - All tests from §6.1/§6.2 of the mini-spec exist, plus additional tests for formulas, iteration-order independence, recomputation, empty grids, and bulk vs per-row/col parity.   

- [x] Behavioral contract satisfied  
  - Determinism, position sensitivity, type sensitivity, sparse-empties behavior, and formula inclusion are all implemented and test-backed for both rows and columns.   

- [x] No undocumented deviations from spec  
  - Unified spec and docs-vs-implementation documents now match the commutative hashing model and explicitly call out deferred normalization and O(k) row/column iteration as future work.   

- [x] Error handling adequate  
  - No new I/O or fallible paths; hashing operates over in-memory structures. Invariants are enforced via debug assertions, consistent with existing code style.   

- [x] No obvious performance regressions  
  - `compute_all_signatures` is O(M) with no per-cell cloning, using XXHash64 as specified; per-row/col methods remain O(M) by design, with their O(k) optimization explicitly deferred. Tests and prior remediation notes confirm that hashing overhead is modest and within expectations.   
