```markdown
# Verification Report: 2025-12-06e-d5-composite-db-keys

## Summary

The D5 “composite primary key” work is correctly implemented at the engine level. Composite keys are handled as ordered tuples in `diff_table_by_key`, `diff_grids_database_mode` emits the expected `DiffOp`s for composite-key scenarios, and all tests promised in the mini‑spec have been added and pass. I found only one minor, documentation‑level spec deviation (error type naming) and a small test coverage gap around more exotic key column layouts; neither impacts correctness for the targeted D5 scenarios. I recommend proceeding to release, with follow‑up tests added opportunistically in a later cycle.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### 1. Mini-spec refers to `DatabaseAlignmentError`, implementation continues to use `KeyAlignmentError`

- **Severity**: Minor  
- **Category**: Spec Deviation  
- **Description**:  
  The mini‑spec lists `DatabaseAlignmentError` as an interface in `core/src/database_alignment.rs` and describes `diff_grids_database_mode` falling back on any `DatabaseAlignmentError`. In the actual code, the alignment error type is (and remains) `KeyAlignmentError` with variants `DuplicateKeyLeft` and `DuplicateKeyRight`; `diff_table_by_key` returns `Result<KeyedAlignment, KeyAlignmentError>` and `diff_grids_database_mode` matches on `Err(_)` and falls back to `diff_grids`.   
- **Evidence**:
  - Mini‑spec scope listing `DatabaseAlignmentError`. :contentReference[oaicite:1]{index=1}  
  - `KeyAlignmentError` definition and use in `diff_table_by_key`.   
  - `diff_grids_database_mode` calling `diff_table_by_key` and falling back on any error.   
- **Impact**:  
  This is a naming inconsistency between planning docs and the code, not a behavioral defect. The runtime semantics (what constitutes an error and that any alignment error triggers fallback to spreadsheet mode) match the mini‑spec and the broader Database Mode spec. The only impact is potential confusion for future readers comparing spec vs implementation.

---

### 2. Composite key behavior is correct but not exercised for non-contiguous or >2-column key lists

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The mini‑spec explicitly calls out that `KeyColumnSpec` must handle arbitrary key column lists such as `[0, 1]`, `[2, 4]`, etc., and that `diff_table_by_key` must support `key_columns.len() >= 1` with ordered tuple semantics. In practice, the new tests only exercise:
  - Single‑column keys (`[0]`) via existing D1 tests.   
  - Two‑column composite keys using contiguous columns `[0, 1]` in both the engine-level database-mode tests and the alignment-layer test.   

  There are no tests for:
  - Non‑contiguous composite key definitions (e.g., `[0, 2]` or `[2, 4]`).  
  - Composite keys of length >2 (e.g., `[0, 1, 2]`).  

  The implementation of `KeyColumnSpec` and `extract_key` is straightforward (it stores the columns as given and iterates them in order), so these cases are almost certainly correct, but they are not locked by tests even though the mini‑spec explicitly mentions arbitrary lists.   
- **Evidence**:
  - Mini‑spec constraints: `KeyColumnSpec` must handle `[0, 1]`, `[2, 4]`, etc.; `KeyValue` must be an ordered tuple.   
  - Existing tests only use `[0]` and `[0, 1]` as key column lists.   
- **Impact**:  
  No current behavior is incorrect for D5 scenarios; the implemented code clearly generalizes to arbitrary small key lists. However, future work (D6–D10) and external users relying on non‑contiguous or 3+ column keys don’t yet have explicit regression coverage. If a later refactor alters `KeyColumnSpec` or `extract_key`, correctness for those shapes could regress unnoticed.

---

### 3. Key column masking is correct but `KeyColumnSpec::is_key_column` is only indirectly tested

- **Severity**: Minor  
- **Category**: Missing Test / Gap  
- **Description**:  
  `diff_grids_database_mode` now uses `KeyColumnSpec::is_key_column(col)` to avoid emitting `CellEdited` on key columns. This matches the clarified contract that unchanged key columns must not be reported as edited.   

  The D5 engine-level test `d5_composite_key_row_added_and_cell_edited` verifies that:
  - Exactly one `CellEdited` is emitted for a changed non‑key column (col 2).  
  - There are no spurious edits on key columns.   

  However, there is no unit test that exercises `is_key_column` directly or with more varied key sets; its correctness is inferred indirectly from these scenarios.
- **Evidence**:
  - `KeyColumnSpec::is_key_column` implementation.   
  - `diff_grids_database_mode` using `is_key_column` to skip key columns when emitting `CellEdited`.   
  - D5 test asserting the edited column is the non-key column (`addr.col == 2`).   
- **Impact**:  
  For existing D5 scenarios, behavior is correct and enforced by tests. The gap is mainly that `is_key_column`’s intended semantics (arbitrary lists, non‑contiguous columns) are not directly locked in, which slightly increases the risk of accidental regressions in future refactors.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
  - `diff_grids_database_mode` updated to respect composite keys and skip key columns when diffing.   
  - `database_alignment.rs` continues to host `KeyColumnSpec`, `KeyValue`, `KeyedAlignment`, and `diff_table_by_key`, now explicitly exercised for composite keys.   
  - `core/tests/d1_database_mode_tests.rs` extended with D5 composite-key scenarios plus a composite duplicate key fallback test.   

- [x] All specified tests created  
  - D5 tests 1–3 (`d5_composite_key_equal_reordered_database_mode_empty_diff`, `d5_composite_key_row_added_and_cell_edited`, `d5_composite_key_partial_key_mismatch_yields_add_and_remove`) exist and match the mini‑spec setups and assertions.   
  - Alignment-layer test `composite_key_alignment_matches_rows_correctly` added in `database_alignment.rs` as specified.   
  - Optional composite duplicate-key fallback test is present (`d5_composite_key_duplicate_keys_fallback_to_spreadsheet_mode`).   

- [x] Behavioral contract satisfied  
  - Reorder‑only composite key case yields empty diff.   
  - New composite key + non-key edit → exactly one `RowAdded`, one `CellEdited`, no `RowRemoved`.   
  - Partial key mismatch → one `RowRemoved`, one `RowAdded`, no `CellEdited` (tuple semantics).   
  - Duplicate composite keys cause fallback to spreadsheet mode, consistent with D1 behavior.   

- [ ] No undocumented deviations from spec (documented deviations with rationale are acceptable)  
  - There is a minor, undocumented naming deviation: the spec mentions `DatabaseAlignmentError` while the code uses `KeyAlignmentError` for alignment errors. Behavior is still as specified.   

- [x] Error handling adequate  
  - Duplicate keys on either side return `KeyAlignmentError::DuplicateKeyLeft/Right` from `diff_table_by_key`.   
  - `diff_grids_database_mode` catches any alignment error and falls back to `diff_grids`, preserving the D1/D6 fallback contract.   

- [x] No obvious performance regressions  
  - Composite keys still use a single `KeyValue` per row (a small `Vec<KeyComponent>`), and `diff_table_by_key` remains an O(N) hash join over rows, consistent with the mini‑spec’s complexity constraints.   
  - No extra maps or per‑cell allocations were introduced in this cycle; changes are localized to using `KeyColumnSpec` for masking key columns and new tests.

```

Since no Critical or Moderate issues were found, I’m not generating a formal Remediation Plan, but here are **recommended follow‑ups** you might queue for a later cycle:

* Add tests covering non‑contiguous composite keys (e.g., `[0, 2]`) and 3‑column keys to lock in the “arbitrary key list” requirement.
* Optionally add a tiny unit test around `KeyColumnSpec::is_key_column` to make its semantics explicit.
* Decide whether to normalize the naming (`DatabaseAlignmentError` vs `KeyAlignmentError`) by either updating docs or adding a type alias for clarity.
