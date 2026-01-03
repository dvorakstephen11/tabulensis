<!-- docs/meta/plans/2025-12-05-g8-row-alignment/spec.md -->

# 2025-12-05-g8-row-alignment – Minimal spreadsheet row alignment for G8

## 1. Scope

This cycle introduces a **minimal spreadsheet‑mode row alignment path** for the grid diff engine and wires it into the existing workbook diff pipeline.

### In scope

Rust modules / types:

- `core/src/engine.rs`
  - `diff_grids` implementation and any small internal helpers it uses.
- `core/src/grid_view.rs`
  - Use `GridView::from_grid` and `HashStats` for row fingerprinting and frequency analysis; no major structural changes expected. 
- (New) small internal alignment helper module (name flexible, e.g. `core/src/row_alignment.rs`):
  - Minimal row-alignment routine for the “single unique row inserted/deleted” case.
  - Internal struct(s) to represent row alignment results (can be a pared‑down `RowAlignment` type consistent with the unified spec’s alignment result section, but only matched/inserted/deleted are required for this cycle). 
- Tests:
  - New workbook‑level grid tests for G8 scenarios (see §5).
  - Optional targeted unit tests for the alignment helper using in‑memory `Grid`/`GridView` values.

Out-of-scope (must **not** be attempted in this cycle):

- General AMR implementation (rare anchors, block moves, gap‑filling strategies beyond the simple G8 case).
- Column alignment or any non‑tail column insert/delete logic.
- Database‑mode alignment (key-based joins, LAPJV, etc.). 
- Large-grid adversarial scenarios (G8a and beyond) or perf benchmarking (P1/P2).

The existing public API (`diff_workbooks`, `DiffOp`, JSON output) must remain stable; only behaviour for the specific G8 scenarios is allowed to change.

---

## 2. Behavioral contract

### 2.1 Baseline: current behaviour (for context)

Today, `diff_grids` is positional:

- It compares cells for all rows `0..min(rows_a, rows_b)` and columns `0..min(cols_a, cols_b)` using identical `(row, col)` indices on both sides.
- It only emits `RowAdded`/`RowRemoved` and `ColumnAdded`/`ColumnRemoved` for **tail** differences (rows/columns that exist only at the bottom/right of one grid). 

Consequences for G8‑style changes:

- If workbook A has rows `1..10` and workbook B is identical but with a new row inserted between rows 5 and 6, the current algorithm:
  - Compares A row 5 vs B row 5 (mismatched), A row 6 vs B row 6 (actually B’s old row 7), etc.
  - Emits a **large block of `CellEdited` ops** for rows below the insertion point, plus some tail row add/remove structure ops.
- This contradicts the G8 milestone intent, which wants “row insert/delete in the middle” to be expressed primarily as **row structure changes**, not as unrelated cell edits. 

### 2.2 Target behaviour for this cycle

This cycle introduces a **narrow, deterministic alignment path** for small spreadsheet‑mode grids where:

- All non‑insert/delete rows are identical between A and B (row content matches exactly).
- One row is inserted in B (G8 insert) **or** one row is removed from A (G8 delete).
- There are no column count changes (same number of columns in A and B).

#### Example 1 – Single row inserted in the middle (G8 insert)

Fixture sketch (matches testing plan G8):

- `row_insert_middle_a.xlsx`:
  - Sheet `Sheet1`, 10 data rows (1..10), small number of columns (e.g., 3–5).
- `row_insert_middle_b.xlsx`:
  - Same as A, but with a new row inserted between original rows 5 and 6.
  - All rows above and below insertion are otherwise identical. :contentReference[oaicite:22]{index=22}

Expected diff (spreadsheet‑mode):

- Exactly **one** `RowAdded` operation:
  - `sheet`: `"Sheet1"`
  - `row_idx`: 5 (0‑based index of the new row in **B**; i.e., inserted between original 4 and 5).
  - `row_signature`: still `None` for this cycle.
- For all other rows:
  - Rows 0–4: treated as unchanged.
  - Rows 6–10 in B should align with rows 5–9 in A.
  - No spurious `CellEdited` ops for rows below the insertion point.

Informal contract:

> “Inserting a unique row in the middle of a small sheet yields a single `RowAdded` at the correct position; unchanged rows keep their alignment.”

#### Example 2 – Single row deleted in the middle (G8 delete)

- `row_delete_middle_a.xlsx`:
  - Same base as `row_insert_middle_a.xlsx`.
- `row_delete_middle_b.xlsx`:
  - Identical to A except one row (e.g. original row 6) is removed; rows below shift up. :contentReference[oaicite:23]{index=23}

Expected diff:

- Exactly **one** `RowRemoved` operation:
  - `sheet`: `"Sheet1"`
  - `row_idx`: 5 (0‑based index of the deleted row in **A**).
  - `row_signature`: `None`.
