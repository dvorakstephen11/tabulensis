# 2025-11-29-refactor Mini-Spec
Architectural Refactoring Based on Design Evaluation

## 1. Scope

This refactoring cycle addresses the high-priority recommendations from the unified design evaluation (`2025-11-29-unified-evaluation.md`). The focus is on resolving performance bottlenecks, unifying the diff representation, improving module structure, and hardening types for future growth.

### 1.1 In-scope modules and types

Rust modules to add or modify:

**New modules:**

- `core/src/container.rs` — Host container abstraction (ZIP I/O, OPC validation)
- `core/src/grid_parser.rs` — Semantic parsing of sheet XML into Grid IR
- `core/src/datamashup_framing.rs` — MS-QDEFF binary framing logic
- `core/src/engine.rs` — Canonical diff engine producing `DiffReport`

**Modified modules:**

- `core/src/workbook.rs` — Sparse `Grid` representation and signature fields
- `core/src/diff.rs` — `#[non_exhaustive]` on enums, constructor functions, error variant
- `core/src/excel_open_xml.rs` — Thin façade re-exporting from split modules
- `core/src/output/json.rs` — Consume `DiffReport` instead of computing diffs
- `core/src/lib.rs` — Re-exports for new modules

**New test files:**

- `core/tests/sparse_grid_tests.rs` — Sparse grid behavior
- `core/tests/engine_tests.rs` — `diff_workbooks` producing `DiffReport`
- `core/tests/signature_tests.rs` — Row/column signature computation

### 1.2 Out of scope for this cycle

- Advanced diff algorithms (Patience Diff, LCS, LAPJV) — these build on this foundation
- M Query domain layer (`Query`, `MStep`, ASTs) — requires sparse grid first
- PBIX host container support — uses same host abstraction pattern
- Database-mode diff (`D1–D7`)
- CLI or WASM integration changes beyond making new types public
- Performance benchmarking harnesses (P1/P2) — deferred to follow-up cycle

---

## 2. Sparse Grid Representation

### 2.1 Problem statement

The current `Grid` uses a dense `Vec<Vec<Cell>>` representation that allocates memory for every cell in the used range. For a sparse sheet with dimension A1:ZZ10000 but only 100 populated cells, this allocates ~6.8 million empty `Cell` structs (702 columns × 10000 rows). Each `Cell` is ~100+ bytes, causing gigabytes of allocation for large sparse files.

This violates the performance goal: "instant diff on 100MB files."

### 2.2 New `Grid` structure

Replace the dense representation with a sparse hash-based structure:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub nrows: u32,
    pub ncols: u32,
    pub cells: HashMap<(u32, u32), Cell>,
    pub row_signatures: Option<Vec<RowSignature>>,
    pub col_signatures: Option<Vec<ColSignature>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}
```

### 2.3 Removed types

Remove the `Row` struct entirely:

```rust
// REMOVE:
// pub struct Row {
//     pub index: u32,
//     pub cells: Vec<Cell>,
// }
```

The `Row` abstraction is not needed for sparse grids. Iteration patterns will change from `grid.rows[r].cells[c]` to `grid.get(r, c)`.

### 2.4 Grid API

Add accessor methods to `Grid`:

```rust
impl Grid {
    pub fn new(nrows: u32, ncols: u32) -> Grid {
        Grid {
            nrows,
            ncols,
            cells: HashMap::new(),
            row_signatures: None,
            col_signatures: None,
        }
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells.get(&(row, col))
    }

    pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut Cell> {
        self.cells.get_mut(&(row, col))
    }

    pub fn insert(&mut self, cell: Cell) {
        self.cells.insert((cell.row, cell.col), cell);
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = &Cell> {
        self.cells.values()
    }

    pub fn rows_iter(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.nrows)
    }

