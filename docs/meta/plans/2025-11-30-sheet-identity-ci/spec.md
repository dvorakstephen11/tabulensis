# 2025-11-30-sheet-identity-ci – Sheet identity semantics (D1)

Goal: bring the implementation of sheet identity in line with the spec’s “(case-insensitive) name plus type” rule, and lock that behavior in with tests, without changing the public API surface.

---

## 1. Scope

### In scope

**Core code**

- `core/src/engine.rs`
  - `diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport`
  - New private helper types/functions for sheet identity and ordering (e.g. `SheetKey`, `make_sheet_key`, sort helper).
- `core/src/workbook.rs`
  - `SheetKind` enum derives (`Hash` in addition to existing traits) to allow using `SheetKind` as part of a HashMap key. :contentReference[oaicite:13]{index=13}

**Tests**

- `core/tests/engine_tests.rs`
  - New tests for sheet identity semantics:
    - Case-insensitive matching between old/new workbooks.
    - Identity including `SheetKind` rather than name alone.

### Out of scope

- Any changes to:
  - Grid diff semantics inside `diff_grids` (still naive row/col loop for now). 
  - Row/column signatures or alignment algorithms (AMR, row hashing, GridView).
  - JSON output format or schema (`DiffReport`, `DiffOp`, `CellDiff`, `serialize_*`). 
  - CLI or fixture generation.
  - Rename detection or similarity-based object matching (that’s a later milestone building on top of correct identity). :contentReference[oaicite:16]{index=16}

---

## 2. Behavioral Contract

This section describes the *observable* behavior of `diff_workbooks` after this change.

Let “identity key” mean `(lowercase(sheet_name), SheetKind)`.

### 2.1 Case-insensitive sheet matching

1. **No changes, names differ only by case**

   - Old workbook: one sheet named `Sheet1`, Worksheet, with a 1×1 grid value `1.0`.
   - New workbook: one sheet named `sheet1`, Worksheet, grid also 1×1 with value `1.0`.

   **Expected behavior**

   - `diff_workbooks(&old, &new)` produces a `DiffReport` with:
     - `ops.is_empty() == true` (no structural or cell-level operations).
   - In particular:
     - No `SheetAdded` for `"sheet1"`.
     - No `SheetRemoved` for `"Sheet1"`.
     - No `CellEdited` operations.

2. **Cell edit on a sheet whose name differs only by case**

   - Old: `Sheet1!A1 = 1.0`
   - New: `sheet1!A1 = 2.0`

   **Expected behavior**

   - Exactly one `DiffOp::CellEdited`:
     - `sheet == "Sheet1"` (uses the **old** workbook’s sheet name as the canonical `SheetId`).
     - `addr.to_a1() == "A1"`.
     - `from.value == Some(Number(1.0))`, `to.value == Some(Number(2.0))`. 
   - No `SheetAdded`/`SheetRemoved` ops for this sheet.

3. **Case-only rename with no content changes**

   - Old: sheet named `Summary`, Worksheet.
   - New: sheet named `SUMMARY`, Worksheet.
   - Grids identical.

   **Expected behavior**

   - `diff_workbooks` treats these as the same sheet; result has:
     - No sheet-level operations.
     - No cell-level operations.
   - Rename detection (including case-only renames) is **not** yet implemented; there is no `Renamed`-style op, and that’s acceptable per this mini-spec.

### 2.2 Identity includes sheet type (SheetKind)

4. **Same name (ignoring case), different type**

   - Old: a `Worksheet` named `Sheet1` (non-empty grid).
   - New: a `Chart` named `Sheet1` (empty or placeholder grid).

   These are different objects per spec, because identity is `(name_ci, type)`. 

   **Expected behavior**

   - `diff_workbooks` emits:
     - One `DiffOp::SheetRemoved { sheet: "Sheet1" }` for the Worksheet.
     - One `DiffOp::SheetAdded { sheet: "Sheet1" }` for the Chart.
   - No attempt is made to grid-diff Worksheet vs Chart.

5. **Multiple sheets with distinct names and same type**

   - Behavior for sheets whose names differ beyond case (e.g., `"Sheet1"` vs `"Budget"`) is unchanged:
     - Sheets only in old → `SheetRemoved`.
     - Sheets only in new → `SheetAdded`.
     - Sheets present in both identity sets → grid-diffed.

