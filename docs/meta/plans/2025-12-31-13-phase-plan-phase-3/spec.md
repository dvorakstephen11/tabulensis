## Phase 3 Implementation Plan: Peak Memory Budgets, Low-Similarity Memory Cuts, and WASM Memory Gates

Phase 3 in your `13_phase_plan.md` is explicitly about making **peak memory a first-class invariant**, addressing **low-similarity memory spikes**, and putting **WASM memory budgets under automated enforcement**. 
This also matches the design review’s highest-priority perf guidance: low-similarity is where “extra structure building for little alignment benefit” tends to dominate, and WASM needs explicit, automated budgets.
Finally, your completion analysis calls out memory behavior at scale (especially low similarity) as a remaining risk area even though the perf tooling exists.

Below is a concrete, codebase-grounded plan to execute Phase 3 with minimal “hand-waving” and clear acceptance criteria.

---

## 0) Ground truth: what you already have (and why Phase 3 is the right next slice)

### You already have reliable native peak-memory instrumentation (non-WASM)

When `--features perf-metrics` is enabled (and not on `wasm32`), the core crate installs a global `CountingAllocator` that tracks **current** and **peak** heap bytes.
The perf pipeline resets peak at the start and captures it into `DiffMetrics.peak_memory_bytes`.

This is a solid foundation for “peak memory under X” invariants on native.

### You already have a “memory budget” concept, but it’s estimate-based (and not enforced as a regression gate)

`HardeningController` supports a configured `max_memory_mb` (wired into `max_memory_bytes`) and can force a positional fallback when an estimated peak exceeds the cap, emitting a warning.

This is useful for *correctness under constraints*, but Phase 3 is about *performance truthfulness + regression prevention*, which requires turning memory into a CI gate.

### The codebase contains the most likely root cause of low-similarity memory spikes

Your own estimator makes it explicit: `estimate_gridview_bytes` includes a `cell_entry_bytes` term proportional to `grid.cell_count() * sizeof((u32, &Cell)) * 5/4`. 

That matches the core structural reality: `GridView` stores per-row `Vec<(u32, &Cell)>`, which is a large, duplicated index of the grid’s contents.

Separately, preflight currently runs *after* building full `GridView`s, even though preflight is often used specifically to decide “this is too different; don’t do heavy alignment.”

Phase 3’s “avoid heavy metadata when bailout threshold is hit” is essentially: **stop paying the GridView tax in cases where you already know you won’t benefit from it**.

---

## 1) Phase 3 outcomes (explicit success criteria)

This phase should end with four “hard” deliverables:

1. **Native CI gate:** At least one low-similarity benchmark is enforced with an explicit `peak_memory_bytes <= cap` invariant (not just baseline regression).
2. **Low-similarity memory reduction:** The low-similarity path avoids constructing high-footprint structures (especially full `GridView`s) before deciding to bail out.
3. **WASM memory gate:** A WASM-targeted harness exists that executes a representative diff workload and fails CI if WASM linear memory exceeds a budget.
4. **Budget ownership:** Budgets live in source control, alongside existing perf tooling, and have an update workflow that’s consistent with how you already handle perf thresholds/baselines.

---

## 2) Workstream A: Turn peak memory into an automated invariant on native

### A1) Extend perf threshold enforcement to support absolute peak-memory caps

Right now:

* `scripts/check_perf_thresholds.py` enforces **absolute time caps**, and **baseline regression** checks for time and peak memory, but it does *not* enforce an absolute peak-memory budget per scenario.
* `scripts/export_e2e_metrics.py` similarly checks absolute time caps and baseline regressions, but not absolute memory caps.

**Implementation steps (native perf):**

1. In `scripts/check_perf_thresholds.py`, extend the threshold schema to allow an optional key, e.g.:

   * `max_peak_memory_bytes` (preferred; matches metric units)
   * or `max_peak_memory_mb` (more human-friendly but requires conversion; bytes is safer).
2. Update `enforce_thresholds()` to check:

   * if `max_peak_memory_bytes` exists: `metrics.peak_memory_bytes <= cap`.
3. Ensure the existing slack-factor scaling applies consistently (so `EXCEL_DIFF_PERF_SLACK_FACTOR` scales the cap, just like time caps). This keeps CI stable across runner variance.

**Implementation steps (e2e perf):**

1. Add the same key to `scripts/export_e2e_metrics.py`’s thresholds logic (it already enforces `max_total_time_s`, `max_parse_time_s`, etc.).
2. Gate on memory in addition to time.

**Acceptance criteria**

