# Design Evaluation Report

## Executive Summary

The Excel Diff Engine represents a robust, domain-aware architectural effort that effectively manages the high accidental complexity of the Excel file format. The system exhibits excellent **Architectural Integrity** in its structural layers, particularly the "Dual-IR" strategy that separates the "Persistence IR" (`Grid` using sparse storage) from the "Computation IR" (`GridView` using dense, ordered vectors). This approach successfully resolves the tension between efficient parsing of huge, sparse XML files and the linear access patterns required for diff algorithms.

However, a critical divergence exists between the implementation and the specifications regarding the core diff algorithm and performance capabilities. While the specification describes a holistic **Anchor-Move-Refine (AMR)** algorithm, the implementation in `engine.rs` uses a **Greedy Waterfall** strategy (sequentially checking for moves, then falling back to alignment). Furthermore, strict safety caps (`MAX_ALIGN_ROWS = 2,000`) place the engine in a "Functional MVP" state, preventing it from meeting the "50,000 row" performance target. The architecture is boxing with one hand tied behind its back, trading scalability for safety by aborting smart diffs on large files.

## Dimension Evaluations

### 1. Architectural Integrity
**Assessment**: **Strong (Structural) / Divergent (Algorithmic)**
**Evidence**:
- **Layer Separation**: The system rigorously honors the "Host → Framing → Semantic → Domain" layers. The `grid_parser` depends only on `workbook` and has zero knowledge of diff logic. `datamashup` effectively isolates the binary framing complexity.
- **IR Coherence**: The `Grid` struct correctly implements the "Layer 0" specification using `HashMap` for sparse storage, prioritizing memory efficiency over raw iteration speed.
- **Algorithmic Divergence**: The `unified_grid_diff_algorithm_specification.md` describes an "Anchor-Move-Refine" algorithm where move candidates are extracted *during* global anchor discovery. The actual code in `engine.rs` implements a priority chain: `detect_exact_rect_block_move` → `detect_exact_row_block_move` → `align_row_changes`. This simpler, greedy approach risks missing complex interleaved edits that a global anchor approach would catch.

### 2. Elegant Simplicity
**Assessment**: **Adequate**
**Evidence**:
- **Abstraction Fidelity**: `CellSnapshot` is a brilliant simplification. By stripping a cell down to its semantic identity (`value` + `formula`) and implementing `PartialEq` to ignore its address, it trivializes the logic for `DiffOp::CellEdited`.
- **Accidental Complexity**: The `row_alignment.rs` module contains dense, imperative logic (manual index management, `while` loops with mutable cursors) that is difficult to verify. This contrasts with the declarative elegance of the `diff` module.
- **Orchestration**: The `diff_grids` function in `engine.rs` acts as a clear waterfall of responsibilities, making the priority of operations (Rect Moves > Row Moves > Alignment) immediately obvious.

### 3. Rust Idiomaticity
**Assessment**: **Excellent**
**Evidence**:
- **Ownership & Lifetimes**: The `GridView<'a>` struct is a masterpiece of Rust design. It leverages lifetimes to create a sortable, hashable view over the `Grid` without a single clone of the heavy cell data, enabling zero-copy analysis.
- **Error Handling**: `thiserror` is used effectively to create precise, typed error hierarchies (`ContainerError`, `DataMashupError`).
- **Type Safety**: Newtypes (`SheetId`, `RowSignature`) and enums (`SheetKind`) prevent "stringly typed" logic errors.

### 4. Maintainability Posture
**Assessment**: **Strong**
**Evidence**:
- **Testing as Documentation**: The integration of Python-generated fixtures (`fixtures/`) with Rust tests allows for precise verification of edge cases (like "fuzzy moves") that are hard to replicate with real-world files. Test names directly reference spec requirements.
- **Change Isolation**: The M language parser is isolated; a rewrite there would not impact the Grid engine.
- **Magic Numbers**: Key heuristic constants (e.g., `MAX_ALIGN_ROWS = 2000`, `MAX_BLOCK_GAP`) are hardcoded `const` definitions in modules, preventing runtime tuning.

### 5. Pattern Appropriateness
**Assessment**: **Strong**
**Evidence**:
- **View Pattern**: The `GridView` effectively adapts the storage-optimized `Grid` (HashMap) into an algorithm-optimized form (Vector) lazily.
- **Visitor**: The XML parsing uses `quick-xml`'s pull/event model, correctly avoiding DOM loading for massive XML files.
- **Strategy**: The `engine.rs` selects between `diff_grids` and `diff_grids_database_mode`, implementing a nascent Strategy pattern.

