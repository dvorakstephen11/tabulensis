```markdown
# Verification Report: 2025-12-05b-gridview-layer1

## Summary

The GV1 GridView + HashStats preprocessing layer is correctly implemented and well covered by targeted tests. The behavior matches the unified grid diff specification: GridView constructs a stable row/column view with metadata derived from the existing hashing semantics, and HashStats implements the rare/common/unique classifications as documented. All scope items from the mini-spec are present, and the existing diff behavior and public APIs remain unchanged. The remaining issues are minor and limited to documentation clarity (a stale threshold value in the mini-spec) and a couple of missing “belt-and-suspenders” tests for degenerate grid shapes. No correctness bugs or performance regressions were found, and all prior remediation items have been addressed in code and tests.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### 1. Mini-spec HashStats threshold example is stale

- **Severity**: Minor  
- **Category**: Spec Deviation / Documentation  
- **Description**:  
  The GV1 mini-spec test plan still describes `hashstats_counts_and_positions_basic` using `threshold = 2` while treating a hash with frequencies `freq_a = 2`, `freq_b = 1` as “common”. :contentReference[oaicite:0]{index=0}  
  After remediation, the canonical semantics are:
  - `is_common(hash, threshold)` uses strict `>` (not `>=`) and ignores missing hashes. :contentReference[oaicite:1]{index=1}  
  - The actual test was updated to use `threshold = 1` so that the same frequencies are “common” under the strict `>` rule. :contentReference[oaicite:2]{index=2}  
- **Evidence**:  
  - Mini-spec HashStats section still narrates the example in terms of a generic `HashStats<H>` with a narrative threshold that was originally 2. :contentReference[oaicite:3]{index=3}  
  - Updated test uses `let threshold = 1;` and asserts `is_common(h2, threshold)` with the 2/1 frequencies. :contentReference[oaicite:4]{index=4}  
  - Unified spec and code both document/enforce the strict-`>` rare/common split.   
- **Impact**:  
  - No runtime or correctness impact: the implementation and tests are self-consistent and match the unified spec.  
  - A contributor reading only the mini-spec might copy the example literally and expect `threshold = 2` to classify `2/1` as “common” under strict-`>` semantics, which could cause confusion when extending tests or algorithms.

---

### 2. Degenerate GridView shapes are untested (but logically supported)

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  `GridView::from_grid` appears correct for all grid shapes, including:
  - `0 × 0` (explicitly tested),  
  - `nrows > 0, ncols = 0`,  
  - `nrows = 0, ncols > 0`.  

  For all of these, the construction algorithm allocates `rows`, `row_meta`, and `col_meta` based on `nrows`/`ncols`, then iterates `grid.cells` (which will be empty when one dimension is zero), and finally builds metadata via index-based access.   

  However, only the fully empty 0×0 case currently has an explicit unit test (`gridview_empty_grid_is_stable`). :contentReference[oaicite:7]{index=7}  
- **Evidence**:  
  - `GridView::from_grid` sizes all arrays from `grid.nrows`/`grid.ncols` and guards access with `debug_assert!` on cell coordinates, so grids with one dimension zero but no cells would follow the same code path as the tested 0×0 case.   
  - `grid_view_tests.rs` includes tests for:
    - Dense 3×3 grid,  
    - Sparse 4×4 grid with low-info classification,  
    - 4×4 grid for row/column signature alignment,  
    - 0×0 grid,  
    - 10 000×10 large sparse grid. :contentReference[oaicite:9]{index=9}  
    None cover `Grid::new(nrows > 0, 0)` or `Grid::new(0, ncols > 0)`.  
- **Impact**:  
  - Risk is low: the code for degenerate shapes is straightforward and mirrors the already-tested 0×0 case.  
  - If future changes alter `Grid::new` or how `Grid.cells` is populated for edge shapes, these paths could regress without being caught by tests. Adding two tiny tests would fully close this gap.

---

### 3. Minor wording mismatch: “No undocumented deviations” vs. mini-spec drift

- **Severity**: Minor  
- **Category**: Gap / Spec Deviation (documentation nit)  
- **Description**:  
  The project docs assert “no undocumented deviations from spec,” but we still have the minor mini-spec drift described in Finding 1. The unified algorithm spec and the implementation are aligned on HashStats semantics; only the GV1 mini-spec’s test description has an outdated threshold number.   
- **Evidence**:  
  - Combined remediation history explicitly calls out this lingering documentation inconsistency. :contentReference[oaicite:11]{index=11}  
  - The mini-spec’s test plan still names the old threshold, even though the accompanying narrative and unified spec now describe the strict-`>` semantics correctly.   
- **Impact**:  
  - This is purely a clarity issue. It doesn’t affect runtime behavior and is already documented in prior remediation notes.  
  - Cleaning it up would eliminate a small source of confusion for future readers who treat the mini-spec as the single source of truth for test parameters.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - `GridView<'a>`, `RowView<'a>`, `RowMeta`, `ColMeta`, and `HashStats` are implemented in `core/src/grid_view.rs` with the expected fields and roles. :contentReference[oaicite:13]{index=13}  
  - `Grid` hashing functions reuse `hash_cell_contribution` and `combine_hashes` from `core/src/hashing.rs`, preserving semantics while consolidating hashing logic.   
  - GridView-related types are exposed to tests via the public `excel_diff` API (tests import `GridView`, `RowMeta`, `HashStats`, etc. from `excel_diff`).   
  - No changes were made to `diff_workbooks`, `DiffOp`, JSON helpers, or the `Grid`/`Cell`/`CellValue` IR.   

- [x] All specified tests created  
  - `core/tests/grid_view_tests.rs` contains:
    - `gridview_dense_3x3_layout_and_metadata`,  
    - `gridview_sparse_rows_low_info_classification`,  
    - `gridview_column_metadata_matches_signatures`,  
    - `gridview_empty_grid_is_stable`,  
    - `gridview_large_sparse_grid_constructs_without_panic`,  
    - plus the additional `gridview_formula_only_row_is_not_low_info` introduced during remediation.   
  - `core/tests/grid_view_hashstats_tests.rs` contains:
    - `hashstats_counts_and_positions_basic`,  
    - `hashstats_rare_but_not_common_boundary`,  
    - `hashstats_equal_to_threshold_behavior`,  
    - `hashstats_empty_inputs`.   
  - `core/tests/signature_tests.rs` includes `gridview_rowmeta_hash_matches_compute_all_signatures`, which ties `RowMeta.hash`/`ColMeta.hash` back to `RowSignature`/`ColSignature`. :contentReference[oaicite:19]{index=19}  
  - Cycle summary shows all of these tests running and passing as part of the full suite.   

- [x] Behavioral contract satisfied  
  - GridView respects length and indexing invariants (`rows.len()`, `row_meta.len()`, `col_meta.len()` match grid dimensions; `row_idx`/`col_idx` equal their indices; `source` points back to the original `Grid`).   
  - `RowView.cells` contains exactly the populated cells in each row and is sorted by column index; tests assert correct `(row, col)` alignment and ordering for a dense 3×3 grid. :contentReference[oaicite:22]{index=22}  
  - `RowMeta.non_blank_count`, `first_non_blank_col`, and `is_low_info` match the documented behaviors across header-like, empty, numeric, whitespace-only, and formula-only rows.   
  - `ColMeta` hashes, counts, and `first_non_blank_row` match `Grid::compute_all_signatures()` and manual counts.   
  - `HashStats` rare/common/unique/appears-in-both semantics match the unified spec’s definitions and are directly locked in by unit tests.   

- [x] No undocumented deviations from spec (behavioral)  
  - Deviations from the original mini-spec are either:
    - Documented and justified in remediation notes (e.g., strict `>` semantics for `is_common`, RowHash-specific constructor),  or  
    - Purely documentation drift in the mini-spec example threshold (Finding 1), while the unified algorithm spec and implementation remain aligned.   

- [x] Error handling adequate  
  - `GridView::from_grid` relies on existing `Grid` invariants and adds `debug_assert!` bounds checks when traversing `grid.cells`, but performs no unchecked indexing that would panic under normal construction.   
  - No new fallible APIs were introduced; errors continue to be surfaced through existing parsing and I/O layers, which are already well covered by tests.   

- [x] No obvious performance regressions  
  - GridView construction performs a single pass over `grid.cells` plus per-row sorting, matching the O(M + R log(M/R)) design in the unified spec.   
  - Column metadata is computed without scanning the full `HashMap` per column, maintaining O(M + C) additional work.   
  - `HashStats::from_row_meta` is O(R_A + R_B) in time and O(U) in memory (no sorting, only single linear passes over row-meta slices).   
  - Full test suite (including the more expensive integration tests and large-grid GridView test) passes with no reported timeouts or regressions.   
```

Since all findings are minor and confined to documentation/test niceties, I recommend proceeding with the branch as-is. If you’d like, I can also sketch a small follow-up patch plan to clean up the mini-spec text and add the two degenerate-shape tests, but those are not blockers for release.
