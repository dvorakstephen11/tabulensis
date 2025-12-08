# Excel Diff Engine: Sprint Plan

This plan addresses all recommendations from the midway design evaluation, organized into seven feature branches with explicit dependencies, acceptance criteria, and technical specifications.

---

## Branch Dependency Graph

```
                    ┌─────────────────────┐
                    │  Branch 3: Config   │
                    │    (DiffConfig)     │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
              ▼                ▼                ▼
┌─────────────────────┐ ┌─────────────────┐ ┌─────────────────────┐
│ Branch 1: Grid      │ │ Branch 4: WASM  │ │ Branch 5: API       │
│ Correctness         │ │ & Memory        │ │ Unification         │
└──────────┬──────────┘ └────────┬────────┘ └─────────────────────┘
           │                     │
           ▼                     │
┌─────────────────────┐          │
│ Branch 2: Grid      │◄─────────┘
│ Scalability (AMR)   │
│ + Perf Infra        │
└─────────────────────┘

┌─────────────────────┐  ┌─────────────────────┐
│ Branch 6: M Parser  │  │ Branch 7: Formula   │
│ (Independent)       │  │ Parser (Independent)│
└─────────────────────┘  └─────────────────────┘
```

**Critical Path:** Branch 3 → Branch 1 → Branch 2

**Parallel Work:** Branches 5, 6, 7 can proceed independently. Branch 4 can proceed in parallel with Branch 1 but must complete before Branch 2's performance validation.

---

## Branch 1: Grid Algorithm Correctness & Hashing

**Goal:** Fix all correctness bugs in the core grid diff algorithm before tackling scalability.

**Depends on:** Branch 3 (DiffConfig) — thresholds for hashing and similarity should come from config.

**Evaluation References:** Priority Recommendations #1 (partial), #8, #9

### 1.1 Fix Silent Data Loss on Rectangular Block Moves

**Problem:** When `detect_rect_block_move` finds a match, `diff_grids` emits the move and returns immediately. Edits outside the moved region are never computed.

**Technical Specification:**

```rust
pub fn diff_grids(old: &GridView, new: &GridView, config: &DiffConfig) -> Vec<DiffOp> {
    let mut ops = Vec::new();
    let mut old_mask = CellMask::all_active(old);
    let mut new_mask = CellMask::all_active(new);

    // Phase 1: Iterative move detection (see 1.2)
    loop {
        if let Some(rect_move) = detect_rect_block_move(old, new, &old_mask, &new_mask, config) {
            ops.push(DiffOp::RectMoved { ... });
            old_mask.exclude_rect(rect_move.old_bounds);
            new_mask.exclude_rect(rect_move.new_bounds);
            continue;
        }
        if let Some(row_move) = detect_row_block_move(old, new, &old_mask, &new_mask, config) {
            ops.push(DiffOp::RowBlockMoved { ... });
            old_mask.exclude_rows(row_move.old_range);
            new_mask.exclude_rows(row_move.new_range);
            continue;
        }
        // ... column block moves ...
        break;
    }

    // Phase 2: Alignment on remaining (unmasked) cells
    let alignment_ops = align_remaining(old, new, &old_mask, &new_mask, config);
    ops.extend(alignment_ops);

    ops
}
```

**Deliverables:**
- [ ] Implement `CellMask` (or `RegionMask`) to track which cells have been accounted for
- [ ] Refactor move detection functions to accept masks and only consider unmasked cells
- [ ] Refactor `diff_grids` to continue after move detection rather than early return
- [ ] Add test: rect move + cell edit outside moved region → both reported
- [ ] Add test: rect move + row insertion outside moved region → both reported

### 1.2 Make Move Detection Iterative

**Problem:** The engine detects at most one structural move per sheet.

**Technical Specification:**

The loop structure in 1.1 handles this. Key considerations:
- Each iteration must make progress (move at least one cell from active to accounted)
- Iteration cap as a safety valve (configurable via `DiffConfig::max_move_iterations`)
- Moves should be detected in order of "confidence" or "size" to avoid fragmentation

**Deliverables:**
- [ ] Implement the iterative loop with mask subtraction
- [ ] Add `DiffConfig::max_move_iterations` (default: 10)
- [ ] Add test: two disjoint row block moves → both detected
- [ ] Add test: row block move + column block move → both detected
- [ ] Add test: three rect moves → all three detected

### 1.3 Remove Column-Index Dependency from Row Hashes

**Problem:** `hash_cell_contribution(col, cell)` includes the column index, so inserting a column at position 0 invalidates every row hash.

**Technical Specification:**

Option A — Content-only row signatures:
```rust
fn compute_row_signature(row: &[CellSnapshot]) -> u128 {
    let mut hasher = XxHash128::default();
    for cell in row.iter().filter(|c| !c.is_blank()) {
        cell.value.hash(&mut hasher);
        cell.formula.hash(&mut hasher);
    }
    hasher.finish_128()
}
```

Option B — Position-invariant multiset hash (commutative):
```rust
fn compute_row_signature(row: &[CellSnapshot]) -> u128 {
    row.iter()
        .filter(|c| !c.is_blank())
        .map(|c| hash_cell_content(c))
        .fold(0u128, |acc, h| acc.wrapping_add(h))
}
```

**Decision:** Option A (ordered) is preferred because row content order matters semantically. The key insight is that we hash *content* in *content order*, not *position*.

**Deliverables:**
- [ ] Refactor `hash_cell_contribution` to exclude column index
- [ ] Update `GridView::from_grid` to use new signature computation
- [ ] Add test: insert column at position 0 → row alignment still succeeds
- [ ] Add test: delete column from middle → row alignment still succeeds
- [ ] Verify existing tests still pass (semantic equivalence preserved)

