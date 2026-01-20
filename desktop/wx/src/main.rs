use std::cell::RefCell;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

mod logic;
mod xrc_validation;

use desktop_backend::{
    BatchOutcome, BatchRequest, BackendConfig, DesktopBackend, DiffErrorPayload, DiffMode, DiffOutcome, DiffRequest,
    DiffRunSummary, ProgressRx, RecentComparison, SearchIndexResult, SearchIndexSummary, SearchResult, SheetPayloadRequest,
};
use logic::{base_name, parse_globs, preset_from_selection};
use log::{debug, info, LevelFilter, Metadata, Record};
use ui_payload::{DiffOptions, DiffPreset};
use wxdragon::prelude::*;
use wxdragon::xrc::{FromXrcPtr, XmlResource};
use wxdragon_sys as ffi;
use xrc_validation::validate_xrc;

const SHEETS_COLUMNS: [(&str, i32); 6] = [
    ("Sheet", 200),
    ("Ops", 70),
    ("Added", 70),
    ("Removed", 80),
    ("Modified", 80),
    ("Moved", 70),
];

const RECENTS_COLUMNS: [(&str, i32); 4] = [
    ("Old", 220),
    ("New", 220),
    ("Last Run", 160),
    ("Mode", 80),
];

const BATCH_COLUMNS: [(&str, i32); 6] = [
    ("Old", 200),
    ("New", 200),
    ("Status", 90),
    ("Ops", 70),
    ("Warnings", 90),
    ("Error", 260),
];

const SEARCH_COLUMNS: [(&str, i32); 5] = [
    ("Kind", 120),
    ("Sheet", 180),
    ("Address", 100),
    ("Label", 200),
    ("Detail", 260),
];

struct MainUi {
    main_frame: Frame,
    open_old_menu: MenuItem,
    open_new_menu: MenuItem,
    exit_menu: MenuItem,
    compare_menu: MenuItem,
    cancel_menu: MenuItem,
    export_audit_menu: MenuItem,
    about_menu: MenuItem,
    status_bar: StatusBar,
    root_tabs: Notebook,
    sheets_list: Panel,
    old_picker: FilePickerCtrl,
    new_picker: FilePickerCtrl,
    compare_btn: Button,
    cancel_btn: Button,
    preset_choice: Choice,
    trusted_checkbox: CheckBox,
    progress_gauge: Gauge,
    progress_text: StaticText,
    summary_text: TextCtrl,
    detail_text: TextCtrl,
    recents_list: Panel,
    open_recent_btn: Button,
    batch_old_dir: DirPickerCtrl,
    batch_new_dir: DirPickerCtrl,
    run_batch_btn: Button,
    include_glob_text: TextCtrl,
    exclude_glob_text: TextCtrl,
    batch_results_list: Panel,
    search_ctrl: SearchCtrl,
    search_scope_choice: Choice,
    search_btn: Button,
    build_old_index_btn: Button,
    build_new_index_btn: Button,
    search_results_list: Panel,
    auto_destroy_root: bool,
    _resource: XmlResource,
}

impl MainUi {
    const XRC_DATA: &'static str = include_str!("../ui/main.xrc");