    pub fn cols_iter(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.ncols)
    }
}
```

### 2.5 Signature types

Move signature types from `diff.rs` to `workbook.rs` and add computation methods:

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowSignature {
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ColSignature {
    pub hash: u64,
}

impl Grid {
    pub fn compute_row_signature(&self, row: u32) -> RowSignature {
        let mut hasher = DefaultHasher::new();
        for col in 0..self.ncols {
            if let Some(cell) = self.get(row, col) {
                cell.value.hash(&mut hasher);
                cell.formula.hash(&mut hasher);
            }
        }
        RowSignature { hash: hasher.finish() }
    }

    pub fn compute_col_signature(&self, col: u32) -> ColSignature {
        let mut hasher = DefaultHasher::new();
        for row in 0..self.nrows {
            if let Some(cell) = self.get(row, col) {
                cell.value.hash(&mut hasher);
                cell.formula.hash(&mut hasher);
            }
        }
        ColSignature { hash: hasher.finish() }
    }

    pub fn compute_all_signatures(&mut self) {
        let mut row_sigs = Vec::with_capacity(self.nrows as usize);
        for row in 0..self.nrows {
            row_sigs.push(self.compute_row_signature(row));
        }
        self.row_signatures = Some(row_sigs);

        let mut col_sigs = Vec::with_capacity(self.ncols as usize);
        for col in 0..self.ncols {
            col_sigs.push(self.compute_col_signature(col));
        }
        self.col_signatures = Some(col_sigs);
    }
}
```

For `CellValue` to be hashable, add `Hash` derive:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Number(OrderedFloat<f64>),  // Use ordered_float crate for Hash
    Text(String),
    Bool(bool),
}

impl Hash for CellValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CellValue::Number(n) => {
                0u8.hash(state);
                n.to_bits().hash(state);
            }
            CellValue::Text(s) => {
                1u8.hash(state);
                s.hash(state);
            }
            CellValue::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
        }
    }
}
```

Alternative without `ordered_float` (simpler, recommended):

```rust
impl Hash for CellValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CellValue::Number(n) => {
                0u8.hash(state);
                n.to_bits().hash(state);
            }
            CellValue::Text(s) => {
                1u8.hash(state);
                s.hash(state);
            }
            CellValue::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
        }
    }
}
```

### 2.6 Parser changes (`build_grid`)

Update the grid builder to produce sparse grids:

```rust
fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, ExcelOpenError> {
    let mut grid = Grid::new(nrows, ncols);

    for parsed in cells {
        let cell = Cell {
            row: parsed.row,
            col: parsed.col,
            address: CellAddress::from_indices(parsed.row, parsed.col),
            value: parsed.value,
            formula: parsed.formula,
        };
        grid.insert(cell);
    }

    Ok(grid)
}
```

### 2.7 Test updates

Update all tests that access `grid.rows[r].cells[c]` to use `grid.get(r, c)`:

**Before:**
```rust
let cell = &sheet.grid.rows[0].cells[0];
```

**After:**
```rust
let cell = sheet.grid.get(0, 0).expect("cell should exist");
```

For iteration over all cells:

**Before:**
```rust
for row in &sheet.grid.rows {
    for cell in &row.cells {
        // ...
    }
}
```

**After:**
```rust
for cell in sheet.grid.iter_cells() {
    // ...
}
```

---

## 3. Canonical Diff Engine

### 3.1 Problem statement

The current diff implementation in `output/json.rs` produces `Vec<CellDiff>` directly, bypassing the `DiffOp`/`DiffReport` IR. This creates parallel representations and prevents future diff algorithms from sharing infrastructure.

### 3.2 New engine module

Create `core/src/engine.rs`:

```rust
use crate::diff::{DiffOp, DiffReport, SheetId};
use crate::workbook::{CellAddress, CellSnapshot, CellValue, Grid, Sheet, Workbook};
use std::collections::HashMap;

pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport {
    let mut ops = Vec::new();

    let old_sheets: HashMap<&str, &Sheet> = old.sheets.iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    let new_sheets: HashMap<&str, &Sheet> = new.sheets.iter()
        .map(|s| (s.name.as_str(), s))
        .collect();

    let mut all_names: Vec<&str> = old_sheets.keys()
        .chain(new_sheets.keys())
        .copied()
        .collect();
    all_names.sort_unstable();
    all_names.dedup();

    for name in all_names {
        let sheet_id: SheetId = name.to_string();

        match (old_sheets.get(name), new_sheets.get(name)) {
            (None, Some(_)) => {
                ops.push(DiffOp::SheetAdded { sheet: sheet_id });
            }
            (Some(_), None) => {
                ops.push(DiffOp::SheetRemoved { sheet: sheet_id });
            }
            (Some(old_sheet), Some(new_sheet)) => {
                diff_grids(&sheet_id, &old_sheet.grid, &new_sheet.grid, &mut ops);
            }
            (None, None) => unreachable!(),
        }
    }

    DiffReport::new(ops)
}

fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    let max_rows = old.nrows.max(new.nrows);
    let max_cols = old.ncols.max(new.ncols);

    for row in 0..max_rows {
        for col in 0..max_cols {
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            let old_snapshot = old_cell.map(CellSnapshot::from_cell);
            let new_snapshot = new_cell.map(CellSnapshot::from_cell);

            if old_snapshot != new_snapshot {
                let addr = CellAddress::from_indices(row, col);
                let from = old_snapshot.unwrap_or_else(|| CellSnapshot::empty(addr));
                let to = new_snapshot.unwrap_or_else(|| CellSnapshot::empty(addr));

                ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
            }
        }
    }
}
```

### 3.3 CellSnapshot helper

Add to `workbook.rs`:

```rust
impl CellSnapshot {
    pub fn empty(addr: CellAddress) -> CellSnapshot {
        CellSnapshot {
            addr,
            value: None,
            formula: None,
        }
    }
}
```

### 3.4 DiffOp constructors

Add constructor functions to `diff.rs` that enforce invariants:

```rust
impl DiffOp {
    pub fn cell_edited(
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    ) -> DiffOp {
        debug_assert_eq!(from.addr, addr, "from.addr must match canonical addr");
        debug_assert_eq!(to.addr, addr, "to.addr must match canonical addr");
        DiffOp::CellEdited { sheet, addr, from, to }
    }

    pub fn row_added(
        sheet: SheetId,
        row_idx: u32,
        row_signature: Option<RowSignature>,
    ) -> DiffOp {
        DiffOp::RowAdded { sheet, row_idx, row_signature }
    }

    pub fn row_removed(
        sheet: SheetId,
        row_idx: u32,
        row_signature: Option<RowSignature>,
    ) -> DiffOp {
        DiffOp::RowRemoved { sheet, row_idx, row_signature }
    }

    pub fn column_added(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnAdded { sheet, col_idx, col_signature }
    }

    pub fn column_removed(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnRemoved { sheet, col_idx, col_signature }
    }

    pub fn block_moved_rows(
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        }
    }

    pub fn block_moved_columns(
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        }
    }
}
```

### 3.5 Refactor `output/json.rs`

Update the JSON output module to consume `DiffReport`:

```rust
use crate::diff::DiffReport;
#[cfg(feature = "excel-open-xml")]
use crate::engine::diff_workbooks as compute_diff;
#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{ExcelOpenError, open_workbook};
use serde::Serialize;
#[cfg(feature = "excel-open-xml")]
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellDiff {
    #[serde(rename = "coords")]
    pub coords: String,
    #[serde(rename = "value_file1")]
    pub value_file1: Option<String>,
    #[serde(rename = "value_file2")]
    pub value_file2: Option<String>,
}

pub fn serialize_diff_report(report: &DiffReport) -> serde_json::Result<String> {
    serde_json::to_string(report)
}

