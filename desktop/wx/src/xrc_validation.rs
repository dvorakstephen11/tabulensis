use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::{HashMap, HashSet};

const REQUIRED_WIDGETS: &[&str] = &[
    "main_frame",
    "main_panel",
    "root_tabs",
    "old_label",
    "old_picker",
    "swap_btn",
    "new_label",
    "new_picker",
    "compare_btn",
    "cancel_btn",
    "compare_help_text",
    "profile_choice",
    "preset_choice",
    "trusted_checkbox",
    "profiles_btn",
    "progress_gauge",
    "progress_text",
    "run_summary_header",
    "run_summary_old",
    "run_summary_new",
    "run_summary_meta",
    "compare_container",
    "compare_splitter",
    "compare_right_panel",
    "sheets_list",
    "sheets_filter_ctrl",
    "hide_m_formatting_checkbox",
    "hide_dax_formatting_checkbox",
    "hide_formula_formatting_checkbox",
    "collapse_moves_checkbox",
    "sheets_filter_status",
    "sheets_empty_panel",
    "sheets_empty_text",
    "sheets_table_host",
    "result_tabs",
    "summary_warning_panel",
    "summary_warning_text",
    "summary_card_added_panel",
    "summary_added_value",
    "summary_added_label",
    "summary_card_removed_panel",
    "summary_removed_value",
    "summary_removed_label",
    "summary_card_modified_panel",
    "summary_modified_value",
    "summary_modified_label",
    "summary_card_moved_panel",
    "summary_moved_value",
    "summary_moved_label",
    "summary_categories_table_host",
    "summary_top_sheets_table_host",
    "summary_text",
    "detail_text",
    "explain_text",
    "grid_panel",
    "recents_list",
    "open_recent_btn",
    "batch_old_dir",
    "batch_new_dir",
    "run_batch_btn",
    "include_glob_text",
    "exclude_glob_text",
    "batch_results_list",
    "search_ctrl",
    "search_scope_choice",
    "search_btn",
    "build_old_index_btn",
    "build_new_index_btn",
    "search_results_list",
];

const ROOT_TAB_LABELS: &[&str] = &["Compare", "Recents", "Batch", "Search"];
const RESULT_TAB_LABELS: &[&str] = &["Summary", "Details", "Explain", "Grid"];

