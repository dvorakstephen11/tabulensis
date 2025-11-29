# Remediation Plan: 2025-11-28b-diffop-pg4

## Overview

This remediation plan addresses the two minor but real gaps identified in the verification report:

1. **Missing JSON shape tests** for non-`CellEdited` `DiffOp` variants (the schema is stable in code and spec but not fully locked in by tests).
2. **Insufficient documentation** of the `CellEdited` address invariants and equality semantics, which currently live only in tests and PG3/PG4 docs.

The plan keeps behavior and wire format fully backward-compatible. All changes are confined to tests (for stronger schema guarantees) and doc comments (for better guidance to future implementers and external consumers).   

---

## Fixes Required

### Fix 1: Lock JSON key-sets for all DiffOp variants

- **Addresses Finding**: _Finding 4 – DiffOp JSON key-set is only strictly locked-in for `CellEdited`._
- **Changes**:

  1. **Extend PG4 JSON shape tests to cover all variants**

     Add new tests in `core/tests/pg4_diffop_tests.rs` that assert the **exact** JSON key-set for representative instances of each `DiffOp` variant, mirroring what `pg4_cell_edited_json_shape` already does for `CellEdited`.   

     Concretely:

     - **Sheet-level ops**  
       New test (name suggestions):

       ```rust
       #[test]
       fn pg4_sheet_added_and_removed_json_shape() { /* ... */ }
       ```

       Pattern:

       - Construct `DiffOp::SheetAdded { sheet: "Sheet1".into() }`.
       - Serialize with `serde_json::to_value`.
       - Assert:
         - `json["kind"] == "SheetAdded"`.
         - `json["sheet"] == "Sheet1"`.
         - Top-level key-set is exactly `{ "kind", "sheet" }`.

       Repeat for `SheetRemoved` in the same test:

       - `DiffOp::SheetRemoved { sheet: "SheetX".into() }`.
       - Assert key-set `{ "kind", "sheet" }`.

     - **Row/column ops with optional signatures**  
       You already assert presence/absence of `row_signature` / `col_signature`, but not the full key set. :contentReference[oaicite:2]{index=2}  

       Add a new test such as:

       ```rust
       #[test]
       fn pg4_row_and_column_json_shape_keysets() { /* ... */ }
       ```

       For each of:

       - `RowAdded` w/ `Some(RowSignature { hash: ... })`
       - `RowAdded` w/ `None`
       - `RowRemoved` w/ `Some` and `None`
       - `ColumnAdded` w/ `Some` and `None`
       - `ColumnRemoved` w/ `Some` and `None`

       Patterns:

       - Compute `let obj = json.as_object().unwrap();`
       - Collect keys into a `BTreeSet<String>`.
       - Assert key-sets:

         - For `RowAdded` with signature: `{ "kind", "sheet", "row_idx", "row_signature" }`
         - Without signature: `{ "kind", "sheet", "row_idx" }`
         - Similarly for `RowRemoved`, `ColumnAdded`, `ColumnRemoved`.

     - **Block move ops**  
       You already check `block_hash` omission/presence but not the full shape. :contentReference[oaicite:3]{index=3}  

       Add a test like:

       ```rust
       #[test]
       fn pg4_block_move_json_shape_keysets() { /* ... */ }
       ```

       For each of:

       - `BlockMovedRows` with `Some(block_hash)` and `None`.
       - `BlockMovedColumns` with `Some(block_hash)` and `None`.

       Assert:

       - With hash: key-set matches the spec, e.g.  
         `{ "kind", "sheet", "src_start_row", "row_count", "dst_start_row", "block_hash" }`  
         and the column equivalent.
       - Without hash: same sets minus `"block_hash"`.

  2. **Keep spec and implementation aligned**

     These tests should reflect the JSON schema as written in:

     - `core/src/diff.rs` definitions for `DiffOp`. :contentReference[oaicite:4]{index=4}  
     - PG4 mini-spec section 2.3 (JSON shape contract) and section 3 (DiffReport invariants).   

     If any discrepancies arise while writing the tests, treat the spec + existing code as the source of truth for this cycle; do **not** change field names or structures.