    pub fn new(parent: Option<&dyn WxWidget>, auto_destroy_root: bool) -> Self {
        maybe_validate_xrc();
        let resource = XmlResource::get();
        resource.init_platform_aware_staticbitmap_handler();
        resource.init_all_handlers();
        info!("Loading XRC data.");
        resource
            .load_from_string(Self::XRC_DATA)
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to load XRC data: {err}\nEnable EXCEL_DIFF_VALIDATE_XRC=1 for structural checks."
                )
            });

        info!("Loading main frame.");
        let main_frame = resource
            .load_frame(parent, "main_frame")
            .unwrap_or_else(|| panic!("Failed to load XRC root object: main_frame"));
        let _menu_bar = main_frame
            .get_menu_bar()
            .unwrap_or_else(|| panic!("Failed to get MenuBar from Frame"));

        let open_old_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "open_old_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: open_old_menu"));
        let open_new_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "open_new_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: open_new_menu"));
        let exit_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "exit_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: exit_menu"));
        let compare_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "compare_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: compare_menu"));
        let cancel_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "cancel_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: cancel_menu"));
        let export_audit_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "export_audit_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: export_audit_menu"));
        let about_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "about_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: about_menu"));

        let status_bar = find_xrc_child::<StatusBar>(&main_frame, "status_bar");
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
        let old_picker = find_xrc_child::<FilePickerCtrl>(&compare_page, "old_picker");
        let new_picker = find_xrc_child::<FilePickerCtrl>(&compare_page, "new_picker");
        let compare_btn = find_xrc_child::<Button>(&compare_page, "compare_btn");
        let cancel_btn = find_xrc_child::<Button>(&compare_page, "cancel_btn");
        let preset_choice = find_xrc_child::<Choice>(&compare_page, "preset_choice");
        let trusted_checkbox = find_xrc_child::<CheckBox>(&compare_page, "trusted_checkbox");
        let progress_gauge = find_xrc_child::<Gauge>(&compare_page, "progress_gauge");
        let progress_text = find_xrc_child::<StaticText>(&compare_page, "progress_text");
        let summary_text = find_xrc_child::<TextCtrl>(&compare_page, "summary_text");
        let detail_text = find_xrc_child::<TextCtrl>(&compare_page, "detail_text");

        let recents_list = find_xrc_child::<Panel>(&recents_page, "recents_list");
        let open_recent_btn = find_xrc_child::<Button>(&recents_page, "open_recent_btn");

        let batch_old_dir = find_xrc_child::<DirPickerCtrl>(&batch_page, "batch_old_dir");
        let batch_new_dir = find_xrc_child::<DirPickerCtrl>(&batch_page, "batch_new_dir");
        let run_batch_btn = find_xrc_child::<Button>(&batch_page, "run_batch_btn");
        let include_glob_text = find_xrc_child::<TextCtrl>(&batch_page, "include_glob_text");
        let exclude_glob_text = find_xrc_child::<TextCtrl>(&batch_page, "exclude_glob_text");
        let batch_results_list = find_xrc_child::<Panel>(&batch_page, "batch_results_list");

        let search_ctrl = find_xrc_child::<SearchCtrl>(&search_page, "search_ctrl");
        let search_scope_choice = find_xrc_child::<Choice>(&search_page, "search_scope_choice");
        let search_btn = find_xrc_child::<Button>(&search_page, "search_btn");
        let build_old_index_btn = find_xrc_child::<Button>(&search_page, "build_old_index_btn");
        let build_new_index_btn = find_xrc_child::<Button>(&search_page, "build_new_index_btn");
        let search_results_list = find_xrc_child::<Panel>(&search_page, "search_results_list");
        debug!("XRC widgets loaded successfully.");

        Self {
            main_frame,
            open_old_menu,
            open_new_menu,
            exit_menu,
            compare_menu,
            cancel_menu,
            export_audit_menu,
            about_menu,
            status_bar,
            root_tabs,
            sheets_list,
            old_picker,
            new_picker,
            compare_btn,
            cancel_btn,
            preset_choice,
            trusted_checkbox,
            progress_gauge,
            progress_text,
            summary_text,
            detail_text,
            recents_list,
            open_recent_btn,
            batch_old_dir,
            batch_new_dir,
            run_batch_btn,
            include_glob_text,
            exclude_glob_text,
            batch_results_list,
            search_ctrl,
            search_scope_choice,
            search_btn,
            build_old_index_btn,
            build_new_index_btn,
            search_results_list,
            auto_destroy_root,
            _resource: resource,
        }
    }
}

impl Drop for MainUi {
    fn drop(&mut self) {
        if self.auto_destroy_root {
            self.main_frame.destroy();
        }
    }
}

fn find_xrc_child<T>(parent: &impl WxWidget, name: &str) -> T
where
    T: FromXrcPtr<RawFfiType = *mut ffi::wxd_Window_t> + WxWidget,
{
    let id = XmlResource::get_xrc_id(name);
    if id == 0 || id == -1 {
        panic!(
            "Failed to find XRC id: {name}. Enable EXCEL_DIFF_VALIDATE_XRC=1 for details."
        );
    }

    let child_ptr = unsafe { ffi::wxd_Window_FindWindowById(parent.handle_ptr(), id) };
    if child_ptr.is_null() {
        panic!(
            "Failed to find XRC child: {name}. Check widget names in the XRC and run with EXCEL_DIFF_VALIDATE_XRC=1."
        );
    }

    unsafe { T::from_xrc_ptr(child_ptr) }
}

