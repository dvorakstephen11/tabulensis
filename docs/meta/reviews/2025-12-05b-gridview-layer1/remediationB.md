```markdown
# Verification Report: 2025-12-05b-gridview-layer1

## Summary

The GridView preprocessing layer and HashStats implementation closely match the mini-spec and unified grid diff specification. All planned code and tests are present, prior remediation items have been implemented, and the existing external behavior (`diff_workbooks`, JSON outputs, G1–G7 tests) remains unchanged and fully passing. I found only a handful of minor issues: a small complexity mismatch in `HashStats::from_row_meta`, a test-plan vs implementation detail mismatch around one threshold example, and a couple of edge cases that are not yet explicitly tested but are low risk. No correctness or safety issues surfaced that would block use of this branch.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### 1. HashStats construction complexity vs mini-spec wording

- **Severity**: Minor  
- **Category**: Spec Deviation / Performance  
- **Description**:  
  The mini-spec states that `HashStats` construction “must be O(R_A + R_B) time and O(U) memory,” where U is the number of unique row hashes. :contentReference[oaicite:0]{index=0}  
  The implementation of `HashStats::from_row_meta` builds `freq_a`, `freq_b`, and `hash_to_positions_b` in a single linear pass over `rows_a` and `rows_b`, but then sorts each `positions` list in `hash_to_positions_b` using `sort_unstable()`. :contentReference[oaicite:1]{index=1}  
  Because `rows_b` is already traversed in monotonically increasing `row_idx` order, the positions are naturally sorted; the explicit sort is therefore unnecessary and bumps worst‑case complexity to O(R_B log R_B).
- **Evidence**:  
  - Mini-spec test/complexity section for HashStats. :contentReference[oaicite:2]{index=2}  
  - `HashStats::from_row_meta` implementation showing the `positions.sort_unstable()` call. :contentReference[oaicite:3]{index=3}  
- **Impact**:  
  - For expected grid sizes (tens of thousands of rows), the extra log factor is unlikely to be noticeable and does not affect correctness.  
  - However, it technically violates the stated O(R_A + R_B) requirement and does extra work that can be avoided for free, and it leaves a small “paper cut” between the documented and actual complexity characteristics.

---

### 2. HashStats threshold example: test plan vs implementation

- **Severity**: Minor  
- **Category**: Spec Deviation / Documentation  
- **Description**:  
  The mini-spec’s test plan still describes `hashstats_counts_and_positions_basic` using `threshold = 2` while asserting that a hash with frequencies 2 in A and 1 in B is “common.” :contentReference[oaicite:4]{index=4}  
  After remediation, the canonical semantics are:
  - `is_common(hash, threshold)` uses strict `>` (not `>=`) and ignores missing hashes.   
  - The actual test was updated to use `threshold = 1` so that the same frequencies are treated as “common” under the strict `>` rule.   
- **Evidence**:  
  - Mini-spec’s description of the test still mentions `threshold = 2`. :contentReference[oaicite:7]{index=7}  
  - `core/tests/grid_view_hashstats_tests.rs` uses `let threshold = 1` in `hashstats_counts_and_positions_basic`. :contentReference[oaicite:8]{index=8}  
  - Remediation plan explicitly calls out adjusting this threshold. :contentReference[oaicite:9]{index=9}  
- **Impact**:  
  - No runtime or correctness impact; the code and tests are internally consistent and match the unified grid diff spec’s definition of rare/common. :contentReference[oaicite:10]{index=10}  
  - The discrepancy may confuse future implementers reading the mini-spec and trying to reason about boundary semantics or add new tests.

---

### 3. HashStats API shape: generic in docs vs RowHash-only in code

- **Severity**: Minor  
- **Category**: Spec Deviation / Documentation  
- **Description**:  
  The mini-spec sketches `HashStats<H>` with a generic `from_row_meta` constructor signature, but GV1 actually implements `from_row_meta` only for `HashStats<RowHash>`, with `RowMeta` hard-wired to `RowHash`.   
  The remediation doc says this is intentional for GV1 and updates the unified spec to clarify that GV1 uses `HashStats<RowHash>::from_row_meta(&[RowMeta], &[RowMeta])` and that broader generic constructors are future work.   
  The code follows that intent:  
  - `HashStats<H>` is generic as a struct,  
  - but `from_row_meta` is implemented only on `impl HashStats<RowHash>`. :contentReference[oaicite:13]{index=13}
- **Evidence**:  
  - Mini-spec type outline still looks fully generic. :contentReference[oaicite:14]{index=14}  
  - HashStats implementation is RowHash-specific for the constructor. :contentReference[oaicite:15]{index=15}  
  - Unified spec explicitly documents GV1 as RowHash-only. :contentReference[oaicite:16]{index=16}  
- **Impact**:  
  - No behavioral impact today; tests and internal usage all target `RowHash`.  
  - Slight documentation/API-shape mismatch that could surprise someone expecting a fully generic constructor from the mini-spec alone.

---

### 4. Degenerate GridView shapes lack explicit tests

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The implementation of `GridView::from_grid` appears correct for all shapes, including:
  - `0 × 0` grids (explicitly tested),  
  - grids with rows but zero columns (`nrows > 0, ncols = 0`),  
  - grids with columns but zero rows (`nrows = 0, ncols > 0`). :contentReference[oaicite:17]{index=17}  
  However, only the fully empty `0 × 0` grid case is covered by tests (`gridview_empty_grid_is_stable`). :contentReference[oaicite:18]{index=18}  
  Shapes with nonzero one dimension and zero the other are not explicitly tested.
- **Evidence**:  
  - `GridView::from_grid` uses `nrows` and `ncols` to size internal vectors and iterates `grid.cells`, which will be empty for these degenerate shapes.   
  - Test suite does not have targeted tests for `(nrows > 0, ncols = 0)` or `(nrows = 0, ncols > 0)` shapes.   
- **Impact**:  
  - Risk is low: code path is straightforward and mirrors the already-tested `0 × 0` case.  
  - Adding explicit tests would future-proof against regressions (e.g., if someone later changes how `Grid` is constructed).

---

### 5. “No undocumented deviations from spec” is very slightly violated (documentation nit)

- **Severity**: Minor  
- **Category**: Gap / Spec Deviation  
- **Description**:  
  Apart from the complexity and API-shape issues already noted, there is a small lingering inconsistency between the mini-spec’s HashStats test example (threshold value) and the implementation/tests after remediation (Finding 2). The unified spec and code are aligned; the only drift is the stale threshold value in the mini-spec test description.   
- **Evidence**:  
  - Mini-spec test plan vs updated tests as in Finding 2.   
- **Impact**:  
  - No runtime impact; this is purely documentation clarity and could mislead new contributors following the test plan verbatim.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - `GridView`, `RowView`, `RowMeta`, `ColMeta`, and `HashStats` implemented in `core/src/grid_view.rs`. :contentReference[oaicite:23]{index=23}  
  - `Grid` hashing functions updated to use shared helpers (`hash_cell_contribution`, `combine_hashes`) without semantic changes.   
  - Public re-exports from `core/src/lib.rs` added for `GridView` and related types. :contentReference[oaicite:25]{index=25}  

- [x] All specified tests created  
  - `core/tests/grid_view_tests.rs` contains `gridview_dense_3x3_layout_and_metadata`, `gridview_sparse_rows_low_info_classification`, `gridview_column_metadata_matches_signatures`, `gridview_empty_grid_is_stable`, and `gridview_large_sparse_grid_constructs_without_panic`, plus the extra `gridview_formula_only_row_is_not_low_info`.   
  - `core/tests/grid_view_hashstats_tests.rs` contains `hashstats_counts_and_positions_basic` and `hashstats_empty_inputs` plus additional boundary tests. :contentReference[oaicite:27]{index=27}  
  - `core/tests/signature_tests.rs` includes `gridview_rowmeta_hash_matches_compute_all_signatures`. :contentReference[oaicite:28]{index=28}  

- [x] Behavioral contract satisfied  
  - Row and column hashes in `RowMeta`/`ColMeta` match existing `RowSignature`/`ColSignature`.   
  - Low‑info classification matches the specification (empty row, whitespace-only, header-like values, numerics, and formula-only rows).   
  - HashStats rare/common/unique semantics match the unified algorithm spec.   

- [ ] No undocumented deviations from spec  
  - Minor deviations remain around the complexity claim for `HashStats::from_row_meta` and the stale threshold value in the mini-spec’s test description (Findings 1 and 2).

- [x] Error handling adequate  
  - `GridView::from_grid` uses `debug_assert!` on bounds and otherwise leverages the existing `Grid` invariants; no new panics or unchecked indexing paths are introduced.   

- [x] No obvious performance regressions  
  - GridView construction is single-pass over `grid.cells` plus row-local sorting, respecting the intended O(M + R log(M/R)) behavior; no O(R×C) scans introduced.   
  - HashStats adds at most a minor O(R log R) factor due to sorting, which is small for realistic R, and is easily optimized away later (Finding 1).  

---

Since all issues are minor and do not affect correctness or external behavior, this branch is safe to proceed to release, with the above notes captured for future cleanup.
```
