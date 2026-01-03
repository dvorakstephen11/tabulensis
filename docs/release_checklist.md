# Release checklist

- Run `cargo test --workspace`.
- Run the fuzz workflow locally or verify the scheduled fuzz job is green.
- Verify WASM size budgets (web demo and wasm_smoke).
- Verify web demo deployment via the pages workflow.
- Manual smoke tests:
  - .xlsx with Power Query M (step diffs render).
  - .pbix/.pbit with DataMashup (query diffs render).
  - .pbix/.pbit without DataMashup (measure diffs render or actionable error).
  - .xlsb input returns `EXDIFF_PKG_009` with a "convert to .xlsx/.xlsm" hint.
  - Permission bindings warning (`EXDIFF_DM_009`) defaults permissions and marks results incomplete.
