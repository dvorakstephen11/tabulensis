Your current “desktop” layer is essentially a thin RPC wrapper around the engine (Tauri commands + progress events + a SQLite store), and the real work is already in reusable Rust crates.  

## The UI system I would switch to: wxWidgets via `wxDragon` (Rust)

If the goal is a desktop app that uses the OS’s own controls and conventions on Windows/macOS/Linux, the closest match in practice is **wxWidgets**:

* wxWidgets explicitly aims for a “native” experience by calling the platform GUI APIs instead of drawing its own faux controls. ([wxwidgets.org][1]) ([wxwidgets.org][2])
* Licensing is typically easier to live with early on: wxWidgets’ license is LGPL-like with an explicit exception that allows distributing derived binaries under your own terms. ([wxwidgets.org][3])
* In Rust, **wxDragon** is currently the most viable path to wxWidgets without writing your own C++ binding layer. It’s actively releasing (tags show v0.9.7 dated **Jan 5, 2026**), which is a very strong signal compared to many Rust GUI efforts that stall out. ([GitHub][4])
* wxDragon also supports scaling up to complex UIs (dockable “pro” layouts via AUI, drag-and-drop, XRC for UI definition, etc.), so you’re not boxed in once the MVP ships. ([Reddit][5])

### Why not the usual Rust GUI crates?

Most Rust-first GUI frameworks (egui/iced/slint/floem/druid-style) are great for productivity but generally **render their own widgets** (Skia/wgpu/etc.). That means they can look consistent across OSes, but they are not actually using the OS widget set, so they will never quite match platform behavior.

### Why not Qt?

Qt is excellent for “serious desktop apps”, but:

* It’s not truly OS-native controls; it’s a cross-platform widget set themed to resemble the OS (good, but different). ([Stack Overflow][6])
* Licensing becomes a real decision sooner (LGPL obligations vs commercial), and you said licensing constraints aren’t decided yet. ([Qt][7])

Given your stated priority is “OS widgets + OS theming + OS behavior” across all three platforms, **wxWidgets is the cleanest fit**, and **wxDragon** is the Rust entry point.

---

## What you’re migrating from (your current desktop shape)

Your workspace today is already split in a way that makes this migration straightforward: `core` (engine), `ui_payload` (UI-friendly data), `cli`, `wasm`, plus `desktop/src-tauri` (desktop shell). 

The Tauri shell currently does three things that matter:

1. **Runs diffs and stores results** (with caching and large-mode streaming) via `DiffRunner`.  
2. **Emits progress** to the UI by broadcasting a `"diff-progress"` event carrying `{ run_id, stage, detail }`. 
3. **Provides “commands”** like `diff_paths_with_sheets`, `cancel_diff`, `load_diff_summary`, `load_sheet_payload`, exports, batch, search.  

This is great news: you can preserve (1) almost entirely, replace (2) with a GUI-agnostic event sink, and replace (3) with direct Rust calls from the new GUI.

---

## Target architecture after the change

### Keep (unchanged)

* `core` crate: diff engine, parsing, streaming modes.
* `ui_payload`: your “view model” / report shaping.
* `cli` + `wasm`: still useful; desktop UI change shouldn’t disturb them.

### Replace

* `desktop/src-tauri` UI shell (Tauri + web assets).

### Add

1. **`desktop_backend` (new Rust library crate)**
   A GUI-independent backend that contains what’s currently embedded in Tauri:

   * `diff_runner` (engine thread, caching, large-mode handling)
   * `store` (SQLite OpStore, summaries, streaming ops)
   * `export` (audit xlsx)
   * `batch`
   * `search`
   * recents + app-data paths

2. **`desktop_gui_wx` (new Rust binary crate)**
   A wxDragon GUI that calls `desktop_backend` directly.

This split is the main “make it maintainable” move: your UI becomes just a consumer of the backend API rather than a web front-end that has to “RPC” into Rust.

---

## The core refactor: remove Tauri from your backend surface area

