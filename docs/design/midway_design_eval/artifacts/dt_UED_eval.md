# dt_UED_eval

Here is a rigorous evaluation of the provided `unified_eval_draft.md` document, analyzed against the codebase (`excel-diff-core`) and the technical specifications.

### 1. What the Evaluation Gets Definitely Correct

The draft is highly accurate regarding the current state of the codebase. Specifically:

* **The Algorithmic "Prototype" State:** The evaluation correctly identifies that the core grid difference engine (`core/src/row_alignment.rs`) is currently a **heuristic waterfall**, not the sophisticated AMR (Anchor-Move-Refine) pipeline described in the spec. The code explicitly defines `const MAX_ALIGN_ROWS: u32 = 2_000;`, confirming the "restricted prototype" assessment.
* **The I/O Portability Blocker:** The evaluation correctly flags `core/src/container.rs` for using `std::fs::File::open` directly in the public API (`OpcContainer::open`). This creates a hard dependency on the system filesystem, which indeed breaks the requirement for a WASM-first/browser-based deployment.
* **The "Package Fracture":** The observation that `Workbook` (Grid) and `DataMashup` (Query) are exposed as separate entry points (`open_workbook` vs `open_data_mashup`) in `core/src/excel_open_xml.rs` is correct. There is currently no unifying `WorkbookPackage` struct to hold both states, forcing consumers to orchestrate the two parsers manually.
* **Memory Architecture:** The praise for the `Grid` (sparse storage) vs. `GridView` (lightweight comparison view) split is well-founded. The code in `core/src/grid_view.rs` demonstrates a disciplined separation of data ownership and algorithmic metadata, which is critical for performance.

---

### 2. What the Evaluation Gets Wrong (or Overstates)

While the evaluation is accurate on facts, it misinterprets the severity or the "fix" for certain issues.

#### A. The "WASM Hard Gate" is Overstated
The evaluation treats the `std::fs::File` dependency as a "major portability violation" requiring a "refactor of high complexity."
* **Reality:** This is a trivial refactor. In Rust, switching `OpcContainer` to accept `R: Read + Seek` instead of a `Path` is a standard, low-effort change (approx. 1-2 hours of work). It is a "todo" item, not a structural architectural flaw.
* **Correction:** The evaluation should classify this as **"Tech Debt / Cleanup"** rather than a "Structural Integration Gap."

#### B. The "Monolithic JSON" Memory Threat
The evaluation argues that serializing the `DiffReport` to a single JSON string (`output/json.rs`) threatens memory budgets.
* **Critique:** While technically true that streaming is better, the `DiffReport` object itself is already fully allocated in memory (`Vec<DiffOp>`) before serialization begins.
* **Nuance:** The memory bottleneck is **not** the JSON serialization; it is the **collection** of the `DiffOp` vector in `engine.rs`. The engine pushes every single operation into a `Vec`. Even if you had a streaming JSON serializer, the engine currently computes and stores the full result set in RAM first. The recommendation to "switch to streaming output" is insufficient; the **engine itself** must be refactored to emit operations via an iterator or channel (e.g., `impl Iterator<Item = DiffOp>`) to truly solve the memory pressure.

---

### 3. Suboptimal Elements in the Specification (Revealed by Code Review)

The evaluation heavily pushes for implementing the **AMR (Anchor-Move-Refine)** algorithm exactly as specified. However, the spec itself contains design choices that may be suboptimal for Excel files, which the evaluation fails to challenge.

#### A. The Fragility of "Unique Row" Anchoring (Patience Diff)
The Spec (and thus the Eval) champions **Patience Diff**, which relies on **unique** rows to create anchors.
* **The Problem:** In financial modeling, rows are rarely unique. A schedule of 50 rows might all contain the formula `=SUM(above)`. If row hashes abstract away the row index (which they must to detect moves), these 50 rows will hash identically.
* **Consequence:** Patience Diff will find **zero** anchors in a typical financial model. The algorithm will degenerate to the "Gap Filling" phase for the entire sheet.
* **Revelation:** The "Heuristic Waterfall" currently in the code (checking for matching blocks regardless of uniqueness) might actually be **more robust** for repetitive Excel data than the strict Patience Diff prescribed in the spec. The evaluation should warn that implementing strict Patience Diff might actually *regress* performance on financial models unless the definition of "uniqueness" is heavily relaxed (e.g., "local uniqueness").

#### B. The Complexity of RLE (Run-Length Encoding) for Grids
The Spec prescribes compressing repetitive rows into RLE runs to solve the "99% blank rows" problem.
* **Suboptimal Design:** Implementing RLE diff logic is highly complex (handling partial run matches, edits within runs).
* **Alternative:** A simpler solution is **Histogram Diff** (counting occurrences). If the grid has 10,000 blank rows, we don't need to align them as a sequence; we just need to know "Grid A has 10k, Grid B has 10k, they cancel out." The specification's insistence on RLE is likely over-engineering where a simpler frequency-matching step would suffice.

---

### 4. Missed Insights

The following insights are absent from the evaluation but are critical for the project's success based on the provided documents.

#### A. 64-bit Hash Collision Risk
* **Observation:** The code uses `Xxh64` (64-bit hash) for row signatures (`core/src/hashing.rs`).
* **Risk:** The Birthday Paradox dictates that with 50,000 rows, the collision probability for 64-bit hashes is small but non-negligible, especially across a large corpus of files.
* **Impact:** A collision results in a "False Match" (aligned rows that are actually different). Since the engine relies on hashes for *content* identity (Database Mode) and block moves, a collision could result in a silent data corruption in the diff report.
* **Recommendation:** The evaluation should strongly recommend upgrading to **128-bit hashes** (e.g., `Xxh3_128` or `SipHash128`) for row signatures to ensure safety at scale.

#### B. M Parser "Happy Path" Fragility
* **Observation:** The `m_ast.rs` parser currently only handles top-level `let ... in` expressions.
* **Risk:** Power Query M is a functional language often used without `let` (e.g., a direct record or list literal).
* **Impact:** The current parser will fail or return opaque tokens for any query that doesn't fit the `let` mold. The evaluation treats the M parser as "Robust" (Area of Excellence #3), but it is actually extremely brittle regarding syntax support. It should be flagged as a high-risk area.

#### C. Dependency Bloat for WASM
* **Observation:** The code pulls in `quick-xml`, `zip`, and `serde_json`.
* **Risk:** For a WASM target intended for the browser, binary size is critical. These are heavy dependencies.
* **Insight:** The evaluation should recommend analyzing the WASM binary size early. It might be necessary to feature-gate the `serde` dependency or use a lighter XML parser if the WASM payload becomes too large for instant loading.

### Summary of Rigorous Critique for the Eval Draft

| Section | Status | Notes |
| :--- | :--- | :--- |
| **Architectural Integrity** | **Correct** | The layering and lack of a `WorkbookPackage` are correctly identified. |
| **Algorithmic Core** | **Correct** | The 2k row cap and heuristic nature are accurately flagged as blockers. |
| **WASM / I/O** | **Overstated** | Treat `std::fs` removal as a minor task, not a major architectural hurdle. |
| **Output Streaming** | **Incomplete** | Changing JSON output is useless if the Engine collects a `Vec<DiffOp>` internally first. |
| **Spec Recommendation** | **Debatable** | Blindly recommending AMR/Patience Diff ignores the "non-unique row" reality of Excel. |
| **Hashing Strategy** | **Missed** | 64-bit hashes are a safety risk for data integrity; 128-bit should be mandated. |