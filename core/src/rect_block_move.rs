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
