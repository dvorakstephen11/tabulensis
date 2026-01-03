# Design Evaluation Report

## Executive Summary

The Excel Diff Engine presents a mature, thoughtfully layered architecture that closely tracks the intent of the specifications and product plan. The core IR (workbook, grid, diff ops), grid alignment pipeline, and M/DataMashup subsystems form a coherent whole: host-specific parsing is kept at the edges, while a relatively small set of domain-centric types (`Workbook`, `Grid`, `DiffOp`, `DataMashup`, `Query`) carry the semantics through the system. The implementation does not yet realize every advanced algorithm described in the grid and M-diff specifications, but it positions those capabilities well: the IR is stable and expressive enough, and the current algorithms form a plausible MVP that can be upgraded rather than replaced.

Architecturally, the system honors a clear layering: Host Container → Binary Framing → Domain IR → Diff Algorithms → Output. Excel Open XML and DataMashup framing are tightly scoped adapters; the IR and diff logic are host-agnostic and re-exported cleanly through `lib.rs`. The grid-processing layer (hashing, grid views, alignment, block-move detection) reflects the difficulty analysis: most of the complexity is concentrated there, and the code is correspondingly dense but not chaotic.

From a Rust perspective, the code is idiomatic and disciplined: ownership is straightforward, error types are domain-specific and propagated via `Result`, and invariants are encoded both in types and in debug assertions. The test suite is extensive and aligned with the testing plan: IR semantics, grid alignment, block moves, database mode, M parsing, and semantic gating all have focused tests that double as executable documentation.

The main gaps are not in structure but in depth. The grid engine implements a single-pass heuristic pipeline rather than the fully staged hybrid alignment (Patience anchoring + adaptive Myers/Histogram + LAPJV) described in the unified grid spec, and M diff currently stops at “query-level alignment + semantic equality gate” rather than full step-level semantic diff and rename detection. These are deliberate deferrals rather than architectural mistakes; the current design remains compatible with the roadmap, but there is still significant algorithmic work ahead.

---

## Dimension Evaluations

### 1. Architectural Integrity

**Assessment**: **Strong**

**Evidence**

* **Layering matches the spec**: `lib.rs` exposes a clean public surface, with host/container concerns (`container`, `excel_open_xml`, `datamashup_framing`, `datamashup_package`) separated from domain IR (`workbook`, `datamashup`, `m_ast`, `m_section`) and diff logic (`engine`, `row_alignment`, `column_alignment`, `rect_block_move`, `database_alignment`, `m_diff`).
* **Host → framing → domain**:

  * Excel: `excel_open_xml` extracts shared strings, sheets, and DataMashup streams from the OPC package; `grid_parser` builds `Workbook`/`Grid` IR.
  * DataMashup: `datamashup_framing` slices the MS-QDEFF top-level stream into package parts, permissions, metadata, and bindings; `datamashup_package` parses the nested ZIP; `m_section` splits `Section1.m` into members; `datamashup` builds `DataMashup` + `Query` IR.
* **Diff directionality**:

  * `engine::diff_workbooks` compares sheets via a `SheetKey` (lowercased name + kind), then delegates to `diff_grids` for each sheet. No lower-level module depends on `engine`. 
  * `diff_grids_database_mode` lives alongside rather than inside grid modules, consuming `Grid` and `KeyColumnSpec` while emitting pure `DiffOp`s.
* **IR coherence**:

  * `Workbook` is a simple collection of `Sheet`s; `SheetKind` tracks worksheets/charts/macros distinctly, matching the spec’s multi-object model.
  * `DiffOp` encodes semantic operations—sheet add/remove, row/column add/remove, row/column/rect block moves, and cell edits—rather than low-level edit scripts. A versioned `DiffReport` wraps the op list for forward compatibility.
  * M diff operates on the `Query` domain type, not raw strings; `MQueryDiff` with `QueryChangeKind` matches the textual-vs-semantic distinction in the spec and testing plan.

**Recommendations**

