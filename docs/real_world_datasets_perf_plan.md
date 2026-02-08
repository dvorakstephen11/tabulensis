# Real-World Datasets: Procurement + Test/Perf Program

**Last updated:** 2026-02-08

This document is a maximally detailed plan for building a **real-world dataset program** for Tabulensis:

- Find and procure real `.xlsx/.xlsm/.xltx/.xltm` and `.pbix/.pbit` files (public + private).
- Turn them into **repeatable performance workloads** and **regression tests**.
- Produce **actionable metrics** (time + memory + throughput) that validate and accentuate this codebase's performance claims.

This plan is written assuming an AI agent running in **Codex CLI** (model: `gpt-5.3-codex` with reasoning effort `xhigh`) will do the procurement and most of the implementation work. If your environment does not expose `gpt-5.3-codex`, fall back to the newest available Codex model (for example `gpt-5.2-codex`). The plan is designed to be executable: it includes concrete file layouts, scripts to add, checklists, and CI strategy.

Related docs you should treat as constraints/source-of-truth:

- Perf harness + policies: `docs/perf_playbook.md`, `benchmarks/README.md`, `AGENTS.md`
- Fixture discipline: `fixtures/README.md`, `docs/maintainers/fixtures.md`
- Private corpora ingestion patterns: `docs/robustness_corpus.md`, `scripts/ingest_private_corpus.py`

---

## Goals

1. **Realistic performance validation**
   - Measure open + diff performance on "in the wild" workbooks/packages, not just synthetic grids.
   - Validate that optimizations/generalizations actually help real customer-like inputs.

2. **Accentuate distinct performance strengths**
   - Create workloads that clearly demonstrate where Tabulensis is fast (or memory efficient) relative to naive approaches:
     - streaming diff paths
     - preflight bailouts
     - move detection / alignment performance on re-ordered data
     - JSONL emit performance for large diffs

3. **Keep the test suite disciplined**
   - No flaky internet dependencies in PR gates by default.
   - Deterministic datasets (pinned hashes) and deterministic transformations.
   - Clear licensing/PII rules so we never accidentally commit restricted data.

## Non-goals

- Building a massive, ever-growing dataset archive in git.
- Turning all real-world datasets into PR gates (nightly/deep validation is the right home).
- "Benchmark theater" (single noisy runs, no provenance, no dataset versioning).

---

## Terminology

- **Dataset**: a real file sourced externally (`.xlsx/.xlsm/...` or `.pbix/.pbit`).
- **Case**: a workload definition over one dataset or a pair of datasets (ex: `open_only`, `diff_identical`, `diff_small_edit`, `diff_row_reorder`).
- **Suite**: a named set of cases that can be run + exported, producing JSON/CSV results (similar to `quick`, `gate`, `e2e`).
- **Pinned**: reproducible by recording a canonical source URL and `sha256`.
- **Committed**: checked into this repo (rare; only small permissive-license files).

## Naming and tag conventions (keep it searchable)

Dataset ids:

- Format: `rw_<source>_<topic>_<year_or_version>` with optional suffixes like `_v2`
- Rules:
  - ASCII only, lowercase, underscore-separated
  - Avoid organization-internal names and any user/customer identifiers
  - Keep stable over time; prefer creating new ids for new versions rather than mutating an existing dataset id

Case ids:

- Format: `<dataset_id>__<workload>__<variant>`
- Examples:
  - `rw_example_city_budget_2024__diff_small_numeric_edit`
  - `rw_example_city_budget_2024__diff_row_block_swap_seed123`

Tag taxonomy (suggested starting set):

- Source/type: `xlsx`, `xlsm`, `pbix`, `pbit`, `real-world`, `derived`
- Stress axes: `openxml-parse-heavy`, `styles-heavy`, `sharedstrings-heavy`, `alignment-heavy`, `move-detection-heavy`, `emit-heavy`, `m-heavy`
- Structure hints: `many-sheets`, `mostly-numeric`, `mixed-types`, `wide-table`, `tall-table`, `sparse`
- Workload: `diff-identical`, `diff-typical`, `diff-adversarial`, `open-only`

---

## Codex CLI Agent: Required Capabilities and Setup

Codex CLI can:

- Run interactively in a local terminal to edit files + run commands.
- Use web search to find sources and confirm licenses.
- Run in non-interactive mode via `codex exec` and emit machine-readable JSON (optionally enforced by an output schema).
- Use sandboxing modes and network access controls (important for safe dataset downloads).

### Recommended Codex configuration (for dataset procurement)

Codex CLI configuration supports:

- user-level config: `~/.codex/config.toml`
- project overrides: `.codex/config.toml` (loaded only when you trust the project)

For dataset procurement, the agent needs **live web search** and usually needs **outbound network access** to download files.

To make the procurement agent reliable and autonomous, prefer a config that explicitly enables:

- Model: `gpt-5.3-codex` (or the newest available Codex model)
- Reasoning effort: `xhigh`
- Live web search
- Workspace-write sandbox with **network access enabled**

Example `config.toml` snippet:

```toml
model = "gpt-5.3-codex"
model_reasoning_effort = "xhigh"
# Use "live" to fetch up-to-date pages; "cached" uses an OpenAI-maintained index.
web_search = "live"
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
network_access = true
```

Notes:

- Codex security controls are a combination of **sandbox mode** (what it can do) and **approval policy** (when it asks). For fully autonomous procurement runs, you may need to adjust approval settings for networked commands; do this only when you trust the repository and the task.
- In interactive Codex CLI sessions, you can switch approval modes via `/permissions`:
  - `Auto` (default): works inside the workspace; still asks before using the network.
  - `Read-only`: consultative; no edits/commands without approval.
  - `Full Access`: can use the network and operate broadly without asking (use sparingly).
- Useful references:
  - Codex config reference: https://developers.openai.com/codex/config-reference/
  - Codex CLI reference: https://developers.openai.com/codex/cli/reference/
  - Codex security: https://developers.openai.com/codex/security/
  - GPT-5.3-Codex release notes / system card: https://openai.com/index/introducing-gpt-5-3-codex/

### Recommended execution styles for the agent

1) Interactive for repo changes:

- Use interactive Codex CLI when creating scripts, editing Rust tests, and iterating locally.

2) Non-interactive for repeatable procurement steps:

- Use `codex exec` to:
  - create dataset "cards" (structured metadata for each candidate dataset)
  - generate consistent tags/labels
  - draft manifest entries that can be mechanically validated

Example pattern:

```bash
codex exec --output-schema datasets/real_world/schemas/dataset_card.schema.json \
  "Find 5 permissive-license .xlsx datasets that stress sharedStrings and styles. Return JSON."
```

### Codex CLI flags the procurement agent should use (non-interactive)

When you want Codex to fully execute procurement steps without human interaction, `codex exec` supports:

- `--json` (aka `--experimental-json`): newline-delimited JSON events (machine-readable progress stream)
- `--output-schema <path>`: constrain the final response to a JSON Schema (structured output)
- `--output-last-message, -o <path>`: write the final assistant message to a file
- `--search`: enable live web search (equivalent to `-c web_search=live`)
- `--sandbox workspace-write`: run commands in a constrained sandbox suitable for repo-local work
- `--full-auto`: shortcut preset (sets approvals to on-request and sandbox to workspace-write)
- `-c, --config key=value`: inline config overrides for the run (repeatable)
  - Values parse as JSON when possible; otherwise they are treated as strings.
- `--model, -m <model>`: override the model for the run
- `codex exec resume --last [<follow-up prompt>]`: continue a previous non-interactive session

The only flag you should avoid in this program is:

- `--yolo` / `--dangerously-bypass-approvals-and-sandbox`: bypasses approvals and sandboxing. Use only inside an externally hardened environment (not recommended for dataset procurement).

Suggested command shape for dataset discovery (structured output saved to disk):

```bash
codex exec --json --sandbox workspace-write \
  -c web_search=live \
  -c sandbox_workspace_write.network_access=true \
  --output-schema datasets/real_world/schemas/dataset_candidates.schema.json \
  -o tmp/dataset_candidates.json \
  "Find 10 public datasets as .xlsx with explicit licenses; return JSON only per schema."
```

---

## Program Architecture (How Real-World Data Fits This Repo)

This repo already has a good separation:

- **Deterministic generated fixtures**: `fixtures/generated/` (gitignored; reproducible from manifests)
- **Private corpora** (hashed; gitignored): `corpus_private/` via `scripts/ingest_private_corpus.py`

We will extend this with a parallel public/real-world dataset program that keeps **data** mostly out of git while committing:

- dataset registry metadata
- licenses/attributions
- deterministic transformations (mutators)
- perf/test harness code
- baseline results (when appropriate)

### Proposed "Real-world corpus tiers"

Tier RW1: Committed micro-real datasets (small, permissive)
- Contents: a tiny curated set of small real files that are safe to commit.
- Purpose: keep at least 1-3 "real" datasets runnable in PR gates without internet.
- Rule: must be clearly permissive/public-domain, small (size budget), and documented.
- Repo-specific constraint: this repo enforces that tests reference files generated by fixture manifests (`scripts/check_fixture_references.py`). If RW1 datasets are used in PR-gated tests, integrate them as `fixtures/templates/` inputs and generate them into `fixtures/generated/` via a manifest entry (for example a simple "copy_template" generator), rather than referencing committed raw paths directly.

Tier RW2: Pinned public datasets (downloaded, not committed)
- Contents: medium/large public datasets stored in a gitignored cache.
- Purpose: nightly and local deep perf validation on realistic workloads.
- Rule: registry includes URL + `sha256` + license info; downloads are repeatable.

