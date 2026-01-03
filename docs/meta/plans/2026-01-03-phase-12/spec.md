## Phase 12 implementation plan: full tabular model diffs + DAX-aware semantics

### Phase 12 scope (as written)

Phase 12 is explicitly about two things: (1) expanding the “model diff” from *measures-only* into a real Tabular model diff (tables/columns/types/relationships + semantic comparison of expressions), and (2) making the PBIX “no DataMashup → use DataModelSchema” path fully exercised in end-to-end tests and benchmarks. 

---

## 1) Reality check: what exists today (and what doesn’t)

### 1.1 Model IR exists, but it’s mostly unused

There is already a minimal tabular IR (`Model`) with `measures` and `tables`, and tables have `columns` with an optional `data_type`. 
Right now, nothing actually populates `tables/columns` from PBIX schema parsing—only measures are constructed. 

### 1.2 DataModelSchema parsing is measures-only

`core/src/tabular_schema.rs` parses `DataModelSchema` JSON and extracts only `model.tables[].measures[]` (with a recursive fallback that still only finds measures). It builds `RawTabularModel { measures }` and `build_model()` turns that into a `Model` containing just measures.

### 1.3 Model diff is measures-only and string-hash based

`core/src/model_diff.rs::diff_models()` only compares measures by name, and for definition changes it hashes the raw expression string (no DAX parsing, no semantic classification). 

### 1.4 PBIX “no DataMashup” path exists, but it’s shallow

There are fixtures and tests that cover:

* PBIX with DataMashup produces query ops. 
* PBIX without DataMashup but with DataModelSchema successfully opens (model-diff feature gated). 
* A PBIT pair exists with a model schema that changes measures (SUM → SUMX, plus a new measure).

So the path exists, but it can only talk about measures.

### 1.5 Host/UI parity gaps are real for non-measure model ops

The web viewer currently categorizes ops into `sheetOps`, `queryOps`, and `measureOps` (by `kind.startsWith("Measure")`). Any future “Table/Column/Relationship” model ops would not be displayed unless you update the viewer logic. 

---

## 2) Phase 12 outcomes (definition of done)

### 2.1 Diff coverage outcomes

By end of Phase 12, a diff between two DataModelSchema-bearing artifacts should be able to emit model ops for:

**Tables**

* Added/removed (rename optional but recommended)

**Columns**

* Added/removed
* Data type changes
* Selected property changes (hidden, format string, sort-by, summarize-by)
* Calculated column definition changes (DAX expression)

**Relationships**

* Added/removed
* Key property changes (cross-filter direction/cardinality/isActive when present)

**Measures**

* Added/removed (already)
* Definition changed with semantic classification (formatting-only vs semantic)

This directly implements the “expand beyond measure-level changes” requirement. 

### 2.2 DAX semantic comparison outcomes

When enabled, measure/column expression changes should be classified as:

* Formatting-only: whitespace/casing/comment-only change
* Semantic: AST/canonical form changed
* Unknown: parse failed (fallback behavior)

This satisfies “semantic comparison of expressions.” 

### 2.3 End-to-end grounding outcomes (PBIX DataModelSchema path)

You should have:

* Fixture pairs that change tables/columns/relationships (not only measures)
* Streaming determinism/order tests that assert query ops come before *all* model ops (not just measures)
* A benchmark/perf test that includes “open + parse schema + diff,” aligned with the design recommendation to measure parsing cost end-to-end (PBIX included “where applicable”).

---

## 3) Workstream A: Expand IR and schema parsing to represent real Tabular models

### 3.1 Extend RawTabularModel (schema-side) first

Current raw schema type is:

* `RawTabularModel { measures: Vec<RawMeasure> }`

Phase 12 should evolve it into something like:

* `RawTabularModel { tables: Vec<RawTable>, relationships: Vec<RawRelationship>, measures: Vec<RawMeasure> }`

Where:

* `RawTable` includes `name`, `columns`, and optionally `measures` (you can keep measures top-level too, but table-scoping becomes important as soon as you have columns/relationships).
* `RawColumn` includes at least:

  * `name`
  * `data_type`
  * `is_hidden` (optional)
  * `format_string` (optional)
  * `sort_by` (optional)
  * `summarize_by` (optional)
  * `expression` (optional; calculated columns)
* `RawRelationship` includes:

  * fromTable/fromColumn/toTable/toColumn
  * optional props (crossFilteringBehavior, cardinality, isActive, name)

