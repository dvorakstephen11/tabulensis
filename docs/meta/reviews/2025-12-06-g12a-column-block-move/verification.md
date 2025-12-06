````markdown
# Verification Report: 2025-12-06b-d1-database-mode-keyed-equality

## Summary

The branch successfully adds a Database Mode entrypoint (`diff_grids_database_mode`), a keyed-alignment helper module (`database_alignment`), and D1-specific tests, and updates documentation to mark D1 as implemented. The implementation matches the D1 / UC‑17 behavioral contract: when grids contain the same unique keys and identical non-key data (even in different row orders), the database-mode diff produces an empty `DiffReport`. All tests in the suite (including new unit and integration tests) pass, and Spreadsheet Mode behavior remains unchanged. The remaining issues are minor and mostly about documentation clarity and future-test coverage rather than correctness. I recommend proceeding to release for this D1 slice.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### 1. D1 Behavioral Contract: Implemented and Verified

- **Severity**: Minor (positive confirmation; no fix required)
- **Category**: Gap (closed) / Spec Conformance
- **Description**:  
  The mini-spec requires a new Database Mode API that:
  - Operates on `Grid` values and returns a `DiffReport`. :contentReference[oaicite:0]{index=0}  
  - Uses keyed alignment (hash-join by key) where row identity is by key, row order is ignored, and unique keys are assumed for D1.   
  - Produces an empty diff when keys and non-key contents match (UC‑17 / D1).   

  The implementation provides:
  - `pub fn diff_grids_database_mode(old: &Grid, new: &Grid, key_columns: &[u32]) -> DiffReport` in `core/src/engine.rs`. :contentReference[oaicite:3]{index=3}  
  - A new `core/src/database_alignment.rs` module with `KeyColumnSpec`, `KeyedAlignment`, and `diff_table_by_key` implementing a hash-join over keys with O(N) construction and deterministic behavior (no reliance on `HashMap` iteration order for output).   
  - D1 integration tests in `core/tests/d1_database_mode_tests.rs` that load `db_equal_ordered_a.xlsx` and `db_equal_ordered_b.xlsx`, call `diff_grids_database_mode(&grid_a, &grid_b, &[0])`, and assert `report.ops.is_empty()` in both ordered and reordered cases.   

- **Evidence**:
  - Mini-spec behavioral contract and interface for `diff_grids_database_mode`. :contentReference[oaicite:6]{index=6}  
  - `database_alignment` module implementing key extraction, key representation, and alignment.   
  - `diff_grids_database_mode` integrating keyed alignment and emitting DiffOps only for non-key differences.   
  - D1 tests and overall test run (59 tests) passing.   

- **Impact**:  
  The core D1 promise—“Database Mode keyed equality produces no diffs when data is identical up to row order”—is met and codified in tests. This is the primary release gate for this branch.

---

### 2. Database Alignment Helper Returns `Result` Instead of Bare Struct (Documented Deviation)