1. **Introduce a `Diffable` abstraction** as sketched in the spec (e.g., `trait Diffable { type Diff; fn diff(&self, other: &Self) -> Self::Diff; }`) and implement it for `Workbook`, `Sheet`, `Grid`, `Query`, etc. This would formalize the existing layering and make `diff_workbooks` a thin orchestrator over domain-level diffs. 
2. **Clarify mixed-mode responsibilities**: the spec calls out sheets that are partially database-like and partially freeform. Right now, database mode is exposed as a separate function (`diff_grids_database_mode`) that uses a synthetic sheet id; capturing the “mode decision” for each region in IR or configuration will make that behavior explicit and testable.
3. **Tie code paths explicitly back to spec sections** via doc-comments (e.g., “Implements Section 9.3 Phase 3: Fuzzy block move detection”). This would make the architecture self-documenting in terms of its spec alignment.

---

### 2. Elegant Simplicity

**Assessment**: **Strong, with localized high-density complexity where appropriate**

**Evidence**

* **Essential IR, minimal ornamentation**:

  * `Workbook`, `Sheet`, `Grid`, `Cell`, `CellSnapshot`, `RowSignature`, `ColSignature` are simple value types that closely mirror the spec’s IR (grid + signatures + cell snapshots). No generic over-abstraction; data structures are straightforward and easily inspected in tests.
  * `DiffReport` and `DiffOp` avoid nested type hierarchies; all diff information flows through a single enum plus a handful of helper constructors.
* **Heuristics structured as a pipeline**:

  * `diff_grids` composes the grid diff logic as a readable sequence of increasingly general strategies: exact rect block move → exact row/column block moves → fuzzy row block move (+ edits) → row alignment → single-column alignment → positional diff. This reads as a “narrative” of increasingly expensive fallback paths.
  * Fuzzy row block move detection uses clear “bail-out” phases (size bounds, low-information dominance, heavy repetition, identical meta, prefix/suffix detection, candidate evaluation). Each guard matches a concrete failure mode from the unified grid spec (repetition, ambiguity, adversarial inputs).
* **M diff semantics explained by tests**:

  * The combination of `m6_textual_m_diff_tests`, `m7_ast_canonicalization_tests`, and `m7_semantic_m_diff_tests` makes the behavior of `diff_m_queries` legible without reading much code: formatting-only changes are suppressed, metadata-only changes get `MetadataChangedOnly`, semantic changes become `DefinitionChanged`, and malformed queries fall back to textual comparison.

The main pockets of complexity (e.g., `detect_fuzzy_row_block_move`, M lexer/parser) are complex because the problem is: the spec explicitly calls out grid diff and semantic M diff as the hardest parts of the system. The implementation contains that complexity in a few well-scoped modules (`row_alignment`, `grid_view`, `m_ast`) rather than letting it leak into the engine or IR.

**Recommendations**

1. **Refactor high-complexity functions into visibly phased helpers.** For example, `detect_fuzzy_row_block_move` could be split into `prepare_meta`, `find_mismatch_window`, `enumerate_candidates`, and `validate_candidate`. The current structure is logically phased but physically monolithic; refactoring would make it easier to reason about and extend.
2. **Surface more “why” in doc-comments** on key algorithms (row alignment, column alignment, database alignment) so a reader can connect them directly to the concepts in the unified grid spec (anchors, gaps, LAP-based matching), even if the implementation remains heuristic.
3. **Consider small “explainer” types** for mid-level concepts (e.g., `Alignment` could expose methods like `is_monotonic`, `inserted_block_ranges`) to encode invariants currently buried in tests.

---

### 3. Rust Idiomaticity

**Assessment**: **Strong**

**Evidence**

* **Ownership and borrowing are clear**:

  * `diff_workbooks` builds `HashMap<SheetKey, &Sheet>` and carries only `&Sheet` references into the diff, avoiding unnecessary clones of heavy `Sheet`/`Grid` data. Grid-level functions consume `&Grid`, and `DiffOp`/`DiffReport` own their resulting data.
  * Grid view construction (`GridView::from_grid`) takes `&Grid` and builds light metadata structures; no surprise shared ownership or reference-counting.
