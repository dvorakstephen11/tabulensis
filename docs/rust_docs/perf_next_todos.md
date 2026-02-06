# Perf Next TODOs (Shortlist)

Date: 2026-02-06

These are the next high-impact ideas to revisit soon (beyond `custom-xml`).

## 1) Stream Sheet XML Parse From ZIP (Avoid Full Part Materialization)

Goal:
- Reduce peak memory and potentially improve throughput by parsing worksheet XML directly from the ZIP entry reader instead of `read_to_end` into `Vec<u8>`.

Where:
- `core/src/container.rs` (expose a size-bounded ZIP entry reader API)
- `core/src/excel_open_xml.rs` (open workbook using streaming reads for sheets)
- `core/src/grid_parser.rs` (support `Read`-based parsing, or an incremental scanner)

Why:
- Large sheets create large temporary buffers; streaming can cap memory spikes and reduce allocator churn.

Risks:
- Parser API changes and more complex error reporting (need line/col or offset mapping).
- Need to preserve existing hardening limits (entry size, total bytes, etc).

Measurement:
- e2e: watch `peak_memory_bytes` and `parse_time_ms`.
- desktop: validate large-mode responsiveness on "mostly identical" workbooks where only a few sheets change.

## 2) Smarter “sharedStrings changed” Handling (Skip More Sheets Safely)

Goal:
- Today: if `xl/sharedStrings.xml` differs, fast diff conservatively parses all matched sheets.
- Improve: still skip sheets that provably do not reference shared strings (or whose relevant shared-string indices are unchanged).

Options (in increasing complexity):
- Lightweight scan: while reading a sheet part, detect whether any cell uses `t=\"s\"` (shared string) and only force-parse those sheets when sharedStrings differs.
- Hybrid fingerprint: compute a “sheet uses shared strings?” boolean cheaply per sheet and include it in the short-circuit logic.

Why:
- Many real edits touch sharedStrings even when only a subset of sheets actually depends on shared strings.

Risks:
- False negatives would be correctness bugs; any heuristic must be conservative.

Measurement:
- Add targeted fixtures: sharedStrings changes + a sheet with only numbers should be skipped; a sheet with `t=\"s\"` must still be parsed.

