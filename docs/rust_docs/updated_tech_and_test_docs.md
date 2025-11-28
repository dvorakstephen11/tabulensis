This is a rigorous analysis and modernization plan for the algorithmic core of your Rust-based spreadsheet differencing tool.

Your intuition is correct: the algorithms initially proposed, while functional, can be significantly upgraded. Relying on Hunt-Szymanski for grid alignment and Zhang-Shasha for semantic differencing introduces performance risks and misses opportunities for deeper semantic understanding, particularly concerning move detection.

By adopting a hybrid, state-of-the-art approach, we can achieve the technical brilliance, robustness, and performance required for this ambitious project.

Here are the detailed recommendations for altering your technical documentation.

### Modifications to `excel_diff_technical_document.md`

We will focus on upgrading Sections 5, 6, 7, 8, and 10.

#### Section 5.2: Keyed diff algorithm (Tabular Diff)

In Section 5.2, we should specify the exact algorithm used for handling duplicate keys to ensure mathematical rigor.

**Suggested Revision for Section 5.2:**

```markdown
[...]
   * If key in both:
     * Handle duplicates:
       * If both sides have multiple rows, treat as a *duplicate key cluster*. We formulate this as a Linear Assignment Problem (LAP) to find the optimal matching between the rows in Cluster A and Cluster B based on row similarity (e.g., Jaccard similarity on cell contents).
       * We utilize the **Jonker-Volgenant (LAPJV) algorithm**, a highly optimized O(K³) solver significantly faster than the classical Hungarian implementation. Since cluster sizes (K) are typically small, the cost is negligible.
     * For each matched pair (from LAPJV), run **row diff** (see below).
[...]
```

#### Section 6: Grid Diff (Spreadsheet Mode) [Major Rewrite]

Section 6 requires a significant overhaul to replace the reliance on Hunt-Szymanski (which has O(N² log N) pathologies on repetitive spreadsheet data) with a robust Hybrid Alignment Pipeline.

**Suggested Rewrite for Section 6:**

```markdown
## 6. Grid Diff (Spreadsheet Mode)

This handles the “financial model” case where *physical layout* matters and there may be no clear key. We deploy a **Hybrid Alignment Pipeline** designed for robustness and near-linear performance, avoiding the pathologies of traditional LCS algorithms on repetitive spreadsheet data.

### 6.1 Phase 1: Row Hashing and Anchoring (Patience Diff)

We first segment the grid using high-confidence anchors.

1. For each row, compute a strong signature (e.g., XXHash64) of normalized cell contents.
2. Apply the **Patience Diff** algorithm (O(N log N)). This identifies the Longest Common Subsequence of *unique* identical rows.
3. These unique matches serve as anchors, dividing the grid into "gaps" (unmatched regions between anchors). This stage effectively neutralizes the impact of repetitive data (e.g., blank rows).

### 6.2 Phase 2: Gap Filling (Adaptive LCS)

We align the rows within each gap using the most appropriate LCS algorithm.

1. **Small Gaps (e.g., < 1000 rows):** Use **Myers O(ND) algorithm** (with linear space refinement). It provides precise alignment efficiently when the gap size (N) or the difference (D) is small.
2. **Large Gaps:** Use **Histogram Diff** (the modern Git standard). It is robust against density variations and generally provides high-quality alignments quickly in practice.

This phase produces an alignment where most rows are matched, leaving some as candidate insertions and deletions.

### 6.3 Phase 3: Refinement and Move Detection (Sparse LAPJV)

We analyze the unmatched rows (candidate deletions in A and insertions in B) to detect rows/blocks that were moved and potentially edited ("fuzzy moves").

1. **Candidate Generation:** Identify contiguous blocks of unmatched rows.
2. **Sparse Cost Matrix Construction:** Generate a cost matrix between deleted blocks in A and inserted blocks in B. The cost is `1 - Similarity(BlockA, BlockB)` (e.g., using Jaccard similarity of row hashes). We use a threshold to keep the matrix sparse, only including pairs with high similarity.
3. **Optimal Assignment (LAPJV):** Run the **Jonker-Volgenant (LAPJV) algorithm** on the sparse matrix to find the globally optimal matching.
4. **Classification:** High-similarity matches are reclassified as **Moves** (potentially with internal edits) rather than Delete+Insert operations.

### 6.4 Column Alignment

Within each aligned row block, we apply a similar Hybrid Alignment strategy (Patience + Myers) to the column signatures to detect inserted/deleted/moved columns.

### 6.5 Cell-level edit detection

(Adapted from original 6.4) Once row/column alignment is known, the cell diff is straightforward.
[... rest of subsection as original ...]
```