Tier RW3: Private enterprise datasets (existing pattern)
- Contents: local-only corpora under `corpus_private/` with hashed filenames.
- Purpose: high-value customer-like coverage without sharing data.
- Rule: never committed; no customer identifiers; hashed storage + minimal metadata only.

### Proposed directory layout (metadata committed; data cached locally)

```text
datasets/
  real_world/
    README.md                         # how to use the program
    registry.yaml                     # dataset definitions (URLs, sha256, license, tags)
    cases.yaml                        # workload definitions over datasets (open/diff/mutate)
    attributions/                     # human-readable attribution notes per dataset
    licenses/                         # license texts (or pointers) for committed datasets
    schemas/
      dataset_card.schema.json        # schema for codex exec outputs
      registry.schema.json            # optional: validate registry.yaml mechanically
corpus_public/                        # gitignored; content-addressed store for RW2 downloads
  index.json
  sha256_<...>.xlsx
  sha256_<...>.pbix
```

Notes:
- `corpus_public/` is intentionally symmetric with `corpus_private/`.
- `registry.yaml` is the single source of truth for what to download and why.
- `cases.yaml` defines *workloads* (including generated diff-pairs) so perf runs are consistent.

---

## Dataset Selection: What To Look For (and Why)

Real-world datasets should stress different hotspots:

### Stress axes (targeted)

1) Parse-heavy (OpenXML)
- Huge `xl/sharedStrings.xml`
- Many sheets
- Many styles/number formats
- Lots of sparse XML (large extents with few populated cells)

2) Diff-heavy (cell edits)
- Medium/large grids with many edited cells and mixed types
- Workbooks where data changes are "localized" (so preflight can bail out alignment/move detection)

3) Alignment/move detection heavy
- Datasets where rows/columns are re-ordered, block-moved, or partially duplicated
- Tables with key-like columns but imperfect keys (real-life alignment pain)

4) Emit/serialization heavy
- Cases that produce large diff outputs (exercise JSONL emit + buffering)

5) PBIX/PBIT: DataMashup and M-heavy
- Samples with many queries
- Query text diffs that are large but mostly similar

### Diversity axes (representativeness)

- Produced by different tools: Excel desktop, Google Sheets export, LibreOffice, Power BI, ETL tools.
- Different domains: budgets, logistics/inventory, timeseries, staffing, financial statements, survey results.
- Different structure: wide tables, tall tables, many worksheets, templates with formulas.

### What to avoid (unless in private tier)

- Anything with unclear licensing.
- Anything that plausibly contains PII (HR exports, patient data, student rosters, etc).
- Anything requiring login/API keys.
- Extremely unstable URLs (session-based, ephemeral signed URLs).

---

## Discovery Playbook (How the Agent Finds Candidate Datasets)

The procurement agent should be systematic and keep a trail.

### Search strategies

Use web search to locate datasets with:

- Direct `.xlsx` downloads from public data portals
- Archived versions (good for diffing "before/after")
- Stable, explicit licenses

Example queries (customize per region/domain):

- `site:.gov filetype:xlsx budget`
- `site:.gov filetype:xlsx payroll` (careful: likely PII; prefer aggregated summaries)
- `open data portal xlsx download license`
- `Power BI sample .pbix download license`
- `site:github.com extension:xlsx license` (only when the repo license clearly covers the file)

### Recommended public sources (high-signal starting points)

Prioritize sources that are:

- Explicitly licensed (public domain / CC / well-scoped terms)
- Versioned or periodically updated (lets you build real "before/after" diffs)
- Stable URLs (or stable APIs that yield deterministic files)

Examples (agent should still verify licensing per dataset):

- National or city open-data portals that publish spreadsheets directly.
- Statistical agencies that publish annual/monthly workbook reports (good natural diffs).
- Finance/budget portals with "FY2024 vs FY2025" workbook exports (often large but mostly numeric).
- Official vendor sample datasets for Power BI (`.pbix` / `.pbit`) with explicit redistribution terms.
- GitHub repositories that clearly license the *data files* (not just the code) and provide immutable release assets.

Avoid sources that require accounts, API keys, or have ambiguous/hostile terms of use.

### Dataset scoring rubric (how the agent should prioritize)

Have the agent assign a simple score to each candidate to build a focused initial suite:

- License clarity (0-3): none/unclear (0), terms-only (1), clear permissive/PD/CC (2), OSI-style or PD/CC0 (3)
- Stability (0-3): unstable URL (0), stable page but moving file (1), versioned file naming (2), immutable release asset/hash available (3)
- Stress value (0-4): none (0), moderate (1-2), strong single-axis stress (3), multi-axis stress (4)
- Size appropriateness (0-2): too small to matter (0), good (1), ideal (2)
- Sensitivity risk (0 to -5): possible PII (-5), ambiguous (-2), clearly aggregated/public (0)