- Rows above the deleted row are unchanged.
- Rows below the deletion line are correctly aligned; no block of `CellEdited` noise.

Informal contract:

> “Deleting a unique row in the middle yields a single `RowRemoved` at that row index; all other rows line up.”

#### 2.3 Behaviour outside the narrow G8 envelope

To keep the cycle small and safe:

- The **alignment path must be gated** behind simple, explicit conditions (see §3).
- If those conditions aren’t met, `diff_grids` must fall back to the existing positional algorithm.

Concretely, alignment should **not** trigger when:

- Row count difference is more than 1.
- There are column count changes.
- GridView/HashStats classification detects:
  - Multiple unmatched hashes (more than one insert/delete),
  - Non‑monotonic row matches (i.e., evidence of row moves),
  - Heavy repetition / low‑info rows dominating the sheet. 

In all such cases, behaviour must remain identical to current tests:

- G1–G7 workbook tests, PG5/PG6 grid tests, and all existing integration tests must still pass unchanged. 

---

## 3. Constraints and invariants

### 3.1 Algorithmic constraints

- **Complexity**:
  - For this cycle, alignment is only enabled for modest‑sized grids (for example, `nrows <= 2_000`, `ncols <= 64`; exact thresholds left to the implementer but must be small and documented).
  - Within that window, the row alignment routine can afford a simple O(R)–O(R²) scan because R is capped by the gating condition.
  - Larger or more complex grids must immediately fall back to the existing positional diff, preserving current complexity guarantees and leaving large-grid performance for later AMR-oriented cycles. 

- **Determinism**:
  - Given the same inputs, the choice between “aligned” vs “positional fallback” must be deterministic (no RNG, no order-dependent hash map iteration).

- **No regression on tail semantics**:
  - Existing behaviour for pure tail row/column appends/deletes (G6, PG5 tail tests) must remain unchanged:
    - Tail row adds/removes still produce `RowAdded`/`RowRemoved` at the end, with no new mid‑sheet structure ops. 

### 3.2 Memory and streaming constraints

- ``GridView::from_grid`` and `HashStats` are already designed to operate in O(R) space for row metadata; this cycle may reuse them but must not:
  - Store full row contents beyond what they already hold.
  - Introduce any O(R × C) auxiliary structures. 
- All new allocations must be bounded by the gating thresholds (small vectors for row matches, simple maps); they must drop promptly after `diff_grids` returns.
- No changes to streaming/large‑file strategy are allowed; the large-file/streaming plan in the unified spec remains future work. 

### 3.3 Invariants

- `DiffOp` schema invariants from PG4 must hold:
  - Variant names unchanged.
  - `RowAdded`/`RowRemoved` continue to use zero-based `row_idx` with semantics “index in the new grid” for added rows and “index in the old grid” for removed rows, as already tested by existing tail-row scenarios. 
- For G8-aligned scenarios:
  - No extra `CellEdited` ops below the insert/delete line.
  - No `RowAdded`/`RowRemoved` emitted outside the intended row index.

---

## 4. Interfaces

### 4.1 Public APIs / IR types

- **Stable this cycle**:
  - `pub fn diff_workbooks(...) -> DiffReport`
  - `DiffOp` enum and its existing variants and fields.
  - `Workbook`, `Sheet`, `Grid`, and snapshot types used in tests.
  - JSON serialization format used in `output::json`. 

- **Internal types that may be introduced/extended**:
  - A non‑exported `RowAlignment` or similar struct, roughly:
    ```text
    struct RowAlignment {
      matched: Vec<(u32, u32)>,   // (row_idx_a, row_idx_b)
      inserted: Vec<u32>,         // row indices in B
      deleted: Vec<u32>,         // row indices in A
    }
    ```
    - Moves can be omitted for now (empty).
    - This aligns conceptually with the “Alignment Result Structures” section of the unified spec but is deliberately minimal for G8. 
  - Private helper functions for:
    - Building row hash sequences from `GridView` (`row_meta`).
    - Discovering unique row matches (based on `HashStats`).
    - Deciding whether the “single-insert/delete” pattern holds and returning `Option<RowAlignment>`.

These helpers must stay internal to the `core` crate; they are not part of the public API surface yet.

---

## 5. Test plan

All new work must be expressed through tests. This cycle primarily targets **Phase 4 – G8** and uses both workbook-level and (optionally) grid-level tests.

### 5.1 New fixtures

Add to `fixtures/manifest.yaml` and generate corresponding `.xlsx` files (using the existing Python fixture generator):

1. `row_insert_middle_a.xlsx`
   - Single sheet `Sheet1`.
   - 10 rows × N columns (e.g., 5), with simple numeric or text data so each row is uniquely identifiable.
   - No formulas necessary; pure values are fine.

