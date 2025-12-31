use std::collections::HashMap;
use std::hash::Hash;
#[cfg(test)]
use std::cell::Cell as ThreadLocalCell;

use crate::config::DiffConfig;
use crate::grid_metadata::classify_row_frequencies;
use crate::hashing::{hash_cell_value, hash_row_content_128};
use crate::memory_estimate::estimate_gridview_bytes;
use crate::workbook::{Cell, CellValue, ColSignature, Grid, RowSignature};
use xxhash_rust::xxh3::Xxh3;

pub use crate::grid_metadata::{FrequencyClass, RowMeta};

pub type RowHash = RowSignature;
pub type ColHash = ColSignature;

#[cfg(test)]
thread_local! {
    static GRIDVIEW_BUILD_COUNT: ThreadLocalCell<usize> = ThreadLocalCell::new(0);
}

#[cfg(test)]
pub(crate) fn reset_gridview_build_count() {
    GRIDVIEW_BUILD_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn gridview_build_count() -> usize {
    GRIDVIEW_BUILD_COUNT.with(|count| count.get())
}

#[derive(Debug)]
pub struct RowView<'a> {
    pub cells: Vec<(u32, &'a Cell)>, // sorted by column index
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        let default_config = DiffConfig::default();
        Self::from_grid_with_config(grid, &default_config)
    }

    pub fn from_grid_with_config(grid: &'a Grid, config: &DiffConfig) -> GridView<'a> {
        #[cfg(test)]
        {
            GRIDVIEW_BUILD_COUNT.with(|count| count.set(count.get().saturating_add(1)));
        }
        let nrows = grid.nrows as usize;
        let ncols = grid.ncols as usize;

        let mut row_counts = vec![0u32; nrows];
        let mut row_first_non_blank: Vec<Option<u32>> = vec![None; nrows];

        let mut col_counts = vec![0u32; ncols];
        let mut col_first_non_blank: Vec<Option<u32>> = vec![None; ncols];
        let mut total_cells: usize = 0;

        for ((row, col), cell) in grid.iter_cells() {
            let r = row as usize;
            let c = col as usize;

            debug_assert!(
                r < nrows && c < ncols,
                "cell coordinates must lie within the grid bounds"
            );

            total_cells = total_cells.saturating_add(1);

            if is_non_blank(cell) {
                row_counts[r] = row_counts[r].saturating_add(1);
                col_counts[c] = col_counts[c].saturating_add(1);

                row_first_non_blank[r] =
                    Some(row_first_non_blank[r].map_or(col, |cur| cur.min(col)));
                col_first_non_blank[c] =
                    Some(col_first_non_blank[c].map_or(row, |cur| cur.min(row)));
            }
        }

        let mut rows: Vec<RowView<'a>> = (0..nrows)
            .map(|idx| RowView {
                cells: Vec::with_capacity(row_counts[idx] as usize),
            })
            .collect();

        for ((row, col), cell) in grid.iter_cells() {
            let r = row as usize;
            debug_assert!(
                r < nrows && (col as usize) < ncols,
                "cell coordinates must lie within the grid bounds"
            );
            rows[r].cells.push((col, cell));
        }

        sort_row_cells(&mut rows, total_cells);

        let mut row_meta =
            build_row_meta(&rows, &row_counts, &row_first_non_blank, config, total_cells);

        classify_row_frequencies(&mut row_meta, config);

        let allow_parallel_cols = !should_force_sequential_col_meta(grid, config);
        let col_meta = build_col_meta(
            &rows,
            &col_counts,
            &col_first_non_blank,
            total_cells,
            allow_parallel_cols,
        );

        GridView {
            rows,
            row_meta,
            col_meta,
            source: grid,
        }
    }

    pub fn is_low_info_dominated(&self) -> bool {
        if self.row_meta.is_empty() {
            return false;
        }
        let low = self.row_meta.iter().filter(|m| m.is_low_info()).count();
        low * 2 > self.row_meta.len()
    }

    pub fn is_blank_dominated(&self) -> bool {
        if self.col_meta.is_empty() {
            return false;
        }
        let blank = self
            .col_meta
            .iter()
            .filter(|m| m.non_blank_count == 0)
            .count();
        blank * 2 > self.col_meta.len()
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
            *stats.freq_a.entry(meta.signature).or_insert(0) += 1;
        }

        for meta in rows_b {
            *stats.freq_b.entry(meta.signature).or_insert(0) += 1;
            stats
                .hash_to_positions_b
                .entry(meta.signature)
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

    pub fn is_unique_to_a(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) == 1
            && self.freq_b.get(&hash).copied().unwrap_or(0) == 0
    }

    pub fn is_unique_to_b(&self, hash: H) -> bool {
        self.freq_a.get(&hash).copied().unwrap_or(0) == 0
            && self.freq_b.get(&hash).copied().unwrap_or(0) == 1
    }

    pub fn max_frequency(&self) -> u32 {
        self.freq_a
            .values()
            .chain(self.freq_b.values())
            .copied()
            .max()
            .unwrap_or(0)
    }

    pub fn has_heavy_repetition(&self, max_repeat: u32) -> bool {
        self.max_frequency() > max_repeat
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

#[cfg(feature = "parallel")]
const PAR_MIN_ROWS: usize = 2048;
#[cfg(feature = "parallel")]
const PAR_MIN_CELLS: usize = 200_000;
#[cfg(feature = "parallel")]
const PAR_MIN_COLS: usize = 8;

const BYTES_PER_MB: u64 = 1024 * 1024;
const COL_META_BUDGET_RATIO_NUMERATOR: u64 = 4;
const COL_META_BUDGET_RATIO_DENOMINATOR: u64 = 5;

#[cfg(feature = "parallel")]
fn should_parallelize_rows(row_len: usize, total_cells: usize) -> bool {
    row_len >= PAR_MIN_ROWS && total_cells >= PAR_MIN_CELLS
}

#[cfg(feature = "parallel")]
fn should_parallelize_cols(col_len: usize, total_cells: usize) -> bool {
    col_len >= PAR_MIN_COLS && total_cells >= PAR_MIN_CELLS
}

fn should_force_sequential_col_meta(grid: &Grid, config: &DiffConfig) -> bool {
    let Some(max_mb) = config.max_memory_mb else {
        return false;
    };
    let max_bytes = (max_mb as u64).saturating_mul(BYTES_PER_MB);
    let estimate = estimate_gridview_bytes(grid);
    estimate.saturating_mul(COL_META_BUDGET_RATIO_DENOMINATOR)
        >= max_bytes.saturating_mul(COL_META_BUDGET_RATIO_NUMERATOR)
}

#[cfg(feature = "parallel")]
fn sort_row_cells(rows: &mut [RowView<'_>], total_cells: usize) {
    if should_parallelize_rows(rows.len(), total_cells) {
        use rayon::prelude::*;
        rows.par_iter_mut()
            .for_each(|r| r.cells.sort_unstable_by_key(|(c, _)| *c));
        return;
    }

    for r in rows.iter_mut() {
        r.cells.sort_unstable_by_key(|(c, _)| *c);
    }
}

#[cfg(not(feature = "parallel"))]
fn sort_row_cells(rows: &mut [RowView<'_>], _total_cells: usize) {
    for r in rows.iter_mut() {
        r.cells.sort_unstable_by_key(|(c, _)| *c);
    }
}

#[cfg(feature = "parallel")]
fn build_row_meta<'a>(
    rows: &[RowView<'a>],
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
    _config: &DiffConfig,
    total_cells: usize,
) -> Vec<RowMeta> {
    if should_parallelize_rows(rows.len(), total_cells) {
        use rayon::prelude::*;
        return rows
            .par_iter()
            .enumerate()
            .map(|(idx, row_view)| {
                row_meta_for_row(idx, row_view, row_counts, row_first_non_blank)
            })
            .collect();
    }

    rows.iter()
        .enumerate()
        .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
        .collect()
}

#[cfg(not(feature = "parallel"))]
fn build_row_meta<'a>(
    rows: &[RowView<'a>],
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
    _config: &DiffConfig,
    _total_cells: usize,
) -> Vec<RowMeta> {
    rows.iter()
        .enumerate()
        .map(|(idx, row_view)| row_meta_for_row(idx, row_view, row_counts, row_first_non_blank))
        .collect()
}

fn build_col_meta_sequential<'a>(
    rows: &[RowView<'a>],
    col_counts: &[u32],
    col_first_non_blank: &[Option<u32>],
) -> Vec<ColMeta> {
    let ncols = col_counts.len();
    let mut col_hashers: Vec<Xxh3> = (0..ncols).map(|_| Xxh3::new()).collect();

    for row_view in rows {
        for (col, cell) in &row_view.cells {
            let idx = *col as usize;
            if idx >= ncols {
                continue;
            }
            let hasher = &mut col_hashers[idx];
            hash_cell_value(&cell.value, hasher);
            cell.formula.hash(hasher);
        }
    }

    (0..ncols)
        .map(|col_idx| ColMeta {
            col_idx: col_idx as u32,
            hash: ColSignature {
                hash: col_hashers[col_idx].digest128(),
            },
            non_blank_count: to_u16(col_counts[col_idx]),
            first_non_blank_row: col_first_non_blank[col_idx].map(to_u16).unwrap_or(0),
        })
        .collect()
}

#[cfg(feature = "parallel")]
fn build_col_meta<'a>(
    rows: &[RowView<'a>],
    col_counts: &[u32],
    col_first_non_blank: &[Option<u32>],
    total_cells: usize,
    allow_parallel: bool,
) -> Vec<ColMeta> {
    let ncols = col_counts.len();
    if !allow_parallel || !should_parallelize_cols(ncols, total_cells) {
        return build_col_meta_sequential(rows, col_counts, col_first_non_blank);
    }

    let mut col_cells: Vec<Vec<&'a Cell>> = (0..ncols)
        .map(|i| Vec::with_capacity(col_counts[i] as usize))
        .collect();

    for row_view in rows {
        for (col, cell) in &row_view.cells {
            let idx = *col as usize;
            if idx < ncols {
                col_cells[idx].push(*cell);
            }
        }
    }

    use rayon::prelude::*;
    let mut out: Vec<ColMeta> = col_cells
        .par_iter()
        .enumerate()
        .map(|(col_idx, cells)| {
            let mut hasher = Xxh3::new();
            for &cell in cells {
                hash_cell_value(&cell.value, &mut hasher);
                cell.formula.hash(&mut hasher);
            }
            ColMeta {
                col_idx: col_idx as u32,
                hash: ColSignature {
                    hash: hasher.digest128(),
                },
                non_blank_count: to_u16(col_counts[col_idx]),
                first_non_blank_row: col_first_non_blank[col_idx].map(to_u16).unwrap_or(0),
            }
        })
        .collect();

    out.sort_unstable_by_key(|m| m.col_idx);
    out
}

#[cfg(not(feature = "parallel"))]
fn build_col_meta<'a>(
    rows: &[RowView<'a>],
    col_counts: &[u32],
    col_first_non_blank: &[Option<u32>],
    _total_cells: usize,
    _allow_parallel: bool,
) -> Vec<ColMeta> {
    build_col_meta_sequential(rows, col_counts, col_first_non_blank)
}

fn row_meta_for_row<'a>(
    idx: usize,
    row_view: &RowView<'a>,
    row_counts: &[u32],
    row_first_non_blank: &[Option<u32>],
) -> RowMeta {
    let count = row_counts.get(idx).copied().unwrap_or(0);
    let non_blank_count = to_u16(count);
    let first_non_blank_col = row_first_non_blank
        .get(idx)
        .and_then(|c| c.map(to_u16))
        .unwrap_or(0);
    let is_low_info = compute_is_low_info(non_blank_count, row_view);

    let signature = RowSignature {
        hash: hash_row_content_128(&row_view.cells),
    };

    let frequency_class = if is_low_info {
        FrequencyClass::LowInfo
    } else {
        FrequencyClass::Common
    };

    RowMeta {
        row_idx: idx as u32,
        signature,
        non_blank_count,
        first_non_blank_col,
        frequency_class,
        is_low_info,
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
            (Some(CellValue::Text(id)), None) => id.0 == 0,
            (Some(CellValue::Number(_)), _) => false,
            (Some(CellValue::Bool(_)), _) => false,
            (Some(CellValue::Error(_)), _) => false,
            (Some(CellValue::Blank), _) => true,
            (None, None) => true,
        },
    }
}

fn to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
