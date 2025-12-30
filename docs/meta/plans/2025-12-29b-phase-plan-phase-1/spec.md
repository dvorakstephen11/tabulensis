## Phase 1 implementation plan: End‑to‑end performance benchmarks (open + parse + diff)

Phase 1 (per `13_phase_plan.md`) is to add **true end‑to‑end** benchmarks that cover **opening + parsing + diffing real workbook artifacts**, and to **version** those results alongside the existing benchmark JSON so performance statements remain “resource‑truthful” over time. 

This plan is intentionally grounded in what the repo already does today (perf-metrics plumbing, perf scripts, fixture generator, CI workflows), while filling the specific gap called out in the evaluation notes: current benchmark outputs largely miss parse cost because they benchmark IR→diff, not `WorkbookPackage::open(...) + diff`.

---

## 1) What “done” means for Phase 1

### Outcomes

1. **A repeatable benchmark path that includes I/O + parse cost**

   * Benchmarks execute the same parsing entry point used by the product: `WorkbookPackage::open(...)` (which already times parsing when `perf-metrics` is enabled).
   * Benchmarks then run a diff (preferably streaming for large artifacts) and collect `DiffMetrics`.

2. **Metrics become internally consistent for end‑to‑end runs**

   * For end‑to‑end runs, `parse_time_ms` must be non‑zero on real artifacts and must be a *subset* of `total_time_ms` (so `diff_time_ms = total_time_ms - parse_time_ms` remains meaningful). The codebase already encodes this “diff_time is derived” expectation in the perf metrics tests. 

3. **Results are versioned and discoverable**

   * A new “latest” JSON (and optionally CSV) sits alongside the existing quick/fullscale outputs (`benchmarks/latest_quick.json`, `benchmarks/latest_fullscale.json`).
   * Historical runs are saved (like `benchmarks/results/...` today) but without breaking existing baseline selection for quick/fullscale suites.

### Acceptance criteria

* You can run one command locally and get:

  * A `benchmarks/latest_e2e.json` (new) containing end‑to‑end metrics where `parse_time_ms > 0` for the large artifacts.
  * At least one timestamped JSON saved into a dedicated results directory (so trends can be built), using the same schema style as existing exporters.
* CI has a workflow that:

  * generates the needed fixtures,
  * runs the end‑to‑end suite,
  * uploads `latest_e2e.json` as an artifact.

---

## 2) Reality check: what exists today (and why it’s not enough)

### The current perf suite is mostly IR→diff

The existing perf tests build large `Grid`s in memory, wrap them with `WorkbookPackage::from(...)`, and diff them—so there is **no `WorkbookPackage::open(...)` parse cost** in those measurements.
This matches the evaluation note that `parse_time_ms` tends to be `0` in exported perf JSON because the measured workloads largely skip file parsing.

### `WorkbookPackage::open` already measures parse time (good)

When `perf-metrics` is enabled, `WorkbookPackage` stores `parse_time_ms`, measured inside `open(...)`.
So the core missing piece is not “how to time parsing”, it’s: **ensuring the benchmark path actually opens real artifacts**, and ensuring we **account for open/parse time correctly** in final `DiffMetrics`.

### There is already a hook that tries to add open parse time… but it’s incomplete

`apply_parse_metrics` adds `old_pkg.parse_time_ms + new_pkg.parse_time_ms` into `DiffMetrics.parse_time_ms` and then recomputes `diff_time_ms` as `total - parse`. 
However, it currently **does not add that parse time into `total_time_ms`**. That makes the “parse is a subset of total” invariant false for end‑to‑end runs and can drive `diff_time_ms` to zero via `saturating_sub`.
Phase 1 needs to fix this to make the exported numbers meaningful.

### Benchmark export + CI pipeline exists, but it’s oriented around the IR-based perf tests

* Quick perf workflow exports `benchmarks/latest_quick.json` via `scripts/check_perf_thresholds.py`.
* Fullscale workflow exports `benchmarks/latest_fullscale.json`.
* There’s also `scripts/export_perf_metrics.py` which writes timestamped JSON into `benchmarks/results/` for historical tracking. 

Phase 1 should **reuse** this style (same JSON structure, same parsing of `PERF_METRIC` lines), but keep end‑to‑end history separate so it doesn’t disrupt quick/fullscale baseline logic.

