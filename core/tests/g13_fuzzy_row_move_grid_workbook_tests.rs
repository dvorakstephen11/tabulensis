use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers, single_sheet_workbook};

#[test]
fn g13_fuzzy_row_move_emits_blockmovedrows_and_celledited() {
    let wb_a = open_workbook(fixture_path("grid_move_and_edit_a.xlsx"))
        .expect("failed to open fixture: grid_move_and_edit_a.xlsx");
    let wb_b = open_workbook(fixture_path("grid_move_and_edit_b.xlsx"))
        .expect("failed to open fixture: grid_move_and_edit_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());

    let block_moves: Vec<(u32, u32, u32, Option<u64>)> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                block_hash,
                ..
            } => Some((*src_start_row, *row_count, *dst_start_row, *block_hash)),
            _ => None,
        })
        .collect();

    assert_eq!(block_moves.len(), 1, "expected a single BlockMovedRows op");
    let (src_start_row, row_count, dst_start_row, block_hash) = block_moves[0];
    assert_eq!(src_start_row, 4);
    assert_eq!(row_count, 4);
    assert_eq!(dst_start_row, 13);
    assert!(block_hash.is_none());

    let edited_rows: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::CellEdited { addr, .. } => Some(addr.row),
            _ => None,
        })
        .collect();
    assert!(
        edited_rows
            .iter()
            .any(|r| *r >= dst_start_row && *r < dst_start_row + row_count),
        "expected a CellEdited inside the moved block"
    );

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowAdded { row_idx, .. } if *row_idx >= dst_start_row && *row_idx < dst_start_row + row_count)),
        "moved rows must not be reported as added"
    );
    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::RowRemoved { row_idx, .. } if *row_idx >= src_start_row && *row_idx < src_start_row + row_count)),
        "moved rows must not be reported as removed"
    );
}

#[test]
fn g13_fuzzy_row_move_can_be_disabled() {
    let base: Vec<Vec<i32>> = (1..=18)
        .map(|r| (1..=3).map(|c| r * 10 + c).collect())
        .collect();
    let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
    let grid_a = grid_from_numbers(&base_refs);

    let mut rows_b = base.clone();
    let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    moved_block[1][1] = 9_999;
    rows_b.splice(12..12, moved_block);
    let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
    let grid_b = grid_from_numbers(&rows_b_refs);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let mut disabled = excel_diff::DiffConfig::default();
    disabled.enable_fuzzy_moves = false;
    let report_disabled = diff_workbooks(&wb_a, &wb_b, &disabled);
    let disabled_moves = report_disabled
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRows { .. }))
        .count();
    let disabled_block_edits = report_disabled
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::CellEdited { addr, .. }
                if addr.row >= 12 && addr.row < 16
            )
        })
        .count();

    let report_enabled = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());
    let enabled_moves = report_enabled
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRows { .. }))
        .count();
    let enabled_block_edits = report_enabled
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::CellEdited { addr, .. }
                if addr.row >= 12 && addr.row < 16
            )
        })
        .count();

    assert!(
        enabled_moves >= disabled_moves,
        "enabling fuzzy moves should not reduce move detection"
    );
    assert!(
        enabled_block_edits > disabled_block_edits,
        "fuzzy move detection should emit edits within the moved block"
    );
}

#[test]
fn g13_in_place_edits_do_not_emit_blockmovedrows() {
    let rows: Vec<Vec<i32>> = (1..=12)
        .map(|r| (1..=3).map(|c| r * 10 + c).collect())
        .collect();
    let rows_refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&rows_refs);

    let mut edited_rows = rows.clone();
    if let Some(cell) = edited_rows.get_mut(5).and_then(|row| row.get_mut(1)) {
        *cell += 7;
    }
    let edited_refs: Vec<&[i32]> = edited_rows.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&edited_refs);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "in-place edits must not be classified as BlockMovedRows"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "edits should still be surfaced as CellEdited"
    );
}

#[test]
fn g13_ambiguous_repeated_blocks_do_not_emit_blockmovedrows() {
    let mut rows_a: Vec<Vec<i32>> = vec![vec![1, 1]; 10];
    rows_a.push(vec![99, 99]);
    rows_a.push(vec![2, 2]);

    let mut rows_b = rows_a.clone();
    let moved = rows_b.remove(10);
    rows_b.insert(3, moved);

    let refs_a: Vec<&[i32]> = rows_a.iter().map(|r| r.as_slice()).collect();
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs_a);
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b, &excel_diff::DiffConfig::default());

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "ambiguous repeated patterns should not emit BlockMovedRows"
    );
    assert!(
        !report.ops.is_empty(),
        "fallback path should produce some diff noise"
    );
}
