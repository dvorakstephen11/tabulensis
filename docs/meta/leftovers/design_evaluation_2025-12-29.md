# Design Evaluation Report

## Executive Summary

The codebase presents a clear architectural spine: it moves from **container + safety limits** (ZIP/OPC validation and bounded reads) into **semantic parsing** (Open XML sheets, DataMashup, optional VBA/model), then into a deliberately compact **IR** (Workbook/Sheet/Grid with interned strings), and finally into a **diff engine** that emits a versioned, streamable `DiffReport` made of `DiffOp` events. That “bytes → meaning → IR → operations” progression is easy to trace through the public entry points in `WorkbookPackage` and the engine’s `diff_workbooks_streaming` family. 

Architecturally, this aligns well with the product’s stated positioning: fast, cross-platform, and semantically deep diffs (not just cell-by-cell deltas) across Excel, Power Query (M), and model artifacts, with an eye toward CLI + WASM + desktop packaging. The existence of dedicated subsystems for M parsing/diffing, model diffing behind feature flags, and a WASM smoke path indicates the implementation is already shaped around that product “north star,” not merely retrofitted for it. 

The main architectural pressures are where the domain is hardest: alignment/move recognition, and resource discipline (time/memory/ops) under adversarial grids. The design already acknowledges this with explicit “hardening” controls, bounded global-move extraction, and multiple fallbacks. Benchmarks show strong speed on the included scenarios (sub-200ms worst case in the provided set), but also highlight that memory peaks can be non-trivial for low-similarity cases, and that the current benchmark artifact doesn’t capture parse cost (many `parse_time_ms` are 0), which matters for the “100MB file” ambition. 

One note on the evaluation inputs: I did not have the literal text of `excel_diff_specification.md` or `unified_grid_diff_algorithm_specification.md` in the provided materials; the evaluation therefore relies on (a) the code, (b) the testing/difficulty/product docs you provided, and (c) the code’s own internal references to those specs (including explicit “spec section” mapping and documented deviations). 


## Dimension Evaluations

### 1. Architectural Integrity
**Assessment**: Strong

**Evidence**:
- **Layering is visible in the code’s “shape.”**  
  - Container/OPC validation and bounded I/O are isolated in `container.rs` (`ZipContainer`, `OpcContainer`, `ContainerLimits`), including explicit zip-bomb style safeguards (max entries / per-part / total uncompressed).   
  - Semantic parsing for `.xlsx` lives in `excel_open_xml.rs` and `grid_parser.rs`, turning sheet XML into `Grid` and workbook metadata into IR types.   
  - DataMashup parsing is split into **framing** (`datamashup_framing.rs`) and **domain building** (`datamashup.rs`), reflecting the “binary framing → semantic interpretation” boundary.   
  - Diff orchestration is explicitly centralized in the engine module (`engine/mod.rs`), which re-exports workbook/grid diff entry points while keeping internals modular (`workbook_diff`, `grid_diff`, `move_mask`, `hardening`). 

- **IR coherence feels intentional, not incidental.**  
  - `Workbook` is a clean container for sheets plus a small set of “non-grid” artifacts (named ranges, charts), and `Sheet` is mostly “name + kind + grid.” :contentReference[oaicite:8]{index=8}  
  - `Grid` has explicit support for dense vs sparse storage with a documented switching heuristic (`DENSE_RATIO_THRESHOLD`, `DENSE_MIN_CELLS`), which suggests the IR was designed to support performance needs rather than merely mirroring parsing output. :contentReference[oaicite:9]{index=9}  
  - Output is mediated through a stable-ish event vocabulary (`DiffOp`) and an explicit report schema version, indicating the IR-to-output boundary is treated as a contract. 

- **Dependency direction is mostly “downward.”**  
  - Container/framing modules depend on shared error codes, not on workbook/diff logic.   
  - The diff engine depends on the IR and parsing products, as expected, rather than the reverse. 

