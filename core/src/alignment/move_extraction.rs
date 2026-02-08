//! Move extraction for AMR alignment.
//!
//! Implements both localized move detection within gaps and the global
//! unanchored match pipeline (Sections 9.5-9.7, 11).
//!
//! ## Current Implementation
//!
//! - `find_block_move`: Scans for contiguous blocks of matching signatures
//!   between old and new slices within a gap. Returns the largest found.
//!
//! - `moves_from_matched_pairs`: Extracts block moves from matched row pairs
//!   where consecutive pairs have the same offset (indicating they moved together).
//!
//! - `extract_global_moves`: Global unanchored match collection, candidate block
//!   construction, LAP assignment, and validation to produce non-overlapping moves.

use std::collections::{HashMap, HashSet};

use crate::alignment::anchor_discovery::Anchor;
use crate::alignment::lap;
use crate::alignment::RowBlockMove;
use crate::config::DiffConfig;
use crate::grid_metadata::{FrequencyClass, RowMeta};
use crate::workbook::RowSignature;

pub fn find_block_move(
    old_slice: &[RowMeta],
    new_slice: &[RowMeta],
    min_len: u32,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    let max_slice_len = config.moves.move_extraction_max_slice_len as usize;
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

        let max_candidates = config.moves.move_extraction_max_candidates_per_sig as usize;
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

#[derive(Clone, Copy, Debug)]
struct MoveCandidate {
    src_start_row: u32,
    dst_start_row: u32,
    row_count: u32,
    similarity: f64,
}

#[derive(Clone, Copy, Debug)]
struct CandidateSeed {
    src_start_row: u32,
    dst_start_row: u32,
    row_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BlockRange {
    start: u32,
    len: u32,
}

#[derive(Clone, Copy, Debug)]
struct MatchPair {
    a: u32,
    b: u32,
    offset: i64,
}

pub(crate) fn extract_global_moves(
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    anchors: &[Anchor],
    config: &DiffConfig,
) -> Vec<RowBlockMove> {
    let pairs = collect_unanchored_pairs(old_meta, new_meta, anchors, config);
    if pairs.is_empty() {
        return Vec::new();
    }

    let candidates = build_candidate_blocks(&pairs, old_meta, new_meta, config);
    if candidates.is_empty() {
        return Vec::new();
    }

    let assigned = assign_moves(&candidates, config);
    let validated = validate_moves(assigned, old_meta, new_meta, config);
    let resolved = resolve_overlaps(validated);

    resolved
        .into_iter()
        .map(|cand| RowBlockMove {
            src_start_row: cand.src_start_row,
            dst_start_row: cand.dst_start_row,
            row_count: cand.row_count,
        })
        .collect()
}

fn collect_unanchored_pairs(
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    anchors: &[Anchor],
    config: &DiffConfig,
) -> Vec<(u32, u32)> {
    let anchored_old: HashSet<u32> = anchors.iter().map(|a| a.old_row).collect();
    let anchored_new: HashSet<u32> = anchors.iter().map(|a| a.new_row).collect();

    let old_map = collect_unanchored_by_signature(old_meta, &anchored_old, config);
    let new_map = collect_unanchored_by_signature(new_meta, &anchored_new, config);

    let max_per_sig = config.moves.move_extraction_max_candidates_per_sig as usize;
    let max_total = config.moves.move_extraction_max_slice_len as usize;

    let mut pairs = Vec::new();
    for (sig, old_rows) in old_map {
        let Some(new_rows) = new_map.get(&sig) else {
            continue;
        };

        for &a in old_rows.iter().take(max_per_sig) {
            for &b in new_rows.iter().take(max_per_sig) {
                pairs.push((a, b));
                if pairs.len() >= max_total {
                    pairs.sort_by_key(|(x, y)| (*x, *y));
                    return pairs;
                }
            }
        }
    }

    pairs.sort_by_key(|(a, b)| (*a, *b));
    pairs
}

fn collect_unanchored_by_signature(
    rows: &[RowMeta],
    anchored: &HashSet<u32>,
    config: &DiffConfig,
) -> HashMap<RowSignature, Vec<u32>> {
    let mut map: HashMap<RowSignature, Vec<u32>> = HashMap::new();

    for meta in rows {
        if anchored.contains(&meta.row_idx) {
            continue;
        }
        if meta.is_low_info() {
            continue;
        }
        if !matches!(
            meta.frequency_class,
            FrequencyClass::Unique | FrequencyClass::Rare
        ) {
            continue;
        }
        if config.moves.move_extraction_max_candidates_per_sig == 0 {
            continue;
        }
        map.entry(meta.signature).or_default().push(meta.row_idx);
    }

    map
}

fn build_candidate_blocks(
    pairs: &[(u32, u32)],
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    config: &DiffConfig,
) -> Vec<MoveCandidate> {
    if pairs.is_empty() {
        return Vec::new();
    }

    let max_gap = config.alignment.small_gap_threshold.max(1) as u32;
    let min_len = config.moves.min_block_size_for_move.max(1);
    let max_len = config.moves.move_extraction_max_slice_len.max(1);
    let threshold = move_similarity_threshold(config);

    let mut sorted: Vec<MatchPair> = pairs
        .iter()
        .map(|(a, b)| MatchPair {
            a: *a,
            b: *b,
            offset: *b as i64 - *a as i64,
        })
        .collect();
    sorted.sort_by_key(|p| (p.a, p.b));

    let mut seeds: Vec<CandidateSeed> = Vec::new();
    let mut seen: HashSet<(u32, u32, u32)> = HashSet::new();

    let mut start = sorted[0];
    let mut prev = sorted[0];
    let mut offset = sorted[0].offset;

    for pair in sorted.iter().skip(1) {
        let delta = pair.a.saturating_sub(prev.a);
        if pair.offset == offset && delta > 0 && delta <= max_gap {
            prev = *pair;
            continue;
        }

        push_candidate_seed(start, prev, min_len, max_len, &mut seeds, &mut seen);

        start = *pair;
        prev = *pair;
        offset = pair.offset;
    }

    push_candidate_seed(start, prev, min_len, max_len, &mut seeds, &mut seen);

    let mut candidates = score_candidates(old_meta, new_meta, threshold, &seeds);
    candidates.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.src_start_row.cmp(&b.src_start_row))
            .then_with(|| a.dst_start_row.cmp(&b.dst_start_row))
            .then_with(|| b.row_count.cmp(&a.row_count))
    });
    candidates
}

