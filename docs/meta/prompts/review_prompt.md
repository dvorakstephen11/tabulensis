# Codebase Context for Review

## Directory Structure

```text
/
  Cargo.lock
  Cargo.toml
  README.md
  core/
    Cargo.lock
    Cargo.toml
    src/
      addressing.rs
      column_alignment.rs
      container.rs
      database_alignment.rs
      datamashup.rs
      datamashup_framing.rs
      datamashup_package.rs
      diff.rs
      engine.rs
      excel_open_xml.rs
      grid_parser.rs
      grid_view.rs
      hashing.rs
      lib.rs
      m_diff.rs
      m_section.rs
      rect_block_move.rs
      row_alignment.rs
      workbook.rs
      output/
        json.rs
        mod.rs
    tests/
      addressing_pg2_tests.rs
      d1_database_mode_tests.rs
      data_mashup_tests.rs
      engine_tests.rs
      excel_open_xml_tests.rs
      g10_row_block_alignment_grid_workbook_tests.rs
      g11_row_block_move_grid_workbook_tests.rs
      g12_column_block_move_grid_workbook_tests.rs
      g12_rect_block_move_grid_workbook_tests.rs
      g13_fuzzy_row_move_grid_workbook_tests.rs
      g1_g2_grid_workbook_tests.rs
      g5_g7_grid_workbook_tests.rs
      g8_row_alignment_grid_workbook_tests.rs
      g9_column_alignment_grid_workbook_tests.rs
      grid_view_hashstats_tests.rs
      grid_view_tests.rs
      integration_test.rs
      m4_package_parts_tests.rs
      m4_permissions_metadata_tests.rs
      m5_query_domain_tests.rs
      m6_textual_m_diff_tests.rs
      m_section_splitting_tests.rs
      output_tests.rs
      pg1_ir_tests.rs
      pg3_snapshot_tests.rs
      pg4_diffop_tests.rs
      pg5_grid_diff_tests.rs
      pg6_object_vs_grid_tests.rs
      signature_tests.rs
      sparse_grid_tests.rs
      common/
        mod.rs
  fixtures/
    manifest.yaml
    pyproject.toml
    README.md
    requirements.txt
    generated/
      column_move_a.xlsx
      column_move_b.xlsx
      col_append_right_a.xlsx
      col_append_right_b.xlsx
      col_delete_middle_a.xlsx
      col_delete_middle_b.xlsx
      col_delete_right_a.xlsx
      col_delete_right_b.xlsx
      col_insert_middle_a.xlsx
      col_insert_middle_b.xlsx
      col_insert_with_edit_a.xlsx
      col_insert_with_edit_b.xlsx
      corrupt_base64.xlsx
      db_equal_ordered_a.xlsx
      db_equal_ordered_b.xlsx
      db_row_added_b.xlsx
      duplicate_datamashup_elements.xlsx
      duplicate_datamashup_parts.xlsx
      equal_sheet_a.xlsx
      equal_sheet_b.xlsx
      grid_large_dense.xlsx
      grid_large_noise.xlsx
      grid_move_and_edit_a.xlsx
      grid_move_and_edit_b.xlsx
      json_diff_bool_a.xlsx
      json_diff_bool_b.xlsx
      json_diff_single_cell_a.xlsx
      json_diff_single_cell_b.xlsx
      json_diff_value_to_empty_a.xlsx
      json_diff_value_to_empty_b.xlsx
      mashup_base64_whitespace.xlsx
      mashup_utf16_be.xlsx
      mashup_utf16_le.xlsx
      metadata_hidden_queries.xlsx
      metadata_missing_entry.xlsx
      metadata_orphan_entries.xlsx
      metadata_query_groups.xlsx
      metadata_simple.xlsx
      metadata_url_encoding.xlsx
      minimal.xlsx
      multi_cell_edits_a.xlsx
      multi_cell_edits_b.xlsx
      multi_query_with_embedded.xlsx
      m_add_query_a.xlsx
      m_add_query_b.xlsx
      m_change_literal_a.xlsx
      m_change_literal_b.xlsx
      m_def_and_metadata_change_a.xlsx
      m_def_and_metadata_change_b.xlsx
      m_metadata_only_change_a.xlsx
      m_metadata_only_change_b.xlsx
      m_remove_query_a.xlsx
      m_remove_query_b.xlsx
      m_rename_query_a.xlsx
      m_rename_query_b.xlsx
      not_a_zip.txt
      no_content_types.xlsx
      one_query.xlsx
      permissions_defaults.xlsx
      permissions_firewall_off.xlsx
      pg1_basic_two_sheets.xlsx
      pg1_empty_and_mixed_sheets.xlsx
      pg1_sparse_used_range.xlsx
      pg2_addressing_matrix.xlsx
      pg3_value_and_formula_cells.xlsx
      pg6_sheet_added_a.xlsx
      pg6_sheet_added_b.xlsx
      pg6_sheet_and_grid_change_a.xlsx
      pg6_sheet_and_grid_change_b.xlsx
      pg6_sheet_removed_a.xlsx
      pg6_sheet_removed_b.xlsx
      pg6_sheet_renamed_a.xlsx
      pg6_sheet_renamed_b.xlsx
      random_zip.zip
      rect_block_move_a.xlsx
      rect_block_move_b.xlsx
      row_append_bottom_a.xlsx
      row_append_bottom_b.xlsx
      row_block_delete_a.xlsx
      row_block_delete_b.xlsx
      row_block_insert_a.xlsx
      row_block_insert_b.xlsx
      row_block_move_a.xlsx
      row_block_move_b.xlsx
      row_delete_bottom_a.xlsx
      row_delete_bottom_b.xlsx
      row_delete_middle_a.xlsx
      row_delete_middle_b.xlsx
      row_insert_middle_a.xlsx
      row_insert_middle_b.xlsx
      row_insert_with_edit_a.xlsx
      row_insert_with_edit_b.xlsx
      sheet_case_only_rename_a.xlsx
      sheet_case_only_rename_b.xlsx
      sheet_case_only_rename_edit_a.xlsx
      sheet_case_only_rename_edit_b.xlsx
      single_cell_value_a.xlsx
      single_cell_value_b.xlsx
    src/
      generate.py
      __init__.py
      generators/
        base.py
        corrupt.py
        database.py
        grid.py
        mashup.py
        perf.py
        __init__.py
    templates/
      base_query.xlsx
  logs/
    2025-11-28b-diffop-pg4/
      activity_log.txt
```

## File Contents

### File: `.gitignore`

```
# Rust
target/
**/target/
**/*.rs.bk

# Python
__pycache__/
**/__pycache__/
.venv/
*.pyc
*.egg-info/

# Shared Generated Data
fixtures/generated/*.xlsx
fixtures/generated/*.pbix
fixtures/generated/*.zip
fixtures/generated/*.csv


# Docs
docs/meta/completion_estimates/
```

---

### File: `Cargo.toml`

```yaml
[workspace]
members = ["core"]
resolver = "2"
```

---

### File: `core\Cargo.toml`

```yaml
[package]
name = "excel_diff"
version = "0.1.0"
edition = "2024"

[lib]
name = "excel_diff"
path = "src/lib.rs"

[features]
default = ["excel-open-xml"]
excel-open-xml = []

[dependencies]
quick-xml = "0.32"
thiserror = "1.0"
zip = { version = "0.6", default-features = false, features = ["deflate"] }
base64 = "0.22"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
xxhash-rust = { version = "0.8", features = ["xxh64"] }

[dev-dependencies]
pretty_assertions = "1.4"
tempfile = "3.10"
```

---

### File: `core\src\addressing.rs`

```rust
//! Excel cell addressing utilities.
//!
//! Provides conversion between A1-style cell addresses (e.g., "B2", "AA10") and
//! zero-based (row, column) index pairs.

use std::fmt;

/// Error returned when parsing an invalid A1-style cell address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressParseError {
    pub input: String,
}

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid cell address: '{}'", self.input)
    }
}

impl std::error::Error for AddressParseError {}

/// Convert zero-based (row, col) indices to an Excel A1 address string.
pub fn index_to_address(row: u32, col: u32) -> String {
    let mut col_index = col;
    let mut col_label = String::new();

    loop {
        let rem = (col_index % 26) as u8;
        col_label.push((b'A' + rem) as char);
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }

    col_label.chars().rev().collect::<String>() + &(row + 1).to_string()
}

/// Parse an A1 address into zero-based (row, col) indices.
/// Returns `None` for malformed addresses.
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    if a1.is_empty() {
        return None;
    }

    let mut col: u32 = 0;
    let mut row: u32 = 0;
    let mut saw_letter = false;
    let mut saw_digit = false;

    for ch in a1.chars() {
        if ch.is_ascii_alphabetic() {
            saw_letter = true;
            if saw_digit {
                // Letters after digits are not allowed.
                return None;
            }
            let upper = ch.to_ascii_uppercase() as u8;
            if !upper.is_ascii_uppercase() {
                return None;
            }
            col = col
                .checked_mul(26)?
                .checked_add((upper - b'A' + 1) as u32)?;
        } else if ch.is_ascii_digit() {
            saw_digit = true;
            row = row.checked_mul(10)?.checked_add((ch as u8 - b'0') as u32)?;
        } else {
            return None;
        }
    }

    if !saw_letter || !saw_digit || row == 0 || col == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_address_examples() {
        assert_eq!(index_to_address(0, 0), "A1");
        assert_eq!(index_to_address(0, 25), "Z1");
        assert_eq!(index_to_address(0, 26), "AA1");
        assert_eq!(index_to_address(0, 27), "AB1");
        assert_eq!(index_to_address(0, 51), "AZ1");
        assert_eq!(index_to_address(0, 52), "BA1");
    }

    #[test]
    fn round_trip_addresses() {
        let addresses = [
            "A1", "B2", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1",
        ];
        for addr in addresses {
            let (r, c) = address_to_index(addr).expect("address should parse");
            assert_eq!(index_to_address(r, c), addr);
        }
    }

    #[test]
    fn invalid_addresses_rejected() {
        let invalid = ["", "1A", "A0", "A", "AA0", "A-1", "A1A"];
        for addr in invalid {
            assert!(address_to_index(addr).is_none(), "{addr} should be invalid");
        }
    }
}
```

---

### File: `core\src\column_alignment.rs`

```rust
use crate::grid_view::{ColHash, ColMeta, GridView, HashStats};
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnAlignment {
    pub(crate) matched: Vec<(u32, u32)>, // (col_idx_a, col_idx_b)
    pub(crate) inserted: Vec<u32>,       // columns present only in B
    pub(crate) deleted: Vec<u32>,        // columns present only in A
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnBlockMove {
    pub src_start_col: u32,
    pub dst_start_col: u32,
    pub col_count: u32,
}

const MAX_ALIGN_ROWS: u32 = 2_000;
const MAX_ALIGN_COLS: u32 = 64;
const MAX_HASH_REPEAT: u32 = 8;

pub(crate) fn detect_exact_column_block_move(old: &Grid, new: &Grid) -> Option<ColumnBlockMove> {
    if old.ncols != new.ncols || old.nrows != new.nrows {
        return None;
    }

    if old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new) {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    if blank_dominated(&view_a) || blank_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);
    if has_heavy_repetition(&stats) {
        return None;
    }

    let meta_a = &view_a.col_meta;
    let meta_b = &view_b.col_meta;
    let n = meta_a.len();

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let prefix = (0..n).find(|&idx| meta_a[idx].hash != meta_b[idx].hash)?;

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = n - 1 - suffix_len;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }
    let tail_start = n - suffix_len;

    let try_candidate = |src_start: usize, dst_start: usize| -> Option<ColumnBlockMove> {
        if src_start >= tail_start || dst_start >= tail_start {
            return None;
        }

        let mut len = 0usize;
        while src_start + len < tail_start && dst_start + len < tail_start {
            if meta_a[src_start + len].hash != meta_b[dst_start + len].hash {
                break;
            }
            len += 1;
        }

        if len == 0 {
            return None;
        }

        let src_end = src_start + len;
        let dst_end = dst_start + len;

        if !(src_end <= dst_start || dst_end <= src_start) {
            return None;
        }

        let mut idx_a = 0usize;
        let mut idx_b = 0usize;

        loop {
            if idx_a == src_start {
                idx_a = src_end;
            }
            if idx_b == dst_start {
                idx_b = dst_end;
            }

            if idx_a >= n && idx_b >= n {
                break;
            }

            if idx_a >= n || idx_b >= n {
                return None;
            }

            if meta_a[idx_a].hash != meta_b[idx_b].hash {
                return None;
            }

            idx_a += 1;
            idx_b += 1;
        }

        for meta in &meta_a[src_start..src_end] {
            if stats.freq_a.get(&meta.hash).copied().unwrap_or(0) != 1
                || stats.freq_b.get(&meta.hash).copied().unwrap_or(0) != 1
            {
                return None;
            }
        }

        Some(ColumnBlockMove {
            src_start_col: meta_a[src_start].col_idx,
            dst_start_col: meta_b[dst_start].col_idx,
            col_count: len as u32,
        })
    };

    if let Some(src_start) =
        (prefix..tail_start).find(|&idx| meta_a[idx].hash == meta_b[prefix].hash)
        && let Some(mv) = try_candidate(src_start, prefix)
    {
        return Some(mv);
    }

    if let Some(dst_start) =
        (prefix..tail_start).find(|&idx| meta_b[idx].hash == meta_a[prefix].hash)
        && let Some(mv) = try_candidate(prefix, dst_start)
    {
        return Some(mv);
    }

    None
}

pub(crate) fn align_single_column_change(old: &Grid, new: &Grid) -> Option<ColumnAlignment> {
    if !is_within_size_bounds(old, new) {
        return None;
    }

    if old.nrows != new.nrows {
        return None;
    }

    let col_diff = new.ncols as i64 - old.ncols as i64;
    if col_diff.abs() != 1 {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    let stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);
    if has_heavy_repetition(&stats) {
        return None;
    }

    if col_diff == 1 {
        find_single_gap_alignment(
            &view_a.col_meta,
            &view_b.col_meta,
            &stats,
            ColumnChange::Insert,
        )
    } else {
        find_single_gap_alignment(
            &view_a.col_meta,
            &view_b.col_meta,
            &stats,
            ColumnChange::Delete,
        )
    }
}

enum ColumnChange {
    Insert,
    Delete,
}

fn find_single_gap_alignment(
    cols_a: &[ColMeta],
    cols_b: &[ColMeta],
    stats: &HashStats<ColHash>,
    change: ColumnChange,
) -> Option<ColumnAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = false;

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < cols_a.len() && idx_b < cols_b.len() {
        let meta_a = cols_a[idx_a];
        let meta_b = cols_b[idx_b];

        if meta_a.hash == meta_b.hash {
            matched.push((meta_a.col_idx, meta_b.col_idx));
            idx_a += 1;
            idx_b += 1;
            continue;
        }

        if skipped {
            return None;
        }

        match change {
            ColumnChange::Insert => {
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.col_idx);
                idx_b += 1;
            }
            ColumnChange::Delete => {
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.col_idx);
                idx_a += 1;
            }
        }

        skipped = true;
    }

    if idx_a < cols_a.len() || idx_b < cols_b.len() {
        if skipped {
            return None;
        }

        match change {
            ColumnChange::Insert if idx_a == cols_a.len() && cols_b.len() == idx_b + 1 => {
                let meta_b = cols_b[idx_b];
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.col_idx);
            }
            ColumnChange::Delete if idx_b == cols_b.len() && cols_a.len() == idx_a + 1 => {
                let meta_a = cols_a[idx_a];
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.col_idx);
            }
            _ => return None,
        }
    }

    if inserted.len() + deleted.len() != 1 {
        return None;
    }

    let alignment = ColumnAlignment {
        matched,
        inserted,
        deleted,
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn is_monotonic(pairs: &[(u32, u32)]) -> bool {
    pairs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1)
}

fn is_unique_to_b(hash: ColHash, stats: &HashStats<ColHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 0
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
}

fn is_unique_to_a(hash: ColHash, stats: &HashStats<ColHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 0
}

fn is_within_size_bounds(old: &Grid, new: &Grid) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= MAX_ALIGN_ROWS && cols <= MAX_ALIGN_COLS
}

fn has_heavy_repetition(stats: &HashStats<ColHash>) -> bool {
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > MAX_HASH_REPEAT
}

fn blank_dominated(view: &GridView<'_>) -> bool {
    if view.col_meta.is_empty() {
        return false;
    }

    let blank_cols = view
        .col_meta
        .iter()
        .filter(|meta| meta.non_blank_count == 0)
        .count();

    blank_cols * 2 > view.col_meta.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::{Cell, CellAddress, CellValue};

    fn grid_from_numbers(rows: &[&[i32]]) -> Grid {
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
    fn single_insert_aligns_all_columns() {
        let base_rows: Vec<Vec<i32>> =
            vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8], vec![9, 10, 11, 12]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&base_refs);

        let inserted_rows: Vec<Vec<i32>> = base_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let mut new_row = row.clone();
                new_row.insert(2, 100 + idx as i32); // insert at index 2 (0-based)
                new_row
            })
            .collect();
        let inserted_refs: Vec<&[i32]> = inserted_rows.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&inserted_refs);

        let alignment =
            align_single_column_change(&grid_a, &grid_b).expect("alignment should succeed");

        assert_eq!(alignment.inserted, vec![2]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[1], (1, 1));
        assert_eq!(alignment.matched[2], (2, 3));
        assert_eq!(alignment.matched[3], (3, 4));
    }

    #[test]
    fn multiple_unique_columns_causes_bailout() {
        let base_rows: Vec<Vec<i32>> = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&base_refs);

        let mut rows_b: Vec<Vec<i32>> = base_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let mut new_row = row.clone();
                new_row.insert(1, 100 + idx as i32); // inserted column
                new_row
            })
            .collect();
        if let Some(cell) = rows_b.get_mut(1).and_then(|row| row.get_mut(3)) {
            *cell = 999;
        }
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&rows_b_refs);

        assert!(align_single_column_change(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn heavy_repetition_causes_bailout() {
        let repetitive_cols = 9;
        let rows: usize = 3;

        let values_a: Vec<Vec<i32>> = (0..rows).map(|_| vec![1; repetitive_cols]).collect();
        let refs_a: Vec<&[i32]> = values_a.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_numbers(&refs_a);

        let values_b: Vec<Vec<i32>> = (0..rows)
            .map(|row_idx| {
                let mut row = vec![1; repetitive_cols];
                row.insert(4, 2 + row_idx as i32);
                row
            })
            .collect();
        let refs_b: Vec<&[i32]> = values_b.iter().map(|r| r.as_slice()).collect();
        let grid_b = grid_from_numbers(&refs_b);

        assert!(align_single_column_change(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_simple_case() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

        let grid_b = grid_from_numbers(&[&[10, 30, 40, 20], &[11, 31, 41, 21]]);

        let mv =
            detect_exact_column_block_move(&grid_a, &grid_b).expect("expected column move found");
        assert_eq!(mv.src_start_col, 1);
        assert_eq!(mv.col_count, 1);
        assert_eq!(mv.dst_start_col, 3);
    }

    #[test]
    fn detect_exact_column_block_move_rejects_internal_edits() {
        let grid_a = grid_from_numbers(&[&[1, 2, 3, 4], &[5, 6, 7, 8], &[9, 10, 11, 12]]);

        let grid_b = grid_from_numbers(&[
            &[1, 3, 4, 2],
            &[5, 7, 8, 6],
            &[9, 11, 12, 999], // edit inside moved column
        ]);

        assert!(detect_exact_column_block_move(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_rejects_repetition() {
        let grid_a = grid_from_numbers(&[&[1, 1, 2, 2], &[10, 10, 20, 20]]);
        let grid_b = grid_from_numbers(&[&[2, 2, 1, 1], &[20, 20, 10, 10]]);

        assert!(detect_exact_column_block_move(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn detect_exact_column_block_move_multi_column_block() {
        let grid_a = grid_from_numbers(&[
            &[10, 20, 30, 40, 50, 60],
            &[11, 21, 31, 41, 51, 61],
            &[12, 22, 32, 42, 52, 62],
        ]);

        let grid_b = grid_from_numbers(&[
            &[10, 40, 50, 20, 30, 60],
            &[11, 41, 51, 21, 31, 61],
            &[12, 42, 52, 22, 32, 62],
        ]);

        let mv =
            detect_exact_column_block_move(&grid_a, &grid_b).expect("expected multi-column move");
        assert_eq!(mv.src_start_col, 3);
        assert_eq!(mv.col_count, 2);
        assert_eq!(mv.dst_start_col, 1);
    }

    #[test]
    fn detect_exact_column_block_move_rejects_two_independent_moves() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40, 50, 60], &[11, 21, 31, 41, 51, 61]]);

        let grid_b = grid_from_numbers(&[&[20, 10, 30, 40, 60, 50], &[21, 11, 31, 41, 61, 51]]);

        assert!(
            detect_exact_column_block_move(&grid_a, &grid_b).is_none(),
            "two independent column swaps must not be detected as a single block move"
        );
    }

    #[test]
    fn detect_exact_column_block_move_swap_as_single_move() {
        let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

        let grid_b = grid_from_numbers(&[&[20, 10, 30, 40], &[21, 11, 31, 41]]);

        let mv = detect_exact_column_block_move(&grid_a, &grid_b)
            .expect("swap of adjacent columns should be detected as single-column move");
        assert_eq!(mv.col_count, 1);
        assert!(
            (mv.src_start_col == 0 && mv.dst_start_col == 1)
                || (mv.src_start_col == 1 && mv.dst_start_col == 0),
            "swap should be represented as moving one column past the other"
        );
    }
}
```

---

### File: `core\src\container.rs`

```rust
//! OPC (Open Packaging Conventions) container handling.
//!
//! Provides abstraction over ZIP-based Office Open XML packages, validating
//! that required structural elements like `[Content_Types].xml` are present.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use zip::ZipArchive;
use zip::result::ZipError;

/// Errors that can occur when opening or reading an OPC container.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContainerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(String),
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
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        }
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.archive.file_names()
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```

---

### File: `core\src\database_alignment.rs`

```rust
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
```

---

### File: `core\src\datamashup.rs`

```rust
//! High-level DataMashup (Power Query) parsing and query extraction.
//!
//! Builds on the low-level framing and package parsing to provide structured
//! access to queries, permissions, and metadata stored in Excel DataMashup sections.

use std::collections::HashMap;

use crate::datamashup_framing::{DataMashupError, RawDataMashup};
use crate::datamashup_package::{PackageParts, parse_package_parts};
use crate::m_section::{SectionParseError, parse_section_members};
use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataMashup {
    pub version: u32,
    pub package_parts: PackageParts,
    pub permissions: Permissions,
    pub metadata: Metadata,
    pub permission_bindings_raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permissions {
    pub can_evaluate_future_packages: bool,
    pub firewall_enabled: bool,
    pub workbook_group_type: Option<String>,
}

impl Default for Permissions {
    fn default() -> Self {
        Permissions {
            can_evaluate_future_packages: false,
            firewall_enabled: true,
            workbook_group_type: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub formulas: Vec<QueryMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMetadata {
    pub item_path: String,
    pub section_name: String,
    pub formula_name: String,
    pub load_to_sheet: bool,
    pub load_to_model: bool,
    pub is_connection_only: bool,
    pub group_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub name: String,
    pub section_member: String,
    pub expression_m: String,
    pub metadata: QueryMetadata,
}

pub fn build_data_mashup(raw: &RawDataMashup) -> Result<DataMashup, DataMashupError> {
    let package_parts = parse_package_parts(&raw.package_parts)?;
    let permissions = parse_permissions(&raw.permissions);
    let metadata = parse_metadata(&raw.metadata)?;

    Ok(DataMashup {
        version: raw.version,
        package_parts,
        permissions,
        metadata,
        permission_bindings_raw: raw.permission_bindings.clone(),
    })
}

pub fn build_queries(dm: &DataMashup) -> Result<Vec<Query>, SectionParseError> {
    let members = parse_section_members(&dm.package_parts.main_section.source)?;

    let mut metadata_index: HashMap<(String, String), QueryMetadata> = HashMap::new();
    for meta in &dm.metadata.formulas {
        metadata_index.insert(
            (meta.section_name.clone(), meta.formula_name.clone()),
            meta.clone(),
        );
    }

    let mut positions: HashMap<String, usize> = HashMap::new();
    let mut queries = Vec::new();

    for member in members {
        let section_name = member.section_name.clone();
        let member_name = member.member_name.clone();
        let key = (section_name.clone(), member_name.clone());
        let metadata = metadata_index
            .get(&key)
            .cloned()
            .unwrap_or_else(|| QueryMetadata {
                item_path: format!("{}/{}", section_name, member_name),
                section_name: section_name.clone(),
                formula_name: member_name.clone(),
                load_to_sheet: false,
                load_to_model: false,
                is_connection_only: true,
                group_path: None,
            });

        let name = format!("{}/{}", section_name, member_name);
        let query = Query {
            name: name.clone(),
            section_member: member.member_name,
            expression_m: member.expression_m,
            metadata,
        };

        if let Some(idx) = positions.get(&name) {
            debug_assert!(
                false,
                "duplicate query name '{}' found in DataMashup section; \
                 later definition will overwrite earlier one",
                name
            );
            queries[*idx] = query;
        } else {
            positions.insert(name, queries.len());
            queries.push(query);
        }
    }

    Ok(queries)
}

pub fn parse_permissions(xml_bytes: &[u8]) -> Permissions {
    if xml_bytes.is_empty() {
        return Permissions::default();
    }

    let Ok(mut text) = String::from_utf8(xml_bytes.to_vec()) else {
        return Permissions::default();
    };
    if let Some(stripped) = text.strip_prefix('\u{FEFF}') {
        text = stripped.to_string();
    }

    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current_tag: Option<String> = None;
    let mut permissions = Permissions::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag =
                    Some(String::from_utf8_lossy(local_name(e.name().as_ref())).to_string());
            }
            Ok(Event::Text(t)) => {
                if let Some(tag) = current_tag.as_deref() {
                    let value = match t.unescape() {
                        Ok(v) => v.into_owned(),
                        Err(_) => {
                            // Any unescape failure means the permissions payload is unusable; fall back to defaults.
                            return Permissions::default();
                        }
                    };
                    match tag {
                        "CanEvaluateFuturePackages" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.can_evaluate_future_packages = v;
                            }
                        }
                        "FirewallEnabled" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.firewall_enabled = v;
                            }
                        }
                        "WorkbookGroupType" => {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() {
                                permissions.workbook_group_type = Some(trimmed.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(tag) = current_tag.as_deref() {
                    let value = String::from_utf8_lossy(&t.into_inner()).to_string();
                    match tag {
                        "CanEvaluateFuturePackages" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.can_evaluate_future_packages = v;
                            }
                        }
                        "FirewallEnabled" => {
                            if let Some(v) = parse_bool(&value) {
                                permissions.firewall_enabled = v;
                            }
                        }
                        "WorkbookGroupType" => {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() {
                                permissions.workbook_group_type = Some(trimmed.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => current_tag = None,
            Ok(Event::Eof) => break,
            Err(_) => return Permissions::default(),
            _ => {}
        }
        buf.clear();
    }

    permissions
}

pub fn parse_metadata(metadata_bytes: &[u8]) -> Result<Metadata, DataMashupError> {
    if metadata_bytes.is_empty() {
        return Ok(Metadata {
            formulas: Vec::new(),
        });
    }

    let xml_bytes = metadata_xml_bytes(metadata_bytes)?;
    let mut text = String::from_utf8(xml_bytes)
        .map_err(|_| DataMashupError::XmlError("metadata is not valid UTF-8".into()))?;
    if let Some(stripped) = text.strip_prefix('\u{FEFF}') {
        text = stripped.to_string();
    }

    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut element_stack: Vec<String> = Vec::new();
    let mut item_type: Option<String> = None;
    let mut item_path: Option<String> = None;
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut formulas: Vec<QueryMetadata> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(local_name(e.name().as_ref())).to_string();
                if name == "Entry"
                    && let Some((typ, val)) = parse_entry_attributes(&e)?
                {
                    entries.push((typ, val));
                }
            }
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(local_name(e.name().as_ref())).to_string();
                if name == "Item" {
                    item_type = None;
                    item_path = None;
                    entries.clear();
                }
                if name == "Entry"
                    && let Some((typ, val)) = parse_entry_attributes(&e)?
                {
                    entries.push((typ, val));
                }
                element_stack.push(name);
            }
            Ok(Event::Text(t)) => {
                if let Some(tag) = element_stack.last() {
                    let value = t
                        .unescape()
                        .map_err(|e| DataMashupError::XmlError(e.to_string()))?
                        .into_owned();
                    match tag.as_str() {
                        "ItemType" => {
                            item_type = Some(value.trim().to_string());
                        }
                        "ItemPath" => {
                            item_path = Some(value.trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(tag) = element_stack.last() {
                    let value = String::from_utf8_lossy(&t.into_inner()).to_string();
                    match tag.as_str() {
                        "ItemType" => {
                            item_type = Some(value.trim().to_string());
                        }
                        "ItemPath" => {
                            item_path = Some(value.trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name_bytes = local_name(e.name().as_ref()).to_vec();
                if name_bytes.as_slice() == b"Item" && item_type.as_deref() == Some("Formula") {
                    let raw_path = item_path.clone().ok_or_else(|| {
                        DataMashupError::XmlError("Formula item missing ItemPath".into())
                    })?;
                    let decoded_path = decode_item_path(&raw_path)?;
                    let (section_name, formula_name) = split_item_path(&decoded_path)?;
                    let load_to_sheet =
                        entry_bool(&entries, &["FillEnabled", "LoadEnabled"]).unwrap_or(false);
                    let load_to_model = entry_bool(
                        &entries,
                        &[
                            "FillToDataModelEnabled",
                            "AddedToDataModel",
                            "LoadToDataModel",
                        ],
                    )
                    .unwrap_or(false);
                    // Group paths are derived solely from per-formula entries for now; the AllFormulas tree is not parsed yet.
                    let group_path = entry_string(
                        &entries,
                        &[
                            "QueryGroupId",
                            "QueryGroupID",
                            "QueryGroupPath",
                            "QueryGroup",
                        ],
                    );

                    let metadata = QueryMetadata {
                        item_path: decoded_path.clone(),
                        section_name,
                        formula_name,
                        load_to_sheet,
                        load_to_model,
                        is_connection_only: !(load_to_sheet || load_to_model),
                        group_path,
                    };
                    formulas.push(metadata);
                }

                if let Some(last) = element_stack.last()
                    && last.as_bytes() == name_bytes.as_slice()
                {
                    element_stack.pop();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DataMashupError::XmlError(e.to_string())),
            _ => {}
        }

        buf.clear();
    }

    Ok(Metadata { formulas })
}

fn metadata_xml_bytes(metadata_bytes: &[u8]) -> Result<Vec<u8>, DataMashupError> {
    if looks_like_xml(metadata_bytes) {
        return Ok(metadata_bytes.to_vec());
    }

    if metadata_bytes.len() >= 8 {
        let content_len = u32::from_le_bytes(metadata_bytes[0..4].try_into().unwrap()) as usize;
        let xml_len = u32::from_le_bytes(metadata_bytes[4..8].try_into().unwrap()) as usize;
        let start = 8usize
            .checked_add(content_len)
            .ok_or_else(|| DataMashupError::XmlError("metadata length overflow".into()))?;
        let end = start
            .checked_add(xml_len)
            .ok_or_else(|| DataMashupError::XmlError("metadata length overflow".into()))?;
        if end <= metadata_bytes.len() {
            return Ok(metadata_bytes[start..end].to_vec());
        }
        return Err(DataMashupError::XmlError(
            "metadata length prefix invalid".into(),
        ));
    }

    Err(DataMashupError::XmlError("metadata XML not found".into()))
}

fn looks_like_xml(bytes: &[u8]) -> bool {
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    if idx >= bytes.len() {
        return false;
    }

    let slice = &bytes[idx..];
    slice.starts_with(b"<")
        || slice.starts_with(&[0xEF, 0xBB, 0xBF])
        || slice.starts_with(&[0xFE, 0xFF])
        || slice.starts_with(&[0xFF, 0xFE])
}

fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..).unwrap_or(name),
        None => name,
    }
}

fn parse_bool(text: &str) -> Option<bool> {
    let trimmed = text.trim();
    let payload = trimmed
        .strip_prefix(|c| c == 'l' || c == 'L')
        .unwrap_or(trimmed);
    let lowered = payload.to_ascii_lowercase();
    match lowered.as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn parse_entry_attributes(
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<Option<(String, String)>, DataMashupError> {
    let mut typ: Option<String> = None;
    let mut value: Option<String> = None;

    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|e| DataMashupError::XmlError(e.to_string()))?;
        let key = local_name(attr.key.as_ref());
        if key == b"Type" {
            typ = Some(
                String::from_utf8(attr.value.as_ref().to_vec())
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?,
            );
        } else if key == b"Value" {
            value = Some(
                String::from_utf8(attr.value.as_ref().to_vec())
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?,
            );
        }
    }

    match (typ, value) {
        (Some(t), Some(v)) => Ok(Some((t, v))),
        _ => Ok(None),
    }
}

fn entry_bool(entries: &[(String, String)], keys: &[&str]) -> Option<bool> {
    for (key, val) in entries {
        if keys.iter().any(|k| k.eq_ignore_ascii_case(key))
            && let Some(b) = parse_bool(val)
        {
            return Some(b);
        }
    }
    None
}

fn entry_string(entries: &[(String, String)], keys: &[&str]) -> Option<String> {
    for (key, val) in entries {
        if keys.iter().any(|k| k.eq_ignore_ascii_case(key)) {
            let trimmed = val.trim();
            let without_prefix = trimmed
                .strip_prefix('s')
                .or_else(|| trimmed.strip_prefix('S'))
                .unwrap_or(trimmed);
            if without_prefix.is_empty() {
                return None;
            }
            return Some(without_prefix.to_string());
        }
    }
    None
}

fn decode_item_path(path: &str) -> Result<String, DataMashupError> {
    let mut decoded = Vec::with_capacity(path.len());
    let bytes = path.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b == b'%' {
            if idx + 2 >= bytes.len() {
                return Err(DataMashupError::XmlError(
                    "invalid percent-encoding in ItemPath".into(),
                ));
            }
            let hi = hex_value(bytes[idx + 1]).ok_or_else(|| {
                DataMashupError::XmlError("invalid percent-encoding in ItemPath".into())
            })?;
            let lo = hex_value(bytes[idx + 2]).ok_or_else(|| {
                DataMashupError::XmlError("invalid percent-encoding in ItemPath".into())
            })?;
            decoded.push(hi << 4 | lo);
            idx += 3;
            continue;
        }
        decoded.push(b);
        idx += 1;
    }
    String::from_utf8(decoded)
        .map_err(|_| DataMashupError::XmlError("invalid UTF-8 in ItemPath".into()))
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + b - b'a'),
        b'A'..=b'F' => Some(10 + b - b'A'),
        _ => None,
    }
}

fn split_item_path(path: &str) -> Result<(String, String), DataMashupError> {
    let mut parts = path.split('/');
    let section = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    if section.is_empty() || rest.is_empty() {
        return Err(DataMashupError::XmlError(
            "invalid ItemPath in metadata".into(),
        ));
    }
    let formula = rest.join("/");
    Ok((section.to_string(), formula))
}
```

---

### File: `core\src\datamashup_framing.rs`

```rust
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use quick_xml::Reader;
use quick_xml::events::Event;
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
    const MIN_SIZE: usize = 4 + 4 * 4;
    if bytes.len() < MIN_SIZE {
        return Err(DataMashupError::FramingInvalid);
    }

    let mut offset: usize = 0;
    let version = read_u32_at(bytes, offset).ok_or(DataMashupError::FramingInvalid)?;
    offset += 4;

    if version != 0 {
        return Err(DataMashupError::UnsupportedVersion(version));
    }

    let package_parts_len = read_length(bytes, offset)?;
    offset += 4;
    let package_parts = take_segment(bytes, &mut offset, package_parts_len)?;

    let permissions_len = read_length(bytes, offset)?;
    offset += 4;
    let permissions = take_segment(bytes, &mut offset, permissions_len)?;

    let metadata_len = read_length(bytes, offset)?;
    offset += 4;
    let metadata = take_segment(bytes, &mut offset, metadata_len)?;

    let permission_bindings_len = read_length(bytes, offset)?;
    offset += 4;
    let permission_bindings = take_segment(bytes, &mut offset, permission_bindings_len)?;

    if offset != bytes.len() {
        return Err(DataMashupError::FramingInvalid);
    }

    Ok(RawDataMashup {
        version,
        package_parts,
        permissions,
        metadata,
        permission_bindings,
    })
}

pub fn read_datamashup_text(xml: &[u8]) -> Result<Option<String>, DataMashupError> {
    let utf8_xml = decode_datamashup_xml(xml)?;

    let mut reader = Reader::from_reader(utf8_xml.as_deref().unwrap_or(xml));
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut found_content: Option<String> = None;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup || found_content.is_some() {
                    return Err(DataMashupError::FramingInvalid);
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t
                    .unescape()
                    .map_err(|e| DataMashupError::XmlError(e.to_string()))?
                    .into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                let data = t.into_inner();
                content.push_str(&String::from_utf8_lossy(&data));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return Err(DataMashupError::FramingInvalid);
                }
                in_datamashup = false;
                found_content = Some(content.clone());
            }
            Ok(Event::Eof) if in_datamashup => {
                return Err(DataMashupError::FramingInvalid);
            }
            Ok(Event::Eof) => return Ok(found_content),
            Err(e) => return Err(DataMashupError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
}

pub fn decode_datamashup_base64(text: &str) -> Result<Vec<u8>, DataMashupError> {
    let cleaned: String = text.split_whitespace().collect();
    STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| DataMashupError::Base64Invalid)
}

pub(crate) fn decode_datamashup_xml(xml: &[u8]) -> Result<Option<Vec<u8>>, DataMashupError> {
    if xml.starts_with(&[0xFF, 0xFE]) {
        return Ok(Some(decode_utf16_xml(xml, true, true)?));
    }
    if xml.starts_with(&[0xFE, 0xFF]) {
        return Ok(Some(decode_utf16_xml(xml, false, true)?));
    }

    decode_declared_utf16_without_bom(xml)
}

fn decode_declared_utf16_without_bom(xml: &[u8]) -> Result<Option<Vec<u8>>, DataMashupError> {
    let attempt_decode = |little_endian| -> Result<Option<Vec<u8>>, DataMashupError> {
        if !looks_like_utf16(xml, little_endian) {
            return Ok(None);
        }
        let decoded = decode_utf16_xml(xml, little_endian, false)?;
        let lower = String::from_utf8_lossy(&decoded).to_ascii_lowercase();
        if lower.contains("encoding=\"utf-16\"") || lower.contains("encoding='utf-16'") {
            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    };

    if let Some(decoded) = attempt_decode(true)? {
        return Ok(Some(decoded));
    }
    attempt_decode(false)
}

fn looks_like_utf16(xml: &[u8], little_endian: bool) -> bool {
    if xml.len() < 4 {
        return false;
    }

    if little_endian {
        xml[0] == b'<' && xml[1] == 0 && xml[2] == b'?' && xml[3] == 0
    } else {
        xml[0] == 0 && xml[1] == b'<' && xml[2] == 0 && xml[3] == b'?'
    }
}

fn decode_utf16_xml(
    xml: &[u8],
    little_endian: bool,
    has_bom: bool,
) -> Result<Vec<u8>, DataMashupError> {
    let start = if has_bom { 2 } else { 0 };
    let body = xml
        .get(start..)
        .ok_or_else(|| DataMashupError::XmlError("invalid UTF-16 XML".into()))?;
    if body.len() % 2 != 0 {
        return Err(DataMashupError::XmlError(
            "invalid UTF-16 byte length".into(),
        ));
    }

    let mut code_units = Vec::with_capacity(body.len() / 2);
    for chunk in body.chunks_exact(2) {
        let unit = if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        code_units.push(unit);
    }

    let utf8 = String::from_utf16(&code_units)
        .map_err(|_| DataMashupError::XmlError("invalid UTF-16 XML".into()))?;
    Ok(utf8.into_bytes())
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    let array: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_le_bytes(array))
}

fn read_length(bytes: &[u8], offset: usize) -> Result<usize, DataMashupError> {
    let len = read_u32_at(bytes, offset).ok_or(DataMashupError::FramingInvalid)?;
    usize::try_from(len).map_err(|_| DataMashupError::FramingInvalid)
}

fn take_segment(bytes: &[u8], offset: &mut usize, len: usize) -> Result<Vec<u8>, DataMashupError> {
    let start = *offset;
    let end = start
        .checked_add(len)
        .ok_or(DataMashupError::FramingInvalid)?;
    if end > bytes.len() {
        return Err(DataMashupError::FramingInvalid);
    }

    let segment = bytes[start..end].to_vec();
    *offset = end;
    Ok(segment)
}

#[cfg(test)]
mod tests {
    use super::{
        DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup,
        read_datamashup_text,
    };

    fn build_dm_bytes(
        version: u32,
        package_parts: &[u8],
        permissions: &[u8],
        metadata: &[u8],
        permission_bindings: &[u8],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&(package_parts.len() as u32).to_le_bytes());
        bytes.extend_from_slice(package_parts);
        bytes.extend_from_slice(&(permissions.len() as u32).to_le_bytes());
        bytes.extend_from_slice(permissions);
        bytes.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
        bytes.extend_from_slice(metadata);
        bytes.extend_from_slice(&(permission_bindings.len() as u32).to_le_bytes());
        bytes.extend_from_slice(permission_bindings);
        bytes
    }

    #[test]
    fn parse_zero_length_stream_succeeds() {
        let bytes = build_dm_bytes(0, b"", b"", b"", b"");
        let parsed = parse_data_mashup(&bytes).expect("zero-length sections should parse");
        assert_eq!(
            parsed,
            RawDataMashup {
                version: 0,
                package_parts: Vec::new(),
                permissions: Vec::new(),
                metadata: Vec::new(),
                permission_bindings: Vec::new(),
            }
        );
    }

    #[test]
    fn parse_basic_non_zero_lengths() {
        let bytes = build_dm_bytes(0, b"AAAA", b"BBBB", b"CCCC", b"DDDD");
        let parsed = parse_data_mashup(&bytes).expect("non-zero lengths should parse");
        assert_eq!(parsed.version, 0);
        assert_eq!(parsed.package_parts, b"AAAA");
        assert_eq!(parsed.permissions, b"BBBB");
        assert_eq!(parsed.metadata, b"CCCC");
        assert_eq!(parsed.permission_bindings, b"DDDD");
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let bytes = build_dm_bytes(1, b"AAAA", b"BBBB", b"CCCC", b"DDDD");
        let err = parse_data_mashup(&bytes).expect_err("version 1 should be unsupported");
        assert!(matches!(err, DataMashupError::UnsupportedVersion(1)));
    }

    #[test]
    fn truncated_stream_errors() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&100u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        let err = parse_data_mashup(&bytes).expect_err("length overflows buffer");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn trailing_bytes_are_invalid() {
        let mut bytes = build_dm_bytes(0, b"", b"", b"", b"");
        bytes.push(0xFF);
        let err = parse_data_mashup(&bytes).expect_err("trailing bytes should fail");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn too_short_stream_is_framing_invalid() {
        let bytes = vec![0u8; 8];
        let err =
            parse_data_mashup(&bytes).expect_err("buffer shorter than header must be invalid");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn utf16_datamashup_xml_decodes_correctly() {
        let xml_text = r#"<?xml version="1.0" encoding="utf-16"?><root xmlns:dm="http://schemas.microsoft.com/DataMashup"><dm:DataMashup>QQ==</dm:DataMashup></root>"#;
        let mut xml_bytes = Vec::with_capacity(2 + xml_text.len() * 2);
        xml_bytes.extend_from_slice(&[0xFF, 0xFE]);
        for unit in xml_text.encode_utf16() {
            xml_bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let text = read_datamashup_text(&xml_bytes)
            .expect("UTF-16 XML should parse")
            .expect("DataMashup element should be found");
        assert_eq!(text.trim(), "QQ==");
    }

    #[test]
    fn utf16_without_bom_with_declared_encoding_parses() {
        let xml_text = r#"<?xml version="1.0" encoding="utf-16"?><root xmlns:dm="http://schemas.microsoft.com/DataMashup"><dm:DataMashup>QQ==</dm:DataMashup></root>"#;
        for &little_endian in &[true, false] {
            let mut xml_bytes = Vec::with_capacity(xml_text.len() * 2);
            for unit in xml_text.encode_utf16() {
                let bytes = if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                xml_bytes.extend_from_slice(&bytes);
            }

            let text = read_datamashup_text(&xml_bytes)
                .expect("UTF-16 XML without BOM should parse when declared")
                .expect("DataMashup element should be found");
            assert_eq!(text.trim(), "QQ==");
        }
    }

    #[test]
    fn elements_with_datamashup_suffix_are_ignored() {
        let xml = br#"<?xml version="1.0"?><root><FooDataMashup>QQ==</FooDataMashup></root>"#;
        let result = read_datamashup_text(xml).expect("parsing should succeed");
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_sibling_datamashup_elements_error() {
        let xml = br#"<?xml version="1.0"?>
<root xmlns:dm="http://schemas.microsoft.com/DataMashup">
  <dm:DataMashup>QQ==</dm:DataMashup>
  <dm:DataMashup>QQ==</dm:DataMashup>
</root>"#;
        let err = read_datamashup_text(xml).expect_err("duplicate DataMashup elements should fail");
        assert!(matches!(err, DataMashupError::FramingInvalid));
    }

    #[test]
    fn decode_datamashup_base64_rejects_invalid() {
        let err = decode_datamashup_base64("!!!").expect_err("invalid base64 should fail");
        assert!(matches!(err, DataMashupError::Base64Invalid));
    }

    #[test]
    fn fuzz_style_never_panics() {
        for seed in 0u64..32 {
            let len = (seed as usize * 7 % 48) + (seed as usize % 5);
            let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut bytes = Vec::with_capacity(len);
            for _ in 0..len {
                state = state
                    .wrapping_mul(2862933555777941757)
                    .wrapping_add(3037000493);
                bytes.push((state >> 32) as u8);
            }
            let _ = parse_data_mashup(&bytes);
        }
    }
}
```

---

### File: `core\src\datamashup_package.rs`

```rust
use crate::datamashup_framing::DataMashupError;
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageXml {
    pub raw_xml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDocument {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedContent {
    /// Normalized PackageParts path for the embedded package (never starts with '/').
    pub name: String,
    pub section: SectionDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageParts {
    pub package_xml: PackageXml,
    pub main_section: SectionDocument,
    pub embedded_contents: Vec<EmbeddedContent>,
}

pub fn parse_package_parts(bytes: &[u8]) -> Result<PackageParts, DataMashupError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|_| DataMashupError::FramingInvalid)?;

    let mut package_xml: Option<PackageXml> = None;
    let mut main_section: Option<SectionDocument> = None;
    let mut embedded_contents: Vec<EmbeddedContent> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|_| DataMashupError::FramingInvalid)?;
        if file.is_dir() {
            continue;
        }

        let raw_name = file.name().to_string();
        let name = normalize_path(&raw_name);
        if package_xml.is_none() && name == "Config/Package.xml" {
            let text = read_file_to_string(&mut file)?;
            package_xml = Some(PackageXml { raw_xml: text });
            continue;
        }
        if main_section.is_none() && name == "Formulas/Section1.m" {
            let text = strip_leading_bom(read_file_to_string(&mut file)?);
            main_section = Some(SectionDocument { source: text });
            continue;
        }
        if name.starts_with("Content/") {
            let mut content_bytes = Vec::new();
            if file.read_to_end(&mut content_bytes).is_err() {
                continue;
            }

            if let Some(section) = extract_embedded_section(&content_bytes) {
                embedded_contents.push(EmbeddedContent {
                    name: normalize_path(&raw_name).to_string(),
                    section: SectionDocument { source: section },
                });
            }
        }
    }

    let package_xml = package_xml.ok_or(DataMashupError::FramingInvalid)?;
    let main_section = main_section.ok_or(DataMashupError::FramingInvalid)?;

    Ok(PackageParts {
        package_xml,
        main_section,
        embedded_contents,
    })
}

fn normalize_path(name: &str) -> &str {
    name.trim_start_matches('/')
}

fn read_file_to_string(file: &mut zip::read::ZipFile<'_>) -> Result<String, DataMashupError> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|_| DataMashupError::FramingInvalid)?;
    String::from_utf8(buf).map_err(|_| DataMashupError::FramingInvalid)
}

fn extract_embedded_section(bytes: &[u8]) -> Option<String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).ok()?;
    find_section_document(&mut archive)
}

fn find_section_document<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    for idx in 0..archive.len() {
        let mut file = match archive.by_index(idx) {
            Ok(file) => file,
            Err(_) => continue,
        };
        if file.is_dir() {
            continue;
        }

        if normalize_path(file.name()) == "Formulas/Section1.m" {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                let text = String::from_utf8(buf).ok()?;
                return Some(strip_leading_bom(text));
            }
        }
    }
    None
}

fn strip_leading_bom(text: String) -> String {
    text.strip_prefix('\u{FEFF}')
        .map(|s| s.to_string())
        .unwrap_or(text)
}
```

---

### File: `core\src\diff.rs`

```rust
//! Diff operations and reports for workbook comparison.
//!
//! This module defines the types used to represent differences between two workbooks:
//! - [`DiffOp`]: Individual operations representing a single change (cell edit, row/column add/remove, etc.)
//! - [`DiffReport`]: A versioned collection of diff operations

use crate::workbook::{CellAddress, CellSnapshot, ColSignature, RowSignature};

pub type SheetId = String;

/// A single diff operation representing one logical change between workbooks.
///
/// Operations are emitted by the diff engine and collected into a [`DiffReport`].
/// The enum is marked `#[non_exhaustive]` to allow future additions.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum DiffOp {
    SheetAdded {
        sheet: SheetId,
    },
    SheetRemoved {
        sheet: SheetId,
    },
    RowAdded {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    RowRemoved {
        sheet: SheetId,
        row_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        row_signature: Option<RowSignature>,
    },
    ColumnAdded {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    ColumnRemoved {
        sheet: SheetId,
        col_idx: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        col_signature: Option<ColSignature>,
    },
    BlockMovedRows {
        sheet: SheetId,
        src_start_row: u32,
        row_count: u32,
        dst_start_row: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    BlockMovedColumns {
        sheet: SheetId,
        src_start_col: u32,
        col_count: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    BlockMovedRect {
        sheet: SheetId,
        src_start_row: u32,
        src_row_count: u32,
        src_start_col: u32,
        src_col_count: u32,
        dst_start_row: u32,
        dst_start_col: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_hash: Option<u64>,
    },
    /// Logical change to a single cell.
    ///
    /// Invariants (maintained by producers and tests, not by the type system):
    /// - `addr` is the canonical location for the edit.
    /// - `from.addr` and `to.addr` must both equal `addr`.
    /// - `CellSnapshot` equality intentionally ignores `addr` and compares only
    ///   `(value, formula)`, so `DiffOp::CellEdited` equality does not by itself
    ///   enforce the address invariants; callers must respect them when
    ///   constructing ops.
    CellEdited {
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    },
}

/// A versioned collection of diff operations between two workbooks.
///
/// The `version` field indicates the schema version for forwards compatibility.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiffReport {
    /// Schema version (currently "1").
    pub version: String,
    /// The list of diff operations.
    pub ops: Vec<DiffOp>,
}

impl DiffReport {
    pub const SCHEMA_VERSION: &'static str = "1";

    pub fn new(ops: Vec<DiffOp>) -> DiffReport {
        DiffReport {
            version: Self::SCHEMA_VERSION.to_string(),
            ops,
        }
    }
}

impl DiffOp {
    pub fn cell_edited(
        sheet: SheetId,
        addr: CellAddress,
        from: CellSnapshot,
        to: CellSnapshot,
    ) -> DiffOp {
        debug_assert_eq!(from.addr, addr, "from.addr must match canonical addr");
        debug_assert_eq!(to.addr, addr, "to.addr must match canonical addr");
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        }
    }

    pub fn row_added(sheet: SheetId, row_idx: u32, row_signature: Option<RowSignature>) -> DiffOp {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn row_removed(
        sheet: SheetId,
        row_idx: u32,
        row_signature: Option<RowSignature>,
    ) -> DiffOp {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        }
    }

    pub fn column_added(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        }
    }

    pub fn column_removed(
        sheet: SheetId,
        col_idx: u32,
        col_signature: Option<ColSignature>,
    ) -> DiffOp {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        }
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

    #[allow(clippy::too_many_arguments)]
    pub fn block_moved_rect(
        sheet: SheetId,
        src_start_row: u32,
        src_row_count: u32,
        src_start_col: u32,
        src_col_count: u32,
        dst_start_row: u32,
        dst_start_col: u32,
        block_hash: Option<u64>,
    ) -> DiffOp {
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash,
        }
    }
}
```

---

### File: `core\src\engine.rs`

```rust
//! Core diffing engine for workbook comparison.
//!
//! Provides the main entry point [`diff_workbooks`] for comparing two workbooks
//! and generating a [`DiffReport`] of all changes.

use crate::column_alignment::{
    ColumnAlignment, ColumnBlockMove, align_single_column_change, detect_exact_column_block_move,
};
use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};
use crate::diff::{DiffOp, DiffReport, SheetId};
use crate::rect_block_move::{RectBlockMove, detect_exact_rect_block_move};
use crate::row_alignment::{
    RowAlignment, RowBlockMove, align_row_changes, detect_exact_row_block_move,
    detect_fuzzy_row_block_move,
};
use crate::workbook::{Cell, CellAddress, CellSnapshot, Grid, Sheet, SheetKind, Workbook};
use std::collections::HashMap;

const DATABASE_MODE_SHEET_ID: &str = "<database>";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

fn make_sheet_key(sheet: &Sheet) -> SheetKey {
    SheetKey {
        name_lower: sheet.name.to_lowercase(),
        kind: sheet.kind.clone(),
    }
}

fn sheet_kind_order(kind: &SheetKind) -> u8 {
    match kind {
        SheetKind::Worksheet => 0,
        SheetKind::Chart => 1,
        SheetKind::Macro => 2,
        SheetKind::Other => 3,
    }
}

pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport {
    let mut ops = Vec::new();

    let mut old_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &old.sheets {
        let key = make_sheet_key(sheet);
        let was_unique = old_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in old workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut new_sheets: HashMap<SheetKey, &Sheet> = HashMap::new();
    for sheet in &new.sheets {
        let key = make_sheet_key(sheet);
        let was_unique = new_sheets.insert(key.clone(), sheet).is_none();
        debug_assert!(
            was_unique,
            "duplicate sheet identity in new workbook: ({}, {:?})",
            key.name_lower, key.kind
        );
    }

    let mut all_keys: Vec<SheetKey> = old_sheets
        .keys()
        .chain(new_sheets.keys())
        .cloned()
        .collect();
    all_keys.sort_by(|a, b| match a.name_lower.cmp(&b.name_lower) {
        std::cmp::Ordering::Equal => sheet_kind_order(&a.kind).cmp(&sheet_kind_order(&b.kind)),
        other => other,
    });
    all_keys.dedup();

    for key in all_keys {
        match (old_sheets.get(&key), new_sheets.get(&key)) {
            (None, Some(new_sheet)) => {
                ops.push(DiffOp::SheetAdded {
                    sheet: new_sheet.name.clone(),
                });
            }
            (Some(old_sheet), None) => {
                ops.push(DiffOp::SheetRemoved {
                    sheet: old_sheet.name.clone(),
                });
            }
            (Some(old_sheet), Some(new_sheet)) => {
                let sheet_id: SheetId = old_sheet.name.clone();
                diff_grids(&sheet_id, &old_sheet.grid, &new_sheet.grid, &mut ops);
            }
            (None, None) => unreachable!(),
        }
    }

    DiffReport::new(ops)
}

pub fn diff_grids_database_mode(old: &Grid, new: &Grid, key_columns: &[u32]) -> DiffReport {
    let spec = KeyColumnSpec::new(key_columns.to_vec());
    let alignment = match diff_table_by_key(old, new, key_columns) {
        Ok(alignment) => alignment,
        Err(_) => {
            let mut ops = Vec::new();
            let sheet_id: SheetId = DATABASE_MODE_SHEET_ID.to_string();
            diff_grids(&sheet_id, old, new, &mut ops);
            return DiffReport::new(ops);
        }
    };

    let mut ops = Vec::new();
    let sheet_id: SheetId = DATABASE_MODE_SHEET_ID.to_string();
    let max_cols = old.ncols.max(new.ncols);

    for row_idx in &alignment.left_only_rows {
        ops.push(DiffOp::row_removed(sheet_id.clone(), *row_idx, None));
    }

    for row_idx in &alignment.right_only_rows {
        ops.push(DiffOp::row_added(sheet_id.clone(), *row_idx, None));
    }

    for (row_a, row_b) in &alignment.matched_rows {
        for col in 0..max_cols {
            if spec.is_key_column(col) {
                continue;
            }

            let addr = CellAddress::from_indices(*row_b, col);
            let from = snapshot_with_addr(old.get(*row_a, col), addr);
            let to = snapshot_with_addr(new.get(*row_b, col), addr);

            if from != to {
                ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
            }
        }
    }

    DiffReport::new(ops)
}

fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    if let Some(mv) = detect_exact_rect_block_move(old, new) {
        emit_rect_block_move(sheet_id, mv, ops);
        return;
    }

    if let Some(mv) = detect_exact_row_block_move(old, new) {
        emit_row_block_move(sheet_id, mv, ops);
    } else if let Some(mv) = detect_exact_column_block_move(old, new) {
        emit_column_block_move(sheet_id, mv, ops);
    } else if let Some(mv) = detect_fuzzy_row_block_move(old, new) {
        emit_row_block_move(sheet_id, mv, ops);
        emit_moved_row_block_edits(sheet_id, old, new, mv, ops);
    } else if let Some(alignment) = align_row_changes(old, new) {
        emit_aligned_diffs(sheet_id, old, new, &alignment, ops);
    } else if let Some(alignment) = align_single_column_change(old, new) {
        emit_column_aligned_diffs(sheet_id, old, new, &alignment, ops);
    } else {
        positional_diff(sheet_id, old, new, ops);
    }
}

fn positional_diff(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    for row in 0..overlap_rows {
        diff_row_pair(sheet_id, old, new, row, row, overlap_cols, ops);
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            ops.push(DiffOp::row_added(sheet_id.clone(), row_idx, None));
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            ops.push(DiffOp::row_removed(sheet_id.clone(), row_idx, None));
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            ops.push(DiffOp::column_added(sheet_id.clone(), col_idx, None));
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            ops.push(DiffOp::column_removed(sheet_id.clone(), col_idx, None));
        }
    }
}

fn emit_row_block_move(sheet_id: &SheetId, mv: RowBlockMove, ops: &mut Vec<DiffOp>) {
    ops.push(DiffOp::BlockMovedRows {
        sheet: sheet_id.clone(),
        src_start_row: mv.src_start_row,
        row_count: mv.row_count,
        dst_start_row: mv.dst_start_row,
        block_hash: None,
    });
}

fn emit_column_block_move(sheet_id: &SheetId, mv: ColumnBlockMove, ops: &mut Vec<DiffOp>) {
    ops.push(DiffOp::BlockMovedColumns {
        sheet: sheet_id.clone(),
        src_start_col: mv.src_start_col,
        col_count: mv.col_count,
        dst_start_col: mv.dst_start_col,
        block_hash: None,
    });
}

fn emit_rect_block_move(sheet_id: &SheetId, mv: RectBlockMove, ops: &mut Vec<DiffOp>) {
    ops.push(DiffOp::BlockMovedRect {
        sheet: sheet_id.clone(),
        src_start_row: mv.src_start_row,
        src_row_count: mv.src_row_count,
        src_start_col: mv.src_start_col,
        src_col_count: mv.src_col_count,
        dst_start_row: mv.dst_start_row,
        dst_start_col: mv.dst_start_col,
        block_hash: mv.block_hash,
    });
}

fn emit_moved_row_block_edits(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    mv: RowBlockMove,
    ops: &mut Vec<DiffOp>,
) {
    let overlap_cols = old.ncols.min(new.ncols);
    for offset in 0..mv.row_count {
        diff_row_pair(
            sheet_id,
            old,
            new,
            mv.src_start_row + offset,
            mv.dst_start_row + offset,
            overlap_cols,
            ops,
        );
    }
}

fn emit_aligned_diffs(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    alignment: &RowAlignment,
    ops: &mut Vec<DiffOp>,
) {
    let overlap_cols = old.ncols.min(new.ncols);

    for (row_a, row_b) in &alignment.matched {
        diff_row_pair(sheet_id, old, new, *row_a, *row_b, overlap_cols, ops);
    }

    for row_idx in &alignment.inserted {
        ops.push(DiffOp::row_added(sheet_id.clone(), *row_idx, None));
    }

    for row_idx in &alignment.deleted {
        ops.push(DiffOp::row_removed(sheet_id.clone(), *row_idx, None));
    }
}

fn diff_row_pair(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    row_a: u32,
    row_b: u32,
    overlap_cols: u32,
    ops: &mut Vec<DiffOp>,
) {
    for col in 0..overlap_cols {
        let addr = CellAddress::from_indices(row_b, col);
        let old_cell = old.get(row_a, col);
        let new_cell = new.get(row_b, col);

        let from = snapshot_with_addr(old_cell, addr);
        let to = snapshot_with_addr(new_cell, addr);

        if from != to {
            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
        }
    }
}

fn emit_column_aligned_diffs(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    alignment: &ColumnAlignment,
    ops: &mut Vec<DiffOp>,
) {
    let overlap_rows = old.nrows.min(new.nrows);

    for row in 0..overlap_rows {
        for (col_a, col_b) in &alignment.matched {
            let addr = CellAddress::from_indices(row, *col_b);
            let old_cell = old.get(row, *col_a);
            let new_cell = new.get(row, *col_b);

            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            if from != to {
                ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
            }
        }
    }

    for col_idx in &alignment.inserted {
        ops.push(DiffOp::column_added(sheet_id.clone(), *col_idx, None));
    }

    for col_idx in &alignment.deleted {
        ops.push(DiffOp::column_removed(sheet_id.clone(), *col_idx, None));
    }
}

fn snapshot_with_addr(cell: Option<&Cell>, addr: CellAddress) -> CellSnapshot {
    match cell {
        Some(cell) => CellSnapshot {
            addr,
            value: cell.value.clone(),
            formula: cell.formula.clone(),
        },
        None => CellSnapshot::empty(addr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_kind_order_ranking_includes_macro_and_other() {
        assert!(
            sheet_kind_order(&SheetKind::Worksheet) < sheet_kind_order(&SheetKind::Chart),
            "Worksheet should rank before Chart"
        );
        assert!(
            sheet_kind_order(&SheetKind::Chart) < sheet_kind_order(&SheetKind::Macro),
            "Chart should rank before Macro"
        );
        assert!(
            sheet_kind_order(&SheetKind::Macro) < sheet_kind_order(&SheetKind::Other),
            "Macro should rank before Other"
        );
    }
}
```

---

### File: `core\src\excel_open_xml.rs`

```rust
//! Excel Open XML file parsing.
//!
//! Provides functions for opening `.xlsx` files and parsing their contents into
//! the internal representation used for diffing.

use crate::container::{ContainerError, OpcContainer};
use crate::datamashup_framing::{
    DataMashupError, RawDataMashup, decode_datamashup_base64, parse_data_mashup,
    read_datamashup_text,
};
use crate::grid_parser::{
    GridParseError, parse_relationships, parse_shared_strings, parse_sheet_xml, parse_workbook_xml,
    resolve_sheet_target,
};
use crate::workbook::{Sheet, SheetKind, Workbook};
use std::collections::HashMap;
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

    let shared_strings = match container
        .read_file_optional("xl/sharedStrings.xml")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_shared_strings(&bytes)?,
        None => Vec::new(),
    };

    let workbook_bytes = container
        .read_file("xl/workbook.xml")
        .map_err(|_| ExcelOpenError::WorkbookXmlMissing)?;

    let sheets = parse_workbook_xml(&workbook_bytes)?;

    let relationships = match container
        .read_file_optional("xl/_rels/workbook.xml.rels")
        .map_err(ContainerError::from)?
    {
        Some(bytes) => parse_relationships(&bytes)?,
        None => HashMap::new(),
    };

    let mut sheet_ir = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = resolve_sheet_target(sheet, &relationships, idx);
        let sheet_bytes =
            container
                .read_file(&target)
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
    let mut found: Option<RawDataMashup> = None;

    for i in 0..container.len() {
        let name = {
            let file = container.archive.by_index(i).ok();
            file.map(|f| f.name().to_string())
        };

        if let Some(name) = name {
            if !name.starts_with("customXml/") || !name.ends_with(".xml") {
                continue;
            }

            let bytes = container
                .read_file(&name)
                .map_err(|e| ContainerError::Zip(e.to_string()))?;

            if let Some(text) = read_datamashup_text(&bytes)? {
                let decoded = decode_datamashup_base64(&text)?;
                let parsed = parse_data_mashup(&decoded)?;
                if found.is_some() {
                    return Err(DataMashupError::FramingInvalid.into());
                }
                found = Some(parsed);
            }
        }
    }

    Ok(found)
}
```

---

### File: `core\src\grid_parser.rs`

```rust
//! XML parsing for Excel worksheet grids.
//!
//! Handles parsing of worksheet XML, shared strings, workbook structure, and
//! relationship files to construct [`Grid`] representations of sheet data.

use crate::addressing::address_to_index;
use crate::workbook::{Cell, CellAddress, CellValue, Grid};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
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

pub struct SheetDescriptor {
    pub name: String,
    pub rel_id: Option<String>,
    pub sheet_id: Option<u32>,
}

pub fn parse_shared_strings(xml: &[u8]) -> Result<Vec<String>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_si = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"si" => {
                current.clear();
                in_si = true;
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" && in_si => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                current.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"si" => {
                strings.push(current.clone());
                in_si = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(strings)
}

pub fn parse_workbook_xml(xml: &[u8]) -> Result<Vec<SheetDescriptor>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut sheets = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"sheet" => {
                let mut name = None;
                let mut rel_id = None;
                let mut sheet_id = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
                    match attr.key.as_ref() {
                        b"name" => {
                            name = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        b"sheetId" => {
                            let parsed = attr.unescape_value().map_err(to_xml_err)?;
                            sheet_id = parsed.into_owned().parse::<u32>().ok();
                        }
                        b"r:id" => {
                            rel_id = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        _ => {}
                    }
                }
                if let Some(name) = name {
                    sheets.push(SheetDescriptor {
                        name,
                        rel_id,
                        sheet_id,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(sheets)
}

pub fn parse_relationships(xml: &[u8]) -> Result<HashMap<String, String>, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut map = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"Relationship" => {
                let mut id = None;
                let mut target = None;
                let mut rel_type = None;
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
                    match attr.key.as_ref() {
                        b"Id" => id = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned()),
                        b"Target" => {
                            target = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        b"Type" => {
                            rel_type = Some(attr.unescape_value().map_err(to_xml_err)?.into_owned())
                        }
                        _ => {}
                    }
                }

                if let (Some(id), Some(target), Some(rel_type)) = (id, target, rel_type)
                    && rel_type.contains("worksheet")
                {
                    map.insert(id, target);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(map)
}

pub fn resolve_sheet_target(
    sheet: &SheetDescriptor,
    relationships: &HashMap<String, String>,
    index: usize,
) -> String {
    if let Some(rel_id) = &sheet.rel_id
        && let Some(target) = relationships.get(rel_id)
    {
        return normalize_target(target);
    }

    let guessed = sheet
        .sheet_id
        .map(|id| format!("xl/worksheets/sheet{id}.xml"))
        .unwrap_or_else(|| format!("xl/worksheets/sheet{}.xml", index + 1));
    normalize_target(&guessed)
}

fn normalize_target(target: &str) -> String {
    let trimmed = target.trim_start_matches('/');
    if trimmed.starts_with("xl/") {
        trimmed.to_string()
    } else {
        format!("xl/{trimmed}")
    }
}

pub fn parse_sheet_xml(xml: &[u8], shared_strings: &[String]) -> Result<Grid, GridParseError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut dimension_hint: Option<(u32, u32)> = None;
    let mut parsed_cells: Vec<ParsedCell> = Vec::new();
    let mut max_row: Option<u32> = None;
    let mut max_col: Option<u32> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"dimension" => {
                if let Some(r) = get_attr_value(&e, b"ref")? {
                    dimension_hint = dimension_from_ref(&r);
                }
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"c" => {
                let cell = parse_cell(&mut reader, e, shared_strings)?;
                max_row = Some(max_row.map_or(cell.row, |r| r.max(cell.row)));
                max_col = Some(max_col.map_or(cell.col, |c| c.max(cell.col)));
                parsed_cells.push(cell);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    if parsed_cells.is_empty() {
        return Ok(Grid::new(0, 0));
    }

    let mut nrows = dimension_hint.map(|(r, _)| r).unwrap_or(0);
    let mut ncols = dimension_hint.map(|(_, c)| c).unwrap_or(0);

    if let Some(max_r) = max_row {
        nrows = nrows.max(max_r + 1);
    }
    if let Some(max_c) = max_col {
        ncols = ncols.max(max_c + 1);
    }

    build_grid(nrows, ncols, parsed_cells)
}

fn parse_cell(
    reader: &mut Reader<&[u8]>,
    start: BytesStart,
    shared_strings: &[String],
) -> Result<ParsedCell, GridParseError> {
    let address_raw = get_attr_value(&start, b"r")?
        .ok_or_else(|| GridParseError::XmlError("cell missing address".into()))?;
    let (row, col) = address_to_index(&address_raw)
        .ok_or_else(|| GridParseError::InvalidAddress(address_raw.clone()))?;

    let cell_type = get_attr_value(&start, b"t")?;

    let mut value_text: Option<String> = None;
    let mut formula_text: Option<String> = None;
    let mut inline_text: Option<String> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"v" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                value_text = Some(text);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"f" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                let unescaped = quick_xml::escape::unescape(&text)
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                formula_text = Some(unescaped);
            }
            Ok(Event::Start(e)) if e.name().as_ref() == b"is" => {
                inline_text = Some(read_inline_string(reader)?);
            }
            Ok(Event::End(e)) if e.name().as_ref() == start.name().as_ref() => break,
            Ok(Event::Eof) => {
                return Err(GridParseError::XmlError(
                    "unexpected EOF inside cell".into(),
                ));
            }
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let value = match inline_text {
        Some(text) => Some(CellValue::Text(text)),
        None => convert_value(value_text.as_deref(), cell_type.as_deref(), shared_strings)?,
    };

    Ok(ParsedCell {
        row,
        col,
        value,
        formula: formula_text,
    })
}

fn read_inline_string(reader: &mut Reader<&[u8]>) -> Result<String, GridParseError> {
    let mut buf = Vec::new();
    let mut value = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"t" => {
                let text = reader
                    .read_text(e.name())
                    .map_err(|e| GridParseError::XmlError(e.to_string()))?
                    .into_owned();
                value.push_str(&text);
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"is" => break,
            Ok(Event::Eof) => {
                return Err(GridParseError::XmlError(
                    "unexpected EOF inside inline string".into(),
                ));
            }
            Err(e) => return Err(GridParseError::XmlError(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(value)
}

fn convert_value(
    value_text: Option<&str>,
    cell_type: Option<&str>,
    shared_strings: &[String],
) -> Result<Option<CellValue>, GridParseError> {
    let raw = match value_text {
        Some(t) => t,
        None => return Ok(None),
    };

    let trimmed = raw.trim();
    if raw.is_empty() || trimmed.is_empty() {
        return Ok(Some(CellValue::Text(String::new())));
    }

    match cell_type {
        Some("s") => {
            let idx = trimmed
                .parse::<usize>()
                .map_err(|e| GridParseError::XmlError(e.to_string()))?;
            let text = shared_strings
                .get(idx)
                .ok_or(GridParseError::SharedStringOutOfBounds(idx))?;
            Ok(Some(CellValue::Text(text.clone())))
        }
        Some("b") => Ok(match trimmed {
            "1" => Some(CellValue::Bool(true)),
            "0" => Some(CellValue::Bool(false)),
            _ => None,
        }),
        Some("str") | Some("inlineStr") => Ok(Some(CellValue::Text(raw.to_string()))),
        _ => {
            if let Ok(n) = trimmed.parse::<f64>() {
                Ok(Some(CellValue::Number(n)))
            } else {
                Ok(Some(CellValue::Text(trimmed.to_string())))
            }
        }
    }
}

fn dimension_from_ref(reference: &str) -> Option<(u32, u32)> {
    let mut parts = reference.split(':');
    let start = parts.next()?;
    let end = parts.next().unwrap_or(start);
    let (start_row, start_col) = address_to_index(start)?;
    let (end_row, end_col) = address_to_index(end)?;
    let height = end_row.checked_sub(start_row)?.checked_add(1)?;
    let width = end_col.checked_sub(start_col)?.checked_add(1)?;
    Some((height, width))
}

fn build_grid(nrows: u32, ncols: u32, cells: Vec<ParsedCell>) -> Result<Grid, GridParseError> {
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

fn get_attr_value(element: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, GridParseError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| GridParseError::XmlError(e.to_string()))?;
        if attr.key.as_ref() == key {
            return Ok(Some(
                attr.unescape_value().map_err(to_xml_err)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

fn to_xml_err(err: quick_xml::Error) -> GridParseError {
    GridParseError::XmlError(err.to_string())
}

struct ParsedCell {
    row: u32,
    col: u32,
    value: Option<CellValue>,
    formula: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{GridParseError, convert_value, parse_shared_strings, read_inline_string};
    use crate::workbook::CellValue;
    use quick_xml::Reader;

    #[test]
    fn parse_shared_strings_rich_text_flattens_runs() {
        let xml = br#"<?xml version="1.0"?>
<sst>
  <si>
    <r><t>Hello</t></r>
    <r><t xml:space="preserve"> World</t></r>
  </si>
</sst>"#;
        let strings = parse_shared_strings(xml).expect("shared strings should parse");
        assert_eq!(strings.first(), Some(&"Hello World".to_string()));
    }

    #[test]
    fn read_inline_string_preserves_xml_space_preserve() {
        let xml = br#"<is><t xml:space="preserve"> hello</t></is>"#;
        let mut reader = Reader::from_reader(xml.as_ref());
        reader.config_mut().trim_text(false);
        let value = read_inline_string(&mut reader).expect("inline string should parse");
        assert_eq!(value, " hello");

        let converted = convert_value(Some(value.as_str()), Some("inlineStr"), &[])
            .expect("inlineStr conversion should succeed");
        assert_eq!(converted, Some(CellValue::Text(" hello".into())));
    }

    #[test]
    fn convert_value_bool_0_1_and_other() {
        let false_val =
            convert_value(Some("0"), Some("b"), &[]).expect("bool cell conversion should succeed");
        assert_eq!(false_val, Some(CellValue::Bool(false)));

        let true_val =
            convert_value(Some("1"), Some("b"), &[]).expect("bool cell conversion should succeed");
        assert_eq!(true_val, Some(CellValue::Bool(true)));

        let none_val = convert_value(Some("2"), Some("b"), &[])
            .expect("unexpected bool tokens should still parse");
        assert!(none_val.is_none());
    }

    #[test]
    fn convert_value_shared_string_index_out_of_bounds_errors() {
        let err = convert_value(Some("5"), Some("s"), &["only".into()])
            .expect_err("invalid shared string index should error");
        assert!(matches!(err, GridParseError::SharedStringOutOfBounds(5)));
    }

    #[test]
    fn convert_value_error_cell_as_text() {
        let value =
            convert_value(Some("#DIV/0!"), Some("e"), &[]).expect("error cell should convert");
        assert_eq!(value, Some(CellValue::Text("#DIV/0!".into())));
    }
}
```

---

### File: `core\src\grid_view.rs`

```rust
use std::collections::HashMap;
use std::hash::Hash;

use crate::hashing::{combine_hashes, hash_cell_contribution};
use crate::workbook::{Cell, CellValue, Grid};

pub type RowHash = u64;
pub type ColHash = u64;

#[derive(Debug)]
pub struct RowView<'a> {
    pub cells: Vec<(u32, &'a Cell)>, // sorted by column index
}

#[derive(Debug, Clone, Copy)]
pub struct RowMeta {
    pub row_idx: u32,
    pub hash: RowHash,
    pub non_blank_count: u16,
    pub first_non_blank_col: u16,
    pub is_low_info: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ColMeta {
    pub col_idx: u32,
    pub hash: ColHash,
    pub non_blank_count: u16,
    pub first_non_blank_row: u16,
}

#[derive(Debug)]
pub struct GridView<'a> {
    pub rows: Vec<RowView<'a>>,
    pub row_meta: Vec<RowMeta>,
    pub col_meta: Vec<ColMeta>,
    pub source: &'a Grid,
}

impl<'a> GridView<'a> {
    pub fn from_grid(grid: &'a Grid) -> GridView<'a> {
        let nrows = grid.nrows as usize;
        let ncols = grid.ncols as usize;

        let mut rows: Vec<RowView<'a>> =
            (0..nrows).map(|_| RowView { cells: Vec::new() }).collect();

        let mut row_hashes = vec![0u64; nrows];
        let mut row_counts = vec![0u32; nrows];
        let mut row_first_non_blank: Vec<Option<u32>> = vec![None; nrows];

        let mut col_hashes = vec![0u64; ncols];
        let mut col_counts = vec![0u32; ncols];
        let mut col_first_non_blank: Vec<Option<u32>> = vec![None; ncols];

        for ((row, col), cell) in &grid.cells {
            let r = *row as usize;
            let c = *col as usize;

            debug_assert!(
                r < nrows && c < ncols,
                "cell coordinates must lie within the grid bounds"
            );

            rows[r].cells.push((*col, cell));

            let row_contribution = hash_cell_contribution(*col, cell);
            row_hashes[r] = combine_hashes(row_hashes[r], row_contribution);

            let col_contribution = hash_cell_contribution(*row, cell);
            col_hashes[c] = combine_hashes(col_hashes[c], col_contribution);

            if is_non_blank(cell) {
                row_counts[r] = row_counts[r].saturating_add(1);
                col_counts[c] = col_counts[c].saturating_add(1);

                row_first_non_blank[r] =
                    Some(row_first_non_blank[r].map_or(*col, |cur| cur.min(*col)));
                col_first_non_blank[c] =
                    Some(col_first_non_blank[c].map_or(*row, |cur| cur.min(*row)));
            }
        }

        for row_view in rows.iter_mut() {
            row_view.cells.sort_by_key(|(col, _)| *col);
        }

        let row_meta = rows
            .iter()
            .enumerate()
            .map(|(idx, row_view)| {
                let count = row_counts.get(idx).copied().unwrap_or(0);
                let non_blank_count = to_u16(count);
                let first_non_blank_col = row_first_non_blank
                    .get(idx)
                    .and_then(|c| c.map(to_u16))
                    .unwrap_or(0);
                let is_low_info = compute_is_low_info(non_blank_count, row_view);

                RowMeta {
                    row_idx: idx as u32,
                    hash: row_hashes.get(idx).copied().unwrap_or(0),
                    non_blank_count,
                    first_non_blank_col,
                    is_low_info,
                }
            })
            .collect();

        let col_meta = (0..ncols)
            .map(|idx| ColMeta {
                col_idx: idx as u32,
                hash: col_hashes.get(idx).copied().unwrap_or(0),
                non_blank_count: to_u16(col_counts.get(idx).copied().unwrap_or(0)),
                first_non_blank_row: col_first_non_blank
                    .get(idx)
                    .and_then(|r| r.map(to_u16))
                    .unwrap_or(0),
            })
            .collect();

        GridView {
            rows,
            row_meta,
            col_meta,
            source: grid,
        }
    }
}

#[derive(Debug, Default)]
pub struct HashStats<H> {
    pub freq_a: HashMap<H, u32>,
    pub freq_b: HashMap<H, u32>,
    pub hash_to_positions_b: HashMap<H, Vec<u32>>,
}

impl HashStats<RowHash> {
    pub fn from_row_meta(rows_a: &[RowMeta], rows_b: &[RowMeta]) -> HashStats<RowHash> {
        let mut stats = HashStats::default();

        for meta in rows_a {
            *stats.freq_a.entry(meta.hash).or_insert(0) += 1;
        }

        for meta in rows_b {
            *stats.freq_b.entry(meta.hash).or_insert(0) += 1;
            stats
                .hash_to_positions_b
                .entry(meta.hash)
                .or_insert_with(Vec::new)
                .push(meta.row_idx);
        }

        stats
    }
}

impl HashStats<ColHash> {
    pub fn from_col_meta(cols_a: &[ColMeta], cols_b: &[ColMeta]) -> HashStats<ColHash> {
        let mut stats = HashStats::default();

        for meta in cols_a {
            *stats.freq_a.entry(meta.hash).or_insert(0) += 1;
        }

        for meta in cols_b {
            *stats.freq_b.entry(meta.hash).or_insert(0) += 1;
            stats
                .hash_to_positions_b
                .entry(meta.hash)
                .or_insert_with(Vec::new)
                .push(meta.col_idx);
        }

        stats
    }
}

impl<H> HashStats<H>
where
    H: Eq + Hash + Copy,
{
    pub fn is_unique(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) == 1
            && self.freq_b.get(&hash).copied().unwrap_or(0) == 1
    }

    pub fn is_rare(&self, hash: H, threshold: u32) -> bool {
        let freq_a = self.freq_a.get(&hash).copied().unwrap_or(0);
        let freq_b = self.freq_b.get(&hash).copied().unwrap_or(0);

        if freq_a == 0 || freq_b == 0 || self.is_unique(hash) {
            return false;
        }

        freq_a <= threshold && freq_b <= threshold
    }

    pub fn is_common(&self, hash: H, threshold: u32) -> bool {
        let freq_a = self.freq_a.get(&hash).copied().unwrap_or(0);
        let freq_b = self.freq_b.get(&hash).copied().unwrap_or(0);

        if freq_a == 0 && freq_b == 0 {
            return false;
        }

        freq_a > threshold || freq_b > threshold
    }

    pub fn appears_in_both(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) > 0
            && self.freq_b.get(&hash).copied().unwrap_or(0) > 0
    }
}

fn is_non_blank(cell: &Cell) -> bool {
    cell.value.is_some() || cell.formula.is_some()
}

fn compute_is_low_info(non_blank_count: u16, row_view: &RowView<'_>) -> bool {
    if non_blank_count == 0 {
        return true;
    }

    if non_blank_count > 1 {
        return false;
    }

    let cell = row_view
        .cells
        .iter()
        .find_map(|(_, cell)| is_non_blank(cell).then_some(*cell));

    match cell {
        None => true,
        Some(cell) => match (&cell.value, &cell.formula) {
            (_, Some(_)) => false,
            (Some(CellValue::Text(s)), None) => s.trim().is_empty(),
            (Some(CellValue::Number(_)), _) => false,
            (Some(CellValue::Bool(_)), _) => false,
            (None, None) => true,
        },
    }
}

fn to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
```

---

### File: `core\src\hashing.rs`

```rust
//! Hash utilities for row/column signature computation.
//!
//! Provides consistent hashing functions used for computing structural
//! signatures that enable efficient alignment during diffing.

use std::hash::{Hash, Hasher};
use xxhash_rust::xxh64::Xxh64;

use crate::workbook::{Cell, ColSignature, RowSignature};

pub(crate) const XXH64_SEED: u64 = 0;
const HASH_MIX_CONSTANT: u64 = 0x9e3779b97f4a7c15;

pub(crate) fn hash_cell_contribution(position: u32, cell: &Cell) -> u64 {
    let mut hasher = Xxh64::new(XXH64_SEED);
    position.hash(&mut hasher);
    cell.value.hash(&mut hasher);
    cell.formula.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn mix_hash(hash: u64) -> u64 {
    hash.rotate_left(13) ^ HASH_MIX_CONSTANT
}

pub(crate) fn combine_hashes(current: u64, contribution: u64) -> u64 {
    current.wrapping_add(mix_hash(contribution))
}

#[allow(dead_code)]
pub(crate) fn compute_row_signature<'a>(
    cells: impl Iterator<Item = (u32, &'a Cell)>,
    row: u32,
) -> RowSignature {
    let hash = cells
        .filter(|(_, cell)| cell.row == row)
        .fold(0u64, |acc, (col, cell)| {
            combine_hashes(acc, hash_cell_contribution(col, cell))
        });
    RowSignature { hash }
}

#[allow(dead_code)]
pub(crate) fn compute_col_signature<'a>(
    cells: impl Iterator<Item = (u32, &'a Cell)>,
    col: u32,
) -> ColSignature {
    let hash = cells
        .filter(|(_, cell)| cell.col == col)
        .fold(0u64, |acc, (row, cell)| {
            combine_hashes(acc, hash_cell_contribution(row, cell))
        });
    ColSignature { hash }
}
```

---

### File: `core\src\lib.rs`

```rust
//! Excel Diff: A library for comparing Excel workbooks.
//!
//! This crate provides functionality for:
//! - Opening and parsing Excel workbooks (`.xlsx` files)
//! - Computing structural and cell-level differences between workbooks
//! - Serializing diff reports to JSON
//! - Parsing Power Query (M) code from DataMashup sections
//!
//! # Quick Start
//!
//! ```ignore
//! use excel_diff::{open_workbook, diff_workbooks};
//!
//! let wb_a = open_workbook("file_a.xlsx")?;
//! let wb_b = open_workbook("file_b.xlsx")?;
//! let report = diff_workbooks(&wb_a, &wb_b);
//!
//! for op in &report.ops {
//!     println!("{:?}", op);
//! }
//! ```

pub mod addressing;
pub(crate) mod column_alignment;
pub mod container;
pub(crate) mod database_alignment;
pub mod datamashup;
pub mod datamashup_framing;
pub mod datamashup_package;
pub mod diff;
pub mod engine;
#[cfg(feature = "excel-open-xml")]
pub mod excel_open_xml;
pub mod grid_parser;
pub mod grid_view;
pub(crate) mod hashing;
pub mod m_diff;
pub mod m_section;
pub mod output;
pub(crate) mod rect_block_move;
pub(crate) mod row_alignment;
pub mod workbook;

pub use addressing::{AddressParseError, address_to_index, index_to_address};
pub use container::{ContainerError, OpcContainer};
pub use datamashup::{
    DataMashup, Metadata, Permissions, Query, QueryMetadata, build_data_mashup, build_queries,
};
pub use datamashup_framing::{DataMashupError, RawDataMashup};
pub use datamashup_package::{
    EmbeddedContent, PackageParts, PackageXml, SectionDocument, parse_package_parts,
};
pub use diff::{DiffOp, DiffReport, SheetId};
pub use engine::{diff_grids_database_mode, diff_workbooks};
#[cfg(feature = "excel-open-xml")]
pub use excel_open_xml::{ExcelOpenError, open_data_mashup, open_workbook};
pub use grid_parser::{GridParseError, SheetDescriptor};
pub use grid_view::{ColHash, ColMeta, GridView, HashStats, RowHash, RowMeta, RowView};
pub use m_diff::{MQueryDiff, QueryChangeKind, diff_m_queries};
pub use m_section::{SectionMember, SectionParseError, parse_section_members};
#[cfg(feature = "excel-open-xml")]
pub use output::json::diff_workbooks_to_json;
pub use output::json::{CellDiff, serialize_cell_diffs, serialize_diff_report};
pub use workbook::{
    Cell, CellAddress, CellSnapshot, CellValue, ColSignature, Grid, RowSignature, Sheet, SheetKind,
    Workbook,
};
```

---

### File: `core\src\m_diff.rs`

```rust
use std::collections::HashMap;

use crate::datamashup::{DataMashup, Query, build_queries};
use crate::m_section::SectionParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryChangeKind {
    Added,
    Removed,
    Renamed { from: String, to: String }, // present for forward compatibility; not emitted yet
    DefinitionChanged,
    MetadataChangedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MQueryDiff {
    pub name: String,
    pub kind: QueryChangeKind,
}

pub fn diff_m_queries(
    old_dm: &DataMashup,
    new_dm: &DataMashup,
) -> Result<Vec<MQueryDiff>, SectionParseError> {
    let old_queries = build_queries(old_dm)?;
    let new_queries = build_queries(new_dm)?;
    Ok(diff_queries(&old_queries, &new_queries))
}

fn diff_queries(old_queries: &[Query], new_queries: &[Query]) -> Vec<MQueryDiff> {
    let mut old_map: HashMap<String, &Query> = HashMap::new();
    for query in old_queries {
        old_map.insert(query.name.clone(), query);
    }

    let mut new_map: HashMap<String, &Query> = HashMap::new();
    for query in new_queries {
        new_map.insert(query.name.clone(), query);
    }

    let mut names: Vec<String> = old_map.keys().chain(new_map.keys()).cloned().collect();
    names.sort();
    names.dedup();

    let mut diffs = Vec::new();
    for name in names {
        match (old_map.get(&name), new_map.get(&name)) {
            (None, Some(_)) => diffs.push(MQueryDiff {
                name,
                kind: QueryChangeKind::Added,
            }),
            (Some(_), None) => diffs.push(MQueryDiff {
                name,
                kind: QueryChangeKind::Removed,
            }),
            (Some(old_q), Some(new_q)) => {
                if old_q.expression_m == new_q.expression_m {
                    if old_q.metadata != new_q.metadata {
                        diffs.push(MQueryDiff {
                            name,
                            kind: QueryChangeKind::MetadataChangedOnly,
                        });
                    }
                } else {
                    diffs.push(MQueryDiff {
                        name,
                        kind: QueryChangeKind::DefinitionChanged,
                    });
                }
            }
            (None, None) => {
                debug_assert!(false, "query name missing from both maps");
            }
        }
    }

    diffs
}
```

---

### File: `core\src\m_section.rs`

```rust
use std::str::Lines;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SectionParseError {
    #[error("missing section header")]
    MissingSectionHeader,
    #[error("invalid section header")]
    InvalidHeader,
    #[error("invalid member syntax")]
    InvalidMemberSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionMember {
    pub section_name: String,
    pub member_name: String,
    pub expression_m: String,
    pub is_shared: bool,
}

pub fn parse_section_members(source: &str) -> Result<Vec<SectionMember>, SectionParseError> {
    let source = strip_leading_bom(source);
    let mut lines = source.lines();
    let section_name = find_section_name(&mut lines)?;

    let mut members = Vec::new();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if !trimmed.starts_with("shared") {
            continue;
        }

        let member = parse_shared_member(trimmed, &mut lines, &section_name)
            .ok_or(SectionParseError::InvalidMemberSyntax)?;
        members.push(member);
    }

    Ok(members)
}

fn find_section_name(lines: &mut Lines<'_>) -> Result<String, SectionParseError> {
    for line in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        match try_parse_section_header(trimmed) {
            Ok(Some(name)) => return Ok(name),
            Ok(None) => continue,
            Err(err) => return Err(err),
        }
    }

    Err(SectionParseError::MissingSectionHeader)
}

fn try_parse_section_header(line: &str) -> Result<Option<String>, SectionParseError> {
    let Some(rest) = line.strip_prefix("section") else {
        return Ok(None);
    };

    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let header_body = rest.trim_start();
    if !header_body.ends_with(';') {
        return Err(SectionParseError::InvalidHeader);
    }

    let without_semicolon = &header_body[..header_body.len() - 1];
    let name_candidate = without_semicolon.trim();
    if name_candidate.is_empty() {
        return Err(SectionParseError::InvalidHeader);
    }

    let mut parts = name_candidate.split_whitespace();
    let name = parts.next().ok_or(SectionParseError::InvalidHeader)?;
    if parts.next().is_some() {
        return Err(SectionParseError::InvalidHeader);
    }

    if !is_valid_identifier(name) {
        return Err(SectionParseError::InvalidHeader);
    }

    Ok(Some(name.to_string()))
}

fn parse_shared_member(
    line: &str,
    remaining_lines: &mut Lines<'_>,
    section_name: &str,
) -> Option<SectionMember> {
    let rest = line.strip_prefix("shared")?;
    if !rest.starts_with(char::is_whitespace) && !rest.is_empty() {
        return None;
    }

    let body = rest.trim_start();
    if body.is_empty() {
        return None;
    }

    let (member_name, after_name) = parse_identifier(body)?;

    let mut expression_source = after_name;
    let eq_index = expression_source.find('=')?;
    if !expression_source[..eq_index].trim().is_empty() {
        return None;
    }
    expression_source = &expression_source[eq_index + 1..];

    let mut expression = expression_source.to_string();
    if let Some(idx) = expression_source.find(';') {
        expression.truncate(idx);
    } else {
        let mut terminator_index = None;
        while terminator_index.is_none() {
            let Some(next_line) = remaining_lines.next() else {
                break;
            };

            expression.push('\n');
            let offset = expression.len();
            expression.push_str(next_line);
            if let Some(idx) = next_line.find(';') {
                terminator_index = Some(offset + idx);
            }
        }

        if let Some(idx) = terminator_index {
            expression.truncate(idx);
        } else {
            return None;
        }
    }

    let expression_m = expression.trim().to_string();

    Some(SectionMember {
        section_name: section_name.to_string(),
        member_name: member_name.to_string(),
        expression_m,
        is_shared: true,
    })
}

fn parse_identifier(text: &str) -> Option<(String, &str)> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("#\"") {
        return parse_quoted_identifier(trimmed);
    }

    parse_unquoted_identifier(trimmed)
}

fn parse_unquoted_identifier(text: &str) -> Option<(String, &str)> {
    if text.is_empty() {
        return None;
    }

    let mut end = 0;
    for ch in text.chars() {
        if ch.is_whitespace() || ch == '=' {
            break;
        }
        end += ch.len_utf8();
    }

    if end == 0 {
        return None;
    }

    let (name, rest) = text.split_at(end);
    if !is_valid_identifier(name) {
        return None;
    }

    Some((name.to_string(), rest))
}

fn parse_quoted_identifier(text: &str) -> Option<(String, &str)> {
    let mut chars = text.char_indices();
    let (_, hash) = chars.next()?;
    if hash != '#' {
        return None;
    }
    if !matches!(chars.next(), Some((_, '"'))) {
        return None;
    }

    let mut name = String::new();
    while let Some((idx, ch)) = chars.next() {
        if ch == '"' {
            if let Some((_, next_ch)) = chars.clone().next()
                && next_ch == '"'
            {
                name.push('"');
                chars.next();
                continue;
            }
            let rest_start = idx + 1;
            let rest = &text[rest_start..];
            return Some((name, rest));
        }

        name.push(ch);
    }

    None
}

fn is_valid_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn strip_leading_bom(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}
```

---

### File: `core\src\rect_block_move.rs`

```rust
use crate::grid_view::{ColHash, ColMeta, GridView, HashStats, RowHash};
use crate::workbook::{Cell, Grid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RectBlockMove {
    pub src_start_row: u32,
    pub src_row_count: u32,
    pub src_start_col: u32,
    pub src_col_count: u32,
    pub dst_start_row: u32,
    pub dst_start_col: u32,
    pub block_hash: Option<u64>,
}

const MAX_RECT_ROWS: u32 = 2_000;
const MAX_RECT_COLS: u32 = 128;
const MAX_HASH_REPEAT: u32 = 8;

pub(crate) fn detect_exact_rect_block_move(old: &Grid, new: &Grid) -> Option<RectBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 || old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new) {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    if blank_dominated(&view_a) || blank_dominated(&view_b) {
        return None;
    }

    let row_stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    let col_stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);

    if has_heavy_repetition(&row_stats) || has_heavy_repetition(&col_stats) {
        return None;
    }

    let diff_positions = collect_differences(old, new);
    if diff_positions.is_empty() {
        return None;
    }

    let row_ranges = find_two_equal_ranges(diff_positions.iter().map(|(r, _)| *r))?;
    let col_ranges = find_two_equal_ranges(diff_positions.iter().map(|(_, c)| *c))?;

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    let expected_mismatches = row_count.checked_mul(col_count)?.checked_mul(2)?;
    if diff_positions.len() as u32 != expected_mismatches {
        return None;
    }

    let mismatches = count_rect_mismatches(old, new, row_ranges.0, col_ranges.0)
        + count_rect_mismatches(old, new, row_ranges.1, col_ranges.1);
    if mismatches != diff_positions.len() as u32 {
        return None;
    }

    if !has_unique_meta(
        &view_a, &view_b, &row_stats, &col_stats, row_ranges, col_ranges,
    ) {
        return None;
    }

    let primary = validate_orientation(old, new, row_ranges, col_ranges);
    let swapped_ranges = ((row_ranges.1, row_ranges.0), (col_ranges.1, col_ranges.0));
    let alternate = validate_orientation(old, new, swapped_ranges.0, swapped_ranges.1);

    match (primary, alternate) {
        (Some(mv), None) => Some(mv),
        (None, Some(mv)) => Some(mv),
        _ => None,
    }
}

fn validate_orientation(
    old: &Grid,
    new: &Grid,
    row_ranges: ((u32, u32), (u32, u32)),
    col_ranges: ((u32, u32), (u32, u32)),
) -> Option<RectBlockMove> {
    if ranges_overlap(row_ranges.0, row_ranges.1) || ranges_overlap(col_ranges.0, col_ranges.1) {
        return None;
    }

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    if rectangles_correspond(
        old,
        new,
        row_ranges.0,
        col_ranges.0,
        row_ranges.1,
        col_ranges.1,
    ) {
        return Some(RectBlockMove {
            src_start_row: row_ranges.0.0,
            src_row_count: row_count,
            src_start_col: col_ranges.0.0,
            src_col_count: col_count,
            dst_start_row: row_ranges.1.0,
            dst_start_col: col_ranges.1.0,
            block_hash: None,
        });
    }

    None
}

fn rectangles_correspond(
    old: &Grid,
    new: &Grid,
    src_rows: (u32, u32),
    src_cols: (u32, u32),
    dst_rows: (u32, u32),
    dst_cols: (u32, u32),
) -> bool {
    let row_count = range_len(src_rows);
    let col_count = range_len(src_cols);

    if row_count != range_len(dst_rows) || col_count != range_len(dst_cols) {
        return false;
    }

    for dr in 0..row_count {
        for dc in 0..col_count {
            let src_r = src_rows.0 + dr;
            let src_c = src_cols.0 + dc;
            let dst_r = dst_rows.0 + dr;
            let dst_c = dst_cols.0 + dc;

            if !cell_content_equal(old.get(src_r, src_c), new.get(dst_r, dst_c)) {
                return false;
            }
        }
    }

    true
}

fn collect_differences(old: &Grid, new: &Grid) -> Vec<(u32, u32)> {
    let mut diffs = Vec::new();

    for row in 0..old.nrows {
        for col in 0..old.ncols {
            if !cell_content_equal(old.get(row, col), new.get(row, col)) {
                diffs.push((row, col));
            }
        }
    }

    diffs
}

fn cell_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
        (Some(cell_a), None) => cell_a.value.is_none() && cell_a.formula.is_none(),
        (None, Some(cell_b)) => cell_b.value.is_none() && cell_b.formula.is_none(),
    }
}

fn count_rect_mismatches(old: &Grid, new: &Grid, rows: (u32, u32), cols: (u32, u32)) -> u32 {
    let mut mismatches = 0u32;
    for row in rows.0..=rows.1 {
        for col in cols.0..=cols.1 {
            if !cell_content_equal(old.get(row, col), new.get(row, col)) {
                mismatches = mismatches.saturating_add(1);
            }
        }
    }
    mismatches
}

fn has_unique_meta(
    view_a: &GridView<'_>,
    view_b: &GridView<'_>,
    row_stats: &HashStats<RowHash>,
    col_stats: &HashStats<ColHash>,
    row_ranges: ((u32, u32), (u32, u32)),
    col_ranges: ((u32, u32), (u32, u32)),
) -> bool {
    for range in [row_ranges.0, row_ranges.1] {
        for idx in range.0..=range.1 {
            if !is_unique_row_in_a(idx, view_a, row_stats)
                || !is_unique_row_in_b(idx, view_b, row_stats)
            {
                return false;
            }
        }
    }

    for range in [col_ranges.0, col_ranges.1] {
        for idx in range.0..=range.1 {
            if !is_unique_col_in_a(idx, view_a, col_stats)
                || !is_unique_col_in_b(idx, view_b, col_stats)
            {
                return false;
            }
        }
    }

    true
}

fn is_unique_row_in_a(idx: u32, view: &GridView<'_>, stats: &HashStats<RowHash>) -> bool {
    view.row_meta
        .get(idx as usize)
        .map(|meta| unique_in_a(meta.hash, stats))
        .unwrap_or(false)
}

fn is_unique_row_in_b(idx: u32, view: &GridView<'_>, stats: &HashStats<RowHash>) -> bool {
    view.row_meta
        .get(idx as usize)
        .map(|meta| unique_in_b(meta.hash, stats))
        .unwrap_or(false)
}

fn is_unique_col_in_a(idx: u32, view: &GridView<'_>, stats: &HashStats<ColHash>) -> bool {
    view.col_meta
        .get(idx as usize)
        .map(|meta| unique_in_a(meta.hash, stats))
        .unwrap_or(false)
}

fn is_unique_col_in_b(idx: u32, view: &GridView<'_>, stats: &HashStats<ColHash>) -> bool {
    view.col_meta
        .get(idx as usize)
        .map(|meta| unique_in_b(meta.hash, stats))
        .unwrap_or(false)
}

fn find_two_equal_ranges<I>(indices: I) -> Option<((u32, u32), (u32, u32))>
where
    I: IntoIterator<Item = u32>,
{
    let mut values: Vec<u32> = indices.into_iter().collect();
    if values.is_empty() {
        return None;
    }

    values.sort_unstable();
    values.dedup();

    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut start = values[0];
    let mut prev = values[0];

    for &val in values.iter().skip(1) {
        if val == prev + 1 {
            prev = val;
            continue;
        }

        ranges.push((start, prev));
        start = val;
        prev = val;
    }
    ranges.push((start, prev));

    if ranges.len() != 2 {
        return None;
    }

    let len0 = range_len(ranges[0]);
    let len1 = range_len(ranges[1]);
    if len0 != len1 {
        return None;
    }

    Some((ranges[0], ranges[1]))
}

fn range_len(range: (u32, u32)) -> u32 {
    range.1.saturating_sub(range.0).saturating_add(1)
}

fn ranges_overlap(a: (u32, u32), b: (u32, u32)) -> bool {
    !(a.1 < b.0 || b.1 < a.0)
}

fn is_within_size_bounds(old: &Grid, new: &Grid) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= MAX_RECT_ROWS && cols <= MAX_RECT_COLS
}

fn low_info_dominated(view: &GridView<'_>) -> bool {
    if view.row_meta.is_empty() {
        return false;
    }

    let low_info_count = view.row_meta.iter().filter(|m| m.is_low_info).count();
    low_info_count * 2 > view.row_meta.len()
}

fn blank_dominated(view: &GridView<'_>) -> bool {
    if view.col_meta.is_empty() {
        return false;
    }

    let blank_cols = view
        .col_meta
        .iter()
        .filter(
            |ColMeta {
                 non_blank_count, ..
             }| *non_blank_count == 0,
        )
        .count();

    blank_cols * 2 > view.col_meta.len()
}

fn has_heavy_repetition<H>(stats: &HashStats<H>) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > MAX_HASH_REPEAT
}

fn unique_in_a<H>(hash: H, stats: &HashStats<H>) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) <= 1
}

fn unique_in_b<H>(hash: H, stats: &HashStats<H>) -> bool
where
    H: Eq + std::hash::Hash + Copy,
{
    stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_a.get(&hash).copied().unwrap_or(0) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::{CellAddress, CellValue};

    fn grid_from_numbers(values: &[&[i32]]) -> Grid {
        let nrows = values.len() as u32;
        let ncols = if nrows == 0 {
            0
        } else {
            values[0].len() as u32
        };

        let mut grid = Grid::new(nrows, ncols);
        for (r, row_vals) in values.iter().enumerate() {
            for (c, v) in row_vals.iter().enumerate() {
                grid.insert(Cell {
                    row: r as u32,
                    col: c as u32,
                    address: CellAddress::from_indices(r as u32, c as u32),
                    value: Some(CellValue::Number(*v as f64)),
                    formula: None,
                });
            }
        }

        grid
    }

    fn base_background(rows: usize, cols: usize) -> Vec<Vec<i32>> {
        (0..rows)
            .map(|r| (0..cols).map(|c| (r as i32) * 1_000 + c as i32).collect())
            .collect()
    }

    fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
        for (r_offset, row_vals) in block.iter().enumerate() {
            for (c_offset, value) in row_vals.iter().enumerate() {
                let row = top + r_offset;
                let col = left + c_offset;
                if let Some(row_slice) = target.get_mut(row)
                    && let Some(cell) = row_slice.get_mut(col)
                {
                    *cell = *value;
                }
            }
        }
    }

    fn grid_from_matrix(matrix: Vec<Vec<i32>>) -> Grid {
        let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
        grid_from_numbers(&refs)
    }

    #[test]
    fn detect_simple_rect_block_move_success() {
        let mut grid_a = base_background(12, 12);
        let mut grid_b = base_background(12, 12);

        let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

        place_block(&mut grid_a, 1, 1, &block);
        place_block(&mut grid_b, 7, 6, &block);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_some(),
            "should detect exact rectangular block move"
        );

        let mv = result.unwrap();
        assert_eq!(mv.src_start_row, 1);
        assert_eq!(mv.src_row_count, 3);
        assert_eq!(mv.src_start_col, 1);
        assert_eq!(mv.src_col_count, 3);
        assert_eq!(mv.dst_start_row, 7);
        assert_eq!(mv.dst_start_col, 6);
    }

    #[test]
    fn detect_bails_on_different_grid_dimensions() {
        let old = grid_from_numbers(&[&[1, 2], &[3, 4]]);
        let new = grid_from_numbers(&[&[1, 2, 5], &[3, 4, 6]]);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(result.is_none(), "different dimensions should bail");
    }

    #[test]
    fn detect_bails_on_empty_grid() {
        let old = Grid::new(0, 0);
        let new = Grid::new(0, 0);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(result.is_none(), "empty grid should bail");
    }

    #[test]
    fn detect_bails_on_identical_grids() {
        let old = grid_from_numbers(&[&[1, 2], &[3, 4]]);
        let new = grid_from_numbers(&[&[1, 2], &[3, 4]]);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "identical grids should bail (no differences)"
        );
    }

    #[test]
    fn detect_bails_on_internal_cell_edit() {
        let mut grid_a = base_background(10, 10);
        let mut grid_b = base_background(10, 10);

        let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

        place_block(&mut grid_a, 1, 1, &block);
        place_block(&mut grid_b, 6, 4, &block);
        grid_b[7][5] = 9_999;

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "move with internal edit should not be detected as exact rectangular move"
        );
    }

    #[test]
    fn detect_bails_on_ambiguous_block_swap() {
        let base: Vec<Vec<i32>> = (0..6)
            .map(|r| (0..6).map(|c| 100 * r + c).collect())
            .collect();
        let mut grid_a = base.clone();
        let mut grid_b = base.clone();

        let block_one = vec![vec![900, 901], vec![902, 903]];
        let block_two = vec![vec![700, 701], vec![702, 703]];

        place_block(&mut grid_a, 0, 0, &block_one);
        place_block(&mut grid_a, 3, 3, &block_two);

        place_block(&mut grid_b, 0, 0, &block_two);
        place_block(&mut grid_b, 3, 3, &block_one);

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "ambiguous block swap should not emit a rectangular move"
        );
    }

    #[test]
    fn detect_bails_on_oversized_row_count() {
        let old = Grid::new(MAX_RECT_ROWS + 1, 10);
        let new = Grid::new(MAX_RECT_ROWS + 1, 10);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "grids exceeding MAX_RECT_ROWS should bail"
        );
    }

    #[test]
    fn detect_bails_on_oversized_col_count() {
        let old = Grid::new(10, MAX_RECT_COLS + 1);
        let new = Grid::new(10, MAX_RECT_COLS + 1);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "grids exceeding MAX_RECT_COLS should bail"
        );
    }

    #[test]
    fn detect_bails_on_single_cell_edit() {
        let old = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);
        let new = grid_from_numbers(&[&[1, 2, 3], &[4, 99, 6], &[7, 8, 9]]);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "single cell edit is not a rectangular block move"
        );
    }

    #[test]
    fn detect_bails_on_pure_row_move_pattern() {
        let old = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9], &[10, 11, 12]]);
        let new = grid_from_numbers(&[&[7, 8, 9], &[4, 5, 6], &[1, 2, 3], &[10, 11, 12]]);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "pure row swap without column displacement is not a rectangular block move"
        );
    }

    #[test]
    fn detect_bails_on_non_contiguous_differences() {
        let mut grid_a = base_background(8, 8);
        let mut grid_b = base_background(8, 8);

        grid_a[1][1] = 111;
        grid_a[5][5] = 555;
        grid_a[1][5] = 115;
        grid_b[1][1] = 555;
        grid_b[5][5] = 111;
        grid_b[1][5] = 999;

        let old = grid_from_matrix(grid_a);
        let new = grid_from_matrix(grid_b);

        let result = detect_exact_rect_block_move(&old, &new);
        assert!(
            result.is_none(),
            "non-contiguous differences should not form a rectangular block move"
        );
    }
}
```

---

### File: `core\src\row_alignment.rs`

```rust
use std::collections::HashSet;

use crate::grid_view::{GridView, HashStats, RowHash, RowMeta};
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowAlignment {
    pub matched: Vec<(u32, u32)>, // (row_idx_a, row_idx_b)
    pub inserted: Vec<u32>,       // row indices in B
    pub deleted: Vec<u32>,        // row indices in A
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowBlockMove {
    pub src_start_row: u32,
    pub dst_start_row: u32,
    pub row_count: u32,
}

const MAX_ALIGN_ROWS: u32 = 2_000;
const MAX_ALIGN_COLS: u32 = 64;
const MAX_HASH_REPEAT: u32 = 8;
const MAX_BLOCK_GAP: u32 = 32;
const MAX_FUZZY_BLOCK_ROWS: u32 = 32;
const FUZZY_SIMILARITY_THRESHOLD: f64 = 0.80;
const _HASH_COLLISION_NOTE: &str = "64-bit hash collision probability ~0.00006% at 2K rows; \
                                    secondary verification deferred to G8a (50K-row adversarial)";

pub(crate) fn detect_exact_row_block_move(old: &Grid, new: &Grid) -> Option<RowBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new) {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats) {
        return None;
    }

    let meta_a = &view_a.row_meta;
    let meta_b = &view_b.row_meta;
    let n = meta_a.len();

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let prefix = (0..n).find(|&idx| meta_a[idx].hash != meta_b[idx].hash)?;

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = n - 1 - suffix_len;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }
    let tail_start = n - suffix_len;

    let try_candidate = |src_start: usize, dst_start: usize| -> Option<RowBlockMove> {
        if src_start >= tail_start || dst_start >= tail_start {
            return None;
        }

        let mut len = 0usize;
        while src_start + len < tail_start && dst_start + len < tail_start {
            if meta_a[src_start + len].hash != meta_b[dst_start + len].hash {
                break;
            }
            len += 1;
        }

        if len == 0 {
            return None;
        }

        let src_end = src_start + len;
        let dst_end = dst_start + len;

        if !(src_end <= dst_start || dst_end <= src_start) {
            return None;
        }

        let mut idx_a = 0usize;
        let mut idx_b = 0usize;

        loop {
            if idx_a == src_start {
                idx_a = src_end;
            }
            if idx_b == dst_start {
                idx_b = dst_end;
            }

            if idx_a >= n && idx_b >= n {
                break;
            }

            if idx_a >= n || idx_b >= n {
                return None;
            }

            if meta_a[idx_a].hash != meta_b[idx_b].hash {
                return None;
            }

            idx_a += 1;
            idx_b += 1;
        }

        for meta in &meta_a[src_start..src_end] {
            if stats.freq_a.get(&meta.hash).copied().unwrap_or(0) != 1
                || stats.freq_b.get(&meta.hash).copied().unwrap_or(0) != 1
            {
                return None;
            }
        }

        Some(RowBlockMove {
            src_start_row: meta_a[src_start].row_idx,
            dst_start_row: meta_b[dst_start].row_idx,
            row_count: len as u32,
        })
    };

    if let Some(src_start) =
        (prefix..tail_start).find(|&idx| meta_a[idx].hash == meta_b[prefix].hash)
        && let Some(mv) = try_candidate(src_start, prefix)
    {
        return Some(mv);
    }

    if let Some(dst_start) =
        (prefix..tail_start).find(|&idx| meta_b[idx].hash == meta_a[prefix].hash)
        && let Some(mv) = try_candidate(prefix, dst_start)
    {
        return Some(mv);
    }

    None
}

pub(crate) fn detect_fuzzy_row_block_move(old: &Grid, new: &Grid) -> Option<RowBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new) {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats) {
        return None;
    }

    let meta_a = &view_a.row_meta;
    let meta_b = &view_b.row_meta;

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let n = meta_a.len();
    let mut prefix = 0usize;
    while prefix < n && meta_a[prefix].hash == meta_b[prefix].hash {
        prefix += 1;
    }
    if prefix == n {
        return None;
    }

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = idx_a;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }

    let mismatch_end = n - suffix_len;
    if mismatch_end <= prefix {
        return None;
    }

    let mid_len = mismatch_end - prefix;
    if mid_len <= 1 {
        return None;
    }

    let max_block_len = mid_len.saturating_sub(1).min(MAX_FUZZY_BLOCK_ROWS as usize);
    if max_block_len == 0 {
        return None;
    }

    let mut candidate: Option<RowBlockMove> = None;

    for block_len in 1..=max_block_len {
        let remaining = mid_len - block_len;

        // Block moved upward: [middle][block] -> [block'][middle]
        if hashes_match(
            &meta_a[prefix..prefix + remaining],
            &meta_b[prefix + block_len..mismatch_end],
        ) {
            let src_block = &meta_a[prefix + remaining..mismatch_end];
            let dst_block = &meta_b[prefix..prefix + block_len];

            if block_similarity(src_block, dst_block) >= FUZZY_SIMILARITY_THRESHOLD {
                let mv = RowBlockMove {
                    src_start_row: src_block[0].row_idx,
                    dst_start_row: dst_block[0].row_idx,
                    row_count: block_len as u32,
                };
                if mv.src_start_row != mv.dst_start_row {
                    if candidate.is_some() {
                        return None;
                    }
                    candidate = Some(mv);
                }
            }
        }

        // Block moved downward: [block][middle] -> [middle][block']
        if hashes_match(
            &meta_a[prefix + block_len..mismatch_end],
            &meta_b[prefix..prefix + remaining],
        ) {
            let src_block = &meta_a[prefix..prefix + block_len];
            let dst_block = &meta_b[prefix + remaining..mismatch_end];

            if block_similarity(src_block, dst_block) >= FUZZY_SIMILARITY_THRESHOLD {
                let mv = RowBlockMove {
                    src_start_row: src_block[0].row_idx,
                    dst_start_row: dst_block[0].row_idx,
                    row_count: block_len as u32,
                };
                if mv.src_start_row != mv.dst_start_row {
                    if candidate.is_some() {
                        return None;
                    }
                    candidate = Some(mv);
                }
            }
        }
    }

    candidate
}

pub(crate) fn align_row_changes(old: &Grid, new: &Grid) -> Option<RowAlignment> {
    let row_diff = new.nrows as i64 - old.nrows as i64;
    if row_diff.abs() == 1 {
        return align_single_row_change(old, new);
    }

    align_rows_internal(old, new, true)
}

pub(crate) fn align_single_row_change(old: &Grid, new: &Grid) -> Option<RowAlignment> {
    align_rows_internal(old, new, false)
}

fn align_rows_internal(old: &Grid, new: &Grid, allow_blocks: bool) -> Option<RowAlignment> {
    if !is_within_size_bounds(old, new) {
        return None;
    }

    if old.ncols != new.ncols {
        return None;
    }

    let row_diff = new.nrows as i64 - old.nrows as i64;
    if row_diff == 0 {
        return None;
    }

    let abs_diff = row_diff.unsigned_abs() as u32;

    if !allow_blocks && abs_diff != 1 {
        return None;
    }

    if abs_diff != 1 && (!allow_blocks || abs_diff > MAX_BLOCK_GAP) {
        return None;
    }

    let view_a = GridView::from_grid(old);
    let view_b = GridView::from_grid(new);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats) {
        return None;
    }

    if row_diff == 1 {
        find_single_gap_alignment(
            &view_a.row_meta,
            &view_b.row_meta,
            &stats,
            RowChange::Insert,
        )
    } else if row_diff == -1 {
        find_single_gap_alignment(
            &view_a.row_meta,
            &view_b.row_meta,
            &stats,
            RowChange::Delete,
        )
    } else if !allow_blocks {
        None
    } else if row_diff > 0 {
        find_block_gap_alignment(
            &view_a.row_meta,
            &view_b.row_meta,
            &stats,
            RowChange::Insert,
            abs_diff,
        )
    } else {
        find_block_gap_alignment(
            &view_a.row_meta,
            &view_b.row_meta,
            &stats,
            RowChange::Delete,
            abs_diff,
        )
    }
}

enum RowChange {
    Insert,
    Delete,
}

fn find_single_gap_alignment(
    rows_a: &[crate::grid_view::RowMeta],
    rows_b: &[crate::grid_view::RowMeta],
    stats: &HashStats<RowHash>,
    change: RowChange,
) -> Option<RowAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = false;

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < rows_a.len() && idx_b < rows_b.len() {
        let meta_a = rows_a[idx_a];
        let meta_b = rows_b[idx_b];

        if meta_a.hash == meta_b.hash {
            matched.push((meta_a.row_idx, meta_b.row_idx));
            idx_a += 1;
            idx_b += 1;
            continue;
        }

        if skipped {
            return None;
        }

        match change {
            RowChange::Insert => {
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
                idx_b += 1;
            }
            RowChange::Delete => {
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
                idx_a += 1;
            }
        }

        skipped = true;
    }

    if idx_a < rows_a.len() || idx_b < rows_b.len() {
        if skipped {
            return None;
        }

        match change {
            RowChange::Insert if idx_a == rows_a.len() && rows_b.len() == idx_b + 1 => {
                let meta_b = rows_b[idx_b];
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
            }
            RowChange::Delete if idx_b == rows_b.len() && rows_a.len() == idx_a + 1 => {
                let meta_a = rows_a[idx_a];
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
            }
            _ => return None,
        }
    }

    if inserted.len() + deleted.len() != 1 {
        return None;
    }

    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn find_block_gap_alignment(
    rows_a: &[crate::grid_view::RowMeta],
    rows_b: &[crate::grid_view::RowMeta],
    stats: &HashStats<RowHash>,
    change: RowChange,
    gap: u32,
) -> Option<RowAlignment> {
    let gap = gap as usize;
    if gap == 0 {
        return None;
    }

    let (shorter_len, longer_len) = match change {
        RowChange::Insert => (rows_a.len(), rows_b.len()),
        RowChange::Delete => (rows_b.len(), rows_a.len()),
    };

    if longer_len.saturating_sub(shorter_len) != gap {
        return None;
    }

    let mut prefix = 0usize;
    while prefix < rows_a.len()
        && prefix < rows_b.len()
        && rows_a[prefix].hash == rows_b[prefix].hash
    {
        prefix += 1;
    }

    let mut suffix = 0usize;
    while suffix < shorter_len.saturating_sub(prefix) {
        let idx_a = rows_a.len() - 1 - suffix;
        let idx_b = rows_b.len() - 1 - suffix;
        if rows_a[idx_a].hash == rows_b[idx_b].hash {
            suffix += 1;
        } else {
            break;
        }
    }

    if prefix + suffix != shorter_len {
        return None;
    }

    let mut matched = Vec::with_capacity(shorter_len);
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    match change {
        RowChange::Insert => {
            let block_start = prefix;
            let block_end = block_start + gap;
            if block_end > rows_b.len() {
                return None;
            }

            for meta in &rows_b[block_start..block_end] {
                if !is_unique_to_b(meta.hash, stats) {
                    return None;
                }
                inserted.push(meta.row_idx);
            }

            for (idx, meta_a) in rows_a.iter().enumerate() {
                let b_idx = if idx < block_start { idx } else { idx + gap };
                matched.push((meta_a.row_idx, rows_b[b_idx].row_idx));
            }
        }
        RowChange::Delete => {
            let block_start = prefix;
            let block_end = block_start + gap;
            if block_end > rows_a.len() {
                return None;
            }

            for meta in &rows_a[block_start..block_end] {
                if !is_unique_to_a(meta.hash, stats) {
                    return None;
                }
                deleted.push(meta.row_idx);
            }

            for (idx_b, meta_b) in rows_b.iter().enumerate() {
                let a_idx = if idx_b < block_start {
                    idx_b
                } else {
                    idx_b + gap
                };
                matched.push((rows_a[a_idx].row_idx, meta_b.row_idx));
            }
        }
    }

    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn is_monotonic(pairs: &[(u32, u32)]) -> bool {
    pairs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1)
}

fn is_unique_to_b(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 0
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
}

fn is_unique_to_a(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 0
}

fn is_within_size_bounds(old: &Grid, new: &Grid) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= MAX_ALIGN_ROWS && cols <= MAX_ALIGN_COLS
}

fn low_info_dominated(view: &GridView<'_>) -> bool {
    if view.row_meta.is_empty() {
        return false;
    }

    let low_info_count = view.row_meta.iter().filter(|m| m.is_low_info).count();
    low_info_count * 2 > view.row_meta.len()
}

fn has_heavy_repetition(stats: &HashStats<RowHash>) -> bool {
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > MAX_HASH_REPEAT
}

fn hashes_match(slice_a: &[RowMeta], slice_b: &[RowMeta]) -> bool {
    slice_a.len() == slice_b.len()
        && slice_a
            .iter()
            .zip(slice_b.iter())
            .all(|(a, b)| a.hash == b.hash)
}

fn block_similarity(slice_a: &[RowMeta], slice_b: &[RowMeta]) -> f64 {
    let tokens_a: HashSet<RowHash> = slice_a.iter().map(|m| m.hash).collect();
    let tokens_b: HashSet<RowHash> = slice_b.iter().map(|m| m.hash).collect();

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();
    let jaccard = if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    };

    let positional_matches = slice_a
        .iter()
        .zip(slice_b.iter())
        .filter(|(a, b)| a.hash == b.hash)
        .count();
    let positional_ratio = (positional_matches as f64 + 1.0) / (slice_a.len() as f64 + 1.0);

    jaccard.max(positional_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::{Cell, CellAddress, CellValue};

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
    fn detects_exact_row_block_move() {
        let base: Vec<Vec<i32>> = (1..=20)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
        rows_b.splice(12..12, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let mv =
            detect_exact_row_block_move(&grid_a, &grid_b).expect("expected block move to be found");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 12,
                row_count: 4
            }
        );
    }

    #[test]
    fn block_move_detection_rejects_internal_edits() {
        let base: Vec<Vec<i32>> = (1..=12)
            .map(|r| (1..=2).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(2..5).collect();
        moved_block[1][0] = 9_999;
        rows_b.splice(6..6, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn detects_fuzzy_row_block_move_with_single_internal_edit() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
        moved_block[1][1] = 9_999;
        rows_b.splice(12..12, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b)
            .expect("expected fuzzy row block move to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 12,
                row_count: 4
            }
        );
    }

    #[test]
    fn fuzzy_move_rejects_low_similarity_block() {
        let base: Vec<Vec<i32>> = (1..=16)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(3..7).collect();
        for row in &mut moved_block {
            for value in row.iter_mut() {
                *value += 50_000;
            }
        }
        rows_b.splice(10..10, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b).is_none());
        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b).is_none(),
            "similarity below threshold should bail out"
        );
    }

    #[test]
    fn fuzzy_move_bails_on_heavy_repetition_or_ambiguous_candidates() {
        let repeated_row = [1, 2];
        let rows_a: Vec<Vec<i32>> = (0..10).map(|_| repeated_row.to_vec()).collect();
        let mut rows_b = rows_a.clone();

        let block: Vec<Vec<i32>> = rows_b.drain(0..3).collect();
        rows_b.splice(5..5, block);

        let rows_a_refs: Vec<&[i32]> = rows_a.iter().map(|row| row.as_slice()).collect();
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&rows_a_refs);
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b).is_none(),
            "heavy repetition or ambiguous candidates should not emit a move"
        );
    }

    #[test]
    fn fuzzy_move_noop_when_grids_identical() {
        let base: Vec<Vec<i32>> = (1..=6)
            .map(|r| (1..=2).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);
        let grid_b = grid_from_rows(&base_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b).is_none());
        assert!(detect_fuzzy_row_block_move(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn detects_fuzzy_row_block_move_upward_with_single_internal_edit() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(12..16).collect();
        moved_block[1][1] = 9_999;
        rows_b.splice(4..4, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b)
            .expect("expected fuzzy row block move upward to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 12,
                dst_start_row: 4,
                row_count: 4
            }
        );
    }

    #[test]
    fn fuzzy_move_bails_on_ambiguous_candidates_below_repetition_threshold() {
        let base: Vec<Vec<i32>> = (1..=16)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_baseline_a = grid_from_rows(&base_refs);

        let mut rows_baseline_b = base.clone();
        let mut moved: Vec<Vec<i32>> = rows_baseline_b.drain(3..7).collect();
        moved[1][1] = 9999;
        rows_baseline_b.splice(10..10, moved);
        let refs_baseline_b: Vec<&[i32]> =
            rows_baseline_b.iter().map(|row| row.as_slice()).collect();
        let grid_baseline_b = grid_from_rows(&refs_baseline_b);

        assert!(
            detect_fuzzy_row_block_move(&grid_baseline_a, &grid_baseline_b).is_some(),
            "baseline: non-ambiguous fuzzy move should be detected"
        );

        let rows_a: Vec<Vec<i32>> = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![100, 200, 300],
            vec![101, 201, 301],
            vec![102, 202, 302],
            vec![103, 203, 303],
            vec![100, 200, 300],
            vec![101, 201, 301],
            vec![102, 202, 302],
            vec![103, 203, 999],
            vec![31, 32, 33],
            vec![34, 35, 36],
        ];

        let mut rows_b = rows_a.clone();
        let block1: Vec<Vec<i32>> = rows_b.drain(2..6).collect();
        rows_b.splice(6..6, block1);

        let refs_a: Vec<&[i32]> = rows_a.iter().map(|r| r.as_slice()).collect();
        let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_rows(&refs_a);
        let grid_b = grid_from_rows(&refs_b);

        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b).is_none(),
            "ambiguous candidates: two similar blocks swapped should trigger ambiguity bail-out"
        );
    }

    #[test]
    fn fuzzy_move_at_max_block_rows_threshold() {
        let base: Vec<Vec<i32>> = (1..=70)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..36).collect();
        moved_block[15][1] = 9_999;
        rows_b.splice(36..36, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b)
            .expect("expected fuzzy move at MAX_FUZZY_BLOCK_ROWS to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 36,
                row_count: 32
            }
        );
    }

    #[test]
    fn fuzzy_move_at_max_hash_repeat_boundary() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_base = grid_from_rows(&base_refs);

        let mut rows_moved = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_moved.drain(4..8).collect();
        moved_block[1][1] = 9_999;
        rows_moved.splice(12..12, moved_block);
        let moved_refs: Vec<&[i32]> = rows_moved.iter().map(|row| row.as_slice()).collect();
        let grid_moved = grid_from_rows(&moved_refs);

        assert!(
            detect_fuzzy_row_block_move(&grid_base, &grid_moved).is_some(),
            "baseline: fuzzy move should work with unique rows"
        );

        let mut base_9_repeat: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        for row in base_9_repeat.iter_mut().take(9) {
            *row = vec![999, 888, 777];
        }
        let refs_9a: Vec<&[i32]> = base_9_repeat.iter().map(|r| r.as_slice()).collect();
        let grid_9a = grid_from_rows(&refs_9a);

        let mut rows_9b = base_9_repeat.clone();
        let mut moved_9: Vec<Vec<i32>> = rows_9b.drain(10..14).collect();
        moved_9[1][1] = 8_888;
        rows_9b.splice(14..14, moved_9);
        let refs_9b: Vec<&[i32]> = rows_9b.iter().map(|r| r.as_slice()).collect();
        let grid_9b = grid_from_rows(&refs_9b);

        assert!(
            detect_fuzzy_row_block_move(&grid_9a, &grid_9b).is_none(),
            "9 repeated rows (> MAX_HASH_REPEAT) should trigger heavy repetition guard"
        );

        let mut base_8_repeat: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        for row in base_8_repeat.iter_mut().take(8) {
            *row = vec![999, 888, 777];
        }
        let refs_8a: Vec<&[i32]> = base_8_repeat.iter().map(|r| r.as_slice()).collect();
        let grid_8a = grid_from_rows(&refs_8a);

        let mut rows_8b = base_8_repeat.clone();
        let mut moved_8: Vec<Vec<i32>> = rows_8b.drain(9..13).collect();
        moved_8[1][1] = 8_888;
        rows_8b.splice(14..14, moved_8);
        let refs_8b: Vec<&[i32]> = rows_8b.iter().map(|r| r.as_slice()).collect();
        let grid_8b = grid_from_rows(&refs_8b);

        assert!(
            detect_fuzzy_row_block_move(&grid_8a, &grid_8b).is_some(),
            "exactly 8 repeated rows (= MAX_HASH_REPEAT, not >) should NOT trigger heavy repetition guard"
        );
    }

    #[test]
    fn aligns_contiguous_block_insert_middle() {
        let base: Vec<Vec<i32>> = (1..=10)
            .map(|r| (1..=4).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let inserted_block: Vec<Vec<i32>> = (0..4)
            .map(|idx| vec![1_000 + idx, 2_000 + idx, 3_000 + idx, 4_000 + idx])
            .collect();
        let mut rows_b = base.clone();
        rows_b.splice(3..3, inserted_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let alignment = align_row_changes(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![3, 4, 5, 6]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 10);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[2], (2, 2));
        assert_eq!(alignment.matched[3], (3, 7));
        assert_eq!(alignment.matched.last(), Some(&(9, 13)));
        assert!(is_monotonic(&alignment.matched));
    }

    #[test]
    fn aligns_contiguous_block_delete_middle() {
        let base: Vec<Vec<i32>> = (1..=10)
            .map(|r| (1..=4).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        rows_b.drain(3..7);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let alignment = align_row_changes(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.deleted, vec![3, 4, 5, 6]);
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.matched.len(), 6);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[2], (2, 2));
        assert_eq!(alignment.matched[3], (7, 3));
        assert_eq!(alignment.matched.last(), Some(&(9, 5)));
        assert!(is_monotonic(&alignment.matched));
    }

    #[test]
    fn block_alignment_bails_on_noncontiguous_changes() {
        let base: Vec<Vec<i32>> = (1..=8)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        rows_b.insert(1, vec![999, 1_000, 1_001]);
        rows_b.insert(5, vec![2_000, 2_001, 2_002]);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(align_row_changes(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn aligns_single_insert_with_unique_row() {
        let base = (1..=10)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base_refs.clone();
        rows_b.insert(
            5,
            &[999, 1000, 1001], // inserted at position 6 (1-based)
        );
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 10);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
        assert_eq!(alignment.matched[5], (5, 6));
        assert_eq!(alignment.matched.last(), Some(&(9, 10)));
    }

    #[test]
    fn rejects_non_monotonic_alignment_with_extra_mismatch() {
        let base_rows = [[11, 12, 13], [21, 22, 23], [31, 32, 33], [41, 42, 43]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = vec![
            base_refs[0],       // same
            &[999, 1000, 1001], // inserted unique row
            base_refs[2],       // move row 3 before row 2 to break monotonicity
            base_refs[1],
            base_refs[3],
        ];
        let grid_b = grid_from_rows(&rows_b);

        assert!(align_single_row_change(&grid_a, &grid_b).is_none());
    }

    #[test]
    fn aligns_insert_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_first_row = [999, 1000, 1001];
        let mut rows_b = vec![new_first_row.as_slice()];
        rows_b.extend(base_refs.iter().copied());
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![0]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 1));
        assert_eq!(alignment.matched[4], (4, 5));
    }

    #[test]
    fn aligns_insert_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_last_row = [999, 1000, 1001];
        let mut rows_b: Vec<&[i32]> = base_refs.clone();
        rows_b.push(new_last_row.as_slice());
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
    }

    #[test]
    fn aligns_delete_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[1..].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![0]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (1, 0));
        assert_eq!(alignment.matched[3], (4, 3));
    }

    #[test]
    fn aligns_delete_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[..4].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![4]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[3], (3, 3));
    }

    #[test]
    fn aligns_single_row_to_two_rows_via_insert() {
        let single_row = [[42, 43, 44]];
        let single_refs: Vec<&[i32]> = single_row.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&single_refs);

        let new_row = [999, 1000, 1001];
        let rows_b: Vec<&[i32]> = vec![single_refs[0], new_row.as_slice()];
        let grid_b = grid_from_rows(&rows_b);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![1]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn aligns_two_rows_to_single_row_via_delete() {
        let two_rows = [[42, 43, 44], [99, 100, 101]];
        let two_refs: Vec<&[i32]> = two_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&two_refs);

        let single_refs: Vec<&[i32]> = vec![two_refs[0]];
        let grid_b = grid_from_rows(&single_refs);

        let alignment =
            align_single_row_change(&grid_a, &grid_b).expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![1]);
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn monotonicity_helper_accepts_valid_sequence() {
        let valid: Vec<(u32, u32)> = vec![(0, 0), (1, 2), (3, 4), (5, 7)];
        assert!(super::is_monotonic(&valid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_a() {
        let invalid: Vec<(u32, u32)> = vec![(0, 0), (2, 2), (1, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_b() {
        let invalid: Vec<(u32, u32)> = vec![(0, 3), (1, 2), (2, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_accepts_empty_and_single() {
        assert!(super::is_monotonic(&[]));
        assert!(super::is_monotonic(&[(5, 10)]));
    }
}
```

---

### File: `core\src\workbook.rs`

```rust
//! Workbook, sheet, and grid data structures.
//!
//! This module defines the core intermediate representation (IR) for Excel workbooks:
//! - [`Workbook`]: A collection of sheets
//! - [`Sheet`]: A named sheet with a grid of cells
//! - [`Grid`]: A sparse 2D grid of cells with optional row/column signatures
//! - [`Cell`]: Individual cell with address, value, and optional formula

use crate::addressing::{AddressParseError, address_to_index, index_to_address};
use crate::hashing::{combine_hashes, hash_cell_contribution};
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
        debug_assert!(
            cell.row < self.nrows && cell.col < self.ncols,
            "cell coordinates must lie within the grid bounds"
        );
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
        let hash = self
            .cells
            .values()
            .filter(|cell| cell.row == row)
            .fold(0u64, |acc, cell| {
                combine_hashes(acc, hash_cell_contribution(cell.col, cell))
            });
        RowSignature { hash }
    }

    pub fn compute_col_signature(&self, col: u32) -> ColSignature {
        let hash = self
            .cells
            .values()
            .filter(|cell| cell.col == col)
            .fold(0u64, |acc, cell| {
                combine_hashes(acc, hash_cell_contribution(cell.row, cell))
            });
        ColSignature { hash }
    }

    pub fn compute_all_signatures(&mut self) {
        let mut row_hashes = vec![0u64; self.nrows as usize];
        let mut col_hashes = vec![0u64; self.ncols as usize];

        for cell in self.cells.values() {
            let row_idx = cell.row as usize;
            let col_idx = cell.col as usize;

            debug_assert!(
                row_idx < row_hashes.len() && col_idx < col_hashes.len(),
                "cell coordinates must lie within the grid bounds"
            );

            let row_contribution = hash_cell_contribution(cell.col, cell);
            row_hashes[row_idx] = combine_hashes(row_hashes[row_idx], row_contribution);

            let col_contribution = hash_cell_contribution(cell.row, cell);
            col_hashes[col_idx] = combine_hashes(col_hashes[col_idx], col_contribution);
        }

        self.row_signatures = Some(
            row_hashes
                .into_iter()
                .map(|hash| RowSignature { hash })
                .collect(),
        );

        self.col_signatures = Some(
            col_hashes
                .into_iter()
                .map(|hash| ColSignature { hash })
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
```

---

### File: `core\src\output\json.rs`

```rust
use crate::diff::DiffReport;
#[cfg(feature = "excel-open-xml")]
use crate::engine::diff_workbooks as compute_diff;
#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{ExcelOpenError, open_workbook};
use serde::Serialize;
use serde::ser::Error as SerdeError;
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

pub fn serialize_cell_diffs(diffs: &[CellDiff]) -> serde_json::Result<String> {
    serde_json::to_string(diffs)
}

pub fn serialize_diff_report(report: &DiffReport) -> serde_json::Result<String> {
    if contains_non_finite_numbers(report) {
        return Err(SerdeError::custom(
            "non-finite numbers (NaN or infinity) are not supported in DiffReport JSON output",
        ));
    }
    serde_json::to_string(report)
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

    report
        .ops
        .iter()
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

fn contains_non_finite_numbers(report: &DiffReport) -> bool {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    report.ops.iter().any(|op| match op {
        DiffOp::CellEdited { from, to, .. } => {
            matches!(from.value, Some(CellValue::Number(n)) if !n.is_finite())
                || matches!(to.value, Some(CellValue::Number(n)) if !n.is_finite())
        }
        _ => false,
    })
}
```

---

### File: `core\src\output\mod.rs`

```rust
pub mod json;
```

---

### File: `core\tests\addressing_pg2_tests.rs`

```rust
use excel_diff::{CellValue, address_to_index, index_to_address, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg2_addressing_matrix_consistency() {
    let workbook =
        open_workbook(fixture_path("pg2_addressing_matrix.xlsx")).expect("address fixture opens");
    let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == "Addresses")
        .unwrap_or_else(|| panic!("Addresses sheet present; found {:?}", sheet_names));

    for cell in sheet.grid.iter_cells() {
        if let Some(CellValue::Text(text)) = &cell.value {
            assert_eq!(cell.address.to_a1(), text.as_str());
            let (r, c) = address_to_index(text).expect("address strings should parse to indices");
            assert_eq!((r, c), (cell.row, cell.col));
            assert_eq!(index_to_address(cell.row, cell.col), cell.address.to_a1());
        }
    }
}
```

---

### File: `core\tests\d1_database_mode_tests.rs`

```rust
use excel_diff::{DiffOp, Grid, Workbook, diff_grids_database_mode, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers};

fn data_grid(workbook: &Workbook) -> &Grid {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == "Data")
        .map(|s| &s.grid)
        .expect("Data sheet present")
}

#[test]
fn d1_equal_ordered_database_mode_empty_diff() {
    let workbook = open_workbook(fixture_path("db_equal_ordered_a.xlsx")).expect("fixture A opens");
    let grid = data_grid(&workbook);

    let report = diff_grids_database_mode(grid, grid, &[0]);
    assert!(
        report.ops.is_empty(),
        "database mode should ignore row order when keyed rows are identical"
    );
}

#[test]
fn d1_equal_reordered_database_mode_empty_diff() {
    let wb_a = open_workbook(fixture_path("db_equal_ordered_a.xlsx")).expect("fixture A opens");
    let wb_b = open_workbook(fixture_path("db_equal_ordered_b.xlsx")).expect("fixture B opens");

    let grid_a = data_grid(&wb_a);
    let grid_b = data_grid(&wb_b);

    let report = diff_grids_database_mode(grid_a, grid_b, &[0]);
    assert!(
        report.ops.is_empty(),
        "keyed alignment should match rows by key and ignore reordering"
    );
}

#[test]
fn d1_spreadsheet_mode_sees_reorder_as_changes() {
    let wb_a = open_workbook(fixture_path("db_equal_ordered_a.xlsx")).expect("fixture A opens");
    let wb_b = open_workbook(fixture_path("db_equal_ordered_b.xlsx")).expect("fixture B opens");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report.ops.is_empty(),
        "Spreadsheet Mode should see structural changes when rows are reordered, \
         demonstrating the semantic difference from Database Mode"
    );
}

#[test]
fn d1_duplicate_keys_fallback_to_spreadsheet_mode() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[1, 99]]);
    let grid_b = grid_from_numbers(&[&[1, 10]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

    assert!(
        !report.ops.is_empty(),
        "duplicate keys cause fallback to spreadsheet mode which should detect differences"
    );

    let has_row_removed = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::RowRemoved { .. }));
    assert!(
        has_row_removed,
        "spreadsheet mode fallback should emit RowRemoved for the missing row"
    );
}

#[test]
fn d1_database_mode_row_added() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20]]);
    let grid_b = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "database mode should emit one RowAdded for key 3"
    );
}

#[test]
fn d1_database_mode_row_removed() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);
    let grid_b = grid_from_numbers(&[&[1, 10], &[2, 20]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "database mode should emit one RowRemoved for key 3"
    );
}

#[test]
fn d1_database_mode_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20]]);
    let grid_b = grid_from_numbers(&[&[1, 99], &[2, 20]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "database mode should emit one CellEdited for the changed non-key cell"
    );
}

#[test]
fn d1_database_mode_cell_edited_with_reorder() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[2, 20], &[3, 30]]);
    let grid_b = grid_from_numbers(&[&[3, 30], &[2, 99], &[1, 10]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0]);

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "database mode should ignore reordering and find only the cell edit for key 2"
    );
}

#[test]
fn d5_composite_key_equal_reordered_database_mode_empty_diff() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 20, 200], &[2, 10, 300]]);
    let grid_b = grid_from_numbers(&[&[2, 10, 300], &[1, 10, 100], &[1, 20, 200]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);
    assert!(
        report.ops.is_empty(),
        "composite keyed alignment should ignore row order differences"
    );
}

#[test]
fn d5_composite_key_row_added_and_cell_edited() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 20, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 150], &[1, 20, 200], &[2, 30, 300]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "new composite key should produce exactly one RowAdded"
    );

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 0,
        "no rows should be removed when only a new composite key is introduced"
    );

    let mut cell_edited_iter = report.ops.iter().filter_map(|op| {
        if let DiffOp::CellEdited { addr, .. } = op {
            Some(addr)
        } else {
            None
        }
    });

    let edited_addr = cell_edited_iter
        .next()
        .expect("one cell edit for changed non-key value");
    assert!(
        cell_edited_iter.next().is_none(),
        "only one CellEdited should be present"
    );
    assert_eq!(edited_addr.col, 2, "only non-key column should be edited");
    assert_eq!(
        edited_addr.row, 0,
        "cell edit should reference the row of key (1,10) in the new grid"
    );
}

#[test]
fn d5_composite_key_partial_key_mismatch_yields_add_and_remove() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100]]);
    let grid_b = grid_from_numbers(&[&[1, 20, 100]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "changed composite key should remove the old tuple"
    );

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "changed composite key should add the new tuple"
    );

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 0,
        "partial key match must not be treated as a cell edit"
    );
}

#[test]
fn d5_composite_key_duplicate_keys_fallback_to_spreadsheet_mode() {
    let grid_a = grid_from_numbers(&[&[1, 10, 100], &[1, 10, 200]]);
    let grid_b = grid_from_numbers(&[&[1, 10, 100]]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1]);

    assert!(
        !report.ops.is_empty(),
        "duplicate composite keys should trigger spreadsheet-mode fallback"
    );

    let has_row_removed = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::RowRemoved { .. }));
    assert!(
        has_row_removed,
        "fallback should emit a RowRemoved reflecting duplicate handling"
    );
}

#[test]
fn d5_non_contiguous_key_columns_equal_reordered_empty_diff() {
    let grid_a = grid_from_numbers(&[
        &[1, 999, 10, 100],
        &[1, 888, 20, 200],
        &[2, 777, 10, 300],
    ]);
    let grid_b = grid_from_numbers(&[
        &[2, 777, 10, 300],
        &[1, 999, 10, 100],
        &[1, 888, 20, 200],
    ]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 2]);
    assert!(
        report.ops.is_empty(),
        "non-contiguous key columns [0,2] should align correctly ignoring row order"
    );
}

#[test]
fn d5_non_contiguous_key_columns_detects_edits_in_skipped_column() {
    let grid_a = grid_from_numbers(&[
        &[1, 999, 10, 100],
        &[1, 888, 20, 200],
        &[2, 777, 10, 300],
    ]);
    let grid_b = grid_from_numbers(&[
        &[2, 111, 10, 300],
        &[1, 222, 10, 100],
        &[1, 333, 20, 200],
    ]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 2]);

    let cell_edited_ops: Vec<_> = report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, .. } = op {
                Some(addr)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        cell_edited_ops.len(),
        3,
        "should detect 3 edits in skipped non-key column 1"
    );

    for addr in &cell_edited_ops {
        assert_eq!(
            addr.col, 1,
            "all edits should be in the skipped column 1, not key columns 0 or 2"
        );
    }

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(row_added_count, 0, "no rows should be added");

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(row_removed_count, 0, "no rows should be removed");
}

#[test]
fn d5_non_contiguous_key_columns_row_added_and_cell_edited() {
    let grid_a = grid_from_numbers(&[
        &[1, 999, 10, 100],
        &[1, 888, 20, 200],
    ]);
    let grid_b = grid_from_numbers(&[
        &[1, 999, 10, 150],
        &[1, 888, 20, 200],
        &[2, 777, 30, 300],
    ]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 2]);

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "new non-contiguous composite key should produce exactly one RowAdded"
    );

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(row_removed_count, 0, "no rows should be removed");

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 1,
        "changed non-key column should produce exactly one CellEdited"
    );
}

#[test]
fn d5_three_column_composite_key_equal_reordered_empty_diff() {
    let grid_a = grid_from_numbers(&[
        &[1, 10, 100, 1000],
        &[1, 10, 200, 2000],
        &[1, 20, 100, 3000],
        &[2, 10, 100, 4000],
    ]);
    let grid_b = grid_from_numbers(&[
        &[2, 10, 100, 4000],
        &[1, 20, 100, 3000],
        &[1, 10, 200, 2000],
        &[1, 10, 100, 1000],
    ]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1, 2]);
    assert!(
        report.ops.is_empty(),
        "three-column composite key should align correctly ignoring row order"
    );
}

#[test]
fn d5_three_column_composite_key_partial_match_yields_add_and_remove() {
    let grid_a = grid_from_numbers(&[
        &[1, 10, 100, 1000],
    ]);
    let grid_b = grid_from_numbers(&[
        &[1, 10, 200, 1000],
    ]);

    let report = diff_grids_database_mode(&grid_a, &grid_b, &[0, 1, 2]);

    let row_removed_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .count();
    assert_eq!(
        row_removed_count, 1,
        "changed third key column should remove the old tuple"
    );

    let row_added_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .count();
    assert_eq!(
        row_added_count, 1,
        "changed third key column should add the new tuple"
    );

    let cell_edited_count = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .count();
    assert_eq!(
        cell_edited_count, 0,
        "partial three-column key match must not be treated as a cell edit"
    );
}
```

---

### File: `core\tests\data_mashup_tests.rs`

```rust
use std::fs::File;
use std::io::{ErrorKind, Read};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use excel_diff::{
    ContainerError, DataMashupError, ExcelOpenError, RawDataMashup, build_data_mashup,
    open_data_mashup,
};
use quick_xml::{Reader, events::Event};
use zip::ZipArchive;

mod common;
use common::fixture_path;

#[test]
fn workbook_without_datamashup_returns_none() {
    let path = fixture_path("minimal.xlsx");
    let result = open_data_mashup(&path).expect("minimal workbook should load");
    assert!(result.is_none());
}

#[test]
fn workbook_with_valid_datamashup_parses() {
    let path = fixture_path("m_change_literal_b.xlsx");
    let raw = open_data_mashup(&path)
        .expect("valid mashup should load")
        .expect("mashup should be present");

    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
    assert!(!raw.metadata.is_empty());

    let assembled = assemble_top_level_bytes(&raw);
    let expected = datamashup_bytes_from_fixture(&path);
    assert_eq!(assembled, expected);
}

#[test]
fn datamashup_with_base64_whitespace_parses() {
    let path = fixture_path("mashup_base64_whitespace.xlsx");
    let raw = open_data_mashup(&path)
        .expect("whitespace in base64 payload should be tolerated")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_le_datamashup_parses() {
    let path = fixture_path("mashup_utf16_le.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16LE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn utf16_be_datamashup_parses() {
    let path = fixture_path("mashup_utf16_be.xlsx");
    let raw = open_data_mashup(&path)
        .expect("UTF-16BE mashup should load")
        .expect("mashup should be present");
    assert_eq!(raw.version, 0);
    assert!(!raw.package_parts.is_empty());
}

#[test]
fn corrupt_base64_returns_error() {
    let path = fixture_path("corrupt_base64.xlsx");
    let err = open_data_mashup(&path).expect_err("corrupt base64 should fail");
    assert!(matches!(
        err,
        ExcelOpenError::DataMashup(DataMashupError::Base64Invalid)
    ));
}

#[test]
fn duplicate_datamashup_parts_are_rejected() {
    let path = fixture_path("duplicate_datamashup_parts.xlsx");
    let err = open_data_mashup(&path).expect_err("duplicate DataMashup parts should be rejected");
    assert!(matches!(
        err,
        ExcelOpenError::DataMashup(DataMashupError::FramingInvalid)
    ));
}

#[test]
fn duplicate_datamashup_elements_are_rejected() {
    let path = fixture_path("duplicate_datamashup_elements.xlsx");
    let err =
        open_data_mashup(&path).expect_err("duplicate DataMashup elements should be rejected");
    assert!(matches!(
        err,
        ExcelOpenError::DataMashup(DataMashupError::FramingInvalid)
    ));
}

#[test]
fn nonexistent_file_returns_io() {
    let path = fixture_path("missing_mashup.xlsx");
    let err = open_data_mashup(&path).expect_err("missing file should error");
    match err {
        ExcelOpenError::Container(ContainerError::Io(e)) => {
            assert_eq!(e.kind(), ErrorKind::NotFound)
        }
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn non_excel_container_returns_not_excel_error() {
    let path = fixture_path("random_zip.zip");
    let err = open_data_mashup(&path).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn missing_content_types_is_not_excel_error() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_data_mashup(&path).expect_err("missing [Content_Types].xml should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn non_zip_file_returns_not_zip_error() {
    let path = fixture_path("not_a_zip.txt");
    let err = open_data_mashup(&path).expect_err("non-zip input should not parse as Excel");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotZipContainer)
    ));
}

#[test]
fn build_data_mashup_smoke_from_fixture() {
    let raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    let dm = build_data_mashup(&raw).expect("build_data_mashup should succeed");

    assert_eq!(dm.version, 0);
    assert!(
        dm.package_parts
            .main_section
            .source
            .contains("section Section1;")
    );
    assert!(!dm.metadata.formulas.is_empty());

    let non_connection: Vec<_> = dm
        .metadata
        .formulas
        .iter()
        .filter(|m| m.section_name == "Section1" && !m.is_connection_only)
        .collect();
    assert_eq!(non_connection.len(), 1);
    let meta = non_connection[0];
    assert_eq!(
        meta.item_path,
        format!("{}/{}", meta.section_name, meta.formula_name)
    );
    assert_eq!(meta.item_path, "Section1/Query1");
    assert_eq!(meta.section_name, "Section1");
    assert_eq!(meta.formula_name, "Query1");
    assert!(meta.load_to_sheet || meta.load_to_model);
}

fn datamashup_bytes_from_fixture(path: &std::path::Path) -> Vec<u8> {
    let file = File::open(path).expect("fixture should be readable");
    let mut archive = ZipArchive::new(file).expect("fixture should be a zip container");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip entry should be readable");
        let name = file.name().to_string();
        if !name.starts_with("customXml/") || !name.ends_with(".xml") {
            continue;
        }

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("XML part should read");
        if let Some(text) = extract_datamashup_base64(&buf) {
            let cleaned: String = text.split_whitespace().collect();
            return STANDARD
                .decode(cleaned.as_bytes())
                .expect("DataMashup base64 should decode");
        }
    }

    panic!("DataMashup element not found in {}", path.display());
}

fn extract_datamashup_base64(xml: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup {
                    return None;
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t.unescape().ok()?.into_owned();
                content.push_str(&text);
            }
            Ok(Event::CData(t)) if in_datamashup => {
                content.push_str(&String::from_utf8_lossy(&t.into_inner()));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return None;
                }
                return Some(content.clone());
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn assemble_top_level_bytes(raw: &RawDataMashup) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&raw.version.to_le_bytes());
    bytes.extend_from_slice(&(raw.package_parts.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.package_parts);
    bytes.extend_from_slice(&(raw.permissions.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permissions);
    bytes.extend_from_slice(&(raw.metadata.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.metadata);
    bytes.extend_from_slice(&(raw.permission_bindings.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&raw.permission_bindings);
    bytes
}
```

---

### File: `core\tests\engine_tests.rs`

```rust
use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, DiffOp, DiffReport, Grid, Sheet, SheetKind,
    Workbook, diff_workbooks,
};

type SheetSpec<'a> = (&'a str, Vec<(u32, u32, f64)>);

fn make_workbook(sheets: Vec<SheetSpec<'_>>) -> Workbook {
    let sheet_ir: Vec<Sheet> = sheets
        .into_iter()
        .map(|(name, cells)| {
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
            Sheet {
                name: name.to_string(),
                kind: SheetKind::Worksheet,
                grid,
            }
        })
        .collect();
    Workbook { sheets: sheet_ir }
}

fn make_sheet_with_kind(name: &str, kind: SheetKind, cells: Vec<(u32, u32, f64)>) -> Sheet {
    let (nrows, ncols) = if cells.is_empty() {
        (0, 0)
    } else {
        let max_row = cells.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
        let max_col = cells.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
        (max_row + 1, max_col + 1)
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, c, val) in cells {
        grid.insert(Cell {
            row: r,
            col: c,
            address: CellAddress::from_indices(r, c),
            value: Some(CellValue::Number(val)),
            formula: None,
        });
    }

    Sheet {
        name: name.to_string(),
        kind,
        grid,
    }
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
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetAdded { sheet } if sheet == "Sheet2"))
    );
}

#[test]
fn sheet_removed_detected() {
    let old = make_workbook(vec![
        ("Sheet1", vec![(0, 0, 1.0)]),
        ("Sheet2", vec![(0, 0, 2.0)]),
    ]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let report = diff_workbooks(&old, &new);
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::SheetRemoved { sheet } if sheet == "Sheet2"))
    );
}

#[test]
fn cell_edited_detected() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("Sheet1", vec![(0, 0, 2.0)])]);
    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
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

#[test]
fn sheet_name_case_insensitive_no_changes() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 1.0)])]);

    let report = diff_workbooks(&old, &new);
    assert!(report.ops.is_empty());
}

#[test]
fn sheet_name_case_insensitive_cell_edit() {
    let old = make_workbook(vec![("Sheet1", vec![(0, 0, 1.0)])]);
    let new = make_workbook(vec![("sheet1", vec![(0, 0, 2.0)])]);

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn sheet_identity_includes_kind() {
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

    let old = Workbook {
        sheets: vec![worksheet],
    };
    let new = Workbook {
        sheets: vec![chart],
    };

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
    assert_eq!(
        removed, 1,
        "expected one SheetRemoved for Worksheet 'Sheet1'"
    );
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[test]
fn deterministic_sheet_op_ordering() {
    let budget_old = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let budget_new = make_sheet_with_kind("Budget", SheetKind::Worksheet, vec![(0, 0, 2.0)]);
    let sheet1_old = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 1, 5.0)]);
    let sheet1_chart = make_sheet_with_kind("sheet1", SheetKind::Chart, Vec::new());
    let summary_new = make_sheet_with_kind("Summary", SheetKind::Worksheet, vec![(0, 0, 3.0)]);

    let old = Workbook {
        sheets: vec![budget_old.clone(), sheet1_old],
    };
    let new = Workbook {
        sheets: vec![budget_new.clone(), sheet1_chart, summary_new],
    };

    let budget_addr = CellAddress::from_indices(0, 0);
    let expected = vec![
        DiffOp::cell_edited(
            "Budget".into(),
            budget_addr,
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(1.0)),
                formula: None,
            },
            CellSnapshot {
                addr: budget_addr,
                value: Some(CellValue::Number(2.0)),
                formula: None,
            },
        ),
        DiffOp::SheetRemoved {
            sheet: "Sheet1".into(),
        },
        DiffOp::SheetAdded {
            sheet: "sheet1".into(),
        },
        DiffOp::SheetAdded {
            sheet: "Summary".into(),
        },
    ];

    let report = diff_workbooks(&old, &new);
    assert_eq!(
        report.ops, expected,
        "ops should be ordered by lowercase name then sheet kind"
    );
}

#[test]
fn sheet_identity_includes_kind_for_macro_and_other() {
    let mut grid = Grid::new(1, 1);
    grid.insert(Cell {
        row: 0,
        col: 0,
        address: CellAddress::from_indices(0, 0),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    let macro_sheet = Sheet {
        name: "Code".to_string(),
        kind: SheetKind::Macro,
        grid: grid.clone(),
    };

    let other_sheet = Sheet {
        name: "Code".to_string(),
        kind: SheetKind::Other,
        grid,
    };

    let old = Workbook {
        sheets: vec![macro_sheet],
    };
    let new = Workbook {
        sheets: vec![other_sheet],
    };

    let report = diff_workbooks(&old, &new);

    let mut added = 0;
    let mut removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Code" => added += 1,
            DiffOp::SheetRemoved { sheet } if sheet == "Code" => removed += 1,
            _ => {}
        }
    }

    assert_eq!(added, 1, "expected one SheetAdded for Other 'Code'");
    assert_eq!(removed, 1, "expected one SheetRemoved for Macro 'Code'");
    assert_eq!(report.ops.len(), 2, "no other ops expected");
}

#[cfg(not(debug_assertions))]
#[test]
fn duplicate_sheet_identity_last_writer_wins_release() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);

    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1, "expected last writer to win");

    match &report.ops[0] {
        DiffOp::SheetRemoved { sheet } => assert_eq!(
            sheet, "sheet1",
            "duplicate identity should prefer the last sheet in release builds"
        ),
        other => panic!("expected SheetRemoved, got {other:?}"),
    }
}

#[test]
fn duplicate_sheet_identity_panics_in_debug() {
    let duplicate_a = make_sheet_with_kind("Sheet1", SheetKind::Worksheet, vec![(0, 0, 1.0)]);
    let duplicate_b = make_sheet_with_kind("sheet1", SheetKind::Worksheet, vec![(0, 1, 2.0)]);
    let old = Workbook {
        sheets: vec![duplicate_a, duplicate_b],
    };
    let new = Workbook { sheets: Vec::new() };

    let result = std::panic::catch_unwind(|| diff_workbooks(&old, &new));
    if cfg!(debug_assertions) {
        assert!(
            result.is_err(),
            "duplicate sheet identities should trigger a debug assertion"
        );
    } else {
        assert!(result.is_ok(), "debug assertions disabled should not panic");
    }
}
```

---

### File: `core\tests\excel_open_xml_tests.rs`

```rust
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::time::SystemTime;

use excel_diff::{ContainerError, ExcelOpenError, SheetKind, open_workbook};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

mod common;
use common::fixture_path;

fn temp_xlsx_path(prefix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    path.push(format!("excel_diff_{prefix}_{nanos}.xlsx"));
    path
}

fn write_zip(entries: &[(&str, &str)], path: &Path) {
    let file = fs::File::create(path).expect("create temp zip");
    let mut writer = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, contents) in entries {
        writer.start_file(*name, options).expect("start zip entry");
        writer
            .write_all(contents.as_bytes())
            .expect("write zip entry");
    }

    writer.finish().expect("finish zip");
}

#[test]
fn open_minimal_workbook_succeeds() {
    let path = fixture_path("minimal.xlsx");
    let workbook = open_workbook(&path).expect("minimal workbook should open");
    assert_eq!(workbook.sheets.len(), 1);

    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.name, "Sheet1");
    assert!(matches!(sheet.kind, SheetKind::Worksheet));
    assert_eq!(sheet.grid.nrows, 1);
    assert_eq!(sheet.grid.ncols, 1);

    let cell = sheet.grid.get(0, 0).expect("A1 should be present");
    assert_eq!(cell.address.to_a1(), "A1");
    assert!(cell.value.is_some());
}

#[test]
fn open_nonexistent_file_returns_io_error() {
    let path = fixture_path("definitely_missing.xlsx");
    let err = open_workbook(&path).expect_err("missing file should error");
    match err {
        ExcelOpenError::Container(ContainerError::Io(e)) => {
            assert_eq!(e.kind(), ErrorKind::NotFound)
        }
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn random_zip_is_not_excel() {
    let path = fixture_path("random_zip.zip");
    let err = open_workbook(&path).expect_err("random zip should not parse");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn no_content_types_is_not_excel() {
    let path = fixture_path("no_content_types.xlsx");
    let err = open_workbook(&path).expect_err("missing content types should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotOpcPackage)
    ));
}

#[test]
fn not_zip_container_returns_error() {
    let path = std::env::temp_dir().join("excel_diff_not_zip.txt");
    fs::write(&path, "this is not a zip container").expect("write temp file");
    let err = open_workbook(&path).expect_err("non-zip should fail");
    assert!(matches!(
        err,
        ExcelOpenError::Container(ContainerError::NotZipContainer)
    ));
    let _ = fs::remove_file(&path);
}

#[test]
fn missing_workbook_xml_returns_workbookxmlmissing() {
    let path = temp_xlsx_path("missing_workbook_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    write_zip(&[("[Content_Types].xml", content_types)], &path);

    let err = open_workbook(&path).expect_err("missing workbook xml should error");
    assert!(matches!(err, ExcelOpenError::WorkbookXmlMissing));

    let _ = fs::remove_file(&path);
}

#[test]
fn missing_worksheet_xml_returns_worksheetxmlmissing() {
    let path = temp_xlsx_path("missing_worksheet_xml");
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#;

    let workbook_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;

    let relationships = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
                Target="worksheets/sheet1.xml"/>
</Relationships>"#;

    write_zip(
        &[
            ("[Content_Types].xml", content_types),
            ("xl/workbook.xml", workbook_xml),
            ("xl/_rels/workbook.xml.rels", relationships),
        ],
        &path,
    );

    let err = open_workbook(&path).expect_err("missing worksheet xml should error");
    match err {
        ExcelOpenError::WorksheetXmlMissing { sheet_name } => {
            assert_eq!(sheet_name, "Sheet1");
        }
        other => panic!("expected WorksheetXmlMissing, got {other:?}"),
    }

    let _ = fs::remove_file(&path);
}
```

---

### File: `core\tests\g10_row_block_alignment_grid_workbook_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn g10_row_block_insert_middle_emits_four_rowadded_and_no_noise() {
    let wb_a = open_workbook(fixture_path("row_block_insert_a.xlsx"))
        .expect("failed to open fixture: row_block_insert_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_block_insert_b.xlsx"))
        .expect("failed to open fixture: row_block_insert_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_added,
        vec![3, 4, 5, 6],
        "expected four RowAdded ops for the inserted block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "no rows should be removed for block insert"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned block insert should not emit CellEdited noise"
    );
}

#[test]
fn g10_row_block_delete_middle_emits_four_rowremoved_and_no_noise() {
    let wb_a = open_workbook(fixture_path("row_block_delete_a.xlsx"))
        .expect("failed to open fixture: row_block_delete_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_block_delete_b.xlsx"))
        .expect("failed to open fixture: row_block_delete_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let rows_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_removed,
        vec![3, 4, 5, 6],
        "expected four RowRemoved ops for the deleted block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "no rows should be added for block delete"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned block delete should not emit CellEdited noise"
    );
}
```

---

### File: `core\tests\g11_row_block_move_grid_workbook_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers, single_sheet_workbook};

#[test]
fn g11_row_block_move_emits_single_blockmovedrows() {
    let wb_a = open_workbook(fixture_path("row_block_move_a.xlsx"))
        .expect("failed to open fixture: row_block_move_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_block_move_b.xlsx"))
        .expect("failed to open fixture: row_block_move_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*src_start_row, 4);
            assert_eq!(*row_count, 4);
            assert_eq!(*dst_start_row, 12);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedRows op, got {:?}", other),
    }

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "pure move should not emit RowAdded"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "pure move should not emit RowRemoved"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "pure move should not emit CellEdited noise"
    );
}

#[test]
fn g11_repeated_rows_do_not_emit_blockmove() {
    let grid_a = grid_from_numbers(&[&[1, 10], &[1, 10], &[2, 20], &[2, 20]]);

    let grid_b = grid_from_numbers(&[&[2, 20], &[2, 20], &[1, 10], &[1, 10]]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "ambiguous repeated rows must not emit BlockMovedRows"
    );

    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "fallback path should emit positional CellEdited noise"
    );
}
```

---

### File: `core\tests\g12_column_block_move_grid_workbook_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers, single_sheet_workbook};

#[test]
fn g12_column_move_emits_single_blockmovedcolumns() {
    let wb_a = open_workbook(fixture_path("column_move_a.xlsx"))
        .expect("failed to open fixture: column_move_a.xlsx");
    let wb_b = open_workbook(fixture_path("column_move_b.xlsx"))
        .expect("failed to open fixture: column_move_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            assert_eq!(sheet, "Data");
            assert_eq!(*src_start_col, 2);
            assert_eq!(*col_count, 1);
            assert_eq!(*dst_start_col, 5);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedColumns op, got {:?}", other),
    }

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::ColumnAdded { .. })),
        "pure move should not emit ColumnAdded"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::ColumnRemoved { .. })),
        "pure move should not emit ColumnRemoved"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. })),
        "pure move should not emit row operations"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "pure move should not emit CellEdited noise"
    );
}

#[test]
fn g12_repeated_columns_do_not_emit_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[1, 1, 2, 2], &[10, 10, 20, 20]]);
    let grid_b = grid_from_numbers(&[&[2, 2, 1, 1], &[20, 20, 10, 10]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. })),
        "ambiguous repeated columns must not emit BlockMovedColumns"
    );

    assert!(
        report.ops.iter().any(|op| matches!(
            op,
            DiffOp::CellEdited { .. } | DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. }
        )),
        "fallback path should emit some other diff operation"
    );
}

#[test]
fn g12_multi_column_block_move_emits_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[
        &[10, 20, 30, 40, 50, 60],
        &[11, 21, 31, 41, 51, 61],
        &[12, 22, 32, 42, 52, 62],
    ]);

    let grid_b = grid_from_numbers(&[
        &[10, 40, 50, 20, 30, 60],
        &[11, 41, 51, 21, 31, 61],
        &[12, 42, 52, 22, 32, 62],
    ]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        1,
        "expected a single diff op for multi-column move"
    );

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            assert_eq!(sheet, "Data");
            assert_eq!(*src_start_col, 3);
            assert_eq!(*col_count, 2, "should detect a 2-column block move");
            assert_eq!(*dst_start_col, 1);
            assert!(block_hash.is_none());
        }
        other => panic!("expected BlockMovedColumns op, got {:?}", other),
    }
}

#[test]
fn g12_two_independent_column_moves_do_not_emit_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[10, 20, 30, 40, 50, 60], &[11, 21, 31, 41, 51, 61]]);

    let grid_b = grid_from_numbers(&[&[20, 10, 30, 40, 60, 50], &[21, 11, 31, 41, 61, 51]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. })),
        "two independent column swaps must not emit BlockMovedColumns"
    );

    assert!(
        !report.ops.is_empty(),
        "fallback path should emit some diff operations"
    );
}

#[test]
fn g12_column_swap_emits_blockmovedcolumns() {
    let grid_a = grid_from_numbers(&[&[10, 20, 30, 40], &[11, 21, 31, 41]]);

    let grid_b = grid_from_numbers(&[&[20, 10, 30, 40], &[21, 11, 31, 41]]);

    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        1,
        "swap should produce single BlockMovedColumns op"
    );

    match &report.ops[0] {
        DiffOp::BlockMovedColumns {
            sheet,
            col_count,
            src_start_col,
            dst_start_col,
            ..
        } => {
            assert_eq!(sheet, "Data");
            assert_eq!(*col_count, 1, "swap is represented as moving one column");
            assert!(
                (*src_start_col == 0 && *dst_start_col == 1)
                    || (*src_start_col == 1 && *dst_start_col == 0),
                "swap should move column 0 or 1 past the other"
            );
        }
        other => panic!("expected BlockMovedColumns, got {:?}", other),
    }
}
```

---

### File: `core\tests\g12_rect_block_move_grid_workbook_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers, single_sheet_workbook};

#[test]
fn g12_rect_block_move_emits_single_blockmovedrect() {
    let wb_a = open_workbook(fixture_path("rect_block_move_a.xlsx"))
        .expect("failed to open fixture: rect_block_move_a.xlsx");
    let wb_b = open_workbook(fixture_path("rect_block_move_b.xlsx"))
        .expect("failed to open fixture: rect_block_move_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash: _,
        } => {
            assert_eq!(sheet, "Data");
            assert_eq!(*src_start_row, 2);
            assert_eq!(*src_row_count, 3);
            assert_eq!(*src_start_col, 1);
            assert_eq!(*src_col_count, 3);
            assert_eq!(*dst_start_row, 9);
            assert_eq!(*dst_start_col, 6);
        }
        other => panic!("expected BlockMovedRect op, got {:?}", other),
    }
}

#[test]
fn g12_rect_block_move_ambiguous_swap_does_not_emit_blockmovedrect() {
    let (grid_a, grid_b) = swap_two_blocks();
    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "ambiguous block swap must not emit BlockMovedRect"
    );
    assert!(
        !report.ops.is_empty(),
        "fallback path should emit some diff operations"
    );
}

#[test]
fn g12_rect_block_move_with_internal_edit_falls_back() {
    let (grid_a, grid_b) = move_with_edit();
    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "move with internal edit should not be treated as exact rectangular move"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "edited block should surface as cell edits or structural diffs"
    );
}

fn swap_two_blocks() -> (excel_diff::Grid, excel_diff::Grid) {
    let base: Vec<Vec<i32>> = (0..6)
        .map(|r| (0..6).map(|c| 100 * r + c).collect())
        .collect();
    let mut grid_a = base.clone();
    let mut grid_b = base.clone();

    let block_one = vec![vec![900, 901], vec![902, 903]];
    let block_two = vec![vec![700, 701], vec![702, 703]];

    place_block(&mut grid_a, 0, 0, &block_one);
    place_block(&mut grid_a, 3, 3, &block_two);

    // Swap the two distinct blocks in grid B.
    place_block(&mut grid_b, 0, 0, &block_two);
    place_block(&mut grid_b, 3, 3, &block_one);

    (grid_from_matrix(grid_a), grid_from_matrix(grid_b))
}

fn move_with_edit() -> (excel_diff::Grid, excel_diff::Grid) {
    let mut grid_a = base_background(10, 10);
    let mut grid_b = base_background(10, 10);

    let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

    place_block(&mut grid_a, 1, 1, &block);
    place_block(&mut grid_b, 6, 4, &block);
    grid_b[7][5] = 9_999; // edit inside the moved block

    (grid_from_matrix(grid_a), grid_from_matrix(grid_b))
}

fn base_background(rows: usize, cols: usize) -> Vec<Vec<i32>> {
    (0..rows)
        .map(|r| (0..cols).map(|c| (r as i32) * 1_000 + c as i32).collect())
        .collect()
}

fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
    for (r_offset, row_vals) in block.iter().enumerate() {
        for (c_offset, value) in row_vals.iter().enumerate() {
            let row = top + r_offset;
            let col = left + c_offset;
            if let Some(row_slice) = target.get_mut(row)
                && let Some(cell) = row_slice.get_mut(col)
            {
                *cell = *value;
            }
        }
    }
}

fn grid_from_matrix(matrix: Vec<Vec<i32>>) -> excel_diff::Grid {
    let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
    grid_from_numbers(&refs)
}
```

---

### File: `core\tests\g13_fuzzy_row_move_grid_workbook_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers, single_sheet_workbook};

#[test]
fn g13_fuzzy_row_move_emits_blockmovedrows_and_celledited() {
    let wb_a = open_workbook(fixture_path("grid_move_and_edit_a.xlsx"))
        .expect("failed to open fixture: grid_move_and_edit_a.xlsx");
    let wb_b = open_workbook(fixture_path("grid_move_and_edit_b.xlsx"))
        .expect("failed to open fixture: grid_move_and_edit_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let block_moves: Vec<(u32, u32, u32, Option<u64>)> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                block_hash,
                ..
            } => Some((*src_start_row, *row_count, *dst_start_row, *block_hash)),
            _ => None,
        })
        .collect();

    assert_eq!(block_moves.len(), 1, "expected a single BlockMovedRows op");
    let (src_start_row, row_count, dst_start_row, block_hash) = block_moves[0];
    assert_eq!(src_start_row, 4);
    assert_eq!(row_count, 4);
    assert_eq!(dst_start_row, 13);
    assert!(block_hash.is_none());

    let edited_rows: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.row),
            _ => None,
        })
        .collect();
    assert!(
        edited_rows
            .iter()
            .any(|r| *r >= dst_start_row && *r < dst_start_row + row_count),
        "expected a CellEdited inside the moved block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { row_idx, .. } if *row_idx >= dst_start_row && *row_idx < dst_start_row + row_count)),
        "moved rows must not be reported as added"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { row_idx, .. } if *row_idx >= src_start_row && *row_idx < src_start_row + row_count)),
        "moved rows must not be reported as removed"
    );
}

#[test]
fn g13_in_place_edits_do_not_emit_blockmovedrows() {
    let rows: Vec<Vec<i32>> = (1..=12)
        .map(|r| (1..=3).map(|c| r * 10 + c).collect())
        .collect();
    let rows_refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&rows_refs);

    let mut edited_rows = rows.clone();
    if let Some(cell) = edited_rows.get_mut(5).and_then(|row| row.get_mut(1)) {
        *cell += 7;
    }
    let edited_refs: Vec<&[i32]> = edited_rows.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&edited_refs);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "in-place edits must not be classified as BlockMovedRows"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "edits should still be surfaced as CellEdited"
    );
}

#[test]
fn g13_ambiguous_repeated_blocks_do_not_emit_blockmovedrows() {
    let mut rows_a: Vec<Vec<i32>> = vec![vec![1, 1]; 10];
    rows_a.push(vec![99, 99]);
    rows_a.push(vec![2, 2]);

    let mut rows_b = rows_a.clone();
    let moved = rows_b.remove(10);
    rows_b.insert(3, moved);

    let refs_a: Vec<&[i32]> = rows_a.iter().map(|r| r.as_slice()).collect();
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs_a);
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "ambiguous repeated patterns should not emit BlockMovedRows"
    );
    assert!(
        !report.ops.is_empty(),
        "fallback path should produce some diff noise"
    );
}
```

---

### File: `core\tests\g1_g2_grid_workbook_tests.rs`

```rust
use excel_diff::{CellValue, DiffOp, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn g1_equal_sheet_produces_empty_diff() {
    let wb_a = open_workbook(fixture_path("equal_sheet_a.xlsx"))
        .expect("failed to open fixture: equal_sheet_a.xlsx");
    let wb_b = open_workbook(fixture_path("equal_sheet_b.xlsx"))
        .expect("failed to open fixture: equal_sheet_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        report.ops.is_empty(),
        "identical 5x5 sheet should produce an empty diff"
    );
}

#[test]
fn g2_single_cell_literal_change_produces_one_celledited() {
    let wb_a = open_workbook(fixture_path("single_cell_value_a.xlsx"))
        .expect("failed to open fixture: single_cell_value_a.xlsx");
    let wb_b = open_workbook(fixture_path("single_cell_value_b.xlsx"))
        .expect("failed to open fixture: single_cell_value_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        1,
        "expected exactly one diff op for a single edited cell"
    );

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
            assert_eq!(from.formula, to.formula, "no formula changes expected");
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
        )),
        "single cell change should not produce row/column structure ops"
    );
}
```

---

### File: `core\tests\g5_g7_grid_workbook_tests.rs`

```rust
use excel_diff::{CellValue, DiffOp, diff_workbooks, open_workbook};
use std::collections::BTreeSet;

mod common;
use common::fixture_path;

#[test]
fn g5_multi_cell_edits_produces_only_celledited_ops() {
    let wb_a = open_workbook(fixture_path("multi_cell_edits_a.xlsx"))
        .expect("failed to open fixture: multi_cell_edits_a.xlsx");
    let wb_b = open_workbook(fixture_path("multi_cell_edits_b.xlsx"))
        .expect("failed to open fixture: multi_cell_edits_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let expected = vec![
        ("B2", CellValue::Number(1.0), CellValue::Number(42.0)),
        ("D5", CellValue::Number(2.0), CellValue::Number(99.0)),
        ("H7", CellValue::Number(3.0), CellValue::Number(3.5)),
        (
            "J10",
            CellValue::Text("x".into()),
            CellValue::Text("y".into()),
        ),
    ];

    assert_eq!(
        report.ops.len(),
        expected.len(),
        "expected one DiffOp per configured edit"
    );
    assert!(
        report
            .ops
            .iter()
            .all(|op| matches!(op, DiffOp::CellEdited { .. })),
        "multi-cell edits should produce only CellEdited ops"
    );

    for (addr, expected_from, expected_to) in expected {
        let (sheet, from, to) = report
            .ops
            .iter()
            .find_map(|op| match op {
                DiffOp::CellEdited {
                    sheet,
                    addr: a,
                    from,
                    to,
                } if a.to_a1() == addr => Some((sheet, from, to)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing CellEdited for {addr}"));

        assert_eq!(sheet, "Sheet1");
        assert_eq!(from.value, Some(expected_from));
        assert_eq!(to.value, Some(expected_to));
        assert_eq!(from.formula, to.formula, "no formula changes expected");
    }

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
        )),
        "multi-cell edits should not produce row/column structure ops"
    );
}

#[test]
fn g6_row_append_bottom_emits_two_rowadded_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("row_append_bottom_a.xlsx"))
        .expect("failed to open fixture: row_append_bottom_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_append_bottom_b.xlsx"))
        .expect("failed to open fixture: row_append_bottom_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two RowAdded ops for appended rows"
    );

    let rows_added: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [10u32, 11u32].into_iter().collect();
    assert_eq!(rows_added, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowRemoved { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "row append should not emit removals, column ops, or cell edits"
    );
}

#[test]
fn g6_row_delete_bottom_emits_two_rowremoved_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("row_delete_bottom_a.xlsx"))
        .expect("failed to open fixture: row_delete_bottom_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_delete_bottom_b.xlsx"))
        .expect("failed to open fixture: row_delete_bottom_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two RowRemoved ops for deleted rows"
    );

    let rows_removed: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [10u32, 11u32].into_iter().collect();
    assert_eq!(rows_removed, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::ColumnAdded { .. }
                | DiffOp::ColumnRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "row delete should not emit additions, column ops, or cell edits"
    );
}

#[test]
fn g7_col_append_right_emits_two_columnadded_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("col_append_right_a.xlsx"))
        .expect("failed to open fixture: col_append_right_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_append_right_b.xlsx"))
        .expect("failed to open fixture: col_append_right_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two ColumnAdded ops for appended columns"
    );

    let cols_added: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [4u32, 5u32].into_iter().collect();
    assert_eq!(cols_added, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnRemoved { .. }
                | DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "column append should not emit removals, row ops, or cell edits"
    );
}

#[test]
fn g7_col_delete_right_emits_two_columnremoved_and_no_celledited() {
    let wb_a = open_workbook(fixture_path("col_delete_right_a.xlsx"))
        .expect("failed to open fixture: col_delete_right_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_delete_right_b.xlsx"))
        .expect("failed to open fixture: col_delete_right_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(
        report.ops.len(),
        2,
        "expected exactly two ColumnRemoved ops for deleted columns"
    );

    let cols_removed: BTreeSet<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    let expected: BTreeSet<u32> = [4u32, 5u32].into_iter().collect();
    assert_eq!(cols_removed, expected);

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnAdded { .. }
                | DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::CellEdited { .. }
        )),
        "column delete should not emit additions, row ops, or cell edits"
    );
}
```

---

### File: `core\tests\g8_row_alignment_grid_workbook_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn single_row_insert_middle_produces_one_row_added() {
    let wb_a = open_workbook(fixture_path("row_insert_middle_a.xlsx"))
        .expect("failed to open fixture: row_insert_middle_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_insert_middle_b.xlsx"))
        .expect("failed to open fixture: row_insert_middle_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(rows_added, vec![5], "expected single RowAdded at index 5");

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "no rows should be removed for middle insert"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned insert should not emit CellEdited noise"
    );
}

#[test]
fn single_row_delete_middle_produces_one_row_removed() {
    let wb_a = open_workbook(fixture_path("row_delete_middle_a.xlsx"))
        .expect("failed to open fixture: row_delete_middle_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_delete_middle_b.xlsx"))
        .expect("failed to open fixture: row_delete_middle_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let rows_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert!(row_signature.is_none());
                Some(*row_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        rows_removed,
        vec![5],
        "expected single RowRemoved at index 5"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "no rows should be added for middle delete"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned delete should not emit CellEdited noise"
    );
}

#[test]
fn alignment_bails_out_when_additional_edits_present() {
    let wb_a = open_workbook(fixture_path("row_insert_with_edit_a.xlsx"))
        .expect("failed to open fixture: row_insert_with_edit_a.xlsx");
    let wb_b = open_workbook(fixture_path("row_insert_with_edit_b.xlsx"))
        .expect("failed to open fixture: row_insert_with_edit_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let rows_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded { row_idx, .. } => Some(*row_idx),
            _ => None,
        })
        .collect();

    assert!(
        rows_added.contains(&10),
        "fallback positional diff should add the tail row"
    );
    assert!(
        !rows_added.contains(&5),
        "mid-sheet RowAdded at 5 would indicate the alignment path was taken"
    );

    let edited_rows: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.row),
            _ => None,
        })
        .collect();

    assert!(
        !edited_rows.is_empty(),
        "fallback positional diff should surface cell edits after the inserted row"
    );
    assert!(
        edited_rows.iter().any(|row| *row >= 5),
        "cell edits should include rows at or below the insertion point"
    );
}
```

---

### File: `core\tests\g9_column_alignment_grid_workbook_tests.rs`

```rust
use excel_diff::{CellValue, DiffOp, Workbook, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn g9_col_insert_middle_emits_one_columnadded_and_no_noise() {
    let wb_a = open_workbook(fixture_path("col_insert_middle_a.xlsx"))
        .expect("failed to open fixture: col_insert_middle_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_insert_middle_b.xlsx"))
        .expect("failed to open fixture: col_insert_middle_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let cols_added: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Data");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        cols_added,
        vec![3],
        "expected single ColumnAdded at inserted position"
    );

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnRemoved { .. } | DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }
        )),
        "column insert should not emit row ops or ColumnRemoved"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned insert should not emit CellEdited noise"
    );
}

#[test]
fn g9_col_delete_middle_emits_one_columnremoved_and_no_noise() {
    let wb_a = open_workbook(fixture_path("col_delete_middle_a.xlsx"))
        .expect("failed to open fixture: col_delete_middle_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_delete_middle_b.xlsx"))
        .expect("failed to open fixture: col_delete_middle_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    let cols_removed: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Data");
                assert!(col_signature.is_none());
                Some(*col_idx)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        cols_removed,
        vec![3],
        "expected single ColumnRemoved at deleted position"
    );

    assert!(
        !report.ops.iter().any(|op| matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }
        )),
        "column delete should not emit ColumnAdded or row ops"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "aligned delete should not emit CellEdited noise"
    );
}

#[test]
fn g9_alignment_bails_out_when_additional_edits_present() {
    let wb_a = open_workbook(fixture_path("col_insert_with_edit_a.xlsx"))
        .expect("failed to open fixture: col_insert_with_edit_a.xlsx");
    let wb_b = open_workbook(fixture_path("col_insert_with_edit_b.xlsx"))
        .expect("failed to open fixture: col_insert_with_edit_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);
    let inserted_idx = find_header_col(&wb_b, "Inserted");

    let has_middle_column_add = report.ops.iter().any(|op| match op {
        DiffOp::ColumnAdded { col_idx, .. } => *col_idx == inserted_idx,
        _ => false,
    });
    assert!(
        !has_middle_column_add,
        "alignment should bail out; no ColumnAdded at the inserted index"
    );

    let edited_cols: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.col),
            _ => None,
        })
        .collect();

    assert!(
        !edited_cols.is_empty(),
        "fallback positional diff should emit CellEdited ops"
    );
    assert!(
        edited_cols.iter().any(|col| *col > inserted_idx),
        "CellEdited ops should appear in columns to the right of the insertion"
    );
}

fn find_header_col(workbook: &Workbook, header: &str) -> u32 {
    workbook
        .sheets
        .iter()
        .flat_map(|sheet| sheet.grid.cells.iter())
        .find_map(|((row, col), cell)| match &cell.value {
            Some(CellValue::Text(text)) if *row == 0 && text == header => Some(*col),
            _ => None,
        })
        .expect("header column should exist in fixture")
}
```

---

### File: `core\tests\grid_view_hashstats_tests.rs`

```rust
use excel_diff::{ColHash, ColMeta, HashStats, RowHash, RowMeta};

fn row_meta(row_idx: u32, hash: RowHash) -> RowMeta {
    RowMeta {
        row_idx,
        hash,
        non_blank_count: 0,
        first_non_blank_col: 0,
        is_low_info: false,
    }
}

fn col_meta(col_idx: u32, hash: ColHash) -> ColMeta {
    ColMeta {
        col_idx,
        hash,
        non_blank_count: 0,
        first_non_blank_row: 0,
    }
}

#[test]
fn hashstats_counts_and_positions_basic() {
    let h1: RowHash = 1;
    let h2: RowHash = 2;
    let h3: RowHash = 3;
    let h4: RowHash = 4;

    let rows_a = vec![
        row_meta(0, h1),
        row_meta(1, h2),
        row_meta(2, h2),
        row_meta(3, h3),
    ];
    let rows_b = vec![row_meta(0, h2), row_meta(1, h3), row_meta(2, h4)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);

    assert_eq!(stats.freq_a.get(&h1).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h1).copied().unwrap_or(0), 0);

    assert_eq!(stats.freq_a.get(&h2).copied().unwrap_or(0), 2);
    assert_eq!(stats.freq_b.get(&h2).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_a.get(&h3).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h3).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_a.get(&h4).copied().unwrap_or(0), 0);
    assert_eq!(stats.freq_b.get(&h4).copied().unwrap_or(0), 1);

    assert_eq!(
        stats.hash_to_positions_b.get(&h2).cloned().unwrap(),
        vec![0]
    );
    assert_eq!(
        stats.hash_to_positions_b.get(&h3).cloned().unwrap(),
        vec![1]
    );
    assert_eq!(
        stats.hash_to_positions_b.get(&h4).cloned().unwrap(),
        vec![2]
    );

    let threshold = 1;
    assert!(stats.is_unique(h3));
    assert!(stats.is_common(h2, threshold));
    assert!(!stats.is_rare(h3, threshold));
    assert!(stats.appears_in_both(h3));
    assert!(!stats.appears_in_both(h1));
    assert!(!stats.appears_in_both(h4));
}

#[test]
fn hashstats_rare_but_not_common_boundary() {
    let h: RowHash = 42;
    let rows_a = vec![row_meta(0, h), row_meta(1, h)];
    let rows_b = vec![row_meta(0, h)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);
    let threshold = 2;

    assert!(stats.is_rare(h, threshold));
    assert!(!stats.is_common(h, threshold));
    assert!(stats.appears_in_both(h));
    assert!(!stats.is_unique(h));
}

#[test]
fn hashstats_equal_to_threshold_behavior() {
    let h: RowHash = 99;
    let rows_a = vec![row_meta(0, h), row_meta(1, h), row_meta(2, h)];
    let rows_b = vec![row_meta(0, h), row_meta(1, h), row_meta(2, h)];

    let stats = HashStats::from_row_meta(&rows_a, &rows_b);
    let threshold = 3;

    assert!(stats.is_rare(h, threshold));
    assert!(!stats.is_common(h, threshold));
    assert!(stats.appears_in_both(h));
    assert!(!stats.is_unique(h));
}

#[test]
fn hashstats_empty_inputs() {
    let stats = HashStats::from_row_meta(&[], &[]);
    let dummy_hash: RowHash = 123;

    assert!(stats.freq_a.is_empty());
    assert!(stats.freq_b.is_empty());
    assert!(stats.hash_to_positions_b.is_empty());

    assert!(!stats.is_unique(dummy_hash));
    assert!(!stats.is_rare(dummy_hash, 1));
    assert!(!stats.is_common(dummy_hash, 0));
    assert!(!stats.appears_in_both(dummy_hash));
}

#[test]
fn hashstats_from_col_meta_tracks_positions() {
    let h1: ColHash = 10;
    let h2: ColHash = 20;
    let h3: ColHash = 30;

    let cols_a = vec![col_meta(0, h1), col_meta(1, h2), col_meta(2, h2)];
    let cols_b = vec![col_meta(0, h2), col_meta(1, h3)];

    let stats = HashStats::from_col_meta(&cols_a, &cols_b);

    assert_eq!(stats.freq_a.get(&h1).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_b.get(&h1).copied().unwrap_or(0), 0);

    assert_eq!(stats.freq_a.get(&h2).copied().unwrap_or(0), 2);
    assert_eq!(stats.freq_b.get(&h2).copied().unwrap_or(0), 1);

    assert_eq!(stats.freq_b.get(&h3).copied().unwrap_or(0), 1);
    assert_eq!(stats.freq_a.get(&h3).copied().unwrap_or(0), 0);

    assert_eq!(
        stats
            .hash_to_positions_b
            .get(&h2)
            .cloned()
            .unwrap_or_default(),
        vec![0]
    );
    assert_eq!(
        stats
            .hash_to_positions_b
            .get(&h3)
            .cloned()
            .unwrap_or_default(),
        vec![1]
    );
}
```

---

### File: `core\tests\grid_view_tests.rs`

```rust
use excel_diff::{Cell, CellAddress, CellValue, Grid, GridView};

mod common;
use common::grid_from_numbers;

fn make_cell(row: u32, col: u32, value: Option<CellValue>, formula: Option<&str>) -> Cell {
    Cell {
        row,
        col,
        address: CellAddress::from_indices(row, col),
        value,
        formula: formula.map(|s| s.to_string()),
    }
}

#[test]
fn gridview_dense_3x3_layout_and_metadata() {
    let grid = grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);

    let view = GridView::from_grid(&grid);

    assert_eq!(view.rows.len(), 3);
    assert_eq!(view.row_meta.len(), 3);
    assert_eq!(view.col_meta.len(), 3);

    for (row_idx, row_view) in view.rows.iter().enumerate() {
        assert_eq!(row_view.cells.len(), 3);
        for (col_idx, (col, cell)) in row_view.cells.iter().enumerate() {
            assert_eq!(*col as usize, col_idx);
            assert_eq!(cell.row as usize, row_idx);
            assert_eq!(cell.col as usize, col_idx);
        }

        let meta = &view.row_meta[row_idx];
        assert_eq!(meta.non_blank_count, 3);
        assert_eq!(meta.first_non_blank_col, 0);
        assert!(!meta.is_low_info);
    }

    for (col_idx, col_meta) in view.col_meta.iter().enumerate() {
        assert_eq!(col_meta.non_blank_count, 3);
        assert_eq!(col_meta.first_non_blank_row, 0);
        assert_eq!(col_meta.col_idx as usize, col_idx);
    }
}

#[test]
fn gridview_sparse_rows_low_info_classification() {
    let mut grid = Grid::new(4, 4);
    grid.insert(make_cell(
        0,
        0,
        Some(CellValue::Text("Header".into())),
        None,
    ));
    grid.insert(make_cell(2, 2, Some(CellValue::Number(10.0)), None));
    grid.insert(make_cell(3, 1, Some(CellValue::Text("   ".into())), None));

    let view = GridView::from_grid(&grid);

    assert_eq!(view.row_meta[0].non_blank_count, 1);
    assert!(!view.row_meta[0].is_low_info);
    assert_eq!(view.row_meta[0].first_non_blank_col, 0);

    assert_eq!(view.row_meta[1].non_blank_count, 0);
    assert!(view.row_meta[1].is_low_info);
    assert_eq!(view.row_meta[1].first_non_blank_col, 0);

    assert_eq!(view.row_meta[2].non_blank_count, 1);
    assert!(!view.row_meta[2].is_low_info);
    assert_eq!(view.row_meta[2].first_non_blank_col, 2);

    assert_eq!(view.row_meta[3].non_blank_count, 1);
    assert!(view.row_meta[3].is_low_info);
    assert_eq!(view.row_meta[3].first_non_blank_col, 1);
}

#[test]
fn gridview_formula_only_row_is_not_low_info() {
    let mut grid = Grid::new(2, 2);
    grid.insert(make_cell(0, 0, None, Some("=A1+1")));

    let view = GridView::from_grid(&grid);

    assert_eq!(view.row_meta[0].non_blank_count, 1);
    assert!(!view.row_meta[0].is_low_info);
}

#[test]
fn gridview_column_metadata_matches_signatures() {
    let mut grid = Grid::new(4, 4);
    grid.insert(make_cell(
        0,
        1,
        Some(CellValue::Text("a".into())),
        Some("=B1"),
    ));
    grid.insert(make_cell(1, 3, Some(CellValue::Number(2.0)), Some("=1+1")));
    grid.insert(make_cell(2, 0, Some(CellValue::Bool(true)), None));
    grid.insert(make_cell(2, 2, Some(CellValue::Text("mid".into())), None));
    grid.insert(make_cell(3, 0, None, Some("=A1")));

    grid.compute_all_signatures();
    let row_signatures = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should be computed")
        .clone();
    let col_signatures = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should be computed")
        .clone();

    let view = GridView::from_grid(&grid);

    for (idx, meta) in view.col_meta.iter().enumerate() {
        assert_eq!(meta.hash, col_signatures[idx].hash);
    }

    for (idx, meta) in view.row_meta.iter().enumerate() {
        assert_eq!(meta.hash, row_signatures[idx].hash);
    }

    assert_eq!(view.col_meta[0].non_blank_count, 2);
    assert_eq!(view.col_meta[0].first_non_blank_row, 2);
    assert_eq!(view.col_meta[1].non_blank_count, 1);
    assert_eq!(view.col_meta[1].first_non_blank_row, 0);
    assert_eq!(view.col_meta[2].non_blank_count, 1);
    assert_eq!(view.col_meta[2].first_non_blank_row, 2);
    assert_eq!(view.col_meta[3].non_blank_count, 1);
    assert_eq!(view.col_meta[3].first_non_blank_row, 1);
}

#[test]
fn gridview_empty_grid_is_stable() {
    let grid = Grid::new(0, 0);

    let view = GridView::from_grid(&grid);

    assert!(view.rows.is_empty());
    assert!(view.row_meta.is_empty());
    assert!(view.col_meta.is_empty());
}

#[test]
fn gridview_large_sparse_grid_constructs_without_panic() {
    let nrows = 10_000;
    let ncols = 10;
    let mut grid = Grid::new(nrows, ncols);

    for r in (0..nrows).step_by(100) {
        let col = (r / 100) % ncols;
        grid.insert(make_cell(
            r,
            col,
            Some(CellValue::Number((r / 100) as f64)),
            None,
        ));
    }

    let view = GridView::from_grid(&grid);

    assert_eq!(view.rows.len(), nrows as usize);
    assert_eq!(view.col_meta.len(), ncols as usize);

    assert_eq!(view.row_meta[1].non_blank_count, 0);
    assert_eq!(view.row_meta[100].non_blank_count, 1);
    assert_eq!(view.row_meta[100].first_non_blank_col, 1);

    assert!(
        view.col_meta
            .iter()
            .any(|meta| meta.non_blank_count > 0 && meta.first_non_blank_row == 0)
    );
}
```

---

### File: `core\tests\integration_test.rs`

```rust
use std::path::PathBuf;

fn get_fixture_path(filename: &str) -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up one level from 'core', then into 'fixtures/generated'
    d.push("../fixtures/generated");
    d.push(filename);
    d
}

#[test]
fn test_locate_fixture() {
    let path = get_fixture_path("minimal.xlsx");
    // This test confirms that the Rust code can locate the Python-generated fixtures
    // using the relative path strategy from the monorepo root.
    assert!(
        path.exists(),
        "Fixture minimal.xlsx should exist at {:?}",
        path
    );
}
```

---

### File: `core\tests\m4_package_parts_tests.rs`

```rust
use std::io::{Cursor, Write};

use excel_diff::{DataMashupError, open_data_mashup, parse_package_parts, parse_section_members};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

mod common;
use common::fixture_path;

const MIN_PACKAGE_XML: &str = "<Package></Package>";
const MIN_SECTION: &str = "section Section1;\nshared Foo = 1;";
const BOM_SECTION: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

#[test]
fn package_parts_contains_expected_entries() {
    let path = fixture_path("one_query.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(
        parts.main_section.source.contains("section Section1;"),
        "main Section1.m should be present"
    );
    assert!(
        parts.main_section.source.contains("shared"),
        "at least one shared query should be present"
    );
    assert!(
        parts.embedded_contents.is_empty(),
        "one_query.xlsx should not contain embedded contents"
    );
}

#[test]
fn embedded_content_detection() {
    let path = fixture_path("multi_query_with_embedded.xlsx");
    let raw = open_data_mashup(&path)
        .expect("fixture should open")
        .expect("mashup should be present");

    let parts = parse_package_parts(&raw.package_parts).expect("PackageParts should parse");

    assert!(
        !parts.embedded_contents.is_empty(),
        "multi_query_with_embedded.xlsx should expose at least one embedded content"
    );

    for embedded in &parts.embedded_contents {
        assert!(
            embedded.section.source.contains("section Section1"),
            "embedded Section1.m should be present for {}",
            embedded.name
        );
        assert!(
            embedded.section.source.contains("shared"),
            "embedded Section1.m should contain at least one shared member for {}",
            embedded.name
        );
    }
}

#[test]
fn parse_package_parts_rejects_non_zip() {
    let bogus = b"this is not a zip file";
    let err = parse_package_parts(bogus).expect_err("non-zip bytes should fail");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_config_package_xml_errors() {
    let bytes = build_zip(vec![(
        "Formulas/Section1.m",
        MIN_SECTION.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Config/Package.xml should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn missing_section1_errors() {
    let bytes = build_zip(vec![(
        "Config/Package.xml",
        MIN_PACKAGE_XML.as_bytes().to_vec(),
    )]);
    let err = parse_package_parts(&bytes)
        .expect_err("missing Formulas/Section1.m should be framing invalid");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_package_xml_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", vec![0xFF, 0xFF, 0xFF]),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
    ]);
    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Package.xml should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn invalid_utf8_in_section1_errors() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", vec![0xFF, 0xFF]),
    ]);

    let err = parse_package_parts(&bytes).expect_err("invalid UTF-8 in Section1.m should error");
    assert!(matches!(err, DataMashupError::FramingInvalid));
}

#[test]
fn embedded_content_invalid_zip_is_skipped() {
    let bytes =
        build_minimal_package_parts_with(vec![("Content/bogus.package", b"not a zip".to_vec())]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_missing_section1_is_skipped() {
    let nested = build_zip(vec![("Config/Formulas.xml", b"<Formulas/>".to_vec())]);
    let bytes = build_minimal_package_parts_with(vec![("Content/no_section1.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_invalid_utf8_is_skipped() {
    let nested = build_zip(vec![("Formulas/Section1.m", vec![0xFF, 0xFF])]);
    let bytes = build_minimal_package_parts_with(vec![("Content/bad_utf8.package", nested)]);
    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(parts.embedded_contents.is_empty());
}

#[test]
fn embedded_content_partial_failure_retains_valid_entries() {
    let good_nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![
        ("Content/good.package", good_nested),
        ("Content/bad.package", b"not a zip".to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    let embedded = &parts.embedded_contents[0];
    assert_eq!(embedded.name, "Content/good.package");
    assert!(embedded.section.source.contains("section Section1;"));
    assert!(embedded.section.source.contains("shared"));
}

#[test]
fn leading_slash_paths_are_accepted() {
    let embedded =
        build_embedded_section_zip("section Section1;\nshared Bar = 2;".as_bytes().to_vec());
    let bytes = build_zip(vec![
        (
            "/Config/Package.xml",
            br#"<Package from="leading"/>"#.to_vec(),
        ),
        ("/Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("/Content/abcd.package", embedded),
        (
            "Config/Package.xml",
            br#"<Package from="canonical"/>"#.to_vec(),
        ),
    ]);

    let parts = parse_package_parts(&bytes).expect("leading slash entries should parse");
    assert!(
        parts.package_xml.raw_xml.contains(r#"from="leading""#),
        "first encountered Package.xml should win"
    );
    assert!(parts.main_section.source.contains("shared Foo = 1;"));
    assert_eq!(parts.embedded_contents.len(), 1);
    assert!(
        parts.embedded_contents[0]
            .section
            .source
            .contains("shared Bar = 2;")
    );
}

#[test]
fn embedded_content_name_is_canonicalized() {
    let nested = build_embedded_section_zip(MIN_SECTION.as_bytes().to_vec());
    let bytes = build_minimal_package_parts_with(vec![("/Content/efgh.package", nested)]);

    let parts =
        parse_package_parts(&bytes).expect("embedded content with leading slash should parse");
    assert_eq!(parts.embedded_contents.len(), 1);
    assert_eq!(parts.embedded_contents[0].name, "Content/efgh.package");
}

#[test]
fn empty_content_directory_is_ignored() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/", Vec::new()),
    ]);

    let parts = parse_package_parts(&bytes).expect("package with empty Content/ directory parses");
    assert!(!parts.package_xml.raw_xml.is_empty());
    assert!(!parts.main_section.source.is_empty());
    assert!(
        parts.embedded_contents.is_empty(),
        "bare Content/ directory should not produce embedded contents"
    );
}

#[test]
fn parse_package_parts_never_panics_on_random_bytes() {
    for seed in 0u64..64 {
        let len = (seed as usize * 13 % 256) + (seed as usize % 7);
        let bytes = random_bytes(seed, len);
        let _ = parse_package_parts(&bytes);
    }
}

#[test]
fn package_parts_section1_with_bom_parses_via_parse_section_members() {
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", BOM_SECTION.as_bytes().to_vec()),
    ]);

    let parts = parse_package_parts(&bytes).expect("BOM-prefixed Section1.m should parse");
    assert!(
        !parts.main_section.source.starts_with('\u{FEFF}'),
        "PackageParts should strip a single leading BOM from Section1.m"
    );
    let members = parse_section_members(&parts.main_section.source)
        .expect("parse_section_members should accept BOM-prefixed Section1");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
}

#[test]
fn embedded_content_section1_with_bom_parses_via_parse_section_members() {
    let embedded = build_embedded_section_zip(BOM_SECTION.as_bytes().to_vec());
    let bytes = build_zip(vec![
        ("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()),
        ("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()),
        ("Content/bom_embedded.package", embedded),
    ]);

    let parts = parse_package_parts(&bytes).expect("outer package should parse");
    assert!(
        !parts.embedded_contents.is_empty(),
        "embedded package should be detected"
    );

    let embedded = parts
        .embedded_contents
        .iter()
        .find(|entry| entry.name == "Content/bom_embedded.package")
        .expect("expected embedded package to round-trip name");

    assert!(
        !embedded.section.source.starts_with('\u{FEFF}'),
        "embedded Section1.m should strip leading BOM"
    );

    let members = parse_section_members(&embedded.section.source)
        .expect("parse_section_members should accept embedded BOM Section1");
    assert!(
        !members.is_empty(),
        "embedded Section1.m should contain members"
    );
    assert!(
        members.iter().any(|member| {
            member.section_name == "Section1"
                && member.member_name == "Foo"
                && member.expression_m == "1"
        }),
        "embedded Section1.m should parse shared Foo = 1"
    );
}

fn build_minimal_package_parts_with(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut all_entries = Vec::with_capacity(entries.len() + 2);
    all_entries.push(("Config/Package.xml", MIN_PACKAGE_XML.as_bytes().to_vec()));
    all_entries.push(("Formulas/Section1.m", MIN_SECTION.as_bytes().to_vec()));
    all_entries.extend(entries);
    build_zip(all_entries)
}

fn build_embedded_section_zip(section_bytes: Vec<u8>) -> Vec<u8> {
    build_zip(vec![("Formulas/Section1.m", section_bytes)])
}

fn build_zip(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, bytes) in entries {
        if name.ends_with('/') {
            writer
                .add_directory(name, options)
                .expect("start zip directory");
        } else {
            writer.start_file(name, options).expect("start zip entry");
            writer.write_all(&bytes).expect("write zip entry");
        }
    }

    writer.finish().expect("finish zip").into_inner()
}

fn random_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(len);
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    for _ in 0..len {
        state = state
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        bytes.push((state >> 32) as u8);
    }
    bytes
}
```

---

### File: `core\tests\m4_permissions_metadata_tests.rs`

```rust
use excel_diff::{
    DataMashupError, Permissions, RawDataMashup, build_data_mashup, build_queries,
    datamashup::parse_metadata, open_data_mashup, parse_package_parts, parse_section_members,
};

mod common;
use common::fixture_path;

fn load_datamashup(path: &str) -> excel_diff::DataMashup {
    let raw = open_data_mashup(fixture_path(path))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

#[test]
fn permissions_parsed_flags_default_vs_firewall_off() {
    let defaults = load_datamashup("permissions_defaults.xlsx");
    let firewall_off = load_datamashup("permissions_firewall_off.xlsx");

    assert_eq!(defaults.version, 0);
    assert_eq!(firewall_off.version, 0);

    assert!(defaults.permissions.firewall_enabled);
    assert!(!defaults.permissions.can_evaluate_future_packages);
    assert!(!firewall_off.permissions.firewall_enabled);
    assert_eq!(
        defaults.permissions.workbook_group_type,
        firewall_off.permissions.workbook_group_type
    );
}

#[test]
fn permissions_missing_or_malformed_yields_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut missing = base_raw.clone();
    missing.permissions = Vec::new();
    missing.permission_bindings = Vec::new();
    let dm = build_data_mashup(&missing).expect("missing permissions should default");
    assert_eq!(dm.permissions, Permissions::default());

    let mut malformed = base_raw.clone();
    malformed.permissions = b"<not-xml".to_vec();
    let dm = build_data_mashup(&malformed).expect("malformed permissions should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn permissions_invalid_entities_yield_defaults() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let invalid_permissions = br#"
        <Permissions>
            <CanEvaluateFuturePackages>&bad;</CanEvaluateFuturePackages>
            <FirewallEnabled>true</FirewallEnabled>
        </Permissions>
    "#;
    let mut raw = base_raw.clone();
    raw.permissions = invalid_permissions.to_vec();

    let dm = build_data_mashup(&raw).expect("invalid permissions entities should default");
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn metadata_empty_bytes_returns_empty_struct() {
    let metadata = parse_metadata(&[]).expect("empty metadata should parse");
    assert!(metadata.formulas.is_empty());
}

#[test]
fn metadata_invalid_header_too_short_errors() {
    let err = parse_metadata(&[0x01]).expect_err("short metadata should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata XML not found"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_length_prefix_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&100u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 10]);

    let err = parse_metadata(&bytes).expect_err("invalid length prefix should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata length prefix invalid"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_invalid_utf8_errors() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&[0xFF, 0xFF]);

    let err = parse_metadata(&bytes).expect_err("invalid utf-8 should error");
    match err {
        DataMashupError::XmlError(msg) => {
            assert!(msg.contains("metadata is not valid UTF-8"));
        }
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_malformed_xml_errors() {
    let xml = b"<LocalPackageMetadataFile><foo";
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    bytes.extend_from_slice(xml);

    let err = parse_metadata(&bytes).expect_err("malformed xml should error");
    match err {
        DataMashupError::XmlError(_) => {}
        other => panic!("expected XmlError, got {other:?}"),
    }
}

#[test]
fn metadata_formulas_match_section_members() {
    let raw = open_data_mashup(fixture_path("metadata_simple.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    let package = parse_package_parts(&raw.package_parts).expect("package parts should parse");
    let metadata = parse_metadata(&raw.metadata).expect("metadata should parse");
    let members =
        parse_section_members(&package.main_section.source).expect("section members should parse");

    let section1_formulas: Vec<_> = metadata
        .formulas
        .iter()
        .filter(|m| m.section_name == "Section1" && !m.is_connection_only)
        .collect();

    assert_eq!(section1_formulas.len(), members.len());
    for meta in section1_formulas {
        assert!(!meta.formula_name.is_empty());
    }
}

#[test]
fn metadata_load_destinations_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let load_to_sheet = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToSheet")
        .expect("LoadToSheet metadata missing");
    assert!(load_to_sheet.load_to_sheet);
    assert!(!load_to_sheet.load_to_model);
    assert!(!load_to_sheet.is_connection_only);

    let load_to_model = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/LoadToModel")
        .expect("LoadToModel metadata missing");
    assert!(!load_to_model.load_to_sheet);
    assert!(load_to_model.load_to_model);
    assert!(!load_to_model.is_connection_only);
}

#[test]
fn metadata_groups_basic_hierarchy() {
    let dm = load_datamashup("metadata_query_groups.xlsx");
    let grouped = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/GroupedFoo")
        .expect("GroupedFoo metadata missing");
    assert_eq!(grouped.group_path.as_deref(), Some("Inputs/DimTables"));

    let root = dm
        .metadata
        .formulas
        .iter()
        .find(|m| m.item_path == "Section1/RootQuery")
        .expect("RootQuery metadata missing");
    assert!(root.group_path.is_none());
}

#[test]
fn metadata_hidden_queries_connection_only() {
    let dm = load_datamashup("metadata_hidden_queries.xlsx");
    let has_connection_only = dm
        .metadata
        .formulas
        .iter()
        .any(|m| !m.load_to_sheet && !m.load_to_model && m.is_connection_only);
    assert!(has_connection_only);
}

#[test]
fn metadata_itempath_decodes_percent_encoded_utf8() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%C3%A9</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar\u{00e9}");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar\u{00e9}");
    assert!(item.load_to_sheet);
    assert!(!item.is_connection_only);
}

#[test]
fn metadata_itempath_decodes_space_and_slash() {
    let xml = r#"
        <LocalPackageMetadataFile>
            <Formulas>
                <Item>
                    <ItemType>Formula</ItemType>
                    <ItemPath>Section1/Foo%20Bar%2FInner</ItemPath>
                    <Entry Type="FillEnabled" Value="l1" />
                </Item>
            </Formulas>
        </LocalPackageMetadataFile>
    "#;

    let metadata = parse_metadata(xml.as_bytes()).expect("metadata should parse");
    assert_eq!(metadata.formulas.len(), 1);
    let item = &metadata.formulas[0];
    assert_eq!(item.item_path, "Section1/Foo Bar/Inner");
    assert_eq!(item.section_name, "Section1");
    assert_eq!(item.formula_name, "Foo Bar/Inner");
}

#[test]
fn permission_bindings_present_flag() {
    let dm = load_datamashup("permissions_defaults.xlsx");
    assert!(!dm.permission_bindings_raw.is_empty());
}

#[test]
fn permission_bindings_missing_ok() {
    let base_raw = open_data_mashup(fixture_path("one_query.xlsx"))
        .expect("fixture should load")
        .expect("DataMashup should be present");

    let mut synthetic = RawDataMashup {
        permission_bindings: Vec::new(),
        ..base_raw.clone()
    };
    synthetic.permissions = Vec::new();
    synthetic.metadata = Vec::new();

    let dm = build_data_mashup(&synthetic).expect("empty bindings should build");
    assert!(dm.permission_bindings_raw.is_empty());
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn build_queries_is_compatible_with_metadata_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");
    assert!(!queries.is_empty());
}
```

---

### File: `core\tests\m5_query_domain_tests.rs`

```rust
use std::collections::HashSet;

use excel_diff::{build_data_mashup, build_queries, open_data_mashup, parse_section_members};

mod common;
use common::fixture_path;

fn load_datamashup(path: &str) -> excel_diff::DataMashup {
    let raw = open_data_mashup(fixture_path(path))
        .expect("fixture should load")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

#[test]
fn metadata_join_simple() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 2);
    let names: HashSet<_> = queries.iter().map(|q| q.name.as_str()).collect();
    assert_eq!(
        names,
        HashSet::from(["Section1/LoadToSheet", "Section1/LoadToModel"])
    );

    let sheet = queries
        .iter()
        .find(|q| q.section_member == "LoadToSheet")
        .expect("LoadToSheet query missing");
    assert!(sheet.metadata.load_to_sheet);
    assert!(!sheet.metadata.load_to_model);

    let model = queries
        .iter()
        .find(|q| q.section_member == "LoadToModel")
        .expect("LoadToModel query missing");
    assert!(!model.metadata.load_to_sheet);
    assert!(model.metadata.load_to_model);
}

#[test]
fn metadata_join_url_encoding() {
    let dm = load_datamashup("metadata_url_encoding.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    let q = &queries[0];
    assert_eq!(q.name, "Section1/Query with space & #");
    assert_eq!(q.section_member, "Query with space & #");
    assert!(q.metadata.load_to_sheet || q.metadata.load_to_model);
}

#[test]
fn member_without_metadata_is_preserved() {
    let dm = load_datamashup("metadata_missing_entry.xlsx");
    assert!(dm.metadata.formulas.is_empty());
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    let q = &queries[0];
    assert_eq!(q.name, "Section1/MissingMetadata");
    assert_eq!(q.section_member, "MissingMetadata");
    assert_eq!(q.metadata.item_path, "Section1/MissingMetadata");
    assert!(!q.metadata.load_to_sheet);
    assert!(!q.metadata.load_to_model);
    assert!(q.metadata.is_connection_only);
    assert_eq!(q.metadata.group_path, None);
}

#[test]
fn query_names_unique() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    let mut seen = HashSet::new();
    for q in &queries {
        assert!(seen.insert(&q.name));
    }
}

#[test]
fn metadata_orphan_entries() {
    let dm = load_datamashup("metadata_orphan_entries.xlsx");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "Section1/Foo");
    assert!(
        dm.metadata
            .formulas
            .iter()
            .any(|m| m.item_path == "Section1/Nonexistent")
    );
}

#[test]
fn queries_preserve_section_member_order() {
    let dm = load_datamashup("metadata_simple.xlsx");
    let members = parse_section_members(&dm.package_parts.main_section.source)
        .expect("Section1 should parse");
    let queries = build_queries(&dm).expect("queries should build");

    assert_eq!(members.len(), queries.len());
    for (idx, (member, query)) in members.iter().zip(queries.iter()).enumerate() {
        assert_eq!(
            query.section_member, member.member_name,
            "query at position {} should match Section1 member order",
            idx
        );
    }
}
```

---

### File: `core\tests\m6_textual_m_diff_tests.rs`

```rust
use excel_diff::{
    DataMashup, QueryChangeKind, SectionParseError, build_data_mashup, diff_m_queries,
    open_data_mashup,
};

mod common;
use common::fixture_path;

fn load_datamashup(name: &str) -> DataMashup {
    let raw = open_data_mashup(fixture_path(name))
        .expect("fixture should open")
        .expect("DataMashup should be present");
    build_data_mashup(&raw).expect("DataMashup should build")
}

fn datamashup_with_section(lines: &[&str]) -> DataMashup {
    let mut dm = load_datamashup("one_query.xlsx");
    let body = lines.join("\n");
    dm.package_parts.main_section.source = format!("section Section1;\n\n{body}\n");
    dm
}

#[test]
fn basic_add_query_diff() {
    let dm_a = load_datamashup("m_add_query_a.xlsx");
    let dm_b = load_datamashup("m_add_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected exactly one diff for added query");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Bar");
    assert_eq!(diff.kind, QueryChangeKind::Added);
}

#[test]
fn basic_remove_query_diff() {
    let dm_a = load_datamashup("m_remove_query_a.xlsx");
    let dm_b = load_datamashup("m_remove_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "expected exactly one diff for removed query"
    );
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Bar");
    assert_eq!(diff.kind, QueryChangeKind::Removed);
}

#[test]
fn literal_change_produces_definitionchanged() {
    let dm_a = load_datamashup("m_change_literal_a.xlsx");
    let dm_b = load_datamashup("m_change_literal_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for changed literal");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn metadata_change_produces_metadataonly() {
    let dm_a = load_datamashup("m_metadata_only_change_a.xlsx");
    let dm_b = load_datamashup("m_metadata_only_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 1, "expected one diff for metadata-only change");
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::MetadataChangedOnly);
}

#[test]
fn definition_and_metadata_change_prefers_definitionchanged() {
    let dm_a = load_datamashup("m_def_and_metadata_change_a.xlsx");
    let dm_b = load_datamashup("m_def_and_metadata_change_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(
        diffs.len(),
        1,
        "expected one diff even when both definition and metadata change"
    );
    let diff = &diffs[0];
    assert_eq!(diff.name, "Section1/Foo");
    assert_eq!(diff.kind, QueryChangeKind::DefinitionChanged);
}

#[test]
fn identical_workbooks_produce_no_diffs() {
    let dm = load_datamashup("one_query.xlsx");

    let diffs = diff_m_queries(&dm, &dm).expect("diff should succeed");

    assert!(
        diffs.is_empty(),
        "identical DataMashup should produce no diffs"
    );
}

#[test]
fn rename_reports_add_and_remove() {
    let dm_a = load_datamashup("m_rename_query_a.xlsx");
    let dm_b = load_datamashup("m_rename_query_b.xlsx");

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 2, "expected add + remove for rename scenario");

    assert_eq!(diffs[0].name, "Section1/Bar");
    assert_eq!(diffs[0].kind, QueryChangeKind::Added);
    assert_eq!(diffs[1].name, "Section1/Foo");
    assert_eq!(diffs[1].kind, QueryChangeKind::Removed);
}

#[test]
fn multiple_diffs_are_sorted_by_name() {
    let dm_a = datamashup_with_section(&["shared Zeta = 1;", "shared Bravo = 1;"]);
    let dm_b = datamashup_with_section(&["shared Alpha = 1;", "shared Delta = 1;"]);

    let diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");

    assert_eq!(diffs.len(), 4, "expected four diffs across both sides");
    assert!(
        diffs.windows(2).all(|w| w[0].name <= w[1].name),
        "diffs should already be sorted by name"
    );
    let names: Vec<_> = diffs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Section1/Alpha",
            "Section1/Bravo",
            "Section1/Delta",
            "Section1/Zeta"
        ],
        "lexicographic ordering should be preserved without resorting"
    );
}

#[test]
fn invalid_section_syntax_propagates_error() {
    let dm_invalid = datamashup_with_section(&["shared Broken // missing '=' and ';'"]);

    let result = diff_m_queries(&dm_invalid, &dm_invalid);

    assert!(matches!(
        result,
        Err(SectionParseError::InvalidMemberSyntax)
    ));
}
```

---

### File: `core\tests\m_section_splitting_tests.rs`

```rust
use excel_diff::{SectionParseError, parse_section_members};

const SECTION_SINGLE: &str = r#"
    section Section1;

    shared Foo = 1;
"#;

const SECTION_MULTI: &str = r#"
    section Section1;

    shared Foo = 1;
    shared Bar = 2;
    Baz = 3;
"#;

const SECTION_NOISY: &str = r#"

// Leading comment

section Section1;

// Comment before Foo
shared Foo = 1;

// Another comment

    shared   Bar   =    2    ;

"#;

const SECTION_WITH_BOM: &str = "\u{FEFF}section Section1;\nshared Foo = 1;";

const SECTION_WITH_QUOTED_IDENTIFIER: &str = r#"
    section Section1;

    shared #"Query with space & #" = 1;
"#;

const SECTION_INVALID_SHARED: &str = r#"
    section Section1;

    shared Broken // missing '=' and ';'
"#;

#[test]
fn parse_single_member_section() {
    let members = parse_section_members(SECTION_SINGLE).expect("single member section parses");
    assert_eq!(members.len(), 1);

    let foo = &members[0];
    assert_eq!(foo.section_name, "Section1");
    assert_eq!(foo.member_name, "Foo");
    assert_eq!(foo.expression_m, "1");
    assert!(foo.is_shared);
}

#[test]
fn parse_multiple_members() {
    let members = parse_section_members(SECTION_MULTI).expect("multi-member section parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].section_name, "Section1");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].section_name, "Section1");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
}

#[test]
fn tolerate_whitespace_comments() {
    let members = parse_section_members(SECTION_NOISY).expect("noisy section still parses");
    assert_eq!(members.len(), 2);

    assert_eq!(members[0].member_name, "Foo");
    assert_eq!(members[0].expression_m, "1");
    assert!(members[0].is_shared);
    assert_eq!(members[0].section_name, "Section1");

    assert_eq!(members[1].member_name, "Bar");
    assert_eq!(members[1].expression_m, "2");
    assert!(members[1].is_shared);
    assert_eq!(members[1].section_name, "Section1");
}

#[test]
fn error_on_missing_section_header() {
    const NO_SECTION: &str = r#"
        shared Foo = 1;
    "#;

    let result = parse_section_members(NO_SECTION);
    assert_eq!(result, Err(SectionParseError::MissingSectionHeader));
}

#[test]
fn section_parsing_tolerates_utf8_bom() {
    let members =
        parse_section_members(SECTION_WITH_BOM).expect("BOM-prefixed section should parse");
    assert_eq!(members.len(), 1);

    let member = &members[0];
    assert_eq!(member.member_name, "Foo");
    assert_eq!(member.section_name, "Section1");
    assert_eq!(member.expression_m, "1");
    assert!(member.is_shared);
}

#[test]
fn parse_quoted_identifier_member() {
    let members = parse_section_members(SECTION_WITH_QUOTED_IDENTIFIER)
        .expect("quoted identifier should parse");
    assert_eq!(members.len(), 1);

    let member = &members[0];
    assert_eq!(member.section_name, "Section1");
    assert_eq!(member.member_name, "Query with space & #");
    assert_eq!(member.expression_m, "1");
    assert!(member.is_shared);
}

#[test]
fn error_on_invalid_shared_member_syntax() {
    let result = parse_section_members(SECTION_INVALID_SHARED);
    assert_eq!(result, Err(SectionParseError::InvalidMemberSyntax));
}
```

---

### File: `core\tests\output_tests.rs`

```rust
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ContainerError, DiffOp, DiffReport, ExcelOpenError,
    diff_workbooks, open_workbook,
    output::json::{
        CellDiff, diff_report_to_cell_diffs, diff_workbooks_to_json, serialize_cell_diffs,
        serialize_diff_report,
    },
};
use serde_json::Value;

mod common;
use common::fixture_path;

fn render_value(value: &Option<excel_diff::CellValue>) -> Option<String> {
    match value {
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(s)) => Some(s.clone()),
        Some(excel_diff::CellValue::Bool(b)) => Some(b.to_string()),
        None => None,
    }
}

fn make_cell_snapshot(addr: CellAddress, value: Option<CellValue>) -> CellSnapshot {
    CellSnapshot {
        addr,
        value,
        formula: None,
    }
}

#[test]
fn diff_report_to_cell_diffs_filters_non_cell_ops() {
    let addr1 = CellAddress::from_indices(0, 0);
    let addr2 = CellAddress::from_indices(1, 1);

    let report = DiffReport::new(vec![
        DiffOp::SheetAdded {
            sheet: "SheetAdded".into(),
        },
        DiffOp::cell_edited(
            "Sheet1".into(),
            addr1,
            make_cell_snapshot(addr1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr1, Some(CellValue::Number(2.0))),
        ),
        DiffOp::RowAdded {
            sheet: "Sheet1".into(),
            row_idx: 5,
            row_signature: None,
        },
        DiffOp::cell_edited(
            "Sheet2".into(),
            addr2,
            make_cell_snapshot(addr2, Some(CellValue::Text("old".into()))),
            make_cell_snapshot(addr2, Some(CellValue::Text("new".into()))),
        ),
        DiffOp::SheetRemoved {
            sheet: "OldSheet".into(),
        },
    ]);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(
        cell_diffs.len(),
        2,
        "only CellEdited ops should be projected"
    );

    assert_eq!(cell_diffs[0].coords, addr1.to_a1());
    assert_eq!(cell_diffs[0].value_file1, Some("1".into()));
    assert_eq!(cell_diffs[0].value_file2, Some("2".into()));

    assert_eq!(cell_diffs[1].coords, addr2.to_a1());
    assert_eq!(cell_diffs[1].value_file1, Some("old".into()));
    assert_eq!(cell_diffs[1].value_file2, Some("new".into()));
}

#[test]
fn diff_report_to_cell_diffs_ignores_block_moved_rect() {
    let addr = CellAddress::from_indices(2, 2);

    let report = DiffReport::new(vec![
        DiffOp::block_moved_rect("Sheet1".into(), 2, 3, 1, 3, 9, 6, Some(0xCAFEBABE)),
        DiffOp::cell_edited(
            "Sheet1".into(),
            addr,
            make_cell_snapshot(addr, Some(CellValue::Number(10.0))),
            make_cell_snapshot(addr, Some(CellValue::Number(20.0))),
        ),
        DiffOp::BlockMovedRows {
            sheet: "Sheet1".into(),
            src_start_row: 0,
            row_count: 2,
            dst_start_row: 5,
            block_hash: None,
        },
        DiffOp::BlockMovedColumns {
            sheet: "Sheet1".into(),
            src_start_col: 0,
            col_count: 2,
            dst_start_col: 5,
            block_hash: None,
        },
    ]);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(
        cell_diffs.len(),
        1,
        "only CellEdited should be projected; BlockMovedRect and other block moves should be ignored"
    );

    assert_eq!(cell_diffs[0].coords, addr.to_a1());
    assert_eq!(cell_diffs[0].value_file1, Some("10".into()));
    assert_eq!(cell_diffs[0].value_file2, Some("20".into()));
}

#[test]
fn diff_report_to_cell_diffs_maps_values_correctly() {
    let addr_num = CellAddress::from_indices(2, 2); // C3
    let addr_bool = CellAddress::from_indices(3, 3); // D4

    let report = DiffReport::new(vec![
        DiffOp::cell_edited(
            "SheetX".into(),
            addr_num,
            make_cell_snapshot(addr_num, Some(CellValue::Number(42.5))),
            make_cell_snapshot(addr_num, Some(CellValue::Number(43.5))),
        ),
        DiffOp::cell_edited(
            "SheetX".into(),
            addr_bool,
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(true))),
            make_cell_snapshot(addr_bool, Some(CellValue::Bool(false))),
        ),
    ]);

    let cell_diffs = diff_report_to_cell_diffs(&report);
    assert_eq!(cell_diffs.len(), 2);

    let number_diff = &cell_diffs[0];
    assert_eq!(number_diff.coords, addr_num.to_a1());
    assert_eq!(number_diff.value_file1, Some("42.5".into()));
    assert_eq!(number_diff.value_file2, Some("43.5".into()));

    let bool_diff = &cell_diffs[1];
    assert_eq!(bool_diff.coords, addr_bool.to_a1());
    assert_eq!(bool_diff.value_file1, Some("true".into()));
    assert_eq!(bool_diff.value_file2, Some("false".into()));
}

#[test]
fn test_json_format() {
    let diffs = vec![
        CellDiff {
            coords: "A1".into(),
            value_file1: Some("100".into()),
            value_file2: Some("200".into()),
        },
        CellDiff {
            coords: "B2".into(),
            value_file1: Some("true".into()),
            value_file2: Some("false".into()),
        },
        CellDiff {
            coords: "C3".into(),
            value_file1: Some("#DIV/0!".into()),
            value_file2: None,
        },
    ];

    let json = serialize_cell_diffs(&diffs).expect("serialization should succeed");
    let value: Value = serde_json::from_str(&json).expect("json should parse");

    assert!(value.is_array(), "expected an array of cell diffs");
    let arr = value
        .as_array()
        .expect("top-level json should be an array of cell diffs");
    assert_eq!(arr.len(), 3);

    let first = &arr[0];
    assert_eq!(first["coords"], Value::String("A1".into()));
    assert_eq!(first["value_file1"], Value::String("100".into()));
    assert_eq!(first["value_file2"], Value::String("200".into()));

    let second = &arr[1];
    assert_eq!(second["coords"], Value::String("B2".into()));
    assert_eq!(second["value_file1"], Value::String("true".into()));
    assert_eq!(second["value_file2"], Value::String("false".into()));

    let third = &arr[2];
    assert_eq!(third["coords"], Value::String("C3".into()));
    assert_eq!(third["value_file1"], Value::String("#DIV/0!".into()));
    assert_eq!(third["value_file2"], Value::Null);
}

#[test]
fn test_json_empty_diff() {
    let fixture = fixture_path("pg1_basic_two_sheets.xlsx");
    let json =
        diff_workbooks_to_json(&fixture, &fixture).expect("diffing identical files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "identical files should produce no diff ops"
    );
}

#[test]
fn test_json_non_empty_diff() {
    let a = fixture_path("json_diff_single_cell_a.xlsx");
    let b = fixture_path("json_diff_single_cell_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_non_empty_diff_bool() {
    let a = fixture_path("json_diff_bool_a.xlsx");
    let b = fixture_path("json_diff_bool_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("true".into()));
            assert_eq!(render_value(&to.value), Some("false".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_diff_value_to_empty() {
    let a = fixture_path("json_diff_value_to_empty_a.xlsx");
    let b = fixture_path("json_diff_value_to_empty_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b).expect("diffing different files should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single diff op");
    match &report.ops[0] {
        DiffOp::CellEdited { addr, from, to, .. } => {
            assert_eq!(addr.to_a1(), "C3");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), None);
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn json_diff_case_only_sheet_name_no_changes() {
    let a = fixture_path("sheet_case_only_rename_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_b.xlsx");

    let old = open_workbook(&a).expect("fixture A should open");
    let new = open_workbook(&b).expect("fixture B should open");

    let report = diff_workbooks(&old, &new);
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should produce no diff ops"
    );
}

#[test]
fn json_diff_case_only_sheet_name_cell_edit() {
    let a = fixture_path("sheet_case_only_rename_edit_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_edit_b.xlsx");

    let old = open_workbook(&a).expect("fixture A should open");
    let new = open_workbook(&b).expect("fixture B should open");

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");
    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_json_case_only_sheet_name_no_changes() {
    let a = fixture_path("sheet_case_only_rename_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_b.xlsx");

    let json =
        diff_workbooks_to_json(&a, &b).expect("diffing case-only sheet rename should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert!(
        report.ops.is_empty(),
        "case-only sheet rename with identical content should serialize to no ops"
    );
}

#[test]
fn test_json_case_only_sheet_name_cell_edit_via_helper() {
    let a = fixture_path("sheet_case_only_rename_edit_a.xlsx");
    let b = fixture_path("sheet_case_only_rename_edit_b.xlsx");

    let json = diff_workbooks_to_json(&a, &b)
        .expect("diffing case-only sheet rename with cell edit should succeed");
    let report: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(report.ops.len(), 1, "expected a single cell edit");

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(render_value(&from.value), Some("1".into()));
            assert_eq!(render_value(&to.value), Some("2".into()));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn test_diff_workbooks_to_json_reports_invalid_zip() {
    let path = fixture_path("not_a_zip.txt");
    let err = diff_workbooks_to_json(&path, &path)
        .expect_err("diffing invalid containers should return an error");

    assert!(
        matches!(
            err,
            ExcelOpenError::Container(ContainerError::NotZipContainer)
        ),
        "expected container error, got {err}"
    );
}

#[test]
fn serialize_diff_report_nan_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::NAN))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("NaN should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());

    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("nan"),
                "error message should mention NaN for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::INFINITY))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("Infinity should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());
    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_neg_infinity_maps_to_serialization_error() {
    let addr = CellAddress::from_indices(0, 0);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(f64::NEG_INFINITY))),
        make_cell_snapshot(addr, Some(CellValue::Number(1.0))),
    )]);

    let err = serialize_diff_report(&report).expect_err("NEG_INFINITY should fail to serialize");
    let wrapped = ExcelOpenError::SerializationError(err.to_string());
    match wrapped {
        ExcelOpenError::SerializationError(msg) => {
            assert!(
                msg.to_lowercase().contains("infinity"),
                "error message should mention infinity for clarity"
            );
        }
        other => panic!("expected SerializationError, got {other:?}"),
    }
}

#[test]
fn serialize_diff_report_with_finite_numbers_succeeds() {
    let addr = CellAddress::from_indices(1, 1);
    let report = DiffReport::new(vec![DiffOp::cell_edited(
        "Sheet1".into(),
        addr,
        make_cell_snapshot(addr, Some(CellValue::Number(2.5))),
        make_cell_snapshot(addr, Some(CellValue::Number(3.5))),
    )]);

    let json = serialize_diff_report(&report).expect("finite values should serialize");
    let parsed: DiffReport = serde_json::from_str(&json).expect("json should parse");
    assert_eq!(parsed.ops.len(), 1);
}
```

---

### File: `core\tests\pg1_ir_tests.rs`

```rust
use excel_diff::{CellValue, Sheet, SheetKind, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg1_basic_two_sheets_structure() {
    let workbook = open_workbook(fixture_path("pg1_basic_two_sheets.xlsx"))
        .expect("pg1 basic fixture should open");
    assert_eq!(workbook.sheets.len(), 2);
    assert_eq!(workbook.sheets[0].name, "Sheet1");
    assert_eq!(workbook.sheets[1].name, "Sheet2");
    assert!(matches!(workbook.sheets[0].kind, SheetKind::Worksheet));
    assert!(matches!(workbook.sheets[1].kind, SheetKind::Worksheet));

    let sheet1 = &workbook.sheets[0];
    assert_eq!(sheet1.grid.nrows, 3);
    assert_eq!(sheet1.grid.ncols, 3);
    assert_eq!(
        sheet1
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_text)),
        Some("R1C1")
    );

    let sheet2 = &workbook.sheets[1];
    assert_eq!(sheet2.grid.nrows, 5);
    assert_eq!(sheet2.grid.ncols, 2);
    assert_eq!(
        sheet2
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_text)),
        Some("S2_R1C1")
    );
}

#[test]
fn pg1_sparse_used_range_extents() {
    let workbook =
        open_workbook(fixture_path("pg1_sparse_used_range.xlsx")).expect("sparse fixture opens");
    let sheet = workbook
        .sheets
        .iter()
        .find(|s| s.name == "Sparse")
        .expect("Sparse sheet present");

    assert_eq!(sheet.grid.nrows, 10);
    assert_eq!(sheet.grid.ncols, 7);

    assert_cell_text(sheet, 0, 0, "A1");
    assert_cell_text(sheet, 1, 1, "B2");
    assert_cell_text(sheet, 9, 6, "G10");
    assert_eq!(sheet.grid.cell_count(), 3);
}

#[test]
fn pg1_empty_and_mixed_sheets() {
    let workbook = open_workbook(fixture_path("pg1_empty_and_mixed_sheets.xlsx"))
        .expect("mixed sheets fixture opens");

    let empty = sheet_by_name(&workbook, "Empty");
    assert_eq!(empty.grid.nrows, 0);
    assert_eq!(empty.grid.ncols, 0);
    assert_eq!(empty.grid.cell_count(), 0);

    let values_only = sheet_by_name(&workbook, "ValuesOnly");
    assert_eq!(values_only.grid.nrows, 10);
    assert_eq!(values_only.grid.ncols, 10);
    let values: Vec<_> = values_only.grid.iter_cells().collect();
    assert!(
        values
            .iter()
            .all(|c| c.value.is_some() && c.formula.is_none()),
        "ValuesOnly cells should have values and no formulas"
    );
    assert_eq!(
        values_only
            .grid
            .get(0, 0)
            .and_then(|cell| cell.value.as_ref().and_then(CellValue::as_number)),
        Some(1.0)
    );

    let formulas = sheet_by_name(&workbook, "FormulasOnly");
    assert_eq!(formulas.grid.nrows, 10);
    assert_eq!(formulas.grid.ncols, 10);
    let first = formulas.grid.get(0, 0).expect("A1 should exist");
    assert_eq!(first.formula.as_deref(), Some("ValuesOnly!A1"));
    assert!(
        first.value.is_some(),
        "Formulas should surface cached values when present"
    );
    assert!(
        formulas.grid.iter_cells().all(|c| c.formula.is_some()),
        "All cells should carry formulas in FormulasOnly"
    );
}

fn sheet_by_name<'a>(workbook: &'a excel_diff::Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("sheet {name} not found"))
}

fn assert_cell_text(sheet: &Sheet, row: u32, col: u32, expected: &str) {
    let cell = sheet
        .grid
        .get(row, col)
        .unwrap_or_else(|| panic!("cell {expected} should exist"));
    assert_eq!(cell.address.to_a1(), expected);
    assert_eq!(
        cell.value
            .as_ref()
            .and_then(CellValue::as_text)
            .unwrap_or(""),
        expected
    );
}
```

---

### File: `core\tests\pg3_snapshot_tests.rs`

```rust
use excel_diff::{
    Cell, CellAddress, CellSnapshot, CellValue, Sheet, Workbook, address_to_index, open_workbook,
};

mod common;
use common::fixture_path;

fn sheet_by_name<'a>(workbook: &'a Workbook, name: &str) -> &'a Sheet {
    workbook
        .sheets
        .iter()
        .find(|s| s.name == name)
        .expect("sheet should exist")
}

fn find_cell<'a>(sheet: &'a Sheet, addr: &str) -> Option<&'a Cell> {
    let (row, col) = address_to_index(addr).expect("address should parse");
    sheet.grid.get(row, col)
}

fn snapshot(sheet: &Sheet, addr: &str) -> CellSnapshot {
    if let Some(cell) = find_cell(sheet, addr) {
        CellSnapshot::from_cell(cell)
    } else {
        let (row, col) = address_to_index(addr).expect("address should parse");
        CellSnapshot {
            addr: CellAddress::from_indices(row, col),
            value: None,
            formula: None,
        }
    }
}

#[test]
fn pg3_value_and_formula_cells_snapshot_from_excel() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let a1 = snapshot(sheet, "A1");
    assert_eq!(a1.addr.to_string(), "A1");
    assert_eq!(a1.value, Some(CellValue::Number(42.0)));
    assert!(a1.formula.is_none());

    let a2 = snapshot(sheet, "A2");
    assert_eq!(a2.value, Some(CellValue::Text("hello".into())));
    assert!(a2.formula.is_none());

    let a3 = snapshot(sheet, "A3");
    assert_eq!(a3.value, Some(CellValue::Bool(true)));
    assert!(a3.formula.is_none());

    let a4 = snapshot(sheet, "A4");
    assert!(a4.value.is_none());
    assert!(a4.formula.is_none());

    let b1 = snapshot(sheet, "B1");
    assert!(matches!(
        b1.value,
        Some(CellValue::Number(n)) if (n - 43.0).abs() < 1e-6
    ));
    assert_eq!(b1.addr.to_string(), "B1");
    let b1_formula = b1.formula.as_deref().expect("B1 should have a formula");
    assert!(b1_formula.contains("A1+1"));

    let b2 = snapshot(sheet, "B2");
    assert_eq!(b2.value, Some(CellValue::Text("hello world".into())));
    assert_eq!(b2.addr.to_string(), "B2");
    let b2_formula = b2.formula.as_deref().expect("B2 should have a formula");
    assert!(b2_formula.contains("hello"));
    assert!(b2_formula.contains("world"));

    let b3 = snapshot(sheet, "B3");
    assert_eq!(b3.value, Some(CellValue::Bool(true)));
    assert_eq!(b3.addr.to_string(), "B3");
    let b3_formula = b3.formula.as_deref().expect("B3 should have a formula");
    assert!(
        b3_formula.contains(">0"),
        "B3 formula should include comparison: {b3_formula:?}"
    );
}

#[test]
fn snapshot_json_roundtrip() {
    let path = fixture_path("pg3_value_and_formula_cells.xlsx");
    let workbook = open_workbook(&path).expect("fixture should load");
    let sheet = sheet_by_name(&workbook, "Types");

    let snapshots = vec![
        snapshot(sheet, "A1"),
        snapshot(sheet, "A2"),
        snapshot(sheet, "B1"),
        snapshot(sheet, "B2"),
        snapshot(sheet, "B3"),
    ];

    for snap in snapshots {
        let addr = snap.addr.to_string();
        let json = serde_json::to_string(&snap).expect("snapshot should serialize");
        let as_value: serde_json::Value =
            serde_json::from_str(&json).expect("snapshot JSON should parse to value");
        assert_eq!(as_value["addr"], serde_json::Value::String(addr));
        let snap_back: CellSnapshot = serde_json::from_str(&json).expect("snapshot should parse");
        assert_eq!(snap.addr, snap_back.addr);
        assert_eq!(snap, snap_back);
    }
}

#[test]
fn snapshot_json_roundtrip_detects_tampered_addr() {
    let snap = CellSnapshot {
        addr: "Z9".parse().expect("address should parse"),
        value: Some(CellValue::Number(1.0)),
        formula: Some("A1+1".into()),
    };

    let mut value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&snap).expect("serialize should work"))
            .expect("serialized JSON should parse");
    value["addr"] = serde_json::Value::String("A1".into());

    let tampered_json = serde_json::to_string(&value).expect("tampered JSON should serialize");
    let tampered: CellSnapshot =
        serde_json::from_str(&tampered_json).expect("tampered JSON should parse");

    assert_ne!(snap.addr, tampered.addr);
    assert_eq!(snap, tampered, "value/formula equality ignores addr");
}

#[test]
fn snapshot_json_rejects_invalid_addr_1a() {
    let json = r#"{"addr":"1A","value":null,"formula":null}"#;
    let result: Result<CellSnapshot, _> = serde_json::from_str(json);
    let err = result
        .expect_err("invalid addr should fail to deserialize")
        .to_string();

    assert!(
        err.contains("invalid cell address"),
        "error should mention invalid cell address: {err}"
    );
    assert!(
        err.contains("1A"),
        "error should include the offending address: {err}"
    );
}

#[test]
fn snapshot_json_rejects_invalid_addr_a0() {
    let json = r#"{"addr":"A0","value":null,"formula":null}"#;
    let result: Result<CellSnapshot, _> = serde_json::from_str(json);
    let err = result
        .expect_err("invalid addr should fail to deserialize")
        .to_string();

    assert!(
        err.contains("invalid cell address"),
        "error should mention invalid cell address: {err}"
    );
    assert!(
        err.contains("A0"),
        "error should include the offending address: {err}"
    );
}
```

---

### File: `core\tests\pg4_diffop_tests.rs`

```rust
use excel_diff::{
    CellAddress, CellSnapshot, CellValue, ColSignature, DiffOp, DiffReport, RowSignature,
};
use serde_json::Value;
use std::collections::BTreeSet;

fn addr(a1: &str) -> CellAddress {
    a1.parse().expect("address should parse")
}

fn snapshot(a1: &str, value: Option<CellValue>, formula: Option<&str>) -> CellSnapshot {
    CellSnapshot {
        addr: addr(a1),
        value,
        formula: formula.map(|s| s.to_string()),
    }
}

fn sample_cell_edited() -> DiffOp {
    DiffOp::CellEdited {
        sheet: "Sheet1".to_string(),
        addr: addr("C3"),
        from: snapshot("C3", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    }
}

// Enforces the invariant documented on DiffOp::CellEdited.
fn assert_cell_edited_invariants(op: &DiffOp, expected_sheet: &str, expected_addr: &str) {
    let expected_addr_parsed: CellAddress =
        expected_addr.parse().expect("expected_addr should parse");
    if let DiffOp::CellEdited {
        sheet,
        addr,
        from,
        to,
    } = op
    {
        assert_eq!(sheet, expected_sheet);
        assert_eq!(*addr, expected_addr_parsed);
        assert_eq!(from.addr, expected_addr_parsed);
        assert_eq!(to.addr, expected_addr_parsed);
    } else {
        panic!("expected CellEdited");
    }
}

fn op_kind(op: &DiffOp) -> &'static str {
    match op {
        DiffOp::SheetAdded { .. } => "SheetAdded",
        DiffOp::SheetRemoved { .. } => "SheetRemoved",
        DiffOp::RowAdded { .. } => "RowAdded",
        DiffOp::RowRemoved { .. } => "RowRemoved",
        DiffOp::ColumnAdded { .. } => "ColumnAdded",
        DiffOp::ColumnRemoved { .. } => "ColumnRemoved",
        DiffOp::BlockMovedRows { .. } => "BlockMovedRows",
        DiffOp::BlockMovedColumns { .. } => "BlockMovedColumns",
        DiffOp::BlockMovedRect { .. } => "BlockMovedRect",
        DiffOp::CellEdited { .. } => "CellEdited",
        _ => "Unknown",
    }
}

fn json_keys(json: &Value) -> BTreeSet<String> {
    json.as_object()
        .expect("object json")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn pg4_construct_cell_edited_diffop() {
    let op = sample_cell_edited();

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
    if let DiffOp::CellEdited { from, to, .. } = &op {
        assert_ne!(from.value, to.value);
    }
}

#[test]
fn pg4_construct_row_and_column_diffops() {
    let row_added_with_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 8,
        row_signature: None,
    };
    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 0,
        col_signature: None,
    };

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_with_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 10);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0xDEADBEEF);
    } else {
        panic!("expected RowAdded with signature");
    }

    if let DiffOp::RowAdded {
        sheet,
        row_idx,
        row_signature,
    } = &row_added_without_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 11);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowAdded without signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_with_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 9);
        assert_eq!(row_signature.as_ref().unwrap().hash, 0x1234);
    } else {
        panic!("expected RowRemoved with signature");
    }

    if let DiffOp::RowRemoved {
        sheet,
        row_idx,
        row_signature,
    } = &row_removed_without_sig
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*row_idx, 8);
        assert!(row_signature.is_none());
    } else {
        panic!("expected RowRemoved without signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_with_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 2);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0xABCDEF);
    } else {
        panic!("expected ColumnAdded with signature");
    }

    if let DiffOp::ColumnAdded {
        sheet,
        col_idx,
        col_signature,
    } = &col_added_without_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 3);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnAdded without signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_with_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 1);
        assert_eq!(col_signature.as_ref().unwrap().hash, 0x123456);
    } else {
        panic!("expected ColumnRemoved with signature");
    }

    if let DiffOp::ColumnRemoved {
        sheet,
        col_idx,
        col_signature,
    } = &col_removed_without_sig
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*col_idx, 0);
        assert!(col_signature.is_none());
    } else {
        panic!("expected ColumnRemoved without signature");
    }

    assert_ne!(row_added_with_sig, row_added_without_sig);
    assert_ne!(row_removed_with_sig, row_removed_without_sig);
    assert_ne!(col_added_with_sig, col_added_without_sig);
    assert_ne!(col_removed_with_sig, col_removed_without_sig);
}

#[test]
fn pg4_construct_block_move_diffops() {
    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_with_hash
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*src_start_row, 10);
        assert_eq!(*row_count, 3);
        assert_eq!(*dst_start_row, 5);
        assert_eq!(block_hash.unwrap(), 0x12345678);
    } else {
        panic!("expected BlockMovedRows with hash");
    }

    if let DiffOp::BlockMovedRows {
        sheet,
        src_start_row,
        row_count,
        dst_start_row,
        block_hash,
    } = &block_rows_without_hash
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*src_start_row, 20);
        assert_eq!(*row_count, 2);
        assert_eq!(*dst_start_row, 0);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedRows without hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_with_hash
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*src_start_col, 7);
        assert_eq!(*col_count, 2);
        assert_eq!(*dst_start_col, 3);
        assert_eq!(block_hash.unwrap(), 0xCAFEBABE);
    } else {
        panic!("expected BlockMovedColumns with hash");
    }

    if let DiffOp::BlockMovedColumns {
        sheet,
        src_start_col,
        col_count,
        dst_start_col,
        block_hash,
    } = &block_cols_without_hash
    {
        assert_eq!(sheet, "Sheet2");
        assert_eq!(*src_start_col, 4);
        assert_eq!(*col_count, 1);
        assert_eq!(*dst_start_col, 9);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedColumns without hash");
    }

    assert_ne!(block_rows_with_hash, block_rows_without_hash);
    assert_ne!(block_cols_with_hash, block_cols_without_hash);
}

#[test]
fn pg4_construct_block_rect_diffops() {
    let rect_with_hash = DiffOp::BlockMovedRect {
        sheet: "Sheet1".to_string(),
        src_start_row: 5,
        src_row_count: 3,
        src_start_col: 2,
        src_col_count: 4,
        dst_start_row: 10,
        dst_start_col: 6,
        block_hash: Some(0xCAFEBABE),
    };
    let rect_without_hash = DiffOp::BlockMovedRect {
        sheet: "Sheet1".to_string(),
        src_start_row: 0,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 1,
        dst_start_row: 20,
        dst_start_col: 10,
        block_hash: None,
    };

    if let DiffOp::BlockMovedRect {
        sheet,
        src_start_row,
        src_row_count,
        src_start_col,
        src_col_count,
        dst_start_row,
        dst_start_col,
        block_hash,
    } = &rect_with_hash
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*src_start_row, 5);
        assert_eq!(*src_row_count, 3);
        assert_eq!(*src_start_col, 2);
        assert_eq!(*src_col_count, 4);
        assert_eq!(*dst_start_row, 10);
        assert_eq!(*dst_start_col, 6);
        assert_eq!(block_hash.unwrap(), 0xCAFEBABE);
    } else {
        panic!("expected BlockMovedRect with hash");
    }

    if let DiffOp::BlockMovedRect {
        sheet,
        src_start_row,
        src_row_count,
        src_start_col,
        src_col_count,
        dst_start_row,
        dst_start_col,
        block_hash,
    } = &rect_without_hash
    {
        assert_eq!(sheet, "Sheet1");
        assert_eq!(*src_start_row, 0);
        assert_eq!(*src_row_count, 1);
        assert_eq!(*src_start_col, 0);
        assert_eq!(*src_col_count, 1);
        assert_eq!(*dst_start_row, 20);
        assert_eq!(*dst_start_col, 10);
        assert!(block_hash.is_none());
    } else {
        panic!("expected BlockMovedRect without hash");
    }

    assert_ne!(rect_with_hash, rect_without_hash);
}

#[test]
fn pg4_cell_edited_json_shape() {
    let op = sample_cell_edited();
    let json = serde_json::to_value(&op).expect("serialize");
    assert_cell_edited_invariants(&op, "Sheet1", "C3");

    assert_eq!(json["kind"], "CellEdited");
    assert_eq!(json["sheet"], "Sheet1");
    assert_eq!(json["addr"], "C3");
    assert_eq!(json["from"]["addr"], "C3");
    assert_eq!(json["to"]["addr"], "C3");

    let obj = json.as_object().expect("object json");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["addr", "from", "kind", "sheet", "to"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(keys, expected);
}

#[test]
fn pg4_row_added_json_optional_signature() {
    let op_without_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: None,
    };
    let json_without = serde_json::to_value(&op_without_sig).expect("serialize without sig");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "RowAdded");
    assert_eq!(json_without["sheet"], "Sheet1");
    assert_eq!(json_without["row_idx"], 10);
    assert!(obj_without.get("row_signature").is_none());

    let op_with_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 123 }),
    };
    let json_with = serde_json::to_value(&op_with_sig).expect("serialize with sig");
    assert_eq!(json_with["row_signature"]["hash"], 123);
}

#[test]
fn pg4_column_added_json_optional_signature() {
    let added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet1".to_string(),
        col_idx: 5,
        col_signature: None,
    };
    let json_added_without = serde_json::to_value(&added_without_sig).expect("serialize no sig");
    let obj_added_without = json_added_without.as_object().expect("object json");
    assert_eq!(json_added_without["kind"], "ColumnAdded");
    assert_eq!(json_added_without["sheet"], "Sheet1");
    assert_eq!(json_added_without["col_idx"], 5);
    assert!(obj_added_without.get("col_signature").is_none());

    let added_with_sig = DiffOp::ColumnAdded {
        sheet: "Sheet1".to_string(),
        col_idx: 6,
        col_signature: Some(ColSignature { hash: 321 }),
    };
    let json_added_with = serde_json::to_value(&added_with_sig).expect("serialize with sig");
    assert_eq!(json_added_with["col_signature"]["hash"], 321);

    let removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: None,
    };
    let json_removed_without =
        serde_json::to_value(&removed_without_sig).expect("serialize removed no sig");
    let obj_removed_without = json_removed_without.as_object().expect("object json");
    assert_eq!(json_removed_without["kind"], "ColumnRemoved");
    assert!(obj_removed_without.get("col_signature").is_none());

    let removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 654 }),
    };
    let json_removed_with =
        serde_json::to_value(&removed_with_sig).expect("serialize removed with sig");
    assert_eq!(json_removed_with["col_signature"]["hash"], 654);
}

#[test]
fn pg4_block_moved_rows_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedRows");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 1,
        row_count: 2,
        dst_start_row: 5,
        block_hash: Some(777),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(777));
}

#[test]
fn pg4_block_moved_columns_json_optional_hash() {
    let op_without_hash = DiffOp::BlockMovedColumns {
        sheet: "SheetX".to_string(),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: None,
    };
    let json_without = serde_json::to_value(&op_without_hash).expect("serialize without hash");
    let obj_without = json_without.as_object().expect("object json");
    assert_eq!(json_without["kind"], "BlockMovedColumns");
    assert!(obj_without.get("block_hash").is_none());

    let op_with_hash = DiffOp::BlockMovedColumns {
        sheet: "SheetX".to_string(),
        src_start_col: 2,
        col_count: 3,
        dst_start_col: 9,
        block_hash: Some(4242),
    };
    let json_with = serde_json::to_value(&op_with_hash).expect("serialize with hash");
    assert_eq!(json_with["block_hash"], Value::from(4242));
}

#[test]
fn pg4_sheet_added_and_removed_json_shape() {
    let added = DiffOp::SheetAdded {
        sheet: "Sheet1".to_string(),
    };
    let added_json = serde_json::to_value(&added).expect("serialize sheet added");
    assert_eq!(added_json["kind"], "SheetAdded");
    assert_eq!(added_json["sheet"], "Sheet1");
    let added_keys = json_keys(&added_json);
    let expected_keys: BTreeSet<String> = ["kind", "sheet"].into_iter().map(String::from).collect();
    assert_eq!(added_keys, expected_keys);

    let removed = DiffOp::SheetRemoved {
        sheet: "SheetX".to_string(),
    };
    let removed_json = serde_json::to_value(&removed).expect("serialize sheet removed");
    assert_eq!(removed_json["kind"], "SheetRemoved");
    assert_eq!(removed_json["sheet"], "SheetX");
    let removed_keys = json_keys(&removed_json);
    assert_eq!(removed_keys, expected_keys);
}

#[test]
fn pg4_row_and_column_json_shape_keysets() {
    let expected_row_with_sig: BTreeSet<String> = ["kind", "row_idx", "row_signature", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_row_without_sig: BTreeSet<String> = ["kind", "row_idx", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_with_sig: BTreeSet<String> = ["col_idx", "col_signature", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();
    let expected_col_without_sig: BTreeSet<String> = ["col_idx", "kind", "sheet"]
        .into_iter()
        .map(String::from)
        .collect();

    let row_added_with_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: Some(RowSignature { hash: 0xDEADBEEF }),
    };
    let row_added_without_sig = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 11,
        row_signature: None,
    };
    let row_removed_with_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 9,
        row_signature: Some(RowSignature { hash: 0x1234 }),
    };
    let row_removed_without_sig = DiffOp::RowRemoved {
        sheet: "Sheet1".to_string(),
        row_idx: 8,
        row_signature: None,
    };

    let col_added_with_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 2,
        col_signature: Some(ColSignature { hash: 0xABCDEF }),
    };
    let col_added_without_sig = DiffOp::ColumnAdded {
        sheet: "Sheet2".to_string(),
        col_idx: 3,
        col_signature: None,
    };
    let col_removed_with_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 1,
        col_signature: Some(ColSignature { hash: 0x123456 }),
    };
    let col_removed_without_sig = DiffOp::ColumnRemoved {
        sheet: "Sheet2".to_string(),
        col_idx: 0,
        col_signature: None,
    };

    let cases = vec![
        (
            row_added_with_sig,
            "RowAdded",
            expected_row_with_sig.clone(),
        ),
        (
            row_added_without_sig,
            "RowAdded",
            expected_row_without_sig.clone(),
        ),
        (
            row_removed_with_sig,
            "RowRemoved",
            expected_row_with_sig.clone(),
        ),
        (
            row_removed_without_sig,
            "RowRemoved",
            expected_row_without_sig.clone(),
        ),
        (
            col_added_with_sig,
            "ColumnAdded",
            expected_col_with_sig.clone(),
        ),
        (
            col_added_without_sig,
            "ColumnAdded",
            expected_col_without_sig.clone(),
        ),
        (
            col_removed_with_sig,
            "ColumnRemoved",
            expected_col_with_sig.clone(),
        ),
        (
            col_removed_without_sig,
            "ColumnRemoved",
            expected_col_without_sig.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_block_move_json_shape_keysets() {
    let expected_rows_with_hash: BTreeSet<String> = [
        "block_hash",
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rows_without_hash: BTreeSet<String> = [
        "dst_start_row",
        "kind",
        "row_count",
        "sheet",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_with_hash: BTreeSet<String> = [
        "block_hash",
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_cols_without_hash: BTreeSet<String> = [
        "col_count",
        "dst_start_col",
        "kind",
        "sheet",
        "src_start_col",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rect_with_hash: BTreeSet<String> = [
        "block_hash",
        "dst_start_col",
        "dst_start_row",
        "kind",
        "sheet",
        "src_col_count",
        "src_row_count",
        "src_start_col",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let expected_rect_without_hash: BTreeSet<String> = [
        "dst_start_col",
        "dst_start_row",
        "kind",
        "sheet",
        "src_col_count",
        "src_row_count",
        "src_start_col",
        "src_start_row",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let block_rows_with_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 10,
        row_count: 3,
        dst_start_row: 5,
        block_hash: Some(0x12345678),
    };
    let block_rows_without_hash = DiffOp::BlockMovedRows {
        sheet: "Sheet1".to_string(),
        src_start_row: 20,
        row_count: 2,
        dst_start_row: 0,
        block_hash: None,
    };
    let block_cols_with_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 7,
        col_count: 2,
        dst_start_col: 3,
        block_hash: Some(0xCAFEBABE),
    };
    let block_cols_without_hash = DiffOp::BlockMovedColumns {
        sheet: "Sheet2".to_string(),
        src_start_col: 4,
        col_count: 1,
        dst_start_col: 9,
        block_hash: None,
    };
    let block_rect_with_hash = DiffOp::BlockMovedRect {
        sheet: "SheetZ".to_string(),
        src_start_row: 2,
        src_row_count: 2,
        src_start_col: 3,
        src_col_count: 4,
        dst_start_row: 8,
        dst_start_col: 1,
        block_hash: Some(0xAABBCCDD),
    };
    let block_rect_without_hash = DiffOp::BlockMovedRect {
        sheet: "SheetZ".to_string(),
        src_start_row: 5,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 2,
        dst_start_row: 10,
        dst_start_col: 4,
        block_hash: None,
    };

    let cases = vec![
        (
            block_rows_with_hash,
            "BlockMovedRows",
            expected_rows_with_hash.clone(),
        ),
        (
            block_rows_without_hash,
            "BlockMovedRows",
            expected_rows_without_hash.clone(),
        ),
        (
            block_cols_with_hash,
            "BlockMovedColumns",
            expected_cols_with_hash.clone(),
        ),
        (
            block_cols_without_hash,
            "BlockMovedColumns",
            expected_cols_without_hash.clone(),
        ),
        (
            block_rect_with_hash,
            "BlockMovedRect",
            expected_rect_with_hash.clone(),
        ),
        (
            block_rect_without_hash,
            "BlockMovedRect",
            expected_rect_without_hash.clone(),
        ),
    ];

    for (op, expected_kind, expected_keys) in cases {
        let json = serde_json::to_value(&op).expect("serialize diffop");
        assert_eq!(json["kind"], expected_kind);
        let keys = json_keys(&json);
        assert_eq!(keys, expected_keys);
    }
}

#[test]
fn pg4_block_rect_json_shape_and_roundtrip() {
    let without_hash = DiffOp::BlockMovedRect {
        sheet: "Sheet1".to_string(),
        src_start_row: 2,
        src_row_count: 3,
        src_start_col: 1,
        src_col_count: 2,
        dst_start_row: 10,
        dst_start_col: 5,
        block_hash: None,
    };
    let with_hash = DiffOp::BlockMovedRect {
        sheet: "Sheet1".to_string(),
        src_start_row: 4,
        src_row_count: 1,
        src_start_col: 0,
        src_col_count: 1,
        dst_start_row: 20,
        dst_start_col: 7,
        block_hash: Some(0x55AA),
    };

    let report = DiffReport::new(vec![without_hash.clone(), with_hash.clone()]);
    let json = serde_json::to_value(&report).expect("serialize rect report");

    let ops_json = json["ops"]
        .as_array()
        .expect("ops should be array for report");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "BlockMovedRect");
    assert_eq!(ops_json[0]["sheet"], "Sheet1");
    assert_eq!(ops_json[0]["src_start_row"], 2);
    assert_eq!(ops_json[0]["src_row_count"], 3);
    assert_eq!(ops_json[0]["src_start_col"], 1);
    assert_eq!(ops_json[0]["src_col_count"], 2);
    assert_eq!(ops_json[0]["dst_start_row"], 10);
    assert_eq!(ops_json[0]["dst_start_col"], 5);
    assert!(
        ops_json[0].get("block_hash").is_none(),
        "block_hash should be omitted when None"
    );

    assert_eq!(ops_json[1]["kind"], "BlockMovedRect");
    assert_eq!(ops_json[1]["block_hash"], Value::from(0x55AA));

    let roundtrip: DiffReport =
        serde_json::from_value(json).expect("roundtrip deserialize rect report");
    assert_eq!(roundtrip.ops, vec![without_hash, with_hash]);
}

#[test]
fn pg4_diffop_roundtrip_each_variant() {
    let ops = vec![
        DiffOp::SheetAdded {
            sheet: "SheetA".to_string(),
        },
        DiffOp::SheetRemoved {
            sheet: "SheetB".to_string(),
        },
        DiffOp::RowAdded {
            sheet: "Sheet1".to_string(),
            row_idx: 1,
            row_signature: Some(RowSignature { hash: 42 }),
        },
        DiffOp::RowRemoved {
            sheet: "Sheet1".to_string(),
            row_idx: 0,
            row_signature: None,
        },
        DiffOp::ColumnAdded {
            sheet: "Sheet1".to_string(),
            col_idx: 2,
            col_signature: None,
        },
        DiffOp::ColumnRemoved {
            sheet: "Sheet1".to_string(),
            col_idx: 3,
            col_signature: Some(ColSignature { hash: 99 }),
        },
        DiffOp::BlockMovedRows {
            sheet: "Sheet1".to_string(),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: Some(1234),
        },
        DiffOp::BlockMovedRows {
            sheet: "Sheet1".to_string(),
            src_start_row: 5,
            row_count: 2,
            dst_start_row: 10,
            block_hash: None,
        },
        DiffOp::BlockMovedColumns {
            sheet: "Sheet2".to_string(),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: Some(888),
        },
        DiffOp::BlockMovedColumns {
            sheet: "Sheet2".to_string(),
            src_start_col: 4,
            col_count: 1,
            dst_start_col: 6,
            block_hash: None,
        },
        DiffOp::BlockMovedRect {
            sheet: "Sheet3".to_string(),
            src_start_row: 1,
            src_row_count: 2,
            src_start_col: 3,
            src_col_count: 4,
            dst_start_row: 10,
            dst_start_col: 20,
            block_hash: Some(0xABCD),
        },
        DiffOp::BlockMovedRect {
            sheet: "Sheet3".to_string(),
            src_start_row: 1,
            src_row_count: 2,
            src_start_col: 3,
            src_col_count: 4,
            dst_start_row: 10,
            dst_start_col: 20,
            block_hash: None,
        },
        sample_cell_edited(),
    ];

    for original in ops {
        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: DiffOp = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized, original);

        if let DiffOp::CellEdited { .. } = &deserialized {
            assert_cell_edited_invariants(&deserialized, "Sheet1", "C3");
        }
    }
}

#[test]
fn pg4_cell_edited_roundtrip_preserves_snapshot_addrs() {
    let op = sample_cell_edited();
    let json = serde_json::to_string(&op).expect("serialize");
    let round_tripped: DiffOp = serde_json::from_str(&json).expect("deserialize");

    assert_cell_edited_invariants(&round_tripped, "Sheet1", "C3");
}

#[test]
fn pg4_diff_report_roundtrip_preserves_order() {
    let op1 = DiffOp::SheetAdded {
        sheet: "Sheet1".to_string(),
    };
    let op2 = DiffOp::RowAdded {
        sheet: "Sheet1".to_string(),
        row_idx: 10,
        row_signature: None,
    };
    let op3 = sample_cell_edited();

    let ops = vec![op1, op2, op3];
    let report = DiffReport::new(ops.clone());
    assert_eq!(report.version, DiffReport::SCHEMA_VERSION);

    let serialized = serde_json::to_string(&report).expect("serialize report");
    let deserialized: DiffReport = serde_json::from_str(&serialized).expect("deserialize report");
    assert_eq!(deserialized.version, "1");
    assert_eq!(deserialized.ops, ops);

    let kinds: Vec<&str> = deserialized.ops.iter().map(op_kind).collect();
    assert_eq!(kinds, vec!["SheetAdded", "RowAdded", "CellEdited"]);
}

#[test]
fn pg4_diff_report_json_shape() {
    let ops = vec![
        DiffOp::SheetRemoved {
            sheet: "SheetX".to_string(),
        },
        DiffOp::RowRemoved {
            sheet: "SheetX".to_string(),
            row_idx: 3,
            row_signature: Some(RowSignature { hash: 7 }),
        },
    ];
    let report = DiffReport::new(ops);
    let json = serde_json::to_value(&report).expect("serialize to value");

    let obj = json.as_object().expect("report json object");
    let keys: BTreeSet<String> = obj.keys().cloned().collect();
    let expected: BTreeSet<String> = ["ops", "version"].into_iter().map(String::from).collect();
    assert_eq!(keys, expected);
    assert_eq!(obj.get("version").and_then(Value::as_str), Some("1"));

    let ops_json = obj
        .get("ops")
        .and_then(Value::as_array)
        .expect("ops should be array");
    assert_eq!(ops_json.len(), 2);
    assert_eq!(ops_json[0]["kind"], "SheetRemoved");
    assert_eq!(ops_json[1]["kind"], "RowRemoved");
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_top_level_addr() {
    let json = r#"{
        "kind": "CellEdited",
        "sheet": "Sheet1",
        "addr": "1A",
        "from": { "addr": "C3", "value": null, "formula": null },
        "to":   { "addr": "C3", "value": null, "formula": null }
    }"#;

    let err = serde_json::from_str::<DiffOp>(json)
        .expect_err("invalid top-level addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diffop_cell_edited_rejects_invalid_snapshot_addrs() {
    let json = r#"{
        "kind": "CellEdited",
        "sheet": "Sheet1",
        "addr": "C3",
        "from": { "addr": "A0", "value": null, "formula": null },
        "to":   { "addr": "C3", "value": null, "formula": null }
    }"#;

    let err = serde_json::from_str::<DiffOp>(json)
        .expect_err("invalid snapshot addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("A0"),
        "error should mention invalid address: {msg}",
    );
}

#[test]
fn pg4_diff_report_rejects_invalid_nested_addr() {
    let json = r#"{
        "version": "1",
        "ops": [{
            "kind": "CellEdited",
            "sheet": "Sheet1",
            "addr": "1A",
            "from": { "addr": "C3", "value": null, "formula": null },
            "to":   { "addr": "C3", "value": null, "formula": null }
        }]
    }"#;

    let err = serde_json::from_str::<DiffReport>(json)
        .expect_err("invalid nested addr should fail to deserialize");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid cell address") && msg.contains("1A"),
        "error should surface nested invalid address: {msg}",
    );
}

#[test]
#[should_panic]
fn pg4_cell_edited_invariant_helper_rejects_mismatched_snapshot_addr() {
    let op = DiffOp::CellEdited {
        sheet: "Sheet1".to_string(),
        addr: addr("C3"),
        from: snapshot("D4", Some(CellValue::Number(1.0)), None),
        to: snapshot("C3", Some(CellValue::Number(2.0)), None),
    };

    assert_cell_edited_invariants(&op, "Sheet1", "C3");
}
```

---

### File: `core\tests\pg5_grid_diff_tests.rs`

```rust
use excel_diff::{CellValue, DiffOp, Grid, diff_workbooks};
use std::collections::BTreeSet;

mod common;
use common::{grid_from_numbers, single_sheet_workbook};

#[test]
fn pg5_1_grid_diff_1x1_identical_empty_diff() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert!(report.ops.is_empty());
}

#[test]
fn pg5_2_grid_diff_1x1_value_change_single_cell_edited() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[2]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(addr.to_a1(), "A1");
            assert_eq!(from.value, Some(CellValue::Number(1.0)));
            assert_eq!(to.value, Some(CellValue::Number(2.0)));
        }
        other => panic!("expected CellEdited, got {other:?}"),
    }
}

#[test]
fn pg5_3_grid_diff_row_appended_row_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowAdded, got {other:?}"),
    }
}

#[test]
fn pg5_4_grid_diff_column_appended_column_added_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnAdded, got {other:?}"),
    }
}

#[test]
fn pg5_5_grid_diff_same_shape_scattered_cell_edits() {
    let old = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]),
    );
    let new = single_sheet_workbook(
        "Sheet1",
        grid_from_numbers(&[&[10, 2, 3], &[4, 50, 6], &[7, 8, 90]]),
    );

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 3);
    assert!(
        report
            .ops
            .iter()
            .all(|op| matches!(op, DiffOp::CellEdited { .. }))
    );

    let edited_addrs: BTreeSet<String> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.to_a1()),
            _ => None,
        })
        .collect();
    let expected: BTreeSet<String> = ["A1", "B2", "C3"].into_iter().map(String::from).collect();
    assert_eq!(edited_addrs, expected);
}

#[test]
fn pg5_6_grid_diff_degenerate_grids() {
    let empty_old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let empty_new = single_sheet_workbook("Sheet1", Grid::new(0, 0));

    let empty_report = diff_workbooks(&empty_old, &empty_new);
    assert!(empty_report.ops.is_empty());

    let old = single_sheet_workbook("Sheet1", Grid::new(0, 0));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 2);

    let mut row_added = 0;
    let mut col_added = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*row_idx, 0);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::ColumnAdded {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*col_idx, 0);
                assert!(col_signature.is_none());
                col_added += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    assert_eq!(col_added, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_7_grid_diff_row_truncated_row_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*row_idx, 1);
            assert!(row_signature.is_none());
        }
        other => panic!("expected RowRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_8_grid_diff_column_truncated_column_removed_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 10], &[2, 20]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1], &[2]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 1);

    match &report.ops[0] {
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        } => {
            assert_eq!(sheet, "Sheet1");
            assert_eq!(*col_idx, 1);
            assert!(col_signature.is_none());
        }
        other => panic!("expected ColumnRemoved, got {other:?}"),
    }
}

#[test]
fn pg5_9_grid_diff_row_and_column_truncated_structure_only() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 2);

    let mut rows_removed = 0;
    let mut cols_removed = 0;
    let mut cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::RowRemoved {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*row_idx, 1);
                assert!(row_signature.is_none());
                rows_removed += 1;
            }
            DiffOp::ColumnRemoved {
                sheet,
                col_idx,
                col_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*col_idx, 1);
                assert!(col_signature.is_none());
                cols_removed += 1;
            }
            DiffOp::CellEdited { .. } => cell_edits += 1,
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(rows_removed, 1);
    assert_eq!(cols_removed, 1);
    assert_eq!(cell_edits, 0);
}

#[test]
fn pg5_10_grid_diff_row_appended_with_overlap_cell_edits() {
    let old = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2], &[3, 4]]));
    let new = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 20], &[30, 4], &[5, 6]]));

    let report = diff_workbooks(&old, &new);
    assert_eq!(report.ops.len(), 3);

    let mut row_added = 0;
    let mut cell_edits = BTreeSet::new();

    for op in &report.ops {
        match op {
            DiffOp::RowAdded {
                sheet,
                row_idx,
                row_signature,
            } => {
                assert_eq!(sheet, "Sheet1");
                assert_eq!(*row_idx, 2);
                assert!(row_signature.is_none());
                row_added += 1;
            }
            DiffOp::CellEdited { addr, .. } => {
                cell_edits.insert(addr.to_a1());
            }
            other => panic!("unexpected op: {other:?}"),
        }
    }

    assert_eq!(row_added, 1);
    let expected: BTreeSet<String> = ["B1", "A2"].into_iter().map(String::from).collect();
    assert_eq!(cell_edits, expected);
}
```

---

### File: `core\tests\pg6_object_vs_grid_tests.rs`

```rust
use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::fixture_path;

#[test]
fn pg6_1_sheet_added_no_grid_ops_on_main() {
    let old = open_workbook(fixture_path("pg6_sheet_added_a.xlsx")).expect("open pg6 added A");
    let new = open_workbook(fixture_path("pg6_sheet_added_b.xlsx")).expect("open pg6 added B");

    let report = diff_workbooks(&old, &new);

    let mut sheet_added = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "NewSheet" => sheet_added += 1,
            DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::CellEdited { sheet, .. }
                if sheet == "Main" =>
            {
                panic!("unexpected grid op on Main: {op:?}");
            }
            DiffOp::SheetAdded { sheet } => {
                panic!("unexpected sheet added: {sheet}");
            }
            DiffOp::SheetRemoved { sheet } => {
                panic!("unexpected sheet removed: {sheet}");
            }
            DiffOp::BlockMovedRows { .. } | DiffOp::BlockMovedColumns { .. } => {
                panic!("block move ops are not expected in PG6.1: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(sheet_added, 1, "exactly one NewSheet addition expected");
    assert_eq!(report.ops.len(), 1, "no other operations expected");
}

#[test]
fn pg6_2_sheet_removed_no_grid_ops_on_main() {
    let old = open_workbook(fixture_path("pg6_sheet_removed_a.xlsx")).expect("open pg6 removed A");
    let new = open_workbook(fixture_path("pg6_sheet_removed_b.xlsx")).expect("open pg6 removed B");

    let report = diff_workbooks(&old, &new);

    let mut sheet_removed = 0;
    for op in &report.ops {
        match op {
            DiffOp::SheetRemoved { sheet } if sheet == "OldSheet" => sheet_removed += 1,
            DiffOp::RowAdded { sheet, .. }
            | DiffOp::RowRemoved { sheet, .. }
            | DiffOp::ColumnAdded { sheet, .. }
            | DiffOp::ColumnRemoved { sheet, .. }
            | DiffOp::CellEdited { sheet, .. }
                if sheet == "Main" =>
            {
                panic!("unexpected grid op on Main: {op:?}");
            }
            DiffOp::SheetAdded { sheet } => {
                panic!("unexpected sheet added: {sheet}");
            }
            DiffOp::SheetRemoved { sheet } => {
                panic!("unexpected sheet removed: {sheet}");
            }
            DiffOp::BlockMovedRows { .. } | DiffOp::BlockMovedColumns { .. } => {
                panic!("block move ops are not expected in PG6.2: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(sheet_removed, 1, "exactly one OldSheet removal expected");
    assert_eq!(report.ops.len(), 1, "no other operations expected");
}

#[test]
fn pg6_3_rename_as_remove_plus_add_no_grid_ops() {
    let old = open_workbook(fixture_path("pg6_sheet_renamed_a.xlsx")).expect("open pg6 rename A");
    let new = open_workbook(fixture_path("pg6_sheet_renamed_b.xlsx")).expect("open pg6 rename B");

    let report = diff_workbooks(&old, &new);

    let mut added = 0;
    let mut removed = 0;

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "NewName" => added += 1,
            DiffOp::SheetRemoved { sheet } if sheet == "OldName" => removed += 1,
            DiffOp::SheetAdded { sheet } => panic!("unexpected sheet added: {sheet}"),
            DiffOp::SheetRemoved { sheet } => panic!("unexpected sheet removed: {sheet}"),
            DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. }
            | DiffOp::CellEdited { .. }
            | DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. } => {
                panic!("no grid-level ops expected for rename scenario: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(
        report.ops.len(),
        2,
        "rename should produce one add and one remove"
    );
    assert_eq!(added, 1, "expected one NewName addition");
    assert_eq!(removed, 1, "expected one OldName removal");
}

#[test]
fn pg6_4_sheet_and_grid_change_composed_cleanly() {
    let old =
        open_workbook(fixture_path("pg6_sheet_and_grid_change_a.xlsx")).expect("open pg6 4 A");
    let new =
        open_workbook(fixture_path("pg6_sheet_and_grid_change_b.xlsx")).expect("open pg6 4 B");

    let report = diff_workbooks(&old, &new);

    let mut scratch_added = 0;
    let mut main_cell_edits = 0;

    for op in &report.ops {
        match op {
            DiffOp::SheetAdded { sheet } if sheet == "Scratch" => scratch_added += 1,
            DiffOp::CellEdited { sheet, .. } => {
                assert_eq!(sheet, "Main", "only Main should have cell edits");
                main_cell_edits += 1;
            }
            DiffOp::SheetRemoved { .. } => {
                panic!("no sheets should be removed in PG6.4: {op:?}");
            }
            DiffOp::RowAdded { .. }
            | DiffOp::RowRemoved { .. }
            | DiffOp::ColumnAdded { .. }
            | DiffOp::ColumnRemoved { .. }
            | DiffOp::BlockMovedRows { .. }
            | DiffOp::BlockMovedColumns { .. } => {
                panic!("no structural row/column ops expected in PG6.4: {op:?}");
            }
            _ => panic!("unexpected op variant: {op:?}"),
        }
    }

    assert_eq!(scratch_added, 1, "exactly one Scratch addition expected");
    assert!(
        main_cell_edits > 0,
        "Main should report at least one cell edit"
    );
}
```

---

### File: `core\tests\signature_tests.rs`

```rust
use excel_diff::{Cell, CellAddress, CellValue, Grid, GridView};

fn make_cell(row: u32, col: u32, value: Option<CellValue>, formula: Option<&str>) -> Cell {
    Cell {
        row,
        col,
        address: CellAddress::from_indices(row, col),
        value,
        formula: formula.map(|s| s.to_string()),
    }
}

#[test]
fn identical_rows_have_same_signature() {
    let mut grid1 = Grid::new(1, 3);
    let mut grid2 = Grid::new(1, 3);
    for c in 0..3 {
        let cell = make_cell(0, c, Some(CellValue::Number(c as f64)), None);
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
        grid1.insert(make_cell(0, c, Some(CellValue::Number(c as f64)), None));
        grid2.insert(make_cell(
            0,
            c,
            Some(CellValue::Number((c + 1) as f64)),
            None,
        ));
    }
    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1, sig2);
}

#[test]
fn compute_all_signatures_populates_fields() {
    let mut grid = Grid::new(5, 5);
    grid.insert(make_cell(
        2,
        2,
        Some(CellValue::Text("center".into())),
        None,
    ));
    assert!(grid.row_signatures.is_none());
    assert!(grid.col_signatures.is_none());
    grid.compute_all_signatures();
    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert_eq!(grid.row_signatures.as_ref().unwrap().len(), 5);
    assert_eq!(grid.col_signatures.as_ref().unwrap().len(), 5);
    assert_ne!(grid.row_signatures.as_ref().unwrap()[2].hash, 0);
    assert_ne!(grid.col_signatures.as_ref().unwrap()[2].hash, 0);
}

#[test]
fn compute_all_signatures_on_empty_grid_produces_empty_vectors() {
    let mut grid = Grid::new(0, 0);

    grid.compute_all_signatures();

    assert!(grid.row_signatures.is_some());
    assert!(grid.col_signatures.is_some());
    assert!(grid.row_signatures.as_ref().unwrap().is_empty());
    assert!(grid.col_signatures.as_ref().unwrap().is_empty());
}

#[test]
fn compute_all_signatures_with_all_empty_rows_and_cols_is_stable() {
    let mut grid = Grid::new(3, 4);

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    assert_eq!(first_rows.len(), 3);
    assert_eq!(first_cols.len(), 4);
    assert!(first_rows.iter().all(|sig| sig.hash == 0));
    assert!(first_cols.iter().all(|sig| sig.hash == 0));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_eq!(first_rows, *second_rows);
    assert_eq!(first_cols, *second_cols);
}

#[test]
fn row_and_col_signatures_match_bulk_computation() {
    let mut grid = Grid::new(3, 2);
    grid.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert(make_cell(1, 1, Some(CellValue::Text("text".into())), None));
    grid.insert(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    for r in 0..3 {
        assert_eq!(
            grid.compute_row_signature(r).hash,
            row_sigs[r as usize].hash
        );
    }

    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");
    for c in 0..2 {
        assert_eq!(
            grid.compute_col_signature(c).hash,
            col_sigs[c as usize].hash
        );
    }
}

#[test]
fn compute_all_signatures_recomputes_after_mutation() {
    let mut grid = Grid::new(3, 3);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert(make_cell(1, 1, Some(CellValue::Text("x".into())), None));

    grid.compute_all_signatures();
    let first_rows = grid.row_signatures.as_ref().unwrap().clone();
    let first_cols = grid.col_signatures.as_ref().unwrap().clone();

    grid.insert(make_cell(1, 1, Some(CellValue::Text("y".into())), None));
    grid.insert(make_cell(2, 2, Some(CellValue::Bool(true)), None));

    grid.compute_all_signatures();
    let second_rows = grid.row_signatures.as_ref().unwrap();
    let second_cols = grid.col_signatures.as_ref().unwrap();

    assert_ne!(first_rows[1].hash, second_rows[1].hash);
    assert_ne!(first_cols[1].hash, second_cols[1].hash);
}

#[test]
fn row_signatures_distinguish_column_positions() {
    let mut grid1 = Grid::new(1, 2);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert(make_cell(0, 1, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(1, 2);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert(make_cell(0, 1, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_row_signature(0);
    let sig2 = grid2.compute_row_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn col_signatures_distinguish_row_positions() {
    let mut grid1 = Grid::new(2, 1);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid1.insert(make_cell(1, 0, Some(CellValue::Number(2.0)), None));

    let mut grid2 = Grid::new(2, 1);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(2.0)), None));
    grid2.insert(make_cell(1, 0, Some(CellValue::Number(1.0)), None));

    let sig1 = grid1.compute_col_signature(0);
    let sig2 = grid2.compute_col_signature(0);
    assert_ne!(sig1.hash, sig2.hash);
}

#[test]
fn row_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid1.insert(make_cell(0, 1, Some(CellValue::Text("mix".into())), None));
    grid1.insert(make_cell(0, 2, Some(CellValue::Bool(true)), None));

    let mut grid2 = Grid::new(1, 3);
    grid2.insert(make_cell(0, 2, Some(CellValue::Bool(true)), None));
    grid2.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(10.0)),
        Some("=A1*2"),
    ));
    grid2.insert(make_cell(0, 1, Some(CellValue::Text("mix".into())), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.row_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.row_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_independent_of_insertion_order() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid1.insert(make_cell(1, 0, Some(CellValue::Text("col".into())), None));
    grid1.insert(make_cell(2, 0, Some(CellValue::Bool(false)), None));

    let mut grid2 = Grid::new(3, 1);
    grid2.insert(make_cell(2, 0, Some(CellValue::Bool(false)), None));
    grid2.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::E)),
        Some("=EXP(1)"),
    ));
    grid2.insert(make_cell(1, 0, Some(CellValue::Text("col".into())), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);

    grid1.compute_all_signatures();
    grid2.compute_all_signatures();

    let bulk_sig1 = grid1.col_signatures.as_ref().unwrap()[0].hash;
    let bulk_sig2 = grid2.col_signatures.as_ref().unwrap()[0].hash;
    assert_eq!(bulk_sig1, bulk_sig2);
}

#[test]
fn col_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(3, 1);
    grid_num.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(3, 1);
    grid_text.insert(make_cell(0, 0, Some(CellValue::Text("1".into())), None));

    let mut grid_bool = Grid::new(3, 1);
    grid_bool.insert(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_col_signature(0).hash;
    let txt = grid_text.compute_col_signature(0).hash;
    let boo = grid_bool.compute_col_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_distinguishes_numeric_text_bool() {
    let mut grid_num = Grid::new(1, 1);
    grid_num.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));

    let mut grid_text = Grid::new(1, 1);
    grid_text.insert(make_cell(0, 0, Some(CellValue::Text("1".into())), None));

    let mut grid_bool = Grid::new(1, 1);
    grid_bool.insert(make_cell(0, 0, Some(CellValue::Bool(true)), None));

    let num = grid_num.compute_row_signature(0).hash;
    let txt = grid_text.compute_row_signature(0).hash;
    let boo = grid_bool.compute_row_signature(0).hash;

    assert_ne!(num, txt);
    assert_ne!(num, boo);
    assert_ne!(txt, boo);
}

#[test]
fn row_signature_ignores_empty_trailing_cells() {
    let mut grid1 = Grid::new(1, 3);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(1, 10);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_row_signature(0).hash;
    let sig2 = grid2.compute_row_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_ignores_empty_trailing_rows() {
    let mut grid1 = Grid::new(3, 1);
    grid1.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let mut grid2 = Grid::new(10, 1);
    grid2.insert(make_cell(0, 0, Some(CellValue::Number(42.0)), None));

    let sig1 = grid1.compute_col_signature(0).hash;
    let sig2 = grid2.compute_col_signature(0).hash;
    assert_eq!(sig1, sig2);
}

#[test]
fn col_signature_includes_formulas_by_default() {
    let mut with_formula = Grid::new(2, 1);
    with_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut without_formula = Grid::new(2, 1);
    without_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = with_formula.compute_col_signature(0).hash;
    let sig_without = without_formula.compute_col_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

#[test]
fn col_signature_includes_formulas_sparse() {
    let mut formula_short = Grid::new(5, 1);
    formula_short.insert(make_cell(
        0,
        0,
        Some(CellValue::Text("foo".into())),
        Some("=A2"),
    ));

    let mut formula_tall = Grid::new(10, 1);
    formula_tall.insert(make_cell(
        0,
        0,
        Some(CellValue::Text("foo".into())),
        Some("=A2"),
    ));

    let mut value_only = Grid::new(10, 1);
    value_only.insert(make_cell(0, 0, Some(CellValue::Text("foo".into())), None));

    let sig_formula_short = formula_short.compute_col_signature(0).hash;
    let sig_formula_tall = formula_tall.compute_col_signature(0).hash;
    let sig_value_only = value_only.compute_col_signature(0).hash;

    assert_eq!(sig_formula_short, sig_formula_tall);
    assert_ne!(sig_formula_short, sig_value_only);
}

#[test]
fn row_signature_includes_formulas_by_default() {
    let mut grid_with_formula = Grid::new(1, 1);
    grid_with_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));

    let mut grid_without_formula = Grid::new(1, 1);
    grid_without_formula.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), None));

    let sig_with = grid_with_formula.compute_row_signature(0).hash;
    let sig_without = grid_without_formula.compute_row_signature(0).hash;
    assert_ne!(sig_with, sig_without);
}

const ROW_SIGNATURE_GOLDEN: u64 = 13_315_384_008_147_106_509;
const ROW_SIGNATURE_WITH_FORMULA_GOLDEN: u64 = 3_920_348_561_402_334_617;

#[test]
fn row_signature_golden_constant_small_grid() {
    let mut grid = Grid::new(1, 3);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(1.0)), None));
    grid.insert(make_cell(0, 1, Some(CellValue::Text("x".into())), None));
    grid.insert(make_cell(0, 2, Some(CellValue::Bool(false)), None));

    let sig = grid.compute_row_signature(0);
    assert_eq!(sig.hash, ROW_SIGNATURE_GOLDEN);
}

#[test]
fn row_signature_golden_constant_with_formula() {
    let mut grid = Grid::new(1, 2);
    grid.insert(make_cell(0, 0, Some(CellValue::Number(10.0)), Some("=5+5")));
    grid.insert(make_cell(0, 1, Some(CellValue::Text("bar".into())), None));

    let sig = grid.compute_row_signature(0);
    assert_eq!(sig.hash, ROW_SIGNATURE_WITH_FORMULA_GOLDEN);
}

#[test]
fn gridview_rowmeta_hash_matches_compute_all_signatures() {
    let mut grid = Grid::new(3, 2);
    grid.insert(make_cell(
        0,
        0,
        Some(CellValue::Number(std::f64::consts::PI)),
        Some("=PI()"),
    ));
    grid.insert(make_cell(1, 1, Some(CellValue::Text("text".into())), None));
    grid.insert(make_cell(2, 0, Some(CellValue::Bool(true)), Some("=A1")));

    grid.compute_all_signatures();

    let row_signatures = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should be computed")
        .clone();
    let col_signatures = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should be computed")
        .clone();

    let view = GridView::from_grid(&grid);

    for (idx, meta) in view.row_meta.iter().enumerate() {
        assert_eq!(meta.hash, row_signatures[idx].hash);
    }

    for (idx, meta) in view.col_meta.iter().enumerate() {
        assert_eq!(meta.hash, col_signatures[idx].hash);
    }
}
```

---

### File: `core\tests\sparse_grid_tests.rs`

```rust
use excel_diff::{Cell, CellAddress, CellValue, Grid};

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

#[test]
fn rows_iter_covers_all_rows() {
    let grid = Grid::new(3, 5);
    let rows: Vec<u32> = grid.rows_iter().collect();
    assert_eq!(rows, vec![0, 1, 2]);
}

#[test]
fn cols_iter_covers_all_cols() {
    let grid = Grid::new(4, 2);
    let cols: Vec<u32> = grid.cols_iter().collect();
    assert_eq!(cols, vec![0, 1]);
}

#[test]
fn rows_iter_and_get_are_consistent() {
    let mut grid = Grid::new(2, 2);
    grid.insert(Cell {
        row: 1,
        col: 1,
        address: CellAddress::from_indices(1, 1),
        value: Some(CellValue::Number(1.0)),
        formula: None,
    });

    for r in grid.rows_iter() {
        for c in grid.cols_iter() {
            let _ = grid.get(r, c);
        }
    }
}

#[test]
fn sparse_grid_all_empty_rows_have_zero_signatures() {
    let mut grid = Grid::new(2, 3);

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(row_sigs.len(), 2);
    assert_eq!(col_sigs.len(), 3);
    assert!(row_sigs.iter().all(|sig| sig.hash == 0));
    assert!(col_sigs.iter().all(|sig| sig.hash == 0));
}

#[test]
fn compute_signatures_on_sparse_grid_produces_hashes() {
    let mut grid = Grid::new(4, 4);
    grid.insert(Cell {
        row: 1,
        col: 3,
        address: CellAddress::from_indices(1, 3),
        value: Some(CellValue::Text("value".into())),
        formula: Some("=A1".into()),
    });

    grid.compute_all_signatures();

    let row_hash = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist")[1]
        .hash;
    let col_hash = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist")[3]
        .hash;

    assert_ne!(row_hash, 0);
    assert_ne!(col_hash, 0);
}

#[test]
fn compute_all_signatures_matches_direct_computation() {
    let mut grid = Grid::new(3, 3);
    grid.insert(Cell {
        row: 0,
        col: 1,
        address: CellAddress::from_indices(0, 1),
        value: Some(CellValue::Number(10.0)),
        formula: Some("=5+5".into()),
    });
    grid.insert(Cell {
        row: 1,
        col: 0,
        address: CellAddress::from_indices(1, 0),
        value: Some(CellValue::Text("x".into())),
        formula: None,
    });
    grid.insert(Cell {
        row: 2,
        col: 2,
        address: CellAddress::from_indices(2, 2),
        value: Some(CellValue::Bool(false)),
        formula: Some("=A1".into()),
    });

    grid.compute_all_signatures();

    let row_sigs = grid
        .row_signatures
        .as_ref()
        .expect("row signatures should exist");
    let col_sigs = grid
        .col_signatures
        .as_ref()
        .expect("col signatures should exist");

    assert_eq!(grid.compute_row_signature(0).hash, row_sigs[0].hash);
    assert_eq!(grid.compute_row_signature(2).hash, row_sigs[2].hash);
    assert_eq!(grid.compute_col_signature(0).hash, col_sigs[0].hash);
    assert_eq!(grid.compute_col_signature(2).hash, col_sigs[2].hash);
}
```

---

### File: `core\tests\common\mod.rs`

```rust
//! Common test utilities shared across integration tests.

#![allow(dead_code)]

use excel_diff::{Cell, CellAddress, CellValue, Grid, Sheet, SheetKind, Workbook};
use std::path::PathBuf;

pub fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path.push(filename);
    path
}

pub fn grid_from_numbers(values: &[&[i32]]) -> Grid {
    let nrows = values.len() as u32;
    let ncols = if nrows == 0 {
        0
    } else {
        values[0].len() as u32
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, row_vals) in values.iter().enumerate() {
        for (c, v) in row_vals.iter().enumerate() {
            grid.insert(Cell {
                row: r as u32,
                col: c as u32,
                address: CellAddress::from_indices(r as u32, c as u32),
                value: Some(CellValue::Number(*v as f64)),
                formula: None,
            });
        }
    }

    grid
}

pub fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook {
    Workbook {
        sheets: vec![Sheet {
            name: name.to_string(),
            kind: SheetKind::Worksheet,
            grid,
        }],
    }
}
```

---

### File: `fixtures\manifest.yaml`

```yaml
scenarios:
  # --- Phase 1.1: Basic File Opening ---
  - id: "smoke_minimal"
    generator: "basic_grid"
    args: { rows: 1, cols: 1 }
    output: "minimal.xlsx"

  # --- Phase 1.2: Is this a ZIP? ---
  - id: "container_random_zip"
    generator: "corrupt_container"
    args: { mode: "random_zip" }
    output: "random_zip.zip"
    
  - id: "container_no_content_types"
    generator: "corrupt_container"
    args: { mode: "no_content_types" }
    output: "no_content_types.xlsx"

  - id: "container_not_zip_text"
    generator: "corrupt_container"
    args: { mode: "not_zip_text" }
    output: "not_a_zip.txt"

  # --- PG1: Workbook -> Sheet -> Grid IR sanity ---
  - id: "pg1_basic_two_sheets"
    generator: "basic_grid"
    args: { rows: 3, cols: 3, two_sheets: true } # Sheet1 3x3, Sheet2 5x2 (logic in generator)
    output: "pg1_basic_two_sheets.xlsx"

  - id: "pg1_sparse"
    generator: "sparse_grid"
    output: "pg1_sparse_used_range.xlsx"

  - id: "pg1_mixed"
    generator: "edge_case"
    output: "pg1_empty_and_mixed_sheets.xlsx"

  # --- PG2: Addressing and index invariants ---
  - id: "pg2_addressing"
    generator: "address_sanity"
    args:
      targets: ["A1", "B2", "C3", "Z1", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1"]
    output: "pg2_addressing_matrix.xlsx"

  # --- PG3: Cell snapshots and comparison semantics ---
  - id: "pg3_types"
    generator: "value_formula"
    output: "pg3_value_and_formula_cells.xlsx"

  # --- Phase 3: Spreadsheet-mode G1/G2 ---
  - id: "g1_equal_sheet"
    generator: "basic_grid"
    args:
      rows: 5
      cols: 5
      sheet: "Sheet1"
    output:
      - "equal_sheet_a.xlsx"
      - "equal_sheet_b.xlsx"

  - id: "g2_single_cell_value"
    generator: "single_cell_diff"
    args:
      rows: 5
      cols: 5
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: 1.0
      value_b: 2.0
    output:
      - "single_cell_value_a.xlsx"
      - "single_cell_value_b.xlsx"

  # --- Phase 3: Spreadsheet-mode G5-G7 ---

  - id: "g5_multi_cell_edits"
    generator: "multi_cell_diff"
    args:
      rows: 20
      cols: 10
      sheet: "Sheet1"
      edits:
        - { addr: "B2", value_a: 1.0, value_b: 42.0 }
        - { addr: "D5", value_a: 2.0, value_b: 99.0 }
        - { addr: "H7", value_a: 3.0, value_b: 3.5 }
        - { addr: "J10", value_a: "x", value_b: "y" }
    output:
      - "multi_cell_edits_a.xlsx"
      - "multi_cell_edits_b.xlsx"

  - id: "g6_row_append_bottom"
    generator: "grid_tail_diff"
    args:
      mode: "row_append_bottom"
      sheet: "Sheet1"
      base_rows: 10
      tail_rows: 2
    output:
      - "row_append_bottom_a.xlsx"
      - "row_append_bottom_b.xlsx"

  - id: "g6_row_delete_bottom"
    generator: "grid_tail_diff"
    args:
      mode: "row_delete_bottom"
      sheet: "Sheet1"
      base_rows: 10
      tail_rows: 2
    output:
      - "row_delete_bottom_a.xlsx"
      - "row_delete_bottom_b.xlsx"

  - id: "g7_col_append_right"
    generator: "grid_tail_diff"
    args:
      mode: "col_append_right"
      sheet: "Sheet1"
      base_cols: 4
      tail_cols: 2
    output:
      - "col_append_right_a.xlsx"
      - "col_append_right_b.xlsx"

  - id: "g7_col_delete_right"
    generator: "grid_tail_diff"
    args:
      mode: "col_delete_right"
      sheet: "Sheet1"
      base_cols: 4
      tail_cols: 2
    output:
      - "col_delete_right_a.xlsx"
      - "col_delete_right_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G8 ---
  - id: "g8_row_insert_middle"
    generator: "row_alignment_g8"
    args:
      mode: "insert"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      insert_at: 6
    output:
      - "row_insert_middle_a.xlsx"
      - "row_insert_middle_b.xlsx"

  - id: "g8_row_delete_middle"
    generator: "row_alignment_g8"
    args:
      mode: "delete"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      delete_row: 6
    output:
      - "row_delete_middle_a.xlsx"
      - "row_delete_middle_b.xlsx"

  - id: "g8_row_insert_with_edit_below"
    generator: "row_alignment_g8"
    args:
      mode: "insert_with_edit"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      insert_at: 6
      edit_row: 8
      edit_col: 3
    output:
      - "row_insert_with_edit_a.xlsx"
      - "row_insert_with_edit_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G9 ---
  - id: "g9_col_insert_middle"
    generator: "column_alignment_g9"
    args:
      mode: "insert"
      sheet: "Data"
      cols: 8
      data_rows: 9
      insert_at: 4
    output:
      - "col_insert_middle_a.xlsx"
      - "col_insert_middle_b.xlsx"

  - id: "g9_col_delete_middle"
    generator: "column_alignment_g9"
    args:
      mode: "delete"
      sheet: "Data"
      cols: 8
      data_rows: 9
      delete_col: 4
    output:
      - "col_delete_middle_a.xlsx"
      - "col_delete_middle_b.xlsx"

  - id: "g9_col_insert_with_edit"
    generator: "column_alignment_g9"
    args:
      mode: "insert_with_edit"
      sheet: "Data"
      cols: 8
      data_rows: 9
      insert_at: 4
      edit_row: 8
      edit_col_after_insert: 7
    output:
      - "col_insert_with_edit_a.xlsx"
      - "col_insert_with_edit_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G10 ---
  - id: "g10_row_block_insert"
    generator: "row_alignment_g10"
    args:
      mode: "block_insert"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      block_rows: 4
      insert_at: 4
    output:
      - "row_block_insert_a.xlsx"
      - "row_block_insert_b.xlsx"

  - id: "g10_row_block_delete"
    generator: "row_alignment_g10"
    args:
      mode: "block_delete"
      sheet: "Sheet1"
      base_rows: 10
      cols: 5
      block_rows: 4
      delete_start: 4
    output:
      - "row_block_delete_a.xlsx"
      - "row_block_delete_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G11 ---
  - id: "g11_row_block_move"
    generator: "row_block_move_g11"
    args:
      sheet: "Sheet1"
      total_rows: 20
      cols: 5
      block_rows: 4
      src_start: 5    # 1-based in A
      dst_start: 13   # 1-based in B
    output:
      - "row_block_move_a.xlsx"
      - "row_block_move_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G12 (column move only - G12a) ---
  - id: "g12_column_block_move"
    generator: "column_move_g12"
    args:
      sheet: "Data"
      cols: 8
      data_rows: 9
      src_col: 3      # 1-based: C
      dst_col: 6      # 1-based: F
    output:
      - "column_move_a.xlsx"
      - "column_move_b.xlsx"

  - id: "g12_rect_block_move"
    generator: "rect_block_move_g12"
    args:
      sheet: "Data"
      rows: 15
      cols: 15
      src_top: 3      # 1-based row in A (Excel row 3)
      src_left: 2     # 1-based column in A (Excel column B)
      dst_top: 10     # 1-based row in B (Excel row 10)
      dst_left: 7     # 1-based column in B (Excel column G)
      block_rows: 3
      block_cols: 3
    output:
      - "rect_block_move_a.xlsx"
      - "rect_block_move_b.xlsx"

  # --- Phase 4: Spreadsheet-mode G13 ---
  - id: "g13_fuzzy_row_move"
    generator: "row_fuzzy_move_g13"
    args:
      sheet: "Data"
      total_rows: 24
      cols: 6
      block_rows: 4
      src_start: 5      # 1-based in A
      dst_start: 14     # 1-based in B
      edits:
        - { row_offset: 1, col: 3, delta: 1 }
    output:
      - "grid_move_and_edit_a.xlsx"
      - "grid_move_and_edit_b.xlsx"

  # --- JSON diff: simple non-empty change ---
  - id: "json_diff_single_cell"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: "1"
      value_b: "2"
    output:
      - "json_diff_single_cell_a.xlsx"
      - "json_diff_single_cell_b.xlsx"

  - id: "json_diff_single_bool"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: true
      value_b: false
    output:
      - "json_diff_bool_a.xlsx"
      - "json_diff_bool_b.xlsx"

  - id: "json_diff_value_to_empty"
    generator: "single_cell_diff"
    args:
      rows: 3
      cols: 3
      sheet: "Sheet1"
      target_cell: "C3"
      value_a: "1"
      value_b: null
    output:
      - "json_diff_value_to_empty_a.xlsx"
      - "json_diff_value_to_empty_b.xlsx"

  # --- Sheet identity: case-only renames ---
  - id: "sheet_case_only_rename"
    generator: "sheet_case_rename"
    args:
      sheet_a: "Sheet1"
      sheet_b: "sheet1"
      cell: "A1"
      value_a: 1.0
      value_b: 1.0
    output:
      - "sheet_case_only_rename_a.xlsx"
      - "sheet_case_only_rename_b.xlsx"

  - id: "sheet_case_only_rename_cell_edit"
    generator: "sheet_case_rename"
    args:
      sheet_a: "Sheet1"
      sheet_b: "sheet1"
      cell: "A1"
      value_a: 1.0
      value_b: 2.0
    output:
      - "sheet_case_only_rename_edit_a.xlsx"
      - "sheet_case_only_rename_edit_b.xlsx"

  # --- PG6: Object graph vs grid responsibilities ---
  - id: "pg6_sheet_added"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_added"
    output:
      - "pg6_sheet_added_a.xlsx"
      - "pg6_sheet_added_b.xlsx"

  - id: "pg6_sheet_removed"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_removed"
    output:
      - "pg6_sheet_removed_a.xlsx"
      - "pg6_sheet_removed_b.xlsx"

  - id: "pg6_sheet_renamed"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_renamed"
    output:
      - "pg6_sheet_renamed_a.xlsx"
      - "pg6_sheet_renamed_b.xlsx"

  - id: "pg6_sheet_and_grid_change"
    generator: "pg6_sheet_scenario"
    args:
      mode: "sheet_and_grid_change"
    output:
      - "pg6_sheet_and_grid_change_a.xlsx"
      - "pg6_sheet_and_grid_change_b.xlsx"

  # --- Milestone 2.2: Base64 Correctness ---
  - id: "corrupt_base64"
    generator: "mashup_corrupt"
    args: 
      base_file: "templates/base_query.xlsx"
      mode: "byte_flip"
    output: "corrupt_base64.xlsx"

  - id: "duplicate_datamashup_parts"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
    output: "duplicate_datamashup_parts.xlsx"

  - id: "duplicate_datamashup_elements"
    generator: "mashup_duplicate"
    args:
      base_file: "templates/base_query.xlsx"
      mode: "element"
    output: "duplicate_datamashup_elements.xlsx"

  - id: "mashup_utf16_le"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-le"
    output: "mashup_utf16_le.xlsx"

  - id: "mashup_utf16_be"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      encoding: "utf-16-be"
    output: "mashup_utf16_be.xlsx"

  - id: "mashup_base64_whitespace"
    generator: "mashup_encode"
    args:
      base_file: "templates/base_query.xlsx"
      whitespace: true
    output: "mashup_base64_whitespace.xlsx"

  # --- Milestone 4.1: PackageParts ---
  - id: "m4_packageparts_one_query"
    generator: "mashup:one_query"
    args:
      base_file: "templates/base_query.xlsx"
    output: "one_query.xlsx"

  - id: "m4_packageparts_multi_embedded"
    generator: "mashup:multi_query_with_embedded"
    args:
      base_file: "templates/base_query.xlsx"
    output: "multi_query_with_embedded.xlsx"

  # --- Milestone 4.2-4.4: Permissions / Metadata ---
  - id: "permissions_defaults"
    generator: "mashup:permissions_metadata"
    args:
      mode: "permissions_defaults"
      base_file: "templates/base_query.xlsx"
    output: "permissions_defaults.xlsx"

  - id: "permissions_firewall_off"
    generator: "mashup:permissions_metadata"
    args:
      mode: "permissions_firewall_off"
      base_file: "templates/base_query.xlsx"
    output: "permissions_firewall_off.xlsx"

  - id: "metadata_simple"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_simple"
      base_file: "templates/base_query.xlsx"
    output: "metadata_simple.xlsx"

  - id: "metadata_query_groups"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_query_groups"
      base_file: "templates/base_query.xlsx"
    output: "metadata_query_groups.xlsx"

  - id: "metadata_hidden_queries"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_hidden_queries"
      base_file: "templates/base_query.xlsx"
    output: "metadata_hidden_queries.xlsx"

  - id: "metadata_missing_entry"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_missing_entry"
      base_file: "templates/base_query.xlsx"
    output: "metadata_missing_entry.xlsx"

  - id: "metadata_url_encoding"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_url_encoding"
      base_file: "templates/base_query.xlsx"
    output: "metadata_url_encoding.xlsx"

  - id: "metadata_orphan_entries"
    generator: "mashup:permissions_metadata"
    args:
      mode: "metadata_orphan_entries"
      base_file: "templates/base_query.xlsx"
    output: "metadata_orphan_entries.xlsx"

  # --- Milestone 6: Basic M Diffs ---
  - id: "m_add_query_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_add_query_a"
      base_file: "templates/base_query.xlsx"
    output: "m_add_query_a.xlsx"

  - id: "m_add_query_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_add_query_b"
      base_file: "templates/base_query.xlsx"
    output: "m_add_query_b.xlsx"

  - id: "m_remove_query_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_remove_query_a"
      base_file: "templates/base_query.xlsx"
    output: "m_remove_query_a.xlsx"

  - id: "m_remove_query_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_remove_query_b"
      base_file: "templates/base_query.xlsx"
    output: "m_remove_query_b.xlsx"

  - id: "m_change_literal_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_change_literal_a"
      base_file: "templates/base_query.xlsx"
    output: "m_change_literal_a.xlsx"

  - id: "m_change_literal_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_change_literal_b"
      base_file: "templates/base_query.xlsx"
    output: "m_change_literal_b.xlsx"

  - id: "m_metadata_only_change_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_metadata_only_change_a"
      base_file: "templates/base_query.xlsx"
    output: "m_metadata_only_change_a.xlsx"

  - id: "m_metadata_only_change_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_metadata_only_change_b"
      base_file: "templates/base_query.xlsx"
    output: "m_metadata_only_change_b.xlsx"

  - id: "m_def_and_metadata_change_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_def_and_metadata_change_a"
      base_file: "templates/base_query.xlsx"
    output: "m_def_and_metadata_change_a.xlsx"

  - id: "m_def_and_metadata_change_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_def_and_metadata_change_b"
      base_file: "templates/base_query.xlsx"
    output: "m_def_and_metadata_change_b.xlsx"

  - id: "m_rename_query_a"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_rename_query_a"
      base_file: "templates/base_query.xlsx"
    output: "m_rename_query_a.xlsx"

  - id: "m_rename_query_b"
    generator: "mashup:permissions_metadata"
    args:
      mode: "m_rename_query_b"
      base_file: "templates/base_query.xlsx"
    output: "m_rename_query_b.xlsx"

  # --- P1: Large Dense Grid (Performance Baseline) ---
  - id: "p1_large_dense"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "dense" # Deterministic "R1C1" style data
    output: "grid_large_dense.xlsx"

  # --- P2: Large Noise Grid (Worst-case Alignment) ---
  - id: "p2_large_noise"
    generator: "perf_large"
    args: 
      rows: 50000 
      cols: 20
      mode: "noise" # Random float data
      seed: 12345
    output: "grid_large_noise.xlsx"

  # --- D1: Keyed Equality (Database Mode) ---
  # File A: Ordered IDs 1..1000
  - id: "db_equal_ordered_a"
    generator: "db_keyed"
    args: { count: 1000, shuffle: false, seed: 42 }
    output: "db_equal_ordered_a.xlsx"

  # File B: Same data, random order (Tests O(N) alignment)
  - id: "db_equal_ordered_b"
    generator: "db_keyed"
    args: { count: 1000, shuffle: true, seed: 42 }
    output: "db_equal_ordered_b.xlsx"

  # --- D2: Row Added (Database Mode) ---
  - id: "db_row_added_b"
    generator: "db_keyed"
    args: 
      count: 1000 
      seed: 42 
      # Inject a new ID at the end
      extra_rows: [{id: 1001, name: "New Row", amount: 999}]
    output: "db_row_added_b.xlsx"
```

---

### File: `fixtures\pyproject.toml`

```yaml
[project]
name = "excel-fixtures"
version = "0.1.0"
description = "Deterministic artifact generator for Excel Diff testing"
readme = "README.md"
requires-python = ">=3.9"
dependencies = [
    "openpyxl>=3.1.0",
    "lxml>=4.9.0",
    "jinja2>=3.1.0",
    "pyyaml>=6.0",
]

[project.scripts]
generate-fixtures = "src.generate:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src"]

```

---

### File: `fixtures\src\generate.py`

```python
import argparse
import yaml
import sys
from pathlib import Path
from typing import Dict, Any, List

# Import generators
from generators.grid import (
    BasicGridGenerator, 
    SparseGridGenerator, 
    EdgeCaseGenerator, 
    AddressSanityGenerator,
    ValueFormulaGenerator,
    SingleCellDiffGenerator,
    MultiCellDiffGenerator,
    GridTailDiffGenerator,
    RowAlignmentG8Generator,
    RowAlignmentG10Generator,
    RowBlockMoveG11Generator,
    RowFuzzyMoveG13Generator,
    ColumnMoveG12Generator,
    RectBlockMoveG12Generator,
    ColumnAlignmentG9Generator,
    SheetCaseRenameGenerator,
    Pg6SheetScenarioGenerator,
)
from generators.corrupt import ContainerCorruptGenerator
from generators.mashup import (
    MashupCorruptGenerator,
    MashupDuplicateGenerator,
    MashupInjectGenerator,
    MashupEncodeGenerator,
    MashupMultiEmbeddedGenerator,
    MashupOneQueryGenerator,
    MashupPermissionsMetadataGenerator,
)
from generators.perf import LargeGridGenerator
from generators.database import KeyedTableGenerator

# Registry of generators
GENERATORS: Dict[str, Any] = {
    "basic_grid": BasicGridGenerator,
    "sparse_grid": SparseGridGenerator,
    "edge_case": EdgeCaseGenerator,
    "address_sanity": AddressSanityGenerator,
    "value_formula": ValueFormulaGenerator,
    "single_cell_diff": SingleCellDiffGenerator,
    "multi_cell_diff": MultiCellDiffGenerator,
    "grid_tail_diff": GridTailDiffGenerator,
    "row_alignment_g8": RowAlignmentG8Generator,
    "row_alignment_g10": RowAlignmentG10Generator,
    "row_block_move_g11": RowBlockMoveG11Generator,
    "row_fuzzy_move_g13": RowFuzzyMoveG13Generator,
    "column_move_g12": ColumnMoveG12Generator,
    "rect_block_move_g12": RectBlockMoveG12Generator,
    "column_alignment_g9": ColumnAlignmentG9Generator,
    "sheet_case_rename": SheetCaseRenameGenerator,
    "pg6_sheet_scenario": Pg6SheetScenarioGenerator,
    "corrupt_container": ContainerCorruptGenerator,
    "mashup_corrupt": MashupCorruptGenerator,
    "mashup_duplicate": MashupDuplicateGenerator,
    "mashup_inject": MashupInjectGenerator,
    "mashup_encode": MashupEncodeGenerator,
    "mashup:one_query": MashupOneQueryGenerator,
    "mashup:multi_query_with_embedded": MashupMultiEmbeddedGenerator,
    "mashup:permissions_metadata": MashupPermissionsMetadataGenerator,
    "perf_large": LargeGridGenerator,
    "db_keyed": KeyedTableGenerator,
}

def load_manifest(manifest_path: Path) -> Dict[str, Any]:
    if not manifest_path.exists():
        print(f"Error: Manifest file not found at {manifest_path}")
        sys.exit(1)
    
    with open(manifest_path, 'r') as f:
        try:
            return yaml.safe_load(f)
        except yaml.YAMLError as e:
            print(f"Error parsing manifest: {e}")
            sys.exit(1)

def ensure_output_dir(output_dir: Path):
    output_dir.mkdir(parents=True, exist_ok=True)

def main():
    script_dir = Path(__file__).parent.resolve()
    fixtures_root = script_dir.parent
    
    default_manifest = fixtures_root / "manifest.yaml"
    default_output = fixtures_root / "generated"

    parser = argparse.ArgumentParser(description="Generate Excel fixtures based on a manifest.")
    parser.add_argument("--manifest", type=Path, default=default_manifest, help="Path to the manifest YAML file.")
    parser.add_argument("--output-dir", type=Path, default=default_output, help="Directory to output generated files.")
    parser.add_argument("--force", action="store_true", help="Force regeneration of existing files.")
    
    args = parser.parse_args()
    
    manifest = load_manifest(args.manifest)
    ensure_output_dir(args.output_dir)
    
    scenarios = manifest.get('scenarios', [])
    print(f"Found {len(scenarios)} scenarios in manifest.")
    
    for scenario in scenarios:
        scenario_id = scenario.get('id')
        generator_name = scenario.get('generator')
        generator_args = scenario.get('args', {})
        outputs = scenario.get('output')
        
        if not scenario_id or not generator_name or not outputs:
            print(f"Skipping invalid scenario: {scenario}")
            continue
            
        print(f"Processing scenario: {scenario_id} (Generator: {generator_name})")
        
        if generator_name not in GENERATORS:
            print(f"  Warning: Generator '{generator_name}' not implemented yet. Skipping.")
            continue
        
        try:
            generator_class = GENERATORS[generator_name]
            generator = generator_class(generator_args)
            generator.generate(args.output_dir, outputs)
            print(f"  Success: Generated {outputs}")
        except Exception as e:
            print(f"  Error generating scenario {scenario_id}: {e}")
            import traceback
            traceback.print_exc()

if __name__ == "__main__":
    main()
```

---

### File: `fixtures\src\__init__.py`

```python

```

---

### File: `fixtures\src\generators\base.py`

```python
"""Base classes for fixture generators."""

from abc import ABC, abstractmethod
from pathlib import Path
from typing import Dict, Any, Union, List


class BaseGenerator(ABC):
    """Abstract base class for all fixture generators."""

    def __init__(self, args: Dict[str, Any]):
        self.args = args

    @abstractmethod
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        """Generate the fixture file(s).

        Args:
            output_dir: The directory to save the file(s) in.
            output_names: The name(s) of the output file(s) as specified in the manifest.
        """
        pass
```

---

### File: `fixtures\src\generators\corrupt.py`

```python
import zipfile
import io
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class ContainerCorruptGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        mode = self.args.get('mode', 'no_content_types')
        
        for name in output_names:
            # Create a dummy zip
            out_path = output_dir / name
            
            if mode == 'random_zip':
                # Just a zip with a text file
                with zipfile.ZipFile(out_path, 'w') as z:
                    z.writestr("hello.txt", "This is not excel")
                    
            elif mode == 'no_content_types':
                # Create a valid excel in memory, then strip [Content_Types].xml
                buffer = io.BytesIO()
                import openpyxl
                wb = openpyxl.Workbook()
                # Add some content just so it's not totally empty
                wb.active['A1'] = 1
                wb.save(buffer)
                buffer.seek(0)
                
                with zipfile.ZipFile(buffer, 'r') as zin:
                    with zipfile.ZipFile(out_path, 'w') as zout:
                        for item in zin.infolist():
                            if item.filename != "[Content_Types].xml":
                                zout.writestr(item, zin.read(item.filename))
            elif mode == 'not_zip_text':
                out_path.write_text("This is not a zip container", encoding="utf-8")
            else:
                raise ValueError(f"Unsupported corrupt_container mode: {mode}")

```

---

### File: `fixtures\src\generators\database.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class KeyedTableGenerator(BaseGenerator):
    """
    Generates datasets with Primary Keys (ID columns).
    Capable of shuffling rows to test O(N) alignment (Database Mode).
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        count = self.args.get('count', 100)
        shuffle = self.args.get('shuffle', False)
        seed = self.args.get('seed', 42)
        extra_rows = self.args.get('extra_rows', [])

        # Use deterministic seed
        rng = random.Random(seed)

        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Data"

            # 1. Define Base Data (List of Dicts)
            # Schema: [ID, Name, Amount, Category]
            data_rows = []
            for i in range(1, count + 1):
                data_rows.append({
                    'id': i,
                    'name': f"Customer_{i}",
                    'amount': i * 10.5,
                    'category': rng.choice(['A', 'B', 'C'])
                })

            # 2. Apply Mutations (Additions)
            # This allows us to inject specific "diffs" like D2 (Row Added)
            for row in extra_rows:
                data_rows.append(row)

            # 3. Apply Shuffle (The core D1 test)
            if shuffle:
                rng.shuffle(data_rows)

            # 4. Write to Sheet
            # Header
            headers = ['ID', 'Name', 'Amount', 'Category']
            ws.append(headers)

            for row in data_rows:
                # Ensure strictly ordered list matching headers
                ws.append([
                    row.get('id'),
                    row.get('name'),
                    row.get('amount'),
                    row.get('category')
                ])

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\grid.py`

```python
import openpyxl
import zipfile
import xml.etree.ElementTree as ET
from openpyxl.utils import get_column_letter
from pathlib import Path
from typing import Union, List, Dict, Any
from .base import BaseGenerator

class BasicGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        rows = self.args.get('rows', 5)
        cols = self.args.get('cols', 5)
        two_sheets = self.args.get('two_sheets', False)
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sheet1"
            
            # Fill grid
            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")
            
            # Check if we need a second sheet
            if two_sheets:
                ws2 = wb.create_sheet(title="Sheet2")
                # Different dimensions for Sheet2 (PG1 requirement: 5x2)
                # If args are customized we might need more logic, but for PG1 this is sufficient or we use defaults
                s2_rows = 5
                s2_cols = 2
                for r in range(1, s2_rows + 1):
                    for c in range(1, s2_cols + 1):
                         ws2.cell(row=r, column=c, value=f"S2_R{r}C{c}")

            wb.save(output_dir / name)

class SparseGridGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Sparse"
            
            # Specifics for pg1_sparse_used_range
            ws['A1'] = "A1"
            ws['B2'] = "B2"
            ws['G10'] = "G10" # Forces extent
            # Row 5 and Col D are empty implicitly by not writing to them
            
            wb.save(output_dir / name)

class EdgeCaseGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
        
        for name in output_names:
            wb = openpyxl.Workbook()
            # Remove default sheet
            default_ws = wb.active
            wb.remove(default_ws)
            
            # Empty Sheet
            wb.create_sheet("Empty")
            
            # Values Only
            ws_val = wb.create_sheet("ValuesOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    ws_val.cell(row=r, column=c, value=r*c)
            
            # Formulas Only
            ws_form = wb.create_sheet("FormulasOnly")
            for r in range(1, 11):
                for c in range(1, 11):
                    # Reference ValuesOnly sheet
                    col_letter = get_column_letter(c)
                    ws_form.cell(row=r, column=c, value=f"=ValuesOnly!{col_letter}{r}")
            
            wb.save(output_dir / name)

class AddressSanityGenerator(BaseGenerator):
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        targets = self.args.get('targets', ["A1", "B2", "Z10"])
        
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Addresses"
            
            for addr in targets:
                ws[addr] = addr
                
            wb.save(output_dir / name)

class ValueFormulaGenerator(BaseGenerator):
    """PG3: Types, formulas, values"""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        for name in output_names:
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = "Types"
            
            ws['A1'] = 42
            ws['A2'] = "hello"
            ws['A3'] = True
            # A4 empty
            
            ws['B1'] = "=A1+1"
            ws['B2'] = '="hello" & " world"'
            ws['B3'] = "=A1>0"
            
            output_path = output_dir / name
            wb.save(output_path)
            self._inject_formula_caches(output_path)

    def _inject_formula_caches(self, path: Path):
        ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
        with zipfile.ZipFile(path, "r") as zf:
            sheet_xml = zf.read("xl/worksheets/sheet1.xml")
            other_files = {
                info.filename: zf.read(info.filename)
                for info in zf.infolist()
                if info.filename != "xl/worksheets/sheet1.xml"
            }

        root = ET.fromstring(sheet_xml)

        def update_cell(ref: str, value: str, cell_type: str | None = None):
            cell = root.find(f".//{{{ns}}}c[@r='{ref}']")
            if cell is None:
                return
            if cell_type:
                cell.set("t", cell_type)
            v = cell.find(f"{{{ns}}}v")
            if v is None:
                v = ET.SubElement(cell, f"{{{ns}}}v")
            v.text = value

        update_cell("B1", "43")
        update_cell("B2", "hello world", "str")
        update_cell("B3", "1", "b")

        ET.register_namespace("", ns)
        updated_sheet = ET.tostring(root, encoding="utf-8", xml_declaration=False)
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("xl/worksheets/sheet1.xml", updated_sheet)
            for name, data in other_files.items():
                zf.writestr(name, data)

class SingleCellDiffGenerator(BaseGenerator):
    """Generates a tiny pair of workbooks with a single differing cell."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("single_cell_diff generator expects exactly two output filenames")

        rows = self.args.get('rows', 3)
        cols = self.args.get('cols', 3)
        sheet = self.args.get('sheet', "Sheet1")
        target_cell = self.args.get('target_cell', "C3")
        value_a = self.args.get('value_a', "1")
        value_b = self.args.get('value_b', "2")

        def create_workbook(value, name: str):
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = sheet

            for r in range(1, rows + 1):
                for c in range(1, cols + 1):
                    ws.cell(row=r, column=c, value=f"R{r}C{c}")

            ws[target_cell] = value
            wb.save(output_dir / name)

        create_workbook(value_a, output_names[0])
        create_workbook(value_b, output_names[1])

class MultiCellDiffGenerator(BaseGenerator):
    """Generates workbook pairs that differ in multiple scattered cells."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("multi_cell_diff generator expects exactly two output filenames")

        rows = self.args.get("rows", 20)
        cols = self.args.get("cols", 10)
        sheet = self.args.get("sheet", "Sheet1")
        edits: List[Dict[str, Any]] = self.args.get("edits", [])

        self._create_workbook(output_dir / output_names[0], sheet, rows, cols, edits, "a")
        self._create_workbook(output_dir / output_names[1], sheet, rows, cols, edits, "b")

    def _create_workbook(
        self,
        path: Path,
        sheet: str,
        rows: int,
        cols: int,
        edits: List[Dict[str, Any]],
        value_key: str,
    ):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        self._fill_base_grid(ws, rows, cols)
        self._apply_edits(ws, edits, value_key)

        wb.save(path)

    def _fill_base_grid(self, ws, rows: int, cols: int):
        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                ws.cell(row=r, column=c, value=f"R{r}C{c}")

    def _apply_edits(self, ws, edits: List[Dict[str, Any]], value_key: str):
        value_field = f"value_{value_key}"

        for edit in edits:
            addr = edit.get("addr")
            if not addr:
                raise ValueError("multi_cell_diff edits require 'addr'")
            if value_field not in edit:
                raise ValueError(f"multi_cell_diff edits require '{value_field}'")
            ws[addr] = edit[value_field]

class GridTailDiffGenerator(BaseGenerator):
    """Generates workbook pairs for simple row/column tail append/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("grid_tail_diff generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Sheet1")

        if mode == "row_append_bottom":
            self._row_append_bottom(output_dir, output_names, sheet)
        elif mode == "row_delete_bottom":
            self._row_delete_bottom(output_dir, output_names, sheet)
        elif mode == "col_append_right":
            self._col_append_right(output_dir, output_names, sheet)
        elif mode == "col_delete_right":
            self._col_delete_right(output_dir, output_names, sheet)
        else:
            raise ValueError(f"Unsupported grid_tail_diff mode: {mode}")

    def _row_append_bottom(self, output_dir: Path, output_names: List[str], sheet: str):
        base_rows = self.args.get("base_rows", 10)
        tail_rows = self.args.get("tail_rows", 2)
        cols = self.args.get("cols", 3)

        self._write_rows(output_dir / output_names[0], sheet, base_rows, cols, 1)
        self._write_rows(
            output_dir / output_names[1],
            sheet,
            base_rows + tail_rows,
            cols,
            1,
        )

    def _row_delete_bottom(self, output_dir: Path, output_names: List[str], sheet: str):
        base_rows = self.args.get("base_rows", 10)
        tail_rows = self.args.get("tail_rows", 2)
        cols = self.args.get("cols", 3)

        self._write_rows(
            output_dir / output_names[0],
            sheet,
            base_rows + tail_rows,
            cols,
            1,
        )
        self._write_rows(output_dir / output_names[1], sheet, base_rows, cols, 1)

    def _col_append_right(self, output_dir: Path, output_names: List[str], sheet: str):
        base_cols = self.args.get("base_cols", 4)
        tail_cols = self.args.get("tail_cols", 2)
        rows = self.args.get("rows", 5)

        self._write_cols(output_dir / output_names[0], sheet, rows, base_cols)
        self._write_cols(
            output_dir / output_names[1],
            sheet,
            rows,
            base_cols + tail_cols,
        )

    def _col_delete_right(self, output_dir: Path, output_names: List[str], sheet: str):
        base_cols = self.args.get("base_cols", 4)
        tail_cols = self.args.get("tail_cols", 2)
        rows = self.args.get("rows", 5)

        self._write_cols(
            output_dir / output_names[0],
            sheet,
            rows,
            base_cols + tail_cols,
        )
        self._write_cols(output_dir / output_names[1], sheet, rows, base_cols)

    def _write_rows(self, path: Path, sheet: str, rows: int, cols: int, start_value: int):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r in range(1, rows + 1):
            ws.cell(row=r, column=1, value=start_value + r - 1)
            for c in range(2, cols + 1):
                ws.cell(row=r, column=c, value=f"R{r}C{c}")

        wb.save(path)

    def _write_cols(self, path: Path, sheet: str, rows: int, cols: int):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                ws.cell(row=r, column=c, value=f"R{r}C{c}")

        wb.save(path)

class RowAlignmentG8Generator(BaseGenerator):
    """Generates workbook pairs for G8-style middle row insert/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_alignment_g8 generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Sheet1")
        base_rows = self.args.get("base_rows", 10)
        cols = self.args.get("cols", 5)
        insert_at = self.args.get("insert_at", 6)  # 1-based position in B
        delete_row = self.args.get("delete_row", 6)  # 1-based position in A
        edit_row = self.args.get("edit_row")  # Optional extra edit row (1-based in B after insert)
        edit_col = self.args.get("edit_col", 2)  # 1-based column for extra edit

        base_data = [self._base_row_values(idx, cols) for idx in range(1, base_rows + 1)]

        if mode == "insert":
            data_a = base_data
            data_b = self._with_insert(base_data, insert_at, cols)
        elif mode == "delete":
            data_a = base_data
            data_b = self._with_delete(base_data, delete_row)
        elif mode == "insert_with_edit":
            data_a = base_data
            data_b = self._with_insert(base_data, insert_at, cols)
            target_row = edit_row or (insert_at + 2)
            if 1 <= target_row <= len(data_b):
                row_values = list(data_b[target_row - 1])
                col_index = max(1, min(edit_col, cols)) - 1
                row_values[col_index] = "EditedAfterInsert"
                data_b[target_row - 1] = row_values
        else:
            raise ValueError(f"Unsupported row_alignment_g8 mode: {mode}")

        self._write_workbook(output_dir / output_names[0], sheet, data_a)
        self._write_workbook(output_dir / output_names[1], sheet, data_b)

    def _base_row_values(self, row_number: int, cols: int) -> List[str]:
        return [f"Row{row_number}_Col{c}" for c in range(1, cols + 1)]

    def _insert_row_values(self, cols: int) -> List[str]:
        return [f"Inserted_Row_Col{c}" for c in range(1, cols + 1)]

    def _with_insert(self, base_data: List[List[str]], insert_at: int, cols: int) -> List[List[str]]:
        insert_idx = max(1, min(insert_at, len(base_data) + 1))
        insert_row = self._insert_row_values(cols)
        return base_data[: insert_idx - 1] + [insert_row] + base_data[insert_idx - 1 :]

    def _with_delete(self, base_data: List[List[str]], delete_row: int) -> List[List[str]]:
        if not (1 <= delete_row <= len(base_data)):
            raise ValueError(f"delete_row must be within 1..{len(base_data)}")
        return base_data[: delete_row - 1] + base_data[delete_row:]

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[str]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RowAlignmentG10Generator(BaseGenerator):
    """Generates workbook pairs for G10 contiguous row block insert/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_alignment_g10 generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Sheet1")
        base_rows = self.args.get("base_rows", 10)
        cols = self.args.get("cols", 5)
        block_rows = self.args.get("block_rows", 4)
        insert_at = self.args.get("insert_at", 4)  # 1-based position of first inserted row in B
        delete_start = self.args.get("delete_start", 4)  # 1-based starting row in A to delete

        base_data = [self._row_values(idx, cols, 0) for idx in range(1, base_rows + 1)]

        if mode == "block_insert":
            data_a = base_data
            data_b = self._with_block_insert(base_data, insert_at, block_rows, cols)
        elif mode == "block_delete":
            data_a = base_data
            data_b = self._with_block_delete(base_data, delete_start, block_rows)
        else:
            raise ValueError(f"Unsupported row_alignment_g10 mode: {mode}")

        self._write_workbook(output_dir / output_names[0], sheet, data_a)
        self._write_workbook(output_dir / output_names[1], sheet, data_b)

    def _row_values(self, row_number: int, cols: int, offset: int) -> List[int]:
        row_id = row_number + offset
        values = [row_id]
        for c in range(1, cols):
            values.append(row_id * 10 + c)
        return values

    def _block_rows(self, count: int, cols: int) -> List[List[int]]:
        return [self._row_values(1000 + idx, cols, 0) for idx in range(1, count + 1)]

    def _with_block_insert(
        self, base_data: List[List[int]], insert_at: int, block_rows: int, cols: int
    ) -> List[List[int]]:
        insert_idx = max(1, min(insert_at, len(base_data) + 1)) - 1
        block = self._block_rows(block_rows, cols)
        return base_data[:insert_idx] + block + base_data[insert_idx:]

    def _with_block_delete(
        self, base_data: List[List[int]], delete_start: int, block_rows: int
    ) -> List[List[int]]:
        if not (1 <= delete_start <= len(base_data)):
            raise ValueError(f"delete_start must be within 1..{len(base_data)}")
        if delete_start - 1 + block_rows > len(base_data):
            raise ValueError("delete block exceeds base data length")

        delete_idx = delete_start - 1
        return base_data[:delete_idx] + base_data[delete_idx + block_rows :]

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[int]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RowBlockMoveG11Generator(BaseGenerator):
    """Generates workbook pairs for G11 exact row block move scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_block_move_g11 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Sheet1")
        total_rows = self.args.get("total_rows", 20)
        cols = self.args.get("cols", 5)
        block_rows = self.args.get("block_rows", 4)
        src_start = self.args.get("src_start", 5)
        dst_start = self.args.get("dst_start", 13)

        if block_rows <= 0:
            raise ValueError("block_rows must be positive")
        if src_start < 1 or src_start + block_rows - 1 > total_rows:
            raise ValueError("source block must fit within total_rows")
        if dst_start < 1 or dst_start + block_rows - 1 > total_rows:
            raise ValueError("destination block must fit within total_rows")

        src_end = src_start + block_rows - 1
        dst_end = dst_start + block_rows - 1
        if not (src_end < dst_start or dst_end < src_start):
            raise ValueError("source and destination blocks must not overlap")

        rows_a = self._build_rows(total_rows, cols, src_start, block_rows)
        rows_b = self._move_block(rows_a, src_start, block_rows, dst_start)

        self._write_workbook(output_dir / output_names[0], sheet, rows_a)
        self._write_workbook(output_dir / output_names[1], sheet, rows_b)

    def _build_rows(self, total_rows: int, cols: int, src_start: int, block_rows: int) -> List[List[str]]:
        block_end = src_start + block_rows - 1
        rows: List[List[str]] = []
        for r in range(1, total_rows + 1):
            if src_start <= r <= block_end:
                rows.append([f"BLOCK_r{r}_c{c}" for c in range(1, cols + 1)])
            else:
                rows.append([f"R{r}_C{c}" for c in range(1, cols + 1)])
        return rows

    def _move_block(
        self, rows: List[List[str]], src_start: int, block_rows: int, dst_start: int
    ) -> List[List[str]]:
        rows_b = [list(r) for r in rows]
        src_idx = src_start - 1
        src_end = src_idx + block_rows
        block = rows_b[src_idx:src_end]
        del rows_b[src_idx:src_end]

        dst_idx = min(dst_start - 1, len(rows_b))

        rows_b[dst_idx:dst_idx] = block
        return rows_b

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[str]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RowFuzzyMoveG13Generator(BaseGenerator):
    """Generates workbook pairs for G13 fuzzy row block move scenarios with internal edits."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("row_fuzzy_move_g13 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Data")
        total_rows = self.args.get("total_rows", 24)
        cols = self.args.get("cols", 6)
        block_rows = self.args.get("block_rows", 4)
        src_start = self.args.get("src_start", 5)
        dst_start = self.args.get("dst_start", 14)
        edits = self.args.get(
            "edits",
            [
                {"row_offset": 1, "col": 3, "delta": 1},
            ],
        )

        if block_rows <= 0:
            raise ValueError("block_rows must be positive")
        if src_start < 1 or src_start + block_rows - 1 > total_rows:
            raise ValueError("source block must fit within total_rows")
        if dst_start < 1 or dst_start + block_rows - 1 > total_rows:
            raise ValueError("destination block must fit within total_rows")

        src_end = src_start + block_rows - 1
        dst_end = dst_start + block_rows - 1
        if not (src_end < dst_start or dst_end < src_start):
            raise ValueError("source and destination blocks must not overlap")

        rows_a = self._build_rows(total_rows, cols, src_start, block_rows)
        rows_b = self._move_block(rows_a, src_start, block_rows, dst_start)
        self._apply_edits(rows_b, dst_start, block_rows, cols, edits)

        self._write_workbook(output_dir / output_names[0], sheet, rows_a)
        self._write_workbook(output_dir / output_names[1], sheet, rows_b)

    def _build_rows(self, total_rows: int, cols: int, block_start: int, block_rows: int) -> List[List[int]]:
        block_end = block_start + block_rows - 1
        rows: List[List[int]] = []
        for r in range(1, total_rows + 1):
            if block_start <= r <= block_end:
                row_id = 1_000 + (r - block_start)
            else:
                row_id = r
            row_values = [row_id]
            for c in range(1, cols):
                row_values.append(row_id * 10 + c)
            rows.append(row_values)
        return rows

    def _move_block(
        self, rows: List[List[int]], src_start: int, block_rows: int, dst_start: int
    ) -> List[List[int]]:
        rows_b = [list(r) for r in rows]
        src_idx = src_start - 1
        src_end = src_idx + block_rows
        block = rows_b[src_idx:src_end]
        del rows_b[src_idx:src_end]

        dst_idx = min(dst_start - 1, len(rows_b))
        rows_b[dst_idx:dst_idx] = block
        return rows_b

    def _apply_edits(
        self,
        rows: List[List[int]],
        dst_start: int,
        block_rows: int,
        cols: int,
        edits: List[Dict[str, Any]],
    ):
        dst_idx = dst_start - 1
        if dst_idx + block_rows > len(rows):
            return

        for edit in edits:
            row_offset = int(edit.get("row_offset", 0))
            col = int(edit.get("col", 1))
            delta = int(edit.get("delta", 1))

            if row_offset < 0 or row_offset >= block_rows:
                continue

            col_idx = max(1, min(col, cols)) - 1
            target_row = dst_idx + row_offset
            if col_idx >= len(rows[target_row]):
                continue
            rows[target_row][col_idx] += delta

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[int]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class ColumnMoveG12Generator(BaseGenerator):
    """Generates workbook pairs for G12 exact column move scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("column_move_g12 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Data")
        cols = self.args.get("cols", 8)
        data_rows = self.args.get("data_rows", 9)
        src_col = self.args.get("src_col", 3)
        dst_col = self.args.get("dst_col", 6)

        if not (1 <= src_col <= cols):
            raise ValueError("src_col must be within 1..cols")
        if not (1 <= dst_col <= cols):
            raise ValueError("dst_col must be within 1..cols")
        if src_col == dst_col:
            raise ValueError("src_col and dst_col must differ for a move")

        base_rows = self._build_rows(cols, data_rows, src_col)
        moved_rows = self._move_column(base_rows, src_col, dst_col)

        self._write_workbook(output_dir / output_names[0], sheet, base_rows)
        self._write_workbook(output_dir / output_names[1], sheet, moved_rows)

    def _build_rows(self, cols: int, data_rows: int, key_col: int) -> List[List[Any]]:
        header: List[Any] = []
        for c in range(1, cols + 1):
            if c == key_col:
                header.append("C_key")
            else:
                header.append(f"Col{c}")

        rows: List[List[Any]] = [header]
        for r in range(1, data_rows + 1):
            row: List[Any] = []
            for c in range(1, cols + 1):
                if c == key_col:
                    row.append(100 * r)
                else:
                    row.append(r * 10 + c)
            rows.append(row)

        return rows

    def _move_column(
        self, rows: List[List[Any]], src_col: int, dst_col: int
    ) -> List[List[Any]]:
        src_idx = src_col - 1
        dst_idx = dst_col - 1
        moved_rows: List[List[Any]] = []

        for row in rows:
            new_row = list(row)
            value = new_row.pop(src_idx)
            insert_at = max(0, min(dst_idx, len(new_row)))
            new_row.insert(insert_at, value)
            moved_rows.append(new_row)

        return moved_rows

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[Any]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class RectBlockMoveG12Generator(BaseGenerator):
    """Generates workbook pairs for G12 exact rectangular block move scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("rect_block_move_g12 generator expects exactly two output filenames")

        sheet = self.args.get("sheet", "Data")
        rows = self.args.get("rows", 15)
        cols = self.args.get("cols", 15)
        src_top = self.args.get("src_top", 3)  # 1-based
        src_left = self.args.get("src_left", 2)  # 1-based (column B)
        dst_top = self.args.get("dst_top", 10)  # 1-based
        dst_left = self.args.get("dst_left", 7)  # 1-based (column G)
        block_rows = self.args.get("block_rows", 3)
        block_cols = self.args.get("block_cols", 3)

        self._write_workbook(
            output_dir / output_names[0],
            sheet,
            rows,
            cols,
            src_top,
            src_left,
            block_rows,
            block_cols,
        )
        self._write_workbook(
            output_dir / output_names[1],
            sheet,
            rows,
            cols,
            dst_top,
            dst_left,
            block_rows,
            block_cols,
        )

    def _write_workbook(
        self,
        path: Path,
        sheet: str,
        rows: int,
        cols: int,
        block_top: int,
        block_left: int,
        block_rows: int,
        block_cols: int,
    ):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        self._fill_background(ws, rows, cols)
        self._write_block(ws, block_top, block_left, block_rows, block_cols)

        wb.save(path)

    def _fill_background(self, ws, rows: int, cols: int):
        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                ws.cell(row=r, column=c, value=self._background_value(r, c))

    def _background_value(self, row: int, col: int) -> int:
        return 1000 * row + col

    def _write_block(self, ws, top: int, left: int, block_rows: int, block_cols: int):
        for r_offset in range(block_rows):
            for c_offset in range(block_cols):
                value = 9000 + r_offset * 10 + c_offset
                ws.cell(row=top + r_offset, column=left + c_offset, value=value)

class ColumnAlignmentG9Generator(BaseGenerator):
    """Generates workbook pairs for G9-style middle column insert/delete scenarios."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("column_alignment_g9 generator expects exactly two output filenames")

        mode = self.args.get("mode")
        sheet = self.args.get("sheet", "Data")
        base_cols = self.args.get("cols", 8)
        data_rows = self.args.get("data_rows", 9)  # excludes header
        insert_at = self.args.get("insert_at", 4)  # 1-based position in B after insert
        delete_col = self.args.get("delete_col", 4)
        edit_row = self.args.get("edit_row", 8)
        edit_col_after_insert = self.args.get("edit_col_after_insert", 7)

        base_table = self._base_table(base_cols, data_rows)

        if mode == "insert":
            data_a = self._clone_rows(base_table)
            data_b = self._with_insert(base_table, insert_at)
        elif mode == "delete":
            data_a = self._clone_rows(base_table)
            data_b = self._with_delete(base_table, delete_col)
        elif mode == "insert_with_edit":
            data_a = self._clone_rows(base_table)
            data_b = self._with_insert(base_table, insert_at)
            row_idx = max(2, min(edit_row, len(data_b))) - 1  # stay below header
            col_idx = max(1, min(edit_col_after_insert, len(data_b[row_idx]))) - 1
            data_b[row_idx][col_idx] = "EditedAfterInsert"
        else:
            raise ValueError(f"Unsupported column_alignment_g9 mode: {mode}")

        self._write_workbook(output_dir / output_names[0], sheet, data_a)
        self._write_workbook(output_dir / output_names[1], sheet, data_b)

    def _base_table(self, cols: int, data_rows: int) -> List[List[str]]:
        header = [f"Col{c}" for c in range(1, cols + 1)]
        rows = [header]
        for r in range(1, data_rows + 1):
            rows.append([f"R{r}_C{c}" for c in range(1, cols + 1)])
        return rows

    def _with_insert(self, base_data: List[List[str]], insert_at: int) -> List[List[str]]:
        insert_idx = max(1, min(insert_at, len(base_data[0]) + 1))
        result: List[List[str]] = []
        for row_idx, row in enumerate(base_data):
            new_row = list(row)
            value = "Inserted" if row_idx == 0 else f"Inserted_{row_idx}"
            new_row.insert(insert_idx - 1, value)
            result.append(new_row)
        return result

    def _with_delete(self, base_data: List[List[str]], delete_col: int) -> List[List[str]]:
        if not base_data:
            return []
        if not (1 <= delete_col <= len(base_data[0])):
            raise ValueError(f"delete_col must be within 1..{len(base_data[0])}")
        result: List[List[str]] = []
        for row in base_data:
            new_row = list(row)
            del new_row[delete_col - 1]
            result.append(new_row)
        return result

    def _clone_rows(self, rows: List[List[str]]) -> List[List[str]]:
        return [list(r) for r in rows]

    def _write_workbook(self, path: Path, sheet: str, rows: List[List[str]]):
        wb = openpyxl.Workbook()
        ws = wb.active
        ws.title = sheet

        for r_idx, row_values in enumerate(rows, start=1):
            for c_idx, value in enumerate(row_values, start=1):
                ws.cell(row=r_idx, column=c_idx, value=value)

        wb.save(path)

class SheetCaseRenameGenerator(BaseGenerator):
    """Generates a pair of workbooks that differ only by sheet name casing, with optional cell edit."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("sheet_case_rename generator expects exactly two output filenames")

        sheet_a = self.args.get("sheet_a", "Sheet1")
        sheet_b = self.args.get("sheet_b", "sheet1")
        cell = self.args.get("cell", "A1")
        value_a = self.args.get("value_a", 1.0)
        value_b = self.args.get("value_b", value_a)

        def create_workbook(sheet_name: str, value, output_name: str):
            wb = openpyxl.Workbook()
            ws = wb.active
            ws.title = sheet_name
            ws[cell] = value
            wb.save(output_dir / output_name)

        create_workbook(sheet_a, value_a, output_names[0])
        create_workbook(sheet_b, value_b, output_names[1])

class Pg6SheetScenarioGenerator(BaseGenerator):
    """Generates workbook pairs for PG6 sheet add/remove/rename vs grid responsibilities."""
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        if len(output_names) != 2:
            raise ValueError("pg6_sheet_scenario generator expects exactly two output filenames")

        mode = self.args.get("mode")
        a_path = output_dir / output_names[0]
        b_path = output_dir / output_names[1]

        if mode == "sheet_added":
            self._gen_sheet_added(a_path, b_path)
        elif mode == "sheet_removed":
            self._gen_sheet_removed(a_path, b_path)
        elif mode == "sheet_renamed":
            self._gen_sheet_renamed(a_path, b_path)
        elif mode == "sheet_and_grid_change":
            self._gen_sheet_and_grid_change(a_path, b_path)
        else:
            raise ValueError(f"Unsupported PG6 mode: {mode}")

    def _fill_grid(self, worksheet, rows: int, cols: int, prefix: str = "R"):
        for r in range(1, rows + 1):
            for c in range(1, cols + 1):
                worksheet.cell(row=r, column=c, value=f"{prefix}{r}C{c}")

    def _gen_sheet_added(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, 5, 5)
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, 5, 5)
        ws_new = wb_b.create_sheet("NewSheet")
        self._fill_grid(ws_new, 3, 3, prefix="N")
        wb_b.save(b_path)

    def _gen_sheet_removed(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, 5, 5)
        ws_old = wb_a.create_sheet("OldSheet")
        self._fill_grid(ws_old, 3, 3, prefix="O")
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, 5, 5)
        wb_b.save(b_path)

    def _gen_sheet_renamed(self, a_path: Path, b_path: Path):
        wb_a = openpyxl.Workbook()
        ws_old = wb_a.active
        ws_old.title = "OldName"
        self._fill_grid(ws_old, 3, 3)
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_new = wb_b.active
        ws_new.title = "NewName"
        self._fill_grid(ws_new, 3, 3)
        wb_b.save(b_path)

    def _gen_sheet_and_grid_change(self, a_path: Path, b_path: Path):
        base_rows = 5
        base_cols = 5

        wb_a = openpyxl.Workbook()
        ws_main_a = wb_a.active
        ws_main_a.title = "Main"
        self._fill_grid(ws_main_a, base_rows, base_cols)
        ws_aux_a = wb_a.create_sheet("Aux")
        self._fill_grid(ws_aux_a, 3, 3, prefix="A")
        wb_a.save(a_path)

        wb_b = openpyxl.Workbook()
        ws_main_b = wb_b.active
        ws_main_b.title = "Main"
        self._fill_grid(ws_main_b, base_rows, base_cols)
        ws_main_b["A1"] = "Main changed 1"
        ws_main_b["B2"] = "Main changed 2"
        ws_main_b["C3"] = "Main changed 3"

        ws_aux_b = wb_b.create_sheet("Aux")
        self._fill_grid(ws_aux_b, 3, 3, prefix="A")

        ws_scratch = wb_b.create_sheet("Scratch")
        self._fill_grid(ws_scratch, 2, 2, prefix="S")
        wb_b.save(b_path)
```

---

### File: `fixtures\src\generators\mashup.py`

```python
import base64
import copy
import io
import random
import re
import struct
import zipfile
from pathlib import Path
from typing import Callable, List, Optional, Union
from xml.etree import ElementTree as ET
from lxml import etree
from .base import BaseGenerator

# XML Namespaces
NS = {'dm': 'http://schemas.microsoft.com/DataMashup'}

class MashupBaseGenerator(BaseGenerator):
    """Base class for handling the outer Excel container and finding DataMashup."""
    
    def _get_mashup_element(self, tree):
        if tree.tag.endswith("DataMashup"):
            return tree
        return tree.find('.//dm:DataMashup', namespaces=NS)

    def _process_excel_container(
        self,
        base_path,
        output_path,
        callback,
        text_mutator: Optional[Callable[[str], str]] = None,
    ):
        """
        Generic wrapper to open xlsx, find customXml, apply a callback to the 
        DataMashup bytes, and save the result.
        """
        # Copy base file structure to output
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for item in zin.infolist():
                    buffer = zin.read(item.filename)
                    
                    # We only care about the item containing DataMashup
                    # Usually customXml/item1.xml, but we check content to be safe
                    has_marker = b"DataMashup" in buffer or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in buffer
                    if item.filename.startswith("customXml/item") and has_marker:
                        # Parse XML
                        root = etree.fromstring(buffer)
                        dm_node = self._get_mashup_element(root)
                        
                        if dm_node is not None:
                            # 1. Decode
                            # The text content might have whitespace/newlines, strip them
                            b64_text = dm_node.text.strip() if dm_node.text else ""
                            if b64_text:
                                raw_bytes = base64.b64decode(b64_text)
                                
                                # 2. Apply modification (The Callback)
                                new_bytes = callback(raw_bytes)
                                
                                # 3. Encode back
                                new_text = base64.b64encode(new_bytes).decode('utf-8')
                                if text_mutator is not None:
                                    new_text = text_mutator(new_text)
                                dm_node.text = new_text
                                buffer = etree.tostring(root, encoding='utf-8', xml_declaration=True)
                    
                    zout.writestr(item, buffer)

class MashupCorruptGenerator(MashupBaseGenerator):
    """Fuzzes the DataMashup bytes to test error handling."""
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        if not base_file_arg:
            raise ValueError("MashupCorruptGenerator requires 'base_file' argument")

        # Resolve base file relative to current working directory or fixtures/templates
        base = Path(base_file_arg)
        if not base.exists():
             # Try looking in fixtures/templates if a relative path was given
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        mode = self.args.get('mode', 'byte_flip')

        def corruptor(data):
            mutable = bytearray(data)
            if len(mutable) == 0:
                return bytes(mutable)

            if mode == 'byte_flip':
                # Flip a byte in the middle
                idx = len(mutable) // 2
                mutable[idx] = mutable[idx] ^ 0xFF
            elif mode == 'truncate':
                return mutable[:len(mutable)//2]
            return bytes(mutable)

        for name in output_names:
            # Convert Path objects to strings for resolve() to work correctly if there's a mix
            # Actually output_dir is a Path. name is str.
            # .resolve() resolves symlinks and relative paths to absolute
            target_path = (output_dir / name).resolve()
            text_mutator = self._garble_base64_text if mode == 'byte_flip' else None
            self._process_excel_container(
                base.resolve(),
                target_path,
                corruptor,
                text_mutator=text_mutator,
            )

    def _garble_base64_text(self, encoded: str) -> str:
        if not encoded:
            return "!!"
        chars = list(encoded)
        chars[0] = "!"
        return "".join(chars)


class MashupInjectGenerator(MashupBaseGenerator):
    """
    Peels the onion:
    1. Parses MS-QDEFF binary header.
    2. Unzips PackageParts.
    3. Injects new M-Code into Section1.m.
    4. Re-zips and fixes header lengths.
    """
    
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]
            
        base_file_arg = self.args.get('base_file')
        new_m_code = self.args.get('m_code')

        if not base_file_arg:
             raise ValueError("MashupInjectGenerator requires 'base_file' argument")
        if new_m_code is None:
             raise ValueError("MashupInjectGenerator requires 'm_code' argument")

        base = Path(base_file_arg)
        if not base.exists():
             candidate = Path("fixtures") / base_file_arg
             if candidate.exists():
                 base = candidate
             else:
                raise FileNotFoundError(f"Template {base} not found.")

        def injector(raw_bytes):
            return self._inject_m_code(raw_bytes, new_m_code)

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, injector)

    def _inject_m_code(self, raw_bytes, m_code):
        # --- 1. Parse MS-QDEFF Header ---
        # Format: Version(4) + LenPP(4) + PackageParts(...) + LenPerm(4) + ...
        # We assume Version is 0 (first 4 bytes)
        
        if len(raw_bytes) < 8:
            return raw_bytes # Too short to handle

        offset = 4
        # Read PackageParts Length
        pp_len = struct.unpack('<I', raw_bytes[offset:offset+4])[0]
        offset += 4
        
        # Extract existing components
        pp_bytes = raw_bytes[offset : offset + pp_len]
        
        # Keep the rest of the stream (Permissions, Metadata, Bindings) intact
        # We just append it later
        remainder_bytes = raw_bytes[offset + pp_len :]

        # --- 2. Modify PackageParts (Inner ZIP) ---
        new_pp_bytes = self._replace_in_zip(pp_bytes, 'Formulas/Section1.m', m_code)

        # --- 3. Rebuild Stream ---
        # New Length for PackageParts
        new_pp_len = len(new_pp_bytes)
        
        # Reconstruct: Version(0) + NewLen + NewPP + Remainder
        header = raw_bytes[:4] # Version
        len_pack = struct.pack('<I', new_pp_len)
        
        return header + len_pack + new_pp_bytes + remainder_bytes

    def _replace_in_zip(self, zip_bytes, filename, new_content):
        """Opens a ZIP byte stream, replaces a file, returns new ZIP byte stream."""
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()
        
        try:
            with zipfile.ZipFile(in_buffer, 'r') as zin:
                with zipfile.ZipFile(out_buffer, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
                    for item in zin.infolist():
                        if item.filename == filename:
                            # Write the new M code
                            zout.writestr(filename, new_content.encode('utf-8'))
                        else:
                            # Copy others
                            zout.writestr(item, zin.read(item.filename))
        except zipfile.BadZipFile:
            # Fallback if inner stream isn't a valid zip (shouldn't happen on valid QDEFF)
            return zip_bytes
            
        return out_buffer.getvalue()


class MashupPackagePartsGenerator(MashupBaseGenerator):
    """
    Generates PackageParts-focused fixtures starting from a base workbook.
    """

    variant: str = "one_query"

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        if self.variant == "one_query":
            return raw_bytes

        version, package_parts, permissions, metadata, bindings = self._split_sections(raw_bytes)
        package_xml, main_section_text, content_types = self._extract_package_parts(package_parts)

        embedded_guid = self.args.get(
            "embedded_guid", "{11111111-2222-3333-4444-555555555555}"
        )
        embedded_section_text = self.args.get(
            "embedded_section",
            self._default_embedded_section(),
        )
        updated_main_section = self._extend_main_section(main_section_text, embedded_guid)
        embedded_bytes = self._build_embedded_package(embedded_section_text, content_types)
        updated_package_parts = self._build_package_parts(
            package_xml,
            updated_main_section,
            content_types,
            embedded_guid,
            embedded_bytes,
        )

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions,
            metadata,
            bindings,
        )

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts_end = offset + package_parts_len
        if package_parts_end > len(raw_bytes):
            raise ValueError("invalid PackageParts length")
        package_parts = raw_bytes[offset:package_parts_end]
        offset = package_parts_end

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions_end = offset + permissions_len
        if permissions_end > len(raw_bytes):
            raise ValueError("invalid permissions length")
        permissions = raw_bytes[offset:permissions_end]
        offset = permissions_end

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata_end = offset + metadata_len
        if metadata_end > len(raw_bytes):
            raise ValueError("invalid metadata length")
        metadata = raw_bytes[offset:metadata_end]
        offset = metadata_end

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings_end = offset + bindings_len
        if bindings_end > len(raw_bytes):
            raise ValueError("invalid bindings length")
        bindings = raw_bytes[offset:bindings_end]
        offset = bindings_end

        if offset != len(raw_bytes):
            raise ValueError("DataMashup trailing bytes mismatch")

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _extract_package_parts(self, package_parts: bytes):
        with zipfile.ZipFile(io.BytesIO(package_parts), "r") as z:
            package_xml = z.read("Config/Package.xml")
            content_types = z.read("[Content_Types].xml")
            main_section = z.read("Formulas/Section1.m")
        return package_xml, main_section.decode("utf-8", errors="ignore"), content_types

    def _extend_main_section(self, base_section: str, embedded_guid: str) -> str:
        stripped = base_section.rstrip()
        lines = [
            stripped,
            "",
            "shared EmbeddedQuery = let",
            f'    Source = Embedded.Value("Content/{embedded_guid}.package")',
            "in",
            "    Source;",
        ]
        return "\n".join(lines)

    def _build_embedded_package(self, section_text: str, content_types_template: bytes) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Formulas/Section1.m", section_text)
        return buffer.getvalue()

    def _build_package_parts(
        self,
        package_xml: bytes,
        main_section: str,
        content_types_template: bytes,
        embedded_guid: str,
        embedded_package: bytes,
    ) -> bytes:
        content_types = self._augment_content_types(content_types_template)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("[Content_Types].xml", content_types)
            z.writestr("Config/Package.xml", package_xml)
            z.writestr("Formulas/Section1.m", main_section)
            z.writestr(f"Content/{embedded_guid}.package", embedded_package)
        return buffer.getvalue()

    def _augment_content_types(self, content_types_bytes: bytes) -> str:
        text = content_types_bytes.decode("utf-8", errors="ignore")
        if "Extension=\"package\"" not in text and "Extension='package'" not in text:
            text = text.replace(
                "</Types>",
                '<Default Extension="package" ContentType="application/octet-stream" /></Types>',
                1,
            )
        return text

    def _default_embedded_section(self) -> str:
        return "\n".join(
            [
                "section Section1;",
                "",
                "shared Inner = let",
                "    Source = 1",
                "in",
                "    Source;",
            ]
        )


class MashupOneQueryGenerator(MashupPackagePartsGenerator):
    variant = "one_query"


class MashupMultiEmbeddedGenerator(MashupPackagePartsGenerator):
    variant = "multi_query_with_embedded"


class MashupDuplicateGenerator(MashupBaseGenerator):
    """
    Duplicates the customXml part that contains DataMashup to produce two
    DataMashup occurrences in a single workbook.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        mode = self.args.get('mode', 'part')
        if not base_file_arg:
            raise ValueError("MashupDuplicateGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            if mode == 'part':
                self._duplicate_datamashup_part(base.resolve(), target_path)
            elif mode == 'element':
                self._duplicate_datamashup_element(base.resolve(), target_path)
            else:
                raise ValueError(f"Unsupported duplicate mode: {mode}")

    def _duplicate_datamashup_part(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            try:
                item1_xml = zin.read("customXml/item1.xml")
                item_props1 = zin.read("customXml/itemProps1.xml")
                item1_rels = zin.read("customXml/_rels/item1.xml.rels")
                content_types = zin.read("[Content_Types].xml")
                workbook_rels = zin.read("xl/_rels/workbook.xml.rels")
            except KeyError as e:
                raise FileNotFoundError(f"Required DataMashup part missing: {e}") from e

            updated_content_types = self._add_itemprops_override(content_types)
            updated_workbook_rels = self._add_workbook_relationship(workbook_rels)
            item2_rels = item1_rels.replace(b"itemProps1.xml", b"itemProps2.xml")
            item_props2 = item_props1.replace(
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BE}",
                b"{37E9CB8A-1D60-4852-BCC8-3140E13993BF}",
            )

            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename == "[Content_Types].xml":
                        data = updated_content_types
                    elif info.filename == "xl/_rels/workbook.xml.rels":
                        data = updated_workbook_rels
                    zout.writestr(info, data)

                zout.writestr("customXml/item2.xml", item1_xml)
                zout.writestr("customXml/itemProps2.xml", item_props2)
                zout.writestr("customXml/_rels/item2.xml.rels", item2_rels)

    def _add_itemprops_override(self, content_types_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/content-types"
        root = ET.fromstring(content_types_bytes)
        override_tag = f"{{{ns}}}Override"
        if not any(
            elem.get("PartName") == "/customXml/itemProps2.xml"
            for elem in root.findall(override_tag)
        ):
            new_override = ET.SubElement(root, override_tag)
            new_override.set("PartName", "/customXml/itemProps2.xml")
            new_override.set(
                "ContentType",
                "application/vnd.openxmlformats-officedocument.customXmlProperties+xml",
            )
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _add_workbook_relationship(self, rels_bytes: bytes) -> bytes:
        ns = "http://schemas.openxmlformats.org/package/2006/relationships"
        root = ET.fromstring(rels_bytes)
        rel_tag = f"{{{ns}}}Relationship"
        existing_ids = {elem.get("Id") for elem in root.findall(rel_tag)}
        next_id = 1
        while f"rId{next_id}" in existing_ids:
            next_id += 1
        new_rel = ET.SubElement(root, rel_tag)
        new_rel.set("Id", f"rId{next_id}")
        new_rel.set(
            "Type",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
        )
        new_rel.set("Target", "../customXml/item2.xml")
        return ET.tostring(root, xml_declaration=True, encoding="utf-8")

    def _duplicate_datamashup_element(self, base_path: Path, output_path: Path):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            root = etree.fromstring(data)
                            dm_node = self._get_mashup_element(root)
                            if dm_node is not None:
                                duplicate = copy.deepcopy(dm_node)
                                parent = dm_node.getparent()
                                if parent is not None:
                                    parent.append(duplicate)
                                    target_root = root
                                else:
                                    container = etree.Element("root", nsmap=root.nsmap)
                                    container.append(dm_node)
                                    container.append(duplicate)
                                    target_root = container
                                data = etree.tostring(
                                    target_root, encoding="utf-8", xml_declaration=True
                                )
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)


class MashupEncodeGenerator(MashupBaseGenerator):
    """
    Re-encodes the DataMashup customXml stream to a target encoding and optionally
    inserts whitespace into the base64 payload.
    """

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get('base_file')
        encoding = self.args.get('encoding', 'utf-8')
        whitespace = bool(self.args.get('whitespace', False))
        if not base_file_arg:
            raise ValueError("MashupEncodeGenerator requires 'base_file' argument")

        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._rewrite_datamashup_xml(base.resolve(), target_path, encoding, whitespace)

    def _rewrite_datamashup_xml(
        self,
        base_path: Path,
        output_path: Path,
        encoding: str,
        whitespace: bool,
    ):
        with zipfile.ZipFile(base_path, 'r') as zin:
            with zipfile.ZipFile(output_path, 'w') as zout:
                for info in zin.infolist():
                    data = zin.read(info.filename)
                    if info.filename.startswith("customXml/item") and (
                        b"DataMashup" in data
                        or b"D\x00a\x00t\x00a\x00M\x00a\x00s\x00h\x00u\x00p" in data
                    ):
                        try:
                            data = self._process_datamashup_stream(data, encoding, whitespace)
                        except etree.XMLSyntaxError:
                            pass
                    zout.writestr(info, data)

    def _process_datamashup_stream(
        self,
        xml_bytes: bytes,
        encoding: str,
        whitespace: bool,
    ) -> bytes:
        root = etree.fromstring(xml_bytes)
        dm_node = self._get_mashup_element(root)
        if dm_node is None:
            return xml_bytes

        if dm_node.text and whitespace:
            dm_node.text = self._with_whitespace(dm_node.text)

        xml_bytes = etree.tostring(root, encoding="utf-8", xml_declaration=True)
        return self._encode_bytes(xml_bytes, encoding)

    def _with_whitespace(self, text: str) -> str:
        cleaned = text.strip()
        if not cleaned:
            return text
        midpoint = max(1, len(cleaned) // 2)
        return f"\n  {cleaned[:midpoint]}\n  {cleaned[midpoint:]}\n"

    def _encode_bytes(self, xml_bytes: bytes, encoding: str) -> bytes:
        enc = encoding.lower()
        if enc == "utf-8":
            return xml_bytes
        if enc == "utf-16-le":
            return self._to_utf16(xml_bytes, little_endian=True)
        if enc == "utf-16-be":
            return self._to_utf16(xml_bytes, little_endian=False)
        raise ValueError(f"Unsupported encoding: {encoding}")

    def _to_utf16(self, xml_bytes: bytes, little_endian: bool) -> bytes:
        text = xml_bytes.decode("utf-8")
        text = self._rewrite_declaration(text)
        encoded = text.encode("utf-16-le" if little_endian else "utf-16-be")
        bom = b"\xff\xfe" if little_endian else b"\xfe\xff"
        return bom + encoded

    def _rewrite_declaration(self, text: str) -> str:
        pattern = r'encoding=["\'][^"\']+["\']'
        if re.search(pattern, text):
            return re.sub(pattern, 'encoding="UTF-16"', text, count=1)
        prefix = "<?xml version='1.0'?>"
        if text.startswith(prefix):
            return text.replace(prefix, "<?xml version='1.0' encoding='UTF-16'?>", 1)
        return text


class MashupPermissionsMetadataGenerator(MashupBaseGenerator):
    """
    Builds fixtures that exercise Permissions and Metadata parsing by rewriting
    the PackageParts Section1.m, Permissions XML, and Metadata XML inside
    the DataMashup stream.
    """

    def __init__(self, args):
        super().__init__(args)
        self.mode = args.get("mode")

    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if not self.mode:
            raise ValueError("MashupPermissionsMetadataGenerator requires 'mode' argument")

        if isinstance(output_names, str):
            output_names = [output_names]

        base_file_arg = self.args.get("base_file", "templates/base_query.xlsx")
        base = Path(base_file_arg)
        if not base.exists():
            candidate = Path("fixtures") / base_file_arg
            if candidate.exists():
                base = candidate
            else:
                raise FileNotFoundError(f"Template {base} not found.")

        for name in output_names:
            target_path = (output_dir / name).resolve()
            self._process_excel_container(base.resolve(), target_path, self._rewrite_datamashup)

    def _rewrite_datamashup(self, raw_bytes: bytes) -> bytes:
        version, package_parts, _, _, bindings = self._split_sections(raw_bytes)
        scenario = self._scenario_definition()

        updated_package_parts = self._replace_section(
            package_parts,
            scenario["section_text"],
        )
        permissions_bytes = self._permissions_bytes(**scenario["permissions"])
        metadata_bytes = self._metadata_bytes(scenario["metadata_entries"])

        return self._assemble_sections(
            version,
            updated_package_parts,
            permissions_bytes,
            metadata_bytes,
            bindings,
        )

    def _scenario_definition(self):
        shared_section_simple = "\n".join(
            [
                "section Section1;",
                "",
                "shared LoadToSheet = 1;",
                "shared LoadToModel = 2;",
            ]
        )

        def default_permissions():
            return {
                "can_eval": False,
                "firewall_enabled": True,
                "group_type": "Organizational",
            }

        def build_section_text(query_specs):
            lines = ["section Section1;", ""]
            for spec in query_specs:
                lines.append(f"shared {spec['name']} = {spec['body']};")
            return "\n".join(lines)

        def build_metadata_entries(query_specs):
            entries = []
            for spec in query_specs:
                stable_entries = []
                if spec.get("load_to_sheet"):
                    stable_entries.append(("FillEnabled", True))
                if spec.get("load_to_model"):
                    stable_entries.append(("FillToDataModelEnabled", True))
                entries.append(
                    {
                        "path": f"Section1/{spec['name']}",
                        "entries": stable_entries,
                    }
                )
            return entries

        def m_diff_scenario(query_specs):
            return {
                "section_text": build_section_text(query_specs),
                "permissions": default_permissions(),
                "metadata_entries": build_metadata_entries(query_specs),
            }

        if self.mode in ("permissions_defaults", "permissions_firewall_off", "metadata_simple"):
            return {
                "section_text": shared_section_simple,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": self.mode != "permissions_firewall_off",
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/LoadToSheet",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/LoadToModel",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", True),
                        ],
                    },
                ],
            }

        if self.mode == "m_add_query_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_add_query_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                    {"name": "Bar", "body": "2", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_remove_query_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                    {"name": "Bar", "body": "2", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_remove_query_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_change_literal_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_change_literal_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "2", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_metadata_only_change_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_metadata_only_change_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": False, "load_to_model": True},
                ]
            )

        if self.mode == "m_def_and_metadata_change_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_def_and_metadata_change_b":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "2", "load_to_sheet": False, "load_to_model": True},
                ]
            )

        if self.mode == "m_rename_query_a":
            return m_diff_scenario(
                [
                    {"name": "Foo", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "m_rename_query_b":
            return m_diff_scenario(
                [
                    {"name": "Bar", "body": "1", "load_to_sheet": True, "load_to_model": False},
                ]
            )

        if self.mode == "metadata_query_groups":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared RootQuery = 1;",
                    "shared GroupedFoo = 2;",
                    "shared NestedBar = 3;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/RootQuery",
                        "entries": [("FillEnabled", True)],
                    },
                    {
                        "path": "Section1/GroupedFoo",
                        "entries": [
                            ("FillEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                    {
                        "path": "Section1/NestedBar",
                        "entries": [
                            ("FillToDataModelEnabled", True),
                            ("QueryGroupPath", "Inputs/DimTables"),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_hidden_queries":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared ConnectionOnly = 1;",
                    "shared VisibleLoad = 2;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/ConnectionOnly",
                        "entries": [
                            ("FillEnabled", False),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                    {
                        "path": "Section1/VisibleLoad",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_missing_entry":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared MissingMetadata = 1;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [],
            }

        if self.mode == "metadata_url_encoding":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    'shared #"Query with space & #" = 1;',
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/Query%20with%20space%20%26%20%23",
                        "entries": [
                            ("FillEnabled", True),
                            ("FillToDataModelEnabled", False),
                        ],
                    },
                ],
            }

        if self.mode == "metadata_orphan_entries":
            section_text = "\n".join(
                [
                    "section Section1;",
                    "",
                    "shared Foo = 1;",
                ]
            )
            return {
                "section_text": section_text,
                "permissions": {
                    "can_eval": False,
                    "firewall_enabled": True,
                    "group_type": "Organizational",
                },
                "metadata_entries": [
                    {
                        "path": "Section1/Foo",
                        "entries": [("FillEnabled", True)],
                    },
                    {
                        "path": "Section1/Nonexistent",
                        "entries": [("FillEnabled", False)],
                    },
                ],
            }

        raise ValueError(f"Unsupported mode: {self.mode}")

    def _split_sections(self, raw_bytes: bytes):
        min_size = 4 + 4 * 4
        if len(raw_bytes) < min_size:
            raise ValueError("DataMashup stream too short")

        offset = 0
        version = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4

        package_parts_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        package_parts = raw_bytes[offset : offset + package_parts_len]
        offset += package_parts_len

        permissions_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        permissions = raw_bytes[offset : offset + permissions_len]
        offset += permissions_len

        metadata_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        metadata = raw_bytes[offset : offset + metadata_len]
        offset += metadata_len

        bindings_len = struct.unpack_from("<I", raw_bytes, offset)[0]
        offset += 4
        bindings = raw_bytes[offset : offset + bindings_len]

        return version, package_parts, permissions, metadata, bindings

    def _assemble_sections(
        self,
        version: int,
        package_parts: bytes,
        permissions: bytes,
        metadata: bytes,
        bindings: bytes,
    ) -> bytes:
        return b"".join(
            [
                struct.pack("<I", version),
                struct.pack("<I", len(package_parts)),
                package_parts,
                struct.pack("<I", len(permissions)),
                permissions,
                struct.pack("<I", len(metadata)),
                metadata,
                struct.pack("<I", len(bindings)),
                bindings,
            ]
        )

    def _replace_section(self, package_parts: bytes, section_text: str) -> bytes:
        return self._replace_in_zip(package_parts, "Formulas/Section1.m", section_text)

    def _replace_in_zip(self, zip_bytes: bytes, filename: str, new_content: str) -> bytes:
        in_buffer = io.BytesIO(zip_bytes)
        out_buffer = io.BytesIO()

        with zipfile.ZipFile(in_buffer, "r") as zin:
            with zipfile.ZipFile(out_buffer, "w", compression=zipfile.ZIP_DEFLATED) as zout:
                for item in zin.infolist():
                    if item.filename == filename:
                        zout.writestr(filename, new_content.encode("utf-8"))
                    else:
                        zout.writestr(item, zin.read(item.filename))
        return out_buffer.getvalue()

    def _permissions_bytes(self, can_eval: bool, firewall_enabled: bool, group_type: str) -> bytes:
        xml = (
            '<?xml version="1.0" encoding="utf-8"?>'
            "<PermissionList xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" "
            "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
            f"<CanEvaluateFuturePackages>{str(can_eval).lower()}</CanEvaluateFuturePackages>"
            f"<FirewallEnabled>{str(firewall_enabled).lower()}</FirewallEnabled>"
            f"<WorkbookGroupType>{group_type}</WorkbookGroupType>"
            "</PermissionList>"
        )
        return ("\ufeff" + xml).encode("utf-8")

    def _metadata_bytes(self, items: List[dict]) -> bytes:
        xml = self._metadata_xml(items)
        xml_bytes = ("\ufeff" + xml).encode("utf-8")
        header = struct.pack("<I", 0) + struct.pack("<I", len(xml_bytes))
        return header + xml_bytes

    def _metadata_xml(self, items: List[dict]) -> str:
        parts = [
            '<?xml version="1.0" encoding="utf-8"?>',
            '<LocalPackageMetadataFile xmlns:xsd="http://www.w3.org/2001/XMLSchema" '
            'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">',
            "<Items>",
            "<Item><ItemLocation><ItemType>AllFormulas</ItemType><ItemPath /></ItemLocation><StableEntries /></Item>",
        ]

        for item in items:
            parts.append("<Item>")
            parts.append("<ItemLocation>")
            parts.append("<ItemType>Formula</ItemType>")
            parts.append(f"<ItemPath>{item['path']}</ItemPath>")
            parts.append("</ItemLocation>")
            parts.append("<StableEntries>")
            for entry_name, entry_value in item.get("entries", []):
                value = self._format_entry_value(entry_value)
                parts.append(f'<Entry Type="{entry_name}" Value="{value}" />')
            parts.append("</StableEntries>")
            parts.append("</Item>")

        parts.append("</Items></LocalPackageMetadataFile>")
        return "".join(parts)

    def _format_entry_value(self, value):
        if isinstance(value, bool):
            return f"l{'1' if value else '0'}"
        return f"s{value}"

```

---

### File: `fixtures\src\generators\perf.py`

```python
import openpyxl
import random
from pathlib import Path
from typing import Union, List
from .base import BaseGenerator

class LargeGridGenerator(BaseGenerator):
    """
    Generates massive grids using WriteOnly mode to save memory.
    Targeting P1/P2 milestones.
    """
    def generate(self, output_dir: Path, output_names: Union[str, List[str]]):
        if isinstance(output_names, str):
            output_names = [output_names]

        rows = self.args.get('rows', 1000)
        cols = self.args.get('cols', 10)
        mode = self.args.get('mode', 'dense')
        seed = self.args.get('seed', 0)

        # Use deterministic seed if provided, otherwise system time
        rng = random.Random(seed)

        for name in output_names:
            # WriteOnly mode is critical for 50k+ rows in Python
            wb = openpyxl.Workbook(write_only=True)
            ws = wb.create_sheet()
            ws.title = "Performance"

            # 1. Header
            header = [f"Col_{c}" for c in range(1, cols + 1)]
            ws.append(header)

            # 2. Data Stream
            for r in range(1, rows + 1):
                row_data = []
                if mode == 'dense':
                    # Deterministic pattern: "R{r}C{c}"
                    # Fast to generate, high compression ratio
                    row_data = [f"R{r}C{c}" for c in range(1, cols + 1)]
                
                elif mode == 'noise':
                    # Random floats: Harder to align, harder to compress
                    row_data = [rng.random() for _ in range(cols)]
                
                ws.append(row_data)

            wb.save(output_dir / name)

```

---

### File: `fixtures\src\generators\__init__.py`

```python
# Generators package

```

---

