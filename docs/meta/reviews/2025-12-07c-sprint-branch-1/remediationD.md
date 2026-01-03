Here’s what I’m going to do in this answer:

1. First, say very clearly what is **actually left in Branch 1** (based on the current code + tests).
2. Then give a **step‑by‑step, very detailed implementation plan** for those remaining pieces — mostly tightening tests and, if needed, making small engine tweaks.

---

## 1. What’s actually left in Branch 1

From reading the plan and the code/tests side‑by‑side, Branch 1 breaks down like this:

### 1.1 Fix Silent Data Loss on Rectangular Block Moves

**Engine:**

* `RegionMask` is implemented and used to track active/excluded cells.
* `diff_grids_with_config` uses an iterative Phase 1 loop over `RegionMask` and then a Phase 2 positional diff on the remaining unmasked cells.
* Masked move detectors (`detect_exact_rect_block_move_masked`, `detect_exact_row_block_move_masked`, `detect_exact_column_block_move_masked`, `detect_fuzzy_row_block_move_masked`) are implemented.

So the **core behavior** (“don’t bail out after the first move; compute edits outside the moved region”) is implemented.

**Tests:**

You do have tests that cover:

* Pure rect move → single `BlockMovedRect`.
* Rect move + cell edits outside block → tests assert that cell edits are emitted.
* Rect move + row insertion/deletion outside block → tests assert row adds/removes are emitted. 

But for the Branch 1.1 deliverables:

> “rect move + cell edit outside moved region → **both** reported”
> “rect move + row insertion outside moved region → **both** reported” 

your tests currently only **assert the “outside change” part**, not “outside change AND a move op”.

Concretely:

* `g14_rect_move_plus_cell_edit_no_silent_data_loss` only checks that at least one `CellEdited` exists; it does **not** assert that a `BlockMovedRect` is present in the same report.
* `g14_rect_move_plus_row_insertion_outside_no_silent_data_loss` / `g14_rect_move_plus_row_deletion_outside_no_silent_data_loss` only assert row adds/removes, not the coexistence with a `BlockMovedRect`. 

So: **Engine behavior is there; tests don’t yet enforce the “both reported” requirement.**

---

### 1.2 Make Move Detection Iterative

**Engine:**

* The iterative loop with mask subtraction is implemented in `diff_grids_with_config`:

  * `max_move_iterations` from `DiffConfig`.
  * Each iteration looks for (in order): rect move → row move → column move → fuzzy row move; then excludes those rows/cols from the masks. 
* `DiffConfig` has `max_move_iterations` with default 10. 

So the **algorithmic part of 1.2 is implemented**.

**Tests:**

Deliverables say: 

* “two disjoint row block moves → both detected”
* “row block move + column block move → both detected”
* “three rect moves → all three detected”

What the current tests actually enforce:

* `g14_three_disjoint_rect_block_moves_detected` **does** assert 3 `BlockMovedRect` ops and total op count == 3, so it fully matches the “three rect moves → all three detected” criterion. 
* `g14_two_disjoint_row_block_moves_detected`:

  * Only asserts that **at least one** `BlockMovedRows` exists or that there are some ops at all; it does **not** verify “both moves detected” or their ranges.
* `g14_row_move_plus_column_move_both_detected`:

  * Only asserts “has_any_move || !ops.is_empty()”; it doesn’t guarantee that you get **one row move AND one column move**.

So: **engine supports multiple moves, but the tests don’t actually force the “both detected” semantics for mixed/multiple move scenarios.**

---

### 1.3 Remove Column-Index Dependency from Row Hashes

* Row hashes now use 128‑bit signatures over cell content only, with no column index mixed in (`hash_row_content_128` iterates cells and hashes value+formula only).
* `Grid::compute_all_signatures` and `GridView::from_grid` use the new row/column signature computation.
* Integration tests in `g15_column_structure_row_alignment_tests` assert that blank column insert/delete **does not create row structural noise** and that column ops are emitted when expected.

This pretty clearly satisfies 1.3.

---

### 1.4 Upgrade to 128‑bit Row Signatures

* `RowSignature` and `ColSignature` are `u128` and have hex serialization.
* `hash_row_content_128` / `hash_col_content_128` use xxHash3 128‑bit (`Xxh3`).
* Comments document collision probability and refer to ~10^-29 at 50K rows (better than the 10^-18 bar).

1.4 looks done.

---

### 1.5 Semantic Float Normalization

* `normalize_float_for_hash` implemented exactly as in the plan and used in:

  * `hashing::hash_cell_value` for numbers.
  * `CellValue::PartialEq` and `Hash` in `workbook.rs`.
* Unit tests cover:

  * 0.0 vs -0.0.
  * 1.0 vs 1.0000000000000002.
  * 1.0 vs 1.0001.
  * NaN variants.
  * Additional large‑number cases.

And you have CellValue hashing tests confirming the same semantics.

So 1.5 is also done.

---

### Net: What remains for Branch 1?

Strictly from the plan vs code:

1. **1.1 – Mixed rect move + outside changes tests are too weak**

   * Need tests that explicitly assert **both**:

     * A `BlockMovedRect` op for the moved block.
     * The external `CellEdited` or `RowAdded`/`RowRemoved` op(s).

