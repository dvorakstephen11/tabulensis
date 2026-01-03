# Verification Report: 2025-12-07b-m7-formatting-no-op

## Summary

This branch successfully wires M AST semantics into `diff_m_queries` as a formatting-only “semantic gate” (M7.1): when query metadata matches and canonical ASTs are equal, textual differences no longer produce `DefinitionChanged`, while all existing M6 behaviors and error modes are preserved. The new logic is confined to `core/src/m_diff.rs`, reuses the existing `m_ast` APIs, keeps `Query`/`DataMashup` unchanged, and is exercised by a dedicated `m7_semantic_m_diff_tests.rs` module plus the existing M6 and AST canonicalization suites, all of which pass.  Documentation has been updated to reflect that M7.1 is implemented at the query layer.  I found no functional bugs or regressions in M query diff behavior; the only discrepancy relative to the mini-spec is that the “AST parse failure” test is implemented at the `diff_m_queries` level rather than directly on the helper function, which is a minor coverage/design deviation rather than a release blocker.  There are no outstanding remediation items for this branch. :contentReference[oaicite:3]{index=3}

## Recommendation

[x] Proceed to release  
[ ] Remediation required  

## Findings

### 1. Semantic gate implementation matches the mini-spec and is well-scoped

- **Severity**: Minor (positive confirmation; not a problem)
- **Category**: Gap (closed) / Spec Alignment
- **Description**:  
  The mini-spec called for extending `diff_m_queries` / `diff_queries` to consult AST semantics when `expression_m` differs, using `parse_m_expression`, `canonicalize_m_ast`, and `ast_semantically_equal`, optionally via an internal helper. :contentReference[oaicite:4]{index=4} The implementation does exactly this:

  * `diff_m_queries` still builds queries via `build_queries` and returns `Result<Vec<MQueryDiff>, SectionParseError>`.   
  * `diff_queries`:
    * Builds `name -> &Query` maps for old/new queries and iterates over the union of names in sorted order (maintaining deterministic ordering).   
    * If `expression_m` strings are equal and metadata differ, emits `MetadataChangedOnly`; if both equal, emits no diff.  
    * If `expression_m` strings differ:
      * When metadata differ, it **does not** consult AST and always emits `DefinitionChanged`.  
      * When metadata are equal, it calls `expressions_semantically_equal` and suppresses the diff (`continue`) only if that returns `true`; otherwise it emits `DefinitionChanged`.   

  * `expressions_semantically_equal`:
    * Attempts to parse both expressions with `parse_m_expression`.  
    * On any `MParseError`, returns `false` (driving textual fallback).  
    * On success, canonicalizes both ASTs and compares via `ast_semantically_equal`.   

  This matches the behavioral contract in the mini-spec, including the “practical rule”: if metadata are identical and canonical ASTs are equal, the query is treated as unchanged despite textual differences. :contentReference[oaicite:9]{index=9}

- **Evidence**:
  - Mini-spec behavioral contract and constraints.   
  - `core/src/m_diff.rs` implementation of `diff_m_queries`, `diff_queries`, and `expressions_semantically_equal`.   
  - `core/src/datamashup.rs` still defines `Query` without embedding ASTs, respecting the scope boundary. :contentReference[oaicite:12]{index=12}  

- **Impact**:  
  This is the desired behavior; there is no issue here. Including it as a “finding” clarifies that the central requirement of this cycle (M7.1 semantic gate) is met, and that the logic is implemented exactly where and how the mini-spec intended.

---

### 2. Formatting-only and negative-control behaviors are correctly enforced

