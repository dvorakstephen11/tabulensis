Here’s how I’d tackle the remaining Branch 1 work, based on the *current* code vs the plan in `next_sprint_plan.md`.

---

## 0. Quick recap of what’s actually left

From comparing `next_sprint_plan.md` Branch 1 with the code and tests:

* **1.1 (masks / silent data loss)** – implemented: `RegionMask`, masked move detectors, iterative masking loop in `diff_grids_with_config`, plus g14 tests that exercise “moves + outside edits” and multi-move scenarios.
* **1.2 (multi-move iteration)** – implemented: loop over `max_move_iterations`, `DiffConfig::max_move_iterations`, and tests for multiple disjoint moves and combinations.
* **1.3–1.5 (hashing & floats)** – **mostly** implemented:

  * 128‑bit row/column signatures with xxHash3; row/col meta wired up; docs in `hashing.rs`.
  * Row/col hashes no longer depend on position indexes for signatures (content-only in sorted order).
  * `normalize_float_for_hash` exists and has the unit tests specified in the plan.

But a few **gaps remain** relative to Branch 1’s specification:

1. **`CellValue::Number` hashing still uses `f64::to_bits()`**, not normalized floats.
2. **Database-mode key hashing still uses raw `to_bits()`** via `KeyValueRepr::Number`, so DB diffs don’t share the float semantics used by row/col signatures.
3. **Branch 1 does not yet have end‑to‑end tests for “recalc noise”** (1.0 vs 1.0000000000000002, etc.) at the diff level; the tests are only at the `normalize_float_for_hash` helper level.
4. **The hash-collision doc in `row_alignment.rs` still references 64‑bit collision rates** instead of the new 128‑bit scheme.
5. **The `hash_cell_contribution` function is deprecated but still position‑dependent**, which contradicts the plan’s wording (“refactor … to exclude column index”). It’s unused in grid diff, but still a source of confusion.
6. **The specific 1.3 tests around “column insert/delete → row alignment still succeeds” are not present as such**, even though higher‑level column insert/delete cases are covered (g5–g9 tests).

Everything else in Branch 1 is either implemented or covered by existing tests.

The plan below is focused on fixing these six items.

---

## 1. Normalize floats in `CellValue::Number` hashing (grid mode)

### 1.1. Wire `normalize_float_for_hash` into `CellValue`’s `Hash`

**Goal:** Match the plan’s deliverable “Use normalized value in `CellValue::Number` hashing,” so any code that hashes a `CellValue` directly (including future uses) sees the same semantics as row/column signatures.

**Where:**

* `core/src/hashings.rs` – `normalize_float_for_hash` and `hash_cell_value`. 
* `core/src/workbook.rs` – `impl Hash for CellValue`. 

**Steps:**

1. **Change `CellValue::Number` hashing to call `normalize_float_for_hash`:**

   * In `impl Hash for CellValue`, replace the `Number` arm’s `n.to_bits().hash(state)` with `normalize_float_for_hash(*n).hash(state)`.
   * Import `crate::hashing::normalize_float_for_hash` at the top of `workbook.rs` (or a local `use` inside the impl to avoid broad exposure).

2. **Ensure there is *one* single float hashing story:**

   * Confirm `hash_cell_value` in `hashing.rs` is already using `normalize_float_for_hash` (it is). 
   * After the change, both:

     * `hash_cell_value(&cell.value, ...)` and
     * `cell.value.hash(&mut hasher)`
       will be consistent for numbers.

3. **Add a small unit test for `CellValue` hashing itself:**

   * In `workbook.rs` tests (or a new `cellvalue_hash_tests` module), add tests like:

     * `CellValue::Number(0.0)` and `CellValue::Number(-0.0)` produce the same hash.
     * `CellValue::Number(1.0)` and `CellValue::Number(1.0000000000000002)` produce the same hash.
     * `CellValue::Number(1.0)` and `CellValue::Number(1.0001)` produce different hashes.
   * Use a standard `Hasher` implementation from `std::collections::hash_map::DefaultHasher` for these tests.

This directly fulfills the Branch 1 “use normalized value in `CellValue::Number` hashing” checkbox and aligns with the Option‑A spec in `next_sprint_plan.md`.