### 1) Replace Tauri’s event emitter with a backend “progress sink”

Right now progress is hard-wired to Tauri:

* `EngineProgress` implements `ProgressCallback` and emits `"diff-progress"` through `AppHandle`. 
* `emit_progress()` is Tauri-specific. 

**Goal:** make `diff_runner` emit progress without knowing anything about a GUI framework.

Concrete design:

* Define a lightweight backend event type (essentially your existing `ProgressEvent`):

  * `run_id: u64`
  * `stage: String`
  * `detail: String`

* Replace `AppHandle` in `DiffRequest` / `SheetPayloadRequest` with something like:

  * `progress: Arc<dyn ProgressSink>`
    or
  * `progress_tx: crossbeam_channel::Sender<ProgressEvent>`

Then:

* The wxDragon UI owns the receiver and updates widgets on the UI thread.
* The CLI can plug in a sink that prints, similar to what you already do for terminal progress. 

This is the biggest unlock: it makes your backend UI-framework-neutral.

### 2) Move “app data dir” logic out of Tauri

Today `recents_path()` and `store_path()` are resolved via `app.path().app_local_data_dir()` (Tauri API). 

For a pure native GUI you should switch to a crate like `directories`/`directories-next` and compute:

* data dir (platform standard)
* `recents.json`
* `diff_store.sqlite`

The backend should expose `AppPaths` so both GUI and CLI can use consistent locations.

### 3) Move file dialogs out of backend

Right now your Tauri command `export_audit_xlsx` uses `rfd::FileDialog` inside the command handler. 

In the wxDragon world:

* GUI shows file/open/save dialogs (wxWidgets native dialogs)
* backend takes plain paths and does work

This keeps backend testable and prevents GUI dependencies creeping into your core.

### 4) Keep your “large mode” design intact

Your `DiffRunner` already chooses between payload vs large-mode and stores results.  

That’s exactly what you want for a desktop GUI:

* if payload is small: render immediately
* if large: render summary and allow “drill down” (you already have `load_sheet_payload` for this)  

So the migration should avoid “reimagining” this logic—just re-home it.

---

## Migration plan in phases (MVP-first, without digging a hole)

### Phase 0: Freeze the behavior contract (1–2 sessions of work)

Make a checklist of what the current desktop UI can do via Tauri commands, because this becomes your functional acceptance suite:

* diff run + cancel 
* progress events (`"diff-progress"`) 
* load summary from SQLite 
* load single-sheet payload (for drill-down)  
* audit export  
* batch + search/index 

You’re not keeping the web UI, but you *are* keeping these user-visible capabilities.

### Phase 1: Create `desktop_backend` by “extract then adapt”

Mechanically move code first; change behavior second.

1. Add a new crate to the workspace:

   * `desktop_backend` (lib)

2. Move these modules from `desktop/src-tauri/src/` into `desktop_backend/src/`:

   * `diff_runner.rs`
   * `store.rs`
   * `export/`
   * `batch.rs`
   * `search.rs`

3. Remove Tauri-only concepts:

   * `tauri::AppHandle` from requests 
   * `tauri::Emitter` usage in progress 
   * Tauri command macros and state types

4. Replace with backend traits/channels:

   * progress sink abstraction
   * “cancel flag” stays as `Arc<AtomicBool>` (already good) 

5. Keep your SQLite schema and OpStore exactly as-is

   * this keeps historical diffs compatible across app upgrades.

Deliverable of Phase 1:

* CLI still builds
* backend unit tests still run
* (optionally) Tauri shell still builds by using a thin adapter layer (you can keep it temporarily while bringing up wxDragon)

### Phase 2: Stand up the wxDragon GUI shell (thin, but real)

Create `desktop_gui_wx` with wxDragon.

MVP window layout (keep it simple):

* Top: Old file picker, New file picker, “Compare” button, “Cancel” button
* Left: recent comparisons list
* Main: summary view
* Bottom: status/progress line

How it connects:

* Compare button -> spawn background thread -> call `desktop_backend::DiffRunner::diff()`
* Progress events -> channel -> handled on UI thread (wx idle handler) -> update status bar
* Cancel -> set the atomic flag (the engine already panics out when cancel flips; you map that to a friendly canceled error today) 

Deliverable of Phase 2:

* A native executable that compares two files and shows “identical vs differences count” + warnings.
* No embedded web UI.

### Phase 3: Restore “product value” screens (results navigation)

You can ship a usable MVP without recreating every pixel of the web UI, but you need basic navigation:

1. **Summary cards**

   * Show counts by category from `DiffRunSummary` and/or payload summary.

2. **Sheet list**

   * In payload mode: list sheets from payload
   * In large mode: list sheets from `summary.sheet_stats` (you already store resolved stats) 

3. **Drill-down**

   * When user clicks a sheet in large mode -> call backend `load_sheet_payload(diff_id, sheet_name)` in background and show a sheet-level view.  

4. **Ops list**

   * Use a virtualized list control (wxWidgets DataView in virtual mode) for performance.

Deliverable of Phase 3:

* Users can navigate from “workbook summary” -> “sheet details” -> “list of changes”.

### Phase 4: Exports + batch + search (the “workflow” features)

Re-introduce the features your Tauri command layer already exposes:

* Audit export: keep backend export logic; GUI asks for a save path and calls `export_audit_xlsx_from_store`. 
* Batch compare: call backend batch runner; show progress and a results table. 
* Search/index: call backend search/index functions; show results list with jump-to-sheet behavior. 

Deliverable of Phase 4:

* Feature parity with the current desktop feature set, without a webview.

### Phase 5: “Feels like a real desktop app”

This is where wxWidgets pays off:

* Menus that match platform conventions (File/Edit/View/Help)
* Keyboard shortcuts (Cmd on mac, Ctrl on Windows/Linux)
* Drag and drop onto the window
* Proper about dialog + version reporting
* High DPI behavior

wxDragon/wxWidgets have support for these classes of integration. ([Reddit][5])

---

## Practical notes for making this migration smooth

### Treat UI as a projection of a single “app state”

A native event-driven GUI gets messy fast if logic leaks into callbacks. Keep a single state struct in the GUI layer:

* selected paths
* current run id + cancel flag
* current diff outcome (payload vs summary)
* current selection: sheet, op, search query, etc.
* status/progress text

Then callbacks just emit messages (“CompareClicked”, “SheetSelected”, “CancelClicked”) and one update function mutates state and refreshes widgets.

This mirrors what your web UI already does conceptually with view-model building, but in Rust.

### Keep the “backend contract” stable

You already have a clean contract in the form of:

* `DiffOutcome` (payload vs summary) 
* `DiffRunSummary` stored in SQLite
* `load_sheet_payload` drill-down 

Keep these types and semantics. It prevents a rewrite cascade.

### Concurrency rule

Do all diff work on background threads, and do **all widget updates on the UI thread**. Your current Tauri implementation already runs diff via `spawn_blocking`; replicate that pattern directly. 

---

## Packaging and distribution (high level)

You’ll no longer use Tauri’s bundler. For wxWidgets/wxDragon you’ll want:

* Windows: build an `.exe` (often feasible with static linking depending on how wxDragon ships libs)
* macOS: `.app` bundle + signing/notarization later
* Linux: AppImage or distro packages (and be realistic about GTK dependencies)

wxDragon is explicitly targeting cross-platform builds, and its recent releases indicate active maintenance. ([GitHub][4])

---

## Summary recommendation

**Switch the desktop UI to wxWidgets via wxDragon.** It’s the best match for “OS-native controls and behavior on all three platforms” while letting you keep a single Rust codebase, and it avoids forcing a licensing decision as early as Qt typically does. ([wxwidgets.org][1])

---

## Concrete work order checklist for the native desktop UI migration

