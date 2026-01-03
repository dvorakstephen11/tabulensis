# Remediation Plan: 2025-11-28-cell-snapshots-pg3

## Overview

This remediation plan focuses exclusively on *test hardening* for areas identified in the latest verification review:

1. Negative JSON deserialization paths for `CellAddress` (invalid A1 addresses in snapshot JSON).
2. End-to-end JSON diff behavior when a cell transitions between “value” and “empty”.
3. Error-path behavior for `diff_workbooks_to_json`, ensuring failures are surfaced as typed errors instead of panics.

No behavioral changes to the core parsing or diff logic are required; this plan only extends tests (plus optional, small spec-aligned helpers if desired). The goal is to make the test suite more exhaustive around failure modes and edge cases while preserving the existing public API and semantics.

## Fixes Required

### Fix 1: Add JSON deserialization tests for invalid `CellAddress`

- **Addresses Finding**: Finding 3 – Limited tests around `CellAddress` JSON failure modes  
- **Intent**: Prove that invalid `addr` strings in snapshot JSON fail cleanly with a clear error message, and never silently produce a bogus `CellAddress`.

#### Changes

**Files to touch**

- `core/tests/pg3_snapshot_tests.rs` (extend existing PG3 snapshot tests; no new module needed).

**Test design**

Add 1–2 negative tests that exercise invalid `addr` strings through the actual serde path of `CellSnapshot`:

1. **Test: `snapshot_json_rejects_invalid_addr_1A`**

   - Construct a raw JSON string for `CellSnapshot` with an invalid A1 address:

     ```json
     {
       "addr": "1A",
       "value": null,
       "formula": null
     }
     ```

     This shape is consistent with the current serde model:

     - `addr` → `CellAddress` via its custom `Deserialize` impl.
     - `value` → `Option<CellValue>` (null → `None`).
     - `formula` → `Option<String>` (null → `None`).

   - In `pg3_snapshot_tests.rs`, add something along the lines of:

     - `let result: Result<CellSnapshot, _> = serde_json::from_str(json_str);`
     - Assert `result.is_err()`.
     - Assert the error’s string form contains both:
       - `"invalid cell address"` (from the `DeError::custom` message).
       - `"1A"` (the offending address).
     - Do **not** assert equality on the full error string; only check `contains(...)` to keep the test robust against upstream formatting changes.

2. **Test: `snapshot_json_rejects_invalid_addr_A0` (optional but recommended)**

   - Same structure as above, but with `addr: "A0"` (invalid because row indexes start at 1).
   - Same assertions: `is_err()`, message includes `"invalid cell address"` and `"A0"`.

**Implementation notes**

- Reuse existing imports in `pg3_snapshot_tests.rs`:

  ```rust
  use excel_diff::{CellSnapshot, /* ... */};
