use crate::diff::{DiffOp, DiffReport, SheetId};
use crate::workbook::{CellAddress, CellSnapshot, Grid, Sheet, SheetKind, Workbook};
use std::collections::HashMap;

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