### 2.3 Determinism and ordering

6. **Deterministic ordering of workbook-level operations**

   - The iteration order over identity keys remains deterministic:
     - Keys are derived from `(lowercase name, kind)`.
     - Union of keys is sorted by `name_lower` (lexicographically), then by a stable SheetKind order (Worksheet < Chart < Macro < Other).
   - For identical inputs across runs/platforms, the sequence of `DiffOp`s at the workbook level does not change due to this refactor (except for the corrected identity semantics themselves). 

### 2.4 Duplicate identity keys (defensive stance)

- A single workbook should not contain two `Sheet` entries whose identity key `(lowercase(name), SheetKind)` collides; this is treated as invalid IR rather than a supported edge case.
- `diff_workbooks` keeps release behavior deterministic (later insert wins) but includes a `debug_assert!` to surface duplicates during development, so producers such as `open_workbook` can be fixed if they ever emit a duplicate.

---

## 3. Constraints

### 3.1 Performance & complexity

- Sheet identity computation remains **O(N)** where N is the number of sheets:
  - Each sheet contributes:
    - A lowercase name allocation (`to_lowercase()`).
    - One HashMap insert into `old_sheets` or `new_sheets`.
- Sorting the union of keys is **O(N log N)** with tiny N (workbooks rarely exceed dozens of sheets).
- No changes to grid-level complexity (`diff_grids` still does an O(R×C) scan for now). 
- No additional allocations proportional to grid size are introduced.

### 3.2 Memory

- The extra memory for sheet identity:
  - One `String` per sheet for `name_lower`.
  - `SheetKind` cloned into the key.
- This is negligible compared to existing workbook and grid storage and does not conflict with the overall memory budget guidance in the grid diff spec. 

### 3.3 Determinism

- The engine must continue to produce deterministic output for identical inputs:
  - Union of keys is sorted before iteration.
  - No use of HashMap iteration order to drive observable ordering. :contentReference[oaicite:22]{index=22}
- The introduction of a new key type must **not** introduce platform-dependent behavior.

---

## 4. Interfaces

### 4.1 Public API surface

No function signatures change.

- `pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport` (in `core/src/engine.rs`) retains the same signature and location; only its *internal* sheet-mapping logic changes. 
- `SheetId` remains a type alias for `String`.
- The `DiffOp` enum remains unchanged at the type level. :contentReference[oaicite:24]{index=24}

**Semantic clarifications to be relied upon going forward:**

- For any `DiffOp` that includes `sheet: SheetId`:
  - If the operation relates to a sheet present in both workbooks:
    - `sheet` is the **old workbook’s** sheet name (original casing).
  - If the operation is `SheetAdded`:
    - `sheet` is the new workbook’s sheet name (original casing).
  - If the operation is `SheetRemoved`:
    - `sheet` is the old workbook’s sheet name (original casing).

This defines a stable, predictable convention clients can depend on.

### 4.2 IR types

- `SheetKind` in `core/src/workbook.rs` gains a `Hash` derive:

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  pub enum SheetKind {
      Worksheet,
      Chart,
      Macro,
      Other,
  }
````

* This trait addition is backward compatible and enables `SheetKind` to be used as part of a HashMap key (`SheetKey`).

### 4.3 Internal helper structures

Introduce a private helper inside `engine.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}
```

With:

* `fn make_sheet_key(sheet: &Sheet) -> SheetKey`

  * Uses `sheet.name.to_lowercase()` and `sheet.kind.clone()`.
* `fn sheet_kind_order(kind: &SheetKind) -> u8`

  * Returns a stable rank (0..=3) used only to enforce deterministic sort order.

These helpers are not exposed outside `engine.rs`.

---

## 5. Test Plan

All work in this cycle must be expressed and validated via tests.

### 5.1 New tests

File: `core/tests/engine_tests.rs` 

Add the following tests:

1. **`sheet_name_case_insensitive_no_changes`**

   **Setup**

   ```rust
   #[test]
   fn sheet_name_case_insensitive_no_changes() {
       let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
       let new = make_workbook(vec![("sheet1", vec![(0, 0, 1.0)])]);

       let report = diff_workbooks(&old, &new);
       assert!(report.ops.is_empty());
   }
   ```

   **Purpose**

   * Asserts that sheets whose names differ only by case are treated as the same identity when content is identical (D1 fix).

2. **`sheet_name_case_insensitive_cell_edit`**

   **Setup**

   ```rust
   #[test]
   fn sheet_name_case_insensitive_cell_edit() {
       let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
       let new = make_workbook(vec![("sheet1", vec![(0, 0, 2.0)])]);

       let report = diff_workbooks(&old, &new);
       assert_eq!(report.ops.len(), 1);

       match &report.ops[0] {
           DiffOp::CellEdited { sheet, addr, from, to } => {
               assert_eq!(sheet, "Sheet1");
               assert_eq!(addr.to_a1(), "A1");
               assert_eq!(from.value, Some(CellValue::Number(1.0)));
               assert_eq!(to.value, Some(CellValue::Number(2.0)));
           }
           other => panic!("expected CellEdited, got {other:?}"),
       }
   }
   ```

   **Purpose**

   * Verifies that:

     * Case-insensitive identity allows content diffing.
     * The canonical `SheetId` used in ops comes from the old workbook’s name.

3. **`sheet_identity_includes_kind`**

   **Setup**

   ```rust
   #[test]
   fn sheet_identity_includes_kind() {
       // small helper grid
       let mut grid = Grid::new(1, 1);
       grid.insert(Cell {
           row: 0,
           col: 0,
           address: CellAddress::from_indices(0, 0),
           value: Some(CellValue::Number(1.0)),
           formula: None,
       });

       let worksheet = Sheet {
           name: "Sheet1".to_string(),
           kind: SheetKind::Worksheet,
           grid: grid.clone(),
       };

       let chart = Sheet {
           name: "Sheet1".to_string(),
           kind: SheetKind::Chart,
           grid,
       };

       let old = Workbook { sheets: vec![worksheet] };
       let new = Workbook { sheets: vec![chart] };

       let report = diff_workbooks(&old, &new);

       let mut added = 0;
       let mut removed = 0;
       for op in &report.ops {
           match op {
               DiffOp::SheetAdded { sheet } if sheet == "Sheet1" => added += 1,
               DiffOp::SheetRemoved { sheet } if sheet == "Sheet1" => removed += 1,
               _ => {}
           }
       }

       assert_eq!(added, 1, "expected one SheetAdded for Chart 'Sheet1'");
       assert_eq!(removed, 1, "expected one SheetRemoved for Worksheet 'Sheet1'");
       assert_eq!(report.ops.len(), 2, "no other ops expected");
   }
   ```

   **Purpose**

   * Confirms that identity includes `SheetKind`, so a Worksheet→Chart change shows up as remove+add, not as a matched sheet.

### 5.2 Existing tests (must continue to pass)

Verify that the following suites remain green without modification:

* `core/tests/engine_tests.rs` existing tests:

  * `identical_workbooks_produce_empty_report`
  * `sheet_added_detected`
  * `sheet_removed_detected`
  * `cell_edited_detected`
  * `diff_report_json_round_trips`
* PG1, PG2, PG3, PG4 test suites:

  * `pg1_ir_tests.rs`
  * `addressing_pg2_tests.rs`
  * `pg3_snapshot_tests.rs`
  * `pg4_diffop_tests.rs`
* `excel_open_xml_tests.rs`, `sparse_grid_tests.rs`, `signature_tests.rs`, `output_tests.rs`.

No golden expectations for sheet names are changed in these tests, so they should continue to pass unchanged.

### 5.3 Milestone alignment

This incremental milestone aligns with:

* Spec section **7.1 Sheet and Object Identity** (enforcing the “case-insensitive name plus type” rule). 
* Docs-vs-implementation discrepancy **D1** and its recommendation to fix identity semantics *before* implementing advanced grid alignment.

Once this mini-milestone is complete and tests are in place, future cycles can safely focus on:

* Enhancing row signatures and GridView structures.
* Implementing the AMR-based alignment pipeline (PG5+, G1–G7).
* Adding rename-detection tests and logic, which will rely on the corrected identity layer.