pub fn serialize_cell_diffs(diffs: &[CellDiff]) -> serde_json::Result<String> {
    serde_json::to_string(diffs)
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<DiffReport, ExcelOpenError> {
    let wb_a = open_workbook(path_a)?;
    let wb_b = open_workbook(path_b)?;
    Ok(compute_diff(&wb_a, &wb_b))
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<String, ExcelOpenError> {
    let report = diff_workbooks(path_a, path_b)?;
    serialize_diff_report(&report).map_err(|e| ExcelOpenError::SerializationError(e.to_string()))
}

pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn render_value(value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(s)) => Some(s.clone()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            None => None,
        }
    }

    report.ops.iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                Some(CellDiff {
                    coords: addr.to_a1(),
                    value_file1: render_value(&from.value),
                    value_file2: render_value(&to.value),
                })
            } else {
                None
            }
        })
        .collect()
}
```

---

## 4. Harden Enums for Future Growth

### 4.1 Mark enums `#[non_exhaustive]`

In `diff.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum DiffOp {
    // ... variants unchanged
}
```

In `excel_open_xml.rs` (or the new error module):

```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExcelOpenError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a ZIP container")]
    NotZipContainer,
    #[error("not an Excel Open XML package")]
    NotExcelOpenXml,
    #[error("workbook.xml missing or unreadable")]
    WorkbookXmlMissing,
    #[error("worksheet XML missing for sheet {sheet_name}")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("XML parse error: {0}")]
    XmlParseError(String),
    #[error("DataMashup base64 invalid")]
    DataMashupBase64Invalid,
    #[error("DataMashup unsupported version {version}")]
    DataMashupUnsupportedVersion { version: u32 },
    #[error("DataMashup framing invalid")]
    DataMashupFramingInvalid,
    #[error("serialization error: {0}")]
    SerializationError(String),
}
```

### 4.2 New error variant

Add `SerializationError` variant (shown above) for JSON serialization failures, replacing the current incorrect mapping to `XmlParseError`.

---

## 5. Refactor Monolithic Parser

### 5.1 Problem statement

`excel_open_xml.rs` is 1000+ lines conflating:
- Host Container layer (ZIP I/O, OPC validation)
- Binary Framing layer (MS-QDEFF parsing)
- Semantic Parsing layer (Grid XML interpretation)

This hinders maintainability and makes it difficult to add PBIX support.

### 5.2 Module split

Create three new modules and reduce `excel_open_xml.rs` to a façade:

#### `core/src/container.rs`

```rust
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use zip::result::ZipError;
use zip::ZipArchive;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContainerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a ZIP container")]
    NotZipContainer,
    #[error("not an OPC package (missing [Content_Types].xml)")]
    NotOpcPackage,
}

pub struct OpcContainer {
    pub(crate) archive: ZipArchive<File>,
}

impl OpcContainer {
    pub fn open(path: impl AsRef<Path>) -> Result<OpcContainer, ContainerError> {
        let file = File::open(path)?;
        let archive = ZipArchive::new(file).map_err(|err| match err {
            ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
                ContainerError::NotZipContainer
            }
            ZipError::Io(e) => ContainerError::Io(e),
            other => ContainerError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                other.to_string(),
            )),
        })?;

        let mut container = OpcContainer { archive };
        if container.read_file("[Content_Types].xml").is_err() {
            return Err(ContainerError::NotOpcPackage);
        }

        Ok(container)
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, ZipError> {
        let mut file = self.archive.by_name(name)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn read_file_optional(&mut self, name: &str) -> Result<Option<Vec<u8>>, std::io::Error> {
        match self.read_file(name) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(ZipError::FileNotFound) => Ok(None),
            Err(ZipError::Io(e)) => Err(e),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        }
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.archive.file_names()
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }
}
```

#### `core/src/grid_parser.rs`

