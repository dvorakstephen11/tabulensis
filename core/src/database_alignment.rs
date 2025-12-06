use crate::workbook::{CellValue, Grid};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyColumnSpec {
    pub columns: Vec<u32>,
}

impl KeyColumnSpec {
    pub fn new(columns: Vec<u32>) -> KeyColumnSpec {
        KeyColumnSpec { columns }
    }

    pub fn is_key_column(&self, col: u32) -> bool {
        self.columns.contains(&col)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum KeyValueRepr {
    None,
    Number(u64),
    Text(String),
    Bool(bool),
}

impl KeyValueRepr {
    fn from_cell_value(value: Option<&CellValue>) -> KeyValueRepr {
        match value {
            Some(CellValue::Number(n)) => KeyValueRepr::Number(n.to_bits()),
            Some(CellValue::Text(s)) => KeyValueRepr::Text(s.clone()),
            Some(CellValue::Bool(b)) => KeyValueRepr::Bool(*b),
            None => KeyValueRepr::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct KeyComponent {
    pub value: KeyValueRepr,
    pub formula: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct KeyValue {
    components: Vec<KeyComponent>,
}

impl KeyValue {
    fn new(components: Vec<KeyComponent>) -> KeyValue {
        KeyValue { components }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyedRow {
    pub key: KeyValue,
    pub row_idx: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyedAlignment {
    pub matched_rows: Vec<(u32, u32)>, // (row_idx_a, row_idx_b)
    pub left_only_rows: Vec<u32>,
    pub right_only_rows: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyAlignmentError {
    DuplicateKeyLeft(KeyValue),
    DuplicateKeyRight(KeyValue),
}

pub(crate) fn diff_table_by_key(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
) -> Result<KeyedAlignment, KeyAlignmentError> {
    let spec = KeyColumnSpec::new(key_columns.to_vec());
    let (left_rows, _left_lookup) = build_keyed_rows(old, &spec, true)?;
    let (right_rows, right_lookup) = build_keyed_rows(new, &spec, false)?;

    let mut matched_rows = Vec::new();
    let mut left_only_rows = Vec::new();
    let mut right_only_rows = Vec::new();

    let mut matched_right_rows: HashSet<u32> = HashSet::new();

    for row in &left_rows {
        if let Some(&row_b) = right_lookup.get(&row.key) {
            matched_rows.push((row.row_idx, row_b));
            matched_right_rows.insert(row_b);
        } else {
            left_only_rows.push(row.row_idx);
        }
    }

    for row in &right_rows {
        if !matched_right_rows.contains(&row.row_idx) {
            right_only_rows.push(row.row_idx);
        }
    }

    Ok(KeyedAlignment {
        matched_rows,
        left_only_rows,
        right_only_rows,
    })
}

fn build_keyed_rows(
    grid: &Grid,
    spec: &KeyColumnSpec,
    is_left: bool,
) -> Result<(Vec<KeyedRow>, HashMap<KeyValue, u32>), KeyAlignmentError> {
    let mut rows = Vec::with_capacity(grid.nrows as usize);
    let mut lookup = HashMap::new();

    for row_idx in 0..grid.nrows {
        let key = extract_key(grid, row_idx, spec);
        if lookup.insert(key.clone(), row_idx).is_some() {
            return Err(if is_left {
                KeyAlignmentError::DuplicateKeyLeft(key)
            } else {
                KeyAlignmentError::DuplicateKeyRight(key)
            });
        }
        rows.push(KeyedRow { key, row_idx });
    }

    Ok((rows, lookup))
}

fn extract_key(grid: &Grid, row_idx: u32, spec: &KeyColumnSpec) -> KeyValue {
    let mut components = Vec::with_capacity(spec.columns.len());

    for &col in &spec.columns {
        let component = match grid.get(row_idx, col) {
            Some(cell) => KeyComponent {
                value: KeyValueRepr::from_cell_value(cell.value.as_ref()),
                formula: cell.formula.clone(),
            },
            None => KeyComponent {
                value: KeyValueRepr::None,
                formula: None,
            },
        };
        components.push(component);
    }

    KeyValue::new(components)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::{Cell, CellAddress};

    fn grid_from_rows(rows: &[&[i32]]) -> Grid {
        let nrows = rows.len() as u32;
        let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
        let mut grid = Grid::new(nrows, ncols);

        for (r_idx, row_vals) in rows.iter().enumerate() {
            for (c_idx, value) in row_vals.iter().enumerate() {
                grid.insert(Cell {
                    row: r_idx as u32,
                    col: c_idx as u32,
                    address: CellAddress::from_indices(r_idx as u32, c_idx as u32),
                    value: Some(CellValue::Number(*value as f64)),
                    formula: None,
                });
            }
        }

        grid
    }

    #[test]
    fn unique_keys_reorder_no_changes() {
        let grid_a = grid_from_rows(&[&[1, 10], &[2, 20], &[3, 30]]);
        let grid_b = grid_from_rows(&[&[3, 30], &[1, 10], &[2, 20]]);

        let alignment = diff_table_by_key(&grid_a, &grid_b, &[0]).expect("unique keys");
        assert_eq!(
            alignment.matched_rows,
            vec![(0, 1), (1, 2), (2, 0)],
            "all keys should align regardless of order"
        );
        assert!(alignment.left_only_rows.is_empty());
        assert!(alignment.right_only_rows.is_empty());
    }

    #[test]
    fn unique_keys_insert_delete_classified() {
        let grid_a = grid_from_rows(&[&[1, 10], &[2, 20]]);
        let grid_b = grid_from_rows(&[&[1, 10], &[2, 20], &[3, 30]]);

        let alignment = diff_table_by_key(&grid_a, &grid_b, &[0]).expect("unique keys");
        assert_eq!(alignment.matched_rows, vec![(0, 0), (1, 1)]);
        assert!(alignment.left_only_rows.is_empty());
        assert_eq!(alignment.right_only_rows, vec![2]);
    }

    #[test]
    fn duplicate_keys_error_or_unsupported() {
        let grid_a = grid_from_rows(&[&[1, 10], &[1, 99]]);
        let grid_b = grid_from_rows(&[&[1, 10]]);

        let err = diff_table_by_key(&grid_a, &grid_b, &[0]).expect_err("duplicate keys");
        assert!(matches!(err, KeyAlignmentError::DuplicateKeyLeft(_)));
    }
}
