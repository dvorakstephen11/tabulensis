# Verification Report: 2025-12-04c-m6-textual-m-diff

## Summary

The implementation for branch `2025-12-04c-m6-textual-m-diff` cleanly matches the mini-spec: it introduces a dedicated `m_diff` module with `QueryChangeKind`, `MQueryDiff`, and `diff_m_queries`, wires it through `lib.rs`, adds the required M6 fixtures and generator modes, and provides an integration test suite that exercises all behaviors in the plan (add/remove/definition-change/metadata-only/rename/identical). The three earlier remediation findings (ordering determinism, error propagation, and combined definition+metadata classification) have all been addressed with targeted fixtures and tests. I did not find any functional bugs, missing required tests, or spec deviations; remaining observations are minor, documentation- or future-hardening-oriented only.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### Finding 1: Prior remediation items fully addressed

- **Severity**: Minor  
- **Category**: Gap (resolved)  
- **Description**:  
  The previous remediation plan called out three issues:
  1. Deterministic ordering of `diff_m_queries` output was not pinned by tests.
  2. Error propagation from `diff_m_queries` for invalid `Section1.m` syntax was untested.
  3. Classification when both definition and metadata change for the same query was untested. :contentReference[oaicite:0]{index=0}  

  All three have now been explicitly covered:

  - **Ordering determinism**:  
    - Implementation sorts the union of old and new query names lexicographically and dedups before diffing. :contentReference[oaicite:1]{index=1}  
    - Test `multiple_diffs_are_sorted_by_name` constructs a multi-query scenario (Zeta/Bravo vs Alpha/Delta) and asserts both length and exact ordering of the resulting diffs. :contentReference[oaicite:2]{index=2}  
    - The rename test `rename_reports_add_and_remove` no longer re-sorts the result defensively; it asserts the natural order directly (`Section1/Bar` Added, then `Section1/Foo` Removed), relying on the implementation’s determinism. :contentReference[oaicite:3]{index=3}  

  - **Error propagation**:  
    - `diff_m_queries` uses `build_queries(old_dm)?` and `build_queries(new_dm)?`, so any `SectionParseError` bubbles out unchanged. :contentReference[oaicite:4]{index=4}  
    - New test `invalid_section_syntax_propagates_error` uses a helper that clones a real `DataMashup` and replaces `Section1.m` with an invalid `shared` line, then asserts `Err(SectionParseError::InvalidMemberSyntax)` from `diff_m_queries`. :contentReference[oaicite:5]{index=5}  

  - **Definition+metadata change**:  
    - New fixtures `m_def_and_metadata_change_a.xlsx` and `_b.xlsx` are wired into `fixtures/manifest.yaml` and generated via `MashupPermissionsMetadataGenerator` modes.   
    - Test `definition_and_metadata_change_prefers_definitionchanged` loads that pair and asserts exactly one diff for `Section1/Foo` with `kind == QueryChangeKind::DefinitionChanged`, confirming that definition changes dominate over metadata differences. :contentReference[oaicite:7]{index=7}  

- **Evidence**:  
  - `core/src/m_diff.rs` implementation of `diff_m_queries` and `diff_queries`. :contentReference[oaicite:8]{index=8}  
  - `core/tests/m6_textual_m_diff_tests.rs` for the three new tests and updated rename test.   
  - `fixtures/manifest.yaml` and `fixtures/src/generators/mashup.py` for the new M6-related fixtures.   
  - Remediation plan text in `combined_remediations.md`. :contentReference[oaicite:11]{index=11}  

- **Impact**:  
  These changes close the earlier coverage gaps and significantly reduce the risk of regressions in ordering, error handling, and classification logic for the M6 textual M diff. No further action is needed here.

---

### Finding 2: Textual snapshots show syntactic truncations (non-code issue)

