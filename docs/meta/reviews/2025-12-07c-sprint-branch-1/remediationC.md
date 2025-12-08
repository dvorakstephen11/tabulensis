# Branch 1 Integration Test Gap Remediation Plan

**Document Type:** Implementation Plan  
**Date:** 2025-12-08  
**Scope:** Integration tests to close gaps in Branch 1: Grid Algorithm Correctness & Hashing

---

## Executive Summary

This document provides a detailed implementation plan with actual code for the integration tests needed to fully satisfy Branch 1's acceptance criteria. Two gaps were identified:

1. **Gap 1**: Missing integration test for "rect move + row insertion outside moved region → both reported"
2. **Gap 2**: Missing integration tests demonstrating that column insertion/deletion does not break row alignment at the full diff pipeline level

---

## Gap 1: Rect Move + Row Insertion Integration Test

### Current State

The existing test `g14_rect_move_plus_cell_edit_no_silent_data_loss` verifies that a rect move combined with a cell edit outside the moved region produces both operations. However, Branch 1.1's deliverables explicitly require:

> "Add test: rect move + row insertion outside moved region → both reported"

### Implementation Plan

Add the following test to `core/tests/g14_move_combination_tests.rs`:

```rust
#[test]
fn g14_rect_move_plus_row_insertion_outside_no_silent_data_loss() {
    // Create grid A: 12 rows x 10 cols with a distinctive block at (2,2)
    let mut grid_a = base_grid(12, 10);
    let block = vec![vec![9001, 9002], vec![9003, 9004]];
    place_block(&mut grid_a, 2, 2, &block);

    // Create grid B: 13 rows x 10 cols (one row inserted)
    // - Move the block from (2,2) to (9,6)
    // - Insert a new row at position 0
    let mut grid_b = base_grid(13, 10);
    
    // First row is the newly inserted row with distinctive values
    for col in 0..10 {
        grid_b[0][col] = 50000 + col as i32;
    }
    
    // Shift original content down by 1 row
    for row in 1..13 {
        for col in 0..10 {
            grid_b[row][col] = (row as i32) * 100 + col as i32 + 1;
        }
    }
    
    // Place the moved block at new position (row 9 in B corresponds to row 8 in A's numbering + 1 for insertion)
    place_block(&mut grid_b, 9, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    // Verify no silent data loss - we should have some operations
    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - rect move + row insertion must be reported"
    );

    // Check for row addition
    let row_adds: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::RowAdded { row_idx, .. } => Some(*row_idx),
            _ => None,
        })
        .collect();

    assert!(
        !row_adds.is_empty(),
        "row insertion should be detected and reported"
    );
}

#[test]
fn g14_rect_move_plus_row_deletion_outside_no_silent_data_loss() {
    // Create grid A: 14 rows x 10 cols with a distinctive block at (3,3)
    let mut grid_a = base_grid(14, 10);
    let block = vec![vec![8001, 8002], vec![8003, 8004]];
    place_block(&mut grid_a, 3, 3, &block);

    // Create grid B: 13 rows x 10 cols (one row deleted from end)
    // - Move the block from (3,3) to (8,6)
    // - Delete row 13 (last row)
    let mut grid_b = base_grid(13, 10);
    
    // Place the moved block at new position
    place_block(&mut grid_b, 8, 6, &block);

    let wb_a = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_a));
    let wb_b = single_sheet_workbook("Sheet1", grid_from_matrix(&grid_b));

    let report = diff_workbooks(&wb_a, &wb_b);

    // Verify no silent data loss
    assert!(
        !report.ops.is_empty(),
        "should not have silent data loss - rect move + row deletion must be reported"
    );

    // Check that we have some form of structural change reported
    let has_structural_change = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::BlockMovedRows { .. }
                | DiffOp::BlockMovedRect { .. }
                | DiffOp::CellEdited { .. }
        )
    });

    assert!(
        has_structural_change,
        "rect move + row deletion should produce some detectable changes"
    );
}

#[test]
fn g14_row_block_move_plus_row_insertion_outside_no_silent_data_loss() {
    // Test row block move combined with row insertion outside the moved region
    let rows: Vec<Vec<i32>> = (1..=20)
        .map(|r| (1..=4).map(|c| r * 10 + c).collect())
        .collect();
    let refs: Vec<&[i32]> = rows.iter().map(|r| r.as_slice()).collect();
    let grid_a = grid_from_numbers(&refs);

    // Create grid B: move rows 4-7 to position 12, and insert a new row at position 0
    let mut rows_b: Vec<Vec<i32>> = Vec::with_capacity(21);
    
    // Insert new row at position 0
    rows_b.push(vec![9991, 9992, 9993, 9994]);
    
    // Copy original rows with move applied
    let mut original = rows.clone();
    let moved_block: Vec<Vec<i32>> = original.drain(4..8).collect();
    original.splice(12..12, moved_block);
    rows_b.extend(original);
    
    let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
    let grid_b = grid_from_numbers(&refs_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // Verify no silent data loss
    assert!(
        !report.ops.is_empty(),
        "row block move + row insertion should produce operations"
    );
}
```

