Incremental milestone: **RS1 – Row/Column Signature Semantics v1**

This milestone strengthens the grid fingerprinting layer so that future alignment work (anchors, Patience Diff, dimension ordering) can safely rely on `RowSignature` and `ColSignature`.

---

## 1. Scope

### 1.1 Rust types and modules

**Primary code:**

- `core/src/workbook.rs`
  - `pub struct RowSignature { pub hash: u64 }`
  - `pub struct ColSignature { pub hash: u64 }`
  - `impl Grid`:
    - `pub fn compute_row_signature(&self, row: u32) -> RowSignature`
    - `pub fn compute_col_signature(&self, col: u32) -> ColSignature`
    - `pub fn compute_all_signatures(&mut self)`

**Tests:**

- `core/tests/signature_tests.rs`
  - Extend to cover position sensitivity, type discrimination, and determinism.
- `core/tests/sparse_grid_tests.rs`
  - Keep existing `compute_all_signatures` coverage; optionally add one sanity assertion if needed, but no behavioral change expected.

**Tooling / deps:**

- `core/Cargo.toml`
  - Add a **stable, explicit hash implementation** for 64-bit row/column hashes (e.g., `xxhash-rust` for XXHash64), wired into the signature computation functions.

**Documentation:**

- `docs/rust_docs/2025-11-30-docs-vs-implementation.md`
  - Update the Part III Preprocessing table to reflect that:
    - Column/row position are now included in signatures.
    - Type discriminants are verified by tests.
    - Hash algorithm is now XXHash64 (or equivalent) instead of `DefaultHasher`.
  - Leave frequency analysis, token interning, and full normalization as “still not implemented”.

No changes in this milestone to:

- `core/src/engine.rs` (`diff_workbooks` / `diff_grids`)
- `core/src/diff.rs` public `DiffOp` / `DiffReport` shapes
- `core/tests/pg5_grid_diff_tests.rs` or any PG-level behavior

---

## 2. Context and Linkage to Milestones

- The unified grid diff spec defines **Phase 1: Preprocessing & Hashing** as the front door to the entire alignment pipeline: row/column hashes, frequency tables, classification, and GridView metadata. 
- The docs-vs-implementation review scores Preprocessing at ~30% complete and explicitly calls out the current signatures as too weak: no column/row position, `DefaultHasher`, and missing type/normalization guarantees. It recommends **“Immediate: Enhance row signatures (column positions, type tags)”** as the first step on the critical path. :contentReference[oaicite:13]{index=13}
- The testing plan’s future **grid alignment milestones** (G8+ row anchors, gap filling, move detection, adaptive dimension ordering) all assume that:
  - Equal hashes imply strong evidence of equal row/column content.
  - Position changes (swapped columns/rows) change the hash. 
- This mini-spec introduces a **new incremental milestone** on the way to those alignment milestones:

> **RS1 – Row/Column Signature Semantics v1**  
> “Row and column signatures are stable, explicit XXHash64-based fingerprints that are sensitive to row/column position and value type, with tests that lock in these properties.”

We stay within Phase 2/early Phase 3: no frequency tables, no anchors, no Patience Diff yet—just solid fingerprints and tests.

---

## 3. Behavioral Contract

### 3.1 Row signatures (`RowSignature`)

**High-level behavior**

For a given row index `r` in a `Grid`:

- `Grid::compute_row_signature(r)` returns a `RowSignature { hash }` that:
  - **Is deterministic**: same grid content ⇒ same `hash` across:
    - Multiple calls in one process.
    - Different builds and platforms (x86, ARM, WASM).
  - **Is position-sensitive horizontally**: if the same values appear in different columns, the hash changes.
  - **Is type-sensitive**: `Number(1.0)` vs `Text("1")` vs `Bool(true)` must hash differently.
  - **Ignores empties**: inserting or removing empty cells (i.e., cells not present in the sparse map) does not change the hash, as long as all non-empty cells (value+formula) remain the same at the same coordinates.
  - **Includes formulas by default**: a value-only cell and a value+formula cell at the same position must produce different hashes if formulas differ.

**Concrete examples**

Let `grid1` and `grid2` be separate `Grid`s with `nrows = 1, ncols = 3`.

1. **Identical rows, same positions ⇒ same hash**

   - `grid1`: row 0: `[1, 2, 3]` in columns A,B,C.
   - `grid2`: row 0: `[1, 2, 3]` in columns A,B,C.
   - Contract:  
     `grid1.compute_row_signature(0).hash == grid2.compute_row_signature(0).hash`

