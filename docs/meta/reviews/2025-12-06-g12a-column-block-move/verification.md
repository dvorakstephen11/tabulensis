# Verification Report: 2025-12-06-g12a-column-block-move

## Summary

The G12a implementation cleanly adds exact column block move detection for small spreadsheet-mode grids and wires it into the diff engine without disturbing existing behavior. The detector mirrors the existing row-block move logic, uses the same GridView/HashStats infrastructure and guards, and emits `BlockMovedColumns` exactly as specified for the new G12 fixture and ambiguity guard scenario. All tests described in the mini-spec (fixtures, workbook-level tests, and detector unit tests) are present and passing, and existing G8–G11 tests still pass unchanged. The remaining issues are minor and mostly about expanding test coverage and clarifying corner-case behavior (multi-column moves, column swaps).

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### 1. Column block move detector matches the planned contract

- **Severity**: Minor (verification note; no action required)
- **Category**: Gap / Spec Alignment
- **Description**:  
  The `ColumnBlockMove` detector is implemented in `core/src/column_alignment.rs` and closely mirrors the existing row-block move detector:

  - Requires identical shapes (`nrows` and `ncols` equal) and non-empty column count.  
  - Applies the same size envelope as G8–G11 (`MAX_ALIGN_ROWS = 2000`, `MAX_ALIGN_COLS = 64`).   
  - Uses `GridView::from_grid` to derive `col_meta` and `HashStats<ColHash>` for both grids.   
  - Bails out when more than half of the columns are entirely blank (`blank_dominated`) or when any column hash repeats more than `MAX_HASH_REPEAT` in either grid.   
  - Finds the first mismatching column (`prefix`), computes a matching suffix (`tail_start`), then searches for a *single* contiguous candidate block that:  
    - Matches exactly between A and B in that region,  
    - Does not overlap between source and destination, and  
    - Leaves the rest of the columns matching 1:1 after removing that block from both sides. :contentReference[oaicite:3]{index=3}  
  - Validates that each column hash in the candidate block is unique in both grids (frequency 1 in `freq_a` and `freq_b`), enforcing the “unambiguous by hash” constraint.   

  This matches the mini-spec’s behavior: “exact column block move detection” that returns `Some(ColumnBlockMove)` only when there is exactly one contiguous column block moved, no internal edits, and no ambiguous matches. 

- **Evidence**:  
  - `ColumnBlockMove` struct and `detect_exact_column_block_move` implementation. :contentReference[oaicite:6]{index=6}  
  - `GridView` / `HashStats` design and usage.   
  - Mini-spec Section 4.2 (“Internal helper APIs”). :contentReference[oaicite:8]{index=8}  

- **Impact**:  
  The implementation satisfies the intended behavioral contract; this finding is recorded to confirm alignment, not to call for remediation.

---

### 2. `diff_grids` integration and DiffOp shape are correct

