Below is a concrete, end-to-end plan to fix the three issues you’re seeing:

1. the UI only uses a small fraction of the window
2. the window chrome on Linux is missing maximize (and likely minimize/resize affordances)
3. the GtkNotebook “size >= 0” warnings at startup

I’m going to anchor the plan in what your code is doing today (wxDragon + XRC) and what the screenshot shows.

---

## What’s actually going wrong

### A. The “unused real estate” problem is classic sizer configuration (or missing sizers)

In wxWidgets, **controls do not automatically grow** just because the window is bigger. A control grows only if:

* it’s inside a **sizer chain** (every ancestor panel has a sizer set), and
* the sizer item has **`proportion > 0`** (XRC: `<option>`) and usually **`wxEXPAND`** (XRC: `<flag>`) so it’s allowed to take extra space.

Your screenshot shows multiple widgets sitting at (roughly) their minimum/best sizes:

* the two file pickers are narrow
* the sheets list is extremely narrow (column header truncated)
* the Summary/Details notebook is stuck at a small fixed size, leaving a huge blank region below/right

That lines up perfectly with your code explicitly setting minimum sizes for the notebooks:

* `root_tabs.set_min_size(min_root_tabs_size())` (640×360) and
* `result_tabs.set_min_size(min_result_tabs_size())` (420×240)

**Key point:** `SetMinSize` prevents shrinking below a size; it does *not* make the control expand. If the sizer item `option` is 0 / not expanding, the notebook will sit near that min/best size forever and you’ll get blank space.

### B. Missing maximize button is almost certainly the frame style in XRC

On Linux/GTK, if the wxFrame does not include `wxMAXIMIZE_BOX` (and often `wxRESIZE_BORDER`), the window manager typically won’t show the maximize button. Your screenshot shows only a close button, which is a strong sign that the frame style is missing maximize/minimize.

You’re not setting frame style in Rust; you load the frame from XRC and then show it.
So the fix is almost certainly in `main.xrc`’s `<style>` for `main_frame`.

### C. The GtkNotebook warnings are very likely caused by the notebook being laid out at 0/negative size during startup

You get the warning twice:

```
GtkNotebook
gtk_box_gadget_distribute: assertion 'size >= 0' failed
```

Twice fits your UI: **two notebooks** (`root_tabs` and `result_tabs`).

This warning tends to happen when GTK is asked to allocate a notebook a size that becomes negative after subtracting borders/tab area/margins — which often occurs when:

* a notebook is briefly sized to (0,0) at realize time, or
* a sizer chain is incomplete and the notebook doesn’t get a sane size until later, or
* there’s a misconfigured sizer item/border combination.

You do call `frame.layout()` via `call_after` after showing the frame, 
but if the notebooks are realized before the layout chain is correct, you can still get the GTK criticals.

---

## The plan

### Phase 1: Fix the window chrome (maximize/minimize/resize) first

This is independent and fast, and it immediately improves usability.

1. **Edit `desktop/wx/ui/main.xrc` → `main_frame` style**

   * Ensure the frame includes at least:

     * `wxRESIZE_BORDER`
     * `wxMAXIMIZE_BOX`
     * `wxMINIMIZE_BOX`
     * `wxSYSTEM_MENU`
     * `wxCLOSE_BOX`
     * `wxCAPTION`
   * Easiest: set it to `wxDEFAULT_FRAME_STYLE` (and add any extras you need).

2. **Verify the window becomes resizable and shows maximize**

   * On GNOME/Ubuntu, this should cause the maximize button to appear (unless the user has globally hidden it in system settings, which is uncommon if other apps show it).

3. Optional: **start maximized by default**

   * If you want “always start maximized”, do it explicitly in Rust after showing:

     * `frame.show(true)` then `frame.maximize(true)`
   * Right now you set a default size (1280×900) and min size (960×640) only.
     That’s fine, but it won’t be maximized.

**Recommendation:** don’t hard-force maximize forever; do “restore last size, else maximize on first run”. That’s the most user-friendly long-term behavior.

---

### Phase 2: Make the layout actually expand (the real “blank space” fix)

This is the core change: ensure your XRC uses sizers correctly, and that every “main content” region is placed in a sizer with the right `option`/`flag`.

#### 2.1 Establish a strict “sizer chain” rule

For every window that should resize nicely:

* Frame client area has a sizer
* Its child panel (`main_panel`) has a sizer
* Each notebook page panel has a sizer
* Any nested panels (like `sheets_list`) have a sizer (you already do this for the DataView panels in code)

If **any** level in that chain is missing, children tend to freeze at their best/min sizes and you get blank areas.

You already set a frame sizer that adds `main_panel` with `Expand` and `proportion=1`: 
So the next most likely missing link is:

