//! Legacy row alignment algorithms (pre-AMR).
//!
//! This module contains the original row alignment implementation that predates
//! the Anchor-Move-Refine (AMR) algorithm in `alignment/`. These functions are
//! retained for:
//!
//! 1. **Fallback scenarios**: The engine may use these when AMR cannot produce
//!    a useful alignment (e.g., heavily repetitive data).
//!
//! 2. **Move detection helpers**: Some functions (`detect_exact_row_block_move`,
//!    `detect_fuzzy_row_block_move`) are still used by the engine's
//!    masked move detection logic.
//!
//! 3. **Test coverage**: Unit tests validate these algorithms work correctly.
//!
//! ## Migration Status
//!
//! The primary alignment path now uses `alignment::align_rows_amr`. The legacy
//! functions are invoked only when:
//! - AMR returns `None` (fallback to `align_row_changes`)
//! - Explicit move detection in masked regions
//!
//! Functions marked `#[allow(dead_code)]` are retained for testing but not
//! called from production code paths.

use std::collections::HashSet;

use crate::config::DiffConfig;
use crate::grid_view::{GridView, HashStats, RowHash, RowMeta};
use crate::workbook::Grid;

pub(crate) use crate::alignment::{RowAlignment, RowBlockMove};

const _HASH_COLLISION_NOTE: &str = "128-bit xxHash3 collision probability ~10^-29 at 50K rows (birthday bound); \
     secondary verification not required; see hashing.rs for detailed rationale.";

pub(crate) fn detect_exact_row_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    let meta_a = &view_a.row_meta;
    let meta_b = &view_b.row_meta;
    let n = meta_a.len();

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let prefix = (0..n).find(|&idx| meta_a[idx].hash != meta_b[idx].hash)?;

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = n - 1 - suffix_len;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }
    let tail_start = n - suffix_len;

    let try_candidate = |src_start: usize, dst_start: usize| -> Option<RowBlockMove> {
        if src_start >= tail_start || dst_start >= tail_start {
            return None;
        }

        let mut len = 0usize;
        while src_start + len < tail_start && dst_start + len < tail_start {
            if meta_a[src_start + len].hash != meta_b[dst_start + len].hash {
                break;
            }
            len += 1;
        }

        if len == 0 {
            return None;
        }

        let src_end = src_start + len;
        let dst_end = dst_start + len;

        if !(src_end <= dst_start || dst_end <= src_start) {
            return None;
        }

        let mut idx_a = 0usize;
        let mut idx_b = 0usize;

        loop {
            if idx_a == src_start {
                idx_a = src_end;
            }
            if idx_b == dst_start {
                idx_b = dst_end;
            }

            if idx_a >= n && idx_b >= n {
                break;
            }

            if idx_a >= n || idx_b >= n {
                return None;
            }

            if meta_a[idx_a].hash != meta_b[idx_b].hash {
                return None;
            }

            idx_a += 1;
            idx_b += 1;
        }

        for meta in &meta_a[src_start..src_end] {
            if stats.freq_a.get(&meta.hash).copied().unwrap_or(0) != 1
                || stats.freq_b.get(&meta.hash).copied().unwrap_or(0) != 1
            {
                return None;
            }
        }

        Some(RowBlockMove {
            src_start_row: meta_a[src_start].row_idx,
            dst_start_row: meta_b[dst_start].row_idx,
            row_count: len as u32,
        })
    };

    if let Some(src_start) =
        (prefix..tail_start).find(|&idx| meta_a[idx].hash == meta_b[prefix].hash)
        && let Some(mv) = try_candidate(src_start, prefix)
    {
        return Some(mv);
    }

    if let Some(dst_start) =
        (prefix..tail_start).find(|&idx| meta_b[idx].hash == meta_a[prefix].hash)
        && let Some(mv) = try_candidate(prefix, dst_start)
    {
        return Some(mv);
    }

    None
}