---

## 3) Workstream A: Fix metrics accounting so end‑to‑end numbers are coherent

### A1. Make end‑to‑end `total_time_ms` actually include open/parse time

**Change target:** `core/src/package.rs` function `apply_parse_metrics`. 

**Current behavior (problem):**

* Adds open parse time into `m.parse_time_ms`
* Leaves `m.total_time_ms` unchanged
* Recomputes `m.diff_time_ms = m.total_time_ms - m.parse_time_ms` (often collapses to 0 for real files) 

**Desired behavior:**

* Let `added = old_pkg.parse_time_ms + new_pkg.parse_time_ms`
* Add `added` to **both**:

  * `m.parse_time_ms`
  * `m.total_time_ms`
* Then recompute:

  * `m.diff_time_ms = m.total_time_ms - m.parse_time_ms`
* Result:

  * `diff_time_ms` stays the same as before (because you add `added` to both total and parse),
  * `total_time_ms` becomes truly “end‑to‑end”,
  * the invariant tested in `perf.rs` (“diff time derived from total minus parse”) remains conceptually correct.

### A2. Add a regression test specifically for this path

Add a small unit/integration test that:

* creates two `WorkbookPackage`s where `parse_time_ms` is forced non‑zero (either by opening a small real fixture when `perf-metrics` is enabled, or by constructing a package-like wrapper in a test-only helper),
* runs `diff_streaming` or `diff_with_progress`,
* asserts:

  * `metrics.total_time_ms >= metrics.parse_time_ms`
  * `metrics.diff_time_ms == metrics.total_time_ms - metrics.parse_time_ms` (or consistent with the derivation rule).

This test protects Phase 1’s intent from future refactors.

### A3. Ensure CLI metrics output benefits automatically

The CLI can write metrics JSON (`--metrics-json`) from both report and streaming summary. 
Since the CLI ultimately consumes the same `DiffMetrics` coming from the core package diff path, fixing `apply_parse_metrics` makes CLI-exported metrics truthful without adding special CLI-only timing logic.

---

## 4) Workstream B: Define the “real artifact” benchmark corpus (fixtures)

### B1. Reuse the existing large perf fixture generators

The repo already has large, deterministic fixture scenarios in `fixtures/manifest.yaml` using `LargeGridGenerator` (`perf_large`) for:

* P1 dense (`grid_large_dense.xlsx`)
* P2 noise (`grid_large_noise.xlsx`)
* P3 adversarial repetitive (`grid_adversarial_repetitive.xlsx`)
* P4 sparse 99% blank (`grid_99_percent_blank.xlsx`)
* P5 identical (`grid_identical.xlsx`)

And the generator is implemented in Python using `openpyxl` write-only mode (so creating large artifacts is feasible). 

### B2. Add a *benchmark-only* manifest so CI doesn’t generate everything

CI currently generates fixtures using `fixtures/manifest_cli_tests.yaml`, not the full manifest. 
For end‑to‑end perf, introduce a new manifest, e.g.:

* `fixtures/manifest_perf_e2e.yaml`

This manifest should generate only:

* the large benchmark artifacts needed by the end‑to‑end suite,
* plus any smaller “smoke” artifacts you want for fast local validation.

This keeps the scheduled workflow predictable and avoids bloating standard CI.

### B3. Ensure you have *pairs* suitable for diff without producing massive op output

If you diff “noise vs noise” with different seeds, you can create an enormous diff output (bad for memory and runtime). For end‑to‑end benchmarks you usually want:

* **large parse cost**
* **large scan/diff cost**
* **small output** (so the benchmark measures engine work, not serialization or storing millions of ops)

Best practice here is to generate pairs that differ by a very small number of cells (the same way the IR-based perf suite models “single cell edit”, etc.).

#### Proposed approach (minimal codebase disruption)

Extend `LargeGridGenerator` to optionally apply a single targeted edit while streaming rows:

* New optional args in the generator (names illustrative):

  * `edit_row` (0-based or 1-based, pick one and document)
  * `edit_col`
  * `edit_value`
* When generating row `r`, if it matches `edit_row`, override the one column value.

Because the generator already builds each row in a loop, this is a straightforward conditional, and it avoids needing to clone huge workbooks in memory. 

