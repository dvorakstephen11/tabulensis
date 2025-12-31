use crate::grid_view::{ColMeta, RowMeta, RowView};
use crate::workbook::{Cell, Grid};
use std::mem::size_of;

pub(crate) fn estimate_gridview_bytes(grid: &Grid) -> u64 {
    let nrows = grid.nrows as u64;
    let ncols = grid.ncols as u64;
    let cell_count = grid.cell_count() as u64;

    let row_view_bytes = nrows.saturating_mul(size_of::<RowView<'static>>() as u64);
    let row_meta_bytes = nrows.saturating_mul(size_of::<RowMeta>() as u64);
    let col_meta_bytes = ncols.saturating_mul(size_of::<ColMeta>() as u64);

    let cell_entry_bytes = cell_count.saturating_mul(size_of::<(u32, &'static Cell)>() as u64);

    let build_row_counts_bytes = nrows
        .saturating_mul(size_of::<u32>() as u64)
        .saturating_add(nrows.saturating_mul(size_of::<Option<u32>>() as u64));
    let build_col_counts_bytes = ncols
        .saturating_mul(size_of::<u32>() as u64)
        .saturating_add(ncols.saturating_mul(size_of::<Option<u32>>() as u64));
    let build_hashers_bytes =
        ncols.saturating_mul(size_of::<xxhash_rust::xxh3::Xxh3>() as u64);

    row_view_bytes
        .saturating_add(row_meta_bytes)
        .saturating_add(col_meta_bytes)
        .saturating_add(cell_entry_bytes)
        .saturating_add(build_row_counts_bytes)
        .saturating_add(build_col_counts_bytes)
        .saturating_add(build_hashers_bytes)
}

pub(crate) fn estimate_advanced_sheet_diff_peak(old: &Grid, new: &Grid) -> u64 {
    let base = estimate_gridview_bytes(old).saturating_add(estimate_gridview_bytes(new));
    let max_rows = old.nrows.max(new.nrows) as u64;
    let max_cols = old.ncols.max(new.ncols) as u64;

    let alignment_overhead = max_rows
        .saturating_add(max_cols)
        .saturating_mul(size_of::<u32>() as u64)
        .saturating_mul(8);

    base.saturating_add(alignment_overhead)
}
