# Design Evaluation Report

## Executive Summary

The Excel Diff Engine presents as a disciplined, mature system that successfully navigates the tension between academic rigor and engineering pragmatism. The architecture avoids the common trap of becoming a monolithic "Excel Parser," instead functioning as a pipeline of specialized engines: a **Container Layer** that abstracts ZIP/OPC complexity, a **Semantic Layer** that frames proprietary binaries, and a **Domain Layer** that operates on a clean, optimized Internal Representation (`RowMeta`, `Grid`).

Most striking is the system's approach to the "Grid Alignment" problem. Confronted with a specification that calls for complex global optimization (Anchor-Move-Refine), the implementation opts for a simplified, opportunistic strategy (`GapStrategy`) that preserves the spirit of the algorithm while drastically reducing code complexity. This decision—to favor *understandable* correctness over *theoretical* perfection—characterizes the codebase.

While the architectural health is excellent, distinct tensions exist. The reliance on recursion for gap alignment poses a latent stack-overflow risk for the WebAssembly target, and the algorithmic requirement for global frequency analysis creates friction with the goal of pure streaming processing. Nevertheless, the codebase stands as a testament to elite systems programming, balancing high-level domain modeling with low-level performance tuning.

## Dimension Evaluations

### 1. Architectural Integrity

**Assessment**: **Strong**

The system honors the separation of concerns laid out in the documentation with fidelity.

* **Layer Separation**: The **Host Layer** (`container.rs`) is cleanly decoupled from the **Semantic Layer** (`datamashup.rs`). The `OpcContainer` abstraction ensures that high-level logic (like extracting `[Content_Types].xml`) does not bleed into raw ZIP handling.
* **Dependency Direction**: The dependency graph is acyclic and logical. The `alignment` module operates purely on `GridView` and `RowMeta`, remaining completely agnostic to whether the data came from an `.xlsx` or `.pbix` file.
* **IR Coherence**: The separation of `RowMeta` (lightweight, used for alignment) from `CellValue` (heavy, used for storage) is a critical architectural decision. It allows the complex AMR algorithms to run on a "view" of the data without thrashing the CPU cache with full cell contents.

**Evidence**: `core/src/alignment/mod.rs` strictly defines the pipeline: Metadata → Anchors → Chain → Strategy → Assembly.

### 2. Elegant Simplicity

**Assessment**: **Strong**

The codebase demonstrates mastery over complexity, particularly in the diff engine.

* **Mastery of Complexity**: The "Intentional Spec Deviations" documented in `alignment/mod.rs` are a highlight. By rejecting the spec's "Global Move Extraction" phase in favor of opportunistic local move detection (`GapStrategy::MoveCandidate`), the implementation avoids `O(N²)` graph complexity while satisfying the 80/20 rule for real-world usage.
* **Abstraction Fidelity**: `GapStrategy` in `gap_strategy.rs` is a high-fidelity abstraction. It collapses the infinite possibilities of row alignment into seven distinct cases (`Empty`, `InsertAll`, `SmallEdit`, etc.), making the recursive logic in `assembly.rs` easy to reason about.
* **Compression**: The Run-Length Encoding in `runs.rs` is a beautifully simple solution to the "template workbook" problem, collapsing thousands of identical rows into single units.

### 3. Rust Idiomaticity

**Assessment**: **Strong**

The code speaks fluent Rust, utilizing the type system to enforce correctness.

* **Type-Driven Design**: The use of newtypes (`RowSignature`, `StringId`) prevents "stringly typed" logic. `FrequencyClass` is an enum, not an integer, making invariants explicit.
* **Ownership & Slices**: The alignment algorithms heavily utilize slices (`&[RowMeta]`) rather than passing ownership or cloning vectors. This demonstrates a strong grasp of Rust's borrow checker to minimize memory churn.
* **Error Handling**: `thiserror` is used effectively to create domain-specific error hierarchies (`ContainerError`, `ConfigError`) rather than generic failure states.

### 4. Maintainability Posture

**Assessment**: **Strong**

The codebase is highly welcoming to future maintainers.

* **Documentation**: The module-level documentation in `alignment/mod.rs` is exemplary. It doesn't just explain *what* the code does, but *why* it differs from the spec. This context is invaluable.
* **Configurability**: `DiffConfig` (`config.rs`) centralizes all "magic numbers" (thresholds, limits). This prevents algorithmic tuning from requiring code changes and allows the engine to be adapted for different environments (e.g., "Fastest" vs. "Most Precise").
* **Testing**: The test suite is granular. Files like `g12_rect_block_move_grid_workbook_tests.rs` suggest a focus on specific topological scenarios rather than just end-to-end smoke tests.

### 5. Pattern Appropriateness

