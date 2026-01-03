# Grid Diff Algorithm Design: Expert Analysis Request

## Context

You are being asked to analyze and propose an optimal algorithm design for a **2D grid diff engine** that will be the core of a commercial Excel comparison tool. This is a difficult problem (rated 18/20 in our difficulty analysis) and we are seeking independent expert opinions to validate or challenge our approach.

Your analysis should be from the perspective of a world-class mathematician and software developer. We want your honest assessment of the best algorithmic approaches, not confirmation of any existing design.

---

## Problem Statement

Given two grids $G_A$ and $G_B$ (representing Excel worksheets), produce a semantically meaningful diff that:

1. **Aligns** rows and columns to handle insertions, deletions, and potential reorderings
2. **Detects** cell-level edits between aligned positions
3. **Identifies** block moves (contiguous regions that relocated intact or with minor edits)
4. **Operates** in near-linear time on grids with 50,000+ rows and 100+ columns
5. **Resists** pathological inputs (repetitive data, adversarial patterns)

The algorithm must support two semantic modes:
- **Spreadsheet Mode**: Physical position matters; alignment is by structural similarity
- **Database Mode**: Rows matched by user-specified or inferred key column(s); physical order is irrelevant

---

## Constraints and Requirements

### Performance Targets

| Metric | Requirement |
|--------|-------------|
| Large grid diff (50K rows × 50 cols, few edits) | < 500ms |
| Large grid diff (50K rows × 50 cols, heavy edits) | < 2 seconds |
| Adversarial case (50K rows, 99% identical/blank) | < 1 second |
| Memory | Must work within WASM constraints; streaming preferred |
| Target workbook size | Up to ~100MB Excel files |

### Pathological Case: Repetitive Data

A critical challenge is handling grids where most rows are identical (e.g., blank rows, repeated header rows, template rows). Naive LCS algorithms can degenerate catastrophically:

- **Hunt-Szymanski**: O(N log N + K log K) where K = number of matches. When 99% of rows are identical, K approaches N², giving O(N² log N).
- **Simple Myers**: O(ND) where D = edit distance. When most rows differ, D ≈ N, giving O(N²).

Any proposed solution must explicitly address how it handles this adversarial case.

### Output Requirements

The diff engine must emit structured operations:
- `RowAdded`, `RowRemoved` for structure changes
- `ColumnAdded`, `ColumnRemoved` for column structure changes
- `CellEdited` for individual cell changes within aligned positions
- `BlockMovedRows`, `BlockMovedColumns` for detected moves
- Optionally: fuzzy move detection (block moved AND some cells edited within it)

---

## Use Cases to Cover

Please ensure your algorithm design addresses all of these scenarios:

### Basic Cases
1. **Identical grids** → empty diff
2. **Single cell change** → one CellEdited
3. **Multiple scattered cell edits** in fixed-size grid → multiple CellEdited, no structure ops
4. **Row append at end** → RowAdded ops, no phantom edits on existing rows
5. **Column append at edge** → ColumnAdded ops

### Alignment Cases
6. **Single row insert in middle** → one RowAdded, rows below correctly aligned (no false CellEdited)
7. **Single row delete in middle** → one RowRemoved, rows below correctly aligned
8. **Single column insert in middle** → one ColumnAdded, columns to right correctly aligned
9. **Block of rows inserted** → contiguous RowAdded ops or BlockAdded
10. **Block of rows deleted** → contiguous RowRemoved ops or BlockRemoved

### Move Detection
11. **Block of rows moved** (cut from position A, paste at position B, content identical) → BlockMovedRows, NOT delete+insert
12. **Block moved with internal edits** → BlockMovedRows + CellEdited within the block
13. **Column moved** → BlockMovedColumns

### Adversarial/Edge Cases
14. **99% blank rows, 1% unique rows, block inserted in middle** → must complete in < 1 second on 50K rows
15. **Completely different grids** (no common content) → graceful degradation, not hang
16. **Very sparse grids** (few cells populated in large logical range) → memory-efficient handling

### Database Mode
17. **Keyed rows, same keys different order** → empty diff (order doesn't matter)
18. **Keyed rows with edits** → CellEdited on changed non-key columns
19. **Duplicate keys** → handle gracefully (cluster matching)
20. **Mixed sheet** (table region with keys + free-form cells around it) → apply appropriate mode to each region

---

## Existing Data Structure

The current implementation uses a sparse grid representation:

```rust
pub struct Grid {
    pub cells: HashMap<(u32, u32), Cell>,
    pub nrows: u32,  // Used range extent
    pub ncols: u32,
}
```

This is memory-efficient for sparse data but may complicate row-oriented algorithms. Your analysis should consider:
- Is this structure optimal for the required algorithms?
- Should we add secondary indices (e.g., row-grouped views)?
- What trade-offs exist between memory efficiency and algorithmic performance?

---

## Questions for Your Analysis

1. **Algorithm Selection**: What algorithm or combination of algorithms would you recommend for the row alignment phase? Why?

2. **Adversarial Robustness**: How specifically does your approach handle the "99% identical rows" case without quadratic blowup?

3. **Move Detection**: What is the right trade-off between optimal move detection (computationally expensive) and heuristic approaches (may miss or mis-classify moves)?

4. **Database vs Spreadsheet Mode**: Should these share infrastructure, or are they fundamentally different problems requiring different algorithms?

5. **Column Alignment**: Should column alignment be a separate phase after row alignment, or should row and column alignment be interleaved/joint?

6. **Data Structure**: Given the sparse HashMap representation, what modifications (if any) would you recommend to support efficient algorithm execution?

7. **Parallelism**: What opportunities exist for parallelizing the diff computation?

8. **Complexity Bounds**: What are the theoretical complexity bounds of your proposed approach? What are the practical expected-case bounds?

---

## Deliverable

Please provide:

1. **High-level architecture** of your proposed algorithm pipeline
2. **Detailed algorithm descriptions** for each phase
3. **Complexity analysis** (worst-case and expected-case)
4. **Pseudocode or detailed descriptions** for critical components
5. **Trade-off analysis** for any design decisions with alternatives
6. **Test case coverage** - how does your design handle each use case listed above?

---

## Notes

- This is a real production system, not an academic exercise. Practical performance matters as much as theoretical elegance.
- The solution must be implementable in Rust and compilable to WASM.
- Memory efficiency is critical due to WASM constraints and 100MB workbook targets.
- We value novel approaches - do not feel constrained by what existing diff tools do.