This section turns the earlier architecture decision into an agent-executable plan: (1) exactly what to move into `desktop_backend`, (2) the public API contract that `desktop_wx` will call, (3) the minimal wxDragon window tree + event wiring for the MVP, and (4) an implementation order that keeps the app runnable at every step.

This plan assumes the current “desktop” logic lives under `desktop/src-tauri/src` and is exposed via Tauri commands like `diff_paths_with_sheets`, `load_diff_summary`, `load_sheet_payload`, `export_audit_xlsx`, `run_batch_compare`, and search/index endpoints.  

---

### 1) Create `desktop_backend` by relocating the non-UI logic

#### 1.1 New crate layout

Create a new workspace member:

* `desktop/backend/` (crate name: `desktop_backend`)

Target: **no `tauri` dependency**, no webview/web server, no GUI dependencies. It should only depend on your existing engine (`excel_diff`), payload builder (`ui_payload`), and persistence/export/search utilities that already exist in the desktop shell. 

#### 1.2 Module move list (exact mapping)

Move these modules (or their contents) from `desktop/src-tauri/src/*` into `desktop/backend/src/*`:

1. **Diff execution + caching + progress emission**

* From: `desktop/src-tauri/src/diff_runner.rs`
* To: `desktop/backend/src/diff_runner.rs` (or rename to `engine.rs`, but keep file `diff_runner.rs` initially to reduce churn)

This file currently holds:

* `DiffRunner`, `DiffRequest`, `SheetPayloadRequest`
* `DiffOutcome` and `DiffErrorPayload`
* workbook/PBIX caches, large-mode decision, store writing, and progress emission via `"diff-progress"` events  

2. **SQLite store**

* From: `desktop/src-tauri/src/store/*`
* To: `desktop/backend/src/store/*`

Includes `OpStore`, `OpStoreSink`, `DiffRunSummary`, `DiffMode`, `RunStatus`, etc.  

3. **Export**

* From: `desktop/src-tauri/src/export/*`
* To: `desktop/backend/src/export/*`

Especially `export_audit_xlsx_from_store` (pure backend logic). 

4. **Batch compare**

* From: `desktop/src-tauri/src/batch.rs`
* To: `desktop/backend/src/batch.rs`

This currently depends on `tauri::AppHandle` only to pass it into `DiffRequest` for progress; that dependency will be removed when progress is decoupled. 

5. **Search + workbook indexing**

* From: `desktop/src-tauri/src/search.rs`
* To: `desktop/backend/src/search.rs`

`build_search_index` currently takes an `_app: AppHandle` but doesn’t use it; drop it. 

6. **Recents + “app data directory” pathing**
   These are currently embedded in `desktop/src-tauri/src/main.rs`:

* `RecentComparison`
* `recents_path(app: &AppHandle) -> PathBuf`
* `store_path(app: &AppHandle) -> PathBuf` (in the same area as `recents_path`)
  Move them into backend as:
* `desktop/backend/src/recents.rs`
* `desktop/backend/src/paths.rs`

The new implementations must not use Tauri’s `app.path().app_local_data_dir()`; they should use a cross-platform “project dirs” resolver (e.g., `directories` crate) to obtain the same class of per-user writable app data directory. 

#### 1.3 What stays in `desktop_wx` (UI-only)

Do **not** move these to backend:

* File/folder pickers, save dialogs (currently `rfd::FileDialog` in `export_audit_xlsx`) 
* Window state, widget state, view models
* Thread orchestration specific to GUI (polling timers, posting UI updates)

The backend should accept **paths** and return **results**; the UI owns dialogs.

---

### 2) Replace Tauri event emission with a backend-native progress channel

Your current progress plumbing is:

* diff engine calls `ProgressCallback::report(phase, current, total, message)`
* desktop wraps this in `EngineProgress` and emits a `"diff-progress"` event through Tauri’s `Emitter`, with `{ runId, stage, detail }` 

In `desktop_backend`, replace that with a channel-based progress bus.

#### 2.1 Progress types (backend-owned)

Keep the semantics identical so the UI can behave the same as the web UI bridge currently does (it filters by run id and updates a status line). 