fn push_candidate_seed(
    start: MatchPair,
    end: MatchPair,
    min_len: u32,
    max_len: u32,
    seeds: &mut Vec<CandidateSeed>,
    seen: &mut HashSet<(u32, u32, u32)>,
) {
    if end.a < start.a {
        return;
    }
    let row_count = end.a - start.a + 1;
    if row_count < min_len || row_count > max_len {
        return;
    }

    let key = (start.a, start.b, row_count);
    if !seen.insert(key) {
        return;
    }

    seeds.push(CandidateSeed {
        src_start_row: start.a,
        dst_start_row: start.b,
        row_count,
    });
}

fn score_candidates(
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    threshold: f64,
    seeds: &[CandidateSeed],
) -> Vec<MoveCandidate> {
    if seeds.is_empty() {
        return Vec::new();
    }

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        let scored: Vec<Option<MoveCandidate>> = seeds
            .par_iter()
            .map(|seed| {
                let similarity = block_similarity(
                    old_meta,
                    new_meta,
                    seed.src_start_row,
                    seed.dst_start_row,
                    seed.row_count,
                );
                if similarity < threshold {
                    None
                } else {
                    Some(MoveCandidate {
                        src_start_row: seed.src_start_row,
                        dst_start_row: seed.dst_start_row,
                        row_count: seed.row_count,
                        similarity,
                    })
                }
            })
            .collect();
        scored.into_iter().flatten().collect()
    }

    #[cfg(not(feature = "parallel"))]
    let mut out = Vec::new();
    #[cfg(not(feature = "parallel"))]
    for seed in seeds {
        let similarity = block_similarity(
            old_meta,
            new_meta,
            seed.src_start_row,
            seed.dst_start_row,
            seed.row_count,
        );
        if similarity < threshold {
            continue;
        }
        out.push(MoveCandidate {
            src_start_row: seed.src_start_row,
            dst_start_row: seed.dst_start_row,
            row_count: seed.row_count,
            similarity,
        });
    }
    #[cfg(not(feature = "parallel"))]
    out
}