pub fn validate_xrc(xrc: &str) -> Result<(), String> {
    let mut reader = Reader::from_str(xrc);
    reader.config_mut().trim_text(true);

    let mut errors = Vec::new();
    let mut names = HashSet::new();
    let mut stack: Vec<ObjectFrame> = Vec::new();
    let mut sizer_stack: Vec<SizerItemFrame> = Vec::new();
    let mut sizer_items: HashMap<String, Vec<SizerItemInfo>> = HashMap::new();
    let mut orient_target: Option<usize> = None;
    let mut label_target: Option<usize> = None;
    let mut flag_target: Option<usize> = None;
    let mut proportion_target: Option<usize> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if tag == "object" {
                    let mut class = None;
                    let mut name = None;
                    for attr in event.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"class" => {
                                class = Some(String::from_utf8_lossy(&attr.value).to_string())
                            }
                            b"name" => {
                                name = Some(String::from_utf8_lossy(&attr.value).to_string())
                            }
                            _ => {}
                        }
                    }

                    if let Some(name) = name.as_ref() {
                        names.insert(name.clone());
                    }

                    let class_name = class.unwrap_or_else(|| "".to_string());
                    let mut frame = ObjectFrame::new(class_name, name);
                    let frame_name = frame.name.clone();

                    if frame.class == "sizeritem" {
                        sizer_stack.push(SizerItemFrame::default());
                    }

                    if frame.class == "notebookpage" {
                        if let Some(parent) = nearest_notebook_mut(&mut stack) {
                            parent.page_count += 1;
                        }
                        frame.is_notebookpage = true;
                    }

                    stack.push(frame);
                    if let Some(sizer_frame) = sizer_stack.last_mut() {
                        if !sizer_frame.saw_object
                            && !matches!(stack.last().map(|f| f.class.as_str()), Some("sizeritem"))
                        {
                            sizer_frame.saw_object = true;
                            if frame_name.is_some() {
                                sizer_frame.child_name = frame_name;
                            }
                        }
                    }
                } else if tag == "sizeritem" {
                    sizer_stack.push(SizerItemFrame::default());
                } else if tag == "orient" {
                    orient_target = nearest_boxsizer_index(&stack);
                } else if tag == "label" {
                    if let Some(frame) = stack.last() {
                        if frame.is_notebookpage && frame.notebookpage_label.is_none() {
                            label_target = Some(stack.len() - 1);
                        }
                    }
                } else if tag == "flag" {
                    if !sizer_stack.is_empty() {
                        flag_target = Some(sizer_stack.len() - 1);
                    }
                } else if tag == "proportion" || tag == "option" {
                    if !sizer_stack.is_empty() {
                        proportion_target = Some(sizer_stack.len() - 1);
                    }
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(index) = orient_target {
                    if let Ok(text) = event.unescape() {
                        let text = text.trim();
                        if text == "wxVERTICAL" || text == "wxHORIZONTAL" {
                            if let Some(frame) = stack.get_mut(index) {
                                frame.boxsizer_orient_ok = true;
                            }
                        }
                    }
                }

                if let Some(index) = label_target {
                    if let Ok(text) = event.unescape() {
                        let text = text.trim();
                        if !text.is_empty() {
                            if let Some(frame) = stack.get_mut(index) {
                                frame.notebookpage_label = Some(text.to_string());
                            }
                        }
                    }
                }

                if let Some(index) = flag_target {
                    if let Ok(text) = event.unescape() {
                        let text = text.trim();
                        if let Some(frame) = sizer_stack.get_mut(index) {
                            if !text.is_empty() {
                                frame.flags = Some(text.to_string());
                            }
                        }
                    }
                }

                if let Some(index) = proportion_target {
                    if let Ok(text) = event.unescape() {
                        let text = text.trim();
                        if let Ok(value) = text.parse::<i32>() {
                            if let Some(frame) = sizer_stack.get_mut(index) {
                                frame.proportion = Some(value);
                            }
                        }
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if tag == "orient" {
                    orient_target = None;
                } else if tag == "label" {
                    label_target = None;
                } else if tag == "flag" {
                    flag_target = None;
                } else if tag == "proportion" || tag == "option" {
                    proportion_target = None;
                } else if tag == "sizeritem" {
                    if let Some(frame) = sizer_stack.pop() {
                        if let Some(name) = frame.child_name {
                            let info = SizerItemInfo {
                                flags: frame.flags.unwrap_or_default(),
                                proportion: frame.proportion.unwrap_or(0),
                            };
                            sizer_items.entry(name).or_default().push(info);
                        }
                    }
                } else if tag == "object" {
                    if let Some(frame) = stack.pop() {
                        if frame.class == "sizeritem" {
                            if let Some(frame) = sizer_stack.pop() {
                                if let Some(name) = frame.child_name {
                                    let info = SizerItemInfo {
                                        flags: frame.flags.unwrap_or_default(),
                                        proportion: frame.proportion.unwrap_or(0),
                                    };
                                    sizer_items.entry(name).or_default().push(info);
                                }
                            }
                        }
                        if frame.class == "wxPanel" {
                            if let Some(parent) = nearest_notebookpage_mut(&mut stack) {
                                parent.notebookpage_has_panel = true;
                            }
                        }

                        if frame.class == "wxBoxSizer" && !frame.boxsizer_orient_ok {
                            errors.push("wxBoxSizer missing valid <orient> value.".to_string());
                        }

                        if frame.is_notebookpage && !frame.notebookpage_has_panel {
                            if let Some(parent) = nearest_notebook_mut(&mut stack) {
                                parent.pages_missing_panel += 1;
                            }
                        }

                        if frame.is_notebookpage && frame.notebookpage_label.is_none() {
                            errors.push("notebookpage missing label.".to_string());
                        }

                        if frame.is_notebookpage {
                            if let Some(label) = frame.notebookpage_label {
                                if let Some(parent) = nearest_notebook_mut(&mut stack) {
                                    parent.page_labels.push(label);
                                }
                            }
                        }

                        if frame.is_notebook && frame.page_count == 0 {
                            if let Some(name) = frame.name.as_ref() {
                                errors.push(format!(
                                    "wxNotebook {name} has no notebookpage children."
                                ));
                            }
                        }

                        if frame.is_notebook && frame.pages_missing_panel > 0 {
                            if let Some(name) = frame.name.as_ref() {
                                errors.push(format!(
                                    "wxNotebook {name} has notebookpage entries without wxPanel children."
                                ));
                            }
                        }

                        if frame.is_notebook {
                            if let Some(name) = frame.name.as_deref() {
                                if let Some(expected) = expected_labels_for(name) {
                                    if frame.page_labels != expected {
                                        errors.push(format!(
                                            "wxNotebook {name} labels mismatch. Expected: [{}] Found: [{}].",
                                            expected.join(", "),
                                            frame.page_labels.join(", ")
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(format!("XRC parse error: {err}")),
            _ => {}
        }
    }

    if !names.contains("main_frame") {
        errors.push("Missing root wxFrame named main_frame.".to_string());
    }

    for name in REQUIRED_WIDGETS {
        if !names.contains(*name) {
            errors.push(format!("Missing widget name: {name}."));
        }
    }

    for (name, min_prop, needs_expand) in expand_requirements() {
        if !sizer_item_ok(&sizer_items, name, min_prop, needs_expand) {
            let expand_note = if needs_expand { " and wxEXPAND" } else { "" };
            errors.push(format!(
                "Widget {name} must be in sizeritem with proportion >= {min_prop}{expand_note}."
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[derive(Debug)]
struct ObjectFrame {
    class: String,
    name: Option<String>,
    boxsizer_orient_ok: bool,
    is_notebook: bool,
    page_count: usize,
    pages_missing_panel: usize,
    is_notebookpage: bool,
    notebookpage_has_panel: bool,
    notebookpage_label: Option<String>,
    page_labels: Vec<String>,
}

impl ObjectFrame {
    fn new(class: String, name: Option<String>) -> Self {
        let is_notebook = class == "wxNotebook"
            && matches!(name.as_deref(), Some("root_tabs") | Some("result_tabs"));
        Self {
            class,
            name,
            boxsizer_orient_ok: false,
            is_notebook,
            page_count: 0,
            pages_missing_panel: 0,
            is_notebookpage: false,
            notebookpage_has_panel: false,
            notebookpage_label: None,
            page_labels: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
struct SizerItemFrame {
    child_name: Option<String>,
    flags: Option<String>,
    proportion: Option<i32>,
    saw_object: bool,
}

#[derive(Debug)]
struct SizerItemInfo {
    flags: String,
    proportion: i32,
}

fn nearest_boxsizer_index(stack: &[ObjectFrame]) -> Option<usize> {
    stack.iter().rposition(|frame| frame.class == "wxBoxSizer")
}

fn nearest_notebook_mut(stack: &mut [ObjectFrame]) -> Option<&mut ObjectFrame> {
    stack.iter_mut().rev().find(|frame| frame.is_notebook)
}

fn nearest_notebookpage_mut(stack: &mut [ObjectFrame]) -> Option<&mut ObjectFrame> {
    stack.iter_mut().rev().find(|frame| frame.is_notebookpage)
}

fn expected_labels_for(name: &str) -> Option<Vec<String>> {
    match name {
        "root_tabs" => Some(
            ROOT_TAB_LABELS
                .iter()
                .map(|label| label.to_string())
                .collect(),
        ),
        "result_tabs" => Some(
            RESULT_TAB_LABELS
                .iter()
                .map(|label| label.to_string())
                .collect(),
        ),
        _ => None,
    }
}

fn expand_requirements() -> Vec<(&'static str, i32, bool)> {
    vec![
        ("root_tabs", 1, true),
        ("compare_splitter", 1, true),
        ("result_tabs", 1, true),
        ("old_picker", 1, true),
        ("new_picker", 1, true),
    ]
}

fn sizer_item_ok(
    items: &HashMap<String, Vec<SizerItemInfo>>,
    name: &str,
    min_prop: i32,
    needs_expand: bool,
) -> bool {
    items.get(name).into_iter().flatten().any(|info| {
        let prop_ok = info.proportion >= min_prop;
        let expand_ok = !needs_expand || info.flags.contains("wxEXPAND");
        prop_ok && expand_ok
    })
}

#[cfg(test)]
mod tests {
    use super::validate_xrc;

    #[test]
    fn xrc_is_structurally_valid() {
        let xrc = include_str!("../ui/main.xrc");
        validate_xrc(xrc).expect("XRC validation failed");
    }
}