### 1.4 Upgrade to 128-bit Row Signatures

**Problem:** 64-bit hashes have non-negligible collision probability at 50K rows across large corpora.

**Technical Specification:**

```rust
pub struct RowSignature(u128);

impl RowSignature {
    pub fn compute(row: &[CellSnapshot]) -> Self {
        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        // ... hash content ...
        Self(hasher.digest128())
    }
}
```

**Deliverables:**
- [ ] Add `xxhash-rust` dependency (or use `siphasher` for SipHash128)
- [ ] Change `row_signatures` type from `Vec<u64>` to `Vec<u128>`
- [ ] Update all signature comparisons
- [ ] Document collision probability analysis in code comments

### 1.5 Semantic Float Normalization

**Problem:** `f64::to_bits()` distinguishes `0.0` from `-0.0` and is sensitive to ULP drift.

**Technical Specification:**

```rust
fn normalize_float_for_hash(n: f64) -> u64 {
    if n.is_nan() {
        return CANONICAL_NAN_BITS;
    }
    if n == 0.0 {
        return 0u64; // Canonical zero (handles -0.0)
    }
    // Round to 15 significant digits
    let magnitude = n.abs().log10().floor() as i32;
    let scale = 10f64.powi(14 - magnitude);
    let normalized = (n * scale).round() / scale;
    normalized.to_bits()
}
```

**Deliverables:**
- [ ] Implement `normalize_float_for_hash` function
- [ ] Use normalized value in `CellValue::Number` hashing
- [ ] Add test: `0.0` and `-0.0` hash identically
- [ ] Add test: `1.0` and `1.0000000000000002` hash identically
- [ ] Add test: `1.0` and `1.0001` hash differently (beyond epsilon)
- [ ] Add test: `NaN` values hash identically regardless of payload

### Acceptance Criteria for Branch 1

- [ ] All existing tests pass
- [ ] No silent data loss: every cell difference is reported
- [ ] Multiple moves per sheet are detected and reported
- [ ] Column insertion/deletion does not break row alignment
- [ ] Float comparison is semantically correct (no spurious diffs from recalc noise)
- [ ] Hash collision probability documented and acceptable (<10^-18 at 50K rows)

---

## Branch 2: Grid Algorithm Scalability (AMR) + Performance Infrastructure

**Goal:** Implement the Anchor-Move-Refine alignment algorithm and remove hard caps. Establish performance regression testing.

**Depends on:** Branch 1 (correctness fixes), Branch 3 (DiffConfig), Branch 4 (memory optimizations help but not strictly required)

**Evaluation References:** Priority Recommendations #1 (main), #6, #7

### 2.1 Refactor Alignment into Spec-Aligned Phases

**Problem:** Current alignment is a monolithic function with interleaved concerns.

**Technical Specification:**

Create module structure:
```
src/
  alignment/
    mod.rs
    row_metadata.rs      // RowMeta, frequency classification
    anchor_discovery.rs  // Find unique/rare row anchors
    anchor_chain.rs      // LIS-based global anchor chain
    move_extraction.rs   // Identify candidate moves from anchor gaps
    gap_strategy.rs      // Per-gap alignment decisions
    assembly.rs          // Final alignment construction
    column_alignment.rs  // Parallel structure for columns
```

Each module should have:
- Clear input/output types
- Unit tests for that phase in isolation
- Doc comments referencing spec sections

**Deliverables:**
- [ ] Create `alignment/` module structure
- [ ] Extract `RowMeta` struct (hash, non_blank_count, frequency_class)
- [ ] Extract `collect_row_metadata` function
- [ ] Extract `discover_anchors` function
- [ ] Extract `build_anchor_chain` (LIS) function
- [ ] Extract `extract_move_candidates` function
- [ ] Extract `fill_gap` function
- [ ] Extract `assemble_alignment` function
- [ ] Ensure existing tests pass through refactored code path

### 2.2 Implement Row Frequency Classification

**Technical Specification:**

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FrequencyClass {
    Unique,      // Appears exactly once
    Rare,        // Appears 2-5 times (configurable)
    Common,      // Appears 6+ times
    LowInfo,     // Blank or near-blank (< N non-blank cells)
}

pub struct RowMeta {
    pub signature: RowSignature,
    pub non_blank_count: u32,
    pub frequency_class: FrequencyClass,
}

impl GridView {
    pub fn classify_row_frequencies(&mut self, config: &DiffConfig) {
        let mut freq_map: HashMap<RowSignature, u32> = HashMap::new();
        for sig in &self.row_signatures {
            *freq_map.entry(*sig).or_default() += 1;
        }
        for (i, sig) in self.row_signatures.iter().enumerate() {
            let count = freq_map[sig];
            self.row_meta[i].frequency_class = match count {
                1 => FrequencyClass::Unique,
                2..=config.rare_threshold => FrequencyClass::Rare,
                _ => FrequencyClass::Common,
            };
            if self.row_meta[i].non_blank_count < config.low_info_threshold {
                self.row_meta[i].frequency_class = FrequencyClass::LowInfo;
            }
        }
    }
}
```

**Deliverables:**
- [ ] Implement `FrequencyClass` enum
- [ ] Implement `RowMeta` struct
- [ ] Add frequency classification to `GridView`
- [ ] Add `DiffConfig::rare_threshold` (default: 5)
- [ ] Add `DiffConfig::low_info_threshold` (default: 2 non-blank cells)
- [ ] Add tests for frequency classification edge cases

### 2.3 Implement Anchor Discovery and LIS Chain

**Technical Specification:**

```rust
pub struct Anchor {
    pub old_row: u32,
    pub new_row: u32,
    pub signature: RowSignature,
}

