```markdown
# Verification Report: 2025-11-28b-diffop-pg4

## Summary

The PG4 cycle cleanly implements the planned `DiffOp` and `DiffReport` types, wires them into the public API, and adds the specified type-level and JSON-level tests. The JSON schema and invariants match the mini-spec and the main Excel diff specification. All prior remediation items for this branch (column JSON tests, `CellEdited` invariant helper, and workspace manifest) are present and validated. I did not find any critical or moderate issues; the only observations are minor hardening opportunities around schema tests and documentation of invariants. Overall, the branch is ready to ship as the PG4 “DiffOp wire format” milestone.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### Finding 1: PG4 scope fully implemented and aligned with spec

- **Severity**: Minor (positive confirmation; no action required)
- **Category**: Spec Alignment
- **Description**:  
  The new `diff` module defines `SheetId`, `RowSignature`, `ColSignature`, `DiffOp`, and `DiffReport` exactly as described in the PG4 mini-spec and Excel diff specification:

  * `DiffOp` is a `#[serde(tag = "kind")]` enum with variants:
    * `SheetAdded/SheetRemoved { sheet: SheetId }`
    * `RowAdded/RowRemoved { sheet, row_idx, row_signature: Option<RowSignature> }`
    * `ColumnAdded/ColumnRemoved { sheet, col_idx, col_signature: Option<ColSignature> }`
    * `BlockMovedRows/BlockMovedColumns { sheet, src_start_*, *_count, dst_start_*, block_hash: Option<u64> }`
    * `CellEdited { sheet, addr: CellAddress, from: CellSnapshot, to: CellSnapshot }`   
  * Optional signatures and `block_hash` fields use `#[serde(skip_serializing_if = "Option::is_none")]` as required by the JSON-schema description.   
  * `DiffReport` has `version: String`, `ops: Vec<DiffOp>`, a `SCHEMA_VERSION` constant of `"1"`, and `DiffReport::new` sets `version` accordingly.   
  * `core/src/lib.rs` exposes these via `pub mod diff;` and `pub use diff::{ColSignature, DiffOp, DiffReport, RowSignature, SheetId};` as specified. :contentReference[oaicite:3]{index=3}  

- **Evidence**: `core/src/diff.rs`, `core/src/lib.rs`, PG4 spec section 2–3. 
- **Impact**: Confirms that downstream consumers can rely on the planned PG4 wire types and JSON schema. No change requested; this is the intended outcome of the cycle.

---

### Finding 2: All planned PG4 tests (plus remediation tests) are present and meaningful

