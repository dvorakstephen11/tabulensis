# Design Evaluation Report

## Executive Summary

This codebase reads like a system that knows what it is: a diff engine for hostile, layered file formats, where performance, determinism, and “human-meaningful” output are as important as raw correctness. The strongest architectural signal is that the intended layering isn’t only described in the spec—it’s operationalized in the repository via explicit modules (container/framing/parsing/domain/diff) and an automated architecture guard that prevents the most common form of entropy: dependency direction inversion.

At the core, the IR is built around a pragmatic “grid + semantics” model: sheets hold a grid that can be sparse or dense, values and formulas are handled distinctly, and text is centralized through a string pool. This gives the engine a stable internal vocabulary for both high-level ops (sheet rename, block move) and low-level ops (cell edit), while enabling streaming outputs via a JSONL sink.

The primary risk is not architectural drift—it’s **algorithmic and representational complexity accumulating in the alignment/move machinery and in the ever-growing `DiffOp` schema**. The code already anticipates this: there are hardening limits, strategy selection, fallback paths, determinism tests, and performance regression scaffolding. However, the provided benchmark artifact is explicitly not “full scale,” and therefore cannot, by itself, validate the “instant diff on 100MB” ambition; the architecture is positioned for it, but the evidence here is still partial.

> Note on sources: the prompt references additional docs (testing plan + unified grid diff algorithm spec). They were not among the uploaded materials I could read. The implementation *does* embed commentary referencing the unified algorithm spec and explicitly calls out deviations, so we can still evaluate alignment with intended behavior from the code and the provided specification.

---

## Dimension Evaluations

### 1. Architectural Integrity

**Assessment**: **Strong**

**Evidence**:

* **The layered pipeline is explicit and enforceable.** The spec describes a five-layer flow (Host Container → Binary Framing → Semantic Parsing → Domain → Diff). 
  In implementation, you can map those layers almost 1:1:

  * Host Container: `core/src/container.rs` (`OpcContainer`, `ZipContainer`, `ContainerLimits`) 
  * Binary framing: `core/src/datamashup_framing.rs` (`parse_data_mashup`, base64 + framing invariants)
  * Semantic parsing: `core/src/excel_open_xml.rs` (OpenXML parts → workbook objects), plus `core/src/datamashup.rs` (embedded OPC mashup → query domain)
  * Domain: `core/src/workbook.rs` (Workbook/Sheet/Grid), `DataMashup`, and package-level domain in `core/src/package.rs`
  * Diff: `core/src/engine/*`, `core/src/m_diff.rs`, `core/src/object_diff.rs`, streaming sinks in `core/src/sink.rs`

* **Boundary violations are actively policed.** The `scripts/arch_guard.py` file encodes forbidden imports (e.g., parse modules must not depend on `diff`, `engine`, `output`; diff modules must not depend on parsing modules). This is unusually strong architectural hygiene for a “fast-moving” codebase and directly supports the spec’s separation goals. 

* **IR coherence is domain-faithful rather than incidental.** The grid model isn’t “just a matrix”: it encodes sparseness, dimensions, cell value vs formula, and attaches stable signatures for alignment and move detection. `WorkbookPackage` then composes workbook + mashup + vba rather than conflating them into one mega-IR, which matches the reality that Excel files contain heterogeneous sub-domains.

* **Data flow is traceable end-to-end.** A clear parse path exists:
  `WorkbookPackage::open` → `ZipContainer` → `open_workbook_from_container` (OpenXML) + `open_data_mashup_from_container` → `parse_data_mashup` → `build_data_mashup`.
  And a clear diff path exists:
  `WorkbookPackage::diff_streaming` → engine workbook diff (grid/sheets) → object diff ops → M diff ops → sink finish, plus summary/warnings. 

**Recommendations**:

* **Make the layering “discoverable” in Rustdoc.** The architecture exists, but a new maintainer would benefit from a single top-level module doc (in `lib.rs` or a `docs/` page) that explains the layers and points to canonical entry points. This reduces reliance on “institutional memory.”

* **Consider splitting `excel_open_xml.rs` along OpenXML concerns.** It currently carries much of the workbook-open complexity; breaking it into `shared_strings`, `workbook_xml`, `sheet_xml`, `relationships`, etc. would strengthen the “chapter structure” of the code without changing behavior.

---

### 2. Elegant Simplicity

**Assessment**: **Adequate**

**Evidence**:

* **Good simplicity where it matters: stable vocab + constrained surfaces.**
  `WorkbookPackage` is a strong façade: it keeps consumers from needing to understand the internal layers, and it creates a single place where “what counts as a workbook diff” is orchestrated. 

