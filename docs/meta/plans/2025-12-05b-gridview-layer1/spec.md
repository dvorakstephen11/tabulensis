# 2025-12-05-gridview-layer1 – GridView preprocessing layer

## 1. Scope

### 1.1 Goal

Introduce the **GridView** preprocessing layer described in the unified grid diff specification (Layer 1), including:

- An ephemeral GridView structure that provides row-oriented and column-oriented access over the existing sparse `Grid`.
- Row and column metadata (`RowMeta`, `ColMeta`) based on existing row/column hashing semantics.
- A small `HashStats` structure for row hash frequency analysis across the two input grids.

This cycle **does not change** visible diff behavior (`diff_workbooks`, `DiffOp`) and does **not** implement advanced alignment (anchors, moves, database mode). It prepares the data structures required for Phase 4 grid milestones.

### 1.2 Modules and types in play

New:

- `core/src/grid_view.rs`
  - `pub struct GridView<'a>`
  - `pub struct RowView<'a>`
  - `pub struct RowMeta`
  - `pub struct ColMeta`
  - `pub struct HashStats<H>`
  - Constructors and helpers (e.g., `GridView::from_grid`, `HashStats::from_row_meta`)

Modified:

- `core/src/workbook.rs`
  - Reuse hashing helpers (`hash_cell_contribution`, `combine_hashes`) via imports.
  - Optionally provide a small helper to build `GridView` (e.g., `Grid::view()`), but **must not** change `Grid`’s external semantics.
- `core/src/lib.rs`
  - Re-export `GridView`, `RowMeta`, `ColMeta` for tests via a public or `pub(crate)`-guarded API, as appropriate.
- `core/tests/signature_tests.rs`
  - Add tests that assert consistency between `RowMeta.hash`/`ColMeta.hash` and existing `RowSignature`/`ColSignature`.

New tests:

- `core/tests/grid_view_tests.rs`
  - Focused unit tests for GridView construction and metadata invariants.
- (Optional but encouraged) `core/tests/grid_view_hashstats_tests.rs`
  - Unit tests for HashStats frequency/counting behavior.

### 1.3 Related milestones

- Existing plan: Phase 4 “Algorithmic heavy lifting (adversarial grids) | M7, G8–G13, D1–D10” in excel_diff_testing_plan.md.
- This cycle defines a new **incremental milestone**:

> **GV1 – GridView preprocessing layer (Row/Column metadata + HashStats)**  
> Sits between Phase 3 (G1–G7 basic grid tests) and Phase 4 (G8–G13 + D1–D10). It validates the data structures and invariants required for later alignment, without yet changing the externally observable diff.

Future cycles targeting G8+ and D-series tests will treat GV1 as a prerequisite.

---

## 2. Behavioral Contract

Although GridView is an internal structure, we treat its behavior as a contract because all future grid-diff algorithms depend on it.

### 2.1 GridView structure and basic invariants

Given a `Grid` with dimensions `(nrows, ncols)` and sparse cell map `cells: HashMap<(row, col), Cell>`:

- `GridView::from_grid(&grid)` produces a `GridView<'_>` such that:
  - `view.rows.len() == grid.nrows as usize`
  - `view.row_meta.len() == grid.nrows as usize`
  - `view.col_meta.len() == grid.ncols as usize`
  - `view.source` points to the same `Grid` (no copying of `Cell` contents).
- For each row index `r` in `[0, nrows)`:
  - `view.rows[r].cells` contains **exactly** the non-empty cells from that row as `(col_index, &Cell)` pairs.
  - `view.rows[r].cells` is sorted by `col_index` ascending.
  - `view.row_meta[r].row_idx == r as u32`.

For each column index `c` in `[0, ncols)`:

- `view.col_meta[c].col_idx == c as u32`.

No row or column is “skipped” – empty rows/columns just have zero counts and zero hashes.

#### Example: Dense 3×3 numeric grid

Grid built via `grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]])`:

- `view.rows[0].cells` ≅ `[(0, &Cell{row:0,col:0}), (1, &Cell{row:0,col:1}), (2, &Cell{row:0,col:2})]`
- `view.row_meta[0].non_blank_count == 3`
- `view.row_meta[0].first_non_blank_col == 0`
- `view.row_meta[0].is_low_info == false`
- Similarly for rows 1 and 2.
- `view.col_meta[0].non_blank_count == 3`, `first_non_blank_row == 0` (and analogous for other columns).

#### Example: Sparse grid with some empty rows

Grid:

- 5 rows × 4 columns.
- Cells at:
  - (0, 1): `"Header"`
  - (2, 0): `42`
  - (4, 3): `"tail"`

Then:

- `view.rows[1].cells` is empty; `view.row_meta[1].non_blank_count == 0`, `is_low_info == true`.
- `view.row_meta[0].first_non_blank_col == 1`.
- `view.row_meta[2].first_non_blank_col == 0`.
- `view.row_meta[4].first_non_blank_col == 3`.
- Column metadata reflects where each column starts:
  - Column 0: `non_blank_count == 1`, `first_non_blank_row == 2`.
  - Column 1: `non_blank_count == 1`, `first_non_blank_row == 0`.
  - Column 3: `non_blank_count == 1`, `first_non_blank_row == 4`.

### 2.2 Row and column hashing semantics

Row and column metadata must use the **same hashing model** as existing signatures:

- For each row `r`, `RowMeta.hash` is computed using the same per-cell contribution and reduction as `RowSignature.hash` in workbook.rs / hashing.rs.
- For each column `c`, `ColMeta.hash` uses the same semantics as `ColSignature.hash`.

Contract:

- If `grid.compute_all_signatures()` has been called, then for every row/column:

  ```rust
  RowMeta.hash == grid.row_signatures.as_ref().unwrap()[r].hash
  ColMeta.hash == grid.col_signatures.as_ref().unwrap()[c].hash
````

* Even if signatures were not precomputed, computing them after building the view must produce the same hash values (`compute_row_signature`/`compute_col_signature` over the grid’s cells match RowMeta/ColMeta hashes).

Row hashing rules:

* Empty cells (no value and no formula) are excluded from hashing, matching the commutative hashing model used for signatures.
* Cell value type, numeric value, string contents, boolean, and formula are all included in the hash contribution.
* Position is encoded as specified in hashing.rs (column index for row hashes, row index for column hashes).

### 2.3 Non-blank counts and low-information rows

For each row:

* `RowMeta.non_blank_count` is the number of cells in that row whose logical content is not empty (`CellValue::Empty` with no formula). Cells with formulas or non-empty values count as non-blank.
* `RowMeta.first_non_blank_col` is the smallest column index among non-blank cells, or `0` when `non_blank_count == 0`.

Low-information classification:

* `RowMeta.is_low_info == true` when:

  * `non_blank_count == 0` (completely blank row), or
  * `non_blank_count == 1` and the single non-blank cell is “trivial content”:

    * Value is empty or a string containing only whitespace (after trimming).
    * No numeric or boolean values count as trivial.
* Otherwise, `is_low_info == false`.

#### Example: template-style sheet

Rows:

1. Row 0: `"Customer Report"` label in A1; all other cells empty.
2. Row 1: all empty.
3. Row 2: real data in columns A–D.

Classification:

* Row 0: `non_blank_count == 1`. If the value is non-whitespace text, `is_low_info == false`. (This row is likely a meaningful header).
* Row 1: `non_blank_count == 0`, `is_low_info == true`.
* Row 2: `non_blank_count >= 1`, `is_low_info == false`.

This contract allows future anchor-finding logic to ignore obviously blank rows and rows with only whitespace while still considering rows that carry real labels or data.

### 2.4 Column metadata

Column metadata mirrors rows:

* `ColMeta.non_blank_count` counts non-empty cells in the column.
* `ColMeta.first_non_blank_row` is the smallest row index with a non-empty cell in that column, or `0` if `non_blank_count == 0`.
* `ColMeta.hash` is the column fingerprint as defined above.

Example from the 5×4 sparse grid:

* Column 1 (header only):

  * `non_blank_count == 1`
  * `first_non_blank_row == 0`
  * `hash` matches `grid.compute_col_signature(1).hash`.

### 2.5 HashStats behavior

For a pair of GridViews `(view_a, view_b)` built from two grids:

* `HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta)` produces:

  * `freq_a: HashMap<RowHash, u32>` – count of each row hash in A.
  * `freq_b: HashMap<RowHash, u32>` – count of each row hash in B.
  * `hash_to_positions_b: HashMap<RowHash, Vec<u32>>` – sorted list of row indices in B where each hash appears.

Derived behaviors:

* `is_unique(hash)` returns true iff that hash appears exactly once in A and once in B.
* `is_rare(hash, threshold)` returns true iff:

  * `freq_a[hash] > 0` and `freq_b[hash] > 0`, and
  * `freq_a[hash] <= threshold` and `freq_b[hash] <= threshold`, and
  * The hash is **not** unique.
* `is_common(hash, threshold)` returns true iff:

  * `freq_a[hash] > threshold` or `freq_b[hash] > threshold`.

Example:

* Grid A row hashes: `[h1, h2, h2, h3]` → freq_a `{h1:1, h2:2, h3:1}`
* Grid B row hashes: `[h2, h3, h4]` → freq_b `{h2:1, h3:1, h4:1}`
* With threshold = 2:

  * `h1`: freq_a=1, freq_b=0 → appears only in A (unmatched).
  * `h2`: freq_a=2, freq_b=1 → `is_common(h2) == true` (above threshold in A).
  * `h3`: freq_a=1, freq_b=1 → unique row candidate.
  * `h4`: freq_a=0, freq_b=1 → appears only in B (unmatched).

This classification will be used in later cycles to drive anchor selection and gap handling.

---

## 3. Constraints

### 3.1 Performance and complexity

* Let `M` = number of non-empty cells, `R` = number of rows, `C` = number of columns.

* GridView construction must satisfy:

  * **Time**:

    * O(M + R log(M/R)) as outlined in the unified spec:

      * Distributing cells into per-row vectors is O(M).
      * Sorting each row’s cells contributes the `log(M/R)` factor.
      * Column metadata computation may be O(R·C) as long as it uses row-local searches (e.g., binary search within each row’s sorted cells) and doesn’t scan full `HashMap`s per column.

  * **Memory**:

    * Additional memory is O(M + R + C) for:

      * RowView cell vectors (one entry per non-empty cell).
      * RowMeta and ColMeta arrays.
      * Vec overhead for per-row cell vectors.
    * No structures may allocate O(R × C) space.

* HashStats construction must be O(R_A + R_B)` time and O(U)` memory, where `U` is the number of unique row hashes across both grids, consistent with Section 7 of the unified spec.

### 3.2 Memory lifecycle

* GridView and HashStats are **ephemeral**:

  * Constructed in grid diff preprocessing (future Phase 1).
  * Dropped after dimension decision and alignment, as per the memory lifecycle table (GridViews released after Phase 5, HashStats after Phase 4).
* This cycle does not yet wire GridView into `diff_workbooks`, but the public API must be designed so that future phases can follow the documented lifecycle without changing Grid/Workbook IR.

### 3.3 Invariants and safety

* Grid invariants from workbook.rs remain in force:

  * All cells `(row, col)` in `Grid.cells` satisfy `row < nrows` and `col < ncols`.
* GridView must assume these invariants and may use `debug_assert!` to enforce them while building rows.
* GridView must never mutate the underlying `Grid` or its `cells` during construction.
* Hashing semantics must remain deterministic across platforms and Rust versions, reusing the existing XXHash64-based helpers with the fixed seed.

---

## 4. Interfaces

### 4.1 New types and functions (proposed signatures)

Exact naming can be adjusted by the implementer as long as semantics are preserved, but this cycle expects something very close to:

```rust
// core/src/grid_view.rs

use crate::workbook::{Cell, Grid};

pub type RowHash = u64;
pub type ColHash = u64;

pub struct RowView<'a> {
    pub cells: Vec<(u32, &'a Cell)>, // sorted by column index
}

