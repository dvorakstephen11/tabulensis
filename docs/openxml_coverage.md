# OpenXML Construct Coverage (Semantic vs Opaque)

This doc describes which OpenXML parts/elements Tabulensis parses semantically versus treats as
opaque. It is the OpenXML analogue of `docs/m_parser_coverage.md`.

## Supported (Semantic)

Tabulensis currently parses these OpenXML constructs into structured IR:

- Workbook structure: `xl/workbook.xml` (sheet list, ids).
- Worksheet grids: `xl/worksheets/*.xml` (cell values + formula text; no formula evaluation).
- Shared strings: `xl/sharedStrings.xml`.
- Defined names (named ranges): `xl/workbook.xml` `<definedName>` into `Workbook.named_ranges`
  (global and sheet-scoped, with `scope` captured).
- Charts (metadata only): drawing relationships + chart parts into `Workbook.charts`:
  - chart name (from the drawing `cNvPr/@name` when present)
  - chart type (first `<*Chart>` tag, e.g. `barChart`)
  - first data range formula `<f>` when present
  - XML hash for change detection
- DataMashup / Power Query: detects and parses the DataMashup part into
  `WorkbookPackage.data_mashup` (query semantics tracked separately in `docs/m_parser_coverage.md`).
- VBA (xlsm): extracts VBA modules into `WorkbookPackage.vba_modules`.

## Opaque / Ignored (By Design)

These constructs are not currently treated as semantic inputs to the diff:

- Formatting/styling: styles, themes, fonts, fills, number formats.
- Layout/rendering: row heights, column widths, conditional formatting, tables, pivot tables.
- Embedded media/shapes beyond chart discovery (images, most drawing primitives).
- External links and calculation engine state.

## Audit Test

The coverage audit test asserts that a few representative constructs remain semantic:

- `core/tests/openxml_coverage_audit_tests.rs`

If OpenXML support regresses into "opaque" behavior (for example charts no longer produce
structured metadata), this test should fail.

## How to Update This Doc

When adding semantic support for a new OpenXML part/element:

1. Add it to **Supported (Semantic)**.
2. Add/update a fixture under `fixtures/manifest_cli_tests.yaml`.
3. Extend `core/tests/openxml_coverage_audit_tests.rs` with an assertion for the new behavior.

