use wxdragon::prelude::Size;

pub(crate) const SHEETS_COLUMNS: [(&str, i32); 6] = [
    ("Sheet", 200),
    ("Ops", 70),
    ("Added", 70),
    ("Removed", 92),
    ("Modified", 80),
    ("Moved", 70),
];

pub(crate) const RECENTS_COLUMNS: [(&str, i32); 4] =
    [("Old", 220), ("New", 220), ("Last Run", 160), ("Mode", 80)];

pub(crate) const BATCH_COLUMNS: [(&str, i32); 6] = [
    ("Old", 200),
    ("New", 200),
    ("Status", 90),
    ("Ops", 70),
    ("Warnings", 90),
    ("Error", 260),
];

pub(crate) const SEARCH_COLUMNS: [(&str, i32); 5] = [
    ("Kind", 120),
    ("Sheet", 180),
    ("Address", 100),
    ("Label", 200),
    ("Detail", 260),
];

pub(crate) const SUMMARY_CATEGORY_COLUMNS: [(&str, i32); 5] = [
    ("Category", 140),
    ("Ops", 70),
    ("High", 70),
    ("Med", 70),
    ("Low", 70),
];

pub(crate) const SUMMARY_TOP_SHEETS_COLUMNS: [(&str, i32); 5] = [
    ("Sheet", 220),
    ("Ops", 70),
    ("High", 70),
    ("Med", 70),
    ("Low", 70),
];

pub(crate) fn default_window_size() -> Size {
    Size::new(1280, 900)
}

pub(crate) fn min_window_size() -> Size {
    Size::new(960, 640)
}

pub(crate) fn min_root_tabs_size() -> Size {
    Size::new(640, 360)
}

pub(crate) const DEFAULT_SASH_POSITION: i32 = 420;
pub(crate) const MIN_SASH_POSITION: i32 = 260;

// wxWidgets key codes for F6/F8 (WXK_F1=340).
pub(crate) const WXK_F6: i32 = 345;
pub(crate) const WXK_F8: i32 = 347;

pub(crate) const RESULT_TAB_DETAILS: i32 = 1;
pub(crate) const RESULT_TAB_EXPLAIN: i32 = 2;
pub(crate) const RESULT_TAB_GRID: i32 = 3;

pub(crate) const GUIDED_EMPTY_SUMMARY: &str =
    "Select Old and New files, pick a preset, then click Compare (F5).\n\nTip: Use Swap to flip Old/New.";
pub(crate) const GUIDED_EMPTY_DETAILS: &str =
    "After comparing, select a sheet to see details.\n\nSelect Old and New files, pick a preset, then click Compare (F5).";
pub(crate) const GUIDED_EMPTY_EXPLAIN: &str =
    "After comparing, click a changed cell in the Grid preview to see a best-effort explanation.";
