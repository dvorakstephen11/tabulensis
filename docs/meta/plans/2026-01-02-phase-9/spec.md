## Phase 9 implementation plan: Weird-file robustness loop

Phase 9 is explicitly: **build a “weird-file robustness loop”** where **corpus growth → fuzz findings → regression fixtures**, expanding the corpus with **nasty enterprise files** (Excel/PBIX/DataMashup) and treating **crashes/misparses as top-priority regressions**. 

This plan turns that sentence into an operational system that fits how this repo already works today: Rust core + `cargo fuzz`, fixture generation via `fixtures/manifest_cli_tests.yaml` + lock verification in CI, and existing structured error codes for container/package/datamashup failures.

---

# 1) Reality check: what already exists (and what Phase 9 should build on)

### 1.1 Fuzzing is already wired in, but the loop is not closed

* There is a `core/fuzz` crate with multiple fuzz targets already present (including `fuzz_open_workbook.rs`).
* The repo has a scheduled GitHub Actions fuzz workflow that runs a subset of fuzz targets for short time windows and uploads artifacts.
* The current committed fuzz corpora are **tiny** (a few seeds per target).

So: fuzz infra exists, but there isn’t yet a systematic “finding → minimized reproduction → committed regression fixture → test” pipeline.

### 1.2 A deterministic fixture system already exists and is enforced in CI

* CI runs a “fixture reference guard”, then generates fixtures from `fixtures/manifest_cli_tests.yaml`, then verifies checksums against `fixtures/manifest_cli_tests.lock.json`.
* The fixture generator supports:

  * deterministic workbook generation via `openpyxl`,
  * container corruption generators,
  * DataMashup mutation generators,
  * PBIX/PBIT generation from an XLSX DataMashup,
  * and **binary template copying** (`copy_template`) which is perfect for storing minimized regression files.

This is exactly the mechanism Phase 9 should use to “fixture-ize” fuzz findings.

### 1.3 The codebase already has “hardening hooks” we can lean on

* ZIP container limits, with explicit error codes and “zip bomb” defenses, already exist (`ContainerLimits`, `ContainerError`).
* DataMashup framing and nested package limits exist (`DataMashupError`, `DataMashupLimits`).
* Package-level errors are structured and expose stable `code()` values for assertions in regression tests (`PackageError::code`, `DataMashupError::code`, etc.).
* `DiffConfig` has hardening knobs like memory and timeout (useful for “corpus smoke runs” that must never hang CI). 

### 1.4 Phase 9 exists because the “tail risk” is real

Your completion analysis calls out that weird/legacy/future file robustness is partially addressed, but the **tail** (unknown enterprise weirdness) is still a key risk area, and fuzz/corpus/triage maturity is part of what determines whether this is truly “done.”

---

# 2) Definitions (so the loop is unambiguous)

These terms matter because Phase 9 is more “process + plumbing” than “one feature.”

### Corpus

A **corpus** is just a curated collection of input files (or extracted byte blobs) that represent real-world and adversarial shapes of data. For us, there are multiple corpora:

* Office containers (`.xlsx`, `.xlsm`)
* Power BI containers (`.pbix`, `.pbit`)
* Extracted binary blobs (e.g., raw `DataMashup` bytes)

### Fuzz finding

A fuzz finding is anything that indicates the system is not robust:

* panic/crash
* OOM-like behavior (runaway allocation)
* infinite loop / timeout
* “misparse” (accepts input but produces logically inconsistent output, nondeterminism, or violates invariants)

### Regression fixture

A regression fixture is a **minimized, committed** file (or generated file) that reliably reproduces a prior finding and is now enforced by tests/CI.

### Closed loop

We have a closed loop when, for any new finding:

1. it becomes a minimized reproduction,
2. it becomes a committed regression fixture,
3. it becomes a test,
4. the fix lands,
5. the minimized reproduction is fed back into fuzz corpora.

That is precisely what Phase 9 asks for. 

---

# 3) Phase 9 objectives and non-objectives

## Objectives

