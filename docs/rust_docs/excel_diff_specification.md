# Excel Diff Engine: Technical Specification

> This document specifies the parsing, data model, and diff algorithms for the Excel Diff Engine.
> For testing strategy and milestones, see `excel_diff_testing_plan.md`.
> Legacy PG3/PG4/PG5 callouts describe the pre-refactor shape; see `docs/meta/plans/2025-11-29-refactor/spec.md` for the current architecture baseline.

---

## 1. Scope and Requirements

The diff engine's job is:

1. Take two Excel (or PBIX/PBIT) workbooks.
2. Build a *semantic* internal representation:
   * Workbook / sheets / tables / cells.
   * Power Query M queries and metadata (via DataMashup).
   * Power Pivot / DAX measures (later phase).
3. Produce a **hierarchical diff**:
   * Workbook and object structure changes.
   * Grid/table changes (rows, columns, cells).
   * Semantic changes in M queries, DAX, and formula logic.
   * Metadata-only changes (load destinations, privacy, groupings, etc.).

The architecture is tuned for:

* Multi-platform Rust/WASM engine.
* "Instant diff" behavior even on ~100 MB workbooks by using streaming, linear or near-linear algorithms where possible.

---

## 2. Architecture Overview

The diff engine is organized as a layered pipeline:

1. **Host Container Layer** - Opens Excel/PBIX files, locates and extracts the DataMashup stream.
2. **Binary Framing Layer** - Parses the MS-QDEFF top-level structure into four sections.
3. **Semantic Parsing Layer** - Interprets PackageParts (OPC/ZIP), Permissions, Metadata, and Bindings.
4. **Domain Layer** - Exposes queries, metadata, and workbook structure as typed Rust structs.
5. **Diff Layer** - Compares two domain representations and emits `DiffOp` events.

```text
Excel / PBIX file
        |
Host Container Layer (OPC/ZIP, base64 decode for Excel)
        |
Binary Framing Layer (MS-QDEFF: version + four length-prefixed slices)
        |
Semantic Parsing Layer (PackageParts / Permissions / Metadata / PermissionBindings)
        |
Domain Layer (Workbook, Sheet, Grid, Query, Metadata, Measure)
        |
Diff Layer -> Vec<DiffOp>
```

This document covers layers 1-4 in detail and specifies the algorithms for layer 5.

---

## 3. Host Container Layer

### 3.1 Excel (.xlsx/.xlsm/.xlsb)

1. Treat the workbook as an **OPC / Open XML package** (ZIP with `[Content_Types].xml`).([bengribaudo.com][1])
2. Iterate `/customXml/item*.xml` parts:
   * Look for a document whose root element is `DataMashup` in namespace `http://schemas.microsoft.com/DataMashup`.([bengribaudo.com][1])
3. The `<DataMashup>` element's text content is **base64**; decode it - this is your **top-level binary stream**.

Edge cases / invariants:

* There should be exactly one `DataMashup` part if the workbook uses Power Query.([bengribaudo.com][1])
* The `sqmid` attribute (optional GUID) is telemetry only; ignore for semantics.

### 3.2 Older PBIX/PBIT

1. Treat `.pbix` / `.pbit` as OPC/ZIP.
2. Open the `DataMashup` file at the root of the package. No base64 wrapper; this *is* the top-level binary stream.([bengribaudo.com][1])

Caveat: newer PBIX with **enhanced dataset metadata** may no longer store a DataMashup file; Power BI regenerates mashups from the tabular model and the M code lives elsewhere (DMVs etc.).([Power BI Community][2])

Your parser should therefore:

* Detect absence of `DataMashup` and clearly report "new-style PBIX without DataMashup; use tabular model path instead."

### 3.3 Error layering

`ExcelOpenError` is the facade surface for workbook parsing and JSON serialization. It is intentionally thin and delegates to layered error types:

* `ExcelOpenError` variants: `Container(#[from] ContainerError)`, `GridParse(#[from] GridParseError)`, `DataMashup(#[from] DataMashupError)`, `WorkbookXmlMissing`, `WorksheetXmlMissing { sheet_name }`, `SerializationError(String)` (used by JSON helpers).
* `ContainerError` covers host issues: `Io`, `NotZipContainer`, `NotOpcPackage` (missing `[Content_Types].xml`).
* `GridParseError` covers sheet XML problems: `XmlError`, `InvalidAddress`, `SharedStringOutOfBounds`.
* `DataMashupError` covers the optional mashup stream: `Base64Invalid`, `UnsupportedVersion { version }`, `FramingInvalid`, `XmlError`.

Host- and DataMashup-specific details live in their respective error types so `ExcelOpenError` stays stable while inner layers can evolve independently.

---

## 4. DataMashup Binary Parsing (MS-QDEFF)

### 4.1 Top-Level Binary Layout

MS-QDEFF defines the top-level stream as:

```text
offset  size  field
0       4     Version                     (uint32 LE, MUST be 0 currently)
4       4     PackagePartsLength          (uint32 LE)
8       N     PackageParts                (N bytes)
...     4     PermissionsLength           (uint32 LE)
...     M     Permissions                 (M bytes)
...     4     MetadataLength              (uint32 LE)
...     K     Metadata                    (K bytes)
...     4     PermissionBindingsLength    (uint32 LE)
...     P     PermissionBindings          (P bytes)
```

Each `*Length` is a 4-byte unsigned little-endian integer.

Invariants to enforce:

* `Version == 0` (for now). Treat any other value as either "future version" (warn but attempt) or hard error, depending on tolerance.
* Total stream length must be **at least** 4 + 4 + 4 + 4 + 4 (header + four zero-length sections).
* Sum of lengths must not exceed the buffer length:

  ```text
  4 + (4+N) + (4+M) + (4+K) + (4+P) == total_bytes
  ```

  or, at minimum, `running_offset <= total_bytes` at each step.

### 4.2 PackageParts (Embedded OPC/ZIP)

MS-QDEFF: `Package Parts` is itself an **OPC package** with at least these parts:

| Part path              | Purpose                                                |
| ---------------------- | ------------------------------------------------------ |
| `/Config/Package.xml`  | Client version, minimum reader version, culture, etc.  |
| `/Formulas/Section1.m` | The Power Query M code (section document).             |
| `/Content/{GUID}`      | 0+ embedded content items, each itself an OPC package. |

These inner OPC packages begin with `PK\x03\x04` signatures; binwalk sees them as embedded ZIPs.([The Biccountant][3])

Practical parsing strategy:

1. Treat `PackageParts` bytes as a ZIP/OPC stream.
2. Use a normal ZIP/OPC library to list entries and extract required parts.
3. Read `/Config/Package.xml` as UTF-8 XML. For the current milestone, surface it as an opaque `PackageXml { raw_xml }` string without parsing individual fields; later milestones may extract structured fields (client version, minimum reader, culture, etc.).
4. Read `/Formulas/Section1.m` as UTF-8 text:
    * This is a Power Query "section document"; Excel/Power BI currently enforce a single section called `Section1` with all members shared if they are loadable.([bengribaudo.com][1])
5. For each `/Content/{GUID}`:
    * Treat as another OPC/ZIP; inside you'll find its own `/Formulas/Section1.m` and `/Config/Formulas.xml`. These are the "embedded contents" used by `Embedded.Value`.([bengribaudo.com][1])
6. Accept PackageParts entries with or without a leading `/`, but store canonical, slash-free paths for consumers (including `EmbeddedContent.name`).
7. Strip a single leading UTF-8 BOM from any Section1.m text (outer or nested) before handing it to M parsing so the section header is discoverable.

This matches what Imke's M code is doing: decode + unzip + select `"Formulas/Section1.m"`.([The Biccountant][4])

### 4.3 Permissions XML

Permissions is a small UTF-8 XML document storing three main values:([bengribaudo.com][1])

* `CanEvaluateFuturePackages` (always false, effectively ignored)
* `FirewallEnabled` (privacy/firewall on/off)
* `WorkbookGroupType` (privacy level when queries read from the workbook)

Surface these as flags. Excel and Power BI override them if Permission Bindings verification fails.

### 4.4 Metadata XML

MS-QDEFF splits this into:

* **Metadata XML**: `LocalPackageMetadataFile`, with:
  * `AllFormulas` section (query groups, relationships to data model, etc.).
  * `Formulas` entries (one per query), keyed as `SectionName/FormulaName` (URL-encoded).
  * Many properties: load destination, result type, last refresh columns, etc.
  * Some values are base64 or custom binary encodings.
* **Metadata Content (OPC)**: rarely used legacy binary stream; can often be ignored safely.

Data Mashup Explorer and Cmdlets translate this verbose mix of XML, JSON-ish content, and binary fields into a neat JSON view - your reference oracle for "what the metadata really means in practice."([bengribaudo.com][1])

Parser guidance:

1. Treat the `Metadata` section as a small header + XML + possibly an embedded OPC stream (see MS-QDEFF A2.5.2 for exact layout).
2. For normal Excel/Power BI, parse the entire `Metadata` bytes as UTF-8 XML; the XML is the "Metadata XML binary stream" described in a separate page.([bengribaudo.com][5])
3. Map known attributes (IsPrivate, LoadToDataModel, etc.) into a strongly typed struct.
4. Preserve unknown attributes as a generic bag for forward compatibility.

### 4.5 Permission Bindings

This blob protects `Permissions` from tampering. On save, Excel/Power BI compute SHA-256 hashes of Package Parts + Permissions, combine them, encrypt with DPAPI (Windows, user-scoped), and store here. On load, if decrypt+verify fails, they ignore `Permissions` and revert to defaults.([bengribaudo.com][1])

For a **cross-platform parser** that only reads M code:

* Treat Permission Bindings as **opaque bytes**, and optionally expose `bindings_present: bool`.
* Do not attempt to verify them. Even Data Mashup Cmdlets currently assume bindings are valid.([bengribaudo.com][6])
* On Windows, a full emulator could use DPAPI (`CryptUnprotectData`) and re-hash to validate.

---

## 5. Internal Data Model (IR)

All diff algorithms operate on a normalized IR; parsing is handled by the DataMashup parser and Open XML layer.

### 5.1 Workbook-Level Types

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
```

### 5.2 Grid and Cell Types

```text
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

The IR does not keep the raw XML at this layer; that is already validated at parse time.

### 5.3 Power Query (M) Model

```text
DataMashup {
    version: u32,
    package_parts: PackageParts,
    permissions: Permissions,
    metadata: Metadata,
    permission_bindings_raw: Vec<u8>,
}

PackageParts {
    package_xml: PackageXml,
    main_section: SectionDocument,
    embedded_contents: Vec<EmbeddedContent>,
}

Query {
    name: String,               // "Section1/Foo"
    section_member: String,     // "Foo"
    expression_m: String,       // original M code
    ast: MModuleAst,            // parsed AST
    steps: Vec<MStep>,          // normalized pipeline representation
    meta: QueryMetadata,
}

MStep {
    name: String,               // "Filtered Rows", "Removed Other Columns"
    kind: StepKind,             // Filter, GroupBy, Join, RenameColumns, ...
    parameters: StepParams,     // structured fields for each kind
}
```

Current parser scope (M7a) is intentionally narrow while the AST surface stabilizes:

- `parse_m_expression` understands top-level `let ... in ...` with nested `let` bindings inside values; other expressions are preserved as opaque token sequences.
- The lexer special-cases only `let`/`in`, quoted identifiers (`#"Foo"`), and hash-prefixed literals like `#date`/`#datetime`, treating other keywords generically.
- Callers should treat failures on richer M syntax as "unsupported grammar" rather than malformed input until broader grammar support lands.

`steps` is the crucial structure for semantic diff. Each step corresponds to a meaningful UI operation (filter, join, column removal, etc.) when possible. The metadata struct mirrors the fields surfaced in the Metadata XML and should preserve unknown attributes for forward compatibility.

Query lists are built by joining `Section1` shared members to Metadata formulas in **Section1 order**. Every shared member produces a `Query`; if Metadata is missing for that member, the builder synthesizes a `QueryMetadata` with `item_path = "{SectionName}/{MemberName}"`, `load_to_sheet = false`, `load_to_model = false`, `is_connection_only = true`, and `group_path = None`. Orphan metadata entries remain in `Metadata.formulas` but are not surfaced as `Query` values.

`parse_section_members` returns `SectionParseError::InvalidMemberSyntax` when a line starting with `shared` is malformed (missing identifier, `=`, or `;`), rather than silently ignoring the line, so `build_queries` only succeeds on syntactically valid section documents.

### 5.4 Data Model (DAX) - Future Phase

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

## 6. Diff Pipeline Overview

The diff engine is organized as a cascade of increasingly fine-grained comparers:

1. **Object graph diff**
   * Workbook metadata, sheets, named ranges, tables, charts, VBA modules.
2. **Tabular diff (Database Mode)**
   * For sheets/tables with a key column.
3. **Grid diff (Spreadsheet Mode)**
   * 2D alignment for non-keyed sheets (financial models, templates).
4. **Semantic diff for logic**
   * M queries, DAX, and cell formulas via AST comparison.
5. **Metadata diff**
   * Load destinations, query groups, privacy flags, etc.

Each stage operates on a well-typed input and emits a stream of `DiffOp` objects:

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

The frontend and CLI consume these ops to render visual diffs or JSON reports, and the testing plan uses the same ops to assert correctness.

### 6.1 JSON cell diff surface

The core crate exposes lightweight JSON helpers for cell-by-cell comparisons as projections from the canonical diff IR:

* `engine::diff_workbooks` is the canonical IR producer, yielding a `DiffReport` of `DiffOp` values.
* `output::json::diff_report_to_cell_diffs` filters `DiffReport` down to `CellDiff { coords, value_file1, value_file2 }` entries for the CLI/fixtures; non-cell ops are intentionally ignored in this projection.
* `serialize_diff_report` and `serialize_cell_diffs` convert either representation to JSON strings.
* `diff_workbooks_to_json` is a convenience shim for fixtures/CLI that opens workbooks, calls `engine::diff_workbooks`, and serializes the `DiffReport`. Serialization failures from this helper are mapped to `ExcelOpenError::SerializationError(String)` for clarity.

---

## 7. Object Graph Diff

Before looking at cells, diff the *structure* of the workbook.

### 7.1 Sheet and Object Identity

We treat:

* Sheets keyed by (case-insensitive) name plus type.
* Tables keyed by sheet + table name.
* Named ranges keyed by name.
* Within a single workbook, `(lowercase(sheet name), SheetKind)` keys must be **unique**. The IR producer (`open_workbook`) is expected to enforce this; `diff_workbooks` defensively `debug_assert!`s if duplicates slip through while keeping release builds deterministic (last writer wins) to avoid surprising consumers.

