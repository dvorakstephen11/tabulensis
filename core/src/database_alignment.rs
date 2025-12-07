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

    #[test]
    fn composite_key_alignment_matches_rows_correctly() {
        let grid_a = grid_from_rows(&[&[1, 10, 100], &[1, 20, 200], &[2, 10, 300]]);
        let grid_b = grid_from_rows(&[&[1, 20, 200], &[2, 10, 300], &[1, 10, 100]]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 1]).expect("unique composite keys");

        assert!(
            alignment.left_only_rows.is_empty(),
            "no left-only rows expected"
        );
        assert!(
            alignment.right_only_rows.is_empty(),
            "no right-only rows expected"
        );

        let mut matched = alignment.matched_rows.clone();
        matched.sort_unstable();

        let mut expected = vec![(0, 2), (1, 0), (2, 1)];
        expected.sort_unstable();

        assert_eq!(
            matched, expected,
            "composite keys should align rows sharing the same key tuple regardless of order"
        );
    }

    #[test]
    fn non_contiguous_key_columns_alignment() {
        let grid_a = grid_from_rows(&[
            &[1, 999, 10, 100],
            &[1, 888, 20, 200],
            &[2, 777, 10, 300],
        ]);
        let grid_b = grid_from_rows(&[
            &[2, 777, 10, 300],
            &[1, 999, 10, 100],
            &[1, 888, 20, 200],
        ]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 2]).expect("unique non-contiguous keys");

        assert!(alignment.left_only_rows.is_empty());
        assert!(alignment.right_only_rows.is_empty());

        let mut matched = alignment.matched_rows.clone();
        matched.sort_unstable();

        let mut expected = vec![(0, 1), (1, 2), (2, 0)];
        expected.sort_unstable();

        assert_eq!(
            matched, expected,
            "non-contiguous key columns [0,2] should align correctly"
        );
    }

    #[test]
    fn three_column_composite_key_alignment() {
        let grid_a = grid_from_rows(&[
            &[1, 10, 100, 1000],
            &[1, 10, 200, 2000],
            &[1, 20, 100, 3000],
            &[2, 10, 100, 4000],
        ]);
        let grid_b = grid_from_rows(&[
            &[2, 10, 100, 4000],
            &[1, 20, 100, 3000],
            &[1, 10, 200, 2000],
            &[1, 10, 100, 1000],
        ]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 1, 2]).expect("unique three-column keys");

        assert!(alignment.left_only_rows.is_empty());
        assert!(alignment.right_only_rows.is_empty());

        let mut matched = alignment.matched_rows.clone();
        matched.sort_unstable();

        let mut expected = vec![(0, 3), (1, 2), (2, 1), (3, 0)];
        expected.sort_unstable();

        assert_eq!(
            matched, expected,
            "three-column composite keys should align correctly"
        );
    }

    #[test]
    fn is_key_column_single_column() {
        let spec = KeyColumnSpec::new(vec![0]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(!spec.is_key_column(1), "column 1 should not be a key column");
        assert!(!spec.is_key_column(2), "column 2 should not be a key column");
    }

    #[test]
    fn is_key_column_contiguous_columns() {
        let spec = KeyColumnSpec::new(vec![0, 1]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(spec.is_key_column(1), "column 1 should be a key column");
        assert!(!spec.is_key_column(2), "column 2 should not be a key column");
        assert!(!spec.is_key_column(3), "column 3 should not be a key column");
    }

    #[test]
    fn is_key_column_non_contiguous_columns() {
        let spec = KeyColumnSpec::new(vec![0, 2]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(!spec.is_key_column(1), "column 1 should not be a key column");
        assert!(spec.is_key_column(2), "column 2 should be a key column");
        assert!(!spec.is_key_column(3), "column 3 should not be a key column");
    }

    #[test]
    fn is_key_column_three_columns() {
        let spec = KeyColumnSpec::new(vec![0, 1, 2]);
        assert!(spec.is_key_column(0));
        assert!(spec.is_key_column(1));
        assert!(spec.is_key_column(2));
        assert!(!spec.is_key_column(3));
    }

    #[test]
    fn is_key_column_non_contiguous_three_columns() {
        let spec = KeyColumnSpec::new(vec![1, 3, 5]);
        assert!(!spec.is_key_column(0), "column 0 should not be a key column");
        assert!(spec.is_key_column(1), "column 1 should be a key column");
        assert!(!spec.is_key_column(2), "column 2 should not be a key column");
        assert!(spec.is_key_column(3), "column 3 should be a key column");
        assert!(!spec.is_key_column(4), "column 4 should not be a key column");
        assert!(spec.is_key_column(5), "column 5 should be a key column");
        assert!(!spec.is_key_column(6), "column 6 should not be a key column");
    }
}
