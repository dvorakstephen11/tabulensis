## 1. Scope and Requirements

The diff engine’s job is:

1. Take two Excel (or PBIX/PBIT) workbooks.
2. Build a *semantic* internal representation:

   * Workbook / sheets / tables / cells.
   * Power Query M queries and metadata (via DataMashup). 
   * Power Pivot / DAX measures (later phase). 
3. Produce a **hierarchical diff**:

   * Workbook & object structure changes.
   * Grid/table changes (rows, columns, cells).
   * Semantic changes in M queries, DAX, and formula logic.
   * Metadata‑only changes (load destinations, privacy, groupings, etc.). 

The architecture is tuned for:

* Multi‑platform Rust/WASM engine. 
* “Instant diff” behavior even on ~100 MB workbooks by using streaming, linear or near‑linear algorithms where possible. 

---

## 2. Internal Data Model

All diff algorithms operate on a normalized IR; parsing is handled by the DataMashup parser and Open XML layer. 

### 2.1 Workbook‑level types

At a high level:

```text
Workbook {
    id: WorkbookId,
    sheets: Vec<Sheet>,
    data_model: Option<DataModel>,   // DAX, relationships (phase 3+)
    mashup: Option<DataMashup>,      // M queries + metadata
}

Sheet {
    name: String,
    kind: SheetKind,                 // Worksheet, Chart, Macro, etc.
    grid: Grid,                      // 2D cells
    tables: Vec<Table>,              // Excel Tables
}

Grid {
    nrows: u32,
    ncols: u32,
    rows: Vec<Row>,
}

Row {
    index: u32,                      // 0-based logical index
    cells: Vec<Cell>,
}

Cell {
    row: u32,
    col: u32,
    address: CellAddress,            // "A1"
    formula: Option<FormulaAst>,     // parsed AST
    value: Option<CellValue>,        // typed value
    format: CellFormatSummary,       // simplified style info
}
```

We *do not* keep the raw XML at this layer; that’s already validated at parse time.

### 2.2 Power Query (M) model

The M parser blueprint exposes: 

```text
DataMashup {
    queries: Vec<Query>,
    metadata: Metadata,
}

Query {
    name: String,            // "Section1/Foo"
    section_member: String,  // "Foo"
    expression_m: String,    // original code
    ast: MModuleAst,         // parsed AST
    steps: Vec<MStep>,       // normalized pipeline-style representation
    meta: QueryMetadata,
}
```

* `steps` is the crucial structure for semantic diff. Each step corresponds to a meaningful UI operation (filter, join, column removal, etc.) when possible.

### 2.3 Data model (DAX) – future‑phase

Analogous to queries:

```text
DataModel {
    tables: Vec<ModelTable>,
    relationships: Vec<ModelRelationship>,
    measures: Vec<Measure>,
}

Measure {
    table: String,
    name: String,
    dax_source: String,
    dax_ast: DaxAst,
}
```

The diff algorithms below are designed so DAX can plug into the same AST diff machinery as M.

---

## 3. High‑Level Diff Pipeline

The diff engine is organized as a cascade of increasingly fine‑grained comparers:

1. **Object graph diff**

   * Workbook metadata, sheets, named ranges, tables, charts, VBA modules.
2. **Tabular diff (Database Mode)**

   * For sheets/tables with a key column.
3. **Grid diff (Spreadsheet Mode)**

   * 2D alignment for non‑keyed sheets (financial models, templates).
4. **Semantic diff for logic**

   * M queries, DAX, and cell formulas via AST comparison.
5. **Metadata diff**

   * Load destinations, query groups, privacy flags, etc.

Each stage operates on a well‑typed input and emits a stream of `DiffOp` objects:

```text
enum DiffOp {
    SheetAdded { name: String, ... },
    SheetRemoved { name: String, ... },
    RowAdded { sheet: SheetId, row: RowIdx, ... },
    RowRemoved { ... },
    CellEdited { sheet: SheetId, addr: CellAddress, from: CellSnapshot, to: CellSnapshot },
    MQueryChange { name: String, detail: MQueryChangeDetail },
    DaxMeasureChange { ... },
    MetadataChange { path: MetadataPath, from: MetaValue, to: MetaValue },
    // etc.
}
```

The **frontend** and **CLI** consume these ops to render visual diffs or JSON reports, and the **testing plan** uses the same ops to assert correctness. 

---

## 4. Object Graph Diff

Before looking at cells, we diff the *structure* of the workbook.

### 4.1 Sheet and object identity

We treat:

* Sheets keyed by (case‑insensitive) name plus type.
* Tables keyed by sheet + table name.
* Named ranges keyed by name.

Algorithm (for any keyed object set):

1. Build maps `A: name -> objectA` and `B: name -> objectB`.
2. Objects only in `A` → `Removed`.
3. Only in `B` → `Added`.
4. In both → recursively diff their content.

To detect **renames**, we add a cheap similarity layer:

* Compute a signature for each object (e.g., hash of first N non‑empty cells or hash of M query name set).
* For objects with high signature similarity but different names, run a stable matching (Hungarian) on `1 - similarity` cost matrix and emit `Renamed` events when cost is below a threshold.

Complexity is dominated by building maps O(n) and a small Hungarian instance O(k³), where k is number of ambiguous candidates (usually small).

---

## 5. Tabular Diff (Database Mode)

For sheets/tables that behave like relational tables (lists of transactions, dimension tables), we use **key‑based diff** for O(N) alignment. This covers the “Database Mode” case described in the product plan. 

### 5.1 Key discovery

We have three modes:

1. **User‑provided key** (strongest): user marks one or more key columns.
2. **Metadata key**: Excel table “unique key” or query metadata tells us the primary key.
3. **Heuristic key inference**:

   * Candidate columns: those with no blanks and high uniqueness ratio.
   * Prefer integer/ID‑like columns (`[A-Z]*\d+` patterns, GUIDs).
   * If no single column qualifies, consider column combinations (limited to small subsets).

We expose the chosen key in the diff report for transparency.

### 5.2 Keyed diff algorithm

Given:

* Two tables `TA` and `TB`.
* Key function `key(row) -> Key`.

1. Build hash maps:

   ```text
   mapA: Key -> Vec<RowA>
   mapB: Key -> Vec<RowB>
   ```

   (vectors allow us to surface duplicate‑key issues explicitly.)

2. For each key in `mapA ∪ mapB`:

   * If key ∈ `A` only → `RowRemoved`.
   * If key ∈ `B` only → `RowAdded`.
   * If key in both:

     * Handle duplicates:

       * If both sides have multiple rows, treat as a *duplicate key cluster* and run a small Hungarian match using row similarity (e.g., Jaccard similarity on changed columns).
     * For each matched pair, run **row diff** (see below).

3. **Row diff**:

   * Compare cells field‑wise:

     * If formulas differ semantically (AST diff) or values differ → `CellEdited`.
     * Track which columns changed to produce per‑row summaries.

Complexity:

* Hash map construction: O(N) (N = total rows).
* Row comparisons: O(M) per matched row (M = columns).
* Hungarian clusters are rare and small; cost negligible compared to O(N·M).

---

## 6. Grid Diff (Spreadsheet Mode)

This is the “financial model” case where *physical layout* and 2D alignment matter and there may be no clear key. Here we deploy the **multi‑pass hierarchical alignment** strategy sketched in the product plan. 

Key idea: reduce the 2D problem to a sequence of mostly 1D problems (rows, then columns), with smart anchors and move detection.

### 6.1 Row‑level alignment via Hunt–Szymanski

We treat each sheet as a sequence of row signatures:

1. For each row, compute a signature:

   * Hash of `(non‑blank cell positions, cell values up to normalization)`.
   * Optionally fold in formula structure but ignore constants for robustness.