2. **1.2 – Multi‑move tests are too weak**

   * Need tests that **precisely assert**:

     * Two disjoint row moves → two `BlockMovedRows` with the right ranges.
     * Row block move + column block move → at least one `BlockMovedRows` and one `BlockMovedColumns` (and no bogus adds/removes for those cells).

Everything else in Branch 1 looks implemented and reasonably tested.

The detailed plan below is aimed at closing exactly those gaps, plus a bit of extra safety around “no silent data loss”.

---

## 2. Detailed implementation plan for the remaining work

I’ll break this into three workstreams:

1. Strengthen 1.1 tests (rect move + outside changes).
2. Strengthen 1.2 tests (multiple moves).
3. Optional robustness checks for “no silent data loss”.

### Workstream A – Strengthen 1.1 “both reported” semantics

**Goal:** For mixed scenarios (rect move + outside edits/row inserts/deletes), assert that the diff report contains **both** the move and the external change.

#### A1. Extract small helpers to interrogate a `DiffReport`

In `core/tests/g14_move_combination_tests.rs` (or a shared test util module):

* Add pure helper functions:

  * `fn collect_rect_moves(report: &DiffReport) -> Vec<&DiffOp>`
  * `fn collect_row_adds(report: &DiffReport) -> Vec<&DiffOp>`
  * `fn collect_row_removes(report: &DiffReport) -> Vec<&DiffOp>`
  * `fn collect_cell_edits(report: &DiffReport) -> Vec<&DiffOp>`

  These just `iter()` and `filter` on `DiffOp` variants, no behavior change. This keeps the assertions in actual tests concise and explicit.

Implementation sketch (no need to literally copy, just conceptually):

* `collect_rect_moves`: filter `DiffOp::BlockMovedRect { .. }`
* `collect_row_adds`: filter `DiffOp::RowAdded { .. }`
* `collect_row_removes`: filter `DiffOp::RowRemoved { .. }`
* `collect_cell_edits`: filter `DiffOp::CellEdited { .. }`

#### A2. Strengthen “rect move + cell edit” test

Test file: `g14_move_combination_tests.rs`.

Current test:

* Only asserts “there is at least one `CellEdited`”.

New behavior:

* Assert:

  * `rect_moves.len() == 1` (or “>= 1” if you want to allow fragmentation, but spec suggests a single rect move).
  * `cell_edits.len() >= 1`.
  * Optionally: the rect move’s src/dst bounds match the scenario you constructed.

Concrete steps:

1. Replace the current `assert!(!cell_edits.is_empty(), ...)` with:

   * `assert_eq!(rect_moves.len(), 1, "expected single BlockMovedRect for the moved block");`
   * `assert!(!cell_edits.is_empty(), "expected cell edits outside the moved block");`

2. If your fixture is deterministic (it is, using `base_background` + `place_block`), also assert specific row/col counts:

   * Check `src_start_row/src_start_col/src_row_count/src_col_count`.
   * This locks the rect move detection to exactly the block you expect.

This directly enforces “both reported”.

#### A3. Strengthen “rect move + row insertion outside” test

Test: `g14_rect_move_plus_row_insertion_outside_no_silent_data_loss`. 

Current behavior:

* Asserts at least one `RowAdded`, and that there are some ops.

New behavior:

* Assert:

  * `rect_moves.len() == 1`.
  * `row_adds.len() >= 1`.
  * Optionally: row add happens at the expected index (outside the moved block).

Steps:

1. Use the same helper `collect_rect_moves` and `collect_row_adds`.
2. In the test:

   * `assert_eq!(rect_moves.len(), 1, "...");`
   * `assert!(!row_adds.is_empty(), "...");`
3. If the inserted row index is known (it is, from the fixture construction), assert it explicitly:

   * `let inserted_rows: Vec<u32> = row_adds.iter().map(|op| op.row_idx).collect();`
   * `assert!(inserted_rows.contains(&expected_row_idx));`

Repeat the same pattern for the “rect move + row deletion outside” test if you want it symmetrical.

#### A4. Add a negative guard test (optional but useful)

To make sure the engine doesn’t “degrade” by dropping the move and treating everything positionally, add:

* A test where:

  * You take a pure rect move fixture.
  * You manually **force** move detection to be disabled (e.g., by setting `config.max_move_iterations = 0` using `diff_workbooks_with_config`) and confirm that in this case you get only positional noise.
  * Then confirm that with default config you see the `BlockMovedRect` + outside edits.

This proves that:

* The presence of the move is **actually coming from the move detection**, not from some incidental positional behavior.

---

### Workstream B – Strengthen 1.2 “multiple moves” tests

**Goal:** Adjust tests so they truly enforce “both detected / all detected”, not just “something happened”.

#### B1. Two disjoint row block moves → both detected

Test: `g14_two_disjoint_row_block_moves_detected`.

Current behavior:

* Asserts that at least one `BlockMovedRows` op exists (or that there are ops), but not that **two** moves exist with the correct bounds.

Plan:

1. Use a deterministic generator:

   * Base grid: use `base_background` (rows tagged with large unique numbers).
   * Apply two non-overlapping row moves:

     * For example: move rows 4–7 to position 20, and rows 10–12 to position 35.
2. Compute diff via `diff_workbooks` or `diff_grids_with_config`.

Then assert:

* Collect row moves: `BlockMovedRows { src_start_row, row_count, dst_start_row, .. }`.
* `assert_eq!(row_moves.len(), 2);`
* Build a `BTreeSet` of triples `(src_start_row, row_count, dst_start_row)` and assert it matches the two expected tuples.

This makes the test fail if:

* Only one row move is detected.
* The moves are fragmented into more than two smaller moves.
* Moves are mis‑identified (wrong src/dst ranges).

If you want to allow harmless fragmentation (e.g., 2 expected moves but engine emits 3 because it splits one), change the assertion to “each expected move is covered by **at least one** emitted move” (checking ranges), but that’s a conscious decision.

#### B2. Row block move + column block move → both detected

Test: `g14_row_move_plus_column_move_both_detected`.

Current behavior:

* Only asserts `has_any_move || !ops.is_empty()`.

Plan:

1. Construct grid A:

   * Use a rectangular region with clearly distinguishable row and column content (similar to existing rect/row/column move fixtures).
2. From that:

   * Produce grid B by:

     * Moving a block of rows (`RowBlockMove`).
     * Moving a (different) block of columns (`ColumnBlockMove`).
   * Ensure:

     * The moved rows and moved columns do not overlap so they are unambiguous.
3. Compute diff.

Then assert:

* Collect `BlockMovedRows` and `BlockMovedColumns`.
* `assert_eq!(row_moves.len(), 1, "...");`
* `assert_eq!(col_moves.len(), 1, "...");`
* Assert the concrete ranges for each.
* Optionally verify that no `RowAdded`/`RowRemoved` or `ColumnAdded`/`ColumnRemoved` ops exist for those rows/cols.

This enforces that:

* The engine can detect **both** a row block move and a column block move on the same sheet.

#### B3. Re‑check “three rect moves → all three detected”

Test: `g14_three_disjoint_rect_block_moves_detected`. 

This one is already strong: it asserts that:

* `rect_moves.len() == 3`.
* `report.ops.len() == 3`.

Plan here is just:

* Sanity review after A/B changes to ensure this test still passes.
* If you ever relax `report.ops.len() == 3` (e.g., to allow edits inside moved blocks), make sure you still assert `rect_moves.len() == 3` and that their ranges match.

#### B4. Add a test for hitting `max_move_iterations` (optional but good)

To ensure the loop termination is correct and safe:

1. Construct a pathological grid that would produce more moves than `config.max_move_iterations`:

   * E.g. eleven small disjoint rect moves when `max_move_iterations = 10`.
2. Run `diff_workbooks_with_config` with `max_move_iterations = 10`.
3. Assert:

   * At most 10 move ops are emitted.
   * Remaining differences are still surfaced via positional diff (e.g., as edits or row/column ops), not silently dropped.

This connects the config knob to observable behavior and guards against infinite or over-long loops.

---

### Workstream C – Optional “no silent data loss” robustness

Strictly speaking, the Branch 1 acceptance says:

> “No silent data loss: every cell difference is reported.” 

The targeted tests you already have (and will strengthen in A/B) cover the important “danger zones” (moves + outside edits). If you want to go further, you can add a more systematic check.

#### C1. Golden “apply diff” check (if you want a stronger guarantee)

Add a test‑only helper:

* `fn apply_diff(base: &Workbook, ops: &[DiffOp]) -> Workbook`

that:

1. Clones the base workbook.
2. Applies `DiffOp`s in order to transform it.
3. Returns the resulting workbook.

Then for a set of scenarios:

* Random or hand‑chosen combinations of:

  * rect/row/column moves,
  * row/column insert/delete,
  * individual cell edits,
* Assert:

```rust
let report = diff_workbooks(&wb_a, &wb_b);
let patched = apply_diff(&wb_a, &report.ops);
assert_eq!(patched, wb_b);
```

If this ever fails, you’ve *proved* the presence of silent data loss (or a spurious change). This is more work, but it’s the cleanest correctness oracle.

You don’t have to implement this now to call Branch 1 “satisfied”, but it’s a natural extension if you want a strong invariant.

---

## 3. Suggested order of execution

If you want this to be efficient and low‑risk:

1. **Tighten tests first (A2, A3, B1, B2)** with no engine changes.

   * Run the suite and see what actually fails.
2. **If any new tests fail**, inspect the failing case to decide:

   * Is the algorithm incorrect?
   * Or is the scenario ambiguous and the test expectation too strict?
3. Only then:

   * Make minimal engine tweaks to satisfy the clarified semantics (e.g., if a combination scenario is falling back to positional and dropping a move).
4. Optionally implement the `max_move_iterations` stress test and/or the `apply_diff` golden check.

Once A + B are done and green, you can very reasonably mark Branch 1 as “fully satisfied” in the sense of the sprint plan: the behaviors are implemented **and** enforced by tests in exactly the combinations the plan calls out.