Target initial shortlist:

- 8-15 candidates total
- 2-3 per stress axis (parse-heavy, alignment-heavy, emit-heavy, PBIX/M-heavy)
- 1-2 "hero" datasets that are slow/huge enough to accentuate wins (kept out of PR gates)

### Intake checklist (per candidate)

- [ ] Source is stable (direct URL, versioned path, or archive).
- [ ] License is explicit and compatible (record license URL + text).
- [ ] File type is supported by Tabulensis (`.xlsx/.xlsm/.xltx/.xltm` or `.pbix/.pbit`).
- [ ] File size is within the program budget (see below).
- [ ] Content is plausibly non-sensitive (no PII; no proprietary exports).
- [ ] Dataset has at least one clear stress axis tag (parse-heavy, alignment-heavy, etc).
- [ ] The dataset can be pinned with `sha256`.

### Size budgets (initial defaults; tune later)

- RW1 committed: aim < 2 MB each (hard cap 10 MB unless explicitly justified).
- RW2 cached: soft cap 200 MB per dataset; hard cap 1 GB (do not blow disks).
- Total `corpus_public/` cache: soft cap 5 GB; agent should refuse to exceed without explicit instruction.

---

## Procurement and Ingestion (Make Downloads Repeatable)

### Step 1: Create `datasets/real_world/registry.yaml`

Each dataset entry should include enough to reproduce the download and enforce governance:

- `id`: stable identifier used in tests/metrics
- `kind`: `xlsx|xlsm|xltx|xltm|pbix|pbit`
- `source_url`: canonical download URL
- `source_homepage`: where humans can read context
- `retrieved_at`: UTC ISO timestamp
- `sha256`: expected hash of the downloaded bytes
- `bytes`: file size
- `license`: SPDX id when possible, else `custom`
- `license_url`: canonical license or terms page
- `attribution`: short text; link to full note in `datasets/real_world/attributions/<id>.md`
- `tags`: list of stress axes / structure hints
- `notes`: optional (ex: known parse warnings)

Example entry (YAML):

```yaml
version: 1
datasets:
  - id: rw_example_city_budget_2024
    kind: xlsx
    source_homepage: "https://example.gov/open-data/budget"
    source_url: "https://example.gov/open-data/budget/fy2024.xlsx"
    retrieved_at: "2026-02-08T00:00:00Z"
    sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    bytes: 12345678
    license: "CC-BY-4.0"
    license_url: "https://example.gov/open-data/license"
    attribution: "Example City Open Data (Budget FY2024)"
    tags: ["openxml-parse-heavy", "styles-heavy", "mostly-numeric", "many-sheets"]
    notes: "Large sharedStrings + many styles; good parse/memory stress."
```

Attribution note template (`datasets/real_world/attributions/<id>.md`):

```markdown
# <dataset id>

## Summary
- What this dataset is (1-2 sentences)
- Why we use it (stress axes, representative structure)

## Source
- Homepage: <url>
- Direct download: <url>
- Retrieved at: <UTC timestamp>
- SHA256: <hex>
- Bytes: <int>

## License
- License: <SPDX id or "custom">
- License URL: <url>
- Any attribution requirements (credit line, link-back, etc)

## Safety/PII
- PII scan: pass/fail + date + tool version (if available)
- Notes: anything potentially sensitive observed

## Known quirks
- Parse warnings (if any)
- Known slow operations (if any)
```

### Step 2: Add a public ingestion script (mirror `ingest_private_corpus.py`)

Add a new script (planned):

- `scripts/ingest_public_corpus.py`

Behavior:

- Inputs: a directory of downloaded files.
- Outputs: content-addressed copies under `corpus_public/` named `sha256_<digest>.<ext>`.
- Writes/updates `corpus_public/index.json` containing:
  - sha256
  - bytes
  - extension
  - ingested_at
  - (optional) `dataset_id` if matched from `registry.yaml`

This gives a safe, deterministic storage layer independent of original filenames and directory structures.

Retention/pruning (planned):

- Add `scripts/prune_public_corpus.py` that:
  - reads `datasets/real_world/registry.yaml` (and optionally `cases.yaml`)
  - computes the set of referenced `sha256` blobs
  - deletes unreferenced blobs from `corpus_public/` (with a dry-run default)
- This keeps developer machines and CI caches from growing without bound.

### Step 3: Add a downloader that enforces hashes

Add a script (planned):

- `scripts/download_real_world_datasets.py`

Behavior:

- Reads `datasets/real_world/registry.yaml`
- Downloads into a temp dir (streaming)
- Computes sha256 + size and verifies against registry
- Ingests into `corpus_public/` (via `ingest_public_corpus.py`)
- Refuses downloads that exceed size budgets unless `--allow-large` is set

