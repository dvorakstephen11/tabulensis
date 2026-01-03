## Phase 2 implementation plan: turn full-scale perf + resource-constraint tests into merge-blocking gates

Phase 2 in `13_phase_plan.md` has two concrete outcomes: (1) take at least one representative 50k/large-file scenario and make it run on every PR with enforced thresholds, and (2) make sure the threshold tooling covers *parse + diff* and yields a clear pass/fail signal for regressions. 

Below is an expanded implementation plan that is explicitly grounded in the current code + CI layout (tests, scripts, fixtures, and GitHub workflows) described in the attached context files.

---

# 1) What exists today (ground truth)

### 1.1 Perf checks that already run on PRs

There is a PR-triggered workflow `.github/workflows/perf.yml` (“Performance Regression”) that builds `core` with the `perf-metrics` feature and runs `python scripts/check_perf_thresholds.py` (quick suite), exporting `benchmarks/latest_quick.csv` and `benchmarks/latest_quick.json` as artifacts. 

The quick suite thresholds are defined in `scripts/check_perf_thresholds.py` as `QUICK_THRESHOLDS` for `perf_p1_*` through `perf_p5_*`, with a suite timeout of 120s (`PERF_TEST_TIMEOUT_SECONDS`).

### 1.2 Full-scale perf tests exist but are not PR gates

There is a scheduled workflow `.github/workflows/perf_fullscale.yml` (“Performance Full Scale”), weekly, that runs `python scripts/check_perf_thresholds.py --full-scale` and uploads `benchmarks/latest_fullscale.*`. 

In code, the 50k tests live in `core/tests/perf_large_grid_tests.rs` and are marked `#[ignore = "Long-running test: run with cargo test --features perf-metrics -- --ignored"]`, e.g. `perf_50k_dense_single_edit`, `perf_50k_completely_different`, etc.

`scripts/check_perf_thresholds.py`’s `--full-scale` mode runs ignored tests via `cargo test ... -- --ignored ...` and applies the `FULL_SCALE_THRESHOLDS` dictionary (currently time-only caps).

### 1.3 End-to-end parse+diff perf tests exist but are only exported, not threshold-gated

There is an end-to-end perf integration test file `core/tests/e2e_perf_workbook_open.rs` (behind `#![cfg(feature = "perf-metrics")]`) with ignored tests like `e2e_p1_dense_single_edit`…`e2e_p5_identical`. They open `.xlsx` fixtures and run `diff_streaming`, and they assert `parse_time_ms > 0` and that `diff_time_ms` is consistent.

Fixtures for those e2e tests are generated from `fixtures/manifest_perf_e2e.yaml` (50k rows; varying modes like dense/noise/repetitive/sparse/identical). 

The scheduled workflow `.github/workflows/perf_e2e.yml` generates fixtures and then runs `python scripts/export_e2e_metrics.py --skip-fixtures --export-csv benchmarks/latest_e2e.csv`, uploading results artifacts — but it does not enforce pass/fail thresholds for regressions.

### 1.4 Why Phase 2 matters in this repo’s current state

Both the design evaluation and completion analysis call out that perf checks need to move from “signals” to “gates,” especially at scale and including parse+diff.

---

# 2) Target end state (definition of done)

Phase 2 is complete when all of the below are true:

1. **PR gate:** At least one representative **50k / large-file** scenario runs on every PR (not just on weekly schedules), and the workflow **fails the PR** if it breaches thresholds.
2. **Full-scale still exists:** The remaining heavy 50k scenarios remain scheduled (ideally nightly) and still produce artifacts (for auditing + longer-horizon regressions).
3. **Parse+diff is threshold-checked:** There is tooling that runs parse+diff tests and returns a **clear pass/fail** (non-zero exit code) on regression, not just exports metrics.
4. **Local reproducibility:** Developers have a clear, documented way to reproduce the gated perf checks locally (same test(s), same scripts).

---

# 3) Workstream A: pick the “always-run” 50k sentinel test(s)

### 3.1 Selection criteria (what makes a good PR gate)

A PR gate test should:

