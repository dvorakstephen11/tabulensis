grid.insert(Cell {
    row: 1,
    col: 3,
    // ...
    value: Some(CellValue::Number(42.0)),
    formula: Some("=A1".into()),
});
grid.compute_all_signatures();
assert_ne!(row_hash, 0);
assert_ne!(col_hash, 0);
``` :contentReference[oaicite:14]{index=14}  

But there is no test that **compares** hashes of “same value with formula” vs “same value without formula”.

- **Evidence**:  
- Behavioral contract §3.1 point 5 in `cycle_plan.md`. :contentReference[oaicite:15]{index=15}  
- `hash_cell_with_position` and overall hashing wiring in `workbook.rs`.   
- Existing tests in `signature_tests.rs` and `sparse_grid_tests.rs` (no formula-vs-no-formula comparison).   

- **Impact**:  
Right now, the implementation *does* include formulas in the hash, so there isn’t a correctness bug. However, a future refactor (e.g., optimizing hashing or introducing normalization) could accidentally drop `cell.formula.hash(state)` and all tests would still pass. Given how heavily formulas matter for spreadsheet semantics and for future “semantic formula comparison” work in the spec, this is worth locking down.

---

### Finding 4 – Column-side invariants only partially test-backed

- **Severity**: Minor  
- **Category**: Missing Test / Gap  
- **Description**:  
The mini-spec requires column signatures to mirror row signature behavior: deterministic, position-sensitive vertically, type-sensitive, ignore empty rows, and include formulas. :contentReference[oaicite:18]{index=18}  
The implementation uses the same `hash_cell_with_position` helper and simply swaps which index is fed as `position`:

```rust
// rows
hash_cell_with_position(cell.col, cell, &mut hasher);
// cols
hash_cell_with_position(cell.row, cell, &mut hasher);
``` :contentReference[oaicite:19]{index=19}  

Current tests cover:

- Column position sensitivity (`col_signatures_distinguish_row_positions`). :contentReference[oaicite:20]{index=20}  
- Existence and non-zero column hashes after `compute_all_signatures`. :contentReference[oaicite:21]{index=21}  

But they do **not** explicitly cover:

- Column type discrimination (`Number(1.0)` vs `Text("1")` vs `Bool(true)` for a single column).
- Column invariance to extra empty rows (the “sparse columns” example in §3.2).
- Column formula inclusion by default (formula vs no-formula in the same column). :contentReference[oaicite:22]{index=22}  

- **Evidence**:  
- Column signature spec §3.2 in `cycle_plan.md`. :contentReference[oaicite:23]{index=23}  
- `col_signatures_distinguish_row_positions` test and related signature tests.   

- **Impact**:  
Because rows and columns share the same hashing helper, the behavior is almost certainly correct today, but the lack of column-focused tests leaves room for future regressions to slip in unnoticed—especially once column-based alignment from the unified spec is implemented.   

---

### Finding 5 – `compute_all_signatures` not factored through `compute_row_signature` / `compute_col_signature`

- **Severity**: Minor  
- **Category**: Spec Deviation / Architectural Drift  
- **Description**:  
The mini-spec’s interface section notes:

```rust
pub fn compute_row_signature(&self, row: u32) -> RowSignature { /* stronger impl */ }
pub fn compute_col_signature(&self, col: u32) -> ColSignature { /* stronger impl */ }
pub fn compute_all_signatures(&mut self) { /* uses the two above */ }
``` :contentReference[oaicite:26]{index=26}  

In reality, `compute_all_signatures` reimplements hashing logic directly by iterating over sorted `cells` and updating `row_hashers` / `col_hashers`, without calling `compute_row_signature` / `compute_col_signature`. :contentReference[oaicite:27]{index=27}  

To keep things in sync, a shared `hash_cell_with_position` helper is used, so the actual hashing behavior is consistent at the moment.   

- **Evidence**:  
- Interface expectations in `cycle_plan.md` §5.2. :contentReference[oaicite:29]{index=29}  
- `compute_all_signatures` implementation in `workbook.rs`. :contentReference[oaicite:30]{index=30}  

- **Impact**:  
This doesn’t currently break behavior (tests like `compute_all_signatures_populates_fields` and the golden-row test all pass).   
The risk is future drift: if someone updates `compute_row_signature` (e.g., to add normalization or a new type tag) but forgets to update the `compute_all_signatures` path, results from direct row calls and the bulk precomputation could diverge. That would be very confusing for alignment work that depends on both.

---

### Finding 6 – Column sparsity invariance not explicitly tested

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
The contract for columns includes the “sparse columns” example:

> `grid1`: `nrows = 5`, only `A1 = "foo"`.  
> `grid2`: `nrows = 10`, only `A1 = "foo"`.  
> Contract: column 0 signatures are equal. :contentReference[oaicite:32]{index=32}  

There is an analogous test for rows (`row_signature_ignores_empty_trailing_cells`), which verifies that extra empty columns do not change a row signature. :contentReference[oaicite:33]{index=33}  
There is no corresponding test for columns and extra empty rows.

- **Evidence**:  
- Row empties test `row_signature_ignores_empty_trailing_cells`. :contentReference[oaicite:34]{index=34}  
- Column contract’s “Sparse columns” example in §3.2. :contentReference[oaicite:35]{index=35}  

- **Impact**:  
Implementation is consistent with the intended invariant (only present cells are hashed; `Grid` is sparse and doesn’t store explicit empties).   
Still, without an explicit column-side test, a later change that, for example, factors `nrows` into the hash (or misuses row indices) could break this property without triggering any test failure.

---

### Finding 7 – Golden constant only covers a row without formulas

- **Severity**: Minor  
- **Category**: Missing Test / Coverage Gap  
- **Description**:  
The test plan calls for at least one “golden constant” to lock in the exact hash for a small row. :contentReference[oaicite:37]{index=37}  
This is implemented as:

```rust
const ROW_SIGNATURE_GOLDEN: u64 = 8_394_164_658_571_930_929;