* **Precise error handling**:

  * Container/data errors are expressed via `ContainerError`, `DataMashupError`, `SectionParseError`, `MParseError`, etc., all of which encode domain-specific failure modes (BOM handling, missing section header, unterminated strings, unbalanced delimiters).
  * M diff uses `Result<Vec<MQueryDiff>, SectionParseError>`, making it explicit that query diff can fail if section parsing fails, and tests assert the exact error kind for malformed sections.
* **Type-driven invariants and helpers**:

  * `CellAddress` provides `from_indices` and `to_a1`, encoding the conversion between numeric indices and A1 notation; tests validate round-trips and invalid addresses.
  * `DiffOp::cell_edited` asserts at construction time that `from.addr` and `to.addr` match the canonical `addr`, enforcing invariants with debug assertions rather than leaving them to consumers.
  * JSON output is gated on an explicit `contains_non_finite_numbers` check, returning a structured `serde_json::Error` if the diff report cannot be serialized safely.
* **Traits used as contracts, not inheritance**:

  * Crate-level exports (e.g., `build_data_mashup`, `build_queries`, `parse_m_expression`, `diff_m_queries`) are simple free functions rather than pseudo-OOP traits. Internal traits are not over-used; behavior is mostly expressed via functions and enums, which is in line with idiomatic Rust for data-heavy domains.

**Recommendations**

1. **Use newtypes for indices where confusion is risky.** `RowIndex(u32)` / `ColIndex(u32)` (or at least type aliases) would help prevent mixing up row/column indices in more complex algorithms, especially in the grid pipeline.
2. **Consider encoding monotonic alignment as a distinct type**, e.g., `MonotonicAlignment(Vec<(RowIndex, RowIndex)>)` with smart constructors; this would move some test-asserted invariants into the type system.
3. **Add feature flags or `no_std` preparation** for WASM builds, if not already planned, to make the Rust/WASM dual-target explicit at the type and module level.

---

### 4. Maintainability Posture

**Assessment**: **Strong**

**Evidence**

* **Well-drawn module boundaries**:

  * `addressing`, `hashing`, `grid_view`, `row_alignment`, `column_alignment`, `rect_block_move`, `database_alignment` form a cohesive “grid algorithms” cluster. They are internal (`pub(crate)` where appropriate) and accessed primarily via `engine`.
  * M-related code (`m_ast`, `m_section`, `datamashup`, `datamashup_package`, `m_diff`) is clustered cleanly; each file has a focused responsibility (lex/parse, section splitting, package structuring, diff).
* **Tests as living documentation**:

  * IR and diff tests (`pg1_ir_tests`, `pg3_snapshot_tests`, `pg4_diffop_tests`, `engine_tests`, `output_tests`) cover scenarios like sheet matching, duplicate sheet identities, block moves, and cell diff projection. These tests read like examples of intended behavior.
  * Grid tests (G1–G13 and related) cover row/column alignment, sparse/dense grids, various move/insert/delete scenarios, and ambiguous candidates; they serve as executable spec for the grid diff heuristics.
  * M tests (M3–M7) systematically exercise section parsing, metadata binding, textual diff, semantic gating, and error handling, aligning with the testing plan’s milestones.
* **Naming discipline**:

  * Names line up with their conceptual roles (`GridView`, `RowMeta`, `HashStats`, `KeyColumnSpec`, `DataMashup`, `MQueryDiff`, `QueryChangeKind::MetadataChangedOnly`). This consistency makes navigation easier and anchors mental models.

In practice, a developer can usually work within one subsystem—grid alignment, M parsing, DataMashup packaging—without needing to understand the entire codebase. The IR types form a common language across modules, reducing surprise when crossing boundaries.

**Recommendations**

1. **Add top-level “map” docs** (e.g., `docs/architecture_grid.md`, `docs/architecture_m.md`) summarizing where each spec section lives in code. The testing plan already acts as a roadmap; a short architecture guide that cross-links to modules would further reduce onboarding time.
2. **Document long-term extension points** (e.g., where DAX/model diff will attach) so future maintainers don’t inadvertently close off those seams.
3. **Keep complex heuristics isolated behind narrow interfaces**. As the grid and M engines gain sophistication, resist the temptation to leak intermediate structures into the broader API; instead, expose higher-level summaries or metrics as separate, opt-in paths.

