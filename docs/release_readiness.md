# Release Readiness

This checklist covers the branch-7 release readiness items.

## PBIX/PBIT support limits

- Only legacy DataMashup-based extraction is supported.
- If a PBIX has no DataMashup (Tabular-only model), the diff returns
  `NoDataMashupUseTabularModel` with error code `EXDIFF_PKG_010`.

## Semantic M diff behavior

- `DiffConfig.enable_m_semantic_diff` defaults to `true`.
- The CLI `--fast` preset sets it to `false`, so semantic M detail is disabled when
  `--fast` is used.
- Use the default or `--precise` presets to include semantic M diffs.

## Resource ceilings and limits

- `DiffConfig.max_memory_mb` enables a memory guard and can force a fallback to
  positional diff with a warning.
- `DiffConfig.timeout_seconds` aborts the diff and returns a partial report with
  warnings.
- Alignment limits (`max_align_rows`, `max_align_cols`) are enforced; behavior is controlled
  by `DiffConfig.on_limit_exceeded` (error vs partial result).

## Release checklist

- [ ] PBIX/PBIT: legacy DataMashup extraction verified; Tabular-only PBIX returns
  `EXDIFF_PKG_010`.
- [ ] Semantic M diff: default includes semantic detail; `--fast` disables it.
- [ ] Resource ceilings: `--timeout` aborts cleanly with partial warnings; `--max-memory`
  triggers memory guard and positional fallback.
- [ ] Perf gates: quick suite baseline + caps pass; full-scale scheduled job green.