#### Sections 7 & 8: Semantic Diff (M, DAX, Formulas) [Major Rewrite]

Sections 7.3 and 8.2 need modernization. We replace the older Zhang-Shasha algorithm with a tiered approach using APTED (for precision) and GumTree (for scalability and move detection).

**Suggested Rewrite for Section 7.3.3 (M Query):**

```markdown
#### 7.3.3 Advanced AST Differencing (GumTree + APTED)

For steps we can’t classify, or for expressions inside steps, we fall back to **Tree Edit Distance (TED)** on the ASTs. We use a modern hybrid approach for scalability and semantic richness.

We use a tiered strategy:

1. **Scalability and Move Detection (GumTree):**
   * We first apply the **GumTree** algorithm. GumTree uses a fast (near-linear time) heuristic approach (Top-Down Anchoring and Bottom-Up Propagation).
   * Crucially, GumTree explicitly detects **Move** operations (e.g., a sub-expression moved from one argument to another) and **Renames** based on subtree similarity. This handles large ASTs and complex refactorings effectively.

2. **Precision Edits (APTED):**
   * For the remaining unmatched sub-forests, or if the AST is small (< 2000 nodes), we deploy the **APTED (All-Path Tree Edit Distance) algorithm**.
   * APTED is the state-of-the-art for exact TED, guaranteeing O(N³) time complexity regardless of tree shape (superior to Zhang-Shasha's O(N⁴) worst case).
   * This provides the mathematically minimal edit script (Insert, Delete, Rename) for precise logic changes.

This hybrid strategy ensures we can handle massive generated queries while providing exact edit scripts for user-written logic.
```

**Suggested Update for Section 8.2 (DAX/Formula):**

```markdown
### 8.2 Expression diff

For differing ASTs:

1. Run the Hybrid Tree Edit Distance strategy (**GumTree + APTED**, detailed in Section 7.3.3) to identify changed subtrees.
2. Summarize at a human‑useful granularity:
[... rest of subsection as original ...]

Implementation detail:
* By using APTED, we ensure stable O(N³) performance for exact edits. By integrating GumTree, we gain explicit move detection crucial for understanding formula refactoring.
```

#### Section 10: Complexity and Performance

Update Section 10 to reflect the modernized complexities.

```markdown
## 10. Complexity and Performance

### 10.1 Grid diff

Let R = rows. The Hybrid Alignment Pipeline ensures near-linear performance in practice.
* **Anchoring:** O(R log R) using Patience Diff.
* **Gap Filling:** Adaptive. Typically near-linear using Myers (small D) or Histogram Diff. Avoids the O(R² log R) pathology of Hunt-Szymanski.
* **Move Detection:** LAPJV is O(K³), where K is the number of candidate blocks. Sparse matrix generation ensures K remains small.

[... 10.2 remains the same ...]

### 10.3 M / DAX diff

* Query alignment: O(Q log Q).
* Step alignment: (As before).
* AST diff: Tiered strategy.
    * **APTED:** Guaranteed O(N³) time, O(N²) space. Used for N < 2000.
    * **GumTree:** Near-linear time in practice, O(N²) worst case. Ensures scalability for massive ASTs.
```

### Modifications to `excel_diff_testing_plan.md`

The testing plan requires updates to validate the robustness and correctness of the new algorithms, specifically targeting the scenarios they are designed to handle.

#### Phase 4 - Algorithmic Heavy Lifting