* `ProgressEvent { run_id: u64, stage: String, detail: String }`

  * `stage` should continue to use the values you already emit (`"diff"`, `"snapshot"`, etc.).  

* `type ProgressTx = crossbeam_channel::Sender<ProgressEvent>`

* `type ProgressRx = crossbeam_channel::Receiver<ProgressEvent>`

#### 2.2 EngineProgress changes (precise edits)

In `diff_runner.rs`:

* Remove `use tauri::{AppHandle, Emitter};` and all `app.emit(...)` code. 
* Replace `DiffRequest.app: AppHandle` with `DiffRequest.progress: ProgressTx`
* Replace `SheetPayloadRequest.app: AppHandle` with `SheetPayloadRequest.progress: ProgressTx`
* Convert `emit_progress(&request.app, ...)` into `emit_progress(&request.progress, ...)`

Keep cancellation exactly as-is: `cancel: Arc<AtomicBool>` and the “panic on cancel” behavior in the progress callback (because the diff engine already expects that pattern). 

This change is what makes `desktop_backend` UI-agnostic while preserving progress behavior.

---

### 3) `desktop_backend` public API contract (functions + types)

This API is intentionally shaped to cover the existing desktop command surface one-for-one (so your UI migration is mostly a call-site swap). 

#### 3.1 Types re-exported from backend modules

Backend should publicly re-export (directly or via a `prelude` module):

* From `store`:

  * `OpStore`
  * `DiffRunSummary`
  * `DiffMode`
  * `RunStatus`
  * `StoreError`
* From `diff_runner`:

  * `DiffRunner`
  * `DiffRequest`
  * `SheetPayloadRequest`
  * `DiffOutcome`
  * `DiffErrorPayload`
* From `batch`:

  * `BatchRequest`
  * `BatchOutcome`
* From `search`:

  * `SearchResult`
  * `SearchIndexSummary`
  * `SearchIndexResult`
* From `recents`:

  * `RecentComparison`
* From `events` (new):

  * `ProgressEvent`
  * `ProgressTx`
  * `ProgressRx`

These names already exist (except events), and most are already `pub` in the current desktop code.   

#### 3.2 Backend facade (recommended)

Add a single “front door” struct to make `desktop_wx` dead simple:

**API contract (signatures):**

```rust
pub struct BackendPaths {
    pub app_data_dir: std::path::PathBuf,
    pub store_db_path: std::path::PathBuf,
    pub recents_json_path: std::path::PathBuf,
}

pub struct BackendConfig {
    pub app_name: String,
    pub app_version: String,
    pub engine_version: String,
}

pub struct DesktopBackend {
    pub paths: BackendPaths,
    pub runner: crate::DiffRunner,
}

impl DesktopBackend {
    pub fn init(cfg: BackendConfig) -> Result<Self, crate::DiffErrorPayload>;

    pub fn new_progress_channel() -> (crate::ProgressTx, crate::ProgressRx);

    pub fn load_recents(&self) -> Result<Vec<crate::RecentComparison>, crate::DiffErrorPayload>;
    pub fn save_recent(&self, entry: crate::RecentComparison) -> Result<Vec<crate::RecentComparison>, crate::DiffErrorPayload>;

    pub fn load_diff_summary(&self, diff_id: &str) -> Result<crate::DiffRunSummary, crate::DiffErrorPayload>;
    pub fn export_audit_xlsx_to_path(&self, diff_id: &str, path: &std::path::Path) -> Result<(), crate::DiffErrorPayload>;
    pub fn default_export_name(summary: &crate::DiffRunSummary, prefix: &str, ext: &str) -> String;

    pub fn search_diff_ops(&self, diff_id: &str, query: &str, limit: usize) -> Result<Vec<crate::SearchResult>, crate::DiffErrorPayload>;

    pub fn build_search_index(&self, path: &std::path::Path, side: &str) -> Result<crate::SearchIndexSummary, crate::DiffErrorPayload>;
    pub fn search_workbook_index(&self, index_id: &str, query: &str, limit: usize) -> Result<Vec<crate::SearchIndexResult>, crate::DiffErrorPayload>;

    pub fn run_batch_compare(&self, request: crate::BatchRequest, progress: crate::ProgressTx) -> Result<crate::BatchOutcome, crate::DiffErrorPayload>;
    pub fn load_batch_summary(&self, batch_id: &str) -> Result<crate::BatchOutcome, crate::DiffErrorPayload>;
}
```