pub struct RowMeta {
    pub row_idx: u32,
    pub hash: RowHash,
    pub non_blank_count: u16,
    pub first_non_blank_col: u16,
    pub is_low_info: bool,
}

pub struct ColMeta {
    pub col_idx: u32,
    pub hash: ColHash,
    pub non_blank_count: u16,
    pub first_non_blank_row: u16,
}

pub struct GridView<'a> {
    pub rows: Vec<RowView<'a>>,
    pub row_meta: Vec<RowMeta>,
    pub col_meta: Vec<ColMeta>,
    pub source: &'a Grid,
}

impl<'a> GridView<'a> {
    pub fn from_grid(grid: &'a Grid) -> GridView<'a>;
}
```

Hash statistics:

```rust
use std::collections::HashMap;

pub struct HashStats<H> {
    pub freq_a: HashMap<H, u32>,
    pub freq_b: HashMap<H, u32>,
    pub hash_to_positions_b: HashMap<H, Vec<u32>>,
}

impl<H> HashStats<H>
where
    H: Eq + std::hash::Hash + Copy,
{
    pub fn from_row_meta(
        rows_a: &[RowMeta],
        rows_b: &[RowMeta],
    ) -> HashStats<H>;

    pub fn is_unique(&self, hash: H) -> bool;
    pub fn is_rare(&self, hash: H, threshold: u32) -> bool;
    pub fn is_common(&self, hash: H, threshold: u32) -> bool;
    pub fn appears_in_both(&self, hash: H) -> bool;
}
```

Exports:

* `core/src/lib.rs` should expose GridView and metadata types at least to tests. Two options:

  * Public API (if we want external callers to use GridView later).
  * `pub use` behind `#[cfg(test)]` or `pub(crate)` plus explicit test module imports.
* No changes to:

  * `diff_workbooks` signature.
  * `DiffOp`, `DiffReport`, or JSON helpers.

### 4.2 Stable vs changeable interfaces in this cycle

Stable (must not change in this cycle):

* `Workbook`, `Sheet`, `Grid`, `Cell`, `CellValue` structures and their public fields.
* `diff_workbooks` and `diff_workbooks_to_json` signatures and behavior.
* `RowSignature`, `ColSignature` structs and how they are serialized in JSON (PG4).

Changeable / new:

* Introduction of GridView and related types.
* Internal helpers on `Grid` if useful (`fn view(&self) -> GridView<'_>`).
* New modules and tests.

---

## 5. Test Plan

All tests for this cycle are grouped under the new incremental milestone:

> **GV1 – GridView preprocessing layer**

### 5.1 New unit tests

File: `core/tests/grid_view_tests.rs`

1. **`gridview_dense_3x3_layout_and_metadata`**

   * Build a 3×3 grid via `grid_from_numbers(&[&[1,2,3], &[4,5,6], &[7,8,9]])`.
   * Construct `GridView::from_grid(&grid)`.
   * Assert:

     * `rows.len() == 3`, `row_meta.len() == 3`, `col_meta.len() == 3`.
     * Each `RowView.cells` has 3 entries with correct `(col, &Cell)` pairs in ascending order.
     * `RowMeta.non_blank_count == 3` and `first_non_blank_col == 0` for all rows.
     * `ColMeta.non_blank_count == 3` and `first_non_blank_row == 0` for all columns.

2. **`gridview_sparse_rows_low_info_classification`**

   * Construct a grid with:

     * Row 0: A1 = `"Header"` (non-whitespace text).
     * Row 1: all empty.
     * Row 2: one numeric cell.
     * Row 3: A1 = `"   "` (whitespace-only string).
   * Build `GridView`.
   * Assert:

     * Row 0: `non_blank_count == 1`, `is_low_info == false`.
     * Row 1: `non_blank_count == 0`, `is_low_info == true`.
     * Row 2: `non_blank_count == 1`, `is_low_info == false`.
     * Row 3: `non_blank_count == 1`, `is_low_info == true`.
     * `first_non_blank_col` matches the positions of the non-empty cells.

3. **`gridview_column_metadata_matches_signatures`**