```rust
use crate::addressing::address_to_index;
use crate::workbook::{Cell, CellAddress, CellValue, Grid, Sheet, SheetKind};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GridParseError {
    #[error("XML parse error: {0}")]
    XmlError(String),
    #[error("invalid cell address: {0}")]
    InvalidAddress(String),
    #[error("shared string index {0} out of bounds")]
    SharedStringOutOfBounds(usize),
}

pub fn parse_shared_strings(xml: &[u8]) -> Result<Vec<String>, GridParseError> {
    // ... existing logic from excel_open_xml.rs
}

pub fn parse_workbook_xml(xml: &[u8]) -> Result<Vec<SheetDescriptor>, GridParseError> {
    // ... existing logic from excel_open_xml.rs
}

pub fn parse_relationships(xml: &[u8]) -> Result<HashMap<String, String>, GridParseError> {
    // ... existing logic from excel_open_xml.rs
}

pub fn parse_sheet_xml(xml: &[u8], shared_strings: &[String]) -> Result<Grid, GridParseError> {
    // ... existing logic, updated for sparse Grid
}

pub struct SheetDescriptor {
    pub name: String,
    pub rel_id: Option<String>,
    pub sheet_id: Option<u32>,
}

pub fn resolve_sheet_target(
    sheet: &SheetDescriptor,
    relationships: &HashMap<String, String>,
    index: usize,
) -> String {
    // ... existing logic from excel_open_xml.rs
}
```

#### `core/src/datamashup_framing.rs`

```rust
use quick_xml::events::Event;
use quick_xml::Reader;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DataMashupError {
    #[error("base64 decoding failed")]
    Base64Invalid,
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u32),
    #[error("invalid framing structure")]
    FramingInvalid,
    #[error("XML parse error: {0}")]
    XmlError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDataMashup {
    pub version: u32,
    pub package_parts: Vec<u8>,
    pub permissions: Vec<u8>,
    pub metadata: Vec<u8>,
    pub permission_bindings: Vec<u8>,
}

pub fn parse_data_mashup(bytes: &[u8]) -> Result<RawDataMashup, DataMashupError> {
    // ... existing logic from excel_open_xml.rs
}

pub fn read_datamashup_text(xml: &[u8]) -> Result<Option<String>, DataMashupError> {
    // ... existing logic from excel_open_xml.rs
}

pub fn decode_datamashup_base64(text: &str) -> Result<Vec<u8>, DataMashupError> {
    // ... existing logic from excel_open_xml.rs
}

pub(crate) fn decode_datamashup_xml(xml: &[u8]) -> Result<Option<Vec<u8>>, DataMashupError> {
    // ... existing logic from excel_open_xml.rs
}

// ... remaining helper functions from excel_open_xml.rs
```

#### `core/src/excel_open_xml.rs` (reduced façade)

