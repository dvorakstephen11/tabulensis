//! Workbook, sheet, and grid data structures.
//!
//! This module defines the core intermediate representation (IR) for Excel workbooks:
//! - [`Workbook`]: A collection of sheets
//! - [`Sheet`]: A named sheet with a grid of cells
//! - [`Grid`]: A sparse 2D grid of cells with optional row/column signatures
//! - [`Cell`]: Individual cell with address, value, and optional formula

use crate::addressing::{AddressParseError, address_to_index, index_to_address};
use crate::hashing::normalize_float_for_hash;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// A snapshot of a cell's logical content for comparison purposes.
///
/// Used in [`DiffOp::CellEdited`] to represent the "before" and "after" states.
/// Equality comparison intentionally ignores `addr` and compares only `(value, formula)`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CellSnapshot {
    pub addr: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

impl CellSnapshot {
    pub fn from_cell(cell: &Cell) -> CellSnapshot {
        CellSnapshot {
            addr: cell.address,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
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
#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

/// A single sheet within a workbook.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The display name of the sheet (e.g., "Sheet1", "Data").
    pub name: String,
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

/// A sparse 2D grid of cells representing sheet data.
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
    /// Sparse storage of non-empty cells, keyed by (row, col).
    pub cells: HashMap<(u32, u32), Cell>,
    /// Optional precomputed row signatures for alignment.
    pub row_signatures: Option<Vec<RowSignature>>,
    /// Optional precomputed column signatures for alignment.
    pub col_signatures: Option<Vec<ColSignature>>,
}

/// A single cell within a grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// Zero-based row index.
    pub row: u32,
    /// Zero-based column index.
    pub col: u32,
    /// The cell's A1-style address (e.g., "B2").
    pub address: CellAddress,
    /// The cell's value, if any.
    pub value: Option<CellValue>,
    /// The cell's formula text (without leading '='), if any.
    pub formula: Option<String>,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Number(f64),
    Text(String),
    Bool(bool),
}

impl PartialEq for CellValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CellValue::Number(a), CellValue::Number(b)) => {
                normalize_float_for_hash(*a) == normalize_float_for_hash(*b)
            }
            (CellValue::Text(a), CellValue::Text(b)) => a == b,
            (CellValue::Bool(a), CellValue::Bool(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for CellValue {}

impl Hash for CellValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            CellValue::Number(n) => {
                0u8.hash(state);
                normalize_float_for_hash(*n).hash(state);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RowSignature {
    pub hash: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColSignature {
    pub hash: u128,
}

#[allow(dead_code)]
mod signature_serde {
    use serde::de::Error as DeError;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize_u128<S>(val: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{:032x}", val).serialize(serializer)
    }

    pub fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u128::from_str_radix(&s, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))
    }
}

impl serde::Serialize for RowSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("RowSignature", 1)?;
        s.serialize_field("hash", &format!("{:032x}", self.hash))?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for RowSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;

        #[derive(serde::Deserialize)]
        struct Helper {
            hash: String,
        }
        let helper = Helper::deserialize(deserializer)?;
        let hash = u128::from_str_radix(&helper.hash, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))?;
        Ok(RowSignature { hash })
    }
}

