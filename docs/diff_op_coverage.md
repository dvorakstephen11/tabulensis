# DiffOp Coverage (Categories)

This doc tracks which diff-operation categories Tabulensis is expected to emit as first-class
signals, and provides a stable "coverage audit" that fails if key categories regress.

`DiffOp` is the canonical typed change stream produced by the diff engine.

## Expected Categories

Tabulensis intentionally emits (at least) these categories:

- Sheet-level: `SheetAdded`, `SheetRemoved`, `SheetRenamed`.
- Grid structure: `RowAdded`, `RowRemoved`, `ColumnAdded`, `ColumnRemoved`, `RowReplaced`,
  `DuplicateKeyCluster`.
- Moves/replacements: `BlockMovedRows`, `BlockMovedColumns`, `BlockMovedRect`, `RectReplaced`.
- Cell edits: `CellEdited` (including `formula_diff` when enabled).
- Workbook objects:
  - named ranges: `NamedRangeAdded`/`Removed`/`Changed`
  - charts: `ChartAdded`/`Removed`/`Changed`
  - VBA: `VbaModuleAdded`/`Removed`/`Changed`
- Power Query / DataMashup: `QueryAdded`/`Removed`/`Renamed`, `QueryDefinitionChanged`,
  `QueryMetadataChanged`.
- Model diff (when `model-diff` is enabled): table/column/relationship/measure ops.

## Projections (Not "Missing Coverage")

Some outputs intentionally project a subset of ops:

- Cell-diff JSON projections ignore block moves and other non-cell ops by design.
- UI payloads may coalesce or summarize ops for presentation.

This is not a coverage gap as long as the underlying `DiffOp` stream remains available and tested.

## Audit Test

- `core/tests/diff_op_coverage_audit_tests.rs`

This test runs a curated set of small fixture diffs and asserts that each major category is still
reachable. If an engine change accidentally collapses a category into "generic" cell edits (or
removes object/query ops), the audit should fail.

## How to Update This Doc

When adding a new `DiffOp` category that should be treated as first-class:

1. Add it to **Expected Categories**.
2. Extend `core/tests/diff_op_coverage_audit_tests.rs` to include a fixture diff that emits it.

