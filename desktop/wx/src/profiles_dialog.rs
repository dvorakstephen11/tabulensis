use std::collections::HashSet;

use wxdragon::prelude::*;

use crate::profiles::CompareProfile;

pub(crate) fn show_profiles_dialog() {
    let mut action: Option<i32> = None;
    let _ = crate::with_ui_context(|ctx| {
        let actions = [
            "Save current as...",
            "Rename selected...",
            "Delete selected",
            "Export selected...",
            "Import profile...",
        ];
        let dialog = SingleChoiceDialog::builder(
            &ctx.ui.frame,
            "Choose a profiles action:",
            "Profiles",
            &actions,
        )
        .build();
        if dialog.show_modal() != ID_OK {
            return;
        }
        action = Some(dialog.get_selection());
    });

    let Some(action) = action else {
        return;
    };

    match action {
        0 => {
            // Save current as...
            let mut name: Option<String> = None;
            let _ = crate::with_ui_context(|ctx| {
                let dialog = TextEntryDialog::builder(
                    &ctx.ui.frame,
                    "Name the new profile:",
                    "Save Profile",
                )
                .build();
                if dialog.show_modal() != ID_OK {
                    return;
                }
                name = dialog
                    .get_value()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty());
            });
            let Some(name) = name else {
                return;
            };

            let _ = crate::with_ui_context(|ctx| {
                let base = ctx
                    .state
                    .selected_profile_id
                    .clone()
                    .and_then(|id| crate::profile_by_id_in_ctx(ctx, &id))
                    .unwrap_or_else(|| crate::profiles::builtin_profiles()[0].clone());

                let profile = CompareProfile {
                    id: crate::profiles::make_profile_id(&name),
                    name,
                    built_in: false,
                    preset: crate::preset_from_choice(&ctx.ui.preset_choice),
                    trusted_files: ctx.ui.trusted_checkbox.is_checked(),
                    noise_filters: crate::noise_filters_from_ui(ctx),
                    ..base
                };

                ctx.state.user_profiles.push(profile.clone());
                crate::profiles::save_user_profiles(
                    &ctx.state.profiles_path,
                    &ctx.state.user_profiles,
                );
                crate::rebuild_profile_choice_in_ctx(ctx, Some(&profile.id));
                crate::update_status_in_ctx(ctx, "Profile saved.");
            });
        }
        1 => {
            // Rename selected...
            let mut current: Option<CompareProfile> = None;
            let _ = crate::with_ui_context(|ctx| {
                current = ctx
                    .state
                    .selected_profile_id
                    .as_deref()
                    .and_then(|id| crate::profile_by_id_in_ctx(ctx, id));
            });
            let Some(current) = current else {
                return;
            };
            if current.built_in {
                let _ = crate::with_ui_context(|ctx| {
                    let dialog = MessageDialog::builder(
                        &ctx.ui.frame,
                        "Built-in profiles cannot be renamed.",
                        "Profiles",
                    )
                    .with_style(MessageDialogStyle::IconInformation | MessageDialogStyle::OK)
                    .build();
                    let _ = dialog.show_modal();
                });
                return;
            }

            let mut new_name: Option<String> = None;
            let _ = crate::with_ui_context(|ctx| {
                let dialog =
                    TextEntryDialog::builder(&ctx.ui.frame, "New profile name:", "Rename Profile")
                        .build();
                if dialog.show_modal() != ID_OK {
                    return;
                }
                new_name = dialog
                    .get_value()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty());
            });
            let Some(new_name) = new_name else {
                return;
            };

            let _ = crate::with_ui_context(|ctx| {
                let mut renamed_id: Option<String> = None;
                if let Some(entry) = ctx
                    .state
                    .user_profiles
                    .iter_mut()
                    .find(|p| p.id == current.id)
                {
                    entry.name = new_name;
                    renamed_id = Some(entry.id.clone());
                }
                if let Some(id) = renamed_id {
                    crate::profiles::save_user_profiles(
                        &ctx.state.profiles_path,
                        &ctx.state.user_profiles,
                    );
                    crate::rebuild_profile_choice_in_ctx(ctx, Some(&id));
                    crate::update_status_in_ctx(ctx, "Profile renamed.");
                }
            });
        }
        2 => {
            // Delete selected
            let mut current: Option<CompareProfile> = None;
            let _ = crate::with_ui_context(|ctx| {
                current = ctx
                    .state
                    .selected_profile_id
                    .as_deref()
                    .and_then(|id| crate::profile_by_id_in_ctx(ctx, id));
            });
            let Some(current) = current else {
                return;
            };
            if current.built_in {
                let _ = crate::with_ui_context(|ctx| {
                    let dialog = MessageDialog::builder(
                        &ctx.ui.frame,
                        "Built-in profiles cannot be deleted.",
                        "Profiles",
                    )
                    .with_style(MessageDialogStyle::IconInformation | MessageDialogStyle::OK)
                    .build();
                    let _ = dialog.show_modal();
                });
                return;
            }

            let mut confirmed = false;
            let _ = crate::with_ui_context(|ctx| {
                let message = format!("Delete profile '{}'?", current.name);
                let dialog = MessageDialog::builder(&ctx.ui.frame, &message, "Delete Profile")
                    .with_style(MessageDialogStyle::IconWarning | MessageDialogStyle::YesNo)
                    .build();
                confirmed = dialog.show_modal() == ID_YES;
            });
            if !confirmed {
                return;
            }

            let _ = crate::with_ui_context(|ctx| {
                ctx.state.user_profiles.retain(|p| p.id != current.id);
                crate::profiles::save_user_profiles(
                    &ctx.state.profiles_path,
                    &ctx.state.user_profiles,
                );
                crate::rebuild_profile_choice_in_ctx(ctx, Some("builtin_default_balanced"));
                crate::update_status_in_ctx(ctx, "Profile deleted.");
            });
        }
        3 => {
            // Export selected...
            let mut current: Option<CompareProfile> = None;
            let _ = crate::with_ui_context(|ctx| {
                current = ctx
                    .state
                    .selected_profile_id
                    .as_deref()
                    .and_then(|id| crate::profile_by_id_in_ctx(ctx, id));
            });
            let Some(current) = current else {
                return;
            };

            let mut path: Option<String> = None;
            let _ = crate::with_ui_context(|ctx| {
                let default_name = format!("{}.json", current.name.replace('/', "_"));
                let dialog = FileDialog::builder(&ctx.ui.frame)
                    .with_message("Export profile JSON")
                    .with_wildcard("JSON files (*.json)|*.json|All files (*.*)|*.*")
                    .with_default_file(&default_name)
                    .with_style(FileDialogStyle::Save | FileDialogStyle::OverwritePrompt)
                    .build();
                if dialog.show_modal() != ID_OK {
                    return;
                }
                path = crate::dialog_selected_path(&dialog);
            });
            let Some(path) = path else {
                return;
            };

            let json = serde_json::to_string_pretty(&current).unwrap_or_else(|_| "{}".to_string());
            if std::fs::write(&path, json).is_ok() {
                let _ = crate::with_ui_context(|ctx| {
                    crate::update_status_in_ctx(ctx, "Profile exported.")
                });
            }
        }
        4 => {
            // Import profile...
            let mut path: Option<String> = None;
            let _ = crate::with_ui_context(|ctx| {
                let dialog = FileDialog::builder(&ctx.ui.frame)
                    .with_message("Import profile JSON")
                    .with_wildcard("JSON files (*.json)|*.json|All files (*.*)|*.*")
                    .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
                    .build();
                if dialog.show_modal() != ID_OK {
                    return;
                }
                path = crate::dialog_selected_path(&dialog);
            });
            let Some(path) = path else {
                return;
            };

            let Ok(contents) = std::fs::read_to_string(&path) else {
                let _ = crate::with_ui_context(|ctx| {
                    crate::update_status_in_ctx(ctx, "Failed to read profile file.");
                });
                return;
            };
            let mut imported: CompareProfile = match serde_json::from_str(&contents) {
                Ok(p) => p,
                Err(_) => {
                    let _ = crate::with_ui_context(|ctx| {
                        crate::update_status_in_ctx(ctx, "Invalid profile JSON.")
                    });
                    return;
                }
            };
            imported.built_in = false;
            if imported.id.trim().is_empty() {
                imported.id = crate::profiles::make_profile_id(&imported.name);
            }

            let _ = crate::with_ui_context(|ctx| {
                let existing_ids: HashSet<String> = crate::available_profiles_in_ctx(ctx)
                    .into_iter()
                    .map(|p| p.id)
                    .collect();
                if existing_ids.contains(&imported.id) {
                    imported.id = crate::profiles::make_profile_id(&imported.name);
                }
                ctx.state.user_profiles.push(imported.clone());
                crate::profiles::save_user_profiles(
                    &ctx.state.profiles_path,
                    &ctx.state.user_profiles,
                );
                crate::rebuild_profile_choice_in_ctx(ctx, Some(&imported.id));
                crate::update_status_in_ctx(ctx, "Profile imported.");
            });
        }
        _ => {}
    }
}