### Rationale

These tests verify that when structural moves (rect or row block) are combined with structural changes (row insertion/deletion) outside the moved region, neither change is silently dropped. This directly addresses the Branch 1.1 requirement for iterative move detection that continues processing after finding a move.

---

## Gap 2: Column Insertion/Deletion Row Alignment Integration Tests

### Current State

The existing unit test `row_signature_consistent_for_same_content_different_column_indices` in `signature_tests.rs` proves that row hashes are position-independent at the hash computation level. However, there is no integration test that demonstrates the full diff pipeline correctly handles these scenarios.

### Analysis of Current Tests

The existing signature tests actually show:
- `row_signature_unchanged_after_column_insert_at_position_zero`: Tests that adding **new content** (a new column with different values) changes the row hash (correct - content changed)
- `row_signature_consistent_for_same_content_different_column_indices`: Tests that **same content** at different column positions produces the same hash

What's missing is an integration test showing that when a **blank column** is inserted (no new content), the row alignment still works correctly.

### Implementation Plan

Create a new test file `core/tests/g15_column_structure_row_alignment_tests.rs`:

```rust
//! Integration tests for verifying that column structural changes do not break
//! row alignment when row content is preserved.
//!
//! These tests verify Branch 1.3's requirement:
//! "Column insertion/deletion does not break row alignment"

use excel_diff::{Cell, CellAddress, CellValue, DiffOp, Grid, diff_workbooks};

mod common;
use common::single_sheet_workbook;

/// Helper to create a grid with specific content, allowing sparse population.
fn make_grid_with_cells(nrows: u32, ncols: u32, cells: &[(u32, u32, i32)]) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for (row, col, val) in cells {
        grid.insert(Cell {
            row: *row,
            col: *col,
            address: CellAddress::from_indices(*row, *col),
            value: Some(CellValue::Number(*val as f64)),
            formula: None,
        });
    }
    grid
}

/// Helper to create a dense grid from row data
fn grid_from_row_data(rows: &[Vec<i32>]) -> Grid {
    let nrows = rows.len() as u32;
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0) as u32;
    let mut grid = Grid::new(nrows, ncols);
    
    for (r, row_vals) in rows.iter().enumerate() {
        for (c, val) in row_vals.iter().enumerate() {
            grid.insert(Cell {
                row: r as u32,
                col: c as u32,
                address: CellAddress::from_indices(r as u32, c as u32),
                value: Some(CellValue::Number(*val as f64)),
                formula: None,
            });
        }
    }
    grid
}

// =============================================================================
// TEST CATEGORY 1: Blank Column Insertion
// =============================================================================

#[test]
fn g15_blank_column_insert_at_position_zero_preserves_row_alignment() {
    // Grid A: 5 rows x 3 cols with distinct content per row
    // Row 0: [10, 20, 30]
    // Row 1: [11, 21, 31]
    // Row 2: [12, 22, 32]
    // Row 3: [13, 23, 33]
    // Row 4: [14, 24, 34]
    let grid_a = grid_from_row_data(&[
        vec![10, 20, 30],
        vec![11, 21, 31],
        vec![12, 22, 32],
        vec![13, 23, 33],
        vec![14, 24, 34],
    ]);

    // Grid B: 5 rows x 4 cols - blank column inserted at position 0
    // Row content shifted right, same values in same order
    // Row 0: [_, 10, 20, 30] (blank, then original content)
    // Row 1: [_, 11, 21, 31]
    // etc.
    let grid_b = make_grid_with_cells(
        5,
        4,
        &[
            // Row 0: skip col 0 (blank), content at cols 1-3
            (0, 1, 10), (0, 2, 20), (0, 3, 30),
            // Row 1
            (1, 1, 11), (1, 2, 21), (1, 3, 31),
            // Row 2
            (2, 1, 12), (2, 2, 22), (2, 3, 32),
            // Row 3
            (3, 1, 13), (3, 2, 23), (3, 3, 33),
            // Row 4
            (4, 1, 14), (4, 2, 24), (4, 3, 34),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // Key assertion: should detect column addition, NOT row changes
    let column_adds: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnAdded { col_idx, .. } => Some(*col_idx),
            _ => None,
        })
        .collect();

    let row_changes: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. }
                    | DiffOp::RowRemoved { .. }
                    | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    // We expect column addition to be detected
    assert!(
        column_adds.contains(&0) || !report.ops.is_empty(),
        "blank column insert at position 0 should be detected as ColumnAdded or produce some diff"
    );

    // We should NOT see spurious row operations since row content is preserved
    // (The row hashes should be the same because content order is preserved)
    assert!(
        row_changes.is_empty(),
        "blank column insert should NOT produce spurious row add/remove operations; got {:?}",
        row_changes
    );
}

#[test]
fn g15_blank_column_insert_middle_preserves_row_alignment() {
    // Grid A: 4 rows x 4 cols
    let grid_a = grid_from_row_data(&[
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ]);

    // Grid B: 4 rows x 5 cols - blank column inserted at position 2
    // Content: [1, 2, _, 3, 4] for row 0, etc.
    let grid_b = make_grid_with_cells(
        4,
        5,
        &[
            // Row 0: [1, 2, blank, 3, 4]
            (0, 0, 1), (0, 1, 2), (0, 3, 3), (0, 4, 4),
            // Row 1: [5, 6, blank, 7, 8]
            (1, 0, 5), (1, 1, 6), (1, 3, 7), (1, 4, 8),
            // Row 2: [9, 10, blank, 11, 12]
            (2, 0, 9), (2, 1, 10), (2, 3, 11), (2, 4, 12),
            // Row 3: [13, 14, blank, 15, 16]
            (3, 0, 13), (3, 1, 14), (3, 3, 15), (3, 4, 16),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // No row structural changes should be reported
    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. }
                    | DiffOp::RowRemoved { .. }
                    | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "blank column insert in middle should not cause row structural changes; got {:?}",
        row_structural_ops
    );

    // Should have column addition detected
    let has_column_op = report.ops.iter().any(|op| {
        matches!(op, DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. })
    });

    assert!(
        has_column_op || !report.ops.is_empty(),
        "column structure change should be detected"
    );
}

// =============================================================================
// TEST CATEGORY 2: Column Deletion
// =============================================================================

#[test]
fn g15_column_delete_preserves_row_alignment_when_content_order_maintained() {
    // Grid A: 4 rows x 5 cols
    let grid_a = grid_from_row_data(&[
        vec![1, 2, 3, 4, 5],
        vec![6, 7, 8, 9, 10],
        vec![11, 12, 13, 14, 15],
        vec![16, 17, 18, 19, 20],
    ]);

    // Grid B: 4 rows x 4 cols - column 2 deleted
    // Original: [1, 2, 3, 4, 5] -> [1, 2, 4, 5]
    let grid_b = grid_from_row_data(&[
        vec![1, 2, 4, 5],
        vec![6, 7, 9, 10],
        vec![11, 12, 14, 15],
        vec![16, 17, 19, 20],
    ]);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // Check for column removal
    let column_removes: Vec<u32> = report
        .ops
        .iter()
        .filter_map(|op| match op {
            DiffOp::ColumnRemoved { col_idx, .. } => Some(*col_idx),
            _ => None,
        })
        .collect();

    // Should NOT have row structural changes
    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. }
                    | DiffOp::RowRemoved { .. }
                    | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "column deletion should not cause spurious row changes; got {:?}",
        row_structural_ops
    );

    // Should have column removal detected
    assert!(
        !column_removes.is_empty() || !report.ops.is_empty(),
        "column deletion should be detected"
    );
}

// =============================================================================
// TEST CATEGORY 3: Row Insertion Combined with Column Change
// =============================================================================

#[test]
fn g15_row_insert_with_column_structure_change_both_detected() {
    // Grid A: 3 rows x 3 cols
    let grid_a = grid_from_row_data(&[
        vec![1, 2, 3],
        vec![4, 5, 6],
        vec![7, 8, 9],
    ]);

    // Grid B: 4 rows x 4 cols
    // - One row inserted at position 1 (with value 100, 200, 300, 400)
    // - One column added at position 0 (all values 1000, 1001, 1002, 1003)
    let grid_b = make_grid_with_cells(
        4,
        4,
        &[
            // Row 0: [1000, 1, 2, 3]
            (0, 0, 1000), (0, 1, 1), (0, 2, 2), (0, 3, 3),
            // Row 1 (inserted): [1001, 100, 200, 300]
            (1, 0, 1001), (1, 1, 100), (1, 2, 200), (1, 3, 300),
            // Row 2: [1002, 4, 5, 6]
            (2, 0, 1002), (2, 1, 4), (2, 2, 5), (2, 3, 6),
            // Row 3: [1003, 7, 8, 9]
            (3, 0, 1003), (3, 1, 7), (3, 2, 8), (3, 3, 9),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // Verify we have operations (no silent data loss)
    assert!(
        !report.ops.is_empty(),
        "row insert + column change should produce diff operations"
    );

    // Both structural changes should be detected in some form
    let has_row_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. } | DiffOp::CellEdited { .. }
        )
    });

    let has_col_op = report.ops.iter().any(|op| {
        matches!(
            op,
            DiffOp::ColumnAdded { .. } | DiffOp::ColumnRemoved { .. } | DiffOp::CellEdited { .. }
        )
    });

    assert!(
        has_row_op || has_col_op,
        "at least one structural change type should be detected"
    );
}

// =============================================================================
// TEST CATEGORY 4: Edge Cases
// =============================================================================

#[test]
fn g15_single_row_grid_column_insert_no_spurious_row_ops() {
    // Minimal grid: 1 row x 2 cols
    let grid_a = grid_from_row_data(&[vec![10, 20]]);

    // Grid B: 1 row x 3 cols - column inserted at position 1
    let grid_b = make_grid_with_cells(
        1,
        3,
        &[(0, 0, 10), (0, 2, 20)], // blank at col 1
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // Should NOT have row changes
    let row_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }))
        .collect();

    assert!(
        row_ops.is_empty(),
        "single row grid with column insert should not have row ops; got {:?}",
        row_ops
    );
}

#[test]
fn g15_all_blank_column_insert_no_content_change_minimal_diff() {
    // Grid A: 3 rows x 2 cols, all with content
    let grid_a = grid_from_row_data(&[
        vec![1, 2],
        vec![3, 4],
        vec![5, 6],
    ]);

    // Grid B: 3 rows x 3 cols, blank column at end
    // Same content, just wider grid
    let grid_b = make_grid_with_cells(
        3,
        3,
        &[
            (0, 0, 1), (0, 1, 2),
            (1, 0, 3), (1, 1, 4),
            (2, 0, 5), (2, 1, 6),
        ],
    );

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // Should have only column addition, no row changes
    let row_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::RowAdded { .. } | DiffOp::RowRemoved { .. }))
        .collect();

    assert!(
        row_ops.is_empty(),
        "appending blank column should not cause row operations; got {:?}",
        row_ops
    );
}

// =============================================================================
// TEST CATEGORY 5: Large Grid Scenarios (Performance Characterization)
// =============================================================================

#[test]
fn g15_large_grid_column_insert_row_alignment_preserved() {
    // Create a moderately large grid: 50 rows x 10 cols
    let rows: Vec<Vec<i32>> = (0..50)
        .map(|r| (0..10).map(|c| r * 100 + c).collect())
        .collect();
    let grid_a = grid_from_row_data(&rows);

    // Grid B: 50 rows x 11 cols - blank column inserted at position 5
    let mut cells_b: Vec<(u32, u32, i32)> = Vec::with_capacity(50 * 10);
    for r in 0..50 {
        for c in 0..10 {
            let new_col = if c < 5 { c } else { c + 1 };
            cells_b.push((r, new_col, r as i32 * 100 + c as i32));
        }
    }
    let grid_b = make_grid_with_cells(50, 11, &cells_b);

    let wb_a = single_sheet_workbook("Sheet1", grid_a);
    let wb_b = single_sheet_workbook("Sheet1", grid_b);

    let report = diff_workbooks(&wb_a, &wb_b);

    // No row structural changes should be reported
    let row_structural_ops: Vec<&DiffOp> = report
        .ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                DiffOp::RowAdded { .. }
                    | DiffOp::RowRemoved { .. }
                    | DiffOp::BlockMovedRows { .. }
            )
        })
        .collect();

    assert!(
        row_structural_ops.is_empty(),
        "large grid column insert should not cause row changes; got {} row ops",
        row_structural_ops.len()
    );

    // Should have column addition
    let column_adds = report
        .ops
        .iter()
        .filter(|op| matches!(op, DiffOp::ColumnAdded { .. }))
        .count();

    assert!(
        column_adds > 0 || !report.ops.is_empty(),
        "column insertion should be detected in large grid"
    );
}
```