* Exercise *real* worst-case-ish code paths that historically regress (preflight / bailout decisions, diff engine hot loops, op emission)
* Be deterministic (stable input generation / stable expected invariants)
* Complete fast enough to keep PR CI acceptable
* Have a meaningful “resource constraint” posture (time and at least baseline memory checks), even if deeper memory budgeting is Phase 3

### 3.2 Best initial sentinel: `perf_50k_dense_single_edit`

`perf_50k_dense_single_edit` is a strong first PR gate because:

* It is truly large (50k x 100 grid) and should remain performant if preflight bailouts are working.
* The test asserts “no warnings,” asserts a `CellEdited` op exists, and asserts move detection and alignment are skipped (`move_detection_time_ms == 0`, `alignment_time_ms == 0`), making it a strong canary for accidentally re-enabling expensive logic at scale. 
* It already has an enforced wall-clock bound in-test (`total_time_ms < 30000`) and has a corresponding cap in `FULL_SCALE_THRESHOLDS` (`max_time_s: 30`).

### 3.3 Optional second sentinel (only after stability is proven)

Add one “harder” case as a second PR gate if CI time allows:

* `perf_50k_completely_different` (forces dissimilar bailout; should remain bounded and skip move/alignment).

This gives you a “small-diff” and a “big-diff” representative guardrail.

---

# 4) Workstream B: introduce a first-class “gate suite” in `check_perf_thresholds.py`

Right now `check_perf_thresholds.py` only understands two modes: quick (PR) and full-scale (scheduled).

Phase 2 needs a third mode: **gate** — a tiny set of representative 50k tests that are always-run on PRs.

### 4.1 Add a gate suite concept (CLI + internal config)

Update `scripts/check_perf_thresholds.py` to support:

* **CLI:**

  * `--suite quick|gate|full-scale` (or `--gate` as a boolean; suite is more extensible)
  * Optional: `--test-target perf_large_grid_tests` (defaults to current behavior), so you can avoid compiling/running unrelated integration test binaries in this workflow.

* **Threshold config:**

  * `GATE_THRESHOLDS = { "perf_50k_dense_single_edit": { "max_time_s": 30 } }` (start with one test)
  * `GATE_PATTERNS = ("perf_50k_dense_single_edit",)` or an explicit list match (recommended for precision)

* **Timeout config:**

  * Gate suite should use a higher timeout than quick but far lower than “all full-scale.” For example, a separate `GATE_TIMEOUT_SECONDS` so you don’t have to inflate the quick suite 120s timeout. (This is a key step to keep PR CI predictable.)

### 4.2 Make cargo invocation targeted (reduces CI cost + noise)

Today the script runs cargo like:

* quick: `cargo test --release --features perf-metrics perf_ -- --nocapture --test-threads=1`
* full-scale: same but with `--ignored`

For gate mode, make it narrower:

* Prefer running only the integration test target containing the 50k sentinel(s):

  * `cargo test --release --features perf-metrics --test perf_large_grid_tests perf_50k_dense_single_edit -- --ignored --nocapture --test-threads=1`
* This leverages the fact that `export_perf_metrics.py` already uses `--test perf_large_grid_tests` for perf runs (so this is aligned with existing scripting patterns).

### 4.3 Threshold enforcement mechanics (keep it simple, then deepen)

The existing script already:

* parses `PERF_METRIC ... key=value` lines
* checks absolute `max_time_s` caps
* checks baseline regression for `total_time_ms` and `peak_memory_bytes` if a baseline exists
* supports a global slack multiplier via `EXCEL_DIFF_PERF_SLACK_FACTOR`

For Phase 2, do the following:

1. **Make absolute caps mandatory** for gate tests (already supported).
2. **Ensure baseline checks actually run** for gate suite by establishing a stable baseline source (see Workstream D).
3. Keep suite output readable (it already prints “Passed suite tests” and threshold check summary). 

### 4.4 Make “resource constraint” explicit (without stealing Phase 3)

Phase 3 is where “peak memory budgets become first-class.” But Phase 2 should still ensure gate tests are RC-relevant by:

