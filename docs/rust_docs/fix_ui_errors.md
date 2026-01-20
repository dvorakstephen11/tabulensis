You’re not looking at a Rust panic or a wxDragon error. That line is GTK (wxGTK on Linux) complaining during *layout* of a native `GtkNotebook`.

### Root cause (what’s *actually* happening)

* `gtk_box_gadget_distribute: assertion 'size >= 0' failed in GtkNotebook` is emitted when GTK’s notebook tries to distribute space to its internal children and, at that instant, the available space is effectively “too small” (often during startup while widgets are being constructed/initialized and are temporarily allocated a tiny size like 1x1, or while the window isn’t visible yet). This frequently shows up as scary “Gtk-CRITICAL”, but the app continues normally.

* In your UI you have **two** notebooks:

  * `root_tabs` (the main app tabs)
  * `result_tabs` (the Summary/Details tabs)

  You explicitly fetch both and set minimum sizes on both during XRC load.
  The fact that the GTK message prints **twice** lines up perfectly with “two notebooks are being laid out”.

* This is a **known** wxGTK/GTK behavior: wxWidgets maintainers explicitly note that despite GTK shouting “CRITICAL”, it can often be safely ignored and is “unfortunately not simple to avoid in all situations”. ([Google Groups][1])
  wxPython users report the same pattern specifically for notebook-heavy UIs during startup while the window is not yet visible. ([Discuss wxPython][2])

So the root cause is:

> **GTK is doing a layout pass on your `wxNotebook`(s) while the frame/page allocation is temporarily tiny (or otherwise smaller than the notebook’s internal minimum needs), and GTK logs a spurious critical assertion.**
> Your current initialization sequence (XRC loads notebooks + you set notebook min sizes early) makes that more likely.

---

## Plan to address it (architecturally clean, not a band-aid)

### 1) Make the “first layout pass” happen with sane geometry

The most robust way to prevent GTK from ever seeing a “1x1-ish notebook” is to ensure the *frame* has a reasonable initial size **before** the notebook is forced through any meaningful layout.

There are two clean approaches:

#### Option A (best for XRC-driven UIs): put initial sizing in XRC

Set `<size>` and `<minsize>` on the `main_frame` object in `desktop/wx/ui/main.xrc`, so the frame is created with sane dimensions during `load_frame()`. This matters because the warning often happens during widget construction *before your Rust code gets a chance to fix sizing* (exactly the situation described in the wxPython thread). ([Discuss wxPython][2])

This is the most “declarative UI” approach and reduces code-driven layout side effects.

#### Option B (best if you want runtime sizing logic): size the frame immediately after load_frame

Move frame sizing earlier (right after `load_frame`) and avoid child min-size constraints that can force GTK into “not enough space” calculations.

### 2) Stop setting min sizes on the notebooks themselves (this is the big self-inflicted trigger)

You already set the **frame** min size and initial size later:

```rust
ctx.ui.frame.set_min_size(Size::new(960, 640));
ctx.ui.frame.set_size(Size::new(1280, 900));
```



That’s the right place to enforce minimum usable real estate. Setting additional min sizes on child notebooks during startup increases the chance GTK decides “I can’t fit my internals” during a transient size pass.

So the “sound” fix is:

* Remove `root_tabs.set_min_size(...)`
* Remove `result_tabs.set_min_size(...)`

If you later discover you *really* need a notebook min size for usability, set it **after** the frame is shown and has a stable allocation (e.g., in your existing `call_after` block) — but start by removing them.

### 3) If it still happens, make page construction lazy (the “excellent architecture” route)

If your diagnostic shows the GTK warning fires inside `load_frame()` (before your code touches anything), the only truly deterministic fixes are:

* **Lazy notebook page construction**: don’t build complex pages until after the frame is shown and laid out.

  * Keep the notebook and placeholder pages in XRC.
  * Load each page’s content from separate XRC fragments (or build programmatically) on first activation.
  * This improves startup time and avoids “build heavy pages while invisible” — exactly what wxPython folks suspect triggers the warnings. ([Discuss wxPython][2])