- **Severity**: Minor (positive confirmation; not a problem)
- **Category**: Gap (closed) / Behavioral Contract
- **Description**:  
  The mini-spec required two key behaviors:

  1. **Formatting-only edits** (`m_formatting_only_a.xlsx` vs `m_formatting_only_b.xlsx`) must produce **no diffs** at all. :contentReference[oaicite:13]{index=13}  
  2. A **negative-control variant** (`m_formatting_only_b.xlsx` vs `m_formatting_only_b_variant.xlsx`) with a real semantic change must produce exactly one `DefinitionChanged` for the affected query. :contentReference[oaicite:14]{index=14}  

  The new test module `core/tests/m7_semantic_m_diff_tests.rs` exercises both scenarios:

  * `formatting_only_diff_produces_no_diffs` loads the two formatting-only fixtures, calls `diff_m_queries`, and asserts `diffs.is_empty()`. :contentReference[oaicite:15]{index=15}  
  * `formatting_variant_with_real_change_still_reports_definitionchanged` compares `m_formatting_only_b.xlsx` with `m_formatting_only_b_variant.xlsx` and asserts:
    * `diffs.len() == 1`
    * `diffs[0].name == "Section1/FormatTest"`
    * `diffs[0].kind == QueryChangeKind::DefinitionChanged`. :contentReference[oaicite:16]{index=16}  

  At the AST layer, `m7_ast_canonicalization_tests.rs` already proves that the formatting-only pair canonicalize to equal ASTs, while the variant does not:

  * `formatting_only_queries_semantically_equal` uses `parse_m_expression` + `canonicalize_m_ast` + `ast_semantically_equal` on the queries from `m_formatting_only_{a,b}.xlsx` and asserts equality. :contentReference[oaicite:17]{index=17}  
  * `formatting_only_variant_detects_semantic_change` does the same for `m_formatting_only_b` versus `m_formatting_only_b_variant` and asserts **non-equality**. :contentReference[oaicite:18]{index=18}  

- **Evidence**:
  - Mini-spec behavioral contract for §2.1 and §2.2. :contentReference[oaicite:19]{index=19}  
  - `m7_ast_canonicalization_tests.rs` formatting-only and variant tests.   
  - `m7_semantic_m_diff_tests.rs` formatting-only/variant tests. :contentReference[oaicite:21]{index=21}  
  - Test run showing the new module passing. :contentReference[oaicite:22]{index=22}  

- **Impact**:  
  This validates that the query-level pipeline now behaves as required for the M7.1 milestone and that the AST canonicalization/equality logic is correctly wired into real fixtures.

---

### 3. Existing M6 behaviors are preserved (no regressions)

- **Severity**: Minor (positive confirmation; not a problem)
- **Category**: Gap (closed) / Regression Guard
- **Description**:  
  The mini-spec explicitly forbids regressions for the original M6 textual diff scenarios. :contentReference[oaicite:23]{index=23} The existing `m6_textual_m_diff_tests.rs` suite remains unchanged and all tests pass:

  * `basic_add_query_diff` / `basic_remove_query_diff`: still get a single `Added` or `Removed` diff for the expected query names.   
  * `literal_change_produces_definitionchanged`: still yields exactly one `DefinitionChanged` for `Foo` when a literal changes. :contentReference[oaicite:25]{index=25}  
  * `metadata_change_produces_metadataonly`: metadata-only change still yields `MetadataChangedOnly` for `Foo`. :contentReference[oaicite:26]{index=26}  
  * `definition_and_metadata_change_prefers_definitionchanged`: combined definition+metadata change still produces a single `DefinitionChanged`, not downgraded by AST semantics. :contentReference[oaicite:27]{index=27}  
  * `identical_workbooks_produce_no_diffs`: diffing a `DataMashup` against itself remains empty. :contentReference[oaicite:28]{index=28}  
  * `rename_reports_add_and_remove`: renames are still expressed as one `Added` + one `Removed`, with diff ordering by name preserved.   
  * `multiple_diffs_are_sorted_by_name`: confirms that diff output remains sorted lexicographically by `name`.   
  * `invalid_section_syntax_propagates_error`: malformed section syntax still surfaces as `Err(SectionParseError::InvalidMemberSyntax)`.   

  Because `diff_m_queries` calls the new AST gate **only** when `expression_m` differs and metadata are identical, metadata-only and definition+metadata cases continue to follow the original textual rules. 

- **Evidence**:
  - Mini-spec regression requirements. :contentReference[oaicite:33]{index=33}  
  - `m6_textual_m_diff_tests.rs` and test run showing all 9 tests passing.   
  - `m_diff.rs` gating condition (`old_q.metadata == new_q.metadata` guard). :contentReference[oaicite:35]{index=35}  

- **Impact**:  
  This demonstrates that adding AST semantics did not alter any existing, externally observable M6 behaviors. The regression risk from this cycle is therefore low.

