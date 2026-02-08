# Maintainer Entry Points

This document maps the main entry points across the codebase so new changes can be scoped fast.

## Core library (public API)

- `core/src/package.rs`: `WorkbookPackage::open`, `WorkbookPackage::diff`, and streaming variants for workbook and PBIX.
- `core/src/config.rs`: `DiffConfig` and hardening/config builders.
- `core/src/session.rs` + `core/src/string_pool.rs`: `DiffSession`, `StringPool`, and `StringId` (also `with_default_session` in `core/src/lib.rs`).
- `core/src/sink.rs`: `DiffSink` trait and lifecycle enforcement; streaming sinks in `core/src/output/json_lines.rs`.

## CLI

- `cli/src/commands/diff.rs`: primary diff command path (opens packages, selects format, runs diff).
- `cli/src/commands/host.rs`: `HostKind` selection + open helpers for workbook vs PBIX.
- `cli/src/output/`: rendering to text/JSON/JSONL and git diff formatting.

## WASM

- `wasm/src/lib.rs`: exported diff functions; host selection via `ui_payload::host_kind_from_name`; memory cap in `wasm_default_config`.
- `ui_payload/src/lib.rs`: host-kind helpers + UI payload builders used by web/desktop.

## Desktop (wxDragon)

- `desktop/wx/src/main.rs`: desktop app orchestration + event handlers + UI state and rendering flow.
- `desktop/wx/src/ui.rs`: XRC load + widget binding + theming (builds `UiHandles`).
- `desktop/wx/src/ui_constants.rs`: shared desktop UI constants (column layouts, sizes, guided-empty strings).
- `desktop/wx/src/profiles.rs`: profile persistence + built-in profiles.
- `desktop/wx/src/profiles_dialog.rs`: profiles actions (save/rename/delete/import/export).
- `desktop/wx/src/explain.rs`: "Explain" text builder for selected cells (best-effort, deterministic).
- `desktop/backend/src/diff_runner.rs`: diff orchestration, store integration, and progress events.
- `desktop/backend/src/store`: persisted op storage + diff summaries.
- Run from source: `cargo run -p desktop_wx --bin desktop_wx` (use `--profile release-desktop` for optimized builds).
- The `desktop_wx` package ships multiple binaries (including `xrc_smoke`), so `--bin` is required when using `cargo run`.

## Web

- `web/diff_worker.js`: web worker that initializes WASM and runs diffs.
- `web/main.js`: UI orchestration + client selection.
- `web/platform.js`: browser worker selection for the web demo.

## If you're changing X, start here

- Alignment/moves: `core/src/engine/grid_diff.rs`, `core/src/engine/move_mask.rs`, `core/src/alignment/`.
- Streaming behavior: `core/src/sink.rs`, `core/src/output/json_lines.rs`, `core/src/package.rs`.
- Parsing: `core/src/excel_open_xml.rs`, `core/src/grid_parser.rs`, `core/src/datamashup_framing.rs`, `core/src/datamashup_package.rs`.