1. **Grow coverage** with “nasty enterprise files” (Excel/PBIX/DataMashup). 
2. **Make fuzzing bite deeper** on real entry points (`WorkbookPackage`, PBIX open).
3. **Institutionalize triage**: every crash/misparse becomes a regression fixture + test. 
4. **Keep CI stable**: robustness verification must be deterministic and fast under `cargo test` + fixture-lock verification.

## Non-objectives (still valuable, but not required for Phase 9 “done”)

* Full semantic validation against Excel as an oracle for all file types.
* Perfect modeling of every Excel feature; for unsupported features we still aim for graceful failure, not fidelity.
* Replacing existing fuzz infra; Phase 9 should extend and operationalize it.

---

# 4) Implementation plan overview (workstreams)

Phase 9 should be executed as **five parallel workstreams**, but with a clear ordering so you can ship iteratively.

1. **Corpus growth & curation** (what files we feed into fuzz and into “robustness smoke tests”)
2. **Fuzz surface expansion** (targets, harness safety, and CI fuzz coverage)
3. **Triage tooling** (make it cheap to convert artifacts into fixtures/tests)
4. **Regression fixture + test integration** (the “closing” part of the loop)
5. **Misparse detection** (define and enforce invariants beyond “no panic”)

Each workstream below includes concrete repo-level deliverables, file touchpoints, and acceptance criteria.

---

# 5) Workstream A: Corpus growth & curation

Phase 9 explicitly calls for expanding the corpus with nasty enterprise files for Excel/PBIX/DataMashup. 
The key is to do this **without poisoning the repo** with huge or sensitive files, while still getting deterministic regression coverage.

## A1) Establish a corpus taxonomy and storage model

Create a structured approach with three “tiers”:

### Tier 1: Public, small, deterministic fixtures (committed)

* These are produced by the existing fixture generator (`fixtures/manifest_cli_tests.yaml`) or copied from committed templates via `copy_template`.
* This tier must always be runnable in CI, because CI generates fixtures and runs tests. 

### Tier 2: Public “fuzz seed corpus” (committed)

* Small seed files stored in `core/fuzz/corpus/<target>/...`.
* Today these corpora are very small.
* Phase 9 should grow these significantly, but keep them curated and small enough to keep repo size sane.

### Tier 3: Private enterprise corpus (not committed)

* This is where you store real-world “nasty enterprise files” that cannot be checked into the repo (PII, licensing, sheer size).
* Phase 9 should still support this tier because it’s where the highest-value weirdness comes from; but it must not break OSS workflows.

**Deliverable:** `docs/robustness_corpus.md` describing:

* tiers,
* file naming conventions,
* privacy rules (hash-based names, no original names),
* how to run corpus smoke and fuzz locally with a private corpus.

## A2) Bootstrap corpus growth from existing deterministic fixtures

You already have a lot of “interesting surface area” fixtures defined in `fixtures/manifest_cli_tests.yaml`:

* container corruption cases (`random_zip.zip`, `no_content_types.xlsx`, `not_a_zip.txt`)
* DataMashup edge cases like duplicate parts/elements, base64 whitespace, UTF-16 encodings, corrupt base64
* PBIX fixtures generated from XLSX (`pbix_legacy_*`, `pbix_no_datamashup.pbix`, and even `.pbit` model fixtures)
* object graph fixtures (named ranges, charts) and binary template `.xlsm` fixtures via `copy_template` (VBA presence/changes)

**Action:** curate a list of “Phase 9 seed fixtures” and turn them into:

* seeds for fuzz targets
* smoke-test inputs for robustness tests

**Deliverable:** a simple config file, e.g. `core/fuzz/seed_fixtures.yaml`:

* lists fixture filenames
* maps each fixture to fuzz target(s) it seeds
* indicates whether it should be in the “robustness smoke set”

## A3) Add an ingestion script that turns fixtures into fuzz corpora

Create a script to refresh fuzz corpora deterministically from fixtures.