Make it resumable and safe:

- Use `curl -L --fail --retry ...` or Python `requests` streaming.
- Use timeouts.
- Record failures in a log file under `logs/` or `tmp/`.

Example usage (intended):

```bash
python3 scripts/download_real_world_datasets.py \
  --registry datasets/real_world/registry.yaml \
  --corpus-dir corpus_public \
  --tmp-dir tmp/real_world_downloads
```

### Procurement checklist (repo changes)

- [x] Add `datasets/real_world/registry.yaml`
- [x] Add `datasets/real_world/attributions/` and at least one dataset card example
- [x] Add `scripts/ingest_public_corpus.py`
- [x] Add `scripts/download_real_world_datasets.py`
- [x] Add `.gitignore` entry for `corpus_public/`

---

## Turning Single Datasets Into Diff Workloads (Pairs)

Most real datasets are single snapshots. Perf validation needs *workloads* that look like:

- identical vs identical (fast paths)
- snapshot vs lightly edited variant (typical edit)
- snapshot vs structurally re-ordered variant (alignment/move detection)
- version A vs version B (real historical diffs)

### Option A (best): Multi-version real datasets

Prefer datasets published periodically (monthly/annual), giving natural "before/after" pairs with realistic diffs.

Registry should support grouping:

- `family_id`: a dataset family
- `version`: a date or label

Case definitions can then reference two versions by id.

### Option B: Deterministic mutators (generate B from A)

Implement a mutator toolchain that creates B from A without rewriting the entire workbook.

Planned script(s):

- `scripts/mutate_openxml_xlsx.py` (zip-level XML patching for minimal diffs)
- `scripts/mutate_pbix.py` (limited; only if safe and supported)

Mutator modes (start small and safe):

- `cell_edit_numeric`: change N numeric cells in a single sheet (avoid sharedStrings complexity)
- `row_block_swap`: swap two row blocks by index (diff/alignment stress without requiring Excel UI semantics)
  - Future extension: `row_block_move` (true "cut/paste" move) if we want stronger move-detection realism
- `row_shuffle_partial`: shuffle within a bounded window (alignment stress)
- `column_insert`: insert a column with deterministic values (structural ops)

Hard rule:
- Do not introduce non-determinism. All mutations must be seedable and stable across runs.

Implementation guidance (avoid accidental "rewrite everything" diffs):

- Prefer zip-level patching:
  - Read the `.xlsx` as a zip.
  - Modify only the minimum required XML parts (typically one worksheet XML).
  - Repack without reordering or reformatting unrelated XML when possible.
- Avoid toolchains that normalize or rewrite the entire workbook (they create huge diffs and don't reflect typical user edits).
- When changing cells:
  - Prefer numeric cells stored inline in sheet XML to avoid sharedStrings churn at first.
  - Keep edits localized (same sheet, same region) so workloads model real "small edits in a large file".
- When doing structural moves:
  - Be explicit about what "move" means (row number changes plus corresponding cell refs).
  - Validate the resulting workbook still opens and that diffs contain the expected structural ops.

### Case registry (`datasets/real_world/cases.yaml`)

Define workloads as data:

- `case_id`: stable (used in perf metrics name)
- `dataset_a`: dataset id
- `dataset_b`: either another dataset id or a derived artifact
- `workload`: `open_only|diff_streaming_fast|diff_full|emit_jsonl|...`
- `diff_config`: preset (`default|fast|precise`) and any special hardening limits
- `limits`: container limits (zip safety), max memory, timeout
- `tags`: stress tags

Example cases (YAML):

```yaml
version: 1
cases:
  - case_id: rw_example_city_budget_2024__diff_small_numeric_edit
    dataset_a: rw_example_city_budget_2024
    dataset_b:
      derived_from: rw_example_city_budget_2024
      derivation:
        tool: mutate_openxml_xlsx.py
        mode: cell_edit_numeric
        seed: 123
        edits: 10
    workload: diff_streaming_fast
    diff_config: default
    limits:
      container:
        max_entries: 10000
        max_part_uncompressed_bytes: 536870912
        max_total_uncompressed_bytes: 1073741824
      hardening:
        max_memory_mb: 4096
        timeout_seconds: 1800
    tags: ["real-world", "diff-typical", "parse-heavy"]
```

---

## Correctness Coverage from Real-World Datasets (Not Just Perf)

Perf metrics without correctness guardrails are dangerous: "fast" can mean "skipped work" or "incomplete output". For real-world datasets, build correctness checks in parallel with perf workloads.

### Correctness tiers (mirrors the repo's fixture discipline)

1) RW1 correctness tests (PR-gated, no network)

