# UI Error Log and Fix Attempts

This document tracks UI/runtime errors observed during development and the actions taken to address them.
Entries are appended chronologically.

---

## 2025-02-14 (Session Log)

### Error: Rust compile errors after XRC/validation changes

**Observed output**
- `desktop/wx/src/main.rs:222:13` mismatched types: expected `Panel`, found `Window`
- `desktop/wx/src/xrc_validation.rs:90:32` borrow of moved value `frame`

**Actions attempted**
- Restored `compare_container` to be looked up via `find_xrc_child::<Panel>(..., "compare_container")`.
- Cloned the `frame` name before pushing into the stack in `xrc_validation.rs` to avoid move/borrow errors.

---

### Error: XRC validation failures for sizer expansion rules

**Observed output**
- Validation panic listing missing expand/proportion for `root_tabs`, `compare_splitter`, `result_tabs`, `old_picker`, `new_picker`.

**Actions attempted**
- Updated the XRC validator to treat `sizeritem` as `<object class="sizeritem">` (not just a `<sizeritem>` tag).
- Added tracking of `flag` and `proportion/option` on sizer items and enforced expansion rules.

---

### Error: GTK critical warnings at startup (scrollbar sizing)

**Observed output**
- `gtk_box_gadget_distribute: assertion 'size >= 0' failed in GtkScrollbar`
- `gtk_widget_get_preferred_width_for_height: assertion 'height >= 0' failed`
- `gtk_widget_size_allocate(): attempt to allocate widget with width X and height -17`
- `pixman_region32_init_rect: Invalid rectangle passed` (BUG)

**Actions attempted**
1. Added splitter in Compare view and initialized it after show, then moved to before show:
   - `compare_splitter.set_minimum_pane_size(200)`
   - `split_vertically` with default sash
   - `set_sash_position` in `call_after`
2. Added layout/size forcing to stabilize GTK allocation:
   - `frame.layout()`
   - `frame.set_size(frame.get_size())` to trigger a size event
3. Added layout debug logging (`EXCEL_DIFF_DEBUG_LAYOUT=1`) to inspect sizes.

**Latest layout debug output**
- `frame=1332x989`
- `root_tabs=1268x839`
- `compare_container=1266x800`
- `result_tabs=1254x353`
- `sheets_list=1254x320`

**Actions attempted after size logging**
- Replaced `<proportion>` with `<option>` in XRC so sizer proportions take effect.
- Deferred creation of all `DataViewCtrl` widgets to a `call_after` after the frame is shown to avoid scrollbars allocating at zero height.

---

### Notes
- `Gdk-Message: Unable to load sb_h_double_arrow / sb_v_double_arrow from the cursor theme` appeared repeatedly. This appears non-fatal and likely theme-related.


---

## 2025-02-14 (Session Log, continued)

### Error: `call_after` closure required `Send` but captured UI handles

**Observed output**
- `E0277: *mut wxd_Window_t cannot be sent between threads safely` (captured `Frame`)
- `E0277: *mut wxd_MenuItem_t cannot be sent between threads safely` (captured `MenuItem`)
- Triggered by `wxdragon::call_after(Box::new(move || { ... }))` capturing `MainUi`/UI handles.

**Actions attempted**
- Refactored `UiHandles` to store `Option<DataViewCtrl>` and `AppState` to store `Option<DataViewListModel>` so UI context can be initialized before list views exist.
- Moved DataViewCtrl creation into a `call_after` closure that **captures nothing** and uses `with_ui_context` to access UI handles.
- Added panel handles (`recents_list_panel`, `batch_results_list_panel`, `search_results_list_panel`) to `UiHandles` so deferred view creation can access parents without capturing `MainUi`.
- Guarded uses of list views in `populate_*` and selection handlers to avoid early access before views are initialized.