* **Switch to `wxAuiNotebook`** for the tab controls (AUI uses different internals than the native GTK notebook). wxDragon has AUI support behind the `aui` feature. ([Docs.rs][3])
  This is a larger UI decision (styling/behavior differences), but it can eliminate this entire GTK notebook warning class.

### 4) Last resort: filter only this known spurious GTK message in release builds

If after structural fixes GTK still emits it (which does happen in wxGTK land), you can keep your logs clean without hiding real problems by filtering only messages that match:

* domain: “Gtk”
* level: critical
* substring: `gtk_box_gadget_distribute` + `GtkNotebook`

Gate it behind an env var like `EXCEL_DIFF_SUPPRESS_GTK=1` (off by default in debug builds).

This is “product polish” only — not the primary fix — because it can mask genuine layout issues if overused.

---

## Concrete code changes you can make right now

### Change 1: Remove notebook min sizes in `MainUi::new`

Replace this code in `desktop/wx/src/main.rs`:

```rust
        let main_panel = find_xrc_child::<Panel>(&main_frame, "main_panel");
        let frame_sizer = BoxSizer::builder(Orientation::Vertical).build();
        frame_sizer.add(&main_panel, 1, SizerFlag::Expand, 0);
        main_frame.set_sizer(frame_sizer, true);
        let root_tabs = find_xrc_child::<Notebook>(&main_frame, "root_tabs");
        root_tabs.set_min_size(Size::new(640, 360));
        let compare_page = root_tabs
            .get_page(0)
            .unwrap_or_else(|| panic!("Missing compare page in root_tabs"));
        let recents_page = root_tabs
            .get_page(1)
            .unwrap_or_else(|| panic!("Missing recents page in root_tabs"));
        let batch_page = root_tabs
            .get_page(2)
            .unwrap_or_else(|| panic!("Missing batch page in root_tabs"));
        let search_page = root_tabs
            .get_page(3)
            .unwrap_or_else(|| panic!("Missing search page in root_tabs"));

        let sheets_list = find_xrc_child::<Panel>(&compare_page, "sheets_list");
        let result_tabs = find_xrc_child::<Notebook>(&compare_page, "result_tabs");
        result_tabs.set_min_size(Size::new(420, 240));
```

with:

```rust
        let main_panel = find_xrc_child::<Panel>(&main_frame, "main_panel");
        let frame_sizer = BoxSizer::builder(Orientation::Vertical).build();
        frame_sizer.add(&main_panel, 1, SizerFlag::Expand, 0);
        main_frame.set_sizer(frame_sizer, true);

        let root_tabs = find_xrc_child::<Notebook>(&main_frame, "root_tabs");
        let compare_page = root_tabs
            .get_page(0)
            .unwrap_or_else(|| panic!("Missing compare page in root_tabs"));
        let recents_page = root_tabs
            .get_page(1)
            .unwrap_or_else(|| panic!("Missing recents page in root_tabs"));
        let batch_page = root_tabs
            .get_page(2)
            .unwrap_or_else(|| panic!("Missing batch page in root_tabs"));
        let search_page = root_tabs
            .get_page(3)
            .unwrap_or_else(|| panic!("Missing search page in root_tabs"));

        let sheets_list = find_xrc_child::<Panel>(&compare_page, "sheets_list");
        let result_tabs = find_xrc_child::<Notebook>(&compare_page, "result_tabs");
```

### Change 2 (optional): don’t set frame size twice

If you decide to move frame sizing into XRC (or into `MainUi::new` immediately after `load_frame`), then replace:

```rust
            ctx.ui.frame.set_min_size(Size::new(960, 640));
            ctx.ui.frame.set_size(Size::new(1280, 900));
            ctx.ui.frame.show(true);
```

with:

```rust
            ctx.ui.frame.show(true);
```

Keep your `call_after` layout — it’s a good GTK stabilizer. 

---

## What I would do next (to make it airtight)

1. Make the above code change (remove notebook min sizes).
2. Add `<size>` and `<minsize>` to `main_frame` in XRC so the *creation-time* geometry is sane (this is the key if the warning is inside `load_frame()`).
3. If you still see it, implement lazy page creation or switch to AUI notebook.
