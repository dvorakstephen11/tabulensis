Below is a concrete, implementable project plan that turns the **next-step work** into a sequence of developer-ready cycles, with clear acceptance criteria, file touchpoints, and verification commands. It’s written to match your repo’s existing workflow (fixture generator + CI gates + perf scripts + web/wasm build) and to close the specific gaps that are visible in the current artifacts.

---

## Scope and definition of “done” for this plan

This plan targets a **release-candidate MVP** where:

* **All repo gates are green**: fixture guard + generated fixtures + `cargo test --workspace` (same as CI) 
* **Performance gates pass** (quick + gate + full-scale + e2e where applicable) using the existing perf scripts and thresholds
* **Developers can run tests locally without “mystery failures”** due to missing generated fixtures (the PBIX embedded-queries failure is the canonical example)
* **Desktop/WASM builds are warning-clean where warnings indicate real configuration drift** (e.g., feature flags referenced in code but not declared) 
* **The “large file” path is validated end-to-end** via the existing 50k-row fixture/perf and e2e suites (already wired into manifests + scripts)

---

## High-level workstreams (in recommended execution order)

| Workstream                                     | Goal                                                                                | Why now                                                                                      |
| ---------------------------------------------- | ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| A. Local reliability + fixture hygiene         | Make tests reproducible locally; eliminate fixture-related failures                 | There’s a real failing test due to missing generated fixture in one run                      |
| B. Feature-flag + warning cleanup              | Remove “cfg feature doesn’t exist” drift and other actionable warnings              | Desktop currently emits unexpected `cfg(feature=...)` warnings                               |
| C. Performance validation + baseline hygiene   | Ensure quick/gate/full-scale/e2e results are stable and baselines are current       | Perf suites + baselines already exist; validate + tune, don’t reinvent                       |
| D. Scenario hardening: workbook/PBIX + M diffs | Add 2–3 “realistic” end-to-end fixtures + assertions that stress the semantic paths | Your unit coverage is strong; the missing piece is “composed” scenarios and regression locks |
| E. Determinism + output stability              | Make it easy to prove “same inputs → same outputs” across threads/platforms         | Required for Git integration + CI trust + WASM parity                                        |
| F. Release readiness pass                      | Dry-run release, docs polish, final gating checklist                                | Release workflow exists; we validate the whole pipeline                                      |

Each workstream below includes **exact changes**, **tests to add**, and **verification steps**.

---

# Workstream A — Local reliability + fixture hygiene

### A1) Fix the PBIX embedded-queries fixture failure “the right way”

**Observed failure:** `info_pbix_includes_embedded_queries` panics because `fixtures/generated/pbix_embedded_queries.pbix` doesn’t exist in that run .
**But the scenario is defined** in the fixture manifests (there is a `branch2_pbix_embedded_queries` scenario generating `pbix_embedded_queries.pbix`) .

#### Implementation steps

1. **Confirm the fixture is present in the correct manifest used for tests.**

   * CI generates from `fixtures/manifest_cli_tests.yaml` 
   * Ensure `branch2_pbix_embedded_queries` is included there (not just in `fixtures/manifest.yaml`).
   * If missing: add the scenario stanza (or include via a shared include mechanism if you have one).

2. **Regenerate the lockfile and fixtures locally**

   * Run:

     * `python -m pip install -r fixtures/requirements.txt`
     * `python -m pip install -e fixtures --no-deps`
     * `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean`
     * `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --verify-lock fixtures/manifest_cli_tests.lock.json`
   * Commit the updated `fixtures/manifest_cli_tests.lock.json` if it changes.

3. **Make the failure mode developer-friendly**

   * Update `core/tests/common/mod.rs` (the `fixture_path()` helper referenced in the panic) to emit a **single actionable message**:

     * “Run `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean`”
     * And optionally: auto-detect CI vs local and link to `fixtures/README.md`.

#### Acceptance criteria

* Running `cargo test --workspace` after fixture generation passes, including the PBIX info test.
* If fixtures are missing, the panic message explicitly tells the dev how to generate them.

