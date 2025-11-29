
# 2025-11-28b-diffop-pg4 Mini-Spec  
DiffOp domain types and JSON wire contract (PG4)

## 1. Scope

### 1.1 In-scope modules and types

Rust modules to add or modify:

- `core/src/diff.rs` (new)
  - Public domain types for the diff layer:
    - `DiffOp`
    - `DiffReport`
    - `SheetId`
    - `RowSignature`
    - `ColSignature`
- `core/src/lib.rs`
  - Re-exports for the new diff types so downstream code can use them:
    - `DiffOp`, `DiffReport`, `SheetId`, `RowSignature`, `ColSignature`
- `core/tests/pg4_diffop_tests.rs` (new)
  - Integration-style tests for PG4 type-level and JSON-level behavior.

Existing modules used but not structurally changed:

- `core/src/workbook.rs`
  - `CellSnapshot`, `CellAddress`, and `CellValue` (used inside `CellEdited` DiffOps).
- `core/src/output/json.rs`
  - Existing JSON `CellDiff` helpers remain as-is; this spec does not wire `DiffOp` into `diff_workbooks_to_json` yet.

### 1.2 Out of scope for this cycle

- No grid alignment or diff algorithm (`PG5`, `G1–G7`, or the hybrid row/column alignment in the spec).
- No database-mode diff (`D1–D7`).
- No M/DataMashup semantic diff or M-specific DiffOp variants (`M6` and later).
- No CLI or WASM integration changes beyond making the new types public.
- No changes to the existing `CellDiff` JSON surface or its tests.

This cycle is purely about defining the DiffOp and DiffReport types and locking in their JSON representation and round-trip behavior (PG4.1–PG4.4).

---

## 2. Behavioral Contract

### 2.1 DiffOp variants and meaning

Introduce a `diff` module with the core enum:

```rust
pub type SheetId = String;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum DiffOp {
    SheetAdded {
        sheet: SheetId,
    },
    SheetRemoved {
        sheet: SheetId,
    },
    RowAdded {
        sheet: SheetId,
        row_idx: u32,                    // zero-based row index in the grid IR
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    RowRemoved {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    ColumnAdded {
        sheet: SheetId,
        col_idx: u32,                    // zero-based column index
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    ColumnRemoved {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    BlockMovedRows {
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    BlockMovedColumns {
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    CellEdited {
        sheet: SheetId,
        addr: crate::workbook::CellAddress,
        from: crate::workbook::CellSnapshot,
        to: crate::workbook::CellSnapshot,
    },
}
````

Support types:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowSignature {
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ColSignature {
    pub hash: u64,
}
```

Plain-language meaning:

* `SheetAdded` / `SheetRemoved`
  Structural changes at the workbook level: a sheet appears or disappears. For now, `sheet` is the sheet’s name as seen in `Workbook.sheets[*].name`.

* `RowAdded` / `RowRemoved`
  A whole row is inserted or deleted in a sheet in spreadsheet mode (grid diff).
  `row_idx` is zero-based index in the `Grid.rows` vector.
  `row_signature` is an optional hash summarizing row contents, useful for debugging and matching but not required for correctness.

* `ColumnAdded` / `ColumnRemoved`
  A column is inserted or deleted.
  `col_idx` is zero-based index in the column dimension of the grid.
  `col_signature` parallels `row_signature`.

* `BlockMovedRows` / `BlockMovedColumns`
  A contiguous block of rows or columns is moved within the same sheet.
  `src_*` and `dst_*` indexes are zero-based; `row_count` / `col_count` gives block size.
  `block_hash` is optional future-proofing for move identity.

* `CellEdited`
  A single cell’s logical content changed.
  `addr` is a `CellAddress` (zero-based row/col underneath, with A1 string JSON representation).
  `from` and `to` are `CellSnapshot` instances:

  * Each snapshot carries `addr`, `value: Option<CellValue>`, and `formula: Option<String>`.
  * Snapshot equality follows the existing PG3 rule (value + formula equality, address ignored for equality).

This cycle does not require producing these ops from real workbooks; it only defines and tests the type- and JSON-level behavior.

### 2.2 Example semantic scenarios

These examples are for intuition and future algorithm tests; in this cycle they exist only as type-level tests (hand-constructed DiffOps).

