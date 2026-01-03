# Detailed Implementation Plan for the Recommendations

Below is a concrete, engineer-ready implementation plan that turns the recommendations into a set of executable workstreams. I’ve organized it as “R1…Rn” so it’s easy to track in issues/PRs, with explicit code touchpoints, sequencing, test additions, and acceptance criteria.

Where relevant, I anchor the plan to:

* **Spec requirements** (e.g., LAPJV-based move detection, column alignment, metrics)
* **Known implementation gaps / intentional deviations** (global move extraction + validation is explicitly called out as future work)
* **Testing plan milestones** (G8–G13, M7.2–M7.4, etc.)
* **Current perf baseline** (notably ~7.8s on 50k dense scenarios) 

---

## R1 — Implement full global move-candidate extraction + validation for AMR alignment

### Outcome

Bring the row-alignment engine from “opportunistic in-gap move detection” to the full spec shape: **global candidate extraction → optimal assignment (LAP) → validation → gap consumption**, improving correctness on complex multi-block moves and stabilizing behavior on adversarial layouts.

### Why this exists

The alignment module explicitly documents the deviation (“no global move-candidate extraction phase”, “no explicit move validation phase”) and move_extraction.rs has a concrete TODO list for what’s missing.

### Implementation steps

#### R1.1 — Define a first-class “unanchored match set”

**Touchpoints**

* `core/src/alignment/anchor_discovery.rs`
* `core/src/alignment/anchor_chain.rs`
* `core/src/alignment/move_extraction.rs`
* `core/src/grid_metadata.rs` / `core/src/grid_view.rs` (RowMeta + frequency classes)

**Work**

1. After anchors/LIS are computed, collect all row indices that remain **unaligned** on each side.
2. Build a representation of “unanchored matches” keyed by signature:

   * Prefer `Unique` and `Rare` signatures.
   * Skip `LowInfo` rows (already a concept).
3. Create a compact structure:

   * `HashMap<RowSignature, SmallVec<[u32; N]>>` for A and B indices
   * `Vec<(a_idx, b_idx)>` candidate pairs for signatures meeting constraints

**Acceptance criteria**

* Unit tests cover:

  * LowInfo rows are ignored.
  * Unique/Rare rows drive candidate generation.
  * Heavy repetition is bounded by config (max candidates per signature, etc.).

#### R1.2 — Candidate block construction from unanchored pairs

**Touchpoints**

* `core/src/alignment/move_extraction.rs`
* `core/src/alignment/gap_strategy.rs`

**Work**

1. Group candidate pairs into **block move candidates**:

   * Sort pairs by `(a_idx, b_idx)`.
   * Identify contiguous runs with stable offset (classic move signature).
2. Build candidate blocks with:

   * `src_range`, `dst_range`, `len`
   * a block signature summary (e.g., multiset hash / rolling hash of row signatures)
3. Filter candidates using config thresholds:

   * min block length
   * minimum similarity score
   * max candidates overall to cap worst-case time

**Acceptance criteria**

* Produces block candidates on G11-style scenarios (row block moved intact). 

#### R1.3 — Sparse cost matrix + LAPJV assignment

**Definitions (so future maintainers don’t have to context-switch)**

* **Hungarian algorithm / LAP**: solves optimal one-to-one assignment between candidates by minimizing total cost.
* **LAPJV**: a fast practical solver for the Linear Assignment Problem, typically faster than textbook Hungarian.

**Touchpoints**

* New module: `core/src/alignment/lap.rs` (or `core/src/alignment/lapjv.rs`)
* `core/src/alignment/move_extraction.rs`

**Work**

1. Construct a **sparse** cost matrix between candidate “deleted blocks” and “inserted blocks”:

   * cost = `1 - similarity(blockA, blockB)` per spec 
   * only include pairs above a similarity threshold (keeps matrix sparse)
2. Solve with LAPJV:

   * Either implement LAPJV internally (preferred if you want deterministic + dependency control),
   * or use a small, audited crate and wrap it behind a trait to keep swapability.
3. Emit globally optimal set of non-overlapping move matches.

**Acceptance criteria**

* Deterministic assignment results across runs.
* Candidate count remains bounded under config caps.
* Perf: assignment time stays negligible on typical sheets (candidate sizes small).

#### R1.4 — Move validation / conflict resolution

**Touchpoints**

