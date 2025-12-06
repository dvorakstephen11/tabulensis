# 2025-12-06-g12a-column-block-move – Column block move detection (G12a)

Incremental milestone **G12a**: implement **exact column block move detection** for small spreadsheet-mode grids, emitting `BlockMovedColumns` when a contiguous column block moves without internal edits and with no other concurrent structural changes.

This is a stepping stone toward full **G12 – Column / rectangular block move** and later fuzzy move detection (G13). 

---

## 1. Scope

### 1.1 In scope

**Primary modules and types**

- `core/src/engine.rs`
  - Extend `diff_grids` spreadsheet-mode pipeline to detect column block moves and emit `DiffOp::BlockMovedColumns` as an early, exact fast path. 
- `core/src/column_alignment.rs`
  - Add a `ColumnBlockMove` helper type and a `detect_exact_column_block_move` function (or equivalent) that mirrors `RowBlockMove` and `detect_exact_row_block_move` in `row_alignment.rs`, but operating over columns. 
  - Reuse `GridView` and `HashStats<ColHash>` to implement size/repetition guards and unique-hash checks for safe move detection. 
- `core/src/diff.rs`
  - No structural changes; ensure `DiffOp::BlockMovedColumns` is used as intended by the engine (already defined and tested for JSON shape). 
- `core/tests/g12_column_block_move_grid_workbook_tests.rs` (new)
  - Workbook-level tests for the G12 column move scenario and ambiguity guard.
- `fixtures/src/generators/grid.py`
  - Add a generator mode for G12 column moves (e.g., `column_move_g12`), similar in style to `row_block_move_g11`. 
- `fixtures/manifest.yaml`
  - Add `g12_column_block_move` entry producing `column_move_a.xlsx` / `column_move_b.xlsx` as described in the testing plan. 

**Testing milestones**

- Advances **Phase 4 – Spreadsheet-mode advanced alignment**, specifically the column-move half of **G12 – Column / rectangular block move**. :contentReference[oaicite:24]{index=24}

### 1.2 Out of scope (for this cycle)

- Rectangular move detection (the `rect_block_move_{a,b}.xlsx` case in G12) – left for a later milestone (e.g., G12b). For now, these cases continue to follow existing behavior (likely delete+add and/or CellEdited noise). :contentReference[oaicite:25]{index=25}
- Fuzzy (edited) move detection for rows, columns, or rectangles (G13 and UC-12). 
- Database-mode alignment (D1–D4) and keyed diff behavior. 
- Changes to public crate-level APIs (`diff_workbooks`, JSON helpers) beyond producing `BlockMovedColumns` in more scenarios (wire format already stable from PG4). 
- Performance optimization on very large grids beyond the existing column-alignment guards; this milestone is limited to the **same small-grid envelope** as current G8/G9/G10/G11 logic. 

---

## 2. Behavioral contract

All examples refer to **spreadsheet mode** (positional identity) and **single-sheet** scenarios for simplicity; contracts apply per sheet.

### 2.1 Simple column move (G12 positive case)

Fixture sketch from testing plan: `column_move_{a,b}.xlsx`. :contentReference[oaicite:30]{index=30}

- **Grid A**:
  - Sheet `Data`, columns A–H.
  - Column C has distinctive header and data; other columns differ in content so C’s column hash is unique.
- **Grid B**:
  - Same sheet and content, except **column C is moved to position F**.
  - No cells in the moved column are edited; no other structural changes.

**Expected diff behavior**

- `diff_workbooks(&wb_a, &wb_b)` returns a `DiffReport` with:
  - Exactly **one** operation: `DiffOp::BlockMovedColumns`.
  - Fields:
    - `sheet == "Data"` (matching the fixture).
    - `src_start_col` = logical zero-based index of column C in A (e.g., `2`).
    - `col_count` = `1` (single column move).
    - `dst_start_col` = logical zero-based index of the insertion position in B (e.g., `5` if moved to be the 6th column).
    - `block_hash.is_none()` for now (engine does not compute block hashes yet; PG4 already allows `None`). 
- **No other grid ops** for that sheet:
  - No `ColumnAdded` or `ColumnRemoved`.
  - No `RowAdded` or `RowRemoved`.
  - No `CellEdited` inside the moved column or elsewhere.