2. Apply **Hunt–Szymanski** (HS) longest common subsequence algorithm on these signatures:

   * HS improves over naive O(n²) LCS by focusing on *rare* symbols, giving O((n + r) log n) where `r` is number of equal‑signature pairs.
   * Result: an ordered set of “anchor” row matches: `(iA, jB)` pairs believed to be the same logical row.

3. Between anchors, classify stretches:

   * A block of rows present in A only → deleted block.
   * In B only → inserted block.
   * Mixed → ambiguous; we recurse with a more permissive similarity metric or fall back to cell‑wise diff.

This alignment already gives robust detection of row insertions/deletions and reorders without treating the entire sheet as “changed”.

### 6.2 Column alignment

Within each aligned row block (range of rows that are matched 1‑to‑1 across sheets), we align columns:

1. Compute column signatures from header row + sample body rows.
2. Run another HS pass on column sequences to detect inserted/deleted/renamed columns.
3. Use these aligned columns when computing cell-level edits, which avoids marking entire blocks as changed when only some columns moved.

For worksheets with “headerless” regions, we can run the same algorithm on a restricted region (e.g., modeling block) identified by heuristics or user hints.

### 6.3 Block move detection

We want to distinguish **moved blocks** from delete+insert pairs.

1. For each maximal deleted row block in A, compute a composite block hash:

   * Combine per-row signatures and relative column signatures.
2. Search in B for inserted blocks with matching or similar hashes.
3. When a match is found:

   * Run a detailed cell diff inside the candidate blocks.
   * If a high fraction of cells are equal, emit `BlockMoved` instead of separate add/remove ops.

This is essentially a rolling‑hash search (rsync‑style), constrained to plausible windows to stay near O(N).

### 6.4 Cell‑level edit detection

Once row/column alignment is known, the cell diff is straightforward:

For each aligned `(rowA, rowB)` and aligned `(colA, colB)`:

1. Compare **formula ASTs** (if any).
2. Compare displayed values and types.
3. Compare formats (if we surface formatting diffs).

Classification:

* If formula ASTs equal (under canonicalization) but values differ:

  * Probably a recalculation difference only → optional `ValueOnlyChanged`.
* If formula AST changed:

  * `FormulaChanged` with an embedded AST diff (see Section 8).
* If one side empty and other non‑empty:

  * `CellAdded` or `CellRemoved`.

For unaligned rows/cols, we emit bulk operations (`RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`) rather than per‑cell ops.

---

## 7. M Query Diff Algorithms

The M diff engine is where we differentiate strongly vs incumbents; it sits on top of the DataMashup parser.  

There are three layers:

1. **Query‑level alignment** (which queries exist?).
2. **Textual diff** (early milestone). 
3. **Semantic (AST + step) diff** (core differentiator). 

### 7.1 Query alignment

We treat queries as keyed by `name = "Section1/MemberName"`.

1. Build maps `A: name -> QueryA`, `B: name -> QueryB`.
2. Direct matches on name → candidate pairs.
3. For unmatched queries, detect **renames**:

   * Compute a `query_signature`:

     * Multiset of top‑level step kinds and names.
     * Normalized hash of the AST (e.g., tree structure without identifiers).
   * For name‑mismatched queries with similar signature, run Hungarian matching on `1 - similarity` cost to find best rename candidates.
   * Thresholded matches become `Renamed { from, to }` events.

Remaining unmatched queries are additions/removals.

### 7.2 Textual diff (Milestone 6)

For each aligned query pair:

1. Compare `expression_m` strings.
2. If identical → no definition change.
3. If different:

   * Run a Myers diff (or other standard text diff) at line level and embed that in `DefinitionChanged` detail.
4. Additionally, compare `QueryMetadata` (load destination, privacy, group):

   * If only metadata changed → emit `MetadataChangedOnly`.

