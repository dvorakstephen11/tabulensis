```markdown
# Verification Report: 2025-12-06c-g12-rect-block-move

## Summary

The implementation of rectangular block moves (`BlockMovedRect`) matches the mini‑spec and the broader Excel diff specification: the new DiffOp variant is fully wired into the IR and JSON surface, spreadsheet‑mode rectangular move detection is implemented conservatively, and the planned PG4 + G12 tests and fixtures are present and passing. The detector is carefully gated (size, low‑info, repetition) and designed to be deterministic and fail‑safe. I did not find any correctness bugs or undocumented behavioral deviations that would block release. The remaining issues are minor and mostly about test depth and maintainability.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### 1. No direct unit tests for `detect_exact_rect_block_move`

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The core rectangular move logic lives in `core/src/rect_block_move.rs` as `detect_exact_rect_block_move`. The mini‑spec suggests (but does not require) module‑level tests for this helper. In the current codebase, the only coverage of this function is via higher‑level workbook tests in `core/tests/g12_rect_block_move_grid_workbook_tests.rs` and the PG4 DiffOp/JSON tests. There is no small, direct unit test that exercises the helper with synthetic grids.   
- **Evidence**:  
  - `detect_exact_rect_block_move` and its helpers are defined in `core/src/rect_block_move.rs` with no `mod tests` in that file.   
  - The mini‑spec explicitly calls out “Engine‑level detection tests (optional but recommended)” for this helper. :contentReference[oaicite:2]{index=2}  
- **Impact**:  
  - Current G12 tests do validate the primary behaviors (simple success case, ambiguous swap, move+edit fallback) at the workbook level, but they don’t directly pin the lower‑level invariants of the detector:  
    - grid size gating (`rows <= 2000`, `cols <= 128`),  
    - low‑info / blank‑dominated bail‑outs,  
    - heavy repetition guard via `HashStats`,  
    - strict mismatch counting and overlap handling.   
  - Future changes to `rect_block_move.rs` could accidentally relax or break these invariants without immediately failing a small, focused unit test. The risk is modest, but this is a high‑complexity area of the engine, so tight unit tests would be valuable insurance.

---

### 2. Gating and repetition logic duplicated instead of shared

- **Severity**: Minor  
- **Category**: Gap (maintainability / architectural)  
- **Description**:  
  The rectangular block move detector re‑implements size and heuristic gating logic that is conceptually the same as what row/column move detection already does, instead of sharing helpers. In `core/src/rect_block_move.rs` we have:  
  - `const MAX_RECT_ROWS: u32 = 2_000;`  
  - `const MAX_RECT_COLS: u32 = 128;`  
  - `const MAX_HASH_REPEAT: u32 = 8;`  
  - plus `is_within_size_bounds`, `low_info_dominated`, `blank_dominated`, and `has_heavy_repetition` using `GridView` and `HashStats`.   

  Very similar “low‑info dominated”, “blank dominated”, and repetition checks already exist for row and column moves in `row_alignment.rs` / `column_alignment.rs`.   
- **Evidence**:  
  - Rectangular detector defines its own constants and helpers instead of calling shared functions. :contentReference[oaicite:6]{index=6}  
  - The mini‑spec explicitly calls for reusing `GridView`, `RowMeta`, `ColMeta`, and `HashStats` and mirroring the existing guards.   
- **Impact**:  
  - Today, the behavior is correct and matches the spec: the detector is gated to `rows <= 2000`, `cols <= 128`, uses low‑info, blank, and repetition guards, and never runs on very large or pathological grids.   
  - But the duplication introduces a maintenance hazard: if thresholds or heuristics are tuned for row/column moves in the future, it’s easy to forget to update the rectangular path, leaving detectors subtly out of sync. That could cause inconsistent behavior (e.g., rectangles still trying to run on sheets where row moves are now gated off).  
  - This is not a release blocker, but factoring the gating into shared helpers would reduce drift risk.

---

### 3. No explicit regression test that `diff_report_to_cell_diffs` ignores `BlockMovedRect`

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The JSON helper `diff_report_to_cell_diffs` intentionally projects only `CellEdited` operations into the CLI/fixtures cell‑diff JSON, ignoring structural ops like `BlockMovedRows`, `BlockMovedColumns`, and now `BlockMovedRect`. The current tests verify that non‑cell ops (sheets, rows, cols) are ignored, but they do not explicitly include a `BlockMovedRect` in the input `DiffReport`.   
- **Evidence**:  
  - `diff_report_to_cell_diffs` uses `if let DiffOp::CellEdited { .. }` pattern matching, so it will naturally ignore any new variants, including `BlockMovedRect`. :contentReference[oaicite:10]{index=10}  
  - The existing test `diff_report_to_cell_diffs_filters_non_cell_ops` only covers `SheetAdded`, `RowAdded`, `SheetRemoved`, etc.; no `BlockMovedRect` case is present. :contentReference[oaicite:11]{index=11}  
  - The mini‑spec explicitly suggests adding a regression test to ensure the new structural op is ignored and doesn’t cause panics.   
- **Impact**:  
  - Behavior is currently correct by construction (the `if let` is non‑exhaustive and only matches `CellEdited`), so there is no functional bug.  
  - However, without a test that includes `BlockMovedRect` in the input, a future refactor of `diff_report_to_cell_diffs` (e.g., changing pattern matching style) could accidentally start mishandling structural ops without immediately failing a dedicated regression.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - `DiffOp::BlockMovedRect` added with the planned fields and `block_hash: Option<u64>`.   
  - `diff_grids` extended with a fast path that calls `rect_block_move::detect_exact_rect_block_move` and emits a single `BlockMovedRect` before any row/column or cell diff work.   
  - `RectBlockMoveG12Generator` added, and `g12_rect_block_move` registered in `fixtures/manifest.yaml`.   
  - `g12_rect_block_move_grid_workbook_tests.rs` created with the planned scenarios (simple move, ambiguous swap, move+edit fallback).   

- [x] All specified tests created  
  - PG4 tests: `pg4_construct_block_rect_diffops` and `pg4_block_rect_json_shape_and_roundtrip` are present and exercise both construction and JSON round‑trip of the new variant.   
  - G12 tests: `g12_rect_block_move_emits_single_blockmovedrect`, `g12_rect_block_move_ambiguous_swap_does_not_emit_blockmovedrect`, and `g12_rect_block_move_with_internal_edit_falls_back` match the mini‑spec’s Example A/B/C expectations.   

- [x] Behavioral contract satisfied  
  - Only exact 2D rectangular moves (single source & destination rectangles, identical internal content, no other differences) produce `BlockMovedRect`. This is enforced by `collect_differences`, strict mismatch counting, non‑overlap checks, and `rectangles_correspond`.   
  - Pure row moves and pure column moves still go through the existing `BlockMovedRows` / `BlockMovedColumns` paths; rectangular detection requires exactly two disjoint row ranges and two disjoint column ranges, so 1D moves do not match.   
  - Ambiguous patterns are rejected by design, as seen in the “ambiguous swap” test that asserts no `BlockMovedRect` is emitted while some diffs still appear.   

- [x] No undocumented deviations from spec (documented deviations with rationale are acceptable)  
  - The implementation aligns with the mini‑spec; the activity log does not list any intentional spec deviations, and I did not find behavioral differences that contradict the documented contract. The only difference from the prose examples is cosmetic (Example B’s narrative vs the concrete 2×2 blocks used in the test), but the intended behavior (“ambiguous -> no BlockMovedRect”) is obeyed.   

- [x] Error handling adequate  
  - The rectangular detector uses a pure `Option<RectBlockMove>` and bails out safely (returning `None`) on any failure to prove an exact, unambiguous rectangle: size violation, low‑info/blank domination, heavy repetition, mismatch count mismatch, overlapping ranges, or conflicting orientations.   
  - When the detector returns `None`, `diff_grids` falls back to the existing row/column alignment and cell diff logic, so there are no new panics or partial results paths.   

- [x] No obvious performance regressions  
  - Rectangular detection is strictly gated to grids with `rows <= 2000` and `cols <= 128` and uses `GridView`/`HashStats` with small auxiliary vectors only; there are no `O(R*C)` allocations beyond the already‑accepted patterns in alignment code.   
  - For larger or highly repetitive / low‑info grids, the detector bails early and the runtime behavior is identical to the pre‑existing engine. Existing G11/G12a tests still pass unmodified.   
```

Since no Critical or Moderate findings were identified, a separate remediation plan is not required for this branch.