---

## 2. Normalize floats in database mode keys

### 2.1. Key normalization in `KeyValueRepr::Number`

**Goal:** Ensure database-mode diff (`diff_table_by_key`) shares the same float semantics as the grid diff, so you don’t get spurious key mismatches due to ULP noise.

**Where:**

* `core/src/database_alignment.rs` – `KeyValueRepr::Number(u64)` plus `from_cell_value` uses `n.to_bits()`. 

**Steps:**

1. **Change `from_cell_value` for `Number`:**

   * Instead of `KeyValueRepr::Number(n.to_bits())`, use `KeyValueRepr::Number(normalize_float_for_hash(*n))`.
   * Import `normalize_float_for_hash` from `crate::hashing`.

2. **Add database-mode tests:**

   In `core/tests/d1_database_mode_tests.rs` (or a new database test file):

   * Build two grids with a key column where:

     * Left has values `[1.0, 2.0, 3.0]`
     * Right has `[1.0000000000000002, 2.0, 3.0]`
   * Run `diff_table_by_key(&grid_a, &grid_b, &[0])` and assert:

     * All rows are matched, no left‑only or right‑only rows.
   * Add a contrasting test where the difference is “large” (e.g. 1.0 vs 1.0001) and assert keys are not equal.

This extends Branch 1’s float semantics to the DB alignment path, which isn’t explicitly called out in the plan but keeps the engine self-consistent.

---

## 3. End‑to‑end tests for float “recalc noise”

Right now, the float normalization is well tested at the helper level (`normalize_float_for_hash` unit tests), but Branch 1’s acceptance criteria talk about **diff behavior**, not just the helper.

### 3.1. Workbook‑mode tests

**Where:**

* `core/tests/g1_g2_grid_workbook_tests.rs` or `g0_smoke_tests.rs` style modules.

**Steps:**

1. **Add “no-op diff under ULP noise” tests:**

   * Construct `old` and `new` workbooks with one sheet, one numeric cell:

     * Old: `1.0`
     * New: `1.0000000000000002`
   * Call `diff_workbooks(&old, &new)` and assert that `report.ops.is_empty()`.

2. **Add “real edit vs ULP tolerance” test:**

   * Old: `1.0`
   * New: `1.0001`
   * Assert there is exactly one `DiffOp::CellEdited` on that cell.

3. **Add a NaN case for coverage:**

   * Old: `f64::from_bits(0x7ff8_0000_0000_0000)`
   * New: `f64::NAN`
   * Assert: no diff ops – they should be treated as equal values.

   (JSON serialization already rejects non‑finite numbers; this test is about internal diff, not JSON.)

### 3.2. Grid‑level row signature sanity test

**Where:**

* `core/tests/grid_view_hashstats_tests.rs` or a small new test module.

**Steps:**

1. Build two `Grid`s with the same shape and identical structure except for a cell pair `(old=1.0, new=1.0000000000000002)` in the same position.
2. Build `GridView::from_grid` for each.
3. Assert that the row(s) containing those cells have identical `RowMeta.hash` values in both views.

This validates that row signatures themselves are tolerant to ULP drift and bolsters the acceptance criteria around “float comparison is semantically correct.”

---

## 4. Update hash collision documentation to 128‑bit

Branch 1 acceptance criteria explicitly mention documenting collision probability at 50K rows. `hashing.rs` already has a good 128‑bit explanation, but `row_alignment.rs` still contains an old 64‑bit `_HASH_COLLISION_NOTE`.

### 4.1. Fix `_HASH_COLLISION_NOTE` and centralize the story

**Where:**

* `core/src/row_alignment.rs` – `_HASH_COLLISION_NOTE` still mentions “64‑bit hash collision probability … at 2K rows”. 

**Steps:**

1. **Update `_HASH_COLLISION_NOTE`:**

   * Replace the text with something aligned to the 128‑bit analysis in `hashing.rs`, e.g.:

     * 128‑bit xxHash3 collision probability ~10⁻²⁹ at 50k rows (birthday bound).
   * Make sure it explicitly states that secondary verification is not currently required given this margin.

