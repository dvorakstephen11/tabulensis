Your diagnosis is basically right about *where* the regression is coming from, but the fix I’d make is **not** “remove the overlap check”. The overlap check is only in the **strict / unmasked** detector; the real issue is that your new **fast path returns early on `None`**, so the masked/iterative rect logic never runs for the g14 multi-move / move+edits scenarios.

### Why those 4 g14 tests now get **0 rect moves**

The failing g14 cases all include **extra diffs beyond the two moved rectangles** (either because there are multiple disjoint moves, or a move plus an unrelated edit). For example, `g14_rect_move_plus_cell_edit_no_silent_data_loss` explicitly adds an unrelated edit at `[0][0]` in addition to the rect move. 

Your new fast path in `detect_exact_rect_block_move_masked` does:

```rust
if !old_mask.has_exclusions()
    && !new_mask.has_exclusions()
    && old.nrows == new.nrows
    && old.ncols == new.ncols
{
    return detect_exact_rect_block_move(old, new, config);
}
```

So on the first iteration (no exclusions), you bypass the masked heuristic path entirely and rely on the strict detector. 

But the strict detector **intentionally rejects** “rect move + anything else” because it requires the mismatch count to equal exactly `2 * row_count * col_count` (i.e., *only* the src and dst rectangles differ). Any extra edit/move means it returns `None`. 

That is exactly why those tests see **0** `BlockMovedRect`.

---

## Minimal, correct fix (exact code change)

Keep the optimization, but make it an **early-success** only. If strict detection fails, **fall through** to the full masked heuristic algorithm.

### Patch: `core/src/engine.rs`

Find `detect_exact_rect_block_move_masked` and replace the current fast-path block with this:

```diff
diff --git a/core/src/engine.rs b/core/src/engine.rs
@@ fn detect_exact_rect_block_move_masked(
-    if !old_mask.has_exclusions()
-        && !new_mask.has_exclusions()
-        && old.nrows == new.nrows
-        && old.ncols == new.ncols
-    {
-        return detect_exact_rect_block_move(old, new, config);
-    }
+    // Fast path: when masks are fully active and shapes match, try the strict
+    // "single clean rect move" detector first.
+    //
+    // IMPORTANT: The strict detector returns None when there are additional diffs
+    // (e.g. move + unrelated edits, or multiple disjoint rect moves). In those cases
+    // we must fall through to the masked heuristic search.
+    if !old_mask.has_exclusions()
+        && !new_mask.has_exclusions()
+        && old.nrows == new.nrows
+        && old.ncols == new.ncols
+    {
+        if let Some(mv) = detect_exact_rect_block_move(old, new, config) {
+            return Some(mv);
+        }
+        // else: fall through to full masked logic
+    }
```

This preserves the performance win for the “easy” case (single clean rect move), but restores correctness for:

* **rect move + outside edits** (`g14_rect_move_plus_cell_edit_no_silent_data_loss`) 
* **multiple disjoint rect moves** (`g14_three_disjoint_rect_block_moves_detected`, etc.) 
* **iteration-limited detection** (`g14_max_move_iterations_limits_detected_moves`) 

---

## Why this fix is the right tradeoff

* The masked rect detector is the only rect path designed to keep working when there are **extra diffs**, because it uses diff-pattern grouping and scoped verification (your full function continues after the strict attempt by collecting diffs and searching candidate row/col range pairs). 
* The strict detector is (correctly) **not** intended to solve those combined cases. 
* So the fast path must be **“try strict, but don’t give up”**, not “try strict, else return None”.

---

## I would *not* remove the overlap check (at least not for this regression)

Based on the code you’ve got, the strict rect detector’s “exact move” criteria (mismatch count, two equal row ranges, two equal col ranges, etc.) is already extremely restrictive. 
The overlap check is not the reason those g14 tests fail; they fail earlier because strict detection isn’t even allowed to succeed in those scenarios, and the fast path prevents fallback.

If you want the overlap check to be less aggressive, keep the `&&` behavior you already applied (only reject if rectangles overlap in **both** dimensions, i.e., truly overlap as rectangles). But again: it’s not what’s breaking g14.

---

## Optional hardening: add a regression unit test (fast + surgical)

If you want a “tripwire” that prevents this exact bug from returning later, add a unit test **inside `engine.rs`** (so it can call the private helper) that asserts the masked detector still finds a move even when strict detection should fail due to an outside edit.

Pseudo-outline (you’ll need to adapt to your existing test helpers):

```rust
#[test]
fn rect_move_masked_falls_back_when_outside_edit_exists() {
    let mut old = base_background(12, 12);
    let mut new = old.clone();

    // perform a clean 2x3 rect move (whatever your helper is)
    apply_rect_move(&mut new, /*src*/ (2,2), /*dst*/ (8,6), /*h*/2, /*w*/3);

    // unrelated edit outside both rectangles
    new.set_number(0, 0, 77777);

    let old_mask = RegionMask::all_active(old.nrows, old.ncols);
    let new_mask = RegionMask::all_active(new.nrows, new.ncols);

    let cfg = DiffConfig::default();
    let mv = detect_exact_rect_block_move_masked(&old, &new, &old_mask, &new_mask, &cfg);

    assert!(mv.is_some(), "masked detector should fall back and detect the move");
}
```

This is essentially the same scenario as the failing g14 test, but it isolates the regression to the one function that caused it.

---

## Quick read on your latest fullscale metrics

From the most recent fullscale run you attached (2025‑12‑14_005407_fullscale), totals look roughly like:

* `perf_50k_99_percent_blank`: **183 ms**
* `perf_50k_identical`: **1100 ms**
* `perf_50k_adversarial_repetitive`: **11583 ms**
* `perf_50k_dense_single_edit`: **11499 ms**
* `perf_50k_completely_different`: **13005 ms** 

The patch above should **not** materially affect those, because the rect masked move loop is gated by `recursive_align_threshold`/`max_move_detection_cols` (so large 50k sheets aren’t running this rect-masked heuristic loop in the first place). 

---

### If you apply only one change: do the “early success only” fast path

That should restore the 4 missing rect moves in g14 without undoing your performance-oriented refactor.
