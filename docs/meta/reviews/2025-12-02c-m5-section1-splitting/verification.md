````markdown
# Verification Report: 2025-12-02c-m5-section1-splitting

## Summary

The implementation of `parse_section_members` and its supporting types in `core/src/m_section.rs` matches the mini-spec and higher-level Excel Diff specification for Milestone 5.1. The new module is wired into `lib.rs`, and the planned unit tests in `core/tests/m_section_splitting_tests.rs` are present and assert the intended behavior: section header detection, shared-member extraction, whitespace/comment tolerance, and error-on-missing-header. Error handling is conservative and panic-free, and no changes were made to other public APIs beyond the planned exports. There are no release-blocking issues; the remaining findings are about untested error paths and forward-compatibility gaps that can be addressed in later cycles.

## Recommendation

[x] Proceed to release  
[ ] Remediation required

## Findings

### 1. Error paths are only partially covered by tests

- **Severity**: Moderate  
- **Category**: Missing Test / Gap  
- **Description**:  
  The mini-spec defines several error behaviors for `parse_section_members`:

  * Missing header → `SectionParseError::MissingSectionHeader`.  
  * Malformed header (e.g. `section;`, `section  ;`) → `SectionParseError::InvalidHeader`.  
  * Malformed `shared` lines are skipped rather than erroring.   

  The implementation correctly distinguishes these cases:

  * `find_section_name` returns `MissingSectionHeader` if no header is found at all. :contentReference[oaicite:1]{index=1}  
  * `try_parse_section_header` rejects malformed headers with `InvalidHeader`. :contentReference[oaicite:2]{index=2}  
  * `parse_shared_member` returns `None` for malformed `shared` declarations, causing them to be skipped.   

  However, test coverage only asserts:

  * The happy paths (`parse_single_member_section`, `parse_multiple_members`, `tolerate_whitespace_comments`).   
  * The missing-header error case (`error_on_missing_section_header`).   

  There are **no tests** that:

  * Assert that malformed headers specifically yield `InvalidHeader`.  
  * Assert that malformed `shared` lines are silently skipped rather than causing errors or panics.  

- **Evidence**:  
  * Error behavior described in cycle plan.   
  * Implementation in `m_section.rs`.   
  * Tests in `core/tests/m_section_splitting_tests.rs`.   

- **Impact**:  
  The current behavior is correct, but untested error paths are easier to regress in future refactors (for example when integrating Section parsing with Metadata or replacing the line-based splitter with a more sophisticated parser). Adding a small number of focused tests would lock in the desired semantics and reduce risk when the M pipeline grows. This does **not** block release but is worthwhile to address soon.

---

### 2. Identifier grammar is limited to ASCII alphanumeric + underscore

- **Severity**: Minor  
- **Category**: Gap / Forward-compatibility risk  
- **Description**:  
  The spec for this cycle defines `<Name>` as “a single identifier (letters, digits, `_`)”.   
  The implementation enforces this via:

  ```rust
  fn is_valid_identifier(name: &str) -> bool {
      !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
  }
````



Combined with `split_identifier`, this means that only simple ASCII identifiers like `Foo`, `Bar2`, `Section1` are allowed as member names. Section members whose names require M’s more general identifier forms (e.g. queries named `"Query with space & #"` that show up as `shared #"Query with space & #"` in M) will be **ignored** for now because their “identifier” fails `is_valid_identifier`.

* **Evidence**:

  * Behavioral spec narrowing `<Name>` to a simple identifier for this cycle.
  * Implementation of `split_identifier` and `is_valid_identifier`.
  * Future tests in the global testing plan explicitly call out URL-encoded metadata names for queries with special characters. 

* **Impact**:

  * **For this cycle**: This is consistent with the mini-spec’s intentionally narrow identifier definition. The tests all use simple names (`Foo`, `Bar`).
  * **For future milestones**: Upcoming tests like `metadata_join_url_encoding` rely on being able to map metadata items with URL-encoded names (including spaces and punctuation) back to section members.  With the current grammar, such queries would have metadata entries but no corresponding `SectionMember`, breaking invariants like `metadata_formulas_match_section_members`. This will need to be extended before those milestones, but it does not violate the scope of the current cycle.

