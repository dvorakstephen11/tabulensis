# Verification Report: 2025-12-07-m-ast-equality

## Summary

The branch cleanly implements the planned M AST slice (`core/src/m_ast.rs`), wires it into the public API, adds the `m_formatting_only_*` fixtures, and introduces `core/tests/m7_ast_canonicalization_tests.rs` with coverage for parsing, canonicalization idempotence, formatting-only equality, semantic-change inequality, nested `let` handling, and error paths. All M3–M6 tests remain green. The five findings from the earlier remediation plan (nested `let` bug, undocumented parser scope, `#date` lexing, missing error tests, and missing AST shape checks) are now addressed in code and tests. I did not find any remaining bugs or spec deviations that would block this milestone; the remaining observations are minor, mostly about future-proofing and API/testing depth.

## Recommendation

[x] Proceed to release
[ ] Remediation required

---

## Findings

### 1. Prior remediation items are fully addressed

* **Severity**: Minor (informational)

* **Category**: Gap / Bug (resolved)

* **Description**:
  The earlier verification identified five issues. All of them have concrete fixes in this branch:

  1. **Nested `let` expressions**
     `parse_let` now tracks a separate `let_depth_in_value` counter and only treats a `KeywordIn` as the outer `let`’s `in` when both `depth == 0` and `let_depth_in_value == 0`. This prevents inner `let ... in ...` expressions from prematurely terminating a binding. 
     New tests `nested_let_in_binding_parses_successfully` and `nested_let_formatting_only_equal` verify both parse success and formatting-only equality in the nested case. 

  2. **Parser grammar scope documentation**
     `parse_m_expression` now has a clear doc comment stating that it currently supports top-level `let ... in ...` with simple identifier bindings, preserves non-`let` inputs as opaque token sequences, and treats other constructs on a best-effort basis. 
     This matches the intended “incremental, partial parser” scope described in the mini-spec and remediation plan.

  3. **`#date` / hash-prefixed literal lexing**
     The lexer’s `ch == '#'` branch now recognizes `#` followed by an identifier-start character as a single identifier token like `"#date"`. 
     The test `hash_date_tokenization_is_atomic` asserts that `#"Foo" = #date(2020,1,1)` tokenizes into `Identifier("Foo")`, `'='`, `Identifier("#date")`, `'('`, numbers, commas, and `')'`. 

  4. **Missing tests for error variants**
     New tests directly exercise all `MParseError` variants:

     * `empty_expression_is_error` (whitespace- and comment-only inputs).
     * `unterminated_string_yields_error`.
     * `unterminated_block_comment_yields_error`.
     * `unbalanced_delimiter_yields_error`.
       These confirm correct error reporting without panics.

  5. **AST shape checks for the basic `let` query**
     `MModuleAst` gained `root_kind_for_testing`, returning an `MAstKind` enum (either `Let { binding_count }` or `Sequence { token_count }`). 
     `basic_let_query_ast_is_let` uses this hook to ensure that the query in `one_query.xlsx` parses to a `Let` root with at least one binding. 

* **Evidence**:

  * `core/src/m_ast.rs` – updated `parse_let`, `tokenize`, `MParseError`, `MModuleAst::root_kind_for_testing`, and `parse_m_expression` docs.
  * `core/tests/m7_ast_canonicalization_tests.rs` – new nested-let, error-path, AST-shape, and hash-literal tests.
  * `combined_remediations.md` – original remediation plan, which these changes match.

* **Impact**:
  The previous moderate-severity issues (nested `let` handling and undocumented grammar scope) are now resolved. The AST API is safe to use for real-world Power Query `let` chains, and callers have clearer expectations about parser coverage. No further action required for these items.

---

### 2. AST tests for Example A stop at root-kind, not binding semantics

* **Severity**: Minor

* **Category**: Missing Test

* **Description**:
  The mini-spec’s contract for Example A describes a root `let` node with bindings for `Source` and `#"Changed Type"` and an `in` expression referencing `#"Changed Type"`. 
  The new `basic_let_query_ast_is_let` test asserts only that the root is `MAstKind::Let { binding_count >= 1 }`. It does not validate:

  * The actual binding names (`"Source"`, `#"Changed Type"`).
  * That the `in` expression’s token sequence references the last binding.

  The other tests (`formatting_only_*` and nested `let` equality) confirm structural equality but don’t explicitly pin the expected binding names or `in`-body relationship for the baseline query.

