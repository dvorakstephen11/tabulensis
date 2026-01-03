# Verification Report: 2025-12-04c-m6-textual-m-diff

## Summary

The branch implements the planned Milestone 6 textual M diff engine as specified: a new `m_diff` module with `QueryChangeKind`, `MQueryDiff`, and `diff_m_queries`; updated `lib.rs` exports; new fixtures and generator modes for the M6 scenarios; and an integration test module `m6_textual_m_diff_tests.rs` that exercises add/remove/definition-change/metadata-only/rename/identical cases end‑to‑end via real Excel files.  The behavior of `diff_m_queries` matches the mini‑spec’s alignment and classification rules, and all tests in the provided cycle summary pass.  I did not find functional bugs in the implemented diff logic, but there are a few test coverage gaps (notably around deterministic ordering and error propagation). These are non‑blocking but worth tightening soon.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

---

## Findings

### 1. Deterministic ordering is not asserted by tests

- **Severity**: Moderate  
- **Category**: Missing Test  
- **Description**:  
  The mini‑spec states that `diff_m_queries` must return a deterministic, **sorted by name** list of `MQueryDiff` values to avoid leaking `HashMap` iteration order into observable behavior.  The implementation does this correctly by building `HashMap<String, &Query>` maps, taking the union of keys, sorting, and deduping before classification. :contentReference[oaicite:3]{index=3}  

  However, the tests do not actually verify this contract. In the rename test, the results are explicitly sorted **in the test** before assertions:

  ```rust
  let mut diffs = diff_m_queries(&dm_a, &dm_b).expect("diff should succeed");
  diffs.sort_by(|a, b| a.name.cmp(&b.name));
  // ... then assert on contents only
  ``` :contentReference[oaicite:4]{index=4}  

  Other tests only exercise single‑diff scenarios, where ordering is trivially satisfied.
- **Evidence**:  
  - Determinism requirement in mini‑spec. :contentReference[oaicite:5]{index=5}  
  - Implementation uses sorted union of names. :contentReference[oaicite:6]{index=6}  
  - `rename_reports_add_and_remove` sorts the vector in the test before checking. :contentReference[oaicite:7]{index=7}
- **Impact**:  
  Today, the implementation is deterministic and sorted, so behavior is correct. But if someone later “simplifies” the code to iterate directly over a `HashMap`’s keys (dropping the sort), all existing tests would still pass, because they either see only one diff or they sort the results in the test. That regression would reintroduce non‑deterministic diff ordering, which is problematic for CLI/JSON consumers and makes diffs noisy and non‑reproducible.

---

### 2. `diff_m_queries` error propagation path is untested

- **Severity**: Moderate  
- **Category**: Missing Test  
- **Description**:  
  The behavioral contract requires that any `SectionParseError` from `build_queries` (e.g., malformed `shared` statements in `Section1.m`) be propagated unchanged from `diff_m_queries`. :contentReference[oaicite:8]{index=8} The implementation calls `build_queries` for old and new mashups and uses `?` to propagate errors, which is correct.   

  The test suite does cover `parse_section_members` error behavior directly via `m_section_splitting_tests.rs` (e.g., `SECTION_INVALID_SHARED` → `SectionParseError::InvalidMemberSyntax`), but there is **no test that exercises this error path through `diff_m_queries` itself**. All `m6_textual_m_diff_tests` use well‑formed fixtures. 
- **Evidence**:  
  - Error‑handling invariant in mini‑spec. :contentReference[oaicite:11]{index=11}  
  - `diff_m_queries` implementation uses `build_queries(old_dm)?` and `build_queries(new_dm)?` and returns `Result<_, SectionParseError>`.   
  - Existing section parser tests only validate `parse_section_members` in isolation. :contentReference[oaicite:13]{index=13}  
  - M6 tests all use valid M fixtures. :contentReference[oaicite:14]{index=14}
- **Impact**:  
  If a future refactor accidentally wraps or swallows `SectionParseError` inside `diff_m_queries` (e.g., mapping it to another error type or returning an empty diff on parse failure), the contract would be broken but current tests would not detect it. Callers who depend on parse errors to signal invalid/corrupt DataMashup streams might then mis‑interpret bad inputs as “no changes.”

---

### 3. No test where both definition and metadata change for the same query

- **Severity**: Minor  
- **Category**: Missing Test  
- **Description**:  
  The spec says: if `expression_m` differs between old and new, the change must be classified as `DefinitionChanged`, **regardless of metadata equality**. If only metadata differs, it is `MetadataChangedOnly`. :contentReference[oaicite:15]{index=15}  

  The implementation follows that rule:

  ```rust
  if old_q.expression_m == new_q.expression_m {
      if old_q.metadata != new_q.metadata {
          // MetadataChangedOnly
      }
  } else {
      // DefinitionChanged
  }
  ``` :contentReference[oaicite:16]{index=16}  

  The tests cover each dimension separately:

  - `literal_change_produces_definitionchanged` – definition change only.   
  - `metadata_change_produces_metadataonly` – metadata change only.   

  There is no fixture or test where both the M code and the metadata change for the **same** query.
- **Evidence**:  
  - Classification rules in mini‑spec. :contentReference[oaicite:19]{index=19}  
  - Implementation’s if/else structure in `diff_queries`. :contentReference[oaicite:20]{index=20}  
  - Existing tests focus on pure definition vs pure metadata changes. :contentReference[oaicite:21]{index=21}
- **Impact**:  
  Current behavior is correct and simple enough that a mistake is unlikely, but there’s no regression test pinning the “definition changes win over metadata changes” rule. A future refactor that, say, refactors the conditions or introduces a combined change type could inadvertently misclassify these cases without being caught by tests. This is low risk but cheap to guard against.

