//! Core diffing engine for workbook comparison.
//!
//! Provides the main entry point [`diff_workbooks`] for comparing two workbooks
//! and generating a [`DiffReport`] of all changes.

use crate::column_alignment::{
    ColumnAlignment, ColumnBlockMove, align_single_column_change, detect_exact_column_block_move,
};
use crate::config::DiffConfig;
use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};
use crate::diff::{DiffOp, DiffReport, SheetId};
use crate::rect_block_move::{RectBlockMove, detect_exact_rect_block_move};
use crate::region_mask::RegionMask;
use crate::row_alignment::{
    RowAlignment, RowBlockMove, align_row_changes, detect_exact_row_block_move,
    detect_fuzzy_row_block_move,
};
use crate::workbook::{Cell, CellAddress, CellSnapshot, Grid, Sheet, SheetKind, Workbook};
use std::collections::{BTreeMap, HashMap};

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
    diff_workbooks_with_config(old, new, &DiffConfig::default())
}

pub fn diff_workbooks_with_config(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
) -> DiffReport {
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
                diff_grids_with_config(
                    &sheet_id,
                    &old_sheet.grid,
                    &new_sheet.grid,
                    config,
                    &mut ops,
                );
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
    diff_grids_with_config(sheet_id, old, new, &DiffConfig::default(), ops);
}