**Recommendations**:
- **Make boundary enforcement more explicit.** Right now, layer separation is mostly conventional (module-level), not enforced by crate boundaries. If you want long-term integrity, consider splitting `core` into subcrates like `container`, `parse_openxml`, `parse_datamashup`, `ir`, `engine`, even if they remain in a workspace. This prevents accidental “upward” dependencies over time.
- **Tighten “umbrella error types.”** `PackageError` includes a `Diff` variant, which can blur the “open vs diff” responsibility boundary if it spreads. Keeping parsing error types separate from diff/runtime error types typically improves mental locality for maintainers. 
- **Consider an explicit “parse output IR” layer boundary.** `open_data_mashup_from_container` does base64 extraction and returns a `RawDataMashup` that later becomes `DataMashup`—good. The same clarity could be reinforced for other artifacts (charts, defined names) by ensuring parsing returns pure IR without embedding diff assumptions. 


### 2. Elegant Simplicity
**Assessment**: Adequate

**Evidence**:
- **Essential complexity is isolated rather than smeared.**  
  The hardest domain work—grid alignment with moves—is concentrated in `alignment/*`, `engine/move_mask.rs`, and `engine/amr.rs`, with a documented pipeline that maps directly to the unified grid diff spec sections (anchors, gap strategies, assembly).   
  Similarly, M semantic diff complexity is cordoned into `m_*` modules rather than interleaved with grid diff code paths. 

- **The core IR is intentionally small.**  
  `Workbook → Sheet → Grid` plus a few workbook-level artifacts is a simple mental model. It’s not pretending to be “Excel’s full universe,” which is often the right move for a diff engine (you only model what you can compare meaningfully). 

- **There is some accidental complexity risk in configuration surface area.**  
  `DiffConfig` centralizes tuning knobs (good), but it is large and multi-domain (alignment thresholds, move extraction caps, preflight heuristics, hardening, semantic policy). Even with presets (`fastest`, `balanced`, `most_precise`) and validation, the density of options increases cognitive load for contributors and for downstream API users. 

- **One notable “simplicity tax”: wrapper-based diffing.**  
  `Diffable for Sheet` clones into temporary `Workbook` instances to reuse workbook diff orchestration. It’s pragmatic, but it’s also a place where the abstraction is doing work for the programmer rather than for the reader: it introduces extra mental steps (“why are we building a workbook to diff a sheet?”) and potential overhead. :contentReference[oaicite:19]{index=19}

**Recommendations**:
- **Refactor `DiffConfig` into named sub-structs** (e.g., `AlignmentConfig`, `MoveConfig`, `PreflightConfig`, `HardeningConfig`, `SemanticConfig`) while keeping serde compatibility. This preserves configurability while making the “shape” of the engine easier to reason about.
- **Offer direct APIs for common leaf diffs** (grid-only, sheet-only) to eliminate the need for “temporary workbook” wrappers in `Diffable`. Keep the reuse internally, but expose a simpler route to callers and tests.
- **Add a short “conceptual narrative” doc** in `core/src/lib.rs` or `/docs` that explicitly walks parse → IR → diff → output with the key types. The code is already modular; a small narrative layer would make it feel simpler to newcomers.


### 3. Rust Idiomaticity
**Assessment**: Strong

**Evidence**:
- **Type-driven boundaries are real, not cosmetic.**  
  The use of interned string IDs (`StringId`) and a `StringPool` makes ownership and deduplication explicit, and it naturally supports compact diff reports.   
  `DiffOp` is `#[non_exhaustive]`, acknowledging future extension without breaking downstream exhaustive matches. :contentReference[oaicite:21]{index=21}

- **Error handling is “Rust-native.”**  
  Errors are represented as typed enums (with `thiserror`), not thrown/exception-like, and flow through `Result`. Many errors include stable codes and user-facing suggestions—useful for a CLI/SDK boundary. 

- **Ownership flows mostly match the domain.**  
  Parsing creates owned IR; diffing borrows `&Grid`/`&Workbook` and streams results to sinks. The engine’s `EmitCtx` bundles references and mutable caches (`FormulaParseCache`) in an idiomatic “context struct” style. 

