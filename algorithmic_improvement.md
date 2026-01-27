Your slowest outlier is dominated by *rectangular block move detection*, not by the “normal” alignment/diff work.

In the latest metrics, `perf_50k_alignment_block_move` spends **~6.44s** in `move_detection_time_ms` out of **~6.95s** total, with an estimated **5,000,000 hash lookups**.  That profile matches an `O(R*C)` scan over the entire grid.

In `core/src/rect_block_move.rs`, the current `detect_exact_rect_block_move` literally constructs `diff_positions` by scanning every cell and pushing every mismatch, i.e. **time Θ(R*C)** and **memory Θ(#diff)**: `collect_differences` loops `for row in 0..old.nrows { for col in 0..old.ncols { ... diffs.push(...) } }`.  In a “row block move” workload, the number of mismatching cells is huge, so you pay the worst-case on both CPU and allocations before you even decide it isn’t a rect move.

### Algorithmic improvement (complexity drop)

You already build `GridView`s inside this function and therefore already have per-row and per-column hashes (`row_meta.signature`, `col_meta.hash`). Use those hashes to get candidate moved ranges **without scanning all cells**:

1. Compute `diff_rows = { i | row_hash_old[i] != row_hash_new[i] }` in `O(R)`.
2. Compute `diff_cols = { j | col_hash_old[j] != col_hash_new[j] }` in `O(C)`.
3. If those indices don’t form *two equal-length contiguous ranges* in rows and cols, bail immediately (cheap rejection).
4. Only if they do, verify by counting mismatches in the 2 (or 4) candidate rectangles and then running the existing `validate_orientation` (which verifies content correspondence). This costs `O(area_of_candidate_rectangles)`, not `O(R*C)`.

So the detector becomes roughly **O(R + C + K)** where `K` is the area you must actually check to confirm the move. For the pathological “row block move” case, you reject quickly without ever building `diff_positions`, which is exactly what the benchmark is punishing today. 

---

## Code change

Replace this function in `core/src/rect_block_move.rs`:

```rust
pub(crate) fn detect_exact_rect_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RectBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 || old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if view_a.is_low_info_dominated() || view_b.is_low_info_dominated() {
        return None;
    }

    if view_a.is_blank_dominated() || view_b.is_blank_dominated() {
        return None;
    }

    let row_stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    let col_stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);

    if row_stats.has_heavy_repetition(config.alignment.max_hash_repeat)
        || col_stats.has_heavy_repetition(config.alignment.max_hash_repeat)
    {
        return None;
    }

    let shared_rows = row_stats
        .freq_a
        .keys()
        .filter(|h| row_stats.freq_b.contains_key(*h))
        .count();
    let shared_cols = col_stats
        .freq_a
        .keys()
        .filter(|h| col_stats.freq_b.contains_key(*h))
        .count();
    if shared_rows == 0 && shared_cols == 0 {
        return None;
    }

    let diff_positions = collect_differences(old, new);
    if diff_positions.is_empty() {
        return None;
    }

    let row_ranges = find_two_equal_ranges(diff_positions.iter().map(|(r, _)| *r))?;
    let col_ranges = find_two_equal_ranges(diff_positions.iter().map(|(_, c)| *c))?;

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    let expected_mismatches = row_count.checked_mul(col_count)?.checked_mul(2)?;
    if diff_positions.len() as u32 != expected_mismatches {
        return None;
    }

    let mismatches = count_rect_mismatches(old, new, row_ranges.0, col_ranges.0)
        + count_rect_mismatches(old, new, row_ranges.1, col_ranges.1);
    if mismatches != diff_positions.len() as u32 {
        return None;
    }

    if !has_unique_meta(
        &view_a, &view_b, &row_stats, &col_stats, row_ranges, col_ranges,
    ) {
        return None;
    }

    let primary = validate_orientation(old, new, row_ranges, col_ranges);
    let swapped_ranges = ((row_ranges.1, row_ranges.0), (col_ranges.1, col_ranges.0));
    let alternate = validate_orientation(old, new, swapped_ranges.0, swapped_ranges.1);

    match (primary, alternate) {
        (Some(mv), None) => Some(mv),
        (None, Some(mv)) => Some(mv),
        _ => None,
    }
}
```

With this:

```rust
pub(crate) fn detect_exact_rect_block_move(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> Option<RectBlockMove> {
    if old.nrows != new.nrows || old.ncols != new.ncols {
        return None;
    }

    if old.nrows == 0 || old.ncols == 0 {
        return None;
    }

    if !is_within_size_bounds(old, new, config) {
        return None;
    }

    let view_a = GridView::from_grid_with_config(old, config);
    let view_b = GridView::from_grid_with_config(new, config);

    if view_a.is_low_info_dominated() || view_b.is_low_info_dominated() {
        return None;
    }

    if view_a.is_blank_dominated() || view_b.is_blank_dominated() {
        return None;
    }

    let row_stats = HashStats::from_row_meta(&view_a.row_meta, &view_b.row_meta);
    let col_stats = HashStats::from_col_meta(&view_a.col_meta, &view_b.col_meta);

    if row_stats.has_heavy_repetition(config.alignment.max_hash_repeat)
        || col_stats.has_heavy_repetition(config.alignment.max_hash_repeat)
    {
        return None;
    }

    let shared_rows = row_stats
        .freq_a
        .keys()
        .filter(|h| row_stats.freq_b.contains_key(*h))
        .count();
    let shared_cols = col_stats
        .freq_a
        .keys()
        .filter(|h| col_stats.freq_b.contains_key(*h))
        .count();
    if shared_rows == 0 && shared_cols == 0 {
        return None;
    }

    let mut diff_rows: Vec<u32> = Vec::new();
    for (idx, (a, b)) in view_a.row_meta.iter().zip(view_b.row_meta.iter()).enumerate() {
        if a.signature != b.signature {
            diff_rows.push(idx as u32);
        }
    }

    let mut diff_cols: Vec<u32> = Vec::new();
    for (idx, (a, b)) in view_a.col_meta.iter().zip(view_b.col_meta.iter()).enumerate() {
        if a.hash != b.hash {
            diff_cols.push(idx as u32);
        }
    }

    if diff_rows.is_empty() || diff_cols.is_empty() {
        return None;
    }

    let row_ranges = find_two_equal_ranges(diff_rows)?;
    let col_ranges = find_two_equal_ranges(diff_cols)?;

    if row_ranges.0 == row_ranges.1 && col_ranges.0 == col_ranges.1 {
        return None;
    }

    let row_count = range_len(row_ranges.0);
    let col_count = range_len(col_ranges.0);

    let expected_mismatches = row_count.checked_mul(col_count)?.checked_mul(2)?;

    let m00 = count_rect_mismatches(old, new, row_ranges.0, col_ranges.0);
    let m11 = count_rect_mismatches(old, new, row_ranges.1, col_ranges.1);
    let diag_mismatches = m00.saturating_add(m11);

    if diag_mismatches != expected_mismatches {
        return None;
    }

    let union_mismatches = if row_ranges.0 == row_ranges.1 || col_ranges.0 == col_ranges.1 {
        diag_mismatches
    } else {
        let m01 = count_rect_mismatches(old, new, row_ranges.0, col_ranges.1);
        let m10 = count_rect_mismatches(old, new, row_ranges.1, col_ranges.0);
        diag_mismatches
            .saturating_add(m01)
            .saturating_add(m10)
    };

    if union_mismatches != expected_mismatches {
        return None;
    }

    if !has_unique_meta(
        &view_a, &view_b, &row_stats, &col_stats, row_ranges, col_ranges,
    ) {
        return None;
    }

    let primary = validate_orientation(old, new, row_ranges, col_ranges);
    let swapped_ranges = ((row_ranges.1, row_ranges.0), (col_ranges.1, col_ranges.0));
    let alternate = validate_orientation(old, new, swapped_ranges.0, swapped_ranges.1);

    match (primary, alternate) {
        (Some(mv), None) => Some(mv),
        (None, Some(mv)) => Some(mv),
        _ => None,
    }
}
```

### Why this should move the needle

This removes the “scan every cell and push every diff coordinate” step that drives the pathological `move_detection_time_ms` in the block-move benchmark.  The detector will now reject non-rect cases after `O(R+C)` hash comparisons, and only do cell-by-cell work over the small candidate rectangles in the rare cases where a rect move is plausible.

If you want an additional follow-up improvement, the next big win would be to stop rebuilding `GridView` inside each move detector (rect/row/col) during the move loop and instead reuse `SheetGridDiffer.old_view/new_view` (those are already built once). 