---

### A2) Add a “one command” local test runner that mirrors CI

CI does:

* fixture reference guard
* generate fixtures
* verify lock
* run tests 

#### Implementation steps

1. Add a Python wrapper: `scripts/dev_test.py` (cross-platform; avoids bash-only tooling).
2. The script should run, in order:

   1. `python scripts/check_fixture_references.py`
   2. `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean`
   3. `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --verify-lock fixtures/manifest_cli_tests.lock.json`
   4. `cargo test --workspace`
3. Make it fail-fast and print the exact command line it’s running.

#### Acceptance criteria

* A new developer can run: `python scripts/dev_test.py` and get the same outcome as CI for the test job.

---

# Workstream B — Feature-flag + warning cleanup

### B1) Fix “unexpected cfg condition value” warnings in desktop

You currently have warnings like:

* `unexpected cfg condition value: perf-metrics`
* `unexpected cfg condition value: model-diff`
  …coming from desktop code guarded by `#[cfg(feature = "...")]`, but those features aren’t declared in the crate’s Cargo features. 

#### Implementation steps

1. In `desktop/src-tauri/Cargo.toml`, add:

   * `[features] perf-metrics = [...]`
   * `[features] model-diff = [...]`
2. Forward features to the underlying crates that actually implement them.

   * Example pattern (you’ll adjust names to match your dependency aliases):

     * `perf-metrics = ["excel_diff/perf-metrics"]`
     * `model-diff = ["excel_diff/model-diff"]`
3. If the features are **not intended** to be user-facing in desktop:

   * Replace `#[cfg(feature="...")]` with `#[cfg(any(test, debug_assertions))]` (or remove the code path).
   * Or hide behind a single `desktop-dev` feature that’s explicitly declared.

#### Acceptance criteria

* `cargo test --workspace` (or at least `cargo build -p excel_diff_desktop`) has **no `unexpected_cfgs` warnings**.

---

### B2) Clean up low-signal warnings that hide real issues

Example: unused import `CallbackSink` in `core/tests/package_streaming_tests.rs` .

#### Implementation steps

* Remove unused imports (or use them).
* If a symbol is intentionally unused in some configurations, prefer `#[allow(unused_imports)]` only when there’s a good reason.

#### Acceptance criteria

* Local build output is quieter, making new regressions easier to spot.

---

# Workstream C — Performance validation + baseline hygiene

This repo already has:

* perf suites (quick/gate/full-scale) + caps for P1–P5 and 50k tests 
* a perf regression workflow that runs quick + gate using pinned baselines 
* scheduled full-scale perf workflow 
* scheduled e2e open+diff metrics with caps + baseline comparison
* many historical benchmark runs checked in (including fullscale + parallel)

So the plan here is: **validate, then tune only if needed**.

---

### C1) Standardize a “perf validation playbook” for devs

#### Implementation steps

Add `docs/perf_playbook.md` with:

* How to run quick suite locally:

  * `python scripts/check_perf_thresholds.py --suite quick --baseline benchmarks/baselines/quick.json --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv`
* How to run gate suite locally:

  * `python scripts/check_perf_thresholds.py --suite gate --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests`
* How to run full-scale:

  * `python scripts/check_perf_thresholds.py --suite full-scale --baseline benchmarks/baselines/full-scale.json`
* How to run e2e:

  * `python scripts/export_e2e_metrics.py --baseline benchmarks/baselines/e2e.json`
* What to do when baselines are missing or stale (script already supports selecting latest results if no pinned baseline exists)

#### Acceptance criteria

* A developer can follow one doc and replicate perf CI locally.

---

### C2) Baseline hygiene and “update protocol”

Baselines are pinned in `benchmarks/baselines/*.json` and the results directory contains many historical files.

#### Implementation steps

1. Decide the rule:

   * Baselines update only when:

     * algorithm changes, or
     * dependencies change in a way that consistently shifts runtime
