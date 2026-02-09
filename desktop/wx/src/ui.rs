use log::{debug, info};
use wxdragon::prelude::*;
use wxdragon::xrc::{FromXrcPtr, XmlResource};
use wxdragon_sys as ffi;

use crate::theme;
use crate::ui_constants::{
    default_window_size, min_root_tabs_size, min_window_size, MIN_SASH_POSITION,
};
use crate::xrc_validation::validate_xrc;

pub(crate) fn build_ui_handles(
    parent: Option<&dyn WxWidget>,
    _auto_destroy_root: bool,
) -> crate::UiHandles {
    maybe_validate_xrc();
    let resource = XmlResource::get();
    resource.init_all_handlers();
    resource.init_platform_aware_staticbitmap_handler();
    resource.init_sizer_handlers();
    info!("Loading XRC data.");
    resource.load_from_string(include_str!("../ui/main.xrc")).unwrap_or_else(|err| {
        show_startup_error(&format!(
            "Failed to load UI resources.\n\n{err}\n\nEnable EXCEL_DIFF_VALIDATE_XRC=1 for structural checks."
        ))
    });

    info!("Loading main frame.");
    let main_frame = resource
        .load_frame(parent, "main_frame")
        .unwrap_or_else(|| show_startup_error("Failed to load main window from UI resources."));
    if parent.is_none() {
        main_frame.set_min_size(min_window_size());
        main_frame.set_size(default_window_size());
    }
    main_frame.add_style(
        WindowStyle::MaximizeBox
            | WindowStyle::MinimizeBox
            | WindowStyle::ThickFrame
            | WindowStyle::SysMenu,
    );
    let _menu_bar = main_frame
        .get_menu_bar()
        .unwrap_or_else(|| panic!("Failed to get MenuBar from Frame"));

    let open_pair_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "open_pair_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: open_pair_menu"));
    let open_old_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "open_old_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: open_old_menu"));
    let open_new_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "open_new_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: open_new_menu"));
    let open_recent_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "open_recent_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: open_recent_menu"));
    let exit_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "exit_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: exit_menu"));
    let compare_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "compare_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: compare_menu"));
    let cancel_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "cancel_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: cancel_menu"));
    let export_audit_menu =
        MenuItem::from_xrc_name(main_frame.window_handle(), "export_audit_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: export_audit_menu"));
    let next_diff_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "next_diff_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: next_diff_menu"));
    let prev_diff_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "prev_diff_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: prev_diff_menu"));
    let copy_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "copy_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: copy_menu"));
    let find_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "find_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: find_menu"));
    let toggle_sheets_menu =
        MenuItem::from_xrc_name(main_frame.window_handle(), "toggle_sheets_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: toggle_sheets_menu"));
    let reset_layout_menu =
        MenuItem::from_xrc_name(main_frame.window_handle(), "reset_layout_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: reset_layout_menu"));
    let minimize_window_menu =
        MenuItem::from_xrc_name(main_frame.window_handle(), "minimize_window_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: minimize_window_menu"));
    let toggle_maximize_window_menu =
        MenuItem::from_xrc_name(main_frame.window_handle(), "toggle_maximize_window_menu")
            .unwrap_or_else(|| panic!("Failed to find menu item: toggle_maximize_window_menu"));
    let license_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "license_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: license_menu"));
    let docs_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "docs_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: docs_menu"));
    let about_menu = MenuItem::from_xrc_name(main_frame.window_handle(), "about_menu")
        .unwrap_or_else(|| panic!("Failed to find menu item: about_menu"));

    let status_bar = find_xrc_child::<StatusBar>(&main_frame, "status_bar");
    let main_panel = find_xrc_child::<Panel>(&main_frame, "main_panel");
    let frame_sizer = BoxSizer::builder(Orientation::Vertical).build();
    frame_sizer.add(&main_panel, 1, SizerFlag::Expand, 0);
    main_frame.set_sizer(frame_sizer, true);
    let root_tabs = find_xrc_child::<Notebook>(&main_frame, "root_tabs");
    root_tabs.set_min_size(min_root_tabs_size());
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

    let compare_container = find_xrc_child::<Panel>(&compare_page, "compare_container");
    let sheets_list = find_xrc_child::<Panel>(&compare_container, "sheets_list");
    let result_tabs = find_xrc_child::<Notebook>(&compare_container, "result_tabs");
    let compare_splitter = find_xrc_child::<SplitterWindow>(&compare_container, "compare_splitter");
    let compare_right_panel = find_xrc_child::<Panel>(&compare_container, "compare_right_panel");
    sheets_list.set_min_size(Size::new(MIN_SASH_POSITION, 240));
    compare_right_panel.set_min_size(Size::new(320, 240));
    let old_label = find_xrc_child::<StaticText>(&compare_container, "old_label");
    let old_picker = find_xrc_child::<FilePickerCtrl>(&compare_container, "old_picker");
    let old_dir_picker = find_xrc_child::<DirPickerCtrl>(&compare_container, "old_dir_picker");
    let swap_btn = find_xrc_child::<Button>(&compare_container, "swap_btn");
    let new_label = find_xrc_child::<StaticText>(&compare_container, "new_label");
    let new_picker = find_xrc_child::<FilePickerCtrl>(&compare_container, "new_picker");
    let new_dir_picker = find_xrc_child::<DirPickerCtrl>(&compare_container, "new_dir_picker");
    let compare_btn = find_xrc_child::<Button>(&compare_container, "compare_btn");
    let cancel_btn = find_xrc_child::<Button>(&compare_container, "cancel_btn");
    let compare_help_text = find_xrc_child::<StaticText>(&compare_container, "compare_help_text");
    let domain_choice = find_xrc_child::<Choice>(&compare_container, "domain_choice");
    let pbip_profile_choice = find_xrc_child::<Choice>(&compare_container, "pbip_profile_choice");
    let profile_choice = find_xrc_child::<Choice>(&compare_container, "profile_choice");
    let preset_choice = find_xrc_child::<Choice>(&compare_container, "preset_choice");
    let trusted_checkbox = find_xrc_child::<CheckBox>(&compare_container, "trusted_checkbox");
    let profiles_btn = find_xrc_child::<Button>(&compare_container, "profiles_btn");
    let progress_gauge = find_xrc_child::<Gauge>(&compare_container, "progress_gauge");
    let progress_text = find_xrc_child::<StaticText>(&compare_container, "progress_text");
    let status_pill = find_xrc_child::<Panel>(&compare_container, "status_pill");
    let summary_warning_panel =
        find_xrc_child::<Panel>(&compare_container, "summary_warning_panel");
    let summary_warning_text =
        find_xrc_child::<StaticText>(&compare_container, "summary_warning_text");
    let summary_card_added_panel =
        find_xrc_child::<Panel>(&compare_container, "summary_card_added_panel");
    let summary_added_value =
        find_xrc_child::<StaticText>(&compare_container, "summary_added_value");
    let summary_added_label =
        find_xrc_child::<StaticText>(&compare_container, "summary_added_label");
    let summary_card_removed_panel =
        find_xrc_child::<Panel>(&compare_container, "summary_card_removed_panel");
    let summary_removed_value =
        find_xrc_child::<StaticText>(&compare_container, "summary_removed_value");
    let summary_removed_label =
        find_xrc_child::<StaticText>(&compare_container, "summary_removed_label");
    let summary_card_modified_panel =
        find_xrc_child::<Panel>(&compare_container, "summary_card_modified_panel");
    let summary_modified_value =
        find_xrc_child::<StaticText>(&compare_container, "summary_modified_value");
    let summary_modified_label =
        find_xrc_child::<StaticText>(&compare_container, "summary_modified_label");
    let summary_card_moved_panel =
        find_xrc_child::<Panel>(&compare_container, "summary_card_moved_panel");
    let summary_moved_value =
        find_xrc_child::<StaticText>(&compare_container, "summary_moved_value");
    let summary_moved_label =
        find_xrc_child::<StaticText>(&compare_container, "summary_moved_label");
    let summary_categories_table_host =
        find_xrc_child::<Panel>(&compare_container, "summary_categories_table_host");
    let summary_top_sheets_table_host =
        find_xrc_child::<Panel>(&compare_container, "summary_top_sheets_table_host");
    let summary_text = find_xrc_child::<TextCtrl>(&compare_container, "summary_text");
    let detail_text = find_xrc_child::<TextCtrl>(&compare_container, "detail_text");
    let pbip_details_panel = find_xrc_child::<Panel>(&compare_container, "pbip_details_panel");
    let pbip_details_header =
        find_xrc_child::<StaticText>(&compare_container, "pbip_details_header");
    let pbip_old_label = find_xrc_child::<StaticText>(&compare_container, "pbip_old_label");
    let pbip_new_label = find_xrc_child::<StaticText>(&compare_container, "pbip_new_label");
    let pbip_old_text = find_xrc_child::<TextCtrl>(&compare_container, "pbip_old_text");
    let pbip_new_text = find_xrc_child::<TextCtrl>(&compare_container, "pbip_new_text");
    let explain_text = find_xrc_child::<TextCtrl>(&compare_container, "explain_text");
    let grid_panel = find_xrc_child::<Panel>(&compare_container, "grid_panel");
    let run_summary_header = find_xrc_child::<Panel>(&compare_container, "run_summary_header");
    let run_summary_old = find_xrc_child::<StaticText>(&compare_container, "run_summary_old");
    let run_summary_new = find_xrc_child::<StaticText>(&compare_container, "run_summary_new");
    let run_summary_meta = find_xrc_child::<StaticText>(&compare_container, "run_summary_meta");
    let sheets_filter_ctrl = find_xrc_child::<SearchCtrl>(&compare_container, "sheets_filter_ctrl");
    let noise_filters_panel =
        find_xrc_child::<Panel>(&compare_container, "noise_filters_panel");
    let hide_m_formatting_checkbox =
        find_xrc_child::<CheckBox>(&compare_container, "hide_m_formatting_checkbox");
    let hide_dax_formatting_checkbox =
        find_xrc_child::<CheckBox>(&compare_container, "hide_dax_formatting_checkbox");
    let hide_formula_formatting_checkbox =
        find_xrc_child::<CheckBox>(&compare_container, "hide_formula_formatting_checkbox");
    let collapse_moves_checkbox =
        find_xrc_child::<CheckBox>(&compare_container, "collapse_moves_checkbox");
    let sheets_filter_status =
        find_xrc_child::<StaticText>(&compare_container, "sheets_filter_status");
    let sheets_empty_panel = find_xrc_child::<Panel>(&compare_container, "sheets_empty_panel");
    let sheets_empty_text = find_xrc_child::<StaticText>(&compare_container, "sheets_empty_text");
    let sheets_table_host = find_xrc_child::<Panel>(&compare_container, "sheets_table_host");

    let recents_list = find_xrc_child::<Panel>(&recents_page, "recents_list");
    let open_recent_btn = find_xrc_child::<Button>(&recents_page, "open_recent_btn");

    let batch_old_dir = find_xrc_child::<DirPickerCtrl>(&batch_page, "batch_old_dir");
    let batch_new_dir = find_xrc_child::<DirPickerCtrl>(&batch_page, "batch_new_dir");
    let run_batch_btn = find_xrc_child::<Button>(&batch_page, "run_batch_btn");
    let include_glob_text = find_xrc_child::<TextCtrl>(&batch_page, "include_glob_text");
    let exclude_glob_text = find_xrc_child::<TextCtrl>(&batch_page, "exclude_glob_text");
    let batch_results_list = find_xrc_child::<Panel>(&batch_page, "batch_results_list");
    let include_glob_label = find_xrc_child::<StaticText>(&batch_page, "include_glob_label");
    let exclude_glob_label = find_xrc_child::<StaticText>(&batch_page, "exclude_glob_label");

    let search_ctrl = find_xrc_child::<SearchCtrl>(&search_page, "search_ctrl");
    let search_scope_choice = find_xrc_child::<Choice>(&search_page, "search_scope_choice");
    let search_btn = find_xrc_child::<Button>(&search_page, "search_btn");
    let build_old_index_btn = find_xrc_child::<Button>(&search_page, "build_old_index_btn");
    let build_new_index_btn = find_xrc_child::<Button>(&search_page, "build_new_index_btn");
    let search_results_list = find_xrc_child::<Panel>(&search_page, "search_results_list");
    debug!("XRC widgets loaded successfully.");

    let top_trim_bar = find_xrc_child::<Panel>(&main_panel, "top_trim_bar");
    let trim_accent_green = find_xrc_child::<Panel>(&top_trim_bar, "trim_accent_green");
    let trim_accent_yellow = find_xrc_child::<Panel>(&top_trim_bar, "trim_accent_yellow");

    trim_accent_green.set_background_color(theme::Palette::ACCENT_GREEN);
    trim_accent_green.set_background_style(BackgroundStyle::Colour);
    trim_accent_yellow.set_background_color(theme::Palette::ACCENT_YELLOW);
    trim_accent_yellow.set_background_style(BackgroundStyle::Colour);

    // Mostly-light theme, with dark gray trim and visible green/yellow status accents.
    theme::apply_surface(&main_panel);
    theme::apply_surface(&root_tabs);
    theme::apply_surface(&compare_page);
    theme::apply_surface(&recents_page);
    theme::apply_surface(&batch_page);
    theme::apply_surface(&search_page);
    theme::apply_surface(&compare_container);
    theme::apply_surface(&run_summary_header);
    theme::apply_surface(&sheets_list);
    theme::apply_surface(&compare_right_panel);
    theme::apply_surface(&grid_panel);
    theme::apply_surface(&recents_list);
    theme::apply_surface(&batch_results_list);
    theme::apply_surface(&search_results_list);

    theme::apply_trim(&top_trim_bar);
    theme::apply_trim(&status_bar);

    theme::apply_content_text(&summary_text, false);
    theme::apply_content_text(&detail_text, true);
    theme::apply_content_text(&pbip_old_text, true);
    theme::apply_content_text(&pbip_new_text, true);
    theme::apply_content_text(&explain_text, true);

    // Summary panel styling.
    summary_warning_panel.set_background_color(theme::Palette::ACCENT_YELLOW);
    summary_warning_panel.set_background_style(BackgroundStyle::Colour);
    summary_warning_text.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    summary_warning_panel.show(false);

    for panel in [
        &summary_card_added_panel,
        &summary_card_removed_panel,
        &summary_card_modified_panel,
        &summary_card_moved_panel,
    ] {
        panel.set_background_color(theme::Palette::CONTENT_BG);
        panel.set_background_style(BackgroundStyle::Colour);
    }

    if let Some(font) = FontBuilder::default()
        .with_point_size(16)
        .with_weight(FontWeight::Bold)
        .build()
    {
        summary_added_value.set_font(&font);
        summary_removed_value.set_font(&font);
        summary_modified_value.set_font(&font);
        summary_moved_value.set_font(&font);
    }

    summary_added_value.set_foreground_color(theme::Palette::ACCENT_GREEN);
    summary_removed_value.set_foreground_color(theme::Palette::ACCENT_RED);
    summary_modified_value.set_foreground_color(theme::Palette::ACCENT_YELLOW);
    summary_moved_value.set_foreground_color(theme::Palette::TEXT_PRIMARY);

    for label in [
        &summary_added_label,
        &summary_removed_label,
        &summary_modified_label,
        &summary_moved_label,
    ] {
        label.set_foreground_color(theme::Palette::TEXT_SECONDARY);
    }

    if let Some(font) = FontBuilder::default().with_weight(FontWeight::Bold).build() {
        old_label.set_font(&font);
        new_label.set_font(&font);
        run_summary_old.set_font(&font);
        run_summary_new.set_font(&font);
    }
    old_label.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    new_label.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    pbip_details_header.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    pbip_old_label.set_foreground_color(theme::Palette::TEXT_SECONDARY);
    pbip_new_label.set_foreground_color(theme::Palette::TEXT_SECONDARY);
    compare_help_text.set_foreground_color(theme::Palette::TEXT_SECONDARY);
    run_summary_old.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    run_summary_new.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    run_summary_meta.set_foreground_color(theme::Palette::TEXT_SECONDARY);

    let old_tip = "Old: baseline workbook (before).";
    let new_tip = "New: updated workbook (after).";
    old_label.set_tooltip(old_tip);
    old_picker.set_tooltip(old_tip);
    old_dir_picker.set_tooltip("Old: baseline folder (before).");
    new_label.set_tooltip(new_tip);
    new_picker.set_tooltip(new_tip);
    new_dir_picker.set_tooltip("New: updated folder (after).");
    swap_btn.set_tooltip("Swap Old and New paths.");

    // Default UI mode is Workbook; PBIP-specific controls are shown when the domain switches.
    old_dir_picker.show(false);
    new_dir_picker.show(false);
    pbip_profile_choice.show(false);
    pbip_details_panel.show(false);

    sheets_filter_ctrl.set_tooltip("Filter sheets by name or counts (e.g., Pivot or 12).");
    sheets_filter_ctrl.show_search_button(false);
    sheets_filter_ctrl.show_cancel_button(true);
    sheets_filter_ctrl.set_background_color(theme::Palette::CONTENT_BG);
    sheets_filter_ctrl.set_background_style(BackgroundStyle::Colour);
    sheets_filter_ctrl.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    sheets_filter_status.set_foreground_color(theme::Palette::TEXT_SECONDARY);
    sheets_empty_text.set_foreground_color(theme::Palette::TEXT_SECONDARY);

    // Make the empty + table areas read as a single content surface (white), like the other
    // DataView-backed panels.
    sheets_table_host.set_background_color(theme::Palette::CONTENT_BG);
    sheets_table_host.set_background_style(BackgroundStyle::Colour);
    sheets_empty_panel.set_background_color(theme::Palette::CONTENT_BG);
    sheets_empty_panel.set_background_style(BackgroundStyle::Colour);
    summary_categories_table_host.set_background_color(theme::Palette::CONTENT_BG);
    summary_categories_table_host.set_background_style(BackgroundStyle::Colour);
    summary_top_sheets_table_host.set_background_color(theme::Palette::CONTENT_BG);
    summary_top_sheets_table_host.set_background_style(BackgroundStyle::Colour);

    trusted_checkbox.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    profile_choice.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    profiles_btn.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    hide_m_formatting_checkbox.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    hide_dax_formatting_checkbox.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    hide_formula_formatting_checkbox.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    collapse_moves_checkbox.set_foreground_color(theme::Palette::TEXT_PRIMARY);
    include_glob_label.set_foreground_color(theme::Palette::TEXT_SECONDARY);
    exclude_glob_label.set_foreground_color(theme::Palette::TEXT_SECONDARY);

    theme::set_status_tone(
        &progress_text,
        &status_pill,
        &progress_gauge,
        theme::StatusTone::Ready,
    );

    crate::UiHandles {
        frame: main_frame,
        main_panel,
        open_pair_menu,
        open_old_menu,
        open_new_menu,
        open_recent_menu,
        exit_menu,
        compare_menu,
        cancel_menu,
        export_audit_menu,
        next_diff_menu,
        prev_diff_menu,
        copy_menu,
        find_menu,
        toggle_sheets_menu,
        reset_layout_menu,
        minimize_window_menu,
        toggle_maximize_window_menu,
        license_menu,
        docs_menu,
        about_menu,
        status_bar,
        progress_text,
        progress_gauge,
        status_pill,
        compare_btn,
        cancel_btn,
        old_picker,
        old_dir_picker,
        new_picker,
        new_dir_picker,
        swap_btn,
        compare_help_text,
        domain_choice,
        pbip_profile_choice,
        profile_choice,
        preset_choice,
        trusted_checkbox,
        profiles_btn,
        run_summary_old,
        run_summary_new,
        run_summary_meta,
        summary_warning_panel,
        summary_warning_text,
        summary_added_value,
        summary_removed_value,
        summary_modified_value,
        summary_moved_value,
        summary_categories_table_host,
        summary_top_sheets_table_host,
        summary_text,
        detail_text,
        pbip_details_panel,
        pbip_details_header,
        pbip_old_text,
        pbip_new_text,
        explain_text,
        grid_panel,
        root_tabs,
        compare_container,
        result_tabs,
        sheets_list_panel: sheets_list,
        sheets_table_host,
        sheets_filter_ctrl,
        noise_filters_panel,
        hide_m_formatting_checkbox,
        hide_dax_formatting_checkbox,
        hide_formula_formatting_checkbox,
        collapse_moves_checkbox,
        sheets_filter_status,
        sheets_empty_panel,
        sheets_empty_text,
        recents_list_panel: recents_list,
        batch_results_list_panel: batch_results_list,
        search_results_list_panel: search_results_list,
        compare_splitter,
        compare_right_panel,
        open_recent_btn,
        run_batch_btn,
        search_btn,
        build_old_index_btn,
        build_new_index_btn,
        search_ctrl,
        search_scope_choice,
        batch_old_dir,
        batch_new_dir,
        include_glob_text,
        exclude_glob_text,
        sheets_view: None,
        recents_view: None,
        batch_view: None,
        search_view: None,
        summary_categories_view: None,
        summary_top_sheets_view: None,
        webview: None,
        grid_webview: None,
        grid_fallback: None,
    }
}