1. **Single cell literal change in an otherwise identical sheet**

   * Before: `Sheet1!C3 = 1`
   * After:  `Sheet1!C3 = 2`
     Expected DiffOps (conceptually):

   ```rust
   DiffOp::CellEdited {
       sheet: "Sheet1".to_string(),
       addr: "C3".parse().unwrap(),
       from: CellSnapshot { addr: "C3".parse().unwrap(), value: Some(CellValue::Number(1.0)), formula: None },
       to:   CellSnapshot { addr: "C3".parse().unwrap(), value: Some(CellValue::Number(2.0)), formula: None },
   }
   ```

2. **Row append at bottom of sheet**

   * Before: 10 populated rows on `Sheet1`.
   * After: 11 populated rows, with a new row at index 10 (zero-based).
     Conceptual DiffOp:

   ```rust
   DiffOp::RowAdded {
       sheet: "Sheet1".to_string(),
       row_idx: 10,
       row_signature: Some(RowSignature { hash: 0xDEADBEEF_u64 }),
   }
   ```

3. **Row truncate at bottom of sheet**

   ```rust
   DiffOp::RowRemoved {
       sheet: "Sheet1".to_string(),
       row_idx: 9,
       row_signature: None,
   }
   ```

4. **Block move of three rows upwards**

   ```rust
   DiffOp::BlockMovedRows {
       sheet: "Sheet1".to_string(),
       src_start_row: 10,
       row_count: 3,
       dst_start_row: 5,
       block_hash: Some(0x12345678_u64),
   }
   ```

These examples will be turned into concrete test cases that assert struct fields, JSON tags, and round-trips.

### 2.3 JSON shape contract

Serialization choices:

* `DiffOp` is a tagged enum using `#[serde(tag = "kind")]`.
* Field names are stable and human-readable; no internal or implementation-specific fields leak.
* Optional fields (`row_signature`, `col_signature`, `block_hash`) are omitted from JSON when `None`.

Representative JSON for `CellEdited`:

```json
{
  "kind": "CellEdited",
  "sheet": "Sheet1",
  "addr": "C3",
  "from": {
    "addr": "C3",
    "value": { "Number": 1.0 },
    "formula": null
  },
  "to": {
    "addr": "C3",
    "value": { "Number": 2.0 },
    "formula": null
  }
}
```

Notes:

* `addr` fields (both on the DiffOp and inside snapshots) use the existing `CellAddress` JSON surface:

  * Serialized as a single A1-style string, e.g., `"C3"`.
  * On deserialize, invalid A1 strings are rejected with the same error behavior as in PG3 tests.
* `CellValue` uses serde’s default enum representation:

  * Numbers: `{ "Number": 42.0 }`
  * Text: `{ "Text": "hello" }`
  * Bool: `{ "Bool": true }`

Representative JSON for a row addition with a signature:

```json
{
  "kind": "RowAdded",
  "sheet": "Sheet1",
  "row_idx": 10,
  "row_signature": { "hash": 3735928559 }
}
```

Representative JSON for a block move without a hash (`block_hash` is omitted):

```json
{
  "kind": "BlockMovedRows",
  "sheet": "Sheet1",
  "src_start_row": 10,
  "row_count": 3,
  "dst_start_row": 5
}
```

---

## 3. DiffReport container and invariants

### 3.1 Type definition

Introduce a simple container for a diff report:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffReport {
    pub version: String,
    pub ops: Vec<DiffOp>,
}

impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            ops,
        }
    }
}
```

JSON shape:

```json
{
  "version": "1",
  "ops": [
    { "kind": "SheetAdded", "sheet": "Sheet1" },
    { "kind": "RowAdded", "sheet": "Sheet1", "row_idx": 10 },
    { "kind": "CellEdited", "sheet": "Sheet1", "addr": "C3", "from": { ... }, "to": { ... } }
  ]
}
```

Invariants for this cycle:

* `version` is always `"1"` for newly created reports via `DiffReport::new`.
* `ops` preserves insertion order; serialization and deserialization must retain order.
* The report does not include file-path metadata yet; that will be added at the CLI / scenario layer later.

---

## 4. Constraints

### 4.1 Performance and memory

* No heavy computation is introduced in this cycle. DiffOps and DiffReport are simple structs/enums with a small number of fields.
* No additional dependencies beyond serde/serde_json, which are already in use for snapshots and JSON cell diffs.
* The new types are `Send`/`Sync` by composition (no interior mutability), suitable for future multi-threaded or async diff pipelines.

### 4.2 Streaming and layering

* DiffOp and DiffReport live in their own `diff` module and do not perform I/O.
* They depend only on `workbook::CellSnapshot` and `CellAddress` for the payload of `CellEdited`.
* Streaming and large-file behavior are unaffected in this cycle; these types are intended to be emitted by future streaming diff algorithms.

### 4.3 Invariants and stability

* `sheet` must always match a `Workbook.sheets[*].name` for valid reports, but this invariant is enforced by producers (algorithms), not by the types themselves.
* `addr` on `CellEdited` DiffOps must match `from.addr` and `to.addr`. This is an invariant the future grid diff implementation must maintain; tests in this cycle will assert it for hand-constructed values.
* Row and column indexes (`row_idx`, `col_idx`, `src_*`, `dst_*`) are zero-based, consistent with the existing `Grid` IR and `CellAddress` internal indices.
* JSON schema (field names, tag name `"kind"`, and container structure) is treated as stable once PG4 is complete; any breaking change must be treated as a new schema version.

---

## 5. Interfaces

### 5.1 Public APIs added or changed

New module:

* `core/src/diff.rs`
  Public types:

  * `pub type SheetId = String;`
  * `pub struct RowSignature { pub hash: u64 }`
  * `pub struct ColSignature { pub hash: u64 }`
  * `pub enum DiffOp { ... }`
  * `pub struct DiffReport { pub version: String, pub ops: Vec<DiffOp> }`

`core/src/lib.rs` re-exports:

```rust
pub mod diff;