pub fn discover_anchors(
    old: &GridView,
    new: &GridView,
    config: &DiffConfig,
) -> Vec<Anchor> {
    // Find rows that are Unique in BOTH grids with matching signatures
    let old_unique: HashMap<RowSignature, u32> = old.row_meta.iter()
        .enumerate()
        .filter(|(_, m)| m.frequency_class == FrequencyClass::Unique)
        .map(|(i, m)| (m.signature, i as u32))
        .collect();

    let mut anchors = Vec::new();
    for (new_idx, meta) in new.row_meta.iter().enumerate() {
        if meta.frequency_class == FrequencyClass::Unique {
            if let Some(&old_idx) = old_unique.get(&meta.signature) {
                anchors.push(Anchor {
                    old_row: old_idx,
                    new_row: new_idx as u32,
                    signature: meta.signature,
                });
            }
        }
    }
    anchors
}

pub fn build_anchor_chain(anchors: Vec<Anchor>) -> Vec<Anchor> {
    // LIS on old_row indices to find longest increasing subsequence
    // This gives us the maximal set of anchors that preserve relative order
    lis_by_key(anchors, |a| a.old_row)
}
```

**Deliverables:**
- [ ] Implement `Anchor` struct
- [ ] Implement `discover_anchors` function
- [ ] Implement LIS algorithm (`lis_by_key`)
- [ ] Implement `build_anchor_chain` function
- [ ] Add test: simple anchor discovery (3 unique rows)
- [ ] Add test: LIS selection (anchors with crossings)
- [ ] Add test: no anchors (all rows common) → graceful fallback

### 2.4 Implement Multi-Gap Alignment

**Problem:** Current alignment handles only single contiguous insert/delete blocks.

**Technical Specification:**

The anchor chain divides both grids into gaps. For each gap:

```rust
pub enum GapStrategy {
    Empty,           // Both sides empty, nothing to do
    InsertAll,       // Old side empty, all new rows are insertions
    DeleteAll,       // New side empty, all old rows are deletions
    SmallEdit,       // Both sides small, use cell-level diff
    MoveCandidate,   // Check for block moves within gap
    RecursiveAlign,  // Gap large enough to recurse with rare anchors
}

pub fn select_gap_strategy(
    old_gap: Range<u32>,
    new_gap: Range<u32>,
    old: &GridView,
    new: &GridView,
    config: &DiffConfig,
) -> GapStrategy {
    let old_len = old_gap.len();
    let new_len = new_gap.len();

    if old_len == 0 && new_len == 0 {
        return GapStrategy::Empty;
    }
    if old_len == 0 {
        return GapStrategy::InsertAll;
    }
    if new_len == 0 {
        return GapStrategy::DeleteAll;
    }
    if old_len <= config.small_gap_threshold && new_len <= config.small_gap_threshold {
        return GapStrategy::SmallEdit;
    }
    // Check for potential moves
    if has_matching_signatures_in_gap(old, new, old_gap, new_gap) {
        return GapStrategy::MoveCandidate;
    }
    if old_len > config.recursive_threshold || new_len > config.recursive_threshold {
        return GapStrategy::RecursiveAlign;
    }
    GapStrategy::SmallEdit
}
```

**Deliverables:**
- [ ] Implement `GapStrategy` enum
- [ ] Implement `select_gap_strategy` function
- [ ] Implement `fill_gap` dispatcher
- [ ] Implement `InsertAll` gap handler
- [ ] Implement `DeleteAll` gap handler
- [ ] Implement `SmallEdit` gap handler (cell-level diff)
- [ ] Implement `MoveCandidate` gap handler
- [ ] Implement `RecursiveAlign` gap handler (recurse with rare anchors)
- [ ] Add test: two disjoint insertion regions → both aligned correctly
- [ ] Add test: insertion + deletion in different regions → both reported
- [ ] Add test: gap contains moved block → move detected within gap

### 2.5 Remove/Raise Hard Caps

**Technical Specification:**

Replace hard constants with configurable limits that serve as safety valves, not functional restrictions:

```rust
pub struct DiffConfig {
    // Safety caps (very high defaults, for truly pathological cases)
    pub max_align_rows: u32,        // Default: 500_000 (was 2_000)
    pub max_align_cols: u32,        // Default: 16_384 (was 64)
    pub max_block_gap: u32,         // Default: 10_000 (was 32)

    // Operational limits
    pub max_move_iterations: u32,   // Default: 20
    pub max_recursion_depth: u32,   // Default: 10

    // When limits hit, behavior selection
    pub on_limit_exceeded: LimitBehavior,
}

pub enum LimitBehavior {
    FallbackToPositional,  // Current behavior
    ReturnPartialResult,   // Return what we have + marker
    ReturnError,           // Fail explicitly
}
```

**Deliverables:**
- [ ] Add configurable limits to `DiffConfig`
- [ ] Remove hardcoded `MAX_ALIGN_ROWS`, `MAX_ALIGN_COLS`, `MAX_BLOCK_GAP` constants
- [ ] Harmonize column caps across modules (alignment vs rect_block_move)
- [ ] Implement `LimitBehavior` handling
- [ ] Add test: 50K row grid aligns successfully
- [ ] Add test: 500 column grid aligns successfully
- [ ] Add test: limit exceeded → configured behavior occurs

### 2.6 Run-Length Encoding for Repetitive Rows (Optional Optimization)

**Technical Specification:**

For grids where >50% of rows share signatures with other rows:

```rust
pub struct RowRun {
    pub signature: RowSignature,
    pub start_row: u32,
    pub count: u32,
}