impl serde::Serialize for ColSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ColSignature", 1)?;
        s.serialize_field("hash", &format!("{:032x}", self.hash))?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for ColSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;

        #[derive(serde::Deserialize)]
        struct Helper {
            hash: String,
        }
        let helper = Helper::deserialize(deserializer)?;
        let hash = u128::from_str_radix(&helper.hash, 16)
            .map_err(|e| DeError::custom(format!("invalid hex hash: {}", e)))?;
        Ok(ColSignature { hash })
    }
}

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
        self.row_signatures = None;
        self.col_signatures = None;
        self.cells.get_mut(&(row, col))
    }

    pub fn insert(&mut self, cell: Cell) {
        debug_assert!(
            cell.row < self.nrows && cell.col < self.ncols,
            "cell coordinates must lie within the grid bounds"
        );
        self.row_signatures = None;
        self.col_signatures = None;
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

        if (self.ncols as usize) <= self.cells.len() {
            for col in 0..self.ncols {
                if let Some(cell) = self.cells.get(&(row, col)) {
                    if cell.value.is_none() && cell.formula.is_none() {
                        continue;
                    }
                    hash_cell_value(&cell.value, &mut hasher);
                    cell.formula.hash(&mut hasher);
                }
            }
        } else {
            let mut row_cells: Vec<&Cell> = self.cells.values().filter(|c| c.row == row).collect();
            row_cells.sort_by_key(|c| c.col);
            for cell in row_cells {
                if cell.value.is_none() && cell.formula.is_none() {
                    continue;
                }
                hash_cell_value(&cell.value, &mut hasher);
                cell.formula.hash(&mut hasher);
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

        if (self.nrows as usize) <= self.cells.len() {
            for row in 0..self.nrows {
                if let Some(cell) = self.cells.get(&(row, col)) {
                    if cell.value.is_none() && cell.formula.is_none() {
                        continue;
                    }
                    hash_cell_value(&cell.value, &mut hasher);
                    cell.formula.hash(&mut hasher);
                }
            }
        } else {
            let mut col_cells: Vec<&Cell> = self.cells.values().filter(|c| c.col == col).collect();
            col_cells.sort_by_key(|c| c.row);
            for cell in col_cells {
                if cell.value.is_none() && cell.formula.is_none() {
                    continue;
                }
                hash_cell_value(&cell.value, &mut hasher);
                cell.formula.hash(&mut hasher);
            }
        }

        ColSignature {
            hash: hasher.digest128(),
        }
    }

    pub fn compute_all_signatures(&mut self) {
        use crate::hashing::{hash_col_content_128, hash_row_content_128};

        let mut row_cells: Vec<Vec<(u32, &Cell)>> = vec![Vec::new(); self.nrows as usize];
        let mut col_cells: Vec<Vec<&Cell>> = vec![Vec::new(); self.ncols as usize];

        for cell in self.cells.values() {
            let row_idx = cell.row as usize;
            let col_idx = cell.col as usize;

            debug_assert!(
                row_idx < row_cells.len() && col_idx < col_cells.len(),
                "cell coordinates must lie within the grid bounds"
            );

            row_cells[row_idx].push((cell.col, cell));
            col_cells[col_idx].push(cell);
        }

        self.row_signatures = Some(
            row_cells
                .into_iter()
                .map(|mut cells| {
                    cells.sort_unstable_by_key(|(col, _)| *col);
                    let hash = hash_row_content_128(&cells);
                    RowSignature { hash }
                })
                .collect(),
        );

        self.col_signatures = Some(
            col_cells
                .into_iter()
                .map(|mut cells| {
                    cells.sort_unstable_by_key(|c| c.row);
                    let hash = hash_col_content_128(&cells);
                    ColSignature { hash }
                })
                .collect(),
        );
    }
}

impl PartialEq for CellSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.formula == other.formula
    }
}

impl Eq for CellSnapshot {}

