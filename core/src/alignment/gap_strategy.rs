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

use crate::alignment::row_metadata::{FrequencyClass, RowMeta};
use crate::config::DiffConfig;

pub const MAX_LCS_GAP_SIZE: u32 = 1500;

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

    let small_threshold = config.small_gap_threshold.min(MAX_LCS_GAP_SIZE);
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

    if old_len > MAX_LCS_GAP_SIZE || new_len > MAX_LCS_GAP_SIZE {
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