pub fn compress_to_runs(meta: &[RowMeta]) -> Vec<RowRun> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < meta.len() {
        let sig = meta[i].signature;
        let start = i;
        while i < meta.len() && meta[i].signature == sig {
            i += 1;
        }
        runs.push(RowRun {
            signature: sig,
            start_row: start as u32,
            count: (i - start) as u32,
        });
    }
    runs
}
```

This is an optimization for adversarial cases (99% blank rows, template rows). Implement if needed after core AMR is working.

**Deliverables:**
- [ ] Implement `RowRun` struct
- [ ] Implement `compress_to_runs` function
- [ ] Implement run-aware alignment (operates on runs, not individual rows)
- [ ] Add test: 10K identical blank rows → runs compress to 1 entry
- [ ] Add test: alternating pattern A-B-A-B → no compression benefit, falls back

### 2.7 Performance Infrastructure

**Technical Specification:**

```rust
#[cfg(feature = "perf-metrics")]
pub struct DiffMetrics {
    pub parse_time_ms: u64,
    pub alignment_time_ms: u64,
    pub move_detection_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub peak_memory_bytes: usize,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
}

#[cfg(feature = "perf-metrics")]
impl DiffMetrics {
    pub fn start_phase(&mut self, phase: Phase) { ... }
    pub fn end_phase(&mut self, phase: Phase) { ... }
}
```

**CI Integration:**

```yaml
# .github/workflows/perf.yml
perf-regression:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Build with perf metrics
      run: cargo build --release --features perf-metrics
    - name: Run perf suite
      run: cargo test --release --features perf-metrics perf_
    - name: Check thresholds
      run: python scripts/check_perf_thresholds.py
```

**Fixtures and Thresholds:**

| Fixture | Rows | Cols | Max Time | Max Memory |
|---------|------|------|----------|------------|
| p1_large_dense | 50,000 | 100 | 5s | 500MB |
| p2_large_noise | 50,000 | 100 | 10s | 600MB |
| p3_adversarial_repetitive | 50,000 | 50 | 15s | 400MB |
| p4_99_percent_blank | 50,000 | 100 | 2s | 200MB |
| p5_identical | 50,000 | 100 | 1s | 300MB |

**Deliverables:**
- [ ] Add `perf-metrics` feature flag
- [ ] Implement `DiffMetrics` struct
- [ ] Add timing instrumentation to key phases
- [ ] Add memory tracking (via `tikv-jemallocator` stats or similar)
- [ ] Create perf test fixtures (use existing manifest definitions)
- [ ] Create `scripts/check_perf_thresholds.py`
- [ ] Add perf regression job to CI
- [ ] Document baseline numbers in code/docs

### Acceptance Criteria for Branch 2

- [ ] AMR algorithm implemented with all phases
- [ ] Multi-gap alignment working (arbitrary number of disjoint edit regions)
- [ ] 50K×100 grid aligns in <5s on reference hardware
- [ ] No hardcoded caps below 100K rows
- [ ] Performance regression tests in CI
- [ ] Adversarial repetitive case (p3) completes without timeout
- [ ] All existing tests pass

---

## Branch 3: Configuration Infrastructure

**Goal:** Centralize all algorithm thresholds and behavioral knobs in a single `DiffConfig` type.

**Depends on:** Nothing (should be done early)

**Evaluation References:** Priority Recommendation #2

### 3.1 Define DiffConfig Structure

**Technical Specification:**

```rust
#[derive(Clone, Debug)]
pub struct DiffConfig {
    // Alignment thresholds
    pub rare_frequency_threshold: u32,
    pub low_info_cell_threshold: u32,
    pub small_gap_threshold: u32,
    pub recursive_align_threshold: u32,

    // Move detection
    pub fuzzy_similarity_threshold: f64,
    pub min_block_size_for_move: u32,
    pub max_move_iterations: u32,

    // Safety limits
    pub max_align_rows: u32,
    pub max_align_cols: u32,
    pub max_recursion_depth: u32,
    pub on_limit_exceeded: LimitBehavior,

    // Output control
    pub include_unchanged_cells: bool,
    pub max_context_rows: u32,

    // Feature flags
    pub enable_fuzzy_moves: bool,
    pub enable_formula_semantic_diff: bool,
    pub enable_m_semantic_diff: bool,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            rare_frequency_threshold: 5,
            low_info_cell_threshold: 2,
            small_gap_threshold: 50,
            recursive_align_threshold: 200,
            fuzzy_similarity_threshold: 0.80,
            min_block_size_for_move: 3,
            max_move_iterations: 20,
            max_align_rows: 500_000,
            max_align_cols: 16_384,
            max_recursion_depth: 10,
            on_limit_exceeded: LimitBehavior::FallbackToPositional,
            include_unchanged_cells: false,
            max_context_rows: 3,
            enable_fuzzy_moves: true,
            enable_formula_semantic_diff: false,
            enable_m_semantic_diff: true,
        }
    }
}

impl DiffConfig {
    pub fn fastest() -> Self {
        Self {
            enable_fuzzy_moves: false,
            small_gap_threshold: 20,
            ..Default::default()
        }
    }

    pub fn most_precise() -> Self {
        Self {
            fuzzy_similarity_threshold: 0.95,
            enable_formula_semantic_diff: true,
            ..Default::default()
        }
    }
}
```

**Deliverables:**
- [ ] Define `DiffConfig` struct with all fields
- [ ] Implement `Default` trait
- [ ] Implement preset constructors (`fastest`, `balanced`, `most_precise`)
- [ ] Add builder pattern for custom configurations
- [ ] Add serde serialization for config persistence

### 3.2 Thread Config Through the Pipeline

**Technical Specification:**

Update all diff function signatures:

```rust
// Before
pub fn diff_workbooks(old: &Workbook, new: &Workbook) -> DiffReport