pub(crate) fn detect_fuzzy_row_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if low_info_dominated(&view_a) || low_info_dominated(&view_b) {
        return None;
    }

    let stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    let meta_a = &view_a.row_meta;
    let meta_b = &view_b.row_meta;

    if meta_a
        .iter()
        .zip(meta_b.iter())
        .all(|(a, b)| a.hash == b.hash)
    {
        return None;
    }

    let n = meta_a.len();
    let mut prefix = 0usize;
    while prefix < n && meta_a[prefix].hash == meta_b[prefix].hash {
        prefix += 1;
    }
    if prefix == n {
        return None;
    }

    let mut suffix_len = 0usize;
    while suffix_len < n.saturating_sub(prefix) {
        let idx_a = n - 1 - suffix_len;
        let idx_b = idx_a;
        if meta_a[idx_a].hash == meta_b[idx_b].hash {
            suffix_len += 1;
        } else {
            break;
        }
    }

    let mismatch_end = n - suffix_len;
    if mismatch_end <= prefix {
        return None;
    }

    let mid_len = mismatch_end - prefix;
    if mid_len <= 1 {
        return None;
    }

    let max_block_len = mid_len
        .saturating_sub(1)
        .min(config.max_fuzzy_block_rows as usize);
    if max_block_len == 0 {
        return None;
    }

    let mut candidate: Option<RowBlockMove> = None;

    for block_len in 1..=max_block_len {
        let remaining = mid_len - block_len;

        // Block moved upward: [middle][block] -> [block'][middle]
        if hashes_match(
            &meta_a[prefix..prefix + remaining],
            &meta_b[prefix + block_len..mismatch_end],
        ) {
            let src_block = &meta_a[prefix + remaining..mismatch_end];
            let dst_block = &meta_b[prefix..prefix + block_len];

            if block_similarity(src_block, dst_block) >= config.fuzzy_similarity_threshold {
                let mv = RowBlockMove {
                    src_start_row: src_block[0].row_idx,
                    dst_start_row: dst_block[0].row_idx,
                    row_count: block_len as u32,
                };
                if mv.src_start_row != mv.dst_start_row {
                    if candidate.is_some() {
                        return None;
                    }
                    candidate = Some(mv);
                }
            }
        }

        // Block moved downward: [block][middle] -> [middle][block']
        if hashes_match(
            &meta_a[prefix + block_len..mismatch_end],
            &meta_b[prefix..prefix + remaining],
        ) {
            let src_block = &meta_a[prefix..prefix + block_len];
            let dst_block = &meta_b[prefix + remaining..mismatch_end];

            if block_similarity(src_block, dst_block) >= config.fuzzy_similarity_threshold {
                let mv = RowBlockMove {
                    src_start_row: src_block[0].row_idx,
                    dst_start_row: dst_block[0].row_idx,
                    row_count: block_len as u32,
                };
                if mv.src_start_row != mv.dst_start_row {
                    if candidate.is_some() {
                        return None;
                    }
                    candidate = Some(mv);
                }
            }
        }
    }

    candidate
}

#[allow(dead_code)]
pub(crate) fn align_row_changes(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    align_row_changes_from_views(&view_a, &view_b, config)
}

pub(crate) fn align_row_changes_from_views(
    old_view: &GridView,
    new_view: &GridView,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    let row_diff = new_view.source.nrows as i64 - old_view.source.nrows as i64;
    if row_diff.abs() == 1 {
        return align_single_row_change_from_views(old_view, new_view, config);
    }

    align_rows_internal(old_view, new_view, true, config)
}

#[allow(dead_code)]
pub(crate) fn align_single_row_change(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);
    align_single_row_change_from_views(&view_a, &view_b, config)
}

pub(crate) fn align_single_row_change_from_views(
    old_view: &GridView,
    new_view: &GridView,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    align_rows_internal(old_view, new_view, false, config)
}

