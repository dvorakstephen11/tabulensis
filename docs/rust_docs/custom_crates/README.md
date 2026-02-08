# Custom Crate Experiments

This directory is the canonical home for Rust custom-crate experiment documentation.
All root-level custom-crate experiment docs were moved here on 2026-02-05.

## Document Index

| File | Focus | Current Role |
| --- | --- | --- |
| `docs/rust_docs/custom_crates/custom_crate_code_experiment.md` | Candidate inventory + methodology | Master candidate list |
| `docs/rust_docs/custom_crates/base64_custom_crate_experiment.md` | Base64 replacement experiments | Historical combined log |
| `docs/rust_docs/custom_crates/lru_custom_crate_code_experiment.md` | Desktop backend tiny LRU plan | Historical guidance |
| `docs/rust_docs/custom_crates/lru_custom_crate_code_experiment_part_2.md` | LRU follow-up analysis | Historical guidance |
| `docs/rust_docs/custom_crates/arc_cache_custom_crate_experiment.md` | Arc-cache and key reuse framing | Historical guidance |
| `docs/rust_docs/custom_crates/desktop_arc_cache_and_key_reuse_experiment.md` | Desktop arc cache implementation plan | Historical guidance |
| `docs/rust_docs/custom_crates/agentic_experiment_playbook.md` | Execution protocol for agents | Active workflow guide |
| `docs/rust_docs/custom_crates/next_experiment_custom_xml.md` | Next-run candidate definition | Active experiment log (Slices 1-2 in progress) |
| `docs/rust_docs/custom_crates/datamodel_schema_custom_json_experiment.md` | DataModelSchema JSON parser (PBIX/PBIT) | Shipped (default-on) |

## Next Recommended Experiment

Active candidate: `custom-xml` (targeting `quick-xml` hot paths in `core/src/grid_parser.rs`, currently scoped to shared strings + worksheet cell/value scanning).

Why this is the next best bet:
- Workbook-open e2e is parse-dominated in recent runs, so parser-path changes can move top-line latency.
- `custom_crate_code_experiment.md` ranks `quick-xml` first by expected impact.
- A narrow, feature-gated parser experiment can be isolated and measured safely.

Status (February 7, 2026):
1. Slice 1 (shared strings) is implemented behind `custom-xml` and has a 5-run confirmation win.
2. Slice 2 (worksheet cell/value scan loop) is implemented behind `custom-xml`.
3. Slice 2 has a 5-run confirmation win (see the log doc for exact numbers).
4. With the OpenXML fast-diff path in place, `custom-xml` still shows a large median win on non-identical e2e cases (see the log doc).
5. A full perf-cycle run (`2026-02-07_221949`) shows large, high-confidence wins with `custom-xml` default-on in `excel_diff` (see the log doc).
6. Next gate is real-world workbook corpus and/or fuzz coverage before declaring the custom parser fully "done".

Suggested first slice:
1. Keep `quick-xml` baseline as default.
2. Add `custom-xml` flag for one constrained path (`sharedStrings.xml` parser only).
3. Run A/B parity and perf with matched sample size and fixtures.
4. Track the run in `docs/rust_docs/custom_crates/next_experiment_custom_xml.md`.

## Rules of Engagement

- Store each experiment's iteration logs in the candidate doc (commands, SHA, metrics, deltas).
- Keep feature-flag matrix explicit (`baseline` vs `custom` vs optional parity).
- Update this index after each iteration with status and follow-up action.
