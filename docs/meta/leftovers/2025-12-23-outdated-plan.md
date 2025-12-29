Below is a 7‑branch delivery plan that focuses on the *actual* gaps implied by the current implementation + the testing/spec roadmap:

* **PBIX/PBIT inputs aren’t supported yet** because your container layer hard‑requires `[Content_Types].xml` (OPC) and your DataMashup extraction path only scans Excel `customXml/item*.xml` for base64 content.
* The **M parser is intentionally shallow**: anything beyond `let`, record/list literals, basic calls, and primitives falls back to `Opaque(...)`.
* Your M diff logic can currently classify **FormattingOnly vs Semantic** by canonical AST hash, but it cannot yet emit **step‑aware structured semantic detail**, which the testing plan explicitly calls for.
* You already have **embedded mini‑packages extracted** in `PackageParts` (and tests around BOM stripping), but the domain diff still needs to explicitly diff embedded contents as a first‑class surface.
* Benchmarks show at least one “should be faster” scenario: **50k dense single edit ~7.8s** in latest results, while the target plan calls out tighter budgets for dense near‑identical cases.

---

## Branch map (7 feature branches)

|  # | Branch name (suggested)                    | Primary goal                                                                         | Depends on                       |
| -: | ------------------------------------------ | ------------------------------------------------------------------------------------ | -------------------------------- |
|  1 | `2025-12-22-pbix-host-support`             | Open `.pbix/.pbit`, extract root `DataMashup`, diff M+metadata                       | —                                |
|  2 | `2025-12-23-m-parser-tier1-ident-access`   | Parse identifiers + access chains + `if`/`each` into structured AST                  | —                                |
|  3 | `2025-12-24-m-parser-tier2-ops-fn-types`   | Add operators, lambdas, type ascription, `try/otherwise`, precedence                 | #2                               |
|  4 | `2025-12-26-m-step-model-extraction`       | Build an M “step pipeline” model from `let` bindings                                 | #2–#3                            |
|  5 | `2025-12-28-m-semantic-diff-details`       | Step-aware semantic diffs + hybrid AST fallback + report UX                          | #4                               |
|  6 | `2025-12-30-datamashup-embedded-hardening` | Embedded content diff + robust PackageParts/Content variants + fuzz/golden           | #1 (optional) + #2–#3 (optional) |
|  7 | `2026-01-02-perf-metrics-release-gates`    | Hit perf budgets (esp. dense-near-equal), add parse/memory metrics, tighten CI gates | after major schema work (#5)     |

### Dependency sketch

* **PBIX support** can ship early and unlock real-world testing.
* **M parser maturity** (branches 2–3) feeds **step model** (4), which feeds **semantic diff details** (5).
* **Embedded + fuzz hardening** (6) becomes far more valuable once PBIX exists (1) and M parsing is richer (2–3).
* **Perf/metrics** (7) should land after (5) so you aren’t chasing shifting baselines.

---

## Branch 1 — `2025-12-22-pbix-host-support`

### Objective

Add first-class `.pbix` / `.pbit` support *for the “legacy PBIX with DataMashup” path* (and a clear error for “no DataMashup → needs tabular model path”), aligned with Phase 3.5 in the testing plan.

### Why this is needed (current behavior)

* `OpcContainer` rejects ZIPs without `[Content_Types].xml`. PBIX files commonly won’t have that, so they currently trip `NotOpcPackage`.
* DataMashup extraction is Excel-specific: scan `customXml/item*.xml`, parse XML, base64 decode. PBIX instead stores `DataMashup` at the ZIP root (per spec).

### Scope of work

1. **Introduce a ZIP container abstraction that does not require OPC**

   * Add `ZipContainer` (or similar) that:

     * Opens arbitrary ZIP.
     * Enforces the existing limits pattern: `max_entries`, `max_part_uncompressed_bytes`, `max_total_uncompressed_bytes` (reuse `ContainerLimits`).
   * Refactor `OpcContainer` to be a thin wrapper around `ZipContainer` that performs the `[Content_Types].xml` requirement for Excel only.

2. **Implement PBIX/PBIT package open**

   * New type: `PbixPackage` (parallel to `WorkbookPackage`) or a general `HostPackage` enum.
   * Implement:

     * `PbixPackage::open(reader)` → `ZipContainer` → read `DataMashup` file at root.
     * If `DataMashup` missing but PBIX-like markers exist → return `NoDataMashupUseTabularModel` (spec already contemplates this as a dedicated error).

3. **Wire diff pipeline**

   * For PBIX host: diff should emit:

     * Query add/remove/rename/change ops.
     * Query metadata changes where metadata exists in the mashup.
   * No grid diff ops (PBIX has no Excel sheets) — report should remain valid.

4. **CLI behavior**

   * CLI currently expects `WorkbookPackage::open(file)` in its flow. 
   * Add host selection:

     * Extension-based: `.xlsx/.xlsm/.xltx/...` → Excel; `.pbix/.pbit` → PBIX. (This is simplest and avoids guessing ZIP contents.)
     * Optional future: inspect ZIP entries when extension is unknown.
   * Enforce option rules:

     * `--sheet/--database/...` invalid for PBIX (or ignored with warning).

5. **Fixtures + tests**

   * Add minimal PBIX fixtures:

     * `pbix_legacy_one_query_{a,b}.pbix` (DataMashup at root, single query modified)
     * `pbix_legacy_multi_query_{a,b}.pbix` (query add/remove)
     * `pbix_no_datamashup.pbix` (contains PBIX markers but no `DataMashup`)
   * Integration tests:

     * `open_pbix_extracts_datamashup`
     * `diff_pbix_emits_query_ops`
     * `pbix_no_datamashup_is_clear_error`

### Definition of done

* `excel_diff_cli diff old.pbix new.pbix` produces a valid `DiffReport` with query ops (no crash, no “NotOpcPackage” false negative).
* Unit + integration tests pass in CI including wasm build gates (core crate already has wasm workflows).

---

## Branch 2 — `2025-12-23-m-parser-tier1-ident-access`

### Objective

Turn the most common currently‑opaque constructs into structured AST nodes, so canonicalization and semantics can work beyond “token soup.”

Your current parser intentionally falls back to `Opaque(tokens)` for many core constructs (identifier refs, `if`, `each`, access chains, etc.).

### Scope of work (Tier 1)

Implement structured parsing for:

1. **Identifier references**

   * `Source`
   * `#"Previous Step"` (quoted identifiers)
2. **Access chains**

   * Field access: `Source[Field]`
   * Item access: `Source{0}`
   * Chained: `Source{0}[Content]`
3. **Conditionals**

   * `if <cond> then <a> else <b>`
4. **Each**

   * `each <expr>` as a real node (not opaque); eventually treated as a lambda.

These are explicitly called out as “currently opaque” by your coverage audit test.

### Implementation notes

* Keep your current “best-effort” parser shape, but extend it from the current decision tree:

  * `let`, parens stripping, record/list literals, call, primitive, else opaque. 
* Add AST variants (examples):

  * `MExpr::Ident { name }`
  * `MExpr::If { cond, then_branch, else_branch }`
  * `MExpr::Each { body }`
  * `MExpr::Access { base, kind: Field/Item, key }` with support for chaining
* Canonicalization updates:

  * Normalize boolean/null tokens should move from “opaque-token canonicalization” to “real AST canonicalization” where possible (you already canonicalize `true/false/null` inside `Opaque`).

### Tests to add/modify

* Update `m8_m_parser_coverage_audit_tests`:

  * Those Tier-1 cases should **stop** being opaque and assert the correct `MAstKind` instead. 
* Add new focused unit tests:

  * `parse_ident_ref`
  * `parse_field_access`
  * `parse_item_access`
  * `parse_access_chain`
  * `parse_if_then_else`
  * `parse_each_expr`

### Definition of done

* Coverage audit no longer expects these constructs to be `Opaque`.
* Canonicalization + equality still preserve formatting-only invariants where already tested.

---

## Branch 3 — `2025-12-24-m-parser-tier2-ops-fn-types`

### Objective

Complete the “expression grammar backbone” needed for real semantic diffs: operators, lambdas, type ascription, and error-handling constructs.

Your current audit test lists these as opaque today: `(x) => x`, `1 + 2`, `not true`, `x as number`. 

### Scope of work (Tier 2)

1. **Function literals**

   * `(x) => x`
   * `(x, y) => x + y` (if you support multi-arg)
2. **Unary operators**

   * `not <expr>`
   * `-<expr>` / `+<expr>`
3. **Binary operators with precedence**

   * arithmetic: `+ - * /`
   * comparisons: `= <> < <= > >=`
   * boolean: `and/or`
   * concatenation: `&`
4. **Type ascription**

   * `<expr> as <type>`
   * Minimal type grammar to start (identifiers like `number`, `text`, etc.)
5. **Try/otherwise**

   * `try <expr> otherwise <expr>` (if you go that far in this branch)

### Implementation notes

* At this point, the ad-hoc “try parse X” stack should become a **proper precedence parser** (Pratt or precedence-climbing), otherwise operators + nested constructs will become fragile.
* Ensure node identity for canonicalization:

  * Operator nodes should canonicalize whitespace and parenthesis-only variance.
  * Preserve associativity rules correctly.

### Tests

* Operator precedence tests (`1 + 2 * 3` parses as `+(1, *(2,3))`).
* Round-trip semantic equality tests where formatting differs.
* Regression tests that `Opaque` only remains for truly unsupported constructs.

### Definition of done

* The earlier “opaque” Tier‑2 cases parse into structured nodes.
* Canonical AST hash becomes much more stable for semantically equivalent code (important for `FormattingOnly` classification).

---

## Branch 4 — `2025-12-26-m-step-model-extraction`

### Objective

Extract an M “pipeline” model from `let … in …` queries so you can talk about changes as “step added/removed/changed,” not “some hash changed.”

The testing plan explicitly wants step-aware assertions and structured detail for semantic changes. 

### Scope of work

1. **Define the step model**

   * New internal types (example shape):

     * `MStep { name, kind, source_refs, params, output_ref }`
     * `StepKind` enum (start with the big ones)

2. **Implement `extract_steps(expr_m) -> StepPipeline`**

   * Parse → canonicalize → walk AST.
   * Identify:

     * binding order (step order)
     * dependencies: which step references which prior step (needs identifier parsing from branches 2–3)

3. **Step classification (start small, high value)**

   * Recognize common transforms:

     * `Table.SelectRows`
     * `Table.RemoveColumns`
     * `Table.RenameColumns`
     * `Table.TransformColumnTypes`
     * `Table.NestedJoin` / `Table.Join`
   * Everything else is `StepKind::Other { function_name_hash, arity, ... }`

4. **Stable step signatures**

   * Per-step hash/signature that:

     * Is resilient to formatting and superficial renames
     * Extracts the key semantic bits (e.g., removed column list, join keys)

### Tests

* Unit tests over raw M snippets (fast, deterministic).
* Add fixtures for at least:

  * one filter step
  * one remove columns step
  * step rename + same body
* Ensure your domain layer still builds queries from `Section1.m` the same way (no breakage).

### Definition of done

* Given a typical Power Query `let` expression, you can output an ordered list of steps with types and key parameters.

---

## Branch 5 — `2025-12-28-m-semantic-diff-details`

### Objective

Upgrade M diffs from “Semantic vs FormattingOnly” to **actionable semantic detail** and scalable AST diffs for complex refactors.

Right now the diff path computes canonical AST hashes and emits `QueryDefinitionChanged { change_kind, old_hash, new_hash }` but provides no structured semantic explanation.
The testing plan calls for step-aware reporting and (eventually) hybrid AST strategies (move-aware and deep-tree robust).

### Scope of work

1. **Extend the diff report schema for query definition changes**

   * Keep the current `QueryChangeKind` logic (it’s already working).
   * Add an optional “semantic detail” payload when semantic diff is enabled:

     * step diffs (`StepAdded/Removed/Modified/Reordered`)
     * fallback AST edit script summary when steps can’t explain it

2. **Step-aware diff**

   * Using `StepPipeline` from branch 4:

     * Align steps (start with order-based alignment + signature similarity).
     * Produce:

       * added/removed steps
       * modified step with “what changed”
   * Ensure the output can support tests like:

     * “exactly one semantically significant change”
     * “mentions the step name or type” 

3. **Hybrid AST diff fallback**

   * Implement two modes:

     * **Small AST**: exact tree edit distance (APTED/Zhang‑Shasha style)
     * **Large AST**: move-aware mapping (GumTree-like heuristics)
   * You can start by defining a generic `TreeNode` view over `MExpr` and then:

     * compute subtree hashes
     * match identical subtrees to detect moves
     * compute edits on the reduced unmatched regions

4. **Fixtures + tests for the hybrid strategy**

   * Add the fixtures described in the testing plan (or equivalent):

     * deep skewed IF tree
     * large refactor with moved block
     * wrap/unwrap function scenario 
   * Tests should assert:

     * completes under a bounded time
     * produces move/insert semantics rather than “replace whole query”

5. **CLI + web viewer presentation**

   * CLI: show a compact semantic summary under each changed query.
   * Web: expand/collapse per query with step details.

### Definition of done

* When semantic M diffs are enabled:

  * most real-world “Power Query editor” edits show as step-level diffs.
  * big refactors still produce a usable structural diff rather than a monolithic “changed.”

---

## Branch 6 — `2025-12-30-datamashup-embedded-hardening`

### Objective

Make the DataMashup surface “complete” for diffing by:

* treating embedded mini-packages as diffable entities,
* hardening PackageParts parsing against more real-world variability,
* expanding fuzz/golden coverage.

You already extract embedded contents in `PackageParts` and have tests around BOM stripping for both main and embedded `Section1.m`.
But the testing plan expects explicit diffs when only embedded contents change. 

### Scope of work

1. **Decide and implement the domain model for embedded contents**

   * Option A: treat each embedded `Content/{GUID}/Formulas/Section1.m` as its own “namespace” of queries.
   * Option B: attach embedded diffs as children of the queries that reference them.
   * Start with A (simpler + deterministic), and upgrade later.

2. **Diff embedded contents**

   * Extend the “build queries” surface:

     * today it reads only `dm.package_parts.main_section.source`. 
   * Add:

     * `build_embedded_queries(dm) -> Vec<Query>` with synthetic names like `Embedded/<content-name>/Section1/<Member>`
   * Feed those queries through the existing query diff path. 

3. **Hardening for PackageParts variants**

   * Expand parsing tolerance for:

     * additional entries
     * slightly different paths
     * multiple embedded packages
   * Enforce resource ceilings via `DataMashupLimits` (already present) and add regression tests for “near the limit” cases.

4. **Fuzz + regression**

   * You already have fuzz targets for DataMashup framing/build and grid diff.
   * Add:

     * a fuzz target for `parse_section_members` + `parse_m_expression` together (to catch parser panics early)
     * corpus seeds based on real world mashups (if you maintain a private corpus, wire it locally even if CI doesn’t ship it)

### Definition of done

* A change isolated to embedded `Content/*.package` yields:

  * an embedded query diff and **no** spurious diffs on unrelated top-level queries.
* Fuzz runs don’t uncover panics in embedded parsing pipelines.

---

## Branch 7 — `2026-01-02-perf-metrics-release-gates`

### Objective

Finish the “ship it” work: performance, instrumentation, and CI gates.

Bench results show:

* `perf_50k_dense_single_edit` ≈ **7821ms** total. 
  And your code comments indicate some metrics like parse time / peak memory are still “future phase.”

### Scope of work

1. **Dense near-equal speedup (P1 class)**

   * Implement a “skip unchanged rows” fast-path:

     * If alignment is identity and a row signature hash matches, avoid per-cell compare for that row.
   * This targets exactly the “single edit in huge dense grid” case where most rows are unchanged.

2. **Instrument parse vs diff time**

   * Add `parse_time_ms` and `diff_time_ms` in `DiffMetrics`/report.
   * Make it available in CLI JSON output for benchmarking pipelines.

3. **Peak memory tracking**

   * Add a gated allocator counter (feature-flagged) or integrate an existing lightweight approach, and plumb `peak_memory_bytes` into metrics.

4. **Tighten perf CI gates**

   * Update `scripts/check_perf_thresholds.py` / perf workflow to enforce:

     * “no regressions vs baseline”
     * and optionally absolute caps for the key scenarios.
   * Add a “full-scale perf” scheduled job if not already present (your repo already stores fullscale benchmark history).

5. **Release readiness checklist updates**

   * Document:

     * PBIX support limits (“legacy DataMashup only”)
     * semantic M diff behavior and how to enable it
     * resource ceilings knobs (`max_memory`, `timeout`, etc. already exist in config)

### Definition of done

* P1 dense-near-equal benchmark improves materially (goal: <5s target, or at least a clear downward step + no regressions).
* Metrics are emitted and used to gate perf PRs.
* Memory ceilings are measurable, not just estimated.

---

## What this 7-branch plan deliberately does *not* fully solve

You’ll have a clean PBIX story for **“PBIX with DataMashup”**, and a correct/clear error for **“PBIX without DataMashup”** (tabular model path). Implementing full tabular model/DAX diff is a separate post‑MVP epic in the testing plan.

If you want, I can also append an “8th (post‑MVP) branch” proposal for DX1+ (tabular model parsing + minimal measure diff), but the 7 above are the most direct path to a shippable PBIX+semantic‑M upgrade without exploding merge conflicts.