- **Severity**: Minor (positive confirmation; no action required)
- **Category**: Test Coverage
- **Description**:  
  The mini-spec’s entire PG4 test plan is implemented in `core/tests/pg4_diffop_tests.rs`, and the prior remediation plan’s additional tests are also present:

  * **Construction tests (PG4.1)**:
    * `pg4_construct_cell_edited_diffop` builds a `CellEdited` op for `Sheet1!C3`, then uses the invariant helper to assert `sheet == "Sheet1"` and `addr`, `from.addr`, `to.addr` all equal `C3`, and that `from.value != to.value`.   
    * `pg4_construct_row_and_column_diffops` covers `RowAdded/RowRemoved/ColumnAdded/ColumnRemoved` with both `Some` and `None` signatures, asserting required fields and that optional signatures behave as expected (plus inequality between the with/without forms).   
    * `pg4_construct_block_move_diffops` constructs `BlockMovedRows`/`BlockMovedColumns` with and without hashes and asserts numeric fields and optional hash behavior.   
  * **JSON shape tests (PG4.2 + remediation)**:
    * `pg4_cell_edited_json_shape` asserts `kind`, `sheet`, `addr`, nested snapshot addresses, and that the top-level key set is exactly `{ "addr","from","kind","sheet","to" }`.   
    * `pg4_row_added_json_optional_signature` verifies omission of `row_signature` when `None` and presence of `row_signature.hash` when `Some`.   
    * `pg4_block_moved_rows_json_optional_hash` asserts omission/presence of `block_hash` for `BlockMovedRows`.   
    * **Remediation adds**: `pg4_column_added_json_optional_signature` and `pg4_block_moved_columns_json_optional_hash` mirror the row-side tests for column signatures and column block hashes.   
  * **Round-trip tests (PG4.3)**:
    * `pg4_diffop_roundtrip_each_variant` serializes/deserializes a representative instance of every `DiffOp` variant (including both `Some`/`None` flavors where applicable) and asserts equality plus `CellEdited` invariants via the helper.   
    * `pg4_cell_edited_roundtrip_preserves_snapshot_addrs` ensures that after JSON round-trip, `CellEdited` snapshots still carry address `C3` and satisfy the invariants helper, guarding against the “address ignored in equality” subtlety.   
  * **DiffReport tests (PG4.4)**:
    * `pg4_diff_report_roundtrip_preserves_order` checks `DiffReport::new` sets version to `SCHEMA_VERSION`, that JSON round-trip preserves `version == "1"` and the exact op sequence, and that the ordered `kind` sequence is as expected.   
    * `pg4_diff_report_json_shape` asserts that a serialized report’s top-level keys are exactly `{"version","ops"}`, `version == "1"`, and `ops[*]["kind"]` matches the constructed variants.   
  * **Invariants helper + negative test (remediation)**:
    * `assert_cell_edited_invariants` centralizes the `CellEdited` address invariants and is used across all relevant tests.   
    * `pg4_cell_edited_invariant_helper_rejects_mismatched_snapshot_addr` is a `#[should_panic]` test that deliberately constructs a `CellEdited` with a mismatched `from.addr` and confirms the helper fails, proving it is sensitive to the invariant.   

  All PG4 tests run and pass under `cargo test`, as shown in `cycle_summary.txt`. :contentReference[oaicite:18]{index=18}

- **Evidence**: `core/tests/pg4_diffop_tests.rs`, PG4 spec section 6, cycle summary test output. 
- **Impact**: This confirms the behavioral contract for PG4 is enforced at the level of type construction, JSON shape, and JSON round-trip; no additional coverage is strictly required for this milestone.

---

### Finding 3: Prior remediation items are fully addressed

- **Severity**: Minor (positive confirmation; no action required)
- **Category**: Spec / Process Alignment
- **Description**:  
  The earlier remediation plan for this branch called out three improvements:

  1. **Column-side JSON shape tests for optional fields**  
     * Implemented as `pg4_column_added_json_optional_signature` and `pg4_block_moved_columns_json_optional_hash`, mirroring row-based tests and validating omission/presence of `col_signature` and `block_hash` keys.   

  2. **`CellEdited` invariant helper and negative test**  
     * The `assert_cell_edited_invariants` helper is present and used in all `CellEdited` tests, and `pg4_cell_edited_invariant_helper_rejects_mismatched_snapshot_addr` confirms that mismatched snapshot addresses are rejected.   

  3. **Workspace manifest so `cargo test` works from repo root**  
     * A root `Cargo.toml` declares a workspace with `core` as the sole member, allowing `cargo test` to run from the repository root without changing the `core` crate definition.   

- **Evidence**: `combined_remediations.md`, `core/tests/pg4_diffop_tests.rs`, root `Cargo.toml`, cycle summary. 
- **Impact**: Process-wise, this closes the loop on the earlier review feedback. No further remediation is needed for those items.

---

### Finding 4: DiffOp JSON key-set is only strictly locked-in for `CellEdited`