2. **Swapped columns ⇒ different hash**

   - `grid1`: row 0: `[1, 2]` in columns A,B.
   - `grid2`: row 0: `[2, 1]` in columns A,B (values swapped).
   - Contract:  
     `grid1.compute_row_signature(0).hash != grid2.compute_row_signature(0).hash`

   This enforces the “column position included in row hash” requirement from the spec. 

3. **Numeric vs text vs bool ⇒ different hashes**

   - `grid1`: `A1 = Number(1.0)`
   - `grid2`: `A1 = Text("1")`
   - `grid3`: `A1 = Bool(true)`
   - Contract:
     - `sig1.hash != sig2.hash`
     - `sig1.hash != sig3.hash`
     - `sig2.hash != sig3.hash`

   This reflects inclusion of a type discriminant in the hash. (The existing `CellValue::hash` already encodes type tags; we are making that behavior explicit and test-backed.) 

4. **Empty cells / sparse rows ⇒ no effect**

   - `grid1`: `ncols = 5`, only `A1 = Number(42)`.
   - `grid2`: `ncols = 10`, only `A1 = Number(42)`; all other positions are empty.
   - Contract:  
     `grid1.compute_row_signature(0).hash == grid2.compute_row_signature(0).hash`

   Sparse rows that differ only in empty cells should hash identically.

5. **Formulas included by default**

   - `grid1`: `A1` value `10`, formula `=5+5`.
   - `grid2`: `A1` value `10`, no formula.
   - Contract:  
     `grid1.compute_row_signature(0).hash != grid2.compute_row_signature(0).hash`

   Later configuration support (`include_formulas`) can relax this; for now we hard-code the “include formulas” default from the spec. :contentReference[oaicite:17]{index=17}

### 3.2 Column signatures (`ColSignature`)

**High-level behavior**

For a given column index `c` in a `Grid`:

- `Grid::compute_col_signature(c)` returns a `ColSignature { hash }` that:
  - **Is deterministic** across runs/platforms.
  - **Is position-sensitive vertically**: reordering rows with the same values changes the hash.
  - **Is type-sensitive**, analogous to rows.
  - **Ignores empty rows** (no cell at `(row, c)`).
  - **Includes formulas** by default.

**Concrete examples**

Let `grid1` and `grid2` have `nrows = 2, ncols = 1`.

1. **Identical column ⇒ same hash**

   - `grid1` column 0: `[1, 2]` in rows 0,1.
   - `grid2` column 0: `[1, 2]` in rows 0,1.
   - Contract:  
     `grid1.compute_col_signature(0).hash == grid2.compute_col_signature(0).hash`

2. **Swapped rows ⇒ different hash**

   - `grid1` column 0: `[1, 2]` (row 0 = 1, row 1 = 2).
   - `grid2` column 0: `[2, 1]` (row 0 = 2, row 1 = 1).
   - Contract:  
     `grid1.compute_col_signature(0).hash != grid2.compute_col_signature(0).hash`

   This mirrors the spec’s requirement to feed row index into column hashes. 

3. **Sparse columns**

   - `grid1`: `nrows = 5`, only `A1 = "foo"`.
   - `grid2`: `nrows = 10`, only `A1 = "foo"`.
   - Contract:  
     `grid1.compute_col_signature(0).hash == grid2.compute_col_signature(0).hash`

### 3.3 Hash algorithm and determinism

- `RowSignature::hash` and `ColSignature::hash` must be computed using a **fixed, explicit 64-bit hash function**, not `std::collections::hash_map::DefaultHasher` or any API with random seeding.
- The implementation should follow the spec’s Hash64 guidance:
  - Use **XXHash64** (or equivalent) with a fixed seed and explicitly defined byte order. 
  - Feed into the hasher, for each non-empty cell:
    - Row hash: `(column_index, type_tag, normalized value, optional formula bytes)`
    - Column hash: `(row_index, type_tag, normalized value, optional formula bytes)`
- Implementation note: per-cell XXHash64 contributions are combined with a commutative reduction (mix + wrapping add) so that `compute_all_signatures` can stream over the sparse `HashMap` once without sorting while remaining deterministic and position/type/formula aware.
- For this milestone:
  - We **require** position indices and a type tag; we **allow** value normalization to remain “as-is” (using current `CellValue::hash`), acknowledging that full normalization (numeric rounding, string Unicode normalization) is deferred to a later milestone.
  - The tests will include at least one **golden constant**: a small row whose `hash` is asserted against a fixed `u64` value, locking in the particular XXHash64 configuration.

