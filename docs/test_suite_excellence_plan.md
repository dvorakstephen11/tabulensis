# Test Suite Excellence Plan

**Last updated:** 2026-02-06

## Goal
Maximize confidence in Tabulensis by systematically expanding test coverage while keeping the suite:
- Deterministic (no flakes, no hidden state)
- Fast by default (PR feedback in minutes)
- Deep when needed (nightly + opt-in heavy validation)
- Reproducible (fixture discipline + locked outputs)
- Actionable (failures point to clear causes and next steps)

This doc is about **how we grow and operate the test suite**. For the “what features exist / what to implement next” roadmap, see the product/algorithm plans (for example `docs/rust_docs/excel_diff_testing_plan.md`).

## Non-goals
- Replacing existing technical specs or algorithm plans
- Turning every validation into a PR gate (we split PR-fast vs nightly-heavy)
- Chasing line coverage at the expense of correctness, robustness, or performance budgets

## Current Baseline (What Exists Today)
This plan builds on existing infrastructure rather than reinventing it:

- **CI-like local entrypoint:** `python3 scripts/dev_test.py`
  - Runs fixture reference guard, generates fixtures, verifies fixture lock, then runs `cargo test --workspace` (with license skip). See `scripts/dev_test.py`.
- **Fixture discipline:** fixtures are generated and not committed; tests must reference fixtures produced by manifests. See `docs/maintainers/fixtures.md` and `scripts/check_fixture_references.py`.
- **Cross-platform smoke:** CI generates `fixtures/manifest_cli_tests.yaml` fixtures then runs `cargo test -p excel_diff` and `cargo test -p excel_diff_cli` on Linux/Windows/macOS (smoke matrix).
- **Architecture guard:** `python3 scripts/arch_guard.py` enforces parse/diff layering.
- **Performance validation:** threshold suites + baselines (`scripts/check_perf_thresholds.py`), plus the full perf-cycle workflow (`scripts/perf_cycle.py`). See `docs/perf_playbook.md`.
- **Robustness corpus + fuzz:** `core/fuzz` targets exist, with seeding + maintenance scripts. See `docs/robustness_corpus.md`, `scripts/seed_fuzz_corpus.py`, `scripts/fuzz_corpus_maint.py`, and `scripts/ingest_private_corpus.py`.
- **Desktop UI visual regression:** deterministic UI scenarios + capture/diff/review pipeline. See `docs/ui_visual_regression.md` and `docs/ui_visual_regression_plan.md`.

## What We Mean By “Coverage”
We will treat “coverage” as multiple dimensions, each with different measurement and different ROI:

- **Scenario coverage:** end-to-end “user-visible” behaviors (open/diff/export/search) across Excel/PBIX and common edge cases.
- **Construct coverage:** language and file-format constructs (OpenXML parts, M syntax, formula constructs) are either semantically supported or explicitly treated as opaque, with tests backing each claim.
  - Example: M semantic coverage tracked in `docs/m_parser_coverage.md` and asserted in `core/tests/m8_m_parser_coverage_audit_tests.rs`.
- **Robustness coverage:** malformed inputs, adversarial grids, limits/ceilings, and “weird file” regressions.
- **Feature/flag coverage:** critical feature combinations compile and (where meaningful) run tests (for example default vs `parallel`, wasm-oriented features, model-diff-only).
- **Platform coverage:** Linux/Windows/macOS smoke stays green; platform-specific path and file-I/O behavior is tested.
- **UI/UX coverage:** deterministic UI states are captured and diffed against baselines for a small curated scenario set.
- **Performance coverage:** we have budgets (time + memory) and regressions are treated as test failures, not “nice-to-have” warnings.

## Test Tiers (PR-Fast vs Nightly-Deep)
Default policy: **PR gates stay fast**; **nightly jobs provide depth**; PRs can opt into heavy validation when a change is risky.

### Tier A: PR “Fast Gates” (minutes)
These should remain the standard PR bar:

- CI-like flow: `python3 scripts/dev_test.py`
- Architecture guard: `python3 scripts/arch_guard.py`
- Clippy with unwrap/expect denied: `cargo clippy --workspace -- -D clippy::unwrap_used -D clippy::expect_used`
- Build examples: `cargo build --workspace --examples`

Conditional PR gate (recommended when touching perf-sensitive paths in core/desktop backend/payload shaping):
- Quick perf suite: `python3 scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json ...`

### Tier B: PR “Opt-In Heavy” (workflow_dispatch / label / manual)
Run on demand for changes that increase risk:

- Gate perf suite (50k smoke): `python3 scripts/check_perf_thresholds.py --suite gate --parallel --baseline benchmarks/baselines/gate.json --test-target perf_large_grid_tests`
- UI visual regression for a small canonical scenario set (example: `scripts/ui_pipeline.sh compare_grid_basic`)
- Timeboxed fuzz smoke for the relevant `core/fuzz` target(s)

### Tier C: Nightly “Deep Validation” (timeboxed; artifacts uploaded)
Nightly should run validations that are too slow/noisy for PRs:

- Full-scale perf suite and/or full perf cycle on stable runners (see `docs/perf_playbook.md`)
- Longer fuzz campaigns per target with crash artifact upload
- Robustness regression sweep and corpus checks
- Optional: code coverage reporting (see Phase 6)

## Phased Roadmap
Each phase has deliverables and exit criteria. Phases are intended to be incremental and independently valuable.

### Phase 0: Make The Suite Legible
**Intent:** Reduce “tribal knowledge” and make it obvious what to run and why.

Deliverables:
- This doc exists (and stays current) as the test-suite “front door”.
- A short “when you change X, run Y” table exists in this doc:
  - Core parse/diff/alignment: run Tier A + perf quick; escalate to gate/full perf cycle as appropriate.
  - Desktop backend/payload shaping: run Tier A + perf quick; consider UI opt-in.
  - UI/XRC wiring changes: run Tier A + UI opt-in scenarios.
  - Fixture generator changes: run Tier A and verify lock behavior.
- A clear map of where tests live:
  - Core: `core/tests/**`
  - CLI: `cli/tests/**`
  - Desktop backend: `desktop/backend/tests/**`
  - Fuzz: `core/fuzz/fuzz_targets/**`
  - UI regression: `desktop/ui_scenarios/**`, `desktop/ui_snapshots/**`, `scripts/ui_*.{sh,js,py}`

Exit criteria:
- A new contributor can run Tier A locally and interpret failures without guesswork.

### Phase 1: Deterministic “User-Visible” Scenario Coverage (Fixture-Backed)
**Intent:** Every major user-visible surface has deterministic fixture-backed tests.

Deliverables:
- Expand `fixtures/manifest_cli_tests.yaml` to cover key scenarios, with tests asserting:
  - Open + diff correctness (Excel + PBIX smoke)
  - Export correctness (stable schemas, deterministic ordering)
  - Search index correctness (where applicable)
  - Error handling for missing/corrupt/non-zip inputs (typed errors, not panics)
- Ensure tests never reference ad-hoc filenames:
  - Update manifest outputs when adding fixture references (enforced by `scripts/check_fixture_references.py`).
  - Update lock files only when fixture definitions change (see `docs/maintainers/fixtures.md`).

Exit criteria:
- There is a feature-to-test matrix in this doc (or a linked doc) mapping each major capability to a fixture scenario + test file(s).

### Phase 2: Construct Coverage Audits (Beyond Line Coverage)
**Intent:** For each “language inside the file,” we explicitly track semantic support and assert it.

Deliverables:
- Continue using `docs/m_parser_coverage.md` + audit tests as the pattern:
  - When a new M construct becomes semantic, update the doc and add/extend tests.
- Add analogous “coverage audit” patterns for:
  - OpenXML part/element support (what we parse vs ignore)
  - Formula handling (canonicalization/diff constructs)
  - Diff-op categories (adds/removes/updates/moves, formatting policies)

Exit criteria:
- Each audit area has:
  - A short doc listing supported vs opaque
  - A test that fails if the implementation regresses into “opaque” unexpectedly

### Phase 3: Robustness Regressions + Fuzz Maturity
**Intent:** Fuzzing continuously hardens the codebase, and every fixed crash becomes a deterministic regression.

Deliverables:
- “Crash to regression” workflow is documented and followed:
  - Reproduce with a minimized seed
  - Add deterministic regression coverage (`robustness_regressions` or an equivalent integration test)
  - Seed the fuzz corpus appropriately (Tier 2 seeds) without bloating it
- Nightly fuzz campaigns (timeboxed) for relevant targets in `core/fuzz/fuzz_targets/**`
- Corpus is maintained:
  - Seed from fixtures: `python3 scripts/seed_fuzz_corpus.py --config core/fuzz/seed_fixtures.yaml`
  - Minimize/limit: `python3 scripts/fuzz_corpus_maint.py` (optionally with `cargo fuzz cmin`)
  - Private corpora ingestion (never committed): `python3 scripts/ingest_private_corpus.py ...` (see `docs/robustness_corpus.md`)

Exit criteria:
- Fuzz runs produce actionable artifacts (seed, stack trace, minimal repro steps).
- Fixed fuzz-found issues always land with a deterministic regression test.

