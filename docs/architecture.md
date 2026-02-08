# Architecture Overview

[Docs index](index.md)

This document gives a high-level tour of how Tabulensis is structured and where to look in the codebase.

## Big picture

```text
           .xlsx/.xlsm
                |
                v
         ZIP/OPC container
        (OpcContainer + limits)
                |
                v
         Workbook IR (core)
     Workbook -> Sheet -> Grid
                |
                +------------------+
                |                  |
                v                  v
        DataMashup (PQ)        VBA modules
       (optional parse)        (optional parse)
                |
                v
            Diff engine
   grid diff + object diff + M diff
                |
                v
      DiffReport (in-memory) OR
  DiffSink + DiffSummary (streaming)
                |
                v
       CLI output (text/json/jsonl/git)
```

## Inputs

Excel workbooks are ZIP/OPC containers. Tabulensis opens them through `OpcContainer` and applies safety limits (`ContainerLimits`) to protect against oversized or malicious archives.

## Parsing pipeline

1. Open the container and parse workbook parts.
2. Build the workbook IR:
   - `Workbook`, `Sheet`, `Grid`, `CellValue`, etc.
3. If present, parse Power Query DataMashup content into `DataMashup` / `Query` structures.

## Diff pipeline

1. **Grid diff**: alignment + move detection + cell edits (sheet-by-sheet).
2. **Object diffs**: named ranges, charts, and VBA modules.
3. **M diffs**: query add/remove/rename and definition/metadata changes (semantic vs formatting-only when enabled).

## Output pipeline

You can choose between:

- **In-memory**: `DiffReport` (all ops collected in memory).
- **Streaming**: implement `DiffSink` and use `diff_streaming` to emit ops incrementally, returning a `DiffSummary`.

The CLI builds on these and supports:

- `text`: human-readable summary
- `json`: full `DiffReport`
- `jsonl`: JSON Lines (header + one op per line)
- `--git-diff`: unified diff style output

## UI surfaces (Desktop / Web)

Tabulensis ships multiple hosts that all consume the same `core/` diff results:

- **Desktop (wxDragon)**: a native shell (`desktop/wx/`) that calls into `desktop/backend/` to run diffs, load summaries/ops ranges, and build UI payloads.
- **Web demo**: a browser UI (`web/`) that runs the WASM build (`wasm/`) in a worker and renders the resulting UI payload.

### Shared UI payloads (`ui_payload/`)

The `ui_payload/` crate exists to keep desktop/web aligned on the "shape" of UI-ready results:

- `ui_payload::DiffWithSheets`: report + sheet metadata needed for sheet list, details, and grid preview.
- `ui_payload::DiffOptions`: host-facing config overrides (preset, limits, trust, semantic toggles).
- `ui_payload::NoiseFilters` + `ui_payload::DiffAnalysis`: deterministic post-processing used to make results scannable (categories/severity + filtering/collapsing in the UI).

### Iteration 1: Profiles, Noise Filters, Explain

Iteration 1 introduced workflow-level controls and first-pass explanation:

- **Profiles** (desktop): `desktop/wx/src/profiles.rs` persists user profiles that bundle:
  - engine-facing settings (preset + semantic toggles + limits),
  - UI-facing noise filters (`ui_payload::NoiseFilters`),
  - and some UI state (selected profile id).
- **Noise filters** are UI post-processing (they do not change the diff engine output); they influence how summaries and lists are computed/displayed.
- **Explain** (desktop): `desktop/wx/src/explain.rs` builds a best-effort, deterministic explanation string for a clicked cell by scanning relevant `DiffOp`s (plus a small set of global ops as context).
