# Desktop Tab-Switch Latency Plan (Summary <-> Details)

**Date:** 2026-02-09  
**Scope:** Native desktop UI (wx/XRC) results notebook in the Compare pane (`desktop/wx/ui/main.xrc`, `desktop/wx/src/main.rs`).  
**Goal:** Make switching between `Summary` and `Details` feel instant and never “hang” the UI, even after loading a large sheet payload.

## 1. Problem Statement

Users report noticeable latency when clicking between the `Summary` and `Details` tabs in the right-side results notebook (“display pane”) after running a diff and navigating sheets.

This plan focuses on the native UI path (not the WebView UI), where `Details` is backed by a large, read-only `wxTextCtrl` (`detail_text`) and content can be multi-megabyte JSON.

## 2. What Actually Happens On Tab Switch (Code-Path Map)

The results notebook is `result_tabs`:
- XRC: `desktop/wx/ui/main.xrc` defines pages: `Summary`, `Details`, `Explain`, `Preview`.
- Event binding: `desktop/wx/src/main.rs` installs `result_tabs.on_page_changed(...)`.

On `EVT_NOTEBOOK_PAGE_CHANGED`:
- If the selected page is `Details`: call `render_staged_detail_payload(ctx)`.
- If the selected page is `Preview`: call `render_grid_for_current_selection(ctx)`.
- `Summary` and `Explain` do not run a dedicated handler on focus.

### 2.1 The `Details` Focus Handler

`render_staged_detail_payload(ctx)`:
1. Reads `ctx.state.pending_detail_payload` and `pending_detail_*` generation counters.
2. If JSON was already rendered for the current generation (`pending_detail_json_gen == Some(gen)`):
   - Reads the *current* `detail_text` value via `ctx.ui.detail_text.get_value()`
   - Compares it to the cached JSON string
   - Calls `set_value()` only if it differs
3. If JSON is in-flight, sets `detail_text = "Rendering JSON..."`.
4. Otherwise, spawns a background thread that runs `serde_json::to_string_pretty(payload)` and, on completion, calls `detail_text.set_value(rendered_json)` on the UI thread.

## 3. Likely Causes (Prioritized) + Why

### Cause A (very likely): `detail_text.get_value()` copies multi-megabyte text on every Details focus

When cached JSON exists, the current code path calls `wxTextCtrl::GetValue()` to decide if it needs to reapply the text. In wxWidgets, `GetValue()` returns a new string (copy) of the control’s entire contents.

If `detail_text` holds a large JSON payload (common in Large mode), then:
- switching to `Details` repeatedly can incur a full copy of the string each time, even when nothing changed
- the cost scales linearly with the text size, producing the exact “small but noticeable” delay described

This is the highest ROI fix because it’s:
- directly on the tab-focus path
- avoidable entirely (we already have generation counters)
- independent of diff/parse performance

### Cause B (likely): `detail_text.set_value(huge_string)` stalls the UI thread when JSON rendering completes

Even with background serialization, `set_value()` must copy the rendered JSON into the native widget and may trigger:
- internal buffer reallocations
- line indexing / layout work
- repaint cost

This tends to manifest as a “freeze” when the JSON finishes rendering (not necessarily exactly at the moment of clicking the tab). If the user clicks between tabs while a large `set_value()` is pending, it can feel like tab switching is slow.

### Cause C (possible): wxNotebook repaint/layout overhead with heavy pages

The `Summary` page contains multiple nested sizers, panels, and virtual DataView tables. wxNotebook page switching can trigger show/hide + relayout + repaint work across the container.

This cause is plausible but should be investigated *after* fixing Cause A, because Cause A alone can fully explain a latency that scales with JSON size.

### Cause D (PBIP-specific, possible): multiple large text controls on Details page

In PBIP domain, `Details` includes:
- `detail_text`
- `pbip_old_text`
- `pbip_new_text`

If these contain large text, the aggregate repaint and memory traffic can increase. PBIP also updates `explain_text` on selection changes, which can contribute to perceived delays.

## 4. Research + Measurement Plan (Confirm, Don’t Guess)

### 4.1 Establish a reproducible repro case

- Use a release-ish build to avoid debug-only slowness:
  - `cargo run -p desktop_wx --profile release-desktop`
- Pick a diff that yields a large sheet payload in Large mode (large op count and/or grid content).
- Steps:
  1. Run Compare
  2. Select a heavy sheet
  3. Open `Details` once (let JSON render)
  4. Switch `Summary` <-> `Details` 10-20 times and observe latency

### 4.2 Add opt-in “UI perf probes” around tab switching and Details rendering

Implement a tiny instrumentation layer (behind an env var, e.g. `EXCEL_DIFF_PROFILE_UI=1`) that logs:
- tab switch handler duration: `page_changed -> handler done`
- `detail_text.get_value()` duration + current text length
- `detail_text.set_value()` duration + new text length
- JSON serialization duration + rendered JSON length

This will quickly answer:
- Is time dominated by `get_value`, `set_value`, notebook relayout, or something else?
- Does the latency scale with JSON size?

### 4.3 Optional: sampling profiler verification

On Linux, use `perf` to confirm CPU hotspots while hammering tab switching:
- Expect to see text widget string copy / GTK text operations if Cause A/B is real.

## 5. Implementation Plan (Fixes), Ordered By ROI

