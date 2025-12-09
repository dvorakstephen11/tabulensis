use std::ops::Range;

use crate::alignment::anchor_chain::build_anchor_chain;
use crate::alignment::anchor_discovery::{Anchor, discover_anchors_from_meta};
use crate::alignment::gap_strategy::{GapStrategy, select_gap_strategy};
use crate::alignment::move_extraction::{find_block_move, moves_from_matched_pairs};
use crate::alignment::row_metadata::RowMeta;
use crate::alignment::runs::compress_to_runs;
use crate::config::{DiffConfig, LimitBehavior};
use crate::grid_view::GridView;
use crate::workbook::Grid;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowAlignment {
    pub matched: Vec<(u32, u32)>,
    pub inserted: Vec<u32>,
    pub deleted: Vec<u32>,
    pub moves: Vec<RowBlockMove>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBlockMove {
    pub src_start_row: u32,
    pub dst_start_row: u32,
    pub row_count: u32,
}

#[derive(Default)]
struct GapAlignmentResult {
    matched: Vec<(u32, u32)>,
    inserted: Vec<u32>,
    deleted: Vec<u32>,
    moves: Vec<RowBlockMove>,
}

pub fn align_rows_amr(old: &Grid, new: &Grid, config: &DiffConfig) -> Option<RowAlignment> {
    if old.nrows.max(new.nrows) > config.max_align_rows
        || old.ncols.max(new.ncols) > config.max_align_cols
    {
        return match config.on_limit_exceeded {
            LimitBehavior::FallbackToPositional => None,
            LimitBehavior::ReturnPartialResult => Some(RowAlignment::default()),
            LimitBehavior::ReturnError => panic!(
                "alignment limits exceeded (rows={}, cols={})",
                old.nrows.max(new.nrows),
                old.ncols.max(new.ncols)
            ),
        };
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    let runs_a = compress_to_runs(&view_a.row_meta);
    let runs_b = compress_to_runs(&view_b.row_meta);
    if runs_a.len() == 1 && runs_b.len() == 1 && runs_a[0].signature == runs_b[0].signature {
        let shared = runs_a[0].count.min(runs_b[0].count);
        let mut matched = Vec::new();
        for offset in 0..shared {
            matched.push((runs_a[0].start_row + offset, runs_b[0].start_row + offset));
        }
        let mut inserted = Vec::new();
        if runs_b[0].count > shared {
            inserted.extend(
                (runs_b[0].start_row + shared)..(runs_b[0].start_row + runs_b[0].count),
            );
        }
        let mut deleted = Vec::new();
        if runs_a[0].count > shared {
            deleted.extend(
                (runs_a[0].start_row + shared)..(runs_a[0].start_row + runs_a[0].count),
            );
        }
        return Some(RowAlignment {
            matched,
            inserted,
            deleted,
            moves: Vec::new(),
        });
    }

    let anchors = build_anchor_chain(discover_anchors_from_meta(&view_a.row_meta, &view_b.row_meta));
    Some(assemble_from_meta(
        &view_a.row_meta,
        &view_b.row_meta,
        anchors,
        config,
        0,
    ))
}

fn assemble_from_meta(
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    anchors: Vec<Anchor>,
    config: &DiffConfig,
    depth: u32,
) -> RowAlignment {
    if old_meta.is_empty() && new_meta.is_empty() {
        return RowAlignment::default();
    }

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();
    let mut moves = Vec::new();

    let mut prev_old = old_meta.first().map(|m| m.row_idx).unwrap_or(0);
    let mut prev_new = new_meta.first().map(|m| m.row_idx).unwrap_or(0);

    for anchor in anchors.iter() {
        let gap_old = prev_old..anchor.old_row;
        let gap_new = prev_new..anchor.new_row;
        let gap_result = fill_gap(gap_old, gap_new, old_meta, new_meta, config, depth);
        matched.extend(gap_result.matched);
        inserted.extend(gap_result.inserted);
        deleted.extend(gap_result.deleted);
        moves.extend(gap_result.moves);

        matched.push((anchor.old_row, anchor.new_row));
        prev_old = anchor.old_row + 1;
        prev_new = anchor.new_row + 1;
    }

    let old_end = old_meta.last().map(|m| m.row_idx + 1).unwrap_or(prev_old);
    let new_end = new_meta.last().map(|m| m.row_idx + 1).unwrap_or(prev_new);
    let tail_result = fill_gap(prev_old..old_end, prev_new..new_end, old_meta, new_meta, config, depth);
    matched.extend(tail_result.matched);
    inserted.extend(tail_result.inserted);
    deleted.extend(tail_result.deleted);
    moves.extend(tail_result.moves);

    matched.sort_by_key(|(a, b)| (*a, *b));
    inserted.sort_unstable();
    deleted.sort_unstable();
    moves.sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));

    RowAlignment {
        matched,
        inserted,
        deleted,
        moves,
    }
}

