use std::cell::RefCell;
use std::cmp::Reverse;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::raw::{c_char, c_void};
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod dev_scenario;
mod explain;
mod grid_preview;
mod logic;
mod profiles;
mod profiles_dialog;
mod theme;
mod ui;
mod ui_constants;
mod xrc_validation;

use desktop_backend::{
    BackendConfig, BatchOutcome, BatchRequest, CellsRangeRequest, DesktopBackend, DiffErrorPayload,
    DiffMode, DiffOutcome, DiffRequest, DiffRunSummary, OpsRangeRequest, ProgressEvent, ProgressRx,
    RangeBounds, RecentComparison, SearchIndexResult, SearchIndexSummary, SearchResult,
    SheetMetaRequest, SheetPayloadRequest,
};
use dev_scenario::{load_from_env as load_dev_scenario, UiScenario};
use excel_diff::{CellAddress, DiffReport};
use license_client::LicenseClient;
use log::{debug, info, LevelFilter, Metadata, Record};
use logic::{base_name, parse_globs, preset_from_selection};
use profiles::CompareProfile;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ui_payload::{DiffOptions, DiffPreset};
use wxdragon::event::{TextEvents, WebViewEvents};
use wxdragon::prelude::*;
use wxdragon::widgets::dataview::{CustomDataViewVirtualListModel, DataViewItemAttr, Variant};
use wxdragon::widgets::{WebView, WebViewBackend, WebViewUserScriptInjectionTime};
use wxdragon_sys as ffi;

#[cfg(target_os = "linux")]
use libc;

use ui_constants::{
    default_window_size, min_window_size, BATCH_COLUMNS, DEFAULT_SASH_POSITION,
    GUIDED_EMPTY_DETAILS, GUIDED_EMPTY_EXPLAIN, GUIDED_EMPTY_SUMMARY, MIN_SASH_POSITION,
    RECENTS_COLUMNS, RESULT_TAB_DETAILS, RESULT_TAB_EXPLAIN, RESULT_TAB_GRID, SEARCH_COLUMNS,
    SHEETS_COLUMNS, SUMMARY_CATEGORY_COLUMNS, SUMMARY_TOP_SHEETS_COLUMNS, WXK_F6, WXK_F8,
};

static PROGRESS_ANIM_GEN: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProgressStage {
    Read,
    Parse,
    Diff,
    Snapshot,
    Batch,
    Other,
}

