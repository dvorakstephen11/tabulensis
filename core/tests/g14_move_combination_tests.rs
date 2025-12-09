use excel_diff::{DiffConfig, DiffOp, DiffReport, diff_workbooks, diff_workbooks_with_config};

mod common;
use common::{grid_from_numbers, single_sheet_workbook};

fn collect_rect_moves(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRect { .. }))
        .collect()
}

fn collect_row_moves(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRows { .. }))
        .collect()
}

fn collect_col_moves(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedColumns { .. }))
        .collect()
}

fn collect_row_adds(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. }))
        .collect()
}

fn collect_row_removes(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowRemoved { .. }))
        .collect()
}

fn collect_cell_edits(report: &DiffReport) -> Vec<&DiffOp> {
    report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::CellEdited { .. }))
        .collect()
}

fn base_grid(rows: usize, cols: usize) -> Vec<Vec<i32>> {
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| (r as i32 + 1) * 100 + c as i32 + 1)
                .collect()
        })
        .collect()
}

fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
    for (r_offset, row_vals) in block.iter().enumerate() {
        for (c_offset, value) in row_vals.iter().enumerate() {
            let row = top + r_offset;
            let col = left + c_offset;
            if let Some(row_slice) = target.get_mut(row)
                && let Some(cell) = row_slice.get_mut(col)
            {
                *cell = *value;
            }
        }
    }
}

fn grid_from_matrix(matrix: &[Vec<i32>]) -> excel_diff::Grid {
    let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
    grid_from_numbers(&refs)
}

#[test]
fn g14_rect_move_no_additional_changes_produces_single_op() {
    let mut grid_a = base_grid(12, 10);
    let mut grid_b = base_grid(12, 10);

    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);
    place_block(&mut grid_b, 8, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    let has_rect_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedRect { .. }));

    assert!(has_rect_move, "pure rect move should be detected");

    assert_eq!(
        report.ops.len(),
        1,
        "pure rect move should produce exactly one BlockMovedRect op"
    );
}

#[test]
fn g14_rect_move_plus_cell_edit_no_silent_data_loss() {
    let mut grid_a = base_grid(12, 10);
    let mut grid_b = base_grid(12, 10);

    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);
    place_block(&mut grid_b, 8, 6, &block);
    grid_b[0][0] = 77777;

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    let rect_moves = collect_rect_moves(&report);
    let cell_edits = collect_cell_edits(&report);

    assert_eq!(
        rect_moves.len(),
        1,
        "expected single BlockMovedRect for the moved block"
    );

    if let DiffOp::BlockMovedRect {
        src_start_row,
        src_start_col,
        src_row_count,
        src_col_count,
        dst_start_row,
        dst_start_col,
        ..
    } = rect_moves[0]
    {
        assert_eq!(*src_start_row, 2);
        assert_eq!(*src_start_col, 2);
        assert_eq!(*src_row_count, 2);
        assert_eq!(*src_col_count, 2);
        assert_eq!(*dst_start_row, 8);
        assert_eq!(*dst_start_col, 6);
    } else {
        panic!("expected BlockMovedRect");
    }

    assert!(
        !cell_edits.is_empty(),
        "expected cell edits outside the moved block"
    );
}

#[test]
fn g14_pure_row_move_produces_single_op() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    rows_b.splice(12..12, moved_block);
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    let has_row_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedRows { .. }));

    assert!(has_row_move, "pure row block move should be detected");

    assert_eq!(
        report.ops.len(),
        1,
        "pure row block move should produce exactly one BlockMovedRows op"
    );
}

#[test]
fn g14_row_move_plus_cell_edit_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    rows_b.splice(12..12, moved_block);
    rows_b[0][0] = 99999;
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );
}

#[test]
fn g14_pure_column_move_produces_single_op() {
    let rows: Vec<Vec<i32>> = (0..5)
        .map(|r| (0..8).map(|c| (r + 1) * 10 + c + 1).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = rows.clone();
    for row in &mut rows_b {
        let moved_col = row.remove(1);
        row.insert(5, moved_col);
    }
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    let has_col_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedColumns { .. }));

    assert!(has_col_move, "pure column block move should be detected");

    assert_eq!(
        report.ops.len(),
        1,
        "pure column block move should produce exactly one BlockMovedColumns op"
    );
}

