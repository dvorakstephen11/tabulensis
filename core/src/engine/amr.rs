use crate::alignment::align_rows_amr_with_signatures_from_views;
use crate::alignment::move_extraction::moves_from_matched_pairs;
use crate::alignment_types::RowAlignment;
use crate::column_alignment::align_single_column_change_from_views;
use crate::config::DiffConfig;
use crate::diff::DiffError;
use crate::grid_view::GridView;
#[cfg(feature = "perf-metrics")]
use crate::perf::{DiffMetrics, Phase};
use crate::sink::DiffSink;
use crate::workbook::{Grid, RowSignature};

use std::collections::{HashMap, HashSet};

use super::context::EmitCtx;
use super::grid_primitives::{
    emit_column_aligned_diffs, emit_row_aligned_diffs, run_positional_diff_with_metrics,
};
use super::move_mask::row_signature_at;

pub(crate) fn row_signature_multiset_equal(a: &Grid, b: &Grid) -> bool {
    if a.nrows != b.nrows {
        return false;
    }

    let mut a_sigs: Vec<RowSignature> = (0..a.nrows)
        .filter_map(|row| row_signature_at(a, row))
        .collect();
    let mut b_sigs: Vec<RowSignature> = (0..b.nrows)
        .filter_map(|row| row_signature_at(b, row))
        .collect();

    a_sigs.sort_unstable_by_key(|s| s.hash);
    b_sigs.sort_unstable_by_key(|s| s.hash);

    a_sigs == b_sigs
}

fn amr_strip_moves_policy(old: &Grid, new: &Grid, alignment: &mut RowAlignment) {
    let mut deleted_from_moves = Vec::new();
    let mut inserted_from_moves = Vec::new();
    for mv in &alignment.moves {
        deleted_from_moves.extend(mv.src_start_row..mv.src_start_row.saturating_add(mv.row_count));
        inserted_from_moves.extend(mv.dst_start_row..mv.dst_start_row.saturating_add(mv.row_count));
    }

    let multiset_equal = row_signature_multiset_equal(old, new);
    if multiset_equal {
        for (a, b) in &alignment.matched {
            if row_signature_at(old, *a) != row_signature_at(new, *b) {
                deleted_from_moves.push(*a);
                inserted_from_moves.push(*b);
            }
        }
    }

    if !deleted_from_moves.is_empty() || !inserted_from_moves.is_empty() {
        let deleted_set: HashSet<u32> = deleted_from_moves.iter().copied().collect();
        let inserted_set: HashSet<u32> = inserted_from_moves.iter().copied().collect();

        alignment
            .matched
            .retain(|(a, b)| !deleted_set.contains(a) && !inserted_set.contains(b));

        alignment.deleted.extend(deleted_set);
        alignment.inserted.extend(inserted_set);
        alignment.deleted.sort_unstable();
        alignment.deleted.dedup();
        alignment.inserted.sort_unstable();
        alignment.inserted.dedup();
    }

    alignment.moves.clear();
}

fn amr_should_fallback_no_matched_rows(alignment: &RowAlignment) -> bool {
    let has_structural = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();
    has_structural && alignment.matched.is_empty()
}

fn amr_should_fallback_row_edits_with_structural(
    old: &Grid,
    new: &Grid,
    alignment: &RowAlignment,
    config: &DiffConfig,
) -> bool {
    let has_structural = !alignment.inserted.is_empty() || !alignment.deleted.is_empty();
    if !has_structural {
        return false;
    }

    let has_row_edits = alignment
        .matched
        .iter()
        .any(|(a, b)| row_signature_at(old, *a) != row_signature_at(new, *b));

    has_row_edits && config.max_move_iterations > 0
}

fn amr_alignment_is_trivial_identity(old: &Grid, new: &Grid, alignment: &RowAlignment) -> bool {
    alignment.moves.is_empty()
        && alignment.inserted.is_empty()
        && alignment.deleted.is_empty()
        && old.nrows == new.nrows
        && alignment.matched.len() as u32 == old.nrows
        && alignment.matched.iter().all(|(a, b)| a == b)
}

fn amr_should_fallback_multiset_reorder(
    old: &Grid,
    new: &Grid,
    alignment: &RowAlignment,
    config: &DiffConfig,
) -> bool {
    let is_trivial = amr_alignment_is_trivial_identity(old, new, alignment);
    !is_trivial
        && alignment.moves.is_empty()
        && row_signature_multiset_equal(old, new)
        && config.max_move_iterations > 0
}

fn amr_can_try_column_alignment(old: &Grid, new: &Grid, alignment: &RowAlignment) -> bool {
    alignment.moves.is_empty()
        && alignment.inserted.is_empty()
        && alignment.deleted.is_empty()
        && old.ncols != new.ncols
}