Then, in `manifest_perf_e2e.yaml`, define each pair as **two scenarios** with identical base parameters:

* `e2e_p1_dense_a.xlsx` : no edit
* `e2e_p1_dense_b.xlsx` : edit one cell
  (Using two scenarios rather than “two outputs in one scenario” avoids RNG-state coupling in the current generator structure.)

Repeat for:

* dense
* noise (same seed + single override)
* repetitive
* sparse
* identical (two identical outputs, no edit)

### B4. Record artifact sizes (optional but strongly recommended)

To make “100MB-class” performance claims defensible, benchmarks should emit file sizes alongside timing.
You can do this without changing core structs by printing additional numeric fields on the `PERF_METRIC` line, e.g.:

* `old_bytes=...`
* `new_bytes=...`
* `total_input_bytes=...`

The existing parsers already ingest numeric `key=value` tokens.

---

## 5) Workstream C: Implement an end‑to‑end benchmark harness in Rust

### C1. Prefer “ignored tests” as the benchmark harness (not Criterion) for this repo

Why:

* The repo already has a mature pipeline for turning test output into JSON (`PERF_METRIC` parsing, export scripts, CI workflows).
* Criterion benches exist but are currently IR-centric and not integrated into the existing JSON export path.

### C2. Add a new integration test binary dedicated to end‑to‑end suite

Example file (name illustrative):

* `core/tests/e2e_perf_workbook_open.rs`

Key requirements:

* Tests should be `#[ignore]` so they don’t run on every `cargo test`. (They run only in scheduled perf workflows or when explicitly requested.)
* Avoid naming tests with substring `perf_` so `scripts/check_perf_thresholds.py` (which runs `cargo test ... perf_ ...`) doesn’t accidentally execute them. 

  * Use a prefix like `e2e_...` for test names and the metric names you print.

### C3. Use the existing fixture path helper

The integration test suite already uses a helper that resolves `../fixtures/generated/<name>` from `CARGO_MANIFEST_DIR`.
Reuse that so benchmarks run the same way as existing integration tests.

### C4. Diff using the streaming path for large artifacts

For large artifacts, prefer `diff_streaming`:

* There is already a streaming example showing `WorkbookPackage::open(...)` + `JsonLinesSink` + `diff_streaming`.
* For benchmarking, replace `JsonLinesSink` with a sink that discards output but still exercises emit calls (e.g., `CallbackSink`), so you measure engine work without storing huge op vectors.

### C5. Emit results in the same `PERF_METRIC` format

Mimic `log_perf_metric(...)` style used by existing perf tests.
Include:

* `total_time_ms`
* `parse_time_ms`
* `diff_time_ms`
* `peak_memory_bytes`
* `rows_processed`
* `cells_compared`
* plus your added artifact bytes fields (recommended).

With the Workstream A fix in place, end‑to‑end runs should now produce:

* meaningful `parse_time_ms`,
* and `total_time_ms` that includes parse, making `diff_time_ms` interpretable.

### C6. Benchmark matrix (what to run)

Start with a small but representative set:

1. **Dense, single edit (large)**

   * stresses shared strings and grid scan
2. **Noise, single edit (large)**

   * stresses parsing diversity
3. **Sparse (99% blank), single edit (large)**

   * stresses sparse representation and scan
4. **Identical (large)**

   * isolates parse and “no-diff” fast paths

These map cleanly onto the existing “P1–P5” modes already defined in fixtures.

---

## 6) Workstream D: Versioned JSON output (alongside existing benchmark JSON)

### D1. Add `benchmarks/latest_e2e.json` (and optional CSV)

Quick and fullscale already export:

* `benchmarks/latest_quick.json` and CSV
* `benchmarks/latest_fullscale.json` and CSV

Phase 1 adds:

* `benchmarks/latest_e2e.json`
* `benchmarks/latest_e2e.csv` (optional but consistent)

### D2. Store historical results in a dedicated directory

Avoid breaking the existing baseline selection logic that expects quick/fullscale history under `benchmarks/results`.

Create:

* `benchmarks/results_e2e/`

Then:

* Save timestamped end‑to‑end runs there.

This also works cleanly with the existing trend tooling because `combine_results_to_csv.py` already supports a configurable `--results-dir`.

### D3. Implement a dedicated exporter script for the e2e suite

