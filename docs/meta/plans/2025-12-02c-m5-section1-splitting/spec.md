# 2025-12-02-m5-section1-splitting – Section1.m splitting (Milestone 5.1)

This cycle progresses **Milestone 5 – Domain layer: M queries & metadata API** by implementing the **Section1.m splitting** capability and associated unit tests. :contentReference[oaicite:12]{index=12}

---

## 1. Scope

### 1.1 New module and types

**Rust crate:** `core` (library `excel_diff`)

New module:

- `core/src/m_section.rs`  
  - Responsible for parsing a Power Query **section document** (the text in `/Formulas/Section1.m`) into named members.

Public types and functions:

```rust
// core/src/m_section.rs
pub struct SectionMember {
    pub section_name: String,   // e.g., "Section1"
    pub member_name: String,    // e.g., "Foo"
    pub expression_m: String,   // text of the right-hand side, without trailing ';'
    pub is_shared: bool,        // true if declared with "shared"
}

#[derive(Debug)]
pub enum SectionParseError {
    MissingSectionHeader,
    InvalidHeader,
    InvalidMemberSyntax,
}

pub fn parse_section_members(source: &str) -> Result<Vec<SectionMember>, SectionParseError>;
````

Exports from `core/src/lib.rs`:

```rust
pub mod m_section;

pub use m_section::{SectionMember, SectionParseError, parse_section_members};
```

These APIs are intentionally small and text-only. They do **not** depend on Excel containers, ZIP/OPC, DataMashup framing, or the grid diff engine. They are a pure-M utility that later layers (DataMashup domain, M diff engine) will build on.

### 1.2 Tests

New test module:

* `core/tests/m_section_splitting_tests.rs`

Tests in this module exercise `parse_section_members` only (no Excel files):

* `parse_single_member_section`
* `parse_multiple_members`
* `tolerate_whitespace_comments`
* `error_on_missing_section_header` (extra guard not explicitly in testing plan but useful to lock API semantics)

All test inputs are inline string literals; we do **not** introduce new `.m` fixture files for this cycle. The testing plan’s `section_single_member.m` / `section_multiple_members.m` are realized as string constants embedded in the tests. 

Out of scope:

* Metadata XML parsing (`Metadata` struct, `QueryMetadata`) and Metadata→Section join (testing plan 4.3, 5.2). 
* Query-level domain model (`struct Query { .. }`) and text diffing (`MQueryDiff`).
* Any changes to grid IR, DiffOp, or diff engine behavior.

---

## 2. Behavioral Contract

### 2.1 Input shape

`parse_section_members` consumes a **Power Query M section document** as UTF‑8 text, typically the contents of `/Formulas/Section1.m` in PackageParts. The testing plan assumes a single section `Section1` containing shared and private members.

Canonical form for this cycle’s tests:

```m
section Section1;

shared Foo = 1;
shared Bar = 2;
Baz = 3;
```

But the parser must tolerate:

* Leading and trailing whitespace.
* Blank lines between declarations.
* Single-line comment lines introduced with `//`.

### 2.2 Section header handling

Rules:

1. The parser **requires** a section header of the form:

   ```m
   section Section1;
   ```

   * `section` keyword (case-sensitive).
   * A single identifier (letters, digits, `_`) as section name.
   * A terminating `;` on the header line (possibly after spaces). 

2. Parsing:

   * Lines before the first valid `section` header are ignored.
   * If no valid header is found, `parse_section_members` returns `Err(SectionParseError::MissingSectionHeader)`.

3. For this cycle, the parser:

   * Accepts **any** identifier as the section name (tests use `"Section1"`).
   * Copies the parsed section name into each `SectionMember.section_name`.

Example:

```rust
let src = r#"
    // Comment
    section Section1;

    shared Foo = 1;
"#;

let members = parse_section_members(src).unwrap();
assert_eq!(members[0].section_name, "Section1");
```

### 2.3 Member detection and splitting

We treat **top-level shared declarations** as queries and ignore purely private members for this cycle.

#### 2.3.1 What counts as a member

A member is recognized if, after the section header:

* At the start of a declaration (after any blank lines/comments), we see:

  ```text
  shared <Name> = <Expression> ;
  ```

  where:

  * `shared` is a literal keyword.
  * `<Name>` is a single identifier.
  * `=` is the first `=` on that logical line.
  * `<Expression>` is everything after `=` up to a terminating `;`, potentially spanning multiple physical lines (but our tests will keep expressions one logical line long). 

For this cycle:

* Only declarations starting with `shared` produce a `SectionMember` with `is_shared = true`.
* Non‑`shared` declarations after the header (e.g., `Baz = 3;`) are **ignored**, effectively treating them as private helpers not exposed as queries.

