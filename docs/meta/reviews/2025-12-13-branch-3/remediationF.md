## #2 Document the new move-detection gates

### `core/src/config.rs`

#### 2.1 Document `max_move_iterations`

Replace this:

```rust
pub struct DiffConfig {
    pub max_move_iterations: u32,
    pub max_align_rows: u32,
```

With this:

```rust
pub struct DiffConfig {
    /// Maximum number of masked move-detection iterations per sheet.
    /// Set to 0 to disable move detection and represent moves as insert/delete.
    pub max_move_iterations: u32,
    pub max_align_rows: u32,
```

#### 2.2 Clarify `recursive_align_threshold` is not a move-detection gate

Replace this:

```rust
#[serde(alias = "recursive_threshold")]
pub recursive_align_threshold: u32,
pub small_gap_threshold: u32,
```

With this:

```rust
/// Row-count threshold for recursive gap alignment. Does not gate masked move detection.
#[serde(alias = "recursive_threshold")]
pub recursive_align_threshold: u32,
pub small_gap_threshold: u32,
```

#### 2.3 Document the new masked move-detection size gates

Replace this:

```rust
pub context_anchor_k1: u32,
pub context_anchor_k2: u32,
pub max_move_detection_rows: u32,
pub max_move_detection_cols: u32,
```

With this:

```rust
pub context_anchor_k1: u32,
pub context_anchor_k2: u32,
/// Masked move detection runs only when max(old.nrows, new.nrows) <= this.
pub max_move_detection_rows: u32,
/// Masked move detection runs only when max(old.ncols, new.ncols) <= this.
pub max_move_detection_cols: u32,
```


### `docs/rust_docs/next_sprint_plan.md`

#### 2.4 Add gates to the Branch 1.2 “Key considerations” bullets

Replace this:

```md
- Each iteration must make progress (move at least one cell from active to accounted)
- Iteration cap as a safety valve (configurable via `DiffConfig::max_move_iterations`)
- Moves should be detected in order of "confidence" or "size" to avoid fragmentation
```

With this:

```md
- Each iteration must make progress (move at least one cell from active to accounted)
- Iteration cap as a safety valve (configurable via `DiffConfig::max_move_iterations`)
- Skip masked move detection on very large sheets (configurable via `DiffConfig::max_move_detection_rows` / `DiffConfig::max_move_detection_cols`)
- Moves should be detected in order of "confidence" or "size" to avoid fragmentation
```

#### 2.5 Add gates to the Branch 2.5 “Operational limits” snippet

Replace this:

```rust
// Operational limits
pub max_move_iterations: u32,   // Default: 20
pub max_recursion_depth: u32,   // Default: 10
```

With this:

```rust
// Operational limits
pub max_move_iterations: u32,       // Default: 20
pub max_move_detection_rows: u32,   // Default: 200
pub max_move_detection_cols: u32,   // Default: 256
pub max_recursion_depth: u32,       // Default: 10
```

#### 2.6 Add gates to the Branch 3.1 DiffConfig struct

Replace this:

```rust
// Move detection
pub fuzzy_similarity_threshold: f64,
pub min_block_size_for_move: u32,
pub max_move_iterations: u32,

// Safety limits
pub max_align_rows: u32,
```

With this:

```rust
// Move detection
pub fuzzy_similarity_threshold: f64,
pub min_block_size_for_move: u32,
pub max_move_iterations: u32,

// Masked move-detection gates (independent of recursive_align_threshold)
pub max_move_detection_rows: u32,
pub max_move_detection_cols: u32,

// Safety limits
pub max_align_rows: u32,
```


## #3 Optional: make phase timing labels match what’s actually measured

### `scripts/visualize_benchmarks.py`

Replace this:

```python
metric_labels = {
    "move_detection_time_ms": "Move Detection",
    "alignment_time_ms": "Alignment",
    "cell_diff_time_ms": "Cell Diff",
}
```

With this:

```python
metric_labels = {
    "move_detection_time_ms": "Fingerprinting + Move Detection",
    "alignment_time_ms": "Alignment (incl. diff)",
    "cell_diff_time_ms": "Cell Diff",
}
```


### `docs/rust_docs/excel_diff_specification.md`

Replace this:

```rust
pub struct DiffMetrics {
    pub alignment_time_ms: u64,       // Time spent in row/column alignment
    pub move_detection_time_ms: u64,  // Time spent detecting block moves
    pub cell_diff_time_ms: u64,       // Time spent comparing cells
    pub total_time_ms: u64,           // Total diff operation time
    pub rows_processed: u64,          // Number of rows examined
    pub cells_compared: u64,          // Number of cell pairs compared
    pub anchors_found: u32,           // Anchor count from AMR alignment
    pub moves_detected: u32,          // Block moves detected
}
```

With this:

```rust
pub struct DiffMetrics {
    pub alignment_time_ms: u64,       // Time spent in alignment stage (may include nested cell diff time)
    pub move_detection_time_ms: u64,  // Time spent in fingerprinting + masked move detection
    pub cell_diff_time_ms: u64,       // Time spent emitting cell diffs
    pub total_time_ms: u64,           // Total diff operation time
    pub rows_processed: u64,          // Number of rows examined
    pub cells_compared: u64,          // Number of cell pairs compared
    pub anchors_found: u32,           // Anchor count from AMR alignment
    pub moves_detected: u32,          // Block moves detected
}
```


### `docs/rust_docs/next_sprint_plan.md` (Perf infrastructure section)

Replace this:

```rust
#[cfg(feature = "perf-metrics")]
pub struct DiffMetrics {
    pub alignment_time_ms: u64,
    pub move_detection_time_ms: u64,
    pub cell_diff_time_ms: u64,
    pub total_time_ms: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
}
```

With this:

```rust
#[cfg(feature = "perf-metrics")]
pub struct DiffMetrics {
    pub alignment_time_ms: u64,       // Alignment stage (may include nested cell diff time)
    pub move_detection_time_ms: u64,  // Fingerprinting + masked move detection
    pub cell_diff_time_ms: u64,       // Emitting cell diffs
    pub total_time_ms: u64,
    pub rows_processed: u64,
    pub cells_compared: u64,
    pub anchors_found: u32,
    pub moves_detected: u32,
}
```