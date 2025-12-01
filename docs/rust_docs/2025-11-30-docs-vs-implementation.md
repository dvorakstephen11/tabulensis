# Documentation vs Implementation Analysis

**Date:** 2025-11-30  
**Purpose:** Evaluate alignment between the unified grid diff specification and the current codebase implementation  
**Scope:** Core implementation in `core/` directory against `unified_grid_diff_algorithm_specification.md`

---

## Executive Summary

The current implementation represents approximately **25-35% of the end-state system** with a **strong architectural foundation** that aligns well with the specification's vision. The implemented portions achieve **~90-95% alignment** with the spec for the features they cover.

### Key Findings

| Category | Assessment |
|----------|------------|
| **Foundation Quality** | Excellent — layered architecture, error handling, and data structures are well-designed |
| **Alignment with Spec** | High for implemented features, with clear extension points for future work |
| **Implementation Gap** | The core grid diff algorithm (the hardest part) is not yet implemented |
| **Trajectory Risk** | Low-to-moderate — current naive diff is placeholder, but infrastructure is solid |

### Implementation Status by Specification Part

| Spec Part | Coverage | Status |
|-----------|----------|--------|
| Part I: Foundations | 60% | Use cases defined; basic infrastructure present |
| Part II: Architecture | 40% | Data structures implemented; pipeline scaffolding missing |
| Part III: Preprocessing | 30% | Row/column signatures exist; full fingerprinting not implemented |
| Part IV: Spreadsheet Mode Alignment | 5% | Only naive cell-by-cell comparison exists |
| Part V: Database Mode | 0% | Not started |
| Part VI: Cell-Level Comparison | 70% | Basic cell diff works; formula semantics missing |
| Part VII: Result Assembly | 60% | DiffOp/DiffReport types complete; coalescing not implemented |
| Part VIII: Robustness | 20% | Error handling present; memory management not implemented |

---

## Part-by-Part Analysis

### Part I: Foundations — Use Cases & Requirements

#### Specification Requirements

The spec defines 20 use cases (UC-01 through UC-20) across five categories:
- Basic Cases (UC-01–05): Identical grids, single cell change, scattered edits, row/column append
- Alignment Cases (UC-06–10): Row/column insert/delete in middle, block operations
- Move Detection Cases (UC-11–13): Block moved rows/columns, fuzzy moves
- Adversarial Cases (UC-14–16): Repetitive content, completely different grids, sparse grids
- Database Mode Cases (UC-17–20): Keyed rows, duplicate keys, mixed sheets

#### Current Implementation Coverage

| Use Case | Implemented | Notes |
|----------|-------------|-------|
| UC-01: Identical grids | ✅ Partial | Returns empty diff, but no early termination optimization |
| UC-02: Single cell change | ✅ Yes | Correctly detects single cell edits |
| UC-03: Scattered edits | ✅ Yes | Multiple CellEdited ops emitted |
| UC-04: Row append | ❌ No | Reports as CellEdited, not RowAdded |
| UC-05: Column append | ❌ No | Reports as CellEdited, not ColumnAdded |
| UC-06–10: Alignment | ❌ No | No row/column alignment implemented |
| UC-11–13: Move detection | ❌ No | DiffOp variants exist but not detected |
| UC-14–16: Adversarial | ❌ No | No special handling |
| UC-17–20: Database mode | ❌ No | Not started |

#### Assessment

The implementation handles the simplest use cases (identical grids, cell edits) but lacks the structural operations that define the product's value proposition. The **DiffOp enum is forward-looking**, containing all the variants needed for future work:

```rust
// From diff.rs - variants are ready but not emitted by current engine
pub enum DiffOp {
    SheetAdded { sheet: SheetId },
    SheetRemoved { sheet: SheetId },
    RowAdded { sheet: SheetId, row_idx: u32, row_signature: Option<RowSignature> },
    RowRemoved { sheet: SheetId, row_idx: u32, row_signature: Option<RowSignature> },
    ColumnAdded { sheet: SheetId, col_idx: u32, col_signature: Option<ColSignature> },
    ColumnRemoved { sheet: SheetId, col_idx: u32, col_signature: Option<ColSignature> },
    BlockMovedRows { sheet: SheetId, src_start_row: u32, row_count: u32, dst_start_row: u32, block_hash: Option<u64> },
    BlockMovedColumns { sheet: SheetId, src_start_col: u32, col_count: u32, dst_start_col: u32, block_hash: Option<u64> },
    CellEdited { sheet: SheetId, addr: CellAddress, from: CellSnapshot, to: CellSnapshot },
}
```