fn show_startup_error(message: &str) -> ! {
    show_startup_error_with_parent(None, message)
}

fn show_startup_error_with_parent(parent: Option<&dyn WxWidget>, message: &str) -> ! {
    if let Some(parent) = parent {
        let dialog = MessageDialog::builder(parent, message, "UI startup error")
            .with_style(MessageDialogStyle::IconError | MessageDialogStyle::OK)
            .build();
        let _ = dialog.show_modal();
    } else {
        let frame = Frame::builder()
            .with_title("Tabulensis")
            .with_size(Size::new(520, 320))
            .build();
        let dialog = MessageDialog::builder(&frame, message, "UI startup error")
            .with_style(MessageDialogStyle::IconError | MessageDialogStyle::OK)
            .build();
        let _ = dialog.show_modal();
        frame.destroy();
    }
    std::process::exit(1);
}

fn find_xrc_child<T>(parent: &impl WxWidget, name: &str) -> T
where
    T: FromXrcPtr<RawFfiType = *mut ffi::wxd_Window_t> + WxWidget,
{
    let id = XmlResource::get_xrc_id(name);
    if id == 0 || id == -1 {
        show_startup_error_with_parent(
            Some(parent),
            &format!("Missing XRC id: {name}. Enable EXCEL_DIFF_VALIDATE_XRC=1 for details."),
        );
    }

    let child_ptr = unsafe { ffi::wxd_Window_FindWindowById(parent.handle_ptr(), id) };
    if child_ptr.is_null() {
        show_startup_error_with_parent(
            Some(parent),
            &format!(
                "Missing XRC widget: {name}. Check widget names in the XRC and run with EXCEL_DIFF_VALIDATE_XRC=1."
            ),
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
        if let Err(err) = validate_xrc(include_str!("../ui/main.xrc")) {
            panic!("XRC validation failed:\n{err}");
        }
    }
}