2. `row_insert_middle_b.xlsx`
   - Clone of `row_insert_middle_a.xlsx`.
   - Insert one new row between original rows 5 and 6 (1‑based), with distinct content so it is clearly not equal to any existing row.

3. `row_delete_middle_a.xlsx`
   - Same structure as `row_insert_middle_a.xlsx`.

4. `row_delete_middle_b.xlsx`
   - Clone of `row_delete_middle_a.xlsx`.
   - Delete one row in the middle (e.g., original row 6), shifting rows below up by one. 

Each fixture entry in the manifest should follow the existing `basic_grid`/`grid_tail_diff` patterns used for G1–G7, so the generator remains consistent. 

### 5.2 New workbook-level tests

Create a new test module, e.g.:

- `core/tests/g8_row_alignment_grid_workbook_tests.rs`

Structure and helpers can mirror `g1_g2_grid_workbook_tests.rs` and `g5_g7_grid_workbook_tests.rs`. 

Tests:

1. `single_row_insert_middle_produces_one_row_added`
   - Open `row_insert_middle_a.xlsx` and `row_insert_middle_b.xlsx`.
   - Run `diff_workbooks`.
   - Filter `DiffOp`s for:
     - `RowAdded`/`RowRemoved` variants.
     - `CellEdited` variants.
   - Assertions:
     - Exactly one `RowAdded`.
     - No `RowRemoved`.
     - The `RowAdded` has:
       - `sheet == "Sheet1"`.
       - `row_idx == 5`.
       - `row_signature.is_none()`.
     - There are **no** `CellEdited` ops for rows >= 5 (or, more strictly, no `CellEdited` ops at all if content is identical except the inserted row).

2. `single_row_delete_middle_produces_one_row_removed`
   - Same pattern with `row_delete_middle_a.xlsx`/`row_delete_middle_b.xlsx`.
   - Assertions:
     - Exactly one `RowRemoved`, no `RowAdded`.
     - `RowRemoved.row_idx == 5`.
     - No `CellEdited` ops at or below the deletion row.

3. `g8_alignment_does_not_trigger_when_rows_differ_beyond_single_insert`
   - Modify one of the fixtures (or create a derivative via generator) where:
     - There is a single row insert in the middle **and** some content change below.
   - For this cycle, either:
     - Assert that the engine falls back to positional diff (e.g., numerous `CellEdited` as today), or
     - Assert a documented hybrid behaviour.
   - The test should clearly encode whichever fallback semantics the implementer chooses, as long as it is deterministic and consistent with the gating rules.

### 5.3 Optional unit tests for alignment helper

If a separate `row_alignment` module is introduced, add a focused test file, e.g.:

- `core/tests/row_alignment_tests.rs`

Tests:

1. `aligns_single_insert_case_with_unique_rows`
   - Build two small `Grid`s in memory with `grid_from_numbers` (10 rows × 3 cols); `grid_b` is `grid_a` with an extra row inserted in the middle. 
   - Construct `GridView`s and run the alignment helper directly.
   - Assert:
     - `RowAlignment.inserted == [5]`.
     - `RowAlignment.deleted` is empty.
     - `matched` pairs cover all other row indices with the expected mapping.

2. `detects_non_monotonic_matches_and_bails`
   - Build grids where one unique row is moved rather than inserted/deleted.
   - Assert that the helper returns `None` (or otherwise signals “no safe single‑insert/delete alignment”), so `diff_grids` will fall back to the baseline algorithm.

These tests are optional but recommended to pin down the internal algorithm contract and make future AMR expansions safer.

### 5.4 Regression / safety checks

No existing tests should be modified to expect new behaviour, except:

- If any current test implicitly relied on the noisy behaviour for “middle row insert/delete” (unlikely, because such fixtures don’t yet exist), the planner expectation is to **update** that test to match the new, cleaner behaviour.

Explicitly re-run and keep passing:

- PG5/PG6 grid diff tests.
- G1–G7 workbook-level grid tests.
- Engine, output, and JSON round‑trip tests (PG4). 

---

## 6. Summary

This cycle delivers a **minimal but end‑to‑end vertical slice** of spreadsheet‑mode row alignment:

- Uses the new GridView/HashStats layer to recognise the simple “single unique row inserted/deleted in the middle” pattern on small grids.
- Emits intuitive `RowAdded`/`RowRemoved` operations instead of noisy `CellEdited` blocks for G8 scenarios.
- Keeps behaviour unchanged elsewhere by gating on simple structural and frequency conditions.
- Anchors the implementation in concrete fixtures and tests that directly reflect the G8 milestone in the testing plan, while leaving G8a, multi‑edit, and large-grid performance for future cycles.