**Why this matters:**
If you hand-copy files into `core/fuzz/corpus`, the corpus will drift and become tribal knowledge. You want a one-command refresh that:

* reads generated fixtures from `fixtures/generated/`
* copies selected `.xlsx` / `.pbix` / `.pbit` files into the right fuzz corpus directories
* optionally extracts raw `DataMashup` bytes into the `fuzz_datamashup_parse` corpus

You already have working DataMashup extraction patterns in both:

* fixture generator PBIX builder (extracting DataMashup bytes from XLSX customXml)
* core tests extracting DataMashup base64 from fixture XLSX and decoding it 

**Deliverable:** `scripts/seed_fuzz_corpus.py` (or Rust tool under `core/bin/` if you prefer):

* input: list of fixture filenames + mapping (from A2)
* output:

  * `core/fuzz/corpus/fuzz_open_workbook/*` contains `.xlsx` / `.xlsm`
  * `core/fuzz/corpus/fuzz_open_pbix/*` contains `.pbix` / `.pbit` (new target; see Workstream B)
  * `core/fuzz/corpus/fuzz_datamashup_parse/*` contains extracted DataMashup blobs (stored as `.bin` or `.dm`)

**Important codebase reality note:**
The fixture reference guard regex currently only recognizes fixture references ending in `xlsx|xlsm|pbix|pbit|zip|txt`. 
If you introduce `.bin` fixtures and want them guard-checked too, update that regex as part of Phase 9 (Workstream D).

## A4) Private enterprise ingestion (optional but high value)

Add a second script:

**Deliverable:** `scripts/ingest_private_corpus.py`

* input: directory of “enterprise” files (xlsx/pbix/pbit)
* output:

  * a local-only hashed storage directory (e.g. `corpus_private/sha256_<hash>.xlsx`)
  * a metadata index JSON (sha256, size, type, source tag, date ingested)
* optional transforms:

  * strip workbook metadata if feasible
  * keep only minimal “repro” parts (hard; can be iterative)

This enables Phase 9 to meet the “nasty enterprise files” requirement without committing them.

---

# 6) Workstream B: Fuzz surface expansion and harness hardening

Today, your scheduled fuzz workflow exists, but it doesn’t fully match Phase 9’s objective (“Excel/PBIX/DataMashup”).

## B1) Promote fuzzing to hit the real entry points

### B1.1 Add workbook open fuzzing to CI schedule

There is already a `fuzz_open_workbook.rs` target. 
It currently:

* constructs strict-ish `ContainerLimits` and calls `OpcContainer::open_from_reader_with_limits(...)`
* then calls `WorkbookPackage::open(cursor)` separately (which may use default limits internally) 

**Implementation tasks:**

1. Update the fuzz harness to ensure **all opening paths use explicit limits**, not defaults.

   * Prefer calling `WorkbookPackage::open_with_limits(...)` (if present) or refactor workbook open API to accept limits cleanly.
2. Make the harness reset the default session per-iteration if the workbook open path uses `with_default_session` internally (to avoid unbounded string interning across fuzz iterations). (Your determinism tests already show the pattern of explicitly controlling sessions.)
3. Add `fuzz_open_workbook` into `.github/workflows/fuzz.yml` run list with safe options.

**Deliverable:** `fuzz.yml` updated so the scheduled job runs:

* `fuzz_datamashup_parse`
* `fuzz_m_section_and_ast`
* `fuzz_diff_grids`
* **`fuzz_open_workbook`** (newly added)

The fuzz workflow already uploads artifacts; we’re just increasing coverage.

### B1.2 Add PBIX open fuzzing (new fuzz target)

You have robust PBIX support in core:

* `PbixPackage::open(...)` and `open_with_limits(...)` exist. 
* PBIX fixtures exist in the fixture manifest and in tests.

**Deliverable:** create `core/fuzz/fuzz_targets/fuzz_open_pbix.rs`

* input: arbitrary bytes
* attempt: `PbixPackage::open_with_limits(Cursor::new(bytes), limits)`
* limits: choose something that prevents zip bombs but still allows realistic files (e.g., entries <= a few thousand, part <= a few MB, total <= tens of MB)

