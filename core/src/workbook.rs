use crate::addressing::{address_to_index, index_to_address};

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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sheet {
    pub name: String,
    pub kind: SheetKind,
    pub grid: Grid,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SheetKind {
    Worksheet,
    Chart,
    Macro,
    Other,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Grid {
    pub nrows: u32,
    pub ncols: u32,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub index: u32,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: Option<CellValue>,
    pub formula: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CellValue {
    Number(f64),
    Text(String),
    Bool(bool),
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
}
