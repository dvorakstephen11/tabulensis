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