**Verdict**: Infrastructure is prepared for the spec's requirements; algorithm implementation is the gap.

---

### Part II: Architecture & Data Structures

#### Specification Requirements

The spec defines a **six-phase pipeline**:
1. Preprocessing & Hashing
2. Dimension Decision
3. Mode Selection
4. Alignment (Spreadsheet or Database)
5. Cell-Level Diff
6. Result Assembly

And a **three-layer data structure design**:
- Layer 0: Sparse Grid IR (persistent)
- Layer 1: Ephemeral GridView (algorithm-friendly)
- Layer 2: Alignment results (output-oriented)

#### Current Implementation

**Pipeline Implementation:**

| Phase | Implementation Status | Location |
|-------|----------------------|----------|
| Phase 1: Preprocessing | Partial | `workbook.rs` — Grid with signature computation |
| Phase 2: Dimension Decision | Not implemented | — |
| Phase 3: Mode Selection | Not implemented | — |
| Phase 4: Alignment | Naive only | `engine.rs` — O(R×C) cell loop |
| Phase 5: Cell-Level Diff | Partial | `engine.rs` — `CellSnapshot` comparison |
| Phase 6: Result Assembly | Partial | `diff.rs` — `DiffReport::new()` |

**Data Structure Implementation:**

The spec's Layer 0 (Sparse Grid IR) is **well-implemented**:

```rust
// From workbook.rs - matches spec's sparse storage requirements
pub struct Grid {
    pub nrows: u32,
    pub ncols: u32,
    pub cells: HashMap<(u32, u32), Cell>,
    pub row_signatures: Option<Vec<RowSignature>>,
    pub col_signatures: Option<Vec<ColSignature>>,
}
```

This aligns with the spec's design principle: "Never allocate structures proportional to the full grid dimensions."

The spec's Layer 1 (GridView) is **not implemented**:
- No ephemeral row-oriented views
- No `RowMeta` or `ColMeta` with hash/frequency data
- No token interning

The spec's Layer 2 (Alignment results) is **partially ready**:
- `DiffOp` and `DiffReport` types are complete
- No `RowAlignment`, `ColumnAlignment`, or `ValidatedMoves` structures

#### Assessment

The foundational data structures are solid but incomplete. The implementation correctly chose a sparse representation for memory efficiency, which the spec later ratified. However, the algorithmic scaffolding (GridView, alignment structures) is missing.

**Critical Gap**: The current engine is an O(R×C) nested loop:

```rust
// From engine.rs - does not implement the spec's alignment pipeline
fn diff_grids(sheet_id: &SheetId, old: &Grid, new: &Grid, ops: &mut Vec<DiffOp>) {
    let max_rows = old.nrows.max(new.nrows);
    let max_cols = old.ncols.max(new.ncols);

    for row in 0..max_rows {
        for col in 0..max_cols {
            // Cell-by-cell comparison without alignment
            let old_cell = old.get(row, col);
            let new_cell = new.get(row, col);
            // ...
        }
    }
}
```

This fundamentally contradicts the spec's Anchor-Move-Refine algorithm and cannot handle insertions, deletions, or moves.

**Verdict**: Data structures are on track; algorithm is placeholder only.

---

### Part III: Preprocessing — Fingerprinting & Hashing

#### Specification Requirements

The spec defines comprehensive fingerprinting:
- Cell normalization (numeric, string, boolean, error, empty)
- Row hash computation with column position encoding
- Column hash computation with row position encoding
- Formula normalization (basic and advanced)
- Frequency analysis and row classification
- Token interning for efficient sequence operations

#### Current Implementation

**Implemented:**
- Basic row signature computation via `DefaultHasher`
- Basic column signature computation
- `compute_all_signatures()` method on Grid

```rust
// From workbook.rs
pub fn compute_row_signature(&self, row: u32) -> RowSignature {
    let mut hasher = DefaultHasher::new();
    for col in 0..self.ncols {
        if let Some(cell) = self.get(row, col) {
            cell.value.hash(&mut hasher);
            cell.formula.hash(&mut hasher);
        }
    }
    RowSignature { hash: hasher.finish() }
}
```

**Not Implemented:**
- Cell value normalization (numeric rounding, string trimming, NaN handling)
- Column position inclusion in row hashes (the spec requires this to distinguish permutations)
- Type discriminant tags in hashes
- Frequency analysis and row classification (unique/rare/common)
- Token interning
- Parallel hash computation

#### Assessment

The signature computation exists but lacks the sophistication required by the spec:

| Spec Requirement | Implementation | Gap |
|-----------------|----------------|-----|
| Column position in hash | ❌ | Rows with swapped values hash identically |
| Type discriminant | ❌ | String "1" and Number 1 may collide |
| Numeric normalization | ❌ | Floating-point representation differences cause false positives |
| Empty cell handling | ⚠️ Partial | Empty cells contribute nothing, which is correct |
| XXHash64/BLAKE3 | ❌ | Uses DefaultHasher (SipHash) |
| Frequency tables | ❌ | Not implemented |

**Verdict**: Signatures exist but need enhancement before alignment algorithms can use them effectively.

---

### Part IV: Spreadsheet Mode Alignment

#### Specification Requirements

This is the core of the spec — the **Anchor-Move-Refine (AMR) Algorithm**:

1. **Anchor Discovery** via Patience Diff
   - Filter to unique rows
   - Build match pairs
   - Extract LIS for monotonic anchor chain
   
2. **Move Candidate Extraction**
   - Identify out-of-order matches
   - Cluster into block candidates
   
3. **Move-Aware Gap Filling**
   - Select strategy: trivial, Myers, RLE, bail-out
   - Mark moved rows, align remaining
   
4. **Move Validation**
   - Verify source/destination consistency
   - Compute similarity for fuzzy moves
   
5. **Column Alignment**
   - Apply similar process to columns

#### Current Implementation

**None of this is implemented.**

The entire alignment pipeline is replaced by a naive cell-by-cell comparison that:
- Assumes rows at position N in Grid A correspond to rows at position N in Grid B
- Cannot detect insertions or deletions
- Cannot detect moves
- Reports misaligned cells as edits rather than structural changes

#### Assessment

This is the **largest gap** between spec and implementation, and it's expected at this phase. The spec rates the grid diff algorithm as the **single most difficult component** (18/20 difficulty score).

The infrastructure is prepared:
- `DiffOp::RowAdded`, `RowRemoved`, `BlockMovedRows`, etc. exist
- `RowSignature` and `ColSignature` types exist
- The sparse Grid structure can support the required algorithms

But the algorithms themselves require significant work:
- Patience Diff / LIS implementation
- Gap filling with Myers or histogram diff
- RLE compression for repetitive content
- Move detection with bipartite matching

**Verdict**: Critical path work not started; infrastructure is ready.

---

### Part V: Database Mode Alignment

#### Specification Requirements

Database mode provides key-based row matching:
- Key extraction from designated columns
- Hash join for O(N) expected alignment
- Duplicate key handling via LAPJV
- Mixed-mode support (table regions + free-form)

#### Current Implementation

**Not implemented.** No key-based alignment, no mode selection, no region detection.

#### Assessment

Database mode is a significant differentiator from competitors. The spec explicitly calls out that incumbent tools rarely support this. However, spreadsheet mode is the more common use case and should be prioritized.

**Verdict**: Deferred work; not critical for MVP.

---

### Part VI: Cell-Level Comparison

#### Specification Requirements

Cell comparison includes:
- Type-aware equality (number tolerance, string normalization)
- Formula semantic comparison (optional AST-based)
- Cell change categorization

#### Current Implementation

Cell comparison is functional but basic:

```rust
// From workbook.rs - CellSnapshot equality ignores address, compares value+formula
impl PartialEq for CellSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.formula == other.formula
    }
}
```

The implementation correctly:
- Compares by value and formula
- Ignores address in equality (semantic comparison)
- Handles all CellValue types (Number, Text, Bool)

The implementation lacks:
- Numeric tolerance for floating-point comparison
- String normalization
- Formula AST comparison (only string comparison)

#### Assessment

Cell comparison works for basic cases. The spec's advanced features (formula AST, numeric tolerance) are nice-to-have enhancements that can be added later.

**Verdict**: Adequate for current phase; enhancements needed later.

---

### Part VII: Result Assembly & Output

#### Specification Requirements

Result assembly includes:
- Coalescing consecutive operations (RowAdded → BlockAddedRows)
- Deduplicating moves vs add/remove
- Detecting rectangular moves
- Deterministic sorting
- JSON serialization

#### Current Implementation

**DiffOp and DiffReport are well-implemented:**

```rust
// From diff.rs
pub struct DiffReport {
    pub version: String,
    pub ops: Vec<DiffOp>,
}
```

**JSON serialization is complete:**

```rust
// From output/json.rs
pub fn serialize_diff_report(report: &DiffReport) -> serde_json::Result<String> {
    // Handles non-finite number rejection
    serde_json::to_string(report)
}
```