* A PR that increases peak memory beyond the cap fails the perf workflow, even if it does not regress relative to baseline.
* The output clearly shows which scenario exceeded the cap and by how many bytes (so updating the budget is an intentional action).

---

### A2) Promote a low-similarity benchmark into a release-gating perf suite

Your current perf enforcement uses suites/patterns that focus on `perf_p1_...perf_p5_...` and `perf_50k_...` style runs.
However, the benchmark singled out in the design evaluation for high peak memory is `preflight_low_similarity` (~96MB peak in the referenced artifact). 

To make Phase 3 “real,” you want **a named low-similarity case in the enforced set**.

**Implementation steps**

1. Decide how to integrate:

   * Option 1 (cleanest): rename the test function (and the `log_perf_metric` name) to match the enforced filter and suite patterns (e.g., `perf_preflight_low_similarity`). This ensures it runs in the same harness that the perf workflow already uses.
   * Option 2: keep the name but add a dedicated suite (patterns) for preflight/memory scenarios. This is slightly more machinery but avoids renaming.
2. Add a threshold entry for the scenario in the appropriate suite thresholds map and include `max_peak_memory_bytes`.

**Acceptance criteria**

* Low-similarity memory behavior becomes a permanent regression guardrail in `.github/workflows/perf.yml` (which already runs perf thresholds in CI). 

---

### A3) Budget selection methodology (make it robust, not brittle)

A good memory budget is:

* strict enough to prevent accidental “oops, we allocated 2x”
* loose enough to survive minor allocator/layout variance

**Practical method**

1. Take the current measured `peak_memory_bytes` from:

   * `benchmarks/baselines/*.json` (quick/gate/fullscale) and/or the current benchmark artifact workflow outputs.
2. Set initial cap = `current_peak * 1.20` (or similar), then tighten after Phase 3 optimizations land.
3. Keep the slack-factor env vars as the “escape hatch” for CI runner variability, not as the normal operating mode.

---

## 3) Workstream B: Reduce peak memory in low-similarity regimes by avoiding unnecessary `GridView` construction

This is the heart of Phase 3’s “avoid heavy metadata when bailout threshold is hit.”

### B1) Move preflight decision *before* building full GridViews (for the dissimilar-bailout case)

Current flow in `diff_grids_core`:

* Build `GridView` for old/new
* Run preflight (`should_short_circuit_to_positional(&old_view, &new_view, ...)`)
* Possibly decide to short-circuit and then run positional diff anyway

This ordering guarantees you pay the high-memory cost even when preflight’s job is to say “don’t do expensive stuff.”

**New target structure**

1. **Preflight (lightweight)** on `Grid` inputs to decide:

   * run full pipeline
   * short-circuit to positional
   * near-identical shortcut (optional, see B2)
2. Only if full pipeline is selected:

   * build `GridView`s
   * proceed with move detection / alignment / etc.

**How to implement in this codebase**

* Introduce a new preflight function that operates on `&Grid` rather than `&GridView`, for example:

  * `preflight_decision_from_grids(old: &Grid, new: &Grid, cfg: &DiffConfig, ctx: &mut EmitCtx<...>) -> PreflightDecisionLite`
* Compute row signatures without building `GridView`:

  * the `Grid` already has `compute_row_signature(row)` which hashes row content in column order without constructing per-row cell lists in the dense case, and with bounded allocations in the sparse case.
* Compute similarity and mismatch statistics from these signature vectors (mirroring what `multiset_equal_and_edit_distance` does today with sets/maps).
* If the result is “short-circuit to positional,” immediately run the existing “no views” positional path (already present):

  * `run_positional_diff_with_metrics(ctx, old, new, config, ...)` which uses `positional_diff` on raw grids.

**Key detail**
You will still want to preserve:

* preflight counters in `DiffMetrics` (so you can verify the phase is still captured and tracked)
* the existing config knobs:

  * `preflight_min_rows`
  * `preflight_overlap_ratio_min`
  * `bailout_similarity_threshold`
  * etc.

**Acceptance criteria**

* In the low-similarity short-circuit path, `GridView::from_grid_with_config` is never called.
* The low-similarity perf test shows a substantial drop in `peak_memory_bytes` (the exact target comes from your chosen budget tightening plan).

---

### B2) Optional “bonus” within scope: avoid full GridViews for near-identical as well

Preflight already supports a “near-identical, diff only changed rows (+ context)” decision, but currently that decision still happens after full views are built.

If you want Phase 3 to pay off in the most common real-world case (“one row changed”), you can extend the “preflight-before-views” approach to near-identical too.

