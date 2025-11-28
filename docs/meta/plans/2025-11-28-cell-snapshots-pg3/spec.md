# Mini-Spec: PG3 Cell Snapshots & Value Semantics  
Branch: `2025-11-28-cell-snapshots-pg3`

## 1. Scope

**Rust modules / files**

- `core/src/workbook.rs`
  - Introduce a `CellSnapshot` type and associated helpers.
  - Add unit tests for snapshot construction and equality (PG3.1, PG3.3).
- `core/src/lib.rs`
  - Re-export `CellSnapshot` alongside existing IR types for external use.
- `core/src/excel_open_xml.rs`
  - No behavioral changes planned for this cycle.
  - Add unit tests focused on value semantics for shared/inline strings, booleans, and error cells (`convert_value`, `parse_shared_strings`, `read_inline_string`). 
- `core/tests/pg3_snapshot_tests.rs` (new)
  - Integration tests using `pg3_value_and_formula_cells.xlsx` for snapshot-from-Excel and JSON round-trip (PG3.2, PG3.4). 

**Fixtures**

- Reuse existing generated fixture: `fixtures/generated/pg3_value_and_formula_cells.xlsx`
  - Sheet `Types` with:
    - A1: number `42`
    - A2: text `"hello"`
    - A3: boolean `TRUE`
    - A4: empty cell
    - B1: formula `=A1+1`
    - B2: formula `="hello" & " world"`
    - B3: formula returning boolean, e.g. `=A1>0` 

**Out of scope**

- Used-range semantics / non‑A1 origins (kept as a separate future incremental milestone).
- Any change to `open_workbook` or `open_data_mashup` public APIs.
- Any diff algorithms, DiffOp types, or grid alignment logic.
- M / DataMashup semantic parsing beyond existing `RawDataMashup`. 

---

## 2. Behavioral Contract

### 2.1 `CellSnapshot` structure

Introduce a lightweight snapshot of a single cell’s logical content:

```rust
pub struct CellSnapshot {
    /// A1-style address of the cell (e.g., "A1").
    pub addr: CellAddress,
    /// Parsed value of the cell, as seen by users.
    pub value: Option<CellValue>,
    /// Formula text if the cell has a formula; None for pure value cells.
    pub formula: Option<String>,
}
````

* `addr` is copied from the source `Cell`.
* `value` is copied from the `Cell`’s `value` field, which already encodes:

  * `Number(f64)` for numeric values.
  * `Text(String)` for text (including error text for now).
  * `Bool(bool)` for booleans.
* `formula` is copied from the `Cell`’s formula field (whatever string was parsed from `<f>`).

Helper constructor:

```rust
impl CellSnapshot {
    pub fn from_cell(cell: &Cell) -> CellSnapshot {
        // Pure data copy; no extra parsing.
    }
}
```

### 2.2 Snapshot behavior on basic values (PG3.1 / PG3.2)

**In-memory examples (PG3.1)**

Given:

```rust
let cell = Cell {
    address: CellAddress::from_str("A1").unwrap(),
    // row/col as in IR, not used by snapshot equality
    value: Some(CellValue::Number(42.0)),
    formula: None,
    // any existing format field unchanged
};
let snap = CellSnapshot::from_cell(&cell);
```

Expectations:

* `snap.addr.to_string() == "A1"`.
* `snap.value == Some(CellValue::Number(42.0))`.
* `snap.formula.is_none()`.

Similarly:

* Text cell:

  * `value = Some(CellValue::Text("hello".into()))`
  * Snapshot value is `"hello"`, `formula = None`.
* Boolean cell:

  * `value = Some(CellValue::Bool(true))`
  * Snapshot value is `Bool(true)`, `formula = None`.
* Empty cell:

  * `value = None`
  * Snapshot value is `None`, `formula = None`.

**From fixture `pg3_value_and_formula_cells.xlsx` (PG3.2)**

For sheet `Types`:

* A1..A4:

  * Snapshots match the in-memory examples above: numeric/text/bool/blank.
* B1 (formula `=A1+1`):

  * `snap.addr == "B1"`.
  * `snap.formula` is `Some(f)` where `f` contains `"A1+1"` (exact leading `=` is not asserted).
  * `snap.value == Some(CellValue::Number(43.0))` within normal floating tolerance.
* B2 (string concatenation):

  * `snap.formula` contains `"hello"` and `"world"`.
  * `snap.value == Some(CellValue::Text("hello world".into()))`.
* B3 (boolean formula):

  * `snap.formula` contains the comparison (e.g. `"A1>0"`).
  * `snap.value == Some(CellValue::Bool(true))`.

The tests only rely on stable textual substrings (like `"A1+1"`) rather than exact formatting of the `<f>` content, so underlying Excel/fixture nuances around leading `=` remain abstracted.

### 2.3 Snapshot equality semantics (PG3.3)

We define `PartialEq` / `Eq` for `CellSnapshot` as:

* Two snapshots are **equal** if and only if:

  * `value` fields are equal, and
  * `formula` fields are equal (both None or same string).
* `addr` is **ignored** for equality.

  * This allows comparing “before” and “after” snapshots without baking location into equality; location is already carried by `CellEdited.sheet` + `addr` in `DiffOp`.

Consequences:

* Same value, same formula, different addresses → `equal`.
* Same formula, different cached values → **not equal**.

  * For example, snapshots from `=A1+1` with cached values 43 vs 44 differ.
* Same value, `formula = None` vs `Some("=A1+1")` → **not equal**.

Formatting is currently **not represented** in `CellSnapshot`, so formatting-only changes are out of scope for equality distinctions. Future extensions may add a format summary field and adjust equality; this cycle codifies only value/formula-based equality.

### 2.4 JSON round-trip semantics (PG3.4)

`CellSnapshot` will derive `serde::Serialize` and `serde::Deserialize`.

* JSON representation is the default `serde` struct layout, e.g.:

```json
{
  "addr": "B1",
  "value": { "Number": 43.0 },
  "formula": "A1+1"
}
```

Round-trip guarantee:

* For any snapshot `s`, `serde_json::from_str(&serde_json::to_string(&s)?)?` yields `s2` such that `s == s2` under the equality rule above.

---

## 3. Constraints

* **No behavior change** to:

  * `open_workbook(path)` public signature or error variants.
  * `open_data_mashup(path)` and `RawDataMashup` APIs.
* `CellSnapshot` must:

  * Live in `core` alongside other IR types (`Workbook`, `Sheet`, `Cell`, etc.).
  * Be `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`.
* Tests may assume only small/medium fixtures; no performance work for large grids in this cycle.
* Dependencies:

  * Add `serde` (with `derive`) as a normal dependency of `core`.
  * `serde_json` is now a runtime dependency to support the JSON diff helpers exposed from `core` (`output::json::*`); this supersedes the earlier PG3 guidance that limited it to tests. Future cycles can re-isolate JSON in a higher-level crate if needed.
  * New deps must remain wasm-friendly and not introduce OS-only APIs.
* `CellValue`’s existing variants are left unchanged; error cells remain represented as `CellValue::Text` with the Excel error string (e.g. `"#DIV/0!"`) for now. This behavior is explicitly tested but can be revisited in a future cycle with a dedicated decision record.

---

## 4. Interfaces

### 4.1 New / extended IR types

In `core/src/workbook.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CellSnapshot {
    pub addr: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

impl CellSnapshot {
    pub fn from_cell(cell: &Cell) -> CellSnapshot {
        CellSnapshot {
            addr: cell.address.clone(),
            value: cell.value.clone(),
            formula: cell.formula.clone(),
        }
    }
}
```

* `Cell`, `CellValue`, `CellAddress` remain as currently implemented.

In `core/src/lib.rs`:

```rust
pub use crate::workbook::{
    Workbook, Sheet, SheetKind, Grid, Row, Cell, CellValue, CellAddress, CellSnapshot,
};
pub use output::json::{CellDiff, serialize_cell_diffs};
#[cfg(feature = "excel-open-xml")]
pub use output::json::{diff_workbooks, diff_workbooks_to_json};
```

The JSON helpers above are part of the public surface to support diff output; this extends the initial PG3 scope where `serde_json` was treated as test-only.

### 4.2 Internal helpers (test-only)

In `core/tests/pg3_snapshot_tests.rs`:

* Helper to fetch a cell by address from a sheet:

```rust
fn find_cell<'a>(sheet: &'a Sheet, addr: &str) -> &'a Cell {
    // Simple linear scan over rows/cells for the small PG3 fixture.
}
```

* Helper to build `CellSnapshot` from a workbook and address for test readability.

These helpers are confined to test modules.

---

## 5. Test Plan

### 5.1 PG3.1 – Snapshot from basic value cells (unit tests)

Location: `core/src/workbook.rs` (test module).

Tests:

1. `snapshot_from_number_cell`

   * Build `Cell` with `addr = "A1"`, `value = Number(42.0)`, `formula = None`.
   * `CellSnapshot::from_cell(&cell)`:

     * `addr == "A1"`.
     * `value == Some(Number(42.0))`.
     * `formula.is_none()`.

2. `snapshot_from_text_cell`

   * `value = Text("hello")`, `formula = None`.
   * Snapshot copies value; `formula.is_none()`.

3. `snapshot_from_bool_cell`

   * `value = Bool(true)`, `formula = None`.

4. `snapshot_from_empty_cell`

   * `value = None`, `formula = None`.

### 5.2 PG3.2 – Snapshot from formula cells (integration test)

Location: `core/tests/pg3_snapshot_tests.rs`.

Test: `pg3_value_and_formula_cells_snapshot_from_excel`

* Open `pg3_value_and_formula_cells.xlsx` via `open_workbook`.
* Get sheet `"Types"`.
* For addresses:

  * A1..A4: assert snapshot behavior as in §2.2.
  * B1: `formula` contains `"A1+1"`, `value == Number(43.0)`.
  * B2: `formula` contains `"hello"` and `"world"`, `value == Text("hello world")`.
  * B3: `formula` contains comparison (`">0"`), `value == Bool(true)`.

### 5.3 PG3.3 – Snapshot equality semantics (unit tests)

Location: `core/src/workbook.rs` (test module).

Tests:

1. `snapshot_equality_same_value_and_formula`

   * Build two identical snapshots (possibly differing in `addr`).
   * Assert `snap1 == snap2`.

2. `snapshot_inequality_different_value_same_formula`

   * Two snapshots with same formula text but different numeric values.
   * Assert `snap1 != snap2`.

3. `snapshot_inequality_value_vs_formula`

   * One snapshot with `value = Some(Number(42.0))`, `formula = None`.
   * Another with same value but `formula = Some("A1+1".into())`.
   * Assert `snap1 != snap2`.

4. `snapshot_equality_ignores_address`

   * Snapshots with same value/formula but different `addr` strings.
   * Assert `snap1 == snap2`.

### 5.4 PG3.4 – Snapshot JSON round-trip

Location: `core/tests/pg3_snapshot_tests.rs`.
res
Test: `snapshot_json_roundtrip`

* Using snapshots built in PG3.2:

  * For each snapshot `s` in a small list:

    * `let json = serde_json::to_string(&s).unwrap();`
    * `let s2: CellSnapshot = serde_json::from_str(&json).unwrap();`
    * `assert_eq!(s, s2);`
* Optionally add a single assertion on JSON shape for one snapshot (e.g., check that `"addr"` and `"value"` keys appear).

### 5.5 Value semantics coverage for Excel parsing (deferred item VAL-001)

Location: `core/src/excel_open_xml.rs` (test module).

New tests:

1. `parse_shared_strings_rich_text_flattens_runs`

   * Feed a minimal `sharedStrings.xml` with `<si>` containing two `<t>` runs (`"Hello"` and `" World"`).
   * Call `parse_shared_strings`.
   * Assert the first entry is `"Hello World"`.

2. `read_inline_string_preserves_xml_space_preserve`

   * Inline cell XML snippet using `<is><t xml:space="preserve"> hello</t></is>`.
   * Use `read_inline_string` to extract the string.
   * Assert value is `" hello"` (leading space preserved).
   * Pass this through `convert_value(Some("inlineStr"), Some(&value), &[])` and assert `CellValue::Text(" hello".into())`.

3. `convert_value_bool_0_1_and_other`

   * `convert_value(Some("b"), Some("0"), _)` → `Some(CellValue::Bool(false))`.
   * `convert_value(Some("b"), Some("1"), _)` → `Some(CellValue::Bool(true))`.
   * `convert_value(Some("b"), Some("2"), _)` → `None` (or whatever current behavior is; test codifies it explicitly based on current implementation).

4. `convert_value_error_cell_as_text`

   * `convert_value(Some("e"), Some("#DIV/0!"), _)` → `Some(CellValue::Text("#DIV/0!".into()))`.
   * This locks in the current “error-as-text” policy until a future decision changes it.

These tests directly address the verification report’s call for value-focused coverage on shared/inline strings and error cells, without changing existing semantics.

---

This mini-spec gives the implementer a precise, test-driven target: introduce a `CellSnapshot` type with well-defined equality and JSON behavior, wire it to the existing IR, and harden value semantics around strings and errors, thereby completing PG3 and closing a key verification gap.
