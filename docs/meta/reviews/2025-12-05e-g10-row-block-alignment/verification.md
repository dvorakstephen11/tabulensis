# Verification Report: 2025-12-05e-g10-row-block-alignment

## Summary

The G10 “contiguous block of rows inserted/deleted” behavior is implemented as planned via a generalized row-alignment helper (`align_row_changes` + `find_block_gap_alignment`) and integrated into the grid diff engine. The new Python fixture generator, manifest entries, Rust unit tests, and workbook-level tests all exist and are wired up. Behavior on the G10 fixtures matches the mini-spec and the broader Excel diff spec: we get exactly four `RowAdded`/`RowRemoved` ops at the expected row indices and no spurious `CellEdited` noise. Existing G1–G9 and grid_view/hashstats tests continue to pass unchanged. I did not find any correctness or spec-violating issues; the only gaps are minor test coverage improvements and maintainability notes.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### 1. Block alignment edge-case tests are limited to “middle” insertion/deletion

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The new block alignment logic correctly supports block insert/delete at the head, tail, or middle of the sheet, and for empty/non-empty grids, but the unit tests only cover a block inserted and deleted in the middle of a 10-row sheet, plus one “two blocks => bail out” negative case. There are no tests that directly exercise:
  - Block insert/delete at the **beginning** or **end** of the sheet.
  - Block insert/delete when the entire sheet is new or removed (e.g., A empty, B has 4 rows; A has 4 rows, B empty).
  - Block gaps larger than `MAX_BLOCK_GAP` (32) to assert that alignment declines and we fall back to positional diff.
  - Block alignment declining on low‑information dominated or heavily repetitive content.

  The implementation of `find_block_gap_alignment` and the gating in `align_rows_internal` clearly support these cases, but they are not explicitly asserted in tests.   

- **Evidence**:  
  - `aligns_contiguous_block_insert_middle`, `aligns_contiguous_block_delete_middle`, and `block_alignment_bails_on_noncontiguous_changes` are present and validate only a 4-row block in the middle and a simple multi-block negative case.   
  - `align_rows_internal`/`find_block_gap_alignment` are written to handle head/tail blocks, arbitrary small gaps up to `MAX_BLOCK_GAP`, and empty grids, but there are no tests targeting those paths.   

- **Impact**:  
  Today’s G10 fixtures (10 rows with a 4-row middle block) are fully covered and correct, so this is not a release blocker. However, without explicit tests, future refactors of the block alignment code could silently break edge cases (e.g., head/tail inserts, very small sheets, or larger blocks) without being caught by CI. Strengthening tests now would reduce regression risk as the alignment engine evolves toward the larger, more complex use cases described in the unified algorithm spec.   

---

### 2. No workbook-level regression test for “block scenario but alignment should bail out”

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  At the **helper** level, there is a negative test that verifies `align_row_changes` returns `None` when there are two disjoint inserted blocks. :contentReference[oaicite:4]{index=4}  
  However, there is no **end-to-end workbook test** that covers a “block‑looking” scenario where alignment is expected to bail out (e.g., two blocks or a block + an additional edit), and then asserts that `diff_workbooks` falls back to positional diff and produces cell‑edit noise instead of block‑aligned row ops.

  G8’s column and row alignment tests do cover this pattern for **single insert** alignment (e.g., “insert with extra edits” cases), but not for the **multi-row block** logic added in this cycle.   

- **Evidence**:  
  - Row-alignment negative test: `block_alignment_bails_on_noncontiguous_changes` in `row_alignment.rs`. :contentReference[oaicite:6]{index=6}  
  - Column-alignment workbook test `g9_alignment_bails_out_when_additional_edits_present` asserts fallback behavior for column alignment but there is no analogous G10 workbook fixture where a multi-row block insert/delete is combined with another change that should force positional diff. :contentReference[oaicite:7]{index=7}  

- **Impact**:  
  Current G10 behavior for the clean block insert/delete fixtures is correct, and helper-level negative tests exist, so there is no immediate correctness risk. The missing workbook‑level negative case is more about **future safety**: it would guarantee that the engine’s sheet-level behavior (not just the pure helper) continues to honor the “bail out and fall back” contract when block alignment’s preconditions are not met.