---

### 3. Expression terminator detection is simplistic (first `;` wins)

* **Severity**: Minor

* **Category**: Gap / Potential Bug in exotic expressions

* **Description**:
  The contract says the expression body is “everything after `=` up to the final `;`, potentially spanning multiple physical lines” and that expressions are otherwise treated as opaque text.

  The implementation:

  * Treats everything after the first `=` as `expression_source`.
  * On the same line, truncates at the **first** `;` found.
  * If no `;` is present, it appends subsequent lines until any `;` is found, then truncates at that first occurrence across lines.

  This matches all of the current tests, which use very simple expressions (`1`, `2`).  But it may misbehave for more exotic cases where `;` appears inside a string literal or other construct in the expression, since we do not perform any M-aware lexing.

* **Evidence**:

  * Expression-body spec.
  * Implementation of `parse_shared_member`’s `;` search.

* **Impact**:

  * For realistic, simple queries (and all tests in this cycle), this is fine.
  * For complex queries that embed `;` inside strings or other constructs, `expression_m` may be truncated earlier than expected. The mini-spec explicitly defers full M parsing and notes that expressions are treated as opaque slices, so this is acceptable for now but worth documenting as a limitation until a proper lexer/AST is introduced.

---

### 4. `InvalidMemberSyntax` variant is currently unused

* **Severity**: Minor

* **Category**: Spec Deviation (very small) / Gap

* **Description**:
  The public error enum includes an `InvalidMemberSyntax` variant as a placeholder.
  The behavioral contract for this cycle says malformed `shared` lines should simply be skipped, and that such cases “may be treated as `InvalidMemberSyntax` in future cycles”.

  The implementation follows the “skip” behavior and **never returns** `InvalidMemberSyntax`; malformed `shared` lines cause `parse_shared_member` to return `None`, and the line is ignored.

* **Evidence**:

  * Error enum and behavior description in cycle plan.
  * Implementation of `parse_shared_member` only returning `Option<SectionMember>` and never constructing `InvalidMemberSyntax`.

* **Impact**:
  This is consistent with the spec’s “for this cycle” behavior; the extra variant is essentially reserved for later. There is no functional bug, but the presence of an unused variant may confuse future readers unless documented as intentional.

---

## Checklist Verification

* [x] All scope items from mini-spec addressed

  * New module `core/src/m_section.rs` with `SectionMember`, `SectionParseError`, and `parse_section_members`.
  * Exports added to `core/src/lib.rs`. 

* [x] All specified tests created

  * `parse_single_member_section`, `parse_multiple_members`, `tolerate_whitespace_comments`, `error_on_missing_section_header` all present in `core/tests/m_section_splitting_tests.rs`.

* [x] Behavioral contract satisfied

  * Header handling, shared-only member enumeration, expression trimming, whitespace and `//` comment handling, and conservative error paths all match the mini-spec.

* [x] No undocumented deviations from spec

  * The only notable limitations (identifier grammar, simple `;` handling) are consistent with the “for this cycle” constraints described in the spec, and future work is already planned for richer M parsing and metadata join.

* [x] Error handling adequate

  * No `unwrap`/`expect` in the new parsing logic; all failures flow through `SectionParseError` or `None` for malformed members. Indexing is guarded by prior checks, so panics are highly unlikely on arbitrary UTF-8 input.

* [x] No obvious performance regressions

  * The parser is a single pass over `source.lines()` with at most one linear scan per member to find `=` and `;`, in line with the O(N) requirement. No large intermediate allocations beyond the expression strings themselves.

```

Since the recommendation is to proceed to release, no separate remediation plan is required for this branch. The moderate and minor findings above can be scheduled into future M-domain milestones (particularly those that add Metadata join and more realistic query fixtures).
```
