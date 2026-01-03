
---

```markdown
# Remediation Plan: 2025-12-05b-gridview-layer1

## Overview

This remediation focuses on aligning `HashStats` semantics with the documented spec, clarifying the boundary between “rare” and “common” hashes, and tightening test coverage around those boundaries and low-info rows. No changes to external diff behavior (`diff_workbooks`, DiffOp, JSON) are required or expected.

## Fixes Required

### Fix 1: Align HashStats threshold semantics and classification

- **Addresses Finding**: Finding 1 (spec deviation and overlap of `is_rare`/`is_common`)
- **Changes**:

  1. **Decide canonical semantics** (recommended: match the unified algorithm spec):

     - `is_rare(hash, threshold)`:
       - `freq_a[hash] > 0 && freq_b[hash] > 0`
       - `freq_a[hash] <= threshold && freq_b[hash] <= threshold`
       - Not unique.
     - `is_common(hash, threshold)`:
       - `freq_a[hash] > threshold || freq_b[hash] > threshold`.

     This keeps rare and common disjoint at the threshold if you treat “rare but not unique” and “common” as mutually exclusive categories.

  2. **Update implementation in `core/src/grid_view.rs`**:

     - In `HashStats::is_common`, change the comparison from `>= threshold` to `> threshold`, keeping the “ignore missing hashes” early-return:

       ```rust
       // existing
       if freq_a == 0 && freq_b == 0 {
           return false;
       }

       freq_a >= threshold || freq_b >= threshold
       ```

       Replace with:

       ```rust
       if freq_a == 0 && freq_b == 0 {
           return false;
       }

       freq_a > threshold || freq_b > threshold
       ```

       (Leave `is_rare` logic intact; it already enforces `freq > 0` in both grids and `!is_unique`.):contentReference[oaicite:39]{index=39}

  3. **Reconcile the test example with the chosen semantics**:

     - In `hashstats_counts_and_positions_basic`, the current test uses `threshold = 2` and asserts `is_common(h2, threshold)` where `h2` has frequencies `2` in A and `1` in B.:contentReference[oaicite:40]{index=40}  
     - With the strict `>` semantics, that case is *not* “common” for `threshold = 2`, but would be “common” for `threshold = 1`.
     - Recommended adjustment:
       - Set `let threshold = 1;` in the test.
       - Keep `assert!(stats.is_common(h2, threshold));` as-is, so “above threshold” means “frequency 2/1 with threshold 1”.
     - Alternatively, if the team decides they truly want inclusive semantics for “common”, update:
       - The mini-spec and unified spec sections that describe `is_common` to use `>= threshold`.
       - Any higher-level documentation that refers to “> threshold”.
       - In that case, also adjust `is_rare` or downstream classification logic so that a given hash can never be both rare and common at the chosen threshold (e.g., define `rare` as `0 < freq <= threshold` and `!is_common`).

- **Tests**:

  - Update and re-run `core/tests/grid_view_hashstats_tests.rs` after the code change.
  - Confirm `hashstats_counts_and_positions_basic` still passes with the new threshold and semantics.

---

### Fix 2: Add explicit HashStats boundary tests

- **Addresses Finding**: Finding 2 (missing tests for rare/common boundaries)
- **Changes**:

  In `core/tests/grid_view_hashstats_tests.rs`:

  1. **Add a test that verifies “rare but not common”**:

     - Construct a scenario like:

       ```rust
       #[test]
       fn hashstats_rare_but_not_common_boundary() {
           let h: RowHash = 42;
           let rows_a = vec![row_meta(0, h), row_meta(1, h)];
           let rows_b = vec![row_meta(0, h)];
           let stats = HashStats::from_row_meta(&rows_a, &rows_b);

           let threshold = 2; // example, must match chosen semantics

           assert!(stats.is_rare(h, threshold));
           assert!(!stats.is_common(h, threshold));
           assert!(stats.appears_in_both(h));
           assert!(!stats.is_unique(h));
       }
       ```

       This locks in that a non-unique, low-frequency hash is “rare” but not “common”.

  2. **Add a test that exercises the equal-to-threshold case explicitly** (after semantics are finalized):

     - For example, a hash with `freq_a = threshold`, `freq_b = threshold`, and an assertion that matches the intended behavior:

       ```rust
       #[test]
       fn hashstats_equal_to_threshold_behavior() {
           let h: RowHash = 99;
           // Construct rows so freq_a == freq_b == threshold
           // ...
           let stats = HashStats::from_row_meta(&rows_a, &rows_b);
           let threshold = 3;

           // Example, if using strict '>' for common:
           assert!(stats.is_rare(h, threshold));
           assert!(!stats.is_common(h, threshold));
       }
       ```

- **Tests**:

  - Run `cargo test grid_view_hashstats_tests::` and the full suite to ensure no regressions.:contentReference[oaicite:42]{index=42}

---

### Fix 3: Add a low-info test for formula-only rows

- **Addresses Finding**: Finding 3 (missing test case for formula-only row)
- **Changes**:

  In `core/tests/grid_view_tests.rs`:

  1. Either extend `gridview_sparse_rows_low_info_classification` or add a new test, e.g.:

     ```rust
     #[test]
     fn gridview_formula_only_row_is_not_low_info() {
         let mut grid = Grid::new(2, 2);
         // Row 0: one formula-only cell
         grid.insert(make_cell(0, 0, None, Some("=A1+1")));

         let view = GridView::from_grid(&grid);

         assert_eq!(view.row_meta[0].non_blank_count, 1);
         assert!(!view.row_meta[0].is_low_info);
     }
     ```

     This directly exercises the `(_, Some(_)) => false` branch in `compute_is_low_info`.

- **Tests**:

  - Re-run `grid_view_tests.rs` (and full test suite) to confirm behavior and performance.

---

### Fix 4 (Optional): Clarify HashStats API shape in docs

- **Addresses Finding**: Finding 4 (spec/API divergence)
- **Changes**:

  - If the current RowHash-specific constructor is intentional for GV1, update the mini-spec’s HashStats section to indicate:

    - `HashStats<RowHash>` is the only instantiation currently supported.
    - `from_row_meta` is specifically `HashStats<RowHash>::from_row_meta(...)`.

  - Alternatively, if the team wants strict adherence to the generic API in the spec, refactor:

    ```rust
    impl<H> HashStats<H>
    where
        H: Eq + Hash + Copy,
    {
        pub fn from_row_meta(
            rows_a: &[RowMetaGeneric<H>],
            rows_b: &[RowMetaGeneric<H>],
        ) -> HashStats<H> { ... }
    }
    ```

    (or an equivalent pattern that still works with `RowMeta` and `RowHash` without breaking existing users).

- **Tests**:

  - No behavioral tests needed; this is documentation/API-shape alignment. Ensure existing builds still pass.

---

## Constraints

- Do **not** change:
  - `diff_workbooks`, `diff_workbooks_to_json`, DiffOp, or any JSON schema.
  - The structure of `Grid`, `Cell`, or `CellValue`.
- Keep:
  - `HashStats` public API surface compatible with any downstream consumers within this crate (no renames/removals of public methods for now).
  - Test fixtures and golden JSON outputs unchanged.
- Maintain:
  - Deterministic hashing behavior (no change to `hash_cell_contribution`, `combine_hashes`, or seeds).

## Expected Outcome

After remediation:

1. `HashStats` semantics for rare/common hashes exactly match the chosen canonical definition, and that definition is consistently reflected in:
   - `core/src/grid_view.rs`
   - `docs/rust_docs/unified_grid_diff_algorithm_specification.md`
   - `cycle_plan.md` (GV1 mini-spec).
2. Rare and common classifications are unambiguous at the threshold and protected by explicit unit tests.
3. Low-information classification has direct test coverage for formula-only rows, in addition to existing header/empty/numeric/whitespace cases.
4. GV1 remains non-invasive: all existing workbook, diff, and PG tests still pass unchanged, confirming that no external behavior was affected by these internal refinements.
