# Remediation Report (Independent Review)

This report is based on the provided **codebase snapshot** (`codebase_context.md`), the reviewer’s report (`remediationE.md`), and the specs/plans (`next_sprint_plan.md`, `excel_diff_specification.md`, `unified_grid_diff_algorithm_specification.md`), plus the benchmark artifacts (`benchmark_results.json`, `cycle_summary.txt`).    

---

## Executive Summary

I agree with the reviewer’s core findings:

1. **Documented DiffConfig defaults/presets do not match implementation** (notably `min_block_size_for_move` default and `most_precise.fuzzy_similarity_threshold`).  
2. **Masked move detection gating is coupled to `recursive_align_threshold`**, which is conceptually unrelated and creates surprising behavior for users tuning recursion. 
3. **`include_unchanged_cells` can create misleading “edits” (no-op `CellEdited`) and is untested in the JSON cell-diff projection**, which currently includes all `CellEdited` without filtering.  
4. Full-scale perf is improved but still **misses the stated Branch 2 target** for the 50k×100 dense case (target `<5s`, observed ~9.2s).  
5. There’s a small hygiene issue: an `unused_mut` warning in an engine test. 

My recommendations are pragmatic:

* **Fix the config drift in code** (not in docs) unless you can point to a deliberate “intentional deviation” record for Branch 3 (none was present in the attached materials).
* **Add a dedicated `max_move_detection_rows` knob** to decouple move-detection gating from recursion tuning while preserving current defaults.
* **Make `diff_report_to_cell_diffs` always filter no-op cell edits**, so `include_unchanged_cells` can’t corrupt a “diffs-only” consumer view.
* **Clarify perf “targets” vs “enforced asserts” in full-scale tests** to avoid misleading output while you continue performance work.

---

## Remediation Plan (What to change, why, and exact code replacements)

### Fix 1: Align DiffConfig defaults/presets to the Branch 3 sprint plan

#### What’s wrong

* Sprint plan specifies `min_block_size_for_move: 3` in `Default`, but implementation uses `1`.  
* Sprint plan specifies `most_precise.fuzzy_similarity_threshold: 0.95` and `enable_formula_semantic_diff: true`, but implementation uses `0.90` and does not set `enable_formula_semantic_diff`.  

Even if the current values were chosen for performance or recall, **the bigger bug is the mismatch between “what users think they’re getting” and what they actually get**.

#### Recommendation

* Update code to match the sprint plan for:

  * `min_block_size_for_move` default = **3**
  * `most_precise.fuzzy_similarity_threshold` = **0.95**
  * `most_precise.enable_formula_semantic_diff` = **true** (even if currently unused, it matches documented intent)

#### Exact code changes

##### 1A) `core/src/config.rs`: add a dedicated row gate for move detection (also used by Fix 2)

This also sets up Fix 2, but I’m including it here because it touches the same struct.

**Code to replace** (tail of `DiffConfig` struct):

```rust
    pub context_anchor_k1: u32,
    pub context_anchor_k2: u32,
    pub max_move_detection_cols: u32,
}
```

**Replace with**:

```rust
    pub context_anchor_k1: u32,
    pub context_anchor_k2: u32,
    pub max_move_detection_rows: u32,
    pub max_move_detection_cols: u32,
}
```



##### 1B) `core/src/config.rs`: update `Default` values

**Code to replace** (relevant lines in `impl Default for DiffConfig`):

```rust
            include_unchanged_cells: false,
            max_context_rows: 3,
            min_block_size_for_move: 1,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            context_anchor_k1: 4,
            context_anchor_k2: 8,
            max_move_detection_cols: 256,
```

**Replace with**:

```rust
            include_unchanged_cells: false,
            max_context_rows: 3,
            min_block_size_for_move: 3,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            context_anchor_k1: 4,
            context_anchor_k2: 8,
            max_move_detection_rows: 200,
            max_move_detection_cols: 256,
```



##### 1C) `core/src/config.rs`: update `fastest()` and `most_precise()` presets

**Code to replace** (`fastest()`):

```rust
    pub fn fastest() -> Self {
        Self {
            max_move_iterations: 5,
            max_block_gap: 1_000,
            small_gap_threshold: 20,
            recursive_align_threshold: 80,
            enable_fuzzy_moves: false,
            enable_m_semantic_diff: false,
            ..Default::default()
        }
    }
```

**Replace with**:

```rust
    pub fn fastest() -> Self {
        Self {
            max_move_iterations: 5,
            max_block_gap: 1_000,
            small_gap_threshold: 20,
            recursive_align_threshold: 80,
            max_move_detection_rows: 80,
            enable_fuzzy_moves: false,
            enable_m_semantic_diff: false,
            ..Default::default()
        }
    }
```



**Code to replace** (`most_precise()`):

```rust
    pub fn most_precise() -> Self {
        Self {
            max_move_iterations: 30,
            max_block_gap: 20_000,
            fuzzy_similarity_threshold: 0.90,
            small_gap_threshold: 80,
            recursive_align_threshold: 400,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            max_move_detection_cols: 256,
            ..Default::default()
        }
    }
```

**Replace with**:

```rust
    pub fn most_precise() -> Self {
        Self {
            max_move_iterations: 30,
            max_block_gap: 20_000,
            fuzzy_similarity_threshold: 0.95,
            small_gap_threshold: 80,
            recursive_align_threshold: 400,
            enable_formula_semantic_diff: true,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            max_move_detection_rows: 400,
            max_move_detection_cols: 256,
            ..Default::default()
        }
    }
```



##### 1D) `core/src/config.rs`: validate new field and update builder API

**Code to replace** (end of `validate()` non-zero checks):

```rust
        ensure_non_zero_u32(self.context_anchor_k1, "context_anchor_k1")?;
        ensure_non_zero_u32(self.context_anchor_k2, "context_anchor_k2")?;
        ensure_non_zero_u32(self.max_move_detection_cols, "max_move_detection_cols")?;
        ensure_non_zero_u32(self.max_context_rows, "max_context_rows")?;
        ensure_non_zero_u32(self.min_block_size_for_move, "min_block_size_for_move")?;
```

**Replace with**:

```rust
        ensure_non_zero_u32(self.context_anchor_k1, "context_anchor_k1")?;
        ensure_non_zero_u32(self.context_anchor_k2, "context_anchor_k2")?;
        ensure_non_zero_u32(self.max_move_detection_rows, "max_move_detection_rows")?;
        ensure_non_zero_u32(self.max_move_detection_cols, "max_move_detection_cols")?;
        ensure_non_zero_u32(self.max_context_rows, "max_context_rows")?;
        ensure_non_zero_u32(self.min_block_size_for_move, "min_block_size_for_move")?;
```



**Code to replace** (builder setters tail):

```rust
    pub fn context_anchor_k2(mut self, value: u32) -> Self {
        self.inner.context_anchor_k2 = value;
        self
    }

    pub fn max_move_detection_cols(mut self, value: u32) -> Self {
        self.inner.max_move_detection_cols = value;
        self
    }
```

**Replace with**:

```rust
    pub fn context_anchor_k2(mut self, value: u32) -> Self {
        self.inner.context_anchor_k2 = value;
        self
    }

    pub fn max_move_detection_rows(mut self, value: u32) -> Self {
        self.inner.max_move_detection_rows = value;
        self
    }

    pub fn max_move_detection_cols(mut self, value: u32) -> Self {
        self.inner.max_move_detection_cols = value;
        self
    }
```



##### 1E) `core/src/config.rs`: tighten tests so this doesn’t drift again

Today `defaults_match_limit_spec` doesn’t assert the most problematic fields, which is how this drift slipped in. 

**Code to replace** (`defaults_match_limit_spec` test):

```rust
    #[test]
    fn defaults_match_limit_spec() {
        let cfg = DiffConfig::default();
        assert_eq!(cfg.max_align_rows, 500_000);
        assert_eq!(cfg.max_align_cols, 16_384);
        assert_eq!(cfg.max_recursion_depth, 10);
        assert!(matches!(
            cfg.on_limit_exceeded,
            LimitBehavior::FallbackToPositional
        ));
    }
```

**Replace with**:

```rust
    #[test]
    fn defaults_match_limit_spec() {
        let cfg = DiffConfig::default();

        assert_eq!(cfg.max_align_rows, 500_000);
        assert_eq!(cfg.max_align_cols, 16_384);
        assert_eq!(cfg.max_recursion_depth, 10);
        assert!(matches!(
            cfg.on_limit_exceeded,
            LimitBehavior::FallbackToPositional
        ));

        assert_eq!(cfg.fuzzy_similarity_threshold, 0.80);
        assert_eq!(cfg.min_block_size_for_move, 3);
        assert_eq!(cfg.max_move_iterations, 20);

        assert_eq!(cfg.recursive_align_threshold, 200);
        assert_eq!(cfg.small_gap_threshold, 50);

        assert_eq!(cfg.max_move_detection_rows, 200);
        assert_eq!(cfg.max_move_detection_cols, 256);

        assert!(!cfg.include_unchanged_cells);
        assert_eq!(cfg.max_context_rows, 3);

        assert!(cfg.enable_fuzzy_moves);
        assert!(cfg.enable_m_semantic_diff);
        assert!(!cfg.enable_formula_semantic_diff);
    }
```

 