---

### 4. AST error handling and fallback behavior are correct

- **Severity**: Minor
- **Category**: Bug (avoided) / Error Handling
- **Description**:  
  The mini-spec requires that AST integration **must not** introduce new public error variants, must treat parse/canonicalization failures as soft failures, and must fall back to the textual behavior when AST parsing fails. 

  The implementation and tests satisfy this:

  * `diff_m_queries` still returns `Result<Vec<MQueryDiff>, SectionParseError>` and only uses `?` on `build_queries`, so the only possible error remains a `SectionParseError` from section parsing. :contentReference[oaicite:37]{index=37}  
  * `expressions_semantically_equal` handles `MParseError` via pattern matching and returns `false` on any error; it never panics or propagates `MParseError` outward.   
  * `semantic_gate_falls_back_on_ast_parse_failure` in `m7_semantic_m_diff_tests.rs` constructs two `DataMashup` values where the only difference is that one query expression is deliberately malformed. It asserts:
    * `diff_m_queries` returns `Ok(diffs)` (no new error).  
    * Exactly one diff is produced.  
    * That diff is `DefinitionChanged` on `"Section1/Foo"`. :contentReference[oaicite:39]{index=39}  

  At the AST level, `m7_ast_canonicalization_tests.rs` already checks that malformed queries produce the expected `MParseError` variants, but those never escape the diff API. 

- **Evidence**:
  - Mini-spec error/fallback contract. :contentReference[oaicite:41]{index=41}  
  - `m_diff.rs` signature and gating behavior. :contentReference[oaicite:42]{index=42}  
  - Fallback test in `m7_semantic_m_diff_tests.rs`. :contentReference[oaicite:43]{index=43}  
  - AST parse error tests in `m7_ast_canonicalization_tests.rs`. :contentReference[oaicite:44]{index=44}  

- **Impact**:  
  Users will never see new error kinds due to partial AST coverage; malformed or unsupported M expressions are simply treated as “textually changed,” which matches the spec’s fallback requirement.

---

### 5. Performance and architectural constraints are respected

- **Severity**: Minor
- **Category**: Gap (closed) / Performance & Architecture
- **Description**:  
  The mini-spec constrained the change to avoid architectural drift or performance regressions:

  * No changes to `Query` / `DataMashup` domain types. :contentReference[oaicite:45]{index=45}  
  * No ASTs stored in the domain model.  
  * Parse at most once per query pair per side, and only when `expression_m` differs and metadata are identical. :contentReference[oaicite:46]{index=46}  

  The implementation adheres to this:

  * `core/src/datamashup.rs` still defines `Query` as `name`, `section_member`, `expression_m`, and `metadata`; there is no embedded AST or additional fields. :contentReference[oaicite:47]{index=47}  
  * `diff_queries` calls `expressions_semantically_equal` **once** per query name where text differs and metadata are equal; each call parses each side once and canonicalizes once. There is no caching, but also no repeated parsing within a single `diff_m_queries` call for a given pair.   
  * Query alignment still uses name-based `HashMap`s and sorted name lists, identical to M6’s complexity profile.   

  The full test run, including many grid and database alignment tests, shows no failures or signs of performance-related flakiness. :contentReference[oaicite:50]{index=50}

- **Evidence**:
  - Mini-spec constraints & invariants. :contentReference[oaicite:51]{index=51}  
  - `datamashup.rs` and `m_diff.rs` implementations.   
  - Global `cargo test` output. :contentReference[oaicite:53]{index=53}  

- **Impact**:  
  The semantic gate adds AST work exactly where expected and does not alter the data model or diff complexity class. Future performance work can safely build on this behavior.

---

### 6. Test plan deviation: helper-level unit test vs integration-level fallback test

