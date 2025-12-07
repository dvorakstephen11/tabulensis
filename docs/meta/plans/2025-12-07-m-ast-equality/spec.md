Incremental Milestone: **M7a – M AST parsing & canonicalization equality**

This cycle introduces a first, testable slice of the M parser and canonicalization layer needed for Milestone 7 semantic diffing, without yet changing the external diff behavior (`diff_m_queries` remains textual).

---

## 1. Scope

### 1.1 Modules and types in play

**New module (planned):**

- `core/src/m_ast.rs` (name flexible, but must be clearly M‑AST focused)
  - `pub struct MModuleAst { … }` – root AST node for a single M query expression.
  - `pub enum MParseError { … }` – recoverable parse errors.
  - `pub fn parse_m_expression(source: &str) -> Result<MModuleAst, MParseError>`
  - `pub fn canonicalize_m_ast(ast: &mut MModuleAst)` – normalize away irrelevant differences (at least whitespace/comments).
  - `pub fn ast_semantically_equal(a: &MModuleAst, b: &MModuleAst) -> bool`

**Existing modules referenced but not structurally changed in this cycle:**

- `core/src/datamashup.rs`
  - `DataMashup`, `Query`, `build_data_mashup`, `build_queries`. `Query` stays as currently implemented (no `ast`/`steps` fields) for this cycle. 
- `core/src/m_diff.rs`
  - `MQueryDiff`, `QueryChangeKind`, `diff_m_queries`. Behavior remains textual (M6) for now. 
- `core/src/lib.rs`
  - Re‑export surface may gain new public items for M AST (e.g., `MModuleAst`, `MParseError`, `parse_m_expression`, `ast_semantically_equal`) but must not drop or rename existing exports used by tests. :contentReference[oaicite:15]{index=15}
- Test modules:
  - `core/tests/m6_textual_m_diff_tests.rs` (must remain green). :contentReference[oaicite:16]{index=16}
  - New tests to be added under `core/tests/` for AST parsing and equality (see Test Plan).

**Out of scope (future cycles):**

- Extending `Query` to carry `ast`/`steps`.
- Integrating AST awareness into `diff_m_queries` or `DiffOp`.
- Implementing full `MStep` / `StepKind` / `StepParams` model.
- Hybrid GumTree + APTED semantic diffing.

This cycle is strictly **parser + canonicalization + equality API** plus tests.

---

## 2. Behavioral Contract

The new behavior is defined in terms of the AST API rather than changes to existing diff functions.

### 2.1 Parsing simple M queries

**Example A – basic let/in query**

Input:

```m
let
    Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
    #"Changed Type" = Table.TransformColumnTypes(Source,{{"A", Int64.Type}})
in
    #"Changed Type"
````

Contract:

* `parse_m_expression` returns `Ok(MModuleAst)`; no panic.
* The root AST node represents a `let` expression with:

  * A sequence of bindings for `Source` and `#"Changed Type"`.
  * An `in` expression referencing `#"Changed Type"`.
* Calling `canonicalize_m_ast` on the AST is idempotent: running it twice yields an AST that is structurally equal to the once‑canonicalized version.

A simple unit test may only assert “parses successfully” and that `ast_semantically_equal(ast.clone(), ast)` is true.

### 2.2 Whitespace and comment insensitivity

Take two queries with identical semantics but different formatting:

**A – “ugly” single‑line form:**

```m
let Source=Excel.CurrentWorkbook(){[Name="Table1"]}[Content] in Source
```

**B – nicely formatted with comments:**

```m
let
    // Load the current workbook table
    Source = Excel.CurrentWorkbook(){[Name = "Table1"]}[Content]
in
    Source
```

Contract:

* `parse_m_expression` succeeds for both A and B.
* After calling `canonicalize_m_ast` on both ASTs, `ast_semantically_equal(&ast_a, &ast_b)` **must return true**.
* Any internal canonical form (normalized spacing, comment stripping) is fine as long as equality holds.

This is the foundation for Milestone 7’s “formatter round‑trip is a no‑op” requirement.

### 2.3 Detecting a simple semantic change

Take B above and a variant with a changed identifier:

**B_variant – different table name:**

```m
let
    // Load a different table
    Source = Excel.CurrentWorkbook(){[Name = "Table2"]}[Content]
in
    Source
```

Contract:

* `parse_m_expression` succeeds for `B_variant`.
* After canonicalization, `ast_semantically_equal(&ast_b, &ast_b_variant)` **must return false**.
* The equality function must at minimum distinguish different constant/identifier payloads even when the tree shape is the same.

The tests do **not** require surfacing a detailed edit script yet; they only require a boolean “same vs different” semantic judgment.

### 2.4 Error handling

For clearly malformed M:

```m
let
    Source = 1
// missing 'in' and expression
```

Contract:

* `parse_m_expression` returns `Err(MParseError::... )`.
* No panics, no undefined behavior.
* Canonicalization and equality functions are never called on error values in tests; implementer may rely on type separation for safety.

### 2.5 Stability guarantees for this cycle

* Existing M5/M6 behavior is preserved:

  * `build_queries` emits the same `Query` list for existing fixtures.
  * `diff_m_queries` returns exactly the same `MQueryDiff` sequences for M6 fixtures.
* The new AST types and functions are **pure additions** to the public API; nothing existing is removed or renamed.

---

## 3. Constraints and Invariants

### 3.1 Performance

* M query bodies are typically small (tens–hundreds of tokens), so a straightforward recursive descent or parser‑combinator implementation is acceptable.
* Complexity constraints for this cycle:

  * Parsing: O(N) or O(N²) in expression length is acceptable.
  * Canonicalization: O(N).
* The AST parser must not materially slow down existing flows when used in tests:

  * It will only be called in new test suites, not yet on the main diff path.
  * Future cycles will revisit performance when wiring AST parsing into `build_queries` or diff pipelines.

### 3.2 Memory and streaming

* Parsing operates on in‑memory `&str` slices from `expression_m`; no need for streaming in this milestone.
* No global mutable state; all parser state lives on the stack or within local structures.

### 3.3 Error handling and safety

* No panics on invalid input; all failures reported via `MParseError`.
* `canonicalize_m_ast` must be safe to call multiple times and must not assume any particular source formatting.
* `ast_semantically_equal` must be:

  * Deterministic.
  * Symmetric.
  * Reflexive (equal to itself after canonicalization).

### 3.4 Scope limitations (explicit)

To keep the milestone manageable, **canonicalization only needs to handle:**

* Ignoring whitespace differences.
* Ignoring comments.
* Normalizing trivial formatting variants (e.g., spacing around `=` and commas).

The following are **out of scope** for canonicalization in this cycle (they remain future enhancements aligned with Section 10.3.1 of the spec):

* Treating commutative operations as equal when operands are reordered (e.g., `A + B` vs `B + A`). 
* Reordering independent `let` steps.
* Identifier renaming normalization.
* Advanced AST diffing (GumTree + APTED) beyond boolean equality.

---

## 4. Interfaces

### 4.1 New public API (core crate)

Add the following to the core library:

* In `core/src/m_ast.rs` (or equivalent):

  * `pub struct MModuleAst { /* opaque for now */ }`
  * `pub enum MParseError { /* minimal, user‑showable Debug/Display */ }`
  * `pub fn parse_m_expression(source: &str) -> Result<MModuleAst, MParseError>`
  * `pub fn canonicalize_m_ast(ast: &mut MModuleAst)`
  * `pub fn ast_semantically_equal(a: &MModuleAst, b: &MModuleAst) -> bool`

* In `core/src/lib.rs`:

  * Re‑export the above types/functions:

    * `pub use m_ast::{MModuleAst, MParseError, parse_m_expression, canonicalize_m_ast, ast_semantically_equal};`

### 4.2 Interfaces intentionally **not** changed

* `Query` in `datamashup.rs` remains:

  ```text
  Query {
      name: String,
      section_member: String,
      expression_m: String,
      metadata: QueryMetadata,
  }
  ```

  No `ast` or `steps` fields are added in this cycle.

* `diff_m_queries` remains a pure textual/metadata diff as per M6 (no AST awareness yet).

* `DiffOp` and `DiffReport` are unchanged.

This keeps the milestone narrow and avoids re‑plumbing the main diff pipeline before the AST API is battle‑tested.

---

## 5. Test Plan

All new work must be grounded in explicit tests. This section defines the tests the implementer must create or extend.

### 5.1 New fixtures (Python) for formatting-only scenarios