---

### 3. Row/column alignment share near-duplicate gating logic

- **Severity**: Minor  
- **Category**: Gap (Maintainability)  
- **Description**:  
  The row and column alignment modules both define very similar concepts and helpers: size gates, repetition thresholds, and uniqueness checks (`MAX_ALIGN_ROWS`, `MAX_ALIGN_COLS`, `MAX_HASH_REPEAT`, `is_within_size_bounds`, `has_heavy_repetition`, `is_unique_to_a/b`). The G10 work added `MAX_BLOCK_GAP` and new block logic only on the row side but otherwise followed the same pattern.   

  This duplication is pre‑existing and not introduced by G10, but G10 extends the row side further. There is a small risk that future changes to thresholds or gating behavior will be applied to one side (rows) but not the other (columns), causing subtle divergence that tests might not fully cover.

- **Evidence**:  
  - `core/src/row_alignment.rs` and `core/src/column_alignment.rs` both define local constants and helpers for alignment gating instead of sharing them.   

- **Impact**:  
  No current correctness or performance impact. This is a maintainability concern: as the unified grid diff algorithm evolves, it will be harder to keep row/column alignment in sync. Refactoring shared gating logic into a common module or clearly documenting intentional differences would reduce this risk.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - Generalized row alignment helper: `align_row_changes` and the new `find_block_gap_alignment` implement contiguous multi-row insert/delete with strict monotonic alignment and uniqueness checks.   
  - Engine integration: `diff_grids` now calls `align_row_changes` first, then column alignment, then positional diff, as planned.   
  - New G10 fixtures added to `fixtures/manifest.yaml` and implemented via the `RowAlignmentG10Generator` (insert/delete modes and settings). :contentReference[oaicite:12]{index=12}  
  - Workbook tests `g10_row_block_insert_middle_emits_four_rowadded_and_no_noise` and `g10_row_block_delete_middle_emits_four_rowremoved_and_no_noise` exist and are wired into the core tests.   

- [x] All specified tests created  
  - Row-alignment unit tests for contiguous block insert/delete and noncontiguous bailing were implemented as outlined in the mini-spec.   
  - G10 workbook tests match the fixture sketches and assertion patterns from the testing plan.   

- [x] Behavioral contract satisfied  
  - For G10 insert: exactly four `RowAdded` ops at row indices `[3, 4, 5, 6]`, no `RowRemoved`, no `CellEdited`, and `row_signature.is_none()` for all row ops. :contentReference[oaicite:16]{index=16}  
  - For G10 delete: exactly four `RowRemoved` ops at row indices `[3, 4, 5, 6]`, no `RowAdded`, no `CellEdited`, and `row_signature.is_none()`.   
  - Existing G1–G9 tests, grid_view/hashstats tests, and PG5 tests all continue to pass, indicating no regression of earlier behavior.   

- [x] No undocumented deviations from spec (documented deviations with rationale are acceptable)  
  - There is no “Intentional Spec Deviations” section in `cycle_summary.txt`, and the implementation closely follows the mini-spec: small block size (≤32 rows), unique block rows, strict monotonic matched pairs, and unchanged public API / output schema.   

- [x] Error handling adequate  
  - Alignment helpers fail fast on out-of-bounds sizes, column mismatch, zero row diff, heavy repetition, and low-info-dominated grids, returning `None` and letting the engine fall back to positional diff.   
  - Workbook-level tests (G8, G9) already validate that when alignment declines, positional diff kicks in and emits appropriate `CellEdited` ops; G10 inherits that engine behavior.   

- [x] No obvious performance regressions  
  - Block alignment is linear in the number of rows in the candidate grids and capped by `MAX_ALIGN_ROWS` and `MAX_BLOCK_GAP`. It reuses `GridView` and `HashStats` without widening gates, as required by the constraints.   
  - There is no additional O(R×C) or quadratic behavior introduced. For cases where alignment declines, the behavior is identical to the previous positional diff path.