- **Tests**:

  - New tests (suggested names; adjust to house style):

    - `pg4_sheet_added_and_removed_json_shape`
    - `pg4_row_and_column_json_shape_keysets`
    - `pg4_block_move_json_shape_keysets`

  - All tests live in `core/tests/pg4_diffop_tests.rs` alongside existing PG4 tests so coverage is localized and discoverable. :contentReference[oaicite:6]{index=6}  

---

### Fix 2: Add DiffOp-level negative tests for invalid addresses

- **Addresses Finding**: _Finding 6 – No direct DiffOp-level tests for invalid/tampered JSON inputs._
- **Changes**:

  The PG3 tests already prove that `CellAddress` and `CellSnapshot` reject invalid A1 strings (e.g., `"1A"`, `"A0"`), and that tampering with the `addr` string changes the deserialized `CellAddress`.   

  This fix adds **PG4-local tests** that exercise the same contract through the `DiffOp` deserialization path, making the invariant explicit at the DiffOp/DiffReport layer (which is what external consumers will target).

  1. **Invalid `addr` on the DiffOp itself**

     New test in `core/tests/pg4_diffop_tests.rs`:

     ```rust
     #[test]
     fn pg4_diffop_cell_edited_rejects_invalid_top_level_addr() {
         let json = r#"{
             "kind": "CellEdited",
             "sheet": "Sheet1",
             "addr": "1A",
             "from": { "addr": "C3", "value": null, "formula": null },
             "to":   { "addr": "C3", "value": null, "formula": null }
         }"#;

         let err = serde_json::from_str::<DiffOp>(json)
             .expect_err("invalid top-level addr should fail to deserialize");
         let msg = err.to_string();
         assert!(
             msg.contains("invalid cell address") && msg.contains("1A"),
             "error should mention invalid address: {msg}",
         );
     }
     ```

     This mirrors the PG3 invalid-addr tests and proves that DiffOp consumers cannot bypass `CellAddress` validation via top-level `addr`.   

  2. **Invalid `addr` inside snapshots**

     Add a second test:

     ```rust
     #[test]
     fn pg4_diffop_cell_edited_rejects_invalid_snapshot_addrs() {
         let json = r#"{
             "kind": "CellEdited",
             "sheet": "Sheet1",
             "addr": "C3",
             "from": { "addr": "A0", "value": null, "formula": null },
             "to":   { "addr": "C3", "value": null, "formula": null }
         }"#;

         let err = serde_json::from_str::<DiffOp>(json)
             .expect_err("invalid snapshot addr should fail to deserialize");
         let msg = err.to_string();
         assert!(
             msg.contains("invalid cell address") && msg.contains("A0"),
             "error should mention invalid address: {msg}",
         );
     }
     ```

     This confirms that DiffOp deserialization composes correctly with PG3 snapshot constraints, not just the top-level `addr`.   

  3. **Optional: DiffReport-level invalid JSON**

     Optionally, add a small test that wraps an invalid CellEdited op inside a `DiffReport` JSON blob (e.g., `{"version":"1","ops":[ /* invalid op */ ]}`) and asserts that `serde_json::from_str::<DiffReport>` fails with an error that includes the underlying `invalid cell address` message. This is mainly a smoke test that the container doesn’t swallow useful diagnostics.

- **Tests**:

  - New tests (names can match snippets above):

    - `pg4_diffop_cell_edited_rejects_invalid_top_level_addr`
    - `pg4_diffop_cell_edited_rejects_invalid_snapshot_addrs`
    - (Optional) `pg4_diff_report_rejects_invalid_nested_addr`

  - These re-use the error message conventions exercised in `pg3_snapshot_tests.rs` so the behavior remains consistent across PG3 and PG4.   

---

### Fix 3: Document CellEdited invariants and equality semantics

- **Addresses Finding**:  
  - _Finding 5 – `CellEdited` invariants are not surfaced in type-level documentation._  
  - (Indirectly supports Finding 2 – we rely on test helper + spec, but future contributors don’t see it in the type definition.)  

