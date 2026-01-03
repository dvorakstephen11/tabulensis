# Next sprint plan: closeout + simplification + parallelism + PBIX (complete story)

## 0) Reality check vs `next_sprint_plan.md`

The 7-branch plan in `next_sprint_plan.md` was aimed at: PBIX host support, richer M parsing, step-model extraction, semantic diff detail, embedded content diffing + fuzz, and perf/metrics gates. 

Based on the current codebase snapshot:

* **PBIX/PBIT host support is already wired end-to-end** (CLI host detection, `PbixPackage`, and PBIX diff integration tests).
* **M parser Tier1+Tier2 are already implemented and enforced by coverage-audit tests** (identifiers/access chains/`if`/`each`, plus operators, lambdas, `try/otherwise`, type-ascription, etc.).
* **Step model extraction + semantic detail payload exist** (`QuerySemanticDetail` with `StepDiff` + AST fallback summary).
* **Embedded mini-package queries are diffed as first-class surface** (there’s a regression test proving an embedded-only change yields an embedded query diff op).
* **Perf/metrics + CI gates exist** (perf workflows + thresholds script + fullscale artifacts), and the “dense single edit” case is already materially improved vs the 7.8s cited in the older plan.

So the “core implementation” portions of branches 1–7 largely look done.

## 1) What’s *still undone* from `next_sprint_plan.md` (and how we’ll finish it)

Here are the gaps that remain, mapped to concrete deliverables:

1. **Web viewer semantic presentation**
   `next_sprint_plan.md` explicitly calls for “Web: expand/collapse per query with step details.” 
   You do have a web demo build pipeline + WASM size budgets, but nothing in the provided context suggests the UI actually renders step diffs / semantic details yet. 
   → Address in **Branch 7** (“Web demo semantic + PBIX UI”).

2. **Release-readiness documentation**
   Branch 7 requires documenting PBIX limits, semantic M behavior, and resource ceilings. 
   There isn’t an obvious dedicated “release readiness checklist” doc surfaced in the context. 
   → Address in **Branch 1** (“Docs closeout + release checklist”).

3. **Fuzz coverage “exists but isn’t fully productized”**
   Branch 6 asked for a fuzz target that exercises `parse_section_members` + `parse_m_expression`. 
   You *do* have a dedicated M fuzz target and corpus seeds, but the CI fuzz workflow currently runs other targets and also maintains a duplicate top-level `fuzz/` harness.
   → Address in **Branch 3** (“Fuzz/CI simplification + run the right targets”).

4. **PBIX “no DataMashup” path is still an explicit limitation**
   The plan *deliberately* didn’t implement “PBIX without DataMashup” (tabular model path). 
   The spec also flags this as a real-world limitation (enhanced metadata PBIX). 
   → Address in **Branch 6** (“Enhanced-metadata PBIX model diff MVP”).

---

## 2) Unnecessary fluff to remove (high-confidence)

These are the clearest “pare it down” candidates visible in the current repo snapshot:

### A) Checked-in temporary artifacts

`tmp/_release_test`, `tmp/openpyxl_sdist`, and `tmp/xlsxwriter_sdist` look like build/debug leftovers, not source-of-truth inputs. 
**Action**: delete from repo + add `tmp/` to `.gitignore`.

### B) Redundant fuzz harnesses

There is both `core/fuzz` and a separate top-level `fuzz/` setup.
**Action**: keep one (recommend `core/fuzz`), delete the other, and simplify CI.

### C) Vendored Python sdists / expanded vendor trees

You have `fixtures/vendor/XlsxWriter-3.2.0` and also a tarball.
**Action**: keep *either* (1) pinned pip deps, or (2) a single vendored artifact — not both.

### D) Legacy API surface that the spec itself calls out as “panic”

The spec says legacy `diff_workbooks_with_config` panics while the new API returns structured errors. 
**Action**: make the panic path non-public / behind an explicit “legacy” feature flag, and point all internal callers at the safe API.