- **Severity**: Minor  
- **Category**: Spec Deviation (benign, documented)  
- **Description**:  
  The mini-spec sketches `diff_table_by_key` as returning a `KeyedAlignment` directly. :contentReference[oaicite:10]{index=10}  
  The implementation instead defines:

  ```rust
  pub(crate) fn diff_table_by_key(
      old: &Grid,
      new: &Grid,
      key_columns: &[u32],
  ) -> Result<KeyedAlignment, KeyAlignmentError>
````

with `KeyAlignmentError::{DuplicateKeyLeft, DuplicateKeyRight}`. 

This allows the helper to enforce the unique-key invariant by erroring on duplicates, which aligns with the spec’s allowance for an “internal error used only in tests” when keys are not globally unique.

* **Evidence**:

  * Mini-spec interface description for `diff_table_by_key`. 
  * Actual signature and error enum in `database_alignment.rs`. 
  * Activity log explicitly noting that duplicate keys are rejected and handled. 

* **Impact**:
  This is a strengthening of the original plan and improves safety around duplicates. It is internal to the crate (no public API change) and consistent with the documented “error-or-unsupported” behavior for duplicate keys. No remediation needed; just be aware of the divergence between the planning sketch and the implemented signature.

---

### 3. Duplicate-Key Behavior: Helper Tested, Entrypoint Fallback Not Explicitly Tested

* **Severity**: Minor

* **Category**: Missing Test

* **Description**:
  The mini-spec says that when keys are not globally unique it is acceptable to treat such tables as “not-database-mode-safe yet,” but the engine **must not** silently misalign rows or panic. 

  Implementation strategy:

  * `diff_table_by_key` detects duplicates and returns a `KeyAlignmentError` instead of producing a potentially misaligned mapping. 
  * `diff_grids_database_mode` catches any error from `diff_table_by_key` and falls back to existing spreadsheet-mode `diff_grids`, using a synthetic `"<database>"` sheet id.
  * Module tests cover the helper’s error case (`duplicate_keys_error_or_unsupported`) but there is no test that exercises the fallback path of `diff_grids_database_mode` itself with a duplicate-key fixture.

* **Evidence**:

  * Safety / fallback constraints in the mini-spec. 
  * Activity log note: “duplicates fall back to spreadsheet-mode diff to avoid silent misalignment.” 
  * Unit test only validates the error from `diff_table_by_key`, not the entrypoint’s behavior.

* **Impact**:
  For D1 (which only exercises unique keys), this path is not part of acceptance and does not affect correctness of current fixtures. However, when a caller accidentally passes duplicate keys, the observable behavior of the public API (`diff_grids_database_mode`) depends on this fallback path. Without an explicit test, regressions (e.g., removing the fallback or changing it to a panic) would not be caught. This is a **test coverage gap**, not a functional bug, and can be addressed in a future cycle.

---

### 4. D1 Integration Tests Only Cover “Empty Diff” Paths

* **Severity**: Minor

* **Category**: Missing Test

* **Description**:
  The new `d1_database_mode_tests.rs` file contains the two planned tests:

  * `d1_equal_ordered_database_mode_empty_diff` (A vs A).
  * `d1_equal_reordered_database_mode_empty_diff` (A vs B, same keys, permuted rows).

  Both only assert that `report.ops.is_empty()`; they do not:

  * Validate that Spreadsheet Mode (`diff_workbooks`) **would** produce a non-empty diff on the reordered pair (optional contrast test from the mini-spec).
  * Exercise non-empty database-mode outputs (row additions/removals or cell edits) even in small in-memory grids (e.g., a tiny D2-style case). The engine-side mapping from `KeyedAlignment` to `DiffOp::{RowAdded, RowRemoved, CellEdited}` is currently untested at the integration level.

* **Evidence**:

  * Test plan for D1 integration tests. 
  * Actual test file contents and assertions. 
  * Code path in `diff_grids_database_mode` that emits non-empty diffs (row-only and cell-only cases).

* **Impact**:
  For the strict D1/UC‑17 slice, “empty diff when data is identical up to row order” is fully covered. However, the untested non-empty paths represent potential future risk when you move into D2–D3 (row added/removed, row updated). If those paths contain bugs, they will remain latent until a later cycle. Given D1’s explicit scope, this is acceptable today but worth codifying as follow-up work.

---

### 5. Synthetic `"<database>"` Sheet Identifier vs. Real Sheet Names

* **Severity**: Minor

* **Category**: Spec Deviation / Design Note

* **Description**:
  The Database Mode entrypoint uses a constant `DATABASE_MODE_SHEET_ID: &str = "<database>"` for all emitted `DiffOp`s, rather than propagating the actual worksheet name.

  The D1 behavioral examples in the mini-spec talk about “that sheet” and show expectations phrased in terms of the primary data sheet, but they never pin down the sheet identifier in the diff for Database Mode.

* **Evidence**:

  * `DATABASE_MODE_SHEET_ID` definition and its use in `diff_grids_database_mode` for row and cell operations.
  * D1 / UC‑17 spec text referring to behavior “for the primary data sheet,” but not specifying how Database Mode names that sheet in IR.

* **Impact**:
  For D1, all tested cases require an **empty** diff, so the sheet ID never appears in assertions and the behavior is effectively unspecified. This design is forward-compatible (you can later treat `<database>` as a virtual sheet or refine it to use real names). However, it’s a slight drift from how Spreadsheet Mode uses actual worksheet names in DiffOps. Documenting this in higher-level docs or comments would avoid surprises for future consumers of `diff_grids_database_mode`.

---

### 6. Docs/Test Plan vs. Fixture Naming Drift

* **Severity**: Minor

* **Category**: Gap (Documentation)

* **Description**:
  The testing plan describes fixtures for D1 as `db_equal_ordered_{a,b}.xlsx` and `db_equal_reordered_{a,b}.xlsx`.
  The actual tests and mini-spec examples for D1 use the pair `db_equal_ordered_a.xlsx` and `db_equal_ordered_b.xlsx`, with B understood to be the permuted version.

* **Evidence**:

  * Testing plan D1 fixture sketch. 
  * Mini-spec Example 3 and integration test using `db_equal_ordered_a` vs `db_equal_ordered_b`.

* **Impact**:
  This is a naming/documentation mismatch rather than a functional issue; the underlying fixtures and tests are consistent with each other. It may cause confusion when someone reads the testing plan in isolation and goes looking for `db_equal_reordered_*` files. A future doc pass can fix this easily.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed

  * New engine API `diff_grids_database_mode` implemented and exported. 
  * New `database_alignment` helper module created. 
  * D1 integration tests and unit tests added.

* [x] All specified tests created

  * `d1_equal_ordered_database_mode_empty_diff` and `d1_equal_reordered_database_mode_empty_diff`.
  * `unique_keys_reorder_no_changes`, `unique_keys_insert_delete_classified`, `duplicate_keys_error_or_unsupported`.

* [x] Behavioral contract satisfied

  * Unique-key Database Mode produces an empty diff when keys and non-key data match regardless of row order (UC‑17 / D1).

* [x] No undocumented deviations from spec (documented deviations with rationale are acceptable)

  * Duplicate-key fallback behavior is described in the activity log. 
  * Helper signature change (returning `Result`) is a reasonable strengthening, not a silent divergence.

* [x] Error handling adequate

  * Duplicate keys cause a controlled `KeyAlignmentError` in the helper and a safe Spreadsheet Mode fallback in the public API.

* [x] No obvious performance regressions

  * Alignment is an O(N) hash join over keys with O(N×M) cell comparison, consistent with the mini-spec and unified spec constraints.
  * All existing PG1–PG6 and M1–M6 tests remain green. 

```

If you’d like, I can also sketch a lightweight follow-up test plan for D2/D3 that specifically exercises the non-empty `diff_grids_database_mode` paths and the duplicate-key fallback.
```