This choice aligns with the typical semantics where only `shared` members are user-visible queries and forms the baseline for `Query.section_member` and `Query.name = "Section1/Foo"` later.

#### 2.3.2 Expression body

For a declaration `shared Foo = Expr;`:

* `member_name` is `"Foo"` (trimmed).
* `expression_m` is the substring between `=` and the final `;`, trimmed of leading/trailing whitespace, but otherwise preserved verbatim (inner spaces, line breaks, etc.).
* The trailing `;` is **not** included in `expression_m`.

Example 1 – single member:

```m
section Section1;

shared Foo = 1;
```

Yields:

```rust
SectionMember {
    section_name: "Section1".into(),
    member_name: "Foo".into(),
    expression_m: "1".into(),
    is_shared: true,
}
```

Example 2 – multiple members with private:

```m
section Section1;

shared Foo = 1;
shared Bar = 2;
Baz = 3;
```

Yields (order preserved):

```rust
vec![
    SectionMember { member_name: "Foo", expression_m: "1", is_shared: true, section_name: "Section1" },
    SectionMember { member_name: "Bar", expression_m: "2", is_shared: true, section_name: "Section1" },
]
```

`Baz` is not returned.

### 2.4 Whitespace and comments

The parser must:

* Ignore blank lines anywhere (before/after section header, between declarations).
* Ignore full-line comments starting with `//` (after optional leading whitespace).
* Permit trailing spaces and tabs on any line.

For this cycle, we **do not** attempt to parse:

* Multi-line comments (`/* ... */`).
* Inline comments after valid code on the same line.
* Arbitrary M constructs inside the expression; expressions are treated as opaque text slices.

Example:

```m
// Top-of-file noise

section Section1;

// A comment before Foo
shared Foo = 1;   // inline comment (not guaranteed to be stripped correctly)

// Another comment
shared Bar =     2    ;
```

Expected behavior:

* `parse_section_members` returns two members (`Foo`, `Bar`).
* `expression_m` for `Bar` is `"2"` after trimming.
* Inline comment after `Foo` may be included in `expression_m` or truncated; tests for this cycle will not rely on inline comments inside expressions.

### 2.5 Error behavior

`parse_section_members` is deliberately conservative:

* If the section header is missing or malformed (e.g., `section;`, `section  ;`), it returns `SectionParseError::MissingSectionHeader` or `InvalidHeader`.
* Once the header is found, malformed `shared` lines (e.g., missing `=`, missing name) are skipped rather than causing a hard error; such cases may be treated as `InvalidMemberSyntax` in future cycles if needed.

For this cycle’s tests:

* Only `missing_section_header` is asserted as an explicit error case.
* All positive test inputs are well-formed, so no `InvalidMemberSyntax` is expected.

---

## 3. Constraints

### 3.1 Complexity and performance

* Parsing is expected to be **O(N)** in the length of `source`:

  * Single pass over lines, with at most one linear scan per declaration to find `=` and terminating `;`.
* No large intermediate allocations:

  * Use slices or minimal `String` clones for `expression_m`.
* This function will eventually be called on real Section1 documents which can be substantial, but still small compared to grid sizes; performance is not critical in this cycle but should be obviously linear and safe.

### 3.2 Robustness and streaming

* `parse_section_members` is pure and does not own I/O: the caller is responsible for reading Section1 bytes as UTF‑8 text.
* The function must **never panic** on arbitrary UTF‑8 input; all failures must flow through `SectionParseError`.

### 3.3 Forward compatibility

The design intentionally separates:

* **Interface** (`SectionMember`, `parse_section_members`) – expected to remain broadly stable.
* **Implementation** – initially a simple line-based splitter, later replaceable by a full M parser or an incremental lexer.

Future work (AST-based M parsing, step-level diffing) can reuse `SectionMember` as a boundary between Section1 documents and higher-level query semantics.

---

## 4. Interfaces

### 4.1 Public API for this cycle

As exported from `lib.rs`:

```rust
pub struct SectionMember {
    pub section_name: String,
    pub member_name: String,
    pub expression_m: String,
    pub is_shared: bool,
}

#[derive(Debug)]
pub enum SectionParseError {
    MissingSectionHeader,
    InvalidHeader,
    InvalidMemberSyntax,
}

pub fn parse_section_members(source: &str) -> Result<Vec<SectionMember>, SectionParseError>;
```

Contract:

* `SectionMember` is a simple POD type, suitable for later embedding in:

  ```rust
  struct Query {
      name: String,           // "Section1/Foo"
      section_member: String, // "Foo"
      expression_m: String,
      metadata: QueryMetadata,
  }
  ```

  as described in the testing plan.

* `SectionParseError` is not exposed outside the crate boundary in any serialized form; it is a normal Rust error.

