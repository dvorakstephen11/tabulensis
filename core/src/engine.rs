//! Core diffing engine for workbook comparison.
//!
//! Provides the main entry point [`diff_workbooks`] for comparing two workbooks
//! and generating a [`DiffReport`] of all changes.

use crate::alignment::move_extraction::moves_from_matched_pairs;
use crate::alignment::{RowAlignment as AmrAlignment, align_rows_amr_with_signatures};
use crate::column_alignment::{
    ColumnAlignment, ColumnBlockMove, align_single_column_change_with_config,
    detect_exact_column_block_move_with_config,
};
use crate::config::{DiffConfig, LimitBehavior};
use crate::database_alignment::{KeyColumnSpec, diff_table_by_key};
use crate::diff::{DiffError, DiffOp, DiffReport, SheetId};
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::rect_block_move::{RectBlockMove, detect_exact_rect_block_move_with_config};
use crate::region_mask::RegionMask;
use crate::row_alignment::{
    RowAlignment as LegacyRowAlignment, RowBlockMove as LegacyRowBlockMove,
    align_row_changes_with_config, detect_exact_row_block_move_with_config,
    detect_fuzzy_row_block_move_with_config,
};
use crate::workbook::{
    Cell, CellAddress, CellSnapshot, ColSignature, Grid, RowSignature, Sheet, SheetKind, Workbook,
};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Default)]
struct DiffContext {
    warnings: Vec<String>,
}

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

pub fn try_diff_workbooks_with_config(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
) -> Result<DiffReport, DiffError> {
    let mut ops = Vec::new();
    let mut ctx = DiffContext::default();
    #[cfg(feature = "perf-metrics")]
    let mut metrics = {
        let mut m = DiffMetrics::default();
        m.start_phase(Phase::Total);
        m
    };

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
                try_diff_grids_with_config(
                    &sheet_id,
                    &old_sheet.grid,
                    &new_sheet.grid,
                    config,
                    &mut ops,
                    &mut ctx,
                    #[cfg(feature = "perf-metrics")]
                    Some(&mut metrics),
                )?;
            }
            (None, None) => unreachable!(),
        }
    }

    #[allow(unused_mut)]
    let mut report = DiffReport::new(ops);
    if !ctx.warnings.is_empty() {
        report.complete = false;
        report.warnings = ctx.warnings;
    }
    #[cfg(feature = "perf-metrics")]
    {
        metrics.end_phase(Phase::Total);
        report.metrics = Some(metrics);
    }
    Ok(report)
}

pub fn diff_workbooks_with_config(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
) -> DiffReport {
    match try_diff_workbooks_with_config(old, new, config) {
        Ok(report) => report,
        Err(e) => panic!("{}", e),
    }
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

            let old_cell = old.get(*row_a, col);
            let new_cell = new.get(*row_b, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(*row_b, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
        }
    }

    DiffReport::new(ops)
}

fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    let mut ctx = DiffContext::default();
    let _ = try_diff_grids_with_config(
        sheet_id,
        old,
        new,
        &DiffConfig::default(),
        ops,
        &mut ctx,
        #[cfg(feature = "perf-metrics")]
        None,
    );
}

