use excel_diff::{DiffOp, diff_workbooks, open_workbook};

mod common;
use common::{fixture_path, grid_from_numbers, single_sheet_workbook};

#[test]
fn g12_rect_block_move_emits_single_blockmovedrect() {
    let wb_a = open_workbook(fixture_path("rect_block_move_a.xlsx"))
        .expect("failed to open fixture: rect_block_move_a.xlsx");
    let wb_b = open_workbook(fixture_path("rect_block_move_b.xlsx"))
        .expect("failed to open fixture: rect_block_move_b.xlsx");

    let report = diff_workbooks(&wb_a, &wb_b);

    assert_eq!(report.ops.len(), 1, "expected a single diff op");

    match &report.ops[0] {
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash: _,
        } => {
            assert_eq!(sheet, "Data");
            assert_eq!(*src_start_row, 2);
            assert_eq!(*src_row_count, 3);
            assert_eq!(*src_start_col, 1);
            assert_eq!(*src_col_count, 3);
            assert_eq!(*dst_start_row, 9);
            assert_eq!(*dst_start_col, 6);
        }
        other => panic!("expected BlockMovedRect op, got {:?}", other),
    }
}

#[test]
fn g12_rect_block_move_ambiguous_swap_does_not_emit_blockmovedrect() {
    let (grid_a, grid_b) = swap_two_blocks();
    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "ambiguous block swap must not emit BlockMovedRect"
    );
    assert!(
        !report.ops.is_empty(),
        "fallback path should emit some diff operations"
    );
}

#[test]
fn g12_rect_block_move_with_internal_edit_falls_back() {
    let (grid_a, grid_b) = move_with_edit();
    let wb_a = single_sheet_workbook("Data", grid_a);
    let wb_b = single_sheet_workbook("Data", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    assert!(
        !report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::BlockMovedRect { .. })),
        "move with internal edit should not be treated as exact rectangular move"
    );
    assert!(
        report
            .ops
            .iter()
            .any(|op| matches!(op, DiffOp::CellEdited { .. })),
        "edited block should surface as cell edits or structural diffs"
    );
}

fn swap_two_blocks() -> (excel_diff::Grid, excel_diff::Grid) {
    let base: Vec<Vec<i32>> = (0..6)
        .map(|r| (0..6).map(|c| 100 * r as i32 + c as i32).collect())
        .collect();
    let mut grid_a = base.clone();
    let mut grid_b = base.clone();

    let block_one = vec![vec![900, 901], vec![902, 903]];
    let block_two = vec![vec![700, 701], vec![702, 703]];

    place_block(&mut grid_a, 0, 0, &block_one);
    place_block(&mut grid_a, 3, 3, &block_two);

    // Swap the two distinct blocks in grid B.
    place_block(&mut grid_b, 0, 0, &block_two);
    place_block(&mut grid_b, 3, 3, &block_one);

    (grid_from_matrix(grid_a), grid_from_matrix(grid_b))
}

fn move_with_edit() -> (excel_diff::Grid, excel_diff::Grid) {
    let mut grid_a = base_background(10, 10);
    let mut grid_b = base_background(10, 10);

    let block = vec![vec![11, 12, 13], vec![21, 22, 23], vec![31, 32, 33]];

    place_block(&mut grid_a, 1, 1, &block);
    place_block(&mut grid_b, 6, 4, &block);
    grid_b[7][5] = 9_999; // edit inside the moved block

    (grid_from_matrix(grid_a), grid_from_matrix(grid_b))
}

fn base_background(rows: usize, cols: usize) -> Vec<Vec<i32>> {
    (0..rows)
        .map(|r| (0..cols).map(|c| (r as i32) * 1_000 + c as i32).collect())
        .collect()
}

fn place_block(target: &mut [Vec<i32>], top: usize, left: usize, block: &[Vec<i32>]) {
    for (r_offset, row_vals) in block.iter().enumerate() {
        for (c_offset, value) in row_vals.iter().enumerate() {
            let row = top + r_offset;
            let col = left + c_offset;
            if let Some(row_slice) = target.get_mut(row) {
                if let Some(cell) = row_slice.get_mut(col) {
                    *cell = *value;
                }
            }
        }
    }
}

fn grid_from_matrix(matrix: Vec<Vec<i32>>) -> excel_diff::Grid {
    let refs: Vec<&[i32]> = matrix.iter().map(|row| row.as_slice()).collect();
    grid_from_numbers(&refs)
}