fn inject_moves_from_insert_delete(
    old: &Grid,
    new: &Grid,
    alignment: &mut RowAlignment,
    row_signatures_old: &[RowSignature],
    row_signatures_new: &[RowSignature],
) {
    if alignment.inserted.is_empty() || alignment.deleted.is_empty() {
        return;
    }

    let mut deleted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.deleted {
        let sig = row_signatures_old
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(old, *row));
        if let Some(sig) = sig {
            deleted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut inserted_by_sig: HashMap<RowSignature, Vec<u32>> = HashMap::new();
    for row in &alignment.inserted {
        let sig = row_signatures_new
            .get(*row as usize)
            .copied()
            .or_else(|| row_signature_at(new, *row));
        if let Some(sig) = sig {
            inserted_by_sig.entry(sig).or_default().push(*row);
        }
    }

    let mut matched_pairs = Vec::new();
    for (sig, deleted_rows) in deleted_by_sig.iter() {
        if deleted_rows.len() != 1 {
            continue;
        }
        if let Some(insert_rows) = inserted_by_sig.get(sig) {
            if insert_rows.len() != 1 {
                continue;
            }
            matched_pairs.push((deleted_rows[0], insert_rows[0]));
        }
    }

    if matched_pairs.is_empty() {
        return;
    }

    let new_moves = moves_from_matched_pairs(&matched_pairs);
    if new_moves.is_empty() {
        return;
    }

    let mut moved_src = HashSet::new();
    let mut moved_dst = HashSet::new();
    for mv in &new_moves {
        for r in mv.src_start_row..mv.src_start_row.saturating_add(mv.row_count) {
            moved_src.insert(r);
        }
        for r in mv.dst_start_row..mv.dst_start_row.saturating_add(mv.row_count) {
            moved_dst.insert(r);
        }
    }

    alignment.deleted.retain(|r| !moved_src.contains(r));
    alignment.inserted.retain(|r| !moved_dst.contains(r));
    alignment.moves.extend(new_moves);
    alignment
        .moves
        .sort_by_key(|m| (m.src_start_row, m.dst_start_row, m.row_count));
}

pub(super) fn try_diff_with_amr<S: DiffSink>(
    emit_ctx: &mut EmitCtx<'_, S>,
    old: &Grid,
    new: &Grid,
    old_view: &GridView,
    new_view: &GridView,
    #[cfg(feature = "perf-metrics")] mut metrics: Option<&mut DiffMetrics>,
) -> Result<bool, DiffError> {
    let Some(amr_result) =
        align_rows_amr_with_signatures_from_views(old_view, new_view, emit_ctx.config)
    else {
        return Ok(false);
    };

    let mut alignment = amr_result.alignment;

    if emit_ctx.config.max_move_iterations > 0 {
        inject_moves_from_insert_delete(
            old,
            new,
            &mut alignment,
            &amr_result.row_signatures_a,
            &amr_result.row_signatures_b,
        );
    } else {
        amr_strip_moves_policy(old, new, &mut alignment);
    }

    if amr_should_fallback_no_matched_rows(&alignment) {
        #[cfg(feature = "perf-metrics")]
        run_positional_diff_with_metrics(emit_ctx, old, new, metrics.as_deref_mut())?;
        #[cfg(not(feature = "perf-metrics"))]
        run_positional_diff_with_metrics(emit_ctx, old, new)?;
        return Ok(true);
    }

    if amr_should_fallback_row_edits_with_structural(old, new, &alignment, emit_ctx.config) {
        #[cfg(feature = "perf-metrics")]
        run_positional_diff_with_metrics(emit_ctx, old, new, metrics.as_deref_mut())?;
        #[cfg(not(feature = "perf-metrics"))]
        run_positional_diff_with_metrics(emit_ctx, old, new)?;
        return Ok(true);
    }

    if amr_can_try_column_alignment(old, new, &alignment)
        && let Some(col_alignment) =
            align_single_column_change_from_views(old_view, new_view, emit_ctx.config)
    {
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            m.start_phase(Phase::CellDiff);
        }
        emit_column_aligned_diffs(emit_ctx, old, new, &col_alignment)?;
        #[cfg(feature = "perf-metrics")]
        if let Some(m) = metrics.as_mut() {
            let overlap_rows = old.nrows.min(new.nrows) as u64;
            m.add_cells_compared(overlap_rows.saturating_mul(col_alignment.matched.len() as u64));
            m.end_phase(Phase::CellDiff);
        }
        return Ok(true);
    }

    if amr_should_fallback_multiset_reorder(old, new, &alignment, emit_ctx.config) {
        #[cfg(feature = "perf-metrics")]
        run_positional_diff_with_metrics(emit_ctx, old, new, metrics.as_deref_mut())?;
        #[cfg(not(feature = "perf-metrics"))]
        run_positional_diff_with_metrics(emit_ctx, old, new)?;
        return Ok(true);
    }

    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.start_phase(Phase::CellDiff);
    }

    let compared = emit_row_aligned_diffs(emit_ctx, old_view, new_view, &alignment)?;
    #[cfg(feature = "perf-metrics")]
    if let Some(m) = metrics.as_mut() {
        m.add_cells_compared(compared);
        m.anchors_found = m
            .anchors_found
            .saturating_add(alignment.matched.len() as u32);
        m.moves_detected = m
            .moves_detected
            .saturating_add(alignment.moves.len() as u32);
        m.end_phase(Phase::CellDiff);
    }
    #[cfg(not(feature = "perf-metrics"))]
    let _ = compared;

    Ok(true)
}