These are “cheap wins” because they mostly reduce maintenance risk and repo weight without changing the diff algorithms.

---

## 3) Parallelization: the cheapest high-impact targets

Your fullscale benchmarks show two hotspots:

* **Signature building dominates** the “dense single edit” case: 1.557s of 1.776s total. 
* **Move detection dominates** the “alignment block move” case: 11.302s of 12.23s total. 

The unified grid spec is explicit: WASM must work single-threaded, but the design should be able to exploit parallelism when available — *without breaking determinism*. 

So the “lowest risk / highest reward” parallel steps are:

1. **Parallel row/col signature computation** (embarrassingly parallel, easy to keep deterministic).
2. **Parallel cell diff within aligned row pairs** (also independent by row, but requires careful ordering).
3. **(Optional) Parallel scoring inside move detection** (bigger win, but higher coupling).

---

## 4) Branch map (7 feature branches)

|  # | Branch                                             | Primary goal                                   | Key deliverables                                                           |
| -: | -------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------- |
|  1 | `2026-01-06-docs-closeout-release-checklist`       | Finish remaining “plan hygiene” work           | Update stale docs, add release checklist, mark previous sprint plan status |
|  2 | `2026-01-07-cli-host-unification-and-polish`       | Make PBIX a first-class CLI citizen everywhere | `info` supports pbix/pbit + embedded queries; consistent host handling     |
|  3 | `2026-01-08-fluff-prune-and-fuzz-ci-simplify`      | Remove repo bloat + simplify fuzz              | delete `tmp/`, unify fuzz harness, run M fuzz target in CI                 |
|  4 | `2026-01-09-parallel-signature-build`              | Low-risk parallelism win                       | rayon-gated parallel row/col hashes; determinism tests                     |
|  5 | `2026-01-10-parallel-cell-diff-and-stable-merge`   | Keep scaling after signatures speed up         | parallel per-row cell diff + stable op ordering + streaming safety         |
|  6 | `2026-01-12-pbix-enhanced-metadata-model-diff-mvp` | “PBIX diff” beyond legacy DataMashup           | model extraction fallback + measure diffs + actionable errors              |
|  7 | `2026-01-14-web-demo-semantic-and-pbix-ui`         | Deliver the web viewer UX that was promised    | step diffs UI, PBIX support UX, keep wasm size budgets                     |

---

# 5) Branch-by-branch plan

## Branch 1 — `2026-01-06-docs-closeout-release-checklist`

### Objective

Make documentation match reality, close out stale artifacts, and add the missing “release readiness” checklist called for in the original plan. 

### Why

Right now, at least one doc (`m_parser_coverage.md`) describes the M parser as much shallower than what your tests enforce today.
This kind of drift becomes a product-quality issue quickly (users and future contributors will optimize for the wrong things).

### Scope

* **Update `m_parser_coverage.md`** to reflect Tier1/Tier2 constructs now parsed (ident/access, `if`, `each`, operators, lambdas, `try/otherwise`, type-ascription), and explicitly list what’s still opaque.
* **Add a “release readiness checklist” doc** (new file, or a dedicated README section) covering:

  * PBIX support boundaries (“legacy DataMashup” vs enhanced metadata).
  * What “semantic M diff details” mean and how they show up.
  * Limit knobs (`max_memory_mb`, `timeout`, `max_ops`, and limit behaviors). 
* **Annotate `next_sprint_plan.md`** as “completed” and enumerate what was intentionally deferred (tabular model path), with pointers to Branch 6. 

### Definition of done

* Docs no longer claim the M parser is “opaque beyond let/records/lists” when the code clearly isn’t.
* There is a clear “ship checklist” that matches the plan’s requirements. 

---

## Branch 2 — `2026-01-07-cli-host-unification-and-polish`

### Objective

Make CLI behavior consistent across commands and truly “host-agnostic” (Excel vs PBIX), not just for `diff`.

### Why

