//! Workbook, sheet, and grid data structures.
//!
//! This module defines the core intermediate representation (IR) for Excel workbooks:
//! - [`Workbook`]: A collection of sheets
//! - [`Sheet`]: A named sheet with a grid of cells
//! - [`Grid`]: A sparse 2D grid of cell content with optional row/column signatures
//! - [`CellContent`]: Value + formula for a single cell (coordinates stored in the grid key)

use crate::addressing::{address_to_index, index_to_address, AddressParseError};
use crate::hashing::normalize_float_for_hash;
use crate::string_pool::{StringId, StringPool};
use rustc_hash::FxHashMap;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// A snapshot of a cell's logical content for comparison purposes.
///
/// Used in [`crate::diff::DiffOp::CellEdited`] to represent the "before" and "after" states.
/// Equality comparison intentionally ignores `addr` and compares only `(value, formula)`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CellSnapshot {
    pub addr: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<StringId>,
}

impl CellSnapshot {
    pub fn from_cell(row: u32, col: u32, cell: &CellContent) -> CellSnapshot {
        CellSnapshot {
            addr: CellAddress::from_indices(row, col),
            value: cell.value.clone(),
            formula: cell.formula,
        }
    }

    pub fn empty(addr: CellAddress) -> CellSnapshot {
        CellSnapshot {
            addr,
            value: None,
            formula: None,
        }
    }
}

/// An Excel workbook containing one or more sheets.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
    pub named_ranges: Vec<NamedRange>,
    pub charts: Vec<ChartObject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedRange {
    pub name: StringId,
    pub refers_to: StringId,
    pub scope: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartInfo {
    pub name: StringId,
    pub chart_type: StringId,
    pub data_range: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartObject {
    pub sheet: StringId,
    /// Optional workbook-internal sheet id for rename-safe matching.
    pub workbook_sheet_id: Option<u32>,
    pub info: ChartInfo,
    pub xml_hash: u128,
}

/// A single sheet within a workbook.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The display name of the sheet (e.g., "Sheet1", "Data").
    pub name: StringId,
    /// Optional workbook-internal sheet id for rename-safe matching.
    pub workbook_sheet_id: Option<u32>,
    /// The type of sheet (worksheet, chart, macro, etc.).
    pub kind: SheetKind,
    /// The grid of cell data.
    pub grid: Grid,
}

/// The type of an Excel sheet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SheetKind {
    Worksheet,
    Chart,
    Macro,
    Other,
}

/// A 2D grid of cells representing sheet data, stored as sparse or dense data.
///
/// # Invariants
///
/// All cells stored in `cells` must satisfy `row < nrows` and `col < ncols`.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// Number of rows in the grid's bounding rectangle.
    pub nrows: u32,
    /// Number of columns in the grid's bounding rectangle.
    pub ncols: u32,
    /// Cell storage (sparse or dense).
    pub cells: GridStorage,
    /// Optional precomputed row signatures for alignment.
    pub row_signatures: Option<Vec<RowSignature>>,
    /// Optional precomputed column signatures for alignment.
    pub col_signatures: Option<Vec<ColSignature>>,
}

/// Storage backend for grid cells.
#[derive(Debug, Clone, PartialEq)]
pub enum GridStorage {
    Sparse(FxHashMap<(u32, u32), CellContent>),
    Dense(DenseGrid),
}

/// Dense row-major storage for grid cells.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseGrid {
    ncols: u32,
    cells: Vec<CellContent>,
    non_empty: usize,
}

/// A single cell's logical content (coordinates live in the `Grid` key).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellContent {
    /// The cell's value, if any.
    pub value: Option<CellValue>,
    /// The cell's formula text (without leading '='), if any.
    pub formula: Option<StringId>,
}

pub type Cell = CellContent;

/// A view of a cell's content together with its coordinates.
#[derive(Debug, Clone, Copy)]
pub struct CellRef<'a> {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: &'a Option<CellValue>,
    pub formula: &'a Option<StringId>,
}

fn cell_is_non_empty(cell: &CellContent) -> bool {
    cell.value.is_some() || cell.formula.is_some()
}

