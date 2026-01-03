## Phase 4 Implementation Plan: Determinism + Streaming Sink Lifecycle Invariants

Phase 4 in `13_phase_plan.md` is explicitly about making two things *non-negotiable*: (1) deterministic output ordering across all important entry points (including streaming + parallel), and (2) a well-defined, well-tested streaming sink lifecycle contract (`begin → emit* → finish`), including how “partial outputs + warnings” work. 

This plan is intentionally grounded in how your codebase is already structured: a diff engine that emits `DiffOp` events into a `DiffSink` seam, with multiple hosts (WorkbookPackage, PBIX/PBIT), multiple sink implementations (Vec, callback, JSONL, desktop DB store), and existing test posture that already treats invariants as “sacred.”

---

## 0. Mental model and vocabulary

Before tasks, here’s the shared picture this phase enforces:

* **Determinism**: “Same inputs + same config ⇒ byte-for-byte identical meaning.” In practice for this repo it means:

  * Same *sequence* of `DiffOp` (order is part of the contract, not just set equality).
  * Same `DiffSummary` (`complete`, `warnings`, `op_count`).
  * Same `StringPool` / string table ordering and `StringId` values in streaming mode.
  * Same results across different rayon thread counts when `parallel` is enabled.

* **Streaming sink lifecycle**: The core contract is already documented in `DiffSink`:

  1. `begin(pool)` once (before any ops),
  2. `emit(op)` zero or more times,
  3. `finish()` once (“even on most error paths”). 
     Phase 4 makes that contract *mechanically enforced* and *covered across entry points*, not “true most of the time.”

* **Stable emission ordering**: Not just deterministic, but predictably ordered (e.g., sheets sorted by name/kind; query ops sorted; package-level sequencing of grid ops vs object ops vs Power Query ops, etc.). There are already dedicated tests for this in some domains (sheet ordering, parallel determinism), and Phase 4 extends that coverage.

* **Partial outputs + warnings semantics**: A diff can be “incomplete” either because it aborted early or because it fell back to a less reliable strategy under resource pressure. In your codebase, `DiffReport`/`DiffSummary` carries `complete` plus a `warnings` vector that explains why output may be missing or degraded.

---

## 1. Establish the Phase 4 invariant set (single source of truth)

### 1.1 Create a “Streaming Output Contract” doc (and keep it close to code)

Deliverable: a short, high-signal doc that lives in-repo (either `/docs/streaming_contract.md` or as a narrative section in `core/src/lib.rs`), and is referenced by:

* `core/src/sink.rs` rustdocs (the authoritative lifecycle contract),
* `JsonLinesSink` docs (string table constraints),
* and the entry-point rustdocs for streaming APIs (Workbook/PBIX/database mode).

This aligns with the design review recommendation to add small maintainability docs and to document sink lifecycle invariants explicitly.

The contract must define, concretely:

**A) Determinism**

* Output ordering is a stable API: consumers may rely on it for UI rendering, diff pagination, caching, and tests.
* Determinism is required across:

  * “report mode” (`DiffReport` returned with `ops` vec),
  * “streaming mode” (`DiffSink`),
  * and “parallel mode” (rayon thread count variance).

**B) Sink lifecycle**

* `begin` is called exactly once before any `emit`.
* `finish` is called exactly once after the last `emit`.
* If `begin` succeeded, `finish` must be attempted exactly once even when the diff fails later (best-effort; do not mask the original error). This strengthens what’s already intended in the `DiffSink` docs.
* No `emit` after `finish` (and tests enforce it).

**C) String table rules for streaming**
This is critical because your JSONL sink writes a one-time header containing the string table:

* `JsonLinesSink::begin()` writes a `"Header"` line containing `pool.strings()` and then streams ops as JSON lines. 
* Therefore: **all strings that will be referenced by any emitted `DiffOp` must already exist in the pool by the time `begin(pool)` is called.**
* After `begin`, the pool must be treated as *frozen* with respect to adding *new* strings (lookups are fine; inserting new ones would make `StringId` references exceed header length).

This contract is already implicitly true for your `WorkbookPackage` streaming path because it computes object ops + M ops (interning required strings) *before* the engine calls `begin` inside the streaming workbook diff. 
Phase 4 makes that rule explicit and enforces it across **all** streaming entry points (including PBIX).