2. Add `scripts/update_baselines.py` (or enhance existing scripts) to:

   * run the suite
   * copy generated `benchmarks/latest_*.json` into `benchmarks/baselines/*.json`
   * print a “diff summary” using your existing `compare_perf_results.py` logic 
3. Require baseline updates to be paired with:

   * a short note in the PR description (why is new baseline justified?)

#### Acceptance criteria

* When perf shifts, updating baselines is a consistent, low-friction process.

---

### C3) If perf fails: a targeted optimization workflow (not a grab bag)

If any of P1–P5 or 50k tests violate caps in `check_perf_thresholds.py`  or e2e caps in `export_e2e_metrics.py` :

#### Implementation steps

1. Identify whether time is in:

   * parsing
   * diff/alignment
   * serialization / output
2. Add exactly one microbenchmark or PERF_METRIC for the hot section.
3. Optimize with a reversible change:

   * reduce allocations
   * swap algorithm threshold (e.g., avoid DP when it explodes)
   * reduce intermediate buffers
4. Prove improvement with:

   * before/after JSON result comparison (use existing benchmark result tooling).

#### Acceptance criteria

* Perf regressions are fixed with a measured loop and a paper trail, not guesswork.

---

# Workstream D — Scenario hardening: workbook + PBIX + M diffs

Your test suite already covers:

* DataMashup packaging edge cases
* metadata parsing, permissions
* textual and semantic M diffs, canonicalization, step detail alignment

The gap that typically remains at this stage is **composed, realistic fixtures** that lock behavior across multiple interacting layers.

---

### D1) Add 2–3 “composed” fixtures that exercise real-world interactions

Use the fixture generator system (`fixtures/src/generators/*`) and manifests. 

#### Fixture set proposal (concrete)

1. **XLSX: “mixed sheet + mashup”**

   * One workbook with:

     * a grid sheet with dense data (small enough for unit tests)
     * DataMashup with 3–5 queries
   * Variant B introduces:

     * a small sheet edit (cell change + row insertion)
     * a query pipeline change (rename step, reorder steps, change a join key)

2. **PBIX: embedded queries plus workbook-ish metadata**

   * Variant A: PBIX with embedded queries already exists as a scenario 
   * Variant B: modify:

     * permissions metadata and one M query body

3. **Adversarial M pipeline**

   * A query with repeated step shapes (to stress step alignment) and a change that causes ambiguous matching.
   * Use it to lock the current step-match cost behavior.

#### Implementation steps

* Extend `fixtures/src/generators/mashup.py` and/or `fixtures/src/generators/pbix.py` (depending on how you currently embed DataMashup into each container) .
* Add scenarios to `fixtures/manifest_cli_tests.yaml`.
* Update lockfile after generation.

#### Acceptance criteria

* New fixtures are generated in CI and referenced by at least:

  * one core integration test
  * one CLI integration test (optional but recommended)

---

### D2) Add “behavior lock” tests for the composed fixtures

Add tests that assert *semantics*, not brittle formatting.

#### Implementation steps

1. In `core/tests/` add a new test file, e.g.:

   * `m9_composed_end_to_end_tests.rs` (or similar naming)
2. Assertions should be:

   * “Workbook diff includes both Sheet diffs and Query diffs”
   * “Query X is classified as semantic change, not rename”
   * “Step diffs include ModifiedStep for ‘Join’ with params_changed=true”
3. Avoid asserting full JSON blobs unless you canonicalize them.

#### Acceptance criteria

* Composed fixtures create stable, meaningful diff signals that catch regressions in layering (container → mashup → M → diff report).

---

# Workstream E — Determinism + output stability

You already have many tests ensuring ordering and consistent semantics (e.g., diffs sorted by name; formatting-only canonicalization) . This workstream makes determinism *provable* at the product boundary.

---

### E1) Add a “same inputs, many thread counts” integration test for the CLI JSON output

#### Implementation steps

