//! Gap strategy selection for AMR alignment.
//!
//! Implements gap strategy selection as described in the unified grid diff
//! specification Sections 9.6 and 12. After anchors divide the grids into
//! gaps, each gap is processed according to its characteristics:
//!
//! - **Empty**: Both sides empty, nothing to do
//! - **InsertAll**: Old side empty, all new rows are insertions
//! - **DeleteAll**: New side empty, all old rows are deletions
//! - **SmallEdit**: Both sides small enough for O(n*m) LCS alignment
//! - **MoveCandidate**: Gap contains matching unique signatures that may indicate moves
//! - **RecursiveAlign**: Gap is large; recursively apply AMR with rare anchors
//! - **HashFallback**: Monotone hash/LIS fallback for large gaps

use std::collections::HashSet;

use crate::grid_metadata::{FrequencyClass, RowMeta};
use crate::config::DiffConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapStrategy {
    Empty,
    InsertAll,
    DeleteAll,
    SmallEdit,
    MoveCandidate,
    RecursiveAlign,
    HashFallback,
}

pub fn select_gap_strategy(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    config: &DiffConfig,
    has_recursed: bool,
) -> GapStrategy {
    let old_len = old_slice.len() as u32;
    let new_len = new_slice.len() as u32;

    if old_len == 0 && new_len == 0 {
        return GapStrategy::Empty;
    }
    if old_len == 0 {
        return GapStrategy::InsertAll;
    }
    if new_len == 0 {
        return GapStrategy::DeleteAll;
    }

    let is_move_candidate = has_matching_signatures(old_slice, new_slice);

    let small_threshold = config.small_gap_threshold.min(config.max_lcs_gap_size);
    if old_len <= small_threshold && new_len <= small_threshold {
        return if is_move_candidate {
            GapStrategy::MoveCandidate
        } else {
            GapStrategy::SmallEdit
        };
    }

    if (old_len > config.recursive_align_threshold || new_len > config.recursive_align_threshold)
        && !has_recursed
    {
        return GapStrategy::RecursiveAlign;
    }

    if is_move_candidate {
        return GapStrategy::MoveCandidate;
    }

    if old_len > config.max_lcs_gap_size || new_len > config.max_lcs_gap_size {
        return GapStrategy::HashFallback;
    }

    GapStrategy::SmallEdit
}

fn has_matching_signatures(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> bool {
    let set: HashSet<_> = old_slice
        .iter()
        .filter(|m| m.frequency_class == FrequencyClass::Unique)
        .map(|m| m.signature)
        .collect();

    new_slice
        .iter()
        .filter(|m| m.frequency_class == FrequencyClass::Unique)
        .any(|m| set.contains(&m.signature))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid_metadata::{FrequencyClass, RowMeta};
    use crate::workbook::RowSignature;

    fn meta(row_idx: u32, hash: u128) -> RowMeta {
        let signature = RowSignature { hash };
        RowMeta {
            row_idx,
            signature,
            non_blank_count: 1,
            first_non_blank_col: 0,
            frequency_class: FrequencyClass::Common,
            is_low_info: false,
        }
    }

    #[test]
    fn respects_configured_max_lcs_gap_size() {
        let config = DiffConfig {
            max_lcs_gap_size: 2,
            small_gap_threshold: 10,
            ..Default::default()
        };
        let rows_a = vec![meta(0, 1), meta(1, 2), meta(2, 3)];
        let rows_b = vec![meta(0, 4), meta(1, 5), meta(2, 6)];

        let strategy = select_gap_strategy(&rows_a, &rows_b, &config, false);
        assert_eq!(strategy, GapStrategy::HashFallback);
    }
}