**Assessment**: **Strong**

Patterns are applied judiciously and effectively.

* **Strategy Pattern**: The `GapStrategy` enum is the correct tool for the alignment problem, avoiding a sprawl of `if/else` statements.
* **Builder Pattern**: `DiffConfigBuilder` allows for readable configuration of the complex diff engine without massive constructor signatures.
* **Interning**: The use of `StringId` (implied by `string_pool.rs`) is the correct pattern for an Excel engine, where repeated strings ("General", "Arial") are ubiquitous.

### 6. Performance Awareness

**Assessment**: **Exemplary**

The specification's demand for "instant diff" has clearly influenced the design.

* **Algorithmic Choices**: The LIS algorithm in `anchor_chain.rs` is `O(N log N)`, suitable for large datasets.
* **Work Limits**: `DiffConfig` enforces strict bounds (`lcs_dp_work_limit`, `max_recursion_depth`). The system explicitly defends itself against "diff bombs" (inputs triggering quadratic behavior) by falling back to coarser algorithms (`HashFallback`) when limits are hit.
* **Memory Efficiency**: The decision to use `u32` for row indices limits the grid to ~4 billion rows (sufficient for Excel's 1M limit) while saving 50% memory on indices compared to `usize` on 64-bit platforms.

### 7. Future Readiness

**Assessment**: **Adequate**

The system is positioned well for growth, though there are architectural seams to watch.

* **WASM Compatibility**: The `wasm_smoke.rs` binary and `wasm.yml` workflow ensure the core logic remains free of OS-specific dependencies.
* **Extension Points**: `datamashup.rs` exposes `Permissions` and `Metadata` structs, ready for the M-language semantic diff logic to populate them.
* **Streaming Tension**: The AMR algorithm relies on global frequency analysis (`classify_row_frequencies`) to identify "Unique" rows. This requires at least one full pass over the data, which conflicts with a pure, single-pass streaming architecture.

## Tensions and Trade-offs

**Spec Purity vs. Engineering Pragmatism**
The specification outlines a rigorous "Global Move Extraction" phase. The implementation trades this for `GapStrategy::MoveCandidate`—a localized, opportunistic move detector.

* *Trade-off*: The engine might fail to identify complex, interleaved moves (e.g., A/B/C -> C/A/B), reporting them as insert/deletes instead.
* *Resolution*: This is the correct decision for an MVP. It reduces code complexity by an order of magnitude while solving the 95% case (simple block moves).

**Recursion vs. Stack Safety**
The AMR algorithm is naturally recursive. The implementation uses explicit recursion in `assembly.rs` (`fill_gap` calls `assemble_from_meta`).

* *Trade-off*: Deep recursion on massive files with specific gap structures could cause stack overflow in WASM environments, which have significantly smaller stacks than native threads.
* *Resolution*: The `DiffConfig` enforces a `max_recursion_depth`. This is a safety valve, but it forces a fallback to potentially suboptimal alignment when hit.

## Areas of Excellence

1. **The `GapStrategy` logic**: It transforms the daunting problem of "diffing two spreadsheets" into a series of small, solvable problems. It is the intellectual heart of the diff engine.
2. **`alignment/mod.rs` documentation**: It is rare to see code that honestly confesses where it deviates from its spec. This intellectual honesty prevents future developers from wasting time "fixing" intentional simplifications.
3. **`runs.rs` Optimization**: Explicitly handling the "template" case (thousands of identical rows) via Run-Length Encoding demonstrates deep domain awareness.

## Priority Recommendations

1. **Investigate Iterative Assembly**: Refactor `assemble_from_meta` to use an explicit heap-allocated stack (iteration) rather than call-stack recursion. This will immunize the WASM build against stack overflows on large files and allow for deeper recursion limits.
2. **Streaming IR Strategy**: To handle 100MB+ files without massive memory pressure, formalize the "Two-Pass" approach: Pass 1 builds only the lightweight `RowMeta` (signatures/indices) to perform alignment; Pass 2 streams the actual cell values for the diff output, using the alignment map generated in Pass 1.
3. **Harden Move Extraction**: Create a specific test suite of "interleaved moves" (A/B/C -> C/A/B) to document exactly which move patterns are currently detected and which are missed. This sets the baseline for future improvements to the move detector.

## Conclusion

The Excel Diff Engine is a well-architected, thoughtful system. It avoids the trap of over-engineering by implementing a "good enough" version of the complex alignment algorithms while maintaining rigorous architectural boundaries. It is idiomatic, maintainable, and visibly tuned for performance. The codebase reflects a maturity that values simplicity and clarity over algorithmic perfection, resulting in a system that is likely to be both robust in production and adaptable to future requirements.