1. In `cli/tests/`, add a test like `determinism_cli_json.rs`.
2. The test should:

   * pick a small-but-nontrivial fixture pair (composed fixture from D1 is perfect)
   * run CLI diff multiple times with:

     * `--threads 1`, `--threads 2`, `--threads 8` (or env var equivalent if you have one)
   * capture JSON output
   * normalize it (sort object arrays if needed; strip timestamps if any)
   * assert byte-for-byte equality of normalized output

#### Acceptance criteria

* You can break determinism only intentionally (and then the test forces you to update normalization rules or document the new behavior).

---

### E2) Add “schema stability guardrails” (pragmatic, not heavyweight)

#### Implementation steps

Pick one:

* **Option A (low overhead):** a “golden hash” test

  * Canonicalize JSON → compute SHA256 → compare to committed expected hash.
* **Option B (slightly heavier):** a JSON schema and a validator test

  * Write `docs/schema/diff_report_vX.json`
  * Validate serialized reports against it in tests.

Given your emphasis on streaming and stability, Option A is usually enough to catch accidental churn.

#### Acceptance criteria

* Any change to the output contract requires an explicit test update and a doc note.

---

# Workstream F — Release readiness pass

You already have a release workflow that:

* checks crate versions vs tag
* builds Windows artifacts
* packages and publishes (dry-run capable) 

So this workstream is about validating the “whole pipeline” and making the repo feel shippable.

---

### F1) Run a full dry-run release locally (or via workflow_dispatch)

#### Implementation steps

1. Ensure `scripts/verify_release_versions.py` passes (already part of release job) 
2. Trigger `release.yml` via workflow_dispatch with `dry_run=true` (or equivalent).
3. Confirm artifacts:

   * Windows exe/zip naming and layout
   * README included
   * version stamping matches expectations

#### Acceptance criteria

* A release can be produced without manual patching.

---

### F2) MVP doc polish (the minimum set)

Focus on “how do I use it” and “how do I develop it”.

#### Implementation steps

1. Update top-level `README.md`:

   * install (cargo install or download release)
   * example commands:

     * diff to text
     * diff to JSON
     * diff to JSONL for large files (if that’s the intended path)
2. Update `fixtures/README.md`:

   * how to generate fixtures
   * common failure mode & fix (ties back to Workstream A)

#### Acceptance criteria

* New developer can run: build → generate fixtures → tests → diff a pair of files.

---

## Suggested branch/cycle breakdown (matches your naming convention)

If you want to keep the work reviewable, split into 4–6 branches:

1. `2026-01-xx-fixtures-local-runner`

   * A1, A2

2. `2026-01-xx-desktop-feature-flags-cleanup`

   * B1, B2

3. `2026-01-xx-perf-playbook-baselines`

   * C1, C2 (C3 only if perf fails)

4. `2026-01-xx-composed-fixtures-e2e-locks`

   * D1, D2

5. `2026-01-xx-determinism-output-stability`

   * E1, E2

6. `2026-01-xx-release-rc-pass`

   * F1, F2

---

## Verification checklist (what the implementing dev should run)

This is the exact “done means done” list:

1. **Test job parity with CI** 

* `python scripts/check_fixture_references.py`
* `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean`
* `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --verify-lock fixtures/manifest_cli_tests.lock.json`
* `cargo test --workspace`

2. **Perf suites**

* `python scripts/check_perf_thresholds.py --suite quick --baseline benchmarks/baselines/quick.json`
* `python scripts/check_perf_thresholds.py --suite gate --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests`
* `python scripts/check_perf_thresholds.py --suite full-scale --baseline benchmarks/baselines/full-scale.json`

3. **E2E metrics (scheduled in CI, but run locally before RC)** 

* `python scripts/export_e2e_metrics.py --baseline benchmarks/baselines/e2e.json`

4. **WASM build smoke (for web demo parity)** 

* `wasm-pack build wasm --release --target web --out-dir ./web/wasm`

5. **Release dry-run** 

* Trigger `release.yml` with dry-run and confirm artifacts