impl ProgressStage {
    fn from_stage_name(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "read" => Self::Read,
            "parse" => Self::Parse,
            "diff" => Self::Diff,
            "snapshot" => Self::Snapshot,
            "batch" => Self::Batch,
            _ => Self::Other,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Read => "Read",
            Self::Parse => "Parse",
            Self::Diff => "Diff",
            Self::Snapshot => "Snapshot",
            Self::Batch => "Batch",
            Self::Other => "Working",
        }
    }

    fn gauge_bounds(self) -> (i32, i32) {
        match self {
            // Short, usually IO-bound.
            Self::Read => (0, 25),
            // Most of the runtime for OpenXML workbooks; give it most of the bar range.
            Self::Parse => (25, 85),
            // Most of the runtime; keep the bar moving over most of the range.
            Self::Diff => (25, 85),
            // Usually quick, but make it feel "near the end".
            Self::Snapshot => (85, 100),
            Self::Batch => (0, 100),
            Self::Other => (0, 100),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct UiState {
    window_x: Option<i32>,
    window_y: Option<i32>,
    window_width: Option<i32>,
    window_height: Option<i32>,
    window_maximized: Option<bool>,
    root_tab: Option<usize>,
    compare_sash: Option<i32>,
    sheets_panel_visible: Option<bool>,
    last_old_path: Option<String>,
    last_new_path: Option<String>,
    profile_id: Option<String>,
    preset_choice: Option<u32>,
    trusted_files: Option<bool>,
}

struct VirtualTable {
    model: CustomDataViewVirtualListModel,
    rows: Rc<RefCell<Vec<Vec<String>>>>,
}

fn load_ui_state(path: &Path) -> UiState {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return UiState::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_ui_state(path: &Path, state: &UiState) {
    let Ok(payload) = serde_json::to_string_pretty(state) else {
        return;
    };
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
    else {
        return;
    };
    let _ = file.write_all(payload.as_bytes());
}

fn clear_ui_state(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn capture_ui_state(ctx: &UiContext) -> UiState {
    let size = ctx.ui.frame.get_size();
    let pos = ctx.ui.frame.get_position();
    let selection = ctx.ui.root_tabs.selection();
    let old_path = ctx.ui.old_picker.get_path();
    let new_path = ctx.ui.new_picker.get_path();

    UiState {
        window_x: Some(pos.x),
        window_y: Some(pos.y),
        window_width: Some(size.width),
        window_height: Some(size.height),
        window_maximized: Some(ctx.ui.frame.is_maximized()),
        root_tab: if selection >= 0 {
            Some(selection as usize)
        } else {
            None
        },
        compare_sash: Some(ctx.state.sheets_sash_position),
        sheets_panel_visible: Some(ctx.state.sheets_panel_visible),
        last_old_path: if old_path.trim().is_empty() {
            None
        } else {
            Some(old_path)
        },
        last_new_path: if new_path.trim().is_empty() {
            None
        } else {
            Some(new_path)
        },
        profile_id: ctx
            .ui
            .profile_choice
            .get_selection()
            .and_then(|idx| ctx.state.profile_choice_ids.get(idx as usize).cloned()),
        preset_choice: ctx.ui.preset_choice.get_selection(),
        trusted_files: Some(ctx.ui.trusted_checkbox.is_checked()),
    }
}

fn apply_frame_state(ctx: &mut UiContext, ui_state: &UiState) {
    if let (Some(width), Some(height)) = (ui_state.window_width, ui_state.window_height) {
        if width > 0 && height > 0 {
            let min_size = min_window_size();
            let width = width.max(min_size.width);
            let height = height.max(min_size.height);
            if let (Some(x), Some(y)) = (ui_state.window_x, ui_state.window_y) {
                ctx.ui.frame.set_size_with_pos(x, y, width, height);
            } else {
                ctx.ui.frame.set_size(Size::new(width, height));
            }
        }
    } else if let (Some(x), Some(y)) = (ui_state.window_x, ui_state.window_y) {
        ctx.ui.frame.move_window(x, y);
    }
}

fn should_start_maximized(ui_state: &UiState) -> bool {
    if window_size_override().is_some() || env_string("EXCEL_DIFF_DEV_SCENARIO").is_some() {
        return false;
    }
    if let Some(value) = env_flag("EXCEL_DIFF_START_MAXIMIZED") {
        return value;
    }
    if let Some(value) = ui_state.window_maximized {
        return value;
    }
    true
}

fn apply_ui_state(ctx: &mut UiContext, ui_state: &UiState) {
    apply_frame_state(ctx, ui_state);
    if let Some(old_path) = ui_state.last_old_path.as_ref() {
        ctx.ui.old_picker.set_path(old_path);
    }
    if let Some(new_path) = ui_state.last_new_path.as_ref() {
        ctx.ui.new_picker.set_path(new_path);
    }

    rebuild_profile_choice_in_ctx(ctx, ui_state.profile_id.as_deref());
    if ui_state.profile_id.is_none() {
        // Backwards-compatible: old UI state persisted preset/trusted before profiles existed.
        if let Some(preset) = ui_state.preset_choice {
            let max = ctx.ui.preset_choice.get_count().saturating_sub(1);
            let choice = (preset as u32).min(max);
            ctx.ui.preset_choice.set_selection(choice);
        }
        if let Some(trusted) = ui_state.trusted_files {
            ctx.ui.trusted_checkbox.set_value(trusted);
        }
    }

    if let Some(tab) = ui_state.root_tab {
        if tab < ctx.ui.root_tabs.get_page_count() {
            ctx.ui.root_tabs.set_selection(tab);
        }
    }

    ctx.ui
        .compare_splitter
        .set_minimum_pane_size(MIN_SASH_POSITION);
    // XRC may create the splitter in a horizontal split mode (top/bottom). Force the
    // legacy UI into a stable vertical split: sheets list on the left, results on the right.
    let _ = ctx.ui.compare_splitter.unsplit(None::<&Panel>);
    ctx.ui
        .compare_splitter
        .initialize(&ctx.ui.compare_right_panel);
    let visible = ui_state
        .sheets_panel_visible
        .unwrap_or(ctx.state.sheets_panel_visible);
    let mut sash = ui_state
        .compare_sash
        .unwrap_or(ctx.state.sheets_sash_position);
    if sash < MIN_SASH_POSITION {
        sash = DEFAULT_SASH_POSITION;
    }
    ctx.state.sheets_panel_visible = visible;
    ctx.state.sheets_sash_position = sash;
    if visible {
        if !ctx.ui.compare_splitter.split_vertically(
            &ctx.ui.sheets_list_panel,
            &ctx.ui.compare_right_panel,
            sash,
        ) {
            ctx.ui.compare_splitter.set_sash_position(sash, false);
        }
    } else {
        // `initialize(compare_right_panel)` above ensures the right panel is the visible one.
        let _ = ctx
            .ui
            .compare_splitter
            .unsplit(Some(&ctx.ui.sheets_list_panel));
    }
    ctx.ui.toggle_sheets_menu.check(visible);
    sync_compare_controls_in_ctx(ctx);
}

struct UiHandles {
    frame: Frame,
    main_panel: Panel,
    open_pair_menu: MenuItem,
    open_old_menu: MenuItem,
    open_new_menu: MenuItem,
    open_recent_menu: MenuItem,
    exit_menu: MenuItem,
    compare_menu: MenuItem,
    cancel_menu: MenuItem,
    export_audit_menu: MenuItem,
    next_diff_menu: MenuItem,
    prev_diff_menu: MenuItem,
    copy_menu: MenuItem,
    find_menu: MenuItem,
    toggle_sheets_menu: MenuItem,
    reset_layout_menu: MenuItem,
    minimize_window_menu: MenuItem,
    toggle_maximize_window_menu: MenuItem,
    license_menu: MenuItem,
    docs_menu: MenuItem,
    about_menu: MenuItem,
    status_bar: StatusBar,
    progress_text: StaticText,
    progress_gauge: Gauge,
    status_pill: Panel,
    compare_btn: Button,
    cancel_btn: Button,
    old_picker: FilePickerCtrl,
    new_picker: FilePickerCtrl,
    swap_btn: Button,
    compare_help_text: StaticText,
    profile_choice: Choice,
    preset_choice: Choice,
    trusted_checkbox: CheckBox,
    profiles_btn: Button,
    run_summary_old: StaticText,
    run_summary_new: StaticText,
    run_summary_meta: StaticText,
    summary_warning_panel: Panel,
    summary_warning_text: StaticText,
    summary_added_value: StaticText,
    summary_removed_value: StaticText,
    summary_modified_value: StaticText,
    summary_moved_value: StaticText,
    summary_categories_table_host: Panel,
    summary_top_sheets_table_host: Panel,
    summary_text: TextCtrl,
    detail_text: TextCtrl,
    explain_text: TextCtrl,
    grid_panel: Panel,
    root_tabs: Notebook,
    compare_container: Panel,
    result_tabs: Notebook,
    sheets_list_panel: Panel,
    sheets_table_host: Panel,
    sheets_filter_ctrl: SearchCtrl,
    hide_m_formatting_checkbox: CheckBox,
    hide_dax_formatting_checkbox: CheckBox,
    hide_formula_formatting_checkbox: CheckBox,
    collapse_moves_checkbox: CheckBox,
    sheets_filter_status: StaticText,
    sheets_empty_panel: Panel,
    sheets_empty_text: StaticText,
    recents_list_panel: Panel,
    batch_results_list_panel: Panel,
    search_results_list_panel: Panel,
    compare_splitter: SplitterWindow,
    compare_right_panel: Panel,
    open_recent_btn: Button,
    run_batch_btn: Button,
    search_btn: Button,
    build_old_index_btn: Button,
    build_new_index_btn: Button,
    search_ctrl: SearchCtrl,
    search_scope_choice: Choice,
    batch_old_dir: DirPickerCtrl,
    batch_new_dir: DirPickerCtrl,
    include_glob_text: TextCtrl,
    exclude_glob_text: TextCtrl,
    sheets_view: Option<DataViewCtrl>,
    recents_view: Option<DataViewCtrl>,
    batch_view: Option<DataViewCtrl>,
    search_view: Option<DataViewCtrl>,
    summary_categories_view: Option<DataViewCtrl>,
    summary_top_sheets_view: Option<DataViewCtrl>,
    webview: Option<WebView>,
    grid_webview: Option<WebView>,
    grid_fallback: Option<TextCtrl>,
}

struct ActiveRun {
    run_id: u64,
    stage: ProgressStage,
    cancel: Arc<AtomicBool>,
    cancel_requested: bool,
}

#[derive(Debug, Clone)]
struct SheetRow {
    sheet_name: String,
    op_count: u64,
    added: u64,
    removed: u64,
    modified: u64,
    moved: u64,
}

struct CancelRestoreSnapshot {
    current_diff_id: Option<String>,
    current_mode: Option<DiffMode>,
    current_summary: Option<DiffRunSummary>,
    current_payload: Option<Arc<ui_payload::DiffWithSheets>>,
    display_analysis: Option<ui_payload::DiffAnalysis>,
    pending_detail_payload: Option<Arc<ui_payload::DiffWithSheets>>,
    pending_detail_sheet_name: Option<String>,
    pending_detail_payload_gen: u64,
    pending_detail_json: Option<String>,
    pending_detail_json_gen: Option<u64>,
    sheet_names: Vec<String>,
    sheets_all: Vec<SheetRow>,
    sheets_filter: String,
    summary_text: String,
    detail_text: String,
    explain_text: String,
    selected_sheet: Option<String>,
    result_tab: usize,
}

struct AppState {
    backend: DesktopBackend,
    engine_version: String,
    run_counter: u64,
    active_run: Option<ActiveRun>,
    cancel_restore_snapshot: Option<CancelRestoreSnapshot>,
    current_diff_id: Option<String>,
    current_mode: Option<DiffMode>,
    current_summary: Option<DiffRunSummary>,
    current_payload: Option<Arc<ui_payload::DiffWithSheets>>,
    pending_detail_payload: Option<Arc<ui_payload::DiffWithSheets>>,
    pending_detail_sheet_name: Option<String>,
    pending_detail_payload_gen: u64,
    pending_detail_render_epoch: u64,
    pending_detail_json: Option<String>,
    pending_detail_json_gen: Option<u64>,
    pending_detail_json_inflight_gen: Option<u64>,
    sheet_names: Vec<String>,
    sheets_all: Vec<SheetRow>,
    sheets_filter: String,
    recents: Vec<RecentComparison>,
    search_old_index: Option<SearchIndexSummary>,
    search_new_index: Option<SearchIndexSummary>,
    batch_outcome: Option<BatchOutcome>,
    sheets_table: Option<VirtualTable>,
    recents_table: Option<VirtualTable>,
    batch_table: Option<VirtualTable>,
    search_table: Option<VirtualTable>,
    summary_categories_table: Option<VirtualTable>,
    summary_top_sheets_table: Option<VirtualTable>,
    webview_enabled: bool,
    sheets_panel_visible: bool,
    sheets_sash_position: i32,
    ui_state_path: PathBuf,
    profiles_path: PathBuf,
    user_profiles: Vec<CompareProfile>,
    profile_choice_ids: Vec<String>,
    selected_profile_id: Option<String>,
    display_analysis: Option<ui_payload::DiffAnalysis>,
    dev_scenario: Option<UiScenario>,
    dev_ready_file: Option<PathBuf>,
    dev_ready_fired: bool,
}

struct UiContext {
    ui: UiHandles,
    state: AppState,
}

thread_local! {
    static UI_CONTEXT: RefCell<Option<UiContext>> = RefCell::new(None);
}

fn with_ui_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut UiContext) -> R,
{
    UI_CONTEXT.with(|ctx| {
        let mut ctx_ref = ctx.borrow_mut();
        let ctx = ctx_ref.as_mut()?;
        Some(f(ctx))
    })
}

fn update_status_in_ctx(ctx: &mut UiContext, message: &str) {
    ctx.ui.progress_text.set_label(message);
    ctx.ui.status_bar.set_status_text(message, 0);
}

fn update_run_summary_header_in_ctx(ctx: &mut UiContext) {
    let old_path = ctx.ui.old_picker.get_path();
    let new_path = ctx.ui.new_picker.get_path();

    if old_path.trim().is_empty() {
        ctx.ui.run_summary_old.set_label("Old: -");
        ctx.ui.run_summary_old.set_tooltip("");
    } else {
        ctx.ui
            .run_summary_old
            .set_label(&format!("Old: {}", base_name(&old_path)));
        ctx.ui.run_summary_old.set_tooltip(&old_path);
    }

    if new_path.trim().is_empty() {
        ctx.ui.run_summary_new.set_label("New: -");
        ctx.ui.run_summary_new.set_tooltip("");
    } else {
        ctx.ui
            .run_summary_new
            .set_label(&format!("New: {}", base_name(&new_path)));
        ctx.ui.run_summary_new.set_tooltip(&new_path);
    }

    let meta = if let Some(active) = ctx.state.active_run.as_ref() {
        if active.cancel_requested {
            "Cancel requested (finishing current step)...".to_string()
        } else {
            "Comparing...".to_string()
        }
    } else if let Some(summary) = ctx.state.current_summary.as_ref() {
        let complete = if summary.complete { "yes" } else { "no" };
        format!(
            "Mode: {} | {} ops | +{} -{} ~{} ↔{} | Warnings: {} | Complete: {}",
            summary.mode.as_str(),
            summary.op_count,
            summary.counts.added,
            summary.counts.removed,
            summary.counts.modified,
            summary.counts.moved,
            summary.warnings.len(),
            complete
        )
    } else {
        "Mode: - | Ops: - | Warnings: - | Complete: -".to_string()
    };
    ctx.ui.run_summary_meta.set_label(&meta);
}

fn sync_sheets_filter_status_in_ctx(ctx: &mut UiContext) {
    let total = ctx.state.sheets_all.len();
    let shown = ctx.state.sheet_names.len();
    let filter = ctx.state.sheets_filter.trim();

    let status = if total == 0 {
        "Sheets: none".to_string()
    } else if filter.is_empty() {
        format!("Sheets: {total} (sorted by ops)")
    } else {
        format!("Sheets: {shown} of {total}")
    };
    ctx.ui.sheets_filter_status.set_label(&status);

    let filter_status = if filter.is_empty() {
        "Filter: none".to_string()
    } else {
        format!("Filter: {filter}")
    };
    ctx.ui.status_bar.set_status_text(&filter_status, 2);
}

fn sync_sheets_panel_state_in_ctx(ctx: &mut UiContext) {
    let total = ctx.state.sheets_all.len();
    let shown = ctx.state.sheet_names.len();
    let filter = ctx.state.sheets_filter.trim();
    let has_run = ctx.state.current_summary.is_some();

    if let Some(active) = ctx.state.active_run.as_ref() {
        ctx.ui.sheets_table_host.show(false);
        ctx.ui.sheets_empty_panel.show(true);
        if active.cancel_requested {
            ctx.ui
                .sheets_empty_text
                .set_label("Cancel requested (finishing current step)...");
        } else {
            ctx.ui.sheets_empty_text.set_label("Comparing...");
        }
        ctx.ui.sheets_filter_ctrl.enable(false);
    } else if total == 0 {
        ctx.ui.sheets_table_host.show(false);
        ctx.ui.sheets_empty_panel.show(true);
        ctx.ui.sheets_filter_ctrl.enable(false);
        if let Some(summary) = ctx.state.current_summary.as_ref() {
            if summary.op_count == 0 {
                ctx.ui
                    .sheets_empty_text
                    .set_label("No differences detected.");
            } else {
                ctx.ui
                    .sheets_empty_text
                    .set_label("No sheet-level changes were detected.");
            }
        } else if !has_run {
            ctx.ui
                .sheets_empty_text
                .set_label("Run Compare to list changed sheets.");
        }
    } else if shown == 0 && !filter.is_empty() {
        ctx.ui.sheets_table_host.show(false);
        ctx.ui.sheets_empty_panel.show(true);
        ctx.ui.sheets_filter_ctrl.enable(true);
        ctx.ui
            .sheets_empty_text
            .set_label("No sheets match the current filter.");
    } else {
        ctx.ui.sheets_table_host.show(true);
        ctx.ui.sheets_empty_panel.show(false);
        ctx.ui.sheets_filter_ctrl.enable(true);
    }

    sync_sheets_filter_status_in_ctx(ctx);
    ctx.ui.sheets_list_panel.layout();
    ctx.ui.compare_container.layout();
}

fn sync_compare_controls_in_ctx(ctx: &mut UiContext) {
    let old_ok = !ctx.ui.old_picker.get_path().trim().is_empty();
    let new_ok = !ctx.ui.new_picker.get_path().trim().is_empty();
    let (running, cancel_requested) = match ctx.state.active_run.as_ref() {
        Some(active) => (true, active.cancel_requested),
        None => (false, false),
    };

    let can_compare = old_ok && new_ok && !running;
    ctx.ui.compare_btn.enable(can_compare);
    ctx.ui.cancel_btn.enable(running && !cancel_requested);
    ctx.ui.swap_btn.enable(!running && (old_ok || new_ok));

    // MenuItem wrappers loaded via XRC can't be enabled/disabled directly; use the MenuBar.
    if let Some(menu_bar) = ctx.ui.frame.get_menu_bar() {
        let _ = menu_bar.enable_item(ctx.ui.compare_menu.get_id(), can_compare);
        let _ = menu_bar.enable_item(ctx.ui.cancel_menu.get_id(), running && !cancel_requested);
    }

    let (help_label, show_help) = if running {
        ("", false)
    } else if !old_ok && !new_ok {
        ("Select Old and New files to enable Compare.", true)
    } else if !old_ok {
        ("Select an Old file to enable Compare.", true)
    } else if !new_ok {
        ("Select a New file to enable Compare.", true)
    } else {
        ("", false)
    };

    let was_shown = ctx.ui.compare_help_text.is_shown();
    if show_help {
        ctx.ui.compare_help_text.set_label(help_label);
    }
    ctx.ui.compare_help_text.show(show_help);
    if was_shown != show_help {
        ctx.ui.compare_container.layout();
        ctx.ui.frame.layout();
    }

    update_run_summary_header_in_ctx(ctx);
    sync_sheets_panel_state_in_ctx(ctx);
}

fn available_profiles_in_ctx(ctx: &UiContext) -> Vec<CompareProfile> {
    let mut profiles = profiles::builtin_profiles();
    profiles.extend(ctx.state.user_profiles.iter().cloned());
    profiles
}

fn profile_by_id_in_ctx(ctx: &UiContext, id: &str) -> Option<CompareProfile> {
    available_profiles_in_ctx(ctx)
        .into_iter()
        .find(|profile| profile.id == id)
}

fn sync_selected_profile_id_in_ctx(ctx: &mut UiContext) {
    let id = ctx
        .ui
        .profile_choice
        .get_selection()
        .and_then(|idx| ctx.state.profile_choice_ids.get(idx as usize).cloned());
    ctx.state.selected_profile_id = id;
}

fn apply_profile_to_controls_in_ctx(ctx: &mut UiContext, profile: &CompareProfile) {
    ctx.ui.trusted_checkbox.set_value(profile.trusted_files);

    let preset_index = match profile.preset {
        DiffPreset::Balanced => 0,
        DiffPreset::Fastest => 1,
        DiffPreset::MostPrecise => 2,
    };
    let max = ctx.ui.preset_choice.get_count().saturating_sub(1);
    ctx.ui.preset_choice.set_selection(preset_index.min(max));

    ctx.ui
        .hide_m_formatting_checkbox
        .set_value(profile.noise_filters.hide_m_formatting_only);
    ctx.ui
        .hide_dax_formatting_checkbox
        .set_value(profile.noise_filters.hide_dax_formatting_only);
    ctx.ui
        .hide_formula_formatting_checkbox
        .set_value(profile.noise_filters.hide_formula_formatting_only);
    ctx.ui
        .collapse_moves_checkbox
        .set_value(profile.noise_filters.collapse_moves);
}

fn rebuild_profile_choice_in_ctx(ctx: &mut UiContext, target_profile_id: Option<&str>) {
    let profiles = available_profiles_in_ctx(ctx);
    ctx.ui.profile_choice.clear();
    ctx.state.profile_choice_ids.clear();

    for profile in &profiles {
        ctx.ui.profile_choice.append(&profile.name);
        ctx.state.profile_choice_ids.push(profile.id.clone());
    }

    let selected_idx = target_profile_id
        .and_then(|target| {
            ctx.state
                .profile_choice_ids
                .iter()
                .position(|id| id == target)
        })
        .or_else(|| {
            ctx.state.selected_profile_id.as_deref().and_then(|target| {
                ctx.state
                    .profile_choice_ids
                    .iter()
                    .position(|id| id == target)
            })
        })
        .unwrap_or(0);

    if ctx.ui.profile_choice.get_count() > 0 {
        ctx.ui.profile_choice.set_selection(selected_idx as u32);
    }
    sync_selected_profile_id_in_ctx(ctx);

    if let Some(profile_id) = ctx.state.selected_profile_id.clone() {
        if let Some(profile) = profile_by_id_in_ctx(ctx, &profile_id) {
            apply_profile_to_controls_in_ctx(ctx, &profile);
        }
    }
}

fn clear_diff_results_in_ctx(ctx: &mut UiContext) {
    ctx.state.current_diff_id = None;
    ctx.state.current_mode = None;
    ctx.state.current_summary = None;
    ctx.state.current_payload = None;
    ctx.state.display_analysis = None;
    ctx.state.pending_detail_payload = None;
    ctx.state.pending_detail_sheet_name = None;
    ctx.state.pending_detail_payload_gen = ctx.state.pending_detail_payload_gen.wrapping_add(1);
    ctx.state.pending_detail_render_epoch = ctx.state.pending_detail_render_epoch.wrapping_add(1);
    ctx.state.pending_detail_json = None;
    ctx.state.pending_detail_json_gen = None;
    ctx.state.pending_detail_json_inflight_gen = None;

    ctx.state.sheets_all.clear();
    ctx.state.sheet_names.clear();
    ctx.state.sheets_filter.clear();
    ctx.ui.sheets_filter_ctrl.set_value("");

    if let Some(view) = ctx.ui.sheets_view {
        view.unselect_all();
    }
    if let Some(table) = ctx.state.sheets_table.as_mut() {
        update_virtual_table(table, Vec::new());
    }
    if let Some(table) = ctx.state.summary_categories_table.as_mut() {
        update_virtual_table(table, Vec::new());
    }
    if let Some(table) = ctx.state.summary_top_sheets_table.as_mut() {
        update_virtual_table(table, Vec::new());
    }
    ctx.ui.summary_warning_panel.show(false);
    ctx.ui.summary_added_value.set_label("0");
    ctx.ui.summary_removed_value.set_label("0");
    ctx.ui.summary_modified_value.set_label("0");
    ctx.ui.summary_moved_value.set_label("0");

    ctx.ui.summary_text.set_value(GUIDED_EMPTY_SUMMARY);
    ctx.ui.detail_text.set_value(GUIDED_EMPTY_DETAILS);
    ctx.ui.explain_text.set_value(GUIDED_EMPTY_EXPLAIN);
    render_grid_placeholder(ctx, "Run a diff to preview grid changes.");
    update_status_counts_in_ctx(ctx, None);
    update_run_summary_header_in_ctx(ctx);
    sync_sheets_panel_state_in_ctx(ctx);
}

fn take_cancel_restore_snapshot_in_ctx(ctx: &mut UiContext) -> CancelRestoreSnapshot {
    let selected_sheet = ctx
        .ui
        .sheets_view
        .and_then(|view| view.get_selected_row())
        .and_then(|row| ctx.state.sheet_names.get(row).cloned());

    CancelRestoreSnapshot {
        current_diff_id: ctx.state.current_diff_id.take(),
        current_mode: ctx.state.current_mode.take(),
        current_summary: ctx.state.current_summary.take(),
        current_payload: ctx.state.current_payload.take(),
        display_analysis: ctx.state.display_analysis.take(),
        pending_detail_payload: ctx.state.pending_detail_payload.take(),
        pending_detail_sheet_name: ctx.state.pending_detail_sheet_name.take(),
        pending_detail_payload_gen: ctx.state.pending_detail_payload_gen,
        pending_detail_json: ctx.state.pending_detail_json.take(),
        pending_detail_json_gen: ctx.state.pending_detail_json_gen.take(),
        sheet_names: std::mem::take(&mut ctx.state.sheet_names),
        sheets_all: std::mem::take(&mut ctx.state.sheets_all),
        sheets_filter: std::mem::take(&mut ctx.state.sheets_filter),
        summary_text: ctx.ui.summary_text.get_value(),
        detail_text: ctx.ui.detail_text.get_value(),
        explain_text: ctx.ui.explain_text.get_value(),
        selected_sheet,
        result_tab: usize::try_from(ctx.ui.result_tabs.selection()).unwrap_or(0),
    }
}

fn restore_cancel_snapshot_in_ctx(ctx: &mut UiContext, snapshot: CancelRestoreSnapshot) {
    ctx.state.current_diff_id = snapshot.current_diff_id;
    ctx.state.current_mode = snapshot.current_mode;
    ctx.state.current_summary = snapshot.current_summary;
    ctx.state.current_payload = snapshot.current_payload;
    ctx.state.display_analysis = snapshot.display_analysis;
    ctx.state.pending_detail_payload = snapshot.pending_detail_payload;
    ctx.state.pending_detail_sheet_name = snapshot.pending_detail_sheet_name;
    ctx.state.pending_detail_payload_gen = snapshot.pending_detail_payload_gen;
    ctx.state.pending_detail_json = snapshot.pending_detail_json;
    ctx.state.pending_detail_json_gen = snapshot.pending_detail_json_gen;
    ctx.state.pending_detail_json_inflight_gen = None;
    ctx.state.sheet_names = snapshot.sheet_names;
    ctx.state.sheets_all = snapshot.sheets_all;
    ctx.state.sheets_filter = snapshot.sheets_filter;

    ctx.ui.summary_text.set_value(&snapshot.summary_text);
    ctx.ui.detail_text.set_value(&snapshot.detail_text);
    ctx.ui.explain_text.set_value(&snapshot.explain_text);
    ctx.ui
        .sheets_filter_ctrl
        .set_value(&ctx.state.sheets_filter);
    ctx.ui.result_tabs.set_selection(snapshot.result_tab);

    // Ensure the virtual table matches the restored sheet list.
    rebuild_sheet_list_in_ctx(ctx);

    if let Some(summary) = ctx.state.current_summary.clone() {
        let analysis = ctx.state.display_analysis.clone();
        apply_summary_panel_in_ctx(ctx, &summary, analysis.as_ref());
    }

    if let Some(selected_sheet) = snapshot.selected_sheet {
        if let Some(idx) = ctx
            .state
            .sheet_names
            .iter()
            .position(|name| name == &selected_sheet)
        {
            if let Some(view) = ctx.ui.sheets_view {
                let _ = view.select_row(idx);
            }
        }
    }

    if ctx.ui.result_tabs.selection() == RESULT_TAB_GRID {
        render_grid_for_current_selection(ctx);
    }
    if ctx.ui.result_tabs.selection() == RESULT_TAB_DETAILS {
        render_staged_detail_payload(ctx);
    }
    if ctx.ui.result_tabs.selection() == RESULT_TAB_EXPLAIN {
        // Explanation is already restored from snapshot; no additional work needed.
    }

    // Clone to avoid holding an immutable borrow across the `&mut UiContext` call.
    let summary_for_status = ctx.state.current_summary.clone();
    update_status_counts_in_ctx(ctx, summary_for_status.as_ref());
    update_run_summary_header_in_ctx(ctx);
    sync_sheets_panel_state_in_ctx(ctx);
}

fn handle_compare_inputs_changed() {
    // Some controls emit change events synchronously when their value is set programmatically.
    // Always defer to avoid re-entrant `with_ui_context()` borrows.
    wxdragon::call_after(Box::new(|| {
        let _ = with_ui_context(|ctx| {
            if ctx.state.active_run.is_none() {
                clear_diff_results_in_ctx(ctx);
            }
            sync_compare_controls_in_ctx(ctx);
        });
    }));
}

fn handle_profile_choice_changed() {
    wxdragon::call_after(Box::new(|| {
        let _ = with_ui_context(|ctx| {
            if ctx.state.active_run.is_some() {
                update_status_in_ctx(ctx, "Profile changes apply after the current run.");
                return;
            }

            sync_selected_profile_id_in_ctx(ctx);
            let Some(profile_id) = ctx.state.selected_profile_id.clone() else {
                return;
            };
            let Some(profile) = profile_by_id_in_ctx(ctx, &profile_id) else {
                return;
            };
            apply_profile_to_controls_in_ctx(ctx, &profile);
            update_status_in_ctx(
                ctx,
                "Profile applied. Re-run Compare to apply engine settings.",
            );
        });
    }));
}

fn handle_noise_filters_changed() {
    let mut request: Option<(DesktopBackend, String, ui_payload::NoiseFilters)> = None;
    let _ = with_ui_context(|ctx| {
        let Some(summary) = ctx.state.current_summary.clone() else {
            return;
        };
        if ctx.state.active_run.is_some() {
            // Allow toggles to update UI state, but don't fight with in-flight runs.
            update_status_in_ctx(ctx, "Filters updated (will apply after the run completes).");
            return;
        }

        let filters = noise_filters_from_ui(ctx);

        // Payload mode can re-analyze in-memory; fall back to store-backed analysis when payload isn't loaded.
        if matches!(ctx.state.current_mode, Some(DiffMode::Payload)) {
            if let Some(payload) = ctx.state.current_payload.as_ref() {
                ctx.state.display_analysis =
                    Some(ui_payload::analyze_report(&payload.report, filters));
                apply_display_analysis_in_ctx(ctx, &summary);
                update_status_in_ctx(ctx, "Filters applied.");
                return;
            }
        }

        let Some(diff_id) = ctx.state.current_diff_id.clone() else {
            return;
        };
        request = Some((ctx.state.backend.clone(), diff_id, filters));
        update_status_in_ctx(ctx, "Applying filters...");
    });

    let Some((backend, diff_id, filters)) = request else {
        return;
    };

    thread::spawn(move || {
        let analysis = backend.load_diff_analysis(&diff_id, filters);
        wxdragon::call_after(Box::new(move || {
            let _ = with_ui_context(|ctx| {
                if ctx.state.current_diff_id.as_deref() != Some(diff_id.as_str()) {
                    return;
                }
                let Some(summary) = ctx.state.current_summary.clone() else {
                    return;
                };
                ctx.state.display_analysis = analysis.ok();
                apply_display_analysis_in_ctx(ctx, &summary);
                update_status_in_ctx(ctx, "Filters applied.");
            });
        }));
    });
}

fn update_status_counts_in_ctx(ctx: &mut UiContext, summary: Option<&DiffRunSummary>) {
    if let Some(analysis) = ctx.state.display_analysis.as_ref() {
        let counts = format!(
            "{} ops | +{} -{} ~{} ↔{}",
            analysis.op_count,
            analysis.counts.added,
            analysis.counts.removed,
            analysis.counts.modified,
            analysis.counts.moved
        );
        ctx.ui.status_bar.set_status_text(&counts, 1);
    } else if let Some(summary) = summary {
        let counts = format!(
            "{} ops | +{} -{} ~{} ↔{}",
            summary.op_count,
            summary.counts.added,
            summary.counts.removed,
            summary.counts.modified,
            summary.counts.moved
        );
        ctx.ui.status_bar.set_status_text(&counts, 1);
    } else {
        ctx.ui.status_bar.set_status_text("", 1);
    }
    sync_sheets_filter_status_in_ctx(ctx);
}

fn update_status(message: &str) {
    let message = message.to_string();
    let _ = with_ui_context(|ctx| update_status_in_ctx(ctx, &message));
}

fn profile_name_for_display_in_ctx(ctx: &UiContext) -> Option<String> {
    let id = ctx.state.selected_profile_id.as_deref()?;
    profile_by_id_in_ctx(ctx, id).map(|p| p.name)
}

fn format_summary_scannable(
    summary: &DiffRunSummary,
    analysis: Option<&ui_payload::DiffAnalysis>,
    profile_name: Option<&str>,
) -> String {
    let mut lines = Vec::new();

    if let Some(profile_name) = profile_name {
        lines.push(format!("Profile: {profile_name}"));
    }

    lines.push(format!("Mode: {}", summary.mode.as_str()));
    lines.push(format!("Diff ID: {}", summary.diff_id));
    lines.push(format!("Old: {}", summary.old_path));
    lines.push(format!("New: {}", summary.new_path));
    lines.push(format!("Started: {}", summary.started_at));
    lines.push(format!(
        "Finished: {}",
        summary.finished_at.as_deref().unwrap_or("in progress"),
    ));

    lines.push(String::new());

    if let Some(analysis) = analysis {
        lines.push("Changes (filtered):".to_string());
        lines.push(format!("  Ops: {}", analysis.op_count));
        lines.push(format!(
            "  Counts: +{} -{} ~{} ↔{}",
            analysis.counts.added,
            analysis.counts.removed,
            analysis.counts.modified,
            analysis.counts.moved
        ));
        lines.push(format!(
            "  Severity: high={} med={} low={}",
            analysis.severity.high, analysis.severity.medium, analysis.severity.low
        ));
        lines.push(String::new());
    }

    lines.push("Run health (raw):".to_string());
    lines.push(format!("  Raw ops: {}", summary.op_count));
    lines.push(format!(
        "  Raw counts: +{} -{} ~{} ↔{}",
        summary.counts.added, summary.counts.removed, summary.counts.modified, summary.counts.moved
    ));
    lines.push(format!(
        "  Complete: {}",
        if summary.complete { "yes" } else { "no" }
    ));
    lines.push(format!("  Warnings: {}", summary.warnings.len()));

    if !summary.warnings.is_empty() {
        lines.push(String::new());
        lines.push("Warnings:".to_string());
        for warning in summary.warnings.iter().take(10) {
            lines.push(format!("- {warning}"));
        }
        if summary.warnings.len() > 10 {
            lines.push(format!(
                "... {} more warning(s)",
                summary.warnings.len() - 10
            ));
        }
    }

    if let Some(analysis) = analysis {
        if !analysis.category_breakdown.is_empty() {
            lines.push(String::new());
            lines.push("Categories (filtered):".to_string());
            for row in &analysis.category_breakdown {
                let name = match row.category {
                    ui_payload::OpCategory::Grid => "Grid",
                    ui_payload::OpCategory::PowerQuery => "Power Query",
                    ui_payload::OpCategory::Model => "Model",
                    ui_payload::OpCategory::Objects => "Objects",
                    ui_payload::OpCategory::Other => "Other",
                };
                lines.push(format!(
                    "- {name}: {} (high={} med={} low={})",
                    row.total, row.severity.high, row.severity.medium, row.severity.low
                ));
            }
        }

        if !analysis.sheets.is_empty() {
            lines.push(String::new());
            lines.push("Top sheets (filtered):".to_string());
            for sheet in analysis.sheets.iter().take(10) {
                lines.push(format!(
                    "- {}: {} ops (high={} med={} low={}) | +{} -{} ~{} ↔{}",
                    sheet.sheet_name,
                    sheet.op_count,
                    sheet.severity.high,
                    sheet.severity.medium,
                    sheet.severity.low,
                    sheet.counts.added,
                    sheet.counts.removed,
                    sheet.counts.modified,
                    sheet.counts.moved
                ));
            }
        }
    }

    lines.join("\n")
}

fn apply_summary_panel_in_ctx(
    ctx: &mut UiContext,
    summary: &DiffRunSummary,
    analysis: Option<&ui_payload::DiffAnalysis>,
) {
    // Warning banner must reflect the raw run, not the filtered view.
    let warn = !summary.complete || !summary.warnings.is_empty();
    if warn {
        let mut text = if summary.complete {
            format!("Warnings: {}", summary.warnings.len())
        } else {
            format!(
                "Run incomplete. Warnings: {} (results may be partial)",
                summary.warnings.len()
            )
        };
        if let Some(first) = summary.warnings.first() {
            if !first.trim().is_empty() {
                text.push_str(" | ");
                text.push_str(first.trim());
                if summary.warnings.len() > 1 {
                    text.push_str(" ...");
                }
            }
        }
        ctx.ui.summary_warning_text.set_label(&text);
        ctx.ui.summary_warning_panel.show(true);
    } else {
        ctx.ui.summary_warning_panel.show(false);
    }

    // Summary cards reflect the filtered view when available.
    let (added, removed, modified, moved) = if let Some(analysis) = analysis {
        (
            analysis.counts.added,
            analysis.counts.removed,
            analysis.counts.modified,
            analysis.counts.moved,
        )
    } else {
        (
            summary.counts.added,
            summary.counts.removed,
            summary.counts.modified,
            summary.counts.moved,
        )
    };
    ctx.ui.summary_added_value.set_label(&added.to_string());
    ctx.ui.summary_removed_value.set_label(&removed.to_string());
    ctx.ui
        .summary_modified_value
        .set_label(&modified.to_string());
    ctx.ui.summary_moved_value.set_label(&moved.to_string());

    // Categories table.
    if let Some(table) = ctx.state.summary_categories_table.as_mut() {
        let rows = analysis
            .map(|analysis| {
                analysis
                    .category_breakdown
                    .iter()
                    .map(|row| {
                        let name = match row.category {
                            ui_payload::OpCategory::Grid => "Grid",
                            ui_payload::OpCategory::PowerQuery => "Power Query",
                            ui_payload::OpCategory::Model => "Model",
                            ui_payload::OpCategory::Objects => "Objects",
                            ui_payload::OpCategory::Other => "Other",
                        };
                        vec![
                            name.to_string(),
                            row.total.to_string(),
                            row.severity.high.to_string(),
                            row.severity.medium.to_string(),
                            row.severity.low.to_string(),
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        update_virtual_table(table, rows);
    }

    // Top sheets table.
    if let Some(table) = ctx.state.summary_top_sheets_table.as_mut() {
        let rows = if let Some(analysis) = analysis {
            analysis
                .sheets
                .iter()
                .take(20)
                .map(|sheet| {
                    vec![
                        sheet.sheet_name.clone(),
                        sheet.op_count.to_string(),
                        sheet.severity.high.to_string(),
                        sheet.severity.medium.to_string(),
                        sheet.severity.low.to_string(),
                    ]
                })
                .collect::<Vec<_>>()
        } else {
            let mut sheets = summary.sheets.clone();
            sheets.sort_by(|a, b| {
                b.op_count.cmp(&a.op_count).then_with(|| {
                    a.sheet_name
                        .to_lowercase()
                        .cmp(&b.sheet_name.to_lowercase())
                })
            });
            sheets
                .into_iter()
                .take(20)
                .map(|sheet| {
                    vec![
                        sheet.sheet_name,
                        sheet.op_count.to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                    ]
                })
                .collect::<Vec<_>>()
        };
        update_virtual_table(table, rows);
    }

    ctx.ui.compare_right_panel.layout();
}

fn apply_display_analysis_in_ctx(ctx: &mut UiContext, summary: &DiffRunSummary) {
    let profile_name = profile_name_for_display_in_ctx(ctx);
    let analysis = ctx.state.display_analysis.clone();

    apply_summary_panel_in_ctx(ctx, summary, analysis.as_ref());

    ctx.ui.summary_text.set_value(&format_summary_scannable(
        summary,
        analysis.as_ref(),
        profile_name.as_deref(),
    ));

    if let Some(analysis) = analysis.as_ref() {
        ctx.state.sheets_all = analysis
            .sheets
            .iter()
            .map(|sheet| SheetRow {
                sheet_name: sheet.sheet_name.clone(),
                op_count: sheet.op_count,
                added: sheet.counts.added,
                removed: sheet.counts.removed,
                modified: sheet.counts.modified,
                moved: sheet.counts.moved,
            })
            .collect();
    } else {
        populate_sheet_list(ctx, summary);
        update_status_counts_in_ctx(ctx, Some(summary));
        return;
    }

    rebuild_sheet_list_in_ctx(ctx);
    update_status_counts_in_ctx(ctx, Some(summary));
}

fn stage_detail_payload(
    ctx: &mut UiContext,
    sheet_name: String,
    payload: ui_payload::DiffWithSheets,
) {
    ctx.state.pending_detail_sheet_name = Some(sheet_name.clone());
    ctx.state.pending_detail_payload = Some(Arc::new(payload));
    ctx.state.pending_detail_payload_gen = ctx.state.pending_detail_payload_gen.wrapping_add(1);
    ctx.state.pending_detail_json = None;
    ctx.state.pending_detail_json_gen = None;
    ctx.state.pending_detail_json_inflight_gen = None;

    if ctx.ui.result_tabs.selection() == RESULT_TAB_DETAILS {
        render_staged_detail_payload(ctx);
        return;
    }

    ctx.ui.detail_text.set_value(&format!(
        "Sheet payload ready for '{sheet_name}'.\nOpen Details for JSON, or Grid for a visual preview."
    ));
    update_status_in_ctx(ctx, "Sheet payload ready (deferred until tab open).");
}

fn render_staged_detail_payload(ctx: &mut UiContext) {
    let Some(payload) = ctx.state.pending_detail_payload.as_ref().cloned() else {
        return;
    };
    let sheet_name = ctx
        .state
        .pending_detail_sheet_name
        .clone()
        .unwrap_or_else(|| "sheet".to_string());

    let gen = ctx.state.pending_detail_payload_gen;
    let epoch = ctx.state.pending_detail_render_epoch;
    if ctx.state.pending_detail_json_gen == Some(gen) {
        if let Some(rendered) = ctx.state.pending_detail_json.as_ref() {
            if ctx.ui.detail_text.get_value() != rendered.as_str() {
                ctx.ui.detail_text.set_value(rendered);
            }
            update_status_in_ctx(ctx, &format!("Sheet payload loaded: {sheet_name}."));
        }
        return;
    }

    if ctx.state.pending_detail_json_inflight_gen == Some(gen) {
        if ctx.ui.detail_text.get_value().trim() != "Rendering JSON..." {
            ctx.ui.detail_text.set_value("Rendering JSON...");
        }
        update_status_in_ctx(ctx, &format!("Rendering JSON: {sheet_name}..."));
        return;
    }

    ctx.state.pending_detail_json_inflight_gen = Some(gen);
    ctx.ui.detail_text.set_value("Rendering JSON...");
    update_status_in_ctx(ctx, &format!("Rendering JSON: {sheet_name}..."));

    let sheet_name_render = sheet_name.clone();
    thread::spawn(move || {
        let text = serde_json::to_string_pretty(payload.as_ref())
            .unwrap_or_else(|_| "{\"error\":\"failed to serialize payload\"}".to_string());
        wxdragon::call_after(Box::new(move || {
            let _ = with_ui_context(|ctx| {
                if ctx.state.pending_detail_render_epoch != epoch {
                    return;
                }
                if ctx.state.pending_detail_payload_gen != gen {
                    return;
                }
                if ctx.state.pending_detail_sheet_name.as_deref()
                    != Some(sheet_name_render.as_str())
                {
                    return;
                }
                ctx.state.pending_detail_json = Some(text);
                ctx.state.pending_detail_json_gen = Some(gen);
                if ctx.state.pending_detail_json_inflight_gen == Some(gen) {
                    ctx.state.pending_detail_json_inflight_gen = None;
                }
                if let Some(rendered) = ctx.state.pending_detail_json.as_ref() {
                    ctx.ui.detail_text.set_value(rendered);
                }
                update_status_in_ctx(ctx, &format!("Sheet payload loaded: {sheet_name_render}."));
            });
        }));
    });
}

fn ensure_grid_preview_ready(ctx: &mut UiContext) {
    if ctx.state.webview_enabled {
        return;
    }
    if ctx.ui.grid_webview.is_some() || ctx.ui.grid_fallback.is_some() {
        return;
    }

    // The legacy UI runs without the full web UI, but we still want a visual grid preview.
    // Use the simplest possible HTML (no external assets) to maximize backend compatibility.
    let backend = if cfg!(target_os = "windows") {
        // Prefer Edge when present, otherwise fall back to the default backend (often IE).
        if WebView::is_backend_available(WebViewBackend::Edge) {
            WebViewBackend::Edge
        } else {
            WebViewBackend::Default
        }
    } else if WebView::is_backend_available(WebViewBackend::WebKit) {
        WebViewBackend::WebKit
    } else {
        WebViewBackend::Default
    };

    if !WebView::is_backend_available(backend) {
        let fallback = TextCtrl::builder(&ctx.ui.grid_panel)
            .with_style(
                wxdragon::widgets::textctrl::TextCtrlStyle::MultiLine
                    | wxdragon::widgets::textctrl::TextCtrlStyle::ReadOnly,
            )
            .build();
        theme::apply_content_text(&fallback, true);
        let sizer = BoxSizer::builder(Orientation::Vertical).build();
        sizer.add(&fallback, 1, SizerFlag::Expand | SizerFlag::All, 0);
        ctx.ui.grid_panel.set_sizer(sizer, true);
        ctx.ui.grid_panel.layout();
        fallback.set_value("Grid preview unavailable: WebView backend not available.");
        ctx.ui.grid_fallback = Some(fallback);
        return;
    }

    let webview = WebView::builder(&ctx.ui.grid_panel)
        .with_backend(backend)
        .build();
    let _ = webview.add_script_message_handler(WEBVIEW_HANDLER_NAME);
    let _ = webview.add_user_script(
        WEBVIEW_BRIDGE_SCRIPT,
        WebViewUserScriptInjectionTime::AtDocumentStart,
    );
    webview.on_script_message_received(move |event| {
        let Some(message) = event.get_string() else {
            return;
        };
        wxdragon::call_after(Box::new(move || handle_grid_webview_message(message)));
    });
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    sizer.add(&webview, 1, SizerFlag::Expand, 0);
    ctx.ui.grid_panel.set_sizer(sizer, true);
    ctx.ui.grid_panel.layout();

    ctx.ui.grid_webview = Some(webview);
    render_grid_placeholder(ctx, "Select a sheet to preview grid changes.");
}

fn render_grid_placeholder(ctx: &mut UiContext, message: &str) {
    ensure_grid_preview_ready(ctx);
    if let Some(text) = ctx.ui.grid_fallback {
        text.set_value(message);
        return;
    }
    let Some(webview) = ctx.ui.grid_webview else {
        return;
    };
    let html = grid_placeholder_html(message);
    webview.set_page(&html, "about:blank");
}

fn grid_placeholder_html(message: &str) -> String {
    let message = grid_preview::escape_html(message);
    format!(
        r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1" />
    <style>
      :root {{
        --bg: #0d1117;
        --panel: #161b22;
        --border: #30363d;
        --text: #e6edf3;
        --muted: #8b949e;
        --mono: ui-monospace, "SFMono-Regular", Menlo, Consolas, "Liberation Mono", monospace;
        --sans: system-ui, -apple-system, "Segoe UI", Arial, sans-serif;
      }}
      body {{
        margin: 0;
        font-family: var(--sans);
        background: var(--bg);
        color: var(--text);
      }}
      .wrap {{
        padding: 14px;
      }}
      .card {{
        border: 1px solid var(--border);
        background: var(--panel);
        border-radius: 12px;
        padding: 14px 16px;
      }}
      .msg {{
        font-size: 13px;
        color: var(--muted);
      }}
      .hint {{
        margin-top: 10px;
        font-family: var(--mono);
        font-size: 12px;
        color: var(--muted);
      }}
    </style>
  </head>
  <body>
    <div class="wrap">
      <div class="card">
        <div class="msg">{message}</div>
        <div class="hint">Tip: select a sheet in the sheet list (View -&gt; Show Sheets if hidden).</div>
      </div>
    </div>
  </body>
</html>"#
    )
}

fn render_grid_for_current_selection(ctx: &mut UiContext) {
    if ctx.state.webview_enabled {
        return;
    }
    let sheet_name = ctx
        .ui
        .sheets_view
        .and_then(|view| view.get_selected_row())
        .and_then(|row| ctx.state.sheet_names.get(row).cloned());
    let Some(sheet_name) = sheet_name else {
        render_grid_placeholder(ctx, "Select a sheet to preview grid changes.");
        return;
    };

    let html: Option<String> = {
        if let (Some(payload), Some(pending_name)) = (
            ctx.state.pending_detail_payload.as_ref(),
            ctx.state.pending_detail_sheet_name.as_deref(),
        ) {
            if pending_name.eq_ignore_ascii_case(&sheet_name) {
                Some(grid_preview::build_sheet_grid_preview_html(
                    &sheet_name,
                    payload,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
    .or_else(|| {
        ctx.state
            .current_payload
            .as_ref()
            .map(|payload| grid_preview::build_sheet_grid_preview_html(&sheet_name, payload))
    });

    let Some(html) = html else {
        render_grid_placeholder(ctx, "Grid preview unavailable: no payload loaded.");
        return;
    };

    render_grid_html(ctx, &html);
}

fn render_grid_html(ctx: &mut UiContext, html: &str) {
    ensure_grid_preview_ready(ctx);
    if ctx.ui.grid_fallback.is_some() {
        // If we're running without a webview backend, keep the fallback message.
        return;
    }
    let Some(webview) = ctx.ui.grid_webview else {
        return;
    };
    webview.set_page(html, "about:blank");
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridWebviewEvent {
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridCellClickParams {
    sheet_name: String,
    row: u32,
    col: u32,
}

fn handle_grid_webview_message(message: String) {
    let event: Result<GridWebviewEvent, _> = serde_json::from_str(&message);
    let event = match event {
        Ok(value) => value,
        Err(err) => {
            debug!("Grid webview message parse error: {err}");
            return;
        }
    };

    match event.method.as_str() {
        "gridCellClick" => {
            let params: Result<GridCellClickParams, _> = event
                .params
                .ok_or_else(|| "Missing gridCellClick params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(value) => value,
                Err(err) => {
                    update_status(&format!("Explain failed: {err}"));
                    return;
                }
            };
            handle_grid_cell_click(params);
        }
        _ => {}
    }
}

fn handle_grid_cell_click(params: GridCellClickParams) {
    let addr = CellAddress::from_indices(params.row, params.col);

    let mut request: Option<(DesktopBackend, String, String, u32, u32)> = None;
    let _ = with_ui_context(|ctx| {
        if ctx.state.active_run.is_some() {
            update_status_in_ctx(ctx, "Explain unavailable while a diff is running.");
            return;
        }

        // Payload mode: explain synchronously from the in-memory report.
        if let Some(payload) = ctx.state.current_payload.as_ref() {
            let text = explain::build_cell_explain_text(&payload.report, &params.sheet_name, addr);
            ctx.ui.explain_text.set_value(&text);
            update_status_in_ctx(ctx, "Explanation updated.");
            return;
        }

        let Some(diff_id) = ctx.state.current_diff_id.clone() else {
            update_status_in_ctx(ctx, "Run a diff before using Explain.");
            return;
        };

        ctx.ui.explain_text.set_value("Loading explanation...");
        update_status_in_ctx(ctx, "Loading explanation...");
        request = Some((
            ctx.state.backend.clone(),
            diff_id,
            params.sheet_name.clone(),
            params.row,
            params.col,
        ));
    });

    let Some((backend, diff_id, sheet_name, row, col)) = request else {
        return;
    };

    thread::spawn(move || {
        let range_report = backend.runner.load_ops_in_range(OpsRangeRequest {
            diff_id: diff_id.clone(),
            sheet_name: sheet_name.clone(),
            range: RangeBounds {
                row_start: Some(row),
                row_end: Some(row),
                col_start: Some(col),
                col_end: Some(col),
            },
        });

        let global_kinds = [
            "QueryAdded",
            "QueryRemoved",
            "QueryRenamed",
            "QueryDefinitionChanged",
            "QueryMetadataChanged",
            "TableAdded",
            "TableRemoved",
            "ModelColumnAdded",
            "ModelColumnRemoved",
            "ModelColumnTypeChanged",
            "ModelColumnPropertyChanged",
            "CalculatedColumnDefinitionChanged",
            "RelationshipAdded",
            "RelationshipRemoved",
            "RelationshipPropertyChanged",
            "MeasureAdded",
            "MeasureRemoved",
            "MeasureDefinitionChanged",
        ];
        let global_ops = backend.load_ops_by_kinds(&diff_id, &global_kinds);

        let explain_text = match range_report {
            Ok(report) => {
                let mut ops = report.ops.clone();
                if let Ok(mut extra) = global_ops {
                    ops.append(&mut extra);
                }
                let mut combined = DiffReport::new(ops);
                combined.strings = report.strings;
                combined.complete = report.complete;
                combined.warnings = report.warnings;
                explain::build_cell_explain_text(
                    &combined,
                    &sheet_name,
                    CellAddress::from_indices(row, col),
                )
            }
            Err(err) => format!("Explain failed: {}", err.message),
        };

        wxdragon::call_after(Box::new(move || {
            let _ = with_ui_context(|ctx| {
                if ctx.state.current_diff_id.as_deref() != Some(diff_id.as_str()) {
                    return;
                }
                ctx.ui.explain_text.set_value(&explain_text);
                update_status_in_ctx(ctx, "Explanation updated.");
            });
        }));
    });
}

fn license_check_disabled() -> bool {
    if env_flag("EXCEL_DIFF_REQUIRE_LICENSE") == Some(true) {
        return false;
    }
    if env_flag("EXCEL_DIFF_SKIP_LICENSE") == Some(true) {
        return true;
    }
    cfg!(debug_assertions)
}

fn ensure_license_ready(action: &str) -> bool {
    if license_check_disabled() {
        static SKIP_NOTED: AtomicBool = AtomicBool::new(false);
        if !SKIP_NOTED.swap(true, Ordering::Relaxed) {
            let _ =
                with_ui_context(|ctx| update_status_in_ctx(ctx, "License check skipped (dev)."));
        }
        return true;
    }
    let result = LicenseClient::from_env().and_then(|client| client.ensure_valid_or_refresh());
    match result {
        Ok(status) => {
            let _ = with_ui_context(|ctx| {
                update_status_in_ctx(ctx, &format!("License status: {}", status.status));
            });
            true
        }
        Err(err) => {
            let message = format!(
                "{action} requires an active license.\n\nError: {err}\n\nUse Help → License to activate or update status."
            );
            let _ = with_ui_context(|ctx| {
                let dialog = MessageDialog::builder(&ctx.ui.frame, &message, "License required")
                    .with_style(MessageDialogStyle::IconWarning | MessageDialogStyle::OK)
                    .build();
                let _ = dialog.show_modal();
            });
            false
        }
    }
}

fn show_license_dialog() {
    let _ = with_ui_context(|ctx| {
        let actions = ["Activate license", "Check status", "Deactivate this device"];
        let dialog = SingleChoiceDialog::builder(
            &ctx.ui.frame,
            "Choose a license action:",
            "License",
            &actions,
        )
        .build();
        if dialog.show_modal() != ID_OK {
            return;
        }

        let selection = dialog.get_selection();
        drop(dialog);

        match selection {
            0 => {
                let input = TextEntryDialog::builder(
                    &ctx.ui.frame,
                    "Enter your license key:",
                    "Activate License",
                )
                .build();
                if input.show_modal() != ID_OK {
                    return;
                }
                let Some(key) = input.get_value() else {
                    return;
                };
                let client = match LicenseClient::from_env() {
                    Ok(client) => client,
                    Err(err) => {
                        update_status_in_ctx(ctx, &format!("License client error: {err}"));
                        return;
                    }
                };
                match client.activate(key.trim()) {
                    Ok(result) => {
                        update_status_in_ctx(ctx, "License activated.");
                        let message = format!(
                            "License activated.\n\nStatus: {}\nDevices: {}",
                            result.status.status, result.status.max_devices
                        );
                        let info = MessageDialog::builder(&ctx.ui.frame, &message, "License")
                            .with_style(
                                MessageDialogStyle::IconInformation | MessageDialogStyle::OK,
                            )
                            .build();
                        let _ = info.show_modal();
                    }
                    Err(err) => {
                        update_status_in_ctx(ctx, &format!("Activation failed: {err}"));
                        let info = MessageDialog::builder(
                            &ctx.ui.frame,
                            &format!("Activation failed:\n{err}"),
                            "License",
                        )
                        .with_style(MessageDialogStyle::IconError | MessageDialogStyle::OK)
                        .build();
                        let _ = info.show_modal();
                    }
                }
            }
            1 => {
                let client = match LicenseClient::from_env() {
                    Ok(client) => client,
                    Err(err) => {
                        update_status_in_ctx(ctx, &format!("License client error: {err}"));
                        return;
                    }
                };
                let status = match client.status_remote(None) {
                    Ok(status) => status,
                    Err(err) => {
                        let info = MessageDialog::builder(
                            &ctx.ui.frame,
                            &format!("Status failed:\n{err}"),
                            "License",
                        )
                        .with_style(MessageDialogStyle::IconError | MessageDialogStyle::OK)
                        .build();
                        let _ = info.show_modal();
                        return;
                    }
                };
                let message = format!(
                    "License: {}\nStatus: {}\nDevices: {} / {}",
                    status.license_key,
                    status.status,
                    status.activations.len(),
                    status.max_devices
                );
                let info = MessageDialog::builder(&ctx.ui.frame, &message, "License")
                    .with_style(MessageDialogStyle::IconInformation | MessageDialogStyle::OK)
                    .build();
                let _ = info.show_modal();
            }
            2 => {
                let client = match LicenseClient::from_env() {
                    Ok(client) => client,
                    Err(err) => {
                        update_status_in_ctx(ctx, &format!("License client error: {err}"));
                        return;
                    }
                };
                match client.deactivate(None) {
                    Ok(()) => {
                        update_status_in_ctx(ctx, "License deactivated for this device.");
                        let info = MessageDialog::builder(
                            &ctx.ui.frame,
                            "This device has been deactivated.",
                            "License",
                        )
                        .with_style(MessageDialogStyle::IconInformation | MessageDialogStyle::OK)
                        .build();
                        let _ = info.show_modal();
                    }
                    Err(err) => {
                        let info = MessageDialog::builder(
                            &ctx.ui.frame,
                            &format!("Deactivation failed:\n{err}"),
                            "License",
                        )
                        .with_style(MessageDialogStyle::IconError | MessageDialogStyle::OK)
                        .build();
                        let _ = info.show_modal();
                    }
                }
            }
            _ => {}
        }
    });
}

fn layout_debug_enabled() -> bool {
    std::env::var("EXCEL_DIFF_DEBUG_LAYOUT")
        .map(|value| value == "1")
        .unwrap_or(false)
}

fn log_layout_sizes(ctx: &UiContext) {
    let frame_size = ctx.ui.frame.get_size();
    let root_tabs_size = ctx.ui.root_tabs.get_size();
    let compare_size = ctx.ui.compare_container.get_size();
    let result_tabs_size = ctx.ui.result_tabs.get_size();
    let sheets_size = ctx.ui.sheets_list_panel.get_size();

    info!(
        "Layout sizes: frame={}x{}, root_tabs={}x{}, compare_container={}x{}, result_tabs={}x{}, sheets_list={}x{}",
        frame_size.width,
        frame_size.height,
        root_tabs_size.width,
        root_tabs_size.height,
        compare_size.width,
        compare_size.height,
        result_tabs_size.width,
        result_tabs_size.height,
        sheets_size.width,
        sheets_size.height
    );
}

const WEBVIEW_HANDLER_NAME: &str = "tabulensis";
const WEBVIEW_BRIDGE_SCRIPT: &str = r#"
(function () {
  window.__TABULENSIS_DESKTOP__ = true;
  window.__tabulensisPostMessage = function (message) {
    try {
      if (window.chrome && window.chrome.webview && typeof window.chrome.webview.postMessage === "function") {
        window.chrome.webview.postMessage(message);
        return true;
      }
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.tabulensis) {
        window.webkit.messageHandlers.tabulensis.postMessage(message);
        return true;
      }
      if (window.external && typeof window.external.invoke === "function") {
        window.external.invoke(message);
        return true;
      }
      if (window.wx && typeof window.wx.postMessage === "function") {
        window.wx.postMessage(message);
        return true;
      }
    } catch (err) {
      console.warn("Tabulensis bridge error:", err);
    }
    return false;
  };
})();
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcRequest {
    id: u64,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiffParams {
    old_path: String,
    new_path: String,
    #[serde(default)]
    options: Option<DiffOptions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SheetPayloadParams {
    diff_id: String,
    sheet_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiffIdParams {
    diff_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenPathParams {
    path: String,
    #[serde(default)]
    reveal: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchIdParams {
    batch_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchDiffParams {
    diff_id: String,
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildSearchIndexParams {
    path: String,
    side: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchWorkbookIndexParams {
    index_id: String,
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RangeParams {
    row_start: Option<u32>,
    row_end: Option<u32>,
    col_start: Option<u32>,
    col_end: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SheetMetaParams {
    diff_id: String,
    sheet_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpsRangeParams {
    diff_id: String,
    sheet_name: String,
    range: Option<RangeParams>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CellsRangeParams {
    diff_id: String,
    sheet_name: String,
    side: String,
    range: Option<RangeParams>,
}

fn resolve_web_index_path() -> Option<PathBuf> {
    if let Ok(root) = std::env::var("EXCEL_DIFF_WEB_ROOT") {
        let candidate = PathBuf::from(root).join("index.html");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join("web").join("index.html"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("web").join("index.html"));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("web").join("index.html"));
                if let Some(grand) = parent.parent() {
                    candidates.push(grand.join("web").join("index.html"));
                }
            }
        }
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/index.html"));

    candidates.into_iter().find(|path| path.exists())
}

fn open_path(path: &Path, reveal: bool) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        if reveal {
            Command::new("explorer")
                .arg("/select,")
                .arg(path)
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            Command::new("cmd")
                .args(["/C", "start", "", path.to_string_lossy().as_ref()])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if cfg!(target_os = "macos") {
        let mut cmd = Command::new("open");
        if reveal {
            cmd.arg("-R");
        }
        cmd.arg(path).spawn().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let target = if reveal {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn path_to_file_url(path: &Path) -> Option<String> {
    let abs = path.canonicalize().ok()?;
    let mut raw = abs.to_string_lossy().replace('\\', "/");
    if cfg!(target_os = "windows") && !raw.starts_with('/') {
        raw = format!("/{raw}");
    }
    Some(format!("file://{raw}"))
}

fn send_rpc_payload(webview: WebView, payload: serde_json::Value) {
    if !webview.is_valid() {
        return;
    }
    let Ok(payload_json) = serde_json::to_string(&payload) else {
        return;
    };
    let script = format!("window.__tabulensisReceive({payload_json});");
    let _ = webview.run_script(&script);
}

fn send_rpc_payload_async(webview: WebView, payload: serde_json::Value) {
    wxdragon::call_after(Box::new(move || send_rpc_payload(webview, payload)));
}

fn rpc_ok(id: u64, result: serde_json::Value) -> serde_json::Value {
    json!({ "id": id, "ok": true, "result": result })
}

fn rpc_err(id: u64, error: serde_json::Value) -> serde_json::Value {
    json!({ "id": id, "ok": false, "error": error })
}

fn rpc_notify(method: &str, params: serde_json::Value) -> serde_json::Value {
    json!({ "method": method, "params": params })
}

fn dialog_selected_path(dialog: &FileDialog) -> Option<String> {
    let path = dialog.get_path().filter(|value| !value.trim().is_empty());
    if path.is_some() {
        return path;
    }
    let paths = dialog.get_paths();
    if let Some(first) = paths.into_iter().find(|value| !value.trim().is_empty()) {
        return Some(first);
    }
    let dir = dialog.get_directory();
    let name = dialog.get_filename();
    if let (Some(dir), Some(name)) = (dir, name) {
        if !dir.trim().is_empty() && !name.trim().is_empty() {
            return Some(PathBuf::from(dir).join(name).to_string_lossy().to_string());
        }
    }
    None
}

fn setup_webview(ctx: &mut UiContext) -> bool {
    let backend = if cfg!(target_os = "windows") {
        if WebView::is_backend_available(WebViewBackend::Edge) {
            WebViewBackend::Edge
        } else {
            update_status_in_ctx(ctx, "WebView2 runtime not available. Using legacy UI.");
            return false;
        }
    } else if WebView::is_backend_available(WebViewBackend::WebKit) {
        WebViewBackend::WebKit
    } else if WebView::is_backend_available(WebViewBackend::Default) {
        WebViewBackend::Default
    } else {
        update_status_in_ctx(ctx, "WebView backend not available. Using legacy UI.");
        return false;
    };

    let Some(index_url) = resolve_web_index_path().and_then(|path| path_to_file_url(&path)) else {
        update_status_in_ctx(ctx, "Web UI not found (set EXCEL_DIFF_WEB_ROOT).");
        return false;
    };

    let webview = WebView::builder(&ctx.ui.main_panel)
        .with_backend(backend)
        .build();
    let _ = webview.add_script_message_handler(WEBVIEW_HANDLER_NAME);
    let _ = webview.add_user_script(
        WEBVIEW_BRIDGE_SCRIPT,
        WebViewUserScriptInjectionTime::AtDocumentStart,
    );

    webview.on_script_message_received(move |event| {
        let Some(message) = event.get_string() else {
            return;
        };
        wxdragon::call_after(Box::new(move || handle_webview_rpc(webview, message)));
    });

    webview.load_url(&index_url);

    // Remove legacy chrome when the web UI is active.
    // This avoids mismatched theming and legacy menu actions that don't target the web UI.
    unsafe {
        ffi::wxd_Frame_SetMenuBar(
            ctx.ui.frame.handle_ptr() as *mut ffi::wxd_Frame_t,
            std::ptr::null_mut(),
        );
    }
    ctx.ui.frame.set_existing_status_bar(None);

    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    sizer.add(&webview, 1, SizerFlag::Expand, 0);
    ctx.ui.main_panel.set_sizer(sizer, true);
    ctx.ui.root_tabs.hide();
    ctx.ui.main_panel.layout();
    ctx.ui.frame.layout();

    ctx.ui.webview = Some(webview);
    ctx.state.webview_enabled = true;
    true
}

fn send_progress_to_webview(webview: WebView, rx: ProgressRx, run_id: u64) {
    thread::spawn(move || {
        for event in rx.iter() {
            if run_id != 0 && event.run_id != run_id {
                continue;
            }
            let payload = rpc_notify(
                "status",
                json!({
                    "stage": event.stage,
                    "phase": event.phase,
                    "detail": event.detail,
                    "percent": event.percent,
                    "source": "desktop"
                }),
            );
            send_rpc_payload_async(webview, payload);
        }
    });
}

fn handle_webview_rpc(webview: WebView, message: String) {
    let request: Result<RpcRequest, _> = serde_json::from_str(&message);
    let request = match request {
        Ok(req) => req,
        Err(err) => {
            let payload = rpc_notify("error", json!({ "message": err.to_string() }));
            send_rpc_payload_async(webview, payload);
            return;
        }
    };

    match request.method.as_str() {
        "ready" => {
            let version =
                with_ui_context(|ctx| ctx.state.engine_version.clone()).unwrap_or_default();
            send_rpc_payload_async(webview, rpc_ok(request.id, json!(version)));
        }
        "getCapabilities" => {
            let caps = with_ui_context(|ctx| {
                ui_payload::HostCapabilities::new(ctx.state.engine_version.clone())
            })
            .unwrap_or_else(|| ui_payload::HostCapabilities::new(String::new()));
            send_rpc_payload_async(webview, rpc_ok(request.id, json!(caps)));
        }
        "openFileDialog" => {
            let path = with_ui_context(|ctx| {
                let dialog = FileDialog::builder(&ctx.ui.frame)
                    .with_message("Open file")
                    .with_wildcard("Excel/PBIX files (*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit)|*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit|All files (*.*)|*.*")
                    .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
                    .build();
                if dialog.show_modal() == ID_OK {
                    dialog_selected_path(&dialog)
                } else {
                    None
                }
            })
            .flatten();
            send_rpc_payload_async(webview, rpc_ok(request.id, json!(path)));
        }
        "openFolderDialog" => {
            let path = with_ui_context(|ctx| {
                let dialog = DirDialog::builder(&ctx.ui.frame, "Select folder", "")
                    .with_style(DirDialogStyle::MustExist.bits())
                    .build();
                if dialog.show_modal() == ID_OK {
                    dialog.get_path()
                } else {
                    None
                }
            })
            .flatten();
            send_rpc_payload_async(webview, rpc_ok(request.id, json!(path)));
        }
        "diff" => {
            if !ensure_license_ready("Run diffs") {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "License required." })),
                );
                return;
            }
            let params: Result<DiffParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing diff params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };

            let mut args = None;
            let _ = with_ui_context(|ctx| {
                if ctx.state.active_run.is_some() {
                    send_rpc_payload_async(
                        webview,
                        rpc_err(request.id, json!({ "message": "Diff already running." })),
                    );
                    return;
                }

                ctx.state.run_counter = ctx.state.run_counter.saturating_add(1);
                let run_id = ctx.state.run_counter;
                let cancel = Arc::new(AtomicBool::new(false));
                ctx.state.active_run = Some(ActiveRun {
                    run_id,
                    stage: ProgressStage::Read,
                    cancel: cancel.clone(),
                    cancel_requested: false,
                });

                let options = params.options.unwrap_or_default();
                let backend = ctx.state.backend.clone();
                args = Some((
                    backend,
                    run_id,
                    cancel,
                    params.old_path,
                    params.new_path,
                    options,
                ));
            });

            let Some((backend, run_id, cancel, old_path, new_path, options)) = args else {
                return;
            };

            let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
            send_progress_to_webview(webview, progress_rx, run_id);

            thread::spawn(move || {
                let result = backend.runner.diff(DiffRequest {
                    old_path,
                    new_path,
                    run_id,
                    options,
                    cancel,
                    progress: progress_tx,
                });
                wxdragon::call_after(Box::new(move || {
                    handle_webview_diff_result(webview, request.id, result)
                }));
            });
        }
        "cancel" => {
            cancel_current();
            send_rpc_payload_async(webview, rpc_ok(request.id, json!(true)));
        }
        "loadRecents" => {
            let result = with_ui_context(|ctx| ctx.state.backend.load_recents());
            match result {
                Some(Ok(recents)) => {
                    send_rpc_payload_async(webview, rpc_ok(request.id, json!(recents)))
                }
                Some(Err(err)) => send_rpc_payload_async(webview, rpc_err(request.id, json!(err))),
                None => send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                ),
            }
        }
        "saveRecent" => {
            let params: Result<RecentComparison, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing recent entry".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let entry = match params {
                Ok(entry) => entry,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let result = with_ui_context(|ctx| ctx.state.backend.save_recent(entry));
            match result {
                Some(Ok(recents)) => {
                    send_rpc_payload_async(webview, rpc_ok(request.id, json!(recents)))
                }
                Some(Err(err)) => send_rpc_payload_async(webview, rpc_err(request.id, json!(err))),
                None => send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                ),
            }
        }
        "loadDiffSummary" => {
            let params: Result<DiffIdParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing diff id".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            thread::spawn(move || {
                let payload = backend.load_diff_summary(&params.diff_id);
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(summary) => send_rpc_payload(webview, rpc_ok(request.id, json!(summary))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "loadSheetPayload" => {
            let params: Result<SheetPayloadParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing sheet payload params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };

            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
            send_progress_to_webview(webview, progress_rx, 0);
            thread::spawn(move || {
                let payload = backend.runner.load_sheet_payload(SheetPayloadRequest {
                    diff_id: params.diff_id,
                    sheet_name: params.sheet_name,
                    cancel: Arc::new(AtomicBool::new(false)),
                    progress: progress_tx,
                });
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(result) => send_rpc_payload(webview, rpc_ok(request.id, json!(result))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "loadSheetMeta" => {
            let params: Result<SheetMetaParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing sheet meta params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };

            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            thread::spawn(move || {
                let payload = backend.runner.load_sheet_meta(SheetMetaRequest {
                    diff_id: params.diff_id,
                    sheet_name: params.sheet_name,
                    cancel: Arc::new(AtomicBool::new(false)),
                });
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(result) => send_rpc_payload(webview, rpc_ok(request.id, json!(result))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "loadOpsInRange" => {
            let params: Result<OpsRangeParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing ops range params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };

            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            let range = params.range.unwrap_or(RangeParams {
                row_start: None,
                row_end: None,
                col_start: None,
                col_end: None,
            });
            thread::spawn(move || {
                let payload = backend.runner.load_ops_in_range(OpsRangeRequest {
                    diff_id: params.diff_id,
                    sheet_name: params.sheet_name,
                    range: RangeBounds {
                        row_start: range.row_start,
                        row_end: range.row_end,
                        col_start: range.col_start,
                        col_end: range.col_end,
                    },
                });
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(result) => send_rpc_payload(webview, rpc_ok(request.id, json!(result))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "loadCellsInRange" => {
            let params: Result<CellsRangeParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing cells range params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };

            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            let range = params.range.unwrap_or(RangeParams {
                row_start: None,
                row_end: None,
                col_start: None,
                col_end: None,
            });
            thread::spawn(move || {
                let payload = backend.runner.load_cells_in_range(CellsRangeRequest {
                    diff_id: params.diff_id,
                    sheet_name: params.sheet_name,
                    side: params.side,
                    range: RangeBounds {
                        row_start: range.row_start,
                        row_end: range.row_end,
                        col_start: range.col_start,
                        col_end: range.col_end,
                    },
                });
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(result) => send_rpc_payload(webview, rpc_ok(request.id, json!(result))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "exportAuditXlsx" => {
            let params: Result<DiffIdParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing diff id".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let selection = with_ui_context(|ctx| {
                let summary = ctx.state.backend.load_diff_summary(&params.diff_id).ok();
                let filename = summary
                    .as_ref()
                    .map(|summary| DesktopBackend::default_export_name(summary, "audit", "xlsx"))
                    .unwrap_or_else(|| "tabulensis-audit.xlsx".to_string());
                let dialog = FileDialog::builder(&ctx.ui.frame)
                    .with_message("Export audit XLSX")
                    .with_default_file(&filename)
                    .with_wildcard("Excel (*.xlsx)|*.xlsx|All files (*.*)|*.*")
                    .with_style(FileDialogStyle::Save | FileDialogStyle::OverwritePrompt)
                    .build();
                if dialog.show_modal() == ID_OK {
                    dialog.get_path()
                } else {
                    None
                }
            })
            .flatten();

            let Some(path) = selection else {
                send_rpc_payload_async(webview, rpc_ok(request.id, json!(null)));
                return;
            };

            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };

            thread::spawn(move || {
                let result = backend.export_audit_xlsx_to_path(&params.diff_id, Path::new(&path));
                wxdragon::call_after(Box::new(move || match result {
                    Ok(()) => send_rpc_payload(webview, rpc_ok(request.id, json!(path))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "openPath" => {
            let params: Result<OpenPathParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing open path params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let path = PathBuf::from(params.path);
            let reveal = params.reveal;
            let result = open_path(&path, reveal);
            match result {
                Ok(()) => send_rpc_payload_async(webview, rpc_ok(request.id, json!(true))),
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })))
                }
            }
        }
        "runBatchCompare" => {
            if !ensure_license_ready("Run batch diffs") {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "License required." })),
                );
                return;
            }
            let params: Result<BatchRequest, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing batch params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };

            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };

            let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
            send_progress_to_webview(webview, progress_rx, 0);

            thread::spawn(move || {
                let outcome = backend.run_batch_compare(params, progress_tx);
                wxdragon::call_after(Box::new(move || match outcome {
                    Ok(result) => send_rpc_payload(webview, rpc_ok(request.id, json!(result))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "loadBatchSummary" => {
            let params: Result<BatchIdParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing batch id".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            thread::spawn(move || {
                let payload = backend.load_batch_summary(&params.batch_id);
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(summary) => send_rpc_payload(webview, rpc_ok(request.id, json!(summary))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "searchDiffOps" => {
            let params: Result<SearchDiffParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing search params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let limit = params.limit.unwrap_or(100);
            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            thread::spawn(move || {
                let payload = backend.search_diff_ops(&params.diff_id, &params.query, limit);
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(results) => send_rpc_payload(webview, rpc_ok(request.id, json!(results))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "buildSearchIndex" => {
            let params: Result<BuildSearchIndexParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing index params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            thread::spawn(move || {
                let payload = backend.build_search_index(Path::new(&params.path), &params.side);
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(summary) => send_rpc_payload(webview, rpc_ok(request.id, json!(summary))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        "searchWorkbookIndex" => {
            let params: Result<SearchWorkbookIndexParams, _> = request
                .params
                .clone()
                .ok_or_else(|| "Missing index params".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()));
            let params = match params {
                Ok(params) => params,
                Err(err) => {
                    send_rpc_payload_async(webview, rpc_err(request.id, json!({ "message": err })));
                    return;
                }
            };
            let limit = params.limit.unwrap_or(100);
            let backend = with_ui_context(|ctx| ctx.state.backend.clone());
            let Some(backend) = backend else {
                send_rpc_payload_async(
                    webview,
                    rpc_err(request.id, json!({ "message": "Backend unavailable." })),
                );
                return;
            };
            thread::spawn(move || {
                let payload = backend.search_workbook_index(&params.index_id, &params.query, limit);
                wxdragon::call_after(Box::new(move || match payload {
                    Ok(results) => send_rpc_payload(webview, rpc_ok(request.id, json!(results))),
                    Err(err) => send_rpc_payload(webview, rpc_err(request.id, json!(err))),
                }));
            });
        }
        _ => {
            send_rpc_payload_async(
                webview,
                rpc_err(request.id, json!({ "message": "Unknown method." })),
            );
        }
    }
}

fn create_dataview(parent: &Panel, columns: &[(&str, i32)]) -> DataViewCtrl {
    let ctrl = DataViewCtrl::builder(parent)
        .with_style(DataViewStyle::RowLines | DataViewStyle::VerticalRules)
        .build();
    theme::apply_content_dataview(&ctrl);

    for (idx, (label, width)) in columns.iter().enumerate() {
        let _ = ctrl.append_text_column(
            label,
            idx,
            *width,
            DataViewAlign::Left,
            DataViewColumnFlags::Resizable,
        );
    }

    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    sizer.add(&ctrl, 1, SizerFlag::Expand | SizerFlag::All, 0);
    parent.set_sizer(sizer, true);
    ctrl
}

fn create_virtual_table(ctrl: &DataViewCtrl) -> VirtualTable {
    let rows: Rc<RefCell<Vec<Vec<String>>>> = Rc::new(RefCell::new(Vec::new()));
    let rows_ref = rows.clone();
    let model = CustomDataViewVirtualListModel::new(
        0,
        rows_ref,
        |data, row, col| {
            let rows = data.borrow();
            let value = rows
                .get(row)
                .and_then(|cols| cols.get(col))
                .cloned()
                .unwrap_or_default();
            Variant::from_string(&value)
        },
        None::<fn(&Rc<RefCell<Vec<Vec<String>>>>, usize, usize, &Variant) -> bool>,
        None::<fn(&Rc<RefCell<Vec<Vec<String>>>>, usize, usize) -> Option<DataViewItemAttr>>,
        None::<fn(&Rc<RefCell<Vec<Vec<String>>>>, usize, usize) -> bool>,
    );
    let _ = ctrl.associate_model(&model);
    VirtualTable { model, rows }
}

fn update_virtual_table(table: &mut VirtualTable, rows: Vec<Vec<String>>) {
    *table.rows.borrow_mut() = rows;
    let size = table.rows.borrow().len();
    table.model.reset(size);
}

fn progress_animation_enabled() -> bool {
    // Avoid non-deterministic visual diffs during headless capture.
    env_string("EXCEL_DIFF_DEV_SCENARIO").is_none()
        && env_string("EXCEL_DIFF_UI_READY_FILE").is_none()
}

fn stop_progress_animation() {
    PROGRESS_ANIM_GEN.fetch_add(1, Ordering::SeqCst);
}

fn start_progress_animation() {
    let gen = PROGRESS_ANIM_GEN
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1);

    if !progress_animation_enabled() {
        return;
    }

    thread::spawn(move || {
        let mut tick: u64 = 0;
        loop {
            if PROGRESS_ANIM_GEN.load(Ordering::Relaxed) != gen {
                break;
            }

            let tick_now = tick;
            wxdragon::call_after(Box::new(move || {
                if PROGRESS_ANIM_GEN.load(Ordering::Relaxed) != gen {
                    return;
                }
                let _ = with_ui_context(|ctx| {
                    let Some(active) = ctx.state.active_run.as_ref() else {
                        return;
                    };
                    let (min, max) = active.stage.gauge_bounds();
                    if max <= min {
                        ctx.ui.progress_gauge.set_value(min);
                        return;
                    }

                    let span = (max - min) as u64;
                    let cycle = (tick_now % (span.saturating_mul(2))) as i32;
                    let offset = if cycle <= span as i32 {
                        cycle
                    } else {
                        (span as i32).saturating_mul(2).saturating_sub(cycle)
                    };

                    let value = min.saturating_add(offset).min(max);
                    ctx.ui.progress_gauge.set_value(value);
                });
            }));

            tick = tick.wrapping_add(1);
            thread::sleep(Duration::from_millis(90));
        }
    });
}

fn format_progress_message(event: &ProgressEvent) -> String {
    let stage = ProgressStage::from_stage_name(&event.stage);
    let mut prefix = stage.label().to_string();

    let phase = event
        .phase
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(phase) = phase {
        prefix.push_str(" (");
        prefix.push_str(phase);
        prefix.push(')');
    }

    let detail = event.detail.trim();
    if detail.is_empty() {
        prefix
    } else {
        format!("{prefix}: {detail}")
    }
}

fn handle_progress_event(event: ProgressEvent) {
    let mut ready_reason: Option<&'static str> = None;
    let _ = with_ui_context(|ctx| {
        // When running under the UI capture harness, freeze once we've signaled readiness so the
        // screenshot is deterministic (important for "working" mid-run captures).
        if ctx.state.dev_ready_file.is_some() && ctx.state.dev_ready_fired {
            return;
        }

        if let Some(active) = ctx.state.active_run.as_mut() {
            if active.cancel_requested {
                // Keep cancel messaging stable; don't overwrite with late progress events.
                return;
            }

            // Ignore stale progress updates for completed/replaced runs.
            if event.run_id > 0 && event.run_id != active.run_id {
                return;
            }

            if event.run_id == active.run_id {
                active.stage = ProgressStage::from_stage_name(&event.stage);

                // In capture mode (no animation), set a stable non-zero indicator for "working".
                if !progress_animation_enabled() {
                    let (min, max) = active.stage.gauge_bounds();
                    let value = min.saturating_add(((max - min).max(1)) / 2);
                    ctx.ui.progress_gauge.set_value(value);
                }
            }
        } else if event.run_id > 0 {
            // If the run is already over, ignore any straggler events from that run.
            return;
        }

        let message = format_progress_message(&event);
        if ctx.ui.progress_text.get_label() != message {
            update_status_in_ctx(ctx, &message);
        }

        let wants_working_ready = ctx.state.dev_ready_file.is_some()
            && !ctx.state.dev_ready_fired
            && ctx
                .state
                .dev_scenario
                .as_ref()
                .and_then(|s| s.ready_on_stage.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_some_and(|target| target.eq_ignore_ascii_case(event.stage.trim()));

        if wants_working_ready {
            ready_reason = Some("working");
        }
    });

    if let Some(reason) = ready_reason {
        mark_ui_ready(reason);
    }
}

fn spawn_progress_forwarder(rx: ProgressRx) {
    thread::spawn(move || {
        for event in rx.iter() {
            wxdragon::call_after(Box::new(move || handle_progress_event(event)));
        }
    });
}

fn preset_from_choice(choice: &Choice) -> DiffPreset {
    let selection = choice.get_selection().unwrap_or(0);
    let selection = i32::try_from(selection).unwrap_or(0);
    preset_from_selection(selection)
}

fn diff_options_from_ui_in_ctx(ctx: &UiContext) -> DiffOptions {
    let preset = preset_from_choice(&ctx.ui.preset_choice);
    let trusted = ctx.ui.trusted_checkbox.is_checked();

    let profile = ctx
        .state
        .selected_profile_id
        .as_deref()
        .and_then(|id| profile_by_id_in_ctx(ctx, id))
        .unwrap_or_else(|| profiles::builtin_profiles()[0].clone());

    DiffOptions {
        preset: Some(preset),
        trusted: Some(trusted),
        limits: profile.limits.clone(),
        config_json: None,
        enable_m_semantic_diff: Some(profile.enable_m_semantic_diff),
        enable_formula_semantic_diff: Some(profile.enable_formula_semantic_diff),
        enable_dax_semantic_diff: Some(profile.enable_dax_semantic_diff),
        semantic_noise_policy: Some(profile.semantic_noise_policy),
    }
}

fn sheet_filter_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|token| token.trim().to_lowercase())
        .filter(|token| !token.is_empty())
        .collect()
}

fn sheet_matches_filter(sheet: &SheetRow, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }

    // Filter across the full row (name + counts), so users can search by either.
    let mut haystack = sheet.sheet_name.to_lowercase();
    haystack.push(' ');
    haystack.push_str(&sheet.op_count.to_string());
    haystack.push(' ');
    haystack.push_str(&sheet.added.to_string());
    haystack.push(' ');
    haystack.push_str(&sheet.removed.to_string());
    haystack.push(' ');
    haystack.push_str(&sheet.modified.to_string());
    haystack.push(' ');
    haystack.push_str(&sheet.moved.to_string());

    tokens.iter().all(|token| haystack.contains(token))
}

fn rebuild_sheet_list_in_ctx(ctx: &mut UiContext) {
    let selected_sheet = ctx
        .ui
        .sheets_view
        .and_then(|view| view.get_selected_row())
        .and_then(|row| ctx.state.sheet_names.get(row).cloned());

    let tokens = sheet_filter_tokens(&ctx.state.sheets_filter);

    let mut sheet_names = Vec::new();
    let mut rows = Vec::new();
    for sheet in ctx.state.sheets_all.iter() {
        if !sheet_matches_filter(sheet, &tokens) {
            continue;
        }
        sheet_names.push(sheet.sheet_name.clone());
        rows.push(vec![
            sheet.sheet_name.clone(),
            sheet.op_count.to_string(),
            sheet.added.to_string(),
            sheet.removed.to_string(),
            sheet.modified.to_string(),
            sheet.moved.to_string(),
        ]);
    }

    ctx.state.sheet_names = sheet_names;
    if let Some(table) = ctx.state.sheets_table.as_mut() {
        update_virtual_table(table, rows);
    }

    // If the selected sheet was filtered away, clear selection and reset the preview.
    if let Some(view) = ctx.ui.sheets_view {
        if let Some(selected) = selected_sheet {
            if let Some(idx) = ctx
                .state
                .sheet_names
                .iter()
                .position(|name| name == &selected)
            {
                let _ = view.select_row(idx);
            } else {
                view.unselect_all();
                render_grid_placeholder(ctx, "Select a sheet to preview grid changes.");
            }
        }
    }

    sync_sheets_panel_state_in_ctx(ctx);
}

fn set_sheets_filter_query_in_ctx(ctx: &mut UiContext, query: String) {
    let query = query.trim().to_string();
    if ctx.state.sheets_filter == query {
        return;
    }
    ctx.state.sheets_filter = query;
    rebuild_sheet_list_in_ctx(ctx);
}

fn populate_sheet_list(ctx: &mut UiContext, summary: &DiffRunSummary) {
    ctx.state.sheets_all = summary
        .sheets
        .iter()
        .map(|sheet| SheetRow {
            sheet_name: sheet.sheet_name.clone(),
            op_count: sheet.op_count,
            added: sheet.counts.added,
            removed: sheet.counts.removed,
            modified: sheet.counts.modified,
            moved: sheet.counts.moved,
        })
        .collect();
    ctx.state
        .sheets_all
        .sort_by_key(|sheet| (Reverse(sheet.op_count), sheet.sheet_name.to_lowercase()));
    rebuild_sheet_list_in_ctx(ctx);
}

fn populate_recents(ctx: &mut UiContext, recents: Vec<RecentComparison>) {
    let Some(table) = ctx.state.recents_table.as_mut() else {
        debug!("Recents view not initialized yet.");
        return;
    };
    let rows = recents
        .iter()
        .map(|entry| {
            vec![
                entry.old_name.clone(),
                entry.new_name.clone(),
                entry.last_run_iso.clone(),
                entry.mode.clone().unwrap_or_else(|| "".to_string()),
            ]
        })
        .collect::<Vec<_>>();

    ctx.state.recents = recents;
    update_virtual_table(table, rows);
}

fn handle_diff_result(result: Result<DiffOutcome, DiffErrorPayload>) {
    let mut ready_reason: Option<&'static str> = None;
    let mut large_analysis_request: Option<(DesktopBackend, String, ui_payload::NoiseFilters)> =
        None;
    let _ = with_ui_context(|ctx| {
        stop_progress_animation();
        ctx.state.active_run = None;

        match result {
            Ok(outcome) => {
                ctx.state.cancel_restore_snapshot = None;
                ctx.ui.progress_gauge.set_value(100);
                info!(
                    "Diff complete: diff_id={} mode={} summary={} payload={}",
                    outcome.diff_id,
                    outcome.mode.as_str(),
                    if outcome.summary.is_some() {
                        "yes"
                    } else {
                        "no"
                    },
                    if outcome.payload.is_some() {
                        "yes"
                    } else {
                        "no"
                    }
                );
                ctx.state.current_diff_id = Some(outcome.diff_id.clone());
                ctx.state.current_mode = Some(outcome.mode);
                ctx.state.current_payload = outcome.payload.map(Arc::new);
                ctx.state.current_summary = outcome.summary.clone();
                ctx.state.pending_detail_payload = None;
                ctx.state.pending_detail_sheet_name = None;
                ctx.state.pending_detail_payload_gen =
                    ctx.state.pending_detail_payload_gen.wrapping_add(1);
                ctx.state.pending_detail_render_epoch =
                    ctx.state.pending_detail_render_epoch.wrapping_add(1);
                ctx.state.pending_detail_json = None;
                ctx.state.pending_detail_json_gen = None;
                ctx.state.pending_detail_json_inflight_gen = None;
                render_grid_placeholder(ctx, "Select a sheet to preview grid changes.");

                if let Some(summary) = outcome.summary {
                    ctx.ui.detail_text.set_value("");
                    ctx.state.sheets_filter.clear();
                    ctx.ui.sheets_filter_ctrl.set_value("");
                    if let Some(view) = ctx.ui.sheets_view {
                        view.unselect_all();
                    }
                    let filters = noise_filters_from_ui(ctx);
                    ctx.state.display_analysis = ctx
                        .state
                        .current_payload
                        .as_ref()
                        .map(|payload| ui_payload::analyze_report(&payload.report, filters));
                    apply_display_analysis_in_ctx(ctx, &summary);
                    update_run_summary_header_in_ctx(ctx);

                    if summary.op_count == 0 {
                        render_grid_placeholder(ctx, "No differences detected.");
                    } else if ctx.state.sheet_names.is_empty() {
                        render_grid_placeholder(ctx, "No sheet-level changes were detected.");
                    }

                    // Large-mode runs don't keep the payload in memory; compute the filtered
                    // breakdown via the store aggregation path.
                    if ctx.state.current_payload.is_none() {
                        large_analysis_request =
                            Some((ctx.state.backend.clone(), outcome.diff_id.clone(), filters));
                    }

                    // Dev scenarios can ask to focus the Grid tab; once we have a summary, select
                    // the first sheet so the preview actually renders.
                    let wants_grid_focus = ctx
                        .state
                        .dev_scenario
                        .as_ref()
                        .and_then(|s| s.focus_panel.as_deref())
                        .map(|value| value.trim().eq_ignore_ascii_case("grid"))
                        .unwrap_or(false);
                    if wants_grid_focus {
                        ctx.ui.root_tabs.set_selection(0);
                        ctx.ui.result_tabs.set_selection(RESULT_TAB_GRID as usize);
                        if !ctx.state.sheet_names.is_empty() {
                            if let Some(view) = ctx.ui.sheets_view {
                                let _ = view.select_row(0);
                            }
                            render_grid_for_current_selection(ctx);
                        } else if summary.op_count == 0 {
                            render_grid_placeholder(ctx, "No differences detected.");
                        } else {
                            render_grid_placeholder(
                                ctx,
                                "No sheet-level grid changes were detected.",
                            );
                        }
                    }

                    let recent = RecentComparison {
                        old_path: summary.old_path.clone(),
                        new_path: summary.new_path.clone(),
                        old_name: base_name(&summary.old_path),
                        new_name: base_name(&summary.new_path),
                        last_run_iso: summary
                            .finished_at
                            .clone()
                            .unwrap_or_else(|| summary.started_at.clone()),
                        diff_id: Some(outcome.diff_id.clone()),
                        mode: Some(summary.mode.as_str().to_string()),
                    };

                    if let Ok(recents) = ctx.state.backend.save_recent(recent) {
                        populate_recents(ctx, recents);
                    }
                } else {
                    update_status_counts_in_ctx(ctx, None);
                }

                update_status_in_ctx(ctx, "Diff complete.");
                theme::set_status_tone(
                    &ctx.ui.progress_text,
                    &ctx.ui.status_pill,
                    &ctx.ui.progress_gauge,
                    theme::StatusTone::Ready,
                );
                if ctx.state.dev_ready_file.is_some() && !ctx.state.dev_ready_fired {
                    ready_reason = Some("diff_complete");
                }
            }
            Err(err) if err.code == "canceled" => {
                log::info!("Diff canceled.");
                ctx.ui.progress_gauge.set_value(0);

                if let Some(snapshot) = ctx.state.cancel_restore_snapshot.take() {
                    restore_cancel_snapshot_in_ctx(ctx, snapshot);
                } else {
                    clear_diff_results_in_ctx(ctx);
                }

                update_status_in_ctx(ctx, "Canceled.");
                theme::set_status_tone(
                    &ctx.ui.progress_text,
                    &ctx.ui.status_pill,
                    &ctx.ui.progress_gauge,
                    theme::StatusTone::Ready,
                );

                if ctx.state.dev_ready_file.is_some() && !ctx.state.dev_ready_fired {
                    ready_reason = Some("diff_canceled");
                }
            }
            Err(err) => {
                ctx.state.cancel_restore_snapshot = None;
                ctx.ui.progress_gauge.set_value(100);

                log::warn!("Diff failed: {}: {}", err.code, err.message);
                ctx.ui
                    .detail_text
                    .set_value(&format!("{}: {}", err.code, err.message));
                update_status_in_ctx(ctx, &format!("Diff failed: {}", err.message));
                render_grid_placeholder(ctx, "Run a diff to preview grid changes.");
                theme::set_status_tone(
                    &ctx.ui.progress_text,
                    &ctx.ui.status_pill,
                    &ctx.ui.progress_gauge,
                    theme::StatusTone::Error,
                );
                update_status_counts_in_ctx(ctx, None);
                if ctx.state.dev_ready_file.is_some() && !ctx.state.dev_ready_fired {
                    ready_reason = Some("diff_failed");
                }
            }
        }

        sync_compare_controls_in_ctx(ctx);
    });

    if let Some((backend, diff_id, filters)) = large_analysis_request {
        thread::spawn(move || {
            let analysis = backend.load_diff_analysis(&diff_id, filters);
            wxdragon::call_after(Box::new(move || {
                let _ = with_ui_context(|ctx| {
                    if ctx.state.current_diff_id.as_deref() != Some(diff_id.as_str()) {
                        return;
                    }
                    let Some(summary) = ctx.state.current_summary.clone() else {
                        return;
                    };
                    ctx.state.display_analysis = analysis.ok();
                    apply_display_analysis_in_ctx(ctx, &summary);
                });
            }));
        });
    }

    if let Some(reason) = ready_reason {
        // For capture/dev scenarios, write the ready file immediately when the UI reaches a stable
        // end-of-run state. The capture script can add a delay if it needs extra settling time.
        mark_ui_ready(reason);
    }
}

fn handle_webview_diff_result(
    webview: WebView,
    request_id: u64,
    result: Result<DiffOutcome, DiffErrorPayload>,
) {
    let payload = match result {
        Ok(outcome) => {
            let _ = with_ui_context(|ctx| {
                ctx.state.current_diff_id = Some(outcome.diff_id.clone());
                ctx.state.current_mode = Some(outcome.mode);
                ctx.state.current_summary = outcome.summary.clone();
                ctx.state.active_run = None;
            });
            rpc_ok(request_id, json!(outcome))
        }
        Err(err) => {
            let _ = with_ui_context(|ctx| {
                ctx.state.active_run = None;
            });
            rpc_err(request_id, json!(err))
        }
    };
    send_rpc_payload(webview, payload);
}

fn start_compare() {
    if !ensure_license_ready("Run diffs") {
        return;
    }
    let mut args = None;
    let _ = with_ui_context(|ctx| {
        let old_path = ctx.ui.old_picker.get_path();
        let new_path = ctx.ui.new_picker.get_path();

        debug!(
            "start_compare: old_path='{}' new_path='{}'",
            old_path, new_path
        );
        if old_path.trim().is_empty() || new_path.trim().is_empty() {
            log::warn!(
                "start_compare: missing old/new path (old_empty={}, new_empty={})",
                old_path.trim().is_empty(),
                new_path.trim().is_empty()
            );
            update_status_in_ctx(ctx, "Select both old and new files.");
            return;
        }

        if ctx.state.active_run.is_some() {
            update_status_in_ctx(ctx, "A diff is already running.");
            return;
        }

        // Keep the previous results intact so we can restore them on Cancel.
        ctx.state.cancel_restore_snapshot = Some(take_cancel_restore_snapshot_in_ctx(ctx));

        ctx.state.run_counter = ctx.state.run_counter.saturating_add(1);
        let run_id = ctx.state.run_counter;
        let cancel = Arc::new(AtomicBool::new(false));
        ctx.state.active_run = Some(ActiveRun {
            run_id,
            stage: ProgressStage::Read,
            cancel: cancel.clone(),
            cancel_requested: false,
        });

        // Scenario harness: allow deterministic "canceled" end-states without relying on races
        // between a fast diff completion and a scheduled cancel event.
        let cancel_immediately = ctx
            .state
            .dev_scenario
            .as_ref()
            .and_then(|s| s.cancel_after_ms)
            .unwrap_or(u64::MAX)
            == 0;
        if cancel_immediately {
            cancel.store(true, Ordering::Relaxed);
        }
        ctx.state.current_payload = None;
        ctx.state.current_summary = None;
        ctx.state.display_analysis = None;
        ctx.state.pending_detail_payload = None;
        ctx.state.pending_detail_sheet_name = None;
        ctx.state.pending_detail_payload_gen = ctx.state.pending_detail_payload_gen.wrapping_add(1);
        ctx.state.pending_detail_render_epoch =
            ctx.state.pending_detail_render_epoch.wrapping_add(1);
        ctx.state.pending_detail_json = None;
        ctx.state.pending_detail_json_gen = None;
        ctx.state.pending_detail_json_inflight_gen = None;
        ctx.state.sheets_all.clear();
        ctx.state.sheet_names.clear();
        ctx.state.sheets_filter.clear();
        ctx.ui.sheets_filter_ctrl.set_value("");
        if let Some(view) = ctx.ui.sheets_view {
            view.unselect_all();
        }
        if let Some(table) = ctx.state.sheets_table.as_mut() {
            update_virtual_table(table, Vec::new());
        }
        if let Some(table) = ctx.state.summary_categories_table.as_mut() {
            update_virtual_table(table, Vec::new());
        }
        if let Some(table) = ctx.state.summary_top_sheets_table.as_mut() {
            update_virtual_table(table, Vec::new());
        }
        ctx.ui.summary_warning_panel.show(false);
        ctx.ui.summary_added_value.set_label("0");
        ctx.ui.summary_removed_value.set_label("0");
        ctx.ui.summary_modified_value.set_label("0");
        ctx.ui.summary_moved_value.set_label("0");
        update_status_counts_in_ctx(ctx, None);

        sync_compare_controls_in_ctx(ctx);
        ctx.ui.progress_gauge.set_value(0);
        ctx.ui.summary_text.set_value("");
        ctx.ui.detail_text.set_value("");
        render_grid_placeholder(ctx, "Comparing...");
        update_status_in_ctx(ctx, "Starting diff...");
        theme::set_status_tone(
            &ctx.ui.progress_text,
            &ctx.ui.status_pill,
            &ctx.ui.progress_gauge,
            theme::StatusTone::Working,
        );
        start_progress_animation();

        let options = diff_options_from_ui_in_ctx(ctx);

        let backend = ctx.state.backend.clone();
        args = Some((backend, run_id, cancel, old_path, new_path, options));
    });

    let Some((backend, run_id, cancel, old_path, new_path, options)) = args else {
        return;
    };

    let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
    spawn_progress_forwarder(progress_rx);

    thread::spawn(move || {
        let result = backend.runner.diff(DiffRequest {
            old_path,
            new_path,
            run_id,
            options,
            cancel,
            progress: progress_tx,
        });
        wxdragon::call_after(Box::new(move || handle_diff_result(result)));
    });
}

fn handle_sheet_selection(row: usize) {
    let mut request = None;
    let _ = with_ui_context(|ctx| {
        let sheet_name = ctx.state.sheet_names.get(row).cloned();
        let mode = ctx.state.current_mode;

        match mode {
            Some(DiffMode::Payload) => {
                if let (Some(summary), Some(sheet_name)) = (&ctx.state.current_summary, sheet_name)
                {
                    if let Some(sheet) = summary
                        .sheets
                        .iter()
                        .find(|sheet| sheet.sheet_name == sheet_name)
                    {
                        let text = format!(
                            "Sheet: {}\nOps: {}\nAdded: {}\nRemoved: {}\nModified: {}\nMoved: {}",
                            sheet.sheet_name,
                            sheet.op_count,
                            sheet.counts.added,
                            sheet.counts.removed,
                            sheet.counts.modified,
                            sheet.counts.moved,
                        );
                        ctx.ui.detail_text.set_value(&text);
                        ctx.state.pending_detail_payload = None;
                        ctx.state.pending_detail_sheet_name = None;
                        ctx.state.pending_detail_payload_gen =
                            ctx.state.pending_detail_payload_gen.wrapping_add(1);
                        ctx.state.pending_detail_render_epoch =
                            ctx.state.pending_detail_render_epoch.wrapping_add(1);
                        ctx.state.pending_detail_json = None;
                        ctx.state.pending_detail_json_gen = None;
                        ctx.state.pending_detail_json_inflight_gen = None;
                        let html = ctx.state.current_payload.as_ref().map(|payload| {
                            grid_preview::build_sheet_grid_preview_html(&sheet_name, payload)
                        });
                        if let Some(html) = html {
                            render_grid_html(ctx, &html);
                        } else {
                            render_grid_placeholder(
                                ctx,
                                "Grid preview unavailable: payload not loaded.",
                            );
                        }
                    }
                }
            }
            Some(DiffMode::Large) => {
                let Some(diff_id) = ctx.state.current_diff_id.clone() else {
                    return;
                };
                let Some(sheet_name) = sheet_name else {
                    return;
                };

                let backend = ctx.state.backend.clone();
                request = Some((backend, diff_id, sheet_name));
                update_status_in_ctx(ctx, "Loading sheet payload...");
                ctx.state.pending_detail_payload = None;
                ctx.state.pending_detail_sheet_name = None;
                ctx.state.pending_detail_payload_gen =
                    ctx.state.pending_detail_payload_gen.wrapping_add(1);
                ctx.state.pending_detail_render_epoch =
                    ctx.state.pending_detail_render_epoch.wrapping_add(1);
                ctx.state.pending_detail_json = None;
                ctx.state.pending_detail_json_gen = None;
                ctx.state.pending_detail_json_inflight_gen = None;
                render_grid_placeholder(ctx, "Loading grid preview...");
            }
            _ => {}
        }
    });

    let Some((backend, diff_id, sheet_name)) = request else {
        return;
    };

    let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
    spawn_progress_forwarder(progress_rx);

    thread::spawn(move || {
        let requested_sheet = sheet_name.clone();
        let payload = backend.runner.load_sheet_payload(SheetPayloadRequest {
            diff_id,
            sheet_name,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: progress_tx,
        });

        wxdragon::call_after(Box::new(move || match payload {
            Ok(payload) => {
                let _ = with_ui_context(|ctx| {
                    stage_detail_payload(ctx, requested_sheet.clone(), payload);
                    let html = ctx.state.pending_detail_payload.as_ref().map(|payload| {
                        grid_preview::build_sheet_grid_preview_html(&requested_sheet, payload)
                    });
                    if let Some(html) = html {
                        render_grid_html(ctx, &html);
                    }
                });
            }
            Err(err) => {
                update_status(&format!("Load failed: {}", err.message));
            }
        }));
    });
}

fn load_diff_summary_into_ui(diff_id: String) {
    let mut request: Option<(DesktopBackend, ui_payload::NoiseFilters)> = None;
    let _ = with_ui_context(|ctx| {
        request = Some((ctx.state.backend.clone(), noise_filters_from_ui(ctx)));
        ctx.ui.summary_text.set_value("Loading summary...");
        ctx.ui.detail_text.set_value("");
        ctx.ui.explain_text.set_value(GUIDED_EMPTY_EXPLAIN);
        render_grid_placeholder(ctx, "Loading summary...");
        update_status_in_ctx(ctx, "Loading summary...");
    });

    let Some((backend, filters)) = request else {
        return;
    };

    thread::spawn(move || {
        let summary = backend.load_diff_summary(&diff_id);
        let analysis = backend.load_diff_analysis(&diff_id, filters);
        wxdragon::call_after(Box::new(move || match summary {
            Ok(summary) => {
                let analysis = analysis.ok();
                let _ = with_ui_context(|ctx| {
                    ctx.state.current_diff_id = Some(diff_id.clone());
                    ctx.state.current_mode = Some(summary.mode);
                    ctx.state.current_summary = Some(summary.clone());
                    ctx.state.display_analysis = analysis;
                    ctx.state.current_payload = None;
                    ctx.state.pending_detail_payload = None;
                    ctx.state.pending_detail_sheet_name = None;
                    ctx.state.pending_detail_payload_gen =
                        ctx.state.pending_detail_payload_gen.wrapping_add(1);
                    ctx.state.pending_detail_render_epoch =
                        ctx.state.pending_detail_render_epoch.wrapping_add(1);
                    ctx.state.pending_detail_json = None;
                    ctx.state.pending_detail_json_gen = None;
                    ctx.state.pending_detail_json_inflight_gen = None;
                    ctx.state.sheets_filter.clear();
                    ctx.ui.sheets_filter_ctrl.set_value("");
                    if let Some(view) = ctx.ui.sheets_view {
                        view.unselect_all();
                    }

                    apply_display_analysis_in_ctx(ctx, &summary);
                    ctx.ui.detail_text.set_value("");
                    ctx.ui.explain_text.set_value(GUIDED_EMPTY_EXPLAIN);
                    render_grid_placeholder(ctx, "Select a sheet to preview grid changes.");
                    update_run_summary_header_in_ctx(ctx);
                    if summary.op_count == 0 {
                        render_grid_placeholder(ctx, "No differences detected.");
                    } else if ctx.state.sheet_names.is_empty() {
                        render_grid_placeholder(ctx, "No sheet-level changes were detected.");
                    }
                    ctx.ui.root_tabs.set_selection(0);
                    update_status_in_ctx(ctx, "Summary loaded.");
                });
            }
            Err(err) => update_status(&format!("Load summary failed: {}", err.message)),
        }));
    });
}

fn run_batch() {
    if !ensure_license_ready("Run batch diffs") {
        return;
    }
    let mut request = None;
    let _ = with_ui_context(|ctx| {
        let old_root = ctx.ui.batch_old_dir.get_path();
        let new_root = ctx.ui.batch_new_dir.get_path();

        if old_root.trim().is_empty() || new_root.trim().is_empty() {
            update_status_in_ctx(ctx, "Select both batch folders.");
            return;
        }

        let batch_request = BatchRequest {
            old_root,
            new_root,
            strategy: "path".to_string(),
            include_globs: parse_globs(&ctx.ui.include_glob_text.get_value()),
            exclude_globs: parse_globs(&ctx.ui.exclude_glob_text.get_value()),
            trusted: ctx.ui.trusted_checkbox.is_checked(),
        };

        let backend = ctx.state.backend.clone();
        request = Some((backend, batch_request));
        ctx.ui.run_batch_btn.enable(false);
        update_status_in_ctx(ctx, "Running batch compare...");
    });

    let Some((backend, request)) = request else {
        return;
    };

    let (progress_tx, progress_rx) = DesktopBackend::new_progress_channel();
    spawn_progress_forwarder(progress_rx);

    thread::spawn(move || {
        let outcome = backend.run_batch_compare(request, progress_tx);
        wxdragon::call_after(Box::new(move || handle_batch_result(outcome)));
    });
}

fn handle_batch_result(result: Result<BatchOutcome, DiffErrorPayload>) {
    let _ = with_ui_context(|ctx| {
        ctx.ui.run_batch_btn.enable(true);
        let Some(table) = ctx.state.batch_table.as_mut() else {
            debug!("Batch view not initialized yet.");
            return;
        };

        match result {
            Ok(outcome) => {
                ctx.state.batch_outcome = Some(outcome.clone());

                let rows = outcome
                    .items
                    .iter()
                    .map(|item| {
                        vec![
                            item.old_path.clone().unwrap_or_else(|| "".to_string()),
                            item.new_path.clone().unwrap_or_else(|| "".to_string()),
                            item.status.clone(),
                            item.op_count
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "".to_string()),
                            item.warnings_count
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "".to_string()),
                            item.error.clone().unwrap_or_else(|| "".to_string()),
                        ]
                    })
                    .collect::<Vec<_>>();

                update_virtual_table(table, rows);
                update_status_in_ctx(ctx, "Batch compare complete.");
            }
            Err(err) => update_status_in_ctx(ctx, &format!("Batch failed: {}", err.message)),
        }
    });
}

fn handle_search() {
    let mut request = None;
    let _ = with_ui_context(|ctx| {
        let query = ctx.ui.search_ctrl.get_value();
        if query.trim().is_empty() {
            update_status_in_ctx(ctx, "Enter a search query.");
            return;
        }

        let scope = ctx.ui.search_scope_choice.get_selection().unwrap_or(0);
        let backend = ctx.state.backend.clone();

        match scope {
            0 => {
                let Some(diff_id) = ctx.state.current_diff_id.clone() else {
                    update_status_in_ctx(ctx, "Run a diff before searching changes.");
                    return;
                };
                request = Some(SearchRequest::DiffOps {
                    backend,
                    diff_id,
                    query,
                });
            }
            1 => {
                let Some(index) = ctx.state.search_old_index.clone() else {
                    update_status_in_ctx(ctx, "Build the workbook index first.");
                    return;
                };
                request = Some(SearchRequest::WorkbookIndex {
                    backend,
                    index_id: index.index_id.clone(),
                    query,
                });
            }
            2 => {
                let Some(index) = ctx.state.search_new_index.clone() else {
                    update_status_in_ctx(ctx, "Build the workbook index first.");
                    return;
                };
                request = Some(SearchRequest::WorkbookIndex {
                    backend,
                    index_id: index.index_id.clone(),
                    query,
                });
            }
            _ => {}
        }
    });

    let Some(request) = request else {
        return;
    };

    thread::spawn(move || match request {
        SearchRequest::DiffOps {
            backend,
            diff_id,
            query,
        } => {
            let result = backend.search_diff_ops(&diff_id, &query, 100);
            wxdragon::call_after(Box::new(move || match result {
                Ok(results) => apply_search_results(results),
                Err(err) => update_status(&format!("Search failed: {}", err.message)),
            }));
        }
        SearchRequest::WorkbookIndex {
            backend,
            index_id,
            query,
        } => {
            let result = backend.search_workbook_index(&index_id, &query, 100);
            wxdragon::call_after(Box::new(move || match result {
                Ok(results) => apply_index_results(results),
                Err(err) => update_status(&format!("Search failed: {}", err.message)),
            }));
        }
    });
}

enum SearchRequest {
    DiffOps {
        backend: DesktopBackend,
        diff_id: String,
        query: String,
    },
    WorkbookIndex {
        backend: DesktopBackend,
        index_id: String,
        query: String,
    },
}

fn apply_search_results(results: Vec<SearchResult>) {
    let _ = with_ui_context(|ctx| {
        let Some(table) = ctx.state.search_table.as_mut() else {
            debug!("Search view not initialized yet.");
            return;
        };
        let rows = results
            .iter()
            .map(|result| {
                vec![
                    result.kind.clone(),
                    result.sheet.clone().unwrap_or_else(|| "".to_string()),
                    result.address.clone().unwrap_or_else(|| "".to_string()),
                    result.label.clone(),
                    result.detail.clone().unwrap_or_else(|| "".to_string()),
                ]
            })
            .collect::<Vec<_>>();

        update_virtual_table(table, rows);
        update_status_in_ctx(ctx, &format!("Search returned {} results.", results.len()));
    });
}

fn apply_index_results(results: Vec<SearchIndexResult>) {
    let _ = with_ui_context(|ctx| {
        let Some(table) = ctx.state.search_table.as_mut() else {
            debug!("Search view not initialized yet.");
            return;
        };
        let rows = results
            .iter()
            .map(|result| {
                vec![
                    result.kind.clone(),
                    result.sheet.clone(),
                    result.address.clone(),
                    "Workbook".to_string(),
                    result.text.clone(),
                ]
            })
            .collect::<Vec<_>>();

        update_virtual_table(table, rows);
        update_status_in_ctx(ctx, &format!("Search returned {} results.", results.len()));
    });
}

fn build_index(side: &str) {
    let mut request = None;
    let _ = with_ui_context(|ctx| {
        let Some(summary) = ctx.state.current_summary.clone() else {
            update_status_in_ctx(ctx, "Run a diff before building indexes.");
            return;
        };

        let path = if side == "old" {
            summary.old_path
        } else {
            summary.new_path
        };

        let backend = ctx.state.backend.clone();
        request = Some((backend, path, side.to_string()));
        update_status_in_ctx(ctx, "Building search index...");
    });

    let Some((backend, path, side)) = request else {
        return;
    };

    thread::spawn(move || {
        let result = backend.build_search_index(Path::new(&path), &side);
        wxdragon::call_after(Box::new(move || match result {
            Ok(summary) => {
                let _ = with_ui_context(|ctx| {
                    if side == "old" {
                        ctx.state.search_old_index = Some(summary);
                    } else {
                        ctx.state.search_new_index = Some(summary);
                    }
                    update_status_in_ctx(ctx, "Search index ready.");
                });
            }
            Err(err) => update_status(&format!("Index failed: {}", err.message)),
        }));
    });
}

fn cancel_current() {
    let _ = with_ui_context(|ctx| {
        if let Some(active) = ctx.state.active_run.as_mut() {
            if active.cancel_requested {
                return;
            }
            active.cancel_requested = true;
            active.cancel.store(true, Ordering::Relaxed);
            update_status_in_ctx(ctx, "Cancel requested (finishing current step)...");
            theme::set_status_tone(
                &ctx.ui.progress_text,
                &ctx.ui.status_pill,
                &ctx.ui.progress_gauge,
                theme::StatusTone::Working,
            );
            sync_compare_controls_in_ctx(ctx);
            update_run_summary_header_in_ctx(ctx);
            sync_sheets_panel_state_in_ctx(ctx);
        }
    });
}

fn open_recent() {
    let mut request = None;
    let _ = with_ui_context(|ctx| {
        let Some(view) = ctx.ui.recents_view else {
            update_status_in_ctx(ctx, "Recents list not ready.");
            return;
        };
        let Some(selected) = view.get_selected_row() else {
            update_status_in_ctx(ctx, "Select a recent comparison.");
            return;
        };

        let entry = ctx.state.recents.get(selected).cloned();
        let Some(entry) = entry else {
            return;
        };

        ctx.ui.old_picker.set_path(&entry.old_path);
        ctx.ui.new_picker.set_path(&entry.new_path);
        sync_compare_controls_in_ctx(ctx);
        request = entry.diff_id.clone();

        if request.is_none() {
            ctx.ui.root_tabs.set_selection(0);
        }
    });

    let Some(diff_id) = request else {
        return;
    };

    load_diff_summary_into_ui(diff_id);
}

fn noise_filters_from_ui(ctx: &UiContext) -> ui_payload::NoiseFilters {
    ui_payload::NoiseFilters {
        hide_m_formatting_only: ctx.ui.hide_m_formatting_checkbox.is_checked(),
        hide_dax_formatting_only: ctx.ui.hide_dax_formatting_checkbox.is_checked(),
        hide_formula_formatting_only: ctx.ui.hide_formula_formatting_checkbox.is_checked(),
        collapse_moves: ctx.ui.collapse_moves_checkbox.is_checked(),
    }
}

fn open_pair_dialog() {
    let _ = with_ui_context(|ctx| {
        let dialog = FileDialog::builder(&ctx.ui.frame)
            .with_message("Open old file")
            .with_wildcard("Excel/PBIX files (*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit)|*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit|All files (*.*)|*.*")
            .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
            .build();
        if dialog.show_modal() != ID_OK {
            return;
        }
        let Some(old_path) = dialog.get_path() else {
            return;
        };

        let dialog = FileDialog::builder(&ctx.ui.frame)
            .with_message("Open new file")
            .with_wildcard("Excel/PBIX files (*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit)|*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit|All files (*.*)|*.*")
            .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
            .build();
        if dialog.show_modal() != ID_OK {
            return;
        }
        let Some(new_path) = dialog.get_path() else {
            return;
        };

        ctx.ui.old_picker.set_path(&old_path);
        ctx.ui.new_picker.set_path(&new_path);
        sync_compare_controls_in_ctx(ctx);
        ctx.ui.root_tabs.set_selection(0);
    });
}

fn swap_old_new_paths() {
    let _ = with_ui_context(|ctx| {
        if ctx.state.active_run.is_some() {
            return;
        }

        let old_path = ctx.ui.old_picker.get_path();
        let new_path = ctx.ui.new_picker.get_path();
        if old_path.trim().is_empty() && new_path.trim().is_empty() {
            return;
        }

        ctx.ui.old_picker.set_path(&new_path);
        ctx.ui.new_picker.set_path(&old_path);
        sync_compare_controls_in_ctx(ctx);
        update_status_in_ctx(ctx, "Swapped Old and New.");
    });
}

fn focus_search() {
    let _ = with_ui_context(|ctx| {
        ctx.ui.root_tabs.set_selection(3);
        ctx.ui.search_ctrl.set_focus();
    });
}

fn copy_current_text() {
    let _ = with_ui_context(|ctx| {
        let selected_tab = ctx.ui.result_tabs.selection();
        let (selected, full) = if selected_tab == 1 {
            (
                ctx.ui.detail_text.get_string_selection(),
                ctx.ui.detail_text.get_value(),
            )
        } else {
            (
                ctx.ui.summary_text.get_string_selection(),
                ctx.ui.summary_text.get_value(),
            )
        };
        let text = if selected.trim().is_empty() {
            full
        } else {
            selected
        };
        if text.trim().is_empty() {
            update_status_in_ctx(ctx, "Nothing to copy.");
            return;
        }
        let clipboard = Clipboard::get();
        if clipboard.set_text(&text) {
            update_status_in_ctx(ctx, "Copied to clipboard.");
        } else {
            update_status_in_ctx(ctx, "Clipboard unavailable.");
        }
    });
}

fn select_next_diff() {
    let _ = with_ui_context(|ctx| {
        let Some(view) = ctx.ui.sheets_view else {
            return;
        };
        let row_count = ctx.state.sheet_names.len();
        if row_count == 0 {
            return;
        }
        let current = view.get_selected_row().unwrap_or(0);
        let next = (current + 1).min(row_count - 1);
        let _ = view.select_row(next);
    });
}

fn select_prev_diff() {
    let _ = with_ui_context(|ctx| {
        let Some(view) = ctx.ui.sheets_view else {
            return;
        };
        let row_count = ctx.state.sheet_names.len();
        if row_count == 0 {
            return;
        }
        let current = view.get_selected_row().unwrap_or(0);
        let prev = current.saturating_sub(1);
        let _ = view.select_row(prev);
    });
}

fn toggle_sheets_panel() {
    let _ = with_ui_context(|ctx| {
        if ctx.state.webview_enabled {
            return;
        }
        if ctx.state.sheets_panel_visible {
            ctx.state.sheets_sash_position = ctx.ui.compare_splitter.sash_position();
            let _ = ctx
                .ui
                .compare_splitter
                .unsplit(Some(&ctx.ui.sheets_list_panel));
            ctx.state.sheets_panel_visible = false;
        } else {
            let sash = ctx.state.sheets_sash_position.max(MIN_SASH_POSITION);
            if !ctx.ui.compare_splitter.split_vertically(
                &ctx.ui.sheets_list_panel,
                &ctx.ui.compare_right_panel,
                sash,
            ) {
                ctx.ui.compare_splitter.set_sash_position(sash, false);
            }
            ctx.state.sheets_panel_visible = true;
        }
        ctx.ui
            .toggle_sheets_menu
            .check(ctx.state.sheets_panel_visible);
        ctx.ui.compare_container.layout();
        ctx.ui.frame.layout();
    });
}

fn reset_layout() {
    let _ = with_ui_context(|ctx| {
        if ctx.state.webview_enabled {
            return;
        }
        ctx.state.sheets_panel_visible = true;
        ctx.state.sheets_sash_position = DEFAULT_SASH_POSITION;
        let _ = ctx.ui.compare_splitter.unsplit(None::<&Panel>);
        if !ctx.ui.compare_splitter.split_vertically(
            &ctx.ui.sheets_list_panel,
            &ctx.ui.compare_right_panel,
            DEFAULT_SASH_POSITION,
        ) {
            ctx.ui
                .compare_splitter
                .set_sash_position(DEFAULT_SASH_POSITION, false);
        }
        ctx.ui.toggle_sheets_menu.check(true);
        ctx.ui.frame.set_size(default_window_size());
        ctx.ui.frame.centre();
        ctx.ui.compare_container.layout();
        ctx.ui.root_tabs.layout();
        ctx.ui.frame.layout();
        update_status_in_ctx(ctx, "Layout reset.");
        clear_ui_state(&ctx.state.ui_state_path);
    });
}

fn minimize_window() {
    let _ = with_ui_context(|ctx| {
        ctx.ui.frame.iconize(true);
    });
}

fn toggle_maximize_window() {
    let _ = with_ui_context(|ctx| {
        if ctx.ui.frame.is_iconized() {
            ctx.ui.frame.iconize(false);
        }
        ctx.ui.frame.maximize(!ctx.ui.frame.is_maximized());
    });
}

fn open_docs() {
    let docs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/desktop.md");
    if !docs_path.exists() {
        let _ = with_ui_context(|ctx| {
            let dialog = MessageDialog::builder(
                &ctx.ui.frame,
                "Docs not found. See docs/desktop.md in the repo.",
                "Docs",
            )
            .with_style(MessageDialogStyle::IconInformation | MessageDialogStyle::OK)
            .build();
            let _ = dialog.show_modal();
        });
        return;
    }

    let path_str = docs_path.to_string_lossy().to_string();
    let launch = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "start", "", &path_str])
            .spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(&path_str).spawn()
    } else {
        Command::new("xdg-open").arg(&path_str).spawn()
    };

    if launch.is_err() {
        let _ = with_ui_context(|ctx| {
            let dialog = MessageDialog::builder(
                &ctx.ui.frame,
                &format!("Open docs at:\n{path_str}"),
                "Docs",
            )
            .with_style(MessageDialogStyle::IconInformation | MessageDialogStyle::OK)
            .build();
            let _ = dialog.show_modal();
        });
    }
}

fn setup_menu_handlers(ids: MenuIds) {
    let MenuIds {
        open_pair_id,
        open_old_id,
        open_new_id,
        open_recent_id,
        exit_id,
        compare_id,
        cancel_id,
        export_id,
        next_diff_id,
        prev_diff_id,
        copy_id,
        find_id,
        toggle_sheets_id,
        reset_layout_id,
        minimize_window_id,
        toggle_maximize_window_id,
        license_id,
        docs_id,
        about_id,
    } = ids;

    let _ = with_ui_context(|ctx| {
        ctx.ui.frame.on_menu_selected(move |event| match event.get_id() {
            id if id == open_pair_id => open_pair_dialog(),
            id if id == open_old_id => {
                let _ = with_ui_context(|ctx| {
                    let dialog = FileDialog::builder(&ctx.ui.frame)
                        .with_message("Open old file")
                        .with_wildcard("Excel/PBIX files (*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit)|*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit|All files (*.*)|*.*")
                        .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
                        .build();
                    if dialog.show_modal() == ID_OK {
                        if let Some(path) = dialog.get_path() {
                            ctx.ui.old_picker.set_path(&path);
                            sync_compare_controls_in_ctx(ctx);
                        }
                    }
                });
            }
            id if id == open_new_id => {
                let _ = with_ui_context(|ctx| {
                    let dialog = FileDialog::builder(&ctx.ui.frame)
                        .with_message("Open new file")
                        .with_wildcard("Excel/PBIX files (*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit)|*.xlsx;*.xlsm;*.xltx;*.xltm;*.xlsb;*.pbix;*.pbit|All files (*.*)|*.*")
                        .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
                        .build();
                    if dialog.show_modal() == ID_OK {
                        if let Some(path) = dialog.get_path() {
                            ctx.ui.new_picker.set_path(&path);
                            sync_compare_controls_in_ctx(ctx);
                        }
                    }
                });
            }
            id if id == open_recent_id => {
                let _ = with_ui_context(|ctx| {
                    ctx.ui.root_tabs.set_selection(1);
                });
            }
            id if id == exit_id => {
                let _ = with_ui_context(|ctx| {
                    ctx.ui.frame.close(true);
                });
            }
            id if id == compare_id => start_compare(),
            id if id == cancel_id => cancel_current(),
            id if id == export_id => export_audit(),
            id if id == next_diff_id => select_next_diff(),
            id if id == prev_diff_id => select_prev_diff(),
            id if id == copy_id => copy_current_text(),
            id if id == find_id => focus_search(),
            id if id == toggle_sheets_id => toggle_sheets_panel(),
            id if id == reset_layout_id => reset_layout(),
            id if id == minimize_window_id => minimize_window(),
            id if id == toggle_maximize_window_id => toggle_maximize_window(),
            id if id == license_id => show_license_dialog(),
            id if id == docs_id => open_docs(),
            id if id == about_id => {
                let _ = with_ui_context(|ctx| {
                    let dialog = MessageDialog::builder(
                        &ctx.ui.frame,
                        &format!("Tabulensis {}", env!("CARGO_PKG_VERSION")),
                        "About",
                    )
                    .build();
                    let _ = dialog.show_modal();
                });
            }
            _ => {}
        });
    });
}

fn export_audit() {
    let mut request = None;
    let _ = with_ui_context(|ctx| {
        let Some(diff_id) = ctx.state.current_diff_id.clone() else {
            update_status_in_ctx(ctx, "Run a diff before exporting.");
            return;
        };

        let Some(summary) = ctx.state.current_summary.clone() else {
            update_status_in_ctx(ctx, "Summary missing.");
            return;
        };

        let filename = DesktopBackend::default_export_name(&summary, "audit", "xlsx");
        let dialog = FileDialog::builder(&ctx.ui.frame)
            .with_message("Export audit XLSX")
            .with_default_file(&filename)
            .with_wildcard("Excel (*.xlsx)|*.xlsx|All files (*.*)|*.*")
            .with_style(FileDialogStyle::Save | FileDialogStyle::OverwritePrompt)
            .build();

        if dialog.show_modal() == ID_OK {
            if let Some(path) = dialog.get_path() {
                let backend = ctx.state.backend.clone();
                request = Some((backend, diff_id, path));
                update_status_in_ctx(ctx, "Exporting audit...");
            }
        }
    });

    let Some((backend, diff_id, path)) = request else {
        return;
    };

    thread::spawn(move || {
        let result = backend.export_audit_xlsx_to_path(&diff_id, Path::new(&path));
        wxdragon::call_after(Box::new(move || match result {
            Ok(()) => update_status("Export complete."),
            Err(err) => update_status(&format!("Export failed: {}", err.message)),
        }));
    });
}

#[derive(Clone, Copy)]
struct MenuIds {
    open_pair_id: i32,
    open_old_id: i32,
    open_new_id: i32,
    open_recent_id: i32,
    exit_id: i32,
    compare_id: i32,
    cancel_id: i32,
    export_id: i32,
    next_diff_id: i32,
    prev_diff_id: i32,
    copy_id: i32,
    find_id: i32,
    toggle_sheets_id: i32,
    reset_layout_id: i32,
    minimize_window_id: i32,
    toggle_maximize_window_id: i32,
    license_id: i32,
    docs_id: i32,
    about_id: i32,
}

fn main() {
    init_logging();
    configure_linux_environment();
    install_glib_log_suppression();
    maybe_redirect_stdio_to_null();
    let backend = DesktopBackend::init(BackendConfig {
        app_name: "excel_diff".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
    })
    .unwrap_or_else(|err| panic!("Backend init failed: {}", err.message));
    install_panic_log(backend.paths.app_data_dir.join("crash.log"));

    let dev_scenario = match load_dev_scenario() {
        Ok(scenario) => scenario,
        Err(err) => {
            eprintln!("UI scenario error: {err}");
            std::process::exit(1);
        }
    };

    wxdragon::main(move |_| {
        let ui_handles = ui::build_ui_handles(None, false);
        let layout_debug = layout_debug_enabled();

        let ui_state_path = backend.paths.app_data_dir.join("ui_state.json");
        let profiles_path = backend.paths.app_data_dir.join("compare_profiles.json");
        let user_profiles = profiles::load_user_profiles(&profiles_path);
        let ui_state = if ui_state_disabled() {
            UiState::default()
        } else {
            load_ui_state(&ui_state_path)
        };
        let state = AppState {
            backend,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            run_counter: 0,
            active_run: None,
            cancel_restore_snapshot: None,
            current_diff_id: None,
            current_mode: None,
            current_summary: None,
            current_payload: None,
            pending_detail_payload: None,
            pending_detail_sheet_name: None,
            pending_detail_payload_gen: 0,
            pending_detail_render_epoch: 0,
            pending_detail_json: None,
            pending_detail_json_gen: None,
            pending_detail_json_inflight_gen: None,
            sheet_names: Vec::new(),
            sheets_all: Vec::new(),
            sheets_filter: String::new(),
            recents: Vec::new(),
            search_old_index: None,
            search_new_index: None,
            batch_outcome: None,
            sheets_table: None,
            recents_table: None,
            batch_table: None,
            search_table: None,
            summary_categories_table: None,
            summary_top_sheets_table: None,
            webview_enabled: false,
            sheets_panel_visible: ui_state.sheets_panel_visible.unwrap_or(true),
            sheets_sash_position: ui_state.compare_sash.unwrap_or(DEFAULT_SASH_POSITION),
            ui_state_path,
            profiles_path,
            user_profiles,
            profile_choice_ids: Vec::new(),
            selected_profile_id: None,
            display_analysis: None,
            dev_scenario: dev_scenario.clone(),
            dev_ready_file: None,
            dev_ready_fired: false,
        };

        UI_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = Some(UiContext {
                ui: ui_handles,
                state,
            });
        });

        let menu_ids = with_ui_context(|ctx| MenuIds {
            open_pair_id: ctx.ui.open_pair_menu.get_id(),
            open_old_id: ctx.ui.open_old_menu.get_id(),
            open_new_id: ctx.ui.open_new_menu.get_id(),
            open_recent_id: ctx.ui.open_recent_menu.get_id(),
            exit_id: ctx.ui.exit_menu.get_id(),
            compare_id: ctx.ui.compare_menu.get_id(),
            cancel_id: ctx.ui.cancel_menu.get_id(),
            export_id: ctx.ui.export_audit_menu.get_id(),
            next_diff_id: ctx.ui.next_diff_menu.get_id(),
            prev_diff_id: ctx.ui.prev_diff_menu.get_id(),
            copy_id: ctx.ui.copy_menu.get_id(),
            find_id: ctx.ui.find_menu.get_id(),
            toggle_sheets_id: ctx.ui.toggle_sheets_menu.get_id(),
            reset_layout_id: ctx.ui.reset_layout_menu.get_id(),
            minimize_window_id: ctx.ui.minimize_window_menu.get_id(),
            toggle_maximize_window_id: ctx.ui.toggle_maximize_window_menu.get_id(),
            license_id: ctx.ui.license_menu.get_id(),
            docs_id: ctx.ui.docs_menu.get_id(),
            about_id: ctx.ui.about_menu.get_id(),
        })
        .unwrap();

        setup_menu_handlers(menu_ids);

        let _webview_enabled = with_ui_context(|ctx| {
            if webview_enabled_by_env() {
                setup_webview(ctx)
            } else {
                false
            }
        })
        .unwrap_or(false);

        let ui_state_for_init = ui_state.clone();
        let should_maximize = should_start_maximized(&ui_state_for_init);
        let should_center = (ui_state_for_init.window_x.is_none()
            || ui_state_for_init.window_y.is_none())
            && !should_maximize;
        let _ = with_ui_context(|ctx| {
            ctx.ui.frame.on_close(|event| {
                if !ui_state_disabled() {
                    let _ = with_ui_context(|ctx| {
                        let state = capture_ui_state(ctx);
                        save_ui_state(&ctx.state.ui_state_path, &state);
                    });
                }
                event.skip(true);
            });

            if !ctx.state.webview_enabled {
                ctx.ui.status_bar.set_fields_count(3);
                ctx.ui.status_bar.set_status_widths(&[-1, 220, 180]);
                update_status_counts_in_ctx(ctx, None);

                ctx.ui.cancel_btn.enable(false);
                update_status_in_ctx(ctx, "Ready");

                ctx.ui.preset_choice.append("Balanced");
                ctx.ui.preset_choice.append("Fastest");
                ctx.ui.preset_choice.append("Most precise");
                ctx.ui.preset_choice.set_selection(0);

                ctx.ui.summary_text.set_value(GUIDED_EMPTY_SUMMARY);
                ctx.ui.detail_text.set_value(GUIDED_EMPTY_DETAILS);
                ctx.ui.explain_text.set_value(GUIDED_EMPTY_EXPLAIN);

                ctx.ui.search_scope_choice.append("Changes");
                ctx.ui.search_scope_choice.append("Old workbook");
                ctx.ui.search_scope_choice.append("New workbook");
                ctx.ui.search_scope_choice.set_selection(0);

                ctx.ui.compare_btn.on_click(|_| start_compare());
                ctx.ui.cancel_btn.on_click(|_| cancel_current());
                ctx.ui.swap_btn.on_click(|_| swap_old_new_paths());
                ctx.ui
                    .profiles_btn
                    .on_click(|_| profiles_dialog::show_profiles_dialog());
                ctx.ui
                    .profile_choice
                    .on_selection_changed(|_| handle_profile_choice_changed());
                ctx.ui.open_recent_btn.on_click(|_| open_recent());
                ctx.ui.run_batch_btn.on_click(|_| run_batch());
                ctx.ui.search_btn.on_click(|_| handle_search());
                ctx.ui.build_old_index_btn.on_click(|_| build_index("old"));
                ctx.ui.build_new_index_btn.on_click(|_| build_index("new"));
                ctx.ui
                    .old_picker
                    .on_file_changed(|_| handle_compare_inputs_changed());
                ctx.ui
                    .new_picker
                    .on_file_changed(|_| handle_compare_inputs_changed());
                ctx.ui.sheets_filter_ctrl.on_text_updated(|event| {
                    let query = event.get_string().unwrap_or_default();
                    // SearchCtrl may emit text events synchronously while we're already in
                    // a `with_ui_context()` borrow (e.g., when we clear it after a run).
                    wxdragon::call_after(Box::new(move || {
                        let _ = with_ui_context(|ctx| set_sheets_filter_query_in_ctx(ctx, query));
                    }));
                });
                ctx.ui.sheets_filter_ctrl.on_cancel_button_clicked(|_| {
                    wxdragon::call_after(Box::new(|| {
                        let _ = with_ui_context(|ctx| {
                            ctx.state.sheets_filter.clear();
                            ctx.ui.sheets_filter_ctrl.set_value("");
                            rebuild_sheet_list_in_ctx(ctx);
                        });
                    }));
                });
                ctx.ui
                    .hide_m_formatting_checkbox
                    .on_toggled(|_| handle_noise_filters_changed());
                ctx.ui
                    .hide_dax_formatting_checkbox
                    .on_toggled(|_| handle_noise_filters_changed());
                ctx.ui
                    .hide_formula_formatting_checkbox
                    .on_toggled(|_| handle_noise_filters_changed());
                ctx.ui
                    .collapse_moves_checkbox
                    .on_toggled(|_| handle_noise_filters_changed());
                ctx.ui.result_tabs.on_page_changed(|event| {
                    if event.get_selection() == Some(RESULT_TAB_DETAILS) {
                        let _ = with_ui_context(|ctx| render_staged_detail_payload(ctx));
                    } else if event.get_selection() == Some(RESULT_TAB_GRID) {
                        let _ = with_ui_context(|ctx| render_grid_for_current_selection(ctx));
                    }
                });
                ctx.ui.frame.on_key_down(|event| {
                    if let wxdragon::event::WindowEventData::Keyboard(key) = event {
                        if let Some(code) = key.get_key_code() {
                            if code == WXK_F6 || (code == WXK_F8 && !key.shift_down()) {
                                select_next_diff();
                            } else if code == WXK_F8 && key.shift_down() {
                                select_prev_diff();
                            }
                        }
                    }
                });

                ctx.ui.compare_splitter.on_sash_position_changed(|event| {
                    if let Some(pos) = event.get_sash_position() {
                        let _ = with_ui_context(|ctx| {
                            ctx.state.sheets_sash_position = pos;
                        });
                    }
                });
                ctx.ui.compare_splitter.on_unsplit(|_| {
                    let _ = with_ui_context(|ctx| {
                        ctx.state.sheets_panel_visible = false;
                        ctx.ui.toggle_sheets_menu.check(false);
                    });
                });

                apply_ui_state(ctx, &ui_state_for_init);
                apply_window_size_override(ctx);
                ctx.ui.compare_container.layout();
                ctx.ui.root_tabs.layout();
            } else {
                apply_frame_state(ctx, &ui_state_for_init);
                apply_window_size_override(ctx);
            }

            ctx.ui.frame.layout();
            ctx.ui.frame.show(true);
            let size = ctx.ui.frame.get_size();
            ctx.ui.frame.set_size(size);
            ctx.ui.frame.layout();
            if should_maximize {
                ctx.ui.frame.maximize(true);
            } else if should_center {
                ctx.ui.frame.centre();
            }
            wxdragon::set_top_window(&ctx.ui.frame);
        });

        let dev_scenario_for_init = dev_scenario.clone();
        wxdragon::call_after(Box::new(move || {
            let mut scenario_to_run: Option<UiScenario> = None;
            let _ = with_ui_context(|ctx| {
                if ctx.state.webview_enabled {
                    return;
                }
                let sheets_view = create_dataview(&ctx.ui.sheets_table_host, &SHEETS_COLUMNS);
                let recents_view = create_dataview(&ctx.ui.recents_list_panel, &RECENTS_COLUMNS);
                let batch_view = create_dataview(&ctx.ui.batch_results_list_panel, &BATCH_COLUMNS);
                let search_view =
                    create_dataview(&ctx.ui.search_results_list_panel, &SEARCH_COLUMNS);
                let summary_categories_view = create_dataview(
                    &ctx.ui.summary_categories_table_host,
                    &SUMMARY_CATEGORY_COLUMNS,
                );
                let summary_top_sheets_view = create_dataview(
                    &ctx.ui.summary_top_sheets_table_host,
                    &SUMMARY_TOP_SHEETS_COLUMNS,
                );

                ctx.state.sheets_table = Some(create_virtual_table(&sheets_view));
                ctx.state.recents_table = Some(create_virtual_table(&recents_view));
                ctx.state.batch_table = Some(create_virtual_table(&batch_view));
                ctx.state.search_table = Some(create_virtual_table(&search_view));
                ctx.state.summary_categories_table =
                    Some(create_virtual_table(&summary_categories_view));
                ctx.state.summary_top_sheets_table =
                    Some(create_virtual_table(&summary_top_sheets_view));

                ctx.ui.sheets_view = Some(sheets_view);
                ctx.ui.recents_view = Some(recents_view);
                ctx.ui.batch_view = Some(batch_view);
                ctx.ui.search_view = Some(search_view);
                ctx.ui.summary_categories_view = Some(summary_categories_view);
                ctx.ui.summary_top_sheets_view = Some(summary_top_sheets_view);
                render_grid_placeholder(ctx, "Select a sheet to preview grid changes.");

                if let Ok(recents) = ctx.state.backend.load_recents() {
                    populate_recents(ctx, recents);
                }

                if let Some(view) = ctx.ui.sheets_view {
                    view.bind_dataview_event(DataViewEventType::SelectionChanged, |event| {
                        if let Some(row) = event.get_row() {
                            let row = row as usize;
                            wxdragon::call_after(Box::new(move || handle_sheet_selection(row)));
                        }
                    });
                }

                if let Some(view) = ctx.ui.batch_view {
                    view.bind_dataview_event(DataViewEventType::ItemActivated, |event| {
                        if let Some(row) = event.get_row() {
                            let diff_id = with_ui_context(|ctx| {
                                ctx.state
                                    .batch_outcome
                                    .as_ref()
                                    .and_then(|outcome| outcome.items.get(row as usize))
                                    .and_then(|item| item.diff_id.clone())
                            })
                            .flatten();
                            if let Some(diff_id) = diff_id {
                                load_diff_summary_into_ui(diff_id);
                            }
                        }
                    });
                }

                if ctx.state.sheets_panel_visible {
                    ctx.ui
                        .compare_splitter
                        .set_sash_position(ctx.state.sheets_sash_position, true);
                }
                ctx.ui.frame.layout();
                let size = ctx.ui.frame.get_size();
                ctx.ui.frame.set_size(size);
                if layout_debug {
                    log_layout_sizes(ctx);
                }

                if let Some(scenario) = dev_scenario_for_init.clone() {
                    apply_dev_scenario(ctx, &scenario);
                    scenario_to_run = Some(scenario);
                }
            });

            if let Some(scenario) = scenario_to_run {
                if scenario.auto_run_diff {
                    start_compare();
                    if let Some(delay_ms) = scenario.cancel_after_ms {
                        if delay_ms > 0 {
                            // For deterministic capture, prefer a simple blocking delay on the UI
                            // thread over cross-thread scheduling (which can be flaky under some
                            // headless/Xvfb setups).
                            thread::sleep(Duration::from_millis(delay_ms));
                        }
                        cancel_current();
                    }
                } else {
                    if scenario.stable_wait_ms > 0 {
                        // See note above: deterministic, headless-friendly readiness.
                        thread::sleep(Duration::from_millis(scenario.stable_wait_ms));
                    }
                    mark_ui_ready("idle_ready");
                }
            }
        }));
    })
    .expect("wxDragon app failed");
}

fn init_logging() {
    static LOGGER: SimpleLogger = SimpleLogger;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log_level_from_env());
}

fn env_flag(name: &str) -> Option<bool> {
    match std::env::var(name).as_deref() {
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES") | Ok("on") | Ok("ON") => {
            Some(true)
        }
        Ok("0") | Ok("false") | Ok("FALSE") | Ok("no") | Ok("NO") | Ok("off") | Ok("OFF") => {
            Some(false)
        }
        _ => None,
    }
}

fn env_string(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn ui_state_disabled() -> bool {
    env_flag("EXCEL_DIFF_UI_DISABLE_STATE").unwrap_or(false)
        || env_string("EXCEL_DIFF_DEV_SCENARIO").is_some()
}

fn parse_window_size(value: &str) -> Option<Size> {
    let clean = value
        .trim()
        .replace('x', " ")
        .replace('X', " ")
        .replace(',', " ");
    let mut parts = clean.split_whitespace();
    let width = parts.next()?.parse::<i32>().ok()?;
    let height = parts.next()?.parse::<i32>().ok()?;
    if width <= 0 || height <= 0 {
        return None;
    }
    Some(Size::new(width, height))
}

fn window_size_override() -> Option<Size> {
    env_string("EXCEL_DIFF_WINDOW_SIZE").and_then(|value| parse_window_size(&value))
}

fn apply_window_size_override(ctx: &mut UiContext) -> bool {
    let Some(size) = window_size_override() else {
        return false;
    };
    let min_size = min_window_size();
    let width = size.width.max(min_size.width);
    let height = size.height.max(min_size.height);
    ctx.ui.frame.set_size(Size::new(width, height));
    true
}

fn dev_ready_file_path() -> Option<PathBuf> {
    env_string("EXCEL_DIFF_UI_READY_FILE").map(PathBuf::from)
}

fn preset_index_from_name(value: &str) -> Option<u32> {
    let normalized = value.trim().to_lowercase().replace('_', "-");
    match normalized.as_str() {
        "balanced" | "default" => Some(0),
        "fast" | "fastest" => Some(1),
        "precise" | "most-precise" => Some(2),
        _ => None,
    }
}

fn apply_focus_panel(ctx: &mut UiContext, focus: Option<&str>) {
    let Some(focus) = focus else {
        return;
    };
    match focus.trim().to_lowercase().as_str() {
        "compare" => {
            ctx.ui.root_tabs.set_selection(0);
        }
        "recents" => {
            ctx.ui.root_tabs.set_selection(1);
        }
        "batch" => {
            ctx.ui.root_tabs.set_selection(2);
        }
        "search" => {
            ctx.ui.root_tabs.set_selection(3);
        }
        "summary" => {
            ctx.ui.root_tabs.set_selection(0);
            ctx.ui.result_tabs.set_selection(0);
        }
        "details" => {
            ctx.ui.root_tabs.set_selection(0);
            ctx.ui.result_tabs.set_selection(1);
            render_staged_detail_payload(ctx);
        }
        "explain" => {
            ctx.ui.root_tabs.set_selection(0);
            ctx.ui
                .result_tabs
                .set_selection(RESULT_TAB_EXPLAIN as usize);
        }
        "grid" => {
            ctx.ui.root_tabs.set_selection(0);
            ctx.ui.result_tabs.set_selection(RESULT_TAB_GRID as usize);
            render_grid_for_current_selection(ctx);
        }
        _ => {}
    }
}

fn apply_dev_scenario(ctx: &mut UiContext, scenario: &UiScenario) {
    ctx.state.dev_scenario = Some(scenario.clone());
    ctx.state.dev_ready_file = dev_ready_file_path();
    ctx.state.dev_ready_fired = false;

    info!(
        "Applying UI scenario '{}' (auto_run_diff={}, expect_mode={:?}, focus_panel={:?})",
        scenario.name, scenario.auto_run_diff, scenario.expect_mode, scenario.focus_panel
    );
    debug!(
        "Scenario paths: old={:?} new={:?} ready_file={:?} cancel_after_ms={:?}",
        scenario.old_path, scenario.new_path, ctx.state.dev_ready_file, scenario.cancel_after_ms
    );
    if let Some(old_path) = scenario.old_path.as_ref() {
        ctx.ui.old_picker.set_path(&old_path.to_string_lossy());
    } else {
        // Ensure deterministic "empty state" captures (do not retain any previous path).
        ctx.ui.old_picker.set_path("");
    }
    if let Some(new_path) = scenario.new_path.as_ref() {
        ctx.ui.new_picker.set_path(&new_path.to_string_lossy());
    } else {
        ctx.ui.new_picker.set_path("");
    }

    if let Some(profile_id) = scenario.profile_id.as_deref() {
        rebuild_profile_choice_in_ctx(ctx, Some(profile_id));
    }

    if let Some(trusted) = scenario.trusted_files {
        ctx.ui.trusted_checkbox.set_value(trusted);
    }

    if let Some(preset) = scenario
        .preset
        .as_ref()
        .and_then(|value| preset_index_from_name(value))
    {
        let max = ctx.ui.preset_choice.get_count().saturating_sub(1);
        let choice = preset.min(max);
        ctx.ui.preset_choice.set_selection(choice);
    }

    sync_compare_controls_in_ctx(ctx);
    apply_focus_panel(ctx, scenario.focus_panel.as_deref());
    let status = scenario
        .description
        .as_deref()
        .map(str::trim)
        .filter(|desc| !desc.is_empty())
        .map(|desc| format!("Scenario loaded: {} ({})", scenario.name, desc))
        .unwrap_or_else(|| format!("Scenario loaded: {}", scenario.name));
    update_status_in_ctx(ctx, &status);
}

fn mark_ui_ready(reason: &str) {
    let reason = reason.to_string();
    debug!("mark_ui_ready invoked: reason={reason}");
    let did_run = with_ui_context(|ctx| {
        if ctx.state.dev_ready_fired {
            return false;
        }
        let Some(path) = ctx.state.dev_ready_file.clone() else {
            return false;
        };
        ctx.state.dev_ready_fired = true;

        let status_text = ctx.ui.progress_text.get_label();
        let root_tab = ctx.ui.root_tabs.selection();
        let result_tab = ctx.ui.result_tabs.selection();
        let selected_sheet = ctx
            .ui
            .sheets_view
            .and_then(|view| view.get_selected_row())
            .and_then(|row| ctx.state.sheet_names.get(row).cloned());
        let scenario = ctx.state.dev_scenario.as_ref();
        let expected_mode = scenario
            .and_then(|s| s.expect_mode.as_ref())
            .map(|value| value.to_lowercase());
        let actual_mode = ctx
            .state
            .current_mode
            .as_ref()
            .map(|mode| mode.as_str().to_string());

        let status = match (&expected_mode, &actual_mode) {
            (Some(expected), Some(actual)) if expected != actual => "mode_mismatch",
            _ => "ok",
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let payload = json!({
            "scenario": scenario.map(|s| s.name.clone()),
            "reason": reason,
            "status": status,
            "timestamp_unix": timestamp,
            "expected_mode": expected_mode,
            "actual_mode": actual_mode,
            "diff_id": ctx.state.current_diff_id,
            "status_text": status_text,
            "sheet_count": ctx.state.sheet_names.len(),
            "root_tab": root_tab,
            "result_tab": result_tab,
            "selected_sheet": selected_sheet,
        });

        debug!(
            "UI ready: status={} reason={} path={:?}",
            status, reason, path
        );
        if let Ok(body) = serde_json::to_string_pretty(&payload) {
            let _ = std::fs::write(path, body);
        }
        true
    })
    .unwrap_or(false);
    if !did_run {
        debug!("mark_ui_ready: no UI context (or no ready file configured).");
    }
}

fn configure_linux_environment() {
    if !cfg!(target_os = "linux") {
        return;
    }

    let suppress = env_flag("EXCEL_DIFF_SUPPRESS_GTK_WARNINGS").unwrap_or(cfg!(debug_assertions));
    if suppress {
        std::env::set_var("GSETTINGS_BACKEND", "memory");
    }

    let disable_overlay = env_flag("EXCEL_DIFF_DISABLE_OVERLAY_SCROLLBARS").unwrap_or(suppress);
    if disable_overlay {
        std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
    }

    if let Ok(theme) = std::env::var("EXCEL_DIFF_CURSOR_THEME") {
        if !theme.trim().is_empty() {
            std::env::set_var("XCURSOR_THEME", theme);
        }
    } else if env_flag("EXCEL_DIFF_FORCE_CURSOR_THEME") == Some(true) || suppress {
        std::env::set_var("XCURSOR_THEME", "Adwaita");
    }

    if let Ok(size) = std::env::var("EXCEL_DIFF_CURSOR_SIZE") {
        if !size.trim().is_empty() {
            std::env::set_var("XCURSOR_SIZE", size);
        }
    } else if suppress {
        std::env::set_var("XCURSOR_SIZE", "24");
    }
}

#[cfg(target_os = "linux")]
extern "C" {
    fn g_log_set_handler(
        log_domain: *const c_char,
        log_levels: i32,
        log_func: Option<extern "C" fn(*const c_char, i32, *const c_char, *mut c_void)>,
        user_data: *mut c_void,
    ) -> u32;
    fn g_log_set_default_handler(
        log_func: Option<extern "C" fn(*const c_char, i32, *const c_char, *mut c_void)>,
        user_data: *mut c_void,
    ) -> u32;
}

#[cfg(target_os = "linux")]
extern "C" fn ignore_glib_log(
    _domain: *const c_char,
    _level: i32,
    _message: *const c_char,
    _data: *mut c_void,
) {
}

#[cfg(target_os = "linux")]
fn install_glib_log_suppression() {
    if !env_flag("EXCEL_DIFF_SUPPRESS_GTK_WARNINGS").unwrap_or(cfg!(debug_assertions)) {
        return;
    }
    let levels = 0xFF;
    unsafe {
        g_log_set_default_handler(Some(ignore_glib_log), std::ptr::null_mut());
        g_log_set_handler(
            std::ptr::null(),
            levels,
            Some(ignore_glib_log),
            std::ptr::null_mut(),
        );
    }
    for domain in ["Gdk", "Gtk", "GLib", "GLib-GObject", "GdkPixbuf", "Pango"] {
        let Ok(cstr) = CString::new(domain) else {
            continue;
        };
        let leaked = Box::leak(cstr.into_boxed_c_str());
        unsafe {
            g_log_set_handler(
                leaked.as_ptr(),
                levels,
                Some(ignore_glib_log),
                std::ptr::null_mut(),
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn install_glib_log_suppression() {}

#[cfg(target_os = "linux")]
fn maybe_redirect_stdio_to_null() {
    // When driving the UI via dev scenarios / capture scripts, we rely on stdout/stderr for logs.
    // Don't redirect them away even if we're suppressing GTK warnings.
    if env_string("EXCEL_DIFF_LOG").is_some()
        || env_string("EXCEL_DIFF_UI_READY_FILE").is_some()
        || env_string("EXCEL_DIFF_DEV_SCENARIO").is_some()
    {
        return;
    }
    if !env_flag("EXCEL_DIFF_SUPPRESS_GTK_WARNINGS").unwrap_or(cfg!(debug_assertions)) {
        return;
    }
    let Ok(file) = OpenOptions::new().read(true).open("/dev/null") else {
        return;
    };
    unsafe {
        let fd = file.as_raw_fd();
        libc::dup2(fd, libc::STDERR_FILENO);
        libc::dup2(fd, libc::STDOUT_FILENO);
    }
    std::mem::forget(file);
}

#[cfg(not(target_os = "linux"))]
fn maybe_redirect_stdio_to_null() {}

fn install_panic_log(path: PathBuf) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "---- crash {} ----", ts);
            let _ = writeln!(file, "{info}");
            if let Ok(backtrace) = std::env::var("RUST_BACKTRACE") {
                if backtrace != "0" {
                    let _ = writeln!(file, "{:?}", std::backtrace::Backtrace::force_capture());
                }
            }
        }
        default_hook(info);
    }));
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn log_level_from_env() -> LevelFilter {
    match std::env::var("EXCEL_DIFF_LOG").as_deref() {
        Ok("error") => LevelFilter::Error,
        Ok("warn") => LevelFilter::Warn,
        Ok("debug") => LevelFilter::Debug,
        Ok("trace") => LevelFilter::Trace,
        Ok("off") => LevelFilter::Off,
        _ => LevelFilter::Warn,
    }
}

fn webview_enabled_by_env() -> bool {
    env_flag("EXCEL_DIFF_USE_WEBVIEW").unwrap_or(false)
}
