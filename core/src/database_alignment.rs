use crate::hashing::normalize_float_for_hash;
use crate::string_pool::{StringId, StringPool};
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

    #[cfg(test)]
    pub fn is_key_column(&self, col: u32) -> bool {
        self.columns.contains(&col)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum KeyValueRepr {
    None,
    Number(u64),
    Text(StringId),
    Bool(bool),
    Error(StringId),
}

impl KeyValueRepr {
    fn from_cell_value(value: Option<&CellValue>) -> KeyValueRepr {
        match value {
            Some(CellValue::Number(n)) => KeyValueRepr::Number(normalize_float_for_hash(*n)),
            Some(CellValue::Text(id)) => KeyValueRepr::Text(*id),
            Some(CellValue::Bool(b)) => KeyValueRepr::Bool(*b),
            Some(CellValue::Blank) => KeyValueRepr::None,
            Some(CellValue::Error(id)) => KeyValueRepr::Error(*id),
            None => KeyValueRepr::None,
        }
    }

    fn to_cell_value(&self) -> Option<CellValue> {
        match self {
            KeyValueRepr::None => None,
            KeyValueRepr::Number(bits) => Some(CellValue::Number(f64::from_bits(*bits))),
            KeyValueRepr::Text(id) => Some(CellValue::Text(*id)),
            KeyValueRepr::Bool(b) => Some(CellValue::Bool(*b)),
            KeyValueRepr::Error(id) => Some(CellValue::Error(*id)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct KeyValue {
    components: Vec<KeyValueRepr>,
}

impl KeyValue {
    fn new(components: Vec<KeyValueRepr>) -> KeyValue {
        KeyValue { components }
    }

    pub(crate) fn as_cell_values(&self) -> Vec<Option<CellValue>> {
        self.components
            .iter()
            .map(KeyValueRepr::to_cell_value)
            .collect()
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
    pub duplicate_clusters: Vec<DuplicateKeyCluster>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DuplicateKeyCluster {
    pub key: KeyValue,
    pub left_rows: Vec<u32>,
    pub right_rows: Vec<u32>,
}

pub(crate) fn diff_table_by_key(
    old: &Grid,
    new: &Grid,
    key_columns: &[u32],
) -> KeyedAlignment {
    let spec = KeyColumnSpec::new(key_columns.to_vec());
    let (left_rows, left_lookup) = build_keyed_rows(old, &spec);
    let (right_rows, right_lookup) = build_keyed_rows(new, &spec);

    let mut matched_rows = Vec::new();
    let mut left_only_rows = Vec::new();
    let mut right_only_rows = Vec::new();
    let mut duplicate_clusters = Vec::new();

    let mut ordered_keys: Vec<KeyValue> = Vec::new();
    let mut seen_keys: HashSet<KeyValue> = HashSet::new();

    for row in &left_rows {
        if seen_keys.insert(row.key.clone()) {
            ordered_keys.push(row.key.clone());
        }
    }

    for row in &right_rows {
        if seen_keys.insert(row.key.clone()) {
            ordered_keys.push(row.key.clone());
        }
    }

    for key in ordered_keys {
        let left = left_lookup.get(&key).cloned().unwrap_or_default();
        let right = right_lookup.get(&key).cloned().unwrap_or_default();

        let left_dupe = left.len() > 1;
        let right_dupe = right.len() > 1;

        if left_dupe || right_dupe {
            duplicate_clusters.push(DuplicateKeyCluster {
                key,
                left_rows: left,
                right_rows: right,
            });
            continue;
        }

        match (left.first().copied(), right.first().copied()) {
            (Some(l), Some(r)) => matched_rows.push((l, r)),
            (Some(l), None) => left_only_rows.push(l),
            (None, Some(r)) => right_only_rows.push(r),
            (None, None) => {}
        }
    }

    KeyedAlignment {
        matched_rows,
        left_only_rows,
        right_only_rows,
        duplicate_clusters,
    }
}

fn build_keyed_rows(
    grid: &Grid,
    spec: &KeyColumnSpec,
) -> (Vec<KeyedRow>, HashMap<KeyValue, Vec<u32>>) {
    let mut rows = Vec::with_capacity(grid.nrows as usize);
    let mut lookup = HashMap::new();

    for row_idx in 0..grid.nrows {
        let key = extract_key(grid, row_idx, spec);
        lookup.entry(key.clone()).or_insert_with(Vec::new).push(row_idx);
        rows.push(KeyedRow { key, row_idx });
    }

    (rows, lookup)
}

fn extract_key(grid: &Grid, row_idx: u32, spec: &KeyColumnSpec) -> KeyValue {
    let mut components = Vec::with_capacity(spec.columns.len());

    for &col in &spec.columns {
        let value = grid
            .get(row_idx, col)
            .map(|cell| KeyValueRepr::from_cell_value(cell.value.as_ref()))
            .unwrap_or(KeyValueRepr::None);
        components.push(value);
    }

    KeyValue::new(components)
}

pub fn suggest_key_columns(grid: &Grid, pool: &StringPool) -> Vec<u32> {
    if grid.nrows == 0 || grid.ncols == 0 {
        return Vec::new();
    }

    let header_matches_key_pattern = |col: u32| -> bool {
        if let Some(cell) = grid.get(0, col) {
            if let Some(CellValue::Text(id)) = &cell.value {
                let text = pool.resolve(*id).trim().to_lowercase();
                return text == "id" || text == "key" || text == "sku" 
                    || text.contains("_id") || text.ends_with("id");
            }
        }
        false
    };

    let data_start = if grid.nrows > 1 { 1 } else { 0 };
    let data_rows = grid.nrows.saturating_sub(data_start);
    if data_rows == 0 {
        return Vec::new();
    }

    let column_non_empty = |col: u32| -> bool {
        for row in data_start..grid.nrows {
            let value = grid
                .get(row, col)
                .map(|cell| KeyValueRepr::from_cell_value(cell.value.as_ref()))
                .unwrap_or(KeyValueRepr::None);
            if matches!(value, KeyValueRepr::None) {
                return false;
            }
        }
        true
    };

    let column_has_unique_values = |col: u32| -> bool {
        let mut seen: HashSet<KeyValueRepr> = HashSet::new();
        for row in data_start..grid.nrows {
            let value = grid
                .get(row, col)
                .map(|cell| KeyValueRepr::from_cell_value(cell.value.as_ref()))
                .unwrap_or(KeyValueRepr::None);
            if !seen.insert(value) {
                return false;
            }
        }
        true
    };

    let column_unique_ratio = |col: u32| -> f64 {
        let mut seen: HashSet<KeyValueRepr> = HashSet::new();
        for row in data_start..grid.nrows {
            let value = grid
                .get(row, col)
                .map(|cell| KeyValueRepr::from_cell_value(cell.value.as_ref()))
                .unwrap_or(KeyValueRepr::None);
            seen.insert(value);
        }
        if data_rows == 0 {
            return 0.0;
        }
        seen.len() as f64 / data_rows as f64
    };

    let mut unique_cols = Vec::new();
    let mut header_unique = Vec::new();
    let mut header_candidates = Vec::new();
    for col in 0..grid.ncols {
        if !column_non_empty(col) {
            continue;
        }
        if header_matches_key_pattern(col) {
            header_candidates.push(col);
        }
        if column_has_unique_values(col) {
            unique_cols.push(col);
            if header_matches_key_pattern(col) {
                header_unique.push(col);
            }
        }
    }

    if header_unique.len() == 1 {
        return vec![header_unique[0]];
    }
    if header_unique.len() > 1 {
        return Vec::new();
    }

    let min_unique_ratio = 0.5;
    let mut candidates: Vec<u32> = (0..grid.ncols)
        .filter(|&col| column_non_empty(col) && column_unique_ratio(col) >= min_unique_ratio)
        .collect();

    if candidates.is_empty() {
        return Vec::new();
    }

    if !header_candidates.is_empty() {
        candidates.retain(|col| header_candidates.contains(col) || !unique_cols.contains(col));
    }

    candidates.sort_unstable();
    const MAX_CANDIDATES: usize = 6;
    if candidates.len() > MAX_CANDIDATES {
        candidates.truncate(MAX_CANDIDATES);
    }

    let composite_unique = |cols: &[u32]| -> bool {
        let mut seen: HashSet<Vec<KeyValueRepr>> = HashSet::new();
        for row in data_start..grid.nrows {
            let mut key = Vec::with_capacity(cols.len());
            for &col in cols {
                let value = grid
                    .get(row, col)
                    .map(|cell| KeyValueRepr::from_cell_value(cell.value.as_ref()))
                    .unwrap_or(KeyValueRepr::None);
                if matches!(value, KeyValueRepr::None) {
                    return false;
                }
                key.push(value);
            }
            if !seen.insert(key) {
                return false;
            }
        }
        true
    };

    let mut unique_pairs = Vec::new();
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            let cols = vec![candidates[i], candidates[j]];
            if composite_unique(&cols) {
                unique_pairs.push(cols);
            }
        }
    }

    if !header_candidates.is_empty() {
        let mut preferred_pairs = Vec::new();
        for pair in &unique_pairs {
            if pair.iter().any(|col| header_candidates.contains(col)) {
                preferred_pairs.push(pair.clone());
            }
        }
        if preferred_pairs.len() == 1 {
            return preferred_pairs.swap_remove(0);
        }
        if preferred_pairs.len() > 1 {
            return Vec::new();
        }
    }

    if unique_cols.len() == 1 {
        return vec![unique_cols[0]];
    }
    if unique_cols.len() > 1 {
        return Vec::new();
    }

    if unique_pairs.len() == 1 {
        return unique_pairs.swap_remove(0);
    }
    if unique_pairs.len() > 1 {
        return Vec::new();
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string_pool::StringPool;
    use crate::workbook::CellValue;

    fn grid_from_rows(rows: &[&[i32]]) -> Grid {
        let nrows = rows.len() as u32;
        let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
        let mut grid = Grid::new(nrows, ncols);

        for (r_idx, row_vals) in rows.iter().enumerate() {
            for (c_idx, value) in row_vals.iter().enumerate() {
                grid.insert_cell(
                    r_idx as u32,
                    c_idx as u32,
                    Some(CellValue::Number(*value as f64)),
                    None,
                );
            }
        }

        grid
    }

    #[test]
    fn unique_keys_reorder_no_changes() {
        let grid_a = grid_from_rows(&[&[1, 10], &[2, 20], &[3, 30]]);
        let grid_b = grid_from_rows(&[&[3, 30], &[1, 10], &[2, 20]]);

        let alignment = diff_table_by_key(&grid_a, &grid_b, &[0]);
        assert_eq!(
            alignment.matched_rows,
            vec![(0, 1), (1, 2), (2, 0)],
            "all keys should align regardless of order"
        );
        assert!(alignment.left_only_rows.is_empty());
        assert!(alignment.right_only_rows.is_empty());
        assert!(alignment.duplicate_clusters.is_empty());
    }

    #[test]
    fn unique_keys_insert_delete_classified() {
        let grid_a = grid_from_rows(&[&[1, 10], &[2, 20]]);
        let grid_b = grid_from_rows(&[&[1, 10], &[2, 20], &[3, 30]]);

        let alignment = diff_table_by_key(&grid_a, &grid_b, &[0]);
        assert_eq!(alignment.matched_rows, vec![(0, 0), (1, 1)]);
        assert!(alignment.left_only_rows.is_empty());
        assert_eq!(alignment.right_only_rows, vec![2]);
        assert!(alignment.duplicate_clusters.is_empty());
    }

    #[test]
    fn duplicate_keys_form_cluster() {
        let grid_a = grid_from_rows(&[&[1, 10], &[1, 99]]);
        let grid_b = grid_from_rows(&[&[1, 10], &[1, 100]]);

        let alignment = diff_table_by_key(&grid_a, &grid_b, &[0]);
        assert_eq!(alignment.duplicate_clusters.len(), 1);
        let cluster = &alignment.duplicate_clusters[0];
        assert_eq!(cluster.left_rows, vec![0, 1]);
        assert_eq!(cluster.right_rows, vec![0, 1]);
    }

    #[test]
    fn composite_key_alignment_matches_rows_correctly() {
        let grid_a = grid_from_rows(&[&[1, 10, 100], &[1, 20, 200], &[2, 10, 300]]);
        let grid_b = grid_from_rows(&[&[1, 20, 200], &[2, 10, 300], &[1, 10, 100]]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 1]);

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
        let grid_a = grid_from_rows(&[&[1, 999, 10, 100], &[1, 888, 20, 200], &[2, 777, 10, 300]]);
        let grid_b = grid_from_rows(&[&[2, 777, 10, 300], &[1, 999, 10, 100], &[1, 888, 20, 200]]);

        let alignment =
            diff_table_by_key(&grid_a, &grid_b, &[0, 2]);

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
            diff_table_by_key(&grid_a, &grid_b, &[0, 1, 2]);

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
        assert!(
            !spec.is_key_column(1),
            "column 1 should not be a key column"
        );
        assert!(
            !spec.is_key_column(2),
            "column 2 should not be a key column"
        );
    }

    #[test]
    fn is_key_column_contiguous_columns() {
        let spec = KeyColumnSpec::new(vec![0, 1]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(spec.is_key_column(1), "column 1 should be a key column");
        assert!(
            !spec.is_key_column(2),
            "column 2 should not be a key column"
        );
        assert!(
            !spec.is_key_column(3),
            "column 3 should not be a key column"
        );
    }

    #[test]
    fn is_key_column_non_contiguous_columns() {
        let spec = KeyColumnSpec::new(vec![0, 2]);
        assert!(spec.is_key_column(0), "column 0 should be a key column");
        assert!(
            !spec.is_key_column(1),
            "column 1 should not be a key column"
        );
        assert!(spec.is_key_column(2), "column 2 should be a key column");
        assert!(
            !spec.is_key_column(3),
            "column 3 should not be a key column"
        );
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
        assert!(
            !spec.is_key_column(0),
            "column 0 should not be a key column"
        );
        assert!(spec.is_key_column(1), "column 1 should be a key column");
        assert!(
            !spec.is_key_column(2),
            "column 2 should not be a key column"
        );
        assert!(spec.is_key_column(3), "column 3 should be a key column");
        assert!(
            !spec.is_key_column(4),
            "column 4 should not be a key column"
        );
        assert!(spec.is_key_column(5), "column 5 should be a key column");
        assert!(
            !spec.is_key_column(6),
            "column 6 should not be a key column"
        );
    }

    #[test]
    fn suggest_key_columns_prefers_composite_when_single_not_unique() {
        let mut pool = StringPool::new();
        let mut grid = Grid::new(4, 3);

        let headers = ["Country", "CustomerID", "Amount"];
        for (col, header) in headers.iter().enumerate() {
            grid.insert_cell(
                0,
                col as u32,
                Some(CellValue::Text(pool.intern(header))),
                None,
            );
        }

        let rows = [
            ("US", 1.0, 100.0),
            ("US", 2.0, 200.0),
            ("CA", 1.0, 300.0),
        ];
        for (idx, (country, customer, amount)) in rows.iter().enumerate() {
            let row = (idx + 1) as u32;
            grid.insert_cell(
                row,
                0,
                Some(CellValue::Text(pool.intern(country))),
                None,
            );
            grid.insert_cell(row, 1, Some(CellValue::Number(*customer)), None);
            grid.insert_cell(row, 2, Some(CellValue::Number(*amount)), None);
        }

        let keys = suggest_key_columns(&grid, &pool);
        assert_eq!(keys, vec![0, 1], "composite key should be inferred");
    }

    #[test]
    fn suggest_key_columns_returns_empty_on_ambiguous_unique_columns() {
        let mut pool = StringPool::new();
        let mut grid = Grid::new(3, 2);

        let headers = ["ID", "SKU"];
        for (col, header) in headers.iter().enumerate() {
            grid.insert_cell(
                0,
                col as u32,
                Some(CellValue::Text(pool.intern(header))),
                None,
            );
        }

        let values = [(1.0, 10.0), (2.0, 11.0)];
        for (idx, (id, sku)) in values.iter().enumerate() {
            let row = (idx + 1) as u32;
            grid.insert_cell(row, 0, Some(CellValue::Number(*id)), None);
            grid.insert_cell(row, 1, Some(CellValue::Number(*sku)), None);
        }

        let keys = suggest_key_columns(&grid, &pool);
        assert!(keys.is_empty(), "multiple unique headers should be ambiguous");
    }
}