`diff` already switches between workbook and PBIX based on extension. 
But the `info` command currently opens only `WorkbookPackage`, which means PBIX/PBIT introspection (queries, embedded queries, etc.) can’t be surfaced consistently.

### Scope

* **Introduce a single CLI “open host” helper**:

  * extension-based host kind
  * shared error messages (especially for PBIX “no DataMashup”).
* **Upgrade `info` to accept `.pbix/.pbit`**:

  * print host kind
  * list queries **including embedded queries** (use the same “build all queries” path the diff uses).
* **Output polish**:

  * Ensure text output doesn’t silently drop newer `DiffOp` variants (especially measure ops that Branch 6 will introduce as first-class output). 
* **Tests**

  * Add CLI integration tests: `excel-diff info foo.pbix` prints query counts; embedded queries appear when present; errors are stable.

### Definition of done

* `diff` and `info` share one host detection + open pipeline.
* PBIX/PBIT `info` works and reports embedded queries (not just top-level ones).

---

## Branch 3 — `2026-01-08-fluff-prune-and-fuzz-ci-simplify`

### Objective

Reduce repo weight and eliminate redundant systems so CI and local dev are simpler.

### Why

You currently carry:

* `tmp/*` debug/build artifacts. 
* both `core/fuzz` **and** a top-level `fuzz/` harness.
* duplicated vendored Python artifacts.

This is the sort of “slow leak” that makes everyone afraid to touch the repo.

### Scope

* **Delete `tmp/` from the repo** and add it to `.gitignore` (right now it isn’t ignored). 
* **Pick one fuzz harness** (recommend keep `core/fuzz`) and delete the other:

  * Update `.github/workflows/fuzz.yml` to run:

    * `fuzz_datamashup_parse`
    * `fuzz_diff_grids`
    * `fuzz_m_section_and_ast` (already exists; make CI actually run it)
* **Vendor cleanup**

  * Remove expanded vendor trees where a pinned dependency is enough.
* **Legacy API cleanup (low-risk option)**

  * Hide/remove the panic-based legacy entrypoints that the spec itself flags as undesirable. 

### Definition of done

* Repo no longer contains `tmp/` artifacts. 
* CI fuzz runs the M parser target and there’s only one fuzz harness.

---

## Branch 4 — `2026-01-09-parallel-signature-build`

### Objective

Add parallelism where it’s cheapest and most guaranteed to help: signature building (row/col hashes).

### Why

In the fullscale benchmark:

* `perf_50k_dense_single_edit`: **1.557s** spent in signature build out of **1.776s** total. 
  Also, the grid spec explicitly calls out structuring for parallelism while maintaining deterministic outputs. 

### Scope

* Add a **native-only `parallel` feature** (enabled by CLI; disabled by wasm build).
* Implement **parallel row signature hashing**:

  * `par_iter` over rows (or chunked parallelism to reduce overhead).
  * Store results in a fixed `Vec<RowSignature>` by index (no nondeterministic merges).
* Keep determinism:

  * Make sure any map/set used during signature aggregation doesn’t leak iteration-order into output.
* Add **determinism tests**:

  * same inputs, different thread counts → identical `DiffReport.ops` ordering.
* Add a small perf metric: `threads_used` in perf metrics output (optional).

### Definition of done

* Multicore native runs show a material drop in `signature_build_time_ms` on the large-grid fixtures, with no output-order changes between thread counts.

---

## Branch 5 — `2026-01-10-parallel-cell-diff-and-stable-merge`

### Objective

After signatures get faster, keep scaling by parallelizing the next cheapest phase: cell diffs across aligned row pairs.

### Why

Even today, the “completely different” case spends **405ms** in cell diff (and could become the next bottleneck after signature parallelism). 
And any parallel work must preserve stable ordering to remain trustworthy. 

### Scope

* Parallelize “cell compare for a row pair”:

  * Each aligned row pair produces a local `Vec<DiffOp>`.
  * Merge results in increasing row order (and within a row, increasing column).