### 6. Performance Awareness
**Assessment**: **Concerning**
**Evidence**:
- **The Safety Cap**: `row_alignment.rs` defines `MAX_ALIGN_ROWS = 2_000`. This hard limit protects the system from O(N²) blowups but explicitly fails the "50,000 rows" requirement. The engine effectively gives up on smart diffing for large files.
- **Missing Compression**: The "Run-Length Encoding (RLE)" strategy described in the specification for handling repetitive rows (the "99% blank" case) is absent from the implementation. This missing feature is likely the reason for the low row limit.
- **Allocation**: `GridView` allocates vectors for every row. For a 100MB file, this creates millions of small allocations, creating significant pressure despite the zero-copy cell design.

### 7. Future Readiness
**Assessment**: **Mixed**
**Evidence**:
- **WASM Blocker**: The core specification requires WASM support, yet `core/src/container.rs` relies heavily on `std::fs::File`. This standard I/O dependency blocks the engine from running in a browser environment.
- **Extensibility**: `DiffOp` is `#[non_exhaustive]`, anticipating future semantic operations.
- **M Language**: The `m_ast` module is mature enough to support the planned semantic comparisons.

---

## Tensions and Trade-offs

**Greedy Heuristics vs. Holistic Alignment**
The current `engine.rs` chooses a "Greedy Waterfall" approach (find a move, apply it, return) over the holistic AMR approach described in the spec.
*   **Trade-off**: The greedy approach is simpler to implement and debug but risks local optima (e.g., misinterpreting a complex edit as a block move). The holistic approach is robust but complex.
*   **Resolution**: The current state is an acceptable MVP, but the deviation from the spec limits the engine's "Elite" potential.

**Sparsity vs. Iteration Speed**
The `Grid` uses `HashMap` for storage.
*   **Trade-off**: Optimizes memory for truly sparse sheets but penalizes iteration speed (cache misses) for dense data. `GridView` pays the cost of linearizing this data once per diff.
*   **Resolution**: This is a sound decision for correctness, ensuring the system doesn't crash on "University of Horrors" files (e.g., data at `A1` and `XFD1048576`).

---

## Areas of Excellence

1.  **`GridView` Abstraction**: This is the architectural linchpin. It successfully isolates the messy storage reality (sparse HashMaps) from the clean algorithmic requirement (sorted slices), leveraging Rust lifetimes to do so with zero cell-data copying.
2.  **Semantic M Parsing**: The `datamashup` module demonstrates exceptional handling of legacy/obfuscated binary formats (`MS-QDEFF`), turning opaque blobs into structured, typed data without leaking abstraction details.
3.  **Fixture Strategy**: The use of Python generators to create "chemically pure" test files (`fixtures/src/generators`) allows for precise verification of algorithms against ground truth, independent of Excel's binary volatility.

---

## Priority Recommendations

1.  **Abstract Container I/O (High)**
    *   **Rationale**: The product vision requires a browser-based WASM engine. `core/src/container.rs` currently blocks this by using `std::fs`.
    *   **Action**: Refactor `OpcContainer` to accept a generic `R: Read + Seek` trait. This allows it to consume a `Cursor<Vec<u8>>` in WASM or a `File` on Desktop.

2.  **Implement RLE and Lift Limits (High)**
    *   **Rationale**: The `MAX_ALIGN_ROWS = 2000` cap prevents the engine from handling real-world datasets (50k rows).
    *   **Action**: Implement the **Run-Length Encoding (RLE)** optimization described in the `Unified Grid Diff Algorithm Specification`. Compressing repetitive rows will allow the safety cap to be raised or removed.

3.  **Refactor Engine to AMR (Medium)**
    *   **Rationale**: The greedy detection strategy is less robust than the specified Anchor-Move-Refine approach.
    *   **Action**: Refactor `engine.rs` to compute high-confidence anchors *first* to partition the grid, and then apply move detection within gaps.

4.  **Configurable Limits (Medium)**
    *   **Rationale**: Hardcoded constants prevent runtime tuning.
    *   **Action**: Lift constants like `MAX_ALIGN_ROWS` into a `DiffConfig` struct passed through the engine.

## Conclusion

The Excel Diff Engine is a robust, well-engineered system built on a foundation of high architectural integrity. The separation between Domain, Semantic, and Persistence layers is excellent. However, the system is currently "pulling its punches"—relying on strict safety caps and greedy heuristics rather than the scalable, holistic algorithms described in its specification. By abstracting the I/O layer and implementing the specified compression strategies (RLE), the engine can evolve from a safe utility into the high-performance semantic engine it aims to be.