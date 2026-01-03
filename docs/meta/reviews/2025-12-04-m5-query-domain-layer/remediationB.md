# Remediation Plan: 2025-12-04-m5-query-domain-layer

## Overview

The implementation of the `Query` domain layer and metadata join is functionally sound and matches the behavior described in the remediation (synthesize default metadata for queries missing metadata). The remaining work is primarily *alignment* work: updating written specs/tests plans to match the implemented behavior and tightening coverage around a few documented guarantees (error variants, ordering, and quoted identifiers).

These fixes are not blocking for release but are recommended to avoid confusion and prevent future regressions.

## Fixes Required

### Fix 1: Align invariants and behavior in specs with metadata-synthesis policy

- **Addresses Finding**:  
  *Finding 1 – Spec invariants are out of date after metadata‑synthesis change.*

- **Changes**:  
  1. **Mini‑spec (`cycle_plan.md`, Query section)**  
     - In §2.2.3 “Join members to metadata”:
       * Replace the decision bullet that currently says “do **not** emit a `Query` without metadata” with text that states:
         * Queries are always emitted for shared Section1 members.
         * When no metadata entry exists, `build_queries` synthesizes a default `QueryMetadata`:
           * `item_path = "{SectionName}/{MemberName}"`
           * `load_to_sheet = false`, `load_to_model = false`, `is_connection_only = true`, `group_path = None`.
     - In §3.3 “Behavioral invariants”:
       * Remove or relax `queries.len() <= dm.metadata.formulas.len()`; explicitly state that synthetic queries are possible in the presence of incomplete metadata.
       * Clarify that “Every `Query` has a `QueryMetadata`” now means “every `Query` has a metadata *struct*; some may be synthesized rather than backed by a concrete metadata XML entry.”

  2. **Global testing/spec docs**  
     - In `excel_diff_testing_plan.md` and `excel_diff_specification.md` (Milestone 5 / Metadata invariants sections), update any language that assumes a strict 1‑to‑1 mapping between metadata entries and queries so it acknowledges synthetic metadata in inconsistent workbooks.   

- **Tests**:  
  *No new tests required for this change*, because `member_without_metadata_is_preserved` already locks in the behavior. You may optionally strengthen that test with a comment or a small assertion that `dm.metadata.formulas.len() == 0` to make the “synthetic metadata” scenario obvious to future readers (the fixture already encodes exactly that).   

---

### Fix 2: Decide and document policy on invalid `shared` member syntax

- **Addresses Finding**:  
  *Finding 2 – Invalid member syntax is silently ignored; `InvalidMemberSyntax` is never used.*

- **Changes**:  
  1. **Clarify intended behavior** (spec-level decision):
     - Either:
       * **Option A**: Keep the current behavior (silently skip malformed `shared` lines) and **update the spec** to say that invalid member syntax is tolerated and ignored, and that `InvalidMemberSyntax` is not used in this milestone; or
       * **Option B**: Change `parse_section_members` so that when `parse_shared_member` returns `None` for a line starting with `shared`, it returns `Err(SectionParseError::InvalidMemberSyntax)`.
  2. **Documentation updates**:
     - In §3.2 “Error handling” of the mini‑spec, rewrite the bullet about “Invalid member syntax” to match the chosen behavior (A: ignored; B: error).   

- **Tests**:  
  *If you choose Option B (fail on invalid syntax):*
  - Add a new unit test in `core/tests/m_section_splitting_tests.rs`, e.g.:
    - Section with a malformed `shared` line (missing `=` or identifier).
    - Assert `parse_section_members` returns `Err(SectionParseError::InvalidMemberSyntax)`. :contentReference[oaicite:39]{index=39}  

  *If you choose Option A (skip invalid syntax):*
  - Add a test that feeds such a malformed section and asserts that:
    - Valid `shared` members still parse.
    - Malformed lines are ignored.
    - No error is returned.

---

### Fix 3: Add an explicit test for `build_queries` ordering

- **Addresses Finding**:  
  *Finding 3 – Query ordering behavior is relied on in spec but not asserted in tests.*

- **Changes**:  
  - Add a test in `core/tests/m5_query_domain_tests.rs` that directly asserts order stability. Two practical options:
    1. Use `metadata_simple.xlsx`:
       - Call `parse_section_members` on `Section1.m` from the fixture.
       - Call `build_queries`.
       - Assert that for all `i`, `queries[i].section_member == members[i].member_name`.
    2. Alternatively, write a synthetic in‑memory Section1.m string with 2–3 shared queries in a known order and a small synthetic `DataMashup` metadata list, then assert that `build_queries` preserves the order seen from `parse_section_members`.  

- **Tests**:  
  - New test, e.g. `queries_preserve_section_member_order`, added alongside existing `metadata_join_*` and invariant tests.   

---

### Fix 4: Add focused unit test for quoted identifiers at the parser layer

- **Addresses Finding**:  
  *Finding 4 – Quoted shared identifiers lack direct unit tests at the Section1 parser layer.*

- **Changes**:  
  - Extend `core/tests/m_section_splitting_tests.rs` with a new constant and test:
    * Section snippet such as:

      ```text
      section Section1;

      shared #"Query with space & #" = 1;
      ```

    * Assertions:
      * `members.len() == 1`
      * `members[0].section_name == "Section1"`
      * `members[0].member_name == "Query with space & #"`
      * `members[0].expression_m == "1"`
      * `members[0].is_shared == true`

  This directly exercises `parse_quoted_identifier` and ensures that any future refactor of the identifier parsing logic cannot regress without a failing unit test.   

- **Tests**:  
  - One new parser‑focused test in `m_section_splitting_tests.rs` as described above.

---

## Constraints

- These changes should **not** alter the runtime behavior of the existing passing tests except where intentionally clarified:
  * The metadata synthesis behavior should remain exactly as implemented in `build_queries`.   
  * Any change to invalid member syntax handling must be coordinated with updated specs and new tests so that behavior is consistent and predictable.
- No changes are required to the grid diff engine, JSON output, or higher‑level diff algorithms in this cycle.
- Keep `Query` and `QueryMetadata` APIs stable so that future M‑diff work can build directly on them.

## Expected Outcome

After these remediation steps:

1. The written mini‑spec, global testing plan, and code will all agree that:
   * Shared Section1 members **always** produce `Query` objects.
   * When metadata is missing, those queries get well‑defined, synthetic, connection‑only metadata.
2. Error semantics around invalid `shared` lines will be explicitly defined and either enforced or clearly documented as “ignored.”
3. Query ordering guarantees will be protected by tests, preventing accidental regressions in future refactors.
4. Quoted identifier support in `parse_section_members` will be backed by a dedicated unit test, making the Section1 parser more robust against subtle changes.

Functionally, the behavior of the current branch will remain the same; what changes is that future contributors will have accurate, executable documentation and tests that match the implemented semantics.