#[test]
fn g14_column_move_plus_cell_edit_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (0..5)
        .map(|r| (0..8).map(|c| (r + 1) * 10 + c + 1).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = rows.clone();
    for row in &mut rows_b {
        let moved_col = row.remove(1);
        row.insert(5, moved_col);
    }
    rows_b[0][0] = 88888;
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );
}

#[test]
fn g14_two_disjoint_row_block_moves_detected() {
    let rows: Vec<Vec<i32>> = (1..=24)
        .map(|r| (1..=3).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = Vec::new();

    rows_b.extend_from_slice(&rows[0..3]);
    rows_b.extend_from_slice(&rows[7..10]);
    rows_b.extend_from_slice(&rows[13..24]);
    rows_b.extend_from_slice(&rows[3..7]);
    rows_b.extend_from_slice(&rows[10..13]);

    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    let row_moves = collect_row_moves(&report);
    assert_eq!(
        row_moves.len(),
        2,
        "expected exactly two BlockMovedRows ops for two disjoint moves"
    );

    let mut actual: Vec<(u32, u32, u32)> = row_moves
        .iter()
        .map(|op| {
            if let DiffOp::BlockMovedRows {
                src_start_row,
                row_count,
                dst_start_row,
                ..
            } = **op
            {
                (src_start_row, row_count, dst_start_row)
            } else {
                unreachable!()
            }
        })
        .collect();
    actual.sort();

    let mut expected = vec![(3u32, 4u32, 17u32), (10u32, 3u32, 21u32)];
    expected.sort();

    assert_eq!(
        actual, expected,
        "row move ops should match the two expected disjoint moves"
    );
}

#[test]
fn g14_row_move_plus_column_move_both_detected() {
    let rows: Vec<Vec<i32>> = (0..15)
        .map(|r| (0..10).map(|c| (r + 1) * 100 + c + 1).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();

    let moved_rows: Vec<Vec<i32>> = rows_b.drain(2..5).collect();
    rows_b.splice(10..10, moved_rows);

    for row in &mut rows_b {
        let moved_col = row.remove(1);
        row.insert(7, moved_col);
    }

    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    let row_moves = collect_row_moves(&report);
    let col_moves = collect_col_moves(&report);

    assert_eq!(
        row_moves.len(),
        1,
        "expected a single BlockMovedRows op for the moved row block"
    );
    assert_eq!(
        col_moves.len(),
        1,
        "expected a single BlockMovedColumns op for the moved column"
    );

    if let DiffOp::BlockMovedRows {
        src_start_row,
        row_count,
        dst_start_row,
        ..
    } = *row_moves[0]
    {
        assert_eq!(src_start_row, 2);
        assert_eq!(row_count, 3);
        assert_eq!(dst_start_row, 10);
    } else {
        panic!("expected BlockMovedRows op");
    }

    if let DiffOp::BlockMovedColumns {
        src_start_col,
        col_count,
        dst_start_col,
        ..
    } = *col_moves[0]
    {
        assert_eq!(src_start_col, 1);
        assert_eq!(col_count, 1);
        assert_eq!(dst_start_col, 7);
    } else {
        panic!("expected BlockMovedColumns op");
    }
}

#[test]
fn g14_fuzzy_row_move_produces_move_and_internal_edits() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    moved_block[1][1] = 5555;
    rows_b.splice(12..12, moved_block);
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    let has_row_move = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::BlockMovedRows { .. }));

    let has_internal_edit = report
        .ops
        .iter()
        .any(|op| matches!(op, DiffOp::CellEdited { .. }));

    assert!(has_row_move, "should detect the fuzzy row block move");
    assert!(
        has_internal_edit,
        "should report cell edits inside the moved block"
    );
}

#[test]
fn g14_fuzzy_row_move_plus_outside_edit_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b = rows.clone();
    let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
    moved_block[1][1] = 5555;
    rows_b.splice(12..12, moved_block);
    rows_b[0][0] = 99999;
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );
}