- Only for small, permissively-licensed datasets.
- Integrate as fixture templates + manifest outputs (see note in RW1 tier).
- Add deterministic tests that assert:
  - open succeeds
  - diff completes (`report.complete == true`)
  - expected "shape" invariants (for example, sheet count, op_count bounds, or known specific ops)
  - JSON and JSONL outputs are well-formed and stable

2) RW2/RW3 correctness smokes (nightly / opt-in)

- Focus on: "no panics, no hangs, no pathological memory blowups, output is self-consistent".
- Use container limits and hardening to keep failures bounded.
- Prefer metamorphic assertions over brittle golden outputs:
  - diff(A, A) emits zero ops
  - op_count emitted by sinks equals reported op_count (already done in `core/tests/e2e_perf_workbook_open.rs`)
  - streaming and non-streaming diffs agree on basic invariants (op_count, completeness, error class)

### Suggested correctness checks to add alongside real-world perf cases

- "Completes under limits": does not exceed max entries/uncompressed bytes, does not timeout.
- "Warnings are explainable": warnings must be in an allowlist for that dataset/case; new warnings fail the test.
- "Deterministic metrics shape": metric keys present and sane (non-negative, consistent relationships like `total = parse + diff` where applicable).
- "Emit validation": JSONL can be streamed and parsed incrementally; JSON output conforms to schema.

---

## Perf/Test Harness Integration (How We Measure and Gate)

### Principles

- PR gates stay fast (no internet).
- Real-world suites are opt-in (local) or nightly (controlled runners).
- Every perf result must record dataset version (`sha256`) to stay interpretable.

### Metrics contract (what to measure and how to report it)

This repo already standardizes perf output via `PERF_METRIC ... key=value ...` lines (see existing suites in `core/tests/perf_large_grid_tests.rs` and `core/tests/e2e_perf_workbook_open.rs`). The real-world suite should reuse that format so it plugs into existing parsing/export patterns.

Minimum required raw metrics per case:

- `total_time_ms`, `parse_time_ms`, `diff_time_ms`
- `peak_memory_bytes`
- `rows_processed`, `cells_compared`
- `old_bytes`, `new_bytes`, `total_input_bytes` (or equivalent)

Recommended additional raw metrics (if the code already provides them):

- `signature_build_time_ms`, `move_detection_time_ms`, `alignment_time_ms`, `cell_diff_time_ms`
- `op_emit_time_ms`, `report_serialize_time_ms`
- storage/buffer bytes (grid, string pool, op buffers, alignment buffers)
- alignment/move counters (`anchors_found`, `moves_detected`)

Derived metrics computed by export scripts (not emitted by Rust):

- `input_mib`: `total_input_bytes / (1024*1024)`
- `throughput_mib_per_s`: `input_mib / (total_time_ms/1000)`
- `cells_per_s`: `cells_compared / (total_time_ms/1000)` (guard division-by-zero)
- `bytes_per_peak_mem`: `total_input_bytes / peak_memory_bytes` (rough efficiency signal)

Reporting requirements (JSON):

- Always include the dataset ids + their `sha256` and sizes so results are auditable.
- Include `git_commit`, `git_branch`, timestamp, runner info when available (OS, CPU, memory).
- Prefer median-of-N runs for any baseline comparison; avoid single-shot claims.

Noise discipline:

- Keep `--test-threads=1` for perf tests (consistent with existing suites).
- On shared runners, expect higher variance; treat RW2/RW3 results as advisory unless they are repeated.
- For changes expected to move results by <5-10%, increase sampling (5+ runs) and use median + IQR (similar to `scripts/perf_cycle_signal.py`).

### Add a new Rust perf test target for real-world cases (planned)

Add something like:

- `core/tests/e2e_perf_real_world.rs`

Rules:

- `#![cfg(feature = "perf-metrics")]`
- `#[ignore]` by default (nightly / manual)
- Use `PERF_METRIC <case_id> key=value ...` lines like existing suites
- Require an env var pointing at the corpus cache, ex:
  - `TABULENSIS_REAL_WORLD_CORPUS_DIR=corpus_public`

Test structure (recommended):

- One test per `case_id`, generated from `cases.yaml`.
- A generator script creates/updates the Rust test file, so adding a case is data-only.

Planned generator:

- `scripts/generate_real_world_perf_tests.py`
  - reads `cases.yaml`
  - writes `core/tests/e2e_perf_real_world.rs` with deterministic test list

### Export script for metrics (planned)

Add:

- `scripts/export_real_world_metrics.py`

Responsibilities:

- Ensure datasets are present (or instruct to run downloader first)
- Run the ignored tests:
  - `cargo test -p excel_diff --release --features perf-metrics --test e2e_perf_real_world -- --ignored --nocapture --test-threads=1`
- Parse `PERF_METRIC` lines into JSON
- Include dataset provenance:
  - dataset ids
  - sha256 of inputs
  - input bytes
