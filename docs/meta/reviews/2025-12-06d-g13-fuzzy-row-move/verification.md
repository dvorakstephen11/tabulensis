```markdown
# Verification Report: 2025-12-06d-g13-fuzzy-row-move

## Summary

The implementation adds a guarded fuzzy row-block move detector to `row_alignment.rs`, integrates it into the `diff_grids` pipeline in `engine.rs`, wires up a G13 workbook fixture and generator, and introduces both unit and integration tests. The behavior matches the mini-spec’s contract: when a contiguous row block is moved and lightly edited, the engine emits a single `BlockMovedRows` plus `CellEdited` operations inside the moved block, without spurious row add/remove ops. All tests described in the mini-spec are present and passing, and existing G1–G12 and D1 suites remain green. The only behavioral deviation from the mini-spec (Jaccard + positional smoothing instead of pure Jaccard) is explicitly documented and well‑justified. I did not find any bugs or missing functionality that should block release; only minor coverage and documentation nits remain.

## Recommendation

[X] Proceed to release  
[ ] Remediation required

## Findings

### Finding 1: Documented change to similarity metric (Jaccard + positional smoothing)

- **Severity**: Minor  
- **Category**: Spec Deviation (documented)  
- **Description**:  
  The mini-spec describes fuzzy detection as using a Jaccard-like set-based similarity over row tokens with a 0.80 threshold. :contentReference[oaicite:0]{index=0}  
  The implementation still computes Jaccard over row-level hashes, but then takes the maximum of:
  - the Jaccard similarity, and  
  - a “positional match” ratio `(matches + 1) / (len + 1)` based on row hashes at aligned positions, before applying the same 0.80 threshold. :contentReference[oaicite:1]{index=1}  

  This behavior is called out explicitly in the Activity Log’s “Intentional Spec Deviations” table. :contentReference[oaicite:2]{index=2}
- **Evidence**:
  - `FUZZY_SIMILARITY_THRESHOLD: f64 = 0.80;` and `block_similarity` in `core/src/row_alignment.rs`.   
  - Activity log deviation explanation in `cycle_summary.txt`. :contentReference[oaicite:4]{index=4}
- **Impact**:  
  This slightly broadens the acceptance region for small blocks that are mostly identical but have one edited row (e.g., the G13 case), where pure set Jaccard would drop below 0.80 because a single changed row introduces an extra hash into the union. With the smoothing, a 4-row block with one edited row hits exactly 0.80 positional similarity while still requiring ≥~80% row-level agreement, and guardrails (size bounds, repetition guard, ambiguity handling) remain intact. This improves positive-case detection without meaningfully increasing false positives, and it is explicitly documented, so no remediation is required.

---

### Finding 2: Edge-case coverage gap for moves at sheet boundaries

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The fuzzy detector’s tests cover:
  - downward moves with an internal edit,  
  - upward moves with an internal edit,  
  - low similarity rejection,  
  - heavy repetition / ambiguous-candidate bail‑out,  
  - behavior at `MAX_FUZZY_BLOCK_ROWS` (32 rows), and  
  - behavior at the `MAX_HASH_REPEAT` repetition threshold.   

  All of these use blocks safely away from row 0 and the last row (e.g., blocks at rows 4–7 moved to 12–15, or 4–35 moved to 36–67). There are no tests where the moved block starts at the first row or ends at the final row.
- **Evidence**:
  - Unit tests in `core/src/row_alignment.rs` under `#[cfg(test)]`, particularly `detects_fuzzy_row_block_move_with_single_internal_edit`, `detects_fuzzy_row_block_move_upward_with_single_internal_edit`, `fuzzy_move_at_max_block_rows_threshold`, and `fuzzy_move_at_max_hash_repeat_boundary`.   
- **Impact**:  
  The implementation appears symmetrical and should behave correctly when prefix or suffix are empty (block at the top or bottom), but indexing logic around `prefix`, `suffix_len`, `mismatch_end`, and slice boundaries is the kind of code that benefits from explicit boundary tests. A bug here would affect only cases where a moved block touches the very first or last row, which are less common. This is a coverage enhancement, not a blocker.

---

### Finding 3: Database-mode fallback behavior is not explicitly exercised with fuzzy moves

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  Database mode (`diff_grids_database_mode`) primarily uses key-based alignment and is explicitly declared out-of-scope for behavior changes in G13.   
  However, when `diff_table_by_key` returns an error (e.g., duplicate keys), it falls back to the standard spreadsheet-mode `diff_grids`, which now includes fuzzy row-block move detection.   

  Existing D1 tests verify that this fallback produces sensible spreadsheet-mode semantics (e.g., detecting row removal in the duplicate-key scenario) but do not cover a case where the fallback sees a fuzzy move pattern (a moved and lightly edited block). 
- **Evidence**:
  - `diff_grids_database_mode` implementation and its `Err(_) => diff_grids(...)` fallback. :contentReference[oaicite:10]{index=10}  
  - `core/tests/d1_database_mode_tests.rs`. 
- **Impact**:  
  In practice, this is likely desirable: when database-mode cannot establish a valid key-based alignment (e.g., duplicate keys), we *want* the fuzzy spreadsheet-mode behavior for UC-12-style edits. The lack of an explicit test means the contract (“DB fallback inherits spreadsheet fuzzy move behavior”) is implicit rather than documented. This is not a correctness bug but a potential future confusion point for maintainers.

