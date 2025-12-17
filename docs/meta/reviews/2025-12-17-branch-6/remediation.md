## Branch 6 completion status

**Branch 6 is functionally very close, but not “completed in full” relative to *next_sprint_plan.md* as written.**

What’s clearly done (and passing):

* **Non-let top-level expression parsing exists**: the parser now recognizes record literals, list literals, qualified function calls, and primitive literals (in addition to `let ... in ...`) and otherwise falls back to `Opaque`.
* **Canonicalization handles the new expression types**:

  * Records: canonicalization sorts fields by name (and canonicalizes values recursively). 
  * Lists: canonicalization preserves order (and canonicalizes items recursively). 
* **Tests prove the branch-6 behaviors**:

  * Parser expansion tests for record/list/call/primitive pass.
  * Record-field-order semantic equivalence and list-order non-equivalence pass.
  * Semantic diff masking works for non-let formatting-only variants. 
* **No regressions**: full unit test suite is green (145 tests).

Benchmarks:

* The attached `benchmark_results.json` shows **full-scale perf results recorded on branch `2025-12-17-branch-6`**. 
* **But** the performance section inside `cycle_summary.txt` reports the same timings while labeling them as branch `2025-12-13-branch-3` / commit `941057538591`. That’s a metadata mismatch between the two artifacts.

What’s not “complete in full” vs Branch 6 plan:

1. **6.1 Audit Current Parser Coverage** is not fully delivered as a concrete artifact set. The code has a short doc comment describing supported forms and the `Opaque` fallback , but Branch 6.1 explicitly calls for:

   * a coverage document,
   * test fixtures for unsupported constructs,
   * and prioritization by real-world frequency.

2. **6.3 “Ensure `canonicalize_tokens` is no longer a no-op for non-let expressions”** is **not literally satisfied**: `canonicalize_tokens` is still a no-op today.

   * You *do* get semantic masking for the newly-parsed non-let forms via the structured AST (records/lists/calls/primitives), and those tests pass .
   * But the sprint-plan deliverable is explicit about `canonicalize_tokens`, so the branch is missing this item if you’re checking boxes strictly.

---

## Implementation plan to finish Branch 6 fully

### 1) Deliver Branch 6.1: Coverage audit doc + prioritized backlog + “unsupported construct” fixtures/tests

#### 1.1 Add a coverage document

Create a new doc file (example path): `docs/rust_docs/m_parser_coverage.md`

**New file content (create this file):**

```md
# M Parser Coverage (Branch 6)

This document describes which Power Query M expression constructs are parsed into structured AST nodes by `core/src/m_ast.rs`, and which are preserved as `Opaque` token sequences.

## Parsed constructs (structured AST)

These forms are parsed into explicit AST variants:

- Let expressions:
  - `let <bindings> in <expr>`
- Record literals:
  - `[Field1 = 1, Field2 = 2]`
- List literals:
  - `{1, 2, 3}`
- Qualified function calls:
  - `Table.FromRows(...)`
  - `Excel.CurrentWorkbook()`
  - `#date(2020, 1, 1)`
- Primitive literals:
  - `"hello"`, `42`, `-1`, `true`, `false`, `null`

## Opaque constructs (token-preserved)

Any expression not matching the structured forms above is preserved as `Opaque(Vec<MToken>)`.
Examples that currently fall back to Opaque include:

- Identifier references:
  - `Source`, `#"Previous Step"`
- If/then/else:
  - `if cond then a else b`
- Each / lambda / function literals:
  - `each _ + 1`
  - `(x) => x`
- Binary / unary operators:
  - `a + b`, `not a`
- Field access / projection:
  - `Source[Field]`, `rec[Name]`
- List indexing / item access:
  - `Source{0}`
- Combined access chains:
  - `Source{0}[Content]`
- Type annotations:
  - `x as number`

## Priority ranking (real-world frequency)

Tier 1 (very common in typical Power Query queries):
1. Identifier references (`Source`, `#"Previous Step"`)
2. Field access (`expr[Field]`) and item access (`expr{0}`), including chains
3. `each` expressions (common in `Table.AddColumn`, `Table.TransformColumns`)
4. `if ... then ... else ...`

Tier 2 (common in more advanced queries):
1. Function literals `(x) => ...`
2. Basic binary/unary operators in expressions beyond literals
3. Type annotations (`as ...`)

Tier 3 (less common / advanced):
1. `try ... otherwise ...`
2. `meta` annotations
3. More complex grammar forms (pattern matching, etc.)

## Test coverage mapping