#[test]
fn row_signature_golden_constant_small_grid() {
    let mut grid = Grid::new(1, 3);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert(make_cell(0, 1, Some(CellValue::Text("x".into())), None));
    grid.insert(make_cell(0, 2, Some(CellValue::Bool(false)), None));

    let sig = grid.compute_row_signature(0);
    assert_eq!(sig.hash, ROW_SIGNATURE_GOLDEN);
}
``` :contentReference[oaicite:38]{index=38}  

This locks in:

- XXHash64 choice and seed.
- Encoding of position, type tag, and value for numbers, text, and booleans.

But it does **not** lock in:

- Column signature behavior.
- Formula inclusion semantics (no formula in this row).

- **Evidence**:  
- Golden constant test in `signature_tests.rs`. :contentReference[oaicite:39]{index=39}  
- Golden requirement in `cycle_plan.md` §6.1 point 5. :contentReference[oaicite:40]{index=40}  

- **Impact**:  
A future change that affects formula hashing or column hashing (e.g., different encoding, normalisation, or a different combination strategy) may not cause the golden-row test to fail. That means we could unintentionally weaken the semantics that future alignment stages rely on while still “passing” RS1 tests.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
- Row/column signatures now use XXHash64; position and type are included; formulas are hashed; tests extended; docs updated; and `xxhash-rust` dependency added.   
- [x] All specified tests created  
- All five tests listed in §6.1 (two position tests, type discrimination, empty-trailing-cells, golden constant) are present.   
- [ ] Behavioral contract satisfied  
- Semantic behavior is correct for type/position/empties/formula inclusion, but performance constraints (O(M) and “no per-cell allocations”) are not fully met.   
- [ ] No undocumented deviations from spec  
- The O(M log M) behavior and cloning in `compute_all_signatures` deviate from the O(M), no-per-cell-allocation constraints in the mini-spec and unified spec. These are partially acknowledged in the activity log but not yet reflected in the spec text.   
- [x] Error handling adequate  
- No new error-prone I/O paths were introduced; hashing operates purely in memory over existing structures.  
- [ ] No obvious performance regressions  
- Compared to the previous naive O(R×C) looping, the new implementation is probably faster for sparse grids, but it still misses the advertised O(M) design and does per-cell cloning. This is an improvement relative to old behavior, but not aligned with the new performance contract.

---

# Remediation Plan: 2025-12-02-row-signatures-v1

## Overview

The current branch delivers the core RS1 goal: deterministic, position- and type-sensitive XXHash64-based row and column signatures with basic test coverage and updated documentation. Remaining work is mostly about tightening conformance to the performance/memory constraints and strengthening tests around formulas and column behavior so that future refactors or normalization changes don’t silently weaken signatures.

None of the findings are critical for immediate release, but addressing them now will prevent subtle regressions when the alignment pipeline (Patience Diff, AMR) starts relying heavily on these fingerprints.

## Fixes Required

### Fix 1: Align `compute_all_signatures` with O(M) and memory constraints

- **Addresses Finding**: Finding 1 (and partially Finding 5)  
- **Changes**:

**Goals:**

- Eliminate cloning of `Cell` values inside `compute_all_signatures`.
- Avoid O(M log M) sorts while still guaranteeing determinism and independence from `HashMap` iteration order.   
- Keep row and column signature semantics exactly as in RS1 (position, type, value, and formula included).

**Suggested direction:**

1. **Stop cloning cells**  
   - Replace `let mut cells: Vec<Cell> = self.cells.values().cloned().collect();` with a structure that only duplicates coordinates or references, not entire `Cell` objects (e.g., a `Vec<(u32, u32)>` of keys or `Vec<&Cell>`). The key is “no new per-cell allocations for full `Cell` contents”.   

2. **Re-think ordering vs determinism**  
   Options:

   - **Option A – Order-independent reduction**  
     - Compute a per-cell hash: `h_cell = H(col_index, type_tag, value, formula)` using XXHash64.  
     - Combine per-row/per-column using a commutative operation (e.g., XOR / wrapping add) so that iteration order no longer matters, making sorting unnecessary. This preserves O(M) behavior as you can stream over `self.cells` once.
     - This may require updating the spec language in §3.3 to clarify the reduction scheme (still based on XXHash64, but not strictly as a single streaming sequence).   

   - **Option B – Sort only lightweight keys**  
     - If streaming XXHash64 over an ordered sequence is considered essential, keep order-sensitive hashing but:
       - Collect a `Vec<(u32, u32)>` of keys, sort that by `(row, col)` then `(col, row)`.
       - For each key, look up `&Cell` from `self.cells` and feed it into the appropriate row/column hasher.
     - This remains O(M log M) but respects the “no new per-cell allocations” constraint by avoiding `Cell` clones and only duplicating small `(u32, u32)` keys. That partially satisfies §4.2 but still doesn’t meet the strict O(M) requirement.

3. **Ensure invariants and tests still pass**  
   - Keep `row_signatures` and `col_signatures` as `Some(Vec<...>)` of lengths `nrows`/`ncols`. :contentReference[oaicite:48]{index=48}  
   - Ensure existing tests (`compute_all_signatures_populates_fields`, sparse grid test, golden constant) continue to pass.   

4. **Document the chosen approach**  
   - Update `cycle_plan.md` / design docs if the implementation settles on a commutative reduction (Option A) instead of sorted-sequence hashing, so the spec and implementation stay in sync.

- **Tests**:

- Extend `sparse_grid_tests.rs` with a simple **consistency test**:

  - Build a small grid.
  - Call `compute_all_signatures()`.
  - For a couple of rows/cols, assert that:
    - `grid.compute_row_signature(r).hash == grid.row_signatures.as_ref().unwrap()[r].hash`
    - Ditto for columns.  

  This guards against divergence between per-row/per-col functions and the bulk computation path, especially after internal refactors.

---

### Fix 2: Add explicit tests for formula inclusion

- **Addresses Finding**: Finding 3 (and part of Finding 7)  
- **Changes**:

**New tests in `core/tests/signature_tests.rs`:**

1. `row_signature_includes_formulas_by_default`  

   - Build:
     - `grid1`: 1×1, `A1` value `10`, formula `"=5+5"`.
     - `grid2`: 1×1, `A1` value `10`, no formula.
   - Assert:

     ```rust
     let sig1 = grid1.compute_row_signature(0).hash;
     let sig2 = grid2.compute_row_signature(0).hash;
     assert_ne!(sig1, sig2);
     ```

   This directly codifies the mini-spec’s example. :contentReference[oaicite:50]{index=50}  

2. `col_signature_includes_formulas_by_default`  

   - Same idea as above, but in a 2×1 grid, differing only in a formula vs no-formula at `row = 0, col = 0`.
   - Assert that column 0 signatures differ.

**Optional golden enhancement:**

- Add a **second golden constant** that covers a row containing at least one formula, using the current implementation to compute and record a fixed `u64`. This locks in formula contribution as part of the golden surface.

- **Tests**:

- Ensure these tests are added alongside the existing RS1 tests in `signature_tests.rs` and run under `cargo test` at the workspace root.   

---

### Fix 3: Strengthen column-side invariants and sparsity coverage

- **Addresses Finding**: Findings 4 and 6  
- **Changes**:

Add the following tests to `core/tests/signature_tests.rs`:

1. `col_signature_distinguishes_numeric_text_bool`  

   - Mirroring the row test:
     - `grid_num`: 3×1, `A1 = Number(1.0)`, `A2 = A3 = empty`.
     - `grid_text`: `A1 = Text("1")`.
     - `grid_bool`: `A1 = Bool(true)`.
   - Assert all three column 0 hashes differ.

2. `col_signature_ignores_empty_trailing_rows`  

   - Mirror the row “empty trailing cells” test:
     - `grid1`: `nrows = 3`, only `A1 = Number(42.0)`.
     - `grid2`: `nrows = 10`, only `A1 = Number(42.0)`.
   - Assert column 0 signatures are equal.

3. (Optional) `col_signature_includes_formulas_sparse`  

   - Combine sparsity and formulas:
     - Two tall grids where only `A1` is non-empty in both, with same value but formula present vs absent.
   - Assert column hashes differ, while adding extra empty rows does not change either hash.

- **Tests**:

- Run `cargo test` and confirm all new tests pass.
- This will more fully align test coverage with the column contract in §3.2 of the mini-spec.   

---

### Fix 4: Clarify or defer O(k) requirement for `compute_row_signature` / `compute_col_signature`

- **Addresses Finding**: Finding 2  
- **Changes**:

Two possible paths:

1. **Implement O(k) behavior now**  

   - Introduce lightweight row/column buckets or an index structure that allows:
     - Iterating only cells in a given row or column without scanning the whole `HashMap`.
     - Maintaining determinism and sparse properties.
   - This is more involved and begins to overlap with the future `GridView` design from the unified spec (Layer 1).   

2. **Document as a deferred constraint**  

   - If the team decides that O(M) per call is acceptable until GridView exists:
     - Adjust `cycle_plan.md` §4.1 to explicitly mark the O(k) requirement as a “future optimization” rather than a current guarantee.
     - Note in documentation that callers should prefer `compute_all_signatures` where possible, and avoid using per-row/col methods in tight loops on huge grids.

- **Tests**:

- If implementing O(k), add a micro-benchmark or at least a sanity test that exercises a grid with many rows but few cells per row and ensures behavior remains correct and deterministic.

---

### Fix 5: Reduce risk of drift between per-row/per-col and bulk signatures

- **Addresses Finding**: Finding 5 (also supports Fix 1)  
- **Changes**:

- Either:
  - Refactor `compute_row_signature` and `compute_col_signature` to share more code with `compute_all_signatures` (e.g., via common helpers that compute per-row/per-col hashes given an iterator of cells).
  - Or add explicit tests that assert equality between:
    - `compute_row_signature(r).hash` and `row_signatures[r].hash` after `compute_all_signatures()`.
    - `compute_col_signature(c).hash` and `col_signatures[c].hash`.

- **Tests**:

- Add a dedicated test (e.g., `row_and_col_signatures_match_bulk_computation`) to `signature_tests.rs`:

  - Build a small grid with multiple rows, columns, and formulas.
  - Call `compute_all_signatures`.
  - For each row and column, assert equality between direct and cached signatures.

This ensures any future changes that modify the hashing path for one but not the other surface immediately in tests.

---

## Constraints

- Any change to the hashing algorithm (including adopting order-independent reductions) that affects existing hashes must:
- Update the golden constant(s) in `signature_tests.rs`.   
- Update documentation in `2025-11-30-docs-vs-implementation.md` and, if necessary, the unified algorithm spec’s hashing section.   
- Public data structures and the `DiffOp` wire format must remain unchanged; PG tests and JSON expectations should continue to pass exactly as today.   
- Any new allocations or indexing structures must respect the overall memory philosophy of the sparse `Grid` IR—no structures proportional to full `nrows × ncols`, only to the number of non-empty cells and dimensions.   

## Expected Outcome

After these remediation steps:

- Row and column signatures remain deterministic, position-sensitive, type-sensitive, and formula-sensitive, now with explicit tests that lock in those behaviors for both rows and columns.
- `compute_all_signatures` better matches the mini-spec’s performance and memory expectations (or the spec is updated to accurately reflect the chosen design).
- Bulk and per-row/per-col signature computations are guaranteed to be consistent, reducing risk as alignment phases start to depend heavily on these hashes.
- The RS1 milestone is fully “hardened” and ready to serve as a reliable foundation for subsequent alignment work (anchors, Patience Diff, AMR) described in the unified grid diff algorithm specification.   
