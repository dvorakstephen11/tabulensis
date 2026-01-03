# Integrated Design Evaluation Report: Excel Diff Engine

**Date:** December 7, 2025
**Evaluator:** Principal Systems Architect
**Subject:** Deep Architectural Audit & Synthesis
**Status:** **Restricted Prototype / Architecturally Sound**
**Target:** Engineering Leadership & Core Architecture Team

-----

## 1\. Executive Summary

This report presents a rigorous architectural audit of the Excel Diff Engine, synthesizing findings from systems-level mechanical analysis (memory models, algorithmic complexity, WASM constraints) and product-level structural analysis (object graph coherence, API usability).

The codebase is best characterized as a **"Robust Walking Skeleton."** It demonstrates exceptional "Vertical Integrity"—the stack from binary parsing (`MS-QDEFF`) to JSON output is fully functional, type-safe, and rigorously tested. The core memory architecture (`Grid` vs. `GridView`) is a sophisticated solution to the "100MB File" problem, successfully balancing storage sparsity with algorithmic density via zero-copy views.

However, the system currently functions as a **"Restricted Prototype"** rather than a production-ready engine due to two critical deficits:

1.  **Algorithmic Maturity (The Scalability Gap)**: The implementation diverges from the specification, utilizing **"Greedy Heuristics"** instead of the specified **"Anchor-Move-Refine" (AMR)** algorithm. Crucially, the absence of **Run-Length Encoding (RLE)** for repetitive data necessitates hard safety caps (`MAX_ALIGN_ROWS = 2,000`), rendering the tool functionally incapable of handling the target 50,000-row datasets without risking $O(N^2)$ performance degradation.
2.  **Domain Fragmentation (The Integration Gap)**: While the internal layers are clean, the top-level API is fractured. The `Workbook` (Grid) and `DataMashup` (Query) domain models remain disjoint, forcing the consumer to manually orchestrate separate parsing and diffing streams.

The foundation is solid, but the engine is currently "pulling its punches"—trading algorithmic correctness and scalability for implementation simplicity.

-----

## 2\. Deep Dimension Evaluations

### Dimension 1: Architectural Integrity

**Synthesis Status**: **Structurally Sound / Horizontally Fractured**

  * **The Vertical Triumph**: The layering strategy is disciplined and effective. The dependency flow is strict: `engine` $\to$ `workbook` $\to$ `container`. The `datamashup` module effectively isolates the complexity of the `MS-QDEFF` binary format (Base64 inside XML inside ZIP) from the rest of the system. This "containment strategy" prevents binary volatility from leaking into the domain logic.
  * **The Horizontal Fracture**: A critical integration gap exists at the top level. `workbook.rs` defines the visual grid, while `datamashup.rs` defines the query logic. There is no `WorkbookPackage` struct that unites them. Consequently, the user must call `open_workbook` and `open_data_mashup` separately. This violates the "System Surface Area" principle; the domain model does not reflect the user's mental model of a single ".xlsx file."
  * **Algorithmic Divergence**: The implementation in `engine.rs` follows a **Waterfall Strategy** (`detect_rect` $\to$ `detect_row` $\to$ `align`). The specification demands a **Global Anchor Strategy** (Find Anchors $\to$ Identify Moves in Gaps). The current architecture is essentially a heuristic shortcut that prioritizes local matches over global structure, risking failure on complex, interleaved edits.

### Dimension 2: Elegant Simplicity

**Synthesis Status**: **High Abstraction Fidelity / Localized Complexity**

  * **The `CellSnapshot` Win**: The decision to decouple cell identity (`address`) from cell content (`value` + `formula`) via `CellSnapshot` is elegant. It allows the diff engine to compare cells purely by content equality (`PartialEq`) without worrying about row shifting, effectively "normalizing" the data for the diff algorithm.
  * **The `DiffOp` Fragmentation**: Simplicity is broken at the output layer. Because the domain model is fractured, `DiffOp` only covers Grid operations. M-Language changes are emitted as a separate `MQueryDiff` stream. A unified consumer must multiplex two distinct change logs.
  * **Implementation Complexity**: The `row_alignment.rs` module contains dense imperative logic—manual index management (`idx_a`, `idx_b`) and dense conditional logic within alignment loops. This brittleness is a symptom of the "Greedy" algorithmic choice; a proper Patience Diff implementation would likely be more declarative.

### Dimension 3: Rust Idiomaticity

**Synthesis Status**: **Elite Memory Management / Mixed Module IO**

  * **Lifetimes & Zero-Copy**: The `GridView<'a>` struct is the codebase's crown jewel. It allows sorting and hashing of row data without cloning the underlying `Cell` objects stored in the `Grid`. This demonstrates elite command of Rust's lifetime system (`'a`), enabling high-performance views over heavy data.
  * **Error Hierarchy**: The use of `thiserror` to define domain-specific error trees (`ContainerError`, `GridParseError`) is idiomatic and allows for precise error handling upstream.
  * **The `std::fs` Blocker**: A critical idiomatic failure exists regarding the target platform. The `OpcContainer` relies on `std::fs::File`, which binds the core logic to the OS filesystem. Idiomatic Rust for portable libraries should accept a generic `R: Read + Seek` trait, allowing the same code to process a `File` (Desktop) or a `Cursor<Vec<u8>>` (WASM/Browser).

### Dimension 4: Maintainability Posture

**Synthesis Status**: **Strong Test Culture / Weak Configurability**

  * **Fixtures as Specification**: The `fixtures/` directory usage is excellent. By generating test files via Python scripts (`generators/grid.py`), the team avoids the fragility of checking in binary Excel files. The tests document exactly *which* edge cases (e.g., "Fuzzy Row Moves") are supported.
  * **Hardcoded Constraints**: A major maintainability risk is the presence of "Magic Constants" deeply embedded in the logic. `MAX_ALIGN_ROWS = 2000`, `MAX_BLOCK_GAP = 32`, and `FUZZY_SIMILARITY_THRESHOLD = 0.80` are defined as `const` in source files. This prevents runtime tuning or configuration injection.