* Ensuring baseline regression checks include `peak_memory_bytes` for gate suite (script already checks this when a baseline exists).
* Keeping the existing in-test assertions that guard against unexpected slow paths (e.g., move/alignment times must be 0 for the dense single edit). 

Optionally (if you want a small Phase 2 add-on that pays off fast): extend threshold dictionaries to allow `max_peak_memory_bytes` and enforce it for gate tests only. This is “Phase 2.5”: it gives you an actual RC gate without requiring the full Phase 3 budgeting strategy.

---

# 5) Workstream C: wire the gate suite into PR CI (`.github/workflows/perf.yml`)

### 5.1 Add a second required step: “gate suite”

`.github/workflows/perf.yml` currently runs only the quick suite. 

Update it to run:

1. **Quick suite** (unchanged)
2. **Gate suite** (new): calls `check_perf_thresholds.py --suite gate` (or equivalent), exports `benchmarks/latest_gate.csv/json`, uploads as artifacts.

Why this structure works well:

* quick suite stays fast and stable
* gate suite is the single large sentinel(s)
* failures are clearly attributable (“quick failed” vs “gate failed”)

### 5.2 Runner variance mitigation (make failures actionable, not flaky)

GitHub hosted runners vary. The script already supports a slack multiplier via `EXCEL_DIFF_PERF_SLACK_FACTOR`.

Plan:

* Set `EXCEL_DIFF_PERF_SLACK_FACTOR` in the workflow environment for **gate suite only**, initially conservative (e.g., 1.2–1.3) and tighten over time once you see stability in CI.
* Keep per-test env overrides for quick suite as-is; for gate suite, consider adding similar env overrides if you anticipate tuning.

### 5.3 Make it a true “merge gate”

The code change alone makes the workflow fail on regression, but making it a *merge gate* usually also requires:

* Branch protection rules requiring the `perf-regression` check to pass before merge.

This is outside the repository code, but it’s necessary to fulfill the “real gates” spirit of Phase 2. 

---

# 6) Workstream D: baseline strategy so regressions are detected as regressions (not just “under 30s”)

Absolute time caps (30s) prevent catastrophic blowups, but they won’t catch “20% slower every week” until it’s too late. Phase 2 explicitly wants a “clear pass/fail signal for regressions.” 

### 6.1 Current baseline behavior

`check_perf_thresholds.py` will load a baseline JSON from `benchmarks/results/*.json` (quick vs full-scale distinguished by `full_scale`) and compare against it with slack (`BASELINE_SLACK_QUICK` / `BASELINE_SLACK_FULL`). But if there is no baseline file present in the repo, it only warns and skips baseline checks.

### 6.2 Make baseline deterministic and repo-local

Implement one of these approaches (ordered by robustness):

**Option A (recommended): add pinned baseline files in-repo**

* Create a new directory: `benchmarks/baselines/`
* Commit:

  * `benchmarks/baselines/quick.json`
  * `benchmarks/baselines/gate.json`
  * (optionally) `benchmarks/baselines/full-scale.json`
* Extend `check_perf_thresholds.py` to accept `--baseline PATH` and use that instead of scanning `benchmarks/results`.

This avoids the “do we have any prior results in this checkout?” problem entirely.

**Option B: keep using `benchmarks/results`, but commit a single baseline result file**

* Run `scripts/export_perf_metrics.py` locally to produce a timestamped JSON in `benchmarks/results/`, then commit that file.
* Gate suite and quick suite will then have a baseline available at PR time.

Option A is cleaner over time because it avoids accumulating lots of timestamped JSON in version control.

### 6.3 Establish the baseline update workflow (human-friendly)

Define and document a simple “baseline bump” procedure:

1. On main (or a known-good commit), run:

   * `python scripts/export_perf_metrics.py` (quick)
   * `python scripts/export_perf_metrics.py --full-scale` (optional)
2. Copy the relevant results into the pinned baseline file(s).
3. Validate with `python scripts/check_perf_thresholds.py --suite ... --baseline ...` locally.
4. Commit baseline bump in a PR with a short justification (e.g., “performance improved due to X” or “acceptable variance due to Y”).