* `main_panel` does not have a sizer that expands `root_tabs`, and/or
* Compare page’s container (`compare_container`) does not give expanding proportion to the bottom region and file pickers.

#### 2.2 Fix `main_panel` → ensure `root_tabs` expands

In XRC (preferred):

* `main_panel` should have a top-level `wxBoxSizer` (vertical) containing `root_tabs`
* the sizer item for `root_tabs` must be:

  * `<option>1</option>`
  * `<flag>wxEXPAND</flag>` (often `wxEXPAND|wxALL`)
  * reasonable border (e.g., 8)

If you can’t / don’t want to do it in XRC, you can also enforce it in Rust by setting a sizer on `main_panel` and adding `root_tabs` with proportion 1 expand — but keep in mind you’ll be mixing declarative layout (XRC) with imperative layout (Rust), and you should do it consistently.

#### 2.3 Redesign the Compare page layout around “fixed top, expanding bottom”

You already require a widget named `compare_container` in XRC. 
Treat that as the **single top-level sizer** for the Compare page.

The Compare page should be a vertical layout like this:

1. **Row: file pickers + actions** (horizontal sizer)

   * `old_picker`: `option=1`, `wxEXPAND`
   * `new_picker`: `option=1`, `wxEXPAND`
   * `compare_btn`: `option=0`
   * `cancel_btn`: `option=0`

This immediately fixes your “path boxes are smushed” problem because each picker gets to stretch horizontally.

2. **Row: options** (horizontal sizer)

   * `preset_choice`: `option=0`
   * `trusted_checkbox`: `option=0`
   * add a stretch spacer after them so they stay left and the row can grow cleanly

3. **Row: progress** (horizontal sizer)

   * `progress_gauge`: `option=1`, `wxEXPAND`
   * `progress_text`: `option=0` (or put it above the gauge in a small vertical stack)

Right now in the screenshot the gauge looks like it isn’t stretching; that’s exactly what this fixes.

4. **Row: main content area** (this must be the “elastic” region)

   * This row should be `option=1`, `wxEXPAND` so it consumes all remaining space.

Inside that row, you have two good options:

**Option A (best UX): use a splitter**

* Put a `wxSplitterWindow` (or `wxSplitterWindow`-equivalent in XRC) here
* Left pane: `sheets_list` panel
* Right pane: `result_tabs` notebook
* Configure:

  * initial sash position around 280–360 px
  * minimum pane size ~200 px
  * sash gravity so resizing favors the right pane

This makes the narrow “Shee…” issue disappear and lets the user choose how wide the sheet list should be.

**Option B (simpler): horizontal box sizer**

* Add `sheets_list` with `option=0` but set a min width on it (e.g., 260–320)
* Add `result_tabs` with `option=1` and `wxEXPAND`

Splitter is noticeably better in practice, but either will fix the “tiny results area with huge blank space” issue.

#### 2.4 Ensure `result_tabs` pages expand their text controls

You already find `summary_text` and `detail_text` as controls (likely inside notebook pages). 
Make sure in XRC each notebook page panel has a vertical sizer and the text control is:

* `option=1`
* `wxEXPAND`
* ideally with a small border

If the text control isn’t in a sizer, it will stay small even if the notebook grows.

#### 2.5 Apply the same “top fixed / bottom expanding” rule to other tabs

Even though the screenshot is Compare, your other pages likely have the same under-expansion if their main list panels aren’t assigned `option=1` + `wxEXPAND`.

You have these panels that are intended to be big list areas:

* `recents_list`
* `batch_results_list`
* `search_results_list`

And you already set a sizer inside each of those panels in Rust when you create the DataViewCtrl.
So the remaining requirement is: **the notebook page’s layout must expand those panels**.

Concretely:

* Recents page: vertical sizer with `recents_list` as `option=1 expand`, and `open_recent_btn` as fixed
* Batch page: top controls fixed, results list `option=1 expand`
* Search page: top search bar fixed, results list `option=1 expand`

This will make every tab actually use the window size.

---

### Phase 3: Fix the GtkNotebook critical warnings (make startup sizing sane)

Do this after Phase 2, because in most cases the warnings disappear naturally once notebooks are sized via a correct sizer chain.

If warnings remain, do the following in order:

#### 3.1 Ensure notebooks are never created “floating”

The common trigger is: notebook exists, but isn’t in an expanding sizer yet at realize time.

Checklist:

* `root_tabs` must be inside a sizer-owned panel (`main_panel`) and that panel must have a sizer.
* `result_tabs` must be inside a sizer-owned container on the compare page and must have a sane size before show.

This is the main structural fix.

#### 3.2 Force an initial layout pass before GTK fully realizes everything

Right now you do:

* show frame
* centre frame
* call_after → layout 

I would change the sequence conceptually to:

1. after loading XRC and setting any sizers/min sizes: call `layout()` immediately
2. show
3. call_after:

   * `layout()` again
   * possibly `send_size_event()` (or its wxDragon equivalent), which forces a full re-layout on GTK once sizes are final

Why this helps: GTK is most likely emitting the assertion during initial size allocation; doing a layout before show reduces the chance that a notebook gets a temporary 0/negative allocation.

#### 3.3 Remove “min size” bandaids if they’re masking bad sizer config

You currently set:

* `root_tabs` min 640×360
* `result_tabs` min 420×240

These are fine as guardrails, but if the sizers are misconfigured, they can also create weird interactions where parents can’t satisfy min sizes cleanly during early layout.

After the sizers are fixed, consider:

* keeping min sizes, but possibly lowering them slightly, or
* removing `result_tabs` min size entirely (often unnecessary once it’s correctly expanding)

(Do this only after sizers are correct.)

---

### Phase 4: Add guardrails so this doesn’t regress

You already have an XRC structural validator (`validate_xrc`) with required widget names and notebook label checks.
That’s a great start. Extend it to catch layout regressions that would recreate your current problem.

#### 4.1 Add XRC validation for “expand-critical” widgets

Parse the XRC to ensure:

* `root_tabs` is inside a sizer item with `wxEXPAND` and `option >= 1`
* the Compare page’s main “content region” is in a sizer item with `option >= 1`
* `result_tabs` is in an expanding sizer item with `option >= 1`
* `old_picker` and `new_picker` have expanding flags (at least `wxEXPAND`) so they can grow horizontally

This can be done by enhancing your existing quick-xml parser:

* track when you’re inside a `sizeritem`
* record `object name`, `flag`, and `option`
* enforce rules for specific widget names

This will stop future UI edits from reintroducing “everything is proportion 0” by accident.

#### 4.2 Add a debug mode that prints sizes after show

Add an env var like `EXCEL_DIFF_DEBUG_LAYOUT=1` that logs:

* frame size
* root_tabs size
* compare page size
* result_tabs size
* sheets_list size

This makes it trivial to confirm that resizing behaves correctly without guessing.

---

## Practical “definition of done” checklist

When you’re finished, you should be able to verify all of this quickly:

* Launch app:

  * No Gtk-CRITICAL notebook warnings
  * Window shows maximize button (and likely minimize)
  * Window can be resized
* Compare tab:

  * File pickers expand horizontally with the window
  * Gauge expands horizontally
  * Sheets list is readable at a reasonable width (and ideally resizable via splitter)
  * Summary/Details area expands to use the full remaining space
* Recents/Batch/Search tabs:

  * Their main list areas expand to fill the window
* Resize the window smaller:

  * UI reflows reasonably
  * Controls don’t overlap
  * Scrollbars appear where appropriate, not giant blank space

---

## The shortest path to a clean fix

If you want the highest-leverage sequence:

1. Fix `main_frame` style in XRC (maximize/minimize/resize)
2. Ensure `main_panel` has a sizer that expands `root_tabs`
3. Fix Compare page sizers so:

   * file pickers expand horizontally
   * the bottom content row is `option=1` expand
   * sheets/results are in a splitter or at least a weighted horizontal sizer
4. Apply same pattern to other notebook pages
5. Only then adjust layout timing (layout before show + send size event) if GTK warnings remain

That sequence addresses the usability pain first and tends to eliminate the GTK warnings as a side effect of correct sizing.

---


Below is an explicit *target* widget + sizer tree for `desktop/wx/ui/main.xrc` that fixes the “everything is crammed into the top-left” problem by ensuring every container has a sizer and every “main content” child is added with `proportion=1` + `wxEXPAND`.

This tree is designed to preserve **all widget names your Rust code/XRC validator require** (e.g. `root_tabs`, `compare_container`, `sheets_list`, `result_tabs`, `summary_text`, etc.).

---

## Target layout tree