fn align_rows_internal(
    old_view: &GridView,
    new_view: &GridView,
    allow_blocks: bool,
    config: &DiffConfig,
) -> Option<RowAlignment> {
    if !is_within_size_bounds(old_view.source, new_view.source, config) {
        return None;
    }

    if old_view.source.ncols != new_view.source.ncols {
        return None;
    }

    let row_diff = new_view.source.nrows as i64 - old_view.source.nrows as i64;
    if row_diff == 0 {
        return None;
    }

    let abs_diff = row_diff.unsigned_abs() as u32;

    if !allow_blocks && abs_diff != 1 {
        return None;
    }

    if abs_diff != 1 && (!allow_blocks || abs_diff > config.max_block_gap) {
        return None;
    }

    if low_info_dominated(old_view) || low_info_dominated(new_view) {
        return None;
    }

    let stats = HashStats::from_row_meta(&old_view.row_meta, &new_view.row_meta);
    if has_heavy_repetition(&stats, config) {
        return None;
    }

    if row_diff == 1 {
        find_single_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Insert,
        )
    } else if row_diff == -1 {
        find_single_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Delete,
        )
    } else if !allow_blocks {
        None
    } else if row_diff > 0 {
        find_block_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Insert,
            abs_diff,
        )
    } else {
        find_block_gap_alignment(
            &old_view.row_meta,
            &new_view.row_meta,
            &stats,
            RowChange::Delete,
            abs_diff,
        )
    }
}

enum RowChange {
    Insert,
    Delete,
}

fn find_single_gap_alignment(
    rows_a: &[crate::grid_view::RowMeta],
    rows_b: &[crate::grid_view::RowMeta],
    stats: &HashStats<RowHash>,
    change: RowChange,
) -> Option<RowAlignment> {
    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = false;

    let mut idx_a = 0usize;
    let mut idx_b = 0usize;

    while idx_a < rows_a.len() && idx_b < rows_b.len() {
        let meta_a = rows_a[idx_a];
        let meta_b = rows_b[idx_b];

        if meta_a.hash == meta_b.hash {
            matched.push((meta_a.row_idx, meta_b.row_idx));
            idx_a += 1;
            idx_b += 1;
            continue;
        }

        if skipped {
            return None;
        }

        match change {
            RowChange::Insert => {
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
                idx_b += 1;
            }
            RowChange::Delete => {
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
                idx_a += 1;
            }
        }

        skipped = true;
    }

    if idx_a < rows_a.len() || idx_b < rows_b.len() {
        if skipped {
            return None;
        }

        match change {
            RowChange::Insert if idx_a == rows_a.len() && rows_b.len() == idx_b + 1 => {
                let meta_b = rows_b[idx_b];
                if !is_unique_to_b(meta_b.hash, stats) {
                    return None;
                }
                inserted.push(meta_b.row_idx);
            }
            RowChange::Delete if idx_b == rows_b.len() && rows_a.len() == idx_a + 1 => {
                let meta_a = rows_a[idx_a];
                if !is_unique_to_a(meta_a.hash, stats) {
                    return None;
                }
                deleted.push(meta_a.row_idx);
            }
            _ => return None,
        }
    }

    if inserted.len() + deleted.len() != 1 {
        return None;
    }

    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn find_block_gap_alignment(
    rows_a: &[crate::grid_view::RowMeta],
    rows_b: &[crate::grid_view::RowMeta],
    stats: &HashStats<RowHash>,
    change: RowChange,
    gap: u32,
) -> Option<RowAlignment> {
    let gap = gap as usize;
    if gap == 0 {
        return None;
    }

    let (shorter_len, longer_len) = match change {
        RowChange::Insert => (rows_a.len(), rows_b.len()),
        RowChange::Delete => (rows_b.len(), rows_a.len()),
    };

    if longer_len.saturating_sub(shorter_len) != gap {
        return None;
    }

    let mut prefix = 0usize;
    while prefix < rows_a.len()
        && prefix < rows_b.len()
        && rows_a[prefix].hash == rows_b[prefix].hash
    {
        prefix += 1;
    }

    let mut suffix = 0usize;
    while suffix < shorter_len.saturating_sub(prefix) {
        let idx_a = rows_a.len() - 1 - suffix;
        let idx_b = rows_b.len() - 1 - suffix;
        if rows_a[idx_a].hash == rows_b[idx_b].hash {
            suffix += 1;
        } else {
            break;
        }
    }

    if prefix + suffix != shorter_len {
        return None;
    }

    let mut matched = Vec::with_capacity(shorter_len);
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    match change {
        RowChange::Insert => {
            let block_start = prefix;
            let block_end = block_start + gap;
            if block_end > rows_b.len() {
                return None;
            }

            for meta in &rows_b[block_start..block_end] {
                if !is_unique_to_b(meta.hash, stats) {
                    return None;
                }
                inserted.push(meta.row_idx);
            }

            for (idx, meta_a) in rows_a.iter().enumerate() {
                let b_idx = if idx < block_start { idx } else { idx + gap };
                matched.push((meta_a.row_idx, rows_b[b_idx].row_idx));
            }
        }
        RowChange::Delete => {
            let block_start = prefix;
            let block_end = block_start + gap;
            if block_end > rows_a.len() {
                return None;
            }

            for meta in &rows_a[block_start..block_end] {
                if !is_unique_to_a(meta.hash, stats) {
                    return None;
                }
                deleted.push(meta.row_idx);
            }

            for (idx_b, meta_b) in rows_b.iter().enumerate() {
                let a_idx = if idx_b < block_start {
                    idx_b
                } else {
                    idx_b + gap
                };
                matched.push((rows_a[a_idx].row_idx, meta_b.row_idx));
            }
        }
    }

    let alignment = RowAlignment {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    };

    debug_assert!(
        is_monotonic(&alignment.matched),
        "matched pairs must be strictly increasing in both dimensions"
    );

    Some(alignment)
}

fn is_monotonic(pairs: &[(u32, u32)]) -> bool {
    pairs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1)
}

