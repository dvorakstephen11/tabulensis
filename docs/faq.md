# FAQ

[Docs index](index.md)

## Why does it say results are incomplete?

Diff results carry a `complete` flag and a list of `warnings`:

- `complete == true`: the engine believes it produced a full diff.
- `complete == false`: the engine returned a partial result (usually to avoid resource exhaustion) and includes at least one warning explaining why.

In the CLI, warnings are printed to stderr as `Warning: ...`.

## How do I diff huge files?

- Prefer streaming output:
  - CLI: `--format jsonl` (writes JSON Lines)
  - Rust: `diff_streaming` / `diff_database_mode_streaming` with a `DiffSink` (e.g., `JsonLinesSink`)
- Consider safety rails:
  - `--max-memory <MB>` / `DiffConfig.max_memory_mb`
  - `--timeout <SECONDS>` / `DiffConfig.timeout_seconds`

## Why are chart diffs "shallow"?

Chart changes are currently reported as add/remove/change at a high level (hash-based), not a deep structural diff of the underlying chart XML.

## How do I interpret error codes?

Error messages include stable error codes like `EXDIFF_PKG_001`. See:

- [Error codes](errors.md)
- Rust: `excel_diff::error_codes`

## Can I use this in the browser?

Yes. The repo includes:

- `wasm/`: WebAssembly bindings
- `web/`: a small web demo frontend