* **Evidence**:

  * `basic_let_query_ast_is_let` only inspects `root_kind_for_testing()` and checks `binding_count >= 1`. 
  * `MExpr::Let` internally carries binding names and body, but these are opaque to tests beyond the binding count. 

* **Impact**:

  * Today, the implementation clearly builds the expected `let` AST, so this is not a correctness bug.
  * A future refactor could accidentally degrade this (e.g., falling back to a `Sequence` for values or changing binding order) while still passing the current root-kind test, slightly weakening the “Example A matches spec narrative” guarantee.

* **Recommendation** (non-blocking):

  * Add an additional test that uses a more introspective view ( even if only in tests, via another helper under `cfg(test)`) to assert that:

    * The first binding is named `"Source"`.
    * The final binding name matches the `in`-body reference tokens.
  * This can be deferred to a future cycle; it’s primarily about locking in expectations for future semantic diff work.

---

### 3. Test-oriented helpers are exposed on a public module

* **Severity**: Minor

* **Category**: Gap

* **Description**:
  The `m_ast` module itself is `pub mod m_ast;` and is re-exported as a module from the crate root. 
  Inside it, there are public, test-oriented helpers:

  * `pub enum MTokenDebug`
  * `pub fn tokenize_for_testing(&str) -> Result<Vec<MTokenDebug>, MParseError>`
  * `pub fn root_kind_for_testing(&self) -> MAstKind` 

  These are documented as “not part of the stable public API,” but they are technically public and reachable by external consumers as `excel_diff::m_ast::MTokenDebug`, etc.

* **Evidence**:

  * Crate root re-exports `MModuleAst`, `MParseError`, `parse_m_expression`, `canonicalize_m_ast`, and `ast_semantically_equal` at the top level, but the entire `m_ast` module is also public, exposing the test utilities.

* **Impact**:

  * There is no functional bug here, and the docs do warn that these are not stable.
  * However, external users may begin to rely on them, making it more expensive to change internal token/AST representations later.

* **Recommendation** (non-blocking):

  * In a future cleanup, consider tightening visibility:

    * Make `MTokenDebug` / `tokenize_for_testing` `pub(crate)` and gate external use behind a feature flag, or
    * Move them into a `#[cfg(test)]`-only module, and keep only `MModuleAst`/`MParseError`/`parse_m_expression` public.
  * For now, the explicit “not part of the stable public API” comment is an adequate warning; this does not block release.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed

  * `core/src/m_ast.rs` implements `MModuleAst`, `MParseError`, `parse_m_expression`, `canonicalize_m_ast`, and `ast_semantically_equal`. 
  * `core/src/lib.rs` re-exports these without changing any existing exports.
  * `Query` and `diff_m_queries` remain unchanged and textual, per M6.

* [x] All specified tests created

  * `parse_basic_let_query_succeeds`.
  * `formatting_only_queries_semantically_equal`.
  * `formatting_only_variant_detects_semantic_change`.
  * `malformed_query_yields_parse_error`.
  * `canonicalization_is_idempotent`.

* [x] Behavioral contract satisfied

  * Example A parses to a `let` AST; canonicalization is idempotent.
  * Formatting-only A vs B compare equal after canonicalization; B vs B_variant compare unequal.
  * Malformed `let` without `in` yields an error (either `MissingInClause` or `InvalidLetBinding`), with no panics.

* [x] No undocumented deviations from spec (documented deviations with rationale are acceptable)

  * The partial-grammar nature of `parse_m_expression` is now explicitly documented at the API level, and the nested-`let` limitation has been removed.

* [x] Error handling adequate

  * All invalid-input paths return `MParseError` variants; lexer and parser avoid panics and unchecked indexing.
  * Tests cover empty input, unterminated string, unterminated block comment, unbalanced delimiters, and malformed `let`. 

* [x] No obvious performance regressions

  * Tokenization and parsing are single-pass over the input with simple stacks and counters; the AST API is used only in tests in this cycle, so it does not affect core diff performance.

---

Given the above, my recommendation is to **proceed to release** this branch. The remaining findings are minor and can be safely deferred to a future cycle focused on expanding the M AST and tightening the public API surface.