fn is_unique_to_b(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 0
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 1
}

fn is_unique_to_a(hash: RowHash, stats: &HashStats<RowHash>) -> bool {
    stats.freq_a.get(&hash).copied().unwrap_or(0) == 1
        && stats.freq_b.get(&hash).copied().unwrap_or(0) == 0
}

fn is_within_size_bounds(old: &Grid, new: &Grid, config: &DiffConfig) -> bool {
    let rows = old.nrows.max(new.nrows);
    let cols = old.ncols.max(new.ncols);
    rows <= config.max_align_rows && cols <= config.max_align_cols
}

fn low_info_dominated(view: &GridView<'_>) -> bool {
    if view.row_meta.is_empty() {
        return false;
    }

    let low_info_count = view.row_meta.iter().filter(|m| m.is_low_info).count();
    low_info_count * 2 > view.row_meta.len()
}

fn has_heavy_repetition(stats: &HashStats<RowHash>, config: &DiffConfig) -> bool {
    stats
        .freq_a
        .values()
        .chain(stats.freq_b.values())
        .copied()
        .max()
        .unwrap_or(0)
        > config.max_hash_repeat
}

fn hashes_match(slice_a: &[RowMeta], slice_b: &[RowMeta]) -> bool {
    slice_a.len() == slice_b.len()
        && slice_a
            .iter()
            .zip(slice_b.iter())
            .all(|(a, b)| a.hash == b.hash)
}

