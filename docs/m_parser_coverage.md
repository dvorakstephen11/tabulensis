# M parser coverage (Branch 6)

## Parsed structural forms
- let ... in ...
- record literal: [Field = Expr, ...]
- list literal: {Expr, ...}
- function call: Name(...) where Name is Identifier(.Identifier)*
- primitive: string, number, true/false/null

## Opaque fallback
Everything else is represented as an opaque token sequence.
Examples:
- field access: [Content]
- indexing: Source{0}[Column]
- operators: 1 + 2
- if/then/else
- each/lambda
- type annotations

## Prioritization by frequency
Most workbook queries start with let bindings and frequently build records, lists, and table-producing function calls (Excel.CurrentWorkbook, Table.*, Value.*). Branch 6 targets these common non-let roots and adds canonicalization wins such as record field ordering; less frequent constructs (operators, indexing, control flow) remain opaque for now.