---

# Remediation Plan: 2025-12-04c-m6-textual-m-diff (Non-blocking Improvements)

## Overview

Although the branch is shippable, there are a few non‑blocking gaps around test coverage and review artifacts. This plan describes small, focused follow‑ups to:

1. Lock in the determinism and ordering guarantees of `diff_m_queries`.  
2. Assert the error‑propagation behavior at the public API level.  
3. Add a regression test for concurrent definition+metadata changes.  

These can be done in a short follow‑up cycle without changing the public API.

## Fixes Required

### Fix 1: Assert sorted, deterministic ordering of `MQueryDiff` results

- **Addresses Finding**: 1 (Deterministic ordering not asserted by tests)  
- **Changes**:
  - File: `core/tests/m6_textual_m_diff_tests.rs` :contentReference[oaicite:39]{index=39}  
  - Add a new test that calls `diff_m_queries` for a scenario that produces **multiple** diffs and asserts that the returned list is already sorted by `name` without re‑sorting in the test. For example:
    - Use the existing `m_add_query_a` / `m_add_query_b` pair but make the “old” side have multiple queries and the “new” side differ on more than one name, or
    - Construct a synthetic `DataMashup` with three or more queries and rely on `diff_queries` directly (if you prefer a pure unit test).
  - In `rename_reports_add_and_remove`, remove or relax the defensive `diffs.sort_by(...)` call so that the test actually observes the natural order emitted by `diff_m_queries`. If you still want defensive behavior, add a **separate** ordering test rather than hiding the contract in the rename test.
- **Tests**:
  - New test, e.g.:
    - `multiple_diffs_are_sorted_by_name()`:
      - Arrange inputs that yield at least three diffs.
      - Assert that `diffs.windows(2).all(|w| w[0].name <= w[1].name)`.
  - Update `rename_reports_add_and_remove` to assert the order, or at least to rely on it.

### Fix 2: Add an integration-level error propagation test for `diff_m_queries`

- **Addresses Finding**: 2 (Error propagation path untested)  
- **Changes**:
  - File: `core/tests/m6_textual_m_diff_tests.rs` (or a new small test module dedicated to error cases). :contentReference[oaicite:40]{index=40}  
  - Introduce a test that ensures `diff_m_queries` returns `Err(SectionParseError::InvalidMemberSyntax)` when given a `DataMashup` whose `Section1.m` contains malformed `shared` syntax.
  - You can:
    - Reuse the constant `SECTION_INVALID_SHARED` pattern from `m_section_splitting_tests.rs` and construct a `DataMashup` by:
      - Opening an existing simple mashup fixture (e.g., `one_query.xlsx`) via `open_data_mashup` / `build_data_mashup`.   
      - Cloning the resulting `DataMashup` and replacing `package_parts.main_section.source` with the invalid M text.
    - Or, for simplicity, produce a small dedicated fixture `m_invalid_shared.xlsx` via the `MashupPermissionsMetadataGenerator`, using a mode that writes a known‑bad `Section1.m`.
- **Tests**:
  - New test, e.g.:
    - `invalid_section_syntax_propagates_error()`:
      - Arrange one or both `DataMashup` values with invalid `Section1.m`.
      - Call `diff_m_queries`.
      - Assert `matches!(result, Err(SectionParseError::InvalidMemberSyntax))`.

### Fix 3: Regression test for combined definition + metadata changes

- **Addresses Finding**: 3 (No test for both definition and metadata changing)  
- **Changes**:
  - Files:
    - `fixtures/manifest.yaml`: add a pair of workbooks `m_def_and_metadata_change_a.xlsx` / `m_def_and_metadata_change_b.xlsx` using `mashup:permissions_metadata` (or similar) where:
      - `Foo` changes from `= 1` to `= 2`, **and**
      - load destination changes (e.g., sheet → model).   
    - `fixtures/src/generators/mashup.py`: extend `_scenario_definition` with corresponding modes if needed.   
    - `core/tests/m6_textual_m_diff_tests.rs`: add a test using that fixture pair.
  - The test should assert:
    - Exactly one diff for `"Section1/Foo"`.
    - `kind == QueryChangeKind::DefinitionChanged` (metadata change should not alter the classification).
- **Tests**:
  - New test, e.g.:
    - `definition_and_metadata_change_prefers_definitionchanged()`:
      - Load `m_def_and_metadata_change_a.xlsx` and `_b.xlsx`.
      - Assert a single `MQueryDiff` of `DefinitionChanged` for `Section1/Foo`.

    - Alternatively, add a sanity test that greps for obvious transformation patterns like `0.4]` in the snapshot and fails if they appear.

## Constraints

- All changes should preserve the existing public API:
  - No changes to `QueryChangeKind` variants or `MQueryDiff` fields.
  - No changes to `diff_m_queries` signature.
- Fixtures must remain deterministic and minimal; use existing generator patterns (`MashupPermissionsMetadataGenerator` + manifest wiring) where possible.   
- New tests should be fast and focused (unit or small integration), keeping overall test runtime acceptable.

## Expected Outcome

After completing this remediation work:

- `diff_m_queries`’s deterministic, lexicographically sorted output will be pinned by tests and harder to regress.  
- Error‑handling behavior for malformed `Section1.m` will be explicitly validated at the `diff_m_queries` API boundary.  
- Classification when both M code and metadata change will be guarded by a regression test.  

All of this strengthens confidence in the M6 textual M diff engine without changing its external behavior or public surface.
