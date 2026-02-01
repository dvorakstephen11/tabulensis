Below is a concrete plan that (a) keeps **all feature logic unified** while you split web UI vs desktop UI, (b) pushes **low‑hanging fruit first**, and (c) fixes the specific desktop issues you pasted (Gdk cursor theme warnings + XRC “spacer” handler errors) .

---

## 1) The architectural rule that makes “two UIs, one app” actually work

If you want the desktop UI to be maximally excellent *and* avoid logic drift between web and desktop, adopt one non‑negotiable:

**All user-visible behavior lives in a single “engine” layer.**
Desktop + Web are just **renderers** that:

* dispatch **Commands**
* receive **Events**
* bind **ViewModels** to widgets

### 1.1 Target layering (Rust-friendly, testable, avoids duplication)

You already have a shape close to this (core + desktop backend + ui_payload) based on your build logs . Formalize it:

**A) `tabulensis_core` (pure domain)**

* deterministic logic only: parsing, diffing, normalization, rules, indexing, export transforms
* no UI concepts, no threads, no file dialogs
* heavy unit + property tests live here

**B) `tabulensis_engine` (application / use-cases)**

* orchestrates workflows: “open left/right”, “compare”, “apply filters”, “export”
* owns authoritative state: current session, options, caches, progress, cancellation
* emits events suitable for *any* UI
* the only place allowed to implement “what happens when user clicks X”

**C) `ui_payload` (shared protocol + DTOs)**

* `Command`, `Event`, `Query`, `Response`, `ErrorPayload`
* should be stable + versioned (even if only internally)

**D) Adapters**

* `desktop_backend`: filesystem, OS integration, native dialogs, clipboard, etc.
* `web_backend`: HTTP/WebSocket adapters, auth, etc.

**E) UI renderers**

* `desktop_wx`: wx widgets, layout, input handling, binding
* `web_ui`: React/Svelte/… (completely separate, calling the same protocol)

### 1.2 Make “unified behavior” enforceable (not aspirational)

Low-effort guardrails that prevent drift:

* **Contract tests** for the protocol:

  * feed commands → assert emitted events & resulting state match snapshots (“golden”)
* **One “feature spec” per workflow** stored as tests:

  * Example: open files → compare → filter → export should always produce identical export bytes given same inputs and options.
* **No direct core calls from UI crates**:

  * UI crates can only call the engine interface (or protocol), not `tabulensis_core` directly.

---

## 2) Desktop UI excellence: what “maximally excellent” means in practice

“High-fidelity desktop-native” usually wins on:

* **latency** (instant interactions)
* **keyboard** (fast power-user workflows)
* **large-data handling** (virtualized tables/grids)
* **native integration** (menus, drag/drop, file associations)
* **polish** (layout, theming, accessibility)

So the plan below is oriented around those strengths—starting with quick wins.

---

## 3) Implementation plan (low-hanging fruit first)

### Phase 0 — Stop the bleeding: clean startup, clean logs, deterministic UI boot

These are the cheapest changes that immediately improve dev velocity and perceived quality.

#### 0.1 Fix XRC “spacer” handler errors (your immediate issue)

You’re seeing:

* `no handler found for XML node "object" (class "spacer")`
* `unexpected item in sizer` 

**Root cause (most likely):** the XRC system hasn’t registered the handler(s) that understand spacer nodes before you load the XRC.

**Fix path A (preferred): init all XRC handlers before loading resources**
In wxWidgets terms, you want the equivalent of:

```cpp
wxXmlResource::Get()->InitAllHandlers();
wxInitAllImageHandlers(); // if you use bitmaps/icons in XRC
wxXmlResource::Get()->Load(...);
```

In your Rust/wxDragon layer, do the same *before* `Load...()`.

**Also add a “fail fast” rule:** if XRC load returns errors, abort startup and show a dialog with “broken UI resources” instead of limping along.

#### 0.2 If InitAllHandlers still doesn’t pick up spacers: explicitly add spacer/sizer handlers

Some builds (or wrappers) effectively omit handlers unless referenced.

In wxWidgets C++ the explicit version looks like:

```cpp
#include <wx/xrc/xh_sizer.h>
wxXmlResource::Get()->AddHandler(new wxSizerXmlHandler);
wxXmlResource::Get()->AddHandler(new wxSpacerXmlHandler);
```

If wxDragon doesn’t expose this directly, the pragmatic fix is:

* add a tiny C++ shim in your wxdragon-sys layer that calls these functions
* bind it to Rust as `xrc_init_handlers()` and call it once at startup

