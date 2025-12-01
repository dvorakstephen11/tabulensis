# Remediation Plan: 2025-12-01-pg5-grid-diff-baseline

## Overview

This remediation plan focuses on tightening the PG5 test surface and keeping documentation in sync with the new behavior. The implementation of `diff_grids` itself appears correct and aligned with the mini-spec; the main risks are untested truncation behavior and a stale docs-vs-implementation snapshot that understates current capabilities.

## Fixes Required

### Fix 1: Add explicit tests for row/column truncation

- **Addresses Finding**: 1 — Row/column truncation semantics are untested  
- **Changes**:

  - **File**: `core/tests/pg5_grid_diff_tests.rs`   
  - Add two–three new test cases that mirror the structure of PG5.3/PG5.4 but with *shrinking* shapes:

    1. **Row truncation** (e.g., “pg5_7_grid_diff_row_truncated_row_removed_only”):

       - `GridA`: `nrows = 2`, `ncols = 1`, A1 = 1, A2 = 2.  
       - `GridB`: `nrows = 1`, `ncols = 1`, A1 = 1.  
       - Expectation:
         - `report.ops.len() == 1`.  
         - Single `DiffOp::RowRemoved { sheet: "Sheet1", row_idx: 1, row_signature: None }`.  
         - No `CellEdited` ops.

    2. **Column truncation** (e.g., “pg5_8_grid_diff_column_truncated_column_removed_only”):

       - `GridA`: `nrows = 2`, `ncols = 2`, with non-empty values in both columns.  
       - `GridB`: `nrows = 2`, `ncols = 1`, column A identical to `GridA`’s column A.  
       - Expectation:
         - `report.ops.len() == 1`.  
         - Single `DiffOp::ColumnRemoved { sheet: "Sheet1", col_idx: 1, col_signature: None }`.  
         - No `CellEdited` ops.

    3. (Optional, but useful) **Simultaneous row and column truncation**:

       - `GridA`: small 2×2 (or 3×3) grid.  
       - `GridB`: smaller 1×1 or 2×1 grid where the overlapping 1×1 block is unchanged.  
       - Expectation:
         - One or more `RowRemoved` and/or `ColumnRemoved` ops for the truncated tails.  
         - No `CellEdited` ops.

  - Use the existing `grid_from_numbers` and `single_sheet_workbook` helpers to keep helpers symmetric and concise. :contentReference[oaicite:28]{index=28}  

- **Tests**:

  - New tests should follow the PG-style naming convention and assertion style already used (single-op assertions with explicit pattern matching on the `DiffOp` variant, sheet id, index, and signature).   
  - Ensure these new tests are clearly labeled as part of PG5 in comments or docstrings, so they remain tied to the PG5 milestone.

---

### Fix 2: Add at least one mixed “shape + content” scenario

- **Addresses Finding**: 2 — No tests for mixed “shape + content” changes  
- **Changes**:

  - **File**: `core/tests/pg5_grid_diff_tests.rs` :contentReference[oaicite:30]{index=30}  

  - Add a test that combines tail append/truncate with overlapping edits. For example:

    - **Row append + cell edits in overlap**:

      - `GridA`: 2×2 grid with values 1..4.  
      - `GridB`: 3×2 grid:
        - First two rows same shape; change, say, cell `B1` from 2 → 20 and `A2` from 3 → 30.  
        - Append a third row with any values.  
      - Expectations:
        - Exactly **two** `CellEdited` ops for the changed overlapping cells (`B1`, `A2`).  
        - Exactly **one** `RowAdded` with `row_idx` equal to the appended row index, `row_signature.is_none()`.  
        - No `CellEdited` for the appended row.

    - A similar test could be added for column append + edits if desired, but one mixed scenario is likely sufficient for this cycle.

- **Tests**:

  - Assert:
    - The counts of `CellEdited` vs `Row*`/`Column*` ops match expectations.  
    - Edited addresses match a small set of expected A1-style strings, similar to PG5.5’s set comparison. :contentReference[oaicite:31]{index=31}  

---

### Fix 3: Update docs-vs-implementation analysis for UC-04/UC-05

- **Addresses Finding**: 3 — Documentation snapshot is now stale  
- **Changes**:

  - **File**: `docs/rust_docs/2025-11-30-docs-vs-implementation.md` (in this review package as `2025-11-30-docs-vs-implementation.md`).   

  - Update the status table and narrative for:
    - UC-04 (Row append)  
    - UC-05 (Column append)

  - Suggested adjustments (conceptual, not exact wording):

    - For UC-04 / UC-05:
      - Change “❌ No – reports as CellEdited, not RowAdded/ColumnAdded” to something like:
        - “✅ Partially – for simple spreadsheet-mode tails (append at end with no reordering), `diff_grids` now emits `RowAdded`/`ColumnAdded` ops per PG5. More advanced alignment cases remain unimplemented.”

    - In the “Implementation Status by Specification Part” section, clarify that:
      - The naive cell-by-cell engine has been extended to emit structural row/column ops for tail-only spreadsheet-mode scenarios, but the full alignment pipeline and more complex use cases are still unimplemented.   

- **Tests**:

  - No code tests required, but once updated, this doc should be referenced from future planning documents as the post-PG5 alignment snapshot.

---

### Fix 4 (Process-only): Persist this remediation plan in the repo

- **Addresses Finding**: 4 — Prior remediation history is effectively empty  
- **Changes**:

  - Ensure this remediation plan is saved under the appropriate `docs/meta` or equivalent location (e.g., `docs/meta/remediations/2025-12-01-pg5-grid-diff-baseline.md`) and that `combined_remediations.md` is regenerated to include it. :contentReference[oaicite:34]{index=34}  

- **Tests**:

  - None (process / documentation only).

---

## Constraints

- Do **not** change the public API of `diff_workbooks` or the `DiffOp` enum; existing PG4 JSON shape and round-trip tests must continue to pass unchanged.   
- Do **not** introduce new DiffOp variants or alignment behavior in this cycle; PG5 is explicitly scoped to “naive spreadsheet mode, tails only, no reordering”.   
- Keep test helpers (`grid_from_numbers`, `single_sheet_workbook`) in-memory only and free of Excel parsing dependencies, consistent with the PG5 design.   
- Maintain current performance characteristics (simple O(R×C) overlapping loop + O(R+C) tails); any optimization work should be deferred to later H1-aligned grid-diff milestones.   

---

## Expected Outcome

After remediation:

- PG5 will include explicit, executable tests for both **append** and **truncate** tail scenarios, plus at least one mixed “shape + content” case, fully pinning down the spreadsheet-mode tail semantics outlined in the mini-spec.   
- `2025-11-30-docs-vs-implementation.md` will accurately reflect that simple row/column appends at the tail are now backed by `RowAdded`/`ColumnAdded` ops rather than cell edits, while still clearly calling out the remaining algorithmic gaps.   
- Future changes to `diff_grids` that break truncate semantics or leak cell edits into tail-only regions will be caught early by CI, before more advanced alignment or database-mode work builds on top of PG5.  
- The remediation history for this branch will be preserved, giving future reviewers clear traceability from findings to fixes.