impl DenseGrid {
    fn new(nrows: u32, ncols: u32) -> Self {
        let size = nrows.saturating_mul(ncols) as usize;
        Self {
            ncols,
            cells: vec![CellContent::default(); size],
            non_empty: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.cells.capacity()
    }

    fn index(&self, row: u32, col: u32) -> Option<usize> {
        if self.ncols == 0 {
            return None;
        }
        let idx = row.saturating_mul(self.ncols).saturating_add(col) as usize;
        self.cells.get(idx).map(|_| idx)
    }

    fn get(&self, row: u32, col: u32) -> Option<&CellContent> {
        let idx = self.index(row, col)?;
        let cell = &self.cells[idx];
        cell_is_non_empty(cell).then_some(cell)
    }

    fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
        let idx = self.index(row, col)?;
        if !cell_is_non_empty(&self.cells[idx]) {
            return None;
        }
        Some(&mut self.cells[idx])
    }

    fn set(&mut self, row: u32, col: u32, value: Option<CellValue>, formula: Option<StringId>) {
        if let Some(idx) = self.index(row, col) {
            let was_non_empty = cell_is_non_empty(&self.cells[idx]);
            let new_cell = CellContent { value, formula };
            let is_non_empty = cell_is_non_empty(&new_cell);
            if !was_non_empty && is_non_empty {
                self.non_empty = self.non_empty.saturating_add(1);
            } else if was_non_empty && !is_non_empty {
                self.non_empty = self.non_empty.saturating_sub(1);
            }
            self.cells[idx] = new_cell;
        }
    }

    fn len(&self) -> usize {
        self.non_empty
    }

    fn iter(&self) -> DenseCellIter<'_> {
        DenseCellIter {
            cells: &self.cells,
            idx: 0,
            row: 0,
            col: 0,
            ncols: self.ncols,
        }
    }
}

impl GridStorage {
    fn new_sparse() -> Self {
        GridStorage::Sparse(FxHashMap::default())
    }

    fn new_dense(nrows: u32, ncols: u32) -> Self {
        GridStorage::Dense(DenseGrid::new(nrows, ncols))
    }

    pub fn len(&self) -> usize {
        match self {
            GridStorage::Sparse(map) => map.len(),
            GridStorage::Dense(grid) => grid.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&CellContent> {
        match self {
            GridStorage::Sparse(map) => map.get(&(row, col)),
            GridStorage::Dense(grid) => grid.get(row, col),
        }
    }

    pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
        match self {
            GridStorage::Sparse(map) => map.get_mut(&(row, col)),
            GridStorage::Dense(grid) => grid.get_mut(row, col),
        }
    }

    pub fn insert(&mut self, row: u32, col: u32, cell: CellContent) {
        if !cell_is_non_empty(&cell) {
            match self {
                GridStorage::Sparse(map) => {
                    map.remove(&(row, col));
                }
                GridStorage::Dense(grid) => {
                    grid.set(row, col, None, None);
                }
            }
            return;
        }
        match self {
            GridStorage::Sparse(map) => {
                map.insert((row, col), cell);
            }
            GridStorage::Dense(grid) => {
                grid.set(row, col, cell.value, cell.formula);
            }
        }
    }

    pub fn iter(&self) -> GridCellIter<'_> {
        match self {
            GridStorage::Sparse(map) => GridCellIter::Sparse(map.iter()),
            GridStorage::Dense(grid) => GridCellIter::Dense(grid.iter()),
        }
    }
}

pub enum GridCellIter<'a> {
    Sparse(std::collections::hash_map::Iter<'a, (u32, u32), CellContent>),
    Dense(DenseCellIter<'a>),
}

impl<'a> Iterator for GridCellIter<'a> {
    type Item = ((u32, u32), &'a CellContent);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            GridCellIter::Sparse(iter) => iter.next().map(|(coord, cell)| (*coord, cell)),
            GridCellIter::Dense(iter) => iter.next(),
        }
    }
}

pub struct DenseCellIter<'a> {
    cells: &'a [CellContent],
    idx: usize,
    row: u32,
    col: u32,
    ncols: u32,
}

impl<'a> Iterator for DenseCellIter<'a> {
    type Item = ((u32, u32), &'a CellContent);

    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < self.cells.len() {
            let idx = self.idx;
            self.idx += 1;

            let row = self.row;
            let col = self.col;

            if self.ncols != 0 {
                self.col += 1;
                if self.col == self.ncols {
                    self.col = 0;
                    self.row += 1;
                }
            }

            let cell = &self.cells[idx];
            if !cell_is_non_empty(cell) {
                continue;
            }
            return Some(((row, col), cell));
        }
        None
    }
}