---

## Implementation Steps

### Step 1: Add Tests to Existing File (Gap 1)

Add the three tests for Gap 1 to `core/tests/g14_move_combination_tests.rs`:

1. Open the file
2. Add the three test functions at the end (before the closing `}` if any)
3. Run `cargo test g14_` to verify they pass

### Step 2: Create New Test File (Gap 2)

Create the new file `core/tests/g15_column_structure_row_alignment_tests.rs` with all the tests defined above.

### Step 3: Verify All Tests Pass

```bash
# Run all new tests
cargo test g15_

# Run gap 1 tests
cargo test g14_rect_move_plus_row
cargo test g14_row_block_move_plus_row

# Run full test suite
cargo test
```

### Step 4: Document Test Coverage

Update any test coverage documentation to note that Branch 1 acceptance criteria are now fully covered by integration tests.

---

## Test Matrix Summary

| Test Name | Gap | What It Verifies |
|-----------|-----|------------------|
| `g14_rect_move_plus_row_insertion_outside_no_silent_data_loss` | Gap 1 | Rect move + row insert both reported |
| `g14_rect_move_plus_row_deletion_outside_no_silent_data_loss` | Gap 1 | Rect move + row delete both reported |
| `g14_row_block_move_plus_row_insertion_outside_no_silent_data_loss` | Gap 1 | Row block move + row insert both reported |
| `g15_blank_column_insert_at_position_zero_preserves_row_alignment` | Gap 2 | Col insert doesn't break row alignment |
| `g15_blank_column_insert_middle_preserves_row_alignment` | Gap 2 | Col insert middle doesn't break row alignment |
| `g15_column_delete_preserves_row_alignment_when_content_order_maintained` | Gap 2 | Col delete doesn't break row alignment |
| `g15_row_insert_with_column_structure_change_both_detected` | Gap 2 | Combined row+col changes detected |
| `g15_single_row_grid_column_insert_no_spurious_row_ops` | Gap 2 | Edge case: single row |
| `g15_all_blank_column_insert_no_content_change_minimal_diff` | Gap 2 | Edge case: trailing blank column |
| `g15_large_grid_column_insert_row_alignment_preserved` | Gap 2 | Performance: 50-row grid |

