# Architecture Debt Plan (Post Iteration 1)

This document tracks the immediate architecture cleanup that became necessary after Iteration 1
introduced profiles, noise filters, summary analysis, and the first "Explain" surface.

Iteration 1 correctly prioritized shipping workflow value; this follow-up focuses on making the
new desktop UX code easier to evolve without regressions.

## Goals

- Reduce the maintenance/regression risk of the wx desktop app by splitting responsibilities into
  smaller, obviously-owned modules.
- Make the docs accurately describe the desktop/web surfaces and the Iteration 1 data flow
  (profiles, noise filters, analysis, explain).
- Keep behavior unchanged (this is an architecture + docs pass, not a UX redesign).

## Non-goals (This Pass)

- Changing `core/` parsing/diff/alignment logic.
- Changing persisted schema for diff stores or UI payloads.
- Redesigning the desktop UI layout/XRC.

## Findings (Suboptimalities)

- `desktop/wx/src/main.rs` was effectively an entire application in one file: UI construction,
  widget lookup, theming, persistence, RPC orchestration, handlers, and explanation logic.
  This is a high-risk change surface (small edits tend to cause accidental regressions).
- `docs/architecture.md` and `docs/maintainers/entrypoints.md` described the core pipeline but did
  not cover the Iteration 1 surface area (profiles/noise filters/analysis/explain) or where those
  features live in code.

## Plan (Phase 1: Implemented Here)

Desktop wx modularization:

- [x] Extract UI constants into `desktop/wx/src/ui_constants.rs`.
- [x] Extract "Explain" text builder into `desktop/wx/src/explain.rs`.
- [x] Extract the profiles action dialog into `desktop/wx/src/profiles_dialog.rs`.
- [x] Extract XRC load + widget binding + theming into `desktop/wx/src/ui.rs` and build `UiHandles`
      via `ui::build_ui_handles(...)`.

Documentation updates:

- [x] Update `docs/architecture.md` to include desktop/web surfaces and Iteration 1 data flow.
- [x] Update `docs/maintainers/entrypoints.md` to include the new desktop wx modules.
- [x] Link this doc from `docs/index.md`.
- [x] Refresh the docs checklist index (`python3 scripts/update_docs_index_checklists.py`).

Validation:

- [x] `cargo check -p desktop_wx`
- [x] `cargo test -p desktop_wx`
- [x] Perf cycle post-run complete + delta recorded (cycle: `benchmarks/perf_cycles/2026-02-08_162446/`).

Perf artifacts:

- Delta summary: `benchmarks/perf_cycles/2026-02-08_162446/cycle_delta.md`
- Noise-aware signal report: `benchmarks/perf_cycles/2026-02-08_162446/cycle_signal.md`

## Future Opportunities (Not Implemented Here)

These are real suboptimalities, but intentionally deferred to keep this pass safe and bounded:

- Web demo remains a large surface area (`web/main.js` is still big despite existing modularization).
  A follow-up could split "profile persistence" and "diff orchestration" concerns further and add
  a tighter test harness around profile/noise-filter behavior.
- Consider consolidating some shared "Explain"/analysis heuristics into `ui_payload/` (or a sibling
  crate) so desktop/web stay in sync over time, while keeping UI rendering concerns separate.