* **The string pool is an elegant compression of repeated text into a single axis of truth.** Using `StringId` everywhere keeps diff payloads small and enables both in-memory (`DiffReport`) and streaming (`JsonLinesSink`) modes. The `FrozenPoolSink` enforcement (no interning after “begin”) shows the team understands that streaming correctness requires strict invariants.

* **Where simplicity strains: the alignment “decision tree.”**
  `engine/grid_diff.rs` contains a multi-stage process: preflight similarity, select AMR vs legacy row alignment vs “single column” alignment, optional move extraction, then cell diff. This is domain-driven complexity, but the control plane is dense—understanding “why this strategy ran” still requires mental simulation across modules.

* **The unified algorithm spec is acknowledged but not fully embodied.** The alignment module explicitly documents deviations (e.g., simplified AMR, Hungarian instead of LAPJV, no histogram diff). That honesty is excellent, but it is also a signal that the implementation is still negotiating its “final form.” 

**Recommendations**:

* **Centralize and surface the strategy selection rationale.**
  Add a single “decision log” structure emitted into `DiffSummary` (or optional debug trace), capturing which alignment path was chosen and why (preflight metrics, thresholds hit, hardening fallback). This makes behavior explainable without reading code.

* **Refactor the alignment pipeline into explicit phases with typed outputs.**
  The code already behaves like a pipeline; making it structurally explicit (Phase1: signatures, Phase2: anchors, Phase3: moves, Phase4: cells) would reduce cognitive load and ease future algorithm swaps.

---

### 3. Rust Idiomaticity

**Assessment**: **Strong**

**Evidence**:

* **Error handling is value-oriented and layered.**
  The spec calls for thin façade errors delegating to layer-specific errors. 
  The implementation follows this: `PackageError` wraps `ContainerError`, `GridParseError`, and `DataMashupError`, and provides stable error codes + “suggestion” messaging. 

* **Type-driven design is used to encode invariants.**

  * `StringId` prevents accidental mixing of raw strings and interned identifiers.
  * `SheetKind` distinguishes sheet semantics (e.g., worksheet vs chart sheet), preventing illegal assumptions at call sites. 
  * `DiffOp` variants are strongly typed per domain operation, supporting stable serialization. 

* **Traits are used as behavioral contracts, not faux inheritance.**
  `DiffSink` and its helpers are a clean “visitor/streaming output” abstraction; generic sinks avoid dynamic dispatch overhead where not needed. The `SinkFinishGuard` is a disciplined use of `unsafe` to enforce single-finish semantics—high-leverage, low-surface-area unsafety. 

* **Feature gating is pragmatic and explicit.**
  `parallel` is blocked on wasm32 via `compile_error!`, and features like `model-diff` and `vba` are optional.

**Recommendations**:

* **Reduce reliance on `with_default_session` for “serious” integrations.**
  The thread-local session is convenient, but it’s hidden state. Keep it as a convenience, but consider making an explicit `DiffSession`-based API the “primary” story in docs, especially for hosts that care about deterministic memory usage and lifecycle.

* **Audit “identity via raw pointers” usage.**
  Some pointer-based identity patterns (e.g., in sheet matching) can be correct but surprising in Rust. Prefer stable IDs where possible (sheet ids already exist), or isolate pointer-identity into a tiny helper with explicit safety reasoning.

---

### 4. Maintainability Posture

**Assessment**: **Strong**

**Evidence**:

* **Subsystem isolation is real.**
  The M-language stack (`m_section`, `m_ast`, `m_ast_diff`, `m_semantic_detail`, `m_diff`) is largely contained, which means a future rewrite of parsing or diffing M code won’t require rewiring the grid diff engine. 

* **Tests appear to encode behavior, not just implementation details.**
  The test suite is organized by milestones/domains (e.g., `g*` grid alignment and move tests, `m*` M parsing/diff tests, determinism tests, schema guards).
  There are explicit determinism tests for parallel and streaming modes, which is exactly the kind of invariant that tends to rot if it isn’t enforced.

* **The repository contains maintenance tooling that suggests long-term intent.**
  Scripts like `arch_guard.py`, perf result comparators, and benchmark visualization tooling indicate an engineering culture that expects evolution and regression pressure.

* **One maintainability red flag lives outside core:**
  The combined build/test logs show “unexpected cfg condition value” warnings in the desktop crate for `perf-metrics` and `model-diff`, suggesting workspace feature definitions aren’t fully aligned across crates. This is the sort of “small friction” that becomes chronic over time. 

**Recommendations**:

* **Fix workspace-wide feature hygiene.**
  Ensure downstream crates define and forward the same feature flags (or remove the cfg gates). This reduces warning noise and prevents “feature drift” as the workspace grows. 

* **Document extension seams explicitly.**
  The seams exist (new sink, new diff algorithm, new host type), but documenting “where to plug in” in one place will shorten onboarding time.

---

### 5. Pattern Appropriateness

**Assessment**: **Strong**

**Evidence**:

* **Streaming output is handled with a clean “strategy/visitor” pattern (`DiffSink`).**
  This avoids building giant intermediate outputs and enables different consumers (CLI, web, desktop store) to share the same engine. The `NoFinishSink` pattern used in `diff_streaming` is a subtle but good design: it prevents the engine from finalizing the stream before object + M ops are appended.

* **Hardening behaves like a circuit breaker, not scattered `if` statements.**
  `HardeningController` centralizes policy for time/memory/op-count limits, and returns either partial results (with warnings) or hard errors depending on config. This is a good match for the product problem: users prefer “some diff now” over “crash later.”

* **Algorithm selection via configuration is used judiciously.**
  `DiffConfig` and presets make it possible to tune the engine for “fastest” vs “most precise” without threading flags through every call site.

**Recommendations**:

* **Keep patterns “thin.”**
  The patterns are currently invisible in the best way. The primary risk is adding indirection (trait objects, factory registries) before the domain truly needs it. Prefer adding new strategies behind existing selection points.

---

### 6. Performance Awareness

**Assessment**: **Strong** (with evidence gaps at true 100MB, end-to-end scale)

**Evidence**:

* **The engine is architected to avoid catastrophic paths.**

  * Preflight similarity checks can short-circuit expensive alignment when similarity is low.
  * Memory estimation (`estimate_gridview_bytes`) and hardening limits gate expensive operations.
  * Sparse grids and optional “upgrade to dense” mean memory usage can adapt to sheet characteristics. 

* **Parallelization is guarded by thresholds.**
  Rather than “rayon everywhere,” the code uses explicit heuristics (min rows/cells/cols) and even disables parallel column meta when memory budgets are tight. That’s the kind of nuance that often separates “fast on my machine” from robust performance.

* **Benchmarks and perf tests exist and encode targets.**
  The provided benchmark artifact shows CI-scale scenarios completing in ~2–196ms and reports memory counters; it is explicitly `full_scale: false`. 
  Separately, there are ignored performance tests for large grids (e.g., 50k rows) with explicit timing assertions (e.g., under ~2s or under ~5s), which indicates performance targets are being treated as regressible invariants—not anecdotes. 

* **Algorithmic honesty is present.**
  The alignment module calls out that it uses Hungarian rather than LAPJV and omits histogram diff, and describes its “simplified AMR.” This matters because it sets expectations: worst-case behavior is being bounded by limits and fallbacks rather than by a single theoretically optimal algorithm.

**Recommendations**:

* **Add end-to-end benchmarks that include parsing + diff.**
  Current microbenchmarks (and even some perf tests) can miss the real 100MB workbook cost dominated by zip IO, XML parsing, shared string resolution, and grid construction. A single “open + diff + emit JSONL” benchmark (with representative fixtures) would close the evidence gap.

* **Optimize signature-building for “preflight bailouts.”**
  The benchmark shows scenarios where signature build dominates time (e.g., ~105ms of signatures in a preflight case). If preflight often leads to fallback, consider computing similarity via sampling first, then building full signatures only if alignment is likely to be beneficial.

* **Revisit assignment/move detection solver choice if scaling demands it.**
  Hungarian is correct but can become expensive as problem size grows; if move detection becomes a hotspot at large scale, switching to LAPJV (or a specialized sparse assignment approach) could be a clean, isolated upgrade point (since it’s already encapsulated). 

---

### 7. Future Readiness

**Assessment**: **Strong**

**Evidence**:

* **WASM isn’t an afterthought.**
  There’s a dedicated `wasm` crate that depends on core with `default-features = false`, and the core crate explicitly blocks unsupported features on wasm32. This is exactly the sort of “buildable seam” that prevents platform-specific assumptions from creeping into core logic.

* **UI payload shaping is separated from engine concerns.**
  The `ui_payload` crate builds snapshots and alignments for the web/desktop UI from the engine’s outputs. This prevents the core IR and diff ops from being polluted by UI-only concerns.

* **PBIX/PBIT support matches spec’s warning about “new-style PBIX.”**
  The spec explicitly notes that enhanced-metadata PBIX may omit `DataMashup` and should fall back to tabular model paths. 
  The implementation does this: `PbixPackage::open` checks `DataMashup`, then looks for `DataModelSchema`, and emits a dedicated error (`NoDataMashupUseTabularModel`) when appropriate. Tests assert these behaviors.

