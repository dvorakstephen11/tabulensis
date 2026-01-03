
Milestone: **Phase 4 – G13 Fuzzy Move Detection (UC‑12: block moved with internal edits)** 

---

## 1. Scope

### In scope

Code and test changes are limited to spreadsheet-mode grid diff row move detection:

- **Rust modules**
  - `core/src/row_alignment.rs`
    - Add a fuzzy row block move detector building on `GridView`, `HashStats<RowHash>`, and existing guards (`MAX_ALIGN_ROWS`, `MAX_ALIGN_COLS`, `MAX_HASH_REPEAT`, low-info checks). 
  - `core/src/engine.rs`
    - Extend `diff_grids` to try fuzzy row-block moves after existing exact move fast paths and before falling back to alignment/positional diff.
    - Reuse the existing `emit_row_block_move` helper to emit `DiffOp::BlockMovedRows` for fuzzy moves as well. 
- **Tests**
  - New unit tests in `row_alignment.rs` for fuzzy move detection.
  - New workbook-level integration tests:
    - `core/tests/g13_fuzzy_row_move_grid_workbook_tests.rs` (name illustrative; implementer may adjust, but tests must clearly map to G13 in the test plan).
- **Fixtures and generators**
  - Extend the Python fixture repo to add a Phase 4 G13 scenario:
    - Generator (e.g.) `RowFuzzyMoveG13Generator` in `fixtures/src/generators/grid.py`.
    - Manifest entry for `g13_fuzzy_row_move` in `fixtures/manifest.yaml`.
    - Generated files:
      - `grid_move_and_edit_a.xlsx`
      - `grid_move_and_edit_b.xlsx` 

### Out of scope

- No changes to:
  - Database mode alignment (`core/src/database_alignment.rs`) or D2–D10 tests. 
  - Column or rectangular fuzzy moves (we continue to support **exact** column/rectangular block moves only).
  - The full AMR pipeline (anchor chain, gap alignment, global move candidate set). We implement a **local**, guard-railed fuzzy detector consistent with the AMR design but not the full pipeline. 
- No public API surface changes:
  - `diff_workbooks`, `diff_grids`, and JSON shapes remain unchanged beyond the presence of additional `BlockMovedRows` ops where previously we had only add/remove/cell-edit noise.

---

## 2. Behavioral Contract

### 2.1 High-level behavior

When a contiguous block of rows is moved and only **some** cells inside the block change, the diff should:

- Emit a **single** `BlockMovedRows` operation describing the move (source range and destination start).  
- Emit `CellEdited` operations **inside that moved block** for cells whose values actually changed.  
- **Not** emit `RowAdded` / `RowRemoved` for any rows that belong to the moved block.  
- Leave all unrelated rows and cells reported exactly as they are today.

This corresponds to UC‑12 in the algorithm spec and G13 in the testing plan. 

### 2.2 Examples

#### Example A – Simple fuzzy move with one edited cell (primary positive case)

- Grid A:
  - Rows 0–3: header + some context rows.
  - Rows 4–7: a 4-row “data block” with distinctive values (e.g., IDs 1001–1004 and several numeric columns).
  - Rows 8–19: more unrelated rows.

- Grid B:
  - Same overall rows, but the 4-row data block now appears at rows 12–15.
  - Exactly one numeric cell inside that 4-row block has changed (e.g., a value changed from 42 to 43).

**Expected diff (spreadsheet mode)**:

- One `DiffOp::BlockMovedRows` with:
  - `src_start_row` = original top row of the block in A (e.g., 4).
  - `row_count` = block height (e.g., 4).
  - `dst_start_row` = top row of the block in B (e.g., 12).
- One or more `DiffOp::CellEdited` operations for modified cells within those rows (exact addresses/values determined by actual fixture).
- No `RowAdded`/`RowRemoved` for the rows 4–7 / 12–15.
- All non-block rows are aligned and compared as today (no regressions to G1–G12 behavior).

#### Example B – Block moved but heavily rewritten (should **not** be fuzzy move)

Same as Example A, but:

- More than ~25–30% of cells in the block are changed (e.g., several entire rows differ substantially).

**Expected behavior**:

- Fuzzy detection **does not trigger** because similarity falls below the threshold.
- The diff falls back to existing behavior: a combination of `RowRemoved` / `RowAdded` and `CellEdited` as appropriate.
- No `BlockMovedRows` op is emitted for this case.

#### Example C – Repeated patterns / ambiguity (bail-out)

- Several identical or near-identical blocks exist in both grids, making it unclear which block moved where (e.g., the same 4-row sequence appears three times in each grid at different positions).

**Expected behavior**:

- Guards based on `HashStats` and similarity should cause the fuzzy move detector to **bail out** in ambiguous scenarios.
- No `BlockMovedRows` op is emitted.
- The diff falls back to alignments/positional diff, preserving determinism (no arbitrary choice between multiple candidates). 