In words: a pure horizontal move of one distinctive column should be summarized as a move, not as deletion + addition + cell edits.

### 2.2 Ambiguous repeated columns (negative guard)

When the moved block is not uniquely identifiable by its hashes (e.g., duplicate columns with identical contents), the engine **must not** emit a `BlockMovedColumns` operation, to avoid guessing.

Example (in-memory grid test, no fixtures required):

- Grid A:
  - Columns 0–3: `[A1,B1]`-style numeric columns where the first two columns are identical and the last two are identical.
- Grid B:
  - Same columns but with the pair swapped (e.g., columns [2,3] moved in front of [0,1]).

**Expected behavior**

- `diff_workbooks` produces **no** `DiffOp::BlockMovedColumns`.
- It is acceptable (and expected) that the engine falls back to positional behavior:
  - either noisy `CellEdited` ops for the swapped columns,
  - or a combination of column adds/removes,
  - but **the only hard requirement** is: ambiguous situations must **not** be reported as a clean column block move.

This mirrors the guard behavior for row block moves (G11), where repeated rows cause the detector to bail out and revert to positional diff. :contentReference[oaicite:32]{index=32}

### 2.3 Non-move scenarios remain unchanged

- Existing contracts for:
  - G1–G7 (simple cell edits, row/column append/truncate),
  - G8 (single row insert/delete),
  - G9 (single column insert/delete),
  - G10 (row block insert/delete), and
  - G11 (row block move)
  **must remain true**. 
- Adding column block move detection must **not**:
  - introduce new spurious `BlockMovedColumns` in existing fixtures,
  - change the output of previously passing tests.

The detector is an **additional fast path** for specific patterns, not a global behavioral change.

---

## 3. Constraints

### 3.1 Size and complexity bounds

To keep this milestone tractable and avoid early performance regressions:

- Column block move detection is only attempted when:
  - Grids have equal shape (`nrows` and `ncols` identical).
  - `nrows <= 2000` and `ncols <= 64`, matching existing row/column alignment bounds. 
- Detector must use `GridView` and `HashStats<ColHash>` (or equivalent) rather than scanning every cell repeatedly:
  - Build `GridView` once per `diff_grids` call (already done for alignment).
  - Column-level metadata (hashes, non-empty counts) should drive detection.

**Complexity target**

- For eligible grids, the additional detector should run in roughly **O(C)** or **O(C²)** with small C (≤ 64):
  - Linear or quadratic scans over column metadata and hashes are acceptable.
  - Do **not** introduce any algorithm with worst-case complexity dependent on `nrows * ncols` beyond the existing positional diff baseline for small grids.

### 3.2 Repetition and low-information guards

- Reuse or mirror guards already present in row/column alignment and row block move detection:
  - **Repetition guard**: if any column hash appears more than `MAX_HASH_REPEAT` times (currently 8) in either grid, bail out of column-move detection to avoid ambiguous matches. 
  - **Low-info guard**: although `ColMeta` does not currently track a low-info flag like rows, use a conservative heuristic:
    - if too many columns are all-blank or near-empty on both sides (e.g., > 50% with `non_blank_count == 0`), bail out to avoid misclassifying shifts among blank columns as moves.
- The detector must treat these guards as **hard bail-outs**: when they trip, do not attempt any move inference.

### 3.3 Determinism and ordering

- Detection must be deterministic:
  - No reliance on `HashMap` iteration order.
  - Operations must be emitted in a stable order.
- Integration with `diff_grids` must respect existing ordering invariants for DiffOps, at least for all currently tested outputs:
  - For the column move fast path, it’s acceptable (and simplest) to **short-circuit** and emit only `BlockMovedColumns` for the sheet (just like row block moves currently short-circuit with `BlockMovedRows`). 

### 3.4 Memory

- No new long-lived allocations proportional to `nrows * ncols`.
- Any temporary collections used for detection (e.g. vectors of candidate column indices) should be bounded by the number of columns (≤ 64 under guards).

---

## 4. Interfaces

### 4.1 Existing public types (must remain stable)

