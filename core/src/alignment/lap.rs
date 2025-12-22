//! Linear assignment solver (Hungarian algorithm).
//!
//! Uses a dense O(n^3) Hungarian implementation over integer costs.
//! Intended for small candidate sets after global move extraction caps.

pub(crate) fn solve(costs: &[Vec<i64>]) -> Vec<usize> {
    let n = costs.len();
    if n == 0 {
        return Vec::new();
    }

    debug_assert!(costs.iter().all(|row| row.len() == n));

    let inf = i64::MAX / 4;
    let mut u = vec![0i64; n + 1];
    let mut v = vec![0i64; n + 1];
    let mut p = vec![0usize; n + 1];
    let mut way = vec![0usize; n + 1];

    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0usize;
        let mut minv = vec![inf; n + 1];
        let mut used = vec![false; n + 1];

        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = inf;
            let mut j1 = 0usize;

            for j in 1..=n {
                if used[j] {
                    continue;
                }
                let cur = costs[i0 - 1][j - 1] - u[i0] - v[j];
                if cur < minv[j] {
                    minv[j] = cur;
                    way[j] = j0;
                }
                if minv[j] < delta {
                    delta = minv[j];
                    j1 = j;
                }
            }

            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }

            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }

        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }

    let mut assignment = vec![0usize; n];
    for j in 1..=n {
        if p[j] > 0 {
            assignment[p[j] - 1] = j - 1;
        }
    }
    assignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_small_assignment() {
        let costs = vec![
            vec![4, 1, 3],
            vec![2, 0, 5],
            vec![3, 2, 2],
        ];

        let assignment = solve(&costs);
        assert_eq!(assignment.len(), 3);

        let total: i64 = assignment
            .iter()
            .enumerate()
            .map(|(i, &j)| costs[i][j])
            .sum();
        assert_eq!(total, 5, "expected minimal total cost");
    }
}
