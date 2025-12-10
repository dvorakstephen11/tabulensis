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

use crate::alignment::row_metadata::RowMeta;
use crate::alignment::RowBlockMove;
use crate::workbook::RowSignature;

pub fn find_block_move(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    min_len: u32,
) -> Option<RowBlockMove> {
    let mut positions: HashMap<RowSignature, Vec<usize>> = HashMap::new();
    for (idx, meta) in old_slice.iter().enumerate() {
        positions.entry(meta.signature).or_default().push(idx);
    }

    let mut best: Option<RowBlockMove> = None;

    for (new_idx, meta) in new_slice.iter().enumerate() {
        if let Some(candidates) = positions.get(&meta.signature) {
            for &old_idx in candidates {
                let mut len = 0usize;
                while old_idx + len < old_slice.len()
                    && new_idx + len < new_slice.len()
                    && old_slice[old_idx + len].signature == new_slice[new_idx + len].signature
                {
                    len += 1;
                }

                if len as u32 >= min_len {
                    let mv = RowBlockMove {
                        src_start_row: old_slice[old_idx].row_idx,
                        dst_start_row: new_slice[new_idx].row_idx,
                        row_count: len as u32,
                    };
                    let take = best
                        .as_ref()
                        .map_or(true, |b| mv.row_count > b.row_count);
                    if take {
                        best = Some(mv);
                    }
                }
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