### Phase 4: UI Visual Regression + End-to-End Desktop Excellence
**Intent:** Prevent UI regressions with a small, stable, deterministic scenario set.

Deliverables:
- Canonical UI scenario set is defined (start small):
  - Examples: `compare_grid_basic`, `compare_large_mode`, `pbix_no_mashup`
- CI opt-in job runs the UI pipeline for the canonical set on Linux (headless) and uploads artifacts on failure.
- Baseline update policy is explicit and reviewable (never update baselines just to “green CI”).

Exit criteria:
- A contributor can run `scripts/ui_pipeline.sh <scenario>` locally and get:
  - Screenshot
  - Diff image/metrics
  - Review report (when enabled)
  …with consistent results across runs on the same machine.

### Phase 5: Performance Is A Test (Budgets + Baselines)
**Intent:** “Correct but too slow” is treated as a regression with a clear workflow.

Deliverables:
- Adopt `docs/perf_playbook.md` as the single source of truth and keep this plan aligned.
- PR-fast:
  - Run quick perf suite when touching core parse/diff/alignment/open or perf-sensitive desktop backend/payload shaping.
- PR opt-in / nightly:
  - Run gate/full-scale and/or full perf cycle depending on change risk (see `docs/perf_playbook.md`).
- Baseline update policy:
  - Only bump baselines when changes are intentional, repeatable, and documented.

Exit criteria:
- Perf regressions are either fixed or explicitly accepted with rationale and updated baselines (never hand-waved).

### Phase 6: Continuous Excellence (Process, Metrics, and Flake Policy)
**Intent:** Prevent long-term quality decay as the suite grows.

Deliverables:
- Flake policy:
  - Flaky tests are treated as bugs; quarantining requires an issue, owner, and expiry.
  - Prefer eliminating nondeterminism (fixed seeds, stable ordering, fixed window sizes, disabled UI state, explicit timeouts).
- Coverage reporting (optional, nightly-only at first):
  - Add a code coverage toolchain (for example `cargo llvm-cov`) and publish reports as nightly artifacts.
  - Use code coverage as a guide, not as the primary KPI.
- Review checklist for new tests:
  - Determinism, fixture manifest updates, meaningful assertions, negative tests, and runtime bounds.

Exit criteria:
- The test suite remains fast on PRs, deep on nightlies, and low-flake over time.

## Standards (Apply To Every New Test)

Determinism checklist:
- No dependence on wall-clock time without tolerances.
- No implicit filesystem state (`ui_state.json` disabled during UI capture; use explicit temp dirs).
- Stable ordering (maps/sets serialized deterministically; CLI outputs stable).

Fixture checklist:
- If a test references `fixtures/generated/<file>`, the fixture must exist in the appropriate manifest.
- Prefer adding a scenario to `fixtures/manifest_cli_tests.yaml` over ad-hoc generation in tests.
- Respect the `--clean` behavior documented in `docs/maintainers/fixtures.md`.

Assertion checklist:
- Assert semantic outcomes, not implementation details.
- Include at least one negative/edge case when adding new parsing/diff logic.
- Prefer errors with stable kinds/codes (easy to match in tests and logs).

Baseline checklist (perf + UI):
- Only update baselines with explicit intent and reviewable artifacts.
- Never “chase green” by bumping baselines due to one-off noise.

## Appendix: Canonical Commands

CI-like local run:
```bash
python3 scripts/dev_test.py
```

Core + CLI smoke (mirrors smoke matrix shape):
```bash
generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean
TABULENSIS_LICENSE_SKIP=1 cargo test -p excel_diff
TABULENSIS_LICENSE_SKIP=1 cargo test -p excel_diff_cli
```

Robustness regressions:
```bash
cargo test -p excel_diff robustness_regressions
```

Perf quick + gate suites:
```bash
python3 scripts/check_perf_thresholds.py --suite quick --parallel --baseline benchmarks/baselines/quick.json \
  --export-json benchmarks/latest_quick.json --export-csv benchmarks/latest_quick.csv

python3 scripts/check_perf_thresholds.py --suite gate --parallel --baseline benchmarks/baselines/gate.json \
  --test-target perf_large_grid_tests
```

Full perf cycle (major perf-risk changes):
```bash
python3 scripts/perf_cycle.py pre
python3 scripts/perf_cycle.py post --cycle <cycle_id>
```

UI visual regression (one-shot):
```bash
scripts/ui_pipeline.sh compare_grid_basic
```

Seed + maintain fuzz corpora:
```bash
python3 scripts/seed_fuzz_corpus.py --config core/fuzz/seed_fixtures.yaml
python3 scripts/fuzz_corpus_maint.py --dry-run
```