Then add it to the fuzz schedule.

## B2) Make fuzz targets “deep enough” to find real bugs

A fuzz target that only fails in the container layer isn’t worthless (it validates limits), but Phase 9 needs deeper coverage of:

* workbook XML parsing (sheet relationships, sharedStrings, workbook.xml, sheet xml)
* DataMashup extraction and parse/build
* PBIX schema fallback behavior (`NoDataMashupUseTabularModel` path)

**Concrete actions:**

* Seed corpora with a mix of:

  * valid small files (reach deep parsing)
  * deliberately broken containers (exercise failure paths)
  * boundary cases around limits (part sizes near limits, many entries)
* For PBIX fuzzing, include both:

  * legacy DataMashup-in-root PBIX (`DataMashup` present)
  * enhanced-metadata style cases (missing DataMashup but has `DataModelSchema`, from fixtures).

## B3) Add corpus minimization + hygiene

As corpora grow, keep them tight:

* Use `cargo fuzz cmin` periodically per target (manual step or a scripted maintenance step).
* Keep seed file counts manageable by aggressively deduping by hash and by “coverage utility” (if you can measure; otherwise by size + distinct feature tags).

**Deliverable:** `scripts/fuzz_corpus_maint.py`

* runs cmin for each target
* enforces max corpus size and max file size per seed (configurable)
* prints a report used in PR review

---

# 7) Workstream C: Triage tooling (turn artifacts into fixes cheaply)

Phase 9 succeeds when “finding → fixture + test” is **routine**, not heroic.

## C1) Standardize what gets triaged

Define a severity rubric:

* **P0:** panic/crash, UB, OOM, timeout/hang
* **P1:** deterministic misparse invariant violation (see Workstream E)
* **P2:** non-determinism (same input yields different output, or self-diff not empty)
* **P3:** low-priority spec deviations that don’t affect stability

**Deliverable:** `docs/fuzz_triage.md`

* how to reproduce locally from artifact
* how to minimize
* how to fixture-ize
* how to write the regression test
* how to add seed back into corpus

## C2) Add scripts that automate 80% of the triage loop

You already have a consistent directory layout:

* fuzz artifacts go under `core/fuzz/artifacts/<target>/...` (and CI uploads them).

**Deliverable 1:** `scripts/fuzz_triage.sh` (or `.py`)

* input: target name + artifact file
* runs:

  * reproduce: `cargo fuzz run <target> <artifact> -runs=1`
  * minimize: `cargo fuzz tmin <target> <artifact>`
  * (optional) shrink corpus: `cargo fuzz cmin <target>`

**Deliverable 2:** `scripts/add_regression_fixture.py`

* input: minimized artifact + classification (xlsx/pbix/datamashup-bytes) + short description
* outputs:

  * places the file into `fixtures/templates/regressions/<area>/...` (or another convention you choose)
  * appends a `copy_template` scenario into `fixtures/manifest_cli_tests.yaml` (or a dedicated “regressions” manifest if you want separation)
  * prompts for / auto-generates an expectation entry for tests (Workstream D)
  * reminds to regenerate lock file

This is grounded in reality because `copy_template` already exists and is designed to copy binary templates to outputs.

---

# 8) Workstream D: Regression fixture + test integration (closing the loop)

This is the “money” part: every finding becomes a fixture + test, and CI enforces it.

## D1) Decide how regression fixtures are represented

### Option D1.1 (recommended): store the full container file

* For workbook issues: store `.xlsx` / `.xlsm`
* For PBIX issues: store `.pbix` / `.pbit`

Pros:

* fixture guard already recognizes these extensions 
* fixture generator already verifies zip structural validity for zip extensions 
* repro is straightforward

### Option D1.2: store extracted bytes as `.bin` (useful for DataMashup-only bugs)

Pros:

* smallest reproduction for `parse_data_mashup` / `build_data_mashup`
  Cons:
* fixture reference guard currently won’t track `.bin` unless you expand its regex 