- **Performance-minded Rust choices show up naturally.**  
  Dense/sparse storage selection, saturating arithmetic in estimators, and caps for move extraction reflect “systems programming” constraints rather than purely algorithmic purity. 

**Recommendations**:
- **Reduce clone-based escape hatches** where feasible (especially the sheet-to-workbook wrapper). Rust makes ownership explicit; fighting that with structural clones is sometimes warranted, but it’s usually better to give the engine a direct path.
- **Keep feature-gating disciplined** as the codebase grows (WASM vs host, model diff vs spreadsheet-only). The current trend is good; preserving it will keep Rust’s compilation model working for you rather than against you. 


### 4. Maintainability Posture
**Assessment**: Strong

**Evidence**:
- **Subsystem boundaries are clear enough to work locally.**  
  A developer can focus on one major “chapter” (Open XML parsing, DataMashup parsing, grid alignment, M semantic diff, formula diff) without immediately needing to understand the whole engine. The module graph largely mirrors the conceptual graph. 

- **Tests function as living documentation, and are structured by behavior.**  
  The test suite is extensive and named by capability areas (grid alignment/moves, M parsing tiers, formula canonicalization, database mode, hardening, determinism). This aligns with a “sacred invariants” approach described in the testing plan: correctness + determinism + limit behavior.   
  The combined test runs show broad pass coverage (including PBIX host support and parallel determinism). 

- **Fuzzing exists where it matters.**  
  There are fuzz targets for parsing and diffing (`fuzz_open_workbook`, `fuzz_datamashup_parse`, `fuzz_diff_grids`, `fuzz_m_section_and_ast`). That is an unusually strong maintainability signal for format-parsing code. :contentReference[oaicite:29]{index=29}

- **One maintainability weak spot: fixture hygiene can break CI semantics.**  
  The combined test history shows at least one failure that was not algorithmic but environmental (missing PBIX fixture path). That’s not a “design flaw,” but it *is* a maintenance hazard unless fixtures are treated as first-class artifacts with reproducible generation and CI validation. 

**Recommendations**:
- **Make fixtures reproducible by construction.**  
  The repository already includes a Python fixtures framework; ensure CI can (a) generate fixtures or (b) validate fixture presence and checksums so missing-file failures don’t leak into unrelated workstreams. 
- **Add “entry-point maps” for maintainers.**  
  For example: “If you’re changing alignment, start in `engine/grid_diff.rs` → `move_mask.rs` → `alignment/*`.” These are small docs, but they reduce the onboarding tax.
- **Keep determinism as a named invariant.**  
  You already have determinism tests; continue to treat deterministic output order as non-negotiable, especially as parallelization grows. 


### 5. Pattern Appropriateness
**Assessment**: Strong

**Evidence**:
- **Streaming via `DiffSink` is the right “invisible pattern.”**  
  The engine emits ops to an abstract sink (`begin/emit/finish`), allowing CLI, JSONL streaming, UI payload building, and future networked streaming without changing core algorithms. This is a domain-perfect fit: spreadsheet diffs can become enormous, and streaming is the only sane way to keep memory predictable. 

- **Strategy selection is applied where it belongs.**  
  The alignment code chooses gap strategies based on metadata + config thresholds, rather than hardcoding one diff algorithm. That’s the Strategy pattern applied to a real domain need: different gap sizes and similarity regimes require different algorithms. 

- **Capped global move extraction is a pragmatic, product-minded pattern choice.**  
  The alignment module explicitly documents bounded global move extraction and an RLE fast path for repetitive grids as intentional simplifications. That’s a healthy stance: patterns and algorithms are used as tools, not as purity contests. 

- **One pattern that may be overreaching: `Diffable` as a universal interface.**  
  It is elegant when it directly maps to diffable entities, but the Sheet implementation that constructs a temporary workbook is a hint that the abstraction is being stretched to maximize reuse rather than clarity. 