* `core/src/alignment/move_extraction.rs`
* `core/src/alignment/assembly.rs`

**Work**

1. Ensure assigned moves don’t overlap in source or destination:

   * If overlaps exist, resolve by:

     * higher similarity score wins,
     * then longer block wins,
     * then stable tie-break (by (src_start, dst_start))
2. Validate a move by “cheap verification”:

   * verify that signatures match across the proposed ranges
   * optionally spot-check a few cells for very high repetition regimes (configurable)

**Acceptance criteria**

* No overlapping moves are emitted.
* G13 “move + edit inside moved block” produces a move op plus minimal cell edits (not delete+insert). 

#### R1.5 — Integrate validated moves into gap filling

**Touchpoints**

* `core/src/alignment/gap_strategy.rs`
* `core/src/alignment/assembly.rs`

**Work**

* Extend gap processing so that validated moves are consumed before falling back to in-gap scanning:

  * New gap strategy: `GapStrategy::ConsumeGlobalMoves`
  * Carve gaps based on move ranges, then recurse/align remaining fragments.

**Acceptance criteria**

* Existing “simplified” behavior still works as fallback when global extraction bails (due to caps/limits).
* Metrics report moves detected and non-zero move detection time when this path triggers. 

---

## R2 — Bring column alignment up to parity with row alignment (including mid-sheet inserts)

### Outcome

Implement column-signature alignment within aligned row blocks, matching the spec’s “Hybrid Alignment strategy” for columns. 

### Implementation steps

#### R2.1 — Make “axis alignment” generic

**Touchpoints**

* `core/src/row_alignment.rs`
* `core/src/column_alignment.rs`
* `core/src/alignment_types.rs`

**Work**

* Factor the shared machinery (signatures → anchors → LIS → gaps) into an axis-generic implementation:

  * `Axis = Row | Col`
  * `Meta = RowMeta | ColMeta`
* Keep row/col wrappers thin to preserve clarity.

**Acceptance criteria**

* No behavior regressions in existing row alignment tests.
* Column alignment uses identical tuning knobs where appropriate (gap thresholds, recursion cap).

#### R2.2 — Implement G9 and G12a fixtures as gating correctness

**Touchpoints**

* `core/tests/g9_column_alignment_grid_workbook_tests.rs`
* `fixtures/src/generators/grid.py` (or whichever generator is used)
* `fixtures/manifest.yaml` already includes relevant scenarios 

**Work**

* Ensure test coverage for:

  * single column insert/delete in the middle (G9) 
  * column move (G12a) 

**Acceptance criteria**

* Emits `ColumnAdded/ColumnRemoved` for insert/delete mid-sheet.
* Emits `BlockMovedColumns` (or equivalent) for column move without spurious cell edits.

---

## R3 — Improve dense-grid performance (bring “50k dense” into target range)

### Outcome

Reduce the current ~7.8s runtime on dense 50k scenarios by removing avoidable overhead (hash-map dominated access patterns, double-scanning, uninstrumented phases). Baseline: `perf_50k_dense_single_edit` and `perf_50k_dense_completely_different` are ~7.8s today. 
This directly serves the product promise of “instant diff” on large files. 

### Implementation steps

#### R3.1 — Identify the real hot path with phase-complete metrics

Right now the benchmark file shows big “total” time with partial phase detail.

**Touchpoints**

* `core/src/perf.rs`
* `core/src/engine/*` (where guards are placed)

**Work**

* Ensure all major phases are timed:

  * workbook open/parse (future metric in spec) 
  * IR build
  * row/col signature build
  * positional diff loop
  * op emission + report serialization (if included)
* Add counters for:

  * hash map lookups (estimated)
  * allocations (approx via internal counters)

**Acceptance criteria**

* A perf run can attribute >90% of time to named phases (not “dark matter”).

#### R3.2 — Add a dense storage mode for grids (without breaking API)

This is the most structurally impactful performance upgrade.

**Touchpoints**

* `core/src/workbook.rs` (Grid)
* `core/src/grid_parser.rs`
* `core/src/grid_view.rs`
* `core/src/engine/grid_primitives.rs` and any code that iterates cells

**Work**

1. Introduce:

   * `enum GridStorage { Sparse(FxHashMap<...>), Dense(DenseGrid) }`