Branch 6 includes tests that prove:
- record/list/call/primitive parse into structured AST
- record field order is canonicalized and semantically equal
- semantic diff masks formatting-only differences for non-let root expressions

Additional tests should exist to pin “unsupported constructs” to Opaque behavior (and keep the coverage map honest).
```

This doc satisfies Branch 6.1’s “document constructs parsed vs opaque” plus “prioritize by frequency” requirements.

#### 1.2 Add “unsupported construct” fixtures/tests

Instead of introducing more `.xlsx` fixtures, you can satisfy “fixture” intent with **string fixtures embedded in a dedicated test module** that:

* Enumerates representative unsupported forms
* Asserts they fall back to `Opaque`
* (Optionally) asserts canonicalization doesn’t crash and is stable

**New file (create): `core/tests/m8_m_parser_coverage_audit_tests.rs`**

```rust
use excel_diff::{canonicalize_m_ast, parse_m_expression, MAstKind};

fn assert_opaque(expr: &str) {
    let mut ast = parse_m_expression(expr).expect("expression should parse into an AST container");
    canonicalize_m_ast(&mut ast);
    match ast.root_kind_for_testing() {
        MAstKind::Opaque { token_count } => assert!(token_count > 0, "opaque token_count must be > 0"),
        other => panic!("expected Opaque, got {:?}", other),
    }
}

#[test]
fn coverage_audit_unsupported_constructs_are_opaque() {
    let cases = [
        "Source",
        "#\"Previous Step\"",
        "if true then 1 else 0",
        "each _ + 1",
        "(x) => x",
        "1 + 2",
        "not true",
        "Source[Field]",
        "Source{0}",
        "Source{0}[Content]",
        "x as number",
    ];

    for expr in cases {
        assert_opaque(expr);
    }
}
```

This gives you an explicit, maintained coverage harness to back Branch 6.1.

---

### 2) Finish Branch 6.3: make `canonicalize_tokens` non-no-op

Right now `canonicalize_tokens` is explicitly a no-op.
To satisfy the deliverable without expanding the grammar, implement **minimal safe token canonicalization** for the case-insensitive literal identifiers you already treat specially at the AST layer (`true`, `false`, `null`).

This is low-risk because:

* It does not attempt to “parse” opaque expressions.
* It only normalizes tokens whose semantics are already treated as case-insensitive in your structured parsing (`eq_ignore_ascii_case` in `parse_primitive`). 

#### 2.1 Code change: `core/src/m_ast.rs`

Replace the existing function:

```rust
fn canonicalize_tokens(tokens: &mut Vec<MToken>) {
    // Tokens are already whitespace/comment free; no additional normalization needed yet.
    let _ = tokens;
}
```

With:

```rust
fn canonicalize_tokens(tokens: &mut Vec<MToken>) {
    for token in tokens.iter_mut() {
        let MToken::Identifier(ident) = token else {
            continue;
        };

        if ident.eq_ignore_ascii_case("true") {
            *ident = "true".to_string();
        } else if ident.eq_ignore_ascii_case("false") {
            *ident = "false".to_string();
        } else if ident.eq_ignore_ascii_case("null") {
            *ident = "null".to_string();
        }
    }
}
```

This makes `canonicalize_tokens` meaningfully non-no-op while staying narrowly scoped.

#### 2.2 Add a test proving the behavior

Create a new test file (or append to the audit test file above) to ensure the canonicalization change is real and stable.

**New file (create): `core/tests/m8_m_canonicalize_tokens_tests.rs`**

```rust
use excel_diff::{ast_semantically_equal, canonicalize_m_ast, parse_m_expression};

#[test]
fn opaque_boolean_literal_case_is_canonicalized() {
    let a = "if TRUE then 1 else 0";
    let b = "if true then 1 else 0";

    let mut ast_a = parse_m_expression(a).expect("a should parse");
    let mut ast_b = parse_m_expression(b).expect("b should parse");

    canonicalize_m_ast(&mut ast_a);
    canonicalize_m_ast(&mut ast_b);

    assert!(ast_semantically_equal(&ast_a, &ast_b));
}

#[test]
fn opaque_null_literal_case_is_canonicalized() {
    let a = "if NULL then 1 else 0";
    let b = "if null then 1 else 0";

    let mut ast_a = parse_m_expression(a).expect("a should parse");
    let mut ast_b = parse_m_expression(b).expect("b should parse");

    canonicalize_m_ast(&mut ast_a);
    canonicalize_m_ast(&mut ast_b);

    assert!(ast_semantically_equal(&ast_a, &ast_b));
}
```

This directly satisfies Branch 6.3’s “canonicalize_tokens not no-op” requirement.