- `DiffOp::BlockMovedColumns { sheet: String, src_start_col: u32, col_count: u32, dst_start_col: u32, block_hash: Option<u64> }`:
  - Do **not** change field names or JSON tags; PG4 tests already lock these in. 
- `diff_workbooks(&Workbook, &Workbook) -> DiffReport`:
  - Signature unchanged; this cycle only adds new possible operations in the report.

### 4.2 Internal helper APIs (may be added/extended)

- New helper struct in `column_alignment.rs` (name suggestion):

  ```rust
  pub(crate) struct ColumnBlockMove {
      pub sheet: String,
      pub src_start_col: u32,
      pub col_count: u32,
      pub dst_start_col: u32,
  }
````

* New helper function (or equivalent) in `column_alignment.rs`:

  ```rust
  pub(crate) fn detect_exact_column_block_move(
      sheet_name: &str,
      grid_a: &Grid,
      grid_b: &Grid,
  ) -> Option<ColumnBlockMove>;
  ```

  Contract:

  * Returns `Some` only when:

    * Shapes match and pass size/repetition/low-info guards.
    * There exists exactly one contiguous block of columns in A whose hashes occur as a contiguous block at a different position in B.
    * All columns outside the candidate block match 1:1 in order.
  * Returns `None` for:

    * Any internal cell edits within the candidate block.
    * Multiple moved blocks or overlapping candidates.
    * Ambiguous matches where block columns are not uniquely identified by hash.

* New internal helper in `engine.rs`:

  ```rust
  fn emit_column_block_move(report: &mut DiffReport, sheet: &str, mv: &ColumnBlockMove) {
      report.ops.push(DiffOp::BlockMovedColumns {
          sheet: sheet.to_string(),
          src_start_col: mv.src_start_col,
          col_count: mv.col_count,
          dst_start_col: mv.dst_start_col,
          block_hash: None,
      });
  }
  ```

### 4.3 Integration point in `diff_grids`

* Extend the early fast-path logic in `diff_grids` (after row block moves, before row/column alignment) roughly like:

  1. If `detect_exact_row_block_move` returns Some → emit `BlockMovedRows` and return (existing behavior).
  2. Else, call `detect_exact_column_block_move`:

     * If Some(mv) → emit `BlockMovedColumns` and return.
  3. Else, proceed with existing row alignment, column alignment, and positional diff logic.

This ordering keeps row-move and column-move detection symmetric and ensures pure moves are handled as special cases before more general alignment.

---

## 5. Test plan

All new behavior must be pinned by tests. No changes should be made without corresponding tests.

### 5.1 New fixtures

**Manifest**

* Add a Phase 4 G12 entry in `fixtures/manifest.yaml`:

  ```yaml
  # --- Phase 4: Spreadsheet-mode G12 (column move only – G12a) ---
  - id: "g12_column_block_move"
    generator: "column_move_g12"
    args:
      sheet: "Data"
      cols: 8
      data_rows: 9
      src_col: 3      # 1-based: C
      dst_col: 6      # 1-based: F
    output:
      - "column_move_a.xlsx"
      - "column_move_b.xlsx"
  ```

  The exact argument names can follow the pattern used by existing G8/G9/G10/G11 generators. 

**Generator**

* In `fixtures/src/generators/grid.py`:

  * Implement `ColumnMoveG12Generator` (or add a `mode: "column_move"` branch to the existing column-alignment generator) that:

    * Builds a sheet `Data` with 8 columns:

      * Column C has a distinctive header (e.g. `"C_key"`) and data values that differ from all other columns (e.g. `100, 200, ...`).
      * Other columns contain different numeric patterns or labels so C’s column hash is unique.
    * Produces `column_move_a.xlsx` with the original ordering.
    * Produces `column_move_b.xlsx` where column C’s entire content is moved to position F and the remainder of the grid is otherwise unchanged.

### 5.2 New Rust tests

#### 5.2.1 Workbook-level G12 tests

Create `core/tests/g12_column_block_move_grid_workbook_tests.rs`:

1. **`g12_column_move_emits_single_blockmovedcolumns`**

   * Load fixtures:

     ```rust
     let wb_a = open_workbook(fixture_path("column_move_a.xlsx")).unwrap();
     let wb_b = open_workbook(fixture_path("column_move_b.xlsx")).unwrap();
     let report = diff_workbooks(&wb_a, &wb_b);
     ```

   * Assertions:

     * `report.ops.len() == 1`.
     * First op is `DiffOp::BlockMovedColumns { sheet, src_start_col, col_count, dst_start_col, block_hash }`.

       * `sheet == "Data"`.
       * `src_start_col` matches the 0-based index of column C in `column_move_a.xlsx`.
       * `col_count == 1`.
       * `dst_start_col` matches the 0-based index for the target position in `column_move_b.xlsx`.
       * `block_hash.is_none()`.
     * No other Block/Row/Column/Cell operations:

       ```rust
       assert!(!report.ops.iter().any(|op| matches!(op, DiffOp::ColumnAdded { .. })));
       assert!(!report.ops.iter().any(|op| matches!(op, DiffOp::ColumnRemoved { .. })));
       assert!(!report.ops.iter().any(|op| matches!(op, DiffOp::RowAdded { .. })));
       assert!(!report.ops.iter().any(|op| matches!(op, DiffOp::RowRemoved { .. })));
       assert!(!report.ops.iter().any(|op| matches!(op, DiffOp::CellEdited { .. })));
       ```

2. **`g12_repeated_columns_do_not_emit_blockmovedcolumns`**

   * Build two small in-memory grids using `grid_from_numbers` and `single_sheet_workbook` helpers (as in the G11 tests). 

   * Example:

     ```rust
     let grid_a = grid_from_numbers(&[
         &[1, 10],
         &[1, 10],
         &[2, 20],
         &[2, 20],
     ]);

     let grid_b = grid_from_numbers(&[
         &[2, 20],
         &[2, 20],
         &[1, 10],
         &[1, 10],
     ]);
     ```

   * Wrap into `Workbook`s and run `diff_workbooks`.

   * Assert:

     * No `DiffOp::BlockMovedColumns` present.
     * At least one other diff op (e.g., `CellEdited` or column add/remove) is present, demonstrating a fallback path rather than “no diff”.

#### 5.2.2 Unit tests in `column_alignment.rs`

Add module-level tests (guarded by `#[cfg(test)]`) to exercise the detector in isolation:

1. **`detect_exact_column_block_move_simple_case`**

   * Construct two small `Grid` instances where a single column with unique content is moved.
   * Call `detect_exact_column_block_move` directly.
   * Assert `Some(ColumnBlockMove { src_start_col, col_count: 1, dst_start_col, .. })` with expected indices.

2. **`detect_exact_column_block_move_rejects_internal_edits`**

   * Same as the previous test, but edit one cell inside the moved column in B.
   * The detector must return `None` (exact, no-internal-edit requirement).

3. **`detect_exact_column_block_move_rejects_repetition`**

   * Construct grids with repeated identical columns and swapped positions.
   * Assert that the detector returns `None`.

These mirror the internal tests used for row block move detection in `row_alignment.rs`.

### 5.3 Regression safety

* Run the existing suite:

  * All PG1–PG6 tests.
  * All G1–G11 grid tests.
  * M1–M6 tests.
* Specifically ensure:

  * `core/tests/g9_column_alignment_grid_workbook_tests.rs` still passes; column insert/delete behavior must not change.
  * `core/tests/g11_row_block_move_grid_workbook_tests.rs` still passes; row-move behavior and BlockMovedRows semantics unchanged. 

### 5.4 Success criteria for this milestone

This G12a milestone is considered **complete** when:

* The new fixtures (`column_move_a.xlsx`, `column_move_b.xlsx`) are generated and referenced in `manifest.yaml`.
* `g12_column_block_move_grid_workbook_tests.rs` passes, confirming:

  * Exact column move → single `BlockMovedColumns` op and no noise.
  * Ambiguous repeated columns → no `BlockMovedColumns`.
* All existing tests (PG1–PG6, G1–G11, M1–M6) continue to pass with no behavior regressions.
* No new clippy warnings are introduced, and the code compiles for both native and WASM targets following the usual project checks.

This leaves rectangular block moves and fuzzy moves explicitly for follow-up milestones (e.g., G12b, G13), building on the same move-detection foundations.