- **Changes**:

  1. **Add doc comments on `DiffOp::CellEdited` in `core/src/diff.rs`**

     Update the `DiffOp` enum definition with targeted Rustdoc comments explaining:

     - What `CellEdited` represents (a logical cell change).  
     - The **address invariants**:

       - `addr` is the canonical cell location for the edit.
       - `from.addr` and `to.addr` must match `addr` for valid DiffOps.
       - This is a **semantic invariant enforced by producers and tests**, not by the type system or `PartialEq`.

     - The relationship to `CellSnapshot` equality:

       - `CellSnapshot` implements `PartialEq`/`Eq` purely in terms of `(value, formula)` and **ignores `addr`**, which is intentional (see PG3 tests).   
       - Therefore, `DiffOp` equality for `CellEdited` does not by itself guarantee address consistency; callers must respect the invariant when constructing ops.

     Example (conceptual, not verbatim):

     ```rust
     /// CellEdited represents a logical change to a single cell.
     ///
     /// Invariants (maintained by producers and test helpers):
     /// - `addr` is the canonical location of the edit.
     /// - `from.addr` and `to.addr` must both equal `addr`.
     /// - `CellSnapshot` equality ignores `addr` and compares only value + formula,
     ///   so `DiffOp::CellEdited` equality does not by itself enforce these address invariants.
     ```

     This ties the code directly back to the PG3/PG4 docs and makes the invariants visible to anyone reading the type.   

  2. **Mention the invariant helper in comments (tests)**

     In `core/tests/pg4_diffop_tests.rs`, add a brief comment above `assert_cell_edited_invariants` referencing the Rustdoc expectation, e.g., “This enforces the invariant documented on DiffOp::CellEdited.” This creates a low-friction path from the tests back to the documented contract.   

  3. **Optional doc cross-link in specification**

     In `excel_diff_specification.md` or the PG4 mini-spec, add a short note (if you’re updating docs as part of this cycle) explicitly connecting:

     - The `CellEdited` invariants.   
     - The fact that `CellSnapshot` equality ignores address, with justification (helps align the spec with the implementation and tests for future planners/reviewers).   

- **Tests**:

  - No new tests required for this fix; it is documentation-only. The invariants are already enforced by existing PG4 tests (including the `#[should_panic]` negative case).   

---

## Constraints

- **No wire format changes**  
  - The JSON schema for `DiffOp` and `DiffReport` is considered stable post-PG4 and is already used as a public contract for downstream consumers and future tooling. No field names, tag values (`"kind"` variants), or container structures may change in this remediation.   

- **No changes to `CellSnapshot` equality semantics**  
  - PG3 tests and the spec rely on equality ignoring `addr`. This must remain unchanged; the documentation will clarify this rather than altering behavior.   

- **Test performance and style**  
  - New PG4 tests must:
    - Be pure Rust unit tests (no Excel fixtures, no I/O).
    - Run quickly and deterministically (simple serde + assertions).
    - Use the existing style (serde_json::to_value / serde_json::from_str, `BTreeSet` for key-sets, etc.).   

- **Branch and process alignment**

  - All changes must be implemented on branch `2025-11-28b-diffop-pg4` and recorded in the activity log and cycle results as described in the meta-process guide, keeping decision/spec/log/verification artifacts aligned.   

---

## Expected Outcome

After completing this remediation:

1. **JSON schema stability is explicitly enforced**  
   - All `DiffOp` variants (not just `CellEdited` and `DiffReport`) have their JSON key-sets locked in by tests. Any accidental addition/removal/renaming of fields will break PG4 tests and be caught immediately.   

2. **DiffOp deserialization behavior for invalid addresses is documented and defended at the right level**  
   - PG4 tests explicitly confirm that invalid A1 addresses (like `"1A"` and `"A0"`) cause `serde_json::from_str::<DiffOp>` and `serde_json::from_str::<DiffReport>` to fail, matching the PG3 behavior and the spec’s expectations.   

3. **`CellEdited` invariants are visible where implementers and integrators look first**  
   - The invariants and equality caveats are clearly documented on the `DiffOp::CellEdited` variant itself, with tests and spec pointing at the same contract. Future contributors are less likely to misuse `CellSnapshot` equality or construct inconsistent `CellEdited` ops.   

4. **No behavior or wire-format regressions**  
   - All changes are additive: tests and documentation only. `cargo test` from both the `core/` crate and the workspace root should continue to pass on this branch with no observable changes to external behavior, keeping the PG4 milestone and the existing product-level commitments intact.   