2. Decide storage at construction:

   * if `filled_cells / (nrows*ncols)` exceeds threshold OR if source indicates “dense used range”, choose Dense.
3. Dense layout:

   * row-major `Vec<CellCompact>` sized `nrows*ncols`
   * keep `CellCompact` small (value tag + pooled string id + optional formula id)
4. Implement `get_cell(row,col)` and iterators with storage-specific fast paths.

**Acceptance criteria**

* Dense perf tests show significant improvement (expect multi-second drop on 5M cell comparisons).
* Sparse behavior is unchanged and memory remains bounded on sparse workbooks.

#### R3.3 — Avoid double work in “near-identical same-shape” cases

If you already compute row signatures for preflight, don’t then do a full positional compare that repeats the same work at the cell granularity.

**Touchpoints**

* `core/src/engine/grid_diff.rs` (preflight + diff selection)
* `core/src/grid_view.rs` (row signature materialization)

**Work**

* For same-shape grids:

  1. Compare row signatures first.
  2. If only K rows differ, only compare cells within those rows (plus optional context rows).
  3. Emit ops from the minimal set.

**Acceptance criteria**

* Single-cell edit case scales closer to “O(total rows + changed row width)” rather than “O(all cells)”, while still correct.

---

## R4 — Implement deferred metrics: parse_time_ms and peak_memory_bytes + stable metrics export

### Outcome

Deliver the spec’s deferred metrics and make perf regressions observable in CI. 

### Implementation steps

#### R4.1 — parse_time_ms

**Touchpoints**

* `core/src/excel_open_xml.rs` (or workbook open path)
* `core/src/package.rs` / `core/src/container.rs`
* `core/src/engine/diff_workbooks*` entrypoints

**Work**

* Start a parse timer at workbook open and stop when IR is built.
* Ensure metrics capture:

  * per-workbook parse time
  * per-sheet parse time (optional, but useful)

**Acceptance criteria**

* parse_time_ms is non-zero in perf runs that open files.

#### R4.2 — peak_memory_bytes

**Touchpoints**

* `core/src/memory_metrics.rs` (exists)
* `core/src/string_pool.rs`
* `core/src/workbook.rs` (Grid allocations)
* allocator integration (feature-gated)

**Work**

* Implement a conservative “accounted allocations” model:

  * track sizes/capacities of major Vec/HashMap pools
  * track string pool bytes
* Optionally integrate with allocator stats behind a feature for native builds.

**Acceptance criteria**

* peak_memory_bytes is stable/deterministic across runs (within small variance).
* Memory spikes correlate with known large allocations.

#### R4.3 — Metrics export contract

**Touchpoints**

* CLI: `cli/src/commands/diff.rs`
* Core: `core/src/perf.rs` (serialize metrics)
* CI scripts already exist in plan; align their expectations.

**Work**

* Add `--metrics-json <path>` (or feature-gated env var) so CI can always scrape metrics without parsing human text.

**Acceptance criteria**

* CI perf workflows consume JSON metrics directly (less brittle than log parsing).

---

## R5 — Tighten architectural boundaries and make the diff driver more “inevitable”

### Outcome

Make the system easier to extend (PBIX/DAX/WASM) by hardening boundaries and turning the orchestration into simple, testable units (per spec’s modularity patterns).

### Implementation steps

#### R5.1 — Adopt a “Diffable” shape at the boundaries (without over-engineering)

The spec explicitly proposes a Diffable trait to keep orchestration clean. 

**Touchpoints**

* `core/src/diff.rs`
* `core/src/workbook_diff.rs`
* `core/src/m_diff.rs`
* `core/src/formula_diff.rs` (if present)

**Work**

* Introduce a config-aware variant:

  * `trait Diffable { type Output; fn diff(&self, other: &Self, ctx: &DiffContext) -> Self::Output; }`
* Implement incrementally:

  1. Workbook-level structure diff
  2. Sheet diff
  3. Grid diff
  4. Query diff

**Acceptance criteria**

* The top-level diff driver becomes a thin composition layer.
* Unit tests can diff a single component without constructing the entire engine session.

#### R5.2 — Enforce dependency direction with feature-gated IO

**Touchpoints**

* `core/Cargo.toml` feature flags
* `core/src/lib.rs` exports
* WASM crate boundaries

**Work**

* Ensure:

  * domain + algorithms can compile without file/OS APIs
  * workbook opening/parsing sits behind explicit features