fn assign_moves(candidates: &[MoveCandidate], config: &DiffConfig) -> Vec<MoveCandidate> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let mut src_blocks: Vec<BlockRange> = Vec::new();
    let mut dst_blocks: Vec<BlockRange> = Vec::new();
    let mut src_index: HashMap<BlockRange, usize> = HashMap::new();
    let mut dst_index: HashMap<BlockRange, usize> = HashMap::new();

    for cand in candidates {
        let src = BlockRange {
            start: cand.src_start_row,
            len: cand.row_count,
        };
        let dst = BlockRange {
            start: cand.dst_start_row,
            len: cand.row_count,
        };
        if !src_index.contains_key(&src) {
            src_index.insert(src, src_blocks.len());
            src_blocks.push(src);
        }
        if !dst_index.contains_key(&dst) {
            dst_index.insert(dst, dst_blocks.len());
            dst_blocks.push(dst);
        }
    }

    if src_blocks.is_empty() || dst_blocks.is_empty() {
        return Vec::new();
    }

    let mut similarity_map: HashMap<(usize, usize), f64> = HashMap::new();
    for cand in candidates {
        let src = BlockRange {
            start: cand.src_start_row,
            len: cand.row_count,
        };
        let dst = BlockRange {
            start: cand.dst_start_row,
            len: cand.row_count,
        };
        let Some(&i) = src_index.get(&src) else {
            continue;
        };
        let Some(&j) = dst_index.get(&dst) else {
            continue;
        };
        let entry = similarity_map.entry((i, j)).or_insert(0.0);
        if cand.similarity > *entry {
            *entry = cand.similarity;
        }
    }

    let size = src_blocks.len().max(dst_blocks.len());
    let scale = 1000i64;
    let mut costs = vec![vec![scale; size]; size];

    let threshold = move_similarity_threshold(config);
    for ((i, j), sim) in similarity_map.iter() {
        if *sim < threshold {
            continue;
        }
        if let (Some(src), Some(dst)) = (src_blocks.get(*i), dst_blocks.get(*j)) {
            if src.len != dst.len {
                continue;
            }
            let cost = ((1.0 - sim) * scale as f64).round() as i64;
            costs[*i][*j] = cost;
        }
    }

    let assignment = lap::solve(&costs);
    let mut out = Vec::new();
    for (i, &j) in assignment.iter().enumerate() {
        if i >= src_blocks.len() || j >= dst_blocks.len() {
            continue;
        }
        let Some(sim) = similarity_map.get(&(i, j)).copied() else {
            continue;
        };
        if sim < threshold {
            continue;
        }
        let src = src_blocks[i];
        let dst = dst_blocks[j];
        if src.len != dst.len || src.len == 0 {
            continue;
        }
        out.push(MoveCandidate {
            src_start_row: src.start,
            dst_start_row: dst.start,
            row_count: src.len,
            similarity: sim,
        });
    }
    out
}

fn validate_moves(
    candidates: Vec<MoveCandidate>,
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    config: &DiffConfig,
) -> Vec<MoveCandidate> {
    candidates
        .into_iter()
        .filter(|cand| {
            block_similarity(
                old_meta,
                new_meta,
                cand.src_start_row,
                cand.dst_start_row,
                cand.row_count,
            ) >= move_similarity_threshold(config)
        })
        .collect()
}

fn resolve_overlaps(mut candidates: Vec<MoveCandidate>) -> Vec<MoveCandidate> {
    candidates.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.row_count.cmp(&a.row_count))
            .then_with(|| a.src_start_row.cmp(&b.src_start_row))
            .then_with(|| a.dst_start_row.cmp(&b.dst_start_row))
    });

    let mut accepted: Vec<MoveCandidate> = Vec::new();
    for cand in candidates {
        if accepted.iter().any(|m| ranges_overlap(m, &cand)) {
            continue;
        }
        accepted.push(cand);
    }

    accepted.sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));
    accepted
}

