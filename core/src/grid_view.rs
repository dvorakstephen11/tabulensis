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

        for positions in stats.hash_to_positions_b.values_mut() {
            positions.sort_unstable();
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

        freq_a >= threshold || freq_b >= threshold
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