**Acceptance criteria**

* WASM builds remain green without conditional compilation hacks leaking everywhere.

---

## R6 — Complete semantic M diff: step-aware DP alignment + GumTree/APTED hybrid

### Outcome

Ship the differentiator the docs call out: step-aware semantic diffing plus move-aware AST differencing at scale.

### Implementation steps

#### R6.1 — Step-aware alignment via costed sequence diff (DP)

Spec expectation: represent queries as step sequences and align them with a DP cost model.

**Touchpoints**

* `core/src/m_ast/step_model.rs`
* `core/src/m_semantic_detail.rs`
* `core/src/m_diff.rs`

**Work**

* Implement DP alignment:

  * match cost = 0 when (kind + key params) equal
  * substitution cost when same kind but param changed
  * insert/delete costs
* Emit structured step changes (added/removed/modified/reordered).

**Acceptance criteria**

* M7.3 fixtures produce structured semantic differences for common user edits (filters, removed columns, join change). 

#### R6.2 — Decide and codify policy for formatting-only changes

The testing plan says formatting-only changes should not surface as semantic ops, while current tests allow a “FormattingOnly” classification.

**Work**

* Make this a config policy:

  * `SemanticNoisePolicy::SuppressFormattingOnly | ReportFormattingOnly`
* Default can remain “report as formatting-only” but UI/consumers can hide it.

**Acceptance criteria**

* Both behaviors are supported and tested.

#### R6.3 — Implement GumTree + APTED hybrid for AST diffs

The testing plan and spec explicitly call this out (move/rename detection at scale + exactness for small/deep trees).

**Touchpoints**

* `core/src/m_ast.rs`
* `core/src/m_semantic_detail.rs`
* New module(s): `core/src/m_ast_diff/gumtree.rs`, `core/src/m_ast_diff/apted.rs`

**Work**

* Build an AST node representation suitable for diffing:

  * stable node ids
  * labels normalized per canonicalization rules
* Implement:

  * GumTree matching (top-down + bottom-up)
  * APTED exact TED (bounded to small/medium N or unmatched sub-forests)
* Produce:

  * move/rename-aware edit script (or at minimum, robust summary that can later be expanded)

**Acceptance criteria**

* M7.4 test suite passes:

  * deep skewed completes reliably
  * large refactor identifies move
  * wrap/unwrap reports structural insert (not text-only). 

#### R6.4 — Query rename detection via matching (Hungarian)

Spec calls for rename detection using similarity + matching.

**Touchpoints**

* `core/src/m_diff.rs`
* `core/src/object_diff.rs` (if shared rename logic exists)
* New small matching module: `core/src/matching/hungarian.rs`

**Work**

* For unmatched queries:

  1. compute signatures (step-kind multiset + normalized AST hash)
  2. build candidate matrix for likely renames
  3. solve assignment for best rename set
  4. emit `QueryRenamed`

**Acceptance criteria**

* Works on “one query renamed” and ambiguous rename scenarios (stable output).

---

## R7 — Hardening: limits, partial results, and error contracts

### Outcome

Make the engine’s behavior under limits *explicit* and reliable, matching the spec’s limit-handling semantics and keeping CLI behavior predictable.

### Implementation steps

#### R7.1 — Make partial-result semantics consistently observable

**Touchpoints**

* `core/src/engine/hardening.rs`
* `core/src/diff.rs` (DiffReport.complete / warnings)

**Work**

* Ensure `LimitBehavior::ReturnPartialResult`:

  * sets `report.complete = false`
  * includes warnings tied to sheet/object scope
* Ensure consumers (CLI/JSON) surface this clearly.

**Acceptance criteria**

* Integration tests confirm warnings appear and exit codes behave consistently.

#### R7.2 — Normalize error typing and mapping across CLI/core

**Touchpoints**

* `core/src/error_codes.rs`
* `cli/src/commands/diff.rs`

**Work**

* Define a single mapping table:

  * invalid args -> exit 2
  * diff exists -> exit 1
  * success/no diff -> exit 0
  * internal failure -> exit 3 (or another reserved)
* Ensure “try_” APIs return `Result` rather than panic.

**Acceptance criteria**

* Existing CLI integration tests remain green and new tests cover additional error cases.

---

## R8 — Testing + fixtures + perf gating: make the plan executable in CI