**Recommendations**:
- **Keep `DiffSink` as the core extensibility seam.** It’s a strong decision that will pay dividends for WASM and desktop integration.
- **Re-scope `Diffable` to where it is truly natural**, and consider making “workbook diff orchestration” explicitly a workbook concern (not a generic trait trick).
- **Document sink lifecycle invariants** (e.g., whether `finish` must be called exactly once, how partial results are represented) and test them heavily; the existence of `NoFinishSink` suggests you already treat this as important. 


### 6. Performance Awareness
**Assessment**: Adequate

**Evidence**:
- **Performance guardrails are built into the architecture, not bolted on.**  
  `HardeningController` checks timeouts, memory caps, and operation limits, and can force fallbacks while preserving partial output + warnings. This directly supports “don’t freeze the UI / don’t OOM the browser” constraints.   
  Move detection is gated by maximum rows/cols (`max_move_detection_rows`, `max_move_detection_cols`), which prevents expensive strategies from triggering on large matrices. 

- **The alignment pipeline is performance-shaped.**  
  Anchor-based alignment (unique/rare signatures), bounded global move extraction, and RLE fast paths all exist explicitly to avoid worst-case behavior on repetitive or adversarial data. 

- **Benchmarks show good speed on the provided workload, but memory peaks are notable.**  
  In `benchmark_results.json`, the slowest benchmark in the set (`preflight_low_similarity`) reports ~196ms total time with a peak memory around 100,503,830 bytes (~96MB). Other cases (e.g., `perf_p2_large_noise`) report ~74ms with ~64MB peak.   
  Total suite time is ~451ms across seven benchmarks in that artifact. :contentReference[oaicite:42]{index=42}

- **But the benchmark artifact appears to under-represent parsing cost.**  
  Several tests report `parse_time_ms: 0`, implying the benchmark is primarily measuring diff of already-built grids rather than “open + parse + diff.” For a “100MB file” claim, parse time (zip + XML parsing + shared string decoding) is a first-class cost center. 

**Recommendations**:
- **Add end-to-end benchmarks that include parsing.**  
  Measure `WorkbookPackage::open` + diff on realistic large `.xlsx` (and PBIX where applicable), record parse time, and keep results versioned alongside the existing metrics. The architecture already tracks perf phases; lean into it. 
- **Investigate peak memory in low-similarity regimes.**  
  Low similarity often triggers “extra structure building for little alignment benefit.” Consider early exits that *avoid* constructing high-footprint metadata when similarity is already below the bailout threshold, or reuse arenas for row/col meta buffers.
- **Ensure WASM budgets are explicitly tested.**  
  96MB peak on a single test might be fine on desktop, borderline in some browser contexts, and dangerous when paired with rendering. Make “peak memory under X for Y-row cases” a named, automated invariant.


### 7. Future Readiness
**Assessment**: Strong

**Evidence**:
- **The codebase already contains the seams required by the product roadmap.**  
  The product differentiation plan emphasizes platform reach (including web/WASM) and semantic depth (M, data model). The repo has explicit WASM support paths and separate modules for M and model diffing, not just “TODO” stubs. 

- **Extension via `DiffOp` is intentionally future-friendly.**  
  `DiffOp` is non-exhaustive, and `DiffReport` includes schema versioning, warnings, and completeness flags—this supports iterative output evolution without breaking consumers. 

- **Optional domains are feature-gated.**  
  Model diff is clearly separated and can be compiled out, which is important both for WASM constraints and for keeping the “core spreadsheet diff” lean. 

- **Determinism and parallelism are being treated as compatible goals.**  
  The existence of parallel determinism tests indicates you’re not naïvely trading correctness for speed. 

**Recommendations**:
- **Preserve “core purity” for WASM.**  
  Keep host-only dependencies gated and avoid leaking filesystem assumptions into the core diff logic. The current approach is sound; it’s just easy to regress accidentally as features grow. 
- **Consider richer sheet identity in IR as roadmap grows.**  
  Today’s sheet matching is largely name-based; future features like three-way merges or robust rename detection may benefit from keeping workbook-internal IDs (when available) in the IR.