**Plan:** support both:

* default to container files
* allow `.bin` for DataMashup-only regressions, and update guard regex to include `.bin` if you adopt this.

## D2) Add a “robustness regression expectations” file

Create a single source of truth mapping fixture → expected outcome.

**Deliverable:** `core/tests/robustness_regressions.yaml` (or JSON)
Each entry includes:

* `file`: fixture name
* `type`: `xlsx|xlsm|pbix|pbit|dm_bytes`
* `expectation`:

  * `ok` or `error`
  * if error: expected `error_code` (e.g., `EXDIFF_CTR_003`, `EXDIFF_PKG_004`, `EXDIFF_DM_003`, etc.)
* `invariants`:

  * `no_panic` (always true)
  * `self_diff_empty` (only for `ok` cases)
  * `deterministic_open` (only for `ok` cases)

## D3) Implement a single Rust test that enforces all robustness regressions

Create: `core/tests/robustness_regressions_tests.rs`

It should:

1. Load the expectations file.
2. For each fixture:

   * open the file using the same APIs your product uses:

     * workbooks: `WorkbookPackage::open(...)` or `open_with_limits(...)`
     * pbix: `PbixPackage::open(...)` or `open_with_limits(...)`
   * if expected error, assert `.code()` matches expectation.
3. For “expected ok” fixtures, run misparse invariants (Workstream E) including:

   * open twice + diff with self should be empty (or op_count == 0)
   * no nondeterminism

### Why this matches the codebase well

* Error codes are already stable and accessible.
* `PbixPackage::diff_streaming` exists and is exercised in tests.
* CI already generates fixtures and runs tests; adding this test just rides the existing pipeline. 

## D4) Ensure fixture guard + lock verification cover new fixtures

CI currently:

* guards fixture references,
* generates fixtures,
* verifies checksums via lock file.

**Phase 9 action items:**

* If you introduce any new fixture extensions (like `.bin`), update:

  * `scripts/check_fixture_references.py` regex extensions list 
  * `fixtures/src/generate.py` zip extension logic only if you want zip validation for new types (probably not needed for `.bin`) 

---

# 9) Workstream E: Misparse detection (beyond “no crash”)

Phase 9 explicitly calls out “misparses” as regressions, not just crashes. 
The trick is to define misparse in a way that is:

* deterministic
* cheap to check
* grounded in your IR and diff architecture

## E1) Core invariant: self-diff must be empty

A powerful “oracle” that doesn’t require Excel itself:

> If I open the same file twice and diff them, the report must contain zero ops (or equivalent “no changes” outcome).

This catches:

* nondeterminism (ordering differences)
* unstable parsing of charts/objects
* hidden data-dependent behavior
* memory corruption style issues where output drifts

You already have streaming diff APIs and tests proving the streaming machinery works and is lifecycle-correct.

**Implementation detail:**
For PBIX, do `pkg.diff_streaming(&pkg2, ...)` or non-streaming `pkg.diff(...)` depending on what’s easiest for assertions.

## E2) Secondary invariants (cheap sanity checks)

For “expected ok” fixtures, add checks like:

* workbook has at least one sheet (or is consistent with workbook.xml)
* string table references are in bounds (you already have `StringId` header checks in JSONL sink tests) 
* DataMashup metadata invariants you already assert in smoke tests (e.g., item_path formatting, presence of formulas). 

## E3) Time/memory bounding for corpus smoke tests

Use `DiffConfig` hardening knobs to ensure corpus smoke tests can’t hang CI:

* set `timeout_seconds` and `max_memory_mb` when running corpus diffs in tests 

This is especially important when you start ingesting “nasty enterprise files” (even if they’re minimized).

---

# 10) Putting it all together: Step-by-step execution order

This is a recommended order that delivers value early and reduces risk.

## Step 1: Establish the Phase 9 corpus lists

* Create `docs/robustness_corpus.md`
* Create `core/fuzz/seed_fixtures.yaml`
* Decide the naming convention for regression files (e.g., `reg_<area>_<shortdesc>_<sha8>.xlsx`)