**Add this new test** (place near the other preset tests):

```rust
    #[test]
    fn most_precise_matches_sprint_plan_values() {
        let cfg = DiffConfig::most_precise();
        assert_eq!(cfg.fuzzy_similarity_threshold, 0.95);
        assert!(cfg.enable_formula_semantic_diff);
    }
```



---

### Fix 2: Decouple masked move-detection gating from `recursive_align_threshold`

#### What’s wrong

In `diff_grids_core`, masked move detection is enabled only when:

* `max(nrows) <= config.recursive_align_threshold`
* and `max(ncols) <= config.max_move_detection_cols` 

This couples:

* recursion threshold (gap recursion in alignment)
  with
* whether you run the expensive masked move detection loop.

That coupling is hard to reason about and not described in the plan excerpt. 

#### Recommendation

* Introduce `DiffConfig::max_move_detection_rows` and use it for gating.
* Keep defaults such that behavior remains unchanged unless users opt in.

#### Exact code changes

##### 2A) `core/src/engine.rs`: change the gating line

**Code to replace**:

```rust
    let move_detection_enabled = old.nrows.max(new.nrows) <= config.recursive_align_threshold
        && old.ncols.max(new.ncols) <= config.max_move_detection_cols;
```

**Replace with**:

```rust
    let move_detection_enabled = old.nrows.max(new.nrows) <= config.max_move_detection_rows
        && old.ncols.max(new.ncols) <= config.max_move_detection_cols;
```



##### 2B) Add a targeted test proving independence from recursion threshold

You already have a limit test that disables moves by setting `max_move_iterations: 0`, but it doesn’t cover this gating coupling. 

**Where**: `core/tests/g14_move_combination_tests.rs` (same file as existing g14 tests). 

**Code to replace** (nothing here yet; use the existing `g14_move_detection_disabled_falls_back_to_positional` test as an anchor and add below it). If you prefer strict “replace blocks only”, replace the entire test function section around `g14_move_detection_disabled_falls_back_to_positional` with the version below that includes an added test.

**Replace with**:

```rust
#[test]
fn g14_move_detection_disabled_falls_back_to_positional() {
    let grid_a = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 20, 30],
        &[100, 200, 300],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
    ]);

    let grid_b = grid_from_numbers(&[
        &[1, 2, 3],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
        &[10, 20, 30],
        &[100, 200, 300],
    ]);

    let config = DiffConfig {
        max_move_iterations: 0,
        ..DiffConfig::default()
    };

    let mut report = DiffReport::new();
    diff_grids(&grid_a, &grid_b, "S", &config, &mut report);

    assert!(
        report.ops.iter().any(|op| matches!(op, DiffOp::RowRemoved { .. })),
        "expected positional fallback when move detection disabled"
    );
    assert!(
        report.ops.iter().any(|op| matches!(op, DiffOp::RowAdded { .. })),
        "expected positional fallback when move detection disabled"
    );
    assert!(
        !report.ops.iter().any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "no block move should be present when move detection disabled"
    );
}

#[test]
fn g14_masked_move_detection_not_gated_by_recursive_align_threshold() {
    let grid_a = grid_from_numbers(&[
        &[1, 2, 3],
        &[10, 20, 30],
        &[100, 200, 300],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
    ]);

    let grid_b = grid_from_numbers(&[
        &[1, 2, 3],
        &[1000, 2000, 3000],
        &[10000, 20000, 30000],
        &[10, 20, 30],
        &[100, 200, 300],
    ]);

    let config = DiffConfig {
        recursive_align_threshold: 1,
        max_move_detection_rows: 10,
        ..DiffConfig::default()
    };

    let mut report = DiffReport::new();
    diff_grids(&grid_a, &grid_b, "S", &config, &mut report);

    assert!(
        report.ops.iter().any(|op| matches!(op, DiffOp::BlockMovedRows { .. })),
        "masked move detection should be enabled by max_move_detection_rows, independent of recursion threshold"
    );
}
```

This test is intentionally minimal: it proves the gating knob you actually care about (`max_move_detection_rows`) is what enables masked moves, not `recursive_align_threshold`.

---