---

### 5. Pattern Appropriateness

**Assessment**: **Strong**

**Evidence**

* **Pragmatic use of enums + helper methods**:

  * `DiffOp` as a single enum with smart constructors (`row_added`, `column_removed`, `block_moved_rect`, `cell_edited`) is a textbook fit for Rust’s pattern-matching strengths. It avoids the over-engineering of separate “pattern classes” and keeps the representation transparent.
  * `QueryChangeKind` similarly encodes the product-level categories (Added/Removed/Renamed/DefinitionChanged/MetadataChangedOnly) in a single place, matching testing and spec nomenclature.
* **Internal traits kept minimal**:

  * The spec proposes a `Diffable` trait; the implementation hasn’t yet generalized around it, but the absence of premature trait abstraction is actually a positive right now: behavior is concentrated in a few orchestrating functions (`diff_workbooks`, `diff_m_queries`) with clear signatures.
* **Polymorphism where it matters**:

  * The distinguishing of sheet kinds (`SheetKind::Worksheet|Chart|Macro|Other`) via an enum instead of trait objects is appropriate: behavior differences are handled at diff time (e.g., grid diff only for worksheets), while representation remains simple.
* **Error patterns match domain boundaries**:

  * Separate error types for container parsing, DataMashup framing, M parsing, and M section splitting mirror the spec’s layer boundaries. This avoids one monolithic error enum that would be harder to evolve and reason about.

There’s little evidence of “pattern for pattern’s sake.” The patterns employed (enums with helper constructors, module-level free functions, small error enums) are simple and well-suited to a parsing/diff engine.

**Recommendations**

1. **When introducing `Diffable`, keep it local and domain-focused.** It should unify diff orchestration rather than becoming a generalized “everything implements Diffable” pattern.
2. **Avoid trait objects in hot paths of the grid engine** unless profiling demonstrates a clear need for polymorphism; generic functions and enums are likely sufficient and more in line with current style.
3. **Consider a small “report builder” abstraction** if future UX needs richer, hierarchical presentation; but keep it separate from the low-level `DiffOp` representation to avoid entangling algorithm and UI concerns.

---

### 6. Performance Awareness

**Assessment**: **Strong in design and heuristics, with remaining work to match the full spec**

**Evidence**

* **Hashing and signatures baked in**:

  * `hashing.rs` uses XXH64 with a fixed seed and a simple mixing strategy to build row/column signatures, matching the spec’s emphasis on strong yet cheap hashing for alignment.
  * `GridView` precomputes per-row and per-column metadata (hashes, density, etc.), enabling alignment algorithms to work on signatures rather than raw cell-by-cell comparisons.
* **Guardrails for adversarial input**:

  * Fuzzy row block move detection explicitly checks grid size bounds, low-information dominance, and heavy repetition via `HashStats` and `MAX_HASH_REPEAT`, then bails out instead of emitting dubious moves. Tests cover ambiguous candidates, heavy repetition, and thresholds.
  * Database alignment uses O(N) hash-based indexing for key-based diff; on failure or “no reliable key,” it falls back to spreadsheet mode via `diff_grids`, aligning with the spec’s requirement to avoid bogus “no diff” and to maintain determinism.
* **Testing plan integrates performance from the start**:

  * The testing plan calls out explicit perf and scaling milestones (large dense sheets with few changes, large noisy sheets, metrics export, CI baselines). The code structure (streaming parsing, lightweight IR, alignment heuristics) is compatible with these goals, even if the full benchmark harness isn’t visible yet in the core crate.

Where the implementation diverges from the spec is in the exact choice of algorithms: the current grid engine uses custom heuristics (signatures, prefix/suffix analysis, fuzzy block matching) rather than the fully staged Patience + adaptive Myers/Histogram + LAPJV pipeline. However, the heuristics embody the same principles—anchor via hashes, avoid quadratic behavior, bail out on ambiguity—so the architecture does not preclude a future upgrade to the full algorithm.

**Recommendations**