- Write results under:
  - `benchmarks/results_real_world/<timestamp>.json`
  - `benchmarks/latest_real_world.json`
  - optionally CSV

### How to gate

Three gating modes, from safest to strongest:

1) Absolute budgets only (no baseline)
- Good early, when the suite is new and noisy.
- Example budgets: "must finish under 60s and under 2GB peak".

2) Baseline-relative budgets (with slack)
- Like `scripts/check_perf_thresholds.py`: compare to pinned baseline with slack.
- Requires stable runner + stable dataset bytes.

3) Perf-cycle integration for major changes
- Add RW suite to `scripts/perf_cycle.py` as an optional stage that runs when `--include-real-world` is set.
- This keeps the core policy: compare pre vs post on the same machine window.

### CI integration (recommended shape)

Real-world suites should generally run in nightly or workflow-dispatch CI jobs, not PR gates.

Suggested job outline:

1. Checkout repo + install toolchain.
2. Download pinned RW2 datasets:
   - `python3 scripts/download_real_world_datasets.py ...`
   - Verify `sha256` matches `datasets/real_world/registry.yaml`.
3. Run real-world correctness smokes (optional) and export metrics:
   - `python3 scripts/export_real_world_metrics.py --export-csv benchmarks/latest_real_world.csv`
4. Compare against baselines (if you adopt baselines):
   - either a dedicated `check_real_world_thresholds.py`
   - or integrate as a suite into `scripts/check_perf_thresholds.py`
5. Upload artifacts:
   - `benchmarks/latest_real_world.json`
   - `benchmarks/latest_real_world.csv`
   - any logs / summaries

Caching:

- Cache `corpus_public/` keyed by a hash of `datasets/real_world/registry.yaml` to avoid re-downloading when unchanged.
- Cache Cargo build artifacts as usual.

Runner stability:

- Prefer a dedicated runner pool or self-hosted runner for baselines.
- Keep CPU frequency scaling / background load as stable as practical.

---

## Governance: Licensing, Privacy, and Safety

### Licensing rules

- Do not download/commit datasets with unclear licenses.
- Prefer public-domain / permissive licenses for RW1 committed datasets.
- For RW2 cached datasets, it's still critical to record license and attribution, even if not committed.

Each dataset must have:

- a `license_url`
- a short license summary in `datasets/real_world/attributions/<id>.md`

### Privacy/PII rules

- Treat all real-world files as untrusted.
- Do not ingest customer data into `corpus_public/`.
- For private files, use the existing `corpus_private/` pattern:
  - hashed filenames
  - no original names
  - no customer identifiers

Recommended additional guardrail (especially before committing RW1 datasets):

- Add a lightweight PII scan step for candidate datasets before they enter RW1 or the "pinned public" registry:
  - Script: `scripts/pii_scan_dataset.py`
  - Inputs: a dataset file (or directory) from `corpus_public/` / downloads
  - Output: a short report (counts + example matches with aggressive redaction) and a pass/fail
  - Heuristics: email-like strings, SSN-like patterns, phone numbers, "Name:" fields, common ID patterns
  - Allowlist: public contact emails are common; allowlist known-safe domains with `--allow-email-domain <domain>`
  - For `.xlsx`: scan `xl/sharedStrings.xml` and sheet cell `<v>` text nodes where applicable
  - For `.pbix`: scan extracted query texts and metadata blobs where your extractor already surfaces text

Policy:

- RW1 (committed) datasets must pass the scan or be rejected.
- RW2 (cached) datasets should pass the scan; if they do not, keep them out of shared runners and treat them like RW3.

### Safety rules

- Never open downloaded spreadsheets in Excel during procurement.
- Apply container limits when parsing/diffing to avoid zip bombs:
  - reuse patterns from `core/tests/e2e_perf_workbook_open.rs`
- Prefer sandboxing for downloads and parsing.

---

## Execution Plan (Agent-Ready, End-to-End)

This is the end-to-end checklist a Codex CLI agent should execute.

### Phase 0: Define the program shape (metadata only)

- [x] Create `datasets/real_world/` directory and add `README.md`
- [x] Add `datasets/real_world/registry.yaml` with 0-3 starter entries (even if not downloaded yet)
- [x] Add `datasets/real_world/cases.yaml` with the intended schema and a few stub cases
- [x] Add schema(s) under `datasets/real_world/schemas/`

Exit criteria:
- The repo has a committed, reviewable place to put dataset metadata and workload definitions.

### Phase 1: Build the public cache pipeline

- [x] Add `.gitignore` entry: `corpus_public/`
- [x] Implement `scripts/ingest_public_corpus.py` (mirror `scripts/ingest_private_corpus.py`)
- [x] Implement `scripts/download_real_world_datasets.py` (enforce sha256 + budgets)
- [x] Add a `make`-like documented command sequence in `datasets/real_world/README.md`

