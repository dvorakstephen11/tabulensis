use crate::addressing::{address_to_index, index_to_address};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use xxhash_rust::xxh64::Xxh64;

/// A snapshot of a cell's logical content (address, value, formula).
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

#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub kind: SheetKind,
    pub grid: Grid,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SheetKind {
    Worksheet,
    Chart,
    Macro,
    Other,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellAddress {
    pub row: u32,
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
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (row, col) = address_to_index(s).ok_or(())?;
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
        CellAddress::from_str(&a1)
            .map_err(|_| DeError::custom(format!("invalid cell address: {a1}")))
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Number(f64),
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowSignature {
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ColSignature {
    pub hash: u64,
}

const XXH64_SEED: u64 = 0;

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
        0..self.nrows
    }

    pub fn cols_iter(&self) -> impl Iterator<Item = u32> + '_ {
        0..self.ncols
    }

    pub fn compute_row_signature(&self, row: u32) -> RowSignature {
        let mut row_cells: Vec<&Cell> =
            self.cells.values().filter(|cell| cell.row == row).collect();
        row_cells.sort_by_key(|cell| cell.col);

        let mut hasher = Xxh64::new(XXH64_SEED);
        for cell in row_cells {
            hash_cell_with_position(cell.col, cell, &mut hasher);
        }
        RowSignature {
            hash: hasher.finish(),
        }
    }

    pub fn compute_col_signature(&self, col: u32) -> ColSignature {
        let mut col_cells: Vec<&Cell> =
            self.cells.values().filter(|cell| cell.col == col).collect();
        col_cells.sort_by_key(|cell| cell.row);

        let mut hasher = Xxh64::new(XXH64_SEED);
        for cell in col_cells {
            hash_cell_with_position(cell.row, cell, &mut hasher);
        }
        ColSignature {
            hash: hasher.finish(),
        }
    }

    pub fn compute_all_signatures(&mut self) {
        let mut row_hashers: Vec<Xxh64> = (0..self.nrows).map(|_| Xxh64::new(XXH64_SEED)).collect();
        let mut col_hashers: Vec<Xxh64> = (0..self.ncols).map(|_| Xxh64::new(XXH64_SEED)).collect();
        let mut cells: Vec<&Cell> = self.cells.values().collect();

        cells.sort_by(|a, b| a.row.cmp(&b.row).then_with(|| a.col.cmp(&b.col)));
        for &cell in &cells {
            hash_cell_with_position(cell.col, cell, &mut row_hashers[cell.row as usize]);
        }

        cells.sort_by(|a, b| a.col.cmp(&b.col).then_with(|| a.row.cmp(&b.row)));
        for &cell in &cells {
            hash_cell_with_position(cell.row, cell, &mut col_hashers[cell.col as usize]);
        }

        self.row_signatures = Some(
            row_hashers
                .into_iter()
                .map(|hasher| RowSignature {
                    hash: hasher.finish(),
                })
                .collect(),
        );

        self.col_signatures = Some(
            col_hashers
                .into_iter()
                .map(|hasher| ColSignature {
                    hash: hasher.finish(),
                })
                .collect(),
        );
    }
}

fn hash_cell_with_position<H: Hasher>(position: u32, cell: &Cell, hasher: &mut H) {
    position.hash(hasher);
    cell.value.hash(hasher);
    cell.formula.hash(hasher);
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
}