Notes on intent:

* `DesktopBackend::init` owns “where do I put SQLite and recents.json” and constructs `DiffRunner` with the resolved store path (mirroring current Tauri setup).  
* `new_progress_channel()` is a convenience so the UI can create a tx/rx pair per long-running task and pass the tx into `DiffRequest` / `SheetPayloadRequest`.
* `export_audit_xlsx_to_path` is the backend portion of what is currently done in the Tauri command (which opens a save dialog, then writes). 
* `run_batch_compare` includes an explicit `progress` parameter because batch is naturally multi-diff and you’ll want to show status.

If you prefer not to have a facade, you can expose only the lower-level functions, but the facade is what makes the wx UI migration go fast.

---

### 4) Minimal wxDragon window tree for the MVP

wxDragon supports both:

* programmatic widget construction (quick start), and
* XRC (XML UI) with typed widget bindings via `include_xrc!`, which scales better as screens grow. ([Docs.rs][1])

For an MVP that can grow without becoming a tangle, use **XRC + `include_xrc!`**.

#### 4.1 MVP window tree (named widgets)

**Root: `MainFrame`**

* `wxFrame` name: `main_frame`

  * `wxMenuBar` name: `menu_bar`

    * `File` menu:

      * `Open Old...`
      * `Open New...`
      * `Exit`
    * `Run` menu:

      * `Compare`
      * `Cancel`
    * `Export` menu:

      * `Export Audit XLSX...`
    * `Help` menu:

      * `About`
  * `wxStatusBar` name: `status_bar`

**Body: `wxNotebook` as top-level navigation**

* `wxNotebook` name: `root_tabs`

  1. Tab: `Compare`

     * `wxPanel` name: `compare_panel`

       * File selection row:

         * `wxFilePickerCtrl` name: `old_picker`
         * `wxFilePickerCtrl` name: `new_picker`
         * `wxButton` name: `compare_btn`
         * `wxButton` name: `cancel_btn`
       * Options row (optional for MVP; can hide behind an “Advanced” collapsible):

         * `wxChoice` name: `preset_choice`
         * `wxCheckBox` name: `trusted_checkbox`
       * Progress row:

         * `wxGauge` name: `progress_gauge` (indeterminate mode is fine initially)
         * `wxStaticText` name: `progress_text`
       * Results area:

         * `wxSplitterWindow` name: `compare_split`

           * Left pane: `wxDataViewListCtrl` name: `sheets_list`

             * Columns: `Sheet`, `Ops`, `Added`, `Removed`, `Modified`, `Moved`
           * Right pane: `wxNotebook` name: `result_tabs`

             * Tab “Summary”: `wxTextCtrl` (multiline, read-only) name: `summary_text`
             * Tab “Details”: `wxTextCtrl` (multiline, read-only) name: `detail_text`

  2. Tab: `Recents`

     * `wxPanel` name: `recents_panel`

       * `wxDataViewListCtrl` name: `recents_list`
       * `wxButton` name: `open_recent_btn`

  3. Tab: `Batch`

     * `wxPanel` name: `batch_panel`

       * `wxDirPickerCtrl` name: `batch_old_dir`
       * `wxDirPickerCtrl` name: `batch_new_dir`
       * `wxTextCtrl` name: `include_glob_text`
       * `wxTextCtrl` name: `exclude_glob_text`
       * `wxButton` name: `run_batch_btn`
       * `wxDataViewListCtrl` name: `batch_results_list`

  4. Tab: `Search`

     * `wxPanel` name: `search_panel`

       * `wxSearchCtrl` name: `search_ctrl`
       * `wxChoice` name: `search_scope_choice` (Changes / Old workbook / New workbook)
       * `wxButton` name: `search_btn`
       * `wxButton` name: `build_old_index_btn`
       * `wxButton` name: `build_new_index_btn`
       * `wxDataViewListCtrl` name: `search_results_list`