#### 0.3 Add an automated “XRC loads” smoke test

This is *extremely* low effort and prevents regressions:

* a test binary (or `#[test]` behind a feature flag) that:

  * initializes wx
  * calls your XRC init
  * loads all XRC files
  * asserts “no XRC errors logged”

This catches mismatched XRC schemas instantly in CI.

---

### Phase 1 — Remove the desktop/web coupling without losing shared logic

You said: separate web UI and desktop UI, but unify feature logic.

#### 1.1 Define the canonical engine API (even if desktop calls it in-process)

A simple model that works great:

* UI thread sends `Command`
* engine runs in background (or same thread if cheap)
* engine emits `Event` stream:

  * progress
  * state updates
  * results deltas
  * errors (structured)

This supports:

* desktop UI (in-process)
* web UI (server process + websocket)
* tests (run engine headless)

**Low-hanging fruit:** use the *same* `ui_payload` in both. You already have a crate named `ui_payload` in your build log —lean into it.

#### 1.2 Decide what’s stateful vs derived

To keep UIs thin, the engine should own:

* current “session” (inputs, options, computed diff index, filters)
* export settings
* caching (e.g., computed sheet summaries)
* long-running tasks + cancellation tokens

The UI should own:

* widget state that doesn’t affect meaning (scroll position, column widths)
* purely presentational state (expanded nodes in tree)

---

### Phase 2 — Desktop skeleton that feels native immediately (quick wins)

This phase is about “it already feels like a real desktop app” even before advanced features.

#### 2.1 App chrome: menus, shortcuts, status, recent files

Low effort, huge payoff.

* **Menu bar** with standard items:

  * File: Open Left, Open Right, Open Pair…, Recent, Export…, Quit
  * Edit: Copy, Find
  * View: Toggle panels, Reset layout
  * Help: About, Docs
* **Keyboard shortcuts** (platform-aware):

  * Ctrl/Cmd+O (open), Ctrl/Cmd+F (find), Ctrl/Cmd+S (export)
  * F6/F8 (next difference), Shift+F8 (prev)
* **Status bar**:

  * current operation (“Comparing…”)
  * row count (“1,284 diffs”)
  * filter summary (“Filtered: 12% hidden”)

#### 2.2 Window layout: choose a strong default (don’t over-engineer)

A solid default for a diff-heavy app:

* **Top toolbar**: Open Left, Open Right, Compare, Export, Search
* **Left panel**: inputs + options + summary
* **Main panel**: results table (diff list)
* **Right/bottom panel**: detail inspector / preview of selected change

Use `wxSplitterWindow` for low complexity initially; you can graduate to AUI docking later.

#### 2.3 First-pass rendering controls (choose scalable widgets now)

* Summary: `wxTreeCtrl` or `wxDataViewTreeCtrl`
* Diff list: `wxDataViewCtrl` with a **virtual model** (this matters for performance)
* Detail: simple `wxPanel` with read-only text; later upgrade to grid preview

---

### Phase 3 — Make it *fast* and *powerful* (where native desktop shines)

This is where you surpass a web UI.

#### 3.1 Virtualization + incremental loading

If diffs can be large:

* never push all rows into the widget at once
* implement:

  * virtual list model
  * background indexing
  * progressive rendering (“first 500 diffs ready…”)

#### 3.2 Instant search + filter UX

* search box with:

  * tokenized filters (`sheet:Summary type:value_changed`)
  * highlight matches
  * debounce on keypress (but keep it snappy)
* filters should be **engine-owned**:

  * UI sends `SetFilter(FilterSpec)`
  * engine responds with `DiffListChanged { visible_count, … }`

That guarantees identical behavior in both UIs.

#### 3.3 Diff navigation & selection semantics

Make keyboard navigation excellent:

* next/prev diff
* jump to next diff in same sheet
* “pin” selection while background tasks run
* consistent selection even when filters change (stable IDs)

This is mostly engine + view model work, not widget work.

---

### Phase 4 — High-fidelity “desktop polish”

This is the difference between “it works” and “it feels premium”.

#### 4.1 Theming and DPI correctness

* Respect system fonts and dark mode where possible.
* Use SVG or multi-resolution icons (`wxBitmapBundle` style approach).
* Test at 100/125/150/200% scaling.
* Ensure consistent spacing: define a tiny “design tokens” module:

  * `SPACING_SM`, `SPACING_MD`, `SPACING_LG`
  * standard margins around panels
  * consistent row heights