fn try_diff_grids_with_config(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
    ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<(), DiffError> {
    if old.nrows == 0 && new.nrows == 0 {
        return Ok(());
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.rows_processed = m
            .rows_processed
            .saturating_add(old.nrows as u64)
            .saturating_add(new.nrows as u64);
        m.start_phase(Phase::MoveDetection);
    }

    let exceeds_limits = old.nrows.max(new.nrows) > config.max_align_rows
        || old.ncols.max(new.ncols) > config.max_align_cols;
    if exceeds_limits {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
        let warning = format!(
            "Sheet '{}': alignment limits exceeded (rows={}, cols={}; limits: rows={}, cols={})",
            sheet_id,
            old.nrows.max(new.nrows),
            old.ncols.max(new.ncols),
            config.max_align_rows,
            config.max_align_cols
        );
        match config.on_limit_exceeded {
            LimitBehavior::FallbackToPositional => {
                positional_diff(sheet_id, old, new, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnPartialResult => {
                ctx.warnings.push(warning);
                positional_diff(sheet_id, old, new, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                }
            }
            LimitBehavior::ReturnError => {
                return Err(DiffError::LimitsExceeded {
                    sheet: sheet_id.clone(),
                    rows: old.nrows.max(new.nrows),
                    cols: old.ncols.max(new.ncols),
                    max_rows: config.max_align_rows,
                    max_cols: config.max_align_cols,
                });
            }
        }
        return Ok(());
    }

    diff_grids_core(
        sheet_id,
        old,
        new,
        config,
        ops,
        ctx,
        #[cfg(feature = "perf-metrics")]
        metrics,
    );
    Ok(())
}

fn diff_grids_core(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
    _ctx: &mut DiffContext,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) {
    let mut old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let mut new_mask = RegionMask::all_active(new.nrows, new.ncols);
    let move_detection_enabled = old.nrows.max(new.nrows) <= config.recursive_align_threshold
        && old.ncols.max(new.ncols) <= 256;
    let mut iteration = 0;

    if move_detection_enabled {
        loop {
            if iteration >= config.max_move_iterations {
                break;
            }

            if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
                break;
            }

            let mut found_move = false;

            if let Some(mv) =
                detect_exact_rect_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_rect_block_move(sheet_id, mv, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
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
                && let Some(mv) =
                    detect_exact_row_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_row_block_move(sheet_id, mv, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && let Some(mv) =
                    detect_exact_column_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_column_block_move(sheet_id, mv, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                old_mask.exclude_cols(mv.src_start_col, mv.col_count);
                new_mask.exclude_cols(mv.dst_start_col, mv.col_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move
                && let Some(mv) =
                    detect_fuzzy_row_block_move_masked(old, new, &old_mask, &new_mask, config)
            {
                emit_row_block_move(sheet_id, mv, ops);
                emit_moved_row_block_edits(sheet_id, old, new, mv, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.moves_detected = m.moves_detected.saturating_add(1);
                }
                old_mask.exclude_rows(mv.src_start_row, mv.row_count);
                new_mask.exclude_rows(mv.dst_start_row, mv.row_count);
                iteration += 1;
                found_move = true;
            }

            if !found_move {
                break;
            }

            if old.nrows != new.nrows || old.ncols != new.ncols {
                break;
            }
        }

        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
    } else {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::MoveDetection);
        }
    }

    if old_mask.has_exclusions() || new_mask.has_exclusions() {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        if old.nrows != new.nrows || old.ncols != new.ncols {
            if diff_aligned_with_masks(sheet_id, old, new, &old_mask, &new_mask, ops) {
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.end_phase(Phase::CellDiff);
                }
                return;
            }
            positional_diff_with_masks(sheet_id, old, new, &old_mask, &new_mask, ops);
        } else {
            positional_diff_masked_equal_size(sheet_id, old, new, &old_mask, &new_mask, ops);
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::CellDiff);
        }
        return;
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::Alignment);
    }

    if let Some(amr_result) = align_rows_amr_with_signatures(old, new, config) {
        let mut alignment = amr_result.alignment;
        let row_signatures_old = amr_result.row_signatures_a;
        let row_signatures_new = amr_result.row_signatures_b;
        inject_moves_from_insert_delete(
            old,
            new,
            &mut alignment,
            &row_signatures_old,
            &row_signatures_new,
        );
        let has_structural_rows = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();
        if has_structural_rows && alignment.matched.is_empty() {
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.start_phase(Phase::CellDiff);
            }
            positional_diff(sheet_id, old, new, ops);
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.add_cells_compared(cells_in_overlap(old, new));
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return;
        }
        if has_structural_rows {
            let has_row_edits = alignment.matched.iter().any(|(a, b)| {
                row_signatures_old.get(*a as usize) != row_signatures_new.get(*b as usize)
            });
            if has_row_edits {
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.start_phase(Phase::CellDiff);
                }
                positional_diff(sheet_id, old, new, ops);
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.add_cells_compared(cells_in_overlap(old, new));
                    m.end_phase(Phase::CellDiff);
                }
                #[cfg(feature = "perf-metrics")]
                if let Some(m) = metrics.as_mut() {
                    m.end_phase(Phase::Alignment);
                }
                return;
            }
        }
        if alignment.moves.is_empty()
            && alignment.inserted.is_empty()
            && alignment.deleted.is_empty()
            && old.ncols != new.ncols
            && let Some(col_alignment) = align_single_column_change_with_config(old, new, config)
        {
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.start_phase(Phase::CellDiff);
            }
            emit_column_aligned_diffs(sheet_id, old, new, &col_alignment, ops);
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                let overlap_rows = old.nrows.min(new.nrows) as u64;
                m.add_cells_compared(
                    overlap_rows.saturating_mul(col_alignment.matched.len() as u64),
                );
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return;
        }
        let alignment_is_trivial_identity = alignment.moves.is_empty()
            && alignment.inserted.is_empty()
            && alignment.deleted.is_empty()
            && old.nrows == new.nrows
            && alignment.matched.len() as u32 == old.nrows
            && alignment.matched.iter().all(|(a, b)| a == b);

        if !alignment_is_trivial_identity
            && alignment.moves.is_empty()
            && row_signature_multiset_equal(old, new)
        {
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.start_phase(Phase::CellDiff);
            }
            positional_diff(sheet_id, old, new, ops);
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.add_cells_compared(cells_in_overlap(old, new));
                m.end_phase(Phase::CellDiff);
            }
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = metrics.as_mut() {
                m.end_phase(Phase::Alignment);
            }
            return;
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        emit_amr_aligned_diffs(sheet_id, old, new, &alignment, ops);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            let overlap_cols = old.ncols.min(new.ncols) as u64;
            m.add_cells_compared((alignment.matched.len() as u64).saturating_mul(overlap_cols));
            m.anchors_found = m
                .anchors_found
                .saturating_add(alignment.matched.len() as u32);
            m.moves_detected = m
                .moves_detected
                .saturating_add(alignment.moves.len() as u32);
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::CellDiff);
        }
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.end_phase(Phase::Alignment);
        }
        return;
    }

    if let Some(alignment) = align_row_changes_with_config(old, new, config) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        emit_aligned_diffs(sheet_id, old, new, &alignment, ops);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            let overlap_cols = old.ncols.min(new.ncols) as u64;
            m.add_cells_compared((alignment.matched.len() as u64).saturating_mul(overlap_cols));
            m.end_phase(Phase::CellDiff);
        }
    } else if let Some(alignment) = align_single_column_change_with_config(old, new, config) {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        emit_column_aligned_diffs(sheet_id, old, new, &alignment, ops);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            let overlap_rows = old.nrows.min(new.nrows) as u64;
            m.add_cells_compared(overlap_rows.saturating_mul(alignment.matched.len() as u64));
            m.end_phase(Phase::CellDiff);
        }
    } else {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        positional_diff(sheet_id, old, new, ops);
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.add_cells_compared(cells_in_overlap(old, new));
            m.end_phase(Phase::CellDiff);
        }
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.end_phase(Phase::Alignment);
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

