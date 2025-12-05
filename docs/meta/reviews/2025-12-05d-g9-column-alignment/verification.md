# Verification Report: 2025-12-05d-g9-column-alignment

## Summary

The G9 column-alignment implementation is present, wired into the engine, and behaves as intended for the planned scenarios. Column metadata is used via `HashStats<ColHash>`; a `ColumnAlignment` struct and `align_single_column_change` function exist with size/repetition gating; and `diff_grids` now routes through a column-aligned fast path before falling back to positional semantics. Workbook-level G9 fixtures (`col_insert_middle`, `col_delete_middle`, `col_insert_with_edit`) plus unit tests for `ColumnAlignment` and `HashStats` are in place and passing, alongside the full existing PG/G test suite. I found no correctness issues likely to cause bad diffs in supported scenarios; there are a couple of minor spec drifts and some test coverage that could be broadened, but nothing that blocks release.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### 1. Column alignment path also used for single tail column append/delete

- **Severity**: Minor  
- **Category**: Spec Deviation  
- **Description**:  
  The mini-spec states that tail-only column adds/deletes (G5/G7) should continue to use the positional path, and that the new column-alignment path is “only for mid-sheet single changes.” :contentReference[oaicite:0]{index=0}  

  In the current implementation, `diff_grids` always tries row alignment first, then column alignment, then positional diff:

  ```rust
  fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
      if let Some(alignment) = align_single_row_change(old, new) {
          emit_aligned_diffs(sheet_id, old, new, &alignment, ops);
      } else if let Some(alignment) = align_single_column_change(old, new) {
          emit_column_aligned_diffs(sheet_id, old, new, &alignment, ops);
      } else {
          positional_diff(sheet_id, old, new, ops);
      }
  }
  ``` :contentReference[oaicite:1]{index=1}  

  `align_single_column_change` applies whenever:
  - `max(nrows) ≤ 2000`, `max(ncols) ≤ 64`
  - `old.nrows == new.nrows`
  - `|new.ncols - old.ncols| == 1`  
  and heavy repetition is absent. :contentReference[oaicite:2]{index=2}  

  That means simple “append one column at the right edge” cases (e.g. PG5.4’s 2×1 → 2×2 grid) now go through `align_single_column_change` + `emit_column_aligned_diffs`, not the old positional `positional_diff` path. The observable DiffOps are still exactly one `ColumnAdded` with no `CellEdited` for existing cells, as enforced by PG5.4 and G7 tests.   

- **Evidence**:  
  - `diff_grids` routing logic. :contentReference[oaicite:4]{index=4}  
  - Column alignment gating conditions in `align_single_column_change`. :contentReference[oaicite:5]{index=5}  
  - PG5.4 / G7 tests asserting tail append/remove semantics (single/multiple columns) still produce only `ColumnAdded`/`ColumnRemoved` with no `CellEdited`.   

- **Impact**:  
  - No correctness impact: `emit_column_aligned_diffs` compares only matched columns and emits exactly one `ColumnAdded` or `ColumnRemoved`, identical to positional semantics in these simple tail cases. :contentReference[oaicite:7]{index=7}  
  - The deviation is architectural/documentation: the spec’s statement “tail-only column adds/deletes still use the positional path” is no longer literally true for single-column tail changes, even though behavior is compatible and well-tested. This could surprise future maintainers relying on the spec’s description of which path runs where.

---

### 2. ColumnAlignment constants duplicated instead of shared

- **Severity**: Minor  
- **Category**: Gap (Maintainability / Architectural)  
- **Description**:  
  The mini-spec calls for reusing the G8 size / repetition limits (rows, columns, hash repetition) for column alignment. :contentReference[oaicite:8]{index=8}  

  In code, both `row_alignment.rs` and `column_alignment.rs` define their own copies of the constants:

  ```rust
  // row_alignment.rs
  const MAX_ALIGN_ROWS: u32 = 2_000;
  const MAX_ALIGN_COLS: u32 = 64;
  const MAX_HASH_REPEAT: u32 = 8;
  ``` :contentReference[oaicite:9]{index=9}  

  ```rust
  // column_alignment.rs
  const MAX_ALIGN_ROWS: u32 = 2_000;
  const MAX_ALIGN_COLS: u32 = 64;
  const MAX_HASH_REPEAT: u32 = 8;
  ``` :contentReference[oaicite:10]{index=10}  

  Values match today, but they are no longer a single shared source of truth.

- **Evidence**:  
  - Spec section 3.1 (performance & size limits) calling for reuse of the same thresholds. :contentReference[oaicite:11]{index=11}  
  - Duplicated constants in both alignment modules.   

- **Impact**:  
  - No current correctness issue, since the constants are identical.  
  - Future risk: changing thresholds for row alignment but forgetting to update the column alignment copy could silently desynchronize the two pathways. This is more of a maintenance hazard than an immediate bug.

---