## Tensions and Trade-offs

- **Semantic depth vs performance predictability**:  
  Semantic diffing for M and formulas is a differentiator, but it competes with “instant diff” constraints. The current design resolves this by making semantics configurable (`enable_m_semantic_diff`, `enable_formula_semantic_diff`) and by giving the engine hardening/fallback routes that preserve partial correctness. 

- **Algorithmic completeness vs maintainability**:  
  The alignment subsystem explicitly deviates from the full spec in bounded ways (caps for determinism, RLE fast paths). This is a healthy trade: the system chooses “good enough for real spreadsheets” over theoretical maximality, while leaving room to deepen the algorithm later. 

- **Streaming outputs vs global reasoning**:  
  Streaming `DiffOp` emission is the right architectural choice, but some diff decisions (move extraction, block replacement heuristics) benefit from global knowledge. The engine resolves this by building `GridView` metadata and then streaming ops, which is a reasonable middle ground—but it drives memory peaks in some cases. 

- **Configurability vs readability**:  
  Centralizing thresholds in `DiffConfig` prevents scattered magic constants, but it can also make the system feel “parameter-heavy.” Presets help, but maintainers will still pay an attention cost as the surface grows. 


## Areas of Excellence

- **Safety-first container handling**: zip/OPC validation with explicit uncompressed size budgets is exactly the kind of “boring correctness” that keeps real-world tools stable. 

- **Alignment subsystem transparency**: the mapping from implementation modules to spec sections, plus documented deviations, is unusually good for a complex diff engine. It turns an algorithmic black box into a navigable system. 

- **Streaming architecture via sinks**: `DiffSink` is an ideal seam for CLI, UI, and WASM; it keeps the engine decoupled from presentation. 

- **Invariants-as-tests posture**: deterministic ordering, hardening behaviors, and key algorithm properties (monotone pairs, swap detection semantics, etc.) are encoded in tests. That’s exactly how you keep a diff engine from “slowly lying” over time. 


## Priority Recommendations

1. **Add end-to-end (parse + diff) benchmarks on large real artifacts**, and store results alongside current benchmark JSON. Today’s benchmark data is valuable but under-represents parsing cost. 

2. **Treat peak memory as a first-class performance metric**, especially for WASM. Investigate low-similarity cases where building metadata may outweigh alignment value, and add “memory budget” regression tests. 

3. **Refactor `DiffConfig` into nested sub-configs** to reduce cognitive load while preserving knobs. Keep presets as the primary user-facing surface. 

4. **Remove or bypass clone-heavy wrapper diffs** (`Diffable for Sheet`) by adding direct APIs for grid/sheet diffing. Keep orchestration reuse internally, but make the abstraction read naturally. 

5. **Enforce architectural boundaries mechanically** (subcrates or stricter `pub(crate)` discipline) to prevent gradual layer leakage as more domains (DAX/model/PBIX details) land. 

6. **Harden fixture management** so CI failures don’t come from missing files. Make fixtures reproducible or validated with manifests/checksums. 

7. **Keep determinism as a non-negotiable invariant** as parallel paths expand; continue investing in determinism tests and stable ordering in engine emission. 


## Conclusion

This codebase already behaves like a system with a stable internal logic: it has a compact IR, explicit parsing boundaries, a modular alignment engine shaped by real-world constraints, and a streamable output model that is well-suited to CLI, desktop, and WASM consumers. The architecture is not merely “working”; it is oriented toward the product’s differentiators—semantic depth and cross-platform delivery—while maintaining a strong posture on safety, determinism, and test-backed invariants. 

The next phase of architectural maturation should focus on *resource truthfulness* (end-to-end benchmarks, memory budgets, parse-time accounting) and on *cognitive economy* (taming config complexity and removing “abstraction stretches” that require too much explanation). If you execute those improvements without losing the current modular clarity, the system looks well-positioned to scale into the roadmap domains (PBIX, data models, richer semantic diffs) while staying fast, deterministic, and maintainable. 
