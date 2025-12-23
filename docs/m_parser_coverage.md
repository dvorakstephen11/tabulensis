# M parser coverage

This document describes the current syntax coverage in the M parser and how it maps to semantic diff output.

## Tier 1 (core syntax)

| Construct | Coverage | Notes |
| --- | --- | --- |
| Identifiers and dotted access | Parsed | Includes `Foo` and `Foo.Bar` chains. |
| `let ... in ...` blocks | Parsed | Primary shape used for Applied Steps extraction. |
| Record literals | Parsed | `[Field = Expr, ...]`. |
| List literals | Parsed | `{Expr, ...}`. |
| Function calls | Parsed | Identifier and qualified calls (e.g., `Table.SelectRows`). |
| `if/then/else` | Parsed | Supported in expressions and steps. |
| `each` shorthand | Parsed | Treated as a lambda. |
| Operators | Parsed | Arithmetic, comparison, and logical operators. |

## Tier 2 (expanded syntax)

| Construct | Coverage | Notes |
| --- | --- | --- |
| Explicit lambdas | Parsed | `(x) => ...` and multi-arg lambdas. |
| `try ... otherwise ...` | Parsed | Error-handling expressions. |
| Type ascription | Parsed | `value as type` (including nullable). |

## Known gaps that fall back to AST summary

Step extraction can fall back to `AstDiffSummary` when the query does not match the Applied Steps shape or when metadata is too dynamic. Known cases include:

- Dynamic step names (computed identifiers) that prevent stable step identity.
- Queries that do not resolve to a linear `let` step pipeline (custom functions or nested transforms).
- Table transformations outside the recognized set (SelectRows, RemoveColumns, RenameColumns, TransformColumnTypes, NestedJoin, Join) that are classified as `Other` steps.