### P0: Make Details focus O(1) relative to existing text size

#### P0.1 Stop calling `detail_text.get_value()` in `render_staged_detail_payload`

Use the existing generation counters, plus one additional “applied-to-widget generation” marker, to determine whether the UI already has the correct text without reading it back from the widget.

Design:
- Add state:
  - `pending_detail_text_applied_gen: Option<u64>` (or `usize` matching `pending_detail_payload_gen`)
  - optionally track `pending_detail_text_applied_sheet: Option<String>` for extra safety
- When we call `detail_text.set_value(rendered_json)`:
  - set `pending_detail_text_applied_gen = Some(gen)`
- When we change `pending_detail_payload_gen` / clear the view:
  - set `pending_detail_text_applied_gen = None`
- On `Details` focus:
  - if `pending_detail_json_gen == Some(gen)` and `pending_detail_text_applied_gen == Some(gen)`:
    - do nothing (no `get_value`, no redundant `set_value`)

#### P0.2 Avoid “status spam” on focus when nothing changed

If the Details text is already applied for the current gen, avoid calling `update_status_in_ctx(...)` on every page focus. (Status updates are not likely the primary cause, but it’s cheap to make them conditional and keeps the UI calmer.)

#### P0.3 Freeze/Thaw around massive `set_value()` calls

When applying large JSON:
- call `detail_text.freeze()` (or equivalent if exposed by `wxdragon`)
- `set_value(rendered_json)`
- `detail_text.thaw()`

This reduces intermediate repaints and can lower perceived jank. If wxdragon doesn’t expose Freeze/Thaw for `TextCtrl`, add the wrapper in wxdragon or use `Window::freeze()` on the parent panel.

### P1: Size-aware Details UX to prevent pathological cases

Even after P0, very large payloads can still make `set_value()` and repaint slow. Address this with guardrails.

#### P1.1 Introduce a “large text policy” for Details

When rendered JSON exceeds a threshold (e.g. 1-2MB, tune via measurement):
- Default to a compact summary view:
  - sheet name
  - payload stats (op count, rows/cols if available)
  - JSON size
  - actions:
    - “Render in-app (slow)”
    - “Copy JSON”
    - “Write to file and open”

This keeps tab switching fast and gives power-users an explicit escape hatch.

#### P1.2 Add “Compact JSON” mode

Offer two render modes:
- Compact: `serde_json::to_string(payload)` (no pretty formatting)
- Pretty: `to_string_pretty`

Compact mode can be dramatically smaller and faster to apply to a text widget, often enough to eliminate perceived lag without removing information.

#### P1.3 Render to a temp file (preferred for huge payloads)

For very large payloads, the best UX may be:
- write JSON to `${temp}/tabulensis/<diff_id>/<sheet>.json`
- show the path + “Open” button
- keep the in-app text control showing a short summary

This also improves copy performance (clipboard operations on multi-MB text can be slow/failure-prone on some platforms).

### P2: Upgrade the Details viewer widget for large documents (if needed)

If P0/P1 still leave tab switching or Details viewing slow for real workloads:

#### P2.1 Replace `wxTextCtrl` with `wxStyledTextCtrl` (Scintilla)

Benefits:
- better performance for large text
- optional JSON syntax highlighting
- better navigation (search, goto line)

Costs:
- additional dependency and platform packaging considerations
- needs wrapping support in wxdragon if not already present

#### P2.2 Alternative: separate “Details” window

Keep notebook switching lightweight by moving heavy Details rendering into a separate modal/non-modal window opened on demand.

## 6. Validation Strategy (Don’t Regress UX)

- Manual acceptance:
  - Switching `Summary` <-> `Details` should feel instant after JSON is already rendered.
  - While JSON is rendering, switching tabs should remain responsive (no UI freezes).
- Instrumentation acceptance (with `EXCEL_DIFF_PROFILE_UI=1`):
  - “Details focus handler” should be ~constant time after the text is applied (no scaling with JSON length).
  - `get_value()` should not appear on the hot path (or should be absent entirely for Details focus).
- Regression:
  - `cargo run -p desktop_wx --bin xrc_smoke`
  - Run at least one deterministic UI snapshot scenario (no layout breaks):
    - `./scripts/ui_capture.sh compare_grid_basic --tag <tag>`

## 7. Task Checklist (Actionable)

- [ ] Add `EXCEL_DIFF_PROFILE_UI=1` instrumentation for result tab switching and Details render (`desktop/wx/src/main.rs`).
- [ ] Reproduce the latency on a known-large sheet payload and capture probe logs (record JSON length + timings).
- [ ] Implement P0.1 generation-based “applied text” marker; remove `detail_text.get_value()` from Details focus path.
- [ ] Make status updates conditional (avoid work on focus when no changes occurred).
- [ ] Add Freeze/Thaw (or equivalent) around applying large JSON to `detail_text`.
- [ ] Re-test: switch tabs repeatedly after JSON is rendered; verify probe logs show constant-time focus.
- [ ] Add size thresholds + large-text policy (summary + explicit actions) for Details.
- [ ] Add Compact vs Pretty render mode toggle for JSON.
- [ ] Implement “write JSON to temp file + open” flow for huge payloads.
- [ ] If still slow: evaluate `wxStyledTextCtrl` migration or separate Details window.