fn row_signature_at(grid: &Grid, row: u32) -> Option<RowSignature> {
    if let Some(sig) = grid
        .row_signatures
        .as_ref()
        .and_then(|rows| rows.get(row as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_row_signature(row))
}

fn row_signature_counts(grid: &Grid) -> HashMap<RowSignature, u32> {
    if let Some(rows) = grid.row_signatures.as_ref() {
        let mut counts: HashMap<RowSignature, u32> = HashMap::with_capacity(rows.len());
        for &sig in rows {
            *counts.entry(sig).or_insert(0) += 1;
        }
        return counts;
    }

    use crate::hashing::hash_row_content_128;

    let nrows = grid.nrows as usize;
    let mut rows: Vec<Vec<(u32, &Cell)>> = vec![Vec::new(); nrows];

    for cell in grid.cells.values() {
        rows[cell.row as usize].push((cell.col, cell));
    }

    let mut counts: HashMap<RowSignature, u32> = HashMap::with_capacity(nrows);
    for mut row_cells in rows {
        row_cells.sort_by_key(|(col, _)| *col);
        let hash = hash_row_content_128(&row_cells);
        let sig = RowSignature { hash };
        *counts.entry(sig).or_insert(0) += 1;
    }

    counts
}

fn row_signature_multiset_equal(a: &Grid, b: &Grid) -> bool {
    if a.nrows != b.nrows {
        return false;
    }
    row_signature_counts(a) == row_signature_counts(b)
}

fn col_signature_at(grid: &Grid, col: u32) -> Option<ColSignature> {
    if let Some(sig) = grid
        .col_signatures
        .as_ref()
        .and_then(|cols| cols.get(col as usize))
    {
        return Some(*sig);
    }
    Some(grid.compute_col_signature(col))
}

fn align_indices_by_signature<T: Copy + Eq>(
    idx_a: &[u32],
    idx_b: &[u32],
    sig_a: impl Fn(u32) -> Option<T>,
    sig_b: impl Fn(u32) -> Option<T>,
) -> Option<(Vec<u32>, Vec<u32>)> {
    if idx_a.is_empty() || idx_b.is_empty() {
        return None;
    }

    if idx_a.len() == idx_b.len() {
        return Some((idx_a.to_vec(), idx_b.to_vec()));
    }

    let (short, long, short_is_a) = if idx_a.len() <= idx_b.len() {
        (idx_a, idx_b, true)
    } else {
        (idx_b, idx_a, false)
    };

    let diff = long.len() - short.len();
    let mut best_offset = 0usize;
    let mut best_matches = 0usize;

    for offset in 0..=diff {
        let mut matches = 0usize;
        for (i, &short_idx) in short.iter().enumerate() {
            let long_idx = long[offset + i];
            let (sig_short, sig_long) = if short_is_a {
                (sig_a(short_idx), sig_b(long_idx))
            } else {
                (sig_b(short_idx), sig_a(long_idx))
            };
            if let (Some(sa), Some(sb)) = (sig_short, sig_long)
                && sa == sb
            {
                matches += 1;
            }
        }
        if matches > best_matches {
            best_matches = matches;
            best_offset = offset;
        }
    }

    if short_is_a {
        let aligned_b = long[best_offset..best_offset + short.len()].to_vec();
        Some((idx_a.to_vec(), aligned_b))
    } else {
        let aligned_a = long[best_offset..best_offset + short.len()].to_vec();
        Some((aligned_a, idx_b.to_vec()))
    }
}

fn inject_moves_from_insert_delete(
    old: &Grid,
    new: &Grid,
    alignment: &mut AmrAlignment,
    row_signatures_old: &[RowSignature],
    row_signatures_new: &[RowSignature],
) {
    if alignment.inserted.is_empty() || alignment.deleted.is_empty() {
        return;
    }

    let mut deleted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.deleted {
        let sig = row_signatures_old
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(old, *row));
        if let Some(sig) = sig {
            deleted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut inserted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.inserted {
        let sig = row_signatures_new
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(new, *row));
        if let Some(sig) = sig {
            inserted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut matched_pairs = Vec::new();
    for (sig, deleted_rows) in deleted_by_sig.iter() {
        if deleted_rows.len() != 1 {
            continue;
        }
        if let Some(insert_rows) = inserted_by_sig.get(sig) {
            if insert_rows.len() != 1 {
                continue;
            }
            matched_pairs.push((deleted_rows[0], insert_rows[0]));
        }
    }

    if matched_pairs.is_empty() {
        return;
    }

    let new_moves = moves_from_matched_pairs(&matched_pairs);
    if new_moves.is_empty() {
        return;
    }

    let mut moved_src = HashSet::new();
    let mut moved_dst = HashSet::new();
    for mv in &new_moves {
        for r in mv.src_start_row..mv.src_start_row.saturating_add(mv.row_count) {
            moved_src.insert(r);
        }
        for r in mv.dst_start_row..mv.dst_start_row.saturating_add(mv.row_count) {
            moved_dst.insert(r);
        }
    }

    alignment.deleted.retain(|r| !moved_src.contains(r));
    alignment.inserted.retain(|r| !moved_dst.contains(r));
    alignment.moves.extend(new_moves);
    alignment
        .moves
        .sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));
}

fn build_projected_grid_from_maps(
    source: &Grid,
    mask: &RegionMask,
    row_map: &[u32],
    col_map: &[u32],
) -> (Grid, Vec<u32>, Vec<u32>) {
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

    (projected, row_map.to_vec(), col_map.to_vec())
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
    config: &DiffConfig,
) -> Option<LegacyRowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_row_block_move_with_config(&old_proj, &new_proj, config)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(LegacyRowBlockMove {
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
    config: &DiffConfig,
) -> Option<ColumnBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, _, old_cols) = build_masked_grid(old, old_mask);
    let (new_proj, _, new_cols) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_exact_column_block_move_with_config(&old_proj, &new_proj, config)?;
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
    config: &DiffConfig,
) -> Option<RectBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let aligned_rows = align_indices_by_signature(
        &old_mask.active_rows().collect::<Vec<_>>(),
        &new_mask.active_rows().collect::<Vec<_>>(),
        |r| row_signature_at(old, r),
        |r| row_signature_at(new, r),
    )?;
    let aligned_cols = align_indices_by_signature(
        &old_mask.active_cols().collect::<Vec<_>>(),
        &new_mask.active_cols().collect::<Vec<_>>(),
        |c| col_signature_at(old, c),
        |c| col_signature_at(new, c),
    )?;
    let (old_proj, old_rows, old_cols) =
        build_projected_grid_from_maps(old, old_mask, &aligned_rows.0, &aligned_cols.0);
    let (new_proj, new_rows, new_cols) =
        build_projected_grid_from_maps(new, new_mask, &aligned_rows.1, &aligned_cols.1);

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

    if let Some(mv_local) = detect_exact_rect_block_move_with_config(&old_proj, &new_proj, config)
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

                if let Some(candidate) =
                    detect_exact_rect_block_move_with_config(&old_scoped, &new_scoped, config)
                {
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
    config: &DiffConfig,
) -> Option<LegacyRowBlockMove> {
    if !old_mask.has_active_cells() || !new_mask.has_active_cells() {
        return None;
    }

    let (old_proj, old_rows, _) = build_masked_grid(old, old_mask);
    let (new_proj, new_rows, _) = build_masked_grid(new, new_mask);

    if old_proj.nrows != new_proj.nrows || old_proj.ncols != new_proj.ncols {
        return None;
    }

    let mv_local = detect_fuzzy_row_block_move_with_config(&old_proj, &new_proj, config)?;
    let src_start_row = *old_rows.get(mv_local.src_start_row as usize)?;
    let dst_start_row = *new_rows.get(mv_local.dst_start_row as usize)?;

    Some(LegacyRowBlockMove {
        src_start_row,
        dst_start_row,
        row_count: mv_local.row_count,
    })
}