```rust
use crate::container::{ContainerError, OpcContainer};
use crate::datamashup_framing::{
    DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup, read_datamashup_text,
};
use crate::grid_parser::{
    GridParseError, parse_relationships, parse_shared_strings, parse_sheet_xml,
    parse_workbook_xml, resolve_sheet_target,
};
use crate::workbook::{Sheet, SheetKind, Workbook};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExcelOpenError {
    #[error("container error: {0}")]
    Container(#[from] ContainerError),
    #[error("grid parse error: {0}")]
    GridParse(#[from] GridParseError),
    #[error("DataMashup error: {0}")]
    DataMashup(#[from] DataMashupError),
    #[error("workbook.xml missing or unreadable")]
    WorkbookXmlMissing,
    #[error("worksheet XML missing for sheet {sheet_name}")]
    WorksheetXmlMissing { sheet_name: String },
    #[error("serialization error: {0}")]
    SerializationError(String),
}

pub fn open_workbook(path: impl AsRef<Path>) -> Result<Workbook, ExcelOpenError> {
    let mut container = OpcContainer::open(path.as_ref())?;

    let shared_strings = match container.read_file_optional("xl/sharedStrings.xml")
        .map_err(|e| ContainerError::Io(e))?
    {
        Some(bytes) => parse_shared_strings(&bytes)?,
        None => Vec::new(),
    };

    let workbook_bytes = container.read_file("xl/workbook.xml")
        .map_err(|_| ExcelOpenError::WorkbookXmlMissing)?;

    let sheets = parse_workbook_xml(&workbook_bytes)?;

    let relationships = match container.read_file_optional("xl/_rels/workbook.xml.rels")
        .map_err(|e| ContainerError::Io(e))?
    {
        Some(bytes) => parse_relationships(&bytes)?,
        None => std::collections::HashMap::new(),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes = container.read_file(&target)
            .map_err(|_| ExcelOpenError::WorksheetXmlMissing {
                sheet_name: sheet.name.clone(),
            })?;
        let grid = parse_sheet_xml(&sheet_bytes, &shared_strings)?;
        sheet_ir.push(Sheet {
            name: sheet.name.clone(),
            kind: SheetKind::Worksheet,
            grid,
        });
    }

    Ok(Workbook { sheets: sheet_ir })
}

pub fn open_data_mashup(path: impl AsRef<Path>) -> Result<Option<RawDataMashup>, ExcelOpenError> {
    let mut container = OpcContainer::open(path.as_ref())?;

    for i in 0..container.len() {
        let name = {
            let file = container.archive.by_index(i).ok();
            file.map(|f| f.name().to_string())
        };

        if let Some(name) = name {
            if !name.starts_with("customXml/") || !name.ends_with(".xml") {
                continue;
            }

            let bytes = container.read_file(&name)
                .map_err(|e| ContainerError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))?;

            if let Some(text) = read_datamashup_text(&bytes)? {
                let decoded = decode_datamashup_base64(&text)?;
                return parse_data_mashup(&decoded).map(Some);
            }
        }
    }

    Ok(None)
}

pub use crate::datamashup_framing::RawDataMashup;
```

### 5.3 Module exports in `lib.rs`

```rust
pub mod addressing;
pub mod container;
pub mod datamashup_framing;
pub mod diff;
pub mod engine;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod grid_parser;
pub mod output;
pub mod workbook;

pub use addressing::{address_to_index, index_to_address};
pub use container::{ContainerError, OpcContainer};
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use diff::{ColSignature, DiffOp, DiffReport, RowSignature, SheetId};
pub use engine::diff_workbooks;
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, open_data_mashup, open_workbook};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
#[cfg(feature = "excel-open-xml")]
pub use output::json::{diff_workbooks_to_json};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, Grid, Sheet, SheetKind, Workbook,
};
```

---

## 6. Test Specifications

### 6.1 Sparse grid tests (`core/tests/sparse_grid_tests.rs`)

```rust
#[test]
fn sparse_grid_empty_has_zero_cells() {
    let grid = Grid::new(1000, 1000);
    assert_eq!(grid.cell_count(), 0);
    assert!(grid.is_empty());
    assert_eq!(grid.nrows, 1000);
    assert_eq!(grid.ncols, 1000);
}

#[test]
fn sparse_grid_insert_and_retrieve() {
    let mut grid = Grid::new(100, 100);
    let cell = Cell {
        row: 50,
        col: 50,
        address: CellAddress::from_indices(50, 50),
        value: Some(CellValue::Number(42.0)),
        formula: None,
    };
    grid.insert(cell);
    assert_eq!(grid.cell_count(), 1);
    let retrieved = grid.get(50, 50).expect("cell should exist");
    assert_eq!(retrieved.value, Some(CellValue::Number(42.0)));
    assert!(grid.get(0, 0).is_none());
}

#[test]
fn sparse_grid_iter_cells_only_populated() {
    let mut grid = Grid::new(1000, 1000);
    for i in 0..10 {
        let cell = Cell {
            row: i * 100,
            col: i * 100,
            address: CellAddress::from_indices(i * 100, i * 100),
            value: Some(CellValue::Number(i as f64)),
            formula: None,
        };
        grid.insert(cell);
    }
    let cells: Vec<_> = grid.iter_cells().collect();
    assert_eq!(cells.len(), 10);
}

#[test]
fn sparse_grid_memory_efficiency() {
    let grid = Grid::new(10_000, 1_000);
    assert!(std::mem::size_of_val(&grid) < 1024);
}
```