**D) “Complete / warnings / partial output” semantics**
Define what `complete=false` means (and what it does *not* mean), and how warnings are ordered and interpreted. The existing `DiffReport` doc comment already ties `complete=false` to missing ops due to resource limits and points to warnings as the explanation. 

---

## 2. Inventory all streaming entry points and current lifecycle behavior

Deliverable: a small “entry point map” section in the contract doc, plus a checklist used by engineers when adding new streaming APIs.

At minimum enumerate and record:

### 2.1 Engine streaming entry points

* `try_diff_workbooks_streaming(...)` / `_with_progress(...)`:

  * Calls `sink.begin(pool)` very early (“before parsing grids”) in the current implementation. 
  * Emits ops via `EmitCtx::emit_op` (which calls `sink.emit(op)` and increments `op_count`). 
  * Must guarantee `finish` on both success and error-after-begin.

* `try_diff_grids_database_mode_streaming(...)`:

  * Begins sink once, emits ops, finishes, but has multiple branches (timeout abort, duplicate-key fallback to spreadsheet mode, normal). 

### 2.2 Package-level streaming entry points

* `WorkbookPackage::diff_streaming_with_pool(...)`:

  * Precomputes object graph diffs and M diffs into vectors (interning required strings), then calls streaming workbook diff with a `NoFinishSink` wrapper, then emits object ops, then emits M ops, then calls `sink.finish()` once.
  * Already has strong lifecycle tests (finish exactly once, no emit after finish, finish on error paths). 

* `WorkbookPackage::diff_database_mode_streaming_with_pool(...)`:

  * Similar pattern: precompute object ops + M ops, then stream database-mode grid ops with `NoFinishSink`, then emit object + M, then finish.

### 2.3 PBIX/PBIT streaming entry point

* `PbixPackage::diff_streaming_with_pool(...)` currently calls `sink.begin(pool)` *before* producing M ops and before any model building/diff. 
  That is a red flag for JSONL correctness because M/model diffs use `pool.intern(...)` for names and metadata values, meaning new strings can appear after the header was written.

### 2.4 Consumers that amplify the importance of correctness

* CLI streaming uses `JsonLinesSink` for **both Workbook and PBIX** streaming output.
* Desktop uses a DB-backed sink (`OpStoreSink`) where `finish()` commits the transaction; missing `finish` is not “just a flush,” it can leave state inconsistent/locked. 

This inventory step is not just documentation: it’s the checklist Phase 4 will use to decide where code changes and tests must be added.

---

## 3. Make sink lifecycle behavior mechanically correct in code

### 3.1 Introduce a standard “finish guard” pattern for streaming functions

Deliverable: a small internal helper used by **every** streaming implementation that directly calls `begin`.

Why: the desired contract says “finish once even on most error paths” , but today some streaming entry points can exit via `?` without executing `finish()` (especially on `sink.emit()` errors or internal errors after begin). Phase 4 makes the contract strict and uniform.

Implementation strategy (grounded in your current patterns):

* Use an RAII guard (`SinkFinishGuard`) that:

  * is created immediately after `begin()` succeeds,
  * calls `finish()` in `Drop` if not already “disarmed,”
  * ignores errors in drop (so it never masks the original error),
  * and provides a `finish_and_disarm()` method for the success path (where finish errors should still be returned).

This matches how `WorkbookPackage::diff_streaming_with_pool` already handles errors: it calls `let _ = sink.finish(); return Err(e);` to ensure a best-effort finish while preserving the original error. 

### 3.2 Apply the guard to all direct-begin streaming entry points

At minimum:

* `try_diff_workbooks_streaming_impl` (engine workbook streaming) 
* `try_diff_grids_database_mode_streaming` (engine DB streaming) 
* `PbixPackage::diff_streaming_with_pool` 
* Any other “top-level streaming” entry point that calls `sink.begin(pool)` itself.

Important nuance:

* Leave the existing `NoFinishSink` pattern alone for nested streaming orchestration. It is an explicit signal that “inner finish must not finish the real sink,” and the design review explicitly calls out `NoFinishSink` as evidence lifecycle already matters.
  Your guard must respect that: if it’s guarding a `NoFinishSink`, “finishing” it is a no-op and remains safe.

