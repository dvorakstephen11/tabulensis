````markdown
# Remediation Plan: 2025-11-28-cell-snapshots-pg3

## Overview

This plan contains only the additional remediation work that is **not** already covered by the existing remediation plan you provided. It focuses on:

1. Hardening the JSON round-trip tests so that they actually validate the `addr` field instead of relying solely on `CellSnapshot`’s value/formula-only equality. :contentReference[oaicite:0]{index=0}  
2. Reconciling the broader `serde` derives on core IR types (`Workbook`, `Sheet`, `Grid`, `Row`, `Cell`, `CellAddress`, `CellValue`) with the architecture and mini-spec, which only explicitly called out `CellSnapshot` for serde support in this phase.   

These fixes assume the other remediation work (implementing PG3 integration tests and aligning `CellAddress` JSON representation) is already in scope from the prior plan.

## Fixes Required

### Fix 1: Harden JSON round-trip test to validate `addr`

- **Addresses Finding**: “JSON round-trip test doesn’t verify that `addr` is preserved” (original Verification Report Finding 2)

- **Changes**:  
  In `core/tests/pg3_snapshot_tests.rs`, strengthen `snapshot_json_roundtrip` so it validates the full snapshot, including `addr`, not just `(value, formula)` via `PartialEq`. Right now it does:

  ```rust
  let snap_back: CellSnapshot = serde_json::from_str(&json).expect("snapshot should parse");
  assert_eq!(snap, snap_back);
  ``` :contentReference[oaicite:2]{index=2}  

  but `CellSnapshot`’s `PartialEq` deliberately ignores `addr`:  

  ```rust
  impl PartialEq for CellSnapshot {
      fn eq(&self, other: &Self) -> bool {
          self.value == other.value && self.formula == other.formula
      }
  }
  ``` :contentReference[oaicite:3]{index=3}  

  That means a bug in `addr` serialization/deserialization would not break this test.

  Update the test as follows:

  - After deserializing, assert **address equality explicitly**:

    ```rust
    let snap_back: CellSnapshot = serde_json::from_str(&json).expect("snapshot should parse");
    assert_eq!(snap.addr, snap_back.addr);
    assert_eq!(snap, snap_back); // still checks value + formula
    ```

  - Keep (or refine) the existing JSON-shape smoke checks (`json.contains("\"addr\"")`, `"\"value\""`), since they still help guard the schema surface, especially once the JSON representation from the other remediation plan is in place.   

- **Tests**:

  - Re-run the full suite to ensure the stronger assertion is compatible with the chosen JSON representation for `addr`:
    - `cargo test`
    - `cargo test --tests`
    - `cargo check --target wasm32-unknown-unknown --no-default-features` (to confirm no serde changes accidentally break the wasm build, per the existing testing constraints).   

  - If desired, add a focused unit test that constructs a `CellSnapshot` with a non-default `addr` (e.g., `Z9`), manually tampers with the JSON to simulate a bad round-trip on `addr`, and asserts that the new `assert_eq!(snap.addr, snap_back.addr)` catches it.

---

### Fix 2: Reconcile serde derives on core IR types with architectural intent

- **Addresses Finding**: “Extra serde derives on core IR types extend the public surface beyond the mini-spec” (original Verification Report Finding 4)

- **Changes**:  
  The current implementation derives `Serialize`/`Deserialize` for the entire workbook IR:

  ```rust
  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  pub struct Workbook { ... }

  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  pub struct Sheet { ... }

  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  pub struct Grid { ... }

  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  pub struct Row { ... }

  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  pub struct Cell { ... }

  #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct CellAddress { ... }

  #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
  pub enum CellValue { ... }
  ``` :contentReference[oaicite:6]{index=6}  

  but the PG3 mini-spec and specification only explicitly required serde for `CellSnapshot` in this cycle, and emphasized keeping the core library format-agnostic while still being easily serializable for snapshots.   

  Decide how much of this broader serde surface you actually want to commit to now, and make the code + docs match:

  **Option A – Intentionally keep broad serde on IR types (and document/test it):**

  - Treat the existing derives on `Workbook`, `Sheet`, `Grid`, `Row`, `Cell`, `CellAddress`, and `CellValue` as *intentional* public surface.
  - Update documentation (e.g., `excel_diff_specification.md` and/or a short addendum to the mini-spec) to note that the full grid IR is now serde-serializable, not just `CellSnapshot`.   
  - Add at least one smoke test that:
    - Builds a small `Workbook` in memory,
    - Serializes it to JSON with `serde_json`,
    - Deserializes it back, and
    - Asserts equality on the IR types you expect to be stable.
  - Clearly treat this JSON as **provisional** (or explicitly versioned) if you don’t want to permanently lock the exact schema yet.

  **Option B – Narrow serde surface back to `CellSnapshot` only for this cycle:**

  - Remove `serde::Serialize` / `serde::Deserialize` from the derives on `Workbook`, `Sheet`, `Grid`, `Row`, `Cell`, `CellAddress`, and `CellValue`, leaving serde derives only on `CellSnapshot` as originally envisioned for PG3.   
  - Confirm there are no tests or production code paths that rely on serializing those broader IR types (a quick search for `serde_json::to_string(&workbook)` or similar should remain empty). :contentReference[oaicite:10]{index=10}  
  - If any code turns out to depend on the broader derives, either:
    - Move that behavior behind an explicit feature flag (e.g., a CLI/UX-only feature), or
    - Promote those derives to an intentional part of the public API and fall back to Option A.

- **Tests**:

  - If you choose **Option A**:
    - Add a simple `Workbook` JSON round-trip test as described above.
    - Re-run the full test suite to ensure no regressions, and consider adding a small assertion on the JSON schema shape (keys present, basic structure) to avoid accidental breaking changes.

  - If you choose **Option B**:
    - Re-run `cargo test` and ensure everything compiles and passes with only `CellSnapshot` using serde.
    - Make sure the PG3 snapshot JSON tests still work (they only need serde for `CellSnapshot`).   

---

## Constraints

- Do **not** change the public signatures or error variants of `open_workbook` / `open_data_mashup` as part of this remediation; those are intentionally out of scope for PG3.   
- Any serde-related changes must remain compatible with the existing `wasm32-unknown-unknown` build and avoid introducing host-only dependencies, consistent with the project’s cross-platform goals.   
- Keep `CellSnapshot`’s Rust shape and equality semantics unchanged (addr + optional value + optional formula; equality is value/formula only). Future milestones build on that contract.   

## Expected Outcome

After this remediation is complete (in addition to the fixes from the prior plan):

- The JSON round-trip tests for `CellSnapshot` will **explicitly verify** that `addr` survives serialization/deserialization, eliminating the blind spot created by equality ignoring `addr`.   
- The serde surface of the core IR types will be **intentional and documented**:
  - Either narrowed back to `CellSnapshot` only (matching the PG3 mini-spec), or
  - Explicitly broadened, with a minimal set of tests that lock in the behavior you want to support.
- With these additional steps layered on top of the existing remediation plan, PG3’s snapshot and JSON behavior will be better specified, better tested, and better aligned with the architecture, reducing the risk of future regressions as later milestones (PG4/PG5 diff work) build on this IR.
````