There is already a script `scripts/compare_perf_results.py` that can make this review concrete (percent deltas, regression lists). Use it in the baseline bump PR description. 

---

# 7) Workstream E: add threshold-gated parse+diff perf (E2E) with clear pass/fail

Phase 2 explicitly requires that threshold tooling covers *parse + diff*, not just the in-memory diff tests.

### 7.1 Choose the mechanism: extend e2e tooling vs unify into `check_perf_thresholds.py`

You have two realistic, codebase-aligned paths:

#### Path 1: Extend `export_e2e_metrics.py` to also enforce thresholds (minimal disruption)

`export_e2e_metrics.py` already:

* (optionally) installs the fixture generator + generates fixtures
* runs `cargo test --release --features perf-metrics --test e2e_perf_workbook_open ... -- --ignored`
* parses `PERF_METRIC`
* writes `benchmarks/latest_e2e.json` plus timestamped results under `benchmarks/results_e2e/`

Add to this script:

* `E2E_THRESHOLDS` with time caps for `e2e_p1_dense`, `e2e_p2_noise`, etc (start with 1–2 cases if needed)
* Optional baseline support similar to `check_perf_thresholds.py` (either pinned baseline file or scanning a baseline dir)
* Exit non-zero on regression (time cap exceeded, baseline regression exceeded, missing metrics, etc.)

Then change `.github/workflows/perf_e2e.yml` from “Export e2e metrics” to “Export + enforce e2e thresholds” so the workflow is a *true gate* for scheduled runs (failures are not just metrics).

#### Path 2: Expand `check_perf_thresholds.py` to support an `--e2e`/`--suite e2e` mode (single-tool consistency)

This yields one canonical “threshold checker,” but it requires it to know how to:

* generate fixtures (or require them to exist)
* run the correct test binary (`--test e2e_perf_workbook_open`)
* interpret / threshold parse vs diff time (optional)

It’s doable, but Path 1 is less invasive and already matches the existing separation of responsibilities (“check thresholds” vs “export metrics”).

### 7.2 Make parse+diff thresholds meaningful (not just “total time”)

The e2e tests emit metrics with both `parse_time_ms` and `diff_time_ms`, and they assert parse is non-zero.

So your thresholding should be layered:

* **Absolute caps:**

  * `max_total_time_s`
  * (optional but recommended) `max_parse_time_s` and `max_diff_time_s`
* **Regression checks:**

  * baseline delta on `total_time_ms`
  * baseline delta on `parse_time_ms` (important: parsing regressions are easy to miss otherwise)
  * baseline delta on `peak_memory_bytes`

This directly satisfies the “covers parse+diff” requirement, and it makes failures actionable (“parsing regressed” vs “diff regressed”).

### 7.3 Keep the always-run PR cost under control

Do **not** make the entire e2e suite a PR-required check immediately; fixture generation can be expensive (the manifest includes 50k-row sheets).

Instead:

* Keep e2e threshold checking **scheduled** (nightly or weekly), but ensure it is pass/fail.
* If you want some parse+diff gating on PRs, add a *single* “e2e sentinel” job that:

  * generates only the `e2e_p1_dense` fixtures (or a smaller dedicated manifest just for the sentinel)
  * runs just the `e2e_p1_dense` case
  * enforces a loose threshold

This can be a follow-on after gate stability is proven.

---

# 8) Workstream F: re-balance scheduled full-scale coverage (weekly → nightly, and/or split suites)

The plan explicitly allows moving the rest to scheduled/nightly “if needed.” 

### 8.1 Convert weekly schedules to nightly (recommended)

Both `perf_fullscale.yml` and `perf_e2e.yml` are currently weekly cron jobs.

Update the cron schedules to nightly (or at least more frequent than weekly). This shortens the feedback loop for regressions that are too expensive to gate on every PR.

### 8.2 Split the scheduled suites by purpose

