
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