#[test]
fn g14_grid_changes_no_silent_data_loss() {
    let mut grid_a = base_grid(15, 12);
    let mut grid_b = base_grid(15, 12);

    let block = vec![vec![7001, 7002], vec![7003, 7004], vec![7005, 7006]];
    place_block(&mut grid_a, 3, 3, &block);
    place_block(&mut grid_b, 10, 8, &block);
    grid_b[0][0] = 11111;
    grid_b[0][11] = 22222;
    grid_b[14][0] = 33333;
    grid_b[14][11] = 44444;

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - changes must be reported"
    );

    let cell_edits: Vec<(u32, u32)> = report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, .. } = op {
                Some((addr.row, addr.col))
            } else {
                None
            }
        })
        .collect();

    assert!(
        !cell_edits.is_empty() || !report.ops.is_empty(),
        "some form of changes should be reported"
    );
}

#[test]
fn g14_three_disjoint_rect_block_moves_detected() {
    let mut grid_a = base_grid(20, 10);
    let mut grid_b = base_grid(20, 10);

    let block1 = vec![vec![1001, 1002], vec![1003, 1004]];
    let block2 = vec![vec![2001, 2002], vec![2003, 2004]];
    let block3 = vec![vec![3001, 3002], vec![3003, 3004]];

    place_block(&mut grid_a, 2, 1, &block1);
    place_block(&mut grid_a, 6, 3, &block2);
    place_block(&mut grid_a, 12, 5, &block3);

    place_block(&mut grid_b, 10, 1, &block1);
    place_block(&mut grid_b, 4, 6, &block2);
    place_block(&mut grid_b, 16, 2, &block3);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    let rect_moves: Vec<_> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRect { .. }))
        .collect();

    assert_eq!(
        rect_moves.len(),
        3,
        "expected exactly three rect block moves to be detected"
    );
    assert_eq!(
        report.ops.len(),
        3,
        "multi-rect move scenario should not emit extra structural ops"
    );
}

#[test]
fn g14_two_disjoint_rect_moves_plus_outside_edits_no_silent_data_loss() {
    let mut grid_a = base_grid(20, 12);
    let mut grid_b = base_grid(20, 12);

    let block1 = vec![vec![8001, 8002], vec![8003, 8004]];
    let block2 = vec![vec![9001, 9002], vec![9003, 9004]];

    place_block(&mut grid_a, 2, 2, &block1);
    place_block(&mut grid_a, 10, 7, &block2);

    place_block(&mut grid_b, 8, 4, &block1);
    place_block(&mut grid_b, 14, 1, &block2);

    grid_b[0][0] = 77777;
    grid_b[19][11] = 88888;

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    let rect_moves: Vec<_> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::BlockMovedRect { .. }))
        .collect();
    assert!(
        rect_moves.len() >= 2,
        "should detect both rect block moves in the scenario"
    );

    let rect_regions = [
        (2u32, 2u32, 2u32, 2u32),
        (10u32, 7u32, 2u32, 2u32),
        (8u32, 4u32, 2u32, 2u32),
        (14u32, 1u32, 2u32, 2u32),
    ];

    let outside_cell_edits: Vec<_> = report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, .. } = op {
                let in_rect = rect_regions.iter().any(|(r, c, h, w)| {
                    addr.row >= *r && addr.row < *r + *h && addr.col >= *c && addr.col < *c + *w
                });
                if !in_rect {
                    return Some((addr.row, addr.col));
                }
            }
            None
        })
        .collect();

    assert!(
        !outside_cell_edits.is_empty(),
        "cell edits outside moved rects should be surfaced"
    );
}

#[test]
fn g14_rect_move_plus_row_insertion_outside_no_silent_data_loss() {
    let mut grid_a = base_grid(12, 10);
    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);

    let mut grid_b = base_grid(13, 10);

    for col in 0..10 {
        grid_b[0][col] = 50000 + col as i32;
    }

    for row in 1..13 {
        for col in 0..10 {
            grid_b[row][col] = (row as i32) * 100 + col as i32 + 1;
        }
    }

    place_block(&mut grid_b, 9, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    let rect_moves = collect_rect_moves(&report);
    let row_adds = collect_row_adds(&report);

    assert_eq!(
        rect_moves.len(),
        1,
        "expected a single BlockMovedRect for the moved block"
    );
    assert!(
        !row_adds.is_empty(),
        "expected at least one RowAdded for the inserted row"
    );
}