fn ranges_overlap(a: &MoveCandidate, b: &MoveCandidate) -> bool {
    let a_src_end = a.src_start_row.saturating_add(a.row_count);
    let b_src_end = b.src_start_row.saturating_add(b.row_count);
    let a_dst_end = a.dst_start_row.saturating_add(a.row_count);
    let b_dst_end = b.dst_start_row.saturating_add(b.row_count);

    let src_overlap = a.src_start_row < b_src_end && b.src_start_row < a_src_end;
    let dst_overlap = a.dst_start_row < b_dst_end && b.dst_start_row < a_dst_end;

    src_overlap || dst_overlap
}

fn block_similarity(
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    src_start: u32,
    dst_start: u32,
    row_count: u32,
) -> f64 {
    if row_count == 0 {
        return 0.0;
    }
    let len = row_count as usize;
    let end_a = src_start as usize + len;
    let end_b = dst_start as usize + len;
    if end_a > old_meta.len() || end_b > new_meta.len() {
        return 0.0;
    }

    let mut matches = 0u32;
    let mut total = 0u32;

    for offset in 0..len {
        let a = &old_meta[src_start as usize + offset];
        let b = &new_meta[dst_start as usize + offset];
        total = total.saturating_add(1);
        if a.is_low_info() || b.is_low_info() {
            continue;
        }
        if a.signature == b.signature {
            matches = matches.saturating_add(1);
        }
    }

    if total == 0 {
        0.0
    } else {
        (matches as f64 + 1.0) / (total as f64 + 1.0)
    }
}

fn move_similarity_threshold(config: &DiffConfig) -> f64 {
    if config.moves.enable_fuzzy_moves {
        config.moves.fuzzy_similarity_threshold
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DiffConfig;
    use crate::workbook::RowSignature;

    fn make_meta(row_idx: u32, hash: u128, class: FrequencyClass) -> RowMeta {
        RowMeta {
            row_idx,
            signature: RowSignature { hash },
            non_blank_count: 1,
            first_non_blank_col: 0,
            frequency_class: class,
            is_low_info: matches!(class, FrequencyClass::LowInfo),
        }
    }

    #[test]
    fn global_moves_ignore_low_info_rows() {
        let old = vec![
            make_meta(0, 1, FrequencyClass::LowInfo),
            make_meta(1, 2, FrequencyClass::LowInfo),
        ];
        let new = vec![
            make_meta(0, 1, FrequencyClass::LowInfo),
            make_meta(1, 2, FrequencyClass::LowInfo),
        ];

        let config = DiffConfig::default();
        let moves = extract_global_moves(&old, &new, &[], &config);
        assert!(moves.is_empty());
    }

    #[test]
    fn global_moves_detects_simple_block_move() {
        let old = vec![
            make_meta(0, 1, FrequencyClass::Unique),
            make_meta(1, 2, FrequencyClass::Unique),
            make_meta(2, 3, FrequencyClass::Unique),
            make_meta(3, 4, FrequencyClass::Unique),
            make_meta(4, 5, FrequencyClass::Unique),
            make_meta(5, 6, FrequencyClass::Unique),
        ];
        let new = vec![
            make_meta(0, 1, FrequencyClass::Unique),
            make_meta(1, 5, FrequencyClass::Unique),
            make_meta(2, 6, FrequencyClass::Unique),
            make_meta(3, 2, FrequencyClass::Unique),
            make_meta(4, 3, FrequencyClass::Unique),
            make_meta(5, 4, FrequencyClass::Unique),
        ];

        let config = DiffConfig::default();
        let moves = extract_global_moves(&old, &new, &[], &config);

        assert!(
            moves
                .iter()
                .any(|mv| mv.src_start_row == 1 && mv.dst_start_row == 3 && mv.row_count == 3),
            "expected move for block [2,3,4] shifted down"
        );
    }

    #[test]
    fn candidate_pairs_are_capped_per_signature() {
        let mut old = Vec::new();
        let mut new = Vec::new();
        for idx in 0..5u32 {
            old.push(make_meta(idx, 42, FrequencyClass::Rare));
            new.push(make_meta(idx, 42, FrequencyClass::Rare));
        }

        let mut config = DiffConfig::default();
        config.moves.move_extraction_max_candidates_per_sig = 2;
        config.moves.move_extraction_max_slice_len = 100;

        let pairs = collect_unanchored_pairs(&old, &new, &[], &config);
        assert!(pairs.len() <= 4);
    }
}
