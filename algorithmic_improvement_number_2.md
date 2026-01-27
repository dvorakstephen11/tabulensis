### Follow-up improvement: stop paying an O(R*C) full-grid scan when the change is a row/column move

Your full-scale benchmark is basically screaming that **rectangular move detection is running (and doing a full grid scan) even when the sheet change is a pure row-block move**.

In `perf_50k_alignment_block_move`, the run spends **6,442ms in move detection out of 6,945ms total**, and the estimator reports **~5,000,000 hash lookups**.  
That `5,000,000` is exactly what you’d expect from scanning a 50k x 50 grid and doing `old.get` + `new.get` per cell (2 * 2.5M = 5M).

#### Why this happens in the current code

`SheetGridDiffer::detect_moves` currently attempts **rect block move detection first**, before it tries exact row or exact column block moves. 

But `detect_exact_rect_block_move_masked` eventually relies on `collect_differences_in_grid`, which is a **nested `for r ... for c ...` loop over the entire grid shape** (not over non-blank cells). 
So for large sheets, you’re effectively doing the most expensive detector first.

Mathematically, this is a decision-tree ordering problem:

* Rect move detection cost is ~O(R*C) `get()` operations (and you see it in metrics).
* Exact row/column move detection is far cheaper in the common “moved block” cases (it’s primarily signature/meta based, and doesn’t need to compare every cell position first).
* If you run the expensive test first, you pay that cost even when a cheaper test would have succeeded immediately.

### The improvement

**Reorder move detection attempts in `detect_moves` so exact row/column block moves run before rectangular moves.**
This preserves correctness (rect detection still runs if row/col doesn’t match), but avoids the full-grid scan in the common “row block moved” / “column block moved” cases.

---

## Code change

### Replace this code (current `detect_moves` ordering)

```rust
pub(super) fn detect_moves(&mut self) -> Result<u32, DiffError> {
    if !self.move_detection_enabled() {
        return Ok(0);
    }

    let mut iteration = 0u32;
    let config = self.emit_ctx.config;

    loop {
        if iteration >= config.moves.max_move_iterations {
            break;
        }

        if !self.old_mask.has_active_cells() || !self.new_mask.has_active_cells() {
            break;
        }

        let mut found_move = false;

        if let Some(mv) = detect_exact_rect_block_move_masked(
            &self.old,
            &self.new,
            &self.old_mask,
            &self.new_mask,
            config,
        ) {
            emit_rect_block_move(&mut self.emit_ctx, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask.exclude_rect_cells(
                mv.src_start_row,
                mv.src_row_count,
                mv.src_start_col,
                mv.src_col_count,
            );
            self.old_mask.exclude_rect_cells(
                mv.dst_start_row,
                mv.src_row_count,
                mv.dst_start_col,
                mv.src_col_count,
            );
            self.new_mask.exclude_rect_cells(
                mv.src_start_row,
                mv.src_row_count,
                mv.src_start_col,
                mv.src_col_count,
            );
            self.new_mask.exclude_rect_cells(
                mv.dst_start_row,
                mv.src_row_count,
                mv.dst_start_col,
                mv.src_col_count,
            );
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && let Some(mv) = detect_exact_row_block_move_masked(
                &self.old,
                &self.new,
                &self.old_mask,
                &self.new_mask,
                config,
            )
        {
            emit_row_block_move(&mut self.emit_ctx, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask
                .exclude_row_range(mv.src_start_row, mv.row_count);
            self.new_mask
                .exclude_row_range(mv.dst_start_row, mv.row_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && let Some(mv) = detect_exact_column_block_move_masked(
                &self.old,
                &self.new,
                &self.old_mask,
                &self.new_mask,
                config,
            )
        {
            emit_column_block_move(&mut self.emit_ctx, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask
                .exclude_col_range(mv.src_start_col, mv.col_count);
            self.new_mask
                .exclude_col_range(mv.dst_start_col, mv.col_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && config.moves.enable_fuzzy_moves
            && let Some(mv) = detect_fuzzy_row_block_move_masked(
                &self.old,
                &self.new,
                &self.old_mask,
                &self.new_mask,
                config,
            )
        {
            emit_row_block_move(&mut self.emit_ctx, mv)?;
            emit_moved_row_block_edits(&mut self.emit_ctx, &self.old, &self.new, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask
                .exclude_row_range(mv.src_start_row, mv.row_count);
            self.new_mask
                .exclude_row_range(mv.dst_start_row, mv.row_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move {
            break;
        }

        if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
            break;
        }
    }

    Ok(iteration)
}
```

