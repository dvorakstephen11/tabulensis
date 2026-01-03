Milestone progress toward **M7 – Semantic (AST) M diffing**, focused on the **formatting-only semantic gate (M7.1)**: use canonical AST equality to suppress `DefinitionChanged` for whitespace/comment-only edits, without yet implementing full step-aware or GumTree/APTED diff.

---

## 1. Scope

### 1.1 Modules in play

**Primary**

- `core/src/m_diff.rs`
  - Extend `diff_m_queries` / `diff_queries` to consult AST semantics when `expression_m` text differs.
- `core/src/m_ast.rs`
  - Reuse `parse_m_expression`, `canonicalize_m_ast`, and `ast_semantically_equal`.
  - Optionally add a small helper for “best-effort semantic equality” on raw `&str` expressions.
- `core/tests/m6_textual_m_diff_tests.rs`
  - Regression guard: existing M6 behaviors must remain unchanged.
- `core/tests/m7_ast_canonicalization_tests.rs`
  - Reference for canonicalization/equality behavior and fixtures; no behavior changes required.

**Tests / fixtures**

- New test module: `core/tests/m7_semantic_m_diff_tests.rs`.
- Existing fixtures:
  - `fixtures/generated/m_formatting_only_a.xlsx`
  - `fixtures/generated/m_formatting_only_b.xlsx`
  - `fixtures/generated/m_formatting_only_b_variant.xlsx` :contentReference[oaicite:12]{index=12}

### 1.2 Out of scope for this cycle

- No changes to:
  - `Query` / `DataMashup` domain types (`core/src/datamashup.rs` stays as-is). :contentReference[oaicite:13]{index=13}
  - Step-level `MStep` representation or any new structured “semantic diff” output format.
  - GumTree/APTED or other tree-edit-distance algorithms.
- No attempt to detect:
  - Query renames (still expressed as Added + Removed).
  - Reordering of independent steps (M7.2).
  - Specific semantic changes such as filter/column modifications (M7.3).

---

## 2. Behavioral Contract

All examples refer to the public API:

```rust
pub fn diff_m_queries(
    old_dm: &DataMashup,
    new_dm: &DataMashup,
) -> Result<Vec<MQueryDiff>, SectionParseError>;
````

and the existing `QueryChangeKind` enum.

### 2.1 Formatting-only edits: no definition change

For workbook fixtures:

* `m_formatting_only_a.xlsx`
* `m_formatting_only_b.xlsx`

Assumptions (from generators/spec):

* They contain the same queries (same `name` / `Section1/MemberName`).
* Each matched query has:

  * Identical `QueryMetadata`.
  * `expression_m` strings that differ only in whitespace, indentation, and/or comments.
  * Canonicalized ASTs that compare equal.

**Contract**

* `diff_m_queries(&dm_a, &dm_b)` must yield **no diffs**:

  ```rust
  let diffs = diff_m_queries(&dm_a, &dm_b)?;
  assert!(diffs.is_empty());
  ```

* This applies to all queries in the pair; there must be no `DefinitionChanged`, `MetadataChangedOnly`, `Added`, or `Removed` for these scenarios.

Practical rule:

> If metadata is identical and canonical ASTs are equal, treat the query definition as unchanged even when `expression_m` text differs.

### 2.2 Negative control: semantic change still surfaces

For:

* `m_formatting_only_b.xlsx`
* `m_formatting_only_b_variant.xlsx`

Assumptions:

* Same query names and metadata.
* `expression_m` differs by a **real semantic change** (e.g., identifier tweak or load-target change) so canonicalized ASTs are **not** equal.

**Contract**

* `diff_m_queries(&dm_b, &dm_b_variant)` returns **exactly one** `MQueryDiff` for the changed query:

  * `name` points at the query (e.g., `"Section1/Foo"`).
  * `kind == QueryChangeKind::DefinitionChanged`.

* No additional spurious diffs (no false Added/Removed, no MetadataChangedOnly).

### 2.3 Existing M6 behaviors remain unchanged

The new semantic gate must not alter the observable behavior of existing M6 scenarios. These regressions are forbidden:

* `m_change_literal_{a,b}.xlsx`:

  * Must still yield exactly one `DefinitionChanged` for `Foo`.
* `m_metadata_only_change_{a,b}.xlsx`:

  * Must still yield exactly one `MetadataChangedOnly` for `Foo`. AST is not consulted when `expression_m` is identical.
* `m_def_and_metadata_change_{a,b}.xlsx`:

  * **Definition+metadata change continues to prefer `DefinitionChanged`.**
  * AST semantics must not downgrade or hide a `DefinitionChanged` when metadata also changed.
* `m_add_query`, `m_remove_query`, `m_rename_query`:

  * Added/Removed classification and name-sorted ordering remain exactly as in `m6_textual_m_diff_tests.rs`. 
* `identical_workbooks_produce_no_diffs`:

  * Diff of a `DataMashup` against itself stays empty; semantic logic does not introduce any new diffs.

### 2.4 Error and fallback behavior

New behavior for AST integration:

* If either side’s `expression_m` fails AST parsing or canonicalization for any reason (`MParseError`), `diff_m_queries`:

  * **Must not** change its error type; it must still return `Err(SectionParseError::...)` only when `build_queries` fails.
  * Treats the query as **semantically unequal** for gating purposes and falls back to the existing **textual behavior**:

    * If metadata differs → `MetadataChangedOnly` or `DefinitionChanged` as today.
    * If metadata identical and text differs → `DefinitionChanged`.
* AST failures must not panic or short-circuit diffing of other queries; they are localized to the affected query pair.

---

## 3. Constraints and Invariants

### 3.1 Performance and complexity

* Complexity:

  * Let `N` be the number of queries and `L` the average expression length.
  * Current M6 path is `O(N * L)` for string operations.
  * The new semantic gate adds AST parsing only for queries where `expression_m` differs and metadata is identical.
  * Expected complexity remains effectively linear in practice:

    * Per changed query: `parse_m_expression` + `canonicalize_m_ast` on each side, followed by an equality check. 

* Budgets:

  * No additional allocations or data structures must be introduced at `DataMashup` or `Query` level (no ASTs stored in the domain model yet).
  * The implementer should avoid repeated parsing of the same expression within a single `diff_m_queries` call; per query pair, parse at most once per side.

### 3.2 Safety and robustness

* `diff_m_queries` remains **total** for well-formed `DataMashup` inputs; AST integration must not introduce new panics or unwraps on user data.
* All AST-related failures are treated as **soft failures** for semantic gating and never escape as new error variants.
* Behaviour in the presence of malformed `Section1.m` remains as in M6:

  * Section syntax errors still surface via `SectionParseError::InvalidMemberSyntax`. 

### 3.3 Determinism

* `diff_m_queries` output remains **deterministically sorted by `name`** for all scenarios (including new ones).
* Semantic gating must not change ordering: if a diff is emitted, it appears in the same position it would have occupied in the M6 ordering.

---

## 4. Interfaces

### 4.1 Public APIs that must stay stable

* `pub fn diff_m_queries(old_dm: &DataMashup, new_dm: &DataMashup) -> Result<Vec<MQueryDiff>, SectionParseError>;`
* `pub enum QueryChangeKind { Added, Removed, Renamed { .. }, DefinitionChanged, MetadataChangedOnly }`
* `pub struct MQueryDiff { pub name: String, pub kind: QueryChangeKind }`
* Existing `m_ast` exports:

  ```rust
  pub use m_ast::{
      MModuleAst,
      MParseError,
      ast_semantically_equal,
      canonicalize_m_ast,
      parse_m_expression,
  };
  ```

No signature changes or new public enum variants in this cycle.

### 4.2 Internal helpers that may be added

Permitted internal additions (non-public, module-private):

* In `core/src/m_diff.rs`:

  ```rust
  fn expressions_semantically_equal(old_expr: &str, new_expr: &str) -> bool { /* best-effort */ }
  ```

  * Implementation:

    * Attempts to parse both via `parse_m_expression`.
    * Runs `canonicalize_m_ast` on both trees.
    * Compares via `ast_semantically_equal` or an equivalent deep comparison.
    * On any `MParseError`, returns `false` (triggering textual fallback).
* Small, test-only helpers may be added to the new test module (e.g., fixture loaders), matching the style in `m6_textual_m_diff_tests.rs`.

These helpers must **not** be re-exported from `lib.rs` in this cycle.

---

## 5. Test Plan

All new behavior must be expressed through explicit tests.

### 5.1 New test module: m7_semantic_m_diff_tests.rs

Create `core/tests/m7_semantic_m_diff_tests.rs` with tests along these lines:

1. **`formatting_only_diff_produces_no_diffs`**

   * Load DataMashup for `m_formatting_only_a.xlsx` and `m_formatting_only_b.xlsx` via `open_data_mashup` → `build_data_mashup`.
   * Call `diff_m_queries`.
   * Assert:

     ```rust
     assert!(diffs.is_empty(), "formatting-only changes should be ignored");
     ```

2. **`formatting_variant_with_real_change_still_reports_definitionchanged`**

   * Load `m_formatting_only_b.xlsx` and `m_formatting_only_b_variant.xlsx`.
   * Call `diff_m_queries`.
   * Assert:

     * `diffs.len() == 1`.
     * `diffs[0].kind == QueryChangeKind::DefinitionChanged`.
     * `diffs[0].name` matches the expected query (e.g., `"Section1/Foo"`), consistent with the fixtures.

3. **`semantic_gate_does_not_mask_metadata_only_or_definition_plus_metadata_changes`**

   * Reuse existing M6 fixtures:

     * `m_metadata_only_change_{a,b}.xlsx`
     * `m_def_and_metadata_change_{a,b}.xlsx`

   * For each pair:

     * Call `diff_m_queries`.
     * Assert the existing expectations:

       * Exactly one `MetadataChangedOnly` for the metadata-only pair.
       * Exactly one `DefinitionChanged` for the “definition + metadata” pair.

   * These tests ensure the semantic gate is only applied when metadata is identical.

4. **`semantic_gate_falls_back_on_ast_parse_failure` (unit-level)**

   * Add a small unit test that exercises the internal `expressions_semantically_equal` helper (if introduced):

     * Choose two expressions where at least one triggers `MParseError` (e.g., a deliberately malformed M expression already used in `m7_ast_canonicalization_tests.rs`).
     * Assert `expressions_semantically_equal(...) == false`.
   * Optionally, construct a synthetic `DataMashup` with an expression known to be outside the current AST subset and verify that `diff_m_queries` still returns `DefinitionChanged` rather than erroring.

### 5.2 Existing tests to keep / rerun

The implementer must ensure all the following continue to pass without modification:

* `core/tests/m6_textual_m_diff_tests.rs`:

  * `basic_add_query_diff`
  * `basic_remove_query_diff`
  * `literal_change_produces_definitionchanged`
  * `metadata_change_produces_metadataonly`
  * `definition_and_metadata_change_prefers_definitionchanged`
  * `identical_workbooks_produce_no_diffs`
  * `rename_reports_add_and_remove`
  * `multiple_diffs_are_sorted_by_name`
  * `invalid_section_syntax_propagates_error`
* `core/tests/m7_ast_canonicalization_tests.rs`:

  * All existing tokenization, parsing, canonicalization, and equality tests must remain green.

### 5.3 Fixtures

* **Reuse only; no new fixtures required in this cycle.**
* Ensure `fixtures/manifest.yaml` entries for:

  * `m_formatting_only_a`
  * `m_formatting_only_b`
  * `m_formatting_only_b_variant`

  remain correct and are used by the new tests.

### 5.4 Documentation touchpoints

As part of the implementation cycle, the implementer should:

* Update `docs/rust_docs/excel_diff_testing_plan.md` to mark **M7.1 (formatting-only semantic gate)** as implemented at the “query-level suppression” layer (semantic equality used to suppress `DefinitionChanged`), with step-aware diff still pending.
* If necessary, add a short note to `docs/rust_docs/excel_diff_specification.md` §10.3.1 clarifying that the current implementation uses canonical AST equality solely to suppress formatting-only changes in the `MQueryDiff` layer and does not yet expose a structured semantic diff.

---

## 6. Linked Milestones

* Primary: **M7 – Semantic (AST) M diffing**, specifically **M7.1 “Formatting-only changes”**.
* Secondary: supports future M7.2–M7.4 work by:

  * Exercising AST parsing and canonicalization on real DataMashup fixtures.
  * Establishing the semantic-gating behavior that step-aware and tree-edit-distance layers will build on.