/// A cell address representing a position in a grid.
///
/// Can be parsed from A1-style strings (e.g., "B2", "AA10") and converted back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    /// Zero-based row index.
    pub row: u32,
    /// Zero-based column index.
    pub col: u32,
}

impl CellAddress {
    pub fn from_indices(row: u32, col: u32) -> CellAddress {
        CellAddress { row, col }
    }

    pub fn from_coords(row: u32, col: u32) -> CellAddress {
        Self::from_indices(row, col)
    }

    pub fn to_a1(&self) -> String {
        index_to_address(self.row, self.col)
    }
}

impl std::str::FromStr for CellAddress {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (row, col) = address_to_index(s).ok_or_else(|| AddressParseError {
            input: s.to_string(),
        })?;
        Ok(CellAddress { row, col })
    }
}

impl std::fmt::Display for CellAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

impl Serialize for CellAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_a1())
    }
}

impl<'de> Deserialize<'de> for CellAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let a1 = String::deserialize(deserializer)?;
        CellAddress::from_str(&a1).map_err(|e| DeError::custom(e.to_string()))
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Blank,
    Number(f64),
    Text(StringId),
    Bool(bool),
    Error(StringId),
}

impl PartialEq for CellValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellValue::Blank, CellValue::Blank) => true,
            (CellValue::Number(a), CellValue::Number(b)) => {
                if a.to_bits() == b.to_bits() {
                    return true;
                }
                if a.is_nan() && b.is_nan() {
                    return true;
                }
                if *a == 0.0 && *b == 0.0 {
                    return true;
                }
                normalize_float_for_hash(*a) == normalize_float_for_hash(*b)
            }
            (CellValue::Text(a), CellValue::Text(b)) => a == b,
            (CellValue::Bool(a), CellValue::Bool(b)) => a == b,
            (CellValue::Error(a), CellValue::Error(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for CellValue {}

impl Hash for CellValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CellValue::Blank => {
                3u8.hash(state);
            }
            CellValue::Number(n) => {
                0u8.hash(state);
                normalize_float_for_hash(*n).hash(state);
            }
            CellValue::Text(id) => {
                1u8.hash(state);
                id.hash(state);
            }
            CellValue::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            CellValue::Error(id) => {
                4u8.hash(state);
                id.hash(state);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RowSignature {
    #[serde(with = "signature_hex")]
    pub hash: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ColSignature {
    #[serde(with = "signature_hex")]
    pub hash: u128,
}

mod signature_hex {
    use serde::de::Error as DeError;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(val: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{:032x}", val);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u128::from_str_radix(&s, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))
    }
}

impl Grid {
    const DENSE_RATIO_THRESHOLD: f64 = 0.40;
    const DENSE_MIN_CELLS: usize = 4096;

    pub fn new(nrows: u32, ncols: u32) -> Grid {
        Grid::new_with_storage(nrows, ncols, GridStorage::new_sparse())
    }

    pub fn new_dense(nrows: u32, ncols: u32) -> Grid {
        Grid::new_with_storage(nrows, ncols, GridStorage::new_dense(nrows, ncols))
    }

    fn new_with_storage(nrows: u32, ncols: u32, cells: GridStorage) -> Grid {
        Grid {
            nrows,
            ncols,
            cells,
            row_signatures: None,
            col_signatures: None,
        }
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&CellContent> {
        self.cells.get(row, col)
    }

    pub fn get_ref(&self, row: u32, col: u32) -> Option<CellRef<'_>> {
        self.get(row, col).map(|cell| CellRef {
            row,
            col,
            address: CellAddress::from_indices(row, col),
            value: &cell.value,
            formula: &cell.formula,
        })
    }

    pub fn get_mut(&mut self, row: u32, col: u32) -> Option<&mut CellContent> {
        let out = self.cells.get_mut(row, col);
        if out.is_some() && (self.row_signatures.is_some() || self.col_signatures.is_some()) {
            self.row_signatures = None;
            self.col_signatures = None;
        }
        out
    }

    pub fn insert_cell(
        &mut self,
        row: u32,
        col: u32,
        value: Option<CellValue>,
        formula: Option<StringId>,
    ) {
        debug_assert!(
            row < self.nrows && col < self.ncols,
            "insert_cell out of bounds: ({row},{col}) for grid {}x{}",
            self.nrows,
            self.ncols
        );
        if self.row_signatures.is_some() || self.col_signatures.is_some() {
            self.row_signatures = None;
            self.col_signatures = None;
        }
        self.cells.insert(row, col, CellContent { value, formula });
        self.maybe_upgrade_to_dense();
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn estimated_bytes(&self) -> u64 {
        use std::mem::size_of;

        let cell_bytes = match &self.cells {
            GridStorage::Sparse(map) => {
                let cell_entry = size_of::<((u32, u32), CellContent)>();
                map.capacity().saturating_mul(cell_entry)
            }
            GridStorage::Dense(grid) => grid.capacity().saturating_mul(size_of::<CellContent>()),
        };
        let row_sig_bytes = self
            .row_signatures
            .as_ref()
            .map(|v| v.capacity().saturating_mul(size_of::<RowSignature>()))
            .unwrap_or(0);
        let col_sig_bytes = self
            .col_signatures
            .as_ref()
            .map(|v| v.capacity().saturating_mul(size_of::<ColSignature>()))
            .unwrap_or(0);

        (cell_bytes + row_sig_bytes + col_sig_bytes) as u64
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells_equal(&self, other: &GridStorage) -> bool {
        self.cells == *other
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = ((u32, u32), &CellContent)> {
        self.cells.iter()
    }

    pub fn iter_cell_refs(&self) -> impl Iterator<Item = CellRef<'_>> {
        self.cells.iter().map(|((row, col), cell)| CellRef {
            row,
            col,
            address: CellAddress::from_indices(row, col),
            value: &cell.value,
            formula: &cell.formula,
        })
    }

    pub fn rows_iter(&self) -> impl Iterator<Item = u32> + '_ {
        0..self.nrows
    }

    pub fn cols_iter(&self) -> impl Iterator<Item = u32> + '_ {
        0..self.ncols
    }

    pub fn compute_row_signature(&self, row: u32) -> RowSignature {
        use crate::hashing::hash_cell_value;
        use std::hash::Hash;
        use xxhash_rust::xxh3::Xxh3;

        let mut hasher = Xxh3::new();

        match &self.cells {
            GridStorage::Dense(grid) => {
                for col in 0..self.ncols {
                    if let Some(cell) = grid.get(row, col) {
                        hash_cell_value(&cell.value, &mut hasher);
                        cell.formula.hash(&mut hasher);
                    }
                }
            }
            GridStorage::Sparse(map) => {
                if (self.ncols as usize) <= map.len() {
                    for col in 0..self.ncols {
                        if let Some(cell) = map.get(&(row, col)) {
                            if cell.value.is_none() && cell.formula.is_none() {
                                continue;
                            }
                            hash_cell_value(&cell.value, &mut hasher);
                            cell.formula.hash(&mut hasher);
                        }
                    }
                } else {
                    let mut row_cells: Vec<(u32, &CellContent)> = map
                        .iter()
                        .filter(|((r, _), _)| *r == row)
                        .map(|((_, c), cell)| (*c, cell))
                        .collect();
                    row_cells.sort_by_key(|(c, _)| *c);
                    for (_, cell) in row_cells {
                        if cell.value.is_none() && cell.formula.is_none() {
                            continue;
                        }
                        hash_cell_value(&cell.value, &mut hasher);
                        cell.formula.hash(&mut hasher);
                    }
                }
            }
        }

        RowSignature {
            hash: hasher.digest128(),
        }
    }

    pub fn compute_col_signature(&self, col: u32) -> ColSignature {
        use crate::hashing::hash_cell_value;
        use std::hash::Hash;
        use xxhash_rust::xxh3::Xxh3;

        let mut hasher = Xxh3::new();

        match &self.cells {
            GridStorage::Dense(grid) => {
                for row in 0..self.nrows {
                    if let Some(cell) = grid.get(row, col) {
                        hash_cell_value(&cell.value, &mut hasher);
                        cell.formula.hash(&mut hasher);
                    }
                }
            }
            GridStorage::Sparse(map) => {
                if (self.nrows as usize) <= map.len() {
                    for row in 0..self.nrows {
                        if let Some(cell) = map.get(&(row, col)) {
                            if cell.value.is_none() && cell.formula.is_none() {
                                continue;
                            }
                            hash_cell_value(&cell.value, &mut hasher);
                            cell.formula.hash(&mut hasher);
                        }
                    }
                } else {
                    let mut col_cells: Vec<(u32, &CellContent)> = map
                        .iter()
                        .filter(|((_, c), _)| *c == col)
                        .map(|((r, _), cell)| (*r, cell))
                        .collect();
                    col_cells.sort_by_key(|(r, _)| *r);
                    for (_, cell) in col_cells {
                        if cell.value.is_none() && cell.formula.is_none() {
                            continue;
                        }
                        hash_cell_value(&cell.value, &mut hasher);
                        cell.formula.hash(&mut hasher);
                    }
                }
            }
        }

        ColSignature {
            hash: hasher.digest128(),
        }
    }

    pub fn compute_all_signatures(&mut self) {
        use crate::hashing::{hash_cell_value, hash_row_content_128};
        use xxhash_rust::xxh3::Xxh3;

        match &self.cells {
            GridStorage::Dense(grid) => {
                let mut row_signatures = Vec::with_capacity(self.nrows as usize);
                for row in 0..self.nrows {
                    let mut row_cells: Vec<(u32, &CellContent)> = Vec::new();
                    for col in 0..self.ncols {
                        if let Some(cell) = grid.get(row, col) {
                            row_cells.push((col, cell));
                        }
                    }
                    row_signatures.push(RowSignature {
                        hash: hash_row_content_128(&row_cells),
                    });
                }

                let mut col_hashers: Vec<Xxh3> = (0..self.ncols).map(|_| Xxh3::new()).collect();
                for row in 0..self.nrows {
                    for col in 0..self.ncols {
                        if let Some(cell) = grid.get(row, col) {
                            let idx = col as usize;
                            if idx >= col_hashers.len() {
                                continue;
                            }
                            hash_cell_value(&cell.value, &mut col_hashers[idx]);
                            cell.formula.hash(&mut col_hashers[idx]);
                        }
                    }
                }

                let col_signatures: Vec<ColSignature> = col_hashers
                    .into_iter()
                    .map(|hasher| ColSignature {
                        hash: hasher.digest128(),
                    })
                    .collect();

                self.row_signatures = Some(row_signatures);
                self.col_signatures = Some(col_signatures);
            }
            GridStorage::Sparse(map) => {
                let mut row_cells: Vec<Vec<(u32, &CellContent)>> =
                    vec![Vec::new(); self.nrows as usize];

                for ((row, col), cell) in map.iter() {
                    let row_idx = *row as usize;
                    if row_idx >= row_cells.len() || *col >= self.ncols {
                        continue;
                    }
                    row_cells[row_idx].push((*col, cell));
                }

                for row in row_cells.iter_mut() {
                    row.sort_by_key(|(col, _)| *col);
                }

                let row_signatures: Vec<RowSignature> = row_cells
                    .iter()
                    .map(|row| RowSignature {
                        hash: hash_row_content_128(row),
                    })
                    .collect();

                let mut col_hashers: Vec<Xxh3> = (0..self.ncols).map(|_| Xxh3::new()).collect();
                for row in row_cells.iter() {
                    for (col, cell) in row.iter() {
                        let idx = *col as usize;
                        if idx >= col_hashers.len() {
                            continue;
                        }
                        hash_cell_value(&cell.value, &mut col_hashers[idx]);
                        cell.formula.hash(&mut col_hashers[idx]);
                    }
                }

                let col_signatures: Vec<ColSignature> = col_hashers
                    .into_iter()
                    .map(|hasher| ColSignature {
                        hash: hasher.digest128(),
                    })
                    .collect();

                self.row_signatures = Some(row_signatures);
                self.col_signatures = Some(col_signatures);
            }
        }
    }

    pub(crate) fn should_use_dense(nrows: u32, ncols: u32, filled_cells: usize) -> bool {
        let total_cells = nrows.saturating_mul(ncols) as usize;
        if total_cells == 0 || total_cells < Self::DENSE_MIN_CELLS {
            return false;
        }
        let ratio = filled_cells as f64 / total_cells as f64;
        ratio >= Self::DENSE_RATIO_THRESHOLD
    }

    fn maybe_upgrade_to_dense(&mut self) {
        let GridStorage::Sparse(map) = &self.cells else {
            return;
        };

        if !Self::should_use_dense(self.nrows, self.ncols, map.len()) {
            return;
        }

        let mut dense = DenseGrid::new(self.nrows, self.ncols);
        for ((row, col), cell) in map.iter() {
            dense.set(*row, *col, cell.value.clone(), cell.formula);
        }
        self.cells = GridStorage::Dense(dense);
    }
}

impl PartialEq for CellSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.formula == other.formula
    }
}

impl Eq for CellSnapshot {}

impl CellValue {
    pub fn as_text_id(&self) -> Option<StringId> {
        if let CellValue::Text(id) = self {
            Some(*id)
        } else {
            None
        }
    }

    pub fn as_text<'a>(&self, pool: &'a StringPool) -> Option<&'a str> {
        self.as_text_id().map(|id| pool.resolve(id))
    }

    pub fn as_number(&self) -> Option<f64> {
        if let CellValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let CellValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string_pool::StringPool;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn addr(a1: &str) -> CellAddress {
        a1.parse().expect("address should parse")
    }

    fn make_cell(
        pool: &mut StringPool,
        address: &str,
        value: Option<CellValue>,
        formula: Option<&str>,
    ) -> ((u32, u32), CellContent) {
        let (row, col) = address_to_index(address).expect("address should parse");
        let formula_id = formula.map(|s| pool.intern(s));
        (
            (row, col),
            CellContent {
                value,
                formula: formula_id,
            },
        )
    }

    #[test]
    fn snapshot_from_number_cell() {
        let mut pool = StringPool::new();
        let ((row, col), cell) = make_cell(&mut pool, "A1", Some(CellValue::Number(42.0)), None);
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "A1");
        assert_eq!(snap.value, Some(CellValue::Number(42.0)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_text_cell() {
        let mut pool = StringPool::new();
        let text_id = pool.intern("hello");
        let ((row, col), cell) = make_cell(&mut pool, "B2", Some(CellValue::Text(text_id)), None);
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "B2");
        assert_eq!(snap.value, Some(CellValue::Text(text_id)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_bool_cell() {
        let mut pool = StringPool::new();
        let ((row, col), cell) = make_cell(&mut pool, "C3", Some(CellValue::Bool(true)), None);
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "C3");
        assert_eq!(snap.value, Some(CellValue::Bool(true)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_empty_cell() {
        let mut pool = StringPool::new();
        let ((row, col), cell) = make_cell(&mut pool, "D4", None, None);
        let snap = CellSnapshot::from_cell(row, col, &cell);
        assert_eq!(snap.addr.to_string(), "D4");
        assert!(snap.value.is_none());
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_equality_same_value_and_formula() {
        let mut pool = StringPool::new();
        let formula_id = pool.intern("A1+1");
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(1.0)),
            formula: Some(formula_id),
        };
        let snap2 = CellSnapshot {
            addr: addr("B2"),
            value: Some(CellValue::Number(1.0)),
            formula: Some(formula_id),
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_different_value_same_formula() {
        let mut pool = StringPool::new();
        let formula_id = pool.intern("A1+1");
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(43.0)),
            formula: Some(formula_id),
        };
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(44.0)),
            formula: Some(formula_id),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_value_vs_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: None,
        };
        let mut pool = StringPool::new();
        let formula_id = pool.intern("A1+1");
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: Some(formula_id),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_equality_ignores_address() {
        let mut pool = StringPool::new();
        let text_id = pool.intern("hello");
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Text(text_id)),
            formula: None,
        };
        let snap2 = CellSnapshot {
            addr: addr("Z9"),
            value: Some(CellValue::Text(text_id)),
            formula: None,
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn cellvalue_as_text_number_bool_match_variants() {
        let mut pool = StringPool::new();
        let text_id = pool.intern("abc");
        let text = CellValue::Text(text_id);
        let number = CellValue::Number(5.0);
        let boolean = CellValue::Bool(true);

        assert_eq!(text.as_text(&pool), Some("abc"));
        assert_eq!(text.as_number(), None);
        assert_eq!(text.as_bool(), None);

        assert_eq!(number.as_text(&pool), None);
        assert_eq!(number.as_number(), Some(5.0));
        assert_eq!(number.as_bool(), None);

        assert_eq!(boolean.as_text(&pool), None);
        assert_eq!(boolean.as_number(), None);
        assert_eq!(boolean.as_bool(), Some(true));
    }

    fn hash_cell_value(value: &CellValue) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn cellvalue_number_hashes_normalize_zero_sign() {
        let h_pos = hash_cell_value(&CellValue::Number(0.0));
        let h_neg = hash_cell_value(&CellValue::Number(-0.0));
        assert_eq!(h_pos, h_neg, "hash should ignore sign of zero");
    }

    #[test]
    fn cellvalue_number_hashes_ignore_ulp_drift() {
        let h_a = hash_cell_value(&CellValue::Number(1.0));
        let h_b = hash_cell_value(&CellValue::Number(1.0000000000000002));
        assert_eq!(h_a, h_b, "minor ULP drift should hash identically");
    }

    #[test]
    fn cellvalue_number_hashes_meaningful_difference() {
        let h_a = hash_cell_value(&CellValue::Number(1.0));
        let h_b = hash_cell_value(&CellValue::Number(1.0001));
        assert_ne!(h_a, h_b, "meaningful numeric changes must alter the hash");
    }

    #[test]
    fn get_mut_clears_cached_signatures() {
        let mut pool = StringPool::new();
        let mut grid = Grid::new(2, 2);
        let id1 = pool.intern("1");
        grid.insert_cell(0, 0, Some(CellValue::Text(id1)), None);
        grid.insert_cell(1, 1, Some(CellValue::Number(2.0)), None);

        grid.compute_all_signatures();
        assert!(grid.row_signatures.is_some());
        assert!(grid.col_signatures.is_some());

        let _ = grid.get_mut(0, 0);

        assert!(grid.row_signatures.is_none());
        assert!(grid.col_signatures.is_none());
    }

    #[test]
    fn insert_clears_cached_signatures() {
        let mut pool = StringPool::new();
        let mut grid = Grid::new(3, 3);
        let id1 = pool.intern("1");
        grid.insert_cell(0, 0, Some(CellValue::Text(id1)), None);

        grid.compute_all_signatures();
        assert!(grid.row_signatures.is_some());
        assert!(grid.col_signatures.is_some());

        let id2 = pool.intern("x");
        grid.insert_cell(1, 1, Some(CellValue::Text(id2)), None);

        assert!(grid.row_signatures.is_none());
        assert!(grid.col_signatures.is_none());
    }

    #[test]
    fn compute_row_signature_matches_cached_for_dense_and_sparse_paths() {
        let mut dense = Grid::new(1, 3);
        dense.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
        dense.insert_cell(0, 1, Some(CellValue::Number(2.0)), None);
        dense.insert_cell(0, 2, Some(CellValue::Number(3.0)), None);
        dense.compute_all_signatures();
        let cached_dense = dense.row_signatures.as_ref().unwrap()[0];
        assert_eq!(dense.compute_row_signature(0), cached_dense);

        let mut sparse = Grid::new(1, 10);
        sparse.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
        sparse.insert_cell(0, 9, Some(CellValue::Number(10.0)), None);
        sparse.compute_all_signatures();
        let cached_sparse = sparse.row_signatures.as_ref().unwrap()[0];
        assert_eq!(sparse.compute_row_signature(0), cached_sparse);
    }

    #[test]
    fn compute_col_signature_matches_cached_for_dense_and_sparse_paths() {
        let mut dense = Grid::new(3, 1);
        dense.insert_cell(0, 0, Some(CellValue::Number(1.0)), None);
        dense.insert_cell(1, 0, Some(CellValue::Number(2.0)), None);
        dense.insert_cell(2, 0, Some(CellValue::Number(3.0)), None);
        dense.compute_all_signatures();
        let cached_dense = dense.col_signatures.as_ref().unwrap()[0];
        assert_eq!(dense.compute_col_signature(0), cached_dense);

        let mut sparse = Grid::new(10, 2);
        sparse.insert_cell(0, 1, Some(CellValue::Number(1.0)), None);
        sparse.insert_cell(2, 1, Some(CellValue::Number(3.0)), None);
        sparse.compute_all_signatures();
        let cached_sparse = sparse.col_signatures.as_ref().unwrap()[1];
        assert_eq!(sparse.compute_col_signature(1), cached_sparse);
    }
}