```text
wxFrame main_frame
  - wxMenuBar (File/Run/Export/Help)
    - wxMenuItem open_old_menu
    - wxMenuItem open_new_menu
    - wxMenuItem exit_menu
    - wxMenuItem compare_menu
    - wxMenuItem cancel_menu
    - wxMenuItem export_audit_menu
    - wxMenuItem about_menu

  - wxStatusBar status_bar

  - wxPanel main_panel
    - wxBoxSizer (VERTICAL)  [the critical missing piece is: main_panel MUST have a sizer]
      - wxNotebook root_tabs                     (proportion=1, flag=wxEXPAND|wxALL, border=6)

        - notebookpage "Compare"
          - wxPanel compare_container             [name must exist]                    :contentReference[oaicite:1]{index=1}
            - wxBoxSizer (VERTICAL)

              - wxBoxSizer (HORIZONTAL)  "compare_file_row"
                - wxFilePickerCtrl old_picker      (proportion=1, wxEXPAND|wxALL, border=4)
                - wxFilePickerCtrl new_picker      (proportion=1, wxEXPAND|wxALL, border=4)
                - wxButton compare_btn             (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)
                - wxButton cancel_btn              (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)

              - wxBoxSizer (HORIZONTAL)  "compare_options_row"
                - wxChoice preset_choice           (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)
                - wxCheckBox trusted_checkbox      (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)
                - (stretch spacer)                 (proportion=1)

              - wxBoxSizer (HORIZONTAL)  "compare_progress_row"
                - wxGauge progress_gauge           (proportion=1, wxEXPAND|wxALL, border=4)
                - wxStaticText progress_text       (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)

              - wxSplitterWindow compare_splitter  (proportion=1, wxEXPAND|wxALL, border=6)
                - wxPanel sheets_list              [EMPTY placeholder panel]          
                - wxPanel compare_right_panel
                  - wxBoxSizer (VERTICAL)
                    - wxNotebook result_tabs        (proportion=1, wxEXPAND)

                      - notebookpage "Summary"     [label must match exactly]         :contentReference[oaicite:3]{index=3}
                        - wxPanel summary_page_panel
                          - wxBoxSizer (VERTICAL)
                            - wxTextCtrl summary_text   (proportion=1, wxEXPAND|wxALL, border=4)

                      - notebookpage "Details"     [label must match exactly]         :contentReference[oaicite:4]{index=4}
                        - wxPanel detail_page_panel
                          - wxBoxSizer (VERTICAL)
                            - wxTextCtrl detail_text    (proportion=1, wxEXPAND|wxALL, border=4)

        - notebookpage "Recents"
          - wxPanel recents_page_panel
            - wxBoxSizer (VERTICAL)
              - wxBoxSizer (HORIZONTAL) "recents_top_row"
                - wxButton open_recent_btn         (proportion=0, wxALL, border=4)
                - (stretch spacer)                 (proportion=1)
              - wxPanel recents_list               [EMPTY placeholder panel]          
                (proportion=1, wxEXPAND|wxALL, border=6)

        - notebookpage "Batch"
          - wxPanel batch_page_panel
            - wxBoxSizer (VERTICAL)

              - wxFlexGridSizer (2 columns) "batch_form"
                row: (StaticText "Old folder") + wxDirPickerCtrl batch_old_dir        (col 1 growable)
                row: (StaticText "New folder") + wxDirPickerCtrl batch_new_dir
                row: (StaticText "Include globs") + wxTextCtrl include_glob_text
                row: (StaticText "Exclude globs") + wxTextCtrl exclude_glob_text
                row: (StaticText "") + wxButton run_batch_btn
                (batch_form itself: proportion=0, wxEXPAND|wxALL, border=6)

              - wxPanel batch_results_list         [EMPTY placeholder panel]
                (proportion=1, wxEXPAND|wxALL, border=6)                               

        - notebookpage "Search"
          - wxPanel search_page_panel
            - wxBoxSizer (VERTICAL)

              - wxBoxSizer (HORIZONTAL) "search_row"
                - wxSearchCtrl search_ctrl         (proportion=1, wxEXPAND|wxALL, border=4)
                - wxChoice search_scope_choice     (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)
                - wxButton search_btn              (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)
                - wxButton build_old_index_btn     (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)
                - wxButton build_new_index_btn     (proportion=0, wxALL|wxALIGN_CENTER_VERTICAL, border=4)

              - wxPanel search_results_list        [EMPTY placeholder panel]
                (proportion=1, wxEXPAND|wxALL, border=6)                               
```

---

## Notes that matter for this XRC in *your* codebase

### 1) The “placeholder list panels” must be empty

Your Rust creates `DataViewCtrl`s dynamically inside these panels and then assigns a new sizer to the panel (`parent.set_sizer(...)`). 
So in XRC, these should be plain `wxPanel` containers with **no children**:

* `sheets_list`
* `recents_list`
* `batch_results_list`
* `search_results_list`

If you put any XRC children in those panels, your code will effectively “take over” layout and you’ll get weird overlapping / missing content.

### 2) The two notebook label sets must stay exact

Your validator expects:

* `root_tabs` pages: `"Compare", "Recents", "Batch", "Search"`
* `result_tabs` pages: `"Summary", "Details"` 

Keep spelling and order identical.

### 3) The real fix for “unused real estate”

The screenshot symptom happens when one or more of these is true:

* `main_panel` has no sizer managing `root_tabs`
* `compare_container` has no sizer managing its rows
* the bottom area (splitter / main content) is added with `proportion=0` or without `wxEXPAND`

The tree above forces:

* `root_tabs` gets `proportion=1`
* the Compare “main area” gets `proportion=1`
* text boxes / notebooks get `proportion=1`

That’s the essential “fill the window” behavior you’re missing.