### 6.2 Engine tests (`core/tests/engine_tests.rs`)

```rust
use excel_diff::{diff_workbooks, DiffOp, DiffReport, Workbook, Sheet, SheetKind, Grid, Cell, CellAddress, CellValue};

fn make_workbook(sheets: Vec<(&str, Vec<(u32, u32, f64)>)>) -> Workbook {
    let sheet_ir: Vec<Sheet> = sheets.into_iter().map(|(name, cells)| {
        let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
        let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
        let mut grid = Grid::new(max_row + 1, max_col + 1);
        for (r, c, val) in cells {
            grid.insert(Cell {
                row: r,
                col: c,
                address: CellAddress::from_indices(r, c),
                value: Some(CellValue::Number(val)),
                formula: None,
            });
        }
        Sheet { name: name.to_string(), kind: SheetKind::Worksheet, grid }
    }).collect();
    Workbook { sheets: sheet_ir }
}

#[test]
fn identical_workbooks_produce_empty_report() {
    let wb = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&wb, &wb);
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_added_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let report = diff_workbooks(&old, &new);
    assert!(report.ops.iter().any(|op| matches!(op, DiffOp::SheetAdded { sheet } if sheet == "Sheet2")));
}

#[test]
fn sheet_removed_detected() {
    let old = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&old, &new);
    assert!(report.ops.iter().any(|op| matches!(op, DiffOp::SheetRemoved { sheet } if sheet == "Sheet2")));
}

#[test]
fn cell_edited_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);
    match &report.ops[0] {
        DiffOp::CellEdited { sheet, addr, from, to } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        _ => panic!("expected CellEdited"),
    }
}

#[test]
fn diff_report_json_round_trips() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new);
    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed);
}
```

### 6.3 Signature tests (`core/tests/signature_tests.rs`)

```rust
use excel_diff::{Grid, Cell, CellAddress, CellValue, RowSignature, ColSignature};

#[test]
fn identical_rows_have_same_signature() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        let cell = Cell {
            row: 0,
            col: c,
            address: CellAddress::from_indices(0, c),
            value: Some(CellValue::Number(c as f64)),
            formula: None,
        };
        grid1.insert(cell.clone());
        grid2.insert(cell);
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_eq!(sig1, sig2);
}

#[test]
fn different_rows_have_different_signatures() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        grid1.insert(Cell {
            row: 0,
            col: c,
            address: CellAddress::from_indices(0, c),
            value: Some(CellValue::Number(c as f64)),
            formula: None,
        });
        grid2.insert(Cell {
            row: 0,
            col: c,
            address: CellAddress::from_indices(0, c),
            value: Some(CellValue::Number((c + 1) as f64)),
            formula: None,
        });
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1, sig2);
}

#[test]
fn compute_all_signatures_populates_fields() {
    let mut grid = Grid::new(5, 5);
    grid.insert(Cell {
        row: 2,
        col: 2,
        address: CellAddress::from_indices(2, 2),
        value: Some(CellValue::Text("center".into())),
        formula: None,
    });
    assert!(grid.row_signatures.is_none());
    assert!(grid.col_signatures.is_none());
    grid.compute_all_signatures();
    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert_eq!(grid.row_signatures.as_ref().unwrap().len(), 5);
    assert_eq!(grid.col_signatures.as_ref().unwrap().len(), 5);
}
```

---

## 7. Migration Path

### 7.1 Backward compatibility

The refactoring changes the `Grid` structure from:
- `grid.rows[r].cells[c]` → `grid.get(r, c)`

This is a breaking change for any code that directly accesses the `rows` field. However, since this is an internal refactoring and there are no external consumers yet, this is acceptable.

### 7.2 Test migration checklist

Files that need updating for the sparse grid API:

- [ ] `core/tests/pg1_ir_tests.rs` — Update grid access patterns
- [ ] `core/tests/pg2_addressing_tests.rs` — Update cell retrieval
- [ ] `core/tests/pg3_snapshot_tests.rs` — Update cell access
- [ ] `core/tests/output_tests.rs` — Update workbook construction
- [ ] `core/tests/excel_open_xml_tests.rs` — Update grid assertions
- [ ] `core/tests/integration_test.rs` — Update if present

### 7.3 Deprecation

Mark any temporary compatibility shims:

```rust
impl Grid {
    #[deprecated(note = "Use grid.get(row, col) instead")]
    pub fn rows_compat(&self) -> Vec<RowCompat> {
        // Temporary compatibility layer
    }
}
```

---

## 8. Performance Considerations

### 8.1 Memory reduction

| Scenario | Dense (current) | Sparse (new) |
|----------|-----------------|--------------|
| 1000x1000 grid, 100 cells | ~100MB | ~10KB |
| 10000x100 grid, 1000 cells | ~100MB | ~100KB |
| 100x100 grid, 10000 cells | ~1MB | ~1MB |

### 8.2 Lookup performance

- Dense: O(1) lookup via `rows[r].cells[c]`
- Sparse: O(1) average lookup via `HashMap::get`

For iteration, sparse grids are faster when sparsity > 50% because we only visit populated cells.

### 8.3 Signature computation

Computing all signatures currently walks the full `nrows x ncols` used range, so complexity is effectively O(R * C) even on sparse sheets. The O(populated_cells) claim is aspirational and will require a future H1 perf pass that aggregates only populated cells. Signatures are computed lazily (only when needed) or eagerly via `compute_all_signatures()`.

---

## 9. Acceptance Criteria

### 9.1 Core functionality

- [ ] `Grid` uses sparse `HashMap<(u32, u32), Cell>` storage
- [ ] `Row` struct is removed
- [ ] `Grid::get(row, col)` returns `Option<&Cell>`
- [ ] `Grid::insert(cell)` adds cells to the map
- [ ] `Grid::iter_cells()` iterates only populated cells
- [ ] `CellValue` implements `Hash`
- [ ] `RowSignature` and `ColSignature` have computation methods on `Grid`

### 9.2 Diff engine

- [ ] `engine::diff_workbooks(&Workbook, &Workbook) -> DiffReport` exists
- [ ] Diff engine detects `SheetAdded`, `SheetRemoved`, `CellEdited`
- [ ] `output::json::diff_workbooks_to_json` uses `DiffReport` internally
- [ ] `DiffOp::cell_edited()` constructor enforces address invariants

### 9.3 Type hardening

- [ ] `DiffOp` is marked `#[non_exhaustive]`
- [ ] `ExcelOpenError` is marked `#[non_exhaustive]`
- [ ] `ExcelOpenError::SerializationError` variant exists

### 9.4 Module structure

- [ ] `container.rs` exists with `OpcContainer` type
- [ ] `grid_parser.rs` exists with sheet parsing logic
- [ ] `datamashup_framing.rs` exists with QDEFF logic
- [ ] `excel_open_xml.rs` is reduced to <200 lines

### 9.5 Tests

- [ ] All existing tests pass after migration
- [ ] New sparse grid tests pass
- [ ] New engine tests pass
- [ ] New signature tests pass

---

## 10. Future Work (Out of Scope)

Items deferred to future cycles:

1. **Advanced Diff Algorithms (H1)**: Patience Diff, LCS, Myers/Histogram, LAPJV for row/column alignment
2. **M Query Domain Layer (H4)**: `Query`, `MStep`, M AST structures
3. **PBIX Host Container**: New `pbix.rs` module using `OpcContainer`
4. **Performance Benchmarks (P1/P2)**: Harness for measuring parse/diff time on large files
5. **Streaming I/O**: Parse directly from ZIP stream without loading entire parts into memory
6. **Shared String Interning**: Reduce memory by keeping string table references instead of cloning
7. **Formula Interning**: Deduplicate identical formulas across cells