2. **Add a short pointer from `row_alignment.rs` comment to the detailed doc in `hashing.rs`:**

   * Example: “See `hashing.rs` for full collision analysis and rationale.”

3. **Double‑check that the acceptance statement in `next_sprint_plan.md` (“<10^-18 at 50K rows”) is satisfied:**

   * The documented 10⁻²⁹ bound is stricter than 10⁻¹⁸, so you’re good; just mention that in the comment.

No functional changes here, just aligning docs with reality and ticking the acceptance checkbox.

---

## 5. Clean up / replace `hash_cell_contribution`

`hash_cell_contribution(position, cell)` is deprecated and no longer used by row/column signatures, but it still hashes the position index and still exists in the module.

### 5.1. Decide: remove or refactor

**Goal:** Satisfy the spirit of “Refactor `hash_cell_contribution` to exclude column index” and prevent future accidental use of position-dependent semantics.

**Options:**

1. **If it’s truly unused in the crate:**

   * Delete it completely from `hashing.rs`.
   * Rely only on `hash_cell_content` / `hash_cell_content_128` and the row/col content hashers.

2. **If you want to keep a “cell fingerprint” utility:**

   * Re‑implement `hash_cell_contribution` as a simple wrapper:

     * Drop the `position` argument.
     * Delegate to `hash_cell_content` (or a renamed `hash_cell_value + formula` helper).
   * Update the deprecation note (or remove the deprecation if you decide to keep it as a first-class helper).

Given the current code, deletion is probably the cleanest: the new API surface (`hash_row_content_128`, `hash_col_content_128`, `hash_cell_content`) already covers all uses.

---

## 6. Add explicit tests for the “column insert/delete → row alignment still succeeds” bullets

The Branch 1 plan calls out two specific tests:

* Insert column at position 0 → row alignment still succeeds
* Delete column from middle → row alignment still succeeds 

The engine already behaves correctly at the **diff** level for column insert/delete (g7 and g9 test suites), but you don’t have tests written in the exact Branch 1 style.

### 6.1. Row alignment behavior under column count mismatch

`align_row_changes` already returns `None` when `old.ncols != new.ncols`, which prevents row alignment from misfiring when columns differ. 

To make the Branch 1 intent explicit:

1. **Unit test `align_row_changes` directly:**

   In `row_alignment.rs`’s test module:

   * Build `grid_a` with shape (N rows, M columns).
   * Build `grid_b` with:

     * Same rows/content plus an **extra blank column at index 0**.
   * Call `align_row_changes(&grid_a, &grid_b)` and assert it returns `None` (i.e., row alignment declines to run because of column mismatch).

   Add a symmetric test for deleting a column from the middle.

2. **Tie this back to the “does not break row alignment” acceptance criterion:**

   * This shows that column insertion/deletion does *not* cause row_alignment to produce incorrect matches; it simply declines, and the surrounding diff engine (plus column alignment logic) handles the structural change instead. The g7/g9 tests already confirm the behavior at the `DiffOp` level.

3. **(Optional) Add a column‑insert integration test that asserts both:**

   * The resulting `DiffOp` list matches expectations (`ColumnAdded` or a fallback `CellEdited` pattern, as appropriate).
   * `align_row_changes` is **not** used (this is harder to test directly; you can rely on unit tests instead of reaching into internals).

This closes the loop on the 1.3 “row alignment still succeeds” language: in the current design, “succeeds” means “doesn’t misalign due to column noise; yields either a correct alignment or opts out.” The tests will encode that explicitly.

---

## 7. Final verification pass

Once the changes above are implemented:

1. **Run the full test suite** (including DB-mode tests if you add them).

2. **Re‑check Branch 1 acceptance criteria manually:**

   * All existing tests pass.
   * No silent data loss (g14 tests, etc.).
   * Multiple moves per sheet detected (g11/g12/g14 tests).
   * Column insertion/deletion covered by g5–g9 tests *and* explicit row-alignment unit tests.
   * Float comparison semantically correct: new end‑to‑end float tests pass.
   * Hash collision probability documented at 128‑bit strength and below the <10⁻¹⁸@50k rows threshold (comment updated).


