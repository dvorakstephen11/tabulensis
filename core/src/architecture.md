# Architecture map (Phase 7)

This is a short internal map of layer ownership to keep refactors aligned with
the desired dependency direction.

Layer map (internal reference):
- container layer: core/src/container.rs
- parse layer: core/src/excel_open_xml.rs, core/src/grid_parser.rs, core/src/datamashup_framing.rs
- IR layer: core/src/workbook.rs, core/src/vba.rs, core/src/string_pool.rs
- diff layer: core/src/engine/, core/src/diff.rs, core/src/object_diff.rs, core/src/m_diff.rs
- output adapters: core/src/output/
- orchestration: core/src/package.rs

Mechanical guardrail:
- scripts/arch_guard.py enforces parse -> diff/package and diff -> parse/container boundaries.

Artifact IR audit notes (Phase 7):
- ChartObject carries xml_hash as a stable fingerprint; accepted as IR.
- NamedRange fields are raw names/refs; no diff-specific preprocessing in parse.
- VBA modules store raw code; normalization and comparisons happen in object_diff.