fn maybe_validate_xrc() {
    let should_validate = cfg!(debug_assertions)
        || std::env::var("EXCEL_DIFF_VALIDATE_XRC")
            .map(|value| value == "1")
            .unwrap_or(false);
    if should_validate {
        if let Err(err) = validate_xrc(MainUi::XRC_DATA) {
            panic!("XRC validation failed:\n{err}");
        }
    }
}

struct UiHandles {
    frame: Frame,
    open_old_menu: MenuItem,
    open_new_menu: MenuItem,
    exit_menu: MenuItem,
    compare_menu: MenuItem,
    cancel_menu: MenuItem,
    export_audit_menu: MenuItem,
    about_menu: MenuItem,
    status_bar: StatusBar,
    progress_text: StaticText,
    progress_gauge: Gauge,
    compare_btn: Button,
    cancel_btn: Button,
    old_picker: FilePickerCtrl,
    new_picker: FilePickerCtrl,
    preset_choice: Choice,
    trusted_checkbox: CheckBox,
    summary_text: TextCtrl,
    detail_text: TextCtrl,
    root_tabs: Notebook,
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
    sheets_view: DataViewCtrl,
    recents_view: DataViewCtrl,
    batch_view: DataViewCtrl,
    search_view: DataViewCtrl,
}

struct ActiveRun {
    cancel: Arc<AtomicBool>,
}