fn diff_grids_with_config(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
) {
    if old.nrows == 0 && new.nrows == 0 {
        return;
    }

    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);
    let mut iteration = 0;

    loop {
        if iteration >= config.max_move_iterations {
            break;
        }

        if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
            break;
        }

        let mut found_move = false;

        if let Some(mv) = detect_exact_rect_block_move_masked(old, new, &old_mask, &new_mask) {
            emit_rect_block_move(sheet_id, mv, ops);
            old_mask.exclude_rect_cells(
                mv.src_start_row,
                mv.src_row_count,
                mv.src_start_col,
                mv.src_col_count,
            );
            new_mask.exclude_rect_cells(
                mv.dst_start_row,
                mv.src_row_count,
                mv.dst_start_col,
                mv.src_col_count,
            );
            old_mask.exclude_rect_cells(
                mv.dst_start_row,
                mv.src_row_count,
                mv.dst_start_col,
                mv.src_col_count,
            );
            new_mask.exclude_rect_cells(
                mv.src_start_row,
                mv.src_row_count,
                mv.src_start_col,
                mv.src_col_count,
            );
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && let Some(mv) = detect_exact_row_block_move_masked(old, new, &old_mask, &new_mask)
        {
            emit_row_block_move(sheet_id, mv, ops);
            old_mask.exclude_rows(mv.src_start_row, mv.row_count);
            new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && let Some(mv) = detect_exact_column_block_move_masked(old, new, &old_mask, &new_mask)
        {
            emit_column_block_move(sheet_id, mv, ops);
            old_mask.exclude_cols(mv.src_start_col, mv.col_count);
            new_mask.exclude_cols(mv.dst_start_col, mv.col_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && let Some(mv) = detect_fuzzy_row_block_move_masked(old, new, &old_mask, &new_mask)
        {
            emit_row_block_move(sheet_id, mv, ops);
            emit_moved_row_block_edits(sheet_id, old, new, mv, ops);
            old_mask.exclude_rows(mv.src_start_row, mv.row_count);
            new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move {
            break;
        }
    }

    if old_mask.has_exclusions() || new_mask.has_exclusions() {
        if old.nrows != new.nrows || old.ncols != new.ncols {
            positional_diff_with_masks(sheet_id, old, new, &old_mask, &new_mask, ops);
        } else {
            positional_diff_masked_equal_size(sheet_id, old, new, &old_mask, &new_mask, ops);
        }
        return;
    }

    if let Some(alignment) = align_row_changes(old, new) {
        emit_aligned_diffs(sheet_id, old, new, &alignment, ops);
    } else if let Some(alignment) = align_single_column_change(old, new) {
        emit_column_aligned_diffs(sheet_id, old, new, &alignment, ops);
    } else {
        positional_diff(sheet_id, old, new, ops);
    }
}

fn cells_content_equal(a: Option<&Cell>, b: Option<&Cell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(cell_a), Some(cell_b)) => {
            cell_a.value == cell_b.value && cell_a.formula == cell_b.formula
        }
        (Some(cell_a), None) => cell_a.value.is_none() && cell_a.formula.is_none(),
        (None, Some(cell_b)) => cell_b.value.is_none() && cell_b.formula.is_none(),
    }
}

fn collect_differences_in_grid(old: &Grid, new: &Grid) -> Vec<(u32, u32)> {
    let mut diffs = Vec::new();

    for row in 0..old.nrows {
        for col in 0..old.ncols {
            if !cells_content_equal(old.get(row, col), new.get(row, col)) {
                diffs.push((row, col));
            }
        }
    }

    diffs
}

fn contiguous_ranges<I>(indices: I) -> Vec<(u32, u32)>
where
    I: IntoIterator<Item = u32>,
{
    let mut values: Vec<u32> = indices.into_iter().collect();
    if values.is_empty() {
        return Vec::new();
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

    ranges
}

fn group_rows_by_column_patterns(diffs: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if diffs.is_empty() {
        return Vec::new();
    }

    let mut row_to_cols: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (row, col) in diffs {
        row_to_cols.entry(*row).or_default().push(*col);
    }

    for cols in row_to_cols.values_mut() {
        cols.sort_unstable();
        cols.dedup();
    }

    let mut rows: Vec<u32> = row_to_cols.keys().copied().collect();
    rows.sort_unstable();

    let mut groups: Vec<(u32, u32)> = Vec::new();
    if let Some(&first_row) = rows.first() {
        let mut start = first_row;
        let mut prev = first_row;
        let mut current_cols = row_to_cols.get(&first_row).cloned().unwrap_or_default();

        for row in rows.into_iter().skip(1) {
            let cols = row_to_cols.get(&row).cloned().unwrap_or_default();
            if row == prev + 1 && cols == current_cols {
                prev = row;
            } else {
                groups.push((start, prev));
                start = row;
                prev = row;
                current_cols = cols;
            }
        }
        groups.push((start, prev));
    }

    groups
}

fn build_masked_grid(source: &Grid, mask: &RegionMask) -> (Grid, Vec<u32>, Vec<u32>) {
    let row_map: Vec<u32> = mask.active_rows().collect();
    let col_map: Vec<u32> = mask.active_cols().collect();

    let nrows = row_map.len() as u32;
    let ncols = col_map.len() as u32;

    let mut row_lookup: Vec<Option<u32>> = vec![None; source.nrows as usize];
    for (new_idx, old_row) in row_map.iter().enumerate() {
        row_lookup[*old_row as usize] = Some(new_idx as u32);
    }

    let mut col_lookup: Vec<Option<u32>> = vec![None; source.ncols as usize];
    for (new_idx, old_col) in col_map.iter().enumerate() {
        col_lookup[*old_col as usize] = Some(new_idx as u32);
    }

    let mut projected = Grid::new(nrows, ncols);

    for cell in source.cells.values() {
        if !mask.is_cell_active(cell.row, cell.col) {
            continue;
        }

        let Some(new_row) = row_lookup.get(cell.row as usize).and_then(|v| *v) else {
            continue;
        };
        let Some(new_col) = col_lookup.get(cell.col as usize).and_then(|v| *v) else {
            continue;
        };

        let mut new_cell = cell.clone();
        new_cell.row = new_row;
        new_cell.col = new_col;
        new_cell.address = CellAddress::from_indices(new_row, new_col);

        projected.cells.insert((new_row, new_col), new_cell);
    }

    (projected, row_map, col_map)
}

fn detect_exact_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<RowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_row_block_move(&old_proj, &new_proj)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(RowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}

fn detect_exact_column_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<ColumnBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, _, old_cols) = build_masked_grid(old, old_mask);
    let (new_proj, _, new_cols) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_column_block_move(&old_proj, &new_proj)?;
    let src_start_col = *old_cols.get(mv_local.src_start_col as usize)?;
    let dst_start_col = *new_cols.get(mv_local.dst_start_col as usize)?;

    Some(ColumnBlockMove {
        src_start_col,
        dst_start_col,
        col_count: mv_local.col_count,
    })
}

