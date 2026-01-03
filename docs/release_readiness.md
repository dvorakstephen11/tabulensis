# Release readiness checklist

## Host formats
- Workbooks: .xlsx, .xlsm, .xltx, .xltm
- Power BI: .pbix, .pbit
- Unsupported (detected): .xlsb returns EXDIFF_PKG_009 with a convert hint

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
- Permission bindings that cannot be validated default permissions and emit EXDIFF_DM_009.

## Determinism
- Parallel runs (different thread counts) produce identical JSON and text outputs.

## CI gates
- Unit tests + integration tests pass.
- Fuzz targets run in CI or scheduled workflow.
- wasm build stays under size budgets.
