From the perf logs, the single biggest opportunity is **move detection on very large sheets**, especially the “50k row block move” case:

* `perf_50k_alignment_block_move` spends **~5.2s in move detection** out of **~5.7s total diff time** (so ~90%+ of the entire diff) on the current branch snapshot. 
* By contrast, the same run shows only **~431ms signature build** and **~58ms cell diff**. 

That strongly suggests: **even a modest algorithmic improvement in move detection will dwarf any other optimization** for these workloads.

## Why move detection is so expensive here

In `SheetGridDiffer::detect_moves`, once **any** move is found, the masks gain exclusions, and subsequent iterations switch to the “masked” move detectors. 

The masked detectors rebuild *projected* grids by iterating all cells and re-inserting active cells:

* `build_masked_grid` iterates `source.iter_cells()` and calls `projected.insert_cell(...)` for each active cell.
* Both `detect_exact_row_block_move_masked` and `detect_exact_column_block_move_masked` call `build_masked_grid` for **old and new**.

So after the first detected move, the algorithm can easily do **another full pass over millions of cells**, allocating and inserting into new grids, even when no further moves exist. That matches the perf signature: “moves_detected = 1” but move detection still costs seconds. 

## The key mathematical observation

For row- and column-block moves, you already have **128-bit row and column signatures** in `GridView`. Those signatures are designed to be safe to compare directly; the code even explicitly notes the collision probability is negligible at the 50k-row scale. 

That means:

* If (after excluding the moved region) the **remaining active rows match in-order by signature**, the remaining active grids are identical, so **no further move can exist** outside the excluded region.
* Same idea for **columns** when the exclusion is column-only.

Critically, this check is **O(R)** (rows) or **O(C)** (cols) instead of rebuilding masked grids which is **O(nnz)** (cells), and on dense sheets `nnz >> R, C`.

In the `perf_50k_alignment_block_move` case, this should eliminate the expensive “second iteration” entirely, because after removing the moved rows, the remaining rows are identical.

## Concrete algorithmic improvement

**Add an early-exit condition inside `detect_moves`:**

After emitting a row/col move and updating masks, perform a cheap signature-based equality check on the remaining active region:

* If the masks contain **only row exclusions** (no col exclusions, no rect exclusions), compare `row_meta[active_row].signature` across the two masks’ active-row iterators.
* If the masks contain **only col exclusions**, do the analogous check with `col_meta[active_col].hash`.
* If they match completely, `break` out of move detection immediately.

This prevents the code from falling into the masked-grid rebuild path when there is provably nothing left to detect.

### Patch

Replace this existing tail of the `detect_moves` loop in `core/src/engine/move_mask.rs`:

```rust
if !found_move {
                break;
            }

            if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
                break;
            }
        }

        Ok(iteration)
```

With this:

```rust
if !found_move {
                break;
            }

            if !self.old_mask.has_excluded_rects()
                && !self.new_mask.has_excluded_rects()
                && self.old_mask.has_excluded_rows()
                && self.new_mask.has_excluded_rows()
                && !self.old_mask.has_excluded_cols()
                && !self.new_mask.has_excluded_cols()
            {
                let mut it_old = self.old_mask.active_rows();
                let mut it_new = self.new_mask.active_rows();
                let mut identical = true;

                loop {
                    match (it_old.next(), it_new.next()) {
                        (None, None) => break,
                        (Some(ro), Some(rn)) => {
                            if self.old_view.row_meta[ro as usize].signature
                                != self.new_view.row_meta[rn as usize].signature
                            {
                                identical = false;
                                break;
                            }
                        }
                        _ => {
                            identical = false;
                            break;
                        }
                    }
                }

                if identical {
                    break;
                }
            } else if !self.old_mask.has_excluded_rects()
                && !self.new_mask.has_excluded_rects()
                && self.old_mask.has_excluded_cols()
                && self.new_mask.has_excluded_cols()
                && !self.old_mask.has_excluded_rows()
                && !self.new_mask.has_excluded_rows()
            {
                let mut it_old = self.old_mask.active_cols();
                let mut it_new = self.new_mask.active_cols();
                let mut identical = true;

                loop {
                    match (it_old.next(), it_new.next()) {
                        (None, None) => break,
                        (Some(co), Some(cn)) => {
                            if self.old_view.col_meta[co as usize].hash
                                != self.new_view.col_meta[cn as usize].hash
                            {
                                identical = false;
                                break;
                            }
                        }
                        _ => {
                            identical = false;
                            break;
                        }
                    }
                }

                if identical {
                    break;
                }
            }

            if self.old.nrows != self.new.nrows || self.old.ncols != self.new.ncols {
                break;
            }
        }

        Ok(iteration)
```

## Why this should materially improve your worst case

In the problematic benchmark:

* Today: `move_detection_time_ms ≈ 5202` and `diff_time_ms ≈ 5703`. 
* The expensive part is consistent with the masked-grid rebuild path (`build_masked_grid` iterating every cell and inserting into a new `Grid`).

After this change, the common “single giant move explains everything” case should:

1. Detect the move using the fast from-views detector (no rebuild).
2. Exclude rows/cols.
3. Immediately prove the remaining active region is identical by signatures (linear in rows or cols).
4. Stop move detection before any masked-grid reconstruction can happen.

Given your current breakdown (431ms signature build + 58ms cell diff + ~5202ms move detection), removing the multi-second masked pass should turn this from “seconds” into “sub-second” for that workload. 