**Approach**

* For near-identical, you already know exactly which row indices changed (based on signature comparisons).
* Implement a “diff only these rows” positional path that does *not* require `RowView`:

  * a new function like `positional_diff_for_rows(old: &Grid, new: &Grid, rows: &[u32], ...)` that loops `(row, col)` and emits edits, bounded to those rows.
* Use the existing “context rows” concept from config (`preflight_context_rows`) to expand the set of rows to compare.

**Acceptance criteria**

* For near-identical sheets above `preflight_min_rows`, peak memory does not scale with total cell count (because full views are avoided).

---

### B3) Add “early exit” inside preflight itself (reduce preflight’s own memory churn)

Even after moving preflight earlier, you can reduce its cost:

Today `multiset_equal_and_edit_distance` constructs:

* `HashSet<RowSignature>` for each side
* a delta `HashMap<RowSignature, i32>` to estimate edit distance
* in-order mismatch scanning

**Low-similarity optimization**

* Compute the Jaccard/overlap ratio *first*.
* If similarity is already below `bailout_similarity_threshold`, return `ShortCircuitToPositional` immediately, skipping delta-map/edit-distance work.

This is exactly in line with “early exits / avoid heavy metadata when bailout threshold is hit.”

**Acceptance criteria**

* In low similarity, preflight allocates less (measurable via `peak_memory_bytes` and/or by adding a debug counter for allocations in perf-metrics builds).

---

### B4) Memory-focused improvements in GridView construction (when you *do* need views)

Even after preflight changes, full pipeline cases still build `GridView`, and on huge dense sheets that’s inherently heavy. There are two grounded levers here:

#### B4.1) Avoid the memory-expensive parallel column-meta builder on large sheets

`GridView` can build column metadata using a parallel path that constructs `col_cells: Vec<Vec<&Cell>>`, which is effectively another per-cell pointer index.

That’s an obvious peak-memory amplifier for big sheets.

**Plan**

* Add a config knob (or a hardening-derived decision) that forces the sequential column meta builder (`build_col_meta_sequential`) when memory is constrained, even if parallel is enabled.
* Tie this to:

  * `max_memory_mb` being set, or
  * “estimated gridview bytes” nearing the budget threshold (you already estimate these).

**Acceptance criteria**

* In large, dense cases, `peak_memory_bytes` drops without correctness changes.

#### B4.2) Reduce Vec overallocation for row cell lists

Right now, row cell vectors grow dynamically as cells are pushed; the estimator even bakes in a 25% overhead factor (`* 5 / 4`). 

**Plan**

* Use a two-pass build for row views:

  1. pass 1: count cells per row
  2. allocate each row `Vec` with exact capacity
  3. pass 2: fill
* This trades extra iteration for lower peak heap use and less allocator churn.

**Acceptance criteria**

* On dense sheets, the `GridView` contribution to peak drops measurably (especially when both old/new views are constructed).

---

## 4) Workstream C: WASM budgets that actually fail CI when exceeded

Phase 3 explicitly calls for explicit WASM budgets and a WASM-targeted perf/memory test path.
Your repo already enforces WASM *size* budgets in CI (`wasm.yml`), but not runtime memory.

### C1) Decide the budget surface: “explicit WASM budgets” means two complementary things

1. **Runtime memory budget gates in CI** (measured, not estimated)
2. **In-product default guardrails** to reduce OOM risk

You already have the “in-product guardrail” mechanism: `DiffConfig.max_memory_mb` + `HardeningController::memory_guard_or_warn`.

What’s missing is runtime verification and a default WASM setting.

---

### C2) Set a default `max_memory_mb` for WASM entry points

Right now `DiffConfig::default()` leaves `max_memory_mb = None`. 

**Plan (minimal blast radius)**

* In the WASM crate entry points (e.g., the exported `diff_files_json`), explicitly set:

  * `config.max_memory_mb = Some(<wasm_default_mb>)`
* Choose a value consistent with browser constraints + UI overhead (and document it).

This leverages the estimate-based guardrails you already wrote, which are specifically designed to fall back to positional diff and emit warnings rather than crashing.

**Acceptance criteria**

* Very large diffs that would previously risk OOM in WASM instead produce a warning + partial/positional output (depending on configured behavior), consistently.

---

### C3) Add a WASM runtime memory harness that measures linear memory usage

WASM memory measurement is different from native:

* You can’t use the `CountingAllocator` approach (it’s `#[cfg(not(target_arch="wasm32"))]`). 
* But you *can* measure WASM linear memory size (the `WebAssembly.Memory.buffer.byteLength`), which is the real “will the browser OOM” budget.

