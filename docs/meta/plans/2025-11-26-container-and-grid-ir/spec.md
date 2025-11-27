Here’s the plan for the next cycle.

## Mini-Spec: Excel Container Open + Workbook/Grid IR + Addressing

### 1. Goal

Stand up the first real slice of the engine:

1. Detect and open an Excel Open XML container (`.xlsx`, `.xlsm` — `.xlsb` optional).

2. Parse enough of the workbook to build the normalized IR for:

   * `Workbook`
   * `Sheet` (name + kind)
   * `Grid` / `Row` / `Cell` (values + formulas, with a well-defined used range)

3. Implement A1-addressing helpers and ensure IR row/column indices and A1 strings stay in sync.

This cycle **does not** implement any diff logic, DataMashup parsing, or M/DAX semantics; it ends with a clean `Workbook` IR and a set of container + IR tests.

---

### 2. Scope of Changes

#### 2.1 New library module and IR types

Create a library entry point (`core/src/lib.rs`) and move the core logic there. `main.rs` can remain a tiny stub or be left as-is for now.

Add modules:

* `workbook` (or `ir`): owns core IR types.
* `excel_open_xml` (or `excel_container`): owns Excel container detection + workbook parsing into IR.
* `addressing`: A1 ↔ (row, col) utilities (can be nested inside `workbook` if preferred).

IR should align with the architecture doc but only the workbook/grid subset is required in this cycle. 

Concrete types to introduce (names are important; field sets can evolve):

```text
Workbook {
    sheets: Vec<Sheet>,
    // data_model, mashup fields will be added later, keep room for them
}

Sheet {
    name: String,
    kind: SheetKind,
    grid: Grid,
}

enum SheetKind {
    Worksheet,
    // Chart, Macro, Other – placeholders for future types
}

Grid {
    nrows: u32,      // logical used range height
    ncols: u32,      // logical used range width
    rows: Vec<Row>,  // length == nrows
}

Row {
    index: u32,      // 0-based row index within grid
    cells: Vec<Cell> // length == ncols
}

Cell {
    row: u32,              // 0-based row index
    col: u32,              // 0-based column index
    address: CellAddress,  // A1-style identity
    value: Option<CellValue>,
    formula: Option<String>,   // raw formula text, no AST yet
    // format summary can be added later as needed
}

enum CellValue {
    Number(f64),
    Text(String),
    Bool(bool),
    // Optional placeholders for Error, DateTime, etc.
}

struct CellAddress {
    row: u32,   // 0-based
    col: u32,   // 0-based
    // A1 conversion implemented via helpers
}
```

These shapes mirror the technical blueprint’s workbook-level types, just omitting M/data-model pieces for now. 

#### 2.2 Excel container parsing and open function

New public API (exact naming can be tuned, but keep the shape):

```text
pub fn open_workbook(path: impl AsRef<Path>) -> Result<Workbook, ExcelOpenError>
```

New error type:

```text
enum ExcelOpenError {
    Io(std::io::Error),
    NotZipContainer,
    NotExcelOpenXml,          // ZIP but missing [Content_Types].xml or core parts
    WorkbookXmlMissing,       // workbook.xml missing or invalid
    WorksheetXmlMissing { sheet_name: String },
    XmlParseError(String),
    // leave room for DataMashup-specific variants later
}
```

Responsibilities:

1. Open the file path, surface `Io` errors cleanly.
2. Detect that it is a ZIP container (e.g., via a `zip` crate) and return `NotZipContainer` if not.
3. Validate it is an Excel Open XML workbook:

   * `[Content_Types].xml` exists.
   * `xl/workbook.xml` exists.
4. Parse `xl/workbook.xml` to discover sheet names and target worksheet part names.
5. For each worksheet:

   * Read the `xl/worksheets/sheetN.xml` part.
   * Walk `<sheetData>` rows and cells to reconstruct the used range:

     * Use `<dimension ref="A1:C3">` if present as a hint.
     * Fall back to scanning rows and cells to compute max row/column indices.
   * Populate `Grid`, `Row`, and `Cell` struct instances, with:

     * `nrows` / `ncols` matching the used range (PG1 semantics).
     * Cells at each (row, col) position (0-based) with:

       * `address` set by A1 helper (see PG2).
       * `value` set from `<v>` (convert numeric/text/bool).
       * `formula` set from `<f>` (raw text).

