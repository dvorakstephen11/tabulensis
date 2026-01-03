# Host Parity Matrix

This document defines the host parity contract for excel_diff and the invariants we enforce in CI.

## Invariants (must always hold)

- Core builds for `wasm32-unknown-unknown` with the minimal feature set (`engine-wasm`).
- Preset names (`fastest`, `balanced`, `most_precise`) map to the same tuning in every host.
- Outcome `mode` semantics are consistent:
  - `payload`: full report + payload (snapshots + alignments where available).
  - `large`: summary-only artifact with details streamed or stored.

## Intended feature sets

- `engine-host`: core default features + `model-diff` (CLI adds `parallel`).
- `engine-wasm`: `--no-default-features --features "excel-open-xml,model-diff"`.

## Parity matrix

| Surface | CLI | Desktop (Tauri) | Web/WASM |
| --- | --- | --- | --- |
| Input kinds | XLSX/XLSM/Xltx/Xltm + PBIX/PBIT | XLSX/XLSM/Xltx/Xltm + PBIX/PBIT | XLSX/XLSM/Xltx/Xltm + PBIX/PBIT |
| Config surface | Presets + limits + hardening flags | Presets + limits + trusted | Presets + limits (host defaults: max memory 256MB) |
| Output surface | `text`, `json` (DiffReport), `jsonl`, `payload`, `outcome` | `DiffOutcome { diffId, mode, payload?, summary?, config? }` | `DiffOutcome` JSON from WASM |
| Large mode policy | Auto-switch to JSONL when `should_use_large_mode` | `mode=large` for workbooks via cell-volume estimate; PBIX streams to store and uses `mode=large` when op count exceeds threshold | `mode=large` for workbooks via cell-volume estimate; PBIX stays payload |
| Streaming policy | JSONL | SQLite op store (payload on demand) | JSONL download for large mode; otherwise payload |
| Versioning/schema | DiffReport `version` is canonical; payload/outcome embed report | Same | Same |
| Feature gates | `vba`, `model-diff`, `parallel`, `std-fs` | `vba`, `model-diff` | `model-diff` (no `vba`, no `std-fs`, no `parallel`) |

## Capability reporting

- CLI: `excel-diff --version --verbose` prints feature flags + thresholds.
- WASM: `get_capabilities()` returns `HostCapabilities`.
- Desktop: `get_capabilities` Tauri command returns `HostCapabilities`.

## CI parity gates

- Build matrix checks:
  - `cargo check -p excel_diff`
  - `cargo check -p excel_diff --no-default-features --features "excel-open-xml,model-diff"`
  - `cargo check -p ui_payload`
  - `cargo check -p excel_diff_wasm --target wasm32-unknown-unknown`
  - `cargo check -p excel_diff_desktop`
- Feature audit (`cargo tree -p excel_diff -e features`):
  - default features
  - `--no-default-features --features "excel-open-xml,model-diff"`
  - `--no-default-features --features "excel-open-xml"`
  - `--no-default-features --features "model-diff"`
- Schema compatibility: web tests load CLI payload/outcome fixtures with `buildWorkbookViewModel` and `renderReportHtml`.
- Large-mode regression: tests assert `mode=large`/JSONL when the threshold is exceeded.