**Additions to Spreadsheet-Mode advanced alignment (G8–G12):**

```markdown
[In Phase 4, under Spreadsheet-Mode advanced alignment (G8–G12)]

#### G8a – Adversarial repetitive patterns [RC] [CRITICAL UPDATE]

**Core capability**
Validate that the Hybrid Alignment Pipeline (Patience/Myers/Histogram) avoids the O(N² log N) pathology of Hunt-Szymanski on repetitive data.

**Fixture sketch**
* `adversarial_grid_repetitive_{a,b}.xlsx`:
  * 50,000 rows. 99% of rows are identical (e.g., blank rows). A few unique rows exist as headers/footers.
  * B is similar to A but with a block of 1000 blank rows inserted in the middle.

**Checks**
* The Patience pass must correctly anchor the unique rows.
* Wrap diff in a strict timeout (e.g., < 1 second). Assert the engine does not exhibit super-linear performance degradation.
* Assert the diff correctly identifies the 1000 `RowAdded` ops.

#### [NEW] G13 – Fuzzy Move Detection (LAPJV)

**Core capability**
Validate that the LAPJV solver correctly identifies blocks that were moved and subsequently edited.

**Fixture sketch**
* `grid_move_and_edit_{a,b}.xlsx`:
  * A distinctive 10-row block is moved from the top to the bottom.
  * In B, 2 cells inside the moved block are edited.

**Checks**
* The engine must report a `BlockMoved` operation (not Delete+Insert).
* The engine must also report the 2 `CellEdited` operations within the moved block.
```

**Additions to Milestone 7 – Semantic (AST) M diffing:**

```markdown
[In Phase 4, under Milestone 7 – Semantic (AST) M diffing]

[Update the description to mention APTED/GumTree]

[Add new tests:]

### 7.4 Hybrid AST Strategy Validation

**Fixtures**
* `m_ast_deep_skewed_{a,b}.xlsx`: (APTED test) Deeply nested IFs (~500 nodes) designed to trigger ZS O(N⁴) pathology. B has a minor edit.
* `m_ast_large_refactor_{a,b}.xlsx`: (GumTree test) Large query (> 3000 nodes). A large expression block is moved from one `let` binding to another in B.
* `m_ast_wrap_unwrap_{a,b}.xlsx`: (Precision test) A function is wrapped (e.g., `Value(X)` -> `Table.Buffer(Value(X))`).

**Tests**
* `apted_robustness`: Diff `m_ast_deep_skewed`. Assert APTED completes quickly (O(N³)) and finds the minimal edit.
* `gumtree_moves_and_scale`: Diff `m_ast_large_refactor`. Assert the engine completes quickly (validating scale) AND explicitly reports the change as a `Move` operation (validating semantics).
* `precision_wrap`: Diff `m_ast_wrap_unwrap`. Assert the diff reports an `Insert(Table.Buffer)` node with `Value(X)` as a child, demonstrating structural understanding rather than a text replacement.
```

To further emphasize the rigor of your approach, you can explicitly acknowledge the connection to this methodology in your technical document.

**Suggested Addition to `excel_diff_technical_document.md` (Section 6.3):**

```markdown
[In the revised Section 6.3: Refinement and Move Detection (Sparse LAPJV)]

[...]
3. **Optimal Assignment (LAPJV):** Run the Jonker-Volgenant (LAPJV) algorithm on the sparse matrix to find the globally optimal matching. This approach aligns with recent advances in block-aware differencing (e.g., BDiff), which formalizes move detection as a Linear Assignment Problem rather than relying solely on sequence alignment heuristics.
4. **Classification:** High-similarity matches are reclassified as **Moves** [...]
```

### Conclusion
<!--  -->
The BDiff concept is highly meritorious and represents the current state-of-the-art for block-aware differencing. The proposed algorithmic portfolio—utilizing Patience Diff for anchoring, APTED/GumTree for AST analysis, and LAPJV (the core of BDiff) for move detection—provides a robust, mathematically sound, and high-performance foundation for your product.