# Formula Coverage (Parsing + Semantic Diff)

This doc describes which Excel formula constructs Tabulensis parses semantically and how semantic
formula diffs are classified.

## Supported (Semantic Parse)

The formula parser (`parse_formula`) supports:

- Literals: numbers, strings, booleans, Excel errors (for example `#DIV/0!`).
- References:
  - cell references (`A1`, `$B$2`, `Sheet1!C3`)
  - range references (`A1:B2`)
  - named references (`MyName`)
- Function calls (`SUM(A1, 2)`), including nested calls.
- Unary operators (`+`, `-`, `%`).
- Binary operators (arithmetic, concatenation `&`, and comparisons).
- Array literals (`{1,2;3,4}`).

Notes:

- Formulas are parsed as text (the engine does not evaluate them).
- If parsing fails for a cell, semantic formula diff falls back to a text-only classification.

## Canonicalization Rules

Semantic comparisons canonicalize the parsed AST:

- Uppercase function names, named refs, and sheet names.
- Sort arguments for commutative functions (currently: `SUM`, `PRODUCT`, `MIN`, `MAX`, `AND`, `OR`).
- Sort operands for commutative binary operators (`+`, `*`, `=`, `<>`).
- Normalize range endpoint order (`B2:A1` becomes `A1:B2`).

## Semantic Formula Diff (`DiffOp::CellEdited.formula_diff`)

Semantic formula diffing is controlled by `DiffConfigBuilder::enable_formula_semantic_diff(true)`.

When enabled, formula changes are classified as:

- `FormattingOnly`: canonicalized ASTs are equal (whitespace/casing/argument order only).
- `Filled`: shift-equivalent (filled down/across) when row/col alignment implies a shift.
- `SemanticChange`: parsed ASTs differ semantically.

When disabled (or when parsing fails), the engine reports `TextChange` for formula differences.

## Audit Test

- `core/tests/formula_coverage_audit_tests.rs` asserts representative constructs remain semantic
  and that semantic diffs classify formatting-only changes correctly.

## How to Update This Doc

When expanding formula semantic support:

1. Update **Supported (Semantic Parse)** / **Canonicalization Rules** as needed.
2. Add an audit case to `core/tests/formula_coverage_audit_tests.rs` (prefer a minimal expression
   plus one end-to-end diff assertion when classification changes).

