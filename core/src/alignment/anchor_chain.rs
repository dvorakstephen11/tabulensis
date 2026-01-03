//! Anchor chain construction using Longest Increasing Subsequence (LIS).
//!
//! Implements anchor chain building as described in the unified grid diff
//! specification Section 10. Given a set of discovered anchors, this module
//! selects the maximal subset that preserves relative order in both grids.
//!
//! For example, if anchors show:
//! - Row A: old=0, new=0
//! - Row B: old=2, new=1  (B moved up)
//! - Row C: old=1, new=2
//!
//! The LIS algorithm selects {A, C} because their old_row indices (0, 1) are
//! increasing, making them a valid ordering chain. Row B is excluded because
//! including it would create a crossing (B is at old=2 but new=1, while C is
//! at old=1 but new=2).

use crate::alignment::anchor_discovery::Anchor;

pub fn build_anchor_chain(mut anchors: Vec<Anchor>) -> Vec<Anchor> {
    // Sort by new_row to preserve destination order before LIS on old_row.
    anchors.sort_by_key(|a| a.new_row);
    let indices = lis_indices(&anchors, |a| a.old_row);
    indices.into_iter().map(|idx| anchors[idx]).collect()
}

fn lis_indices<T, F>(items: &[T], key: F) -> Vec<usize>
where
    F: Fn(&T) -> u32,
{
    let mut piles: Vec<usize> = Vec::new();
    let mut predecessors: Vec<Option<usize>> = vec![None; items.len()];

    for (idx, item) in items.iter().enumerate() {
        let k = key(item);
        let pos = piles
            .binary_search_by_key(&k, |&pile_idx| key(&items[pile_idx]))
            .unwrap_or_else(|insert_pos| insert_pos);

        if pos > 0 {
            predecessors[idx] = Some(piles[pos - 1]);
        }

        if pos == piles.len() {
            piles.push(idx);
        } else {
            piles[pos] = idx;
        }
    }

    let Some(&last) = piles.last() else {
        return Vec::new();
    };

    let mut result: Vec<usize> = Vec::new();
    let mut current = last;
    loop {
        result.push(current);
        if let Some(prev) = predecessors[current] {
            current = prev;
        } else {
            break;
        }
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::anchor_discovery::Anchor;
    use crate::workbook::RowSignature;

    #[test]
    fn builds_increasing_chain() {
        let anchors = vec![
            Anchor {
                old_row: 0,
                new_row: 0,
                signature: RowSignature { hash: 1 },
            },
            Anchor {
                old_row: 2,
                new_row: 1,
                signature: RowSignature { hash: 2 },
            },
            Anchor {
                old_row: 1,
                new_row: 2,
                signature: RowSignature { hash: 3 },
            },
        ];

        let chain = build_anchor_chain(anchors);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].old_row, 0);
        assert_eq!(chain[1].old_row, 1);
    }
}
