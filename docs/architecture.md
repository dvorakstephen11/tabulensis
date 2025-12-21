# Architecture Overview

[Docs index](index.md)

This document gives a high-level tour of how Excel Diff is structured and where to look in the codebase.

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

Excel workbooks are ZIP/OPC containers. Excel Diff opens them through `OpcContainer` and applies safety limits (`ContainerLimits`) to protect against oversized or malicious archives.

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