#### 4.2 Accessibility and UX finishing

* full keyboard access to every control
* visible focus rings
* high-contrast safe coloring (don’t rely on red/green only)
* screen-reader friendly labels (where supported)

#### 4.3 State persistence

* restore:

  * last opened pair (optional)
  * window size/position
  * splitter positions
  * last used filters
* but **don’t** persist ephemeral bugs (provide “Reset layout”)

---

### Phase 5 — Packaging, updates, crash diagnostics

This is part of “excellent” desktop UX.

* installers per OS
* auto-update strategy (or at least “check for updates”)
* crash report capture (even if only local file)
* dependency bundling strategy (especially important if using webview backends)

---

## 4) Fixing your current desktop issues (specific, actionable)

### 4.1 XRC `spacer` handler errors

You’re getting XRC “no handler… class spacer” + “unexpected item in sizer” .

**What to do:**

1. **Initialize XRC handlers before any XRC load**

   * equivalent of `InitAllHandlers()`
2. **If spacers still fail, explicitly register sizer + spacer handlers**
3. **Add an XRC smoke test** to prevent regression

**Fallback option (if you need a quick unblock):**

* Replace spacers in XRC with a `wxPanel` (or equivalent) with fixed min size.
* This is uglier long-term, but will remove handler dependence.

---

### 4.2 Gdk cursor theme warnings (`sb_h_double_arrow`, `sb_v_double_arrow`)

You’re seeing:

* `Unable to load sb_h_double_arrow from the cursor theme`
* `Unable to load sb_v_double_arrow from the cursor theme` 

**What this usually means:** the *system* cursor theme is incomplete (common in minimal Linux environments / WSL GUI stacks). It’s not typically an application bug.

**Your options, from “most correct” to “most pragmatic”:**

1. **Document + fix dev environment**

   * Ensure a cursor theme that includes those cursors is installed.
   * Set `XCURSOR_THEME` to something known-good (e.g., Adwaita) in your dev run script.

2. **App-level mitigation (only if you really want silent logs)**

   * On Linux, set environment variables early in `main()` (before GTK initializes), for example:

     * `XCURSOR_THEME`
     * `XCURSOR_SIZE`
   * Or install/ship guidance for required system packages.

3. **Ignore**

   * If functionality is fine, treat it as a harmless warning.
   * But if you’re aiming for “maximally excellent,” I’d at least do (1) and ensure your CI/dev docs don’t spam warnings.

---

## 5) Bonus: you likely also want to address the GTK scrollbar criticals

Your logs show repeated GTK criticals about `gtk_box_gadget_distribute: assertion 'size >= 0' failed in GtkScrollbar` .

These are often triggered by layout calculations producing negative allocations (usually a sizer/min-size issue or a GTK quirk when a widget collapses too small).

**Practical mitigation checklist:**

* Ensure every panel created from XRC has:

  * a sizer attached
  * sane min sizes (never negative)
* After building the frame:

  * call `Layout()` and send an initial size event (the wxWidgets equivalent) so controls get a real allocation early
* Avoid “fixed pixel sizes” in XRC unless necessary; prefer proportions + expand flags.
* If it’s a GTK overlay scrollbar quirk in your environment, disabling overlay scrollbars can reduce noise (environment-level workaround).

Even if you don’t solve the root cause immediately, reducing these makes the desktop app feel more stable and makes real issues easier to spot.

---

## 6) A concrete “first 10 tasks” checklist (ordered for fastest payoff)

1. **Add XRC handler initialization** before loading XRC (fix spacer errors).
2. **Add XRC smoke test** that fails CI on any XRC parse/handler error.
3. **Fail fast on XRC load errors** (dialog + exit) instead of partial UI.
4. **Fix dev cursor theme warnings** via documented package/env setup (or app env set).
5. **Introduce engine command/event interface** (even if desktop calls directly).
6. **Refactor desktop UI to call engine only** (no direct domain logic).
7. **Implement menus + shortcuts + status bar** (native feel immediately).
8. **Build the main layout** (splitter + results table + details).
9. **Virtualize the diff list** (so it stays fast as data grows).
10. **Implement search/filter as engine-owned specs** (guarantees parity with web).

---

If you tell me what toolkit you’re using for the desktop UI (it looks like wxDragon/wxWidgets + XRC from your logs ) and whether your XRC is generated by wxFormBuilder/wxGlade, I can give you an exact “here’s the handler init function to call in wxDragon” snippet and an example XRC smoke test structure.