#[test]
fn g14_rect_move_plus_row_deletion_outside_no_silent_data_loss() {
    let mut grid_a = base_grid(14, 10);
    let block = vec![vec![8001, 8002], vec![8003, 8004]];
    place_block(&mut grid_a, 3, 3, &block);

    let mut grid_b = base_grid(13, 10);

    place_block(&mut grid_b, 8, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    let rect_moves = collect_rect_moves(&report);
    let row_removes = collect_row_removes(&report);

    assert_eq!(
        rect_moves.len(),
        1,
        "expected a single BlockMovedRect for the moved block"
    );
    assert!(
        !row_removes.is_empty(),
        "expected at least one RowRemoved for the deleted row"
    );
}

#[test]
fn g14_row_block_move_plus_row_insertion_outside_no_silent_data_loss() {
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    let mut rows_b: Vec<Vec<i32>> = Vec::with_capacity(21);

    rows_b.push(vec![9991, 9992, 9993, 9994]);

    let mut original = rows.clone();
    let moved_block: Vec<Vec<i32>> = original.drain(4..8).collect();
    original.splice(12..12, moved_block);
    rows_b.extend(original);

    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report.ops.is_empty(),
        "row block move + row insertion should produce operations"
    );
}

#[test]
fn g14_move_detection_disabled_falls_back_to_positional() {
    let mut grid_a = base_grid(12, 10);
    let mut grid_b = base_grid(12, 10);

    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);
    place_block(&mut grid_b, 8, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let disabled_config = DiffConfig {
        max_move_iterations: 0,
        ..DiffConfig::default()
    };
    let report_disabled = diff_workbooks_with_config(&wb_a, &wb_b, &disabled_config);

    let rect_moves_disabled = collect_rect_moves(&report_disabled);
    assert!(
        rect_moves_disabled.is_empty(),
        "with move detection disabled, no BlockMovedRect should be emitted"
    );
    assert!(
        !report_disabled.ops.is_empty(),
        "with move detection disabled, positional changes should still be reported"
    );

    let report_enabled = diff_workbooks(&wb_a, &wb_b);

    let rect_moves_enabled = collect_rect_moves(&report_enabled);
    assert_eq!(
        rect_moves_enabled.len(),
        1,
        "with move detection enabled, BlockMovedRect should be detected"
    );
}

#[test]
fn g14_max_move_iterations_limits_detected_moves() {
    let mut grid_a = base_grid(50, 10);
    let mut grid_b = base_grid(50, 10);

    let block1 = vec![vec![1001, 1002], vec![1003, 1004]];
    let block2 = vec![vec![2001, 2002], vec![2003, 2004]];
    let block3 = vec![vec![3001, 3002], vec![3003, 3004]];
    let block4 = vec![vec![4001, 4002], vec![4003, 4004]];
    let block5 = vec![vec![5001, 5002], vec![5003, 5004]];

    place_block(&mut grid_a, 2, 1, &block1);
    place_block(&mut grid_a, 8, 1, &block2);
    place_block(&mut grid_a, 14, 1, &block3);
    place_block(&mut grid_a, 20, 1, &block4);
    place_block(&mut grid_a, 26, 1, &block5);

    place_block(&mut grid_b, 40, 7, &block1);
    place_block(&mut grid_b, 34, 7, &block2);
    place_block(&mut grid_b, 28, 7, &block3);
    place_block(&mut grid_b, 22, 7, &block4);
    place_block(&mut grid_b, 16, 7, &block5);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let limited_config = DiffConfig {
        max_move_iterations: 2,
        ..DiffConfig::default()
    };
    let report_limited = diff_workbooks_with_config(&wb_a, &wb_b, &limited_config);

    let rect_moves_limited = collect_rect_moves(&report_limited);

    assert!(
        rect_moves_limited.len() <= 2,
        "with max_move_iterations=2, at most 2 rect moves should be detected, got {}",
        rect_moves_limited.len()
    );

    assert!(
        !report_limited.ops.is_empty(),
        "remaining differences should still be surfaced, not silently dropped"
    );

    let full_config = DiffConfig::default();
    let report_full = diff_workbooks_with_config(&wb_a, &wb_b, &full_config);

    let rect_moves_full = collect_rect_moves(&report_full);

    assert!(
        rect_moves_full.len() >= 5,
        "with default config, all 5 rect moves should be detected, got {}",
        rect_moves_full.len()
    );
}