- **Severity**: Minor (verification note)
- **Category**: Gap / Spec Alignment
- **Description**:  
  `diff_grids` in `core/src/engine.rs` has been updated to insert column block move detection as an early fast path, immediately after row-block moves and before row/column alignment:

  1. Try `detect_exact_row_block_move` → if `Some`, emit `BlockMovedRows` and return.  
  2. Else try `detect_exact_column_block_move` → if `Some`, emit `BlockMovedColumns` and return.  
  3. Else fall back to row alignment, column alignment, then positional diff. :contentReference[oaicite:9]{index=9}  

  The new helper:

  ```rust
  fn emit_column_block_move(sheet_id: &SheetId, mv: ColumnBlockMove, ops: &mut Vec<DiffOp>) {
      ops.push(DiffOp::BlockMovedColumns {
          sheet: sheet_id.clone(),
          src_start_col: mv.src_start_col,
          col_count: mv.col_count,
          dst_start_col: mv.dst_start_col,
          block_hash: None,
      });
  }
````

uses the existing `DiffOp::BlockMovedColumns` variant without changing its shape.

For pure column moves, `row`-level hashes *do* change (because row hashing incorporates the column index), so `detect_exact_row_block_move` will properly return `None`, and column detection gets the chance to fire as intended.

* **Evidence**:

  * `diff_grids` early-return ordering and `emit_column_block_move` helper. 
  * `DiffOp` enum definition with `BlockMovedColumns`. 
  * `GridView` hashing semantics (row hashes are position-sensitive).

* **Impact**:
  Pure column moves are now recognized as a single `BlockMovedColumns` op, as required, and existing phases still run for all other cases. No regressions are apparent.

---

### 3. All planned tests and fixtures for G12a are present and passing

* **Severity**: Minor (verification note)

* **Category**: Gap / Missing Test (resolved)

* **Description**:
  The mini-spec’s testing plan for G12a is fully implemented:

  * **Fixtures & generator**

    * `fixtures/manifest.yaml` includes a Phase 4 G12 entry `g12_column_block_move` with the expected args and outputs (`column_move_a.xlsx`, `column_move_b.xlsx`).
    * `ColumnMoveG12Generator` in `fixtures/src/generators/grid.py` creates a `Data` sheet with 8 columns where one “key” column has distinctive header `"C_key"` and data (`100 * r`), and other columns use different numeric patterns (`r * 10 + c`) so that the key column’s hash is unique. It writes the unmodified grid as `column_move_a.xlsx` and a version with the key column moved from `src_col` to `dst_col` as `column_move_b.xlsx`.

  * **Workbook-level tests**

    * `core/tests/g12_column_block_move_grid_workbook_tests.rs` contains:

      * `g12_column_move_emits_single_blockmovedcolumns`: asserts that diffing the G12 fixture yields exactly one `BlockMovedColumns` op with `sheet == "Data"`, `src_start_col == 2` (0-based C), `col_count == 1`, `dst_start_col == 5` (0-based F), `block_hash.is_none()`, and no `ColumnAdded/ColumnRemoved/RowAdded/RowRemoved/CellEdited` ops.
      * `g12_repeated_columns_do_not_emit_blockmovedcolumns`: constructs an in-memory scenario with repeated, swapped columns and asserts `DiffOp::BlockMovedColumns` is *not* produced and that some other diff op exists, confirming the ambiguity guard.

  * **Unit tests in the detector**

    * `column_alignment::tests` includes:

      * `detect_exact_column_block_move_simple_case` — small `Grid` with a single unique column moved, expecting `Some(ColumnBlockMove { src_start_col, col_count: 1, dst_start_col })`.
      * `detect_exact_column_block_move_rejects_internal_edits` — same setup but with a single changed cell inside the moved column; asserts `None`.
      * `detect_exact_column_block_move_rejects_repetition` — repeated identical columns swapped; asserts `None`.

  * **Regression safety**

    * `cycle_summary.txt` shows `cargo test` runs 53 tests successfully, including all existing G8–G11 workbook tests and the new G12 tests. 

* **Impact**:
  The behavioral contract for G12a is well covered by tests at both the detector and workbook levels, and basic regression safety is satisfied.

---

### 4. Multi-column block moves are supported in code but untested

* **Severity**: Minor

* **Category**: Missing Test

* **Description**:
  The `ColumnBlockMove` type and detector are general over `col_count` and are perfectly capable of representing moves of a contiguous block of *multiple* columns:

  * `ColumnBlockMove` tracks `col_count: u32`. 
  * The detector’s `len` variable can span multiple columns, and `try_candidate` checks that the entire candidate range matches and the remainder of the columns align 1:1. 

  However, all current tests (both workbook-level and unit tests) focus on **single-column** moves (`col_count == 1`):

  * G12 fixture moves only column C to position F.
  * The unit tests explicitly assert `col_count == 1` in the simple case.

  There is no test that diffing a workbook with a genuinely multi-column contiguous block move yields a `BlockMovedColumns` op with `col_count > 1`.

* **Evidence**:

  * `ColumnBlockMove` and `detect_exact_column_block_move` implementation. 
  * G12 fixture specification and generator.
  * Detector unit tests’ expectations.

* **Impact**:
  For this G12a milestone, the explicit spec example is a single-column move, so the core contract is satisfied. However, the name “column block move” and the generalized implementation suggest that multi-column moves are *intended* to work; without tests, regressions in this behavior would be easy to miss. This is a good candidate for a future small follow-up test-only change.

---

### 5. No explicit negative tests for “two independent column moves” at the workbook level

* **Severity**: Minor

* **Category**: Missing Test

* **Description**:
  The mini-spec requires `detect_exact_column_block_move` to return `None` when there are multiple moved blocks or overlapping candidates.  The implementation enforces this via:

  * Rest-of-array validation that, after removing a *single* candidate block from A and B, all remaining columns match 1:1 in order. If more than one block moved, this check fails and the function returns `None`. 

  The unit tests do cover:

  * Internal cell edits (must return `None`).
  * Repetition-based ambiguity (must return `None`).

  But there is no explicit workbook-level test for a sheet where **two disjoint column blocks move**. In such a case, the detector should bail out and let the alignment/positional diff handle the scenario.

* **Evidence**:

  * Mini-spec multi-block rejection requirement. 
  * Candidate validation logic in `detect_exact_column_block_move`. 
  * Existing G12 tests (only single-block scenarios covered).

* **Impact**:
  This is not a correctness bug in the current implementation; the code logically rejects multi-block moves. But a regression test would help ensure future refactors don’t accidentally loosen this invariant.

---

### 6. Minor internal spec deviation: `ColumnBlockMove` doesn’t carry `sheet` name

* **Severity**: Minor

* **Category**: Spec Deviation (internal, documented here)

* **Description**:
  The mini-spec’s sketch of the helper API suggested a `detect_exact_column_block_move(sheet_name: &str, old: &Grid, new: &Grid) -> Option<ColumnBlockMove>` where the helper might store the `sheet` name in the move struct. 

  The actual implementation is:

  ```rust
  pub(crate) struct ColumnBlockMove {
      pub src_start_col: u32,
      pub dst_start_col: u32,
      pub col_count: u32,
  }

  pub(crate) fn detect_exact_column_block_move(old: &Grid, new: &Grid) -> Option<ColumnBlockMove> { ... }
  ```

  with the sheet name passed separately into `emit_column_block_move` from `diff_grids`.

  This is a purely internal design deviation; the external `DiffOp::BlockMovedColumns` still includes the `sheet` field and is emitted correctly.

* **Evidence**:

  * Mini-spec helper API sketch. 
  * `ColumnBlockMove` definition and `emit_column_block_move` usage.

* **Impact**:
  No behavioral impact. The separation of concerns is arguably cleaner (detector works on `Grid`s only). This deviation does not require remediation but is worth noting as “code diverges slightly from earlier planning doc.”

---

### 7. Behavior for column swaps is under-specified but likely acceptable

* **Severity**: Minor

* **Category**: Gap / Spec Clarification

* **Description**:
  Because the detector looks for *any* single contiguous block whose movement explains the difference between A and B, some patterns that are intuitively “two columns swapped” can also be expressed as “a single column moved past another”. For example, swapping columns 0 and 1 while keeping others fixed can be seen as “move column 1 in A to position 0 in B”.

  The current algorithm:

  * Finds the first mismatch index (`prefix`).
  * Searches for a matching column hash from the other grid in the `[prefix..tail_start)` region.
  * Once a candidate block is found that makes the remainder align 1:1, it is accepted as a valid single-block move (subject to uniqueness and repetition checks). 

  There is no test asserting whether swaps should be treated as a move vs. a more general structural diff. The unified grid spec is neutral here; it just defines `BlockMovedColumns` structurally, without singling out swaps.

* **Evidence**:

  * Column block detection algorithm structure. 
  * Unified grid spec for `BlockMovedColumns`. 

* **Impact**:
  In practice, representing a swap as a “move of one column past another” is reasonable and probably desirable. The only risk is surprise if future consumers expected “swap” to be modeled purely as two moves or as add/remove. This is best addressed by adding explicit tests clarifying the intended behavior, but it is *not* a blocker.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed

  * Engine fast-path integration, detector implementation, generator, fixtures, and tests all match the planned scope.

* [x] All specified tests created

  * Workbook-level G12 tests and detector unit tests are present and match the mini-spec’s descriptions.

* [x] Behavioral contract satisfied

  * Simple column move → single `BlockMovedColumns` with correct indices and no noise.
  * Ambiguous repeated columns → no `BlockMovedColumns`; fallback ops present.

* [x] No undocumented deviations from spec (documented deviations with rationale are acceptable)

  * Only notable deviation is the internal helper signature/struct field choice for `ColumnBlockMove`, which does not affect observable behavior and is documented above.

* [x] Error handling adequate

  * All new logic returns `Option` with early bailouts on non-supported shapes, size limits, blank dominance, and heavy repetition; no panics or unwraps in production paths.

* [x] No obvious performance regressions

  * Detector reuses `GridView` and `HashStats` with the same small-grid bounds as G8–G11; heavy repetition and blank-dominated guards remain in place to avoid pathological cases.

---

Overall, the implementation is solid, aligned with the plan, and safe to ship. The remaining items are incremental test and spec-clarification opportunities rather than blockers.