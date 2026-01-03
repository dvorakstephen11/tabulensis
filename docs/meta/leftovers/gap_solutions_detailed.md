# Detailed Solutions for Algorithm Design Gaps

**Date:** 2025-11-30  
**Status:** Approved Design Extension  
**Context:** Solutions for gaps identified in the Grid Diff Algorithm Synthesis

---

## Table of Contents

1. [Formula Semantics](#1-formula-semantics)
2. [Memory Budget and Streaming Strategy](#2-memory-budget-and-streaming-strategy)
3. [Key Inference Algorithm](#3-key-inference-algorithm)
4. [Fuzzy Match Threshold Calibration](#4-fuzzy-match-threshold-calibration)
5. [Deterministic Output Guarantees](#5-deterministic-output-guarantees)
6. [Rectangular Block Move Detection](#6-rectangular-block-move-detection)
7. [String Interning System](#7-string-interning-system)

---

## 1. Formula Semantics

### 1.1 Problem Statement

Excel formulas are semantically rich structures that string comparison handles poorly:

| Scenario | String Diff Says | Semantic Reality |
|----------|-----------------|------------------|
| `=A1+B1` → `=B1+A1` | Changed | Equivalent (commutative) |
| `=SUM(A1:A10)` → `=SUM(A1:A11)` | Changed | Range extended by 1 |
| `=A1` → `=A2` | Changed | Reference shifted (possibly due to row insert) |
| `=VLOOKUP(A1,B:C,2)` → `=XLOOKUP(A1,B:B,C:C)` | Changed | Modernized function (equivalent) |

### 1.2 Formula Representation

#### 1.2.1 Abstract Syntax Tree (AST)

```rust
pub enum FormulaNode {
    Number(f64),
    String(String),
    Boolean(bool),
    Error(ErrorType),
    
    CellRef(CellReference),
    RangeRef(RangeReference),
    NamedRef(String),
    
    UnaryOp {
        op: UnaryOperator,
        operand: Box<FormulaNode>,
    },
    BinaryOp {
        op: BinaryOperator,
        left: Box<FormulaNode>,
        right: Box<FormulaNode>,
    },
    FunctionCall {
        name: String,
        args: Vec<FormulaNode>,
    },
    Array {
        rows: Vec<Vec<FormulaNode>>,
    },
}

pub struct CellReference {
    pub sheet: Option<String>,
    pub col: ColRef,
    pub row: RowRef,
}

pub enum ColRef {
    Absolute(u32),
    Relative(i32),
}

pub enum RowRef {
    Absolute(u32),
    Relative(i32),
}

pub struct RangeReference {
    pub start: CellReference,
    pub end: CellReference,
}
```

#### 1.2.2 Parsing

Use a standard Excel formula parser (e.g., adapt from `calamine` or build using `nom`/`pest`):

```rust
pub fn parse_formula(text: &str) -> Result<FormulaNode, ParseError> {
    let tokens = tokenize(text)?;
    let ast = parse_expression(&tokens)?;
    Ok(ast)
}

fn tokenize(text: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    
    while let Some(&c) = chars.peek() {
        match c {
            '=' if tokens.is_empty() => { chars.next(); }
            '0'..='9' | '.' => tokens.push(scan_number(&mut chars)?),
            '"' => tokens.push(scan_string(&mut chars)?),
            'A'..='Z' | 'a'..='z' | '_' | '$' => tokens.push(scan_identifier(&mut chars)?),
            '+' | '-' | '*' | '/' | '^' | '&' | '=' | '<' | '>' => {
                tokens.push(scan_operator(&mut chars)?);
            }
            '(' | ')' | ',' | ':' | '!' | '{' | '}' | ';' => {
                tokens.push(Token::Punctuation(c));
                chars.next();
            }
            ' ' | '\t' => { chars.next(); }
            _ => return Err(ParseError::UnexpectedChar(c)),
        }
    }
    
    Ok(tokens)
}
```

### 1.3 Formula Canonicalization

Before comparison, normalize formulas to a canonical form:

```rust
pub fn canonicalize(node: &FormulaNode) -> FormulaNode {
    match node {
        FormulaNode::BinaryOp { op, left, right } => {
            let left_canon = canonicalize(left);
            let right_canon = canonicalize(right);
            
            if op.is_commutative() {
                let (ordered_left, ordered_right) = order_operands(&left_canon, &right_canon);
                FormulaNode::BinaryOp {
                    op: *op,
                    left: Box::new(ordered_left),
                    right: Box::new(ordered_right),
                }
            } else {
                FormulaNode::BinaryOp {
                    op: *op,
                    left: Box::new(left_canon),
                    right: Box::new(right_canon),
                }
            }
        }
        FormulaNode::FunctionCall { name, args } => {
            let canon_args: Vec<_> = args.iter().map(canonicalize).collect();
            let normalized_name = normalize_function_name(name);
            
            if is_commutative_function(&normalized_name) {
                let mut sorted_args = canon_args;
                sorted_args.sort_by(|a, b| node_sort_key(a).cmp(&node_sort_key(b)));
                FormulaNode::FunctionCall {
                    name: normalized_name,
                    args: sorted_args,
                }
            } else {
                FormulaNode::FunctionCall {
                    name: normalized_name,
                    args: canon_args,
                }
            }
        }
        _ => node.clone(),
    }
}

fn order_operands(left: &FormulaNode, right: &FormulaNode) -> (FormulaNode, FormulaNode) {
    if node_sort_key(left) <= node_sort_key(right) {
        (left.clone(), right.clone())
    } else {
        (right.clone(), left.clone())
    }
}

fn node_sort_key(node: &FormulaNode) -> String {
    match node {
        FormulaNode::Number(n) => format!("0:{}", n),
        FormulaNode::String(s) => format!("1:{}", s),
        FormulaNode::CellRef(r) => format!("2:{}:{}", r.col, r.row),
        FormulaNode::FunctionCall { name, .. } => format!("3:{}", name),
        _ => format!("9:{:?}", node),
    }
}

impl BinaryOperator {
    fn is_commutative(&self) -> bool {
        matches!(self, 
            BinaryOperator::Add | 
            BinaryOperator::Multiply | 
            BinaryOperator::Equal |
            BinaryOperator::NotEqual
        )
    }
}

fn is_commutative_function(name: &str) -> bool {
    matches!(name.to_uppercase().as_str(),
        "SUM" | "PRODUCT" | "MAX" | "MIN" | "AVERAGE" | 
        "AND" | "OR" | "GCD" | "LCM"
    )
}
```

### 1.4 Formula Equality

After canonicalization, compare ASTs for structural equality:

```rust
pub fn formulas_equal(a: &FormulaNode, b: &FormulaNode) -> bool {
    let canon_a = canonicalize(a);
    let canon_b = canonicalize(b);
    structural_equal(&canon_a, &canon_b)
}

fn structural_equal(a: &FormulaNode, b: &FormulaNode) -> bool {
    match (a, b) {
        (FormulaNode::Number(x), FormulaNode::Number(y)) => (x - y).abs() < 1e-10,
        (FormulaNode::String(x), FormulaNode::String(y)) => x == y,
        (FormulaNode::Boolean(x), FormulaNode::Boolean(y)) => x == y,
        (FormulaNode::CellRef(x), FormulaNode::CellRef(y)) => refs_equal(x, y),
        (FormulaNode::RangeRef(x), FormulaNode::RangeRef(y)) => {
            refs_equal(&x.start, &y.start) && refs_equal(&x.end, &y.end)
        }
        (FormulaNode::BinaryOp { op: op_a, left: l_a, right: r_a },
         FormulaNode::BinaryOp { op: op_b, left: l_b, right: r_b }) => {
            op_a == op_b && structural_equal(l_a, l_b) && structural_equal(r_a, r_b)
        }
        (FormulaNode::FunctionCall { name: n_a, args: a_a },
         FormulaNode::FunctionCall { name: n_b, args: a_b }) => {
            n_a.eq_ignore_ascii_case(n_b) && 
            a_a.len() == a_b.len() &&
            a_a.iter().zip(a_b.iter()).all(|(x, y)| structural_equal(x, y))
        }
        _ => false,
    }
}
```

### 1.5 Formula Diff: Tree Edit Distance

When formulas differ, compute the minimal edit to transform one into the other:

```rust
pub struct FormulaDiff {
    pub edit_distance: usize,
    pub operations: Vec<FormulaEditOp>,
}

pub enum FormulaEditOp {
    Keep { node: FormulaNode },
    Replace { old: FormulaNode, new: FormulaNode },
    Insert { node: FormulaNode, position: TreePosition },
    Delete { node: FormulaNode },
    UpdateRef { old_ref: CellReference, new_ref: CellReference },
    UpdateRange { old_range: RangeReference, new_range: RangeReference },
}

pub fn diff_formulas(old: &FormulaNode, new: &FormulaNode) -> FormulaDiff {
    let canon_old = canonicalize(old);
    let canon_new = canonicalize(new);
    
    compute_tree_edit_distance(&canon_old, &canon_new)
}

fn compute_tree_edit_distance(a: &FormulaNode, b: &FormulaNode) -> FormulaDiff {
    if structural_equal(a, b) {
        return FormulaDiff {
            edit_distance: 0,
            operations: vec![FormulaEditOp::Keep { node: a.clone() }],
        };
    }
    
    match (a, b) {
        (FormulaNode::CellRef(ref_a), FormulaNode::CellRef(ref_b)) => {
            FormulaDiff {
                edit_distance: 1,
                operations: vec![FormulaEditOp::UpdateRef {
                    old_ref: ref_a.clone(),
                    new_ref: ref_b.clone(),
                }],
            }
        }
        (FormulaNode::RangeRef(range_a), FormulaNode::RangeRef(range_b)) => {
            FormulaDiff {
                edit_distance: 1,
                operations: vec![FormulaEditOp::UpdateRange {
                    old_range: range_a.clone(),
                    new_range: range_b.clone(),
                }],
            }
        }
        (FormulaNode::FunctionCall { name: n_a, args: args_a },
         FormulaNode::FunctionCall { name: n_b, args: args_b }) 
            if n_a.eq_ignore_ascii_case(n_b) => {
            diff_function_args(n_a, args_a, args_b)
        }
        _ => {
            FormulaDiff {
                edit_distance: tree_size(a) + tree_size(b),
                operations: vec![FormulaEditOp::Replace {
                    old: a.clone(),
                    new: b.clone(),
                }],
            }
        }
    }
}
```

### 1.6 Reference Shift Detection

Detect when references shifted due to row/column insertions:

```rust
pub struct ReferenceShift {
    pub row_delta: i32,
    pub col_delta: i32,
}

pub fn detect_reference_shift(
    formula_a: &FormulaNode,
    formula_b: &FormulaNode,
    row_alignment: &[(u32, u32)],
    col_alignment: &[(u32, u32)],
) -> Option<ReferenceShift> {
    let refs_a = extract_cell_refs(formula_a);
    let refs_b = extract_cell_refs(formula_b);
    
    if refs_a.len() != refs_b.len() {
        return None;
    }
    
    let mut deltas: Vec<(i32, i32)> = Vec::new();
    
    for (ref_a, ref_b) in refs_a.iter().zip(refs_b.iter()) {
        let row_delta = compute_row_delta(ref_a, ref_b, row_alignment);
        let col_delta = compute_col_delta(ref_a, ref_b, col_alignment);
        deltas.push((row_delta, col_delta));
    }
    
    if deltas.iter().all(|d| *d == deltas[0]) {
        Some(ReferenceShift {
            row_delta: deltas[0].0,
            col_delta: deltas[0].1,
        })
    } else {
        None
    }
}
```

### 1.7 Integration with Cell Diff

```rust
pub fn diff_cell_with_formula(
    cell_a: &Cell,
    cell_b: &Cell,
) -> CellDiffResult {
    match (&cell_a.formula, &cell_b.formula) {
        (Some(f_a), Some(f_b)) => {
            let ast_a = parse_formula(f_a).ok();
            let ast_b = parse_formula(f_b).ok();
            
            match (ast_a, ast_b) {
                (Some(a), Some(b)) => {
                    if formulas_equal(&a, &b) {
                        if values_equal(&cell_a.value, &cell_b.value) {
                            CellDiffResult::Unchanged
                        } else {
                            CellDiffResult::ValueChanged {
                                old: cell_a.value.clone(),
                                new: cell_b.value.clone(),
                                formula_unchanged: true,
                            }
                        }
                    } else {
                        let formula_diff = diff_formulas(&a, &b);
                        CellDiffResult::FormulaChanged {
                            old_formula: f_a.clone(),
                            new_formula: f_b.clone(),
                            edit_distance: formula_diff.edit_distance,
                            operations: formula_diff.operations,
                        }
                    }
                }
                _ => {
                    CellDiffResult::FormulaChanged {
                        old_formula: f_a.clone(),
                        new_formula: f_b.clone(),
                        edit_distance: levenshtein(f_a, f_b),
                        operations: vec![],
                    }
                }
            }
        }
        (None, Some(f_b)) => CellDiffResult::FormulaAdded { formula: f_b.clone() },
        (Some(f_a), None) => CellDiffResult::FormulaRemoved { formula: f_a.clone() },
        (None, None) => {
            if values_equal(&cell_a.value, &cell_b.value) {
                CellDiffResult::Unchanged
            } else {
                CellDiffResult::ValueChanged {
                    old: cell_a.value.clone(),
                    new: cell_b.value.clone(),
                    formula_unchanged: false,
                }
            }
        }
    }
}
```

---

## 2. Memory Budget and Streaming Strategy

### 2.1 Memory Model

#### 2.1.1 WASM Constraints

| Constraint | Typical Limit | Conservative Target |
|------------|---------------|---------------------|
| WASM linear memory | 4GB max | 2GB practical |
| Browser tab memory | 4-8GB | 2GB recommended |
| Mobile browser | 1-2GB | 512MB target |

#### 2.1.2 Excel File Expansion

```
100MB .xlsx file:
├── Compressed XML: 100MB on disk
├── Decompressed XML: ~400-600MB
├── Parsed cell structures: ~200-400MB
├── Diff working memory: ~100-200MB
└── Peak total: ~800MB - 1.2GB
```

### 2.2 Memory Budget Specification

```rust
pub struct MemoryBudget {
    pub max_total_bytes: usize,
    pub max_per_grid_bytes: usize,
    pub max_working_set_bytes: usize,
    pub streaming_threshold_bytes: usize,
}

impl MemoryBudget {
    pub fn default_wasm() -> Self {
        MemoryBudget {
            max_total_bytes: 1_500_000_000,        // 1.5GB
            max_per_grid_bytes: 600_000_000,       // 600MB per grid
            max_working_set_bytes: 300_000_000,    // 300MB for diff operations
            streaming_threshold_bytes: 50_000_000, // Stream if > 50MB
        }
    }
    
    pub fn conservative_mobile() -> Self {
        MemoryBudget {
            max_total_bytes: 400_000_000,
            max_per_grid_bytes: 150_000_000,
            max_working_set_bytes: 100_000_000,
            streaming_threshold_bytes: 20_000_000,
        }
    }
}
```

### 2.3 Memory Tracking

```rust
pub struct MemoryTracker {
    allocated: AtomicUsize,
    peak: AtomicUsize,
    budget: MemoryBudget,
}

impl MemoryTracker {
    pub fn allocate(&self, bytes: usize) -> Result<MemoryGuard, MemoryError> {
        let current = self.allocated.fetch_add(bytes, Ordering::SeqCst);
        let new_total = current + bytes;
        
        self.peak.fetch_max(new_total, Ordering::SeqCst);
        
        if new_total > self.budget.max_total_bytes {
            self.allocated.fetch_sub(bytes, Ordering::SeqCst);
            return Err(MemoryError::BudgetExceeded {
                requested: bytes,
                available: self.budget.max_total_bytes.saturating_sub(current),
            });
        }
        
        Ok(MemoryGuard {
            tracker: self,
            bytes,
        })
    }
    
    pub fn current_usage(&self) -> usize {
        self.allocated.load(Ordering::SeqCst)
    }
    
    pub fn peak_usage(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }
}

pub struct MemoryGuard<'a> {
    tracker: &'a MemoryTracker,
    bytes: usize,
}

impl Drop for MemoryGuard<'_> {
    fn drop(&mut self) {
        self.tracker.allocated.fetch_sub(self.bytes, Ordering::SeqCst);
    }
}
```

### 2.4 Streaming Parser Architecture

For large files, parse incrementally without loading everything into memory:

```rust
pub trait StreamingGridParser {
    fn estimate_size(&mut self) -> Result<GridSizeEstimate, ParseError>;
    fn parse_row(&mut self) -> Result<Option<StreamedRow>, ParseError>;
    fn skip_to_row(&mut self, row_idx: u32) -> Result<(), ParseError>;
}

pub struct GridSizeEstimate {
    pub row_count: u32,
    pub col_count: u32,
    pub non_empty_cell_estimate: u64,
    pub estimated_memory_bytes: u64,
}

pub struct StreamedRow {
    pub row_idx: u32,
    pub cells: Vec<(u32, Cell)>,
    pub hash: RowHash,
}

pub struct StreamingXlsxParser {
    archive: ZipArchive<File>,
    sheet_xml: XmlReader,
    current_row: u32,
    row_buffer: Vec<u8>,
}

impl StreamingGridParser for StreamingXlsxParser {
    fn parse_row(&mut self) -> Result<Option<StreamedRow>, ParseError> {
        loop {
            match self.sheet_xml.read_event_into(&mut self.row_buffer)? {
                Event::Start(e) if e.name().as_ref() == b"row" => {
                    let row_idx = extract_row_index(&e)?;
                    let cells = self.parse_row_cells()?;
                    let hash = compute_row_hash(&cells);
                    
                    return Ok(Some(StreamedRow { row_idx, cells, hash }));
                }
                Event::Eof => return Ok(None),
                _ => continue,
            }
        }
    }
}
```

### 2.5 Chunked Diff Strategy

For very large grids, process in chunks:

```rust
pub struct ChunkedDiffConfig {
    pub chunk_size_rows: u32,
    pub overlap_rows: u32,
}

impl Default for ChunkedDiffConfig {
    fn default() -> Self {
        ChunkedDiffConfig {
            chunk_size_rows: 10_000,
            overlap_rows: 100,
        }
    }
}

pub fn diff_chunked<P: StreamingGridParser>(
    parser_a: &mut P,
    parser_b: &mut P,
    config: &ChunkedDiffConfig,
    memory: &MemoryTracker,
) -> Result<DiffResult, DiffError> {
    let estimate_a = parser_a.estimate_size()?;
    let estimate_b = parser_b.estimate_size()?;
    
    if should_use_streaming(&estimate_a, &estimate_b, memory) {
        diff_streaming(parser_a, parser_b, config, memory)
    } else {
        let grid_a = load_full_grid(parser_a, memory)?;
        let grid_b = load_full_grid(parser_b, memory)?;
        diff_in_memory(&grid_a, &grid_b)
    }
}

fn should_use_streaming(
    est_a: &GridSizeEstimate,
    est_b: &GridSizeEstimate,
    memory: &MemoryTracker,
) -> bool {
    let total_estimate = est_a.estimated_memory_bytes + est_b.estimated_memory_bytes;
    let available = memory.budget.max_total_bytes - memory.current_usage();
    
    total_estimate > available as u64 * 80 / 100
}

fn diff_streaming<P: StreamingGridParser>(
    parser_a: &mut P,
    parser_b: &mut P,
    config: &ChunkedDiffConfig,
    memory: &MemoryTracker,
) -> Result<DiffResult, DiffError> {
    let mut all_ops = Vec::new();
    let mut chunk_start = 0u32;
    
    loop {
        let chunk_a = load_chunk(parser_a, chunk_start, config.chunk_size_rows, memory)?;
        let chunk_b = load_chunk(parser_b, chunk_start, config.chunk_size_rows, memory)?;
        
        if chunk_a.is_empty() && chunk_b.is_empty() {
            break;
        }
        
        let chunk_ops = diff_chunk(&chunk_a, &chunk_b, chunk_start)?;
        all_ops.extend(chunk_ops);
        
        chunk_start += config.chunk_size_rows - config.overlap_rows;
        
        drop(chunk_a);
        drop(chunk_b);
    }
    
    Ok(DiffResult { ops: deduplicate_ops(all_ops) })
}
```

### 2.6 Memory-Efficient Data Structures

```rust
pub struct CompactCell {
    pub value_id: u32,
    pub formula_id: Option<u32>,
}

pub struct CompactGrid {
    pub cells: HashMap<(u32, u32), CompactCell>,
    pub string_pool: StringPool,
    pub nrows: u32,
    pub ncols: u32,
}

impl CompactGrid {
    pub fn memory_usage(&self) -> usize {
        let cell_overhead = self.cells.len() * (8 + 8 + std::mem::size_of::<CompactCell>());
        let string_pool_size = self.string_pool.total_bytes();
        cell_overhead + string_pool_size
    }
}
```

---

## 3. Key Inference Algorithm

### 3.1 Problem Definition

Given a table region, automatically identify which column(s) serve as the primary key for row matching in database mode.

### 3.2 Column Scoring Function

```rust
pub struct ColumnKeyScore {
    pub col_idx: u32,
    pub uniqueness: f64,
    pub coverage: f64,
    pub stability: f64,
    pub data_type_score: f64,
    pub name_hint_score: f64,
    pub composite_score: f64,
}

pub fn score_column_as_key(
    col_idx: u32,
    view_a: &GridView,
    view_b: &GridView,
    header: Option<&str>,
) -> ColumnKeyScore {
    let values_a = extract_column_values(col_idx, view_a);
    let values_b = extract_column_values(col_idx, view_b);
    
    let uniqueness = compute_uniqueness(&values_a, &values_b);
    let coverage = compute_coverage(&values_a, &values_b);
    let stability = compute_stability(&values_a, &values_b);
    let data_type_score = score_data_type(&values_a);
    let name_hint_score = score_header_name(header);
    
    let composite_score = 
        uniqueness * 0.35 +
        coverage * 0.20 +
        stability * 0.20 +
        data_type_score * 0.15 +
        name_hint_score * 0.10;
    
    ColumnKeyScore {
        col_idx,
        uniqueness,
        coverage,
        stability,
        data_type_score,
        name_hint_score,
        composite_score,
    }
}
```

### 3.3 Uniqueness Calculation

```rust
fn compute_uniqueness(values_a: &[CellValue], values_b: &[CellValue]) -> f64 {
    let total_a = values_a.len();
    let total_b = values_b.len();
    
    if total_a == 0 && total_b == 0 {
        return 0.0;
    }
    
    let unique_a = values_a.iter().collect::<HashSet<_>>().len();
    let unique_b = values_b.iter().collect::<HashSet<_>>().len();
    
    let ratio_a = if total_a > 0 { unique_a as f64 / total_a as f64 } else { 0.0 };
    let ratio_b = if total_b > 0 { unique_b as f64 / total_b as f64 } else { 0.0 };
    
    (ratio_a + ratio_b) / 2.0
}
```

### 3.4 Coverage Calculation

Coverage measures how many rows have non-empty values in this column:

```rust
fn compute_coverage(values_a: &[CellValue], values_b: &[CellValue]) -> f64 {
    let non_empty_a = values_a.iter().filter(|v| !v.is_empty()).count();
    let non_empty_b = values_b.iter().filter(|v| !v.is_empty()).count();
    
    let total_a = values_a.len();
    let total_b = values_b.len();
    
    if total_a + total_b == 0 {
        return 0.0;
    }
    
    (non_empty_a + non_empty_b) as f64 / (total_a + total_b) as f64
}
```

### 3.5 Stability Calculation

Stability measures whether values in this column are preserved across versions:

```rust
fn compute_stability(values_a: &[CellValue], values_b: &[CellValue]) -> f64 {
    let set_a: HashSet<_> = values_a.iter().filter(|v| !v.is_empty()).collect();
    let set_b: HashSet<_> = values_b.iter().filter(|v| !v.is_empty()).collect();
    
    if set_a.is_empty() && set_b.is_empty() {
        return 0.0;
    }
    
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    
    intersection as f64 / union as f64
}
```

### 3.6 Data Type Scoring

ID-like data types are better key candidates:

```rust
fn score_data_type(values: &[CellValue]) -> f64 {
    let non_empty: Vec<_> = values.iter().filter(|v| !v.is_empty()).collect();
    
    if non_empty.is_empty() {
        return 0.0;
    }
    
    let mut scores = Vec::new();
    
    for value in &non_empty {
        let score = match value {
            CellValue::Number(n) if is_integer(*n) && *n > 0.0 => 0.9,
            CellValue::String(s) if looks_like_id(s) => 1.0,
            CellValue::String(s) if looks_like_guid(s) => 1.0,
            CellValue::String(s) if looks_like_code(s) => 0.85,
            CellValue::Number(_) => 0.5,
            CellValue::String(s) if s.len() < 50 => 0.6,
            CellValue::String(_) => 0.3,
            _ => 0.1,
        };
        scores.push(score);
    }
    
    scores.iter().sum::<f64>() / scores.len() as f64
}

fn looks_like_id(s: &str) -> bool {
    let patterns = [
        r"^[A-Z]{2,4}-\d{4,}$",
        r"^\d{6,}$",
        r"^[A-Z]+\d+$",
        r"^ID[-_]?\d+$",
    ];
    patterns.iter().any(|p| Regex::new(p).unwrap().is_match(s))
}

fn looks_like_guid(s: &str) -> bool {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
        .unwrap()
        .is_match(s)
}

fn looks_like_code(s: &str) -> bool {
    s.len() >= 3 && s.len() <= 20 && 
    s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}
```

### 3.7 Header Name Scoring

```rust
fn score_header_name(header: Option<&str>) -> f64 {
    let header = match header {
        Some(h) => h.to_lowercase(),
        None => return 0.0,
    };
    
    let strong_indicators = ["id", "key", "code", "sku", "ean", "upc", "isbn"];
    let moderate_indicators = ["number", "num", "no", "ref", "reference", "identifier"];
    let weak_indicators = ["name", "title"];
    let negative_indicators = ["description", "notes", "comment", "date", "amount", "total"];
    
    if strong_indicators.iter().any(|&ind| header.contains(ind)) {
        return 1.0;
    }
    if moderate_indicators.iter().any(|&ind| header.contains(ind)) {
        return 0.7;
    }
    if weak_indicators.iter().any(|&ind| header == ind) {
        return 0.4;
    }
    if negative_indicators.iter().any(|&ind| header.contains(ind)) {
        return 0.1;
    }
    
    0.3
}
```

### 3.8 Composite Key Detection

```rust
pub struct KeyCandidate {
    pub columns: Vec<u32>,
    pub score: f64,
    pub uniqueness: f64,
}

pub fn find_best_key(
    view_a: &GridView,
    view_b: &GridView,
    headers: &[Option<String>],
    max_key_columns: usize,
) -> Option<KeyCandidate> {
    let single_scores: Vec<_> = (0..view_a.ncols())
        .map(|col| score_column_as_key(col, view_a, view_b, headers.get(col as usize).and_then(|h| h.as_deref())))
        .collect();
    
    let mut candidates = Vec::new();
    
    for score in &single_scores {
        if score.uniqueness >= 0.95 && score.coverage >= 0.90 {
            candidates.push(KeyCandidate {
                columns: vec![score.col_idx],
                score: score.composite_score,
                uniqueness: score.uniqueness,
            });
        }
    }
    
    if candidates.is_empty() && max_key_columns >= 2 {
        for i in 0..single_scores.len() {
            for j in (i+1)..single_scores.len() {
                if single_scores[i].composite_score < 0.3 || single_scores[j].composite_score < 0.3 {
                    continue;
                }
                
                let composite_uniqueness = compute_composite_uniqueness(
                    &[single_scores[i].col_idx, single_scores[j].col_idx],
                    view_a,
                    view_b,
                );
                
                if composite_uniqueness >= 0.95 {
                    let combined_score = (single_scores[i].composite_score + single_scores[j].composite_score) / 2.0 * 0.9;
                    candidates.push(KeyCandidate {
                        columns: vec![single_scores[i].col_idx, single_scores[j].col_idx],
                        score: combined_score,
                        uniqueness: composite_uniqueness,
                    });
                }
            }
        }
    }
    
    candidates.into_iter()
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
}

fn compute_composite_uniqueness(
    columns: &[u32],
    view_a: &GridView,
    view_b: &GridView,
) -> f64 {
    let keys_a: HashSet<Vec<CellValue>> = view_a.rows.iter()
        .map(|row| {
            columns.iter()
                .map(|&col| row.cells.iter().find(|(c, _)| *c == col).map(|(_, cell)| cell.value.clone()).unwrap_or(CellValue::Empty))
                .collect()
        })
        .collect();
    
    let keys_b: HashSet<Vec<CellValue>> = view_b.rows.iter()
        .map(|row| {
            columns.iter()
                .map(|&col| row.cells.iter().find(|(c, _)| *c == col).map(|(_, cell)| cell.value.clone()).unwrap_or(CellValue::Empty))
                .collect()
        })
        .collect();
    
    let unique_a = keys_a.len() as f64 / view_a.rows.len() as f64;
    let unique_b = keys_b.len() as f64 / view_b.rows.len() as f64;
    
    (unique_a + unique_b) / 2.0
}
```

---

## 4. Fuzzy Match Threshold Calibration

### 4.1 Threshold Types

```rust
pub struct FuzzyThresholds {
    pub exact_move_threshold: f64,
    pub fuzzy_move_threshold: f64,
    pub block_similarity_min: f64,
    pub duplicate_cluster_match_max_cost: f64,
}

impl Default for FuzzyThresholds {
    fn default() -> Self {
        FuzzyThresholds {
            exact_move_threshold: 1.0,
            fuzzy_move_threshold: 0.80,
            block_similarity_min: 0.60,
            duplicate_cluster_match_max_cost: 0.50,
        }
    }
}
```

### 4.2 Adaptive Threshold Calculation

Thresholds should adapt to data characteristics:

```rust
pub fn compute_adaptive_thresholds(
    view_a: &GridView,
    view_b: &GridView,
    base: &FuzzyThresholds,
) -> FuzzyThresholds {
    let avg_row_width = compute_avg_row_width(view_a, view_b);
    let data_heterogeneity = compute_data_heterogeneity(view_a, view_b);
    
    let width_factor = if avg_row_width > 20.0 {
        0.95
    } else if avg_row_width > 10.0 {
        1.0
    } else {
        1.05
    };
    
    let heterogeneity_factor = if data_heterogeneity > 0.8 {
        1.05
    } else if data_heterogeneity > 0.5 {
        1.0
    } else {
        0.95
    };
    
    FuzzyThresholds {
        exact_move_threshold: base.exact_move_threshold,
        fuzzy_move_threshold: (base.fuzzy_move_threshold * width_factor * heterogeneity_factor).clamp(0.60, 0.95),
        block_similarity_min: (base.block_similarity_min * width_factor).clamp(0.40, 0.80),
        duplicate_cluster_match_max_cost: base.duplicate_cluster_match_max_cost,
    }
}

fn compute_avg_row_width(view_a: &GridView, view_b: &GridView) -> f64 {
    let total_cells = view_a.row_meta.iter().map(|m| m.non_blank_count as u64).sum::<u64>()
        + view_b.row_meta.iter().map(|m| m.non_blank_count as u64).sum::<u64>();
    let total_rows = (view_a.rows.len() + view_b.rows.len()) as u64;
    
    if total_rows == 0 { 0.0 } else { total_cells as f64 / total_rows as f64 }
}

fn compute_data_heterogeneity(view_a: &GridView, view_b: &GridView) -> f64 {
    let unique_hashes_a = view_a.row_meta.iter().map(|m| m.hash).collect::<HashSet<_>>().len();
    let unique_hashes_b = view_b.row_meta.iter().map(|m| m.hash).collect::<HashSet<_>>().len();
    
    let total_rows = view_a.rows.len() + view_b.rows.len();
    let unique_hashes = unique_hashes_a + unique_hashes_b;
    
    if total_rows == 0 { 0.0 } else { unique_hashes as f64 / total_rows as f64 }
}
```

### 4.3 Similarity Metrics

```rust
pub enum SimilarityMetric {
    Jaccard,
    Dice,
    Overlap,
    WeightedJaccard,
}

pub fn compute_row_similarity(
    row_a: &RowView,
    row_b: &RowView,
    metric: SimilarityMetric,
) -> f64 {
    match metric {
        SimilarityMetric::Jaccard => jaccard_similarity(row_a, row_b),
        SimilarityMetric::Dice => dice_similarity(row_a, row_b),
        SimilarityMetric::Overlap => overlap_similarity(row_a, row_b),
        SimilarityMetric::WeightedJaccard => weighted_jaccard_similarity(row_a, row_b),
    }
}

fn jaccard_similarity(row_a: &RowView, row_b: &RowView) -> f64 {
    let cells_a: HashSet<_> = row_a.cells.iter()
        .map(|(col, cell)| (*col, hash_cell(cell)))
        .collect();
    let cells_b: HashSet<_> = row_b.cells.iter()
        .map(|(col, cell)| (*col, hash_cell(cell)))
        .collect();
    
    let intersection = cells_a.intersection(&cells_b).count();
    let union = cells_a.union(&cells_b).count();
    
    if union == 0 { 1.0 } else { intersection as f64 / union as f64 }
}

fn dice_similarity(row_a: &RowView, row_b: &RowView) -> f64 {
    let cells_a: HashSet<_> = row_a.cells.iter()
        .map(|(col, cell)| (*col, hash_cell(cell)))
        .collect();
    let cells_b: HashSet<_> = row_b.cells.iter()
        .map(|(col, cell)| (*col, hash_cell(cell)))
        .collect();
    
    let intersection = cells_a.intersection(&cells_b).count();
    let total = cells_a.len() + cells_b.len();
    
    if total == 0 { 1.0 } else { 2.0 * intersection as f64 / total as f64 }
}

fn overlap_similarity(row_a: &RowView, row_b: &RowView) -> f64 {
    let cells_a: HashSet<_> = row_a.cells.iter()
        .map(|(col, cell)| (*col, hash_cell(cell)))
        .collect();
    let cells_b: HashSet<_> = row_b.cells.iter()
        .map(|(col, cell)| (*col, hash_cell(cell)))
        .collect();
    
    let intersection = cells_a.intersection(&cells_b).count();
    let min_size = cells_a.len().min(cells_b.len());
    
    if min_size == 0 { 1.0 } else { intersection as f64 / min_size as f64 }
}

fn weighted_jaccard_similarity(row_a: &RowView, row_b: &RowView) -> f64 {
    let mut matched_weight = 0.0;
    let mut total_weight = 0.0;
    
    let cells_b_map: HashMap<u32, u64> = row_b.cells.iter()
        .map(|(col, cell)| (*col, hash_cell(cell)))
        .collect();
    
    for (col, cell) in &row_a.cells {
        let weight = cell_weight(cell);
        total_weight += weight;
        
        if let Some(&hash_b) = cells_b_map.get(col) {
            if hash_cell(cell) == hash_b {
                matched_weight += weight;
            }
        }
    }
    
    for (col, cell) in &row_b.cells {
        if !row_a.cells.iter().any(|(c, _)| c == col) {
            total_weight += cell_weight(cell);
        }
    }
    
    if total_weight == 0.0 { 1.0 } else { matched_weight / total_weight }
}

fn cell_weight(cell: &Cell) -> f64 {
    match &cell.value {
        CellValue::Empty => 0.1,
        CellValue::Boolean(_) => 0.5,
        CellValue::Number(_) => 1.0,
        CellValue::String(s) => (1.0 + (s.len() as f64).ln()).min(3.0),
        CellValue::Error(_) => 0.3,
    }
}
```

### 4.4 Threshold Validation

```rust
pub struct ThresholdValidation {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
}

pub fn validate_threshold(
    ground_truth_moves: &[(Range<u32>, Range<u32>)],
    detected_moves: &[ValidatedMove],
    threshold: f64,
) -> ThresholdValidation {
    let mut true_positives = 0;
    let mut false_positives = 0;
    
    for detected in detected_moves {
        let is_true_positive = ground_truth_moves.iter().any(|(src, dst)| {
            ranges_overlap(src, &detected.source_rows) && ranges_overlap(dst, &detected.dest_rows)
        });
        
        if is_true_positive {
            true_positives += 1;
        } else {
            false_positives += 1;
        }
    }
    
    let false_negatives = ground_truth_moves.len().saturating_sub(true_positives);
    
    let precision = if true_positives + false_positives > 0 {
        true_positives as f64 / (true_positives + false_positives) as f64
    } else {
        1.0
    };
    
    let recall = if true_positives + false_negatives > 0 {
        true_positives as f64 / (true_positives + false_negatives) as f64
    } else {
        1.0
    };
    
    let f1_score = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    
    ThresholdValidation {
        true_positives,
        false_positives,
        false_negatives,
        precision,
        recall,
        f1_score,
    }
}
```

---

## 5. Deterministic Output Guarantees

### 5.1 Requirements

- Same inputs must produce byte-identical outputs
- Parallel execution must not affect output order
- Hash-based iteration must be stabilized

### 5.2 Deterministic Hasher

```rust
use std::hash::BuildHasher;

pub struct DeterministicHasher {
    seed: u64,
}

impl DeterministicHasher {
    pub fn new() -> Self {
        DeterministicHasher { seed: 0x517cc1b727220a95 }
    }
}

impl BuildHasher for DeterministicHasher {
    type Hasher = XxHash64;
    
    fn build_hasher(&self) -> XxHash64 {
        XxHash64::with_seed(self.seed)
    }
}

pub type DeterministicHashMap<K, V> = HashMap<K, V, DeterministicHasher>;
pub type DeterministicHashSet<T> = HashSet<T, DeterministicHasher>;
```

### 5.3 Sorted Parallel Collection

```rust
use rayon::prelude::*;

pub fn parallel_map_sorted<T, U, F>(items: &[T], f: F) -> Vec<U>
where
    T: Sync,
    U: Send + Ord,
    F: Fn(&T) -> U + Sync,
{
    let mut results: Vec<(usize, U)> = items
        .par_iter()
        .enumerate()
        .map(|(i, item)| (i, f(item)))
        .collect();
    
    results.sort_by_key(|(i, _)| *i);
    results.into_iter().map(|(_, u)| u).collect()
}

pub fn parallel_flat_map_sorted<T, U, F>(items: &[T], f: F) -> Vec<U>
where
    T: Sync,
    U: Send,
    F: Fn(&T) -> Vec<(SortKey, U)> + Sync,
{
    let mut results: Vec<(SortKey, U)> = items
        .par_iter()
        .flat_map(|item| f(item))
        .collect();
    
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results.into_iter().map(|(_, u)| u).collect()
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortKey {
    pub primary: u32,
    pub secondary: u32,
    pub tertiary: u32,
}
```

### 5.4 Operation Ordering

```rust
impl DiffOp {
    pub fn sort_key(&self) -> (u8, u32, u32, u32) {
        match self {
            DiffOp::RowRemoved { row_a } => (0, *row_a, 0, 0),
            DiffOp::RowAdded { row_b } => (1, *row_b, 0, 0),
            DiffOp::ColumnRemoved { col_a } => (2, 0, *col_a, 0),
            DiffOp::ColumnAdded { col_b } => (3, 0, *col_b, 0),
            DiffOp::BlockMovedRows { source, dest } => (4, source.start, dest.start, 0),
            DiffOp::BlockMovedColumns { source, dest } => (5, 0, source.start, dest.start),
            DiffOp::CellEdited { row_a, col_a, .. } => (6, *row_a, *col_a, 0),
        }
    }
}

pub fn finalize_diff_result(mut ops: Vec<DiffOp>) -> DiffResult {
    ops.sort_by_key(|op| op.sort_key());
    DiffResult { ops }
}
```

### 5.5 Deterministic Cell Diff

```rust
pub fn deterministic_cell_diff(
    matched_rows: &[(u32, u32)],
    view_a: &GridView,
    view_b: &GridView,
    col_mapping: &[Option<u32>],
) -> Vec<CellEdit> {
    let mut all_edits: Vec<(SortKey, CellEdit)> = matched_rows
        .par_iter()
        .flat_map(|(row_a, row_b)| {
            let edits = diff_aligned_rows(
                &view_a.rows[*row_a as usize],
                &view_b.rows[*row_b as usize],
                col_mapping,
            );
            edits.into_iter().map(|edit| {
                let key = SortKey {
                    primary: *row_a,
                    secondary: edit.col_a,
                    tertiary: 0,
                };
                (key, edit)
            }).collect::<Vec<_>>()
        })
        .collect();
    
    all_edits.sort_by(|a, b| a.0.cmp(&b.0));
    all_edits.into_iter().map(|(_, edit)| edit).collect()
}
```

---

## 6. Rectangular Block Move Detection

### 6.1 Problem Definition

Detect when a rectangular region (spanning multiple rows AND columns) has been moved as a unit:

```
Grid A:                    Grid B:
┌───┬───┬───┬───┐         ┌───┬───┬───┬───┐
│   │   │ X │ Y │         │   │   │   │   │
├───┼───┼───┼───┤         ├───┼───┼───┼───┤
│   │   │ Z │ W │  ──►    │ X │ Y │   │   │
├───┼───┼───┼───┤         ├───┼───┼───┼───┤
│   │   │   │   │         │ Z │ W │   │   │
└───┴───┴───┴───┘         └───┴───┴───┴───┘
```

### 6.2 Data Structures

```rust
pub struct RectRegion {
    pub row_range: Range<u32>,
    pub col_range: Range<u32>,
}

pub struct RectMove {
    pub source: RectRegion,
    pub dest: RectRegion,
    pub similarity: f64,
}

pub struct RectMoveCandidate {
    pub row_move: Option<ValidatedMove>,
    pub col_move: Option<ColumnMove>,
    pub correlation_score: f64,
}
```

### 6.3 Detection Algorithm

```rust
pub fn detect_rect_moves(
    row_moves: &[ValidatedMove],
    col_moves: &[ColumnMove],
    view_a: &GridView,
    view_b: &GridView,
) -> Vec<RectMove> {
    if row_moves.is_empty() || col_moves.is_empty() {
        return Vec::new();
    }
    
    let mut rect_moves = Vec::new();
    
    for row_move in row_moves {
        for col_move in col_moves {
            let correlation = compute_move_correlation(row_move, col_move, view_a, view_b);
            
            if correlation > 0.8 {
                let source = RectRegion {
                    row_range: row_move.source_rows.clone(),
                    col_range: col_move.source.clone(),
                };
                let dest = RectRegion {
                    row_range: row_move.dest_rows.clone(),
                    col_range: col_move.dest.clone(),
                };
                
                let similarity = compute_rect_similarity(&source, &dest, view_a, view_b);
                
                if similarity > 0.9 {
                    rect_moves.push(RectMove {
                        source,
                        dest,
                        similarity,
                    });
                }
            }
        }
    }
    
    deduplicate_rect_moves(rect_moves)
}

fn compute_move_correlation(
    row_move: &ValidatedMove,
    col_move: &ColumnMove,
    view_a: &GridView,
    view_b: &GridView,
) -> f64 {
    let source_rect = extract_rect_cells(
        &row_move.source_rows,
        &col_move.source,
        view_a,
    );
    let dest_rect = extract_rect_cells(
        &row_move.dest_rows,
        &col_move.dest,
        view_b,
    );
    
    if source_rect.is_empty() && dest_rect.is_empty() {
        return 0.0;
    }
    
    let source_hash = hash_cell_set(&source_rect);
    let dest_hash = hash_cell_set(&dest_rect);
    
    if source_hash == dest_hash { 1.0 } else { 0.0 }
}

fn compute_rect_similarity(
    source: &RectRegion,
    dest: &RectRegion,
    view_a: &GridView,
    view_b: &GridView,
) -> f64 {
    let source_cells = extract_rect_cells(&source.row_range, &source.col_range, view_a);
    let dest_cells = extract_rect_cells(&dest.row_range, &dest.col_range, view_b);
    
    let source_set: HashSet<_> = source_cells.iter()
        .map(|((r, c), cell)| (r - source.row_range.start, c - source.col_range.start, hash_cell(cell)))
        .collect();
    let dest_set: HashSet<_> = dest_cells.iter()
        .map(|((r, c), cell)| (r - dest.row_range.start, c - dest.col_range.start, hash_cell(cell)))
        .collect();
    
    let intersection = source_set.intersection(&dest_set).count();
    let union = source_set.union(&dest_set).count();
    
    if union == 0 { 1.0 } else { intersection as f64 / union as f64 }
}

fn extract_rect_cells(
    row_range: &Range<u32>,
    col_range: &Range<u32>,
    view: &GridView,
) -> Vec<((u32, u32), &Cell)> {
    let mut cells = Vec::new();
    
    for row_idx in row_range.clone() {
        if let Some(row) = view.rows.get(row_idx as usize) {
            for (col, cell) in &row.cells {
                if col_range.contains(col) {
                    cells.push(((row_idx, *col), *cell));
                }
            }
        }
    }
    
    cells
}
```

### 6.4 Output Integration

```rust
pub enum DiffOp {
    // ... existing variants ...
    BlockMovedRect {
        source_rows: Range<u32>,
        source_cols: Range<u32>,
        dest_rows: Range<u32>,
        dest_cols: Range<u32>,
    },
}

pub fn merge_rect_moves_into_result(
    mut ops: Vec<DiffOp>,
    rect_moves: &[RectMove],
) -> Vec<DiffOp> {
    for rect_move in rect_moves {
        ops.retain(|op| !is_covered_by_rect_move(op, rect_move));
        
        ops.push(DiffOp::BlockMovedRect {
            source_rows: rect_move.source.row_range.clone(),
            source_cols: rect_move.source.col_range.clone(),
            dest_rows: rect_move.dest.row_range.clone(),
            dest_cols: rect_move.dest.col_range.clone(),
        });
    }
    
    ops
}

fn is_covered_by_rect_move(op: &DiffOp, rect_move: &RectMove) -> bool {
    match op {
        DiffOp::BlockMovedRows { source, dest } => {
            ranges_subset(source, &rect_move.source.row_range) &&
            ranges_subset(dest, &rect_move.dest.row_range)
        }
        DiffOp::BlockMovedColumns { source, dest } => {
            ranges_subset(source, &rect_move.source.col_range) &&
            ranges_subset(dest, &rect_move.dest.col_range)
        }
        _ => false,
    }
}
```

---

## 7. String Interning System

### 7.1 Design Goals

- Deduplicate repeated string values across cells
- Reduce memory footprint for large spreadsheets
- Maintain O(1) lookup and comparison via IDs
- Support concurrent access for parallel parsing

### 7.2 String Pool Implementation

```rust
use parking_lot::RwLock;
use std::sync::Arc;

pub struct StringPool {
    strings: RwLock<Vec<Arc<str>>>,
    index: RwLock<HashMap<Arc<str>, u32, DeterministicHasher>>,
}

impl StringPool {
    pub fn new() -> Self {
        StringPool {
            strings: RwLock::new(Vec::new()),
            index: RwLock::new(HashMap::with_hasher(DeterministicHasher::new())),
        }
    }
    
    pub fn intern(&self, s: &str) -> StringId {
        {
            let index = self.index.read();
            if let Some(&id) = index.get(s) {
                return StringId(id);
            }
        }
        
        let mut strings = self.strings.write();
        let mut index = self.index.write();
        
        if let Some(&id) = index.get(s) {
            return StringId(id);
        }
        
        let id = strings.len() as u32;
        let arc: Arc<str> = s.into();
        strings.push(arc.clone());
        index.insert(arc, id);
        
        StringId(id)
    }
    
    pub fn get(&self, id: StringId) -> Option<Arc<str>> {
        let strings = self.strings.read();
        strings.get(id.0 as usize).cloned()
    }
    
    pub fn total_bytes(&self) -> usize {
        let strings = self.strings.read();
        strings.iter().map(|s| s.len() + std::mem::size_of::<Arc<str>>()).sum()
    }
    
    pub fn unique_count(&self) -> usize {
        self.strings.read().len()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StringId(pub u32);

impl StringId {
    pub const EMPTY: StringId = StringId(u32::MAX);
}
```

### 7.3 Interned Cell Value

```rust
#[derive(Clone, PartialEq)]
pub enum InternedCellValue {
    Empty,
    Boolean(bool),
    Number(f64),
    String(StringId),
    Error(ErrorType),
}

pub struct InternedCell {
    pub value: InternedCellValue,
    pub formula: Option<StringId>,
}

impl InternedCell {
    pub fn from_cell(cell: &Cell, pool: &StringPool) -> Self {
        let value = match &cell.value {
            CellValue::Empty => InternedCellValue::Empty,
            CellValue::Boolean(b) => InternedCellValue::Boolean(*b),
            CellValue::Number(n) => InternedCellValue::Number(*n),
            CellValue::String(s) => InternedCellValue::String(pool.intern(s)),
            CellValue::Error(e) => InternedCellValue::Error(e.clone()),
        };
        
        let formula = cell.formula.as_ref().map(|f| pool.intern(f));
        
        InternedCell { value, formula }
    }
    
    pub fn to_cell(&self, pool: &StringPool) -> Cell {
        let value = match &self.value {
            InternedCellValue::Empty => CellValue::Empty,
            InternedCellValue::Boolean(b) => CellValue::Boolean(*b),
            InternedCellValue::Number(n) => CellValue::Number(*n),
            InternedCellValue::String(id) => {
                CellValue::String(pool.get(*id).map(|s| s.to_string()).unwrap_or_default())
            }
            InternedCellValue::Error(e) => CellValue::Error(e.clone()),
        };
        
        let formula = self.formula.and_then(|id| pool.get(id).map(|s| s.to_string()));
        
        Cell { value, formula }
    }
}
```

### 7.4 Interned Grid

```rust
pub struct InternedGrid {
    pub cells: HashMap<(u32, u32), InternedCell>,
    pub pool: Arc<StringPool>,
    pub nrows: u32,
    pub ncols: u32,
}

impl InternedGrid {
    pub fn from_grid(grid: &Grid, pool: Arc<StringPool>) -> Self {
        let cells = grid.cells.iter()
            .map(|((r, c), cell)| ((*r, *c), InternedCell::from_cell(cell, &pool)))
            .collect();
        
        InternedGrid {
            cells,
            pool,
            nrows: grid.nrows,
            ncols: grid.ncols,
        }
    }
    
    pub fn memory_usage(&self) -> MemoryBreakdown {
        let cell_overhead = self.cells.len() * (
            8 + // HashMap entry overhead
            8 + // (u32, u32) key
            std::mem::size_of::<InternedCell>()
        );
        
        let pool_size = self.pool.total_bytes();
        
        MemoryBreakdown {
            cell_structures: cell_overhead,
            string_pool: pool_size,
            total: cell_overhead + pool_size,
        }
    }
}

pub struct MemoryBreakdown {
    pub cell_structures: usize,
    pub string_pool: usize,
    pub total: usize,
}
```

### 7.5 Shared Pool for Diff

```rust
pub fn diff_with_shared_pool(
    grid_a: &Grid,
    grid_b: &Grid,
    config: &DiffConfig,
) -> DiffResult {
    let pool = Arc::new(StringPool::new());
    
    let interned_a = InternedGrid::from_grid(grid_a, pool.clone());
    let interned_b = InternedGrid::from_grid(grid_b, pool.clone());
    
    log::info!(
        "String pool: {} unique strings, {} bytes",
        pool.unique_count(),
        pool.total_bytes()
    );
    
    diff_interned_grids(&interned_a, &interned_b, config)
}
```

### 7.6 Memory Savings Analysis

```rust
pub fn estimate_interning_savings(grid: &Grid) -> InterningSavings {
    let mut string_counts: HashMap<&str, usize> = HashMap::new();
    let mut total_string_bytes = 0usize;
    
    for cell in grid.cells.values() {
        if let CellValue::String(s) = &cell.value {
            *string_counts.entry(s.as_str()).or_insert(0) += 1;
            total_string_bytes += s.len();
        }
        if let Some(f) = &cell.formula {
            *string_counts.entry(f.as_str()).or_insert(0) += 1;
            total_string_bytes += f.len();
        }
    }
    
    let unique_bytes: usize = string_counts.keys().map(|s| s.len()).sum();
    let id_overhead = string_counts.values().sum::<usize>() * 4;
    
    let without_interning = total_string_bytes;
    let with_interning = unique_bytes + id_overhead;
    
    InterningSavings {
        total_strings: string_counts.values().sum(),
        unique_strings: string_counts.len(),
        bytes_without_interning: without_interning,
        bytes_with_interning: with_interning,
        savings_bytes: without_interning.saturating_sub(with_interning),
        savings_percent: if without_interning > 0 {
            100.0 * (without_interning - with_interning) as f64 / without_interning as f64
        } else {
            0.0
        },
    }
}

pub struct InterningSavings {
    pub total_strings: usize,
    pub unique_strings: usize,
    pub bytes_without_interning: usize,
    pub bytes_with_interning: usize,
    pub savings_bytes: usize,
    pub savings_percent: f64,
}
```

---

## 8. Implementation Priority

| Gap | Priority | Complexity | Dependencies |
|-----|----------|------------|--------------|
| String Interning | **High** | Low | None (can retrofit) |
| Deterministic Output | **High** | Low | None |
| Memory Budget | **High** | Medium | String Interning |
| Key Inference | **Medium** | Medium | None |
| Fuzzy Thresholds | **Medium** | Medium | Test dataset |
| Formula Semantics | **Medium** | High | Parser infrastructure |
| Rectangular Moves | **Low** | Medium | Row/Column moves |

### Recommended Implementation Order

1. **String Interning** — Immediate memory benefits, low risk
2. **Deterministic Output** — Essential for testing, simple to implement
3. **Memory Budget** — Required for production robustness
4. **Key Inference** — Enables auto database mode
5. **Fuzzy Thresholds** — Improves move detection quality
6. **Formula Semantics** — Significant effort, high value for power users
7. **Rectangular Moves** — Polish feature, can defer