### Dimension 5: Performance Awareness

**Synthesis Status**: **Safe but Unscalable**

  * **The Safety Cap Conflict**: The `MAX_ALIGN_ROWS = 2000` cap is an admission of missing functionality. The specification calls for **Run-Length Encoding (RLE)** to compress repetitive data (e.g., thousands of empty rows). The implementation lacks RLE. Without RLE, a standard diff algorithm is $O(N^2)$ or $O(ND)$ on empty space. The cap is a mechanical fuse installed because the compression engine is missing. It renders the tool "Safe" (it won't crash) but "Unscalable" (it aborts on real-world data).
  * **Allocation Churn**: While `GridView` is zero-copy for cells, it allocates a `Vec` for every row. On a large sheet, this results in significant allocator pressure. An "Arena" or "slab" allocator approach for the view layer would improve cache locality.

-----

## 3\. Deep Dive: The Algorithmic Gap

The most significant finding of this evaluation is the distance between the **Specified Algorithm** and the **Implemented Heuristic**.

| Feature | Specification (AMR) | Implementation (Greedy) | Consequence |
| :--- | :--- | :--- | :--- |
| **Control Flow** | Global Anchor Discovery $\to$ Gap Filling | Rect Move $\to$ Row Move $\to$ Align | Engine misses interleaved edits; cannot handle moves *inside* aligned blocks. |
| **Move Detection** | Identification of "out-of-order" anchors in the global chain. | Iterative search for matching hash blocks. | High complexity; implementation is brittle and limited to contiguous blocks. |
| **Repetitive Data** | RLE Compression (10,000 empty rows $\to$ 1 run). | **Missing** | Engine requires `MAX_ALIGN_ROWS` cap to prevent OOM/Timeouts on empty sheets. |
| **Complexity** | $O(N \log N)$ (Expected) | $O(N)$ (Capped) | Performance is artificial; the engine simply refuses to work on large inputs. |

**Assessment**: The current engine is a prototype. It validates the data structures but does not solve the core computational problem of diffing large spreadsheets.

-----

## 4\. Deep Dive: The Domain Model Fracture

The system forces the consumer to act as the integrator, managing disparate streams of data that should be unified.

**Current Workflow (Fractured):**

```rust
// User opens the file TWICE (Inefficient & Brittle)
let wb = open_workbook("file.xlsx")?;
let mashup = open_data_mashup("file.xlsx")?; 

// User runs TWO diffs
let report = diff_workbooks(&wb_a, &wb_b);
let m_diff = diff_m_queries(&mashup_a, &mashup_b);

// User must manually merge `report` and `m_diff` into a unified UI model
```

**Target Workflow (Unified):**

```rust
// User opens the package once
let pkg = WorkbookPackage::open("file.xlsx")?; 

// User runs ONE diff
let report = pkg.diff(&other_pkg); 

// DiffReport contains both Grid ops and M ops
match report.ops[0] {
    DiffOp::RowAdded { .. } => { ... }
    DiffOp::QueryChanged { .. } => { ... }
}
```

This fracture increases the "System Surface Area" unnecessarily and precludes optimizations where grid data and query data might interact (e.g., verifying if a query load destination matches a sheet table).

-----

## 5\. Priority Recommendations & Roadmap

### Phase 1: The Scalability Unlock (Weeks 1-2)

**Goal:** Remove the 2,000 row limit.

1.  **Implement RLE**: Modify `GridView` construction to detect and compress consecutive rows with identical hashes (specifically targeting empty/template rows) into `RowRun` tokens.
2.  **Lift the Cap**: Once RLE is in place, raise `MAX_ALIGN_ROWS` to 50,000 and verify performance against the `perf_large` fixture.

### Phase 2: Platform Abstraction (Week 3)

**Goal:** Enable WASM compilation.

1.  **Generic Container**: Refactor `OpcContainer::new()` to accept `R: Read + Seek` instead of `Path`.
2.  **Feature Flags**: Gate `std::fs` convenience methods behind a `default = ["std"]` feature flag. Ensure `core` is `no_std` compatible (or at least `wasm32` compatible without OS deps).

### Phase 3: Algorithmic Refactor (Weeks 4-6)

**Goal:** Align implementation with Specification.

1.  **Invert Control Flow**: Deprecate the `detect_exact_*_move` waterfall. Implement the `Patience Diff` (LIS) algorithm to find global anchors first.
2.  **Move-Aware Gap Filling**: Re-implement move detection as a classification of non-anchor rows within gaps, rather than a pre-pass.

### Phase 4: Domain Unification (Week 7)

**Goal:** Fix the fractured API.

1.  **Unified Struct**: Introduce `WorkbookPackage` that owns both `Workbook` and `DataMashup`.
2.  **Unified DiffOp**: Expand `DiffOp` to include `MQueryChanged` and `MQueryAdded` variants.
3.  **Unified Entry Point**: Expose `diff_packages` that orchestrates the sub-engines.

## 6\. Conclusion

The Excel Diff Engine is a success of **Data Engineering** but a work-in-progress of **Algorithm Engineering**. The team has successfully tamed the hostile environment of Excel file formats, building a safe, typed, and clean memory model. However, the logic that runs *atop* that model is currently provisional. To graduate from "Prototype" to "Product," the team must prioritize the implementation of the specified compression (RLE) and anchoring algorithms to unlock the performance and scalability promised in the specification.