## Step 2: Build `scripts/seed_fuzz_corpus.py`

* Start by seeding from existing fixtures:

  * `one_query.xlsx`, `multi_query_with_embedded.xlsx`
  * DataMashup oddities: `duplicate_datamashup_parts.xlsx`, `mashup_utf16_le.xlsx`, etc.
  * PBIX: `pbix_legacy_multi_query_a.pbix`, `pbix_no_datamashup.pbix`, `.pbit` models
  * container corruption: `random_zip.zip`, `no_content_types.xlsx`

## Step 3: Expand fuzz targets in CI

* Add `fuzz_open_workbook` to scheduled fuzz and harden its harness for limits + session reset.
* Add `fuzz_open_pbix` and put it into the schedule.

## Step 4: Build the triage scripts

* `scripts/fuzz_triage.sh`
* `scripts/add_regression_fixture.py`

## Step 5: Add regression expectations + the single “robustness_regressions” test

* Implement `core/tests/robustness_regressions_tests.rs`
* Enforce:

  * correct error codes for expected failures
  * self-diff empty for expected ok
  * bounded time/memory via `DiffConfig`

## Step 6: Backfill: convert any existing known weirdness into explicit regressions

* Start by turning any previously manually handled weird fixtures into entries in the expectations file (even if they’re already covered by other tests).
* Ensure every “known weird” file has an explicit expected result.

## Step 7: Ongoing loop (the operational cadence)

* Weekly (or per release):

  * run scheduled fuzz (already exists)
  * triage new artifacts
  * convert each into:

    * minimized reproduction
    * committed regression fixture
    * test expectation entry
    * fix + corpus seed update

---

# 11) Definition of done (Phase 9 exit criteria)

Phase 9 is complete when all of the following are true:

### Closed-loop criteria

* Every fuzz crash / timeout / OOM / misparse is turned into:

  * a minimized reproduction file,
  * a committed regression fixture (via manifest + lock),
  * and a deterministic test that fails before the fix and passes after.

### Coverage criteria

* Fuzz CI includes coverage for:

  * DataMashup parse/build (already there)
  * M parsing (already there)
  * grid diff fuzzing (already there)
  * workbook open fuzzing (added in Phase 9)
  * PBIX open fuzzing (added in Phase 9; PBIX open exists and supports limits) 

### Corpus criteria

* `core/fuzz/corpus/*` is no longer “toy”: it contains a curated, documented set of seeds representing:

  * valid files with key features (charts, named ranges, vba, mashups)
  * invalid containers
  * DataMashup encoding and framing weirdness
  * PBIX missing/present datamashup and schema variants

### CI stability criteria

* CI remains deterministic:

  * fixtures generated and checksums verified via lock file
  * robustness regression tests run as part of `cargo test --workspace` and do not flake

---

# 12) Risks and mitigations (based on real repo behavior)

### Risk: fuzzing “open workbook” leaks memory via persistent session state

Mitigation:

* reset session per fuzz iteration (or ensure open APIs don’t intern unboundedly during fuzz runs)

### Risk: corpora become too large and slow CI/fuzz

Mitigation:

* strict per-file size limits
* cmin maintenance script
* keep “enterprise tier” corpora out of repo, but require minimized repros be committed

### Risk: “misparse” is too vague

Mitigation:

* define misparse operationally via invariants: self-diff empty, deterministic open, bounded runtime, stable error codes.

### Risk: adding new fixture extensions breaks fixture guard assumptions

Mitigation:

* either stay within existing extensions (`xlsx|xlsm|pbix|pbit|zip|txt`) or deliberately expand guard regex to include the new ones. 

---

## What you’ll have at the end of Phase 9

A system where:

* You can throw weird enterprise files at the project (safely),
* fuzz finds the sharp edges,
* and every sharp edge becomes a permanent regression test backed by deterministic fixture generation and CI lock verification.

That’s the “robustness flywheel” Phase 9 is calling for.
