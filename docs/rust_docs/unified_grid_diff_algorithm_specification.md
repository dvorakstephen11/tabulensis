# Unified Grid Diff Algorithm Specification

**Document Type:** Algorithmic Specification (Non-Implementation)  
**Date:** 2025-11-30  
**Status:** Authoritative Design Reference  
**Scope:** Core 2D Grid Diff Engine for Excel and Power BI Comparison

---

## Table of Contents

- [Part I: Foundations](#part-i-foundations)
  - [1. Executive Summary & Strategic Context](#1-executive-summary--strategic-context)
  - [2. Use Cases & Requirements Mapping](#2-use-cases--requirements-mapping)
  - [3. Algorithmic Foundations Reference](#3-algorithmic-foundations-reference)
- [Part II: Architecture & Data Structures](#part-ii-architecture--data-structures)
- [Part III: Preprocessing](#part-iii-preprocessing)
- [Part IV: Spreadsheet Mode Alignment](#part-iv-spreadsheet-mode-alignment)
- [Part V: Database Mode Alignment](#part-v-database-mode-alignment)
- [Part VI: Cell-Level Comparison](#part-vi-cell-level-comparison)
- [Part VII: Result Assembly & Output](#part-vii-result-assembly--output)
- [Part VIII: Robustness & Performance](#part-viii-robustness--performance)
- [Part IX: Analysis & Validation](#part-ix-analysis--validation)
- [Part X: Appendices](#part-x-appendices)

---

# Part I: Foundations

---

## 1. Executive Summary & Strategic Context

### 1.1 Document Purpose

This specification defines the algorithmic design for the 2D grid diff engine that powers Excel and Power BI worksheet comparison. It is intended as the authoritative reference for implementation, describing in precise detail **what** the algorithms must accomplish and **why** each design decision was made.

This document does not contain implementation code. It specifies behavior, data flows, complexity guarantees, and decision rationale. Implementers should treat this specification as the source of truth when building the diff engine in Rust, with the expectation that the resulting code will compile to WebAssembly for browser deployment alongside native builds.

### 1.2 Product Vision

The grid diff engine is the computational core of a commercial Excel and Power BI comparison tool designed to serve:

- **Financial analysts** comparing budget versions, forecasts, and actuals
- **Auditors** tracking changes in regulatory submissions and compliance documents
- **Data engineers** validating ETL transformations and data migrations
- **Business users** reviewing collaborative edits to shared workbooks

The tool must deliver results that are:

1. **Semantically meaningful**: Users understand not just *that* something changed, but *what kind* of change occurred (cell edit, row insertion, block move, etc.)
2. **Blazingly fast**: Sub-second response for typical workloads; under 2 seconds for heavily edited 50K-row sheets
3. **Universally deployable**: Runs in the browser via WebAssembly, in desktop applications, and on servers
4. **Robust**: Never crashes, hangs, or produces nonsensical results—even on adversarial inputs

### 1.3 Competitive Positioning

The market for spreadsheet comparison tools includes established players (Beyond Compare, Synkronizer, XLCompare) and lightweight alternatives (manual VLOOKUP, conditional formatting). This engine differentiates through:

| Capability | Incumbent Tools | This Engine |
|------------|-----------------|-------------|
| **Move Detection** | Limited or none | Full block-level moves (rows, columns, rectangles) with fuzzy matching |
| **Database Mode** | Rare | Native key-based comparison with duplicate key handling |
| **Formula Awareness** | String-level only | AST-based semantic comparison with canonicalization |
| **Modern Excel Support** | Partial | Full Power Query, dynamic arrays, structured references |
| **Performance on Large Files** | Often quadratic | Guaranteed near-linear with explicit adversarial defenses |
| **Deployment Model** | Desktop installers | WASM-first with zero installation |

The grid diff algorithm is rated the single most difficult component of the system (18/20 difficulty score) due to the combination of algorithmic complexity, performance requirements, and edge case handling.

### 1.4 Performance Requirements

The engine must meet these performance targets on commodity hardware (2020-era laptop, single-threaded WASM):

| Scenario | Grid Size | Edit Complexity | Target Time |
|----------|-----------|-----------------|-------------|
| Identical grids | 50K × 100 | None | < 200ms |
| Few scattered edits | 50K × 100 | 10-50 cell changes | < 500ms |
| Block inserted/deleted | 50K × 100 | 1000-row block | < 500ms |
| Heavy edits | 50K × 100 | 30% rows changed | < 2s |
| Adversarial (99% blank) | 50K × 100 | Block insert in middle | < 1s |
| Completely different | 50K × 100 | No shared content | < 500ms |

These targets imply that the algorithm must achieve **O(N log N)** or better complexity in practice, with no hidden O(N²) behavior on pathological inputs.

### 1.5 WASM Deployment Constraints

WebAssembly deployment imposes constraints that influence algorithm design:

**Memory Limits**: Browser WASM environments typically cap memory at 2-4GB, with practical limits around 1.5-2GB for stable operation. A 100MB Excel file may expand to 400-1000MB in parsed form. The algorithm must:
- Operate within a defined memory budget (default 1.5GB total, 600MB per grid)
- Avoid allocating data structures proportional to `nrows × ncols` (which could be 5 billion cells for a full Excel sheet)
- Support streaming/chunked processing for files exceeding memory thresholds

**Single-Threaded by Default**: While WASM threads (SharedArrayBuffer) exist, they require specific browser configurations and are not universally available. The algorithm must:
- Achieve target performance on a single thread
- Be structured to exploit parallelism when available, without depending on it

**No System Calls**: WASM cannot access the filesystem, spawn processes, or perform other system operations. All data must be passed in; all results passed out.

**Deterministic Execution**: The same inputs must always produce the same outputs, regardless of HashMap iteration order or parallel execution paths. This is essential for testing, caching, and user trust.

### 1.6 Dual-Mode Architecture

The engine supports two semantic modes that share infrastructure but use different alignment strategies:

**Spreadsheet Mode** (Positional Identity)
- Rows and columns have identity based on their position in the grid
- Insertions and deletions cause subsequent rows/columns to shift
- Move detection identifies content that changed position without changing substance
- Use case: Comparing versions of a financial model, audit workpaper, or report template

**Database Mode** (Key-Based Identity)
- Rows have identity based on the values in designated key column(s)
- Row order is semantically irrelevant; reordering is not reported as a change
- Key collisions (duplicate keys) are handled via similarity-based matching
- Use case: Comparing data exports, CRM snapshots, or ETL outputs

A single worksheet may contain regions of both types (e.g., a data table with keys surrounded by free-form notes). The engine must support mixed-mode processing through region segmentation.

### 1.7 Key Algorithmic Innovations

This specification incorporates several algorithmic advances beyond standard diff approaches:

**Anchor-Move-Refine (AMR) Algorithm**: Unlike traditional diff algorithms that detect moves as a post-processing step (identify deletions and insertions, then match them), AMR integrates move detection into the alignment process itself. Move candidates are identified *before* gap filling, allowing the gap filler to skip rows that moved elsewhere rather than treating them as deletions. This produces cleaner edit scripts with the same O(N log N) complexity.

**Adaptive Dimension Ordering**: The engine dynamically chooses whether to align rows or columns first based on a column stability check. When columns are stable (the common case), row-first alignment is simpler. When significant column changes are detected, column-first alignment with row hash recomputation produces more accurate results.

**Run-Length Compression for Repetitive Data**: Grids with highly repetitive content (blank rows, template rows, repeated values) are compressed into runs before alignment. This defeats the quadratic behavior that afflicts naive LCS algorithms on repetitive inputs, reducing a 50,000-row grid with 99% blank rows to perhaps a dozen runs.

**Cost-Capped Bail-Out**: When a segment of the grid is so different that alignment would be expensive and semantically useless, the algorithm detects this condition and treats the entire segment as a block replacement. This bounds worst-case behavior and produces more honest user-facing results.

### 1.8 Quality Attributes

Beyond performance, the diff engine must exhibit:

**Correctness**: The reported differences must accurately reflect the actual differences between grids. A cell reported as unchanged must have identical content in both grids. A row reported as moved must have the same content at its source and destination positions.

**Minimality**: The edit script should be close to minimal—not necessarily optimal (which may be NP-hard to compute), but avoiding obviously redundant operations like reporting a move as delete+insert when the match is unambiguous.

**Stability**: Small changes to input should produce small changes to output. Adding one row should not cause the entire diff to be recomputed from scratch in a way that changes unrelated operations.

**Interpretability**: The diff should make sense to a human user. Operations should correspond to plausible editing actions. The distinction between "row moved" and "row deleted, similar row inserted elsewhere" should match user intuition.

### 1.9 Design Principles

The following principles guide design decisions throughout this specification:

1. **Sparse Over Dense**: Never allocate structures proportional to the full grid dimensions. All data structures should be proportional to non-empty cells, non-empty rows, or the number of detected changes.

2. **1D Over 2D**: Decompose the 2D alignment problem into separate 1D problems (rows, then columns). Joint 2D dynamic programming is combinatorially explosive and unnecessary for spreadsheet semantics.

3. **Anchors First**: Establish high-confidence matches (anchors) before attempting to align ambiguous regions. Anchors partition the problem into independent subproblems.

4. **Fail Fast, Fail Safely**: Detect pathological conditions early (no shared content, excessive repetition, memory pressure) and degrade gracefully to simpler behavior rather than attempting expensive computations that may not terminate.

5. **Determinism is Non-Negotiable**: Every code path must produce identical output for identical input. This requires explicit handling of HashMap iteration order, parallel result collection, and floating-point comparison.

### 1.10 Specification Structure

The remainder of this document is organized as follows:

- **Part I (Sections 1-3)**: Foundations including use cases and algorithmic prerequisites
- **Part II (Sections 4-5)**: Architecture and data structure design
- **Part III (Sections 6-8)**: Preprocessing, fingerprinting, and dimension ordering
- **Part IV (Sections 9-14)**: Spreadsheet mode alignment (the AMR algorithm)
- **Part V (Sections 15-18)**: Database mode alignment
- **Part VI (Sections 19-20)**: Cell-level comparison including formula semantics
- **Part VII (Sections 21-23)**: Result assembly and output formatting
- **Part VIII (Sections 24-26)**: Robustness, memory management, and parallelism
- **Part IX (Sections 27-29)**: Complexity analysis and configuration
- **Part X (Sections 30-32)**: Appendices with pseudocode, examples, and glossary

Each section specifies behavior in sufficient detail that an implementer can write correct code without making significant design decisions. Ambiguities should be resolved by reference to the stated design principles and quality attributes.

---

## 2. Use Cases & Requirements Mapping

### 2.1 Use Case Framework

This section defines the complete set of use cases that the grid diff algorithm must handle correctly. Each use case specifies:

- **Scenario**: A concrete example of input conditions
- **Expected Behavior**: What the algorithm must produce
- **Acceptance Criteria**: Testable conditions for correctness
- **Priority**: Must-have (M), Should-have (S), or Nice-to-have (N)
- **Complexity Impact**: How the scenario affects algorithm design

Use cases are organized into five categories: Basic, Alignment, Move Detection, Adversarial/Edge, and Database Mode.

---

### 2.2 Basic Cases (UC-01 through UC-05)

These represent the simplest and most common scenarios. The algorithm must handle all basic cases with optimal performance and zero false positives.

#### UC-01: Identical Grids

**Scenario**: Two grids with identical structure and cell values.

**Expected Behavior**: The algorithm produces an empty diff—no operations of any kind.

**Acceptance Criteria**:
- Output contains zero `RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`, `CellEdited`, or move operations
- Runtime is dominated by hashing, not alignment (O(M) where M = non-empty cells)
- Memory usage is minimal beyond the input grids

**Priority**: Must-have

**Complexity Impact**: This is the fast path. Row and column hashes match perfectly; anchoring finds complete alignment; no gap filling or cell comparison is needed. The algorithm should detect this condition early and short-circuit.

---

#### UC-02: Single Cell Change

**Scenario**: Two grids that differ in exactly one cell value.

**Expected Behavior**: The algorithm produces exactly one `CellEdited` operation identifying the changed cell with its old and new values.

**Acceptance Criteria**:
- Exactly one `CellEdited` operation is emitted
- No structural operations (`RowAdded`, `RowRemoved`, etc.) are emitted
- The operation correctly identifies row index, column index, old value, and new value
- Runtime remains O(M) since structure is unchanged

**Priority**: Must-have

**Complexity Impact**: Row hashes differ for the affected row; column hashes differ for the affected column. However, since only one row/column is affected, alignment is trivial. The key is ensuring that a single cell change does not trigger cascading false positives.

---

#### UC-03: Multiple Scattered Cell Edits

**Scenario**: Two grids with identical structure but multiple cells changed across different rows and columns.

**Expected Behavior**: The algorithm produces one `CellEdited` operation for each changed cell, with no structural operations.

**Acceptance Criteria**:
- Number of `CellEdited` operations equals number of actually changed cells
- No structural operations are emitted
- Operations are sorted in deterministic order (by row, then column)
- Each operation correctly identifies the cell location and value change

**Priority**: Must-have

**Complexity Impact**: Multiple rows have different hashes, but anchor discovery still finds the unchanged rows as anchors, and gap filling handles the changed rows. The cell-level diff phase does the actual comparison work.

---

#### UC-04: Row Append at End

**Scenario**: Grid B has one or more additional rows at the end compared to Grid A.

**Expected Behavior**: The algorithm produces `RowAdded` operations for the appended rows. No existing cells are reported as changed.

**Acceptance Criteria**:
- `RowAdded` operations are emitted for each new row (or a single block operation)
- No `CellEdited` operations are emitted for existing rows
- No `RowRemoved` operations are emitted
- Row indices in Grid B are correctly identified

**Priority**: Must-have

**Complexity Impact**: All rows in Grid A anchor to their corresponding positions in Grid B. The tail gap contains only Grid B rows, which are marked as insertions. This is a trivial case for the gap filler.

---

#### UC-05: Column Append at Edge

**Scenario**: Grid B has one or more additional columns at the right edge compared to Grid A.

**Expected Behavior**: The algorithm produces `ColumnAdded` operations for the appended columns. No existing cells are reported as changed.

**Acceptance Criteria**:
- `ColumnAdded` operations are emitted for each new column
- No `CellEdited` operations are emitted for existing cells
- Column indices in Grid B are correctly identified
- Cell values in new columns are not compared against anything (they're purely additions)

**Priority**: Must-have

**Complexity Impact**: Column alignment handles this trivially. The key requirement is that row hashes must not be invalidated by the presence of new columns—either by hashing only matched columns, or by accepting that row hashes will differ and relying on column alignment to establish the correct mapping.

---

### 2.3 Alignment Cases (UC-06 through UC-10)

These cases test the core alignment algorithm's ability to correctly identify insertions and deletions while maintaining correct correspondence between unchanged content.

#### UC-06: Single Row Insert in Middle

**Scenario**: Grid B has one additional row inserted somewhere in the middle of the grid.

**Expected Behavior**: The algorithm produces exactly one `RowAdded` operation. Rows above and below the insertion point are correctly aligned, with no spurious `CellEdited` operations.

**Acceptance Criteria**:
- Exactly one `RowAdded` operation is emitted
- Rows before the insertion point align to the same indices in both grids
- Rows after the insertion point in Grid B align to their corresponding (shifted) positions in Grid A
- No `CellEdited` operations are emitted unless cells actually changed

**Priority**: Must-have

**Complexity Impact**: This is the canonical test of sequence alignment. Anchors from unique rows above and below the insertion point establish the correct alignment. The gap containing the inserted row is trivially resolved. The challenge is ensuring that row indices in `CellEdited` operations (if any) correctly reflect the alignment, not raw grid positions.

---

#### UC-07: Single Row Delete in Middle

**Scenario**: Grid B is missing one row that was present in Grid A.

**Expected Behavior**: The algorithm produces exactly one `RowRemoved` operation. Remaining rows are correctly aligned.

**Acceptance Criteria**:
- Exactly one `RowRemoved` operation is emitted
- Rows before the deletion point align correctly
- Rows after the deletion point align to their shifted positions
- No spurious `CellEdited` operations

**Priority**: Must-have

**Complexity Impact**: Mirror of UC-06. The deleted row appears in a gap as content from Grid A with no corresponding content in Grid B.

---

#### UC-08: Single Column Insert in Middle

**Scenario**: Grid B has one additional column inserted in the middle of the grid.

**Expected Behavior**: The algorithm produces exactly one `ColumnAdded` operation. Cells in existing columns are correctly aligned and compared.

**Acceptance Criteria**:
- Exactly one `ColumnAdded` operation is emitted
- Cells in columns before the insertion point compare correctly
- Cells in columns after the insertion point compare to their shifted counterparts
- No spurious `CellEdited` operations for cells whose values haven't changed

**Priority**: Must-have

**Complexity Impact**: Column alignment handles this directly. The critical requirement is that the column mapping is correctly applied during cell-level comparison, so that column 5 in Grid A is compared to column 6 in Grid B (if column 4 was inserted), not to column 5.

---

#### UC-09: Block of Rows Inserted

**Scenario**: Grid B has a contiguous block of multiple rows (e.g., 10-1000 rows) inserted at some position.

**Expected Behavior**: The algorithm produces `RowAdded` operations for all inserted rows. These may be coalesced into a block operation for presentation purposes.

**Acceptance Criteria**:
- All inserted rows are identified
- Rows before and after the block align correctly
- No spurious edits in unaffected rows
- Runtime remains near-linear regardless of block size

**Priority**: Must-have

**Complexity Impact**: The anchor chain excludes the inserted block entirely. A single gap contains all the new rows. The gap filler marks them as insertions in O(block_size) time.

---

#### UC-10: Block of Rows Deleted

**Scenario**: Grid B is missing a contiguous block of rows that was present in Grid A.

**Expected Behavior**: The algorithm produces `RowRemoved` operations for all deleted rows.

**Acceptance Criteria**:
- All deleted rows are identified
- Remaining rows align correctly
- No spurious edits
- Runtime near-linear

**Priority**: Must-have

**Complexity Impact**: Mirror of UC-09.

---

### 2.4 Move Detection Cases (UC-11 through UC-13)

These cases test the algorithm's ability to recognize when content has changed position without changing substance.

#### UC-11: Block of Rows Moved (Unchanged Content)

**Scenario**: A contiguous block of rows appears in both grids but at different positions. The content of the block is identical.

**Expected Behavior**: The algorithm produces a `BlockMovedRows` operation identifying the source and destination positions. No `RowAdded` or `RowRemoved` operations for the moved rows. No `CellEdited` operations within the moved block.

**Acceptance Criteria**:
- A single `BlockMovedRows` operation is emitted
- Source position (in Grid A) is correctly identified
- Destination position (in Grid B) is correctly identified
- Block size matches the actual moved content
- No delete+insert operations for the moved rows
- No `CellEdited` operations within the block

**Priority**: Must-have

**Complexity Impact**: This is the primary justification for the AMR (Anchor-Move-Refine) algorithm. Without move-aware alignment, the moved block would appear as deletions at the source and insertions at the destination. The algorithm must recognize the hash match between deleted and inserted blocks and reclassify them as a move.

---

#### UC-12: Block Moved with Internal Edits (Fuzzy Move)

**Scenario**: A block of rows has moved to a different position AND some cells within the block have been edited.

**Expected Behavior**: The algorithm produces a `BlockMovedRows` operation AND `CellEdited` operations for the changed cells within the block.

**Acceptance Criteria**:
- A `BlockMovedRows` operation is emitted
- `CellEdited` operations are emitted for cells that actually changed
- The block is not reported as separate delete+insert operations
- The threshold for recognizing a fuzzy move is configurable (default: 80% similarity)

**Priority**: Should-have

**Complexity Impact**: Exact hash match fails because content differs. The algorithm must compute block similarity (e.g., Jaccard similarity of row hashes) and accept blocks above a threshold as moves. After classifying as a move, the algorithm must perform cell-level diff within the block to identify internal edits.

---

#### UC-13: Column Moved

**Scenario**: One or more columns appear in both grids but at different horizontal positions.

**Expected Behavior**: The algorithm produces a `BlockMovedColumns` operation.

**Acceptance Criteria**:
- `BlockMovedColumns` operation is emitted
- Source and destination column ranges are correct
- No `ColumnAdded`/`ColumnRemoved` for moved columns
- Cells in moved columns compare correctly to their corresponding cells

**Priority**: Should-have

**Complexity Impact**: Column alignment must detect moves using the same hash-and-match approach as row moves. Since column count is small (typically ≤100), this can use a more expensive algorithm (even O(C²)) without performance concerns.

---

### 2.5 Adversarial and Edge Cases (UC-14 through UC-16)

These cases test robustness against inputs that could cause naive algorithms to exhibit poor performance or produce incorrect results.

#### UC-14: 99% Identical Rows (Repetitive Content)

**Scenario**: Grid has 50,000 rows where 49,500 are identical (e.g., blank rows or template rows). The remaining 500 rows are unique. A block of rows is inserted in the middle.

**Expected Behavior**: The algorithm correctly identifies the inserted block and aligns the unique rows. Runtime remains under 1 second.

**Acceptance Criteria**:
- Inserted rows are correctly identified
- Unique rows align correctly
- No quadratic blowup in runtime (must complete in <1s on target hardware)
- Memory usage does not explode
- The repetitive rows are not the source of spurious edit operations

**Priority**: Must-have

**Complexity Impact**: This is the critical adversarial case that motivates several design decisions:
- Patience Diff anchoring on *unique* rows only, ignoring high-frequency rows
- Run-length encoding of repetitive content
- Gap strategy selection that detects repetitive gaps and uses counting/RLE alignment instead of full DP

Without these defenses, Hunt-Szymanski LCS would see K ≈ N² matches (every blank row matches every other blank row), causing O(N² log N) behavior.

---

#### UC-15: Completely Different Grids

**Scenario**: Two grids share almost no content—different dimensions, different values, no meaningful alignment exists.

**Expected Behavior**: The algorithm quickly recognizes the lack of commonality and produces a simple "all removed, all added" result rather than attempting expensive alignment.

**Acceptance Criteria**:
- Runtime is bounded (under 500ms regardless of grid size)
- No quadratic alignment attempts
- Output clearly indicates the grids are essentially different documents
- Optionally, a similarity score is provided

**Priority**: Must-have

**Complexity Impact**: Early similarity detection must identify this condition before expensive alignment begins. When detected:
- Skip anchor discovery (no anchors will be found)
- Skip gap filling (would find no matches)
- Emit block removal of all Grid A content and block addition of all Grid B content

---

#### UC-16: Very Sparse Grids

**Scenario**: Grids have large logical dimensions (e.g., 10,000 rows × 1,000 columns) but only a small number of non-empty cells (e.g., 5,000 cells scattered throughout).

**Expected Behavior**: The algorithm operates efficiently in proportion to actual content, not logical dimensions.

**Acceptance Criteria**:
- Runtime is O(M) where M = non-empty cells, not O(R × C)
- Memory usage is proportional to M, not R × C
- Empty rows are handled efficiently (possibly ignored entirely)
- Sparse regions do not cause spurious operations

**Priority**: Must-have

**Complexity Impact**: The sparse grid representation must be preserved throughout. Row/column hashes must be computable without materializing full rows. Alignment operates on non-empty content only.

---

### 2.6 Database Mode Cases (UC-17 through UC-20)

These cases test the key-based alignment mode where row identity is determined by column values rather than position.

#### UC-17: Keyed Rows, Same Keys Different Order

**Scenario**: Both grids contain the same keys in different orders. Non-key column values are identical.

**Expected Behavior**: The algorithm produces no diff—the grids are semantically identical in database mode.

**Acceptance Criteria**:
- Empty diff output (no operations)
- Row reordering is not reported as a change
- Keys are correctly matched across grids
- All non-key columns compare equal

**Priority**: Must-have for database mode

**Complexity Impact**: Hash join on keys provides O(N) expected alignment. Row order is explicitly ignored. This is a key differentiator from spreadsheet mode.

---

#### UC-18: Keyed Rows with Edits

**Scenario**: Both grids contain the same keys. Some non-key column values have changed.

**Expected Behavior**: The algorithm produces `CellEdited` operations for changed cells, with row identity determined by key match.

**Acceptance Criteria**:
- `CellEdited` operations correctly identify changed cells
- Row matching is based on key values, not position
- Operations reference the correct row in each grid
- Key columns themselves are not reported as edited (if unchanged)

**Priority**: Must-have for database mode

**Complexity Impact**: After key-based row matching, cell comparison proceeds normally. The alignment is trivial (hash join); the work is in cell comparison.

---

#### UC-19: Duplicate Keys

**Scenario**: Multiple rows in one or both grids share the same key value.

**Expected Behavior**: The algorithm matches rows within each key cluster based on content similarity. Unmatched rows are reported as added/removed.

**Acceptance Criteria**:
- Rows with unique keys match 1:1
- Rows with duplicate keys form clusters
- Within clusters, rows are matched by content similarity (not position)
- Optimal or near-optimal matching minimizes reported changes
- Unmatched rows in clusters are reported as additions/removals

**Priority**: Should-have for database mode

**Complexity Impact**: Duplicate key clusters require solving a bipartite matching problem. Within each cluster:
- Construct a cost matrix based on row similarity (Jaccard, Hamming, etc.)
- Solve using Hungarian algorithm or LAPJV
- Cluster sizes are typically small (single digits), so O(K³) is acceptable

---

#### UC-20: Mixed Sheet (Table Region + Free-Form Cells)

**Scenario**: A worksheet contains a structured data table (with identifiable keys) surrounded by free-form content (titles, notes, summaries).

**Expected Behavior**: The algorithm uses database mode for the table region and spreadsheet mode for the surrounding content.

**Acceptance Criteria**:
- Table region is correctly identified
- Database mode alignment is applied within the table
- Spreadsheet mode alignment is applied outside the table
- Results are merged coherently
- Changes that span region boundaries are handled sensibly

**Priority**: Nice-to-have (requires region detection capability)

**Complexity Impact**: Requires a segmentation phase before alignment. Table detection may use heuristics (header row detection, data type consistency, Excel Table metadata). Each region is diffed independently, then results are merged.

---

### 2.7 Requirements Traceability Matrix

The following matrix maps use cases to algorithm phases:

| Use Case | Preprocessing | Dim. Order | Mode Selection | Row Align | Col Align | Move Detect | Cell Diff |
|----------|--------------|------------|----------------|-----------|-----------|-------------|-----------|
| UC-01 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-02 | ● | ○ | ○ | ● | ● | ○ | ● |
| UC-03 | ● | ○ | ○ | ● | ● | ○ | ● |
| UC-04 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-05 | ● | ○ | ○ | ○ | ● | ○ | ○ |
| UC-06 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-07 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-08 | ● | ● | ○ | ○ | ● | ○ | ● |
| UC-09 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-10 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-11 | ● | ○ | ○ | ● | ○ | ● | ○ |
| UC-12 | ● | ○ | ○ | ● | ○ | ● | ● |
| UC-13 | ● | ○ | ○ | ○ | ● | ● | ○ |
| UC-14 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-15 | ● | ○ | ○ | ● | ○ | ○ | ○ |
| UC-16 | ● | ○ | ○ | ● | ● | ○ | ● |
| UC-17 | ● | ○ | ● | ● | ○ | ○ | ○ |
| UC-18 | ● | ○ | ● | ● | ○ | ○ | ● |
| UC-19 | ● | ○ | ● | ● | ○ | ○ | ● |
| UC-20 | ● | ○ | ● | ● | ● | ○ | ● |

Legend: ● = Critical path for this use case, ○ = Not applicable or trivial

---

### 2.8 Priority Summary

**Must-Have (16 use cases)**: UC-01 through UC-11, UC-14 through UC-18

These use cases represent core functionality that every user will encounter. The algorithm cannot ship without correct and performant handling of all must-have cases.

**Should-Have (3 use cases)**: UC-12, UC-13, UC-19

These use cases represent important functionality that significantly improves user experience but is not blocking for initial release. Fuzzy move detection and column moves provide semantic richness; duplicate key handling provides robustness in database mode.

**Nice-to-Have (1 use case)**: UC-20

Mixed sheet handling requires additional infrastructure (region detection) and serves a subset of users. It can be implemented after core functionality is stable.

---

## 3. Algorithmic Foundations Reference

### 3.1 Purpose of This Section

This section provides a reference for the fundamental algorithms that compose the grid diff engine. For each algorithm, we specify:

- What problem it solves
- How it works (conceptually, not implementation)
- Its complexity characteristics
- Why it was chosen over alternatives
- How it integrates with the overall design

Implementers should understand these foundations before reading the detailed design sections. The algorithms are presented in order of their role in the pipeline.

---

### 3.2 Longest Increasing Subsequence (LIS)

#### Problem Definition

Given a sequence of numbers S = [s₁, s₂, ..., sₙ], find the longest subsequence (not necessarily contiguous) such that each element is strictly greater than the previous.

**Example**: For S = [3, 1, 4, 1, 5, 9, 2, 6], one LIS is [1, 4, 5, 6] with length 4.

#### Role in Grid Diff

LIS is used to extract a monotonic chain of anchor matches. When we find rows that match between grids (same hash), we get pairs (posA, posB). A valid alignment requires these pairs to be monotonic in both dimensions—if row i in A matches row j in B, and row i' > i in A matches row j' in B, then j' > j.

The LIS of the B-positions (when pairs are sorted by A-position) gives the maximum number of matches that can be simultaneously honored without crossing.

#### Algorithm: Patience Sorting

The standard O(N log N) algorithm for LIS uses a technique related to the card game Patience:

1. Maintain a list of "piles," each containing indices
2. For each element, binary search to find the leftmost pile whose top is ≥ current element
3. Place the element on that pile (or start a new pile if none qualifies)
4. The number of piles equals the LIS length
5. Backtracking through predecessor links recovers the actual subsequence

**Complexity**: O(N log N) time, O(N) space

#### Why Patience/LIS for Anchoring

- **Robustness**: Finds the maximum set of non-crossing matches automatically
- **Speed**: O(N log N) regardless of input patterns
- **Stability**: Small changes to input produce small changes to the anchor chain
- **Well-studied**: Mature algorithms with known edge cases

Alternative considered: Naive O(N²) DP for LIS. Rejected due to quadratic complexity on large grids.

---

### 3.3 Patience Diff

#### Problem Definition

Given two sequences A and B, find an alignment that prioritizes matching "unique" elements (elements appearing exactly once in each sequence) to establish reliable anchor points.

#### Role in Grid Diff

Patience Diff is the primary anchoring strategy for row alignment. It:

1. Identifies rows with unique hashes (appear once in Grid A and once in Grid B)
2. These unique rows form reliable match candidates
3. Applies LIS to extract a monotonic chain of these matches
4. The resulting anchors partition the grids into independent gaps

#### Algorithm Sketch

1. **Build hash frequency tables** for both sequences
2. **Filter to unique elements**: hash appears exactly once in A AND exactly once in B
3. **Build match pairs**: For each unique hash, record (positionA, positionB)
4. **Sort by A-position**: Arrange pairs in order of their A index
5. **Extract LIS of B-positions**: Find the longest increasing subsequence
6. **Return anchor chain**: The pairs participating in the LIS

#### Complexity

- Building frequency tables: O(N)
- Filtering and pairing: O(N)
- Sorting: O(K log K) where K = number of unique matches
- LIS: O(K log K)
- **Total**: O(N + K log K), which is O(N log N) worst case

#### Why Patience Diff for Row Alignment

- **Ignores repetitive content**: High-frequency rows (blank, template) are automatically excluded from anchoring, preventing the quadratic explosion that affects other algorithms
- **Finds structural skeleton**: Unique rows typically represent meaningful content (headers, distinct data rows), creating a semantically relevant partition
- **Handles large inputs**: O(N log N) scales to 50K+ rows
- **Git heritage**: Proven effective in text diff (git diff uses Patience as an option)

Alternative considered: Full LCS on all rows. Rejected because K (matches) could be O(N²) with repetitive data.

---

### 3.4 Myers Diff Algorithm

#### Problem Definition

Given two sequences A and B, find a shortest edit script (SES) that transforms A into B using only insertions and deletions. Equivalently, find a longest common subsequence (LCS).

#### Role in Grid Diff

Myers diff is used for **gap filling**—aligning the rows within segments between anchors. When anchor discovery partitions the grid into gaps, each gap is a smaller alignment problem that Myers handles efficiently.

#### Algorithm Concept

Myers models the problem as shortest path finding in an "edit graph":

- Nodes are positions (i, j) representing "consumed i elements of A and j elements of B"
- Horizontal edges (delete from A) have cost 1
- Vertical edges (insert from B) have cost 1
- Diagonal edges (match) have cost 0 and are available when A[i] = B[j]

The algorithm finds the shortest path from (0, 0) to (N, M) using a clever wavefront expansion:

1. D = 0: Start at origin, extend diagonally as far as possible
2. D = 1: Try paths with exactly 1 edit, extend diagonally
3. D = k: Try paths with exactly k edits
4. Stop when reaching (N, M)

#### Complexity

- **Time**: O(ND) where N = max(|A|, |B|) and D = edit distance
- **Space**: O(N) with Hirschberg's linear-space refinement, O(N²) naive

The O(ND) complexity is key: when sequences are similar (small D), the algorithm is fast. When sequences are very different (D ≈ N), it degrades to O(N²), but this is detected and handled by bail-out strategies.

#### Why Myers for Gap Filling

- **Optimal for similar sequences**: Most gaps between anchors have small edit distance
- **Adaptive complexity**: Fast when changes are few, only slow when necessary
- **Well-understood**: Standard algorithm with mature implementations
- **Linear space possible**: Important for WASM memory constraints

Alternatives considered:
- Hunt-Szymanski: O(N log N + K log K) where K = matches. Rejected because K → N² on repetitive data
- Histogram diff: Heuristic, faster but not optimal. Used as secondary strategy for larger gaps

---

### 3.5 Run-Length Encoding (RLE)

#### Problem Definition

Given a sequence with repeated elements, compress it by replacing runs of identical elements with (element, count) pairs.

**Example**: [A, A, A, B, B, A] → [(A, 3), (B, 2), (A, 1)]

#### Role in Grid Diff

RLE compresses row sequences before alignment when repetitive content is detected. This transforms the 99% blank rows problem:

- 50,000 rows with 49,500 blanks → perhaps 10 runs
- Alignment operates on runs, not individual rows
- O(50,000²) potential problem becomes O(10²)

#### Application to Grid Diff

1. **Tokenize rows**: Each row gets a token ID based on its hash
2. **Build runs**: Consecutive rows with the same token form a run
3. **Align runs**: Modified alignment algorithm compares run tokens
4. **Expand results**: Convert run-level alignment back to row-level operations

When two runs of the same token match:
- Match count = min(run_length_A, run_length_B) rows
- Excess rows = |run_length_A - run_length_B| are added/removed

#### Complexity Impact

- RLE construction: O(N)
- Alignment on runs: O(R²) or O(RD) where R = number of runs
- For repetitive data: R << N, providing dramatic speedup

#### Why RLE for Repetitive Content

- **Defeats adversarial inputs**: The 99% blank case is the primary motivation
- **Preserves semantics**: Matching runs of blanks is equivalent to matching individual blanks
- **Composable**: Works with Myers, Patience, and other algorithms
- **Low overhead**: When data isn't repetitive, R ≈ N and RLE adds minimal cost

Alternative considered: Just detecting repetitive gaps and using counting heuristics. RLE is more general and integrates better with standard algorithms.

---

### 3.6 Linear Assignment Problem (LAPJV / Hungarian)

#### Problem Definition

Given an N×M cost matrix C where C[i][j] is the cost of assigning worker i to task j, find an assignment that minimizes total cost. Each worker is assigned to at most one task; each task is assigned to at most one worker.

#### Role in Grid Diff

The assignment problem appears in two contexts:

1. **Move detection**: Matching deleted blocks with inserted blocks to identify moves
2. **Duplicate key resolution**: Matching rows within a duplicate-key cluster in database mode

In both cases, we have two sets of entities and need to find the optimal pairing based on a similarity/cost metric.

#### Algorithm: Jonker-Volgenant (LAPJV)

LAPJV is the standard efficient algorithm for the assignment problem:

1. Initialize dual variables (prices) for rows and columns
2. Iteratively find augmenting paths that improve the assignment
3. Use shortest-path-based augmentation for efficiency

**Complexity**: O(N³) for an N×N matrix

For rectangular matrices (N×M), complexity is O(min(N,M) × N × M).

#### Why LAPJV for Block Matching

- **Globally optimal**: Unlike greedy matching, LAPJV finds the true minimum-cost assignment
- **Handles competing candidates**: When block A could match either B1 or B2, LAPJV considers all options
- **Small problem size**: Applied only to blocks (dozens, not thousands), so O(K³) with K ≤ 50-100 is negligible

The key insight is that LAPJV is applied at the **block level**, not the row level. The number of unmatched blocks after anchor-based alignment is naturally small, making the cubic complexity acceptable.

Alternative considered: Greedy matching (match highest-similarity pairs first). Simpler but can produce suboptimal results when blocks compete for matches.

---

### 3.7 Hash Functions

#### Problem Definition

Map arbitrary data to fixed-size fingerprints such that:
- Equal inputs produce equal outputs (determinism)
- Different inputs almost always produce different outputs (collision resistance)
- Computation is fast

#### Role in Grid Diff

Hashing is used throughout the pipeline:
- **Row fingerprinting**: Each row gets a hash summarizing its content
- **Column fingerprinting**: Each column gets a hash
- **Block hashing**: Contiguous blocks get hashes for move detection
- **Token interning**: Mapping hashes to compact integer IDs

#### Hash Function Selection

Two hash functions are supported:

**XXHash64 (Default)**
- Output: 64 bits
- Speed: ~15 GB/s on modern CPUs
- Collision probability: ~1/2⁶⁴ per pair, birthday bound ~2³² items before 50% collision
- For 50K rows: Collision probability ≈ 0.00006%

**BLAKE3 (128-bit mode)**
- Output: 128 bits (truncated from 256)
- Speed: ~10 GB/s on modern CPUs (slightly slower than XXHash64)
- Collision probability: Negligible even at scale
- Provides cryptographic-grade collision resistance

#### Selection Criteria

| Factor | XXHash64 | BLAKE3-128 |
|--------|----------|------------|
| Speed | 10-15% faster | Baseline |
| Collision risk at 50K rows | ~0.00006% | Negligible |
| Order-independent schemes | Limited (XOR-based) | Safe (product + XOR) |
| Recommended for | Performance-critical | Audit/compliance, maximum safety |

**Default choice**: XXHash64 for typical workloads. The collision probability is acceptable for diff operations, and the speed advantage compounds across millions of cells.

**When to use BLAKE3**: When the diff result will be used for audit trails, compliance verification, or any context where even theoretical collision risk is unacceptable.

#### Hashing Strategy

Row hashes must capture semantic content while being computable efficiently:

**Included in hash**:
- Cell values (normalized: numbers to canonical form, strings trimmed)
- Cell types (to distinguish "1" from 1.0)
- Column positions (so [A:1, B:2] ≠ [A:2, B:1])
- Formula text (normalized)

**Excluded from hash** (by default):
- Cell formatting (font, color, borders)
- Comments and notes
- Conditional formatting state

The hash must be **order-dependent** within a row (column positions matter) but can be computed by iterating cells in any order as long as the combination function is consistent.

---

### 3.8 Jaccard Similarity and Related Metrics

#### Problem Definition

Measure the similarity between two sets (or multisets) to support fuzzy matching decisions.

#### Role in Grid Diff

Similarity metrics are used for:
- Deciding if two blocks are similar enough to classify as a "fuzzy move"
- Matching rows within duplicate-key clusters
- Early bail-out detection (if grids have very low similarity, skip alignment)

#### Metrics

**Jaccard Similarity**

J(A, B) = |A ∩ B| / |A ∪ B|

- Range: 0 (no overlap) to 1 (identical)
- Properties: Symmetric, penalizes both additions and removals equally
- Use case: General-purpose similarity for block comparison

**Dice Coefficient**

D(A, B) = 2|A ∩ B| / (|A| + |B|)

- Range: 0 to 1
- Properties: More forgiving of size differences than Jaccard
- Use case: When one block might be a subset of another

**Overlap Coefficient**

O(A, B) = |A ∩ B| / min(|A|, |B|)

- Range: 0 to 1
- Properties: Measures containment; equals 1 if smaller set is contained in larger
- Use case: Detecting when a block was expanded or contracted

**Hamming Similarity (for fixed-size comparisons)**

H(A, B) = (number of matching positions) / (total positions)

- Range: 0 to 1
- Use case: Comparing aligned rows cell-by-cell

#### Default Choice

**Jaccard similarity** is the default for block comparison because:
- It's symmetric (A vs B same as B vs A)
- It penalizes both extra and missing elements
- It's easy to compute from hash sets
- It's well-understood with intuitive interpretation

#### Threshold Selection

Similarity thresholds determine when blocks are considered "the same" for move detection:

| Threshold | Interpretation | Use Case |
|-----------|---------------|----------|
| 1.0 | Exact match only | Exact move detection |
| 0.8-0.9 | High similarity, few edits | Fuzzy move (default: 0.80) |
| 0.5-0.7 | Moderate similarity | Aggressive fuzzy matching |
| < 0.5 | Low similarity | Treat as delete + insert |

Thresholds may be adjusted based on row width (narrow rows need higher thresholds) and data heterogeneity.

---

### 3.9 Algorithm Selection Summary

| Algorithm | Problem | Where Used | Complexity | Why Chosen |
|-----------|---------|------------|------------|------------|
| LIS/Patience Sort | Monotonic subsequence | Anchor extraction | O(K log K) | Optimal, handles crossing matches |
| Patience Diff | Unique-based anchoring | Row alignment | O(N log N) | Ignores repetitive data |
| Myers Diff | Optimal LCS/SES | Gap filling | O(ND) | Adaptive, optimal for similar sequences |
| RLE | Sequence compression | Repetitive gaps | O(N) | Defeats adversarial repetition |
| LAPJV | Bipartite assignment | Move detection, clusters | O(K³) | Globally optimal matching |
| XXHash64/BLAKE3 | Fingerprinting | Everywhere | O(bytes) | Fast, collision-resistant |
| Jaccard | Set similarity | Fuzzy matching | O(|A|+|B|) | Intuitive, symmetric |

The combination of these algorithms, applied at appropriate points in the pipeline, achieves the performance and correctness requirements specified in Sections 1 and 2.

---

### 3.10 Complexity Guarantee Summary

By careful algorithm selection and composition, the grid diff engine achieves:

| Input Pattern | Complexity | Rationale |
|--------------|------------|-----------|
| Typical (few edits) | O(M + R log R) | Anchors partition quickly; small gaps |
| Heavy edits (30% changed) | O(M + R log R + D) | Myers on gaps; D is bounded by actual changes |
| Repetitive (99% same) | O(M + R) | RLE compresses; alignment on runs |
| Completely different | O(M + R) | Early bail-out; block replacement |
| Database mode | O(M + R) | Hash join; small cluster matching |

Where:
- M = number of non-empty cells
- R = number of rows
- D = total edit distance across all gaps

The critical guarantee is that **no input pattern causes O(R²) behavior**. This is achieved through:
1. Patience Diff ignoring high-frequency rows
2. RLE compressing repetitive content
3. Cost caps and bail-out for pathological gaps
4. LAPJV applied only to small block sets

---

# Part II: Architecture & Data Structures

## 4. Pipeline Architecture

### 4.1 Overview

The grid diff engine processes two input grids through a six-phase pipeline. Each phase has well-defined inputs, outputs, and responsibilities. The pipeline is designed for:

- **Modularity**: Phases can be tested, optimized, and replaced independently
- **Parallelism**: Multiple phases contain embarrassingly parallel operations
- **Early termination**: Fast paths exit early when possible (identical grids, completely different grids)
- **Memory efficiency**: Intermediate structures are ephemeral and sized to actual content

### 4.2 Pipeline Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           GRID DIFF PIPELINE                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                         PHASE 1: PREPROCESSING                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │ │
│  │  │ Build Row   │  │ Build Col   │  │  Compute    │  │  Frequency  │        │ │
│  │  │   Views     │  │   Views     │  │   Hashes    │  │  Analysis   │        │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │ │
│  │                              ↓                                              │ │
│  │                    GridView A, GridView B                                   │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                             │
│                                    ▼                                             │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                     PHASE 2: DIMENSION DECISION                             │ │
│  │                                                                             │ │
│  │  Check column stability → Decide row-first or column-first ordering        │ │
│  │                                                                             │ │
│  │  Output: DimensionOrder (RowFirst | ColumnFirst { col_mapping })            │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                             │
│                                    ▼                                             │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                       PHASE 3: MODE SELECTION                               │ │
│  │                                                                             │ │
│  │  Determine Spreadsheet vs Database mode (per region if mixed)               │ │
│  │  Infer keys if database mode and not user-specified                         │ │
│  │                                                                             │ │
│  │  Output: ModeConfig per region                                              │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                             │
│                    ┌───────────────┴───────────────┐                            │
│                    ▼                               ▼                            │
│  ┌─────────────────────────────┐    ┌─────────────────────────────┐            │
│  │      PHASE 4a: SPREADSHEET  │    │      PHASE 4b: DATABASE     │            │
│  │        MODE ALIGNMENT       │    │       MODE ALIGNMENT        │            │
│  │                             │    │                             │            │
│  │  • AMR Row Alignment        │    │  • Key Extraction           │            │
│  │    - Anchor Discovery       │    │  • Hash Join                │            │
│  │    - Move Candidate Extract │    │  • Duplicate Cluster Match  │            │
│  │    - Move-Aware Gap Fill    │    │                             │            │
│  │    - Move Validation        │    │                             │            │
│  │  • Column Alignment         │    │                             │            │
│  │                             │    │                             │            │
│  │  Output: RowAlignment,      │    │  Output: KeyedAlignment     │            │
│  │          ColumnAlignment,   │    │                             │            │
│  │          ValidatedMoves     │    │                             │            │
│  └─────────────────────────────┘    └─────────────────────────────┘            │
│                    │                               │                            │
│                    └───────────────┬───────────────┘                            │
│                                    ▼                                             │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                       PHASE 5: CELL-LEVEL DIFF                              │ │
│  │                                                                             │ │
│  │  For each aligned row pair:                                                 │ │
│  │    Compare cells using column mapping                                       │ │
│  │    Apply formula semantic analysis if applicable                            │ │
│  │    Emit CellEdited operations                                               │ │
│  │                                                                             │ │
│  │  Output: Vec<CellEdit>                                                      │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                             │
│                                    ▼                                             │
│  ┌────────────────────────────────────────────────────────────────────────────┐ │
│  │                      PHASE 6: RESULT ASSEMBLY                               │ │
│  │                                                                             │ │
│  │  • Coalesce operations (consecutive adds → block add)                       │ │
│  │  • Deduplicate move vs add/remove                                           │ │
│  │  • Detect rectangular moves (correlated row + column moves)                 │ │
│  │  • Sort for deterministic output                                            │ │
│  │  • Emit final DiffOps                                                       │ │
│  │                                                                             │ │
│  │  Output: DiffResult                                                         │ │
│  └────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Phase Descriptions

#### Phase 1: Preprocessing & Hashing

**Purpose**: Transform sparse input grids into algorithm-friendly views and compute all fingerprints.

**Inputs**:
- Grid A: Sparse cell map with dimensions
- Grid B: Sparse cell map with dimensions
- Hash configuration (64-bit or 128-bit)

**Operations**:
1. Construct ephemeral `GridView` for each grid containing:
   - `RowView` array with cells sorted by column
   - `RowMeta` array with hash, non-blank count, first non-blank column
   - `ColMeta` array with hash, non-blank count, first non-blank row
2. Compute row hashes by iterating cells within each row
3. Compute column hashes by iterating cells within each column
4. Build frequency tables: count of each hash in each grid
5. Classify rows: unique, rare, common, low-information

**Outputs**:
- `GridView` A with metadata
- `GridView` B with metadata
- `HashStats` with frequency information

**Complexity**: O(M) where M = total non-empty cells across both grids

**Parallelism**: Row hashing is embarrassingly parallel; each row can be processed independently. Column hashing requires coordination but can be parallelized across columns.

---

#### Phase 2: Dimension Decision

**Purpose**: Determine whether to align rows first or columns first based on column stability.

**Inputs**:
- `GridView` A and B with column metadata
- Column stability threshold (default: 0.90)

**Operations**:
1. Compute Jaccard similarity of column hash sets
2. If similarity ≥ threshold: use row-first ordering
3. If similarity < threshold: use column-first ordering
   - Align columns first using Myers/Patience
   - Generate column mapping (colA → colB)
   - Optionally recompute row hashes using only matched columns

**Outputs**:
- `DimensionOrder::RowFirst` OR
- `DimensionOrder::ColumnFirst { col_mapping: Vec<Option<u32>> }`

**Complexity**: O(C) where C = number of columns

**Rationale**: When columns are inserted/deleted, every row's hash changes (if computed over all columns). By detecting column instability and establishing a column mapping first, row alignment becomes more accurate.

---

#### Phase 3: Mode Selection

**Purpose**: Determine which alignment mode to use for each region of the grid.

**Inputs**:
- `GridView` A and B
- User configuration (explicit mode, explicit keys)
- Table metadata from Excel (if available)

**Operations**:
1. If user specified mode, use it
2. If Excel Table metadata exists, extract key columns
3. Otherwise, attempt key inference:
   - Score columns by uniqueness, coverage, stability, data type, header name
   - If qualifying key found, use database mode
   - Otherwise, use spreadsheet mode
4. For mixed sheets: identify table regions and their boundaries

**Outputs**:
- `ResolvedMode::Spreadsheet` OR
- `ResolvedMode::Database { key_columns: Vec<u32> }`
- Optionally: region boundaries for mixed-mode processing

**Complexity**: O(R × K) where K = number of candidate key columns (typically small)

---

#### Phase 4a: Spreadsheet Mode Alignment

**Purpose**: Align rows and columns based on positional identity using the AMR algorithm.

**Inputs**:
- `GridView` A and B
- `HashStats`
- `DimensionOrder`
- Gap configuration (thresholds, limits)

**Operations** (AMR Algorithm):

1. **Anchor Discovery**:
   - Filter to unique rows (frequency = 1 in both grids)
   - Build match pairs (rowA, rowB) for matching hashes
   - Extract LIS to get monotonic anchor chain
   - Partition grids into gaps between anchors

2. **Move Candidate Extraction**:
   - Identify rows matched by hash but NOT in anchor chain
   - These are "out-of-order matches" = potential moves
   - Cluster consecutive matches into block candidates

3. **Move-Aware Gap Filling**:
   For each gap between anchors:
   - Check if move candidates overlap this gap
   - Mark moved rows (don't align them locally)
   - Select gap strategy based on size and composition:
     - Trivial (one side empty)
     - Myers (small gaps)
     - RLE (repetitive content)
     - Bail-out (pathological)
   - Align remaining rows

4. **Move Validation**:
   - Verify each move candidate has source marked "moved away" and destination marked "moved here"
   - Compute similarity for fuzzy moves
   - Accept moves above threshold

5. **Column Alignment**:
   - Apply same pipeline to columns (simpler due to small C)
   - Detect column moves

**Outputs**:
- `RowAlignment`: matched pairs, insertions, deletions
- `ColumnAlignment`: col_mapping, added, removed
- `ValidatedMoves`: list of confirmed row and column moves

**Complexity**: O(R log R) for anchoring, O(R) for gap filling with RLE, O(K³) for move validation where K = block count

---

#### Phase 4b: Database Mode Alignment

**Purpose**: Align rows based on key column values, ignoring row order.

**Inputs**:
- `GridView` A and B
- Key column indices
- Cluster matching configuration

**Operations**:

1. **Key Extraction**:
   - For each row, extract values from key columns
   - Compute key hash

2. **Hash Join**:
   - Build map: key_hash → Vec<row_indices> for each grid
   - For each key, identify rows in A and B with that key

3. **Matching**:
   - 1:1 matches: direct alignment
   - 0:N matches: all additions or all removals
   - N:M matches (duplicate keys): form cluster

4. **Cluster Resolution**:
   - Build similarity matrix for cluster
   - Solve assignment problem with LAPJV
   - Accept matches below cost threshold

**Outputs**:
- `KeyedAlignment`: matched pairs, additions, removals
- Cluster resolution details

**Complexity**: O(R) expected for hash join, O(K³) per cluster for LAPJV

---

#### Phase 5: Cell-Level Diff

**Purpose**: Compare individual cells within aligned row/column pairs.

**Inputs**:
- `GridView` A and B
- Row alignment (from Phase 4a or 4b)
- Column mapping (from Phase 2 or 4a)

**Operations**:

1. For each aligned row pair (rowA, rowB):
   - Get cells from both rows
   - Use column mapping to identify corresponding cells
   - Compare cell values and formulas:
     - For values: type-aware equality (number tolerance, string normalization)
     - For formulas: optional AST-based semantic comparison
   - Emit `CellEdit` for differences

2. Handle edge cases:
   - Cell in A but not in B (at mapped column): value cleared
   - Cell in B but not in A: value added
   - Both empty: no operation

**Outputs**:
- `Vec<CellEdit>`: list of cell-level changes

**Complexity**: O(M) where M = non-empty cells in aligned rows

**Parallelism**: Each row pair is independent; embarrassingly parallel.

---

#### Phase 6: Result Assembly

**Purpose**: Transform raw alignment data into user-friendly diff operations.

**Inputs**:
- Row and column alignments
- Validated moves
- Cell edits

**Operations**:

1. **Coalesce consecutive operations**:
   - Consecutive `RowAdded` → `BlockAddedRows`
   - Consecutive `RowRemoved` → `BlockRemovedRows`
   - Same for columns

2. **Deduplicate moves**:
   - Rows marked as moved should not appear as add+remove
   - Remove redundant operations

3. **Detect rectangular moves** (optional):
   - If row move and column move are correlated (same cells involved)
   - Emit `BlockMovedRect` instead of separate row and column moves

4. **Sort operations**:
   - Apply deterministic sort key: (op_type, row, col)
   - Ensures identical output for identical input regardless of processing order

5. **Build final result**:
   - Collect all operations
   - Attach metadata (similarity scores, confidence levels)

**Outputs**:
- `DiffResult`: final list of `DiffOp` operations

**Complexity**: O(ops × log ops) for sorting

---

### 4.4 Phase Dependencies

The following diagram shows data dependencies between phases:

```
Phase 1 ────────────────────────┐
   │                            │
   ▼                            │
Phase 2 ◄───────────────────────┤
   │                            │
   ▼                            │
Phase 3                         │
   │                            │
   ├──────────┬─────────────────┤
   ▼          ▼                 │
Phase 4a   Phase 4b             │
   │          │                 │
   └────┬─────┘                 │
        ▼                       │
     Phase 5 ◄──────────────────┘
        │
        ▼
     Phase 6
```

**Critical path**: Phases 1 → 2 → 3 → 4 → 5 → 6 must execute sequentially.

**Parallel opportunities within phases**:
- Phase 1: Row hashing, column hashing (parallel across rows/columns)
- Phase 4: Gap filling (parallel across gaps)
- Phase 5: Cell comparison (parallel across row pairs)

---

### 4.5 Early Termination Paths

The pipeline includes fast paths for common cases:

#### Identical Grids

**Detection**: After Phase 1, if all row hashes match in order and all column hashes match in order.

**Action**: Skip Phases 2-5; emit empty result in Phase 6.

**Complexity**: O(M) for hashing only.

#### Completely Different Grids

**Detection**: After Phase 1, compute rough similarity. If < 5% of rows have matching hashes.

**Action**: Skip detailed alignment; emit block removal of A and block addition of B.

**Complexity**: O(M + R) for hashing and similarity check.

#### Single-Mode Sheets

**Detection**: Phase 3 determines a single mode applies to the entire sheet.

**Action**: Skip region segmentation; run Phase 4a or 4b on entire grid.

---

### 4.6 Memory Lifecycle

Each phase allocates and releases memory at specific points:

| Structure | Allocated | Released | Peak Size |
|-----------|-----------|----------|-----------|
| Input Grids | Before Phase 1 | After Phase 6 | O(M) |
| GridViews | Phase 1 | After Phase 5 | O(M) |
| HashStats | Phase 1 | After Phase 4 | O(R + C) |
| Anchor Chain | Phase 4 | After Phase 4 | O(R) |
| Move Candidates | Phase 4 | After Phase 4 | O(K) |
| Gap Alignments | Phase 4 | After Phase 4 | O(R) |
| Cell Edits | Phase 5 | After Phase 6 | O(edits) |
| DiffResult | Phase 6 | Returned to caller | O(ops) |

**Peak memory**: O(M) for GridViews plus O(R + C) for metadata. Never O(R × C).

---

### 4.7 Error Handling Strategy

Each phase can fail; errors are handled as follows:

| Error Type | Detection Point | Handling |
|------------|-----------------|----------|
| Memory exhaustion | Any phase | Return error with partial results if possible |
| Hash collision detected | Phase 4 (mismatched content for same hash) | Fall back to content comparison |
| Timeout exceeded | Phase 4 gap filling | Bail out to block replacement |
| Invalid key columns | Phase 3 | Fall back to spreadsheet mode |
| Corrupt cell data | Phase 1 or 5 | Skip cell, log warning, continue |

The pipeline should never panic or crash; all exceptional conditions produce either:
- A valid (possibly degraded) result, OR
- An explicit error value with diagnostic information

---

### 4.8 Configuration Injection

Each phase accepts configuration that controls its behavior:

```
DiffConfig
├── HashConfig
│   ├── hash_type: Fast64 | Safe128
│   └── include_formulas: bool
├── DimensionConfig
│   └── column_stability_threshold: f64
├── ModeConfig
│   ├── mode: Spreadsheet | Database { keys } | Auto
│   └── key_inference_enabled: bool
├── AlignmentConfig
│   ├── small_gap_threshold: usize
│   ├── repetitive_threshold: f64
│   ├── bailout_similarity_threshold: f64
│   └── max_gap_cost: usize
├── MoveConfig
│   ├── min_block_size: usize
│   ├── fuzzy_threshold: f64
│   └── detect_rectangular: bool
└── OutputConfig
    ├── coalesce_blocks: bool
    └── include_metadata: bool
```

Configuration is threaded through the pipeline; phases extract the sections they need.

---

## 5. Data Structure Design

### 5.1 Design Philosophy

The data structures for the grid diff engine must satisfy competing requirements:

1. **Memory efficiency**: Operate within WASM constraints (1.5GB budget)
2. **Algorithmic efficiency**: Support fast row-oriented and column-oriented access
3. **Sparsity preservation**: Never allocate proportional to R × C
4. **Lifetime clarity**: Clear ownership and explicit allocation/deallocation points
5. **Interoperability**: Work with the existing parsing and serialization infrastructure

The solution is a **layered architecture**:
- **Layer 0**: Sparse Grid IR (existing, canonical, persistent)
- **Layer 1**: Ephemeral GridView (algorithm-friendly, transient)
- **Layer 2**: Alignment results (minimal, output-oriented)

Each layer is optimized for its purpose; data flows upward from Layer 0 during preprocessing and downward to results during assembly.

---

### 5.2 Layer 0: Sparse Grid Intermediate Representation

#### Structure

The input grids use a sparse representation optimized for parsing and storage:

```
Grid
├── cells: HashMap<(row: u32, col: u32), Cell>
├── nrows: u32    // logical row count (used range)
└── ncols: u32    // logical column count (used range)

Cell
├── value: CellValue
└── formula: Option<String>

CellValue
├── Empty
├── Boolean(bool)
├── Number(f64)
├── String(String)
└── Error(ErrorType)
```

#### Properties

**Memory usage**: O(M) where M = number of non-empty cells. A 50,000 × 100 grid with 10% density uses memory for 500,000 cells, not 5,000,000.

**Access patterns**:
- Random access by (row, col): O(1) average via HashMap
- Iteration over all cells: O(M)
- Iteration over cells in a specific row: O(M) worst case (must scan all cells)
- Iteration over cells in a specific column: O(M) worst case

The last two access patterns are problematic for row-oriented algorithms. This motivates Layer 1.

#### Preservation Rationale

The sparse Grid IR is **preserved unchanged** for several reasons:

1. **Parsing investment**: The Excel/Power BI parsers already produce this format
2. **Storage efficiency**: Serialization benefits from sparsity
3. **API stability**: Existing code depends on this interface
4. **Memory pressure**: Duplicating data increases peak memory

Rather than modifying the IR, we build ephemeral views that provide the access patterns algorithms need.

---

### 5.3 Layer 1: Ephemeral GridView

#### Purpose

The GridView provides efficient row-oriented and column-oriented access for diff algorithms without duplicating cell data.

#### Structure

```
GridView<'a>                         // Lifetime tied to source Grid
├── rows: Vec<RowView<'a>>           // One per logical row (0..nrows)
├── row_meta: Vec<RowMeta>           // Metadata for each row
├── col_meta: Vec<ColMeta>           // Metadata for each column
└── source: &'a Grid                 // Reference to original grid

RowView<'a>
└── cells: Vec<(col: u32, cell: &'a Cell)>  // Sorted by column

RowMeta
├── row_idx: u32                     // Redundant but useful
├── hash: RowHash                    // Content fingerprint
├── non_blank_count: u16             // Number of non-empty cells
├── first_non_blank_col: u16         // Leftmost non-empty column
└── is_low_info: bool                // True if likely template/blank

ColMeta
├── col_idx: u32
├── hash: ColHash
├── non_blank_count: u16
└── first_non_blank_row: u16
```

#### Construction Algorithm

Building a GridView from a Grid requires a single pass over the cells:

1. **Allocate row vectors**: Create `rows` with `nrows` empty `RowView` entries
2. **Distribute cells**: For each `(row, col, cell)` in `grid.cells`:
   - Push `(col, &cell)` to `rows[row].cells`
3. **Sort within rows**: For each `RowView`, sort `cells` by column index
4. **Compute row metadata**: For each row:
   - Compute hash from sorted cells
   - Count non-blank cells
   - Record first non-blank column
   - Classify as low-info if appropriate
5. **Compute column metadata**: For each column index 0..ncols:
   - Gather cells from all rows at that column
   - Compute hash, count, first non-blank row

**Complexity**: O(M + R log(M/R)) where the log factor comes from sorting cells within rows.

#### Memory Analysis

| Component | Size | Total for 50K × 100, 10% density |
|-----------|------|----------------------------------|
| RowView.cells entries | 8 bytes each (u32 + pointer) | 500K × 8 = 4 MB |
| RowMeta entries | ~24 bytes each | 50K × 24 = 1.2 MB |
| ColMeta entries | ~20 bytes each | 100 × 20 = 2 KB |
| Vec overhead | ~24 bytes per Vec | 50K × 24 = 1.2 MB |
| **Total** | | **~6.5 MB** |

This overhead is acceptable relative to the Grid itself (which contains the actual cell data).

#### Access Patterns Supported

| Operation | GridView Complexity | Grid-only Complexity |
|-----------|---------------------|----------------------|
| Get all cells in row r | O(cells in row) | O(M) |
| Get hash for row r | O(1) | O(cells in row) |
| Iterate rows in order | O(R) | O(R) |
| Merge-compare two rows | O(cells in row) | O(M) |

The GridView transforms the expensive O(M) row access into O(cells-in-row), enabling efficient alignment algorithms.

---

### 5.4 Dual Hash System

#### Hash Types

The engine supports two hash widths, selectable at configuration time:

```
Hash64
└── value: u64

Hash128
└── value: u128
```

Both implement the same trait interface, allowing generic algorithms:

```
Trait RowHasher
- type Hash: Eq + Hash + Copy + Ord + Default
- fn hash_row(&self, cells: &[(u32, &Cell)]) -> Self::Hash
- fn hash_column(&self, cells: &[(u32, &Cell)]) -> Self::Hash
- fn combine(&self, a: Self::Hash, b: Self::Hash) -> Self::Hash
```

`combine` is a commutative, order-independent reduction over per-cell contributions, ensuring deterministic hashes even when iterating sparsely stored cells without sorting.

#### Hash64 Implementation (XXHash64)

**Properties**:
- Output: 64 bits
- Speed: ~15 GB/s
- Collision probability at 50K rows: ~0.00006% (birthday paradox)

**Hash computation**:
1. For each non-empty cell, compute a per-cell contribution with XXHash64 over `(position, type_tag, value_bytes, optional_formula_bytes)`, where `position` is the column index for row hashes and the row index for column hashes.
2. Combine contributions using the commutative reducer `combine` (mix + wrapping add). Because position is encoded inside each contribution, the final hash remains position-sensitive even though reduction is order-independent.
3. Return the accumulator as the row/column fingerprint. No sorting of sparse cells is required; streaming over a `HashMap` produces identical results across platforms.

**Implementation Note (RS1 milestone)**: The sparse `Grid` layer uses the commutative reducer so hashing can stream over `HashMap`-backed cells without cloning or sorting. Higher-level `GridView` structures may still materialize sorted row/column views for other algorithms, but the fingerprint semantics are defined in terms of the commutative reduction, not iteration order.

#### Hash128 Implementation (BLAKE3 truncated)

**Properties**:
- Output: 128 bits (truncated from BLAKE3's 256)
- Speed: ~10 GB/s
- Collision probability: Negligible at any practical scale

**Hash computation**: Same as Hash64 but using BLAKE3 hasher.

#### Selection Guidance

| Use Case | Recommended | Rationale |
|----------|-------------|-----------|
| Standard diff operations | Hash64 | 10-15% faster, sufficient collision resistance |
| Audit/compliance | Hash128 | Extra safety margin for critical applications |
| Order-independent hashing | Hash128 | Supports product + XOR combinations safely |
| WASM performance-critical | Hash64 | Minimize computation time |

The choice is made once per diff operation and affects all row/column hashing.

---

### 5.5 Row and Column Metadata

#### RowMeta Fields

| Field | Type | Purpose |
|-------|------|---------|
| `row_idx` | u32 | Original row index (for result assembly) |
| `hash` | RowHash | Content fingerprint for matching |
| `non_blank_count` | u16 | Density indicator; 0 = blank row |
| `first_non_blank_col` | u16 | Leftmost content; useful for sparse alignment |
| `is_low_info` | bool | True if row contains only blanks or trivial content |

#### Classification Rules

A row is classified as **low-information** if:
- `non_blank_count == 0` (completely blank), OR
- `non_blank_count == 1` AND the single cell contains only whitespace or a trivial constant

Low-information rows are treated specially:
- Excluded from anchor discovery (high frequency makes them useless as anchors)
- Compressed via RLE during gap alignment
- Not used for similarity computation

#### ColMeta Fields

| Field | Type | Purpose |
|-------|------|---------|
| `col_idx` | u32 | Original column index |
| `hash` | ColHash | Content fingerprint for column matching |
| `non_blank_count` | u16 | How many rows have content in this column |
| `first_non_blank_row` | u16 | Topmost content |

Column metadata is primarily used for column alignment and dimension decision.

---

### 5.6 Hash Statistics Structure

After computing hashes, frequency analysis enables classification:

```
HashStats<H: Hash>
├── freq_a: HashMap<H, u32>          // Count of each hash in Grid A
├── freq_b: HashMap<H, u32>          // Count of each hash in Grid B
└── hash_to_positions_b: HashMap<H, Vec<u32>>  // Positions of each hash in B
```

#### Derived Classifications

```
fn is_unique(&self, hash: H) -> bool:
    freq_a[hash] == 1 AND freq_b[hash] == 1

fn is_rare(&self, hash: H, threshold: u32) -> bool:
    freq_a[hash] <= threshold AND freq_b[hash] <= threshold
    AND freq_a[hash] > 0 AND freq_b[hash] > 0

fn is_common(&self, hash: H, threshold: u32) -> bool:
    freq_a[hash] > threshold OR freq_b[hash] > threshold

fn appears_in_both(&self, hash: H) -> bool:
    freq_a[hash] > 0 AND freq_b[hash] > 0
```

Default threshold for rare/common boundary: 3-8 occurrences.

---

### 5.7 String Interning System

#### Problem

Spreadsheets often contain repeated string values:
- Column headers appearing in every row (in some export formats)
- Category values (status codes, department names)
- Empty string placeholders
- Date strings formatted identically

Without deduplication, memory scales with total string characters rather than unique strings.

#### Solution: String Pool

```
StringPool
├── strings: Vec<String>             // Unique strings, indexed by ID
├── lookup: HashMap<String, StringId> // Deduplication lookup
└── next_id: StringId                // Next available ID

StringId = u32                        // Compact reference
```

#### Interning Process

When parsing a cell with string value:

1. Check if string exists in `lookup`
2. If yes: return existing `StringId`
3. If no:
   - Add string to `strings` vector
   - Insert into `lookup`
   - Return new `StringId`

#### Memory Savings

For a grid with:
- 500,000 string cells
- Average string length: 20 characters
- 10,000 unique strings

**Without interning**: 500,000 × 20 = 10 MB in strings
**With interning**: 10,000 × 20 + 500,000 × 4 = 2.2 MB

Savings: ~78% for this example. Real-world savings range from 30-80%.

#### Integration with Cell Structure

With interning, the Cell structure becomes:

```
Cell (interned variant)
├── value: CellValueInterned
└── formula_id: Option<StringId>

CellValueInterned
├── Empty
├── Boolean(bool)
├── Number(f64)
├── String(StringId)          // 4 bytes instead of 24+
└── Error(ErrorType)
```

The pool must be shared across both grids being compared, enabling O(1) string equality via ID comparison.

---

### 5.8 Alignment Result Structures

#### Row Alignment Output

```
RowAlignment
├── matched: Vec<(row_a: u32, row_b: u32)>  // Aligned row pairs
├── insertions: Vec<u32>                     // Rows only in B
├── deletions: Vec<u32>                      // Rows only in A
├── moves_from: Vec<u32>                     // Rows that moved (source)
└── moves_to: Vec<u32>                       // Rows that moved (dest)
```

#### Column Alignment Output

```
ColumnAlignment
├── col_mapping: Vec<Option<u32>>           // colA -> colB mapping
├── col_added: Vec<u32>                      // Columns only in B
├── col_removed: Vec<u32>                    // Columns only in A
└── col_moves: Vec<ColumnMove>               // Detected column moves

ColumnMove
├── source: Range<u32>
└── dest: Range<u32>
```

#### Validated Move Output

```
ValidatedMove
├── source_rows: Range<u32>                  // Rows in A
├── dest_rows: Range<u32>                    // Rows in B
├── similarity: f64                          // 1.0 = exact, <1.0 = fuzzy
└── is_fuzzy: bool
```

#### Cell Edit Output

```
CellEdit
├── row_a: u32
├── col_a: u32
├── row_b: u32
├── col_b: u32
├── old_value: CellSnapshot
├── new_value: CellSnapshot
└── formula_change: Option<FormulaChange>

CellSnapshot
├── value: CellValue
└── formula: Option<String>

FormulaChange
├── old_formula: String
├── new_formula: String
├── semantic_equal: bool                     // After canonicalization
└── edit_ops: Vec<FormulaEditOp>             // If AST diff computed
```

---

### 5.9 Final DiffOp Schema

The output schema represents all possible diff operations:

```
DiffOp
├── RowAdded { row_b: u32 }
├── RowRemoved { row_a: u32 }
├── ColumnAdded { col_b: u32 }
├── ColumnRemoved { col_a: u32 }
├── CellEdited {
│     row_a: u32, col_a: u32,
│     row_b: u32, col_b: u32,
│     old: CellSnapshot,
│     new: CellSnapshot
│   }
├── BlockMovedRows {
│     source: Range<u32>,
│     dest: Range<u32>,
│     is_fuzzy: bool
│   }
├── BlockMovedColumns {
│     source: Range<u32>,
│     dest: Range<u32>
│   }
└── BlockMovedRect {                         // Optional: correlated row+col move
      source_rows: Range<u32>,
      source_cols: Range<u32>,
      dest_rows: Range<u32>,
      dest_cols: Range<u32>
    }
```

#### Sort Key for Determinism

Each DiffOp has a sort key ensuring consistent output order:

```
fn sort_key(op: &DiffOp) -> (u8, u32, u32, u32):
    match op:
        RowRemoved { row_a } => (0, row_a, 0, 0)
        RowAdded { row_b } => (1, row_b, 0, 0)
        ColumnRemoved { col_a } => (2, col_a, 0, 0)
        ColumnAdded { col_b } => (3, col_b, 0, 0)
        BlockMovedRows { source, .. } => (4, source.start, source.end, 0)
        BlockMovedColumns { source, .. } => (5, source.start, source.end, 0)
        BlockMovedRect { source_rows, source_cols, .. } => 
            (6, source_rows.start, source_cols.start, 0)
        CellEdited { row_a, col_a, .. } => (7, row_a, col_a, 0)
```

Sorting by this key guarantees:
- Structural operations before cell edits
- Removals before additions (for clearer presentation)
- Position-ordered within each operation type

---

### 5.10 Memory Budget Enforcement

To prevent runaway memory usage, the engine tracks allocations:

```
MemoryTracker
├── current_bytes: AtomicUsize
├── peak_bytes: AtomicUsize
├── budget_bytes: usize
└── allocation_log: Vec<AllocationRecord>    // Debug builds only

AllocationRecord
├── phase: Phase
├── structure: &'static str
├── bytes: usize
└── timestamp: Instant
```

#### Usage Pattern

Before allocating a large structure:

1. Estimate required bytes
2. Call `tracker.try_allocate(bytes)`
3. If returns `Ok(guard)`: proceed with allocation
4. If returns `Err(MemoryExhausted)`: trigger fallback behavior
5. When structure is dropped, guard decrements counter

#### Fallback Behaviors

When memory budget would be exceeded:

| Phase | Fallback |
|-------|----------|
| Preprocessing | Switch to streaming mode |
| Gap Filling | Bail out, treat as block replacement |
| Move Detection | Skip move detection for this region |
| Cell Diff | Process rows in smaller batches |

---

# Part III: Preprocessing

## 6. Fingerprinting & Hashing

### 6.1 Purpose of Fingerprinting

Fingerprinting transforms each row and column into a compact, fixed-size digest that enables:

1. **Fast equality testing**: Compare 8-16 bytes instead of scanning all cells
2. **Anchor discovery**: Identify unique and rare rows for Patience Diff
3. **Move detection**: Match blocks by comparing hash sequences
4. **Early termination**: Detect identical or completely different grids quickly

A well-designed fingerprinting strategy is foundational to the entire diff algorithm. Poor fingerprinting leads to false matches (collisions), missed matches (over-sensitive hashing), or performance problems (slow hash computation).

---

### 6.2 Cell Normalization

Before hashing, cell values must be normalized to ensure semantically equivalent values produce identical hashes.

#### Numeric Normalization

**Problem**: Floating-point representation can vary. Excel stores numbers as IEEE 754 doubles, but parsing/serialization may introduce tiny differences.

**Normalization rules**:
1. Round to 15 significant digits (Excel's display precision)
2. Convert -0.0 to 0.0
3. Canonicalize NaN to a single representation
4. Represent as raw u64 bits after normalization

**Example**:
- `1.0000000000000001` and `1.0` → both hash as `1.0`
- `-0.0` → hashes as `0.0`

#### String Normalization

**Problem**: Strings may have leading/trailing whitespace, inconsistent case (for case-insensitive comparison), or Unicode normalization differences.

**Normalization rules** (default):
1. Trim leading and trailing whitespace
2. Preserve case (case-sensitive comparison by default)
3. Apply Unicode NFC normalization
4. Empty string after trimming → treated as Empty cell

**Configurable options**:
- Case-insensitive mode: lowercase before hashing
- Whitespace-insensitive mode: collapse internal whitespace

#### Boolean Normalization

Boolean values are simple: `true` and `false` map to fixed byte sequences.

- `true` → `0x01`
- `false` → `0x00`

#### Error Normalization

Excel errors (#VALUE!, #REF!, #DIV/0!, etc.) are normalized by error type:

| Error | Normalized Byte |
|-------|-----------------|
| #NULL! | 0x01 |
| #DIV/0! | 0x02 |
| #VALUE! | 0x03 |
| #REF! | 0x04 |
| #NAME? | 0x05 |
| #NUM! | 0x06 |
| #N/A | 0x07 |
| #GETTING_DATA | 0x08 |
| #SPILL! | 0x09 |
| #CALC! | 0x0A |

#### Empty Cell Handling

Empty cells are **not included** in the hash computation. This ensures:
- Sparse rows with different empty cell patterns hash identically if their non-empty cells match
- Adding empty cells to a row doesn't change its hash

---

### 6.3 Row Hash Computation

#### Algorithm Overview

For each row, compute a hash over the sorted sequence of non-empty cells. The algorithm processes cells in column order, feeding each cell's data into the hasher.

**Process:**
1. Sort the row's non-empty cells by column index (ensures deterministic ordering)
2. Initialize the hasher
3. For each non-empty cell, feed into the hasher:
   - The column index (4 bytes)
   - A type discriminant byte (to distinguish types with similar representations)
   - The normalized cell value (variable length based on type)
   - If configured, the normalized formula string
4. Finalize and return the hash

**Value Encoding by Type:**
- **Boolean**: 1 byte (0 or 1)
- **Number**: 8 bytes (normalized f64 bit representation)
- **String**: Length prefix followed by UTF-8 bytes
- **Error**: 1 byte error code

Empty cells are skipped entirely—they do not contribute to the hash.

#### Why Column Position is Included

The hash includes column indices to distinguish rows with the same values in different positions:

- Row with `A1="foo", B1="bar"` → different hash than
- Row with `A1="bar", B1="foo"`

Without column positions, these rows would collide, causing incorrect alignment.

#### Why Type Tag is Included

The type discriminant distinguishes values that might have the same byte representation:

- String `"1"` → different hash than Number `1.0`
- String `"true"` → different hash than Boolean `true`

This prevents false matches between semantically different cells.

#### Order Dependency

Row hashes are **order-dependent** (column order matters within a row) but can be computed by iterating cells in any order, provided they are sorted before hashing. The sort ensures deterministic hash computation regardless of HashMap iteration order.

---

### 6.4 Column Hash Computation

Column hashes are computed analogously to row hashes, but iterating vertically instead of horizontally.

**Process:**
1. Collect all non-empty cells in the specified column across all rows
2. These cells are naturally ordered by row index
3. For each non-empty cell, feed into the hasher:
   - The row index (4 bytes)
   - A type discriminant byte
   - The normalized cell value
4. Finalize and return the hash

The key difference from row hashing is that **row position** (not column position) is included, since we're identifying column content across rows.

#### When Column Hashes Are Computed

Column hashes may be computed at different times:

1. **Phase 1 (unconditional)**: For dimension decision
2. **After row alignment (conditional)**: For column alignment, using only matched rows

Computing column hashes over matched rows only produces more stable results when row structure has changed.

---

### 6.5 Formula Normalization

When formulas are included in hashing, they must be normalized to avoid false differences.

#### Basic Normalization

1. **Whitespace removal**: Strip all whitespace not inside string literals
2. **Case normalization**: Uppercase function names, lowercase sheet names
3. **Reference format**: Normalize to A1 style (convert R1C1 if present)

#### Advanced Normalization (Optional)

For semantic formula comparison (Section 20), more aggressive normalization:

1. **Commutative reordering**: Sort operands of + and * by canonical key
2. **Constant folding**: `=1+1` → `=2`
3. **Reference normalization**: Convert relative to absolute based on cell position

Advanced normalization is expensive and typically reserved for cell-level diff, not row hashing.

---

### 6.6 Formula Inclusion Decision

Whether to include formulas in row hashes is configurable:

| Setting | Behavior | Use Case |
|---------|----------|----------|
| `include_formulas: true` | Hash includes formula text | Compare workbooks where formulas matter |
| `include_formulas: false` | Hash includes only values | Compare data exports, computed results |

**Default**: `true` (formulas matter)

When `include_formulas: false`, two cells with the same value but different formulas (`=A1+B1` vs `=SUM(A1:B1)` both evaluating to 10) will match.

---

### 6.7 Parallel Hash Computation

Row and column hashing are embarrassingly parallel—each row (or column) can be processed independently with no shared mutable state.

#### Row Hashing Parallelization

Rows can be partitioned across available threads, with each thread processing its subset and computing hashes independently. Results are collected into a vector indexed by row position.

**Properties:**
- No synchronization needed during hash computation
- Near-linear speedup (~3.5x on 4 cores)
- Cache-friendly: each thread processes contiguous row data

#### Column Hashing Parallelization

Columns can similarly be processed in parallel, though the access pattern is less cache-friendly since computing each column hash requires scanning all rows.

**Properties:**
- Each column is independent
- Lower speedup (~2-3x on 4 cores) due to memory access patterns
- Each column scan touches data from all rows (cache-unfriendly)

#### Speedup Expectations

On a 4-core machine:
- Row hashing: ~3.5x speedup (near-linear, cache-friendly)
- Column hashing: ~2-3x speedup (memory-bound, cache-unfriendly)

---

### 6.8 Hash Collision Analysis

#### Birthday Paradox Bounds

For a hash of width w bits, the probability of at least one collision among n items is approximately:

```
P(collision) ≈ 1 - e^(-n² / 2^(w+1))
```

For n = 50,000 rows:

| Hash Width | Collision Probability |
|------------|----------------------|
| 64 bits | ~0.00007% (1 in 1.4 million) |
| 128 bits | ~10^(-29) (negligible) |

#### Collision Consequences

If two different rows produce the same hash:

1. **False anchor**: Both rows might be considered "unique" when neither is
2. **False match**: Rows might be aligned incorrectly
3. **Missed detection**: A change might go unreported

#### Collision Mitigation

1. **Use 128-bit hashes** for safety-critical applications
2. **Verify matches**: After hash-based alignment, verify by content comparison
3. **Secondary hash**: Store a second independent hash for verification

The engine uses **trust but verify**: hash-based matching for speed, with optional content verification for detected matches.

---

### 6.9 Hash Stability Requirements

Hashes must be **deterministic** across:

1. **Runs**: Same input always produces same hash
2. **Platforms**: Hash is identical on x86, ARM, WASM
3. **Rust versions**: Stable hash algorithm, not default Hasher

**Non-requirements**:
- Cryptographic security (we're not defending against attackers)
- Collision resistance beyond birthday bound (acceptable for diff)

To ensure stability:
- Use a fixed-seed hasher (not SipHash with random seed)
- Use a well-specified algorithm (XXHash64, BLAKE3)
- Define byte order explicitly (little-endian)

---

### 6.10 Hash Caching Strategy

Hashes are computed once and reused:

| Hash Type | Computed In | Stored In | Used By |
|-----------|-------------|-----------|---------|
| Row hash | Phase 1 | RowMeta | Anchoring, gap filling, move detection |
| Column hash | Phase 1 or 2 | ColMeta | Dimension decision, column alignment |
| Block hash | Phase 4 | Local variable | Move detection |

Block hashes (hash of a sequence of row hashes) are computed on-demand during move detection and not cached, as the set of candidate blocks is small.

---

### 6.11 Incremental Hashing (Future Optimization)

For scenarios where grids are compared repeatedly with small changes (e.g., live editing), incremental hashing could reduce computation:

**Approach**:
1. Cache row hashes from previous comparison
2. Track which rows changed (by cell modification timestamp)
3. Recompute only changed row hashes

**Complexity**:
- Requires change tracking in Grid structure
- Invalidation on any cell change in a row
- Not implemented in initial version

This is documented for future consideration; the initial implementation recomputes all hashes each comparison.

---

### 6.12 Fingerprinting Summary

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Hash algorithm | XXHash64 (default) or BLAKE3-128 | Speed vs. safety trade-off |
| Column position | Included in row hash | Distinguish value permutations |
| Type discriminant | Included | Distinguish `"1"` from `1` |
| Empty cells | Excluded | Sparse rows with different empties should match |
| Formulas | Configurable, default included | User choice based on use case |
| Normalization | Numbers, strings, formulas | Avoid false differences |
| Parallelism | Rows parallel, columns parallel | Maximize throughput |
| Determinism | Required | Reproducible results |

---

## 7. Frequency Analysis & Classification

### 7.1 Purpose of Frequency Analysis

After computing row hashes, frequency analysis categorizes rows by how often their hash appears across both grids. This classification drives critical algorithmic decisions:

| Classification | Anchoring | Gap Strategy | Move Detection |
|----------------|-----------|--------------|----------------|
| Unique | Primary anchors | Not needed (anchored) | Exact match |
| Rare | Secondary anchors | Myers diff | Fuzzy match candidate |
| Common | Excluded | RLE compression | Excluded |
| Low-information | Excluded | Counting heuristic | Excluded |

Proper classification is the key to achieving O(N log N) performance on repetitive data: by excluding common rows from expensive operations, we avoid the quadratic blowup that afflicts naive algorithms.

---

### 7.2 Building Frequency Tables

#### Process

Building frequency tables requires two passes over the row metadata:

1. **Count occurrences**: For each grid, iterate over all row hashes and count how many times each hash appears. Store in a hash-to-count map.

2. **Build position index**: For Grid B specifically, build a reverse index mapping each hash to the list of row indices where it appears. This enables efficient lookup during anchor discovery.

**Output Structure:**
- `freq_a`: Map from hash to occurrence count in Grid A
- `freq_b`: Map from hash to occurrence count in Grid B
- `hash_to_positions_b`: Map from hash to list of row indices in Grid B

**Complexity**: O(R_A + R_B) where R = number of rows

**Memory**: O(U) where U = number of unique hashes (U ≤ R_A + R_B)

---

### 7.3 Row Classification Categories

#### Unique Rows

**Definition**: A hash is unique if it appears exactly once in Grid A AND exactly once in Grid B.

**Classification rule**: `freq_a[hash] == 1 AND freq_b[hash] == 1`

**Properties**:
- Unique rows are perfect anchors: unambiguous 1:1 correspondence
- Form the foundation of Patience Diff
- Typically represent meaningful content (headers, distinct data rows)

**Expected count**: In typical spreadsheets, 30-80% of rows are unique.

#### Rare Rows

**Definition**: A hash is rare if it appears in both grids but with low frequency (≤ threshold in each), and is not unique.

**Classification rule**: Hash appears >0 times in both grids, ≤threshold in each, but not exactly 1 in both.

**Default threshold**: 4 (configurable)

**Properties**:
- Can be used as secondary anchors after unique rows
- Require disambiguation (multiple candidates in each grid)
- More ambiguity but still useful for structural alignment

#### Common Rows

**Definition**: A hash is common if it appears frequently (>threshold) in at least one grid.

**Classification rule**: `freq_a[hash] > threshold OR freq_b[hash] > threshold`

**Properties**:
- Excluded from anchor discovery (too ambiguous)
- Aligned via counting or RLE, not sequence alignment
- Examples: blank rows, template rows, repeated headers

#### Unmatched Rows

**Definition**: A hash appears in only one grid.

**Classification rules**:
- Only in A: `freq_a[hash] > 0 AND freq_b[hash] == 0`
- Only in B: `freq_a[hash] == 0 AND freq_b[hash] > 0`

**Properties**:
- Cannot participate in anchoring
- Will be classified as additions or deletions
- May participate in move detection (if similar hash found)

---

### 7.4 Low-Information Row Detection

Beyond hash frequency, we classify rows by their **information content**.

#### Definition

A row is low-information if:
1. `non_blank_count == 0` (completely blank), OR
2. `non_blank_count == 1` AND the cell contains trivial content, OR
3. `non_blank_count <= 2` AND all cells are short trivial values

#### Trivial Content Examples

| Content | Trivial? | Rationale |
|---------|----------|-----------|
| Empty string | Yes | No semantic content |
| Single space | Yes | Likely formatting artifact |
| "0" or "0.0" | Yes | Common placeholder |
| Single digit | Configurable | May be meaningful |
| "N/A", "TBD" | Configurable | Common placeholders |
| "-" or "–" | Yes | Common empty indicator |

#### Classification Process

A row is classified as low-information through a two-step check:

1. **Cell count check**: If the row has zero non-blank cells, it's definitely low-info. If it exceeds the configured maximum (default: 2), it's definitely not low-info.

2. **Content check**: For rows with 1-2 non-blank cells, examine each cell's content:
   - Empty cells: trivial
   - Booleans: NOT trivial (meaningful data)
   - Numbers: trivial only if zero and configured as such
   - Strings: trivial if they match known placeholder patterns (whitespace, "-", "N/A", etc.)
   - Errors: NOT trivial (meaningful data)

A row is low-information only if ALL its non-blank cells contain trivial content.

#### Why Detect Low-Information Rows

Low-information rows:
1. **Dominate repetitive grids**: The "99% blank" scenario
2. **Have high hash frequency**: Many blanks → same hash
3. **Provide no structural signal**: Aligning blank to blank is arbitrary
4. **Can be aligned by counting**: No need for expensive sequence alignment

By detecting these rows, the algorithm can apply specialized strategies that avoid quadratic behavior.

---

### 7.5 Token Interning

To enable efficient sequence operations, row hashes are interned into compact integer tokens.

#### Token Structure

```
TokenMap
├── hash_to_token: HashMap<Hash, Token>   // Deduplicate
├── token_to_hash: Vec<Hash>              // Reverse lookup
└── next_token: Token                     // Allocation counter

Token = u32                                // Compact representation
```

#### Interning Process

Tokenization assigns compact integer IDs to row hashes:

1. **Process both grids**: Iterate through all row hashes in Grid A, then Grid B
2. **Check for existing token**: For each hash, look up if it's already been interned
3. **Assign or reuse token**: If the hash is new, assign the next available token ID and record the bidirectional mapping. If it exists, reuse the existing token.
4. **Build token sequences**: The result is two vectors of tokens (one per grid) plus a shared token map for bidirectional lookup.

The shared token map enables efficient comparison: if two rows from different grids have the same token, they have the same hash (and thus the same content).

#### Benefits of Tokenization

| Operation | With Hashes | With Tokens |
|-----------|-------------|-------------|
| Equality check | 8-16 bytes | 4 bytes |
| HashMap lookup | Hash the hash | Direct index |
| Memory per row | 8-16 bytes | 4 bytes |
| Cache efficiency | Lower | Higher |

For a 50K row grid, tokenization saves ~400KB of memory and improves cache locality for sequence operations.

---

### 7.6 Run-Length Encoding

After tokenization, consecutive rows with the same token are compressed into runs.

#### Run Structure

```
Run
├── token: Token          // The repeated row pattern
├── start_idx: u32        // First row index in this run
├── length: u32           // Number of consecutive rows
└── is_low_info: bool     // All rows in run are low-information
```

#### RLE Construction

Run construction is a single-pass algorithm over the token sequence:

1. **Initialize**: Start with the first token as the current run
2. **Extend or close**: For each subsequent token:
   - If it matches the current run's token, extend the run (increment length, update low-info flag)
   - If it differs, close the current run and start a new one
3. **Track low-info**: A run is marked low-info only if ALL rows in it are low-information
4. **Finalize**: Emit the last run when the sequence ends

**Output per run:**
- Token (the repeated pattern)
- Start index (first row of the run)
- Length (number of consecutive rows)
- Low-info flag (whether all rows are trivial)

**Complexity**: O(R) single pass

#### Compression Ratio Examples

| Scenario | Rows | Runs | Compression |
|----------|------|------|-------------|
| All unique | 50,000 | 50,000 | 1:1 (no benefit) |
| 10% repeated | 50,000 | ~45,000 | 1.1:1 |
| 50% blank | 50,000 | ~1,000 | 50:1 |
| 99% blank | 50,000 | ~50 | 1000:1 |

The compression ratio directly impacts alignment performance: aligning 50 runs is trivial; aligning 50,000 rows could be quadratic.

---

### 7.7 Classification Output Structure

The complete classification output:

```
ClassificationResult
├── tokens_a: Vec<Token>              // Row tokens for Grid A
├── tokens_b: Vec<Token>              // Row tokens for Grid B
├── token_map: TokenMap               // Bidirectional mapping
├── runs_a: Vec<Run>                  // RLE for Grid A
├── runs_b: Vec<Run>                  // RLE for Grid B
├── hash_stats: HashStats             // Frequency tables
├── unique_count: usize               // How many unique hashes
├── common_count: usize               // How many common hashes
└── low_info_row_count: (usize, usize)  // Per grid

// Derived classifications (methods, not stored)
impl ClassificationResult:
    fn is_unique(&self, token: Token) -> bool
    fn is_rare(&self, token: Token) -> bool
    fn is_common(&self, token: Token) -> bool
    fn is_low_info(&self, token: Token) -> bool
    fn get_positions_in_b(&self, token: Token) -> &[u32]
```

---

### 7.8 Early Termination Checks

Classification enables early termination for special cases:

#### Identical Grids Check

**Condition**: Both grids have the same number of rows AND all tokens match in order (token sequence equality).

**Action**: Skip all remaining phases; return empty diff.

**Cost**: O(R) for the sequence comparison, but this is already computed during tokenization.

#### Completely Different Check

**Condition**: Compute the fraction of unique tokens that appear in both grids. If this "shared token fraction" is below a threshold, the grids are effectively different documents.

**Default threshold**: 0.02 (less than 2% shared content)

**Action**: Skip alignment entirely; emit block removal of all Grid A content and block addition of all Grid B content.

**Rationale**: Attempting detailed alignment on unrelated content wastes computation and produces meaningless results.

#### Heavily Repetitive Check

**Condition**: Compute the RLE compression ratio (rows ÷ runs) for each grid. If either exceeds a threshold, the grid is dominated by repetitive content.

**Default threshold**: 10.0 (10:1 compression ratio)

**Action**: Force RLE-based alignment for all gaps to avoid quadratic behavior on repetitive content.

**Rationale**: A 50,000-row grid that compresses to 50 runs can be aligned in O(50²) instead of O(50,000²).

---

### 7.9 Statistics and Diagnostics

Classification produces statistics useful for diagnostics and tuning:

```
ClassificationStats
├── total_rows_a: usize
├── total_rows_b: usize
├── unique_tokens: usize
├── rare_tokens: usize
├── common_tokens: usize
├── low_info_rows_a: usize
├── low_info_rows_b: usize
├── runs_a: usize
├── runs_b: usize
├── compression_ratio_a: f64
├── compression_ratio_b: f64
├── shared_token_fraction: f64
└── predicted_complexity: Complexity  // Estimated algorithm tier

enum Complexity:
    Trivial,      // Identical or nearly so
    Linear,       // Few edits, many anchors
    NLogN,        // Typical case
    Quadratic,    // Worst case gap filling
    BailOut       // Too different, skip alignment
```

These statistics can be logged for performance analysis and can inform adaptive algorithm selection.

---

### 7.10 Classification Thresholds

All classification thresholds are configurable:

| Threshold | Default | Range | Effect |
|-----------|---------|-------|--------|
| `rare_max_frequency` | 4 | 2-10 | Max frequency to consider "rare" |
| `common_min_frequency` | 5 | 3-20 | Min frequency to consider "common" |
| `low_info_max_cells` | 2 | 0-5 | Max non-empty cells for low-info |
| `zero_is_trivial` | true | bool | Treat 0 as trivial content |
| `completely_different_threshold` | 0.02 | 0-0.1 | Similarity below = completely different |
| `heavily_repetitive_ratio` | 10.0 | 5-50 | Compression ratio for RLE path |

#### Threshold Tuning Guidance

- **Conservative (fewer false positives)**: Higher `rare_max_frequency`, lower `low_info_max_cells`
- **Aggressive (faster, more bail-outs)**: Lower thresholds, faster early termination
- **Data-dependent**: Financial data may have many zeros; set `zero_is_trivial: false`

---

### 7.11 Classification Summary

| Input | Output | Used By |
|-------|--------|---------|
| Row hashes | Token sequences | Sequence alignment |
| Token frequencies | Unique/rare/common classification | Anchor discovery |
| Row metadata | Low-info classification | Gap strategy |
| Token sequences | Run sequences (RLE) | Repetitive gap alignment |
| Frequency tables | Hash positions | Anchor candidate lookup |

Classification transforms raw hash data into actionable classifications that drive efficient algorithm selection throughout the pipeline.

---

## 8. Adaptive Dimension Ordering

### 8.1 The Dimension Ordering Problem

When comparing two grids, both rows and columns may have changed. The question arises: should we align rows first and then columns, or columns first and then rows?

**The problem with row-first (naive approach)**:

When a column is inserted in Grid B, every row's hash changes (because column positions are part of the hash). Consider:

Grid A, Row 5: `[A:10, B:20, C:30]` → Hash: X
Grid B, Row 5: `[A:10, B:NEW, C:20, D:30]` → Hash: Y (column B inserted)

Even though the data in columns A, C, D is identical, the row hashes differ because column positions shifted. Row alignment will fail to match these rows, producing false "row changed" results.

**The insight**:

If we detect that columns have changed and establish a column mapping first, we can recompute row hashes using only matched columns, producing stable hashes that align correctly.

---

### 8.2 When Each Ordering Applies

| Scenario | Preferred Order | Rationale |
|----------|-----------------|-----------|
| Columns unchanged | Row-first | Simpler, no recomputation needed |
| Few column changes | Row-first | Column changes handled in column alignment |
| Many column changes | Column-first | Prevent false row mismatches |
| Column inserted/deleted in middle | Column-first | Critical for correct row alignment |
| Only column moves | Either | Move detection handles both |

**Default strategy**: Row-first (simpler, works for 90%+ of cases), with automatic fallback to column-first when column instability is detected.

---

### 8.3 Column Stability Check

Before choosing dimension order, perform a cheap check to estimate column stability.

#### Process

1. **Collect column hashes**: Build a set of column hashes for each grid
2. **Compute Jaccard similarity**: Calculate `|intersection| / |union|` of the two hash sets
3. **Compare to threshold**: If similarity ≥ threshold, columns are stable

**Output:**
- `is_stable`: Boolean indicating whether row-first is safe
- `similarity`: The computed Jaccard similarity (0.0 to 1.0)
- `matched_count`: Number of column hashes appearing in both grids
- `total_unique`: Total unique column hashes across both grids

**Edge case**: If both grids have zero columns, similarity is 1.0 (trivially stable).

**Complexity**: O(C) where C = number of columns

#### Interpreting Similarity

| Similarity | Interpretation | Action |
|------------|---------------|--------|
| 1.0 | All columns match | Row-first |
| 0.95 - 1.0 | Minor column changes | Row-first |
| 0.80 - 0.95 | Moderate column changes | Threshold-dependent |
| < 0.80 | Significant column changes | Column-first |

#### Default Threshold

**Default**: 0.90

This means: if 90%+ of columns are unchanged, use row-first. Otherwise, switch to column-first.

The threshold balances:
- **Higher (e.g., 0.95)**: More conservative, rarely switches, simpler
- **Lower (e.g., 0.80)**: More aggressive, catches more column change cases

---

### 8.4 Dimension Order Decision

The decision process has two levels:

#### Fast Path (O(C))

First, check if columns are identical in order:
- Same column count in both grids
- All column hashes match pairwise in position order

If this fast check passes, use row-first ordering immediately. This catches the common case (no column changes) without computing full column alignment.

#### Full Stability Check

If the fast path fails, run the full column stability check (Section 8.3):
- If similarity ≥ threshold: use row-first ordering
- If similarity < threshold: use column-first ordering

When column-first is selected, perform preliminary column alignment immediately to generate the column mapping needed for row hash recomputation.

**Output:**
- `DimensionOrder::RowFirst`: Proceed with standard row alignment
- `DimensionOrder::ColumnFirst { col_mapping }`: Row hashes must be recomputed using only matched columns

---

### 8.5 Column Alignment for Recomputation

When column-first is selected, preliminary column alignment establishes which columns correspond between grids.

#### Process

1. **Tokenize columns**: Convert column hashes to compact tokens
2. **Run sequence alignment**: Apply Myers diff to the column token sequences (O(C²) is acceptable for small column counts)
3. **Build column mapping**: From the edit script, construct a mapping `colA → Option<colB>`:
   - Match operations: `mapping[colA] = Some(colB)`
   - Delete operations: `mapping[colA] = None` (column removed)
   - Insert operations: Skip (column added in B, no source in A)

**Output**: A vector of length `ncols_a` where each element is either `Some(colB)` indicating the corresponding column in B, or `None` indicating the column was removed.

**Note**: This is a preliminary alignment. Full column alignment (including move detection) happens later in Phase 4. The purpose here is solely to enable stable row hash recomputation.

---

### 8.6 Row Hash Recomputation

When column-first is selected, row hashes must be recomputed using only matched columns.

#### Why Recomputation is Necessary

Original row hashes include all columns. If a column was inserted in B, every row in B has a different hash than its counterpart in A—even if the actual content in matched columns is identical. Recomputation removes this noise.

#### Process

1. **Identify matched columns**: From the column mapping, determine which columns in the source grid have corresponding columns in the target grid
2. **Filter cells**: For each row, include only cells from matched columns
3. **Preserve original indices**: The column index fed to the hasher should be the original column index (not the mapped position), ensuring comparable hashes within each grid
4. **Recompute hashes**: Hash the filtered cell set for each row

**Key Properties:**
- Deleted columns do not contribute to the recomputed hash
- New columns do not affect the hash of existing content
- Rows with identical content in matched columns will have identical recomputed hashes

#### Example

Grid A columns: `[A, B, C]`
Grid B columns: `[A, X, B, C]` (X inserted at position 1)

Column mapping (A→B): `[Some(0), Some(2), Some(3)]`
Matched columns in A: `{0, 1, 2}` (all columns)

After recomputation, a row in A with values `[10, 20, 30]` and a row in B with values `[10, NEW, 20, 30]` will have matching hashes (both computed from A=10, B=20, C=30).

**Complexity**: O(M) where M = non-empty cells in matched columns. Can be parallelized across rows.

---

### 8.7 Column-First Workflow

When column-first is selected, the workflow becomes:

```
1. Phase 1: Preprocessing
   - Build GridViews
   - Compute initial row/column hashes
   
2. Phase 2: Dimension Decision
   - Detect column instability
   - Perform preliminary column alignment
   - Generate column mapping
   
3. Phase 2b: Row Hash Recomputation (NEW)
   - Recompute row hashes for Grid A using matched columns
   - Recompute row hashes for Grid B using matched columns
   - Update RowMeta with new hashes
   - Rebuild frequency tables with new hashes
   
4. Phase 3: Mode Selection (unchanged)

5. Phase 4a: Spreadsheet Alignment
   - Row alignment uses recomputed hashes
   - Column alignment uses preliminary mapping (refine if needed)
   
6. Phases 5-6: (unchanged)
```

The additional Phase 2b adds O(M) work for hash recomputation, but this is typically small compared to the alignment benefit.

---

### 8.8 Trade-offs

#### Row-First Advantages

- Simpler implementation
- No recomputation overhead
- Works for the common case (no column changes)

#### Column-First Advantages

- Correct handling of column insertions/deletions
- More stable row alignment
- Fewer false "row changed" reports

#### When Column-First Hurts

- If columns are stable, recomputation is wasted work
- If column alignment is wrong, row alignment will be wrong too

The threshold-based automatic selection mitigates these risks.

---

### 8.9 Alternative: Order-Independent Row Hashing

An alternative approach avoids the dimension ordering problem entirely:

**Idea**: Use order-independent hashing where column position doesn't matter.

```
row_hash = XOR of cell_hashes for all cells in row
         OR
row_hash = PRODUCT of (cell_hash | 1) mod large_prime
```

**Problems**:
- Can't distinguish `[A:1, B:2]` from `[A:2, B:1]` (values swapped)
- Higher collision probability
- Less useful for detecting specific changes

**Verdict**: Order-independent hashing is rejected for row fingerprinting. The adaptive dimension ordering approach provides better correctness with acceptable overhead.

---

### 8.10 Configuration Options

```
DimensionConfig
├── column_stability_threshold: f64     // Default: 0.90
├── force_row_first: bool               // Default: false
├── force_column_first: bool            // Default: false
└── recompute_on_instability: bool      // Default: true
```

| Option | Effect |
|--------|--------|
| `column_stability_threshold` | Similarity required to use row-first |
| `force_row_first` | Skip column check, always row-first |
| `force_column_first` | Always column-first (for testing) |
| `recompute_on_instability` | If false, use row-first even when unstable |

For most use cases, defaults are appropriate. Force options are for debugging and special scenarios.

---

### 8.11 Dimension Ordering Summary

| Step | Complexity | Output |
|------|------------|--------|
| Column hash collection | O(C) | Two hash sets |
| Jaccard similarity | O(C) | Stability score |
| Decision | O(1) | Row-first or column-first |
| Column alignment (if column-first) | O(C²) | Column mapping |
| Row hash recomputation (if column-first) | O(M) | Updated row hashes |

**Total overhead for column-first**: O(C² + M)

This overhead is justified when column changes would cause incorrect row alignment. The threshold-based automatic selection ensures we pay this cost only when necessary.

---

# Part IV: Spreadsheet Mode Alignment

## 9. Anchor-Move-Refine (AMR) Algorithm Overview

### 9.1 The Core Innovation

The Anchor-Move-Refine (AMR) algorithm is the heart of the spreadsheet mode alignment. Unlike traditional diff algorithms that treat move detection as a post-processing step, AMR integrates move detection into the alignment process itself.

**Traditional approach** (Align-Then-Detect):
```
1. Align rows using LCS/Myers → produces deletions and insertions
2. Scan deletions and insertions for matching content
3. Reclassify matching pairs as moves
```

**AMR approach** (Detect-Then-Align):
```
1. Discover anchors (high-confidence matches)
2. Extract move candidates (out-of-order matches)
3. Fill gaps with move-awareness (skip moved content)
4. Validate and emit moves
```

The key insight is that identifying move candidates **before** gap filling allows the gap filler to produce cleaner alignments. Content that moved elsewhere is not treated as a deletion requiring local alignment; it's marked as "moved away" and skipped.

---

### 9.2 Why AMR is Superior

#### Problem with Post-Hoc Move Detection

Consider this example:

**Grid A**:
```
Row 0: Header
Row 1: Alpha
Row 2: Beta
Row 3: Gamma  ← Block to be moved
Row 4: Delta
Row 5: Epsilon
Row 6: Zeta
Row 7: Footer
```

**Grid B**:
```
Row 0: Header
Row 1: Gamma  ← Block moved here
Row 2: Delta
Row 3: Epsilon
Row 4: Alpha
Row 5: Beta
Row 6: Zeta
Row 7: Footer
```

**Traditional LCS alignment** would find the longest common subsequence. Depending on tie-breaking, it might produce:

```
Matched: Header, Gamma, Delta, Epsilon, Zeta, Footer
Deleted: Alpha, Beta (from A rows 1-2)
Inserted: Alpha, Beta (in B rows 4-5)
```

Post-hoc move detection would then:
1. Find that deleted [Alpha, Beta] matches inserted [Alpha, Beta]
2. Reclassify as a move

**The problem**: The alignment already committed to matching Gamma at position 1 in A to position 1 in B. But these are different structural positions! The LCS was seduced by the matching content without understanding that Gamma moved.

**With AMR**:
1. Anchors: Header (0→0), Zeta (6→6), Footer (7→7)
2. Move candidate: [Gamma, Delta, Epsilon] appears at A[3-5] and B[1-3]
3. Gap filling: 
   - Gap between Header and Zeta knows that [Gamma, Delta, Epsilon] moved
   - Aligns [Alpha, Beta] in A to [Alpha, Beta] in B correctly
4. Result: Clean move operation + correct alignment of remaining content

---

### 9.3 AMR Phase Summary

AMR consists of four phases, each building on the previous:

| Phase | Name | Input | Output | Complexity |
|-------|------|-------|--------|------------|
| 1 | Anchor Discovery | Token sequences, frequency stats | Monotonic anchor chain | O(R log R) |
| 2 | Move Candidate Extraction | Anchor chain, hash matches | Block move candidates | O(R) |
| 3 | Move-Aware Gap Filling | Gaps, move candidates | Per-gap alignments | O(R) typical |
| 4 | Move Validation | Gap results, candidates | Validated moves, final alignment | O(K) |

**Total complexity**: O(R log R) dominated by anchor discovery (LIS computation).

---

### 9.4 Phase 1: Anchor Discovery

**Purpose**: Establish high-confidence row matches that partition the alignment problem.

**Key concepts**:
- **Anchor**: A pair (rowA, rowB) where the rows have the same hash and are both unique
- **Anchor chain**: A sequence of anchors that is monotonic in both dimensions
- **Gap**: The rows between consecutive anchors

**Algorithm sketch**:
1. Filter to unique rows (hash appears once in each grid)
2. Build match pairs from unique hashes
3. Sort by position in Grid A
4. Extract LIS (Longest Increasing Subsequence) by position in Grid B
5. The LIS gives the maximum non-crossing anchor chain

**Properties**:
- Anchors are reliable: unique matches can't be confused with other rows
- Anchors partition the problem: each gap is an independent subproblem
- Anchors establish structure: the skeleton of the alignment is fixed

**Detailed specification**: Section 10

---

### 9.5 Phase 2: Move Candidate Extraction

**Purpose**: Identify rows that match by hash but are not in the anchor chain—these are potential moves.

**Key concepts**:
- **Out-of-order match**: Rows that match (same hash) but would cross an anchor if included
- **Move candidate**: A contiguous block of out-of-order matches
- **Block**: Consecutive rows that moved together

**Algorithm sketch**:
1. Find all hash matches (row in A has same hash as row in B)
2. Subtract anchor pairs (already aligned)
3. Remaining matches are "out of order"
4. Cluster consecutive out-of-order matches into blocks
5. Each block is a move candidate

**Properties**:
- Move candidates are identified before gap filling
- Gap filling knows which rows "moved away" vs. were truly deleted
- Block clustering captures multi-row moves

**Detailed specification**: Section 11

---

### 9.6 Phase 3: Move-Aware Gap Filling

**Purpose**: Align the rows within each gap, respecting the identified move candidates.

**Key concepts**:
- **Gap**: Rows between consecutive anchors in each grid
- **Move-aware**: Rows that are move candidates are skipped, not aligned locally
- **Gap strategy**: Different algorithms based on gap characteristics

**Gap strategies**:

| Strategy | When Used | Complexity | Description |
|----------|-----------|------------|-------------|
| Trivial | One side empty | O(1) | All adds or all deletes |
| Myers | Small gap, diverse content | O(ND) | Optimal LCS |
| RLE | Large gap, repetitive | O(R) | Run-length alignment |
| Bail-out | Large gap, no similarity | O(1) | Block replacement |

**Algorithm sketch** (per gap):
1. Identify rows in this gap that are move candidates
2. Mark "moved from" (source rows) and "moved to" (destination rows)
3. Filter to remaining rows (non-moved)
4. Select gap strategy based on size and composition
5. Align remaining rows
6. Combine alignment with move markers

**Properties**:
- Moved rows don't pollute local alignment
- Each gap uses the most efficient strategy
- Pathological gaps bail out to prevent quadratic blowup

**Detailed specification**: Section 12

---

### 9.7 Phase 4: Move Validation

**Purpose**: Confirm move candidates and emit final operations.

**Key concepts**:
- **Validation**: A move is valid if both source and destination were handled as moves
- **Fuzzy move**: Blocks with high but imperfect similarity
- **Internal edits**: Cell changes within a moved block

**Algorithm sketch**:
1. For each move candidate:
   - Check that source rows were marked "moved from" in gap filling
   - Check that destination rows were marked "moved to"
   - If both: candidate is validated
2. For validated moves:
   - Compute exact similarity
   - If similarity < 1.0: mark as fuzzy move
   - Emit BlockMovedRows operation
3. Any unvalidated candidates become regular add/delete

**Properties**:
- Only emits moves that the gap filler confirmed
- Supports fuzzy moves with configurable threshold
- Final result is coherent and non-redundant

**Detailed specification**: Section 13

---

### 9.8 AMR Data Flow

```
                    ┌─────────────────────────────────────────┐
                    │          Token Sequences                │
                    │     tokens_a: [t0, t1, t2, ...]         │
                    │     tokens_b: [t0, t3, t1, ...]         │
                    └─────────────────┬───────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        PHASE 1: ANCHOR DISCOVERY                             │
│                                                                              │
│  Unique matches: [(0,0), (2,4), (5,6), ...]                                 │
│  LIS extraction → Anchor chain: [(0,0), (2,4), (5,6)]                       │
│                                                                              │
│  Gaps: [(1..1, 1..3), (3..4, 5..5), ...]                                    │
└─────────────────────────────────┬───────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    PHASE 2: MOVE CANDIDATE EXTRACTION                        │
│                                                                              │
│  All matches: [(0,0), (1,4), (2,1), (3,2), ...]                             │
│  Subtract anchors: [(1,4), (3,2), ...]                                      │
│  Cluster: Block{rows_a: 3..4, rows_b: 1..2}                                 │
│                                                                              │
│  Move candidates: [Block{...}, ...]                                          │
└─────────────────────────────────┬───────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    PHASE 3: MOVE-AWARE GAP FILLING                           │
│                                                                              │
│  For each gap:                                                               │
│    - Exclude rows in move candidates                                         │
│    - Select strategy (Myers/RLE/Bail-out)                                   │
│    - Align remaining rows                                                    │
│    - Record: matched, inserted, deleted, moved_from, moved_to               │
│                                                                              │
│  Gap alignments: [GapResult{...}, GapResult{...}, ...]                      │
└─────────────────────────────────┬───────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       PHASE 4: MOVE VALIDATION                               │
│                                                                              │
│  For each move candidate:                                                    │
│    - Verify source marked as moved_from                                      │
│    - Verify dest marked as moved_to                                          │
│    - Compute similarity, check threshold                                     │
│    - Emit ValidatedMove or reject                                            │
│                                                                              │
│  Final: ValidatedMoves + RowAlignment (matched, added, removed)             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### 9.9 Complexity Analysis

#### Anchor Discovery (Phase 1)

- Build unique match pairs: O(R)
- Sort by A-position: O(U log U) where U = unique matches
- LIS computation: O(U log U)
- **Total**: O(R + U log U) = O(R log R) worst case

#### Move Candidate Extraction (Phase 2)

- Find all matches: O(R) using hash table
- Subtract anchors: O(U)
- Cluster into blocks: O(R) single scan
- **Total**: O(R)

#### Move-Aware Gap Filling (Phase 3)

Depends on gap strategy:
- Trivial gaps: O(1) each
- Small Myers gaps: O(gap_size × edit_distance)
- RLE gaps: O(runs²) but runs << rows for repetitive data
- Bail-out: O(1)

**Key insight**: The sum of gap sizes equals R, but we never run O(N²) on the full R because:
1. Anchors split the problem into smaller pieces
2. Repetitive gaps use RLE (run count, not row count)
3. Large dissimilar gaps bail out

**Expected**: O(R) across all gaps
**Worst case** (all gaps use Myers, high edit distance): O(R × D) where D = total edits

#### Move Validation (Phase 4)

- Iterate move candidates: O(K) where K = candidates (K << R)
- Similarity check per candidate: O(block_size)
- **Total**: O(K × average_block_size) ≈ O(R) worst case, typically O(K)

#### Overall AMR Complexity

| Case | Complexity | Scenario |
|------|------------|----------|
| Best | O(R) | Many anchors, trivial gaps |
| Expected | O(R log R) | Typical edits, good anchor coverage |
| Worst | O(R × D) | Few anchors, many edits, no repetition |
| Adversarial (repetitive) | O(R) | RLE handles 99% blank case |

The O(R log R) expected case and O(R) adversarial case meet the performance requirements.

---

### 9.10 Comparison with Traditional Approaches

| Aspect | Traditional LCS | AMR |
|--------|----------------|-----|
| Move detection | Post-hoc | Integrated |
| Anchor usage | Sometimes | Always (core component) |
| Repetitive handling | Problematic | RLE built-in |
| Complexity guarantee | O(N²) possible | O(N log N) expected |
| Edit script quality | May have spurious edits | Cleaner, move-aware |
| Implementation complexity | Simpler | More complex |

The increased implementation complexity of AMR is justified by:
1. Better worst-case performance guarantees
2. Higher quality edit scripts
3. Correct move detection for complex scenarios
4. Unified handling of the adversarial case

---

### 9.11 AMR Configuration

```
AMRConfig
├── anchor_threshold: FrequencyThreshold    // What counts as "unique"
│   └── max_frequency: u32                  // Default: 1 (truly unique)
├── move_config: MoveConfig
│   ├── min_block_size: usize               // Default: 2
│   ├── fuzzy_threshold: f64                // Default: 0.80
│   └── max_candidates: usize               // Default: 1000
├── gap_config: GapConfig
│   ├── small_gap_threshold: usize          // Default: 256
│   ├── repetitive_threshold: f64           // Default: 0.80
│   ├── bailout_similarity: f64             // Default: 0.05
│   └── max_gap_cost: usize                 // Default: 100_000
└── parallel: bool                          // Default: true (if available)
```

Most users should use defaults. Tuning is for specific workloads:
- Increase `min_block_size` if too many small moves reported
- Decrease `fuzzy_threshold` to catch more fuzzy moves
- Increase `small_gap_threshold` if grids are very similar (afford more Myers)
- Decrease `bailout_similarity` to be more aggressive about bail-out

---

### 9.12 AMR Summary

AMR transforms the alignment problem from a monolithic sequence comparison into a structured, multi-phase process:

1. **Anchors** provide the skeleton—reliable matches that won't change
2. **Move candidates** capture structural reorganization before it confuses local alignment
3. **Move-aware gaps** use the right algorithm for each segment
4. **Validation** ensures only real moves are reported

This structure delivers:
- **Performance**: O(R log R) expected, O(R) for adversarial repetitive data
- **Quality**: Cleaner edit scripts with proper move semantics
- **Robustness**: Explicit handling of pathological cases

The following sections (10-13) specify each phase in full detail.

---

## 10. AMR Phase 1: Anchor Discovery

### 10.1 Purpose

Anchor discovery identifies high-confidence row matches that form the structural skeleton of the alignment. These anchors:

1. **Partition the problem**: Divide the grid into independent gaps
2. **Establish invariants**: Anchors don't change during gap filling
3. **Guide move detection**: Matches outside the anchor chain are move candidates
4. **Bound complexity**: Gap sizes are limited by anchor density

A good anchor chain balances coverage (many anchors) with reliability (only unambiguous matches).

---

### 10.2 Anchor Definition

**Formal definition**: An anchor is a pair (rowA, rowB) such that:
- `hash(rowA) == hash(rowB)`
- `freq_a[hash] == 1` (rowA is unique in Grid A)
- `freq_b[hash] == 1` (rowB is unique in Grid B)

**Properties of anchors**:
- **Unambiguous**: There's only one possible match for each anchor row
- **Reliable**: If hashes match and both are unique, the rows are almost certainly the same
- **Non-crossing**: The anchor chain enforces monotonicity (order preservation)

---

### 10.3 Anchor Chain Definition

**Formal definition**: An anchor chain is a sequence of anchors [(a₀, b₀), (a₁, b₁), ..., (aₖ, bₖ)] such that:
- a₀ < a₁ < ... < aₖ (strictly increasing in Grid A)
- b₀ < b₁ < ... < bₖ (strictly increasing in Grid B)

The anchor chain represents rows that are aligned "in the same order" in both grids. Rows matched but not in the chain are "out of order"—potential moves.

**Maximum anchor chain**: The longest possible anchor chain (maximum number of anchors satisfying the monotonicity constraint).

---

### 10.4 Algorithm: Patience Diff Anchoring

The anchor discovery algorithm is a variant of Patience Diff:

**Step 1: Index unique tokens in Grid B**

Build a map from each unique token to its position in Grid B. Only tokens with frequency 1 in both grids are included.

**Step 2: Build candidate anchor pairs**

Iterate through Grid A in order. For each unique token, look up its position in Grid B using the index from Step 1. If found, create a candidate anchor pair (posA, posB, token).

Since we iterate A in order, candidates are automatically sorted by their A-position.

**Step 3: Extract B-positions**

From the candidate list, extract just the B-positions into a separate sequence. This sequence may not be sorted (if rows moved).

**Step 4: Compute LIS of B-positions**

Apply the Longest Increasing Subsequence algorithm to the B-positions. The LIS identifies which candidates can be simultaneously included without crossing (violating monotonicity).

**Step 5: Build anchor chain**

The indices returned by LIS identify which candidates form the maximum non-crossing anchor chain. Extract these candidates to form the final anchor chain.

**Result**: A monotonic sequence of anchor pairs where both A-positions and B-positions are strictly increasing.

---

### 10.5 Longest Increasing Subsequence (LIS)

The LIS algorithm finds the longest strictly increasing subsequence of a sequence. For anchor discovery, we compute LIS on the B-positions of candidates.

#### Algorithm: Patience Sorting

The standard O(N log N) LIS algorithm uses a technique called "patience sorting":

1. **Maintain piles**: Conceptually maintain a list of "piles" of indices. Each pile is sorted in decreasing order of the values they represent.

2. **Place each element**: For each element in the sequence:
   - Binary search to find the leftmost pile whose top element has value ≥ current value
   - Place the current element on that pile (or start a new pile if none qualifies)
   - Record a "predecessor" link to the top of the previous pile (if any)

3. **Determine LIS length**: The number of piles equals the LIS length.

4. **Reconstruct LIS**: Follow predecessor links backward from the last pile to recover the actual subsequence.

**Complexity**: O(N log N) where N = number of candidates. The log factor comes from binary search at each step.

**Correctness**: The patience sorting invariant guarantees that the number of piles equals the LIS length. Predecessor tracking enables reconstruction of one (of possibly many) optimal solutions.

---

### 10.6 Gap Computation

Once the anchor chain is established, compute the gaps between consecutive anchors.

#### Process

1. **Add sentinels**: Conceptually add a "before first row" sentinel at the start and an "after last row" sentinel at the end. This ensures we capture:
   - Rows before the first anchor (head gap)
   - Rows after the last anchor (tail gap)

2. **Iterate anchor pairs**: For each consecutive pair of (virtual) anchors, compute the gap:
   - Gap in A: rows from (prev_anchor.row_a + 1) to (next_anchor.row_a - 1)
   - Gap in B: rows from (prev_anchor.row_b + 1) to (next_anchor.row_b - 1)

3. **Emit non-empty gaps**: Only emit gaps where at least one side has rows

#### Gap Structure

Each gap contains:
- `range_a`: The row indices in Grid A belonging to this gap (exclusive end)
- `range_b`: The row indices in Grid B belonging to this gap (exclusive end)
- `preceding_anchor`: The anchor before this gap (None for head gap)
- `following_anchor`: The anchor after this gap (None for tail gap)

#### Properties

- Gaps are non-overlapping and cover all non-anchor rows
- Each gap is an independent subproblem for alignment
- The number of gaps is at most (anchor_count + 1)

---

### 10.7 Anchor Quality Metrics

Not all anchor chains are equally useful. Quality metrics help diagnose anchor effectiveness and trigger fallback strategies.

#### Metrics Computed

| Metric | Definition | Significance |
|--------|------------|--------------|
| `anchor_count` | Number of anchors in the chain | More anchors = finer partitioning |
| `coverage_a` | Fraction of A rows that are anchors | Higher = better structural coverage |
| `coverage_b` | Fraction of B rows that are anchors | Higher = better structural coverage |
| `max_gap_size` | Largest gap (max of A and B sides) | Indicates worst-case gap filling cost |
| `avg_gap_size` | Average gap size | Indicates typical gap filling cost |
| `gap_count` | Number of gaps | More gaps = more parallelization opportunity |

**Quality interpretation**:

| Metric | Good | Concerning | Action |
|--------|------|------------|--------|
| coverage > 0.3 | Many anchors | Few unique rows | May need rare-row fallback |
| max_gap_size < 500 | Small gaps | Large unanchored region | Consider secondary anchors |
| avg_gap_size < 50 | Fine partitioning | Coarse partitioning | Acceptable if repetitive |

---

### 10.8 Secondary Anchor Expansion (Optional)

When anchor coverage is low, optionally extend with rare (non-unique) rows.

#### When to Trigger

Secondary expansion is considered when:
- Anchor count is below a minimum threshold
- One or more gaps are very large (exceeding a size threshold)

#### Process

For each large gap:
1. **Identify rare tokens**: Find rows in the gap with tokens that are rare (frequency ≤ threshold) but not unique
2. **Check for unambiguous matches**: For each rare token, check if it appears exactly once in BOTH the A-side and B-side of this gap
3. **Add secondary anchors**: If a rare token is unambiguous within the gap context, add it as a secondary anchor
4. **Recompute gaps**: After adding secondary anchors, gaps must be recomputed

#### Trade-offs

- **Benefit**: More anchors = smaller gaps = better alignment
- **Risk**: Rare-but-not-unique matches may be incorrect if the "unambiguous within gap" heuristic fails
- **Recommendation**: Use conservatively; only when primary anchor coverage is very low (<10%)

**When to use secondary anchors**:
- Coverage < 10%
- Max gap size > 1000
- Rare matches are unambiguous within their gaps

**Trade-off**: Secondary anchors increase coverage but may introduce errors if the "rare" match is actually ambiguous in context.

---

### 10.9 Handling Edge Cases

#### No Unique Rows

If no rows are unique in both grids (all rows repeated), anchor discovery produces an empty chain.

**Fallback**: Treat the entire grid as one gap. Gap filling will use RLE or bail-out strategy.

#### All Rows Unique

If every row is unique, anchor discovery produces a chain of all matched rows.

**Result**: Minimal or no gaps. The diff is the symmetric difference of rows.

#### Out-of-Order Unique Rows

If unique rows exist but their order completely reverses (A = [1,2,3], B = [3,2,1]):

**LIS result**: Length 1 (any single element)
**Gaps**: Most rows end up in gaps
**Move detection**: Will identify large moves

This is a valid but degenerate case; the algorithm handles it correctly.

---

### 10.10 Anchor Discovery Output

```
AnchorDiscoveryResult
├── anchor_chain: Vec<Anchor>     // The monotonic anchor sequence
├── gaps: Vec<Gap>                // Gaps between anchors
├── quality: AnchorQuality        // Metrics about anchor coverage
├── unanchored_matches: Vec<(u32, u32)>  // Matches not in chain (for Phase 2)
└── unique_count: usize           // How many unique rows existed

Anchor
├── row_a: u32                    // Position in Grid A
├── row_b: u32                    // Position in Grid B
└── token: Token                  // The matching token (hash)
```

The `unanchored_matches` field contains all hash matches that were not included in the anchor chain—these become move candidates in Phase 2.

---

### 10.11 Complexity Analysis

| Step | Complexity | Notes |
|------|------------|-------|
| Build unique position map | O(R_B) | Single pass over Grid B |
| Build candidate pairs | O(R_A) | Single pass over Grid A |
| Extract B-positions | O(U) | U = unique matches |
| LIS computation | O(U log U) | Patience sorting |
| Gap computation | O(U) | Single pass over anchors |
| Quality metrics | O(G) | G = number of gaps |
| **Total** | **O(R + U log U)** | Dominated by LIS |

Since U ≤ min(R_A, R_B), the complexity is **O(R log R)** worst case.

For typical spreadsheets where many rows are unique (U ≈ 0.3R to 0.8R), the algorithm is very efficient.

---

### 10.12 Anchor Discovery Summary

Anchor discovery transforms the raw token sequences into:
1. A reliable skeleton of matched rows (anchor chain)
2. Independent subproblems (gaps) for further alignment
3. Candidates for move detection (unanchored matches)

**Key properties**:
- O(R log R) complexity via LIS
- Only uses unique rows (maximum reliability)
- Monotonicity ensures non-crossing alignment
- Gaps are independent (parallelizable in Phase 3)

The anchor chain is the foundation on which the rest of AMR builds.

---

## 11. AMR Phase 2: Move Candidate Extraction

### 11.1 Purpose

Move candidate extraction identifies rows that match by hash but were excluded from the anchor chain. These "out-of-order" matches are potential moves—content that exists in both grids but at different structural positions.

**Key insight**: If row A[i] has the same hash as row B[j], but (i, j) is not in the anchor chain, then either:
- The row moved from position i to position j, OR
- The row was deleted at i and coincidentally re-added at j

Phase 2 identifies these candidates; Phase 4 determines which are true moves.

---

### 11.2 Out-of-Order Match Definition

**Formal definition**: An out-of-order match is a pair (rowA, rowB) such that:
- `hash(rowA) == hash(rowB)` (content matches)
- (rowA, rowB) is NOT in the anchor chain
- Including (rowA, rowB) in the chain would violate monotonicity

**Intuition**: These are rows that "crossed" an anchor. If anchors say row 5 in A matches row 10 in B, but row 7 in A also matches row 3 in B, then (7, 3) is out-of-order—row 7 would have to go "backwards" to position 3.

---

### 11.3 Extraction Algorithm

The extraction process has two phases:

#### Phase A: Find Out-of-Order Matches

1. **Build anchor lookup sets**: Create sets of anchored row indices for both grids (O(U) where U = anchor count)

2. **Scan for matches**: For each non-anchored row in Grid A:
   - Look up all positions in Grid B with the same token
   - For each position that is also non-anchored, record the match triple (rowA, rowB, token)

3. **Result**: A list of all matching pairs that were excluded from the anchor chain

**Why these are "out of order"**: If they had been in order (monotonic), they would have been included in the anchor chain. Their exclusion means including them would violate monotonicity.

#### Phase B: Cluster into Candidates

The out-of-order matches are clustered into block candidates (see Section 11.4).

---

### 11.4 Block Clustering

Consecutive out-of-order matches form move blocks.

#### Process

1. **Sort matches**: Sort by (row_a, row_b) to bring consecutive matches together

2. **Identify consecutive runs**: Iterate through sorted matches. A match extends the current block if:
   - Its row_a equals the current block's end in A
   - Its row_b equals the current block's end in B
   - Otherwise, close the current block and start a new one

3. **Filter by size**: Only emit blocks that meet the minimum size threshold

#### Minimum Block Size

Default: 2 rows

**Rationale**: Single-row "moves" are usually coincidental matches (two different rows happen to have the same content). Requiring at least 2 consecutive matching rows increases confidence that the block genuinely moved.

**Configurable**: Can be set higher (e.g., 3-5) to reduce false positive moves, or to 1 for aggressive move detection.

#### Output

Each move candidate includes:
- Source row range in Grid A
- Destination row range in Grid B  
- The sequence of tokens (for verification)
- Similarity score (1.0 for exact matches)

---

### 11.5 Move Candidate Structure

```
MoveCandidate
├── range_a: Range<u32>          // Source rows in Grid A
├── range_b: Range<u32>          // Destination rows in Grid B
├── tokens: Vec<Token>           // Sequence of row tokens in the block
├── similarity: f64              // 1.0 for exact, <1.0 for fuzzy
└── validated: bool              // Set in Phase 4
```

---

### 11.6 Handling Ambiguous Matches

When a token appears multiple times in both grids (rare but not unique), multiple pairings are possible:

```
Grid A: [X, Y, X]  (positions 0, 2 have same token X)
Grid B: [X, X, Y]  (positions 0, 1 have same token X)

Possible matches: (0,0), (0,1), (2,0), (2,1)
```

**Strategy**: Include all possible matches in clustering. Phase 4 validation will resolve ambiguity by checking which matches are consistent with gap filling results.

---

### 11.7 Move Candidate Output

```
MoveCandidateResult
├── candidates: Vec<MoveCandidate>    // Block move candidates
├── moved_from_a: HashSet<u32>        // All A rows that might have moved
├── moved_to_b: HashSet<u32>          // All B rows that might be move destinations
└── candidate_count: usize
```

The `moved_from_a` and `moved_to_b` sets are used in Phase 3 to skip these rows during gap filling.

---

### 11.8 Complexity

- Building anchored sets: O(U) where U = anchor count
- Scanning for matches: O(R_A × avg_positions_per_token)
- Clustering: O(M log M) where M = match count
- **Total**: O(R + M log M), typically O(R)

---

## 12. AMR Phase 3: Move-Aware Gap Filling

### 12.1 Purpose

Gap filling aligns the rows within each gap between anchors. The "move-aware" aspect means:
- Rows identified as move candidates are **skipped**, not aligned locally
- This prevents moved content from being treated as deletions/insertions
- The gap filler focuses on truly local changes

---

### 12.2 Gap Processing Overview

Gap processing is embarrassingly parallel—each gap is independent and can be processed concurrently.

**Process:**
1. Partition gaps across available threads
2. Each thread processes its assigned gaps using the move-aware algorithm
3. Collect results into a vector ordered by gap position

**Determinism**: Results are collected in gap order (not completion order) to ensure deterministic output regardless of thread scheduling.

---

### 12.3 Single Gap Algorithm

Processing a single gap involves four steps:

#### Step 1: Identify Move Candidates in This Gap

Check each row in the gap against the global move candidate sets:
- `moved_from`: Rows in A-side of gap that are sources of moves
- `moved_to`: Rows in B-side of gap that are destinations of moves

#### Step 2: Compute Remaining Rows

Filter out move candidate rows to get the "truly local" rows:
- `remaining_a`: Gap rows in A that didn't move away
- `remaining_b`: Gap rows in B that didn't move in

These remaining rows are what the gap filler actually needs to align.

#### Step 3: Select and Apply Gap Strategy

Based on the remaining rows' characteristics, select the appropriate strategy (Section 12.4) and execute it.

#### Step 4: Combine Results

The gap alignment output includes:
- Alignment results from the chosen strategy (matched pairs, deletions, insertions)
- The move information (moved_from, moved_to)
- Diagnostic: which strategy was used

**Key Insight**: By excluding move candidates before alignment, the gap filler doesn't waste effort trying to align rows that have already been accounted for elsewhere.

---

### 12.4 Gap Strategy Selection

The gap filler chooses among four strategies based on gap characteristics:

| Strategy | Condition | Complexity |
|----------|-----------|------------|
| **Trivial** | One side is empty | O(n) |
| **Myers** | Small gap (≤ threshold) | O(n × d) |
| **RLE** | Large gap with repetitive content | O(runs²) |
| **BailOut** | Large gap with low similarity | O(1) |

#### Decision Flow

1. **Trivial check**: If either side is empty, all rows are pure additions or deletions. No alignment needed.

2. **Size check**: If the larger side is ≤ small_gap_threshold (default: 256), use Myers. The potential O(n²) cost is bounded.

3. **Repetitive check**: Compute the fraction of rows that are low-information. If > repetitive_threshold (default: 0.80), use RLE alignment.

4. **Similarity check**: Compute a quick similarity estimate (e.g., Jaccard of token sets). If < bailout_threshold (default: 0.05), treat as completely different—bail out.

5. **Default**: For gaps that pass all checks but aren't small, use Myers anyway (with cost caps if needed).

---

### 12.5 Gap Strategy Implementations

#### Trivial Alignment

When one side is empty, the result is trivial:
- All rows in the non-empty side are either pure deletions (if A-side) or pure insertions (if B-side)
- No matching is attempted
- No matching pairs are produced

**Output**: Empty matches, all of A as deletions OR all of B as insertions.

#### Myers Alignment

Standard sequence alignment on the gap's tokens:

1. **Extract tokens**: Get the row token for each row in the gap (from RowMeta)
2. **Run Myers diff**: Apply the standard O(ND) Myers algorithm to find the shortest edit script
3. **Convert to alignment**: Map the edit script (Match/Delete/Insert operations) back to row indices

**Complexity**: O(n × d) where n = gap size, d = edit distance. Bounded by the gap size threshold.

#### RLE Alignment

For repetitive gaps, align on runs rather than individual rows:

1. **Build runs**: Compress each side into a sequence of runs (token, count, low-info flag)
2. **Align runs**: Use a greedy merge approach:
   - If runs have the same token: match min(length_a, length_b) rows
   - If one run is low-info and the other isn't: skip the low-info run
   - If both are high-info but different: advance the shorter run
3. **Expand to rows**: Convert run-level matches back to row-level matches

**Key optimization**: A gap with 10,000 rows but only 5 distinct runs operates in O(5²), not O(10,000²).

#### Bail-Out Alignment

When grids share almost no content in the gap:
- Don't attempt expensive alignment
- Mark everything in A as deleted
- Mark everything in B as inserted
- Produces honest "replaced" result rather than spurious partial matches

**When used**: Similarity < 5% (configurable). This is the "escape hatch" for pathological gaps.

---

### 12.6 Gap Alignment Output

```
GapAlignment
├── matched: Vec<(u32, u32)>     // Aligned row pairs within gap
├── deleted: Vec<u32>            // Rows only in A (truly deleted)
├── inserted: Vec<u32>           // Rows only in B (truly inserted)
├── moved_from: Vec<u32>         // Rows that moved away from this gap
├── moved_to: Vec<u32>           // Rows that moved into this gap
└── strategy_used: GapStrategy   // For diagnostics
```

---

### 12.7 Complexity

| Strategy | Complexity | When Used |
|----------|------------|-----------|
| Trivial | O(1) | One side empty |
| Myers | O(gap × edits) | Small gaps |
| RLE | O(runs²) | Large repetitive |
| Bail-out | O(1) | Large dissimilar |

**Key guarantee**: No individual gap causes O(R²) work. Either the gap is small (bounded Myers), repetitive (RLE compresses), or dissimilar (bail-out).

---

## 13. AMR Phase 4: Move Validation & Emission

### 13.1 Purpose

Move validation confirms which move candidates are genuine moves and emits the final alignment result. A candidate is validated if:
- Its source rows were marked as `moved_from` in gap filling
- Its destination rows were marked as `moved_to` in gap filling
- The rows weren't claimed by local alignment

---

### 13.2 Validation Algorithm

The validation process confirms that move candidates are consistent with gap filling results.

#### Process

1. **Collect global move sets**: Aggregate all `moved_from` and `moved_to` rows across all gap alignments

2. **Validate each candidate**: For each move candidate, check:
   - **Source validity**: All rows in the candidate's source range appear in the global `moved_from` set
   - **Destination validity**: All rows in the candidate's destination range appear in the global `moved_to` set

3. **Accept or reject**: Candidates passing both checks are validated; others are rejected

4. **Compute similarity**: For validated candidates, compute exact similarity to determine if fuzzy

#### Validation Logic
        
        // Check all dest rows are in moved_to
        let dest_valid = candidate.range_b.clone()
            .all(|r| all_moved_to.contains(&r))
        
        if source_valid AND dest_valid:
            validated.push(ValidatedMove {
                source_rows: candidate.range_a.clone(),
                dest_rows: candidate.range_b.clone(),
                similarity: candidate.similarity,
                is_fuzzy: candidate.similarity < 1.0
            })
    
    return validated
```

---

### 13.3 Fuzzy Move Detection

For blocks with high but imperfect similarity:

```
function compute_block_similarity(
    range_a: &Range<u32>,
    range_b: &Range<u32>,
    view_a: &GridView,
    view_b: &GridView
) -> f64:
    
    let tokens_a: HashSet<Token> = range_a.clone()
        .map(|r| view_a.row_meta[r as usize].token)
        .collect()
    
    let tokens_b: HashSet<Token> = range_b.clone()
        .map(|r| view_b.row_meta[r as usize].token)
        .collect()
    
    // Jaccard similarity
    let intersection = tokens_a.intersection(&tokens_b).count()
    let union = tokens_a.union(&tokens_b).count()
    
    return intersection as f64 / union as f64
```

**Threshold**: Default 0.80. Blocks with similarity ≥ 0.80 are moves; below are delete+insert.

---

### 13.4 Final Alignment Assembly

```
function assemble_row_alignment(
    anchor_chain: &[Anchor],
    gap_alignments: &[GapAlignment],
    validated_moves: &[ValidatedMove]
) -> RowAlignment:
    
    let mut matched: Vec<(u32, u32)> = Vec::new()
    let mut inserted: Vec<u32> = Vec::new()
    let mut deleted: Vec<u32> = Vec::new()
    
    // Add anchor matches
    for anchor in anchor_chain:
        matched.push((anchor.row_a, anchor.row_b))
    
    // Add gap matches (excluding moved rows)
    let moved_a: HashSet<u32> = validated_moves.iter()
        .flat_map(|m| m.source_rows.clone())
        .collect()
    
    let moved_b: HashSet<u32> = validated_moves.iter()
        .flat_map(|m| m.dest_rows.clone())
        .collect()
    
    for gap_alignment in gap_alignments:
        for (a, b) in &gap_alignment.matched:
            matched.push((*a, *b))
        
        for &a in &gap_alignment.deleted:
            if !moved_a.contains(&a):
                deleted.push(a)
        
        for &b in &gap_alignment.inserted:
            if !moved_b.contains(&b):
                inserted.push(b)
    
    // Sort for determinism
    matched.sort()
    deleted.sort()
    inserted.sort()
    
    return RowAlignment { matched, inserted, deleted }
```

---

### 13.5 ValidatedMove Structure

```
ValidatedMove
├── source_rows: Range<u32>      // Rows in Grid A
├── dest_rows: Range<u32>        // Rows in Grid B
├── similarity: f64              // 1.0 = exact, <1.0 = fuzzy
├── is_fuzzy: bool               // True if internal edits likely
└── internal_alignment: Option<Vec<(u32, u32)>>  // Row mapping within block (for fuzzy)
```

For fuzzy moves, `internal_alignment` maps which source row corresponds to which dest row within the block (computed via small LCS on block content).

---

### 13.6 Complete AMR Output

```
AMRResult
├── row_alignment: RowAlignment
│   ├── matched: Vec<(u32, u32)>
│   ├── inserted: Vec<u32>
│   └── deleted: Vec<u32>
├── validated_moves: Vec<ValidatedMove>
├── anchor_chain: Vec<Anchor>
├── gap_count: usize
├── quality_metrics: AMRQuality
│   ├── anchor_coverage: f64
│   ├── move_count: usize
│   ├── fuzzy_move_count: usize
│   └── bailout_gap_count: usize
└── strategies_used: HashMap<GapStrategy, usize>
```

---

### 13.7 Phase 4 Complexity

- Collecting moved sets: O(R)
- Validating candidates: O(K × block_size) where K = candidates
- Assembling result: O(R)
- Sorting: O(R log R)
- **Total**: O(R log R)

---

### 13.8 AMR Phases Complete Summary

| Phase | Input | Output | Complexity |
|-------|-------|--------|------------|
| 1. Anchor Discovery | Tokens, stats | Anchor chain, gaps | O(R log R) |
| 2. Move Extraction | Anchors, all matches | Move candidates | O(R) |
| 3. Gap Filling | Gaps, candidates | Per-gap alignments | O(R) typical |
| 4. Validation | Candidates, gap results | Final alignment, moves | O(R log R) |
| **Total** | | | **O(R log R)** |

The AMR algorithm delivers:
- Correct alignment with move detection integrated
- Guaranteed O(R log R) complexity
- Robustness against repetitive data via RLE
- Clean separation of concerns across phases

---

## 14. Column Alignment

### 14.1 Purpose

Column alignment establishes correspondence between columns in Grid A and Grid B. While row alignment handles the dominant dimension (typically many rows), column alignment handles the secondary dimension (typically few columns, ≤100).

**Key differences from row alignment**:
- Column count is small: O(C²) algorithms are acceptable
- Column moves are less common than row moves
- Column alignment may be conditioned on row alignment results

---

### 14.2 When Column Alignment Runs

Column alignment timing depends on dimension ordering (Section 8):

| Dimension Order | Column Alignment Timing | Hash Basis |
|-----------------|------------------------|------------|
| Row-first | After row alignment | All rows |
| Column-first | Before row alignment | All rows (preliminary) |
| Column-first | After row alignment | Matched rows only (refined) |

For row-first (the common case), column alignment uses hashes computed over all rows.

---

### 14.3 Column Alignment Algorithm

Since column count is small (typically ≤100), we use a straightforward sequence alignment approach.

#### Step 1: Compute Column Hashes

If row alignment is available, compute column hashes over **matched rows only** for stability. Otherwise, use the pre-computed hashes over all rows.

#### Step 2: Tokenize

Convert column hashes to compact tokens using a shared token map.

#### Step 3: Sequence Alignment

Apply Myers diff to the column token sequences. Since C is small, O(C²) worst case is acceptable (~10,000 operations for 100 columns).

#### Step 4: Build Column Mapping

Walk the edit script to build:
- `col_mapping`: For each column in A, the corresponding column in B (or None if removed)
- `col_added`: Columns present only in B
- `col_removed`: Columns present only in A

#### Step 5: Detect Column Moves

Among removed and added columns, identify pairs with matching hashes—these are columns that moved rather than being deleted/added.

---

### 14.4 Column Hash Conditioning

When row alignment is known, column hashes should be computed over matched rows only.

#### Process

1. **Identify matched rows**: Extract the set of row indices from the row alignment that are matched
2. **Iterate columns**: For each column index:
   - Scan only matched rows
   - Feed each cell's row index and value into the hasher
3. **Finalize**: Produce one hash per column

#### Benefit

Columns that are identical in matched rows will have identical hashes, even if unmatched rows differ. This produces more stable column alignment when rows have been added or removed.

---

### 14.5 Column Move Detection

Column moves are detected by matching removed columns to added columns by hash.

#### Process

1. **Index removed columns**: Build a map from column hash to list of removed column indices
2. **Match added columns**: For each added column:
   - Look up its hash in the removed index
   - If found, pair it with the first unused removed column with that hash
   - Mark the removed column as used to prevent double-matching
3. **Emit moves**: Each matched pair becomes a column move

#### Properties

- **Simpler than row moves**: No block clustering needed (columns rarely move in groups)
- **Greedy matching**: First-come-first-served for hash collisions
- **Small scale**: With C ≤ 100, even O(C²) approaches would be acceptable

---

### 14.6 Column Alignment Output

```
ColumnAlignment
├── col_mapping: Vec<Option<u32>>    // colA -> colB (None if deleted)
├── col_added: Vec<u32>              // Columns only in B
├── col_removed: Vec<u32>            // Columns only in A
└── col_moves: Vec<ColumnMove>       // Detected column moves

ColumnMove
├── source_col: u32                  // Column index in A
└── dest_col: u32                    // Column index in B
```

---

### 14.7 Complexity

- Column hash computation: O(M) where M = cells in matched rows
- Tokenization: O(C)
- Myers diff: O(C × D) where D = column edit distance, worst case O(C²)
- Move detection: O(C)
- **Total**: O(M + C²)

Since C ≤ 100 typically, the O(C²) term is ~10,000 operations—negligible.

---

# Part V: Database Mode Alignment

## 15. Key Extraction & Hashing

### 15.1 Purpose

Database mode treats rows as records identified by key columns. Key extraction computes a composite key for each row that determines its identity regardless of position.

**When database mode applies**:
- User explicitly specifies key columns
- Excel Table metadata identifies a primary key
- Key inference (Section 18) detects likely keys

---

### 15.2 Key Column Configuration

```
KeyConfig
├── columns: Vec<u32>                // Key column indices
├── case_sensitive: bool             // Default: true
├── trim_whitespace: bool            // Default: true
└── null_handling: NullHandling      // How to handle empty key cells

enum NullHandling:
    TreatAsValue,     // Empty is a valid key value
    ExcludeRow,       // Rows with empty keys are excluded
    Error             // Fail if empty key encountered
```

---

### 15.3 Key Extraction Algorithm

Key extraction builds a keyed representation of each row for hash join alignment.

#### Process (per row)

1. **Extract key values**: For each key column, find the cell value and normalize it
2. **Handle null keys**: Apply the configured null handling:
   - `TreatAsValue`: Empty is a valid key value
   - `ExcludeRow`: Skip rows with empty key columns
   - `Error`: Fail if any key column is empty
3. **Compute key hash**: Hash the sequence of key values (position-sensitive)
4. **Compute content hash**: Hash the non-key columns (for duplicate resolution similarity)
5. **Build KeyedRow**: Package row index, key hash, content hash, and key values

#### Output

A vector of KeyedRow structures, one per row (or fewer if ExcludeRow filtered some out).

---

### 15.4 Key Value Normalization

Key values are normalized to ensure consistent matching.

**Normalization Rules by Type:**

| Type | Normalization Applied |
|------|----------------------|
| String | Trim whitespace (if configured), lowercase (if case-insensitive) |
| Number | Canonicalize to standard representation (handle -0, NaN, precision) |
| Boolean | No normalization needed |
| Empty | Preserved as empty |
| Error | Preserved as-is |

**Configuration Options:**
- `trim_whitespace` (default: true): Remove leading/trailing whitespace from strings
- `case_sensitive` (default: true): If false, convert strings to lowercase before comparison

---

### 15.5 Key Hash Computation

The key hash uniquely identifies a composite key value.

#### Hashing Strategy

For each key column value (in order):
1. Feed the column position index (distinguishes [A,B] from [B,A])
2. Feed a type discriminant byte
3. Feed the type-specific value encoding:
   - Empty: nothing additional
   - Boolean: 1 byte (0 or 1)
   - Number: 8 bytes (f64 bit representation)
   - String: UTF-8 bytes
   - Error: error code byte

Finalize the hasher to produce the key hash.

#### Properties

- **Deterministic**: Same key values always produce same hash
- **Position-sensitive**: Key column order matters
- **Type-sensitive**: String "1" and number 1 produce different hashes

---

### 15.6 KeyedRow Structure

```
KeyedRow
├── row_idx: u32                     // Original row index in grid
├── key_hash: KeyHash                // Hash of key column values
├── content_hash: RowHash            // Hash of non-key columns
└── key_values: Vec<CellValue>       // Actual key values (for display/debugging)
```

---

### 15.7 Complexity

- Key extraction: O(R × K) where K = number of key columns
- Hash computation: O(K) per row
- **Total**: O(R × K), typically O(R) since K is small (1-3 columns)

---

## 16. Hash Join Alignment

### 16.1 Purpose

Hash join alignment matches rows between grids based on key equality. Unlike spreadsheet mode (which aligns by position), database mode matches rows that have the same key values regardless of where they appear.

---

### 16.2 Building Key Maps

Key maps provide efficient lookup from key hash to the list of rows with that key.

**Construction:**
1. Initialize an empty map for each grid (key hash → list of row indices)
2. Iterate through all keyed rows in Grid A, appending each row's index to the list for its key hash
3. Repeat for Grid B

**Output:** Two maps, each mapping from KeyHash to Vec<row_index>.

**Purpose:** Enables O(1) lookup of all rows sharing a given key, which is the foundation of hash join.

---

### 16.3 Join Algorithm

The hash join processes each unique key and categorizes the match:

**Process:**

1. **Build key maps**: Create hash-to-rows maps for both grids
2. **Collect all keys**: Union of keys from both maps
3. **Process each key**: Categorize based on occurrence counts in each grid

**Categorization Logic:**

| Rows in A | Rows in B | Category | Action |
|-----------|-----------|----------|--------|
| 0 | ≥1 | Key only in B | Mark all B rows as additions |
| ≥1 | 0 | Key only in A | Mark all A rows as removals |
| 1 | 1 | Perfect match | Record as matched pair |
| >1 or >1 | >1 or >1 | Duplicate keys | Defer to cluster resolution |

**Output:** Lists of matched pairs, additions, removals, and duplicate clusters requiring further processing.

---

### 16.4 Handling 1:1 Matches

For keys appearing exactly once in each grid, the match is unambiguous. The single row from A is paired with the single row from B.

These matched pairs are then passed to the cell-level diff phase, which compares the non-key column values to identify actual changes.

---

### 16.5 Handling Key Additions/Removals

Keys appearing in only one grid indicate structural changes:

| Scenario | Interpretation | Operation |
|----------|---------------|-----------|
| Key in A only | Row(s) were deleted | `RowRemoved` for each |
| Key in B only | Row(s) were added | `RowAdded` for each |

---

### 16.6 Duplicate Key Detection

When a key appears multiple times in either grid, we cannot determine the match by key alone. These cases are collected into **duplicate clusters** for specialized resolution.

**Duplicate Cluster Structure:**
- `key_hash`: The key that has duplicates
- `rows_a`: All row indices in Grid A with this key
- `rows_b`: All row indices in Grid B with this key

Duplicate clusters require content-based matching (Section 17).

---

### 16.7 KeyedAlignment Output

The hash join produces a partial alignment:

**Output Fields:**
- `matched`: Row pairs with unique 1:1 key matches
- `added`: Row indices only in Grid B (key not in A)
- `removed`: Row indices only in Grid A (key not in B)
- `duplicate_clusters`: Key clusters requiring further resolution

---

### 16.8 Complexity

- Building key maps: O(R_A + R_B)
- Collecting all keys: O(K) where K = unique keys
- Processing each key: O(1) for non-duplicate, O(cluster_size) for duplicate
- **Total**: O(R) expected

The O(R) complexity is a key advantage of database mode over spreadsheet mode's O(R log R).

---

### 16.9 Database Mode vs Spreadsheet Mode

| Aspect | Spreadsheet Mode | Database Mode |
|--------|-----------------|---------------|
| Row identity | Position | Key value |
| Order matters | Yes | No |
| Complexity | O(R log R) | O(R) |
| Move detection | Yes (reordering is a change) | N/A (reordering ignored) |
| Duplicate handling | By position | By content similarity |

Database mode is simpler and faster when key-based identity is appropriate.

---

## 17. Duplicate Key Cluster Resolution

### 17.1 The Duplicate Key Problem

When multiple rows share the same key value, hash join cannot determine which row in A corresponds to which row in B. Consider:

```
Grid A (key = "Product"):
  Row 0: Product="Widget", Price=10, Qty=5
  Row 1: Product="Widget", Price=12, Qty=3

Grid B (key = "Product"):
  Row 0: Product="Widget", Price=10, Qty=8
  Row 1: Product="Widget", Price=15, Qty=3
```

Both rows have key "Widget". The correct matching is:
- A[0] ↔ B[0] (Price=10 unchanged, Qty changed)
- A[1] ↔ B[1] (Price changed, Qty=3 unchanged)

But we could incorrectly match A[0] ↔ B[1], producing nonsensical diffs.

---

### 17.2 Solution: Content-Based Matching

Within each duplicate cluster, match rows by **similarity of non-key content**. Rows that are most similar should be matched.

---

### 17.3 Cluster Resolution Algorithm

Cluster resolution finds the best matching between rows in A and rows in B that share the same key.

#### Step 1: Build Cost Matrix

For each pair of rows (one from A, one from B in the cluster):
- Compute content similarity (Jaccard or Hamming on non-key cells)
- Convert to cost: `cost = 1.0 - similarity`
- Store in an n×m cost matrix

#### Step 2: Solve Assignment Problem

Select algorithm based on cluster size:
- **Small clusters** (n, m ≤ threshold, default 16): Use exact Hungarian/LAPJV algorithm for optimal solution
- **Large clusters**: Use greedy approximation (faster but potentially suboptimal)

#### Step 3: Build Resolution

From the assignment result:
- Accept matches where cost ≤ max_cost threshold
- Reject matches where cost > max_cost (rows are too different to be the same record)
- Unassigned A rows → removals
- Unassigned B rows → additions

**Output:** Matched pairs within the cluster, plus lists of unmatched rows.

---

### 17.4 Row Similarity Computation

Similarity is computed over **non-key columns only** (key columns match by definition).

#### Process

1. **Extract non-key cells**: Filter out key column cells from both rows
2. **Collect all columns**: Union of columns present in either row
3. **Count matches**: For each column, compare values using type-aware equality
4. **Compute ratio**: `similarity = matches / total_columns`

#### Edge Cases

- If both rows have no non-key content: similarity = 1.0 (trivially equal)
- Missing cells are treated as distinct from present cells
- Type mismatches (e.g., string vs number) count as non-matching

---

### 17.5 Hungarian Algorithm (LAPJV)

For small clusters (n, m ≤ 16), use the exact Hungarian algorithm.

#### Overview

The Hungarian algorithm (also known as LAPJV - Jonker-Volgenant) solves the linear assignment problem optimally:
- Given an n×m cost matrix
- Find an assignment of rows to columns minimizing total cost
- Each row is assigned to at most one column; each column to at most one row

#### Handling Rectangular Matrices

If n ≠ m, pad the cost matrix to square by adding dummy rows/columns with infinite cost. After solving, assignments to dummy elements indicate unmatched rows/columns.

#### Output

For each row i: `Some(j)` if assigned to column j, `None` if unassigned.

**Complexity**: O(K³) where K = max(n, m). Acceptable for small clusters (K ≤ 16 → ≤4096 operations).

---

### 17.6 Greedy Approximation

For large clusters, use greedy matching as a faster alternative to the exact algorithm.

#### Algorithm

1. **Build candidate list**: Enumerate all (row, col, cost) triples where cost < infinity
2. **Sort by cost**: Order candidates from lowest to highest cost
3. **Greedy selection**: Process candidates in order:
   - If the row is unassigned AND the column is unused, make the assignment
   - Otherwise skip (already used)

#### Properties

- **Complexity**: O(K² log K) for sorting
- **Quality**: Not guaranteed optimal, but typically close for well-structured cost matrices
- **Use case**: Clusters larger than the exact threshold (default: 16)

#### Trade-off

Greedy may produce suboptimal assignments when there are "competing" matches (row i could match column j₁ or j₂, and the greedy choice affects later options). For typical duplicate-key scenarios with high similarity between correct matches, this rarely matters.

---

### 17.7 Cluster Resolution Output

```
ClusterResolution
├── matched: Vec<(u32, u32)>     // Matched row pairs within cluster
├── removed: Vec<u32>            // A rows with no match (deleted)
└── added: Vec<u32>              // B rows with no match (added)
```

---

### 17.8 Configuration

```
ClusterConfig
├── key_columns: HashSet<u32>    // Columns to exclude from similarity
├── exact_threshold: usize       // Max cluster size for exact algorithm (default: 16)
├── max_cost: f64               // Max cost to accept a match (default: 0.5)
└── similarity_metric: Metric    // Jaccard, Hamming, or Weighted
```

---

### 17.9 Complexity Analysis

| Cluster Size | Algorithm | Complexity |
|--------------|-----------|------------|
| K ≤ 16 | Hungarian | O(K³) |
| K > 16 | Greedy | O(K² log K) |

Since duplicate clusters are rare and typically small (data quality implies unique keys), the total cost across all clusters is usually negligible.

---

## 18. Key Inference Algorithm

### 18.1 Purpose

When the user doesn't specify key columns, the algorithm attempts to infer which columns form a natural key. Good key inference is critical for usability—users shouldn't have to manually specify keys for obvious data tables.

---

### 18.2 Column Scoring Function

Each column is scored as a potential key component based on multiple factors.

#### Scoring Process

1. **Extract values**: Get all cell values from the column in both grids
2. **Compute per-factor scores**: Calculate each of the five factors (see 18.3)
3. **Average across grids**: For factors computed per-grid, average the scores
4. **Compute weighted composite**: Combine factors using the weights below

#### Composite Score Formula

```
composite = 0.35 × uniqueness + 0.20 × coverage + 0.20 × stability + 0.15 × type_score + 0.10 × header_score
```

#### Output

A ColumnKeyScore structure containing:
- Column index
- Individual factor scores
- Composite score (0.0 to 1.0)

---

### 18.3 Scoring Factors

#### Uniqueness (35% weight)

What fraction of values are unique within the column?

**Computation**: Count distinct non-empty values, divide by total non-empty values.

**Interpretation**: 1.0 = all unique (perfect key), 0.5 = half duplicates, 0.0 = all identical

#### Coverage (20% weight)

What fraction of rows have non-empty values in this column?

**Computation**: Count non-empty values, divide by total rows.

**Interpretation**: 1.0 = no blanks (ideal), 0.5 = half blank (problematic for key)

#### Stability (20% weight)

What fraction of values appear in both grids?

**Computation**: Jaccard similarity of the non-empty value sets from each grid. Filters out empty values, then computes |intersection| / |union|.

**Interpretation**: 1.0 = identical values (stable key), 0.0 = no overlap (not useful)

#### Data Type Score (15% weight)

Does the data look like an identifier?

**Scoring by pattern** (sample first 100 non-empty values):

| Pattern | Score | Example |
|---------|-------|---------|
| UUID format | 1.0 | `550e8400-e29b-41d4-a716-446655440000` |
| ID pattern (letters + numbers) | 1.0 | `ABC-1234`, `SKU001` |
| Positive integer | 0.9 | `1`, `42`, `1000` |
| Alphanumeric code | 0.85 | `A1B2C3` |
| Short string (<50 chars) | 0.6 | `John Smith` |
| Long string | 0.3 | Descriptions, notes |
| Boolean | 0.1 | TRUE, FALSE |

The final score is the average across sampled values.

#### Header Score (10% weight)

Does the column header suggest a key?

**Header pattern matching** (case-insensitive on row 0):

| Pattern Category | Patterns | Score |
|-----------------|----------|-------|
| Strong indicators | id, key, code, sku, ean, upc, isbn, guid, uuid | 1.0 |
| Moderate indicators | number, num, no, ref, reference, identifier | 0.7 |
| Weak indicators | name, title | 0.4 |
| Negative indicators | description, notes, comment, date, amount, total, price | 0.1 |
| Unknown | (none matched) | 0.3 |

---

### 18.4 Single-Column Key Selection

The algorithm attempts to find a single column that can serve as the primary key.

#### Selection Process

1. **Filter by requirements**: Column must meet minimum thresholds:
   - Uniqueness ≥ 0.95 (default): Nearly all values unique
   - Coverage ≥ 0.90 (default): At least 90% of rows have values

2. **Select best**: Among qualifying columns, choose the one with the highest composite score

3. **Return result**: The column index of the best candidate, or None if no column qualifies

---

### 18.5 Composite Key Detection

If no single column qualifies, try pairs of columns as a composite key.

#### Process

1. **Filter candidates**: Only consider columns with composite score ≥ 0.3 (individually not terrible)

2. **Try all pairs**: For each pair of candidate columns:
   - Compute combined uniqueness (how many rows have unique combinations)
   - If combined uniqueness meets the threshold, compute a penalized score

3. **Apply penalty**: Composite keys receive a 10% score penalty (prefer single-column keys when possible)

4. **Select best pair**: Return the pair with highest penalized score, or None if no pair qualifies

#### Why Limit to Pairs

Three or more columns as a composite key is rarely needed and often indicates poor data quality. The algorithm does not attempt combinations larger than pairs.

---

### 18.6 Confidence Reporting

Key inference reports confidence level to guide downstream mode selection.

#### Confidence Levels

| Level | Score Range | Meaning |
|-------|-------------|---------|
| High | > 0.9 | Strong key candidate, safe to use automatically |
| Medium | 0.7 - 0.9 | Reasonable candidate, may want to confirm with user |
| Low | < 0.7 | Weak or no candidate, fall back to spreadsheet mode |

#### Inference Process

1. **Score all columns**: Compute ColumnKeyScore for each column in the grid
2. **Try single-column key**: If a column qualifies, use it with its score's confidence
3. **Try composite key**: If no single column qualifies, try pairs with penalized scoring
4. **Fall back**: If neither works, return empty key list with Low confidence

---

### 18.7 Fallback Behavior

When no reliable key is detected, the engine decides mode based on confidence and configuration.

#### Decision Priority

1. **Explicit configuration**: If user forced spreadsheet mode, use it regardless of inference
2. **Explicit keys**: If user specified key columns, use database mode with those columns
3. **Inferred keys**: Otherwise, decision depends on inference confidence

#### Confidence-Based Actions

| Confidence | Action |
|------------|--------|
| High | Use inferred key for database mode |
| Medium | Use inferred key, but include a warning in the result |
| Low / None | Fall back to spreadsheet mode |

The warning for medium confidence allows the caller to surface this to users, who can then confirm or override the key selection.

---

### 18.8 Key Inference Output

The key inference result includes:

| Field | Description |
|-------|-------------|
| `key_columns` | The inferred key column indices (empty if none found) |
| `confidence` | How reliable the inference is (High/Medium/Low) |
| `scores` | Detailed scores for all columns (for debugging/display) |
| `recommendation` | Human-readable explanation of the inference decision |

---

### 18.9 Complexity

- Score all columns: O(R × C) for extracting values, O(R) per column for scoring
- Check pairs: O(C²) pairs, O(R) each to check uniqueness
- **Total**: O(R × C²)

Since C is small (typically ≤ 100), this is O(R) in practice.

---

# Part VI: Cell-Level Comparison

---

## Section 19: Aligned Cell Comparison

### 19.1 Purpose

After structural alignment (rows and columns), the engine must compare individual cells within matched row pairs to identify value-level changes. This phase produces the granular cell edit operations that users ultimately see.

**Inputs:**
- List of matched row pairs from Phase 4: `(row_a, row_b)`
- Column mapping from column alignment: `col_a → col_b` or unmapped
- The original grid views with cell data

**Outputs:**
- List of cell-level edit operations with old/new values and change classification

---

### 19.2 Merge-Style Iteration

For each matched row pair, the engine performs a coordinated scan of both rows' cells. This is conceptually similar to the merge step in merge-sort: two sorted sequences (cells ordered by column) are traversed in parallel.

The iteration must respect the column mapping. When comparing cell at column 3 in grid A, the engine looks up which column in grid B corresponds to column 3 (if any), then compares those specific cells.

**Three cases arise during iteration:**

1. **Both cells exist**: Compare values and formulas; emit edit if different
2. **Cell exists in A but not B**: The cell was cleared (unless the entire column was removed)
3. **Cell exists in B but not A**: The cell was added (unless it's in a newly added column)

---

### 19.3 Cell Equality Rules

Cell equality is type-aware and must handle Excel's value semantics correctly.

**Numeric Comparison:**
Numbers require epsilon-based comparison due to floating-point representation. The engine should use both absolute tolerance (for values near zero) and relative tolerance (for large values). Special values require explicit handling: NaN equals NaN for diff purposes, and infinities match only if they have the same sign.

**String Comparison:**
Strings are compared exactly, preserving case sensitivity. Leading/trailing whitespace is significant. Unicode normalization should be applied consistently (NFC recommended).

**Boolean and Error Comparison:**
Direct equality check. Error types (DIV/0, N/A, VALUE, etc.) must match exactly.

**Empty Cell Semantics:**
An empty cell equals another empty cell. A cell containing an empty string is NOT equal to a truly empty cell—this distinction matters for some Excel workflows.

**Formula Consideration:**
Two cells with identical values but different formulas are NOT equal. The cell comparison must check both the computed value AND the formula text (or defer to Section 20 for semantic formula comparison).

---

### 19.4 Change Classification

When cells differ, the edit should be classified to help users understand the nature of the change:

| Classification | Condition |
|----------------|-----------|
| **ValueChanged** | Same type, different value |
| **TypeChanged** | Different value types (e.g., number → string) |
| **FormulaChanged** | Formula text differs |
| **FormulaResultChanged** | Same formula, different computed value |
| **Cleared** | Non-empty cell became empty |
| **Added** | Empty cell became non-empty |
| **FormulaAdded** | Plain value replaced with formula |
| **FormulaRemoved** | Formula replaced with plain value |

This classification enables the UI to display changes with appropriate context (e.g., highlighting formula changes differently from value changes).

---

### 19.5 Column Mapping Considerations

The column alignment produces three categories of columns:

**Matched Columns**: Exist in both grids at potentially different positions. Cell comparison proceeds normally using the mapping.

**Removed Columns**: Exist only in grid A. Cells in these columns should NOT generate individual "cleared" operations—the column removal subsumes them. This prevents noise (e.g., 10,000 "cell cleared" operations when a column is deleted).

**Added Columns**: Exist only in grid B. Whether to emit individual "cell added" operations for non-empty cells is a configuration choice. Some users want the detail; others find it redundant with the column addition.

**Moved Columns**: Exist in both grids but at different positions. The engine must correctly associate column 3 in A with column 7 in B (for example) and compare cells accordingly. The cell edit output should include both column positions for clarity.

---

### 19.6 Handling Sparse Data

Excel grids are typically sparse—most cells are empty. The engine should only iterate over non-empty cells, not the full column range. This is achieved by:

1. Iterating the non-empty cells of row A
2. For each, looking up the corresponding cell in row B via the column mapping
3. Separately scanning row B for cells that have no corresponding source in row A

This approach ensures O(non-empty cells) complexity rather than O(total columns).

---

### 19.7 Parallelization

Cell comparison across different row pairs is embarrassingly parallel—there are no dependencies between comparisons of (row 1A, row 1B) and (row 2A, row 2B). The engine should:

1. Partition matched row pairs across available threads
2. Each thread independently computes cell edits for its partition
3. Results are collected and sorted for deterministic output

The final sort key should be (row_a, col_a) to ensure consistent ordering regardless of parallel execution order.

---

### 19.8 Output Structure

Each cell edit record contains:
- Source position (row_a, col_a) — set to sentinel value if cell was added
- Target position (row_b, col_b) — set to sentinel value if cell was cleared
- Old cell state (value, formula, type)
- New cell state (value, formula, type)
- Change classification
- Optional: detailed formula diff (see Section 20)

---

### 19.9 Complexity

- Per-row comparison: O(cells in row) — linear in non-empty cells
- All matched rows: O(M_matched) where M_matched = total cells in matched rows
- This is bounded by O(M) where M = total non-empty cells in both grids
- Parallelization reduces wall-clock time proportionally to available cores

---

## Section 20: Formula Semantic Analysis

### 20.1 Problem Statement

Simple string comparison of formulas produces misleading results in several common scenarios:

| Scenario | String Result | Correct Result |
|----------|---------------|----------------|
| `=A1+B1` vs `=B1+A1` | Different | Equivalent (addition is commutative) |
| `=sum(A1:A10)` vs `=SUM(A1:A10)` | Different | Equivalent (case-insensitive) |
| `=A5*2` → `=A6*2` after row insert | Different | Equivalent (reference shifted mechanically) |
| `=A1*2` vs `=A1*3` | Different | Actually different (semantic change) |

Without semantic analysis, users see noise: formulas reported as "changed" that are actually equivalent, or mechanical reference shifts reported as edits when they're just structural consequences.

---

### 20.2 Solution: Abstract Syntax Tree Representation

Formulas must be parsed into an Abstract Syntax Tree (AST) that captures structure rather than text. The AST represents:

**Literals**: Numbers, strings, booleans, and error values as leaf nodes.

**References**: Cell references with row/column indices and absolute/relative markers. Range references as pairs of cell references. Named ranges as symbolic references. Sheet-qualified references for cross-sheet formulas.

**Operators**: Binary operators (arithmetic, comparison, concatenation) and unary operators (negation, percentage) as interior nodes with their operands as children.

**Functions**: Function name plus an ordered list of argument expressions. The function name should be stored in normalized (uppercase) form.

**Special Constructs**: Array literals, structured table references ([@Column] syntax), implicit intersection (@), and dynamic array spill references (A1#).

---

### 20.3 Canonicalization

Before comparison, the AST must be transformed into a canonical form that eliminates syntactic variations while preserving semantics.

**Commutative Operator Normalization:**
For addition, multiplication, equality, and inequality operators, the operands should be sorted into a deterministic order. The sort key should be based on node type first (literals before references before operators), then by content within type. After canonicalization, `A1+B1` and `B1+A1` produce identical ASTs.

**Commutative Function Normalization:**
Functions with order-independent arguments (SUM, PRODUCT, MAX, MIN, AVERAGE, AND, OR, GCD, LCM) should have their argument lists sorted using the same ordering. This ensures `SUM(B1,A1)` canonicalizes identically to `SUM(A1,B1)`.

**Function Name Normalization:**
All function names should be converted to uppercase. This handles case variations in user input.

**Whitespace and Formatting:**
The AST naturally ignores whitespace since parsing discards it. No explicit normalization needed.

---

### 20.4 Semantic Equality

Two formulas are semantically equal if and only if their canonical ASTs are structurally identical. The comparison is recursive:

- Literals match if their values match (with floating-point tolerance for numbers)
- References match if they refer to the same cell(s) with the same absolute/relative mode
- Operators match if the operator type matches and all operands match recursively
- Functions match if the name matches (after uppercasing) and all arguments match in order

This definition means `=A1+B1` equals `=B1+A1` (after canonicalization), but `=A1+B1` does NOT equal `=A1-B1` (different operator).

---

### 20.5 Reference Shift Detection

When rows or columns are inserted or deleted, Excel automatically adjusts references in all formulas. A formula might change from `=A5` to `=A6` purely because row 5 was inserted above. This is not a semantic change—it's a mechanical consequence of structural changes.

The engine should detect uniform reference shifts:

1. Extract all cell/range references from both the old and new formula
2. If the formulas have the same structure (same operators, functions, same number of references), compute the delta for each reference
3. If all relative references shifted by the same (row_delta, col_delta), report this as a "reference shift" rather than a semantic change
4. Absolute references (with $) should NOT shift—if they differ, it's a real change

The detected shift should be validated against the structural alignment: if 2 rows were inserted above this formula's position, the expected row_delta is +2. Matching shifts are reported as mechanical; non-matching shifts indicate intentional formula modification.

---

### 20.6 Formula Diff Output

When formulas are semantically different, the engine should provide structured information about the difference:

**Unchanged**: Formulas are byte-identical.

**Semantic Equivalent**: Formulas differ textually but are semantically identical after canonicalization. Example: `=A1+B1` vs `=B1+A1`.

**References Shifted**: Formulas differ only by a uniform reference adjustment that corresponds to detected row/column insertions. Report the shift delta.

**Semantic Different**: Formulas have actual structural differences. Optionally, compute a tree edit distance to identify specifically what changed (which function, which argument, which reference).

---

### 20.7 Tree Edit Distance (Optional)

For formulas that differ semantically, a tree edit distance algorithm can identify the minimal set of changes:

**Keep**: Subtree is unchanged
**Replace**: One subtree replaced by another  
**Insert**: New subtree added
**Delete**: Subtree removed
**UpdateReference**: Cell or range reference changed (special case worth distinguishing)

Standard tree edit distance algorithms (Zhang-Shasha, APTED) can be applied. However, since Excel formulas are typically shallow (depth ≤ 10) and small (< 50 nodes), a simplified recursive diff usually suffices.

This detailed diff is optional—it's valuable for power users auditing formula changes but adds implementation complexity.

---

### 20.8 Integration with Cell Comparison

When comparing two cells that both contain formulas:

1. First check if formula text is identical (fast path—skip parsing)
2. If different, parse both formulas into ASTs
3. Canonicalize both ASTs
4. Compare canonical ASTs for semantic equality
5. If semantically equal, report "formula rewritten but equivalent"
6. If different, check for uniform reference shift correlated with structural changes
7. If shift detected and validated, report "references shifted by (row, col)"
8. Otherwise, report semantic difference with optional tree diff

When one cell has a formula and the other doesn't, report formula addition or removal appropriately.

---

### 20.9 Handling Parse Errors

Some formulas may fail to parse (malformed syntax, unsupported features). The engine should:

1. Attempt to parse both formulas
2. If both fail to parse, fall back to string comparison
3. If one parses and one doesn't, report as different
4. Log parse failures for diagnostic purposes but don't crash the diff

---

### 20.10 Configuration Options

**enable_semantic_comparison** (default: true): When false, use simple string comparison for formulas. Faster but produces more noise.

**detect_reference_shifts** (default: true): When true, identify and report mechanical reference shifts. Reduces noise for sheets with insertions/deletions.

**compute_tree_diff** (default: false): When true, compute detailed tree edit distance for different formulas. More informative but more expensive.

---

### 20.11 Complexity

- Formula parsing: O(formula length)
- Canonicalization: O(N log N) where N = AST nodes (due to child sorting)
- AST equality: O(N)
- Reference shift detection: O(number of references)
- Tree diff: O(N²) worst case, O(N) typical for small trees

Most formulas are short (< 50 characters, < 20 AST nodes), so these costs are negligible per formula. For sheets with thousands of formulas, parsing can be parallelized.

---

# Part VII: Result Assembly & Output

---

## Section 21: DiffOp Schema and Operation Assembly

### 21.1 Purpose

After alignment and cell-level comparison, the engine has produced various intermediate results: matched row pairs, unmatched rows, move candidates, column mappings, and cell edits. The result assembly phase transforms these intermediate results into a coherent, user-consumable diff output.

**Inputs from earlier phases:**
- Row alignment: matched pairs, deletions (rows in A only), insertions (rows in B only)
- Column alignment: matched pairs, added columns, removed columns, column moves
- Validated moves: row block moves, column block moves, rectangular moves
- Cell edits: value changes, formula changes, type changes for matched row/column pairs

**Output:**
- A sorted list of DiffOp operations representing all changes
- Summary statistics (counts by operation type)
- Optional: structured metadata for UI rendering

---

### 21.2 DiffOp Taxonomy

The engine produces a finite set of operation types, organized into categories:

#### Row Operations

| Operation | Meaning |
|-----------|---------|
| **RowAdded** | A new row exists in grid B that has no corresponding row in grid A |
| **RowRemoved** | A row exists in grid A that has no corresponding row in grid B |
| **BlockMovedRows** | A contiguous block of rows was relocated within the sheet |

#### Column Operations

| Operation | Meaning |
|-----------|---------|
| **ColumnAdded** | A new column exists in grid B that has no corresponding column in grid A |
| **ColumnRemoved** | A column exists in grid A that has no corresponding column in grid B |
| **BlockMovedColumns** | A contiguous block of columns was relocated within the sheet |

#### Cell Operations

| Operation | Meaning |
|-----------|---------|
| **CellEdited** | A cell's content changed between aligned positions |

#### Composite Operations

| Operation | Meaning |
|-----------|---------|
| **BlockMovedRect** | A rectangular region was relocated (correlated row and column move) |

---

### 21.3 Operation Representation

Each operation type requires specific fields to fully describe the change.

#### RowAdded

| Field | Description |
|-------|-------------|
| `row_b` | Row index in grid B where the new row appears |
| `content_hash` | Hash of the row content (for display/debugging) |

The actual cell content is retrieved from grid B when needed for display.

#### RowRemoved

| Field | Description |
|-------|-------------|
| `row_a` | Row index in grid A that was removed |
| `content_hash` | Hash of the row content |

#### BlockMovedRows

| Field | Description |
|-------|-------------|
| `source_range` | Start and end row indices in grid A |
| `dest_range` | Start and end row indices in grid B |
| `is_fuzzy` | Whether the move involves internal edits |
| `similarity` | If fuzzy, the measured similarity (0.0-1.0) |

The block size can be computed as `source_range.end - source_range.start`.

#### ColumnAdded

| Field | Description |
|-------|-------------|
| `col_b` | Column index in grid B where the new column appears |
| `header_value` | If the first row is a header, the column header (for display) |

#### ColumnRemoved

| Field | Description |
|-------|-------------|
| `col_a` | Column index in grid A that was removed |
| `header_value` | If applicable, the column header |

#### BlockMovedColumns

| Field | Description |
|-------|-------------|
| `source_range` | Start and end column indices in grid A |
| `dest_range` | Start and end column indices in grid B |
| `is_fuzzy` | Whether the move involves internal edits |

#### CellEdited

| Field | Description |
|-------|-------------|
| `row_a`, `col_a` | Cell position in grid A |
| `row_b`, `col_b` | Cell position in grid B (may differ due to row/column shifts) |
| `old_value` | The cell's value in grid A |
| `new_value` | The cell's value in grid B |
| `old_formula` | The cell's formula in grid A (if any) |
| `new_formula` | The cell's formula in grid B (if any) |
| `change_type` | Classification of the change (see Section 19.4) |
| `formula_diff` | If formulas differ, optional structured diff (see Section 20) |

#### BlockMovedRect

| Field | Description |
|-------|-------------|
| `source_rows` | Row range in grid A |
| `source_cols` | Column range in grid A |
| `dest_rows` | Row range in grid B |
| `dest_cols` | Column range in grid B |
| `internal_edits` | Count of cell edits within the moved block |

---

### 21.4 Operation Assembly Process

The assembly process transforms raw alignment results into the final operation list.

#### Step 1: Collect Structural Operations

First, gather all row and column structural changes:

1. **Row deletions**: For each row in grid A marked as "deleted" (no match in B), check if it belongs to a validated move block. If yes, skip it (the move operation will cover it). If no, emit a RowRemoved operation.

2. **Row insertions**: For each row in grid B marked as "inserted" (no match in A), check if it belongs to a validated move block. If yes, skip it. If no, emit a RowAdded operation.

3. **Row moves**: For each validated row block move, emit a BlockMovedRows operation.

4. **Column operations**: Apply the same logic to column deletions, insertions, and moves.

5. **Rectangular moves**: For each detected correlated rectangular move, emit a BlockMovedRect operation and suppress the corresponding row and column move operations to avoid redundancy.

#### Step 2: Collect Cell Operations

For each matched row pair, collect the cell edit operations produced by Phase 5 (cell-level diff). These operations already contain full information about the change.

**Filtering considerations:**

- If a cell is in a removed column, do NOT emit a CellEdited for it becoming empty—the column removal subsumes this change.
- If a cell is in an added column, the decision to emit CellEdited for its population depends on configuration. By default, only emit if the row was matched (not added), since cell additions in added rows are implicit.
- If a cell is in a moved row or column, adjust the position information to reflect the move, but still emit the cell edit if the content changed.

#### Step 3: Coalesce Consecutive Operations

For presentation clarity, consecutive operations of the same type can be coalesced:

**Row additions**: If rows 5, 6, 7, 8 are all added, they can be reported as a single "4 rows added starting at row 5" rather than four separate operations. The coalesced form is more compact for summary display while the detailed form is available for drill-down.

**Row deletions**: Similarly, consecutive deleted rows can be grouped.

**Cell edits**: Cell edits within the same row can be grouped for row-centric display.

Coalescing is optional and controlled by output format configuration. The underlying operation list remains granular.

---

### 21.5 Subsumption Rules

Certain operations subsume others, preventing redundant or confusing output:

| Primary Operation | Subsumed Operations |
|-------------------|---------------------|
| RowRemoved | All CellEdited/Cleared for cells in that row |
| RowAdded | All CellAdded for cells in that row |
| ColumnRemoved | All CellEdited/Cleared for cells in that column |
| ColumnAdded | All CellAdded for cells in that column |
| BlockMovedRows | Individual RowRemoved/RowAdded for rows in the block |
| BlockMovedColumns | Individual ColumnRemoved/ColumnAdded for columns in the block |
| BlockMovedRect | The corresponding BlockMovedRows and BlockMovedColumns |

The subsumption principle: **a change should be reported at the highest semantic level that captures it**. Reporting both "row 5 removed" and "cells A5:Z5 cleared" is redundant; only the row removal should appear.

---

### 21.6 Position Normalization

Operations reference positions in both grid A and grid B. For user-facing output, positions may need normalization:

**A-relative positions**: Describe where things were in the original (grid A)
**B-relative positions**: Describe where things are in the modified version (grid B)
**Delta positions**: Describe the offset (e.g., "moved from row 10 to row 5, delta -5")

The operation structure stores both A and B positions to support all presentation needs. The UI layer can choose which to emphasize based on context (e.g., "what changed from the original" vs. "what the result looks like").

---

### 21.7 Operation Ordering

The final operation list must be sorted for deterministic, intuitive output.

#### Primary Sort Key: Operation Category

Operations are ordered by category priority:

1. **Structural removals** (RowRemoved, ColumnRemoved) — show what was taken away first
2. **Structural additions** (RowAdded, ColumnAdded) — then what was added
3. **Structural moves** (BlockMovedRows, BlockMovedColumns, BlockMovedRect) — then relocations
4. **Cell edits** (CellEdited) — finally, in-place changes

#### Secondary Sort Key: Position

Within each category, operations are sorted by position:

- Row operations: by row index (in the relevant grid)
- Column operations: by column index
- Cell edits: by (row, column) lexicographically

#### Tertiary Sort Key: Tie-Breaking

For operations at the same position (rare), use a deterministic tie-breaker based on content hash or operation type ordinal.

The goal is that the same inputs always produce the same output order, regardless of the order in which operations were discovered during parallel processing.

---

### 21.8 Summary Statistics

In addition to the operation list, the result should include aggregate statistics:

| Statistic | Description |
|-----------|-------------|
| `total_operations` | Total count of all operations |
| `rows_added` | Count of RowAdded operations |
| `rows_removed` | Count of RowRemoved operations |
| `rows_moved` | Count of rows covered by BlockMovedRows |
| `columns_added` | Count of ColumnAdded operations |
| `columns_removed` | Count of ColumnRemoved operations |
| `columns_moved` | Count of columns covered by BlockMovedColumns |
| `cells_edited` | Count of CellEdited operations |
| `cells_by_change_type` | Breakdown of cell edits by classification |
| `similarity_score` | Overall grid similarity (0.0-1.0) |

These statistics enable quick assessment without iterating the full operation list.

---

### 21.9 Complexity

- Collecting structural operations: O(R + C) where R = rows, C = columns
- Collecting cell operations: O(cell edits)
- Checking subsumption: O(operations) with hash set lookups for move membership
- Sorting: O(ops × log(ops))

**Total: O(M + ops × log(ops))** where M = total cells, ops = number of diff operations.

In practice, ops << M (most cells are unchanged), so the sorting cost is dominated by cell comparison time from earlier phases.

---

## Section 22: Deterministic Output Guarantees

### 22.1 Problem Statement

When the diff engine processes the same two grids, the output must be identical—byte-for-byte—across:

- Multiple runs on the same machine
- Runs on different machines with different architectures
- Runs using different thread counts or parallelism configurations
- Runs using different compiler versions (for the same source code)

Non-deterministic output causes multiple problems:

**Testing**: Automated tests comparing expected vs. actual output fail intermittently.

**Caching**: Results cannot be cached reliably if the same inputs produce different outputs.

**Auditing**: Financial and compliance use cases require reproducible comparisons.

**User Trust**: Seeing different results for the same comparison erodes confidence.

---

### 22.2 Sources of Non-Determinism

Several language and runtime features can introduce non-determinism:

#### HashMap/HashSet Iteration Order

Standard hash maps use randomized hashing for security (preventing hash-flooding DoS attacks). This means iterating over a HashMap produces keys in arbitrary order that changes between runs.

**Impact**: If the engine processes row hashes, column hashes, or operations by iterating a HashMap, the processing order—and potentially the output—varies.

#### Parallel Execution Order

When work is distributed across multiple threads, the completion order is non-deterministic. Thread 2 might finish before thread 1 depending on scheduling.

**Impact**: If parallel results are collected into a vector in completion order, the final vector order varies.

#### Floating-Point Operations

IEEE 754 floating-point arithmetic is associative in exact math but not in practice. `(a + b) + c` may differ from `a + (b + c)` due to rounding.

**Impact**: If hash computation or similarity calculation uses floating-point arithmetic in parallel with different reduction orders, results may vary.

#### System Time Dependencies

Using current time for any computation (seeding random numbers, generating IDs) introduces variation.

**Impact**: Any use of timestamps or random values in the diff computation breaks determinism.

---

### 22.3 HashMap Determinism

All internal HashMaps and HashSets must use a deterministic hasher with a fixed seed.

#### Fixed-Seed Hasher

The hasher should be initialized with a constant seed known at compile time. This ensures that:

- The same key always hashes to the same bucket
- Iteration order is consistent for the same set of keys
- Different platforms (x86, ARM, WASM) produce identical behavior

A suitable approach is to use a hasher implementation that accepts a seed parameter, initialized to a fixed constant like `0x517cc1b727220a95`.

#### When to Use Deterministic Hash Maps

Use deterministic hash structures for:
- All alignment data structures (hash-to-row mappings, frequency tables)
- Move candidate tracking
- Cell edit collection during parallel processing
- Any structure iterated during output generation

Standard (random-seeded) hash maps can be used for:
- Internal caches where iteration order doesn't affect output
- Temporary structures whose contents are immediately sorted before use

---

### 22.4 Parallel Result Collection

When parallel operations produce results, the collection strategy must ensure deterministic ordering.

#### Position-Tagged Collection

Each parallel work unit should associate its results with a source position (row index, gap index, etc.):

1. Partition work by position (e.g., each thread processes specific row ranges)
2. Each thread produces results tagged with their source position
3. After parallel collection, sort results by source position
4. Only after sorting proceed to downstream processing

#### Example: Parallel Cell Diff

When comparing cells across matched rows in parallel:

1. Assign each matched row pair a sequential index (0, 1, 2, ...)
2. Each thread processes its assigned row pairs, producing cell edits tagged with the row pair index
3. Collect all cell edits into a single vector
4. Sort by (row_pair_index, column) before output

This ensures that whether thread 1 or thread 2 finishes first, the final output order depends only on position, not execution timing.

---

### 22.5 Deterministic Sort Keys

All sorting operations must use fully-specified, deterministic comparison keys.

#### Complete Ordering

For DiffOp sorting, the comparison function must define a total order with no ties that depend on memory addresses or pointer values. The sort key should include:

1. Operation type ordinal (RowRemoved=0, RowAdded=1, etc.)
2. Primary position (row index for row operations, column for column operations)
3. Secondary position (column for cell edits)
4. Tertiary tie-breaker (content hash or operation-specific fields)

#### Stable Sorting

When tie-breakers are needed, they should be based on content, not on discovery order. For example, if two operations genuinely have identical sort keys, compare their content hashes rather than relying on stable sort preservation of input order (which may have been non-deterministic).

---

### 22.6 Floating-Point Determinism

Floating-point computations must be handled carefully to ensure consistent results.

#### Numeric Comparison

Cell value comparison uses epsilon-based floating-point comparison. The comparison function should use a fixed tolerance and handle special cases (NaN, infinity) deterministically:

- NaN == NaN evaluates to true for diff purposes (both cells contain the same special value)
- +Infinity == +Infinity, -Infinity == -Infinity
- 0.0 == -0.0 (signed zero normalization)

#### Similarity Calculations

When computing similarity scores (Jaccard, Dice, etc.), use the same formula consistently:
- Integer operations where possible (count intersections and unions)
- Division only at the end
- Defined behavior for edge cases (empty sets → similarity 0.0 or 1.0 by convention)

#### Hash Computation for Floats

When hashing floating-point values:
- Normalize to a canonical bit representation (e.g., convert -0.0 to 0.0)
- Use the bit representation, not the formatted string
- Handle NaN by mapping to a single canonical NaN value

---

### 22.7 String Ordering

When strings are used as sort keys, the comparison must be consistent.

#### Byte-Wise Comparison

For determinism across platforms, compare strings as byte sequences (UTF-8 encoded), not using locale-sensitive collation. This ensures "apple" < "banana" produces the same result everywhere.

#### Unicode Normalization

If strings may have different Unicode representations for the same visual content (e.g., é as single codepoint vs. e + combining accent), normalize to NFC before any comparison or hashing.

---

### 22.8 Testing Determinism

The engine should include tests that verify determinism:

#### Multi-Run Tests

For a given input pair:
1. Run the diff 10 times
2. Serialize each result
3. Assert all 10 results are byte-identical

#### Thread Variation Tests

For a given input pair:
1. Run with 1 thread, save result
2. Run with 2 threads, save result
3. Run with 4 threads, save result
4. Assert all results are identical

#### Cross-Platform Tests (CI)

Run the same test inputs on:
- x86_64 Linux
- ARM64 macOS
- WASM (Node.js runtime)

Assert identical results across platforms.

---

### 22.9 Determinism Violations: Detection and Prevention

#### Debug Mode Checks

In debug builds, add assertions that detect common determinism violations:

- Assert that collections being iterated were sorted since their last modification
- Assert that hash map contents are processed in sorted key order when order matters
- Assert that parallel results are sorted before consumption

#### Code Review Guidelines

Flag these patterns in code review:
- HashMap iteration without subsequent sorting
- Collecting parallel results without position tags
- Using system time or random values in computation paths
- Floating-point reduction in parallel without defined order

---

### 22.10 Performance Impact

Determinism measures have minimal performance impact:

| Measure | Cost |
|---------|------|
| Fixed-seed hasher | ~0% (hasher is equally fast) |
| Parallel result tagging | ~1% (extra position field per result) |
| Post-parallel sorting | ~2-5% (typically small result sets) |
| Deterministic float handling | ~0% (same operations, just defined order) |

The total overhead for determinism guarantees is under 5%, which is acceptable for the benefits of reproducible output.

---

## Section 23: Output Formatting and Serialization

### 23.1 Purpose

The diff result must be serialized into formats suitable for different consumers: UI rendering, programmatic processing, storage, and human inspection. This section specifies the supported output formats and their characteristics.

**Requirements:**
- Support multiple output formats from a single internal representation
- Preserve all semantic information without loss
- Enable efficient streaming for large result sets
- Provide both human-readable and machine-parseable options

---

### 23.2 Output Format Overview

| Format | Primary Use | Characteristics |
|--------|-------------|-----------------|
| JSON | API responses, web UI | Structured, verbose, widely supported |
| JSON Lines | Streaming, large diffs | One operation per line, streamable |
| Binary | Internal caching, performance | Compact, fast serialization |
| Text Summary | CLI output, logs | Human-readable, lossy |
| Patch Format | Apply changes programmatically | Minimal, action-oriented |

---

### 23.3 JSON Output Format

The primary output format for API and UI consumption.

#### Top-Level Structure

The JSON output contains:

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Format version for compatibility (e.g., "1.0") |
| `metadata` | object | Information about the comparison |
| `summary` | object | Aggregate statistics |
| `operations` | array | The list of diff operations |

#### Metadata Object

| Field | Type | Description |
|-------|------|-------------|
| `grid_a_rows` | integer | Row count of grid A |
| `grid_a_cols` | integer | Column count of grid A |
| `grid_b_rows` | integer | Row count of grid B |
| `grid_b_cols` | integer | Column count of grid B |
| `mode` | string | "spreadsheet" or "database" |
| `key_columns` | array | If database mode, the key column indices |
| `timestamp` | string | ISO 8601 timestamp of comparison |
| `hash_algorithm` | string | Hash algorithm used ("xxhash64" or "blake3_128") |

#### Summary Object

| Field | Type | Description |
|-------|------|-------------|
| `total_operations` | integer | Total count of operations |
| `rows_added` | integer | Count of RowAdded operations |
| `rows_removed` | integer | Count of RowRemoved operations |
| `rows_moved` | integer | Total rows in BlockMovedRows operations |
| `columns_added` | integer | Count of ColumnAdded operations |
| `columns_removed` | integer | Count of ColumnRemoved operations |
| `columns_moved` | integer | Total columns in BlockMovedColumns operations |
| `cells_edited` | integer | Count of CellEdited operations |
| `similarity` | number | Overall similarity score (0.0-1.0) |

#### Operation Objects

Each operation is serialized with a `type` discriminator and type-specific fields.

**RowAdded:**
```
{
  "type": "row_added",
  "row_b": 42,
  "content_preview": "First three cells..."
}
```

**RowRemoved:**
```
{
  "type": "row_removed",
  "row_a": 17,
  "content_preview": "First three cells..."
}
```

**BlockMovedRows:**
```
{
  "type": "block_moved_rows",
  "source_start": 10,
  "source_end": 15,
  "dest_start": 50,
  "dest_end": 55,
  "is_fuzzy": false,
  "similarity": 1.0
}
```

**CellEdited:**
```
{
  "type": "cell_edited",
  "row_a": 5,
  "col_a": 3,
  "row_b": 7,
  "col_b": 3,
  "old_value": "123.45",
  "new_value": "200.00",
  "old_type": "number",
  "new_type": "number",
  "change_class": "value_changed"
}
```

---

### 23.4 JSON Lines Format

For streaming large diffs, JSON Lines (NDJSON) provides one operation per line.

#### Format

Each line is a complete JSON object representing one operation. The first line contains metadata, subsequent lines contain operations.

```
{"type":"metadata","version":"1.0","grid_a_rows":50000,...}
{"type":"summary","total_operations":1234,...}
{"type":"row_removed","row_a":5,...}
{"type":"cell_edited","row_a":10,"col_a":2,...}
```

#### Advantages

- **Streamable**: Consumer can process operations before the entire output is generated
- **Memory-efficient**: No need to hold entire operation list in memory
- **Resumable**: If processing fails, can resume from a specific line number
- **Appendable**: Additional operations can be appended without reparsing

---

### 23.5 Binary Format

For internal caching and maximum performance, a compact binary format.

#### Design Principles

- Fixed-size headers for random access
- Variable-length data sections with length prefixes
- Little-endian byte order
- Optional compression (LZ4 for speed)

#### Structure

```
[Header: 64 bytes]
  - Magic bytes: 4 bytes ("EDIF")
  - Version: 2 bytes
  - Flags: 2 bytes (compression, etc.)
  - Operation count: 4 bytes
  - Metadata offset: 8 bytes
  - Summary offset: 8 bytes
  - Operations offset: 8 bytes
  - Reserved: 28 bytes

[Metadata section]
  - Grid dimensions, mode, etc.

[Summary section]
  - Statistics as fixed-size integers

[Operations section]
  - Each operation: type byte + type-specific payload
```

The binary format is 5-10x more compact than JSON and 10-50x faster to serialize/deserialize.

---

### 23.6 Text Summary Format

For CLI output and logs, a human-readable summary.

#### Format

```
=== Grid Diff Summary ===
Grid A: 1000 rows × 50 columns
Grid B: 1050 rows × 50 columns
Mode: Spreadsheet

Changes:
  + 52 rows added
  - 2 rows removed
  ↔ 100 rows moved (2 blocks)
  Δ 156 cells edited

Details:
  RowRemoved: row 15
  RowRemoved: row 892
  BlockMovedRows: rows 100-149 → rows 200-249
  BlockMovedRows: rows 500-520 → rows 50-70
  ... (showing first 10 of 156 cell edits)
  CellEdited: A5: "old" → "new"
  CellEdited: B10: 100 → 200
```

This format is lossy—it doesn't include all details, but provides a quick overview.

---

### 23.7 Patch Format

For programmatic application of changes, a structured patch format.

#### Design

The patch format focuses on minimal information needed to transform grid A into grid B.

**Row operations** specify position and content:
```
-ROW 15
+ROW 42 ["value1", "value2", ...]
MOVE_ROWS 100-149 → 200
```

**Cell operations** specify position and new value:
```
CELL A5 = "new value"
CELL B10:formula = "=SUM(A1:A10)"
```

The patch format enables "apply diff" functionality where the diff result can be used to update a copy of grid A to match grid B.

---

### 23.8 Content Previews

For display purposes, operations may include content previews—abbreviated representations of cell values.

#### Preview Rules

- **String values**: First 50 characters, with "..." if truncated
- **Numbers**: Full precision up to 15 significant digits
- **Formulas**: First 30 characters of formula text
- **Row previews**: First 3 non-empty cells, comma-separated

#### Sanitization

Content previews must be sanitized for the output format:
- JSON: Escape control characters, quotes
- HTML: Escape `<`, `>`, `&`
- Plain text: Replace non-printable characters with placeholders

---

### 23.9 Localization Considerations

Numeric and date formatting may need localization for display:

| Element | Default | Localized Example |
|---------|---------|-------------------|
| Numbers | Period decimal | Comma decimal (1.234,56) |
| Dates | ISO 8601 | Locale format (31/12/2025) |
| Booleans | TRUE/FALSE | Language-specific |

The internal representation always uses neutral formats (ISO dates, period decimals). Localization is applied only at the display layer.

---

### 23.10 Compression

For large diffs transmitted over networks or stored:

| Algorithm | Use Case | Ratio | Speed |
|-----------|----------|-------|-------|
| None | Small diffs (<10KB) | 1:1 | Fastest |
| LZ4 | Real-time, moderate compression | 2-3:1 | Very fast |
| Zstd | Storage, better compression | 4-6:1 | Fast |
| Gzip | Maximum compatibility | 5-8:1 | Moderate |

The format header indicates compression so consumers can decompress appropriately.

---

### 23.11 Streaming Output

For very large diffs, the output should be streamable without buffering the entire result.

#### Requirements

- Begin emitting output before all operations are computed
- Emit operations in deterministic order (see Section 22)
- Provide a mechanism to signal completion or error

#### Implementation Strategy

1. Emit metadata and preliminary summary (with placeholder counts)
2. Stream operations as they are sorted and ready
3. Emit final summary with accurate counts
4. Signal completion

For JSON output, this requires either:
- JSON Lines format (natural streaming)
- Chunked transfer encoding with incremental JSON parsing
- Wrapper format with separate metadata and operation streams

---

### 23.12 Output Size Estimation

Before generating output, estimate the size to choose appropriate strategies.

#### Heuristics

- Operations count × average operation size
- JSON overhead ~2x of raw data
- Binary format ~0.3x of JSON size

If estimated output exceeds thresholds (e.g., 10MB), automatically:
- Use streaming format
- Enable compression
- Truncate content previews

---

# Part VIII: Robustness & Performance

---

## Section 24: Error Handling and Robustness

### 24.1 Design Philosophy

The diff engine must be robust against:

1. **Malformed inputs**: Invalid file formats, corrupted data, unexpected structures
2. **Pathological inputs**: Adversarial patterns that trigger worst-case behavior
3. **Resource exhaustion**: Memory limits, time limits, cancellation
4. **Internal errors**: Bugs, assertion failures, panics

The principle is **graceful degradation**: when something goes wrong, produce the best possible result rather than failing entirely. Report issues clearly but continue processing where feasible.

---

### 24.2 Input Validation

Before processing, validate inputs to catch obvious problems early.

#### Grid Structure Validation

| Check | Error Response |
|-------|----------------|
| Negative row/column indices | Reject with clear error |
| Row count exceeds maximum (1M) | Warn, truncate to maximum |
| Column count exceeds maximum (16K) | Warn, truncate to maximum |
| Sparse grid with >10M non-empty cells | Warn, may degrade performance |

#### Cell Value Validation

| Check | Error Response |
|-------|----------------|
| String exceeds 32KB | Truncate with warning |
| Formula exceeds 8KB | Store as string, skip parsing |
| Invalid UTF-8 in string | Replace invalid bytes, warn |
| NaN in unexpected context | Normalize to canonical NaN |

Validation errors should be collected and reported in the result metadata, not thrown as exceptions that abort processing.

---

### 24.3 Pathological Input Detection

Detect inputs that would cause problematic behavior and adapt accordingly.

#### Extreme Repetition

**Detection**: Compute the ratio of unique row hashes to total rows. If uniqueness < 5%, the grid is highly repetitive.

**Response**: Switch to run-length alignment strategies (Section 7.7). Skip Patience Diff anchor phase (which would find few/no anchors). Limit move detection scope.

#### Nearly Identical Grids

**Detection**: If >99% of rows have hash matches, grids are nearly identical.

**Response**: Fast-path to cell-level diff only. Skip structural alignment overhead.

#### Completely Different Grids

**Detection**: If <1% of rows have hash matches, grids share almost no content.

**Response**: Bail out of alignment. Report entire grid A as "removed" and grid B as "added". This is faster and more honest than attempting to find spurious alignments.

#### Single Dominant Value

**Detection**: One cell value (e.g., 0 or empty) appears in >80% of cells.

**Response**: Exclude dominant value from hash computation. Align based on non-dominant cells only. Prevents false matches where two "all zeros" rows align despite different structures.

---

### 24.4 Timeout and Cancellation

Long-running operations must support timeout and cancellation.

#### Timeout Mechanism

Each phase of the diff has a time budget. If a phase exceeds its budget:

| Phase | Budget (default) | Timeout Response |
|-------|------------------|------------------|
| Preprocessing | 30s | Abort with error |
| Row alignment | 60s | Bail out, mark as "replaced" |
| Column alignment | 10s | Skip, assume identity mapping |
| Cell diff | 60s | Truncate, report partial results |
| Result assembly | 10s | Emit collected operations |

The total timeout defaults to 120s but is configurable.

#### Cancellation

The engine accepts a cancellation token that can be signaled externally (e.g., user clicks "Cancel"). At safe points, the engine checks the token and aborts cleanly if set.

Safe points include:
- Between phases
- Between gap alignments
- Every N rows during cell diff

On cancellation, return partial results with a flag indicating incomplete processing.

---

### 24.5 Memory Pressure Handling

When memory is constrained, degrade gracefully rather than crashing.

#### Detection

The memory tracker (Section 25) monitors allocation. When approaching budget limits:

| Threshold | Response |
|-----------|----------|
| 70% of budget | Log warning, continue |
| 85% of budget | Disable optional features (move detection, fuzzy matching) |
| 95% of budget | Switch to streaming/chunked mode |
| 100% of budget | Abort current operation, proceed with partial results |

#### Recovery Strategies

- **Release caches**: Drop precomputed data that can be recomputed
- **Reduce scope**: Process fewer rows per chunk
- **Simplify algorithms**: Use bail-out strategies instead of full alignment
- **Spill to disk**: For native (non-WASM) builds, swap data to temporary files

---

### 24.6 Internal Error Handling

Bugs in the diff engine should not crash the application.

#### Panic Handling

All entry points should catch panics and convert them to error results:
- Log the panic message and backtrace
- Return an error result indicating internal failure
- Include diagnostic information for bug reports

#### Assertion Failures

Debug assertions verify invariants during development. In production:
- Assertions that fail indicate bugs, not user errors
- Log the failure with context
- Attempt to continue if the invariant is non-critical
- Abort if continuing would produce incorrect results

#### Defensive Coding

The engine assumes inputs may violate documented preconditions:
- Check array bounds before access
- Validate indices received from earlier phases
- Handle None/null cases even when "impossible"

---

### 24.7 Error Reporting

Errors are classified and reported consistently.

#### Error Categories

| Category | Meaning | User Action |
|----------|---------|-------------|
| **InputError** | Problem with input data | Fix the input |
| **ResourceError** | Memory, time, or space limit | Reduce input size or increase limits |
| **InternalError** | Bug in the engine | Report to developers |
| **PartialResult** | Processing incomplete | Review available results |

#### Error Structure

Each error includes:
- Category code
- Human-readable message
- Technical details (for logging)
- Context (which phase, which row, etc.)
- Suggested remediation

---

### 24.8 Result Completeness Flags

The result indicates whether processing completed fully.

| Flag | Meaning |
|------|---------|
| `complete: true` | All phases finished successfully |
| `complete: false` | Some phases aborted or timed out |
| `truncated: true` | Result was truncated (too many operations) |
| `warnings: [...]` | Non-fatal issues encountered |

These flags enable consumers to make informed decisions about trusting the result.

---

## Section 25: Memory Management

### 25.1 Memory Constraints

The diff engine operates under memory constraints, especially in WASM environments.

#### Environment Limits

| Environment | Typical Limit | Target Usage |
|-------------|---------------|--------------|
| Browser WASM | 2-4 GB | ≤1.5 GB |
| Node.js WASM | 4 GB | ≤2 GB |
| Native 64-bit | 16+ GB | ≤4 GB |
| Mobile | 1-2 GB | ≤500 MB |

#### Allocation Breakdown

For a typical diff of two 50K-row grids:

| Component | Estimated Size |
|-----------|----------------|
| Grid A parsed data | 50-200 MB |
| Grid B parsed data | 50-200 MB |
| Row views and metadata | 10-20 MB |
| Hash tables for alignment | 10-50 MB |
| Cell diff working set | 5-20 MB |
| Result operations | 1-50 MB |
| **Total** | 126-540 MB |

The engine must track and control allocation to stay within limits.

---

### 25.2 Memory Budget System

Implement a centralized memory tracking system.

#### Budget Definition

The memory budget specifies:
- **Total limit**: Maximum memory for the entire diff operation
- **Per-grid limit**: Maximum memory for each input grid
- **Working set limit**: Maximum memory for intermediate structures

Defaults:
```
Total limit: 1.5 GB
Per-grid limit: 600 MB
Working set limit: 300 MB
```

#### Tracking Mechanism

The memory tracker maintains:
- Current allocated bytes
- Peak allocated bytes
- Allocation history (for debugging)
- Soft and hard thresholds

All major allocations should go through the tracker:
1. Request allocation with estimated size
2. Tracker checks against budget
3. If within budget, return success and increment counter
4. If over budget, return failure or trigger fallback

---

### 25.3 Allocation Patterns

Different phases have different allocation patterns.

#### Preprocessing

Allocates:
- Row views: Vec of Vec (one per row, cells within)
- Row metadata: Fixed size per row
- String pool: Grows with unique strings

These allocations persist through alignment and cell diff phases.

#### Alignment

Allocates:
- Hash maps for frequency analysis
- Anchor lists
- Move candidate structures
- Gap alignment working memory

Most alignment structures can be released after row alignment completes.

#### Cell Diff

Allocates:
- Cell edit records
- Formula ASTs (if semantic comparison enabled)

These persist to result assembly.

#### Result Assembly

Allocates:
- Final operation list
- Serialization buffers

---

### 25.4 Memory-Efficient Data Structures

Use compact representations to reduce memory footprint.

#### String Interning

Instead of storing each string value directly, use a string pool:
- Store unique strings once
- Reference strings by 4-byte ID
- Savings: 50-80% for repetitive data

See gap_solutions_specification.md Section 7 for full design.

#### Compact Indices

Use the smallest integer type that fits:
- Row indices: u32 (supports up to 4B rows)
- Column indices: u16 (supports up to 65K columns)
- String IDs: u32

#### Sparse Structures

Never allocate dense arrays for full grid dimensions:
- Cell storage: HashMap<(row, col), Cell> or Vec<(col, Cell)> per row
- Avoid: Vec<Vec<Cell>> sized to nrows × ncols

---

### 25.5 Streaming and Chunking

For grids exceeding memory budget, process in chunks.

#### Pre-Diff Size Estimation

Before loading grids fully:
1. Parse metadata (dimensions, shared strings table size)
2. Sample a portion (first 1000 rows) to estimate cell density
3. Compute estimated memory: cells × avg_cell_size + overhead
4. If estimate > streaming threshold, activate streaming mode

#### Streaming Parser

Read and process one row at a time:
1. Parse row from file
2. Compute row hash
3. Store hash and minimal metadata
4. Release row cell data (or store only if needed)

This bounds memory to O(chunk_size) rather than O(total_rows).

#### Chunked Diff

Process alignment in chunks of 10,000-50,000 rows:
1. Load chunk from both grids
2. Run alignment on chunk
3. Store chunk results
4. Release chunk data
5. Repeat for next chunk

Include overlap between chunks (100-500 rows) to handle edits at boundaries.

---

### 25.6 Memory Release Points

Explicitly release memory when structures are no longer needed.

| After Phase | Release |
|-------------|---------|
| Preprocessing complete | Raw parsed cell strings (if interned) |
| Row alignment complete | Frequency tables, anchor work structures |
| Column alignment complete | Column hash maps |
| Cell diff complete | Row views (if operations stored values) |
| Serialization complete | Operation list (if streaming output) |

In Rust, dropping structures automatically releases memory. The design should ensure structures have appropriate lifetimes.

---

### 25.7 Memory Monitoring

Provide visibility into memory usage.

#### Metrics

Track and expose:
- Current allocated bytes
- Peak allocated bytes
- Allocation count
- Bytes by category (grids, alignment, results)

#### Diagnostics

In debug/diagnostic mode:
- Log allocations over 1MB
- Track allocation site (which function)
- Detect potential leaks (growing without release)

---

## Section 26: Parallelism Strategy

### 26.1 Parallelism Context

The diff engine should exploit parallelism where available while functioning correctly single-threaded.

#### Environment Constraints

| Environment | Parallelism |
|-------------|-------------|
| Browser WASM (default) | Single-threaded |
| Browser WASM (SharedArrayBuffer) | Limited multi-threading |
| Node.js WASM | Multi-threaded via workers |
| Native | Full multi-threading |

The algorithm must achieve target performance single-threaded, with parallelism as a bonus.

---

### 26.2 Parallelizable Operations

Not all operations benefit from parallelism. Identify the opportunities.

#### Embarrassingly Parallel

These operations have no dependencies between work units:

| Operation | Unit of Work | Speedup |
|-----------|--------------|---------|
| Row hashing | Each row | ~Linear |
| Column hashing | Each column | ~0.7x linear (cache effects) |
| Cell comparison | Each matched row pair | ~Linear |
| Operation serialization | Each operation | ~Linear |

#### Partially Parallel

These have some dependencies or shared state:

| Operation | Parallelism Approach | Speedup |
|-----------|---------------------|---------|
| Gap filling | Each gap independent | ~Linear (gap-limited) |
| String interning | Concurrent hash map | ~0.5x linear |
| Move candidate clustering | Per-block, then merge | ~0.5x linear |

#### Sequential Only

These must run sequentially:

| Operation | Reason |
|-----------|--------|
| LIS computation | Inherently sequential algorithm |
| Budget tracking | Shared mutable state |
| Output ordering | Must preserve determinism |

---

### 26.3 Parallelization Architecture

#### Work Partitioning

For row-oriented operations:
1. Divide rows into N partitions of approximately equal size
2. Assign each partition to a thread/task
3. Each thread processes its partition independently
4. Collect and merge results

For gap-oriented operations:
1. Each gap between anchors is an independent work unit
2. Assign gaps to threads as they become available (work stealing)
3. Larger gaps should not block smaller ones

#### Thread Pool

Use a fixed-size thread pool matching available cores:
- Native: Number of physical cores (or logical with hyperthreading consideration)
- WASM: As many workers as supported, typically 4-8

Avoid spawning threads per operation—thread creation overhead dominates for small work units.

---

### 26.4 Parallel Hashing

Row and column hashing are the primary parallelization targets.

#### Row Hash Parallelism

Each thread computes hashes for a subset of rows:
1. Partition row indices: thread i gets rows [i×chunk, (i+1)×chunk)
2. Each thread accesses its rows, computes hashes
3. Results written to a shared output vector (no synchronization needed—each thread writes to distinct indices)

**Memory access pattern**: Good locality—each thread reads contiguous rows.

**Expected speedup**: 3.5x on 4 cores (cache effects reduce from theoretical 4x).

#### Column Hash Parallelism

Each thread computes hashes for a subset of columns:
1. Partition column indices
2. Each thread iterates all rows for its columns
3. Results written to distinct indices

**Memory access pattern**: Poor locality—each column scan touches all rows.

**Expected speedup**: 2-3x on 4 cores (memory-bound).

---

### 26.5 Parallel Cell Comparison

Cell comparison is highly parallelizable.

#### Strategy

1. Create a list of matched row pairs from alignment
2. Partition the list across threads
3. Each thread compares cells for its assigned pairs
4. Collect results with (row_pair_index) tag
5. Sort by tag for deterministic order

**Work balancing**: Row pairs may have different cell counts. For better balance:
- Estimate work per pair (number of non-empty cells)
- Assign pairs to balance total work, not just pair count

---

### 26.6 Parallel Gap Filling

Gaps between anchors can be filled in parallel.

#### Strategy

1. Enumerate all gaps with their boundaries
2. Each gap is an independent work unit
3. Use work-stealing to handle variable gap sizes
4. Results include gap index for ordering

**Complication**: Move detection creates dependencies between gaps (a move source in one gap corresponds to a move destination in another). Two approaches:

**Approach A**: Sequential move identification, parallel gap filling
1. Identify all move candidates (sequential)
2. Fill gaps in parallel, each gap aware of moves involving it
3. Validate moves (sequential)

**Approach B**: Parallel gap filling, post-merge move detection
1. Fill all gaps in parallel (ignoring moves)
2. Merge results
3. Run move detection on merged results
4. Adjust gap results to account for moves

Approach A produces cleaner results; Approach B is simpler to implement. The recommended approach is A.

---

### 26.7 Synchronization Points

Certain operations require synchronization:

| Point | Synchronization |
|-------|-----------------|
| Partition assignment | Atomic counter or upfront division |
| Result collection | Write to distinct slots, no locks needed |
| Memory tracking | Atomic add/subtract |
| String interning | Concurrent hash map with lock or lock-free |

#### String Interning Concurrency

When parsing in parallel, string interning needs thread-safety:

**Option 1**: Segment the pool by thread, merge at end
- Each thread has its own pool during parsing
- Merge pools and reassign IDs after parsing
- Simple but requires ID remapping

**Option 2**: Concurrent hash map
- Single shared pool protected by read-write lock
- Read-heavy workload (check existence) benefits from concurrent readers
- Moderate write contention for new strings

**Recommendation**: Option 2 with a high-performance concurrent map implementation.

---

### 26.8 WASM Threading Considerations

WASM threading requires special handling.

#### SharedArrayBuffer Requirement

Parallel WASM requires SharedArrayBuffer, which needs:
- Secure context (HTTPS)
- Cross-origin isolation headers
- Browser support

The engine should detect availability and fall back to single-threaded if unavailable.

#### Web Workers

WASM threads are implemented as Web Workers:
- Workers have separate memory by default
- SharedArrayBuffer enables shared memory
- Thread spawning is slower than native

**Strategy**: Initialize worker pool once, reuse for multiple diffs.

#### Atomic Operations

Use WASM atomics for synchronization:
- Atomic load/store for flags
- Atomic add for counters
- Memory barriers where needed

---

### 26.9 Speedup Expectations

Based on the parallelizable fraction of work:

| Cores | Theoretical Speedup | Practical Speedup |
|-------|---------------------|-------------------|
| 1 | 1.0x | 1.0x |
| 2 | 1.8x | 1.6x |
| 4 | 3.2x | 2.5x |
| 8 | 5.0x | 3.5x |

Diminishing returns above 4 cores due to:
- Sequential portions (Amdahl's Law)
- Memory bandwidth limits
- Synchronization overhead

---

### 26.10 Single-Threaded Fallback

The engine must function correctly without parallelism.

#### Detection

Check parallelism availability:
- WASM: Check for SharedArrayBuffer
- Native: Check thread spawn capability

#### Fallback Behavior

When parallelism is unavailable:
- All parallel-for becomes sequential-for
- Thread pools become no-ops
- Concurrent containers become standard containers

The same code paths execute; only the execution model changes.

---

# Part IX: Analysis & Validation

---

## Section 27: Complexity Analysis

### 27.1 Complexity Goals

The diff algorithm must achieve:

| Scenario | Target Complexity |
|----------|-------------------|
| Typical case | O(M + R log R) |
| Worst case | O(M + R²) bounded |
| Adversarial case | O(M + R) via bail-out |

Where:
- M = number of non-empty cells
- R = number of rows
- C = number of columns (typically small, C << R)

---

### 27.2 Phase-by-Phase Analysis

#### Phase 1: Preprocessing

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Grid view construction | O(M) | Iterate all cells once |
| Cell sorting within rows | O(M log(M/R)) | Sort each row's cells by column |
| Row hash computation | O(M) | Process each cell once |
| Column hash computation | O(M) | Process each cell once |
| Frequency table construction | O(R) | One entry per row |
| **Total** | **O(M log(M/R))** | Dominated by sorting |

For typical grids where rows have few cells, log(M/R) ≈ log(C) is small.

#### Phase 2: Dimension Decision

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Column hash comparison | O(C) | One pass over columns |
| Jaccard similarity | O(C) | Set intersection |
| **Total** | **O(C)** | Negligible |

#### Phase 3: Mode Selection

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Key column scoring | O(R × C) | Score all columns |
| Composite key check | O(R × C²) | Try all pairs |
| **Total** | **O(R × C²)** | C is small, so O(R) |

#### Phase 4a: Spreadsheet Alignment (AMR)

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Anchor discovery | O(R) | Build hash-to-position map |
| LIS computation | O(A log A) | A = anchor count ≤ R |
| Move candidate extraction | O(R) | Single pass over matches |
| Gap enumeration | O(A) | Iterate anchors |
| **Per gap**: | | |
| - Trivial gap | O(1) | Empty or single-element |
| - Myers on gap | O(g²) | g = gap size |
| - Repetitive alignment | O(g) | Run-length compression |
| Total gaps | O(Σg² or Σg) | Depends on gap structure |
| Move validation | O(K) | K = move candidate count |
| **Total** | **O(R log R + Σg²)** | |

**Gap size analysis**:
- Best case (many anchors): gaps are small, Σg² ≈ O(R)
- Typical case: moderate gaps, Σg² ≈ O(R log R)
- Worst case (few anchors): one or few large gaps, Σg² ≈ O(R²)
- Adversarial (no anchors): bail-out triggers, O(R)

#### Phase 4b: Database Alignment

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Key extraction | O(R) | One pass |
| Hash join | O(R) expected | Hash table lookup |
| Duplicate cluster matching | O(Σd³) | d = cluster size |
| **Total** | **O(R + Σd³)** | |

With cluster size capped (e.g., d ≤ 16), the cubic term is bounded: O(R + K × 16³) = O(R) where K = cluster count.

#### Phase 5: Cell-Level Diff

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Cell iteration | O(M) | All cells in matched rows |
| Value comparison | O(1) per cell | Or O(formula length) for formulas |
| Classification | O(1) per edit | |
| **Total** | **O(M)** | Linear in cells |

#### Phase 6: Result Assembly

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Operation collection | O(ops) | One pass |
| Subsumption checking | O(ops) | Hash set lookups |
| Sorting | O(ops log ops) | |
| Serialization | O(output size) | |
| **Total** | **O(ops log ops)** | ops ≤ R + M |

---

### 27.3 Overall Complexity

**Typical case** (many anchors, small gaps):
```
O(M log(M/R)) + O(R log R) + O(M) + O(ops log ops)
= O(M + R log R)
```

**Worst case** (few anchors, large gaps, no bail-out):
```
O(M) + O(R²) + O(M)
= O(M + R²)
```

**Adversarial case** (triggers bail-out):
```
O(M) + O(R) + O(M)
= O(M + R)
```

---

### 27.4 Space Complexity

| Structure | Space | Notes |
|-----------|-------|-------|
| Grid views | O(M) | References to cells |
| Row metadata | O(R) | Fixed size per row |
| Hash tables | O(R) | For frequency, positions |
| Anchor list | O(A) | A ≤ R |
| Move candidates | O(R) | Worst case |
| Cell edits | O(E) | E = edit count |
| **Total** | **O(M + R)** | Linear |

The algorithm never allocates O(R²) or O(R × C) dense structures.

---

### 27.5 Adversarial Analysis

Inputs specifically designed to trigger worst-case behavior.

#### Adversarial Pattern 1: All Identical Rows

**Input**: Grid A and B have 50,000 identical rows (all cells = 0).

**Without defense**: Anchor discovery finds no unique rows. One giant gap. Myers diff takes O(R²) = O(2.5 billion) operations.

**With defense**: Repetitive pattern detected (Section 24.3). Run-length alignment reduces to O(1) runs. Total: O(R).

#### Adversarial Pattern 2: Interleaved Unique Rows

**Input**: Grid A = [1, 2, 3, ..., 50000]. Grid B = [50000, 49999, ..., 1] (reversed).

**Without defense**: All rows are unique anchors, but LIS gives only 1 anchor. One giant gap. Myers: O(R²).

**With defense**: Gap too large triggers bail-out. Entire grid reported as "replaced". Total: O(R).

#### Adversarial Pattern 3: Slightly Different Rows

**Input**: Grid A rows have pattern [i, 0, 0, ...]. Grid B rows have pattern [i, 1, 0, ...]. No exact matches.

**Without defense**: No anchors from hashing. Fall through to gap filling.

**With defense**: Detect low similarity early (Section 24.3). Bail out. Total: O(R).

---

### 27.6 Performance Targets

Concrete performance targets for reference workloads:

| Workload | Size | Target Time | Implied Complexity |
|----------|------|-------------|-------------------|
| Small identical | 1K rows | <50ms | O(R) |
| Small edited | 1K rows, 10% changed | <100ms | O(R log R) |
| Medium identical | 50K rows | <200ms | O(R) |
| Medium edited | 50K rows, 10% changed | <500ms | O(R log R) |
| Large identical | 500K rows | <2s | O(R) |
| Large edited | 500K rows, 10% changed | <5s | O(R log R) |

These targets assume single-threaded WASM execution. Native parallel execution should achieve 2-3x speedup.

---

### 27.7 Benchmarking Strategy

To validate complexity guarantees:

#### Scaling Tests

For each workload type, measure time at increasing sizes:
- 1K, 5K, 10K, 50K, 100K, 500K rows
- Plot time vs. size on log-log scale
- Slope indicates complexity: 1 = linear, 2 = quadratic

#### Adversarial Tests

Specifically test patterns from Section 27.5:
- Verify defenses activate
- Confirm O(R) or better
- No test should exceed 10 seconds regardless of input

#### Regression Tests

Track performance across code changes:
- Alert on >10% regression for any benchmark
- Investigate and justify any complexity change

---

## Section 28: Configuration Reference

### 28.1 Configuration Overview

The diff engine exposes configuration options to customize behavior for different use cases. Configuration is provided at diff invocation time and affects algorithm choices, thresholds, output format, and resource limits.

**Design principles:**
- Sensible defaults that work well for common cases
- All options optional—engine works with zero configuration
- Validation rejects invalid combinations with clear error messages
- Configuration is immutable during a diff operation

---

### 28.2 Algorithm Configuration

#### Mode Selection

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `mode` | enum | `auto` | Comparison mode: `spreadsheet`, `database`, or `auto` |
| `key_columns` | array | `[]` | Column indices to use as keys in database mode |
| `force_mode` | bool | `false` | If true, use specified mode even if inference suggests otherwise |

When `mode` is `auto`, the engine infers the appropriate mode based on data characteristics. If `key_columns` is non-empty and `mode` is `auto` or `database`, database mode is used with the specified keys.

#### Dimension Ordering

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dimension_order` | enum | `auto` | `row_first`, `column_first`, or `auto` |
| `column_stability_threshold` | float | `0.90` | Jaccard threshold for column stability check |

When `dimension_order` is `auto`, the engine checks column stability and chooses accordingly.

#### Hash Algorithm

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `hash_algorithm` | enum | `xxhash64` | `xxhash64` (fast) or `blake3_128` (safe) |

Use `blake3_128` for audit/compliance scenarios where collision probability must be minimized.

---

### 28.3 Alignment Configuration

#### Anchor Discovery

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `anchor_uniqueness_required` | bool | `true` | Only use rows unique in both grids as anchors |
| `rare_row_threshold` | int | `3` | Maximum frequency for "rare" rows eligible as secondary anchors |

#### Gap Filling

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `small_gap_threshold` | int | `256` | Gaps smaller than this use Myers directly |
| `repetitive_threshold` | float | `0.80` | Fraction of low-info rows to trigger run-length strategy |
| `bailout_similarity_threshold` | float | `0.05` | Below this similarity, treat gap as complete replacement |
| `max_gap_size` | int | `10000` | Gaps larger than this trigger bail-out |

#### Move Detection

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_move_detection` | bool | `true` | Whether to detect block moves |
| `move_min_block_size` | int | `2` | Minimum rows for a block to qualify as move |
| `fuzzy_move_threshold` | float | `0.80` | Similarity threshold for fuzzy move detection |
| `max_move_distance` | int | `unlimited` | Maximum rows a block can move (limits search) |

---

### 28.4 Cell Comparison Configuration

#### Value Comparison

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `numeric_tolerance_relative` | float | `1e-10` | Relative tolerance for float comparison |
| `numeric_tolerance_absolute` | float | `1e-14` | Absolute tolerance for float comparison |
| `string_case_sensitive` | bool | `true` | Case-sensitive string comparison |
| `whitespace_sensitive` | bool | `true` | Consider leading/trailing whitespace |

#### Formula Handling

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `include_formulas` | bool | `true` | Include formulas in row hashing and comparison |
| `semantic_formula_comparison` | bool | `true` | Use AST-based formula comparison |
| `detect_reference_shifts` | bool | `true` | Identify mechanical reference shifts |
| `compute_formula_tree_diff` | bool | `false` | Compute detailed tree edit distance |

---

### 28.5 Database Mode Configuration

#### Key Handling

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `auto_detect_keys` | bool | `true` | Automatically infer key columns |
| `key_uniqueness_threshold` | float | `0.95` | Minimum uniqueness for key candidate |
| `key_coverage_threshold` | float | `0.90` | Minimum non-empty fraction for key candidate |
| `max_composite_key_columns` | int | `2` | Maximum columns in composite key |

#### Duplicate Key Handling

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `duplicate_key_strategy` | enum | `similarity` | `similarity`, `first_match`, or `report_all` |
| `max_duplicate_cluster_size` | int | `16` | Maximum cluster size for similarity matching |
| `duplicate_match_threshold` | float | `0.50` | Similarity threshold for matching in clusters |

---

### 28.6 Resource Limits

#### Memory

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `memory_budget_total` | int | `1610612736` | Total memory budget in bytes (1.5 GB) |
| `memory_budget_per_grid` | int | `629145600` | Per-grid memory budget (600 MB) |
| `streaming_threshold` | int | `52428800` | Switch to streaming above this (50 MB) |

#### Time

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `timeout_total_ms` | int | `120000` | Total timeout (120 seconds) |
| `timeout_alignment_ms` | int | `60000` | Alignment phase timeout (60 seconds) |
| `timeout_cell_diff_ms` | int | `60000` | Cell diff phase timeout (60 seconds) |

#### Size

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_rows` | int | `1048576` | Maximum rows to process |
| `max_columns` | int | `16384` | Maximum columns to process |
| `max_operations` | int | `1000000` | Maximum operations in result |

---

### 28.7 Output Configuration

#### Format

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `output_format` | enum | `json` | `json`, `json_lines`, `binary`, `text`, `patch` |
| `include_content_previews` | bool | `true` | Include cell value previews in operations |
| `preview_max_length` | int | `50` | Maximum characters in content preview |
| `coalesce_operations` | bool | `false` | Combine consecutive add/remove operations |

#### Compression

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `compression` | enum | `none` | `none`, `lz4`, `zstd`, `gzip` |
| `compression_level` | int | `3` | Compression level (format-dependent) |

---

### 28.8 Parallelism Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable_parallelism` | bool | `true` | Use parallel processing if available |
| `thread_count` | int | `auto` | Number of threads (auto = detect cores) |
| `parallel_hash_threshold` | int | `1000` | Minimum rows to parallelize hashing |
| `parallel_diff_threshold` | int | `100` | Minimum matched pairs to parallelize cell diff |

---

### 28.9 Debugging and Diagnostics

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `collect_metrics` | bool | `false` | Collect timing and count metrics |
| `include_diagnostics` | bool | `false` | Include diagnostic info in result |
| `log_level` | enum | `warn` | Logging verbosity: `off`, `error`, `warn`, `info`, `debug`, `trace` |
| `determinism_checks` | bool | `false` | Enable runtime determinism assertions |

---

### 28.10 Configuration Profiles

Pre-defined profiles for common scenarios:

#### Performance Profile

Optimizes for speed:
```
hash_algorithm: xxhash64
semantic_formula_comparison: false
compute_formula_tree_diff: false
enable_move_detection: true (but limited)
max_move_distance: 1000
```

#### Accuracy Profile

Optimizes for correctness:
```
hash_algorithm: blake3_128
semantic_formula_comparison: true
compute_formula_tree_diff: true
fuzzy_move_threshold: 0.90
numeric_tolerance_relative: 1e-12
```

#### Audit Profile

For compliance and auditing:
```
hash_algorithm: blake3_128
semantic_formula_comparison: true
detect_reference_shifts: true
include_diagnostics: true
collect_metrics: true
coalesce_operations: false
```

#### Large File Profile

For very large files:
```
streaming_threshold: 10485760 (10 MB)
enable_move_detection: false
max_gap_size: 1000
parallel_hash_threshold: 500
```

---

## Section 29: Validation Strategy

### 29.1 Validation Goals

The diff engine must be validated for:

1. **Correctness**: Results accurately reflect differences between grids
2. **Completeness**: All differences are detected and reported
3. **Performance**: Meets timing targets across workload spectrum
4. **Robustness**: Handles edge cases and pathological inputs gracefully
5. **Determinism**: Same inputs always produce identical outputs

---

### 29.2 Test Categories

#### Unit Tests

Test individual components in isolation:

| Component | Test Focus |
|-----------|------------|
| Cell normalization | Type-specific normalization, edge cases |
| Row hashing | Determinism, collision resistance, formula handling |
| LIS computation | Correctness on various sequences |
| Myers diff | Edit script correctness, boundary cases |
| Similarity metrics | Jaccard, Dice, overlap calculations |
| Key inference | Column scoring, composite key detection |

#### Integration Tests

Test component interactions:

| Test Type | Description |
|-----------|-------------|
| Phase transitions | Data flows correctly between phases |
| Mode selection | Auto mode chooses correctly |
| Move + cell diff | Moves detected and cells within compared |
| Streaming | Chunked processing produces same results as full |

#### End-to-End Tests

Test complete diff operations:

| Scenario | Validation |
|----------|------------|
| Identical grids | Zero operations, high similarity |
| Single cell edit | One CellEdited operation |
| Row inserted | RowAdded at correct position |
| Block moved | BlockMovedRows with correct ranges |
| Complex edits | Multiple operation types, correct counts |

---

### 29.3 Correctness Validation

#### Invariant Testing

Verify invariants that must always hold:

1. **Alignment coverage**: Every row in both grids is either matched, added, removed, or part of a move
2. **No duplicate matches**: Each row matched at most once
3. **Move consistency**: Move sources equal destinations in content
4. **Cell edit validity**: Old value exists in A, new value exists in B
5. **Position validity**: All row/column indices within grid bounds

#### Reconstruction Testing

Verify that applying diff operations transforms grid A into grid B:

1. Start with a copy of grid A
2. Apply each operation:
   - RowRemoved: Delete the row
   - RowAdded: Insert the row from B
   - BlockMovedRows: Relocate rows
   - CellEdited: Update cell value
3. Compare result to grid B
4. Assert identical

This is the ultimate correctness test—if reconstruction succeeds, the diff is correct.

#### Oracle Testing

For complex scenarios, compare against a reference implementation:

1. Implement a simple O(N²) diff as reference
2. Run both algorithms on same inputs
3. Compare results (allowing for equivalent representations)

The reference implementation is too slow for production but useful for validation.

---

### 29.4 Performance Validation

#### Benchmark Suite

Standard benchmarks covering the workload spectrum:

| Benchmark | Size | Characteristics |
|-----------|------|-----------------|
| `bench_identical_small` | 1K rows | No changes |
| `bench_identical_medium` | 50K rows | No changes |
| `bench_scatter_small` | 1K rows | 1% random cell edits |
| `bench_scatter_medium` | 50K rows | 1% random cell edits |
| `bench_block_insert` | 50K rows | 1K row block inserted |
| `bench_block_move` | 50K rows | 1K row block moved |
| `bench_heavy_edit` | 50K rows | 30% rows changed |
| `bench_adversarial_repeat` | 50K rows | 99% identical rows |
| `bench_adversarial_reverse` | 50K rows | Completely reversed |

#### Performance Regression Testing

Track benchmark times across commits:

1. Run benchmarks on every PR
2. Compare to baseline (main branch)
3. Flag regressions >10%
4. Block merge for regressions >25% without justification

#### Scaling Validation

Verify complexity claims:

1. Run each benchmark at 1K, 5K, 10K, 50K, 100K rows
2. Plot time vs. size
3. Fit to model (linear, linearithmic, quadratic)
4. Assert matches expected complexity

---

### 29.5 Robustness Validation

#### Edge Case Tests

| Case | Test |
|------|------|
| Empty grids | Both empty, one empty, all empty cells |
| Single cell | 1×1 grids with various changes |
| Single row/column | Degenerate dimensions |
| Maximum size | At configured limits |
| Unicode | Various scripts, emoji, combining characters |
| Special floats | NaN, infinity, -0, subnormals |
| Long strings | Near and at limits |
| Deep formulas | Complex nested expressions |

#### Adversarial Input Tests

| Pattern | Expected Behavior |
|---------|-------------------|
| All identical rows | Bail-out activates, linear time |
| Completely reversed | Bail-out activates, linear time |
| Alternating unique/repeated | Graceful degradation |
| One dominant value | Excluded from hashing |
| Pathological key distribution | Falls back to spreadsheet mode |

#### Fuzz Testing

Generate random inputs to find unexpected failures:

1. Random grid generation with configurable parameters
2. Random mutations (edits, insertions, deletions, moves)
3. Run diff and validate invariants
4. Track coverage to ensure diverse inputs

---

### 29.6 Determinism Validation

#### Multi-Run Tests

For each test case:
1. Run diff N times (N ≥ 10)
2. Serialize each result to bytes
3. Assert all N results byte-identical

#### Cross-Platform Tests

Run identical tests on:
- x86_64 Linux
- x86_64 Windows
- ARM64 macOS
- WASM (Node.js)
- WASM (Browser via headless Chrome)

Assert results match across all platforms.

#### Parallel Variation Tests

For parallelizable tests:
1. Run with 1, 2, 4, 8 threads
2. Assert all produce identical results

---

### 29.7 Test Data Generation

#### Synthetic Data

Generate test grids programmatically:

| Generator | Purpose |
|-----------|---------|
| `random_grid(rows, cols, density)` | Random values at specified density |
| `sequential_grid(rows, cols)` | Predictable patterns |
| `clone_and_mutate(grid, edits)` | Create version B from A |
| `insert_block(grid, position, size)` | Insert row/column blocks |
| `move_block(grid, from, to, size)` | Create moved blocks |
| `shuffle_rows(grid, fraction)` | Random reordering |

#### Real-World Data

Collect anonymized test cases from:
- Financial models (budget comparisons)
- Data exports (CRM, ERP snapshots)
- Report templates (versioned layouts)

Ensure test data covers:
- Various industries and use cases
- Different Excel features used
- Range of sizes and edit patterns

---

### 29.8 Continuous Integration

#### PR Validation

On every pull request:
1. All unit tests pass
2. All integration tests pass
3. All end-to-end tests pass
4. Benchmark suite runs (regression check)
5. Coverage maintained or improved

#### Nightly Validation

Extended tests run nightly:
1. Full benchmark suite with larger inputs
2. Cross-platform tests
3. Fuzz testing (extended duration)
4. Memory leak detection
5. Performance profiling

#### Release Validation

Before each release:
1. All nightly tests pass
2. Manual testing of key scenarios
3. Performance comparison to previous release
4. Documentation accuracy check

---

### 29.9 Test Infrastructure

#### Test Fixtures

Maintain a library of test fixtures:
- Small grids embedded in test code
- Medium grids in files (version controlled)
- Large grids generated on-demand (reproducible)

#### Assertion Helpers

Custom assertions for diff validation:
- `assert_operation_count(result, type, expected)`
- `assert_cell_edited(result, row, col, old, new)`
- `assert_row_moved(result, from, to)`
- `assert_reconstruction_matches(result, grid_a, grid_b)`

#### Test Utilities

- Grid comparison (structural equality)
- Result serialization (for determinism checks)
- Performance measurement (wall time, CPU time)
- Memory tracking (peak usage)

---

# Part X: Appendices

---

## Section 30: Algorithm Pseudocode Summary

### 30.1 Purpose

This appendix provides concise pseudocode for the key algorithms, serving as an implementation reference. The pseudocode uses a syntax-agnostic style that maps directly to Rust, Python, or similar languages.

---

### 30.2 Main Entry Point

```
function diff_grids(grid_a, grid_b, config):
    # Phase 1: Preprocessing
    view_a = build_grid_view(grid_a, config.hasher)
    view_b = build_grid_view(grid_b, config.hasher)
    stats = compute_hash_stats(view_a, view_b)
    
    # Phase 2: Dimension ordering
    dim_order = decide_dimension_order(view_a, view_b, config)
    if dim_order == ColumnFirst:
        col_alignment = align_columns(view_a, view_b, config)
        recompute_row_hashes(view_a, view_b, col_alignment)
    
    # Phase 3: Mode selection
    mode = select_mode(view_a, view_b, config)
    
    # Phase 4: Alignment
    if mode == Spreadsheet:
        row_alignment = amr_align_rows(view_a, view_b, stats, config)
    else:
        row_alignment = database_align_rows(view_a, view_b, mode.key_columns, config)
    
    if dim_order != ColumnFirst:
        col_alignment = align_columns(view_a, view_b, config)
    
    # Phase 5: Cell diff
    cell_edits = diff_matched_cells(row_alignment.matched, view_a, view_b, col_alignment, config)
    
    # Phase 6: Result assembly
    result = assemble_result(row_alignment, col_alignment, cell_edits, config)
    return result
```

---

### 30.3 AMR Row Alignment

```
function amr_align_rows(view_a, view_b, stats, config):
    # Phase 1: Anchor discovery
    unique_matches = find_unique_hash_matches(view_a, view_b, stats)
    anchors = longest_increasing_subsequence(unique_matches, key=row_b)
    
    # Phase 2: Move candidate extraction
    all_matches = find_all_hash_matches(view_a, view_b)
    non_anchor_matches = all_matches - anchors
    move_candidates = cluster_into_blocks(non_anchor_matches)
    
    # Phase 3: Move-aware gap filling
    gaps = enumerate_gaps(anchors, view_a.nrows, view_b.nrows)
    gap_results = []
    
    for gap in gaps:
        moves_in_gap = find_moves_overlapping(move_candidates, gap)
        remaining_a = gap.rows_a - moves_in_gap.sources
        remaining_b = gap.rows_b - moves_in_gap.destinations
        
        strategy = select_gap_strategy(remaining_a, remaining_b, stats, config)
        alignment = apply_strategy(strategy, remaining_a, remaining_b, view_a, view_b)
        gap_results.append((gap, alignment, moves_in_gap))
    
    # Phase 4: Assemble alignment
    matched = anchors + flatten(gap_results.alignments)
    deleted = collect_deleted(view_a, matched, move_candidates)
    inserted = collect_inserted(view_b, matched, move_candidates)
    moves = validate_moves(move_candidates, gap_results)
    
    return RowAlignment(matched, deleted, inserted, moves)
```

---

### 30.4 Longest Increasing Subsequence

```
function longest_increasing_subsequence(sequence):
    if sequence is empty:
        return []
    
    n = length(sequence)
    piles = []           # piles[i] = list of indices in pile i
    predecessors = [None] * n
    
    for i in 0..n:
        val = sequence[i]
        pile_idx = binary_search(piles, val, key=lambda p: sequence[last(p)])
        
        if pile_idx > 0:
            predecessors[i] = last(piles[pile_idx - 1])
        
        if pile_idx == length(piles):
            piles.append([i])
        else:
            piles[pile_idx].append(i)
    
    # Backtrack to construct LIS
    result = []
    current = last(last(piles)) if piles else None
    while current is not None:
        result.prepend(current)
        current = predecessors[current]
    
    return result
```

---

### 30.5 Gap Strategy Selection

```
function select_gap_strategy(rows_a, rows_b, stats, config):
    len_a = length(rows_a)
    len_b = length(rows_b)
    max_len = max(len_a, len_b)
    
    # Empty or one-sided gaps
    if max_len == 0 or len_a == 0 or len_b == 0:
        return Trivial
    
    # Small gaps use Myers
    if max_len <= config.small_gap_threshold:
        return Myers
    
    # Check for repetitive content
    low_info_count = count_low_info_rows(rows_a, rows_b, stats)
    if low_info_count / max_len > config.repetitive_threshold:
        return Repetitive
    
    # Check similarity for bail-out
    similarity = estimate_gap_similarity(rows_a, rows_b, stats)
    if similarity < config.bailout_similarity_threshold:
        return BailOut
    
    # Large gap that's not obviously bad
    if max_len > config.max_gap_size:
        return BailOut
    
    return Myers
```

---

### 30.6 Database Mode Alignment

```
function database_align_rows(view_a, view_b, key_columns, config):
    # Extract keyed rows
    keyed_a = extract_keys(view_a, key_columns)
    keyed_b = extract_keys(view_b, key_columns)
    
    # Build key-to-rows mapping
    map_a = group_by(keyed_a, key=row.key_hash)
    map_b = group_by(keyed_b, key=row.key_hash)
    
    all_keys = union(keys(map_a), keys(map_b))
    
    matched = []
    deleted = []
    inserted = []
    
    for key in all_keys:
        rows_a = map_a.get(key, [])
        rows_b = map_b.get(key, [])
        
        if length(rows_a) == 0:
            inserted.extend(rows_b)
        else if length(rows_b) == 0:
            deleted.extend(rows_a)
        else if length(rows_a) == 1 and length(rows_b) == 1:
            matched.append((rows_a[0], rows_b[0]))
        else:
            # Duplicate key cluster
            cluster_match = match_duplicate_cluster(rows_a, rows_b, view_a, view_b, config)
            matched.extend(cluster_match.matched)
            deleted.extend(cluster_match.unmatched_a)
            inserted.extend(cluster_match.unmatched_b)
    
    return RowAlignment(matched, deleted, inserted, moves=[])
```

---

### 30.7 Cell Comparison

```
function diff_matched_cells(matched_rows, view_a, view_b, col_mapping, config):
    edits = []
    
    for (row_a, row_b) in matched_rows:
        cells_a = view_a.rows[row_a].cells
        cells_b = view_b.rows[row_b].cells
        
        cells_b_by_col = index_by(cells_b, key=col)
        visited_b = set()
        
        # Check cells in A
        for (col_a, cell_a) in cells_a:
            col_b = col_mapping.get(col_a)
            if col_b is None:
                continue  # Column removed, don't report cell
            
            cell_b = cells_b_by_col.get(col_b)
            visited_b.add(col_b)
            
            if cell_b is None:
                if not cell_a.is_empty:
                    edits.append(CellEdit(row_a, col_a, row_b, col_b, cell_a, Empty, Cleared))
            else:
                if not cells_equal(cell_a, cell_b, config):
                    change_type = classify_change(cell_a, cell_b)
                    edits.append(CellEdit(row_a, col_a, row_b, col_b, cell_a, cell_b, change_type))
        
        # Check cells in B not in A
        for (col_b, cell_b) in cells_b:
            if col_b in visited_b:
                continue
            col_a = col_mapping.reverse_get(col_b)
            if col_a is None:
                continue  # Column added, don't report cell
            if not cell_b.is_empty:
                edits.append(CellEdit(row_a, col_a, row_b, col_b, Empty, cell_b, Added))
    
    return edits
```

---

## Section 31: Worked Examples

### 31.1 Purpose

This appendix provides detailed worked examples showing how the algorithm processes specific inputs, step by step. These examples aid understanding and serve as reference test cases.

---

### 31.2 Example 1: Simple Row Insertion

**Grid A:**
```
Row 0: ["Name", "Value"]
Row 1: ["Alice", 100]
Row 2: ["Bob", 200]
Row 3: ["Charlie", 300]
```

**Grid B:**
```
Row 0: ["Name", "Value"]
Row 1: ["Alice", 100]
Row 2: ["Bob", 200]
Row 3: ["Carol", 250]      <- New row
Row 4: ["Charlie", 300]
```

**Processing:**

1. **Preprocessing**: Compute row hashes
   - Row 0 (A/B): hash = H0
   - Row 1 (A/B): hash = H1
   - Row 2 (A/B): hash = H2
   - Row 3 (A): hash = H3 ("Charlie")
   - Row 3 (B): hash = H4 ("Carol") - new
   - Row 4 (B): hash = H3 ("Charlie")

2. **Anchor Discovery**: All rows with unique hashes
   - H0: A[0] ↔ B[0] ✓
   - H1: A[1] ↔ B[1] ✓
   - H2: A[2] ↔ B[2] ✓
   - H3: A[3] ↔ B[4] ✓ (Charlie moved from row 3 to row 4)
   - H4: only in B[3] - not an anchor

3. **LIS**: Positions [0, 1, 2, 4] in B are monotonic. All anchors kept.

4. **Gap Analysis**:
   - Gap between anchor at B[2] and anchor at B[4]: B rows [3..4)
   - Gap contains one row: B[3] with hash H4

5. **Gap Filling**: B[3] has no match in A. Mark as insertion.

6. **Result**:
   - Matched: (A[0], B[0]), (A[1], B[1]), (A[2], B[2]), (A[3], B[4])
   - Inserted: B[3]

**Output:**
```
RowAdded { row_b: 3, content: ["Carol", 250] }
```

---

### 31.3 Example 2: Block Move

**Grid A:**
```
Row 0: Header
Row 1: Alpha
Row 2: Beta
Row 3: Gamma
Row 4: Delta
Row 5: Epsilon
Row 6: Zeta
Row 7: Footer
```

**Grid B:**
```
Row 0: Header
Row 1: Gamma
Row 2: Delta
Row 3: Epsilon
Row 4: Alpha     <- Block moved here
Row 5: Beta      <- Block moved here
Row 6: Zeta
Row 7: Footer
```

**Processing:**

1. **Row Hashes**: Each row has a unique hash (H0-H7).

2. **Anchor Discovery**: All rows unique, all are candidates.
   - H0: A[0] ↔ B[0] (Header)
   - H1: A[1] ↔ B[4] (Alpha)
   - H2: A[2] ↔ B[5] (Beta)
   - H3: A[3] ↔ B[1] (Gamma)
   - H4: A[4] ↔ B[2] (Delta)
   - H5: A[5] ↔ B[3] (Epsilon)
   - H6: A[6] ↔ B[6] (Zeta)
   - H7: A[7] ↔ B[7] (Footer)

3. **LIS on B positions**: Sequence is [0, 4, 5, 1, 2, 3, 6, 7]
   - Longest increasing subsequence: [0, 1, 2, 3, 6, 7] (length 6)
   - This corresponds to: Header, Gamma, Delta, Epsilon, Zeta, Footer
   - Anchors: A[0], A[3], A[4], A[5], A[6], A[7] ↔ B[0], B[1], B[2], B[3], B[6], B[7]

4. **Move Candidate Extraction**:
   - Non-anchor matches: Alpha, Beta (A[1,2] ↔ B[4,5])
   - These are consecutive in both grids → form block candidate

5. **Gap Analysis**:
   - Gap between anchors A[0] and A[3]: contains A[1,2] (Alpha, Beta)
   - Gap between anchors B[3] and B[6]: contains B[4,5] (Alpha, Beta)
   - The block A[1,2] appears at B[4,5] → move detected

6. **Move Validation**: Block [1,2] in A maps to [4,5] in B with identical content.

**Interpretation Note**: The user might visually perceive "Gamma/Delta/Epsilon moved up", but the LIS algorithm anchors on the longest monotonic chain. Since the {Header, Gamma, Delta, Epsilon, Zeta, Footer} chain is longer (6 elements) than {Header, Alpha, Beta, Zeta, Footer} (5 elements), the algorithm anchors on the former and reports Alpha/Beta as having moved. Both interpretations describe the same transformation; the algorithm chooses the one with maximal anchor coverage.

**Output:**
```
BlockMovedRows { source: 1..3, dest: 4..6 }
```

---

### 31.4 Example 3: Database Mode with Duplicate Keys

**Grid A (key column 0):**
```
Row 0: [ID, Name, Value]   <- Header
Row 1: [1, "Alice", 100]
Row 2: [2, "Bob", 200]
Row 3: [2, "Bob Jr", 201]  <- Duplicate key
Row 4: [3, "Charlie", 300]
```

**Grid B (key column 0):**
```
Row 0: [ID, Name, Value]   <- Header
Row 1: [1, "Alice", 150]   <- Value changed
Row 2: [2, "Robert", 200]  <- Name changed (was Bob)
Row 3: [2, "Bob Jr", 201]  <- Unchanged
Row 4: [4, "Diana", 400]   <- New key
```

**Processing:**

1. **Key Extraction** (column 0):
   - A keys: "ID", 1, 2, 2, 3
   - B keys: "ID", 1, 2, 2, 4

2. **Hash Join**:
   - Key "ID": A[0] ↔ B[0] (1:1 match)
   - Key 1: A[1] ↔ B[1] (1:1 match)
   - Key 2: A[2,3] ↔ B[2,3] (duplicate cluster)
   - Key 3: A[4] only (removed)
   - Key 4: B[4] only (added)

3. **Duplicate Cluster Resolution** (Key 2):
   - Compute similarity matrix:
     - A[2] ("Bob", 200) vs B[2] ("Robert", 200): similarity 0.5
     - A[2] ("Bob", 200) vs B[3] ("Bob Jr", 201): similarity 0.3
     - A[3] ("Bob Jr", 201) vs B[2] ("Robert", 200): similarity 0.3
     - A[3] ("Bob Jr", 201) vs B[3] ("Bob Jr", 201): similarity 1.0
   - Optimal assignment: A[2]↔B[2], A[3]↔B[3]

4. **Cell Diff**:
   - (A[1], B[1]): Value 100→150
   - (A[2], B[2]): Name "Bob"→"Robert"

**Output:**
```
RowRemoved { row_a: 4 }           // Charlie (key 3)
RowAdded { row_b: 4 }             // Diana (key 4)
CellEdited { row_a: 1, col_a: 2, old: 100, new: 150 }
CellEdited { row_a: 2, col_a: 1, old: "Bob", new: "Robert" }
```

---

### 31.5 Example 4: Column Insertion with Row Edits

**Grid A:**
```
     Col0    Col1
Row0 "Name"  "Score"
Row1 "Alice" 100
Row2 "Bob"   200
```

**Grid B:**
```
     Col0    Col1     Col2
Row0 "Name"  "Grade"  "Score"    <- New column inserted
Row1 "Alice" "A"      100
Row2 "Bob"   "B"      250        <- Score changed
```

**Processing:**

1. **Column Stability Check**:
   - A columns: [hash("Name"...), hash("Score"...)]
   - B columns: [hash("Name"...), hash("Grade"...), hash("Score"...)]
   - Similarity: 2 shared / 3 total = 0.67 < 0.90 threshold
   - Decision: Column-first mode

2. **Column Alignment**:
   - Col A[0] ↔ Col B[0] (Name)
   - Col A[1] ↔ Col B[2] (Score)
   - Col B[1] added (Grade)

3. **Row Hash Recomputation** (using matched columns only):
   - Now row hashes computed over Name and Score only
   - All rows have exact matches after recomputation

4. **Row Alignment**: All rows match 1:1

5. **Cell Diff**:
   - (A[2] col 1, B[2] col 2): 200 → 250

**Output:**
```
ColumnAdded { col_b: 1, header: "Grade" }
CellEdited { row_a: 2, col_a: 1, row_b: 2, col_b: 2, old: 200, new: 250 }
```

---

## Section 32: Glossary

### 32.1 Terms

**Anchor**
A high-confidence row match that partitions the alignment problem. Typically a row with a hash that appears exactly once in each grid.

**AMR (Anchor-Move-Refine)**
The primary alignment algorithm that integrates move detection into the alignment process rather than treating it as a post-processing step.

**Bail-Out**
A strategy for handling gaps or grids where alignment would be expensive and unlikely to yield meaningful results. Treats the entire region as a block replacement.

**Cell Edit**
A diff operation indicating that a cell's content changed between the source and target positions in aligned rows.

**Column Mapping**
The correspondence between columns in grid A and grid B, accounting for insertions, deletions, and reordering.

**Column Stability**
A measure of how similar the column structure is between two grids. High stability (few column changes) enables row-first alignment.

**Composite Key**
A key for database mode consisting of multiple columns whose combined values identify rows.

**Database Mode**
A comparison mode where rows are matched by key column values rather than position. Row order is semantically irrelevant.

**Dimension Ordering**
The choice of whether to align rows first then columns, or columns first then rows. Affects accuracy when column structure changes.

**Duplicate Cluster**
In database mode, a set of rows sharing the same key value. Requires similarity-based matching to pair rows correctly.

**Ephemeral View**
A transient data structure built for algorithmic processing and discarded after use. Avoids permanent memory allocation.

**Fingerprint**
A hash value representing the content of a row or column, used for fast equality testing and anchor discovery.

**Fuzzy Move**
A block move where the moved content has some internal edits, matched by similarity rather than exact hash equality.

**Gap**
A region between anchors containing rows that need alignment. Gaps are filled using Myers diff or specialized strategies.

**Grid**
The 2D data structure being compared, representing a worksheet with rows, columns, and cells.

**Grid View**
An ephemeral, algorithm-friendly representation of a grid with row-oriented access and precomputed metadata.

**Hash Collision**
When two different rows produce the same hash value. Rare with 64-bit hashes, negligible with 128-bit.

**Interning**
Storing unique strings once and referencing them by ID, reducing memory usage for repetitive data.

**Key Column**
In database mode, a column whose values identify rows for matching purposes.

**Key Inference**
Automatic detection of which columns should serve as keys for database mode comparison.

**LIS (Longest Increasing Subsequence)**
The algorithm used to find the longest chain of monotonically aligned anchors from candidate matches.

**Low-Information Row**
A row with minimal non-empty cells (typically ≤1), often repeated many times. Requires special handling to avoid false matches.

**Move Detection**
Identifying when content changed position (rows or columns relocated) rather than being deleted and re-inserted.

**Myers Diff**
The O(ND) algorithm for computing the shortest edit script between two sequences, used for gap filling.

**Normalization**
Transforming values to a canonical form before comparison (e.g., trimming whitespace, rounding floats).

**Operation (DiffOp)**
A single unit of change in the diff result: RowAdded, RowRemoved, CellEdited, BlockMovedRows, etc.

**Patience Diff**
The anchor-based diff algorithm that uses unique elements to partition alignment, reducing complexity.

**Rectangular Move**
A correlated row and column move where a 2D block was cut and pasted to a different location.

**Row Alignment**
The mapping between rows in grid A and grid B, including matches, insertions, deletions, and moves.

**Run-Length Encoding**
Compressing sequences of identical elements into (value, count) pairs. Used for repetitive gap alignment.

**Similarity Score**
A numeric measure (0.0-1.0) of how alike two rows, blocks, or grids are.

**Sparse Grid**
A grid where most cells are empty. Requires memory-efficient representation.

**Spreadsheet Mode**
A comparison mode where rows are matched by position, insertions/deletions cause shifts, and moves are detected.

**Streaming Mode**
Processing large grids chunk-by-chunk rather than loading entirely into memory.

**String Pool**
The data structure implementing string interning, mapping strings to IDs and vice versa.

**Subsumption**
When one operation (e.g., RowRemoved) implies others (cell clears), the implied operations are suppressed to avoid redundancy.

**Threshold**
A configurable numeric value that controls algorithm behavior (e.g., similarity threshold for fuzzy matching).

**Trivial Gap**
A gap with no content on one or both sides, requiring no actual alignment work.

---

### 32.2 Abbreviations

| Abbreviation | Meaning |
|--------------|---------|
| AMR | Anchor-Move-Refine |
| AST | Abstract Syntax Tree |
| CI | Continuous Integration |
| LCS | Longest Common Subsequence |
| LIS | Longest Increasing Subsequence |
| NDJSON | Newline-Delimited JSON |
| NFC | Unicode Normalization Form Composed |
| WASM | WebAssembly |

---

### 32.3 Symbols

| Symbol | Meaning |
|--------|---------|
| R | Number of rows |
| C | Number of columns |
| M | Number of non-empty cells |
| A | Number of anchors |
| U | Number of unique row matches |
| K | Number of move candidates or clusters |
| g | Gap size (rows in a gap region) |
| D | Edit distance |
| N | Generic sequence length (used in algorithm descriptions) |
| O(...) | Asymptotic complexity notation |

---

## Document Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.1 | 2025-12-02 | RS1 doc update | Clarified commutative XXHash64 reduction for row/col signatures (order-independent over sparse iteration with position-tagged contributions). |
| 1.0 | 2025-11-30 | - | Initial specification |

---

## End of Document