**Not implemented:**
- Operation coalescing (consecutive adds → block add)
- Move deduplication
- Rectangular move detection
- Deterministic sorting of operations

#### Assessment

The output layer is well-designed with forward-looking types. The `DiffOp` enum includes all necessary variants, and JSON serialization works correctly. Advanced coalescing features can be added when the alignment algorithms produce the raw operations.

**Verdict**: Good foundation; coalescing is future work.

---

### Part VIII: Robustness & Performance

#### Specification Requirements

The spec defines:
- Memory budget management (1.5GB total, 600MB per grid)
- Streaming/chunked processing for large files
- Cost-capped bail-out for pathological gaps
- Early termination for identical/completely different grids
- Parallel processing opportunities

#### Current Implementation

**Error handling is well-implemented:**

```rust
// Layered error types matching spec exactly
pub enum ExcelOpenError {
    Container(ContainerError),
    GridParse(GridParseError),
    // ...
}
```

**Not implemented:**
- Memory budget tracking
- Streaming processing
- Bail-out strategies
- Early termination detection
- Parallel processing

#### Assessment

The error handling infrastructure is excellent. Performance and robustness features are appropriately deferred until the core algorithms exist.

**Verdict**: Error handling solid; performance optimization is future work.

---

## Alignment Summary

### What's Working Well

1. **Layered Architecture**: Container → Parser → IR → Diff → Output matches the spec's vision
2. **Sparse Grid IR**: Memory-efficient representation that supports algorithm requirements
3. **DiffOp Schema**: All variants defined with correct serialization
4. **Error Handling**: Proper layering with contextual errors
5. **Test Coverage**: PG1-PG4 tests validate implemented features

### What Needs Work

1. **Grid Diff Algorithm**: The entire AMR pipeline is unimplemented
2. **Row/Column Signatures**: Need enhancement (column position, type tags, frequency analysis)
3. **Alignment Data Structures**: GridView, RowMeta, ColMeta, alignment results
4. **Coalescing and Optimization**: Post-processing of diff results

### Known Discrepancies (from previous analysis)

| ID | Issue | Status | Recommendation |
|----|-------|--------|----------------|
| D1 | Sheet identity case-sensitive | Resolved (2025-11-30-sheet-identity-ci) | Covered by engine + fixture tests; keep focused on grid diff/signature work |
| D2 | UTF-16 handling undocumented | Open | Document in spec |
| D3 | Missing IR fields (mashup, tables) | Open | Add placeholders |
| D4 | Sparse vs row-oriented Grid | Resolved | Sparse is correct choice |
| D5 | Missing M/DAX DiffOp variants | Open | Add placeholders |

Sheet identity now matches the spec: the engine keys sheets by `(lowercase name, SheetKind)`, orders workbook-level ops deterministically, and is guarded by both in-memory engine tests and fixture-based case-only rename checks.

---

## Trajectory Assessment

### Is the Implementation on Track?

**Yes, with caveats.**

The implementation represents a solid foundation for the specified system. The architectural decisions (sparse Grid, layered errors, comprehensive DiffOp schema) align with the spec and will support future development.

The main risk is **not** architectural deviation but **incomplete implementation** of the core algorithms. The naive cell-by-cell engine is a placeholder that must be replaced before the product can deliver its value proposition.

### Recommended Priority Order

1. **Immediate**: Enhance row signatures (column positions, type tags)
2. **Soon**: Implement frequency analysis and anchor discovery
3. **Soon**: Implement Patience Diff for row anchoring
4. **Medium-term**: Implement gap filling (Myers diff)
5. **Medium-term**: Implement move detection
6. **Later**: Database mode, formula AST, performance optimization

### Risk Matrix

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Algorithm complexity underestimated | Medium | High | Start with simpler cases (UC-01–UC-10) |
| Sparse Grid insufficient for algorithms | Low | High | Already validated by analysis |
| Performance targets missed | Medium | Medium | Build metrics infrastructure early |
| Hash collisions cause incorrect diffs | Low | Medium | Verify matches after hash-based alignment |

---

## Conclusion

The codebase is **on a healthy trajectory** toward implementing the specification. The foundational layers (container handling, parsing, IR, output serialization) are well-implemented and align closely with the spec. The implementation has made good architectural choices that the spec later ratified.

The critical path forward is implementing the grid diff algorithms (Spec Part IV). This is appropriately recognized as the hardest work and represents the main gap between current state and end-state. The infrastructure is ready to support this work; what remains is the algorithmic implementation itself.

**Overall Assessment**: The implementation is approximately **30% complete** and **on track** for successful completion of the specified system.

