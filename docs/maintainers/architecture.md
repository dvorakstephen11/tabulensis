# Architecture: Parse -> IR -> Diff -> Output

This narrative describes how bytes flow through the system using real types and files.

## Parse (bytes -> structured meaning)

- Containers and bounds: `core/src/container.rs` enforces ZIP/OPC safety and limits.
- OpenXML workbook parsing: `core/src/excel_open_xml.rs` + `core/src/grid_parser.rs` build `Workbook` data.
- DataMashup framing and package parsing: `core/src/datamashup_framing.rs` and `core/src/datamashup_package.rs` split and decode the QDEFF stream.
- DataMashup semantics: `core/src/datamashup.rs` and `core/src/m_section.rs` build query/metadata structures.

## IR (intermediate representation)

IR is the internal, minimal model used by the diff engine.

- `WorkbookPackage` (in `core/src/package.rs`) is the top-level container for workbook + optional DataMashup/VBA/model data.
- `Workbook`, `Sheet`, `Grid`, `Cell`, and `CellValue` live in `core/src/workbook.rs` and represent sheet contents and metadata.
- `StringPool` + `StringId` (in `core/src/string_pool.rs`) intern strings for compact, deterministic reports.

## Diff (IR -> operations)

- The diff engine orchestrates grid/object/M diffs and emits `DiffOp` operations (`core/src/diff.rs`).
- Grid/sheet/workbook diff entry points live in `core/src/engine/` (`grid_diff.rs`, `sheet_diff.rs`, `workbook_diff.rs`).
- Move/alignment logic sits in `core/src/engine/move_mask.rs` and `core/src/alignment/`.
- Output can be collected (`DiffReport` in `core/src/diff.rs`) or streamed via `DiffSink` (`core/src/sink.rs`).

## Output (operations -> serialized/UI)

- JSON report serialization: `core/src/output/json.rs`.
- JSONL streaming: `core/src/output/json_lines.rs` and the `DiffSink` contract in `docs/streaming_contract.md`.
- Host-specific formatting:
  - CLI uses `cli/src/output/`.
  - Desktop/Web build UI payloads in `ui_payload/src/lib.rs`.

## Notes on determinism and hardening

- Deterministic ordering and streaming lifecycle are defined in `docs/streaming_contract.md`.
- Hardening controls (timeouts, memory, op caps) are configured via `DiffConfig` in `core/src/config.rs`.