In `core/tests/perf_large_grid_tests.rs`, there is at least one “coverage” style full-scale test (`perf_50k_alignment_block_move`) that is about ensuring alignment/move detection runs (expects non-zero alignment or move detection time), not about being fast. 

Plan:

* Keep “coverage” cases scheduled (nightly), and give them explicit thresholds once you’ve measured stable ranges.
* Keep “RC / must always be bounded” cases as PR gates.

This avoids conflating “should run and produce reasonable metrics” with “must be super fast.”

---

# 9) Developer workflow + documentation (so gates don’t feel mysterious)

Create a single developer-facing doc (e.g., `docs/perf.md` or a README section) that includes:

### 9.1 Local commands (exactly reproducible)

* Quick suite:

  * `python scripts/check_perf_thresholds.py --suite quick --export-json benchmarks/latest_quick.json`
* Gate suite:

  * `python scripts/check_perf_thresholds.py --suite gate --export-json benchmarks/latest_gate.json`
* Full-scale scheduled suite:

  * `python scripts/check_perf_thresholds.py --suite full-scale --export-json benchmarks/latest_fullscale.json`
* E2E parse+diff threshold run:

  * `python scripts/export_e2e_metrics.py ...` (and the new threshold flag / mode you add)

Tie these commands directly to the same scripts and test targets used in CI.

### 9.2 Triage playbook when a gate fails

Document the steps:

* Rerun locally with `--nocapture` (already in the script cargo invocation) 
* Inspect `benchmarks/latest_gate.json` fields (e.g., `total_time_ms`, `peak_memory_bytes`, `move_detection_time_ms`, `alignment_time_ms`) to see what regressed
* If the change was expected (algorithm tradeoff), update thresholds/baseline with rationale; otherwise optimize/fix.

---

# 10) Validation checklist (how you know Phase 2 is *actually* delivered)

Run through these checks before calling Phase 2 “done”:

1. **PR workflow shows two perf steps:** quick + gate, both producing artifacts, both failing the workflow on breach.
2. **Gate suite truly runs a 50k scenario**: confirm the output includes `PERF_METRIC perf_50k_dense_single_edit ... rows_processed=50000 ...`.
3. **Regression is detected, not just catastrophic failures:** change the code artificially to slow the gate test (e.g., disable preflight bailout) and confirm:

   * thresholds fail
   * baseline regression (if enabled) fails
4. **E2E perf workflow becomes pass/fail:** induce a regression (or tighten threshold temporarily) and confirm `.github/workflows/perf_e2e.yml` fails the job, not just exports a CSV.
5. **Docs exist and are accurate:** a dev can reproduce the CI gate locally with one command per suite.

---

# 11) Concrete deliverables list (what to change, where)

To make this implementation plan immediately actionable, here is the “touch list”:

### Code + scripts

* `scripts/check_perf_thresholds.py`

  * add gate suite CLI + config
  * add targeted cargo invocation for gate mode
  * add baseline path support (recommended)
  * ensure output explicitly prints suite name and the exact tests enforced
* `scripts/export_e2e_metrics.py`

  * add threshold enforcement (or add a sibling script `check_e2e_thresholds.py`)
* (optional) `benchmarks/baselines/*` pinned baselines, committed

### CI workflows

* `.github/workflows/perf.yml`

  * add the gate suite run + artifact upload 
* `.github/workflows/perf_fullscale.yml`

  * move to nightly and/or clarify it’s “extended coverage,” not the only place 50k runs happen
* `.github/workflows/perf_e2e.yml`

  * keep fixture generation, but change the script invocation so the workflow fails on regression

### Tests (minimal changes)

* Prefer to **keep** the full-scale tests `#[ignore]` and selectively run them with `--ignored` in gate/full-scale modes (this matches existing conventions in both perf and e2e tests).

---

If you want, I can also draft a “ready to paste” checklist-format PR plan (PR 1: gate suite plumbing, PR 2: CI wiring, PR 3: e2e thresholds + nightly schedules, PR 4: baselines + docs) — but the sections above already specify the exact files, hooks, and test names that Phase 2 must wire together in *this* repo.