This widget set is entirely within wxDragon’s supported mapping for XRC widgets (file/dir pickers, notebook, splitter, dataview, search control, etc.). ([Docs.rs][2])

#### 4.2 Event wiring (exact behavior)

**Core pattern:**

* UI thread owns widgets.
* Long-running backend calls run in a worker thread.
* Worker thread sends progress to `ProgressTx`.
* UI polls `ProgressRx` on a timer and updates `progress_text` + `status_bar`.

Event wiring map:

1. `compare_btn.on_click`

* Validate both paths exist.
* Create:

  * `cancel_flag = Arc<AtomicBool>::new(false)`
  * `(progress_tx, progress_rx) = DesktopBackend::new_progress_channel()`
  * `run_id = next_counter()`
* Spawn thread:

  * Call `backend.runner.diff(DiffRequest { old_path, new_path, run_id, options, cancel: cancel_flag.clone(), progress: progress_tx })`
* UI state:

  * store `active = { run_id, cancel_flag, progress_rx }`
  * disable compare button, enable cancel button, show indeterminate gauge

2. `cancel_btn.on_click`

* If there is an active run, set `cancel_flag.store(true, Ordering::Relaxed)`
* UI: set status text “Canceling…”

3. UI timer (e.g., 30–60ms)

* Drain `progress_rx.try_iter()` and apply the latest:

  * `progress_text.set_label(event.detail)`
  * `status_bar.set_status_text(event.detail, 0)`
* If you want “percentage”, you can extend `ProgressEvent` later; for MVP the detail text is already being produced. 

4. Diff completion (worker thread -> UI)

* When thread finishes, it should post a UI update:

  * if `DiffOutcome.mode == Payload`: show summary + populate sheet list immediately
  * if `DiffOutcome.mode == Large`: show summary (from `DiffOutcome.summary`) and populate sheet list; selecting a sheet can later call `load_sheet_payload` 
* Also call `save_recent(...)` with `{ diff_id, mode }` filled.

5. `Export Audit XLSX...` menu item / button

* Requires an existing `diff_id`
* UI opens “Save file” dialog (wx file dialog or a FilePicker-like control flow)
* Calls `backend.export_audit_xlsx_to_path(diff_id, chosen_path)`  

6. `sheets_list` selection changed

* If current diff is `Payload`, you can display per-sheet details from payload immediately.
* If `Large`, spawn a thread:

  * `backend.runner.load_sheet_payload(SheetPayloadRequest { diff_id, sheet_name, cancel: Arc<AtomicBool>, progress: progress_tx })`
  * update right-side “Details” tab on completion
  * emit progress stage `"snapshot"` (mirrors current behavior). 

7. `run_batch_btn.on_click`

* Spawn thread: `backend.run_batch_compare(request, progress_tx)`
* Populate `batch_results_list` when done. 

8. Search tab:

* “Changes” scope: `backend.search_diff_ops(diff_id, query, limit)` 
* “Old/New workbook” scope:

  * Ensure an index exists (build if needed): `build_search_index(path, side)` 
  * Then search: `search_workbook_index(index_id, query, limit)` 

---

### 5) Implementation order that keeps the app runnable

This is the “never break the build” sequence. At the end of each step, **you can run something**.

#### Step 0: Keep the existing desktop runnable while you build the new one

* Do not delete `desktop/src-tauri` yet.
* Add new crates in parallel and keep workspace building.

Definition of done:

* `cargo build` succeeds for the workspace.
* Existing CLI/web continue working.

#### Step 1: Add `desktop_backend` crate (compile-only)

