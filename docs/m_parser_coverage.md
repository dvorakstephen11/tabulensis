# Semantic M parser coverage

This doc describes which Power Query (M) constructs are parsed semantically versus treated as opaque text.

## Terms

- Semantic parse: the code builds an AST and can reason about structure (steps, references).
- Opaque: the code treats the query definition as a string blob and only reports text/format changes.

## Supported constructs

The semantic parser supports:
- Records, lists
- let/in expressions
- if/then/else expressions
- Binary operators (arithmetic, comparisons, logical)
- Function definitions (lambdas)
- try/otherwise
- Type ascription

## Not yet semantic

Anything not in the supported list is treated as opaque. In those cases:
- the diff can still detect that the query changed
- step-level diffs may be missing
- AST summary may be unknown or absent

## How to update this doc

Whenever new M syntax becomes semantic, add it to Supported constructs and add a fixture + test that exercises it.
