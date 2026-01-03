# 2025-12-06-g11-row-block-move – Mini-Spec

## 1. Scope

### 1.1 Rust modules and types

**Primary modules**

- `core/src/engine.rs`
  - `fn diff_grids(...)`
  - New helper for emitting row-block move ops

- `core/src/row_alignment.rs`
  - `RowAlignment`
  - Alignment guards and helpers (size bounds, low_info_dominated, has_heavy_repetition)
  - **New** G11-specific helper for exact row block move detection

- `core/src/grid_view.rs`
  - `GridView`, `RowMeta`, `RowHash`
  - `HashStats<RowHash>`

**Types and APIs in play**

- `excel_diff::DiffOp::BlockMovedRows` # :contentReference[oaicite:16]{index=16}
- `excel_diff::DiffOp::{RowAdded, RowRemoved, CellEdited}`
- `excel_diff::DiffReport`
- `excel_diff::SheetId`
- `excel_diff::workbook::Grid`

**Fixtures & test harness**

- `fixtures/manifest.yaml`
  - **New** fixture id: `g11_row_block_move`

- `fixtures/src/generators/grid.py`
  - **New** generator: `RowBlockMoveG11Generator` exposed under id `"row_block_move_g11"`

- `core/tests`
  - **New** integration test file: `g11_row_block_move_grid_workbook_tests.rs`
  - Optional: unit tests inside `row_alignment.rs` for pure move detection helper

This branch does **not** change:

- DataMashup / M-diff modules
- Database mode (D1–D10)
- Output/serialization formats beyond using existing `DiffOp::BlockMovedRows`

---

## 2. Behavioral Contract

### 2.1 High-level behavior (G11)

When a contiguous block of rows moves within a sheet, with **no internal edits** and **no other structural changes**, the diff engine must:

- Emit a **single** `DiffOp::BlockMovedRows` describing the move.
- Emit **no** `RowAdded` or `RowRemoved` operations for the moved rows.
- Emit **no** `CellEdited` operations for cells inside the moved block. # 
- Emit **no** `CellEdited` operations outside the block (the remainder of the sheet is identical).

Contract is restricted to:

- Spreadsheet mode (row order semantically meaningful).
- Same sheet, same grid dimensions (`nrows` and `ncols` identical).
- Exactly one moved row block, non-overlapping with its destination.
- Block content identical at source and destination.

G11 does **not** require:

- Support for multiple independent row moves in one sheet.
- Moves that overlap or partially overlap their original positions.
- Any fuzzy move detection (moves with internal edits): this remains G12/G13. # 

### 2.2 Concrete example

**Sheet A (1-based rows; 0-based indices in parens):**

- Rows 1–4 (0–3): ordinary rows
- Rows 5–8 (4–7): distinctive “BLOCK” rows
- Rows 9–20 (8–19): ordinary rows

**Sheet B:**

- Rows 1–12 (0–11): rows 1–4 and 9–16 from A, in order
- Rows 13–16 (12–15): the BLOCK rows from A (originally 5–8)
- Rows 17–20 (16–19): remaining rows from A

In zero-based coordinates:

- `src_start_row = 4`, `row_count = 4`
- `dst_start_row = 12`

Expected diff behavior:

- `DiffReport::ops` contains exactly one operation:

  ```rust
  DiffOp::BlockMovedRows {
      sheet: "Sheet1".to_string(),
      src_start_row: 4,
      row_count: 4,
      dst_start_row: 12,
      block_hash: None,
  }
````

* There are **no** `RowAdded`, `RowRemoved`, or `CellEdited` entries in the report.

### 2.3 Failure-mode behavior

If the sheet does **not** fit the G11 constraints (for example):

* Block content changed internally.
* Multiple or ambiguous repeated rows inside the candidate block.
* Very large sheet beyond G8/G10 alignment bounds.
* Too many repeated or low-information rows per HashStats.

Then:

* The new move detector must return `None`.
* `diff_grids` must fall back to existing behavior: row alignment (G8/G10), column alignment (G9), or positional diff, whichever applies. #
* Existing tests for G1–G10, PG5, etc. must remain passing and unchanged in expectations.

False positives are **not allowed**; ambiguous scenarios must fall back rather than mis-reporting moves.

---

## 3. Constraints and Invariants

### 3.1 Grid and data constraints

The G11 implementation must respect the same guardrails as G8/G10 row alignment:

* **Size bounds**: only attempt move detection when

  * `max(old.nrows, new.nrows) <= 2_000`
  * `max(old.ncols, new.ncols) <= 64` #
* **Shape equality**:

  * `old.nrows == new.nrows`
  * `old.ncols == new.ncols`
* **Row info quality**:

  * Abort if either side is `low_info_dominated(view)` (more than half of rows are low-info). #
  * Abort if `has_heavy_repetition(stats)` (max row-hash frequency exceeds `MAX_HASH_REPEAT`). # 

Additional G11-specific constraints to keep detection safe:

* **Uniqueness in the candidate block**:

  * All row hashes in the moved block must have frequency `== 1` in both A and B (`HashStats::freq_a` and `freq_b`), to avoid ambiguous matches in repetitive data. #
* **Single block only**:

  * The algorithm validates that, after removing the candidate block from A and B, the remaining row-hash sequence is identical. If not, detection fails.

### 3.2 Performance constraints

* Complexity target: **O(R)** or **O(R log R)** on row count R within small-grid bounds.

  * One or two linear scans over row metadata.
  * One extra pass over the candidate block for uniqueness checks.
* No additional heap allocations beyond:

  * Local `Vec`/maps sized O(R) at most.
  * Reuse of `GridView` and `HashStats<RowHash>` which are already built in existing alignment paths.

We explicitly **do not** attempt to implement the full AMR pipeline in this branch; this is an exact move detector layered on top of existing primitives. #

### 3.3 Diff invariants

Given a scenario that passes all guards and is classified as a pure block move:

* The engine must emit **one** `BlockMovedRows` operation and **no other operations**.
* `src_start_row`, `row_count`, and `dst_start_row` must correspond to actual row indices in grid A/B:

  * 0-based indices matching `Grid`'s row indexing.
  * `row_count` equals the number of moved rows.
* `block_hash` will remain `None` in this branch; block hashing is deferred to a later performance/observability pass.

Outside G11 scenarios, **no change** to existing invariants:

* G8/G10 still produce row add/remove ops for inserts/deletes.
* Positional diff still handles same-shape edits without structural alignment when no special case applies.

---

## 4. Interfaces and Internal APIs

### 4.1 New helper in `row_alignment.rs`

Add a public-to-crate helper, keeping the module as the home of row-diff heuristics:

```rust
// row_alignment.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowBlockMove {
    pub src_start_row: u32,
    pub dst_start_row: u32,
    pub row_count: u32,
}