1. **Instrument the existing grid engine with metrics** (aligned row ratio, candidate block counts, bail-out reasons) and wire them into the `metrics-export` harness described in the testing plan. This will provide hard data to decide where to invest in algorithmic upgrades.
2. **Incrementally adopt unified grid phases**: for example, replace the current anchor/gap logic with Patience Diff over row hashes, then keep the existing mid- and late-stage heuristics as fallbacks. This will move implementation closer to the spec without a rewrite.
3. **Add perf-focused integration tests** (`grid_large_dense`, `grid_large_noise`) as specified, and track time/memory to ensure the engine meets the “instant diff on 100MB files” goal.

---

### 7. Future Readiness

**Assessment**: **Adequate to Strong**

**Evidence**

* **Host-agnostic core with host-specific adapters**:

  * Core IR (`Workbook`, `Grid`, `DataMashup`, `Query`) and diff logic are independent of Excel as a host; `excel_open_xml` and DataMashup extraction are feature-gated and optional. This matches the product-plan aspiration to support PBIX and other hosts later.
* **WASM compatibility in mind**:

  * The specification explicitly targets WASM; the current code avoids OS-specific APIs in core logic, and the parsing stack uses crates (e.g., `quick-xml`, `zip`) that have WASM-friendly variants. The absence of threads or global state in core diff logic simplifies future WASM builds.
* **Clear seams for DAX and model diff**:

  * M diff is already factored as `m_ast` + `m_diff` over a `Query` IR; the spec proposes an analogous approach for DAX and tabular models. Adding `dax_ast`, `dax_diff`, and model IR alongside the existing M modules will slot naturally into the current architecture.
* **Versioned diff schema**:

  * `DiffReport` includes a `version` field with `SCHEMA_VERSION` constant, explicitly designed for forward compatibility of the wire format. This is essential for web clients and long-lived diff archives.

The main future-risk is not structural but **semantic drift**: if the engine’s public API hard-codes assumptions like “one diff pipeline per sheet” or “no explicit notion of object graph diff,” it may be harder to bolt on richer object graphs (measures, tables, relationships) later. The current design is flexible enough, but those assumptions are not yet codified or tested.

**Recommendations**

1. **Define a lightweight object-graph diff layer** that treats sheets, queries, measures, and tables uniformly as “objects with names + metadata + content diff.” This can sit above the current grid and M diff layers and will scale to DAX/model diff.
2. **Add WASM build targets and tests** early, even if only for a subset of functionality, to prevent hidden dependencies from creeping into the core.
3. **Reserve identifiers and `DiffOp` variants** now for future concepts (e.g., `MeasureEdited`, `RelationshipAdded`), even if they are not yet emitted, to reduce future breaking changes.

---

## Tensions and Trade-offs

1. **Heuristic grid diff vs. fully specified algorithm**
   The current grid engine favors relatively hand-rolled heuristics (fuzzy row block move, custom row alignment) over the more elaborate hybrid pipeline in the spec. This reduces implementation complexity and accelerates MVP but risks some edge-case behaviors diverging from the formal design. The tension is between pragmatic, test-driven heuristics and a more mathematically principled alignment pipeline.

2. **Simple, flat IR vs. richer object graph**
   The IR is intentionally simple: workbooks, sheets, grids, DataMashup, queries. This makes it easy to understand and test, but more complex Excel/PBIX structures (data models, relationships, measures) will need an expanded IR. The challenge will be introducing that richness without destabilizing the existing engine and APIs.

3. **Semantic M diff depth vs. performance and scope**
   The M engine currently stops at a semantic equality gate (AST canonicalization + `ast_semantically_equal`) to suppress formatting-only changes while still reporting real semantic changes as `DefinitionChanged`. The full step-aware diff (with detailed step-level change reporting) is more ambitious and computationally heavier. The tension is between “good enough diff categories now” and “deeply structured semantic diffs later.”

4. **Host flexibility vs. implementation effort**
   Targeting Excel and PBIX, plus Mac/Win/Web, pushes the design toward strict layering and host-neutral IR. This is architecturally sound but imposes constraints that make some shortcuts (e.g., COM integration, host-specific hacks) unattractive. The benefit is long-term portability; the cost is increased initial complexity in parsing and packaging.

---

## Areas of Excellence

