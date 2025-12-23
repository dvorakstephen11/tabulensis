# Release readiness

This document summarizes what the tool supports today, how semantic M diffs are reported, and the resource ceilings that can affect completeness.

## Support boundaries

- Excel workbooks (.xlsx/.xlsm): grid diffs, named ranges, charts, and VBA modules (when the `vba` feature is enabled).
- Power Query M diffs: extracted from DataMashup in workbooks and legacy PBIX/PBIT files; embedded queries are included.
- PBIX/PBIT enhanced metadata: when DataMashup is missing, the engine falls back to DataModelSchema and emits measure-level diffs only.
- Not supported: report visuals/layout diffs in PBIX/PBIT, and deeper model diffs beyond measures.

## Semantic M diff

- `QueryChangeKind` values:
  - `semantic`: canonicalized AST differs (meaningful change).
  - `formatting_only`: whitespace/comments changed but meaning is the same.
  - `renamed`: query name changed (definition may be unchanged).
- Step diffs: a human-friendly Applied Steps view derived from the M AST, reporting added/removed/modified/reordered steps and common Table.* step types.
- AST fallback summary: if step extraction is incomplete, the report includes `AstDiffSummary` with counts and diff mode.

## Resource ceilings

These knobs bound time and memory for large diffs. When a limit is hit, the report sets `complete=false` and emits warnings.

- `DiffConfig.max_memory_mb`: soft memory cap for advanced alignment strategies.
- `DiffConfig.timeout_seconds`: overall timeout for the diff engine.
- `DiffConfig.max_ops`: stops emitting ops after the limit is reached.
- `DiffConfig.on_limit_exceeded`:
  - `FallbackToPositional`: continue with a cheaper positional diff and warn.
  - `ReturnPartialResult`: return whatever was emitted so far and warn.
  - `ReturnError`: abort with an error.

Suggested presets:
- Fast: `DiffConfig::fastest()` (smaller limits, fewer semantic features).
- Precise: `DiffConfig::most_precise()` (more detailed output, higher cost).

Pick the preset that matches the expected file size and desired fidelity; tune `max_ops` and `timeout_seconds` for hard ceilings in CI or web usage.