### 3. Bail‑out behavior only workbook‑tested for “insert + edit to the right” variant

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The mini-spec defines a “bail-out” scenario `col_insert_with_edit_{a,b}.xlsx`, where a mid-sheet column insert is combined with additional cell edits in columns **after** the insertion. The column-alignment path must bail out and fall back to positional semantics.   

  The implementation meets this requirement:

  - `align_single_column_change` builds `HashStats<ColHash>`, enforces heavy repetition limits, and then `find_single_gap_alignment` allows at most **one** gap resolved as a unique insert/delete. A second mismatch causes an immediate `None` (bail-out). :contentReference[oaicite:14]{index=14}  
  - For `col_insert_with_edit`, the inserted column is unique to B, and the edited column to the right has a changed hash, resulting in two mismatches and thus a bail-out.  
  - The integration test `g9_alignment_bails_out_when_additional_edits_present` asserts:
    - No `ColumnAdded` at the interior insertion index.
    - At least one `CellEdited` to the right of the inserted column.   

  However, there are workbook-level tests only for:

  - Insert + edit **below/right** of the insertion (the planned fixture).  
  - No symmetric fixture for:
    - Insert + edit in a column *before* the insertion.
    - Single-column **delete** in the middle plus additional edits (the “delete + edit” analogue).

  At the implementation level, the bail-out logic is symmetric: a second mismatch for any reason (regardless of whether the changed column lies before or after the gap, or whether it’s insert or delete) causes `find_single_gap_alignment` to return `None`, so the behavior is correct by construction. But only a subset of these variants is pinned by integration tests.

- **Evidence**:  
  - `align_single_column_change` + `find_single_gap_alignment` logic and its single-gap guard. :contentReference[oaicite:16]{index=16}  
  - Existing G9 integration test fixtures and tests.   
  - Spec’s description of “insert_with_edit” as a generic pattern (not only “edit below/right”). :contentReference[oaicite:18]{index=18}  

- **Impact**:  
  - No practical correctness problem now: the underlying algorithm clearly bails on **any** extra mismatches, not just “edit to the right” cases.  
  - But the lack of fixtures for “insert+edit to the left” and “delete+edit” leaves some room for future regressions if `find_single_gap_alignment` is refactored without re-considering those symmetric scenarios. Adding fixtures would better lock in the intended “pure structural only” invariant.

---

### 4. Spec vs implementation: location of ColumnAlignment unit tests

- **Severity**: Minor  
- **Category**: Spec Deviation / Missing Test File (but not missing tests)  
- **Description**:  
  The test plan suggests a new file `core/tests/column_alignment_tests.rs` with three unit tests: `single_insert_aligns_all_columns`, `multiple_unique_columns_causes_bailout`, and an optional `heavy_repetition_causes_bailout`. :contentReference[oaicite:19]{index=19}  

  Implementation-wise, these tests exist, but they are placed inside `core/src/column_alignment.rs` under `#[cfg(test)]` rather than in a separate `core/tests` file:

  ```rust
  #[cfg(test)]
  mod tests {
      #[test]
      fn single_insert_aligns_all_columns() { .. }

      #[test]
      fn multiple_unique_columns_causes_bailout() { .. }

      #[test]
      fn heavy_repetition_causes_bailout() { .. }
  }
  ``` :contentReference[oaicite:20]{index=20}  

- **Evidence**:  
  - Test plan’s explicit mention of a new `core/tests/column_alignment_tests.rs` file. :contentReference[oaicite:21]{index=21}  
  - Existing tests defined inline in `column_alignment.rs`. :contentReference[oaicite:22]{index=22}  

- **Impact**:  
  - No missing coverage; the specified scenarios are indeed tested.  
  - The only difference is organizational (module tests vs integration tests file). This is fully acceptable given the mini-spec is not a binding contract, but it’s worth noting as an undocumented deviation from the execution details of the plan.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - `HashStats<ColHash>::from_col_meta` implemented and used by column alignment.   
  - `ColumnAlignment` and `align_single_column_change` implemented with size/repetition/shape gating and unique-column detection. :contentReference[oaicite:24]{index=24}  
  - `diff_grids` integration and `emit_column_aligned_diffs` added.   

- [x] All specified tests created  
  - G9 workbook tests: `g9_col_insert_middle_*`, `g9_col_delete_middle_*`, `g9_alignment_bails_out_when_additional_edits_present`.   
  - ColumnAlignment unit tests: `single_insert_aligns_all_columns`, `multiple_unique_columns_causes_bailout`, `heavy_repetition_causes_bailout`. :contentReference[oaicite:27]{index=27}  
  - `HashStats` column variant tests in `grid_view_hashstats_tests.rs`.   
  - Full regression suite (PG5, G5/G7, G8, etc.) run and passing per `cycle_summary.txt`.   

- [x] Behavioral contract satisfied  
  - UC‑08 (single column insert in middle) and UC‑07 analogue (single column delete in middle) now emit exactly one `ColumnAdded` or `ColumnRemoved` and no spurious `CellEdited`.   
  - `col_insert_with_edit` correctly forces a bail-out from column alignment, falling back to positional semantics with cell edits to the right.   

- [ ] No undocumented deviations from spec (documented deviations with rationale are acceptable)  
  - Column alignment is also used for single tail column append/delete cases, contrary to the mini-spec’s “tail-only → positional” description, although behavior remains correct.   
  - ColumnAlignment tests live inline in the module instead of in the suggested separate test file.   

- [x] Error handling adequate  
  - Alignment functions return `Option<…>` and bail out cleanly when size/shape/repetition or uniqueness conditions fail.  
  - No new panics in production paths; only `debug_assert!` for monotonicity.   

- [x] No obvious performance regressions  
  - Column alignment is gated to the same small-grid limits as row alignment (`≤ 2000` rows, `≤ 64` cols).   
  - For incompatible shapes (e.g., both row and column counts differ, or sizes above thresholds), the engine falls back immediately to positional diff, preserving previous behavior.   

---

Overall, the implementation faithfully delivers the planned G9 functionality, keeps all existing tests green, and introduces only minor, non-blocking spec/organization drift. I recommend proceeding to release and capturing the noted deviations (especially the tail-append path behavior and the duplicated constants) in follow-up documentation or a small clean-up PR when convenient.
