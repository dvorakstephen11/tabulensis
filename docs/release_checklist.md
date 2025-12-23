# Release checklist

- Run `cargo test --workspace`.
- Run the fuzz workflow locally or verify the scheduled fuzz job is green.
- Verify WASM size budgets (web demo and wasm_smoke).
- Verify web demo deployment via the pages workflow.
- Manual smoke tests:
  - .xlsx with Power Query M (step diffs render).
  - .pbix/.pbit with DataMashup (query diffs render).
  - .pbix/.pbit without DataMashup (measure diffs render or actionable error).