Algorithm (for any keyed object set):

1. Build maps `A: name -> objectA` and `B: name -> objectB`.
2. Objects only in `A` -> `Removed`.
3. Only in `B` -> `Added`.
4. In both -> recursively diff their content.

To detect **renames**, add a cheap similarity layer:

* Compute a signature for each object (e.g., hash of first N non-empty cells or hash of M query name set).
* For objects with high signature similarity but different names, run a stable matching (Hungarian) on `1 - similarity` cost matrix and emit `Renamed` events when cost is below a threshold.

Complexity is dominated by building maps O(n) and a small Hungarian instance O(k^3), where k is number of ambiguous candidates (usually small).

---

## 8. Tabular Diff (Database Mode)

For sheets/tables that behave like relational tables (lists of transactions, dimension tables), use **key-based diff** for O(N) alignment. This covers the "Database Mode" case described in the product plan.

### 8.1 Key Discovery

We have three modes:

1. **User-provided key** (strongest): user marks one or more key columns.
2. **Metadata key**: Excel table "unique key" or query metadata tells us the primary key.
3. **Heuristic key inference**:
   * Candidate columns: those with no blanks and high uniqueness ratio.
   * Prefer integer/ID-like columns (`[A-Z]*\d+` patterns, GUIDs).
   * If no single column qualifies, consider column combinations (limited to small subsets).

Expose the chosen key in the diff report for transparency.

### 8.2 Keyed Diff Algorithm

Given:

* Two tables `TA` and `TB`.
* Key function `key(row) -> Key`.

1. Build hash maps:

   ```text
   mapA: Key -> Vec<RowA>
   mapB: Key -> Vec<RowB>
   ```

   (vectors allow us to surface duplicate-key issues explicitly.)

2. For each key in `mapA union mapB`:
   * If key in `A` only -> `RowRemoved`.
   * If key in `B` only -> `RowAdded`.
   * If key in both:
     * Handle duplicates:
       * If both sides have multiple rows, treat as a *duplicate key cluster*. Formulate this as a Linear Assignment Problem (LAP) that finds the optimal matching between rows in Cluster A and Cluster B based on row similarity (e.g., Jaccard similarity on cell contents).
       * Use the **Jonker-Volgenant (LAPJV) algorithm**, an optimized O(K^3) solver faster than the classical Hungarian approach. Cluster sizes K are typically tiny, so the cost is negligible.
     * For each matched pair, run **row diff** (see below).

3. **Row diff**:
   * Compare cells field-wise:
     * If formulas differ semantically (AST diff) or values differ -> `CellEdited`.
     * Track which columns changed to produce per-row summaries.

Complexity:

* Hash map construction: O(N) (N = total rows).
* Row comparisons: O(M) per matched row (M = columns).
* Hungarian clusters are rare and small; cost negligible compared to the O(N*M) row comparison work.

---

## 9. Grid Diff (Spreadsheet Mode)

> **Detailed Specification:** For comprehensive algorithm design including pseudocode, worked examples, complexity analysis, configuration reference, and validation strategy, see [`docs/design/docs_versus_implementation/unified_grid_diff_algorithm_specification.md`](unified_grid_diff_algorithm_specification.md). This section provides a high-level overview.

This handles the "financial model" case where *physical layout* matters and there may be no clear key. We deploy a **Hybrid Alignment Pipeline** designed for robustness and near-linear performance, avoiding the pathologies of traditional LCS algorithms on repetitive spreadsheet data.

### 9.1 Phase 1: Row Hashing and Anchoring (Patience Diff)

We first segment the grid using high-confidence anchors.

1. For each row, compute a strong signature (e.g., XXHash64) of normalized cell contents.
2. Apply the **Patience Diff** algorithm (O(N log N)). This identifies the Longest Common Subsequence of *unique* identical rows.
3. These unique matches serve as anchors, dividing the grid into "gaps" (unmatched regions between anchors). This stage effectively neutralizes the impact of repetitive data (e.g., blank rows).

### 9.2 Phase 2: Gap Filling (Adaptive LCS)

We align the rows within each gap using the most appropriate LCS algorithm.

1. **Small Gaps (e.g., < 1000 rows):** Use **Myers O(ND) algorithm** (with linear space refinement). It provides precise alignment efficiently when the gap size (N) or the difference (D) is small.
2. **Large Gaps:** Use **Histogram Diff** (the modern Git standard). It is robust against density variations and generally provides high-quality alignments quickly in practice.

This phase produces an alignment where most rows are matched, leaving some as candidate insertions and deletions.

### 9.3 Phase 3: Refinement and Move Detection (Sparse LAPJV)

Analyze the unmatched rows (candidate deletions in A and insertions in B) to detect rows/blocks that were moved and potentially edited ("fuzzy moves").