impl CellValue {
    pub fn as_text(&self) -> Option<&str> {
        if let CellValue::Text(s) = self {
            Some(s)
        } else {
            None
        }
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
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn addr(a1: &str) -> CellAddress {
        a1.parse().expect("address should parse")
    }

    fn make_cell(address: &str, value: Option<CellValue>, formula: Option<&str>) -> Cell {
        let (row, col) = address_to_index(address).expect("address should parse");
        Cell {
            row,
            col,
            address: CellAddress::from_indices(row, col),
            value,
            formula: formula.map(|s| s.to_string()),
        }
    }

    #[test]
    fn snapshot_from_number_cell() {
        let cell = make_cell("A1", Some(CellValue::Number(42.0)), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "A1");
        assert_eq!(snap.value, Some(CellValue::Number(42.0)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_text_cell() {
        let cell = make_cell("B2", Some(CellValue::Text("hello".into())), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "B2");
        assert_eq!(snap.value, Some(CellValue::Text("hello".into())));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_bool_cell() {
        let cell = make_cell("C3", Some(CellValue::Bool(true)), None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "C3");
        assert_eq!(snap.value, Some(CellValue::Bool(true)));
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_from_empty_cell() {
        let cell = make_cell("D4", None, None);
        let snap = CellSnapshot::from_cell(&cell);
        assert_eq!(snap.addr.to_string(), "D4");
        assert!(snap.value.is_none());
        assert!(snap.formula.is_none());
    }

    #[test]
    fn snapshot_equality_same_value_and_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(1.0)),
            formula: Some("A1+1".into()),
        };
        let snap2 = CellSnapshot {
            addr: addr("B2"),
            value: Some(CellValue::Number(1.0)),
            formula: Some("A1+1".into()),
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn snapshot_inequality_different_value_same_formula() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(43.0)),
            formula: Some("A1+1".into()),
        };
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(44.0)),
            formula: Some("A1+1".into()),
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
        let snap2 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Number(42.0)),
            formula: Some("A1+1".into()),
        };
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn snapshot_equality_ignores_address() {
        let snap1 = CellSnapshot {
            addr: addr("A1"),
            value: Some(CellValue::Text("hello".into())),
            formula: None,
        };
        let snap2 = CellSnapshot {
            addr: addr("Z9"),
            value: Some(CellValue::Text("hello".into())),
            formula: None,
        };
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn cellvalue_as_text_number_bool_match_variants() {
        let text = CellValue::Text("abc".into());
        let number = CellValue::Number(5.0);
        let boolean = CellValue::Bool(true);

        assert_eq!(text.as_text(), Some("abc"));
        assert_eq!(text.as_number(), None);
        assert_eq!(text.as_bool(), None);

        assert_eq!(number.as_text(), None);
        assert_eq!(number.as_number(), Some(5.0));
        assert_eq!(number.as_bool(), None);

        assert_eq!(boolean.as_text(), None);
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
        let mut grid = Grid::new(2, 2);
        grid.insert(make_cell("A1", Some(CellValue::Number(1.0)), None));
        grid.insert(make_cell("B2", Some(CellValue::Number(2.0)), None));

        grid.compute_all_signatures();
        assert!(grid.row_signatures.is_some());
        assert!(grid.col_signatures.is_some());

        let _ = grid.get_mut(0, 0);

        assert!(grid.row_signatures.is_none());
        assert!(grid.col_signatures.is_none());
    }

    #[test]
    fn insert_clears_cached_signatures() {
        let mut grid = Grid::new(3, 3);
        grid.insert(make_cell("A1", Some(CellValue::Number(1.0)), None));

        grid.compute_all_signatures();
        assert!(grid.row_signatures.is_some());
        assert!(grid.col_signatures.is_some());

        grid.insert(make_cell("B2", Some(CellValue::Text("x".into())), None));

        assert!(grid.row_signatures.is_none());
        assert!(grid.col_signatures.is_none());
    }

    #[test]
    fn compute_row_signature_matches_cached_for_dense_and_sparse_paths() {
        let mut dense = Grid::new(1, 3);
        dense.insert(make_cell("A1", Some(CellValue::Number(1.0)), None));
        dense.insert(make_cell("B1", Some(CellValue::Number(2.0)), None));
        dense.insert(make_cell("C1", Some(CellValue::Number(3.0)), None));
        dense.compute_all_signatures();
        let cached_dense = dense.row_signatures.as_ref().unwrap()[0];
        assert_eq!(dense.compute_row_signature(0), cached_dense);

        let mut sparse = Grid::new(1, 10);
        sparse.insert(make_cell("A1", Some(CellValue::Number(1.0)), None));
        sparse.insert(make_cell("J1", Some(CellValue::Number(10.0)), None));
        sparse.compute_all_signatures();
        let cached_sparse = sparse.row_signatures.as_ref().unwrap()[0];
        assert_eq!(sparse.compute_row_signature(0), cached_sparse);
    }

    #[test]
    fn compute_col_signature_matches_cached_for_dense_and_sparse_paths() {
        let mut dense = Grid::new(3, 1);
        dense.insert(make_cell("A1", Some(CellValue::Number(1.0)), None));
        dense.insert(make_cell("A2", Some(CellValue::Number(2.0)), None));
        dense.insert(make_cell("A3", Some(CellValue::Number(3.0)), None));
        dense.compute_all_signatures();
        let cached_dense = dense.col_signatures.as_ref().unwrap()[0];
        assert_eq!(dense.compute_col_signature(0), cached_dense);

        let mut sparse = Grid::new(10, 2);
        sparse.insert(make_cell("B1", Some(CellValue::Number(1.0)), None));
        sparse.insert(make_cell("B3", Some(CellValue::Number(3.0)), None));
        sparse.compute_all_signatures();
        let cached_sparse = sparse.col_signatures.as_ref().unwrap()[1];
        assert_eq!(sparse.compute_col_signature(1), cached_sparse);
    }
}