**Key principle:** keep raw types as `String`/`bool`/`Option`, intern later in `build_model()`, consistent with the current architecture.

### 3.2 Extend Model IR (diff-side) to carry relationships and column props

Current model IR is minimal and lacks relationships. 

Extend under `feature = "model-diff"`:

* Add `Model.relationships: Vec<ModelRelationship>`
* Extend `ModelTable` / `ModelColumn` to include the subset of properties you will diff (don’t add everything; add what you can reliably parse and what matters).
* If you want calculated columns, you need `ModelColumn.expression: Option<StringId>`.

This is still aligned with “minimal IR” intent, just less stubby.

### 3.3 Update `parse_data_model_schema()` to extract tables/columns/relationships

Right now `try_collect_from_model_tables()` only looks at `tables[].measures[]`. 

Phase 12 steps:

1. **Add structured parsing for columns**

   * In `try_collect_from_model_tables()`, for each table:

     * Read `t["columns"]` array when present
     * Parse per-column fields (name, dataType, expression, etc.)
     * Append to `RawTable.columns`

2. **Add structured parsing for relationships**

   * Look for `model["relationships"]` array
   * Parse endpoints (from/to table/column) and relevant properties
   * Append to `RawTabularModel.relationships`

3. **Keep the recursive fallback, but keep it measure-only**

   * The existing fallback (`collect_measures_anywhere`) is a useful “don’t fail on odd schemas” measure safety net.
   * For columns/relationships, don’t attempt an analogous “find anywhere” pass in Phase 12—it’s too ambiguous and will create junk diffs.

4. **Deterministic normalization**

   * Today you normalize by sorting measures and deduping.
   * Extend normalization to:

     * sort tables by name (case-insensitive key)
     * sort columns within tables
     * sort relationships by a stable composite key

This matters because later diff ordering will otherwise depend on string interning order.

### 3.4 Update `build_model()` to populate tables/columns/relationships

Today `build_model()` interns only measure full_name + expression and leaves `Model.tables` empty.

Phase 12 `build_model()` responsibilities:

* Build `ModelTable { name, columns }`
* Build `ModelColumn { name, data_type, … }`
* Build `ModelRelationship { … }`
* Build measures either:

  * as today (`Model.measures`), or
  * nested under tables (recommended long term), but if you keep top-level measures, also include enough table context in their IDs/op fields to render nicely.

---

## 4) Workstream B: Expand DiffOp for model objects and wire kinds everywhere

### 4.1 Add new DiffOp variants

Today DiffOp already supports `MeasureAdded/Removed/DefinitionChanged` (model-diff gated).

Add new variants (also behind `model-diff`):

**Tables**

* `TableAdded { name }`
* `TableRemoved { name }`
* (optional) `TableRenamed { from, to }`

**Columns**

* `ModelColumnAdded { table, name, data_type }`
* `ModelColumnRemoved { table, name }`
* `ModelColumnTypeChanged { table, name, old_type, new_type }`
* `ModelColumnPropertyChanged { table, name, field, old, new }`
* `CalculatedColumnDefinitionChanged { table, name, change_kind, old_hash, new_hash }`

**Relationships**

* `RelationshipAdded { from_table, from_column, to_table, to_column }`
* `RelationshipRemoved { … }`
* `RelationshipPropertyChanged { … }`

Keep each op “small and countable,” consistent with existing diff ops patterns.

### 4.2 Update kind mapping (`diff_op_kind`) and op classification

The system already has a `diff_op_kind()` mapping used by some consumers. It currently knows about Measure* kinds. 

Phase 12 needs:

* New kind strings for new variants
* A generalized helper in core or in consumers:

  * `is_model_op()` rather than special-casing `startsWith("Measure")`

### 4.3 Update consumers that assume “Measure is the only model thing”

At minimum:

**Web viewer**

* `categorizeOps()` currently recognizes measure ops only via prefix. 
  Update to group `Table*`, `ModelColumn*`, `Relationship*`, and `Measure*` under a unified “Model” section (or split by subtype, but don’t drop them on the floor).

**CLI**

* CLI already has a measure section and `is_measure_op()` routing.
  Add:
* A “Model” section and rendering for table/column/relationship ops
* Counts broken down by model category (or a single “model changes” count)

**String-ID coverage tests**

* There’s explicit logic to collect string IDs from ops; it currently includes measure ops but won’t include new model ops unless updated. 