* Guarantee deterministic ordering even with streaming:

  * For `JsonLinesSink`, enforce a stable op sequence (sheet → kind → address) before emitting.
* Extend determinism tests to cover:

  * dense single edit
  * adversarial repetitive
  * block move fixture

### Definition of done

* Heavy-diff scenarios speed up with threads.
* Output op ordering is identical across thread counts and matches the spec’s determinism expectations. 

---

## Branch 6 — `2026-01-12-pbix-enhanced-metadata-model-diff-mvp`

### Objective

Make PBIX diff work for the real-world case where `DataMashup` is missing (enhanced metadata PBIX), by falling back to tabular-model extraction and producing **measure diffs**.

### Why

Both the plan and the spec acknowledge this limitation: PBIX without `DataMashup` needs a tabular model path.
You already have a model diff layer that emits `MeasureAdded/Removed/MeasureDefinitionChanged` ops. 

### Scope (MVP path)

* **Extraction**: when PBIX/PBIT has no `DataMashup`, attempt to find a model schema payload:

  * best-effort support for `DataModelSchema` where present (especially common in PBIT exports).
  * keep the current clear error if schema isn’t extractable, but make it actionable (e.g., “export as PBIT to expose DataModelSchema”).
* **Diff**:

  * Use existing model diff to emit measure ops.
  * Treat measure expression changes as text-level first (semantic DAX diff can come later).
* **Reporting & CLI**:

  * Ensure CLI text/json output includes these measure ops.
* **Tests/fixtures**:

  * Add PBIT fixture with `DataModelSchema` (at least one measure) and a changed version.
  * Add a PBIX “no DataMashup but has model schema” fixture if feasible.

### Definition of done

* A “PBIX without DataMashup” file can still produce meaningful diff output (at minimum: measures added/removed/changed), instead of only returning `NoDataMashupUseTabularModel`.

---

## Branch 7 — `2026-01-14-web-demo-semantic-and-pbix-ui`

### Objective

Deliver the web viewer UX that was explicitly part of the semantic diff work: per-query expand/collapse with step diffs — and make PBIX/PBIT usable in the web demo.

### Why

The original plan calls out web viewer presentation for semantic diffs. 
You already have a web demo pipeline with strict wasm size budgets. 
So the missing piece is “turn the JSON into something humans can use.”

### Scope

* **UI rendering**

  * Show per-query cards:

    * change kind (semantic vs formatting-only vs rename)
    * step diffs (`StepAdded/Removed/Modified/Reordered`)
    * AST fallback summary when step diffs aren’t available
* **PBIX/PBIT in web**

  * Allow `.pbix/.pbit` upload, run diff, display query + measure sections.
  * If enhanced-metadata PBIX can’t be processed in WASM (size/feature constraints), show a crisp “what to do” message.
* **Stay within budgets**

  * Keep current size budgets intact. 
* **Lightweight UI testing**

  * At minimum: a “golden report” JSON is rendered without runtime errors.

### Definition of done

* Web demo can render:

  * semantic M step diffs for changed queries
  * PBIX query diffs (legacy DataMashup)
  * PBIX/PBIT measure diffs (from Branch 6)
* No wasm size regressions. 

---

## 6) How this directly satisfies your four goals

* **Undone from `next_sprint_plan.md`**: Branch 1 (release checklist + doc truth), Branch 3 (CI fuzz integration), Branch 7 (web semantic UI), Branch 6 (tabular/model path which was explicitly deferred).
* **Remove fluff**: Branch 3 deletes `tmp/`, removes duplicated fuzz harness, trims vendor bloat, and optionally deprecates panic-based legacy APIs.
* **Parallelization (cheap wins)**: Branch 4 (signature build) hits the biggest current hotspot; Branch 5 parallelizes the next phase while preserving determinism.
* **PBIX diff feature**: you already have the legacy DataMashup PBIX story; Branch 2 makes it usable across CLI commands; Branch 6 turns “no DataMashup” into meaningful diffs via model/measure extraction; Branch 7 makes it visible in the web demo.
