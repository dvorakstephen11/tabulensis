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