pub use diff::{ColSignature, DiffOp, DiffReport, RowSignature, SheetId};
```

No changes to existing public APIs:

* `CellSnapshot`, `CellAddress`, `CellValue`, `Workbook`, `Sheet`, `Grid` remain as documented.
* JSON cell-diff helpers remain:

  * `CellDiff`
  * `serialize_cell_diffs`
  * `diff_workbooks`
  * `diff_workbooks_to_json`

The CLI and any external consumers can begin to depend on `DiffOp` and `DiffReport` as stable types, even though the engine does not yet produce them from real workbooks.

### 5.2 Future evolution

This cycle does not:

* Wire `DiffReport` into the CLI or any end-to-end diff function.
* Add M-specific or metadata-specific DiffOp variants (those will come with M6 and metadata milestones).
* Introduce a `Diffable` trait or a full `diff_workbooks -> Vec<DiffOp>` pipeline; that is left for later PG5/Gx work.

---

## 6. Test Plan (PG4)

All tests for this cycle live in a new file:

* `core/tests/pg4_diffop_tests.rs`

They are pure Rust tests; no new Excel fixtures are required.

### 6.1 PG4.1 – DiffOp construction and required fields

**Tests to add:**

1. `pg4_construct_cell_edited_diffop`

   * Construct a `CellEdited` DiffOp with:

     * `sheet = "Sheet1"`
     * `addr = "C3".parse().unwrap()`
     * `from` and `to` snapshots with different values and identical addresses.
   * Assertions:

     * `matches!(op, DiffOp::CellEdited { .. })`
     * `sheet == "Sheet1"`
     * `addr.to_string() == "C3"`
     * `from.addr == addr` and `to.addr == addr`
     * `from.value != to.value`

2. `pg4_construct_row_and_column_diffops`

   * Construct each of:

     * `RowAdded` with and without `row_signature`.
     * `RowRemoved` with and without `row_signature`.
     * `ColumnAdded` and `ColumnRemoved` similarly.
   * Assertions:

     * All required fields (`sheet`, `row_idx` / `col_idx`) have expected values.
     * Optional signatures can be `Some` or `None` and are preserved on equality comparisons.

3. `pg4_construct_block_move_diffops`

   * Build `BlockMovedRows` and `BlockMovedColumns` with:

     * Non-zero `row_count` / `col_count`.
     * A `Some(block_hash)` variant and a `None` variant.
   * Assertions:

     * Numeric fields match the input.
     * Optional `block_hash` behaves as expected.

### 6.2 PG4.2 – JSON shape tests

**Tests to add:**

1. `pg4_cell_edited_json_shape`

   * Construct a `CellEdited` DiffOp as in 6.1.
   * Serialize with `serde_json::to_value(&op)`.
   * Assertions against the JSON value:

     * `json["kind"] == "CellEdited"`.
     * `json["sheet"] == "Sheet1"`.
     * `json["addr"] == "C3"`.
     * `json["from"]["addr"] == "C3"`.
     * `json["to"]["addr"] == "C3"`.
     * No unexpected top-level keys:

       * Collect object keys and assert they equal the set `{"kind","sheet","addr","from","to"}`.

2. `pg4_row_added_json_optional_signature`

   * Create `RowAdded` with `row_signature: None` and serialize.
   * Assert:

     * `json["kind"] == "RowAdded"`.
     * `json["sheet"]` and `json["row_idx"]` present.
     * `json.as_object().unwrap().get("row_signature").is_none()` (field omitted when `None`).
   * Create another `RowAdded` with `row_signature: Some(RowSignature { hash: 123 })`.

     * Assert `json["row_signature"]["hash"] == 123`.

3. `pg4_block_moved_rows_json_optional_hash`

   * Same pattern as above for `BlockMovedRows` and `block_hash`.

### 6.3 PG4.3 – JSON round-trip stability

**Tests to add:**

1. `pg4_diffop_roundtrip_each_variant`

   * For each variant constructed in 6.1:

     * Serialize to string with `serde_json::to_string`.
     * Deserialize back to `DiffOp`.
   * Assertions:

     * `deser == original`.
     * The variant matches (`matches!` or equality).
     * For `CellEdited`, re-assert `sheet`, `addr`, `from`, and `to` fields.

2. `pg4_cell_edited_roundtrip_preserves_snapshot_addrs`

   * Use a `CellEdited` with snapshots whose `addr` matches the DiffOp `addr`.
   * Round-trip through JSON.
   * Assert:

     * `from.addr == "C3".parse().unwrap()`.
     * `to.addr == "C3".parse().unwrap()`.
     * No addr changes during serialization or deserialization.

### 6.4 PG4.4 – DiffReport list container

**Tests to add:**

1. `pg4_diff_report_roundtrip_preserves_order`

   * Build several DiffOps of different variants in a specific order.

   * Create a report:

     ```rust
     let ops = vec![op1.clone(), op2.clone(), op3.clone()];
     let report = DiffReport::new(ops.clone());
     ```

   * Serialize with `serde_json::to_string(&report)`.

   * Deserialize back to `DiffReport`.

   * Assertions:

     * `report.version == "1"` both before and after round-trip.
     * `report.ops == ops`.
     * The sequence of `kind` strings in `report.ops` matches the original order.

2. `pg4_diff_report_json_shape`

   * Serialize a small `DiffReport`.
   * Parse to `serde_json::Value`.
   * Assertions:

     * Top-level object has exactly two keys: `"version"` and `"ops"`.
     * `version` equals `"1"`.
     * `ops` is an array.
     * Each element in `ops` has a `"kind"` field consistent with its constructed variant.

---

## 7. Milestone linkage

This cycle advances:

* **Testing Plan:** Phase 3 – MVP diff slice

  * **PG4 – DiffOp type-level tests and JSON contract**

    * PG4.1: verified via construction tests in §6.1.
    * PG4.2: verified via JSON shape tests in §6.2.
    * PG4.3: verified via round-trip tests in §6.3.
    * PG4.4: verified via DiffReport container tests in §6.4.

Downstream milestones that will build on this work:

* PG5/PG6 (in-memory grid diff and object graph vs grid semantics) will emit these DiffOps from `Workbook` / `Sheet` / `Grid` comparisons.
* G1–G7 and D1–D7 integration tests will assert on `Vec<DiffOp>` or `DiffReport` rather than ad-hoc JSON, giving a clearer boundary between algorithms and presentation.
* M6 (first M diff engine) will later extend `DiffOp` with M-specific variants without revisiting the core grid-related ops defined here.

This keeps the DiffOp and DiffReport contract small but stable, with explicit tests that the implementer and future milestones can rely on.
