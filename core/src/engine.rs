use crate::diff::{DiffOp, DiffReport, SheetId};
use crate::workbook::{CellAddress, CellSnapshot, Grid, Sheet, Workbook};
use std::collections::HashMap;

pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport {
    let mut ops = Vec::new();

    let old_sheets: HashMap<&str, &Sheet> =
        old.sheets.iter().map(|s| (s.name.as_str(), s)).collect();
    let new_sheets: HashMap<&str, &Sheet> =
        new.sheets.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut all_names: Vec<&str> = old_sheets
        .keys()
        .chain(new_sheets.keys())
        .copied()
        .collect();
    all_names.sort_unstable();
    all_names.dedup();

    for name in all_names {
        let sheet_id: SheetId = name.to_string();

        match (old_sheets.get(name), new_sheets.get(name)) {
            (None, Some(_)) => {
                ops.push(DiffOp::SheetAdded { sheet: sheet_id });
            }
            (Some(_), None) => {
                ops.push(DiffOp::SheetRemoved { sheet: sheet_id });
            }
            (Some(old_sheet), Some(new_sheet)) => {
                diff_grids(&sheet_id, &old_sheet.grid, &new_sheet.grid, &mut ops);
            }
            (None, None) => unreachable!(),
        }
    }

    DiffReport::new(ops)
}

fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    let max_rows = old.nrows.max(new.nrows);
    let max_cols = old.ncols.max(new.ncols);

    for row in 0..max_rows {
        for col in 0..max_cols {
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);

            let old_snapshot = old_cell.map(CellSnapshot::from_cell);
            let new_snapshot = new_cell.map(CellSnapshot::from_cell);

            if old_snapshot != new_snapshot {
                let addr = CellAddress::from_indices(row, col);
                let from = old_snapshot.unwrap_or_else(|| CellSnapshot::empty(addr));
                let to = new_snapshot.unwrap_or_else(|| CellSnapshot::empty(addr));

                ops.push(DiffOp::cell_edited(sheet_id.clone(), addr, from, to));
            }
        }
    }
}