Add:

* `scripts/export_e2e_metrics.py`

Responsibilities:

1. **Generate fixtures**

   * Install fixture generator (same pattern as CI) and run `generate-fixtures --manifest fixtures/manifest_perf_e2e.yaml --force`.
2. **Run ignored tests**

   * `cargo test --release --features perf-metrics --test e2e_perf_workbook_open e2e_ -- --ignored --nocapture --test-threads=1`
3. **Parse `PERF_METRIC` lines**

   * Reuse the same parsing approach as existing scripts (regex extracting numeric key/value pairs).
4. **Write outputs**

   * Timestamped JSON into `benchmarks/results_e2e/`
   * Copy/symlink latest to `benchmarks/latest_e2e.json`
   * Optionally produce CSV for quick inspection

This gives you the same “versioned” semantics already used by `scripts/export_perf_metrics.py`, but isolated to the new suite. 

---

## 7) Workstream E: CI workflow to produce and publish the e2e benchmark artifacts

### E1. Add a new workflow (don’t overload existing quick/fullscale perf workflows)

Create:

* `.github/workflows/perf_e2e.yml`

Trigger:

* `workflow_dispatch` (manual)
* `schedule` (nightly/weekly)

Core steps (patterned after current workflows):

1. Checkout
2. Setup Rust
3. Setup Python
4. Install fixtures generator deps (same steps as `ci.yml`) 
5. Generate e2e fixtures via `manifest_perf_e2e.yaml`
6. Run `python scripts/export_e2e_metrics.py`
7. Upload artifacts:

   * `benchmarks/latest_e2e.json`
   * (optional) `benchmarks/latest_e2e.csv`
   * the timestamped JSON written into `benchmarks/results_e2e/`

This keeps Phase 1 changes low-risk: quick/fullscale threshold gating remains unchanged.

---

## 8) Quality, determinism, and “resource‑truthful” guardrails

### Determinism

* Fixture generation is deterministic by design (seed-driven YAML).
* Use explicit seeds for A/B pairs and apply a deterministic single-cell override for the “B” variant.

### Keeping runtime reasonable

* Streaming diffs + minimal edits prevent output explosion while still scanning large sheets.
* Keep the e2e suite small (4–6 tests) so scheduled jobs finish reliably.

### Avoid polluting quick/fullscale suites

* Don’t name tests with `perf_` substring to avoid execution under `check_perf_thresholds.py`, which filters by `"perf_"`. 
* Keep results history separate (`benchmarks/results_e2e/`) so quick/fullscale baseline comparisons are not accidentally disabled by “latest file wins” logic.

---

## 9) Suggested implementation sequence (small, reviewable changes)

1. **Metrics correctness PR**

   * Fix `apply_parse_metrics` to add open parse time to `total_time_ms` as well as `parse_time_ms`.
   * Add a regression test ensuring `total >= parse` after applying package parse time.

2. **Fixture PR**

   * Add `fixtures/manifest_perf_e2e.yaml`
   * Extend `LargeGridGenerator` with optional single-cell override support. 

3. **Rust harness PR**

   * Add `core/tests/e2e_perf_workbook_open.rs` with ignored tests.
   * Emit `PERF_METRIC` lines including input byte sizes.

4. **Export + versioning PR**

   * Add `scripts/export_e2e_metrics.py`
   * Add `benchmarks/results_e2e/` directory (empty) and document how to run combine/visualize with `--results-dir`. 

5. **CI workflow PR**

   * Add `.github/workflows/perf_e2e.yml` scheduled + manual.
   * Upload `latest_e2e.json` artifact.

---

## 10) Final definition of Phase 1 complete

Phase 1 is complete when:

* End‑to‑end benchmarks exist that:

  * open generated “real” `.xlsx` artifacts via `WorkbookPackage::open(...)`,
  * diff them via a production-like path (streaming strongly preferred), 
  * output coherent metrics where `parse_time_ms` is non‑zero and `total_time_ms` includes that parse time.

* A new versioned artifact `benchmarks/latest_e2e.json` is produced and published in CI alongside the existing quick/fullscale JSON artifacts.

* A history directory exists (`benchmarks/results_e2e/`) so you can generate trend CSV/plots over time using the existing tooling with `--results-dir`.