struct AppState {
    backend: DesktopBackend,
    run_counter: u64,
    active_run: Option<ActiveRun>,
    current_diff_id: Option<String>,
    current_mode: Option<DiffMode>,
    current_summary: Option<DiffRunSummary>,
    current_payload: Option<ui_payload::DiffWithSheets>,
    sheet_names: Vec<String>,
    recents: Vec<RecentComparison>,
    search_old_index: Option<SearchIndexSummary>,
    search_new_index: Option<SearchIndexSummary>,
    batch_outcome: Option<BatchOutcome>,
    sheets_model: DataViewListModel,
    recents_model: DataViewListModel,
    batch_model: DataViewListModel,
    search_model: DataViewListModel,
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

fn update_status(message: &str) {
    let message = message.to_string();
    let _ = with_ui_context(|ctx| update_status_in_ctx(ctx, &message));
}

fn create_dataview(parent: &Panel, columns: &[(&str, i32)]) -> DataViewCtrl {
    let ctrl = DataViewCtrl::builder(parent)
        .with_style(DataViewStyle::RowLines | DataViewStyle::VerticalRules)
        .build();

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

fn rebuild_model(ctrl: &DataViewCtrl, columns: &[(&str, i32)], rows: Vec<Vec<String>>) -> DataViewListModel {
    let model = DataViewListModel::new();
    for (label, _) in columns {
        let _ = model.append_column(label);
    }

    for (row_idx, row) in rows.into_iter().enumerate() {
        let _ = model.append_row();
        for (col_idx, value) in row.into_iter().enumerate() {
            let _ = model.set_value(row_idx, col_idx, value);
        }
    }

    let _ = ctrl.associate_model(&model);
    model
}

fn spawn_progress_forwarder(rx: ProgressRx) {
    thread::spawn(move || {
        for event in rx.iter() {
            let detail = event.detail;
            wxdragon::call_after(Box::new(move || update_status(&detail)));
        }
    });
}

fn preset_from_choice(choice: &Choice) -> DiffPreset {
    let selection = choice.get_selection().unwrap_or(0);
    let selection = i32::try_from(selection).unwrap_or(0);
    preset_from_selection(selection)
}

fn populate_sheet_list(ctx: &mut UiContext, summary: &DiffRunSummary) {
    ctx.state.sheet_names = summary
        .sheets
        .iter()
        .map(|sheet| sheet.sheet_name.clone())
        .collect();

    let rows = summary
        .sheets
        .iter()
        .map(|sheet| {
            vec![
                sheet.sheet_name.clone(),
                sheet.op_count.to_string(),
                sheet.counts.added.to_string(),
                sheet.counts.removed.to_string(),
                sheet.counts.modified.to_string(),
                sheet.counts.moved.to_string(),
            ]
        })
        .collect::<Vec<_>>();

    ctx.state.sheets_model = rebuild_model(&ctx.ui.sheets_view, &SHEETS_COLUMNS, rows);
}

fn populate_recents(ctx: &mut UiContext, recents: Vec<RecentComparison>) {
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
    ctx.state.recents_model = rebuild_model(&ctx.ui.recents_view, &RECENTS_COLUMNS, rows);
}

fn handle_diff_result(result: Result<DiffOutcome, DiffErrorPayload>) {
    let _ = with_ui_context(|ctx| {
        ctx.ui.compare_btn.enable(true);
        ctx.ui.cancel_btn.enable(false);
        ctx.ui.progress_gauge.set_value(100);
        ctx.state.active_run = None;

        match result {
            Ok(outcome) => {
                ctx.state.current_diff_id = Some(outcome.diff_id.clone());
                ctx.state.current_mode = Some(outcome.mode);
                ctx.state.current_payload = outcome.payload;
                ctx.state.current_summary = outcome.summary.clone();

                if let Some(summary) = outcome.summary {
                    ctx.ui.summary_text
                        .set_value(&serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string()));
                    ctx.ui.detail_text.set_value("");
                    populate_sheet_list(ctx, &summary);

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
                }

                update_status_in_ctx(ctx, "Diff complete.");
            }
            Err(err) => {
                ctx.ui
                    .detail_text
                    .set_value(&format!("{}: {}", err.code, err.message));
                update_status_in_ctx(ctx, &format!("Diff failed: {}", err.message));
            }
        }
    });
}

fn start_compare() {
    let mut args = None;
    let _ = with_ui_context(|ctx| {
        let old_path = ctx.ui.old_picker.get_path();
        let new_path = ctx.ui.new_picker.get_path();

        if old_path.trim().is_empty() || new_path.trim().is_empty() {
            update_status_in_ctx(ctx, "Select both old and new files.");
            return;
        }

        if ctx.state.active_run.is_some() {
            update_status_in_ctx(ctx, "A diff is already running.");
            return;
        }

        ctx.state.run_counter = ctx.state.run_counter.saturating_add(1);
        let run_id = ctx.state.run_counter;
        let cancel = Arc::new(AtomicBool::new(false));
        ctx.state.active_run = Some(ActiveRun { cancel: cancel.clone() });
        ctx.state.current_payload = None;
        ctx.state.current_summary = None;
        ctx.state.sheet_names.clear();

        ctx.ui.compare_btn.enable(false);
        ctx.ui.cancel_btn.enable(true);
        ctx.ui.progress_gauge.set_value(0);
        update_status_in_ctx(ctx, "Starting diff...");

        let options = DiffOptions {
            preset: Some(preset_from_choice(&ctx.ui.preset_choice)),
            trusted: Some(ctx.ui.trusted_checkbox.is_checked()),
            ..DiffOptions::default()
        };

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
                if let (Some(summary), Some(sheet_name)) = (&ctx.state.current_summary, sheet_name) {
                    if let Some(sheet) = summary.sheets.iter().find(|sheet| sheet.sheet_name == sheet_name) {
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
        let payload = backend.runner.load_sheet_payload(SheetPayloadRequest {
            diff_id,
            sheet_name,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: progress_tx,
        });

        wxdragon::call_after(Box::new(move || match payload {
            Ok(payload) => {
                let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
                let _ = with_ui_context(|ctx| {
                    ctx.ui.detail_text.set_value(&text);
                    update_status_in_ctx(ctx, "Sheet payload loaded.");
                });
            }
            Err(err) => {
                update_status(&format!("Load failed: {}", err.message));
            }
        }));
    });
}

fn load_diff_summary_into_ui(diff_id: String) {
    let backend = with_ui_context(|ctx| ctx.state.backend.clone());
    let Some(backend) = backend else {
        return;
    };

    thread::spawn(move || {
        let summary = backend.load_diff_summary(&diff_id);
        wxdragon::call_after(Box::new(move || match summary {
            Ok(summary) => {
                let _ = with_ui_context(|ctx| {
                    ctx.state.current_diff_id = Some(diff_id.clone());
                    ctx.state.current_mode = Some(summary.mode);
                    ctx.state.current_summary = Some(summary.clone());
                    ctx.state.current_payload = None;

                    ctx.ui.summary_text
                        .set_value(&serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string()));
                    ctx.ui.detail_text.set_value("");
                    populate_sheet_list(ctx, &summary);
                    ctx.ui.root_tabs.set_selection(0);
                    update_status_in_ctx(ctx, "Summary loaded.");
                });
            }
            Err(err) => update_status(&format!("Load summary failed: {}", err.message)),
        }));
    });
}

fn run_batch() {
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
                            item.op_count.map(|v| v.to_string()).unwrap_or_else(|| "".to_string()),
                            item.warnings_count
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "".to_string()),
                            item.error.clone().unwrap_or_else(|| "".to_string()),
                        ]
                    })
                    .collect::<Vec<_>>();

                ctx.state.batch_model = rebuild_model(&ctx.ui.batch_view, &BATCH_COLUMNS, rows);
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
                request = Some(SearchRequest::DiffOps { backend, diff_id, query });
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
        SearchRequest::DiffOps { backend, diff_id, query } => {
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

        ctx.state.search_model = rebuild_model(&ctx.ui.search_view, &SEARCH_COLUMNS, rows);
        update_status_in_ctx(ctx, &format!("Search returned {} results.", results.len()));
    });
}