---

### Finding 4: Older testing-plan text mentions a 10-row block; actual fixture uses 4 rows

- **Severity**: Minor  
- **Category**: Spec Deviation (documentation drift)  
- **Description**:  
  The high-level `excel_diff_testing_plan.md` describes the G13 fixture as moving a distinctive **10-row** block from top to bottom with 2 edits inside. :contentReference[oaicite:12]{index=12}  
  The mini-spec for this cycle refines this to “a contiguous 3–6 row block (e.g., rows 4–7)” in a 20–30 row sheet. :contentReference[oaicite:13]{index=13}  
  The actual Python generator `RowFuzzyMoveG13Generator` and `fixtures/manifest.yaml` configure a 4-row block (`block_rows: 4`). :contentReference[oaicite:14]{index=14}  
- **Evidence**:
  - G13 section of `excel_diff_testing_plan.md` vs. mini-spec Section 5.1 vs. `RowFuzzyMoveG13Generator` and manifest entry `g13_fuzzy_row_move`. 
- **Impact**:  
  Functionally this is fine: the observable behavior (BlockMovedRows + CellEdited operations with no RowAdded/RowRemoved on the moved block) is identical for 4-row and 10-row examples, and the mini-spec is the newer and more precise planning document. This is simply a minor documentation drift that could surprise someone reading only the older testing-plan doc.

---

## Checklist Verification

- [X] **All scope items from mini-spec addressed**  
  - Fuzzy row-block detector added to `core/src/row_alignment.rs` using `GridView`, `HashStats<RowHash>`, size guards, low-info checks, and repetition guard.   
  - `diff_grids` in `core/src/engine.rs` updated to run fuzzy detection after exact moves and before row-alignment/positional diff, and to emit both `BlockMovedRows` and `CellEdited` inside the moved block via `emit_moved_row_block_edits`.   
  - `RowFuzzyMoveG13Generator`, manifest entry `g13_fuzzy_row_move`, and the `grid_move_and_edit_{a,b}.xlsx` fixtures are present. :contentReference[oaicite:18]{index=18}  
  - Workbook-level tests `core/tests/g13_fuzzy_row_move_grid_workbook_tests.rs` added. :contentReference[oaicite:19]{index=19}  

- [X] **All specified tests created**  
  - Unit tests in `row_alignment.rs`:
    - `detects_fuzzy_row_block_move_with_single_internal_edit`  
    - `detects_fuzzy_row_block_move_upward_with_single_internal_edit`  
    - `fuzzy_move_rejects_low_similarity_block`  
    - `fuzzy_move_bails_on_heavy_repetition_or_ambiguous_candidates`  
    - `fuzzy_move_noop_when_grids_identical`  
    - plus boundary tests (`fuzzy_move_at_max_block_rows_threshold`, `fuzzy_move_at_max_hash_repeat_boundary`, ambiguous-candidate scenarios).   
  - Integration tests in `core/tests/g13_fuzzy_row_move_grid_workbook_tests.rs`:
    - Positive fuzzy move with BlockMovedRows + CellEdited and no RowAdded/RowRemoved inside the moved block.  
    - In-place edits only (no move) do not emit BlockMovedRows.  
    - Ambiguous repeated blocks do not emit BlockMovedRows and fall back to other diffs. :contentReference[oaicite:21]{index=21}  

- [X] **Behavioral contract satisfied**  
  - In the G13 workbook case (`grid_move_and_edit_{a,b}.xlsx`), the engine emits exactly one `BlockMovedRows` with the expected `src_start_row`, `row_count`, and `dst_start_row`, plus `CellEdited` operations inside the moved block, and no `RowAdded`/`RowRemoved` for rows within that block.   
  - For heavily edited blocks (low similarity), for in-place edits with no move, and for ambiguous/repetitive patterns, fuzzy detection returns `None` and `BlockMovedRows` is not emitted, matching the mini-spec’s Examples B–D.   

- [X] **No undocumented deviations from spec (documented deviations with rationale are acceptable)**  
  - The only behavioral deviation relative to the mini-spec is the addition of positional smoothing to the similarity metric, which is clearly documented in the Activity Log’s “Intentional Spec Deviations” table and does not change the 0.80 threshold or guardrails.   

- [X] **Error handling adequate**  
  - Fuzzy detection short-circuits on:
    - mismatched grid shapes,  
    - empty grids,  
    - size bounds (`MAX_ALIGN_ROWS`, `MAX_ALIGN_COLS`),  
    - low-information grids,  
    - heavy repetition via `HashStats`, and  
    - identical-row cases.   
  - In ambiguous multi-candidate scenarios, it returns `None` rather than picking an arbitrary move, as required by the mini-spec.   

- [X] **No obvious performance regressions**  
  - Fuzzy detection runs only when grids are within existing alignment bounds (≤2000 rows, ≤64 columns).   
  - It operates on a single mismatch region with block height capped at 32 rows (`MAX_FUZZY_BLOCK_ROWS`), and uses small `HashSet<RowHash>` allocations per candidate, which is well within the performance envelope described in the unified algorithm spec.   
  - Full `cargo test` (including all G1–G13 and D1 tests) passes, and no hot-path structures (e.g., database-mode alignment, column/rectangular moves) were altered.   

```