#[cfg(feature = "perf-metrics")]
fn cells_in_overlap(old: &Grid, new: &Grid) -> u64 {
    let overlap_rows = old.nrows.min(new.nrows) as u64;
    let overlap_cols = old.ncols.min(new.ncols) as u64;
    overlap_rows.saturating_mul(overlap_cols)
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

fn diff_aligned_with_masks(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    old_mask: &RegionMask,
    new_mask: &RegionMask,
    ops: &mut Vec<DiffOp>,
) -> bool {
    let old_rows: Vec<u32> = old_mask.active_rows().collect();
    let new_rows: Vec<u32> = new_mask.active_rows().collect();
    let old_cols: Vec<u32> = old_mask.active_cols().collect();
    let new_cols: Vec<u32> = new_mask.active_cols().collect();

    let Some((rows_a, rows_b)) = align_indices_by_signature(
        &old_rows,
        &new_rows,
        |r| row_signature_at(old, r),
        |r| row_signature_at(new, r),
    ) else {
        return false;
    };

    let (cols_a, cols_b) = align_indices_by_signature(
        &old_cols,
        &new_cols,
        |c| col_signature_at(old, c),
        |c| col_signature_at(new, c),
    )
    .unwrap_or((old_cols.clone(), new_cols.clone()));

    if rows_a.len() != rows_b.len() || cols_a.len() != cols_b.len() {
        return false;
    }

    for (row_a, row_b) in rows_a.iter().zip(rows_b.iter()) {
        for (col_a, col_b) in cols_a.iter().zip(cols_b.iter()) {
            if !old_mask.is_cell_active(*row_a, *col_a) || !new_mask.is_cell_active(*row_b, *col_b)
            {
                continue;
            }
            let old_cell = old.get(*row_a, *col_a);
            let new_cell = new.get(*row_b, *col_b);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(*row_b, *col_b);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
        }
    }

    let rows_a_set: HashSet<u32> = rows_a.iter().copied().collect();
    let rows_b_set: HashSet<u32> = rows_b.iter().copied().collect();

    for row_idx in new_rows.iter().filter(|r| !rows_b_set.contains(r)) {
        if new_mask.is_row_active(*row_idx) {
            ops.push(DiffOp::row_added(sheet_id.clone(), *row_idx, None));
        }
    }

    for row_idx in old_rows.iter().filter(|r| !rows_a_set.contains(r)) {
        if old_mask.is_row_active(*row_idx) {
            ops.push(DiffOp::row_removed(sheet_id.clone(), *row_idx, None));
        }
    }

    let cols_a_set: HashSet<u32> = cols_a.iter().copied().collect();
    let cols_b_set: HashSet<u32> = cols_b.iter().copied().collect();

    for col_idx in new_cols.iter().filter(|c| !cols_b_set.contains(c)) {
        if new_mask.is_col_active(*col_idx) {
            ops.push(DiffOp::column_added(sheet_id.clone(), *col_idx, None));
        }
    }

    for col_idx in old_cols.iter().filter(|c| !cols_a_set.contains(c)) {
        if old_mask.is_col_active(*col_idx) {
            ops.push(DiffOp::column_removed(sheet_id.clone(), *col_idx, None));
        }
    }

    true
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
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
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
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, col);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
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

fn emit_row_block_move(sheet_id: &SheetId, mv: LegacyRowBlockMove, ops: &mut Vec<DiffOp>) {
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
    mv: LegacyRowBlockMove,
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
    alignment: &LegacyRowAlignment,
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

fn emit_amr_aligned_diffs(
    sheet_id: &SheetId,
    old: &Grid,
    new: &Grid,
    alignment: &AmrAlignment,
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

    for mv in &alignment.moves {
        ops.push(DiffOp::BlockMovedRows {
            sheet: sheet_id.clone(),
            src_start_row: mv.src_start_row,
            row_count: mv.row_count,
            dst_start_row: mv.dst_start_row,
            block_hash: None,
        });
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
        let old_cell = old.get(row_a, col);
        let new_cell = new.get(row_b, col);

        if cells_content_equal(old_cell, new_cell) {
            continue;
        }

        let addr = CellAddress::from_indices(row_b, col);
        let from = snapshot_with_addr(old_cell, addr);
        let to = snapshot_with_addr(new_cell, addr);

        ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
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
            let old_cell = old.get(row, *col_a);
            let new_cell = new.get(row, *col_b);

            if cells_content_equal(old_cell, new_cell) {
                continue;
            }

            let addr = CellAddress::from_indices(row, *col_b);
            let from = snapshot_with_addr(old_cell, addr);
            let to = snapshot_with_addr(new_cell, addr);

            ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
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