fn apply_index_results(results: Vec<SearchIndexResult>) {
    let _ = with_ui_context(|ctx| {
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

        ctx.state.search_model = rebuild_model(&ctx.ui.search_view, &SEARCH_COLUMNS, rows);
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
        if let Some(active) = ctx.state.active_run.as_ref() {
            active.cancel.store(true, Ordering::Relaxed);
            update_status_in_ctx(ctx, "Canceling...");
        }
    });
}

fn open_recent() {
    let mut request = None;
    let _ = with_ui_context(|ctx| {
        let Some(selected) = ctx.ui.recents_view.get_selected_row() else {
            update_status_in_ctx(ctx, "Select a recent comparison.");
            return;
        };

        let entry = ctx.state.recents.get(selected).cloned();
        let Some(entry) = entry else {
            return;
        };

        ctx.ui.old_picker.set_path(&entry.old_path);
        ctx.ui.new_picker.set_path(&entry.new_path);
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

fn setup_menu_handlers(ids: MenuIds) {
    let MenuIds {
        open_old_id,
        open_new_id,
        exit_id,
        compare_id,
        cancel_id,
        export_id,
        about_id,
    } = ids;

    let _ = with_ui_context(|ctx| {
        ctx.ui.frame.on_menu_selected(move |event| match event.get_id() {
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
                        }
                    }
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
            id if id == about_id => {
                let _ = with_ui_context(|ctx| {
                    let dialog = MessageDialog::builder(
                        &ctx.ui.frame,
                        &format!("Excel Diff {}", env!("CARGO_PKG_VERSION")),
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
    open_old_id: i32,
    open_new_id: i32,
    exit_id: i32,
    compare_id: i32,
    cancel_id: i32,
    export_id: i32,
    about_id: i32,
}

fn main() {
    init_logging();
    wxdragon::main(|_| {
        let backend = DesktopBackend::init(BackendConfig {
            app_name: "excel_diff".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
        })
        .unwrap_or_else(|err| panic!("Backend init failed: {}", err.message));

        let ui = MainUi::new(None, false);

        let sheets_view = create_dataview(&ui.sheets_list, &SHEETS_COLUMNS);
        let recents_view = create_dataview(&ui.recents_list, &RECENTS_COLUMNS);
        let batch_view = create_dataview(&ui.batch_results_list, &BATCH_COLUMNS);
        let search_view = create_dataview(&ui.search_results_list, &SEARCH_COLUMNS);

        let sheets_model = rebuild_model(&sheets_view, &SHEETS_COLUMNS, Vec::new());
        let recents_model = rebuild_model(&recents_view, &RECENTS_COLUMNS, Vec::new());
        let batch_model = rebuild_model(&batch_view, &BATCH_COLUMNS, Vec::new());
        let search_model = rebuild_model(&search_view, &SEARCH_COLUMNS, Vec::new());

        let ui_handles = UiHandles {
            frame: ui.main_frame,
            open_old_menu: ui.open_old_menu,
            open_new_menu: ui.open_new_menu,
            exit_menu: ui.exit_menu,
            compare_menu: ui.compare_menu,
            cancel_menu: ui.cancel_menu,
            export_audit_menu: ui.export_audit_menu,
            about_menu: ui.about_menu,
            status_bar: ui.status_bar,
            progress_text: ui.progress_text,
            progress_gauge: ui.progress_gauge,
            compare_btn: ui.compare_btn,
            cancel_btn: ui.cancel_btn,
            old_picker: ui.old_picker,
            new_picker: ui.new_picker,
            preset_choice: ui.preset_choice,
            trusted_checkbox: ui.trusted_checkbox,
            summary_text: ui.summary_text,
            detail_text: ui.detail_text,
            root_tabs: ui.root_tabs,
            open_recent_btn: ui.open_recent_btn,
            run_batch_btn: ui.run_batch_btn,
            search_btn: ui.search_btn,
            build_old_index_btn: ui.build_old_index_btn,
            build_new_index_btn: ui.build_new_index_btn,
            search_ctrl: ui.search_ctrl,
            search_scope_choice: ui.search_scope_choice,
            batch_old_dir: ui.batch_old_dir,
            batch_new_dir: ui.batch_new_dir,
            include_glob_text: ui.include_glob_text,
            exclude_glob_text: ui.exclude_glob_text,
            sheets_view,
            recents_view,
            batch_view,
            search_view,
        };

        let state = AppState {
            backend,
            run_counter: 0,
            active_run: None,
            current_diff_id: None,
            current_mode: None,
            current_summary: None,
            current_payload: None,
            sheet_names: Vec::new(),
            recents: Vec::new(),
            search_old_index: None,
            search_new_index: None,
            batch_outcome: None,
            sheets_model,
            recents_model,
            batch_model,
            search_model,
        };

        UI_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = Some(UiContext {
                ui: ui_handles,
                state,
            });
        });

        let menu_ids = with_ui_context(|ctx| MenuIds {
            open_old_id: ctx.ui.open_old_menu.get_id(),
            open_new_id: ctx.ui.open_new_menu.get_id(),
            exit_id: ctx.ui.exit_menu.get_id(),
            compare_id: ctx.ui.compare_menu.get_id(),
            cancel_id: ctx.ui.cancel_menu.get_id(),
            export_id: ctx.ui.export_audit_menu.get_id(),
            about_id: ctx.ui.about_menu.get_id(),
        })
        .unwrap();

        setup_menu_handlers(menu_ids);

        let _ = with_ui_context(|ctx| {
            ctx.ui.root_tabs.set_selection(0);
            ctx.ui.cancel_btn.enable(false);
            update_status_in_ctx(ctx, "Ready");

            ctx.ui.preset_choice.append("Balanced");
            ctx.ui.preset_choice.append("Fastest");
            ctx.ui.preset_choice.append("Most precise");
            ctx.ui.preset_choice.set_selection(0);

            ctx.ui.search_scope_choice.append("Changes");
            ctx.ui.search_scope_choice.append("Old workbook");
            ctx.ui.search_scope_choice.append("New workbook");
            ctx.ui.search_scope_choice.set_selection(0);

            if let Ok(recents) = ctx.state.backend.load_recents() {
                populate_recents(ctx, recents);
            }

            ctx.ui.compare_btn.on_click(|_| start_compare());
            ctx.ui.cancel_btn.on_click(|_| cancel_current());
            ctx.ui.open_recent_btn.on_click(|_| open_recent());
            ctx.ui.run_batch_btn.on_click(|_| run_batch());
            ctx.ui.search_btn.on_click(|_| handle_search());
            ctx.ui.build_old_index_btn.on_click(|_| build_index("old"));
            ctx.ui.build_new_index_btn.on_click(|_| build_index("new"));

            ctx.ui.sheets_view.bind_dataview_event(DataViewEventType::SelectionChanged, |event| {
                if let Some(row) = event.get_row() {
                    handle_sheet_selection(row as usize);
                }
            });

            ctx.ui.batch_view.bind_dataview_event(DataViewEventType::ItemActivated, |event| {
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

            ctx.ui.frame.show(true);
            ctx.ui.frame.centre();
            wxdragon::set_top_window(&ctx.ui.frame);
            wxdragon::call_after(Box::new(|| {
                let _ = with_ui_context(|ctx| ctx.ui.frame.layout());
            }));
        });
    })
    .expect("wxDragon app failed");
}

fn init_logging() {
    static LOGGER: SimpleLogger = SimpleLogger;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log_level_from_env());
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
        _ => LevelFilter::Info,
    }
}