fn detect_exact_rect_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<RectBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, old_cols) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, new_cols) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let map_move = |mv_local: RectBlockMove,
                    row_map_old: &[u32],
                    row_map_new: &[u32],
                    col_map_old: &[u32],
                    col_map_new: &[u32]|
     -> Option<RectBlockMove> {
        let src_start_row = *row_map_old.get(mv_local.src_start_row as usize)?;
        let dst_start_row = *row_map_new.get(mv_local.dst_start_row as usize)?;
        let src_start_col = *col_map_old.get(mv_local.src_start_col as usize)?;
        let dst_start_col = *col_map_new.get(mv_local.dst_start_col as usize)?;

        Some(RectBlockMove {
            src_start_row,
            dst_start_row,
            src_start_col,
            dst_start_col,
            src_row_count: mv_local.src_row_count,
            src_col_count: mv_local.src_col_count,
            block_hash: mv_local.block_hash,
        })
    };

    if let Some(mv_local) = detect_exact_rect_block_move(&old_proj, &new_proj)
        && let Some(mapped) = map_move(mv_local, &old_rows, &new_rows, &old_cols, &new_cols)
    {
        return Some(mapped);
    }

    let diff_positions = collect_differences_in_grid(&old_proj, &new_proj);
    if diff_positions.is_empty() {
        return None;
    }

    let row_ranges = group_rows_by_column_patterns(&diff_positions);
    let col_ranges_full = contiguous_ranges(diff_positions.iter().map(|(_, c)| *c));
    let has_prior_exclusions = old_mask.has_exclusions() || new_mask.has_exclusions();
    if !has_prior_exclusions && row_ranges.len() <= 2 && col_ranges_full.len() <= 2 {
        return None;
    }

    let range_len = |range: (u32, u32)| range.1.saturating_sub(range.0).saturating_add(1);
    let in_range = |idx: u32, range: (u32, u32)| idx >= range.0 && idx <= range.1;
    let rectangles_match = |src_rows: (u32, u32),
                            src_cols: (u32, u32),
                            dst_rows: (u32, u32),
                            dst_cols: (u32, u32)|
     -> bool {
        let row_count = range_len(src_rows);
        let col_count = range_len(src_cols);

        for dr in 0..row_count {
            for dc in 0..col_count {
                let src_row = src_rows.0 + dr;
                let src_col = src_cols.0 + dc;
                let dst_row = dst_rows.0 + dr;
                let dst_col = dst_cols.0 + dc;

                if !cells_content_equal(
                    old_proj.get(src_row, src_col),
                    new_proj.get(dst_row, dst_col),
                ) {
                    return false;
                }
            }
        }

        true
    };

    for (row_idx, &row_a) in row_ranges.iter().enumerate() {
        for &row_b in row_ranges.iter().skip(row_idx + 1) {
            if range_len(row_a) != range_len(row_b) {
                continue;
            }

            let cols_row_a: Vec<u32> = diff_positions
                .iter()
                .filter_map(|(r, c)| if in_range(*r, row_a) { Some(*c) } else { None })
                .collect();
            let cols_row_b: Vec<u32> = diff_positions
                .iter()
                .filter_map(|(r, c)| if in_range(*r, row_b) { Some(*c) } else { None })
                .collect();
            let col_ranges_a = contiguous_ranges(cols_row_a);
            let col_ranges_b = contiguous_ranges(cols_row_b);
            let mut col_pairs: Vec<((u32, u32), (u32, u32))> = Vec::new();

            for &col_a in &col_ranges_a {
                for &col_b in &col_ranges_b {
                    if range_len(col_a) != range_len(col_b) {
                        continue;
                    }
                    col_pairs.push((col_a, col_b));
                }
            }

            if col_pairs.is_empty() {
                continue;
            }

            for (col_a, col_b) in col_pairs {
                let mut scoped_old_mask = RegionMask::all_active(old_proj.nrows, old_proj.ncols);
                let mut scoped_new_mask = RegionMask::all_active(new_proj.nrows, new_proj.ncols);

                for row in 0..old_proj.nrows {
                    if !in_range(row, row_a) && !in_range(row, row_b) {
                        scoped_old_mask.exclude_row(row);
                        scoped_new_mask.exclude_row(row);
                    }
                }

                for col in 0..old_proj.ncols {
                    if !in_range(col, col_a) && !in_range(col, col_b) {
                        scoped_old_mask.exclude_col(col);
                        scoped_new_mask.exclude_col(col);
                    }
                }

                let (old_scoped, scoped_old_rows, scoped_old_cols) =
                    build_masked_grid(&old_proj, &scoped_old_mask);
                let (new_scoped, scoped_new_rows, scoped_new_cols) =
                    build_masked_grid(&new_proj, &scoped_new_mask);

                if old_scoped.nrows != new_scoped.nrows || old_scoped.ncols != new_scoped.ncols {
                    continue;
                }

                if let Some(candidate) = detect_exact_rect_block_move(&old_scoped, &new_scoped) {
                    let scoped_row_map_old: Option<Vec<u32>> = scoped_old_rows
                        .iter()
                        .map(|idx| old_rows.get(*idx as usize).copied())
                        .collect();
                    let scoped_row_map_new: Option<Vec<u32>> = scoped_new_rows
                        .iter()
                        .map(|idx| new_rows.get(*idx as usize).copied())
                        .collect();
                    let scoped_col_map_old: Option<Vec<u32>> = scoped_old_cols
                        .iter()
                        .map(|idx| old_cols.get(*idx as usize).copied())
                        .collect();
                    let scoped_col_map_new: Option<Vec<u32>> = scoped_new_cols
                        .iter()
                        .map(|idx| new_cols.get(*idx as usize).copied())
                        .collect();

                    if let (
                        Some(row_map_old),
                        Some(row_map_new),
                        Some(col_map_old),
                        Some(col_map_new),
                    ) = (
                        scoped_row_map_old,
                        scoped_row_map_new,
                        scoped_col_map_old,
                        scoped_col_map_new,
                    ) && let Some(mapped) = map_move(
                        candidate,
                        &row_map_old,
                        &row_map_new,
                        &col_map_old,
                        &col_map_new,
                    ) {
                        return Some(mapped);
                    }
                }

                let row_len = range_len(row_a);
                let col_len = range_len(col_a);
                if row_len == 0 || col_len == 0 {
                    continue;
                }

                let candidates = [
                    (row_a, col_a, row_b, col_b),
                    (row_a, col_b, row_b, col_a),
                    (row_b, col_a, row_a, col_b),
                    (row_b, col_b, row_a, col_a),
                ];

                for (src_rows, src_cols, dst_rows, dst_cols) in candidates {
                    if range_len(src_rows) != range_len(dst_rows)
                        || range_len(src_cols) != range_len(dst_cols)
                    {
                        continue;
                    }

                    if rectangles_match(src_rows, src_cols, dst_rows, dst_cols) {
                        let mapped = RectBlockMove {
                            src_start_row: *old_rows.get(src_rows.0 as usize)?,
                            dst_start_row: *new_rows.get(dst_rows.0 as usize)?,
                            src_start_col: *old_cols.get(src_cols.0 as usize)?,
                            dst_start_col: *new_cols.get(dst_cols.0 as usize)?,
                            src_row_count: range_len(src_rows),
                            src_col_count: range_len(src_cols),
                            block_hash: None,
                        };
                        return Some(mapped);
                    }
                }
            }
        }
    }

    None
}