Exit criteria:
- Agent can run one command to download and ingest pinned datasets into `corpus_public/`.

### Phase 2: Dataset introspection and tagging (optional but high ROI)

- [x] Add `scripts/inspect_dataset.py` to compute:
  - sheets count
  - sharedStrings count/bytes (if cheap)
  - style count (if available)
  - approximate used range per sheet
  - file size and zip entry summary
- [x] Include safety stats:
  - zip entry count, max entry uncompressed bytes, total uncompressed bytes (estimate)
  - compression ratio extremes (basic zip-bomb detection)
- [x] Store inspection results under `datasets/real_world/inspections/<id>.json` (committed)
- [x] Use inspections to choose which datasets become perf cases

Exit criteria:
- We can justify why each dataset exists (what it stresses), with data.

### Phase 3: Build deterministic diff pairs

- [x] Implement `scripts/mutate_openxml_xlsx.py` (start with numeric cell edit + row block swap)
- [x] Extend `cases.yaml` to include derived artifacts:
  - A: original
  - B: mutated output (written into `corpus_public/` as content-addressed)
- [x] Document mutation semantics and validate diffs are meaningful (ops > 0)

Exit criteria:
- For at least one dataset, we have `diff_small_edit` and `diff_row_block_swap` workloads.

### Phase 4: Add real-world perf tests + export

- [x] Implement `scripts/generate_real_world_perf_tests.py`
- [x] Add `core/tests/e2e_perf_real_world.rs` generated from `cases.yaml`
- [x] Implement `scripts/export_real_world_metrics.py` (JSON + CSV)
- [x] Add `benchmarks/results_real_world/` directory (gitignored or committed depending on policy)

Exit criteria:
- A single command produces `benchmarks/latest_real_world.json` with provenance and metrics.

### Phase 5: Make it maintainable (baselines + CI strategy)

- [x] Decide whether RW1 committed datasets exist (small, permissive)
  - Decision (current): RW1 does **not** exist; we use RW2 pinned public datasets downloaded into `corpus_public/`.
- [ ] If RW1 exists:
  - [ ] Add a `real-world` suite to `scripts/check_perf_thresholds.py`
  - [ ] Add `benchmarks/baselines/real-world.json`
- [x] If RW1 does not exist:
  - [x] Keep real-world suite as nightly/local only; document how baselines are compared
    - Baselines are compared via `scripts/export_real_world_metrics.py --baseline <path> --baseline-slack <ratio>`.
    - Nightly CI runner: `.github/workflows/perf_real_world.yml` (exports JSON/CSV and uploads artifacts).
- [x] Add doc updates:
  - [x] link from `docs/perf_playbook.md` to this doc
  - [x] document expected runtimes

Expected runtime (order-of-magnitude; depends on CPU and case set):
- 1-5 minutes total for the initial suite in `datasets/real_world/cases.yaml`
- individual "hero" workbooks can take ~1-2 minutes per diff case

Exit criteria:
- Clear "where it runs" story: PR-fast vs nightly-deep, with budgets and provenance.

---

## Prompt Templates (For the Procurement Agent)

These are short, copy/paste-able task prompts for a Codex CLI agent.

### Template A: Find candidate datasets

"Using web search, find N public datasets with direct `.xlsx` downloads and explicit permissive licenses. Prefer files that stress: [axes]. For each candidate return a JSON object with: id, homepage_url, direct_download_url, license_url, license_summary, estimated_size, tags, and why it stresses the system."

### Template B: Pin and record a dataset

"Download dataset `<url>` into a temp dir. Compute sha256 and size. If the license is acceptable, add an entry to `datasets/real_world/registry.yaml` and add an attribution note in `datasets/real_world/attributions/<id>.md`."

### Template C: Create a diff workload

"Given dataset `<id>` (from `corpus_public/`), generate a deterministic mutated variant using `mutate_openxml_xlsx.py` mode `<mode>`. Add a case to `datasets/real_world/cases.yaml` and validate running the diff produces ops and PERF_METRIC output."

### Template D: Export real-world metrics

"Run the real-world perf export script, write `benchmarks/latest_real_world.json` and a CSV, and summarize the slowest cases and peak-memory cases with their dataset ids and sha256."

---

## Appendix: Suggested Initial Case Set (Minimal, Actionable)

Start with a small set of cases; expand only when the pipeline is working.

1) `rw_open_heavy_sharedstrings`
- workload: open + diff (small edit)
- goal: stress sharedStrings parsing + memory

2) `rw_diff_alignment_row_block_swap`
- workload: diff with row block swap
- goal: alignment-heavy path on non-synthetic data

3) `rw_emit_jsonl_large_ops`
- workload: diff then emit JSONL
- goal: measure op emit + serialization overhead

4) `rw_pbix_m_query_diff`
- workload: extract + diff M query text
- goal: PBIX performance realism
