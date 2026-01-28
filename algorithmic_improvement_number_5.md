### What the metrics are really telling you

In your latest suite run, the diff engine is spending **most of its time in “SignatureBuild”**: **3693ms out of 4547ms total (~81%)**. 

Two specific tests make the problem painfully clear:

* **`perf_50k_dense_single_edit`**: total **1047ms**, but **870ms** is signature build, while only **700 cells** are actually compared in the diff stage. 
  That means you’re paying a near-full-grid cost to discover a tiny change.

* **`perf_50k_completely_different`**: total **1202ms**, with **815ms signature build** and **386ms cell diff** (5,000,000 cells compared). 
  Here, preflight is *more expensive than the full positional diff it ultimately chooses*.

This is also a regression vs the older baseline you included:

* Baseline `dense_single_edit`: **371ms total**, **296ms signature build**. 
* Baseline `completely_different`: **453ms total**, **295ms signature build**. 

So the bottleneck moved: move-detection was historically the monster (e.g. baseline block-move had ~**5.3s move_detection_time** with an estimated **5,000,000 hash lookups** , and your “improvement #4” writeup is explicitly about eliminating that kind of inner-loop cost ). Now that move detection is cheap in the latest run (block-move move detection is ~**50ms** ), **preflight signature work became the dominant term**.

### Where the wasted work is coming from in the code

Right now, preflight does this for same-shape, large grids:

1. Builds **all row signatures** for **both** grids (`row_signatures_for_grid` loops every row and calls `grid.compute_row_signature(row)`).
2. Builds sets and a multiset delta, then computes Jaccard, multiset edit distance, etc.
3. If it short-circuits (near-identical or dissimilar), `diff_grids_core` ends SignatureBuild immediately and runs positional diff. 

So for **near-identical** and **dissimilar** cases, the entire `signature_build_time_ms` is basically “preflight overhead”. That matches the metrics: 870ms to discover a single edit. 

### The key mathematical insight that unlocks a faster preflight

Your near-identical logic uses two distinct signals:

* **In-order mismatches** (rows where old_signature[i] != new_signature[i])
* **Reorder suspicion**, based on comparing in-order mismatches to the **multiset edit distance** between row signature multisets.

Here’s the crucial observation:

If you can identify which rows are **content-equal in-order**, then every matching row contributes `+1` and `-1` to the same signature in the multiset delta — they **cancel exactly**.
So the multiset delta (and thus the multiset edit distance) depends **only on the mismatched rows**.

Formally, let `S_old(i)` and `S_new(i)` be row signatures. Define delta per signature `h`:

`delta(h) = count_old(h) - count_new(h)`

Split rows into `M` (mismatched rows) and `E` (equal rows). For any `i in E`, `S_old(i)=S_new(i)` so it adds `+1` and `-1` to the same `h`, net 0. Therefore:

`delta(h) = sum_{i in M} [S_old(i)=h] - sum_{i in M} [S_new(i)=h]`

Meaning: **you do not need signatures for all rows** to compute multiset edit distance — you only need signatures for the mismatched rows, *if mismatched rows were identified by real content equality*.

That allows an algorithmic rewrite of preflight:

* Identify mismatched rows by **direct row content comparison** (one pass over cells, early-stopping at mismatch_max+1).
* Compute multiset edit distance by hashing **only those mismatched rows** (<= 32 by config).
* Use a cheap **sampled Jaccard** to detect “clearly dissimilar” without building all signatures.

### The algorithmic improvement

#### 1) Fast near-identical detection for dense sheets

For dense-ish sheets (high fill ratio), do:

* Scan rows in-order, compare row content cell-by-cell (using the same `cells_content_equal` used elsewhere), and collect mismatched row indices. Stop once mismatches exceed `mismatch_max + 1`.
* If mismatches <= mismatch_max and match ratio >= threshold:

  * Compute row signatures **only for those mismatched rows** (<= 32 rows)
  * Compute exact multiset edit distance using only those mismatches (exact because matched rows cancel)
  * If not multiset-equal and not reorder-suspected, short-circuit near-identical with those mismatched rows.

This replaces the current “hash 100k rows” behavior for `dense_single_edit` with:

* One cell-equality scan (about half the per-cell work vs hashing both grids), and
* Hashing at most 32 rows for reorder detection.

This directly targets the 870ms preflight cost dominating that benchmark. 

#### 2) Cheap dissimilar detection (sampled Jaccard)

For dissimilar detection, instead of building full signature arrays:

* Sample up to N rows (e.g. 4096 evenly spaced), compute row signatures for those rows only, build two sets, compute Jaccard.
* If sampled Jaccard is below the configured `bailout_similarity_threshold`, short-circuit dissimilar.

This turns `completely_different` from “hash everything then positional diff anyway” into “hash a small sample then positional diff”. It targets the 815ms wasted preflight. 

#### 3) Preserve behavior on sparse sheets

The dense row-scan can be painful on extremely sparse sheets if implemented as `get(row,col)` in a tight loop. So:

* Use the dense scan only when both grids are “dense enough” (simple density heuristic using `cell_count` vs `nrows*ncols`).
* For sparse cases, fall back to your current signature-based preflight (but you still get the sampled Jaccard early bailout).

### Expected impact on your provided benchmarks

Based on your metrics:

* `perf_50k_dense_single_edit`: signature build is 870ms. 
  This change should drop it substantially because you stop hashing both entire grids; you do one equality scan + hash <= 32 rows.

* `perf_50k_completely_different`: signature build is 815ms and cell diff is 386ms. 
  This change should drop signature build close to “sample cost” (tens of ms), making total time approach “positional diff time + overhead” instead of “positional diff + almost another full pass”.

And importantly, this is the same spirit as the lesson in your improvement #4 writeup: eliminate “full-size work that you don’t actually need to do” in the early decision path.

---

## Code to replace

```rust
fn preflight_decision_from_grids(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> PreflightLite {
    let nrows_old = old.nrows as usize;
    let nrows_new = new.nrows as usize;
    let ncols_old = old.ncols as usize;
    let ncols_new = new.ncols as usize;

    if nrows_old != nrows_new || ncols_old != ncols_new {
        return PreflightLite {
            decision: PreflightDecision::RunFullPipeline,
            mismatched_rows: Vec::new(),
        };
    }

    let nrows = nrows_old;
    if nrows < config.preflight.preflight_min_rows as usize {
        return PreflightLite {
            decision: PreflightDecision::RunFullPipeline,
            mismatched_rows: Vec::new(),
        };
    }

    let old_signatures = row_signatures_for_grid(old);
    let new_signatures = row_signatures_for_grid(new);

    let (in_order_matches, old_sig_set, new_sig_set) =
        compute_row_signature_stats(&old_signatures, &new_signatures);

    let in_order_mismatches = nrows.saturating_sub(in_order_matches);
    let in_order_match_ratio = if nrows > 0 {
        in_order_matches as f64 / nrows as f64
    } else {
        1.0
    };

    let intersection_size = old_sig_set.intersection(&new_sig_set).count();
    let union_size = old_sig_set.union(&new_sig_set).count();
    let jaccard = if union_size > 0 {
        intersection_size as f64 / union_size as f64
    } else {
        1.0
    };

    if jaccard < config.preflight.bailout_similarity_threshold {
        return PreflightLite {
            decision: PreflightDecision::ShortCircuitDissimilar,
            mismatched_rows: Vec::new(),
        };
    }

    let (multiset_equal, multiset_edit_distance_rows) =
        multiset_equal_and_edit_distance(&old_signatures, &new_signatures);

    let reorder_suspected = (in_order_mismatches as u64) > multiset_edit_distance_rows;

    let near_identical = in_order_mismatches
        <= config.preflight.preflight_in_order_mismatch_max as usize
        && in_order_match_ratio >= config.preflight.preflight_in_order_match_ratio_min
        && !multiset_equal
        && !reorder_suspected;

    if near_identical {
        return PreflightLite {
            decision: PreflightDecision::ShortCircuitNearIdentical,
            mismatched_rows: mismatched_rows_from_signatures(&old_signatures, &new_signatures),
        };
    }

    PreflightLite {
        decision: PreflightDecision::RunFullPipeline,
        mismatched_rows: Vec::new(),
    }
}

fn compute_row_signature_stats(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> (usize, HashSet<RowSignature>, HashSet<RowSignature>) {
    let mut in_order_matches = 0usize;
    let mut old_sig_set = HashSet::with_capacity(old_signatures.len());
    let mut new_sig_set = HashSet::with_capacity(new_signatures.len());

    for (old_sig, new_sig) in old_signatures.iter().zip(new_signatures.iter()) {
        if old_sig == new_sig {
            in_order_matches += 1;
        }
        old_sig_set.insert(*old_sig);
        new_sig_set.insert(*new_sig);
    }

    (in_order_matches, old_sig_set, new_sig_set)
}

fn multiset_equal_and_edit_distance(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> (bool, u64) {
    let mut delta: HashMap<RowSignature, i32> = HashMap::new();
    for sig in old_signatures {
        *delta.entry(*sig).or_insert(0) += 1;
    }
    for sig in new_signatures {
        *delta.entry(*sig).or_insert(0) -= 1;
    }

    let mut equal = true;
    let mut sum_abs: u64 = 0;
    for (_sig, d) in delta {
        if d != 0 {
            equal = false;
            sum_abs = sum_abs.saturating_add(d.unsigned_abs() as u64);
        }
    }

    (equal, sum_abs / 2)
}

fn mismatched_rows_from_signatures(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> Vec<u32> {
    let mut rows = Vec::new();
    let count = old_signatures.len().min(new_signatures.len());
    for idx in 0..count {
        let a = old_signatures[idx];
        let b = new_signatures[idx];
        if a != b {
            rows.push(idx as u32);
        }
    }
    rows
}

fn row_signatures_for_grid(grid: &Grid) -> Vec<RowSignature> {
    if let Some(sigs) = &grid.row_signatures {
        return sigs.clone();
    }
    let mut out = Vec::with_capacity(grid.nrows as usize);
    for row in 0..grid.nrows {
        out.push(grid.compute_row_signature(row));
    }
    out
}
```

## Replace with

```rust
fn preflight_decision_from_grids(
    old: &Grid,
    new: &Grid,
    config: &DiffConfig,
) -> PreflightLite {
    let nrows_old = old.nrows as usize;
    let nrows_new = new.nrows as usize;
    let ncols_old = old.ncols as usize;
    let ncols_new = new.ncols as usize;

    if nrows_old != nrows_new || ncols_old != ncols_new {
        return PreflightLite {
            decision: PreflightDecision::RunFullPipeline,
            mismatched_rows: Vec::new(),
        };
    }

    let nrows = nrows_old;
    if nrows < config.preflight.preflight_min_rows as usize {
        return PreflightLite {
            decision: PreflightDecision::RunFullPipeline,
            mismatched_rows: Vec::new(),
        };
    }

    let dense_scan = should_use_dense_row_scan(old, new);
    if dense_scan {
        let max_mismatches = config.preflight.preflight_in_order_mismatch_max as usize;
        let mismatched_rows =
            mismatched_rows_by_content_limit(old, new, max_mismatches.saturating_add(1));
        let in_order_mismatches = mismatched_rows.len();
        let in_order_matches = nrows.saturating_sub(in_order_mismatches);
        let in_order_match_ratio = if nrows > 0 {
            in_order_matches as f64 / nrows as f64
        } else {
            1.0
        };

        if in_order_mismatches <= max_mismatches
            && in_order_match_ratio >= config.preflight.preflight_in_order_match_ratio_min
        {
            let (multiset_equal, multiset_edit_distance_rows) =
                multiset_equal_and_edit_distance_for_mismatched_rows(old, new, &mismatched_rows);

            let reorder_suspected = (in_order_mismatches as u64) > multiset_edit_distance_rows;
            let near_identical = !multiset_equal && !reorder_suspected;

            if near_identical {
                return PreflightLite {
                    decision: PreflightDecision::ShortCircuitNearIdentical,
                    mismatched_rows,
                };
            }
        }
    }

    let sample_rows = sample_row_indices(old.nrows, 4096);
    let jaccard = sample_row_signature_jaccard(old, new, &sample_rows);

    if jaccard < config.preflight.bailout_similarity_threshold {
        return PreflightLite {
            decision: PreflightDecision::ShortCircuitDissimilar,
            mismatched_rows: Vec::new(),
        };
    }

    if !dense_scan {
        let old_signatures = row_signatures_for_grid(old);
        let new_signatures = row_signatures_for_grid(new);

        let (in_order_matches, old_sig_set, new_sig_set) =
            compute_row_signature_stats(&old_signatures, &new_signatures);

        let in_order_mismatches = nrows.saturating_sub(in_order_matches);
        let in_order_match_ratio = if nrows > 0 {
            in_order_matches as f64 / nrows as f64
        } else {
            1.0
        };

        let intersection_size = old_sig_set.intersection(&new_sig_set).count();
        let union_size = old_sig_set.union(&new_sig_set).count();
        let jaccard = if union_size > 0 {
            intersection_size as f64 / union_size as f64
        } else {
            1.0
        };

        if jaccard < config.preflight.bailout_similarity_threshold {
            return PreflightLite {
                decision: PreflightDecision::ShortCircuitDissimilar,
                mismatched_rows: Vec::new(),
            };
        }

        let (multiset_equal, multiset_edit_distance_rows) =
            multiset_equal_and_edit_distance(&old_signatures, &new_signatures);

        let reorder_suspected = (in_order_mismatches as u64) > multiset_edit_distance_rows;

        let near_identical = in_order_mismatches
            <= config.preflight.preflight_in_order_mismatch_max as usize
            && in_order_match_ratio >= config.preflight.preflight_in_order_match_ratio_min
            && !multiset_equal
            && !reorder_suspected;

        if near_identical {
            return PreflightLite {
                decision: PreflightDecision::ShortCircuitNearIdentical,
                mismatched_rows: mismatched_rows_from_signatures(&old_signatures, &new_signatures),
            };
        }
    }

    PreflightLite {
        decision: PreflightDecision::RunFullPipeline,
        mismatched_rows: Vec::new(),
    }
}

fn should_use_dense_row_scan(old: &Grid, new: &Grid) -> bool {
    let max_cells = (old.nrows as u64).saturating_mul(old.ncols as u64);
    if max_cells == 0 {
        return false;
    }
    let old_dense = (old.cell_count() as u64) > (max_cells / 2);
    let new_dense = (new.cell_count() as u64) > (max_cells / 2);
    old_dense && new_dense
}

fn mismatched_rows_by_content_limit(old: &Grid, new: &Grid, max_out: usize) -> Vec<u32> {
    let mut out = Vec::new();
    for row in 0..old.nrows {
        if !rows_content_equal(old, new, row) {
            out.push(row);
            if out.len() >= max_out {
                break;
            }
        }
    }
    out
}

fn rows_content_equal(old: &Grid, new: &Grid, row: u32) -> bool {
    for col in 0..old.ncols {
        if !cells_content_equal(old.get(row, col), new.get(row, col)) {
            return false;
        }
    }
    true
}

fn multiset_equal_and_edit_distance_for_mismatched_rows(
    old: &Grid,
    new: &Grid,
    mismatched_rows: &[u32],
) -> (bool, u64) {
    let mut delta: HashMap<RowSignature, i32> = HashMap::new();
    for &row in mismatched_rows {
        let a = old.compute_row_signature(row);
        let b = new.compute_row_signature(row);
        *delta.entry(a).or_insert(0) += 1;
        *delta.entry(b).or_insert(0) -= 1;
    }

    let mut equal = true;
    let mut sum_abs: u64 = 0;
    for (_sig, d) in delta {
        if d != 0 {
            equal = false;
            sum_abs = sum_abs.saturating_add(d.unsigned_abs() as u64);
        }
    }

    (equal, sum_abs / 2)
}

fn sample_row_indices(nrows: u32, max_samples: usize) -> Vec<u32> {
    let n = nrows as usize;
    if n == 0 {
        return Vec::new();
    }
    let target = max_samples.min(n);
    let step = (n / target).max(1);

    let mut out = Vec::with_capacity(target + 1);
    let mut idx = 0usize;
    while idx < n && out.len() < target {
        out.push(idx as u32);
        idx = idx.saturating_add(step);
    }

    if *out.last().unwrap_or(&0) != nrows.saturating_sub(1) {
        out.push(nrows.saturating_sub(1));
    }

    out.sort_unstable();
    out.dedup();
    out
}

fn sample_row_signature_jaccard(old: &Grid, new: &Grid, sample_rows: &[u32]) -> f64 {
    let mut a = HashSet::with_capacity(sample_rows.len());
    let mut b = HashSet::with_capacity(sample_rows.len());

    for &row in sample_rows {
        a.insert(old.compute_row_signature(row));
        b.insert(new.compute_row_signature(row));
    }

    let intersection = a.intersection(&b).count();
    let union = a.len() + b.len() - intersection;
    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

fn compute_row_signature_stats(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> (usize, HashSet<RowSignature>, HashSet<RowSignature>) {
    let mut in_order_matches = 0usize;
    let mut old_sig_set = HashSet::with_capacity(old_signatures.len());
    let mut new_sig_set = HashSet::with_capacity(new_signatures.len());

    for (old_sig, new_sig) in old_signatures.iter().zip(new_signatures.iter()) {
        if old_sig == new_sig {
            in_order_matches += 1;
        }
        old_sig_set.insert(*old_sig);
        new_sig_set.insert(*new_sig);
    }

    (in_order_matches, old_sig_set, new_sig_set)
}

fn multiset_equal_and_edit_distance(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> (bool, u64) {
    let mut delta: HashMap<RowSignature, i32> = HashMap::new();
    for sig in old_signatures {
        *delta.entry(*sig).or_insert(0) += 1;
    }
    for sig in new_signatures {
        *delta.entry(*sig).or_insert(0) -= 1;
    }

    let mut equal = true;
    let mut sum_abs: u64 = 0;
    for (_sig, d) in delta {
        if d != 0 {
            equal = false;
            sum_abs = sum_abs.saturating_add(d.unsigned_abs() as u64);
        }
    }

    (equal, sum_abs / 2)
}

fn mismatched_rows_from_signatures(
    old_signatures: &[RowSignature],
    new_signatures: &[RowSignature],
) -> Vec<u32> {
    let mut rows = Vec::new();
    let count = old_signatures.len().min(new_signatures.len());
    for idx in 0..count {
        let a = old_signatures[idx];
        let b = new_signatures[idx];
        if a != b {
            rows.push(idx as u32);
        }
    }
    rows
}

fn row_signatures_for_grid(grid: &Grid) -> Vec<RowSignature> {
    if let Some(sigs) = &grid.row_signatures {
        return sigs.clone();
    }
    let mut out = Vec::with_capacity(grid.nrows as usize);
    for row in 0..grid.nrows {
        out.push(grid.compute_row_signature(row));
    }
    out
}
```

---

If you want a second, follow-on improvement after this one lands: the next obvious target is **reducing GridView construction cost** for the full pipeline path (e.g. `perf_50k_alignment_block_move` still pays ~1058ms in SignatureBuild while move detection is only ~50ms ). But the preflight change above is the cleanest high-leverage fix because it attacks the biggest source of wasted work shown directly by your metrics.