### Fix 3: Make `include_unchanged_cells` safe for consumer projections

#### What’s wrong

* In `diff_row_pair_sparse`, you emit a `DiffOp::CellEdited` when `old_val == new_val` if `include_unchanged_cells` is enabled. 
* `diff_report_to_cell_diffs` then includes **all** `CellEdited` ops without checking whether `from` equals `to`. 

That combination means a consumer turning on `include_unchanged_cells` can easily get a “diff list” full of no-op “edits”.

Also: database-mode diff compares every column index and `continue`s on equality; applying `include_unchanged_cells` there naïvely would explode output (including implicit blanks), so I recommend *documenting* that the flag is diagnostic + spreadsheet-mode oriented unless/until database diff becomes sparse-aware.  

#### Recommendation

* **At minimum**: in `diff_report_to_cell_diffs`, filter out no-op edits (`from == to`), unconditionally.
* Add a unit test to lock this behavior.
* Add a short doc comment in `DiffConfig` that `include_unchanged_cells` may emit no-op `CellEdited` operations (diagnostic behavior) so downstream consumers shouldn’t treat all `CellEdited` as semantic edits unless they also check `from != to`.

#### Exact code changes

##### 3A) `core/src/output/json.rs`: filter no-op edits in `diff_report_to_cell_diffs`

**Code to replace**:

```rust
pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn render_value(value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(s)) => Some(s.clone()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            None => None,
        }
    }

    report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                Some(CellDiff {
                    coords: addr.to_a1(),
                    value_file1: render_value(&from.value),
                    value_file2: render_value(&to.value),
                })
            } else {
                None
            }
        })
        .collect()
}
```

**Replace with**:

```rust
pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn render_value(value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(s)) => Some(s.clone()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            None => None,
        }
    }

    report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                if from == to {
                    return None;
                }
                Some(CellDiff {
                    coords: addr.to_a1(),
                    value_file1: render_value(&from.value),
                    value_file2: render_value(&to.value),
                })
            } else {
                None
            }
        })
        .collect()
}
```



##### 3B) `core/tests/output_tests.rs`: add a unit test for filtering no-op edits

**Code to replace** (the section containing `diff_report_to_cell_diffs_maps_values_correctly`, replacing it with a version that includes the new test right after):

```rust
#[test]
fn diff_report_to_cell_diffs_maps_values_correctly() {
    let addr_a1 = CellAddress::from_indices(0, 0);
    let addr_a2 = CellAddress::from_indices(1, 0);

    let report = DiffReport::new(vec![
        DiffOp::cell_edited(
            "Sheet1".to_string(),
            addr_a1,
            make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr_a1, Some(CellValue::Number(2.0))),
        ),
        DiffOp::cell_edited(
            "Sheet1".to_string(),
            addr_a2,
            make_cell_snapshot(addr_a2, Some(CellValue::Text("old".to_string()))),
            make_cell_snapshot(addr_a2, Some(CellValue::Text("new".to_string()))),
        ),
    ]);

    let diffs = diff_report_to_cell_diffs(&report);

    assert_eq!(diffs.len(), 2);
    assert_eq!(diffs[0].coords, "A1");
    assert_eq!(diffs[0].value_file1, Some("1".to_string()));
    assert_eq!(diffs[0].value_file2, Some("2".to_string()));
    assert_eq!(diffs[1].coords, "A2");
    assert_eq!(diffs[1].value_file1, Some("old".to_string()));
    assert_eq!(diffs[1].value_file2, Some("new".to_string()));
}
```

**Replace with**:

```rust
#[test]
fn diff_report_to_cell_diffs_maps_values_correctly() {
    let addr_a1 = CellAddress::from_indices(0, 0);
    let addr_a2 = CellAddress::from_indices(1, 0);

    let report = DiffReport::new(vec![
        DiffOp::cell_edited(
            "Sheet1".to_string(),
            addr_a1,
            make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr_a1, Some(CellValue::Number(2.0))),
        ),
        DiffOp::cell_edited(
            "Sheet1".to_string(),
            addr_a2,
            make_cell_snapshot(addr_a2, Some(CellValue::Text("old".to_string()))),
            make_cell_snapshot(addr_a2, Some(CellValue::Text("new".to_string()))),
        ),
    ]);

    let diffs = diff_report_to_cell_diffs(&report);

    assert_eq!(diffs.len(), 2);
    assert_eq!(diffs[0].coords, "A1");
    assert_eq!(diffs[0].value_file1, Some("1".to_string()));
    assert_eq!(diffs[0].value_file2, Some("2".to_string()));
    assert_eq!(diffs[1].coords, "A2");
    assert_eq!(diffs[1].value_file1, Some("old".to_string()));
    assert_eq!(diffs[1].value_file2, Some("new".to_string()));
}

#[test]
fn diff_report_to_cell_diffs_filters_no_op_cell_edits() {
    let addr_a1 = CellAddress::from_indices(0, 0);
    let addr_a2 = CellAddress::from_indices(1, 0);

    let report = DiffReport::new(vec![
        DiffOp::cell_edited(
            "Sheet1".to_string(),
            addr_a1,
            make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr_a1, Some(CellValue::Number(1.0))),
        ),
        DiffOp::cell_edited(
            "Sheet1".to_string(),
            addr_a2,
            make_cell_snapshot(addr_a2, Some(CellValue::Number(1.0))),
            make_cell_snapshot(addr_a2, Some(CellValue::Number(2.0))),
        ),
    ]);

    let diffs = diff_report_to_cell_diffs(&report);

    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].coords, "A2");
    assert_eq!(diffs[0].value_file1, Some("1".to_string()));
    assert_eq!(diffs[0].value_file2, Some("2".to_string()));
}
```



---

### Fix 4: Reconcile “target <5s” vs what is actually enforced in full-scale perf tests

#### What’s wrong

Full-scale benchmarks show:

* `perf_50k_dense_single_edit`: **9168 ms**
* `perf_50k_completely_different`: **10241 ms** 

But the `#[ignore]` test prints `"target: <5s"` and enforces only `<30s`. 

That mismatch isn’t a “code bug,” but it’s a maintainability hazard: it makes people think the target is enforced when it isn’t.

#### Recommendation

Until the engine actually hits `<5s` reliably on reference hardware, don’t print a naked “target: <5s” without also stating what is enforced.

#### Exact code changes

In `core/tests/perf_large_grid_tests.rs`, adjust the tail strings to include both target and enforcement.

**Code to replace** (example in `perf_50k_dense_single_edit`):

```rust
    log_perf_metric("perf_50k_dense_single_edit", &metrics, " (target: <5s)");
```

**Replace with**:

```rust
    log_perf_metric(
        "perf_50k_dense_single_edit",
        &metrics,
        " (enforced: <30s; target: <5s)",
    );
```



Repeat similarly for:

* `perf_50k_completely_different`: enforced `<60s`, target `<10s` 
* `perf_50k_adversarial_repetitive`: enforced `<120s`, target `<15s` 

---

### Fix 5: Remove the remaining test warning (`unused_mut`)

#### What’s wrong

There’s an `unused_mut` warning from a test in `core/src/engine.rs`. 

#### Exact code change

**Code to replace**:

```rust
        let mut base: Vec<Vec<i32>> = (0..rows)
            .map(|r| (0..cols).map(|c| (r as i32) * 100 + c as i32).collect())
            .collect();
```

**Replace with**:

```rust
        let base: Vec<Vec<i32>> = (0..rows)
            .map(|r| (0..cols).map(|c| (r as i32) * 100 + c as i32).collect())
            .collect();
```



---

## Notes on Specs vs “Best Code”

You mentioned some deviations were intentional for performance. In this set of issues, I’d separate them into two buckets:

### Bucket A: “Mismatch/ambiguity” problems (should fix even if performance motivated)

* Defaults/preset mismatches: users can’t make good decisions if docs and behavior diverge.
* Coupled knobs: leads to surprising tuning behavior.
* No-op “edit” projection: creates user-facing foot-guns.

These fixes are about **clarity and correctness**, and most can be done with near-zero runtime impact.

### Bucket B: “True performance” gaps (bigger work, separate track)

Full-scale numbers are still far from:

* Branch 2 target `<5s` for dense 50k×100 (currently ~9.2s)  
* And even farther from the more aggressive unified-grid performance targets like “completely different grids <500ms” (if you treat that spec as binding). 

I would not try to “paper over” those with config changes; they likely require deeper changes in preprocessing/hashing/representation.

---

## Deliverable Checklist

If you apply the code changes above, you’ll have:

* [x] Defaults/presets match sprint plan (or at least, match the plan you shipped) 
* [x] Masked move detection gating is explicit and independently tunable 
* [x] `include_unchanged_cells` cannot corrupt `diff_report_to_cell_diffs` output, and behavior is tested  
* [x] Perf test output no longer implies strict enforcement where there isn’t any 
* [x] No leftover test warning noise 