// After
pub fn diff_workbooks(old: &Workbook, new: &Workbook, config: &DiffConfig) -> DiffReport
```

Functions to update:
- `diff_workbooks`
- `diff_grids`
- `diff_grids_database_mode`
- `align_rows`
- `align_columns`
- `detect_rect_block_move`
- `detect_row_block_move`
- `detect_column_block_move`
- `diff_m_queries`

**Deliverables:**
- [ ] Update all public diff APIs to accept `&DiffConfig`
- [ ] Update all internal functions to pass config through
- [ ] Remove all hardcoded `const` thresholds from algorithm code
- [ ] Update all tests to use explicit configs (or `DiffConfig::default()`)
- [ ] Add tests verifying different configs produce expected trade-offs

### Acceptance Criteria for Branch 3

- [ ] No hardcoded algorithm constants remain in diff code
- [ ] All diff functions accept `&DiffConfig`
- [ ] Preset configs work as documented
- [ ] Config is serializable for persistence/debugging
- [ ] Tests demonstrate config affects behavior

---

## Branch 4: WASM & Memory Readiness

**Goal:** Make the core engine compile to WASM and handle large workbooks without exhausting memory.

**Depends on:** Branch 3 (DiffConfig for streaming thresholds)

**Evaluation References:** Priority Recommendations #3, #4

### 4.1 String Interning

**Technical Specification:**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(u32);

pub struct StringPool {
    strings: Vec<String>,
    index: HashMap<String, StringId>,
}

impl StringPool {
    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.index.get(s) {
            return id;
        }
        let id = StringId(self.strings.len() as u32);
        self.strings.push(s.to_owned());
        self.index.insert(s.to_owned(), id);
        id
    }

    pub fn resolve(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }
}

// Updated CellValue
pub enum CellValue {
    Blank,
    Number(f64),
    Text(StringId),  // Was: Text(String)
    Boolean(bool),
    Error(StringId), // Was: Error(String)
}

// Updated SheetId
pub type SheetId = StringId;  // Was: String
```

**Deliverables:**
- [ ] Implement `StringPool` and `StringId`
- [ ] Update `CellValue::Text` to use `StringId`
- [ ] Update `SheetId` to use `StringId`
- [ ] Update `DiffOp` to use `StringId` for sheet references
- [ ] Thread `StringPool` through parsing and diff
- [ ] Update serialization to include string table in metadata
- [ ] Add test: 50K identical strings → single allocation
- [ ] Measure memory improvement on repetitive fixtures

### 4.2 Eliminate Coordinate Redundancy

**Technical Specification:**

Current `Cell` struct stores coordinates three times:
1. HashMap key `(row, col)`
2. `Cell.row` and `Cell.col` fields
3. `Cell.address.row` and `Cell.address.col`

Reduce to just the HashMap key:

```rust
// Before
pub struct Cell {
    pub row: u32,
    pub col: u32,
    pub address: CellAddress,
    pub value: CellValue,
    pub formula: Option<String>,
}

// After
pub struct CellContent {
    pub value: CellValue,
    pub formula: Option<StringId>,  // Also intern formulas
}

// Grid stores: HashMap<(u32, u32), CellContent>
// CellAddress derived on-demand when needed for output
```

**Deliverables:**
- [ ] Rename `Cell` to `CellContent`, remove coordinate fields
- [ ] Update `Grid` to use `HashMap<(u32, u32), CellContent>`
- [ ] Create `CellAddress::from_coords(row, col)` for output generation
- [ ] Update all code that accessed `cell.row`/`cell.col` to use key
- [ ] Measure memory improvement (expect ~16 bytes/cell saved)

### 4.3 Abstract I/O to Read + Seek

**Technical Specification:**

```rust
// Before (in container.rs)
pub fn open(path: &Path) -> Result<OpcContainer, ContainerError> {
    let file = std::fs::File::open(path)?;
    // ...
}

// After
pub fn open_from_reader<R: Read + Seek>(reader: R) -> Result<OpcContainer, ContainerError> {
    let mut zip = ZipArchive::new(reader)?;
    // ...
}

// Convenience wrapper in a separate module (not compiled for WASM)
#[cfg(feature = "std-fs")]
pub fn open_from_path(path: &Path) -> Result<OpcContainer, ContainerError> {
    let file = std::fs::File::open(path)?;
    open_from_reader(file)
}
```

**Deliverables:**
- [ ] Refactor `OpcContainer::open` to accept `R: Read + Seek`
- [ ] Create `std-fs` feature flag for path-based convenience functions
- [ ] Ensure core crate compiles with `default-features = false`
- [ ] Add WASM compile test to CI

### 4.4 Streaming Output

**Technical Specification:**

```rust
pub trait DiffSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;
    fn finish(self) -> Result<(), DiffError>;
}

pub struct VecSink(Vec<DiffOp>);
pub struct JsonLinesSink<W: Write>(W);
pub struct CallbackSink<F: FnMut(DiffOp)>(F);

impl<W: Write> DiffSink for JsonLinesSink<W> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        serde_json::to_writer(&mut self.0, &op)?;
        self.0.write_all(b"\n")?;
        Ok(())
    }
}

// Updated diff API
pub fn diff_workbooks_streaming<S: DiffSink>(
    old: &Workbook,
    new: &Workbook,
    config: &DiffConfig,
    sink: &mut S,
) -> Result<DiffSummary, DiffError>
```