### With this code (row/col first; rect later)

```rust
pub(super) fn detect_moves(&mut self) -> Result<u32, DiffError> {
    if !self.move_detection_enabled() {
        return Ok(0);
    }

    let mut iteration = 0u32;
    let config = self.emit_ctx.config;

    loop {
        if iteration >= config.moves.max_move_iterations {
            break;
        }

        if !self.old_mask.has_active_cells() || !self.new_mask.has_active_cells() {
            break;
        }

        let mut found_move = false;

        if let Some(mv) = detect_exact_row_block_move_masked(
            &self.old,
            &self.new,
            &self.old_mask,
            &self.new_mask,
            config,
        ) {
            emit_row_block_move(&mut self.emit_ctx, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask
                .exclude_row_range(mv.src_start_row, mv.row_count);
            self.new_mask
                .exclude_row_range(mv.dst_start_row, mv.row_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && let Some(mv) = detect_exact_column_block_move_masked(
                &self.old,
                &self.new,
                &self.old_mask,
                &self.new_mask,
                config,
            )
        {
            emit_column_block_move(&mut self.emit_ctx, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask
                .exclude_col_range(mv.src_start_col, mv.col_count);
            self.new_mask
                .exclude_col_range(mv.dst_start_col, mv.col_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && let Some(mv) = detect_exact_rect_block_move_masked(
                &self.old,
                &self.new,
                &self.old_mask,
                &self.new_mask,
                config,
            )
        {
            emit_rect_block_move(&mut self.emit_ctx, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask.exclude_rect_cells(
                mv.src_start_row,
                mv.src_row_count,
                mv.src_start_col,
                mv.src_col_count,
            );
            self.old_mask.exclude_rect_cells(
                mv.dst_start_row,
                mv.src_row_count,
                mv.dst_start_col,
                mv.src_col_count,
            );
            self.new_mask.exclude_rect_cells(
                mv.src_start_row,
                mv.src_row_count,
                mv.src_start_col,
                mv.src_col_count,
            );
            self.new_mask.exclude_rect_cells(
                mv.dst_start_row,
                mv.src_row_count,
                mv.dst_start_col,
                mv.src_col_count,
            );
            iteration += 1;
            found_move = true;
        }

        if !found_move
            && config.moves.enable_fuzzy_moves
            && let Some(mv) = detect_fuzzy_row_block_move_masked(
                &self.old,
                &self.new,
                &self.old_mask,
                &self.new_mask,
                config,
            )
        {
            emit_row_block_move(&mut self.emit_ctx, mv)?;
            emit_moved_row_block_edits(&mut self.emit_ctx, &self.old, &self.new, mv)?;
            #[cfg(feature = "perf-metrics")]
            if let Some(m) = self.emit_ctx.metrics.as_mut() {
                m.moves_detected = m.moves_detected.saturating_add(1);
            }
            self.old_mask
                .exclude_row_range(mv.src_start_row, mv.row_count);
            self.new_mask
                .exclude_row_range(mv.dst_start_row, mv.row_count);
            iteration += 1;
            found_move = true;
        }

        if !found_move {
            break;
        }

        if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
            break;
        }
    }

    Ok(iteration)
}
```

---

## What you should see in the benchmarks

On `perf_50k_alignment_block_move`, this should eliminate the **5,000,000 `get()`-style lookups** that come from running rect detection first, and therefore cut **multiple seconds** off `move_detection_time_ms` (since that phase is currently ~6.4s).  

If you want an additional “next” follow-up after this: the next obvious target is to avoid rebuilding any per-view hashes inside the row/col detectors when `old_mask/new_mask` have no exclusions (i.e., use the already-built `GridView` meta). But the ordering change above is the big, low-risk win that directly matches the 5,000,000-lookups signature in the perf logs.