1. **Candidate Generation:** Identify contiguous blocks of unmatched rows.
2. **Sparse Cost Matrix Construction:** Generate a cost matrix between deleted blocks in A and inserted blocks in B. The cost is `1 - Similarity(BlockA, BlockB)` (e.g., using Jaccard similarity of row hashes). Use a threshold to keep the matrix sparse, only including pairs with high similarity.
3. **Optimal Assignment (LAPJV):** Run the **Jonker-Volgenant (LAPJV) algorithm** on the sparse matrix to find the globally optimal matching. This aligns with recent block-aware differencing work (e.g., BDiff), which frames move detection as a Linear Assignment Problem instead of relying only on sequence heuristics.
4. **Classification:** High-similarity matches are reclassified as **Moves** (potentially with internal edits) rather than Delete+Insert operations.

### 9.4 Column Alignment

Within each aligned row block, apply a similar Hybrid Alignment strategy (Patience + Myers/Histogram) to the column signatures to detect inserted/deleted/moved columns.

### 9.5 Cell-Level Edit Detection

Once row/column alignment is known, the cell diff is straightforward.

For each aligned `(rowA, rowB)` and aligned `(colA, colB)`:

1. Compare **formula ASTs** (if any).
2. Compare displayed values and types.
3. Compare formats (if we surface formatting diffs).

Classification:

* If formula ASTs equal (under canonicalization) but values differ:
  * Probably a recalculation difference only -> optional `ValueOnlyChanged`.
* If formula AST changed:
  * `FormulaChanged` with an embedded AST diff (see Section 11).
* If one side is empty and the other is non-empty:
  * `CellAdded` or `CellRemoved`.

For unaligned rows/cols, emit bulk operations (`RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`) rather than per-cell ops.

---

## 10. M Query Diff Algorithms

> **Note:** This section assumes queries have been parsed per Section 4. The M diff engine operates on the domain-layer `Query` objects defined in Section 5.3.

The M diff engine is where we differentiate strongly vs incumbents; it sits on top of the DataMashup parser.

There are three layers:

1. **Query-level alignment** (which queries exist?).
2. **Textual diff** (early milestone).
3. **Semantic (AST + step) diff** (core differentiator).

### 10.1 Query Alignment

Treat queries as keyed by `name = "Section1/MemberName"`.

1. Build maps `A: name -> QueryA`, `B: name -> QueryB`.
2. Direct matches on name -> candidate pairs.
3. For unmatched queries, detect **renames**:
   * Compute a `query_signature`:
     * Multiset of top-level step kinds and names.
     * Normalized hash of the AST (e.g., tree structure without identifiers).
   * For name-mismatched queries with similar signature, run Hungarian matching on `1 - similarity` cost to find best rename candidates.
   * Thresholded matches become `Renamed { from, to }` events.
4. Remaining unmatched queries are additions/removals.

### 10.2 Textual Diff

For each aligned query pair:

1. Compare `expression_m` strings.
2. If identical -> no definition change.
3. If different:
   * Run a Myers diff (or other standard text diff) at line level and embed that in `DefinitionChanged` detail.
4. Additionally, compare `QueryMetadata` (load destination, privacy, group):
   * If only metadata changed -> emit `MetadataChangedOnly`.

This layer gives a fast MVP and matches the testing plan's `MQueryDiff` enum.

### 10.3 Semantic (AST + Step) Diff

Once AST parsing is in place, upgrade `DefinitionChanged` to structured semantic information.

#### 10.3.1 Canonicalization

Before diffing:

1. Strip whitespace and comments.
2. Normalize:
   * Commutative operations: sort operands (for simple arithmetic, logical ops).
   * `let` chains: canonicalize step order when dependencies permit (e.g., reorder independent steps if desired).
3. Normalize identifiers if you want to treat purely cosmetic renames as non-changes (configurable).

If two canonical ASTs are byte-identical, treat the queries as semantically equal even if text is very different. (This supports the "formatting only" milestone.)

Current code applies this equality at the `MQueryDiff` layer to suppress `DefinitionChanged` for formatting-only edits; structured step-aware semantic diffs are still pending.

#### 10.3.2 Step-Aware Diff

Most user-visible changes correspond to adding/removing/modifying **steps** in the query's transformation pipeline.

Represent each query as:

```text
MStep {
    name: String,           // "Filtered Rows", "Removed Other Columns"
    kind: StepKind,         // Filter, GroupBy, Join, RenameColumns, ...
    parameters: StepParams, // structured field for each kind
}
```

Algorithm:

1. Build sequences `SA` and `SB` of MSteps (in order).
2. Compute an alignment using a costed sequence diff (e.g., dynamic programming):
   * Cost 0 if `kind` and key parameters match.
   * Moderate cost for parameter changes (e.g., filter predicate changes).
   * Higher cost for insert/remove.
3. The DP yields an alignment matrix; backtrack to produce step-level changes:
   * `StepAdded { position, step }`
   * `StepRemoved { position, step }`
   * `StepModified { from, to, detail }`
