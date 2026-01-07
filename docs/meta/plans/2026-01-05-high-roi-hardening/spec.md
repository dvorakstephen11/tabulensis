Below is a concrete implementation plan for items **#1, #2, and #4** from the “What I would do next (highest ROI to raise ‘ship confidence’)” section of `project_completion_estimate_2026_01_05.md`. 

The guiding idea across all three: **convert “we have the scaffolding” into “release is impossible unless the scaffolding passes.”** That’s what turns anxiety into momentum.

---

## What “ship confidence” means in practice

A product like this is “shippable” when you can cut a release without wondering:

* “Did performance regress on big files?”
* “Will CI or a user hit a missing-fixture landmine?”
* “Are some crates quietly compiling with nonsense feature gates?”

Items #1/#2/#4 each remove one of those categories of doubt. 

---


## Item 4: Tighten workspace feature hygiene (especially desktop)

### Target outcome

No more `unexpected cfg condition value` warnings (especially in the desktop crate), and CI actively enforces that the key feature sets compile.

This is directly called out both in the completion estimate and design evaluation: the desktop crate emits “unexpected cfg” warnings for `perf-metrics` and `model-diff`, signaling feature drift/misalignment. fileciteturn5file0fileciteturn12file0

### Why it’s high ROI

* It’s a **fast** fix.
* It removes warning noise that can hide real problems.
* It makes the desktop surface feel less “experimental” (which is exactly the concern highlighted).

### Implementation plan

#### 4.1 Inventory the actual feature gates in desktop

From the build logs, these are already present in code:

* `desktop/src-tauri/src/diff_runner.rs` has `#[cfg(feature = "perf-metrics")]` 
* `desktop/src-tauri/src/export/audit_xlsx.rs` has multiple `#[cfg(feature = "model-diff")]` 
* There are additional `model-diff` gates elsewhere in desktop (the logs show multiple instances). 

**Task:** run a quick repo-wide search (ripgrep) limited to desktop:

* `rg 'cfg\(feature\s*=\s*"' desktop/src-tauri/src`

Deliverable: a list of every `cfg(feature=...)` used in desktop and which file it’s in.

#### 4.2 Decide what those features should *mean*

You want these features to be *real knobs*, not cosmetic `cfg`s:

* **desktop `perf-metrics`** should:

  * enable whatever extra perf instrumentation you intended in the desktop runner
  * and **forward** the feature to the engine crate dependency if that’s what it depends on (likely `excel_diff/perf-metrics`)

* **desktop `model-diff`** should:

  * enable model-diff related UI/export paths
  * and **forward** to the engine feature that actually implements it (the design eval notes model diff exists behind `model-diff` and is integrated into PBIX diffing/export surfaces). 

Deliverable: a 1–2 paragraph “feature semantics” note (you can put it in desktop README or a short `desktop/src-tauri/FEATURES.md`).

#### 4.3 Define/align features in `desktop/src-tauri/Cargo.toml`

Right now, Rust is warning because the desktop crate is using `cfg(feature="...")` without declaring those features in its Cargo manifest. 

**Task:** add a `[features]` section to the desktop crate and define:

* `perf-metrics = [...]`
* `model-diff = [...]`

…and forward them to the correct dependency features. The typical pattern is:

* `perf-metrics = ["excel_diff/perf-metrics"]`
* `model-diff = ["excel_diff/model-diff"]`

(Exact dependency names depend on how desktop declares its dependencies; the point is to forward features rather than creating “dead” toggles.)

Deliverable: desktop crate builds cleanly without the `unexpected_cfgs` warnings.

#### 4.4 Make CI enforce “no unexpected cfgs” for desktop

You do **not** need to `-Dwarnings` the whole repo (desktop currently emits many normal warnings), but you *do* want to prevent this specific class from reappearing.

**Task:** add a CI step that runs:

* `cargo check -p excel_diff_desktop` with a lint setting that turns `unexpected_cfgs` into an error

There are two common approaches:

1. Set `RUSTFLAGS` for that step to deny the specific lint (targeted, low disruption).
2. Add `#![deny(unexpected_cfgs)]` at crate root (more permanent, but touches code).

Deliverable:

* CI fails if someone adds `#[cfg(feature="foo")]` in desktop without defining `foo`.

#### 4.5 Add CI coverage for feature combinations you care about

The completion estimate explicitly wants “ensure CI checks compile the key feature sets.” 

Minimal set for desktop:

* default features: `cargo check -p excel_diff_desktop`
* model diff enabled: `cargo check -p excel_diff_desktop --features model-diff`
* perf-metrics enabled: `cargo check -p excel_diff_desktop --features perf-metrics`

If you prefer fewer steps:

* `cargo check -p excel_diff_desktop --all-features`

Deliverable:

* desktop compiles under those feature sets on CI, and no unexpected cfg warnings appear. 

---

## Item 2: Unify fixtures so CLI/integration tests never depend on missing files

### Target outcome

A test can’t accidentally reference a fixture that isn’t guaranteed to exist, and the project has a single source of truth for “fixtures required by tests.”

This addresses the risk called out explicitly: a known failure mode was “missing PBIX fixture file” in CLI integration tests, while similar PBIX samples exist in fuzz corpus—classic “fixtures live in two places” drift. 

### Why it’s high ROI

Fixture flakiness creates “release dread.” Even if the core engine is solid, missing-file failures make the project feel unreliable.

You already have the right scaffolding:

* deterministic fixture generator with manifests
* a fixture reference guard script
* a dev test runner that generates + verifies fixtures before running tests

The plan is to **close the remaining gaps**.

### Implementation plan

#### 2.1 Make the fixture reference guard *actually complete*

Right now, `scripts/check_fixture_references.py` scans:

* `core/tests/**/*.rs`
* `cli/tests/**/*.rs`
* workflows under `.github/workflows` 

But you have at least one important test suite that sources fixture names from YAML:

* `core/tests/robustness_regressions_tests.rs` loads `core/tests/robustness_regressions.yaml` and uses the `file:` field to decide what to open. 

Those `file:` names **do not appear in Rust source**, so a `.rs`-only scan can miss fixture requirements until runtime.

**Task:** extend `scripts/check_fixture_references.py` to also scan:

* `core/tests/**/*.yaml` and `core/tests/**/*.yml`
* `cli/tests/**/*.yaml` and `cli/tests/**/*.yml`

Good news: your existing regex-based scanner likely already catches quoted fixture-like names; you just need to include these files in the scan set.

Deliverable:

* If `robustness_regressions.yaml` (or any future YAML suite) references a fixture not present in `fixtures/manifest_cli_tests.yaml`, the guard fails **before** tests run.

#### 2.2 Run the fixture guard as a first-class CI “gate”

You already do this in `scripts/dev_test.py`:

* run fixture reference guard
* generate fixtures from `fixtures/manifest_cli_tests.yaml`
* verify against lockfile
* then run `cargo test --workspace` 

**Task:** ensure CI has a job that runs exactly this “canonical flow” (either by calling `python scripts/dev_test.py` directly, or replicating the steps).

Deliverable:

* no CI path can “forget” to generate fixtures and then run integration tests.

#### 2.3 Enforce lockfile discipline for fixtures

The generator supports `--verify-lock` (and you already use it in `dev_test.py`). 

**Task:** make sure the CI path used for tests includes a verify-lock step for the test fixture manifest(s) that matter:

* `fixtures/manifest_cli_tests.yaml` with its lock JSON
* and (separately) whatever manifest powers release smoke tests

Deliverable:

* fixture content cannot drift silently; if generator logic changes, you either update lock intentionally or CI fails.

#### 2.4 Unify PBIX fixture sourcing (tests must not rely on fuzz corpus)

The completion estimate’s suspicion is exactly right: if you have PBIX samples in fuzz corpus but tests expect them in generated fixtures, you can end up with:

* duplicated “canonical” samples
* tests referencing one location while the sample only exists in the other 

**Task:** pick a single canonical source for each fixture used by tests:

* If a PBIX is used by tests: it **must** be produced by the fixture generator manifest used for tests (typically `manifest_cli_tests.yaml`).
* The fuzz corpus can still contain PBIX samples, but treat fuzz corpus as **derived** (seeded from fixtures or explicitly maintained), not as the canonical source for test fixtures.

You already have tooling that suggests this direction (a “seed fuzz corpora from fixtures” style script exists). 

Deliverable:

* Any test-required PBIX fixture is generated into `fixtures/generated/` during the standard fixture generation step.

#### 2.5 Make local dev less error-prone (reduce “I ran cargo test and it failed”)

Your helper already gives a good error message when fixtures are missing, pointing devs toward generating them. 

**Task:** institutionalize one “happy path” command:

* Promote `python scripts/dev_test.py` as *the* supported local test entry point.
* Optionally add a thin wrapper like `make test` / `just test` that runs it.

Deliverable:

* fewer “missing fixture” surprises locally (which is psychologically a huge win).

---

## Item 1: Make full-scale perf + memory a hard gate

### Target outcome

You can’t cut a release unless **full-scale** performance + peak memory checks pass, and the latest full-scale artifacts are produced and reviewed as part of the release process.

This is explicitly the highest ROI ship-confidence move in the completion estimate. 

### Current state (good scaffolding, missing “hard gate”)

You already have:

* `perf.yml` running quick + gate suites on push/PR, exporting artifacts 
* `perf_fullscale.yml` running full-scale on a schedule / manual dispatch, exporting artifacts 
* `scripts/check_perf_thresholds.py` which enforces:

  * absolute time caps for selected tests
  * baseline regression checks for total time and peak memory

But full-scale is not wired into the *release* pipeline, and full-scale thresholds currently only cap time (memory is only baseline-relative).

### Implementation plan

#### 1.1 Make full-scale perf a release-blocking job

The simplest, most reliable interpretation of “hard gate” is: **release workflow must run it**.

**Task:** add a job to `.github/workflows/release.yml` (before packaging jobs) that:

* checks out code
* installs Rust
* sets up Python
* runs:

`python scripts/check_perf_thresholds.py --suite full-scale --baseline benchmarks/baselines/full-scale.json --export-csv benchmarks/latest_fullscale.csv --export-json benchmarks/latest_fullscale.json`

* uploads the artifacts (CSV + JSON) as release workflow artifacts.

**Wire dependency:** all build/package jobs depend on this perf gate job.

Deliverable:

* if full-scale perf regresses, the release tag build fails early, before you publish binaries.

#### 1.2 Ensure full-scale artifacts are produced continuously (not just at release time)

The completion estimate wording emphasizes “always produced,” not “only when someone runs it manually.” 

**Task:** update `.github/workflows/perf_fullscale.yml` triggers:

* add `push` on `main/master` in addition to schedule + workflow_dispatch. 

Deliverable:

* every merge to main produces `latest_fullscale.*` artifacts (and catches regressions days before release).

#### 1.3 Strengthen full-scale memory gating (absolute caps, not only baseline-relative)

Right now:

* Quick suite includes one explicit `max_peak_memory_bytes` cap (`perf_preflight_low_similarity`)
* Full-scale suite caps time but not peak memory in its threshold table. 

Baseline regression checks already include peak memory comparisons, but baseline-relative-only can hide “baseline drift” if someone updates baseline without scrutiny.

**Task:** add explicit `max_peak_memory_bytes` for each full-scale threshold entry in `scripts/check_perf_thresholds.py`:

* `perf_50k_dense_single_edit`
* `perf_50k_completely_different`
* `perf_50k_adversarial_repetitive`
* `perf_50k_99_percent_blank`
* `perf_50k_identical` 

How to pick caps (practical approach):

* Start from your pinned baseline numbers in `benchmarks/baselines/full-scale.json`.
* Set caps with headroom (e.g., baseline * 1.25) on CI, then tighten once stable.
* Keep caps high enough to avoid spurious failures, low enough to catch real regressions.

Deliverable:

* full-scale suite can fail even if someone “updates the baseline” to hide a memory regression.

#### 1.4 Make missing-baseline an error in full-scale mode

In `check_perf_thresholds.py`, baseline checks currently warn+skip if baseline lacks a test. 

For a hard gate, you don’t want:

* a perf test renamed/added
* baseline missing it
* gate still “passes” with a warning

**Task:** add a strict mode:

* either a new flag like `--require-baseline`
* or implicitly: if suite == `full-scale`, missing baseline for any expected test is a failure

Deliverable:

* baseline coverage stays complete; changes to perf tests force an intentional baseline update.

#### 1.5 Make trend changes visible in the CI logs (not only pass/fail)

When you’re in “ship mode,” confidence comes from being able to *see* deltas.

You already have a comparator script: `scripts/compare_perf_results.py`. 

**Task:** after full-scale run (in perf_fullscale workflow and in release perf gate job), run:

* `python scripts/compare_perf_results.py benchmarks/baselines/full-scale.json benchmarks/latest_fullscale.json`

Deliverable:

* workflow logs show “what got slower/faster” by percentage, so you don’t need to download artifacts to learn anything.

#### 1.6 Document the baseline update ritual (so it doesn’t become scary)

You already have `scripts/update_baselines.py` that runs suites and updates pinned baselines, including full-scale. 

**Task:** document a clear policy:

* When baseline updates are allowed
* How to run them (`python scripts/update_baselines.py --suite full-scale`)
* What constitutes “acceptable regression” vs “fix it”

Deliverable:

* baseline changes become a deliberate, reviewable action—never a desperate “make CI green.”

---

## Recommended execution order to keep momentum

If the goal is “market ASAP,” you want early wins that reduce dread:

1. **Item #4 (feature hygiene)**: quick fix, immediately reduces build “spookiness.” 
2. **Item #2 (fixtures single source of truth)**: removes a class of flaky failures that ruin release confidence. 
3. **Item #1 (full-scale perf + memory hard gate)**: final “you can ship without fear” lock. 

