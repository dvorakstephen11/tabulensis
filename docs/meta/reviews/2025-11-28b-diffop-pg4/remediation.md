# Remediation Plan: 2025-11-28b-diffop-pg4

## Overview

The PG4 implementation is correct and can ship as-is, but there are a few low-risk, non-blocking improvements worth addressing in a follow-up cycle:

1. Strengthen JSON shape tests for column-related optional fields to fully mirror the spec’s symmetry.
2. Make the `CellEdited` address invariants more robust against misuse by clarifying or encoding them beyond the current spot-check tests.
3. Align the repository layout with the expectations of automated test runners by enabling `cargo test` from the repo root.

These items improve test coverage, reduce future footguns, and smooth integration with CI and future agents without changing the current external behavior.

## Fixes Required

### Fix 1: Add JSON shape tests for column-side optional fields

- **Addresses Finding**: Finding 2 – Missing symmetrical JSON shape tests for column variants
- **Changes**:
  - File to modify: `core/tests/pg4_diffop_tests.rs`.:contentReference[oaicite:40]{index=40}  
  - Add tests mirroring the existing row-based JSON tests:
    1. A test for `ColumnAdded` / `ColumnRemoved` similar to `pg4_row_added_json_optional_signature`, e.g.:
       - Construct `ColumnAdded` with `col_signature: None`, serialize to `serde_json::Value`, and assert:
         - `kind == "ColumnAdded"`, `sheet` and `col_idx` present.
         - `col_signature` key is *absent* from the object when `None`.
       - Construct a second `ColumnAdded` with `Some(ColSignature { hash: 123 })` and assert `json["col_signature"]["hash"] == 123`.
       - Repeat as appropriate for `ColumnRemoved` or cover both in a single test.
    2. A test for `BlockMovedColumns` similar to `pg4_block_moved_rows_json_optional_hash`:
       - Variant with `block_hash: None` and one with `Some(...)`.
       - Assert that `block_hash` is omitted when `None` and present with the expected value when `Some`.
  - Keep the style and helpers consistent with existing PG4 JSON tests (use `serde_json::to_value`, `Value` indexing, and key-set assertions where helpful).
- **Tests**:
  - New tests (names are suggestions; adjust as desired for house style):
    - `pg4_column_added_json_optional_signature`
    - `pg4_block_moved_columns_json_optional_hash`
  - Ensure they run alongside existing PG4 tests and pass under `cargo test` from `core/`.

### Fix 2: Make `CellEdited` address invariants harder to misuse

- **Addresses Finding**: Finding 3 – `DiffOp` equality for `CellEdited` does not encode address invariants
- **Changes**:
  - Keep `CellSnapshot` equality semantics as-is (value + formula only), since they are relied upon elsewhere.:contentReference[oaicite:41]{index=41}  
  - Introduce an explicit invariant-check helper for `CellEdited` DiffOps and use it in tests:
    - Location: `core/tests/pg4_diffop_tests.rs` (test-only helper) or a small internal helper in `core/src/diff.rs`.
    - Behavior (conceptually):
      - Pattern match on `DiffOp::CellEdited { addr, from, to, .. }`.
      - Assert `from.addr == *addr` and `to.addr == *addr`.
    - Use this helper in all tests that construct `CellEdited` ops, including the round-trip tests, instead of duplicating field assertions.
  - Optionally, add a short doc comment near the `CellEdited` variant in `core/src/diff.rs` emphasizing that the invariant is *semantic* and not enforced by equality:
    - Explain that `DiffOp` equality for `CellEdited` uses snapshot equality (ignoring snapshot addresses), and callers who care about the invariant should use the helper or manually check addresses.
- **Tests**:
  - Refactor existing tests (`pg4_construct_cell_edited_diffop`, `pg4_cell_edited_json_shape`, `pg4_diffop_roundtrip_each_variant`, `pg4_cell_edited_roundtrip_preserves_snapshot_addrs`) to call the new invariant helper, so any future regression is caught in one place.:contentReference[oaicite:42]{index=42}  
  - Optionally add a small negative test that constructs a deliberately invalid `CellEdited` (mismatched snapshot address) and asserts that the invariant helper fails (e.g., via `should_panic` or by returning a `Result`).

### Fix 3: Enable `cargo test` from repository root

- **Addresses Finding**: Finding 4 – Root-level `cargo test` fails due to missing workspace manifest
- **Changes**:
  - Add a minimal workspace `Cargo.toml` at the repo root that includes `core` as a member, so `cargo test` from the root runs the `core` crate’s tests:
    - Example outline (conceptual, not exact code):
      - `[workspace]`
      - `members = ["core"]`
  - Ensure this does not change the `core/Cargo.toml` package definition or features.:contentReference[oaicite:43]{index=43}  
  - If the evaluation harness expects any specific workspace configuration, mirror that here (e.g., no extra members).
- **Tests**:
  - Once added, run `cargo test` from the repo root to verify:
    - The workspace is discovered.
    - All existing tests (including PG4) run and pass.
  - Update any developer documentation or internal scripts that currently assume `core/` as the starting directory, if necessary.

## Constraints

- Avoid semantic changes to the DiffOp / DiffReport wire contract:
  - No renaming of fields or variants.
  - No changes to the JSON representation beyond adding tests and comments.
- Keep `CellSnapshot` equality semantics unchanged; PG3 tests and the spec rely on this behavior.  
- The new tests should be fast and deterministic, matching the existing unit test style (no external I/O, no Excel fixtures for PG4).  
- The root workspace manifest should be minimal and must not introduce new crates or dependencies beyond the existing `core` crate.

## Expected Outcome

After remediation:

1. JSON schema stability for column-related DiffOps will be protected by explicit tests mirroring the row-side coverage, reducing the chance of silent regressions when refactoring or extending the diff layer.
2. The `CellEdited` invariant about snapshot addresses will be captured in a single, reusable helper and consistently exercised across tests, making it harder for future code to drift away from the spec without being noticed.
3. The repository will support `cargo test` from the root, aligning developer and CI workflows with the documented meta-process and avoiding spurious test failures due to layout assumptions.

These changes are incremental, backward compatible, and can be implemented in a small follow-up cycle without blocking the current PG4 release.