- **Severity**: Minor  
- **Category**: Gap (review artefact / docs)  
- **Description**:  
  In the `codebase_context.md` snapshot, some Rust lines that use the `?` operator appear without the final semicolon, e.g.,

  ```rust
  let new_queries = build_queries(new_dm)?
  Ok(diff_queries(&old_queries, &new_queries))

and in `m_section.rs` a similar pattern occurs after `ok_or(SectionParseError::InvalidMemberSyntax)?`.

However, the test results in `cycle_summary.txt` show that the crate compiles and all tests, including those touching these code paths, pass successfully.  This strongly suggests the missing semicolons are an artifact of how `codebase_context.md` was produced (e.g., truncation mid-line), not a real syntax error in the repository.

* **Evidence**:

  * `codebase_context.md` for the truncated lines.
  * `cycle_summary.txt` showing successful `cargo test` runs across lib and test binaries.

* **Impact**:
  No functional impact expected in the actual code. The only practical risk is minor confusion for future reviewers consuming `codebase_context.md` without cross-checking against the real repo.

* **Suggested action (optional, non-blocking)**:

  * Note in internal tooling docs that `codebase_context.md` may truncate lines, and that syntax must be confirmed via the real repo or CI logs.
  * No code changes required.

---

### Finding 3: Error propagation tests cover one variant only (adequate but could be expanded)

* **Severity**: Minor

* **Category**: Missing Test (non-blocking)

* **Description**:
  The mini-spec states that `diff_m_queries` must propagate any `SectionParseError` returned by `build_queries` unchanged. 
  The new test exercises `SectionParseError::InvalidMemberSyntax` specifically, while other variants (`MissingSectionHeader`, `InvalidHeader`) are covered indirectly through lower-level `m_section_splitting_tests.rs` but not via `diff_m_queries` itself.

  Given the implementation simply uses the `?` operator on `build_queries` calls, correctness for all variants follows mechanically, so this is more about belt-and-suspenders coverage than a real gap.

* **Evidence**:

  * Error-handling spec and constraints. 
  * `diff_m_queries` signature and implementation (uses `?`). 
  * Existing tests for `SectionParseError` variants in `m_section_splitting_tests.rs`. 

* **Impact**:
  Very low. If `build_queries` continues to return `SectionParseError` directly, the `?` operator ensures propagation. A regression here would almost certainly be caught by the existing m_section tests and other DataMashup tests.

* **Suggested action (optional, non-blocking)**:

  * Optionally add a second error-propagation test (e.g., for `MissingSectionHeader`) at the `diff_m_queries` layer to make the contract even more explicit.
  * This is not required to ship M6.

---

### Finding 4: Multi-diff ordering test uses synthetic metadata mismatch (intentional, but worth documenting)

* **Severity**: Minor

* **Category**: Gap (test design clarity)

* **Description**:
  The helper `datamashup_with_section` used by `multiple_diffs_are_sorted_by_name` constructs a new `Section1.m` body on top of the `one_query.xlsx` fixture, without adjusting metadata. 

  In this scenario:

  * `parse_section_members` will see the new queries (`Zeta`, `Bravo`, `Alpha`, `Delta`).
  * `build_queries` will attach default metadata to those queries where no matching metadata entry exists, while the original metadata may now include an "orphan" entry for a non-existent query. This is a pattern explicitly tested and supported in `metadata_orphan_entries`. 

  The test only inspects the names and kinds of diffs and does not rely on metadata fields. This is safe and matches intended behavior, but it means the fixture is “slightly unrealistic” (metadata and formulas don’t perfectly line up).

* **Evidence**:

  * `datamashup_with_section` helper in `m6_textual_m_diff_tests.rs`. 
  * `metadata_orphan_entries` test showing that orphan metadata entries are tolerated. 

* **Impact**:
  None on correctness; the code path is well-defined for orphan metadata. However, future readers might momentarily wonder whether misaligned metadata could influence diff classification in this test.

* **Suggested action (optional, non-blocking)**:

  * Add a brief comment to `datamashup_with_section` explaining that it intentionally relies on `build_queries`’ handling of missing/extra metadata and that the test only asserts names/kinds, not metadata-driven behavior.
  * Alternatively, add a future multi-diff scenario via the Python generator for fully realistic metadata. Not necessary to ship this branch.

---

### Overall Assessment of Behavioral Contract vs. Implementation

For clarity, here is a direct mapping of the mini-spec’s behavioral contract to the observed implementation and tests:

1. **Scope (M6.1 + no-rename-detection 6.2)**

   * Implemented `core/src/m_diff.rs` with `QueryChangeKind`, `MQueryDiff`, and `diff_m_queries`. 
   * Exported these via `core/src/lib.rs`. 
   * Did not touch `diff.rs`, `engine.rs`, or JSON/CLI surfacing.
   * Behavior for renames is explicitly codified as `Removed(Foo)` + `Added(Bar)`; the `Renamed` variant is defined but not produced, per spec.

2. **Diff classification rules**

   * Implementation exactly follows the spec’s map-based classification for Added, Removed, DefinitionChanged, and MetadataChangedOnly using `Query.name`, `expression_m`, and `metadata`.
   * Tests:

     * `basic_add_query_diff`: asserts a single `Added` diff for `Section1/Bar`. 
     * `basic_remove_query_diff`: asserts a single `Removed` diff for `Section1/Bar`. 
     * `literal_change_produces_definitionchanged`: asserts `DefinitionChanged` for `Section1/Foo`. 
     * `metadata_change_produces_metadataonly`: asserts `MetadataChangedOnly` for `Section1/Foo`. 
     * `definition_and_metadata_change_prefers_definitionchanged`: asserts that when both expression and metadata change, the classification is `DefinitionChanged`. 
     * `identical_workbooks_produce_no_diffs`: asserts empty diff when comparing a `DataMashup` to itself. 
     * `rename_reports_add_and_remove`: asserts two diffs for rename scenario (`Added(Bar)`, `Removed(Foo)`), matching the current non-rename-detection behavior.

3. **Determinism & complexity**

   * `diff_queries` builds two maps, collects and sorts the union of keys, then iterates that sorted list, yielding O(N log N) behavior and deterministic ordering.
   * `multiple_diffs_are_sorted_by_name` pins ordering in tests. 
   * No additional dependencies were introduced; Cargo.toml remains unchanged aside from previous work.

4. **Error handling**

   * `diff_m_queries` returns `Result<Vec<MQueryDiff>, SectionParseError>` and uses `?` on `build_queries`, so any `SectionParseError` is propagated.
   * New test asserts propagation for invalid member syntax; existing tests cover other `SectionParseError` variants at the parser level.
   * Implementation uses `debug_assert!` only for logically unreachable branches (name missing in both maps). 

5. **Fixtures & generators**

   * `fixtures/manifest.yaml` contains all required M6 scenarios: add/remove/change-literal/metadata-only/def+metadata/rename.
   * `fixtures/src/generators/mashup.py` extends `MashupPermissionsMetadataGenerator` with `m_add_query_*`, `m_remove_query_*`, `m_change_literal_*`, `m_metadata_only_change_*`, `m_def_and_metadata_change_*`, and `m_rename_query_*` modes that produce exactly the behaviors described in the mini-spec.

6. **Integration with the rest of the engine**

   * `Query` and `QueryMetadata` behavior remain as established in M5, including handling of orphan metadata, default metadata for missing entries, and name invariants.
   * The new M diff module remains an internal domain-level API and is not yet wired into `DiffOp`/JSON, exactly as the plan’s "non-goals" specified.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed
* [x] All specified tests created (plus additional remediation tests)
* [x] Behavioral contract satisfied (classification, determinism, error handling)
* [x] No undocumented deviations from spec (only flexible generator choices, which the spec allowed)
* [x] Error handling adequate and covered by unit/integration tests
* [x] No obvious performance regressions; implementation is O(N log N) over query count with small N, and test timing remains fast.

```
