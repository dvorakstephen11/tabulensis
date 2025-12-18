//! Move extraction from alignment gaps.
//!
//! Implements localized move detection within gaps. This is a simplified approach
//! compared to the full spec (Sections 9.5-9.7, 11) which describes global
//! move-candidate extraction and validation phases.
//!
//! ## Current Implementation
//!
//! - `find_block_move`: Scans for contiguous blocks of matching signatures
//!   between old and new slices within a gap. Returns the largest found.
//!
//! - `moves_from_matched_pairs`: Extracts block moves from matched row pairs
//!   where consecutive pairs have the same offset (indicating they moved together).
//!
//! ## Future Work (TODO)
//!
//! To implement full spec compliance, this module would need:
//!
//! 1. Global unanchored match collection (all out-of-order signature matches)
//! 2. Candidate move construction from unanchored matches
//! 3. Move validation to resolve overlapping/conflicting candidates
//! 4. Integration with gap filling to consume validated moves

use std::collections::HashMap;

use crate::alignment::RowBlockMove;
use crate::config::DiffConfig;
use crate::grid_metadata::RowMeta;
use crate::workbook::RowSignature;

pub fn find_block_move(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    min_len: u32,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    let max_slice_len = config.move_extraction_max_slice_len as usize;
    if old_slice.len() > max_slice_len || new_slice.len() > max_slice_len {
        return None;
    }

    let mut positions: HashMap<RowSignature, Vec<usize>> = HashMap::new();
    for (idx, meta) in old_slice.iter().enumerate() {
        if meta.is_low_info() {
            continue;
        }
        positions.entry(meta.signature).or_default().push(idx);
    }

    let mut best: Option<RowBlockMove> = None;
    let mut best_len: usize = 0;

    for (new_idx, meta) in new_slice.iter().enumerate() {
        if meta.is_low_info() {
            continue;
        }

        let Some(candidates) = positions.get(&meta.signature) else {
            continue;
        };

        let max_candidates = config.move_extraction_max_candidates_per_sig as usize;
        for &old_idx in candidates.iter().take(max_candidates) {
            let max_possible = (old_slice.len() - old_idx).min(new_slice.len() - new_idx);
            if max_possible <= best_len {
                continue;
            }

            let mut len = 0usize;
            while len < max_possible
                && old_slice[old_idx + len].signature == new_slice[new_idx + len].signature
            {
                len += 1;
            }

            if len >= min_len as usize && len > best_len {
                best_len = len;
                best = Some(RowBlockMove {
                    src_start_row: old_slice[old_idx].row_idx,
                    dst_start_row: new_slice[new_idx].row_idx,
                    row_count: len as u32,
                });
            }
        }
    }

    best
}

pub fn moves_from_matched_pairs(pairs: &[(u32, u32)]) -> Vec<RowBlockMove> {
    if pairs.is_empty() {
        return Vec::new();
    }

    let mut sorted = pairs.to_vec();
    sorted.sort_by_key(|(a, b)| (*a, *b));

    let mut moves = Vec::new();
    let mut start = sorted[0];
    let mut prev = sorted[0];
    let mut run_len = 1u32;
    let mut current_offset: i64 = prev.1 as i64 - prev.0 as i64;

    for &(a, b) in sorted.iter().skip(1) {
        let offset = b as i64 - a as i64;
        if offset == current_offset && a == prev.0 + 1 && b == prev.1 + 1 {
            run_len += 1;
            prev = (a, b);
            continue;
        }

        if run_len > 1 && current_offset != 0 {
            moves.push(RowBlockMove {
                src_start_row: start.0,
                dst_start_row: start.1,
                row_count: run_len,
            });
        }

        start = (a, b);
        prev = (a, b);
        current_offset = offset;
        run_len = 1;
    }

    if run_len > 1 && current_offset != 0 {
        moves.push(RowBlockMove {
            src_start_row: start.0,
            dst_start_row: start.1,
            row_count: run_len,
        });
    }

    moves
}