### 3.3 Expand lifecycle tests beyond WorkbookPackage streaming

You already have excellent package-level streaming lifecycle tests:

* finish called exactly once
* no emit after finish
* finish invoked even when emission errors occur mid-stream 

Phase 4 extends that coverage to:

* engine workbook streaming,
* engine database-mode streaming,
* PBIX streaming.

Concrete test additions:

**A) A reusable StrictLifecycleSink**
Add a sink used in multiple tests that:

* records call order (`begin_seen`, `finish_seen`, `emit_count`, `finish_count`),
* panics/fails if `emit` happens before `begin`,
* returns an error if `emit` happens after `finish`,
* asserts `finish_count == 1` for all cases where begin succeeded.

You already have the pattern in tests for package streaming; this just generalizes it and uses it at more entry points. 

**B) “Finish on emit error” tests for engine streaming**
Create an erroring sink (like your existing “fail after N emits” sink) and use it with:

* `try_diff_workbooks_streaming` and
* `try_diff_grids_database_mode_streaming`,
  ensuring that even if `emit()` fails mid-stream, `finish()` is still called exactly once.

**C) “Finish on internal error after begin” tests**
Use a config that triggers a `ReturnError`-style limit breach during diff, but only after begin has happened. You already test limit behaviors in `limit_behavior_tests.rs` and hardening behaviors in `hardening_tests.rs`; reuse those patterns to force a post-begin error.

---

## 4. Enforce the “string table is complete at begin” invariant

This is the most likely place Phase 4 uncovers a real correctness bug, because JSONL streaming is structurally constrained:

* `JsonLinesSink::begin` writes the header exactly once and is explicitly idempotent (`wrote_header`). 
* If new strings are added to the pool after begin, subsequent ops may reference string IDs that the consumer cannot resolve from the header. That’s not “slightly wrong”—it makes streamed output self-inconsistent.

### 4.1 Audit: where do we `intern()` after calling `begin()`?

Known hotspot from codebase reality:

* `PbixPackage::diff_streaming_with_pool` calls `sink.begin(pool)` and then calls `diff_m_ops_for_packages(...)` and model building/diff. 
* `diff_m_ops_for_packages` interns query names and metadata strings (`pool.intern(...)`) and sorts keys to ensure deterministic op order. 

So PBIX streaming is extremely likely to violate the JSONL header invariant today.

### 4.2 Fix PBIX streaming to satisfy the invariant

Deliverable: PBIX streaming begins only after all potentially-new strings have been interned.

Two viable approaches:

**Option 1 (recommended for Phase 4): Reorder PBIX streaming to precompute ops before begin**

* Compute `m_ops` (and `model_ops` if model diff enabled) *before* calling `sink.begin(pool)`.
* Then call `sink.begin(pool)` and emit ops.
* Then call `sink.finish()`.

This is the simplest correctness-first change, and it’s consistent with your `WorkbookPackage` streaming orchestration pattern (which already computes some vectors before the inner streaming call begins).

**Option 2 (more complex): Pre-intern pass**

* Walk DataMashup + model schema inputs to intern all strings up front, then begin early, then diff.
  This preserves “time to first header byte,” but is substantially more code and easier to miss edge strings.

Phase 4’s goal is invariants; start with Option 1 and only invest in Option 2 if “early header emission” is a proven requirement.

### 4.3 Add tests that prove “header contains all referenced strings”

You already have an excellent precedent:

* `package_streaming_json_lines_header_includes_m_strings` verifies that JSONL header includes M strings in workbook package streaming. 

Phase 4 should replicate that coverage for PBIX streaming in two layers:

**A) Core-level PBIX streaming JSONL correctness test**

* Build a `JsonLinesSink` to an in-memory buffer, run PBIX `diff_streaming`, parse the first line header (`strings` array length), deserialize subsequent lines into `DiffOp`, and assert every `StringId` referenced by those ops is `< strings.len()`.
* If there’s already a helper to collect string IDs from `DiffOp` in the existing test, reuse it. 

**B) CLI-level PBIX JSONL regression test**
Your current CLI test `diff_pbix_jsonl_writes_header_and_ops` only checks:

* first line is `"Header"`,
* `strings` exists,
* at least one Query op exists. 

Phase 4 should upgrade it to the stronger invariant:

* all string IDs used in emitted ops are resolvable from the header string table.

This directly protects the real production surface: CLI JSONL PBIX output.

### 4.4 Add a “no new strings after begin” enforcement sink (test-only)

Deliverable: a test sink wrapper that:

* captures `pool.len()` at `begin`,
* and on every `emit` asserts `pool.len()` is unchanged.

This is a cheap but powerful tripwire for future regressions in any streaming entry point.

---

## 5. Expand determinism coverage across entry points (including streaming + parallel)

This is the first bullet of Phase 4: broaden determinism tests beyond the subset already covered. 

You already have:

* streaming determinism across two runs for workbook engine streaming (`streaming_produces_ops_in_consistent_order`) 
* op-count consistency tests for streaming (`streaming_summary_matches_collected_ops`) 
* parallel determinism tests for grid diff ops across thread counts (rayon pools) 
* deterministic sheet op ordering tests 

Phase 4 expands this into a **determinism matrix**.

### 5.1 Build a reusable determinism harness (test utility)

Deliverable: a helper in `core/tests/common` that runs a closure multiple times and compares outputs.

At minimum, support:

* “structured determinism”: compare `Vec<DiffOp>` + `DiffSummary`,
* “streamed JSONL determinism”: compare parsed header+ops, and also validate StringId range.

Also add a variant that uses **fresh sessions** per run (new `DiffSession` / new `StringPool`), because reusing the same pool across runs can mask nondeterministic interning order. The current streaming determinism test reuses one session for both runs. 

### 5.2 Determinism tests to add (core-level)

**A) WorkbookPackage streaming determinism**

* Choose a fixture pair that exercises:

  * grid ops,
  * plus Power Query ops,
  * plus at least one object op if possible (charts/named ranges/vba).
    Your fixtures manifest clearly includes generators for named ranges, charts, VBA, and multiple M scenarios.
* Run `WorkbookPackage::diff_streaming` twice with fresh sessions; ensure identical op sequence and identical summary.

**B) Database-mode streaming determinism**

* Use existing DB fixtures (the CLI already has D1/D2 tests) and run database-mode streaming twice, compare ops order exactly. 

**C) PBIX/PBIT streaming determinism**

* Use the already-present PBIX fixtures in tests (`pbix_legacy_multi_query_a.pbix` / `_b.pbix`) and run PBIX streaming twice, compare JSONL outputs structurally (header strings + ops).
* Add determinism coverage for PBIT model diff fixtures (`pbit_model_a.pbit` / `_b.pbit`) so measure ops and string pool ordering are stable. Those fixtures are generated by your fixtures generator and already used in CLI tests.

### 5.3 Determinism tests to add (CLI-level, high-value end-to-end)

Because your design review and completion analysis emphasize entry-point wiring and host parity, Phase 4 should include at least one end-to-end determinism check at the CLI surface.

Add an integration test that:

* runs the CLI JSONL diff twice on the same fixtures,
* asserts stdout is byte-identical across runs.

Do this for:

* one `.xlsx` pair that triggers streaming,
* one `.pbix` pair.

This catches “oops, HashMap iteration leaked into output order” at the exact product boundary.

### 5.4 Parallel determinism: extend beyond grids to streaming paths

The design review explicitly calls out the importance of keeping determinism as parallel paths expand.

Deliverable: extend the existing `parallel_determinism_tests` pattern to streaming APIs:

* Under `cfg(feature = "parallel")`, run the same streaming diff in a 1-thread rayon pool and a 4-thread pool and assert outputs are identical.
* Apply at least to:

  * streaming workbook diff (engine or package),
  * database-mode streaming (grid alignment can involve parallel meta building),
  * optionally PBIX if any parallelism exists there. 

---

## 6. Lock down stable emission ordering (and test what matters)

This phase is not about inventing a new ordering; it’s about **making the existing ordering explicit** and ensuring it doesn’t regress.

### 6.1 Write down the ordering contract per host

Deliverable: in the “Streaming Output Contract” doc, define:

* **Workbook engine ordering**:

  * Sheet-level operations are ordered deterministically by (name_lower, kind order). This already has a direct test. 
  * Within a sheet, cell/row/col ops must have stable ordering (whatever the engine already does).

* **WorkbookPackage ordering**:

  * Grid ops first (streamed from the engine),
  * then object graph ops (named ranges/charts/vba),
  * then Power Query ops.
    This is how the current streaming orchestration is structured (engine streaming first, then `object_ops`, then `m_ops`). 

* **PBIX/PBIT ordering**:

  * Power Query ops first (if present),
  * then model ops (if present / feature-enabled). 

### 6.2 Add one “ordering contract” test per host

Avoid brittle tests that assert every detail, but add guardrails that prevent accidental category reordering:

* For workbook package streaming:

  * Assert that once the first M op appears (e.g., kind starts with `"Query"`), no grid/object ops appear after it.
* For PBIX streaming:

  * Assert that query ops appear before measure ops (if both exist).

Your existing tests already classify ops (e.g., `DiffOp::is_m_op`) and search for Query/Measure by kind in CLI integration tests. Reuse the same technique at core test level.

---

## 7. Document and test partial output + warnings semantics in streaming mode

Phase 4 explicitly calls this out. 

### 7.1 Make “complete=false” behavior unambiguous

In the contract doc, specify:

* `complete=true` means the diff ran without any conditions that might omit or degrade expected operations.
* `complete=false` means consumers must treat output as potentially partial/degraded, and must consult `warnings` for why. This aligns with the `DiffReport` doc. 
* `warnings` ordering should be deterministic.

### 7.2 Add streaming-specific tests for limit/hardening behaviors

Your design review notes `HardeningController` supports fallbacks while “preserving partial output + warnings.” 
Phase 4 should ensure streaming paths behave the same way.

Concrete tests:

* **Streaming timeout / op limit**:

  * Configure a very small timeout or op cap that triggers an abort.
  * Assert:

    * `summary.complete == false`
    * warnings contain the appropriate message
    * `op_count` matches emitted ops (if partial emission is allowed under that behavior)
    * `finish()` was called once.

* **Database-mode duplicate key fallback**:

  * Trigger the “duplicate keys → fallback to spreadsheet diff” branch and assert:

    * deterministic warning text
    * stable output order
    * finish called once.

---

## 8. Phase 4 acceptance criteria

Phase 4 is “done” when the following are true:

### 8.1 Sink lifecycle invariants

For every streaming entry point (engine workbook streaming, engine DB streaming, WorkbookPackage streaming, WorkbookPackage DB streaming, PBIX streaming):

* `begin` happens exactly once before any `emit`.
* `finish` happens exactly once after begin succeeds, even if an error occurs later (best-effort on error paths).
* no `emit` after `finish`.
* tests enforce these behaviors using strict sinks.

### 8.2 JSONL header/string table correctness

* For workbook package JSONL streaming, the existing header coverage test continues to pass. 
* New PBIX JSONL header coverage tests pass:

  * all `StringId` values referenced by streamed ops are within the header `strings` array.
* CLI PBIX JSONL integration test enforces the same invariant at the binary boundary.

### 8.3 Determinism across entry points and parallel thread counts

* Determinism tests exist for:

  * engine streaming,
  * workbook package streaming,
  * database-mode streaming,
  * PBIX/PBIT streaming,
  * and at least one CLI JSONL end-to-end test.
* Under `parallel`, streaming outputs are identical across at least 1-thread vs 4-thread rayon pools.

### 8.4 Stable emission ordering is protected

* Documented ordering rules exist per host.
* One ordering-contract test per host prevents silent category reordering regressions.

---

## 9. Practical sequencing (so this doesn’t sprawl)

If you want an execution order that minimizes backtracking:

1. **Write the contract doc + update rustdocs** (locks scope and definitions).
2. **Add strict lifecycle sinks and apply them to engine + PBIX + DB streaming** (tests should currently expose gaps).
3. **Fix PBIX streaming header correctness** (reorder begin or pre-intern).
4. **Add determinism harness + fresh-session determinism tests** (including parallel variants).
5. **Add ordering-contract tests + partial-output semantics tests** (guardrails + clarity).

That sequence aligns with the “invariants-as-tests posture” that your design evaluation praises as a core strength of this repo. 