This layer gives a fast MVP and matches the testing plan’s `MQueryDiff` enum. 

### 7.3 Semantic (AST + step) diff

Once AST parsing is in place, we upgrade `DefinitionChanged` to structured semantic information.

#### 7.3.1 Canonicalization

Before diffing:

1. Strip whitespace and comments.
2. Normalize:

   * Commutative operations: sort operands (for simple arithmetic, logical ops).
   * `let` chains: canonicalize step order when dependencies permit (e.g., reorder independent steps if desired).
3. Normalize identifiers if you want to treat purely cosmetic renames as non‑changes (configurable).

If two canonical ASTs are byte‑identical, we treat the queries as semantically equal even if text is very different. (This supports the “formatting only” milestone.) 

#### 7.3.2 Step‑aware diff

Most user‑visible changes correspond to adding/removing/modifying **steps** in the query’s transformation pipeline.

We represent each query as:

```text
MStep {
    name: String,           // "Filtered Rows", "Removed Other Columns"
    kind: StepKind,         // Filter, GroupBy, Join, RenameColumns, ...
    parameters: StepParams, // structured field for each kind
}
```

Algorithm:

1. Build sequences `SA` and `SB` of MSteps (in order).

2. Compute an alignment using a costed sequence diff (e.g. dynamic programming):

   * Cost 0 if `kind` and key parameters match.
   * Moderate cost for parameter changes (e.g., filter predicate changes).
   * Higher cost for insert/remove.

3. The DP yields an alignment matrix; we backtrack to produce step‑level changes:

   * `StepAdded { position, step }`
   * `StepRemoved { position, step }`
   * `StepModified { from, to, detail }`

4. For each `StepModified`, we can drill into parameter structure:

   * For filters: report column and predicate change (“Region changed from `<> null` to `= "EMEA"`”). 
   * For joins: report join type changes (Inner → LeftOuter), join key changes, etc.
   * For projections: columns added/removed.

This gives exactly the semantics the testing plan expects for filters, column removals, join changes, etc. 

#### 7.3.3 Fallback tree edit distance

For steps we can’t classify, or for expressions inside steps, we can fall back to **tree edit distance** (Zhang–Shasha):

* Nodes are AST constructs.
* Edit operations: insert, delete, substitute.
* Costs chosen so that structurally small changes lead to small distances.

This yields compact diffs (“function call changed from `Table.AddColumn` to `Table.TransformColumns`”) without requiring handcrafted handling for every possible M construct.

---

## 8. DAX and Formula Diff Algorithms

DAX and Excel formulas are both expression languages; we can reuse much of the M semantic machinery.

### 8.1 Parsing and canonicalization

For each formula / measure:

1. Parse into an AST (operators, function calls, references).
2. Canonicalize:

   * Normalize whitespace, casing.
   * Reorder commutative subtrees when safe.
   * Optional: normalize equivalent syntaxes (`AND(a,b)` vs `a && b`).

If canonical ASTs are equal → no logical change (formatting only).

### 8.2 Expression diff

For differing ASTs:

1. Run tree edit distance to identify changed subtrees.
2. Summarize at a human‑useful granularity:

   * “Measure `TotalSales` changed aggregation from SUM to AVERAGE.”
   * “Filter condition on `Calendar[Year]` changed from `>= 2020` to `>= 2021`.”

Implementation detail:

* Because DAX formulas are relatively small, typical tree edit distance costs are tiny; performance is dominated by parsing, not diff.

---

## 9. Metadata Diff Algorithms

Metadata includes:

* Where queries load (sheet vs model vs connection‑only).
* Query display folders/groups.
* Table relationships (for the data model).
* Permissions/privacy flags.

We treat metadata as a *typed key–value tree*:

```text
MetadataPath = Vec<String>;  // e.g. ["MQuery", "Section1/Foo", "LoadToSheet"]
MetaValue    = Enum { Bool, Int, String, Enum, Json, ... }
```

Algorithm:

1. Flatten both metadata trees into maps from `MetadataPath` to `MetaValue`.
2. For each path in the union:

   * If value only in A → `MetadataRemoved`.
   * Only in B → `MetadataAdded`.
   * Both but unequal → `MetadataChanged`.

Changes are grouped under logical domains (e.g., query `Foo`’s load destinations), which drives user‑facing categories like `MetadataChangedOnly` when the query logic didn’t change. 

---

## 10. Complexity and Performance

### 10.1 Grid diff

Let:

* `R` = rows, `C` = columns.

Row HS alignment: O((R + r) log R) where `r` is number of equal signature pairs; typical real‑world sheets have relatively distinctive rows (low `r`).

Within blocks:

* Column HS: O((C + c) log C).
* Cell diffs: only for aligned rows/cols, typically O(R·C) in the common case but with early exits for equal rows/cells.

Block move detection uses rolling hashes and windows, keeping the total cost linear in practice.

### 10.2 Tabular diff

Keyed diff is O(N) in number of rows plus O(M) per changed row; we never perform global quadratic algorithms on big tables.

### 10.3 M / DAX diff

* Query alignment: O(Q log Q) for maps and small matching, `Q` = number of queries.
* Step alignment uses dynamic programming; step sequences rarely exceed a few dozen entries, so complexity is negligible.
* AST diff costs are small because expressions are compact.

The design is consistent with the product‑plan goal: “compare 100MB files in under ~2 seconds” given streaming parsers and native Rust performance. 

---

## 11. Implementation Patterns in Rust

### 11.1 Diffable trait

To keep the engine modular:

```text
trait Diffable {
    type Diff;
    fn diff(&self, other: &Self) -> Self::Diff;
}
```

We implement `Diffable` for:

* `Workbook`, `Sheet`, `Grid`, `Row`, `Cell`.
* `Query`, `MStep`, `Measure`.
* Metadata trees.

Each `Diff` type is convertible to `Vec<DiffOp>`, so a generic driver can orchestrate the whole comparison:

```text
fn diff_workbooks(a: &Workbook, b: &Workbook) -> Vec<DiffOp> {
    let mut ops = Vec::new();
    ops.extend(a.diff(b).into_ops());
    ops
}
```

### 11.2 Streaming and memory

* XML and DataMashup parsing is already designed to be streaming and bounded. 
* For huge sheets, we can compute row signatures on a streaming basis, storing only hashes and basic row metadata until we need to inspect cells.

### 11.3 Testability

The testing plan is aligned with these abstractions:

* Unit tests exercise `Diffable` implementations (e.g., M query diff kinds). 
* Integration tests run end‑to‑end from real `.xlsx` pairs to JSON diff reports, asserting counts and categories (added/removed/definition vs metadata changes). 

This separation keeps the core algorithms independently verifiable while still matching the product‑level contracts (CLI, web viewer).

---

## 12. Putting It All Together

End‑to‑end, the diff identification process is:

1. Parse both workbooks into `Workbook` IR, including `DataMashup` and data model if present. 

2. Run:

   * Object graph diff (sheets, tables, queries, measures).
   * For each sheet/table: choose **Database** or **Spreadsheet** mode and run the corresponding alignment algorithm.
   * For each aligned cell/formula/measure: perform AST diff to distinguish formatting vs logic changes.
   * For each M query: run step‑aware semantic diff; same for DAX measures.
   * For metadata: run tree diff, using specialized categories like `MetadataChangedOnly`.

3. Aggregate the resulting `DiffOp` stream by scope (Workbook → Sheet → Object → Cell/Query) for presentation and for automated consumers (CLI/JSON, CI integrations).

The result is an engine that:

* Understands Excel files as multi‑layered data products, not just grids.
* Uses alignment and AST techniques that are asymptotically efficient and tuned to real‑world patterns.
* Lines up exactly with the testing milestones and product roadmap you already outlined.