pub(crate) fn detect_exact_row_block_move(
    old: &Grid,
    new: &Grid,
) -> Option<RowBlockMove> {
    // Implementation described in Section 5.2
}
```

Key properties:

* Uses `GridView::from_grid` and `HashStats::from_row_meta` for row hashes and frequency statistics.
* Applies the same `is_within_size_bounds`, `low_info_dominated`, and `has_heavy_repetition` guards as `align_row_changes`. #
* Returns `None` on all ambiguous or unsupported cases.

This function is **internal** to the crate and only called from `engine::diff_grids` and `row_alignment` unit tests.

### 4.2 Engine integration in `engine.rs`

Update `diff_grids` to consult the move detector before alignment:

```rust
fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    if let Some(mv) = detect_exact_row_block_move(old, new) {
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

New helper for operation emission:

```rust
fn emit_row_block_move(
    sheet_id: &SheetId,
    mv: RowBlockMove,
    ops: &mut Vec<DiffOp>,
) {
    ops.push(DiffOp::BlockMovedRows {
        sheet: sheet_id.clone(),
        src_start_row: mv.src_start_row,
        row_count: mv.row_count,
        dst_start_row: mv.dst_start_row,
        block_hash: None,
    });
}
```

Notes:

* No cell-level diff is performed in the move path; the move detector already requires that all non-block rows match exactly, so extra work is unnecessary.
* This leaves room for G12/G13 to re-use the same `RowBlockMove` struct while adding `is_fuzzy` / similarity and internal alignment logic.

---

## 5. Algorithm Sketch (Detection Logic)

This is the intended behavior for `detect_exact_row_block_move`.

### 5.1 Precondition checks

1. If shape differs (`nrows` or `ncols`), bail out.
2. If either grid is empty, bail out.
3. If size bounds exceeded or low-info / repetition guards fail, bail out.
4. Build `GridView` for both grids and `HashStats<RowHash>` from row metadata.

If all row hashes at each index match (`meta_a[i].hash == meta_b[i].hash` for all i), grids are identical → no move, return `None`.

### 5.2 Candidate block detection

Let `n = old.nrows` and `meta_a`, `meta_b` be row metadata arrays.

1. **Find first mismatch from top**:

   * `prefix = smallest i in [0, n)` where `meta_a[i].hash != meta_b[i].hash`.
   * If none, return `None` (already covered above).

2. **Find first mismatch from bottom**:

   * `suffix_len = number of matching rows from the bottom`:
     increment while `meta_a[n-1-j].hash == meta_b[n-1-j].hash`.
   * `tail_start = n - suffix_len`.
   * The mismatch window is `[prefix, tail_start)`.

3. **Find candidate source block**:

   * Let `h = meta_b[prefix].hash` (hash for the first mismatching row in B).
   * Search `meta_a` in `[prefix, tail_start)` for first index `src_start` with `meta_a[src_start].hash == h`.
   * If not found, return `None`.

4. **Grow block length**:

   * Starting from `(src_start, prefix)`, grow `len` while hashes match pairwise:

     * While `src_start + len < tail_start` and `prefix + len < tail_start` and
       `meta_a[src_start + len].hash == meta_b[prefix + len].hash`, increment `len`.
   * If `len == 0`, return `None`.

5. **Check non-overlap**:

   * Let `src_end = src_start + len`, `dst_start = prefix`, `dst_end = dst_start + len`.
   * Require non-overlapping ranges: `(src_end <= dst_start) || (dst_end <= src_start)`.
   * If ranges overlap, return `None`.

### 5.3 Validation

1. **Sequence equality after removing block**:

   Simulate removal of `[src_start, src_end)` from A and `[dst_start, dst_end)` from B and verify the remaining sequences of hashes are identical:

   * Two indices `ia`, `ib` traverse `[0, n)` skipping their respective block ranges.
   * At each step, `meta_a[ia].hash` must equal `meta_b[ib].hash`.
   * If any mismatch or uneven consumption occurs, return `None`.

2. **Uniqueness of block rows**:

   For each row index `k` in `[src_start, src_end)`:

   * Let `h = meta_a[k].hash`.
   * Require `HashStats::freq_a[h] == 1` and `freq_b[h] == 1`.
   * If any row hash appears more than once on either side, return `None`.

3. **Construct result**:

   * Map logical indices:

     ```rust
     let src_start_row = meta_a[src_start].row_idx;
     let dst_start_row = meta_b[dst_start].row_idx;
     let row_count = len;
     ```

   * Return `Some(RowBlockMove { src_start_row, dst_start_row, row_count })`.

---

## 6. Test Plan

All new work must be expressed through tests. This branch defines the following new tests.

### 6.1 New fixtures

Extend `fixtures/manifest.yaml` with a new G11 entry:

```yaml
  # --- Phase 4: Spreadsheet-mode G11 ---
  - id: "g11_row_block_move"
    generator: "row_block_move_g11"
    args:
      sheet: "Sheet1"
      total_rows: 20
      cols: 5
      block_rows: 4
      src_start: 5    # 1-based in A
      dst_start: 13   # 1-based in B
    output:
      - "row_block_move_a.xlsx"
      - "row_block_move_b.xlsx"
```

Implement `RowBlockMoveG11Generator` in `fixtures/src/generators/grid.py`:

* Build base data as a list of `total_rows` rows, each with `cols` cells.
* For the block rows, give easily identifiable values (e.g. `"BLOCK_rX_cY"`) to help debugging.
* A:

  * Rows are `[1..total_rows]` in order, with block at `src_start..src_start+block_rows-1`.
* B:

  * Same rows, but the block is cut out and reinserted starting at `dst_start`.

Generator must produce idempotent, deterministic fixtures as existing generators do. #

### 6.2 Workbook-level G11 tests

New file: `core/tests/g11_row_block_move_grid_workbook_tests.rs`.

#### 6.2.1 Positive case – exact block move

Test name: `g11_row_block_move_emits_single_blockmovedrows`

Steps:

1. Open fixtures:

   ```rust
   let wb_a = open_workbook(fixture_path("row_block_move_a.xlsx")).unwrap();
   let wb_b = open_workbook(fixture_path("row_block_move_b.xlsx")).unwrap();
   ```

2. Compute diff:

   ```rust
   let report = diff_workbooks(&wb_a, &wb_b);
   ```

3. Extract `BlockMovedRows` ops:

   * Assert exactly one such op.
   * Assert:

     * `sheet == "Sheet1"`
     * `src_start_row == 4` (1-based 5 → 0-based 4)
     * `row_count == 4`
     * `dst_start_row == 12` (1-based 13 → 0-based 12)
     * `block_hash.is_none()`.

4. Assert **no noise**:

   * No `RowAdded` or `RowRemoved` anywhere in the report.
   * No `CellEdited` anywhere in the report.

This directly codifies the testing plan’s “single BlockMovedRows and no CellEdited” checks. #

#### 6.2.2 Negative case – ambiguous move (repetition)

Optional but recommended test: `g11_repeated_rows_do_not_emit_blockmove`.

* Build two small grids in code (no fixture):

  * A: 6 rows, with some repeated rows (e.g., rows 2–3 identical, rows 4–5 identical).
  * B: rearrange rows such that there is a plausible “block move”, but row hashes are non-unique.

* Use `single_sheet_workbook` helper (as in PG5 tests) to construct `Workbook` instances. #

* Assert that:

  * `diff_workbooks` emits **no** `BlockMovedRows`.
  * Either we get positional `CellEdited` noise or row add/remove operations, but not a move.

This ensures the uniqueness guard is respected and we don’t misclassify repeated patterns as moves.

### 6.3 Row-alignment unit tests

Inside `row_alignment.rs`’s test module, add a unit test leveraging `grid_from_rows`:

* Construct in-memory `Grid` pairs matching the G11 scenario with integer values.
* Call `detect_exact_row_block_move(&grid_a, &grid_b)` directly and assert:

  * `Some(RowBlockMove { src_start_row: 4, dst_start_row: 12, row_count: 4 })`.

Also add at least one negative case (different content in the block) asserting `None`.

These tests keep `detect_exact_row_block_move` covered even if higher-level behavior changes later.

---

## 7. Out of Scope / Future Work

This branch explicitly **does not** attempt:

* Fuzzy moves (blocks with internal edits, G12/G13).
* Column or rectangular moves (`BlockMovedColumns`, `BlockMovedRect`). #
* Database-mode keyed comparisons (D1–D10).
* Block-level hashing or similarity scoring; `block_hash` stays `None`.

Future branches can:

* Extend `RowBlockMove` with `similarity` and `is_fuzzy`.
* Integrate with an AMR-style `ValidatedMove` structure.
* Generalize detection to multi-block and fuzzy moves while preserving G11’s exact behavior as a fast path.

This mini-spec is complete for implementing milestone G11 as defined in the testing plan while staying compatible with the long-term AMR design.
