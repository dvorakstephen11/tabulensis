# Tabulensis Product Summary

This document provides an executive summary of the features and capabilities in the Tabulensis codebase.

## Product overview

Tabulensis is a structured diff engine for Excel workbooks and Power BI packages. It compares two files and produces a detailed change report covering grid-level edits, object changes, and Power Query (M) differences.

## Supported inputs

- Excel workbooks: `.xlsx`, `.xlsm`, `.xltx`, `.xltm`
- Power BI packages: `.pbix`, `.pbit`
- `.xlsb` is detected but not supported and returns a structured error with a conversion hint.

## Core diff capabilities

- Grid diffing per sheet: alignment, move detection, and cell edits
- Sheet structure changes (row/column add/remove and block moves)
- Object diffs for named ranges, charts, and VBA modules (chart diffs are shallow)
- Power Query (M) diffing when DataMashup content is present
- Semantic diff toggles for M and formulas
- Database mode (key-based row alignment) for table-like sheets, including auto-detected composite keys

## Output and integration

- CLI output formats: text, JSON, JSONL, UI payload, and outcome envelope
- Unified diff output for Git tooling via `--git-diff`
- `tabulensis info` produces a stable text summary suitable for Git textconv
- Streaming diff APIs via `DiffSink` (e.g., JSONL) for large files

## Performance and safety controls

- Presets: fastest, balanced (default), and most precise
- Memory and time limits with controlled fallbacks (`max_memory`, `timeout`)
- Alignment limits with configurable behavior on limit exceeded
- ZIP/OPC container limits and zip bomb protections
- `complete` flag plus warnings for partial results

## Platforms and delivery surfaces

- CLI binary: `tabulensis`
- Rust library: `excel_diff` crate (`WorkbookPackage`, `DiffConfig`, streaming sinks)
- WebAssembly bindings and browser demo (local processing in the browser)
- Prebuilt binaries for Windows and macOS; source builds via Rust nightly

## Licensing and entitlement

- Client-side activation required to run diffs
- Environment variables support offline mode and local token use
- Minimal licensing backend included in `license_service/` (Stripe or mock mode)

## Diagnostics and error handling

- Stable error codes across package, grid, container, DataMashup, and diff layers
- Context-rich error reporting (file path, part path, and XML position when available)
- Exit codes distinguish identical, different/incomplete, and error states

## Known limitations

- `.xlsb` not supported
- PBIX/PBIT support is limited to legacy DataMashup extraction; tabular-only PBIX files return a specific error
- DataMashup permission bindings that cannot be validated default permissions and mark results incomplete
- Chart diffs are hash-based rather than deep structural diffs

## Testing and fixtures

- Rust test suite via `cargo test`
- End-to-end tests via `python scripts/dev_test.py`
- Fixture generation pipeline under `fixtures/`