fn detect_fuzzy_row_block_move_masked(
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
) -> Option<RowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_fuzzy_row_block_move(&old_proj, &new_proj)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(RowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
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

fn positional_diff_with_masks(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    ops: &mut Vec<DiffOp>,
) {
    let overlap_rows = old.nrows.min(new.nrows);
    let overlap_cols = old.ncols.min(new.ncols);

    for row in 0..overlap_rows {
        for col in 0..overlap_cols {
            if !old_mask.is_cell_active(row, col) || !new_mask.is_cell_active(row, col) {
                continue;
            }
            let addr = CellAddress::from_indices(row, col);
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            if from != to {
                ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
            }
        }
    }

    if new.nrows > old.nrows {
        for row_idx in old.nrows..new.nrows {
            if new_mask.is_row_active(row_idx) {
                ops.push(DiffOp::row_added(sheet_id.clone(), row_idx, None));
            }
        }
    } else if old.nrows > new.nrows {
        for row_idx in new.nrows..old.nrows {
            if old_mask.is_row_active(row_idx) {
                ops.push(DiffOp::row_removed(sheet_id.clone(), row_idx, None));
            }
        }
    }

    if new.ncols > old.ncols {
        for col_idx in old.ncols..new.ncols {
            if new_mask.is_col_active(col_idx) {
                ops.push(DiffOp::column_added(sheet_id.clone(), col_idx, None));
            }
        }
    } else if old.ncols > new.ncols {
        for col_idx in new.ncols..old.ncols {
            if old_mask.is_col_active(col_idx) {
                ops.push(DiffOp::column_removed(sheet_id.clone(), col_idx, None));
            }
        }
    }
}

fn positional_diff_masked_equal_size(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    ops: &mut Vec<DiffOp>,
) {
    let row_shift_zone =
        compute_combined_shift_zone(old_mask.row_shift_bounds(), new_mask.row_shift_bounds());
    let col_shift_zone =
        compute_combined_shift_zone(old_mask.col_shift_bounds(), new_mask.col_shift_bounds());

    let stable_rows: Vec<u32> = (0..old.nrows)
        .filter(|&r| !is_in_zone(r, &row_shift_zone))
        .collect();
    let stable_cols: Vec<u32> = (0..old.ncols)
        .filter(|&c| !is_in_zone(c, &col_shift_zone))
        .collect();

    for &row in &stable_rows {
        for &col in &stable_cols {
            if !old_mask.is_cell_active(row, col) || !new_mask.is_cell_active(row, col) {
                continue;
            }
            let addr = CellAddress::from_indices(row, col);
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            if from != to {
                ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
            }
        }
    }
}

fn compute_combined_shift_zone(a: Option<(u32, u32)>, b: Option<(u32, u32)>) -> Option<(u32, u32)> {
    match (a, b) {
        (Some((a_min, a_max)), Some((b_min, b_max))) => Some((a_min.min(b_min), a_max.max(b_max))),
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}

fn is_in_zone(idx: u32, zone: &Option<(u32, u32)>) -> bool {
    match zone {
        Some((min, max)) => idx >= *min && idx <= *max,
        None => false,
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
