mod common;

use common::{diff_fixture_pkgs, grid_from_numbers, single_sheet_workbook};
use excel_diff::{DiffConfig, DiffOp, Workbook, WorkbookPackage};

#[derive(Default)]
struct SawOps {
    sheet_added: bool,
    sheet_removed: bool,
    sheet_renamed: bool,

    row_added: bool,
    row_removed: bool,
    col_added: bool,
    col_removed: bool,

    block_moved_rows: bool,
    block_moved_cols: bool,
    block_moved_rect: bool,

    row_replaced: bool,
    rect_replaced: bool,

    cell_edited: bool,

    named_range: bool,
    chart: bool,
    vba: bool,
    query: bool,
}

impl SawOps {
    fn update_from_ops(&mut self, ops: &[DiffOp]) {
        for op in ops {
            match op {
                DiffOp::SheetAdded { .. } => self.sheet_added = true,
                DiffOp::SheetRemoved { .. } => self.sheet_removed = true,
                DiffOp::SheetRenamed { .. } => self.sheet_renamed = true,

                DiffOp::RowAdded { .. } => self.row_added = true,
                DiffOp::RowRemoved { .. } => self.row_removed = true,
                DiffOp::ColumnAdded { .. } => self.col_added = true,
                DiffOp::ColumnRemoved { .. } => self.col_removed = true,

                DiffOp::BlockMovedRows { .. } => self.block_moved_rows = true,
                DiffOp::BlockMovedColumns { .. } => self.block_moved_cols = true,
                DiffOp::BlockMovedRect { .. } => self.block_moved_rect = true,

                DiffOp::RowReplaced { .. } => self.row_replaced = true,
                DiffOp::RectReplaced { .. } => self.rect_replaced = true,

                DiffOp::CellEdited { .. } => self.cell_edited = true,

                DiffOp::NamedRangeAdded { .. }
                | DiffOp::NamedRangeRemoved { .. }
                | DiffOp::NamedRangeChanged { .. } => self.named_range = true,

                DiffOp::ChartAdded { .. }
                | DiffOp::ChartRemoved { .. }
                | DiffOp::ChartChanged { .. } => self.chart = true,

                DiffOp::VbaModuleAdded { .. }
                | DiffOp::VbaModuleRemoved { .. }
                | DiffOp::VbaModuleChanged { .. } => self.vba = true,

                DiffOp::QueryAdded { .. }
                | DiffOp::QueryRemoved { .. }
                | DiffOp::QueryRenamed { .. }
                | DiffOp::QueryDefinitionChanged { .. }
                | DiffOp::QueryMetadataChanged { .. } => self.query = true,

                _ => {}
            }
        }
    }
}

fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> excel_diff::DiffReport {
    WorkbookPackage::from(old.clone()).diff(&WorkbookPackage::from(new.clone()), config)
}

#[test]
fn coverage_audit_major_diff_op_categories_are_reachable() {
    let mut saw = SawOps::default();
    let cfg = DiffConfig::default();

    let reports = [
        diff_fixture_pkgs("pg6_sheet_added_a.xlsx", "pg6_sheet_added_b.xlsx", &cfg),
        diff_fixture_pkgs("pg6_sheet_removed_a.xlsx", "pg6_sheet_removed_b.xlsx", &cfg),
        diff_fixture_pkgs("pg6_sheet_renamed_a.xlsx", "pg6_sheet_renamed_b.xlsx", &cfg),
        diff_fixture_pkgs("row_append_bottom_a.xlsx", "row_append_bottom_b.xlsx", &cfg),
        diff_fixture_pkgs("row_delete_bottom_a.xlsx", "row_delete_bottom_b.xlsx", &cfg),
        diff_fixture_pkgs("col_append_right_a.xlsx", "col_append_right_b.xlsx", &cfg),
        diff_fixture_pkgs("col_delete_right_a.xlsx", "col_delete_right_b.xlsx", &cfg),
        diff_fixture_pkgs("row_block_move_a.xlsx", "row_block_move_b.xlsx", &cfg),
        diff_fixture_pkgs("column_move_a.xlsx", "column_move_b.xlsx", &cfg),
        diff_fixture_pkgs("rect_block_move_a.xlsx", "rect_block_move_b.xlsx", &cfg),
        diff_fixture_pkgs("multi_cell_edits_a.xlsx", "multi_cell_edits_b.xlsx", &cfg),
        diff_fixture_pkgs("named_ranges_a.xlsx", "named_ranges_b.xlsx", &cfg),
        diff_fixture_pkgs("charts_a.xlsx", "charts_b.xlsx", &cfg),
        diff_fixture_pkgs("vba_base.xlsm", "vba_changed.xlsm", &cfg),
        diff_fixture_pkgs("m_embedded_change_a.xlsx", "m_embedded_change_b.xlsx", &cfg),
    ];

    for report in &reports {
        saw.update_from_ops(&report.ops);
    }

    let row_replace_cfg = DiffConfig::builder()
        .dense_row_replace_ratio(0.5)
        .dense_row_replace_min_cols(1)
        .dense_rect_replace_min_rows(0)
        .build()
        .expect("valid config should build");
    let rect_replace_cfg = DiffConfig::builder()
        .dense_row_replace_ratio(0.5)
        .dense_row_replace_min_cols(1)
        .dense_rect_replace_min_rows(2)
        .build()
        .expect("valid config should build");

    let old_row = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2, 3, 4, 5]]));
    let new_row = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[10, 20, 30, 40, 50]]));
    let row_report = diff_workbooks(&old_row, &new_row, &row_replace_cfg);
    saw.update_from_ops(&row_report.ops);

    let old_rect = single_sheet_workbook("Sheet1", grid_from_numbers(&[&[1, 2, 3], &[4, 5, 6]]));
    let new_rect =
        single_sheet_workbook("Sheet1", grid_from_numbers(&[&[10, 20, 30], &[40, 50, 60]]));
    let rect_report = diff_workbooks(&old_rect, &new_rect, &rect_replace_cfg);
    saw.update_from_ops(&rect_report.ops);

    assert!(saw.sheet_added, "expected a SheetAdded op category");
    assert!(saw.sheet_removed, "expected a SheetRemoved op category");
    assert!(saw.sheet_renamed, "expected a SheetRenamed op category");

    assert!(saw.row_added, "expected a RowAdded op category");
    assert!(saw.row_removed, "expected a RowRemoved op category");
    assert!(saw.col_added, "expected a ColumnAdded op category");
    assert!(saw.col_removed, "expected a ColumnRemoved op category");

    assert!(
        saw.block_moved_rows,
        "expected a BlockMovedRows op category"
    );
    assert!(
        saw.block_moved_cols,
        "expected a BlockMovedColumns op category"
    );
    assert!(
        saw.block_moved_rect,
        "expected a BlockMovedRect op category"
    );

    assert!(saw.row_replaced, "expected a RowReplaced op category");
    assert!(saw.rect_replaced, "expected a RectReplaced op category");

    assert!(saw.cell_edited, "expected a CellEdited op category");

    assert!(saw.named_range, "expected a named range op category");
    assert!(saw.chart, "expected a chart op category");
    assert!(saw.vba, "expected a VBA module op category");
    assert!(saw.query, "expected a query op category");
}