#### Example D – Pure in-place edits (no move)

- No rows change position; a few cells within a block are edited.

**Expected behavior**:

- Fuzzy move detection must not misclassify this as a move.
- Behavior remains exactly as today: simple `CellEdited` operations, no `BlockMovedRows` ops.
- This is validated with unit tests and at least one workbook test slicing the G8/G1 fixtures with internal edits.

---

## 3. Constraints

### 3.1 Performance and complexity

- **Row/column bounds**: Fuzzy detection applies only when:
  - `max(old.nrows, new.nrows) <= MAX_ALIGN_ROWS` (currently 2,000).
  - `max(old.ncols, new.ncols) <= MAX_ALIGN_COLS` (currently 64). :contentReference[oaicite:21]{index=21}
- **Repetition guard**:
  - Reuse existing `HashStats<RowHash>` logic and `MAX_HASH_REPEAT` to bail out when row hashes are heavily repeated on either side; this avoids quadratic scanning over highly repetitive data. :contentReference[oaicite:22]{index=22}
- **Local scanning only**:
  - No unbounded O(R²) candidate search across all row pairs.
  - Candidate blocks must be **contiguous** ranges and limited in height (e.g., up to 32 rows) to keep similarity computation bounded.
- **Determinism**:
  - If multiple candidate destinations have similarity above the threshold, the detector must **not** pick one arbitrarily.
  - In ambiguous multi-candidate cases, the detector returns `None` and the engine falls back to existing logic.

### 3.2 Similarity semantics

- Fuzzy detection operates on **row-level tokens/hashes**, consistent with the AMR design’s Jaccard-based `compute_block_similarity`: 
  - Each row contributes a token derived from `GridView.row_meta` (already present).
  - Similarity between source and destination blocks is computed using a set-based Jaccard-like measure over these row tokens.
- **Threshold**:
  - Default threshold: **0.80** similarity.
  - A candidate block move is accepted as fuzzy only if:
    - Block dimensions match (same `row_count`).
    - Jaccard similarity of row tokens ≥ 0.80.
- **Cell diff within block**:
  - After classifying a fuzzy move, we rely on existing `diff_row_pair` logic to generate `CellEdited` operations inside the moved block (rows aligned 1:1 by relative index within the block). 

### 3.3 Invariants

- No changes to:
  - `RowAlignment` invariants (matched pairs strictly increasing, `inserted` and `deleted` index sets sorted). :contentReference[oaicite:25]{index=25}
  - The semantics of existing G8–G12 tests (row/column insert/delete, exact moves). All those tests must remain green.
- Rectangular block detection (`BlockMovedRect`) retains its current behavior: any internal edit in the rectangle prevents `BlockMovedRect` emission and falls back to other paths. Fuzzy detection will be row-only, not rectangular, for this cycle. 

---

## 4. Interfaces

### 4.1 Public API

No changes:

- `pub fn diff_workbooks(...) -> DiffReport`
- `pub fn diff_grids_database_mode(...) -> DiffReport` (database mode untouched).
- Diff report JSON representation remains structurally identical; only the **content** (presence of `BlockMovedRows`) changes for qualifying fuzzy-move cases.

### 4.2 Internal APIs and types

#### New or extended functions (internal)