### Outcome

Turn the testing plan’s Phase 4+5 items into a continuous safety net: correctness, adversarial resilience, and performance regression detection.

### Implementation steps

#### R8.1 — Land missing gating fixtures and tests for G8a–G13

**Touchpoints**

* `fixtures/manifest.yaml` (many are already sketched) 
* `core/tests/g10_*.rs`, `g11_*.rs`, `g12_*.rs`, `g13_*.rs`
* Perf tests for adversarial repetitive: enforce a strict timeout and correctness. 

**Acceptance criteria**

* G8a confirms:

  * no super-linear blowups
  * correct identification of inserted blank-row block as RowAdded, not mass edits. 

#### R8.2 — Make perf thresholds meaningful and stable

**Touchpoints**

* `.github/workflows/perf*.yml`
* `scripts/check_perf_thresholds.py`
* benchmark artifacts like `benchmark_results.json` 

**Work**

* Create two tiers:

  * “quick” perf suite: small enough for PR gating
  * “fullscale” perf suite: nightly + release gating
* Gate on:

  * total time
  * key phase times (once R4 is done)
  * memory ceiling deltas

**Acceptance criteria**

* Perf regressions are caught as close to introduction as possible.
* Thresholds are tied to metrics, not log parsing.

#### R8.3 — Fuzzing integration

**Touchpoints**

* `core/fuzz/*`, top-level `fuzz/*`
* add corpus seeds for DataMashup framing and grid diff

**Work**

* Add a nightly/weekly CI job that runs fuzz for bounded time.
* Add “crashers” to corpus automatically via CI artifacts.

**Acceptance criteria**

* Any new crash results in a minimized reproducer and a regression test.

---

## R9 — Future readiness: PBIX/WASM seams and post-MVP DAX/model scaffolding

### Outcome

Keep the architecture welcoming to the roadmap without bloating the MVP path: PBIX support, WASM viewer, DAX/model diff stubs.

### Implementation steps

#### R9.1 — PBIX host support using shared DataMashup extraction

Testing plan expects legacy PBIX DataMashup extraction and a structured error for tabular-only PBIX.

**Touchpoints**

* `core/src/container.rs`
* `core/src/datamashup_package.rs`
* `core/tests/pbix_host_support_tests.rs`

**Acceptance criteria**

* `legacy.pbix` extracts mashup bytes and diffs queries.
* “enhanced metadata pbix” returns `NoDataMashupUseTabularModel` (or equivalent), never panics. 

#### R9.2 — WASM compatibility guardrails

**Touchpoints**

* `wasm/src/lib.rs`
* `core/src/bin/wasm_smoke.rs` 

**Work**

* Keep IO behind feature flags.
* Ensure no accidental `std::fs` usage in WASM build.

**Acceptance criteria**

* WASM smoke test stays green.

#### R9.3 — DAX/model diff stubs (post-MVP, but prepare the seam)

**Touchpoints**

* New IR module: `core/src/model.rs`
* New diff module: `core/src/model_diff.rs`
* Extend DiffOp minimally (feature-gated)

**Acceptance criteria**

* IR and diff entrypoints exist without requiring full parser immediately.

---

# Recommended sequencing (dependency-aware, minimal rework)

1. **R4 (metrics) first**: without phase-complete metrics, performance work is guesswork. 
2. **R3 (dense perf) next**: it directly attacks current benchmark gaps.
3. **R1 + R2 (alignment completeness)**: global move extraction + column parity unlock G8–G13 reliability.
4. **R6 (semantic M diff)**: build the flagship differentiator with strong tests (M7.3/M7.4).
5. **R7 + R8 (hardening + CI gates)**: keep future work safe and deterministic.
6. **R9 (PBIX/WASM/DAX seams)**: land as clean extensions without contaminating core.

---

# Definition of done (global)

A recommendation is “implemented” only when all of the following are true:

* **Correctness**: the relevant fixtures/tests (G*, M*, D*) exist and pass. Make sure all tests pass (so you must run them) and performance metrics are run.
* **Performance**: key perf tests are tracked and don’t regress; improvements are visible against the current baseline JSON. 
* **Observability**: metrics identify where time/memory goes (including parse + peak memory). 
* **Maintainability**: new algorithms are isolated behind narrow modules/traits, with deterministic tie-breaking.