* **Feature-gated model diff exists.**
  The model diff path is gated behind `model-diff` and is already integrated into PBIX diffing and (via exports) the public surface.

**Recommendations**:

* **Stabilize and version “public schema surfaces.”**
  `DiffOp` already has a `SCHEMA_VERSION`. Keep that discipline for UI payload schemas too, and treat them as public contracts (especially if the product plan includes integrations and tooling).

* **Plan for “host kinds” as first-class.**
  `WorkbookPackage` vs `PbixPackage` is a solid start. If additional hosts appear (xlsb, csv, database extracts), consider a thin `HostPackage` enum façade at the API level to avoid proliferation of parallel APIs.

---

## Tensions and Trade-offs

* **Streaming vs completeness of information**
  Streaming JSONL wants to emit ops as they are discovered, but the string-table and determinism constraints demand that strings be interned before streaming begins. The “freeze the pool” approach (and the enforcement via sink wrappers) is a conscious trade: it imposes discipline on the engine so that outputs remain stable and streamable.

* **Determinism vs parallelism**
  Parallel speedups are attractive on large grids, but non-deterministic iteration order can poison outputs. The presence of explicit parallel determinism tests suggests the codebase treats determinism as sacred rather than “nice-to-have.”

* **Algorithmic ambition vs safety**
  The alignment/move detection stack aims for meaningful diffs (moves, structure-aware alignment) but includes hardening and fallback paths to avoid worst-case blowups. This is a realistic posture for Excel-scale inputs: correctness matters, but predictability matters more in user workflows.

---

## Areas of Excellence

* **Architecture enforcement as code** (`scripts/arch_guard.py`)
  This is rare and high value: it prevents accidental coupling and keeps the system’s “shape” stable as features are added. 

* **Streaming sink contract + RAII enforcement** (`DiffSink`, `SinkFinishGuard`, `FrozenPoolSink`)
  The output pipeline has been treated like a first-class subsystem with correctness constraints, not a logging side-effect.

* **Hardening controller and limits-first thinking**
  Centralized limit behavior is exactly what a diff engine needs when exposed to adversarial or pathological files.

* **PBIX “new style” support with dedicated UX messaging**
  The implementation doesn’t just fail; it fails with a meaningful, actionable error aligned with the spec’s caveats.

---

## Priority Recommendations

1. **Add an end-to-end (open + diff + emit) benchmark suite for large fixtures.**
   The current artifacts show strong micro performance but don’t validate full workbook handling at 100MB scale. Make this a first-class gate to match product promises.

2. **Make the diff strategy selection explainable via structured tracing.**
   Emit (optionally) a “why this path” record: preflight similarity, alignment path chosen, move detection attempted/skipped, hardening thresholds hit. This is invaluable for debugging user reports.

3. **Treat the alignment implementation notes as technical debt items with owners.**
   The explicit deviations from the unified spec are good; now turn them into trackable work: histogram diff, improved assignment solver, etc. 

4. **Refactor `excel_open_xml.rs` into smaller, domain-named modules.**
   Keep the public behavior identical, but reduce file-level cognitive load and allow specialists (parsing vs diffing) to work in isolation.

5. **Keep `DiffOp` growth under control by grouping semantics.**
   Consider introducing internal sub-enums (`GridOp`, `ObjectOp`, `MOp`, `ModelOp`) and mapping them to the serialized form. The goal is maintainability without breaking schema stability.

6. **Fix workspace feature flag mismatches that generate `unexpected_cfgs` warnings.**
   Warning noise tends to hide real issues; align `Cargo.toml` features across crates (desktop/web/cli) so cfg gates are meaningful. 

7. **Consider a sampling-based preflight to reduce wasted signature work.**
   If preflight regularly leads to fallback, full signature computation may be avoidable in some cases.

---

## Conclusion

The Excel Diff Engine exhibits a rare combination of traits for a system in this problem space: disciplined layering, an IR that matches the domain, streaming-oriented output design, and an explicit posture toward hardening and determinism. The architecture is not only plausible—it is enforceable, and it already shows evidence of being built to survive growth.

The path forward is less about “new patterns” and more about protecting what is already working: keep the architectural boundaries sharp, make algorithm selection explainable, validate performance claims end-to-end, and manage schema complexity as a first-class responsibility. If those are done well, this codebase is structurally prepared to deliver on the product vision (fast, semantic, workflow-native diffs across Excel and Power BI artifacts) without collapsing under its own ambition.