### 4.2 Interfaces that must remain stable

For this cycle, **no** existing public APIs are allowed to change:

* `RawDataMashup`, `open_data_mashup`, and DataMashup framing APIs stay as-is.
* Grid IR (`Workbook`, `Sheet`, `Grid`, `Cell`) and DiffOp types remain unchanged.
* JSON output and `DiffReport` schema remain untouched.

Future cycles may introduce:

* A `DataMashup` domain struct that wraps `RawDataMashup` plus parsed PackageParts/Metadata, and uses `parse_section_members` to populate `Vec<Query>`.

This cycle’s spec deliberately avoids committing to that shape beyond the `SectionMember` contract.

---

## 5. Test Plan

This cycle is explicitly tied to **Milestone 5 – Domain layer: M queries & metadata API, Section 5.1 "Section1.m splitting"**. 

### 5.1 New unit tests

File: `core/tests/m_section_splitting_tests.rs`

#### 5.1.1 `parse_single_member_section`

**Input:**

Inline string constant (mirrors `section_single_member.m`):

```rust
const SECTION_SINGLE: &str = r#"
    section Section1;

    shared Foo = 1;
"#;
```

**Behavior to assert:**

* `parse_section_members(SECTION_SINGLE)` returns `Ok(members)` with:

  * `members.len() == 1`
  * `members[0].section_name == "Section1"`
  * `members[0].member_name == "Foo"`
  * `members[0].expression_m == "1"` (no trailing `;`, whitespace trimmed)
  * `members[0].is_shared == true`

This test codifies the **happy-path** for a single shared member.

#### 5.1.2 `parse_multiple_members`

**Input:**

Inline string (mirrors `section_multiple_members.m` from the testing plan):

```rust
const SECTION_MULTI: &str = r#"
    section Section1;

    shared Foo = 1;
    shared Bar = 2;
    Baz = 3;
"#;
```

**Behavior to assert:**

* `parse_section_members(SECTION_MULTI)` returns `Ok(members)` with:

  * `members.len() == 2`
  * In order:

    * `members[0].member_name == "Foo"`
    * `members[1].member_name == "Bar"`
  * Both members:

    * Have `section_name == "Section1"`.
    * Have `is_shared == true`.
    * `expression_m == "1"` or `"2"` respectively.

* There is **no** member for `Baz`.

This test **locks in** the decision that private members (no `shared` keyword) are ignored when constructing the query list, matching the testing plan’s “private ones either included/excluded depending on API design (tests codify that decision).” 

#### 5.1.3 `tolerate_whitespace_comments`

**Input:**

```rust
const SECTION_NOISY: &str = r#"

// Leading comment

section Section1;

// Comment before Foo
shared Foo = 1;

// Another comment

    shared   Bar   =    2    ;

"#;
```

**Behavior to assert:**

* `parse_section_members(SECTION_NOISY)` returns two members (`Foo`, `Bar`) with the **same field values** as in `parse_multiple_members`.
* Blank lines and `//` comment lines do not affect the result.
* The extra spaces around `shared`, the name, and `=` do not change the parsed names or expressions (beyond trimming).

This confirms the parser is resilient to reasonable formatting noise and comments in the section, per testing-plan expectations.

#### 5.1.4 `error_on_missing_section_header`

**Input:**

```rust
const NO_SECTION: &str = r#"
    shared Foo = 1;
"#;
```

**Behavior to assert:**

* `parse_section_members(NO_SECTION)` returns `Err(SectionParseError::MissingSectionHeader)`.

This test ensures callers do not accidentally treat a bare chunk of M as a complete section document and locks in the requirement that a valid header must be present.

### 5.2 Existing tests to keep unchanged

* All DataMashup framing tests in `core/tests/data_mashup_tests.rs` remain as-is; they continue to verify that we can extract `dm_bytes` and split PackageParts, Permissions, Metadata, PermissionBindings per MS‑QDEFF.
* All grid IR, DiffOp, PG1–PG6 tests remain unchanged and serve as a regression net for other subsystems.

### 5.3 Future tests enabled by this work (not in this cycle)

This spec lays groundwork for upcoming Milestone 4 & 5 tests that will build on `parse_section_members`:

* `metadata_formulas_match_section_members` – use Section splitting + Metadata XML to assert count/consistency of formula entries. 
* `metadata_join_simple` / `metadata_join_url_encoding` – combine Section members with Metadata `Formulas` keyed by `SectionName/FormulaName`. 
* `query_names_unique` and `metadata_orphan_entries` – operate on `Vec<Query>` where `Query` wraps `SectionMember` plus `QueryMetadata`.

Those tests remain out of scope for this cycle; they are noted here to show how this work advances the overall M‑side roadmap toward Milestone 6 (textual M diff engine).