Sheets without any used cells should get `grid.nrows == 0` and `grid.ncols == 0` and `rows.is_empty() == true`. 

xlsx-only is sufficient for this cycle; `.xlsm`/`.xlsb` can either be unsupported (return `NotExcelOpenXml`) or trivially supported if the container layout is identical.

#### 2.3 Addressing helpers (PG2)

Implement A1 address conversions consistent with Excel’s convention:

* Columns: 0 → `A`, 25 → `Z`, 26 → `AA`, 27 → `AB`, …, 51 → `AZ`, 52 → `BA`, …
* Rows: 0 → `1`, 1 → `2`, etc.

API sketch:

```text
impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> CellAddress;
    pub fn to_a1(&self) -> String;
}

pub fn index_to_address(row: u32, col: u32) -> String;
pub fn address_to_index(a1: &str) -> Option<(u32, u32)>;
```

`CellAddress::from_indices` and `index_to_address` should agree exactly, and `address_to_index` should be a true inverse for valid A1 strings used in tests.

Every `Cell` created by the parser must have `address` consistent with its `(row, col)` indices and the worksheet layout.

---

### 3. Behavioral Contract

#### 3.1 Container open and error behavior (Milestone 1.1 + 1.2)

Given the existing fixtures:

* `minimal.xlsx` (from `smoke_minimal`):

  * `open_workbook("minimal.xlsx")`:

    * Succeeds.
    * Returns a `Workbook` with:

      * Exactly 1 sheet named `"Sheet1"`.
      * That sheet has `Worksheet` kind.
      * `grid.nrows == 1`, `grid.ncols == 1`.
      * Cell at `(0,0)` with address `"A1"` and some non-empty value.

* Non-existent path:

  * `open_workbook("no_such_file.xlsx")` returns `Err(ExcelOpenError::Io(e))` where `e.kind() == ErrorKind::NotFound`.

* `random_zip.zip` (from `container_random_zip`):

  * ZIP container that is not an Excel OPC package.
  * `open_workbook` returns `Err(ExcelOpenError::NotExcelOpenXml)` (or equivalent).

* `no_content_types.xlsx` (from `container_no_content_types`):

  * Valid ZIP with workbook-ish contents but `[Content_Types].xml` removed.
  * `open_workbook` returns `Err(ExcelOpenError::NotExcelOpenXml)`.

No panics, unwraps, or generic “other error” messages are allowed for these happy/negative paths.

#### 3.2 Workbook → Sheet → Grid IR (PG1)

Fixtures:

1. `pg1_basic_two_sheets.xlsx` (`Sheet1` 3×3, `Sheet2` 5×2):

   * `Workbook.sheets.len() == 2`.
   * `sheets[0].name == "Sheet1"`, `sheets[1].name == "Sheet2"`.
   * `Sheet1.grid.nrows == 3`, `Sheet1.grid.ncols == 3`.
   * `Sheet2.grid.nrows == 5`, `Sheet2.grid.ncols == 2`.
   * Grid contents: at least one cell’s `value` matches the openpyxl-generated text pattern (`"R{r}C{c}"` for Sheet1 / `"S2_R{r}C{c}"` for Sheet2).

2. `pg1_sparse_used_range.xlsx`:

   * One sheet named `"Sparse"`.
   * IR must:

     * Include cell `(0,0)`/`"A1"`, `(1,1)`/`"B2"`, `(9,6)`/`"G10"`.
     * Set `grid.nrows` and `grid.ncols` so that all these cells are inside the used range (i.e., includes row 10 and column G).
     * Represent holes (e.g., row 5, column D) as “empty” cells according to your chosen convention:

       * Either explicit `Cell` instances with `value == None`, or no `Cell` in that position and `Row.cells.len() == ncols` with empties encoded; tests will codify whichever representation you choose.