4. For each `StepModified`, drill into parameter structure:
   * For filters: report column and predicate change ("Region changed from `<> null` to `= \"EMEA\"`").
   * For joins: report join type changes (Inner -> LeftOuter), join key changes, etc.
   * For projections: columns added/removed.

This gives the semantics the testing plan expects for filters, column removals, join changes, etc.

#### 10.3.3 Advanced AST Differencing (GumTree + APTED)

For steps we cannot classify, or for expressions inside steps, fall back to **Tree Edit Distance** (TED) on the ASTs using a hybrid approach for scalability and semantic richness.

Tiered strategy:

1. **Scalability and Move Detection (GumTree):**
   * Apply the **GumTree** algorithm. It uses fast, near-linear heuristics (top-down anchoring, bottom-up propagation).
   * GumTree explicitly detects **Move** operations and **Renames** based on subtree similarity, which is critical for large ASTs and refactor-style changes.
2. **Precision Edits (APTED):**
   * For remaining unmatched sub-forests, or if the AST is small (< 2000 nodes), deploy **APTED (All-Path Tree Edit Distance)**.
   * APTED is state-of-the-art for exact TED, guaranteeing O(N^3) time regardless of tree shape and improving on Zhang-Shasha's worst-case behavior.
   * This yields the minimal edit script (Insert, Delete, Rename) for precise logic changes.

This hybrid strategy handles massive generated queries while still producing exact edit scripts for user-written logic.

---

## 11. DAX and Formula Diff Algorithms

DAX and Excel formulas are both expression languages; we can reuse much of the M semantic machinery.

### 11.1 Parsing and Canonicalization

For each formula / measure:

1. Parse into an AST (operators, function calls, references).
2. Canonicalize:
   * Normalize whitespace, casing.
   * Reorder commutative subtrees when safe.
   * Optional: normalize equivalent syntaxes (`AND(a,b)` vs `a && b`).

If canonical ASTs are equal -> no logical change (formatting only).

### 11.2 Expression Diff

For differing ASTs:

1. Run the Hybrid Tree Edit Distance strategy (**GumTree + APTED**, detailed in Section 10.3.3) to identify changed subtrees and moves.
2. Summarize at a human-usable granularity:
   * "Measure `TotalSales` changed aggregation from SUM to AVERAGE."
   * "Filter condition on `Calendar[Year]` changed from `>= 2020` to `>= 2021`."

Implementation detail:

* APTED guarantees stable O(N^3) exact edits for small/medium ASTs; GumTree provides near-linear performance and explicit move detection for larger formulas, so parsing remains the dominant cost.

---

## 12. Metadata Diff Algorithms

Metadata includes:

* Where queries load (sheet vs model vs connection-only).
* Query display folders/groups.
* Table relationships (for the data model).
* Permissions/privacy flags.

Treat metadata as a *typed key-value tree*:

```text
MetadataPath = Vec<String>;  // e.g. ["MQuery", "Section1/Foo", "LoadToSheet"]
MetaValue    = Enum { Bool, Int, String, Enum, Json, ... }
```

Algorithm:

1. Flatten both metadata trees into maps from `MetadataPath` to `MetaValue`.
2. For each path in the union:
   * If value only in A -> `MetadataRemoved`.
   * Only in B -> `MetadataAdded`.
   * Both but unequal -> `MetadataChanged`.