   * Build a 4×4 grid with non-empty cells in several columns and rows.
   * Call `grid.compute_all_signatures()`.
   * Build `GridView`.
   * For each column `c`:

     * Assert `view.col_meta[c].hash == grid.col_signatures.as_ref().unwrap()[c].hash`.
     * Assert `non_blank_count` and `first_non_blank_row` match a manual count.
   * Similarly, choose a subset of rows and assert `RowMeta.hash == row_signatures[r].hash` and `non_blank_count` matches.

4. **`gridview_empty_grid_is_stable`**

   * `Grid::new(0, 0)` with no cells.
   * Build `GridView`.
   * Assert:

     * `rows.len() == 0`, `row_meta.len() == 0`, `col_meta.len() == 0`.
     * No panics.

5. **`gridview_large_sparse_grid_constructs_without_panic`**

   * Build a grid with e.g. `nrows = 10_000`, `ncols = 10`, and ~1% density (a few cells per hundred rows) using a simple loop.
   * Build `GridView`.
   * Assert:

     * `rows.len() == 10_000`.
     * A small sample of row_meta and col_meta have sensible counts (e.g., rows with cells have non-zero `non_blank_count`).
   * Purpose: basic sanity check that the memory shape is reasonable and construction runs in acceptable time; no explicit timing asserts.

### 5.2 Signature alignment tests

File: `core/tests/signature_tests.rs` (extend existing suite)

6. **`gridview_rowmeta_hash_matches_compute_all_signatures`**

   * Use an existing grid setup from signature tests (including formulas and mixed value types).
   * Call `grid.compute_all_signatures()`.
   * Build `GridView`.
   * For all rows:

     * Assert `RowMeta.hash == grid.row_signatures.as_ref().unwrap()[r].hash`.
   * For all columns:

     * Assert `ColMeta.hash == grid.col_signatures.as_ref().unwrap()[c].hash`.

This test locks in the equivalence of GridView hashing and the existing signature computation.

### 5.3 HashStats tests

File: `core/tests/grid_view_hashstats_tests.rs` (new)

7. **`hashstats_counts_and_positions_basic`**

   * Construct two in-memory vectors of `RowMeta` with hashes like:

     * A: `[h1, h2, h2, h3]`
     * B: `[h2, h3, h4]`
   * Build `HashStats::from_row_meta`.
   * Assert:

     * `freq_a[h1] == 1`, `freq_b[h1] == 0`.
     * `freq_a[h2] == 2`, `freq_b[h2] == 1`.
     * `freq_a[h3] == 1`, `freq_b[h3] == 1`.
     * `freq_a[h4] == 0`, `freq_b[h4] == 1`.
     * `hash_to_positions_b[h2] == vec![0]`, `hash_to_positions_b[h3] == vec![1]`, `hash_to_positions_b[h4] == vec![2]`.
   * With `threshold = 2`, assert:

     * `is_unique(h3) == true`.
     * `is_common(h2, threshold) == true`.
     * `is_rare(h3, threshold) == false` (because it is unique, not rare).
     * `appears_in_both(h3) == true`.
     * `appears_in_both(h1) == false` and `appears_in_both(h4) == false`.

8. **`hashstats_empty_inputs`**

   * Pass empty slices for A and B.
   * Assert:

     * All maps are empty.
     * `is_unique`, `is_rare`, `is_common`, `appears_in_both` all behave sensibly (e.g., return false for a dummy hash).

### 5.4 Out of scope / non-goals for this cycle

* No changes to `diff_grids` behavior:

  * Existing PG5 and G1–G7 tests must continue to pass unchanged.
  * No new row/column insert/move detection is implemented here.
* No integration of GridView into the production diff pipeline yet:

  * Subsequent cycles will:

    * Hook GridView + HashStats into Phase 1 preprocessing.
    * Implement dimension decision (row-first vs column-first).
    * Implement AMR alignment and move detection.
    * Add G8+ and D-series tests and fixtures.

---

This mini-spec defines the GV1 incremental milestone for the grid diff engine: a tested, efficient GridView preprocessing layer with row/column metadata and hash frequency analysis, ready to support Phase 4 alignment algorithms without altering current diff semantics.