3. `pg1_empty_and_mixed_sheets.xlsx`:

   * Sheets: `"Empty"`, `"ValuesOnly"`, `"FormulasOnly"` (in some order).

   Behavioral expectations:

   * `Empty.grid.nrows == 0`, `Empty.grid.ncols == 0`, `Empty.grid.rows.is_empty()`.
   * `ValuesOnly.grid.nrows == 10`, `ncols == 10`, many cells with `value: Some(Number(_))` and `formula == None`.
   * `FormulasOnly.grid.nrows == 10`, `ncols == 10`, cells where:

     * `formula.is_some()` (e.g., `=ValuesOnly!A1`) and
     * `value.is_some()` (cached result from file, likely `r*c`).

The IR must be deterministic: repeated parses of the same file produce structurally equal `Workbook` instances (same sheet order, same used-range extents, same addresses and values).

#### 3.3 Addressing invariants (PG2)

Fixture: `pg2_addressing_matrix.xlsx`.

* For each non-empty cell on the `Addresses` sheet:

  * The text stored in the cell is its own address string (e.g., `"A1"`, `"B2"`, `"Z10"`, `"AA1"`, `"AAA1"`).
  * The IR must satisfy:

    * `cell.address.to_a1() == cell.value.as_text().unwrap()` for all those cells.
    * If we call `address_to_index(cell.value_text)`, we get `(row, col)` matching `cell.row` and `cell.col`.
    * `index_to_address(cell.row, cell.col) == cell.address.to_a1()`.

Additionally, pure helper tests (no Excel):

* `index_to_address(0,0) == "A1"`.
* `index_to_address(0,25) == "Z1"`.
* `index_to_address(0,26) == "AA1"`.
* `index_to_address(0,27) == "AB1"`.
* `index_to_address(0,51) == "AZ1"`.
* `index_to_address(0,52) == "BA1"`.
* Round-trip for the address set:

  * `"A1","B2","Z10","AA1","AA10","AB7","AZ5","BA1","ZZ10","AAA1"`.

---

### 4. Constraints and Non-Goals

#### 4.1 Constraints

* **WASM build guard**: The shared core (IR and parsing logic) must compile for `wasm32-unknown-unknown` when run with `cargo check --target wasm32-unknown-unknown --no-default-features`. No host-only APIs should leak into the core crate’s public surface. Filesystem access should be isolated in a thin layer that can be swapped out for WASM later (e.g., path-based loader vs. in-memory byte loader). 
* **Streaming is a future concern**: It is acceptable to read the worksheet XML into memory in this cycle, as long as the code is structured so that we can later switch to streaming XML without breaking the IR or public APIs. Avoid embedding high-level DOM wrappers that prevent streaming (e.g., building a full DOM tree when a pull parser would do).
* **No Excel COM / platform-specific APIs**: The design must remain cross-platform and future-friendly for macOS and web/WASM, per the product differentiation plan. That means pure file/ZIP/XML parsing, not Excel automation.
* **Error transparency**: Errors should be explicit and typed (`ExcelOpenError`), not hidden behind generic `Box<dyn Error>`; tests will assert specific variants for key failure cases (random ZIP, missing `[Content_Types].xml`, etc.).

#### 4.2 Non-goals (for this cycle)

* No `DataMashup` or MS-QDEFF parsing (Milestones 2–5) — those will be a separate cycle.
* No diff computation (`DiffOp`, grid or DB-mode diffs, M semantic diffs) — those start in the PG4/M6/G1+ cycles.
* No DAX/data-model parsing.
* No CLI or JSON export changes beyond what tests need; CLI remains trivial for now.

---

### 5. Interfaces and Module Boundaries

#### 5.1 Public IR surface

From `core/src/lib.rs` (or equivalent):

```text
pub mod workbook;       // IR types
pub mod excel_open_xml; // container + parse

pub use workbook::{Workbook, Sheet, SheetKind, Grid, Row, Cell, CellValue, CellAddress};
pub use excel_open_xml::{open_workbook, ExcelOpenError};
```

The implementer can choose internal layout, but the above is the intended public API that later diff modules and frontends will depend on.

#### 5.2 Excel parsing internals (flexible)

Inside `excel_open_xml`:

* Keep container-opening and XML parsing logic behind private helpers:

  * `fn open_zip(path: &Path) -> Result<ZipArchive<...>, ExcelOpenError>`
  * `fn parse_workbook_xml(...) -> Result<Vec<SheetDescriptor>, ExcelOpenError>`
  * `fn parse_sheet_xml(...) -> Result<Grid, ExcelOpenError>`

This separation will make it easier to:

* Plug in alternative readers (e.g., a streaming open-xml layer).
* Test parts in isolation (unit tests for address parsing, sheet parsing from raw XML strings).

---

### 6. Test Plan for This Cycle

All tests live under `core/tests/` (integration) and/or `core/src/...` (unit). They must be deterministic and use the existing fixtures under `fixtures/generated`.

#### 6.1 Container and file-open tests (Milestone 1.1/1.2)

**New tests** (integration-level):

1. `open_minimal_workbook_succeeds`:

   * Use the existing `get_fixture_path("minimal.xlsx")`.
   * Call `open_workbook`.
   * Assert:

     * `Ok(Workbook)`.
     * Sheet count, sheet names, grid size, and cell contents as in 3.1.

2. `open_nonexistent_file_returns_io_error`:

   * Construct a path to `fixtures/generated/definitely_missing.xlsx` (no need for fixture).
   * Assert `Err(ExcelOpenError::Io(e))` with `e.kind() == NotFound`.

3. `random_zip_is_not_excel`:

   * Use `get_fixture_path("random_zip.zip")`.
   * Assert `Err(ExcelOpenError::NotExcelOpenXml)`.

4. `no_content_types_is_not_excel`:

   * Use `get_fixture_path("no_content_types.xlsx")`.
   * Assert `Err(ExcelOpenError::NotExcelOpenXml)`.

Optionally add a very small in-memory test for `NotZipContainer` using a temporary `.txt` file created in the test.

#### 6.2 PG1 IR tests

**New tests**:

1. `pg1_basic_two_sheets_structure`:

   * Open `pg1_basic_two_sheets.xlsx`.
   * Assert:

     * 2 sheets with expected names and `SheetKind::Worksheet`.
     * `Sheet1.grid.nrows == 3`, `ncols == 3`.
     * `Sheet2.grid.nrows == 5`, `ncols == 2`.

2. `pg1_sparse_used_range_extents`:

   * Open `pg1_sparse_used_range.xlsx`.
   * Find sheet `"Sparse"`.
   * Assert:

     * Contains cells at addresses `"A1"`, `"B2"`, `"G10"`.
     * `grid.nrows` and `grid.ncols` include row 10 and column G.
     * No panic / index errors when iterating the full grid.

3. `pg1_empty_and_mixed_sheets`:

   * Open `pg1_empty_and_mixed_sheets.xlsx`.
   * Identify sheets `"Empty"`, `"ValuesOnly"`, `"FormulasOnly"`.
   * Assert:

     * `Empty.grid.nrows == 0`, `ncols == 0`.
     * `ValuesOnly.grid` has 10×10 non-empty numeric values and no formulas.
     * `FormulasOnly.grid` has 10×10 formulas referencing `ValuesOnly`, with non-empty `formula` and `value` fields.

#### 6.3 PG2 addressing tests

**Unit tests** (no Excel):

1. `index_to_address_small_cases`:

   * Assert the mapping examples listed in 3.3.

2. `address_round_trip_known_addresses`:

   * For the address list used in PG2, assert:

     * `address_to_index(a1)` returns `(r, c)`.
     * `index_to_address(r, c) == a1`.

**Integration test**:

3. `pg2_addressing_matrix_consistency`:

   * Open `pg2_addressing_matrix.xlsx`.
   * For each non-empty cell:

     * Confirm its text value is equal to `cell.address.to_a1()`.
     * Confirm `address_to_index(cell.value_text)` matches `cell.row`/`cell.col`.

#### 6.4 WASM compatibility guard

Add a simple command/script and document it (and/or a CI job if CI config exists):

* Command: `cargo check --target wasm32-unknown-unknown --no-default-features` in the `core` crate.

No code changes are needed beyond ensuring dependencies used by IR and parsing modules are wasm-safe (or gated behind `cfg` if they are not). This check doesn’t need an automated test; it is a build-only gate.