**Ordering Decision:**

The evaluation notes tension between streaming and globally-sorted output. Resolution:

- **Within a sheet:** Emit in row-major order (stable, predictable)
- **Across sheets:** Emit sheets in sorted name order
- **No global sort** of the entire diff; order is deterministic by construction

This allows true streaming without a collection-and-sort step.

**Deliverables:**
- [ ] Define `DiffSink` trait
- [ ] Implement `VecSink` (for backward compatibility)
- [ ] Implement `JsonLinesSink`
- [ ] Implement `CallbackSink`
- [ ] Refactor diff engine to emit through sink rather than return Vec
- [ ] Provide `diff_workbooks` wrapper that uses `VecSink` internally
- [ ] Remove global sort from diff pipeline
- [ ] Document ordering guarantees
- [ ] Add test: streaming output produces same ops as vec output (order may differ)

### 4.5 WASM Build Gate

**Technical Specification:**

```yaml
# .github/workflows/wasm.yml
wasm-build:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install wasm target
      run: rustup target add wasm32-unknown-unknown
    - name: Build core for WASM
      run: cargo build --target wasm32-unknown-unknown --no-default-features -p excel_diff_core
    - name: Check size
      run: |
        SIZE=$(stat -c%s target/wasm32-unknown-unknown/release/excel_diff_core.wasm)
        if [ $SIZE -gt 5000000 ]; then
          echo "WASM size $SIZE exceeds 5MB limit"
          exit 1
        fi
```

**Deliverables:**
- [ ] Add `wasm32-unknown-unknown` build to CI
- [ ] Fix any compilation errors for WASM target
- [ ] Add WASM size budget check (initial target: <5MB)
- [ ] Create minimal WASM smoke test (parse small workbook, run diff)
- [ ] Document WASM usage in README

### Acceptance Criteria for Branch 4

- [ ] String interning implemented and reduces memory on repetitive data
- [ ] Coordinate redundancy eliminated
- [ ] Core crate compiles to WASM
- [ ] Streaming output works without full materialization
- [ ] WASM build in CI with size budget
- [ ] 50K row workbook parseable without OOM in 256MB WASM heap

---

## Branch 5: API Unification

**Goal:** Present a single, coherent domain model and diff API to consumers.

**Depends on:** Branch 3 (DiffConfig)

**Evaluation References:** Priority Recommendation #5

### 5.1 Implement WorkbookPackage

**Technical Specification:**

```rust
pub struct WorkbookPackage {
    pub workbook: Workbook,
    pub data_mashup: Option<DataMashup>,
    // Future: pub data_model: Option<DataModel>,
}

impl WorkbookPackage {
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self, PackageError> {
        let mut container = OpcContainer::open_from_reader(reader)?;

        let workbook = parse_workbook(&mut container)?;

        let data_mashup = match container.get_part("customXml/item1.xml") {
            Some(part) => Some(parse_data_mashup(part)?),
            None => None,
        };

        Ok(Self { workbook, data_mashup })
    }
}
```

**Deliverables:**
- [ ] Define `WorkbookPackage` struct
- [ ] Implement `WorkbookPackage::open`
- [ ] Consolidate error types into `PackageError`
- [ ] Update examples and documentation

### 5.2 Extend DiffOp for M Queries

**Technical Specification:**

```rust
pub enum DiffOp {
    // Existing grid operations
    SheetAdded { sheet: SheetId },
    SheetRemoved { sheet: SheetId },
    RowsInserted { sheet: SheetId, start: u32, count: u32 },
    RowsDeleted { sheet: SheetId, start: u32, count: u32 },
    RowBlockMoved { sheet: SheetId, from: u32, to: u32, count: u32 },
    // ... other grid ops ...
    CellEdited { sheet: SheetId, row: u32, col: u32, old: CellSnapshot, new: CellSnapshot },

    // New: M query operations
    QueryAdded { name: StringId },
    QueryRemoved { name: StringId },
    QueryDefinitionChanged {
        name: StringId,
        change_kind: QueryChangeKind,
        old_hash: u64,
        new_hash: u64,
    },
    QueryMetadataChanged { name: StringId, field: StringId },

    // Future: DAX operations (reserved)
    // MeasureAdded { ... },
    // MeasureRemoved { ... },
    // MeasureDefinitionChanged { ... },
}

pub enum QueryChangeKind {
    Semantic,      // AST structure changed
    FormattingOnly, // Whitespace/comments only
    Renamed,       // Query renamed (not yet emitted)
}
```

**Deliverables:**
- [ ] Add M query variants to `DiffOp`
- [ ] Update `diff_m_queries` to return `Vec<DiffOp>` instead of `Vec<MQueryDiff>`
- [ ] Deprecate `MQueryDiff` type (or make it internal)
- [ ] Add reserved variant comments for DAX
- [ ] Update serialization schema

### 5.3 Unified diff_packages API

**Technical Specification:**

```rust
impl WorkbookPackage {
    pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
        let mut ops = Vec::new();

        // Object graph diff (sheets, tables, named ranges)
        ops.extend(diff_object_graph(&self.workbook, &other.workbook, config));

        // Grid diff per sheet
        for (sheet_id, old_sheet, new_sheet) in matched_sheets(&self.workbook, &other.workbook) {
            let sheet_ops = diff_sheet(old_sheet, new_sheet, config);
            ops.extend(sheet_ops);
        }

        // M query diff
        if let (Some(old_dm), Some(new_dm)) = (&self.data_mashup, &other.data_mashup) {
            ops.extend(diff_m_queries(&old_dm.queries, &new_dm.queries, config));
        }

        DiffReport { ops, metadata: ... }
    }

    pub fn diff_streaming<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        // Same logic but emits through sink
    }
}
```