- **Severity**: Minor
- **Category**: Missing Test
- **Description**:  
  For `DiffOp::CellEdited`, there is a precise key-set assertion verifying that the top-level JSON object contains exactly `"kind","sheet","addr","from","to"`. Other variants (e.g., `RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`, `BlockMovedRows`, `BlockMovedColumns`, `SheetAdded`, `SheetRemoved`) are tested for presence/absence of specific fields (`row_idx`, `col_idx`, optional signatures, `block_hash`) but do not assert a complete key-set; they would not catch accidental introduction of extraneous keys on those variants.

  * `pg4_cell_edited_json_shape` explicitly checks the full key-set for `CellEdited`.   
  * `pg4_row_added_json_optional_signature`, `pg4_column_added_json_optional_signature`, and the block-move JSON tests verify specific fields and omission rules but not that no extra fields exist.   

  The PG4 spec states that DiffOp JSON schema (field names, tag name `"kind"`, and container structure) is treated as stable once PG4 is complete. 

- **Evidence**: `core/tests/pg4_diffop_tests.rs`, PG4 spec JSON-schema discussion. 
- **Impact**:  
  This is not a current bug—the implementation matches the spec and all tests pass—but future changes that accidentally add unexpected fields to, say, `RowAdded` could slip through existing tests. Given that extra fields are often backwards-compatible at the JSON level, this is low risk but slightly weakens our guardrails around schema stability.

---

### Finding 5: `CellEdited` invariants are not surfaced in type-level documentation

- **Severity**: Minor
- **Category**: Gap
- **Description**:  
  The invariant that `CellEdited.addr` must match `from.addr` and `to.addr` is enforced in tests via `assert_cell_edited_invariants` and called out in the PG4 spec.   
  However, the `DiffOp::CellEdited` variant in `core/src/diff.rs` has no doc comments explaining this semantic invariant or clarifying that equality for `CellEdited` uses snapshot equality, which ignores snapshot `addr`.   

  The earlier remediation plan suggested (optionally) documenting this near the `CellEdited` variant so that future authors and external consumers do not assume equality implies address alignment. :contentReference[oaicite:30]{index=30}  

- **Evidence**: `core/src/diff.rs`, PG4 spec section 4.3, PG3 snapshot tests. 
- **Impact**:  
  This is a usability/documentation gap rather than a correctness defect. Today, all internal constructions of `CellEdited` obey the invariant and the tests will catch regressions. But without type-level documentation, future contributors (or external code constructing DiffOps manually) may rely on `PartialEq` alone and accidentally produce inconsistent `CellEdited` values that won’t be obviously flagged without tests.

---

### Finding 6: No direct DiffOp-level tests for invalid/tampered JSON inputs

- **Severity**: Minor
- **Category**: Missing Test
- **Description**:  
  PG3 snapshot tests comprehensively cover serialization/deserialization behavior of `CellSnapshot`, including:

  * Round-tripping snapshots through JSON.
  * Detecting tampered `addr` fields.
  * Rejecting invalid A1-style addresses like `"1A"` and `"A0"`. :contentReference[oaicite:32]{index=32}  

  PG4 tests exercise only valid JSON for `DiffOp` and `DiffReport`; they do not attempt to deserialize malformed or tampered DiffOp JSON (e.g., invalid `addr` string inside a `CellEdited` or structurally invalid objects). In practice, these inputs would be handled by the same `CellAddress` and `CellSnapshot` deserializers, so behavior should be correct today.   

- **Evidence**: `core/tests/pg3_snapshot_tests.rs`, `core/tests/pg4_diffop_tests.rs`. 
- **Impact**:  
  Very low risk for current code, since DiffOp deserialization composes the already-tested snapshot/CellAddress deserializers. However, if future changes introduce custom `Deserialize` implementations for `DiffOp` or `DiffReport`, or if external JSON is fed directly into these types, having at least one negative test at the DiffOp level would more explicitly lock in the “invalid addresses are rejected” contract in the PG4 context.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed  
- [x] All specified tests created  
- [x] Behavioral contract satisfied  
- [x] No undocumented deviations from spec  
- [x] Error handling adequate (no panics or unchecked invariants in library code for this cycle)  
- [x] No obvious performance regressions (only lightweight enum/struct definitions and unit tests added)
```