### 3.4 Non-goals for this milestone

Explicitly out of scope:

- Implementing the full normalization rules from spec section 6.2 (numeric rounding to 15 significant digits, Unicode NFC normalization, etc.). 
- Introducing `GridView`, `RowMeta`, `ColMeta`, frequency tables, or classification (unique/rare/common/low-info).
- Using signatures inside `diff_grids` or `diff_workbooks` for alignment. PG5 and engine behavior stay unchanged.
- Any change to `DiffOp` wire format: `RowSignature`/`ColSignature` remain `{ "hash": number }` when present; only their values become stronger and deterministic.

---

## 4. Constraints

### 4.1 Performance

- `Grid::compute_row_signature` and `Grid::compute_col_signature` are currently **O(M)** scans over the sparse map; the intended **O(k)** behavior (row/col-local iteration) remains a follow-up optimization once row/column indexing (GridView) is introduced.
- `Grid::compute_all_signatures` must remain **O(M)** where `M` is total non-empty cells:
  - Implementation: single pass over each row and column using commutative accumulation; no per-cell cloning or sorting needed for determinism.
- Hash computation overhead should be modest:
  - For small unit tests and typical PG-level scenarios, overhead is negligible.
  - For large grids (up to tens of thousands of rows), hashing should not dominate; the chosen hash (XXHash64) is explicitly spec’d as “fast enough” for this. 

### 4.2 Memory

- No new per-cell allocations during hashing beyond what already exists in `Grid`:
  - Use existing `Cell` structures and iterate them.
  - Avoid building per-row or per-column collections beyond transient small stacks or fixed-size scratch buffers.
- `row_signatures` and `col_signatures` on `Grid` remain:
  - `Option<Vec<RowSignature>>` of length `nrows`
  - `Option<Vec<ColSignature>>` of length `ncols`

### 4.3 Determinism and platform independence

- Hash results must be:
  - Independent of Rust’s `Hash` trait implementation for `HashMap` (no `DefaultHasher`).
  - Independent of iteration order of `Grid.cells` thanks to the order-independent commutative reduction over per-cell XXHash64 contributions (HashMap iteration order does not affect the result).
  - Identical across architectures and build modes (debug vs release).

### 4.4 Invariants

- `Grid::compute_all_signatures`:
  - After calling, `row_signatures` is `Some` with length `nrows`, `col_signatures` is `Some` with length `ncols`.
  - Re-calling recomputes all hashes and overwrites previous ones; no caching assumptions in this milestone.
- `RowSignature` and `ColSignature` remain **pure data carriers**:
  - No behavior or configuration attached; just the `hash: u64` field.

---

## 5. Interfaces

### 5.1 Public data structures

