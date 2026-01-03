# Release readiness checklist

## Host formats
- Workbooks: .xlsx, .xlsm, .xltx, .xltm
- Power BI: .pbix, .pbit

## PBIX boundaries
- If PBIX has DataMashup, Power Query diffs are available.
- If PBIX has no DataMashup but has DataModelSchema, model diffs (measures) are available.
- If PBIX has neither, the tool should return a clear error.

## Outputs
- Text output: all DiffOp variants should be represented (at least a fallback line).
- JSON output: schema_version is present and stable.

## Limits / knobs
- max memory, timeout, max ops: documented and tested.
- When limits hit: report.complete=false and warnings populated.

## Determinism
- Parallel runs (different thread counts) produce identical JSON and text outputs.

## CI gates
- Unit tests + integration tests pass.
- Fuzz targets run in CI or scheduled workflow.
- wasm build stays under size budgets.