Changes are grouped under logical domains (e.g., query `Foo`'s load destinations), which drives user-facing categories like `MetadataChangedOnly` when the query logic did not change.

---

## 13. Complexity and Performance

### 13.1 Grid Diff

Let `R` = rows. The Hybrid Alignment Pipeline keeps performance near-linear in practice.

* **Anchoring:** O(R log R) via Patience Diff on row signatures.
* **Gap Filling:** Adaptive. Typically near-linear using Myers (small `D`) or Histogram Diff (large gaps). Avoids the Hunt-Szymanski O(R^2 log R) pathology on repetitive data.
* **Move Detection:** LAPJV is O(K^3), where `K` is the number of candidate blocks. Sparse matrix construction keeps `K` small.

### 13.2 Tabular Diff

Keyed diff is O(N) in number of rows plus O(M) per changed row; we never perform global quadratic algorithms on big tables.

### 13.3 M / DAX Diff

* Query alignment: O(Q log Q) for maps and small matching, `Q` = number of queries.
* Step alignment: same DP approach; sequences are short, so cost is negligible.
* AST diff: tiered strategy.
  * **APTED:** Guaranteed O(N^3) time, O(N^2) space. Used for N < 2000 or for unmatched sub-forests where exactness matters.
  * **GumTree:** Near-linear time in practice, O(N^2) worst case. Used to scale to very large ASTs while detecting moves/renames.

The design remains consistent with the product-plan goal: "compare 100MB files in under ~2 seconds" given streaming parsers and native Rust performance.

---

## 14. Implementation Patterns in Rust

### 14.1 Diffable Trait

To keep the engine modular:

```text
trait Diffable {
    type Diff;
    fn diff(&self, other: &Self) -> Self::Diff;
}
```

Implement `Diffable` for:

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

### 14.2 Streaming and Memory

* XML and DataMashup parsing is already designed to be streaming and bounded.
* For huge sheets, compute row signatures on a streaming basis, storing only hashes and basic row metadata until cells must be inspected.

### 14.3 Testability

The testing plan aligns with these abstractions:

* Unit tests exercise `Diffable` implementations (e.g., M query diff kinds).
* Integration tests run end-to-end from real `.xlsx` pairs to JSON diff reports, asserting counts and categories (added/removed/definition vs metadata changes).

This separation keeps the core algorithms independently verifiable while still matching the product-level contracts (CLI, web viewer).

---

## 15. Developer Tools and Reverse Engineering

### 15.1 Binwalk: Recon and Validation

While the spec defines where the OPC package lives, binwalk is useful for:

* **Recon on unknown sections or future versions:**
  * Run binwalk on the raw DataMashup bytes, look for additional embedded ZIPs, zlib streams, etc. This can highlight implementation quirks or vendor extensions.([GitHub][7])
* **Validation:**
  * Confirm that the Package Parts slice starting at `offset = 8` really contains a ZIP signature (`PK\x03\x04`) near the start.
  * Quickly eyeball corrupted or partially-truncated DataMashup streams.

Automation options:

* Call binwalk as a subprocess in a test harness to check slicing.
* Or embed a Rust binwalk-like crate (e.g., `binwalk` on crates.io) to scan for ZIP signatures and validate `PackagePartsLength`.([Crates.io][8])
* During exploration or regression, run binwalk on extracted slices from your parser to keep offsets honest.

### 15.2 Kaitai Struct: Formalizing the Binary Layout

Kaitai is ideal for expressing the **top-level stream** and delegating sub-parsers (ZIP, XML) to other code.([doc.kaitai.io][9])

Conceptual `datamashup.ksy`:

```yaml
meta:
  id: datamashup
  endian: le

seq:
  - id: version
    type: u4
  - id: package_parts_len
    type: u4
  - id: package_parts
    size: package_parts_len
  - id: permissions_len
    type: u4
  - id: permissions
    size: permissions_len
  - id: metadata_len
    type: u4
  - id: metadata
    size: metadata_len
  - id: permission_bindings_len
    type: u4
  - id: permission_bindings
    size: permission_bindings_len

instances:
  is_supported_version:
    value: version == 0
```

Workflow:

* Use the Kaitai Web IDE (`ide.kaitai.io`) to load a sample DataMashup binary and the spec, inspect parsed fields and offsets, and expand as needed.
* Compile to Rust/C#/etc. and wrap with "decode base64 -> feed into Kaitai parser -> get slices -> pass slices to ZIP/XML/DPAPI libraries."
* Iteratively add nested specs if you want tighter validation of Permissions or Metadata headers; XML bodies are easier to parse with an XML library.

### 15.3 Workflow for Unknown Binary Properties

For corners not covered by the spec:

1. **Locate property:** In Metadata XML, look for attributes or text marked as base64 or that appear binary.
2. **Isolate bytes:** Base64-decode that property; save as a standalone `.bin`.
3. **Run binwalk:** If it identifies ZIP/zlib/etc., you know it's a nested container. If not, inspect entropy and patterns (fixed length? recognizable ASCII?).
4. **Define a mini Kaitai spec:** Start with length-prefixed fields, GUIDs, timestamps, etc. Iterate in Web IDE until the hex view and parsed fields line up sensibly.
5. **Codify:** Once understood, add it as a dedicated decoder, but keep the original bytes around for future-proofing.

This property-focused RE loop lets you decode only what the product needs (e.g., load destinations, last refresh schema) while staying resilient to legacy or future files.

---

## 16. End-to-End Processing Summary

End-to-end, the diff identification process is:

1. Parse both workbooks into `Workbook` IR, including `DataMashup` and data model if present.
2. Run:
   * Object graph diff (sheets, tables, queries, measures).
   * For each sheet/table: choose **Database** or **Spreadsheet** mode and run the corresponding alignment algorithm.
   * For each aligned cell/formula/measure: perform AST diff to distinguish formatting vs logic changes.
   * For each M query: run step-aware semantic diff; same for DAX measures.
   * For metadata: run tree diff, using specialized categories like `MetadataChangedOnly`.
3. Aggregate the resulting `DiffOp` stream by scope (Workbook -> Sheet -> Object -> Cell/Query) for presentation and for automated consumers (CLI/JSON, CI integrations).

The result is an engine that:

* Understands Excel files as multi-layered data products, not just grids.
* Uses alignment and AST techniques that are asymptotically efficient and tuned to real-world patterns.
* Lines up with the testing milestones and product roadmap already outlined.

### Quick Reference

If you want one compact mental picture to guide implementation:

* Excel / PBIX host file = **OPC/ZIP**.
* Inside that, **DataMashup** = base64 (Excel) or raw (PBIX) **MS-QDEFF top-level stream**.
* MS-QDEFF top-level stream =
  `Version(=0)` + `len+PackageParts(OPC ZIP)` + `len+Permissions(XML)` + `len+Metadata(XML)` + `len+PermissionBindings(binary)`.
* `PackageParts` contains:
  * `/Config/Package.xml` (who wrote this, culture, versions).
  * `/Formulas/Section1.m` (all M code).
  * `/Content/*` (embedded mini-mashups for `Embedded.Value`).
* `Metadata` glues the M code to workbook/model semantics.
* `PermissionBindings` is a DPAPI-protected hash that you can safely treat as opaque for read-only tools.

Binwalk helps you *find* and *sanity-check* embedded containers; Kaitai helps you *encode the spec as executable schema* and avoid off-by-one bugs. Build the parser as a clean hierarchy with strong invariants at each layer so it can slot straight into the Excel diff engine - even on weird, non-standard, or future files.

---

## 17. Performance Metrics and Limit Handling

### 17.1 DiffMetrics Structure

When the `perf-metrics` feature is enabled, the diff engine collects timing and count metrics:

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

**Deferred metrics**: `parse_time_ms` and `peak_memory_bytes` are planned for future phases
when parser instrumentation and memory allocator integration are ready.

### 17.2 Limit Handling

The engine enforces configurable limits to prevent runaway computation on pathological inputs:

```rust
pub struct DiffConfig {
    pub max_align_rows: u32,      // Default: 500,000
    pub max_align_cols: u32,      // Default: 16,384
    pub max_recursion_depth: u32, // Default: 10
    pub on_limit_exceeded: LimitBehavior,
}

pub enum LimitBehavior {
    FallbackToPositional,  // Use simple positional diff (default)
    ReturnPartialResult,   // Return partial result with warnings
    ReturnError,           // Return structured DiffError
}
```

When limits are exceeded:

- **FallbackToPositional**: Completes the diff using simple cell-by-cell comparison.
  Report is marked `complete = true` with no warnings.
- **ReturnPartialResult**: Completes the diff using positional comparison.
  Report is marked `complete = false` with a warning message including the sheet name.
- **ReturnError**: Returns `DiffError::LimitsExceeded` with sheet name and limit details.
  Legacy API (`diff_workbooks_with_config`) panics; new API (`try_diff_workbooks_with_config`)
  returns the error.

### 17.3 Performance Regression Testing

The project includes CI infrastructure for performance regression:

- **Fixtures P1-P5**: Standard test grids ranging from dense data to repetitive/sparse patterns
- **Perf tests**: `cargo test --features perf-metrics perf_` runs scaled-down versions for CI
- **Threshold script**: `scripts/check_perf_thresholds.py` validates timing against targets
- **CI workflow**: `.github/workflows/perf.yml` runs perf suite on every push/PR

See `docs/meta/logs/2025-12-09-sprint-branch-2/activity_log.md` for implementation details
and intentional spec deviations.

---

Last updated: 2025-12-10

[1]: https://bengribaudo.com/blog/2020/04/22/5198/data-mashup-binary-stream "The Data Mashup Binary Stream: How Power Queries Are Stored | Ben Gribaudo"
[2]: https://community.powerbi.com/t5/Desktop/DataMashup-file-no-longer-exists/td-p/1145141?utm_source=chatgpt.com "DataMashup file no longer exists"
[3]: https://www.thebiccountant.com/2017/10/15/bulk-extracting-power-query-m-code-from-multiple-pbix-files-in-power-bi/?utm_source=chatgpt.com "Bulk-extracting Power Query M-code from multiple pbix files in Power BI"
[4]: https://www.thebiccountant.com/2017/10/15/bulk-extracting-power-query-m-code-from-multiple-pbix-files-in-power-bi/ "Bulk-extracting Power Query M-code from multiple pbix files in Power BI"
[5]: https://bengribaudo.com/tools/datamashupcmdlets?utm_source=chatgpt.com "Data Mashup Cmdlets"
[6]: https://bengribaudo.com/tools/datamashupcmdlets "Data Mashup Cmdlets | Ben Gribaudo"
[7]: https://github.com/ReFirmLabs/binwalk?utm_source=chatgpt.com "ReFirmLabs/binwalk: Firmware Analysis Tool"
[8]: https://crates.io/crates/binwalk?utm_source=chatgpt.com "binwalk - crates.io: Rust Package Registry"
[9]: https://doc.kaitai.io/user_guide.html?utm_source=chatgpt.com "Kaitai Struct User Guide"
[10]: https://bengribaudo.com/blog/2020/06/04/5298/shedding-light-on-the-mysterious-embedded-value "Shedding Light on the Mysterious Embedded.Value | Ben Gribaudo"
[11]: https://radacad.com/exposing-m-code-and-query-metadata-of-power-bi-pbix-file/ "Exposing M Code and Query Metadata of Power BI (PBIX) File - RADACAD"
[12]: https://bengribaudo.com/tools/datamashupexplorer "Data Mashup Explorer | Ben Gribaudo"
