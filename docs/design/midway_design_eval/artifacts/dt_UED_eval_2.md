Based on a rigorous analysis of the provided documents and codebase, here is the evaluation of your `unified_eval_draft.md`.

### Executive Summary

Your evaluation draft is **directionally accurate** and successfully identifies the project's status as a "restricted prototype." You correctly diagnosed the structural deviations (Waterfall vs. Spec AMR) and the architectural strengths (`Grid` vs. `GridView`).

**However, the evaluation is too generous.** It frames the algorithmic issues as "heuristics" or "tuning" problems, whereas the code actually contains **functional voids** where mandatory subsystems should be. Specifically, you missed that the alignment logic is mathematically incapable of handling disjoint edits (e.g., two separate insertions), and you overlooked a critical memory violation in the Input IR that poses a greater risk to WASM stability than the output serialization issues you flagged.

---

### 1. What the Draft Gets Definitely Correct

*   **The "Restricted Prototype" Verdict:** You correctly identified the `MAX_ALIGN_ROWS = 2_000` constant in `row_alignment.rs` as a hard operational ceiling that disqualifies the engine for the 50,000-row requirement.
*   **Architectural Layering:** You rightly praised the separation of persistent storage (`Grid`) from algorithmic views (`GridView`), which aligns perfectly with Specification Section 5.3.
*   **The "Waterfall" Deviation:** You accurately noted that `diff_grids` in `engine.rs` implements a greedy precedence chain (`detect_rect` → `detect_row_block` → `align`) instead of the integrated Anchor-Move-Refine (AMR) pipeline.
*   **WASM Incompatibility:** You correctly identified the `std::fs::File` dependency in `core/src/container.rs` as a hard blocker for the target architecture.

---

### 2. What the Evaluation Gets Wrong (or Misses)

The draft misses three specific technical failures that define *why* the engine is capped and memory-inefficient.

#### A. The Alignment Logic is "Single-Gap" Only (Functional Limitation)
You described the alignment as "heuristic," implying it is valid but suboptimal. **In reality, the current logic cannot perform a general diff.**
*   **The Evidence:** In `core/src/row_alignment.rs`, the function `align_rows_internal` branches strictly to `find_single_gap_alignment` or `find_block_gap_alignment`.
*   **The Failure:** `find_block_gap_alignment` explicitly requires that `prefix_match + suffix_match == shorter_length`. This means the engine can only detect **one contiguous block** of insertions/deletions.
*   **Impact:** If a user inserts Row 5 *and* Row 100 (two disjoint edits), the alignment functions return `None`. The engine falls back to `positional_diff`, which produces a noisy, unusable edit script. The engine lacks the core Sequence Alignment algorithms (Myers, LCS, or Patience) required for standard editing scenarios.

#### B. Missing String Interning (WASM Memory Violation)
You critiqued the `DiffReport` output for holding `String` identifiers, but missed a far more dangerous violation in the **Input IR**.
*   **Spec Requirement:** Section 5.7 mandates a `StringPool` and `StringId` to prevent memory explosion on repetitive text (e.g., a "Status" column with 50,000 "Pending" entries).
*   **The Code:** `core/src/workbook.rs` defines `CellValue::Text(String)`. Every cell owns its own heap-allocated string.
*   **Impact:** A 50,000-row sheet will trigger 50,000 heap allocations. This ensures the engine will crash via OOM (Out of Memory) in a WASM environment (~1.5GB limit) during parsing, long before diffing begins.

#### C. Missing Root Cause for the Row Cap (RLE)
You noted the 2,000-row cap but attributed it generally to the "heuristic" nature of the code.
*   **The Spec:** Section 3.5 mandates **Run-Length Encoding (RLE)** to handle "Adversarial Case UC-14" (e.g., 50k rows where 99% are blank).
*   **The Code:** `row_alignment.rs` checks `has_heavy_repetition` and simply **bails out** (`return None`) if true.
*   **Impact:** The cap exists specifically because the RLE subsystem is missing. Without it, comparing empty rows is an $O(N^2)$ operation, forcing the hard limit to prevent browser freezes.

---

### 3. Suboptimal Specifications (Revealed by the Code)

The codebase reveals inherent tensions in the specification that the draft failed to highlight:

1.  **Streaming vs. Deterministic Sorting:**
    *   **The Spec:** Demands "Instant diff... streaming" (Section 1) *and* "Deterministic output... sorted by (op, row, col)" (Phase 6).
    *   **The Conflict:** You cannot stream the output if you must globally sort it at the end. The implementation correctly sorts (accumulating `Vec<DiffOp>`), which causes the memory bottleneck you observed. The Spec should be relaxed to allow "Stream-safe" ordering (e.g., row-order) to enable true streaming.

2.  **Optimistic Dimension Ordering:**
    *   **The Spec:** Requires computing Jaccard similarity for *all* columns (Phase 2) before deciding on Row vs. Column alignment.
    *   **The Code:** Optimistically tries Row alignment first. This "lazy" approach is actually faster for 99% of spreadsheets and is a valid optimization over the Spec's heavy pre-pass.

---

### 4. Missed Insights

*   **Database Mode is Dead Code:** While the low-level logic exists in `database_alignment.rs`, the main entry point `diff_workbooks` **never calls it**. The automatic detection logic (Spec Phase 3) to switch modes is completely missing.
*   **Excel Semantics vs. M Semantics:** The codebase contains a mature parser for Power Query M (`m_ast.rs`) that handles semantic canonicalization. However, it completely lacks an **Excel Formula Parser**. Excel formulas are compared as raw strings (`Option<String>`), violating the requirement for semantic formula diffs.