* Create crate + modules.
* Copy code into new paths with minimal edits.
* Update imports inside moved files to match new module structure.
* Don’t touch logic yet.

Definition of done:

* `cargo test -p desktop_backend` (or `cargo build -p desktop_backend`) works.

#### Step 2: Remove Tauri types from backend (progress channel refactor)

* Replace `AppHandle`/`Emitter` with `ProgressTx` as described.
* Drop any `tauri` dependency from backend Cargo.toml.

Definition of done:

* `desktop_backend` compiles with **no tauri** dependency.
* A trivial “smoke binary” (optional) can call `DiffRunner` and receive `ProgressEvent`s on a channel.

#### Step 3: Create `desktop_wx` crate that shows an empty window

* Add new crate (e.g., `desktop/wx/`) depending on `wxdragon` and `desktop_backend`.
* Use wxDragon quick start to create `Frame` and show it. ([Docs.rs][1])

Definition of done:

* `cargo run -p desktop_wx` launches a native window on your dev OS.

#### Step 4: Add the MVP XRC UI skeleton + tab navigation (still no backend calls)

* Create `ui/main.xrc`
* Add `include_xrc!("ui/main.xrc", MainUi);` and instantiate it. ([Docs.rs][1])

Definition of done:

* App launches and shows tabs + controls.

#### Step 5: Implement Compare tab end-to-end with “summary as text”

* Wire `compare_btn` to run a diff in a worker thread.
* Show the returned `DiffRunSummary` (or payload summary) serialized into `summary_text` as JSON for now.
* Implement cancel.
* Implement timer polling of `ProgressRx` and update `progress_text`.

Definition of done:

* You can select two files, click Compare, see progress messages, and see a finished summary.

#### Step 6: Populate `sheets_list` from `DiffRunSummary.sheets`

* Use `DiffRunSummary.sheets` to fill the left list with per-sheet counts. 
* When a sheet is selected:

  * In `Payload` mode: show any available details you can immediately (even if just “sheet selected” + counts)
  * In `Large` mode: wire `load_sheet_payload` and display resulting payload JSON in `detail_text` (still MVP-simple)

Definition of done:

* Large-mode diffs are navigable: sheet selection triggers background “load sheet payload” and updates UI.

#### Step 7: Export audit XLSX

* Add save dialog in UI.
* Call backend export to the chosen path.

Definition of done:

* “Export Audit XLSX” produces a file on disk. 

#### Step 8: Recents tab

* On app start, call `backend.load_recents()` and populate list.
* After each diff, call `backend.save_recent(...)`.

Definition of done:

* Recents persists across app restarts. 

#### Step 9: Batch tab

* Wire run.
* Show results list; double-click a row switches to Compare tab and loads the diff summary for that diff id.

Definition of done:

* Batch compare works without freezing the UI. 

#### Step 10: Search tab

* Implement “changes search” via `search_diff_ops`
* Implement workbook index build + search

Definition of done:

* You can search within a diff and within indexed workbooks. 

#### Step 11: Replace JSON text dumps with real viewers (post-MVP polish)

* Use `DataView` with custom renderers for richer diff displays if desired. ([Docs.rs][3])
* Add filtering, grouping, and better sheet-level details.

---

### 6) Notes that affect the work order (why this sequencing matters)

* Cross-platform distribution and packaging is consistently one of the hardest parts of “desktop MVP” work; plan to treat it as its own milestone once the wx app is functionally complete. 
* A native-feeling Mac app experience is explicitly one of the big differentiation opportunities; wxWidgets-style native controls and platform conventions support that direction. 
* wxDragon has concrete platform build prerequisites (CMake + a C++ toolchain; on Linux you need GTK dev packages, etc.), so CI updates should be done after the app skeleton compiles locally. ([Docs.rs][1])

(Those last two points are about *scheduling risk* and *product leverage*, not code structure; they’re included here so the agent doesn’t accidentally burn days on packaging before the UI is stable.)