1. **IR & DiffOp design**
   The core IR and diff representation are particularly well-executed: they are simple, versioned, and semantically meaningful, and they map cleanly to the user-facing categories described in the product and testing docs. `DiffOp` is both expressive and easy to consume (including JSON serialization with stability guarantees).

2. **Grid statistics and fuzzy move detection**
   The `GridView` + `HashStats` + fuzzy row block move logic is a standout: it thoughtfully handles low-information content, repetition, ambiguous candidates, and size thresholds, with comprehensive tests that cover corner cases. This is exactly where the difficulty analysis says the system should concentrate its sophistication.

3. **DataMashup and M parsing robustness**
   The DataMashup framing and M parsing stack is carefully engineered, with attention to BOM handling, comments, malformed input, quoted identifiers, and metadata edge cases. The tests show a strong bias toward robustness and non-panicking behavior even on random or corrupted inputs.

4. **Semantic M diff gating**
   The use of AST canonicalization to distinguish formatting-only differences from semantic changes is a strong differentiator vs. incumbents and is well integrated with the `MQueryDiff` layer. It strikes a nice balance between complexity and value: even without full step-aware diff, the engine already avoids noisy diffs caused solely by whitespace and minor formatting.

5. **Testing discipline and alignment with plan**
   The breadth and focus of the test suite—especially around tricky areas like M parsing, semantic equality, block moves, and database alignment—reflect the testing plan’s philosophy and provide a strong safety net for future refactoring.

---

## Priority Recommendations

1. **Incrementally align the grid engine with the unified grid diff specification.**

   * Introduce explicit anchoring (e.g., Patience Diff on row signatures) and gap-filling phases.
   * Retain the existing fuzzy and exact block-move detectors as specialized refinement steps.
   * Add perf and robustness tests from the grid-focused milestones (P1/P2, G8–G13).

2. **Introduce a `Diffable` trait over IR types and refactor `diff_workbooks` to orchestrate via this abstraction.**
   This will clarify responsibilities between object-graph diff, grid diff, and M diff, and make future DAX/model diff easier to integrate.

3. **Extend M diff toward step-aware semantics and rename detection.**

   * Add query-level signature computation and Hungarian/LAP-based rename detection as described in the spec.
   * Introduce a basic step model (MStep) and step alignment for common transformations (filters, joins, projections).
   * Keep the current `MQueryDiff` categories but add richer detail fields as needed.

4. **Embed performance metrics and regression checks into CI.**

   * Implement the `metrics-export` harness for diff runs.
   * Add canonical large-grid fixtures and enforce no-regression thresholds on runtime and memory.

5. **Add WASM build + smoke tests early.**
   Compile the core crate to WASM and run a subset of parsing + diff tests in a WASM environment to ensure no hidden platform dependencies and to validate determinism across platforms.

6. **Document and test mixed database/spreadsheet sheets.**
   Implement and test the behavior for sheets that mix keyed table regions with free-form cells, as called out in the testing plan (D10). Make the mode decision explicit, not emergent.

7. **Refactor and document high-density heuristic code.**
   Split especially dense functions (fuzzy moves, complex parsers) into well-named phases and add doc-comments linking them to specific spec sections and test scenarios, to keep them approachable as they evolve.

---

## Conclusion

The current Excel Diff Engine codebase is architecturally sound, well-aligned with its specifications, and written in idiomatic, maintainable Rust. It successfully channels the complexity of Excel, DataMashup, and M into a small set of coherent IR types and modules, concentrating algorithmic sophistication where the difficulty analysis says it belongs: grid alignment and semantic M diff.

The system is already differentiated from typical incumbents by its semantic awareness and host-independent design, and it appears ready to support the next stages of the roadmap—richer grid algorithms, DAX/model diff, and WASM deployment—without major architectural surgery. The remaining work is largely a matter of deepening the algorithms and tightening the feedback loop between spec, metrics, and implementation. If the recommendations above are followed—with particular emphasis on grid algorithm refinement, Diffable abstractions, and step-aware M diff—the engine will not only remain robust and maintainable, but also fulfill the product vision of a “modern Excel diff” that understands spreadsheets as full-fledged data products rather than mere grids of numbers.