No shape changes, but semantics are tightened.

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowSignature {
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ColSignature {
    pub hash: u64,
}
````

These types are used in `DiffOp::RowAdded/RowRemoved` and `DiffOp::ColumnAdded/ColumnRemoved` as optional fields; this milestone does **not** change that mapping.

### 5.2 Grid methods

Existing methods keep the same signatures:

```rust
impl Grid {
    pub fn compute_row_signature(&self, row: u32) -> RowSignature { /* stronger impl */ }

    pub fn compute_col_signature(&self, col: u32) -> ColSignature { /* stronger impl */ }

    pub fn compute_all_signatures(&mut self) { /* shares the same per-cell hashing scheme as row/col helpers and populates row_signatures/col_signatures in one pass */ }
}
```

Behavioral changes for this milestone:

* Internally switch from `DefaultHasher` to an explicit XXHash64 (or equivalent) implementation.
* Incorporate:

  * Column index into row hashing.
  * Row index into column hashing.
  * An explicit type tag in the byte stream (can re-use or wrap `CellValue::hash` as long as tests are satisfied).
* Maintain API surface so existing callers and JSON tests compile unchanged.

### 5.3 Configuration

* No public configuration changes in this milestone (no `hash_algorithm` or `include_formulas` knobs yet).
* Internally, code may be refactored to make adding `hash_algorithm` config easier later, but that is not required by this spec.

---

## 6. Test Plan

All new/updated tests must be added in the Rust crate and run under `cargo test` from the workspace root.

### 6.1 Extend `core/tests/signature_tests.rs`

**New tests to add:**

1. `row_signatures_distinguish_column_positions`

   * Build two 1×2 grids:

     * `grid1`: `A1 = Number(1.0)`, `B1 = Number(2.0)`
     * `grid2`: `A1 = Number(2.0)`, `B1 = Number(1.0)`
   * Assert:

     ```rust
     let sig1 = grid1.compute_row_signature(0);
     let sig2 = grid2.compute_row_signature(0);
     assert_ne!(sig1.hash, sig2.hash);
     ```

2. `col_signatures_distinguish_row_positions`

   * Build two 2×1 grids:

     * `grid1`: `A1 = 1`, `A2 = 2`
     * `grid2`: `A1 = 2`, `A2 = 1`
   * Assert:

     ```rust
     let sig1 = grid1.compute_col_signature(0);
     let sig2 = grid2.compute_col_signature(0);
     assert_ne!(sig1.hash, sig2.hash);
     ```

3. `row_signature_distinguishes_numeric_text_bool`

   * Three 1×1 grids:

     * `grid_num`: `A1 = Number(1.0)`
     * `grid_text`: `A1 = Text("1".into())`
     * `grid_bool`: `A1 = Bool(true)`
   * Assert:

     ```rust
     let num = grid_num.compute_row_signature(0).hash;
     let txt = grid_text.compute_row_signature(0).hash;
     let boo = grid_bool.compute_row_signature(0).hash;
     assert_ne!(num, txt);
     assert_ne!(num, boo);
     assert_ne!(txt, boo);
     ```

4. `row_signature_ignores_empty_trailing_cells`

   * `grid1`: `nrows = 1, ncols = 3`, only `A1 = Number(42.0)`.
   * `grid2`: `nrows = 1, ncols = 10`, only `A1 = Number(42.0)`.
   * Assert:

     ```rust
     let sig1 = grid1.compute_row_signature(0).hash;
     let sig2 = grid2.compute_row_signature(0).hash;
     assert_eq!(sig1, sig2);
     ```

5. `row_signature_golden_constant_small_grid`

   * Choose a simple but non-trivial row, e.g.:

     * `A1 = Number(1.0)`, `B1 = Text("x".into())`, `C1 = Bool(false)`
   * Compute its signature once with the new implementation and **record the resulting `u64`** as a constant in the test.
   * Assert:

     ```rust
     let sig = grid.compute_row_signature(0);
     assert_eq!(sig.hash, EXPECTED_GOLDEN_HASH);
     ```
   * Purpose: ensures that any future change to the hashing algorithm is intentional and will be noticed.

**Existing tests to keep (possibly lightly updated):**

* `identical_rows_have_same_signature`
* `different_rows_have_different_signatures`
* Any tests that check `compute_all_signatures` populates both `row_signatures` and `col_signatures`.

They should continue to pass under the new semantics; at most they may need minor adjustments if they relied on now-relaxed assumptions.

### 6.2 `core/tests/sparse_grid_tests.rs`

Existing test (from snippet):

* A test that:

  * Builds a 5×5 sparse grid.
  * Calls `compute_all_signatures()`.
  * Asserts:

    * `row_signatures.is_some()`
    * `col_signatures.is_some()`
    * Lengths equal to dimensions. 

No behavior change is required; this test should continue to pass unchanged. Optional enhancement (only if trivial):

* Add a sanity assertion that at least one row and one column hash is non-zero after computing signatures for a non-empty grid.

### 6.3 Documentation alignment checks

While not executable tests, the implementer should:

* Update the **Preprocessing** section of `2025-11-30-docs-vs-implementation.md` to mark:

  * “Column position in hash” as ✅ implemented.
  * “Type discriminant” as ✅ implemented and covered by tests.
  * “XXHash64/BLAKE3” as ✅ (XXHash64 implemented for Row/Column signatures).
* Leave “Numeric normalization”, “Frequency tables”, “Token interning”, and “Parallel hash computation” as ❌ or partial, to be addressed by future milestones.

---

## 7. Expected Milestone Outcome

After this cycle:

* `RowSignature` and `ColSignature` represent **spec-compliant, deterministic, position- and type-sensitive fingerprints** suitable for use in:

  * Anchor discovery.
  * Frequency analysis and classification.
  * Move detection and dimension ordering.
* The behavior is locked in by concrete unit tests that:

  * Catch regressions in position sensitivity, type handling, and determinism.
  * Provide a stable golden hash for at least one row.
* The Preprocessing section in the docs-vs-implementation review is updated to reflect this progress, and the next natural milestone becomes **frequency analysis + anchor selection**, building directly on these signatures.