fn block_similarity(slice_a: &[RowMeta], slice_b: &[RowMeta]) -> f64 {
    let tokens_a: HashSet<RowHash> = slice_a.iter().map(|m| m.hash).collect();
    let tokens_b: HashSet<RowHash> = slice_b.iter().map(|m| m.hash).collect();

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();
    let jaccard = if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    };

    let positional_matches = slice_a
        .iter()
        .zip(slice_b.iter())
        .filter(|(a, b)| a.hash == b.hash)
        .count();
    let positional_ratio = (positional_matches as f64 + 1.0) / (slice_a.len() as f64 + 1.0);

    jaccard.max(positional_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::CellValue;

    fn grid_from_rows(rows: &[&[i32]]) -> Grid {
        let nrows = rows.len() as u32;
        let ncols = if nrows == 0 { 0 } else { rows[0].len() as u32 };
        let mut grid = Grid::new(nrows, ncols);

        for (r_idx, row_vals) in rows.iter().enumerate() {
            for (c_idx, value) in row_vals.iter().enumerate() {
                grid.insert_cell(
                    r_idx as u32,
                    c_idx as u32,
                    Some(CellValue::Number(*value as f64)),
                    None,
                );
            }
        }

        grid
    }

    #[test]
    fn detects_exact_row_block_move() {
        let base: Vec<Vec<i32>> = (1..=20)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
        rows_b.splice(12..12, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let mv = detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected block move to be found");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 12,
                row_count: 4
            }
        );
    }

    #[test]
    fn block_move_detection_rejects_internal_edits() {
        let base: Vec<Vec<i32>> = (1..=12)
            .map(|r| (1..=2).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(2..5).collect();
        moved_block[1][0] = 9_999;
        rows_b.splice(6..6, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detects_fuzzy_row_block_move_with_single_internal_edit() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..8).collect();
        moved_block[1][1] = 9_999;
        rows_b.splice(12..12, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected fuzzy row block move to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 12,
                row_count: 4
            }
        );
    }

    #[test]
    fn fuzzy_move_rejects_low_similarity_block() {
        let base: Vec<Vec<i32>> = (1..=16)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(3..7).collect();
        for row in &mut moved_block {
            for value in row.iter_mut() {
                *value += 50_000;
            }
        }
        rows_b.splice(10..10, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "similarity below threshold should bail out"
        );
    }

    #[test]
    fn fuzzy_move_bails_on_heavy_repetition_or_ambiguous_candidates() {
        let repeated_row = [1, 2];
        let rows_a: Vec<Vec<i32>> = (0..10).map(|_| repeated_row.to_vec()).collect();
        let mut rows_b = rows_a.clone();

        let block: Vec<Vec<i32>> = rows_b.drain(0..3).collect();
        rows_b.splice(5..5, block);

        let rows_a_refs: Vec<&[i32]> = rows_a.iter().map(|row| row.as_slice()).collect();
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&rows_a_refs);
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "heavy repetition or ambiguous candidates should not emit a move"
        );
    }

    #[test]
    fn fuzzy_move_noop_when_grids_identical() {
        let base: Vec<Vec<i32>> = (1..=6)
            .map(|r| (1..=2).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);
        let grid_b = grid_from_rows(&base_refs);

        assert!(detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
        assert!(detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn detects_fuzzy_row_block_move_upward_with_single_internal_edit() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(12..16).collect();
        moved_block[1][1] = 9_999;
        rows_b.splice(4..4, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default())
            .expect("expected fuzzy row block move upward to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 12,
                dst_start_row: 4,
                row_count: 4
            }
        );
    }

    #[test]
    fn fuzzy_move_bails_on_ambiguous_candidates_below_repetition_threshold() {
        let base: Vec<Vec<i32>> = (1..=16)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_baseline_a = grid_from_rows(&base_refs);

        let mut rows_baseline_b = base.clone();
        let mut moved: Vec<Vec<i32>> = rows_baseline_b.drain(3..7).collect();
        moved[1][1] = 9999;
        rows_baseline_b.splice(10..10, moved);
        let refs_baseline_b: Vec<&[i32]> =
            rows_baseline_b.iter().map(|row| row.as_slice()).collect();
        let grid_baseline_b = grid_from_rows(&refs_baseline_b);

        assert!(
            detect_fuzzy_row_block_move(&grid_baseline_a, &grid_baseline_b, &DiffConfig::default())
                .is_some(),
            "baseline: non-ambiguous fuzzy move should be detected"
        );

        let rows_a: Vec<Vec<i32>> = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![100, 200, 300],
            vec![101, 201, 301],
            vec![102, 202, 302],
            vec![103, 203, 303],
            vec![100, 200, 300],
            vec![101, 201, 301],
            vec![102, 202, 302],
            vec![103, 203, 999],
            vec![31, 32, 33],
            vec![34, 35, 36],
        ];

        let mut rows_b = rows_a.clone();
        let block1: Vec<Vec<i32>> = rows_b.drain(2..6).collect();
        rows_b.splice(6..6, block1);

        let refs_a: Vec<&[i32]> = rows_a.iter().map(|r| r.as_slice()).collect();
        let refs_b: Vec<&[i32]> = rows_b.iter().map(|r| r.as_slice()).collect();
        let grid_a = grid_from_rows(&refs_a);
        let grid_b = grid_from_rows(&refs_b);

        assert!(
            detect_fuzzy_row_block_move(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "ambiguous candidates: two similar blocks swapped should trigger ambiguity bail-out"
        );
    }

    #[test]
    fn fuzzy_move_at_max_block_rows_threshold() {
        let config = DiffConfig::default();
        let base: Vec<Vec<i32>> = (1..=70)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_b.drain(4..36).collect();
        moved_block[15][1] = 9_999;
        rows_b.splice(36..36, moved_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(
            detect_exact_row_block_move(&grid_a, &grid_b, &config).is_none(),
            "internal edits should prevent exact move detection"
        );

        let mv = detect_fuzzy_row_block_move(&grid_a, &grid_b, &config)
            .expect("expected fuzzy move at configured max_fuzzy_block_rows to be detected");
        assert_eq!(
            mv,
            RowBlockMove {
                src_start_row: 4,
                dst_start_row: 36,
                row_count: config.max_fuzzy_block_rows
            }
        );
    }

    #[test]
    fn fuzzy_move_at_max_hash_repeat_boundary() {
        let base: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_base = grid_from_rows(&base_refs);

        let mut rows_moved = base.clone();
        let mut moved_block: Vec<Vec<i32>> = rows_moved.drain(4..8).collect();
        moved_block[1][1] = 9_999;
        rows_moved.splice(12..12, moved_block);
        let moved_refs: Vec<&[i32]> = rows_moved.iter().map(|row| row.as_slice()).collect();
        let grid_moved = grid_from_rows(&moved_refs);

        assert!(
            detect_fuzzy_row_block_move(&grid_base, &grid_moved, &DiffConfig::default()).is_some(),
            "baseline: fuzzy move should work with unique rows"
        );

        let mut base_9_repeat: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        for row in base_9_repeat.iter_mut().take(9) {
            *row = vec![999, 888, 777];
        }
        let refs_9a: Vec<&[i32]> = base_9_repeat.iter().map(|r| r.as_slice()).collect();
        let grid_9a = grid_from_rows(&refs_9a);

        let mut rows_9b = base_9_repeat.clone();
        let mut moved_9: Vec<Vec<i32>> = rows_9b.drain(10..14).collect();
        moved_9[1][1] = 8_888;
        rows_9b.splice(14..14, moved_9);
        let refs_9b: Vec<&[i32]> = rows_9b.iter().map(|r| r.as_slice()).collect();
        let grid_9b = grid_from_rows(&refs_9b);

        assert!(
            detect_fuzzy_row_block_move(&grid_9a, &grid_9b, &DiffConfig::default()).is_none(),
            "repetition guard should trigger when repeat count exceeds max_hash_repeat"
        );

        let mut base_8_repeat: Vec<Vec<i32>> = (1..=18)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        for row in base_8_repeat.iter_mut().take(8) {
            *row = vec![999, 888, 777];
        }
        let refs_8a: Vec<&[i32]> = base_8_repeat.iter().map(|r| r.as_slice()).collect();
        let grid_8a = grid_from_rows(&refs_8a);

        let mut rows_8b = base_8_repeat.clone();
        let mut moved_8: Vec<Vec<i32>> = rows_8b.drain(9..13).collect();
        moved_8[1][1] = 8_888;
        rows_8b.splice(14..14, moved_8);
        let refs_8b: Vec<&[i32]> = rows_8b.iter().map(|r| r.as_slice()).collect();
        let grid_8b = grid_from_rows(&refs_8b);

        assert!(
            detect_fuzzy_row_block_move(&grid_8a, &grid_8b, &DiffConfig::default()).is_some(),
            "repeat count equal to max_hash_repeat should not trigger heavy repetition guard"
        );
    }

    #[test]
    fn aligns_contiguous_block_insert_middle() {
        let base: Vec<Vec<i32>> = (1..=10)
            .map(|r| (1..=4).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let inserted_block: Vec<Vec<i32>> = (0..4)
            .map(|idx| vec![1_000 + idx, 2_000 + idx, 3_000 + idx, 4_000 + idx])
            .collect();
        let mut rows_b = base.clone();
        rows_b.splice(3..3, inserted_block);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let alignment = align_row_changes(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![3, 4, 5, 6]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 10);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[2], (2, 2));
        assert_eq!(alignment.matched[3], (3, 7));
        assert_eq!(alignment.matched.last(), Some(&(9, 13)));
        assert!(is_monotonic(&alignment.matched));
    }

    #[test]
    fn aligns_contiguous_block_delete_middle() {
        let base: Vec<Vec<i32>> = (1..=10)
            .map(|r| (1..=4).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        rows_b.drain(3..7);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        let alignment = align_row_changes(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.deleted, vec![3, 4, 5, 6]);
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.matched.len(), 6);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[2], (2, 2));
        assert_eq!(alignment.matched[3], (7, 3));
        assert_eq!(alignment.matched.last(), Some(&(9, 5)));
        assert!(is_monotonic(&alignment.matched));
    }

    #[test]
    fn block_alignment_bails_on_noncontiguous_changes() {
        let base: Vec<Vec<i32>> = (1..=8)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base.clone();
        rows_b.insert(1, vec![999, 1_000, 1_001]);
        rows_b.insert(5, vec![2_000, 2_001, 2_002]);
        let rows_b_refs: Vec<&[i32]> = rows_b.iter().map(|row| row.as_slice()).collect();
        let grid_b = grid_from_rows(&rows_b_refs);

        assert!(align_row_changes(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn align_row_changes_rejects_column_insert_mismatch() {
        let grid_a = grid_from_rows(&[&[10, 11, 12], &[20, 21, 22]]);
        let grid_b = grid_from_rows(&[&[0, 10, 11, 12], &[0, 20, 21, 22], &[0, 30, 31, 32]]);

        assert!(
            align_row_changes(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "column insertion changing column count should skip row alignment"
        );
    }

    #[test]
    fn align_row_changes_rejects_column_delete_mismatch() {
        let grid_a = grid_from_rows(&[&[10, 11, 12, 13], &[20, 21, 22, 23], &[30, 31, 32, 33]]);
        let grid_b = grid_from_rows(&[&[10, 12, 13], &[30, 32, 33]]);

        assert!(
            align_row_changes(&grid_a, &grid_b, &DiffConfig::default()).is_none(),
            "column deletion changing column count should skip row alignment"
        );
    }

    #[test]
    fn aligns_single_insert_with_unique_row() {
        let base = (1..=10)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let base_refs: Vec<&[i32]> = base.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let mut rows_b = base_refs.clone();
        rows_b.insert(
            5,
            &[999, 1000, 1001], // inserted at position 6 (1-based)
        );
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 10);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
        assert_eq!(alignment.matched[5], (5, 6));
        assert_eq!(alignment.matched.last(), Some(&(9, 10)));
    }

    #[test]
    fn rejects_non_monotonic_alignment_with_extra_mismatch() {
        let base_rows = [[11, 12, 13], [21, 22, 23], [31, 32, 33], [41, 42, 43]];
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = vec![
            base_refs[0],       // same
            &[999, 1000, 1001], // inserted unique row
            base_refs[2],       // move row 3 before row 2 to break monotonicity
            base_refs[1],
            base_refs[3],
        ];
        let grid_b = grid_from_rows(&rows_b);

        assert!(align_single_row_change(&grid_a, &grid_b, &DiffConfig::default()).is_none());
    }

    #[test]
    fn aligns_insert_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_first_row = [999, 1000, 1001];
        let mut rows_b = vec![new_first_row.as_slice()];
        rows_b.extend(base_refs.iter().copied());
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![0]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 1));
        assert_eq!(alignment.matched[4], (4, 5));
    }

    #[test]
    fn aligns_insert_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let new_last_row = [999, 1000, 1001];
        let mut rows_b: Vec<&[i32]> = base_refs.clone();
        rows_b.push(new_last_row.as_slice());
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![5]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 5);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[4], (4, 4));
    }

    #[test]
    fn aligns_delete_at_row_zero() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[1..].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![0]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (1, 0));
        assert_eq!(alignment.matched[3], (4, 3));
    }

    #[test]
    fn aligns_delete_at_last_row() {
        let base_rows: Vec<Vec<i32>> = (1..=5)
            .map(|r| (1..=3).map(|c| r * 10 + c).collect())
            .collect();
        let base_refs: Vec<&[i32]> = base_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&base_refs);

        let rows_b: Vec<&[i32]> = base_refs[..4].to_vec();
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![4]);
        assert_eq!(alignment.matched.len(), 4);
        assert_eq!(alignment.matched[0], (0, 0));
        assert_eq!(alignment.matched[3], (3, 3));
    }

    #[test]
    fn aligns_single_row_to_two_rows_via_insert() {
        let single_row = [[42, 43, 44]];
        let single_refs: Vec<&[i32]> = single_row.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&single_refs);

        let new_row = [999, 1000, 1001];
        let rows_b: Vec<&[i32]> = vec![single_refs[0], new_row.as_slice()];
        let grid_b = grid_from_rows(&rows_b);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert_eq!(alignment.inserted, vec![1]);
        assert!(alignment.deleted.is_empty());
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn aligns_two_rows_to_single_row_via_delete() {
        let two_rows = [[42, 43, 44], [99, 100, 101]];
        let two_refs: Vec<&[i32]> = two_rows.iter().map(|row| row.as_slice()).collect();
        let grid_a = grid_from_rows(&two_refs);

        let single_refs: Vec<&[i32]> = vec![two_refs[0]];
        let grid_b = grid_from_rows(&single_refs);

        let alignment = align_single_row_change(&grid_a, &grid_b, &DiffConfig::default())
            .expect("alignment should succeed");
        assert!(alignment.inserted.is_empty());
        assert_eq!(alignment.deleted, vec![1]);
        assert_eq!(alignment.matched.len(), 1);
        assert_eq!(alignment.matched[0], (0, 0));
    }

    #[test]
    fn monotonicity_helper_accepts_valid_sequence() {
        let valid: Vec<(u32, u32)> = vec![(0, 0), (1, 2), (3, 4), (5, 7)];
        assert!(super::is_monotonic(&valid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_a() {
        let invalid: Vec<(u32, u32)> = vec![(0, 0), (2, 2), (1, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_rejects_non_increasing_b() {
        let invalid: Vec<(u32, u32)> = vec![(0, 3), (1, 2), (2, 4)];
        assert!(!super::is_monotonic(&invalid));
    }

    #[test]
    fn monotonicity_helper_accepts_empty_and_single() {
        assert!(super::is_monotonic(&[]));
        assert!(super::is_monotonic(&[(5, 10)]));
    }
}