fn fill_gap(
    old_gap: Range<u32>,
    new_gap: Range<u32>,
    old_meta: &[RowMeta],
    new_meta: &[RowMeta],
    config: &DiffConfig,
    depth: u32,
) -> GapAlignmentResult {
    let old_slice = slice_by_range(old_meta, &old_gap);
    let new_slice = slice_by_range(new_meta, &new_gap);
    let has_recursed = depth >= config.max_recursion_depth;
    let strategy = select_gap_strategy(old_slice, new_slice, config, has_recursed);

    match strategy {
        GapStrategy::Empty => GapAlignmentResult::default(),
        GapStrategy::InsertAll => GapAlignmentResult {
            matched: Vec::new(),
            inserted: (new_gap.start..new_gap.end).collect(),
            deleted: Vec::new(),
            moves: Vec::new(),
        },
        GapStrategy::DeleteAll => GapAlignmentResult {
            matched: Vec::new(),
            inserted: Vec::new(),
            deleted: (old_gap.start..old_gap.end).collect(),
            moves: Vec::new(),
        },
        GapStrategy::SmallEdit => align_small_gap(old_slice, new_slice),
        GapStrategy::MoveCandidate => {
            let mut result = align_small_gap(old_slice, new_slice);
            let mut detected_moves = moves_from_matched_pairs(&result.matched);
            if detected_moves.is_empty() {
                if let Some(mv) = find_block_move(old_slice, new_slice, 1) {
                    detected_moves.push(mv);
                }
            }
            result.moves.extend(detected_moves);
            result
        }
        GapStrategy::RecursiveAlign => {
            if depth >= config.max_recursion_depth {
                return align_small_gap(old_slice, new_slice);
            }
            let anchors = build_anchor_chain(discover_anchors_from_meta(old_slice, new_slice));
            let alignment = assemble_from_meta(old_slice, new_slice, anchors, config, depth + 1);
            GapAlignmentResult {
                matched: alignment.matched,
                inserted: alignment.inserted,
                deleted: alignment.deleted,
                moves: alignment.moves,
            }
        }
    }
}

fn slice_by_range<'a>(meta: &'a [RowMeta], range: &Range<u32>) -> &'a [RowMeta] {
    if meta.is_empty() || range.start >= range.end {
        return &[];
    }
    let base = meta.first().map(|m| m.row_idx).unwrap_or(0);
    if range.start < base {
        return &[];
    }
    let start = (range.start - base) as usize;
    if start >= meta.len() {
        return &[];
    }
    let end = (start + (range.end - range.start) as usize).min(meta.len());
    &meta[start..end]
}

fn align_small_gap(old_slice: &[RowMeta], new_slice: &[RowMeta]) -> GapAlignmentResult {
    let m = old_slice.len();
    let n = new_slice.len();
    if m == 0 && n == 0 {
        return GapAlignmentResult::default();
    }

    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if old_slice[i].signature == new_slice[j].signature {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut matched = Vec::new();
    let mut inserted = Vec::new();
    let mut deleted = Vec::new();

    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if old_slice[i].signature == new_slice[j].signature {
            matched.push((old_slice[i].row_idx, new_slice[j].row_idx));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            deleted.push(old_slice[i].row_idx);
            i += 1;
        } else {
            inserted.push(new_slice[j].row_idx);
            j += 1;
        }
    }

    while i < m {
        deleted.push(old_slice[i].row_idx);
        i += 1;
    }
    while j < n {
        inserted.push(new_slice[j].row_idx);
        j += 1;
    }

    if matched.is_empty() && m == n {
        matched = old_slice
            .iter()
            .zip(new_slice.iter())
            .map(|(a, b)| (a.row_idx, b.row_idx))
            .collect();
        inserted.clear();
        deleted.clear();
    }

    GapAlignmentResult {
        matched,
        inserted,
        deleted,
        moves: Vec::new(),
    }
}
