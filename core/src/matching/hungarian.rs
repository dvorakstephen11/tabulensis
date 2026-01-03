//! Hungarian assignment wrapper for rectangular cost matrices.
//!
//! Uses the dense Hungarian implementation in `alignment::lap` and pads to a square matrix.

pub(crate) fn solve_rect(costs: &[Vec<i64>], pad_cost: i64) -> Vec<usize> {
    let rows = costs.len();
    let cols = costs.iter().map(|row| row.len()).max().unwrap_or(0);
    let size = rows.max(cols);

    if size == 0 {
        return Vec::new();
    }

    let mut square = vec![vec![pad_cost; size]; size];
    for (i, row) in costs.iter().enumerate() {
        for (j, &cost) in row.iter().enumerate() {
            square[i][j] = cost;
        }
    }

    crate::alignment::lap::solve(&square)
}