---

## 5) Workstream C: Implement real tabular diffing (tables/columns/relationships)

### 5.1 Refactor `diff_models()` into sub-diffs

Right now it’s measure-only. 

Refactor plan:

* `diff_tables(old, new, pool) -> Vec<DiffOp>`
* `diff_columns(old_table, new_table, pool) -> Vec<DiffOp>`
* `diff_relationships(old, new, pool) -> Vec<DiffOp>`
* `diff_measures(old, new, pool, config) -> Vec<DiffOp>` (see DAX section)

And then `diff_models()` orchestrates and concatenates in a stable ordering (e.g., Tables → Columns → Relationships → Measures).

### 5.2 Use stable, rename-resistant matching keys (case-insensitive)

Elsewhere in the codebase, case-insensitive keys are used to avoid spurious diffs on casing changes. 
Do the same for:

* table names
* column names within table
* relationship endpoint names
* measure names

This also sets you up to add rename detection later.

### 5.3 Column diff semantics

At a minimum, emit:

* Added/removed columns
* Type change ops (`old_type != new_type`)
* Property change ops for the limited set you parsed (hidden, format, sort-by, summarize-by)
* For calculated columns: definition changed ops (and later DAX semantic classification)

### 5.4 Relationship diff semantics

Relationships are best keyed by endpoint tuple:

* (fromTable, fromColumn, toTable, toColumn)
  If the schema includes a stable relationship name/id, you can incorporate it, but endpoint tuple is a good minimal key.

Emit:

* Added/removed relationships
* Property changes for cross-filter/cardinality/isActive when present

### 5.5 Guard against op explosions (practical necessity)

Once you diff columns, large models can produce huge op lists. Right now PBIX diff doesn’t run through the same “hardening”/max-ops machinery that grid diff does, and PBIX streaming tests assume a straightforward op emission path.

Phase 12 should introduce a containment strategy for model ops, e.g.:

* A per-model-op cap (reuse `DiffConfig` hardening max ops, or add a model-specific cap)
* If the cap is hit:

  * stop emitting further model ops
  * mark `DiffReport.complete = false`
  * add a warning (DiffReport has warnings/completeness fields already, and design eval explicitly calls output evolution future-friendly).

---

## 6) Workstream D: DAX semantic comparison for measure/calculated column expressions

### 6.1 Anchor this in existing semantics patterns in the repo

You already have “semantic diff” concepts:

* M semantic diff produces structured semantic detail and change kinds. 
* Semantic toggles live in `SemanticConfig` (`enable_m_semantic_diff`, `enable_formula_semantic_diff`, `semantic_noise_policy`).
* Design eval explicitly praises “semantics configurable” as the way to manage depth vs perf. 

Phase 12 should treat DAX the same way: configurable, safe fallback, deterministic.

### 6.2 Add config flag: `enable_dax_semantic_diff`

Extend `SemanticConfig` with:

* `enable_dax_semantic_diff: bool` (default off initially)

Reuse `semantic_noise_policy` to optionally suppress formatting-only changes (just like you’d want for M).

### 6.3 Implement a minimal DAX parser + canonicalizer (goal: “ignore formatting reliably”)

You do *not* need a full DAX engine for Phase 12. You need a parser sufficient to create a stable AST for common expressions.

**Minimal grammar coverage (practical)**

* Literals: numbers, strings
* Identifiers
* Function calls: `NAME(arg, …)`
* Binary ops: `+ - * / ^ & = <> < > <= >= && ||`
* Unary ops: `- + NOT`
* Parentheses
* References:

  * Table/column: `Sales[Amount]` / `'Sales Table'[Amount]`
  * Measures: `[Total Sales]`
* VAR/RETURN blocks (very common in real measures)

**Canonicalization**

* normalize whitespace by parsing
* normalize keyword/function casing
* optionally normalize identifier casing (table/column names are typically case-insensitive in practice)

**Outputs**

* `semantic_hash(expr: &str) -> Result<u64, ParseError>`
* `change_kind(old_expr, new_expr) -> {FormattingOnly|Semantic|Unknown}`

No interning is required; keep it local and return hashes + classification.

### 6.4 Wire DAX semantics into model diff ops

Today `MeasureDefinitionChanged` includes only `old_hash/new_hash` and is based on raw expression strings. 

Phase 12 implementation approach:

* Update `diff_models()` signature to accept `config` (or at least `SemanticConfig`), because today it can’t decide semantic behavior. 
* On expression change:

  * If DAX semantic diff enabled:

    * parse both, compute semantic hashes, decide `change_kind`
    * if formatting-only and policy suppresses: emit nothing
    * otherwise emit `MeasureDefinitionChanged` with `change_kind` (add field or new op)
  * If disabled or parse fails:

    * behave like today, but you may want `change_kind = Unknown` when parse fails (so consumers know it fell back)

Apply the same approach to calculated columns once you store `ModelColumn.expression`.

---

## 7) Workstream E: Make PBIX “no DataMashup → schema” path fully end-to-end (fixtures, tests, benchmarks)

This is the second explicit bullet in Phase 12. 

### 7.1 Add fixtures that change more than measures

Your fixture generator already supports embedding a raw `model_schema` JSON in “no_datamashup” PBIX/PBIT generation.

Add new fixture pairs similar to existing `pbit_model_a/b`, but include:

* tables + columns
* relationships
* calculated columns

Example fixture themes (each should have an “A” and “B”):

1. Column add/remove
2. Column type change
3. Relationship add/remove/change
4. Calculated column expression formatting-only change
5. Calculated column expression semantic change

These will directly exercise the new parsing + diff ops.

### 7.2 Update streaming order test from “query before measure” to “query before model”

There’s already a unit test ensuring query ops are emitted before measure ops in PBIX streaming diff.

Once you add table/column/relationship ops, change the invariant to:

* No `op.is_m_op()` may appear after any `op.is_model_op()`

This preserves the “M first, model second” ordering while generalizing it.

### 7.3 Add PBIX end-to-end perf/benchmark coverage that includes schema parse

Design evaluation calls out that benchmark data can under-represent parsing cost and recommends end-to-end “open + parse + diff” benchmarks, including PBIX where applicable.

Phase 12 should add at least one of:

* A perf/integration test that opens PBIX/PBIT from disk, diffs, and records/validates metrics (if you extend PBIX diff to report metrics).
* A Criterion benchmark that runs “open + parse + diff” on a schema-only PBIT pair, using in-memory bytes (Cursor) to reduce filesystem noise.

Even if you don’t fully wire PBIX into `DiffMetrics` in Phase 12, you can still create a benchmark harness that measures wall-clock time for `PbixPackage::open` + `.diff()` and reports it.

---

## 8) Workstream F: Host parity (CLI + web viewer) so users actually see the value

### 8.1 Web viewer changes

Update categorization so new model ops appear (not silently ignored). Today it’s measure-only detection. 

Recommended UI grouping:

* Queries (M)
* Model (Tables/Columns/Relationships/Measures)
* Sheet/grid ops

### 8.2 CLI output changes

CLI already prints and counts measures separately.
Phase 12 should add:

* A “Model” section (tables/columns/relationships)
* Model op formatting in both text and git-diff modes
* Counts updated accordingly

---

## 9) Suggested sequencing (keeps the codebase green as you land changes)

1. **Schema parsing + IR expansion**

* Extend RawTabularModel → parse tables/columns/relationships
* Extend Model → build those structures
* Unit tests for parsing/building

2. **DiffOp extensions + consumer plumbing**

* Add new op variants + kind mapping
* Update viewer/CLI so new ops are visible (even before diff logic fully emits them)

3. **Diff logic for tables/columns/relationships**

* Implement diff_tables/diff_columns/diff_relationships
* Fixture-backed integration tests

4. **DAX semantic layer**

* Add config toggle
* Add parser/canonicalization + change_kind
* Wire into measure + calculated column diffs
* Add formatting-only suppression tests

5. **PBIX end-to-end grounding**

* New schema-only fixtures
* Updated streaming ordering test
* New e2e benchmark/perf test for PBIX/PBIT open+diff

---

## 10) Final acceptance checklist for Phase 12

* [ ] DataModelSchema parser populates tables/columns/relationships (not just measures).
* [ ] DiffOps exist for table/column/relationship changes (and are mapped to kind strings). 
* [ ] Model diff emits those ops deterministically and in stable order.
* [ ] DAX semantic diff can classify formatting-only vs semantic changes (configurable, safe fallback).
* [ ] PBIX “no DataMashup” path has fixture pairs that cover all new model op types and has at least one end-to-end benchmark/perf test.
* [ ] CLI + web viewer display the new model ops (no “invisible diffs”).