**Plan**

1. Create a Node-based harness (fits your existing CI patterns; you already run Node in `web_ui_tests.yml`).
2. The harness should:

   * import the built wasm package
   * capture `memory.buffer.byteLength` before
   * run one or more representative diff workloads
   * capture `byteLength` after (since wasm memory grows monotonically, “after” is effectively the peak)
   * fail if above budget

**Workload selection**
Pick workloads that specifically target Phase 3 risks:

* low similarity dense-ish grids (to validate the “don’t build heavy metadata when bailing” changes)
* a moderate near-identical scenario (to validate the near-identical short-circuit if implemented)

**How to supply workloads**
Two grounded options:

* Option A (synthetic, deterministic, no fixtures): add a small exported wasm function solely for benchmarking that constructs two grids and runs `diff_grids_core` / workbook diff with a no-op sink.
* Option B (fixture-based): generate a couple of fixture XLSX files and diff them, but this requires bringing fixture generation into wasm CI (more moving parts).

Given your Phase 3 text explicitly suggests a JS harness and you already have rich synthetic grid creators in tests, Option A is typically the fastest to make robust.

**Acceptance criteria**

* A new CI step in `.github/workflows/wasm.yml` runs the harness and fails if memory exceeds budget (analogous to size budgets).

---

### C4) Track and tighten: treat WASM budget as a “living cap”

Once you land the low-similarity improvements, you should:

* record the observed WASM linear memory under your harness
* set the budget to a small margin above it
* tighten over time, rather than setting a huge “never fail” cap

This is exactly the “named automated invariant” concept, but for WASM.

---

## 5) Workstream D: Regression tests specifically for “low similarity peak behavior”

Phase 3 explicitly calls out “add regression tests for low-similarity peak behavior.”

You want two layers:

### D1) CI perf gate (absolute cap) for one representative low-similarity scenario

This is covered by Workstream A (absolute `max_peak_memory_bytes`).

### D2) Targeted unit/integration test that asserts the code path avoids view construction

Budgets alone tell you **something got worse**; they don’t tell you **why**.

Add a deterministic test that checks *behavioral intent*, for example:

* In the dissimilar preflight short-circuit path, confirm the “view build” phase time is zero or not started, or record a debug flag/counter in perf-metrics builds.
* Alternatively, expose a tiny hook in `GridView::from_grid_with_config` under `cfg(test)` that increments a counter, and assert it remains zero for the low-similarity preflight shortcut.

This is consistent with how your tests already assert phase timings are zero to verify bailouts (e.g., `move_detection_time_ms == 0`, `alignment_time_ms == 0`).

**Acceptance criteria**

* A test fails if someone accidentally reorders the pipeline to build views before preflight again.

---

## 6) Rollout sequencing (minimize risk while keeping momentum)

A safe order that avoids “everything changed at once”:

1. **Add enforcement hooks first** (A1 + C3 harness scaffolding without strict caps):

   * Implement the threshold key + reporting, but initially set caps very high (or “warn-only” via CI artifacts).
2. **Promote a low-similarity case into the enforced suite** (A2):

   * Make sure it runs reliably in CI.
3. **Refactor preflight ordering** (B1 + B3):

   * This should yield immediate memory wins and make the low-similarity case cheaper.
4. **Tighten native budget**:

   * Convert from “very high cap” to “real cap.”
5. **Add WASM default max_memory_mb** (C2) and enforce WASM budgets (C3/C4):

   * Once the harness is stable, make it a hard gate.
6. **Optional deeper GridView memory improvements** (B4):

   * These are worth it if the budgets show you still have headroom issues in full-pipeline cases.

---

## 7) Concrete “Definition of Done” checklist for Phase 3

A Phase 3 PR series is “done” when all of the following are true:

* [ ] `check_perf_thresholds.py` supports an absolute peak-memory cap per scenario and enforces it. 
* [ ] At least one low-similarity benchmark is included in the perf workflow and has an explicit peak-memory cap.
* [ ] Low-similarity preflight short-circuit happens before building full `GridView`s (validated by a deterministic test).
* [ ] WASM CI includes a runtime memory harness that fails on budget exceedance, analogous to existing wasm size budgets.
* [ ] WASM entry points set an explicit default `max_memory_mb` (or clearly document why they do not).
* [ ] The design-evaluation concern (“low similarity: metadata outweighs benefit; WASM budgets explicitly tested”) is concretely resolved by measurable reductions and hard gates.