- **Severity**: Minor
- **Category**: Missing Test / Spec Deviation
- **Description**:  
  The mini-spec’s test plan for M7.1 specified that `m7_semantic_m_diff_tests.rs` should include a **unit-level** test for the internal `expressions_semantically_equal` helper, plus optionally an integration-level test that exercises `diff_m_queries` with a malformed expression to ensure fallback. 

  What the implementation provides:

  * A test named `semantic_gate_falls_back_on_ast_parse_failure`, placed in `core/tests/m7_semantic_m_diff_tests.rs`, which operates at the `diff_m_queries` level. It creates synthetic `DataMashup` instances with a malformed and a valid expression, then asserts that a single `DefinitionChanged` diff is produced and that no error is returned. :contentReference[oaicite:55]{index=55}  
  * No direct test calls `expressions_semantically_equal` itself (it is a private helper in `m_diff.rs`, so direct testing would require either moving tests alongside the module or changing visibility). :contentReference[oaicite:56]{index=56}  

  So, conceptually, the “fallback on AST parse failure” behavior **is** tested, but the test operates through the public API rather than directly asserting `expressions_semantically_equal(...) == false` on parse error as the mini-spec sketched.

- **Evidence**:
  - Mini-spec test plan, §5.1 point 4. :contentReference[oaicite:57]{index=57}  
  - `m7_semantic_m_diff_tests.rs` implementation of `semantic_gate_falls_back_on_ast_parse_failure`. :contentReference[oaicite:58]{index=58}  
  - `m_diff.rs` visibility of `expressions_semantically_equal`. :contentReference[oaicite:59]{index=59}  

- **Impact**:  
  This is a small coverage/design deviation, not a functional bug:

  * The integration-level test is arguably *stronger* in terms of user-observable behavior—it proves the public API degrades gracefully on AST parse failure.  
  * What’s missing is very fine-grained, direct verification of the helper’s contract in isolation (e.g., ensuring it doesn’t accidentally return `true` on malformed input under any refactor).

  **Recommendation (non-blocking):**

  * For future refactors, consider either:
    * Moving a small test module into the same crate as `m_diff.rs` to unit-test `expressions_semantically_equal`, or  
    * Adjusting the mini-spec to explicitly describe the current integration-level test as the canonical coverage for this behavior.

  This does **not** need remediation before release; it’s simply a point where spec and implementation style diverged slightly without being documented in the activity log.

---

### 7. Pre-existing doc vs implementation gap: textual diff detail not yet implemented

- **Severity**: Minor
- **Category**: Gap (Documentation / Long-term Spec)
- **Description**:  
  Section 10.2 of `excel_diff_specification.md` describes the “Textual Diff” layer as running a line-level Myers diff and embedding that detail inside `DefinitionChanged`.  In the actual implementation, `QueryChangeKind::DefinitionChanged` is a simple enum variant with no attached textual or structural diff payload, and `diff_m_queries` merely classifies queries into `Added`, `Removed`, `DefinitionChanged`, or `MetadataChangedOnly` without computing or storing a text diff. 

  This gap clearly predates the current branch (the type and behavior were already present in the M6 milestone), and the mini-spec for this cycle deliberately scoped work to “query-level suppression for formatting-only edits,” not to implementing textual or step-aware AST diff details. :contentReference[oaicite:62]{index=62} The current branch only adds semantic gating; it does not regress or change the lack of textual diff detail.

- **Evidence**:
  - Spec description of textual diff detail in §10.2.   
  - `QueryChangeKind` and `MQueryDiff` definitions and current usage.   
  - Mini-spec scope explicitly limiting this cycle to the formatting-only gate. :contentReference[oaicite:65]{index=65}  

- **Impact**:  
  This is not a regression or a problem introduced in this branch, but it is a divergence between the long-term spec and the current implementation. It should be tracked as an outstanding feature for a future “text diff detail” / full M7 milestone, not as a blocker for releasing this specific semantic gate.

---

## Checklist Verification

- [x] All scope items from mini-spec addressed   
- [x] All specified tests created (names and scenarios implemented; one is integration-level rather than helper-level)   
- [x] Behavioral contract satisfied (formatting-only no-op, negative control, M6 regressions, error fallback)   
- [ ] No undocumented deviations from spec (documented deviations with rationale are acceptable)  
  - Unchecked due to the minor but real divergence in how the “AST parse failure” test is implemented (integration-level vs helper-level) relative to the mini-spec text.   
- [x] Error handling adequate (no new public errors; AST failures handled as soft gating failures)   
- [x] No obvious performance regressions (no domain-model changes; AST work only on changed queries with identical metadata; full test suite remains green) 