- In `core/src/row_alignment.rs`:

  ```rust
  // Exact move detector remains as-is.
  pub(crate) fn detect_exact_row_block_move(old: &Grid, new: &Grid) -> Option<RowBlockMove>;

  // New fuzzy detector (name illustrative; final name can differ if consistent):
  pub(crate) fn detect_fuzzy_row_block_move(old: &Grid, new: &Grid) -> Option<RowBlockMove>;
````

* `detect_fuzzy_row_block_move`:

  * Applies the size/low-info/repetition guards.
  * Identifies at most one unambiguous `(src_start_row, dst_start_row, row_count)` candidate.
  * Uses block similarity (Jaccard on row tokens) to decide acceptance.
  * Returns `None` on any ambiguity or insufficient similarity.

* In `core/src/engine.rs`:

  ```rust
  fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
      if let Some(mv) = detect_exact_rect_block_move(old, new) {
          emit_rect_block_move(sheet_id, mv, ops);
          return;
      }

      if let Some(mv) = detect_exact_row_block_move(old, new) {
          emit_row_block_move(sheet_id, mv, ops);
      } else if let Some(mv) = detect_exact_column_block_move(old, new) {
          emit_column_block_move(sheet_id, mv, ops);
      } else if let Some(mv) = detect_fuzzy_row_block_move(old, new) {
          // New fuzzy fast path
          emit_row_block_move(sheet_id, mv, ops);
      } else if let Some(alignment) = align_row_changes(old, new) {
          emit_aligned_diffs(sheet_id, old, new, &alignment, ops);
      } else if let Some(alignment) = align_single_column_change(old, new) {
          emit_column_aligned_diffs(sheet_id, old, new, &alignment, ops);
      } else {
          positional_diff(sheet_id, old, new, ops);
      }
  }
  ```

  * Fuzzy move detection runs **after** exact row/column moves and before G8-style row/column alignment.

#### DiffOp

* We continue to use the existing `DiffOp::BlockMovedRows` variant:

  ```rust
  BlockMovedRows {
      sheet: SheetId,
      src_start_row: u32,
      row_count: u32,
      dst_start_row: u32,
      block_hash: Option<u64>,
  }
  ```

* For this milestone:

  * It is acceptable for `block_hash` to remain `None` for fuzzy moves, as tests focus on operation classification, not block hashing.
  * Adding block-hash semantics for fuzzy moves (e.g., Some(fingerprint)) is explicitly **not required** and can be deferred to a later milestone.

---

## 5. Test Plan

All new work must be expressed via tests, following the existing style of Phase 3/4 grid tests.

### 5.1 New fixtures

1. **G13 primary fixture: grid_move_and_edit**

   * Add generator (illustrative name) `RowFuzzyMoveG13Generator` in `fixtures/src/generators/grid.py`:

     * Base grid:

       * One sheet (e.g., `"Data"`).
       * 20–30 rows, 6–8 columns.
       * Distinctive ID column plus additional numeric/text columns.
     * Variant A (`grid_move_and_edit_a.xlsx`):

       * Contains a contiguous 3–6 row block (e.g., rows 4–7) with recognizable data.
     * Variant B (`grid_move_and_edit_b.xlsx`):

       * The same block appears at a different position (e.g., rows 12–15).
       * Exactly one or two cells inside the block have changed values.
   * Register in `fixtures/manifest.yaml` under an ID such as `g13_fuzzy_row_move`:

     * Phase 4, spreadsheet-mode G13 grouping.

### 5.2 Unit tests (Rust)

Add tests to `core/src/row_alignment.rs` (under `#[cfg(test)]`):

1. `detects_fuzzy_row_block_move_with_single_internal_edit`

   * Build small in-memory grids using helper `grid_from_rows`, based on the G11 test pattern but with one edited cell inside the moved block.
   * Assert that:

     * `detect_exact_row_block_move` returns `None`.
     * `detect_fuzzy_row_block_move` returns `Some(RowBlockMove { src_start_row, dst_start_row, row_count })` with expected values.

2. `fuzzy_move_rejects_low_similarity_block`

   * Same base grids, but modify many cells in the moved block.
   * Assert that `detect_fuzzy_row_block_move` returns `None`.

3. `fuzzy_move_bails_on_heavy_repetition_or_ambiguous_candidates`

   * Construct grids with multiple repeated candidate blocks sharing the same hashes.
   * Ensure that repetition or ambiguous candidate detection causes `detect_fuzzy_row_block_move` to return `None`.

4. `fuzzy_move_noop_when_grids_identical`

   * For identical grids, both exact and fuzzy detectors must return `None`.

### 5.3 Integration tests (workbook-level)

Create `core/tests/g13_fuzzy_row_move_grid_workbook_tests.rs`:

1. **`g13_fuzzy_row_move_emits_blockmovedrows_and_celledited`**

   * Load `grid_move_and_edit_a.xlsx` and `grid_move_and_edit_b.xlsx`.
   * Run `diff_workbooks`.
   * Assertions:

     * Exactly one `BlockMovedRows` op:

       * Validate `src_start_row`, `row_count`, and `dst_start_row` match the fixture design.
     * At least one `CellEdited` whose row index lies within the moved block’s source/destination ranges.
     * No `RowAdded` / `RowRemoved` operations whose row indices fall inside that moved block’s rows.
     * Other operations (if any) are limited to the edited cells and any unrelated noise outside the block (but fixture should avoid unrelated changes).

2. **`g13_in_place_edits_do_not_emit_blockmovedrows`**

   * Construct a variant fixture in-memory or via a second manifest entry where the block is not moved, only edited.
   * Assert:

     * `BlockMovedRows` does **not** appear.
     * Edits are represented as `CellEdited` only.

3. **`g13_ambiguous_repeated_blocks_do_not_emit_blockmovedrows`**

   * Build small grids directly in the test (following the style used for G12 rectangular tests).
   * Scenario: two identical copies of a block are swapped or shifted around.
   * Assert:

     * No `BlockMovedRows` ops are emitted.
     * Diff still contains some operations (fallback path).

### 5.4 Regression coverage

* Confirm existing test suites remain green:

  * All G1–G12 grid tests (row/column edits, tail add/remove, row alignment, block moves, rectangular moves).
  * D1 database mode tests.
  * M-diff and PG tests.
* As part of this branch, run:

  * `cargo fmt`
  * `cargo clippy --all-targets`
  * `cargo test` (full suite)

Successful completion of the above tests (especially the new G13 fixtures and workbook tests) will mark **Phase 4 – G13 Fuzzy Move Detection** as implemented in the testing plan.
