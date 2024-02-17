use std::path::Path;

use crate::CameraState;

use super::{RenderModelAction, SsbhApp};
use egui::{special_emojis::GITHUB, Button, KeyboardShortcut, Ui};
use rfd::FileDialog;

pub fn menu_bar(app: &mut SsbhApp, ui: &mut Ui) {
    let open_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::O);
    let add_shortcut = egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::O,
    );
    let reload_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::R);

    // Shortcuts need to be handled even while the menu is not open.
    if ui.input_mut(|i| i.consume_shortcut(&open_shortcut)) {
        app.add_folder_to_workspace_from_dialog(true);
    }

    if ui.input_mut(|i| i.consume_shortcut(&add_shortcut)) {
        app.add_folder_to_workspace_from_dialog(false)
    }

    if ui.input_mut(|i| i.consume_shortcut(&reload_shortcut)) {
        app.reload_workspace();
    }

    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            let button = |ui: &mut Ui, text: &str| ui.add(Button::new(text).wrap(false));
            let shortcut_button = |ui: &mut Ui, text: &str, shortcut| {
                ui.add(
                    Button::new(text)
                        .wrap(false)
                        .shortcut_text(format_shortcut(shortcut)),
                )
            };

            if shortcut_button(ui, "🗀 Open Folder...", &open_shortcut).clicked() {
                ui.close_menu();
                if let Some(folder) = FileDialog::new().pick_folder() {
                    app.add_folder_to_workspace(folder, true);
                }
            }

            // TODO: Find a cleaner way to write this.
            let mut recent = None;
            ui.menu_button("Open Recent Folder", |ui| {
                for folder in &app.preferences.recent_folders {
                    if button(ui, folder).clicked() {
                        ui.close_menu();
                        recent = Some(folder.clone());
                    }
                }
                ui.separator();
                if ui.button("Clear Recently Opened").clicked() {
                    app.preferences.recent_folders.clear();
                }
            });
            if let Some(recent) = recent {
                app.add_folder_to_workspace(Path::new(&recent), true);
            }
            ui.separator();

            if shortcut_button(ui, "🗀 Add Folder to Workspace...", &add_shortcut).clicked() {
                ui.close_menu();
                if let Some(folder) = FileDialog::new().pick_folder() {
                    app.add_folder_to_workspace(folder, false);
                }
            }

            // TODO: Find a cleaner way to write this.
            let mut recent = None;
            ui.menu_button("Add Recent Folder to Workspace", |ui| {
                for folder in &app.preferences.recent_folders {
                    if button(ui, folder).clicked() {
                        ui.close_menu();
                        recent = Some(folder.clone());
                    }
                }
                ui.separator();
                if ui.button("Clear Recently Opened").clicked() {
                    app.preferences.recent_folders.clear();
                }
            });
            if let Some(recent) = recent {
                app.add_folder_to_workspace(Path::new(&recent), false);
            }
            ui.separator();

            if shortcut_button(ui, "Reload Workspace", &reload_shortcut).clicked() {
                ui.close_menu();
                app.reload_workspace();
            }

            if button(ui, "Clear Workspace").clicked() {
                ui.close_menu();
                app.clear_workspace();
            }
        });

        // TODO: Add icons?
        ui.menu_button("Menu", |ui| {
            if ui.button("Render Settings").clicked() {
                ui.close_menu();
                app.ui_state.render_settings_open = true;
            }

            if ui.button("Stage Lighting").clicked() {
                ui.close_menu();
                app.ui_state.stage_lighting_open = true;
            }

            if ui.button("Material Presets").clicked() {
                ui.close_menu();
                app.ui_state.preset_editor_open = true;
            }

            if ui.button("⛭ Preferences").clicked() {
                ui.close_menu();
                app.ui_state.preferences_window_open = true;
            }

            if ui.button("Device Info").clicked() {
                ui.close_menu();
                app.ui_state.device_info_window_open = true;
            }
        });

        ui.menu_button("Viewport", |ui| {
            if ui.button("Camera Settings").clicked() {
                ui.close_menu();
                app.ui_state.camera_settings_open = true;
            }
            if ui.button("Reset Camera").clicked() {
                ui.close_menu();
                app.camera_state = CameraState {
                    values: app.preferences.default_camera.clone(),
                    ..Default::default()
                };
                app.should_update_camera = true;
            }
            ui.separator();

            if ui.button("Save Screenshot...").clicked() {
                ui.close_menu();
                if let Some(file) = FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "tif", "bmp"])
                    .save_file()
                {
                    app.screenshot_to_render = Some(file);
                }
            }

            ui.menu_button("Render Animation", |ui| {
                if ui
                    .add(Button::new("Render to Image Sequence...").wrap(false))
                    .clicked()
                {
                    ui.close_menu();
                    if let Some(file) = FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "tif", "bmp"])
                        .save_file()
                    {
                        app.animation_image_sequence_to_render = Some(file);
                    }
                }

                if ui
                    .add(Button::new("Render to GIF...").wrap(false))
                    .clicked()
                {
                    ui.close_menu();
                    if let Some(file) = FileDialog::new().add_filter("GIF", &["gif"]).save_file() {
                        app.animation_gif_to_render = Some(file);
                    }
                }
            });
        });

        ui.menu_button("Meshes", |ui| {
            if ui.button("Show All").clicked() {
                ui.close_menu();
                app.render_model_actions
                    .push_back(RenderModelAction::ShowAll);
            }

            if ui.button("Hide All").clicked() {
                ui.close_menu();
                app.render_model_actions
                    .push_back(RenderModelAction::HideAll);
            }

            if ui.button("Hide Expressions").clicked() {
                ui.close_menu();
                app.render_model_actions
                    .push_back(RenderModelAction::HideExpressions);
            }
        });

        ui.menu_button("View", |ui| {
            ui.checkbox(&mut app.show_left_panel, "Left Panel");
            ui.checkbox(&mut app.show_right_panel, "Right Panel");
            ui.checkbox(&mut app.show_bottom_panel, "Bottom Panel");
        });

        ui.menu_button("Help", |ui| {
            if ui.button(format!("{GITHUB} Wiki")).clicked() {
                ui.close_menu();
                let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }

            if ui.button(format!("{GITHUB} Discussion Forum")).clicked() {
                ui.close_menu();
                let link = "https://github.com/ScanMountGoat/ssbh_editor/discussions";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }

            if ui.button(format!("{GITHUB} Report Issue")).clicked() {
                ui.close_menu();
                let link = "https://github.com/ScanMountGoat/ssbh_editor/issues";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }

            if ui.button(format!("{GITHUB} Changelog")).clicked() {
                ui.close_menu();
                let link = "https://github.com/ScanMountGoat/ssbh_editor/blob/main/CHANGELOG.md";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }
        });
    });
}

fn format_shortcut(shortcut: &KeyboardShortcut) -> String {
    // egui has this method but doesn't format shortcut symbols.
    // TODO: This function might not be needed on newer versions.
    // TODO: Store keyboard shortcuts in a single place?
    let ctrl = if cfg!(target_os = "macos") {
        "⌘ "
    } else {
        "Ctrl+"
    };

    let ctrl_shift = if cfg!(target_os = "macos") {
        "⇧ ⌘ "
    } else {
        "Ctrl+Shift+"
    };

    let key = shortcut.logical_key.name();
    if shortcut.modifiers.command {
        if shortcut.modifiers.shift {
            format!("{ctrl_shift}{key}")
        } else {
            format!("{ctrl}{key}")
        }
    } else {
        key.to_owned()
    }
}