**Deliverables:**
- [ ] Implement `WorkbookPackage::diff`
- [ ] Implement `WorkbookPackage::diff_streaming`
- [ ] Deprecate standalone `diff_workbooks` and `diff_m_queries` (keep as internal)
- [ ] Add projection helpers: `DiffReport::grid_ops()`, `DiffReport::m_ops()`
- [ ] Update all examples and docs to use new API

### Acceptance Criteria for Branch 5

- [ ] Single `WorkbookPackage::open` parses entire file
- [ ] Single `WorkbookPackage::diff` produces unified results
- [ ] M query changes appear in same `DiffOp` stream as grid changes
- [ ] Old APIs deprecated with clear migration path
- [ ] Public API surface reduced

---

## Branch 6: M Parser Expansion

**Goal:** Extend M parser beyond `let ... in` to handle all common top-level expression forms.

**Depends on:** Nothing (independent)

**Evaluation References:** Priority Recommendation #10

### 6.1 Audit Current Parser Coverage

**Deliverables:**
- [ ] Document which M constructs are fully parsed vs. treated as opaque sequences
- [ ] Create test fixtures for each unsupported construct
- [ ] Prioritize by frequency in real-world Power Query usage

### 6.2 Implement Non-Let Top-Level Expressions

**Technical Specification:**

Currently unsupported top-level forms:
- Direct record literals: `[Field1 = 1, Field2 = 2]`
- Direct list literals: `{1, 2, 3}`
- Direct function calls: `Table.FromRows(...)`
- Direct primitive expressions: `"hello"`, `42`

```rust
pub enum MExpression {
    Let { bindings: Vec<MBinding>, body: Box<MExpression> },
    Record { fields: Vec<MField> },
    List { items: Vec<MExpression> },
    FunctionCall { name: String, args: Vec<MExpression> },
    // ... other expression types ...
    Primitive(MPrimitive),
    Opaque(Vec<MToken>), // Fallback for truly unparseable
}
```

**Deliverables:**
- [ ] Extend `MExpression` enum with new variants
- [ ] Implement record literal parsing
- [ ] Implement list literal parsing
- [ ] Implement function call parsing
- [ ] Implement primitive expression parsing
- [ ] Update `canonicalize` to handle new expression types
- [ ] Add tests for each new construct

### 6.3 Update Semantic Comparison

**Deliverables:**
- [ ] Ensure `canonicalize_tokens` is no longer a no-op for non-let expressions
- [ ] Implement canonicalization for records (sort fields by name)
- [ ] Implement canonicalization for lists (preserve order)
- [ ] Add semantic equivalence tests: `[B=2, A=1]` equals `[A=1, B=2]`

### Acceptance Criteria for Branch 6

- [ ] All common M top-level forms parsed to AST
- [ ] `Opaque` fallback used only for genuinely obscure constructs
- [ ] Semantic diff correctly identifies formatting-only changes for all forms
- [ ] No regressions in existing M diff tests

---

## Branch 7: Excel Formula Parser

**Goal:** Add semantic diff capability for Excel formulas.

**Depends on:** Nothing (independent), but integrates with Branch 5's unified DiffOp

**Evaluation References:** Priority Recommendation #11

### 7.1 Implement Formula AST

**Technical Specification:**

```rust
pub enum FormulaExpr {
    Number(f64),
    Text(String),
    Boolean(bool),
    Error(ExcelError),
    CellRef(CellReference),
    RangeRef(RangeReference),
    NamedRef(String),
    FunctionCall { name: String, args: Vec<FormulaExpr> },
    BinaryOp { op: BinaryOperator, left: Box<FormulaExpr>, right: Box<FormulaExpr> },
    UnaryOp { op: UnaryOperator, operand: Box<FormulaExpr> },
    Array { rows: Vec<Vec<FormulaExpr>> },
}

pub struct CellReference {
    pub sheet: Option<String>,
    pub col: ColRef,
    pub row: RowRef,
}

pub enum ColRef {
    Absolute(u32),  // $A
    Relative(u32),  // A
}

pub enum RowRef {
    Absolute(u32),  // $1
    Relative(u32),  // 1
}
```

**Deliverables:**
- [ ] Define `FormulaExpr` AST types
- [ ] Define `CellReference` and `RangeReference` types
- [ ] Implement formula parser (consider using `pest` or `nom`)
- [ ] Handle R1C1 vs A1 notation
- [ ] Handle array formulas
- [ ] Handle structured references (table references)
- [ ] Add comprehensive parser tests

### 7.2 Implement Formula Canonicalization

**Technical Specification:**

```rust
impl FormulaExpr {
    pub fn canonicalize(&self) -> FormulaExpr {
        match self {
            FormulaExpr::FunctionCall { name, args } => {
                let canon_name = name.to_uppercase();
                let canon_args: Vec<_> = args.iter().map(|a| a.canonicalize()).collect();

                // Commutative functions: sort arguments
                if is_commutative(&canon_name) {
                    let mut sorted = canon_args;
                    sorted.sort_by_key(|a| a.canonical_hash());
                    return FormulaExpr::FunctionCall { name: canon_name, args: sorted };
                }

                FormulaExpr::FunctionCall { name: canon_name, args: canon_args }
            }
            FormulaExpr::BinaryOp { op, left, right } if op.is_commutative() => {
                let l = left.canonicalize();
                let r = right.canonicalize();
                if l.canonical_hash() > r.canonical_hash() {
                    FormulaExpr::BinaryOp { op: *op, left: Box::new(r), right: Box::new(l) }
                } else {
                    FormulaExpr::BinaryOp { op: *op, left: Box::new(l), right: Box::new(r) }
                }
            }
            // ... other cases ...
        }
    }
}

fn is_commutative(func: &str) -> bool {
    matches!(func, "SUM" | "PRODUCT" | "AND" | "OR" | "MAX" | "MIN" | ...)
}
```

