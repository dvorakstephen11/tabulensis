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

Additional tests should exist to pin unsupported constructs to Opaque behavior (and keep the coverage map honest).