Extend the fixture generator/manifest (under `fixtures/src/generators/mashup.py` and `fixtures/manifest.yaml`) to add:

1. **`m_formatting_only_{a,b}.xlsx`**

   * Contains a single query `Section1/FormatTest` whose M body is semantically identical between A and B but formatted differently:

     * A: single‑line “ugly” form.
     * B: multi‑line pretty‑printed form with comments and different spacing.
   * No metadata differences between A and B.

2. **`m_formatting_only_{b_variant}.xlsx`**

   * Starts from B and changes a single semantic detail, e.g.:

     * `Name="Table1"` → `Name="Table2"`, or
     * A literal value `= 1` → `= 2`.

Update the manifest with entries like:

```yaml
- id: m_formatting_only
  kind: excel_triple
  a: fixtures/generated/m_formatting_only_a.xlsx
  b: fixtures/generated/m_formatting_only_b.xlsx
  b_variant: fixtures/generated/m_formatting_only_b_variant.xlsx
```

(The exact schema can follow existing patterns; the key is that Rust tests can discover all three paths.)

### 5.2 New Rust tests – AST parsing & canonicalization

Create a new test module, for example:

* `core/tests/m7_ast_canonicalization_tests.rs`

Tests to include:

1. **`parse_basic_let_query_succeeds`**

   * Build a `DataMashup` from an existing simple fixture (e.g., `one_query.xlsx`) and grab `expression_m` for the single query.
   * Call `parse_m_expression(&expr)` and assert `Ok(ast)` (no error).

2. **`formatting_only_queries_semantically_equal`**

   * Helper:

     ```rust
     fn load_query_expression(name: &str) -> String { /* open_data_mashup + build_data_mashup + build_queries */ }
     ```

   * Load `expression_m` from `m_formatting_only_a.xlsx` and `m_formatting_only_b.xlsx`.

   * Parse both to ASTs, canonicalize both, then assert:

     ```rust
     assert!(ast_semantically_equal(&ast_a, &ast_b));
     ```

   * This is the core positive case for M7.1.

3. **`formatting_only_variant_detects_semantic_change`**

   * Load `expression_m` from `m_formatting_only_b.xlsx` and `m_formatting_only_b_variant.xlsx`.

   * Parse + canonicalize like above.

   * Assert:

     ```rust
     assert!(!ast_semantically_equal(&ast_b, &ast_b_variant));
     ```

   * This guards against over‑aggressive canonicalization that erases real changes.

4. **`malformed_query_yields_parse_error`**

   * Use an inline string with an obviously incomplete `let` expression (no `in` clause).
   * Call `parse_m_expression` directly and assert it returns `Err(MParseError::...)`.

5. **`canonicalization_is_idempotent`**

   * Parse a non‑trivial query into `ast`.
   * Clone it, run `canonicalize_m_ast` once on one copy and twice on the other.
   * Assert they are equal under a direct AST equality check (which can be a derived `PartialEq` on the struct).

### 5.3 Existing tests to keep green

* All existing M‑related tests must still pass with no modifications:

  * `core/tests/m4_permissions_metadata_tests.rs`
  * `core/tests/data_mashup_tests.rs`
  * `core/tests/m5_query_domain_tests.rs`
  * `core/tests/m_section_splitting_tests.rs`
  * `core/tests/m6_textual_m_diff_tests.rs`

* This ensures the new parser layer is additive and does not break completed milestones M3–M6.

### 5.4 Milestone mapping

This incremental milestone ties into Phase 4 of the testing plan as a partial fulfillment of Milestone 7:

* Directly advances **M7.1 “formatter round‑trip is a no‑op”** by providing the AST + canonicalization foundation and tests for semantic equality vs inequality.
* It does **not** yet satisfy M7.2–M7.4 (step reordering, step‑aware semantic changes, or GumTree+APTED validation); those will be covered by future cycles once the AST surface is stable.

---

## 6. Summary

This cycle:

* Introduces a dedicated M AST module with parsing, canonicalization, and equality primitives.
* Adds fixtures and tests that distinguish formatting‑only changes from genuine semantic changes at the AST level.
* Leaves the existing textual diff engine (`diff_m_queries`) and `Query` IR unchanged, keeping the MVP M6 behavior stable.
* Provides a narrow but concrete step toward Milestone 7, reducing H3/H4 risk without taking on the full semantic diff problem in one jump.