**Deliverables:**
- [ ] Implement `canonicalize` for all expression types
- [ ] Identify and handle commutative functions
- [ ] Identify and handle commutative operators (+, *, AND, OR)
- [ ] Case-normalize function names
- [ ] Add canonicalization tests

### 7.3 Implement Reference Shift Detection

**Technical Specification:**

When a formula is copied/filled, references shift. Detect when two formulas are "the same modulo shift":

```rust
pub fn formulas_equivalent_modulo_shift(
    f1: &FormulaExpr,
    f2: &FormulaExpr,
    row_shift: i32,
    col_shift: i32,
) -> bool {
    match (f1, f2) {
        (FormulaExpr::CellRef(r1), FormulaExpr::CellRef(r2)) => {
            refs_match_with_shift(r1, r2, row_shift, col_shift)
        }
        (FormulaExpr::FunctionCall { name: n1, args: a1 },
         FormulaExpr::FunctionCall { name: n2, args: a2 }) => {
            n1 == n2 && a1.len() == a2.len() &&
            a1.iter().zip(a2.iter()).all(|(x, y)|
                formulas_equivalent_modulo_shift(x, y, row_shift, col_shift))
        }
        // ... other cases ...
    }
}
```

**Deliverables:**
- [ ] Implement `formulas_equivalent_modulo_shift`
- [ ] Integrate with cell diff to detect "formula filled" vs "formula changed"
- [ ] Add test: `=A1+B1` in C1 vs `=A2+B2` in C2 → equivalent (row shift)
- [ ] Add test: `=A1+B1` vs `=A1+B2` → not equivalent

### 7.4 Integrate with Cell Diff

**Technical Specification:**

```rust
pub fn diff_cell_formulas(
    old: &Option<String>,
    new: &Option<String>,
    config: &DiffConfig,
) -> FormulaDiffResult {
    if !config.enable_formula_semantic_diff {
        return FormulaDiffResult::from_text_comparison(old, new);
    }

    match (old, new) {
        (None, None) => FormulaDiffResult::Unchanged,
        (None, Some(_)) => FormulaDiffResult::Added,
        (Some(_), None) => FormulaDiffResult::Removed,
        (Some(old_str), Some(new_str)) => {
            if old_str == new_str {
                return FormulaDiffResult::Unchanged;
            }

            let old_ast = parse_formula(old_str).ok();
            let new_ast = parse_formula(new_str).ok();

            match (old_ast, new_ast) {
                (Some(o), Some(n)) => {
                    if o.canonicalize() == n.canonicalize() {
                        FormulaDiffResult::FormattingOnly
                    } else {
                        FormulaDiffResult::SemanticChange
                    }
                }
                _ => FormulaDiffResult::TextChange, // Parse failed, fall back
            }
        }
    }
}
```

**Deliverables:**
- [ ] Implement `diff_cell_formulas`
- [ ] Update `CellEdited` DiffOp to include formula diff classification
- [ ] Add `DiffConfig::enable_formula_semantic_diff` flag
- [ ] Add integration tests with real formula patterns

### Acceptance Criteria for Branch 7

- [ ] Formula parser handles common Excel formula syntax
- [ ] Canonicalization normalizes commutative operations
- [ ] Reference shift detection works for fill patterns
- [ ] Semantic formula diff integrated with cell diff
- [ ] Feature flag allows disabling for performance

---

## Execution Order

### Phase 1: Foundation (Weeks 1-2)
- **Branch 3: Configuration Infrastructure** (1 week)
  - Unblocks all other work
  - Small, well-defined scope

### Phase 2: Correctness (Weeks 2-4)
- **Branch 1: Grid Algorithm Correctness** (2-3 weeks)
  - Critical bug fixes
  - Must complete before scalability work

### Phase 3: Parallel Work (Weeks 4-8)
Run in parallel based on team capacity:

- **Branch 2: Grid Scalability + Perf Infra** (3-4 weeks)
  - Depends on Branch 1
  - Largest single branch

- **Branch 4: WASM & Memory** (2-3 weeks)
  - Can start after Branch 3
  - Memory work helps Branch 2 validation

- **Branch 5: API Unification** (1-2 weeks)
  - Independent
  - Good for parallel work

- **Branch 6: M Parser** (2 weeks)
  - Fully independent
  - Good for parallel work

### Phase 4: Enhancement (Weeks 8-10)
- **Branch 7: Formula Parser** (2-3 weeks)
  - Independent but lower priority
  - Can be deferred if needed

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| AMR complexity exceeds estimate | Start with phased refactor (2.1) to create seams; implement incrementally |
| WASM size budget exceeded | Feature-gate heavy dependencies; measure early and often |
| String interning breaks existing code | Comprehensive test coverage before starting; incremental migration |
| Formula parser scope creep | Define MVP formula subset; expand iteratively |
| Performance regressions | Establish baselines in Branch 2 perf infra before other changes |

---

## Definition of Done (All Branches)

- [ ] All new code has unit tests
- [ ] All public APIs documented
- [ ] No new clippy warnings
- [ ] CI passes (including new WASM and perf gates)
- [ ] Code reviewed by at least one other engineer
- [ ] Integration tests cover happy path and key error cases
- [ ] Memory and performance within budgets
- [ ] Backward compatibility maintained or migration path documented

