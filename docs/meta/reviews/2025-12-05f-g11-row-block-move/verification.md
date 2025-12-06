```markdown
# Verification Report: 2025-12-06-g11-row-block-move

## Summary

The implementation of G11 “Block of Rows Moved” matches the mini-spec and the underlying technical specification: an exact, single block row move on spreadsheet-mode sheets is detected via row hashes and emitted as a single `DiffOp::BlockMovedRows` with no extra noise. The new `RowBlockMove` helper, detection logic, engine integration, fixtures, and tests are all present and wired correctly. No blocking bugs or undocumented spec deviations were found. A few small test-coverage improvements are suggested but are not release-blocking.

## Recommendation

[x] Proceed to release  
[ ] Remediation required  

## Findings

### 1. G11 feature is implemented as planned

- **Severity**: Minor  
- **Category**: Gap (positive confirmation)  
- **Description**:  
  The mini-spec calls for:
  - A new `RowBlockMove` helper and exact move detector in `row_alignment.rs`.  
  - Integration in `diff_grids` so that exact moves emit `BlockMovedRows` before row/column alignment and positional diff.  
  - A new grid fixture (`g11_row_block_move`) and generator, plus workbook-level and unit tests. :contentReference[oaicite:0]{index=0}  

  The code implements:
  - `RowBlockMove { src_start_row, dst_start_row, row_count }` and `detect_exact_row_block_move(old, new)` in `core/src/row_alignment.rs`, with size-bounds, low-info, and heavy-repetition guards reused from the existing alignment path.   
  - `diff_grids` now calls `detect_exact_row_block_move` first, and on success emits a `DiffOp::BlockMovedRows` via `emit_row_block_move`. On failure it falls back to the existing row-alignment, column-alignment, and positional diff pipeline. :contentReference[oaicite:2]{index=2}  
  - The `BlockMovedRows` DiffOp shape (fields and JSON) remains exactly as in the spec and prior PG4 tests.   
  - Fixture manifest entry `g11_row_block_move` and `RowBlockMoveG11Generator` in `fixtures/src/generators/grid.py`.   
  - New integration tests `g11_row_block_move_grid_workbook_tests.rs` and row-alignment unit tests for the detection helper.   

- **Evidence**:  
  - Mini-spec sections 1 (Scope) and 2 (Behavioral Contract). :contentReference[oaicite:6]{index=6}  
  - `RowBlockMove` and `detect_exact_row_block_move` implementation.   
  - `diff_grids` and `emit_row_block_move`. :contentReference[oaicite:8]{index=8}  
  - Fixtures and tests.   

- **Impact**:  
  This confirms that the core of G11 is implemented and integrated as planned; no action needed, but it anchors the rest of the review.

---

### 2. Exact move detection logic matches the behavioral contract

- **Severity**: Minor  
- **Category**: Gap (positive confirmation)  
- **Description**:  
  The contract for G11 is: same grid dimensions, exactly one non-overlapping moved block, identical block content, no internal edits, and no other differences. The detector enforces this:

  - Pre-conditions:
    - Requires same `nrows` and `ncols`; otherwise returns `None`. :contentReference[oaicite:10]{index=10}  
    - Rejects empty grids and grids outside the row/column bounds used by row alignment (`MAX_ALIGN_ROWS`, `MAX_ALIGN_COLS`).   
    - Rejects low-info-dominated and heavy-repetition grids (shared guards with G8/G10).   

  - Exact-match requirement:
    - Builds `GridView` and row `HashStats`.   
    - If all row hashes match pairwise, it returns `None` (no-op). :contentReference[oaicite:14]{index=14}  

  - Single-block constraint:
    - Finds the first mismatching prefix index and a matching suffix from the end, defining a single “mismatch window”. :contentReference[oaicite:15]{index=15}  
    - For a candidate `(src_start, dst_start)`, it:
      - Grows a run where row hashes match pairwise.
      - Requires non-overlap (`src_end <= dst_start || dst_end <= src_start`).
      - Verifies that, after *removing* these ranges, the remaining row-hash sequences are identical. :contentReference[oaicite:16]{index=16}  

  - Uniqueness of block rows:
    - For each row in the candidate source block, it checks that the row hash appears exactly once in both grids (`freq_a == 1 && freq_b == 1`).   
    - This matches the mini-spec’s “no ambiguous repeated rows inside the moved block” constraint while still allowing repeated rows elsewhere. :contentReference[oaicite:18]{index=18}  

  Positive and negative unit tests in `row_alignment.rs` confirm the algorithm for a representative downward move and for a moved block with internal edits. :contentReference[oaicite:19]{index=19}  

- **Evidence**:  
  - G11 behavioral contract & failure modes.   
  - `detect_exact_row_block_move` implementation and unit tests.   

- **Impact**:  
  The detector is faithful to the intended behavior and should only fire in the strict G11 case. No remediation needed.

---

### 3. Workbook-level behavior for the happy path is fully covered

- **Severity**: Minor  
- **Category**: Gap (positive confirmation)  
- **Description**:  
  The core end-to-end behavior for G11 is tested:

  - Fixture `row_block_move_a.xlsx` vs `row_block_move_b.xlsx` is generated by `RowBlockMoveG11Generator`, which:
    - Builds “ordinary” rows with `R{r}_C{c}` values.
    - Builds a distinctive “BLOCK” segment with `BLOCK_r{r}_c{c}` values.
    - Moves the block from `src_start_row` to `dst_start_row` without altering content. :contentReference[oaicite:22]{index=22}  

  - `g11_row_block_move_emits_single_blockmovedrows` asserts:
    - Exactly one diff op.
    - It is `DiffOp::BlockMovedRows` with `src_start_row = 4`, `row_count = 4`, `dst_start_row = 12`, and `block_hash == None`, matching the concrete example in the mini-spec.   
    - No `RowAdded`, `RowRemoved`, or `CellEdited` ops. :contentReference[oaicite:24]{index=24}  

  This directly exercises `detect_exact_row_block_move` and `emit_row_block_move` through the public API (`diff_workbooks`), confirming that the wire-up from workbook to ops is correct.   

- **Evidence**:  
  - Generator and manifest entry.   
  - G11 happy-path workbook test. :contentReference[oaicite:27]{index=27}  

- **Impact**:  
  Happy-path behavior is well covered; no remediation required.

---

### 4. Negative behavior for ambiguous repeated rows and internal edits is covered, but not exhaustively

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The mini-spec calls out several failure modes where the move detector must return `None` and let the existing pipeline handle the diff: internal edits, ambiguous repeats, large/low-info sheets, and extra edits outside the block. :contentReference[oaicite:28]{index=28}  

  Current tests cover:

  - **Internal edits inside the moved block**:  
    `block_move_detection_rejects_internal_edits` constructs a grid where a candidate block is moved but one cell inside the block changes. It asserts detection returns `None`. :contentReference[oaicite:29]{index=29}  

  - **Ambiguous repeated rows in the block**:  
    `g11_repeated_rows_do_not_emit_blockmove` creates a 4-row sheet with two pairs of identical rows that swap positions; it asserts that no `BlockMovedRows` op is emitted, and that we fall back to positional `CellEdited` noise. :contentReference[oaicite:30]{index=30}  

  Not currently covered (but inferred from code):

  - A “block move + extra edit *outside* the block” scenario where, for example, the bottom-most row changes in addition to the move. The implementation’s “remove the candidate block and require equality” logic guarantees this will return `None`, but there is no explicit test. :contentReference[oaicite:31]{index=31}  
  - Low-info-dominated or heavy-repetition grids that technically contain a moved block but are intended to bypass the alignment-style detectors entirely. Guards are present and reused, but not specifically tested under G11.   

- **Evidence**:  
  - Mini-spec failure-mode bullets. :contentReference[oaicite:33]{index=33}  
  - Negative tests for internal edits and repeated rows.   
  - Guard logic in `detect_exact_row_block_move`.   

- **Impact**:  
  The behavior is correct by inspection, and core risk cases (internal edit, ambiguous duplicates) are covered. Additional negative tests would harden the suite and protect against future refactors but are not required for this release.

---

### 5. No documented or undocumented spec deviations found

- **Severity**: Minor  
- **Category**: Spec Deviation (none)  
- **Description**:  
  - `DiffOp::BlockMovedRows` shape and JSON remain exactly as in the main Excel diff spec and PG4 tests (same fields, same optional `block_hash`).   
  - The mini-spec’s requirement that `block_hash` be left `None` for G11 is respected in `emit_row_block_move` and the G11 workbook test.   
  - The mini-spec explicitly limits G11 to spreadsheet-mode grid diffs and *does not* change DataMashup or database-mode paths; `diff_grids` remains the grid-only path, and DataMashup tests continue to assert that no `BlockMovedRows` ops appear there.   
  - No “Intentional Spec Deviations” are recorded in the cycle summary, and no behavioral differences vs. the mini-spec were observed in code.   

- **Evidence**:  
  - Excel Diff specification for `DiffOp`.   
  - Mini-spec constraints and scope exclusions. :contentReference[oaicite:41]{index=41}  
  - Engine integration and tests.   

- **Impact**:  
  No spec deviations need remediation.

---

### 6. Small test-coverage gap for “move upward” direction

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The algorithm is symmetrical in that it can detect a block moved earlier in the sheet (source > dest) or later (source < dest), via two candidate-search passes.   

  Current tests exercise:

  - A representative **downward** move (src = 4, dst = 12) in both unit tests and workbook tests.   

  There is no explicit test where a block moves “upwards” (e.g., from rows 12–15 to 4–7):

  - The algorithm should handle this via the first candidate path (searching for `meta_b[prefix]` in `meta_a[prefix..]`), but this is not currently asserted via tests.   

- **Evidence**:  
  - Detection logic’s two candidate-search branches.   
  - Existing tests all move the block down.   

- **Impact**:  
  Risk is low because the symmetry is straightforward and the logic has been manually validated, but adding an “upward move” test would further reduce regression risk.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed   
- [x] All specified tests created (G11 fixtures, workbook tests, and row-alignment unit tests)   
- [x] Behavioral contract satisfied (single `BlockMovedRows` for pure move; no RowAdded / RowRemoved / CellEdited noise)   
- [x] No undocumented deviations from spec (documented deviations not present)   
- [x] Error handling adequate (guards for size, low-info, heavy repetition, and empty grids)   
- [x] No obvious performance regressions (detection is O(R) in row count, uses same GridView/HashStats building blocks as G8/G10, and bails out early for out-of-bounds or degenerate cases)   
```

Since all findings are minor and primarily about strengthening tests rather than fixing defects, the overall recommendation is to **proceed to release**. No dedicated remediation cycle is required, though adding the suggested extra tests in a future hardening pass would be beneficial.