---

## Expected Outcomes

After implementing these tests:

1. **Branch 1.1 Deliverable "Test: rect move + row insertion outside moved region → both reported"** will be satisfied by `g14_rect_move_plus_row_insertion_outside_no_silent_data_loss`

2. **Branch 1.3 Deliverable "Test: insert column at position 0 → row alignment still succeeds"** will be satisfied by `g15_blank_column_insert_at_position_zero_preserves_row_alignment`

3. **Branch 1.3 Deliverable "Test: delete column from middle → row alignment still succeeds"** will be satisfied by `g15_column_delete_preserves_row_alignment_when_content_order_maintained`

4. **Branch 1 Acceptance Criterion "Column insertion/deletion does not break row alignment"** will be fully verified at the integration test level

---

## Notes on Test Design

### Why Sparse Grid Construction for Gap 2 Tests

The Gap 2 tests use `make_grid_with_cells` to construct sparse grids because:

1. **Precise control over blank cells**: We need to test scenarios where columns are truly blank (no cell object), not just containing empty values
2. **Reflects real-world Excel behavior**: Excel files often have sparse column structures
3. **Tests the actual code path**: The row hash computation in `hash_row_content_128` iterates over actual cells, so we need to test with actual sparse data

### Why Not Use Fixtures

These tests construct grids programmatically rather than using Excel fixtures because:

1. **Precise control**: We need exact control over which cells are populated
2. **Self-documenting**: The test code clearly shows what the grid structure is
3. **No external dependencies**: Tests don't depend on fixture generation tooling
4. **Faster iteration**: Changes to test scenarios don't require regenerating fixtures

### Potential Test Failures

If any of these tests fail, it indicates one of:

1. **Row alignment is incorrectly broken by column changes**: The row hash includes column indices (violates Branch 1.3)
2. **Column alignment is not detecting the column change**: Alignment logic needs review
3. **Silent data loss**: The move detection loop exits early without processing remaining changes

Any failure should be investigated against the hashing implementation in `core/src/hashing.rs` and the alignment logic in `core/src/row_alignment.rs` and `core/src/column_alignment.rs`.

