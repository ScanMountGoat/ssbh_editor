use std::path::Path;

use crate::{
    app::shortcut::{format_shortcut, ADD_FOLDER, OPEN_FOLDER, RELOAD_SHORTCUT},
    CameraState,
};

use super::{RenderAction, RenderModelAction, SsbhApp};
use egui::{special_emojis::GITHUB, Button, TextWrapMode, Ui};
use rfd::FileDialog;

pub fn menu_bar(app: &mut SsbhApp, ui: &mut Ui) {
    // Shortcuts need to be handled even while the menu is not open.
    if ui.input_mut(|i| i.consume_shortcut(&OPEN_FOLDER)) {
        app.add_folder_to_workspace_from_dialog(true);
    }

    if ui.input_mut(|i| i.consume_shortcut(&ADD_FOLDER)) {
        app.add_folder_to_workspace_from_dialog(false)
    }

    if ui.input_mut(|i| i.consume_shortcut(&RELOAD_SHORTCUT)) {
        app.reload_workspace();
    }

    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            let button =
                |ui: &mut Ui, text: &str| ui.add(Button::new(text).wrap_mode(TextWrapMode::Extend));
            let shortcut_button = |ui: &mut Ui, text: &str, shortcut| {
                ui.add(
                    Button::new(text)
                        .wrap_mode(TextWrapMode::Extend)
                        .shortcut_text(format_shortcut(shortcut)),
                )
            };

            if shortcut_button(ui, "🗀 Open Folder...", &OPEN_FOLDER).clicked() {
                if let Some(folder) = FileDialog::new().pick_folder() {
                    app.add_folder_to_workspace(folder, true);
                }
            }

            // TODO: Find a cleaner way to write this.
            let mut recent = None;
            ui.menu_button("Open Recent Folder", |ui| {
                for folder in &app.preferences.recent_folders {
                    if button(ui, folder).clicked() {
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

            if shortcut_button(ui, "🗀 Add Folder to Workspace...", &ADD_FOLDER).clicked() {
                if let Some(folder) = FileDialog::new().pick_folder() {
                    app.add_folder_to_workspace(folder, false);
                }
            }

            // TODO: Find a cleaner way to write this.
            let mut recent = None;
            ui.menu_button("Add Recent Folder to Workspace", |ui| {
                for folder in &app.preferences.recent_folders {
                    if button(ui, folder).clicked() {
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

            if shortcut_button(ui, "Reload Workspace", &RELOAD_SHORTCUT).clicked() {
                app.reload_workspace();
            }

            if button(ui, "Clear Workspace").clicked() {
                app.clear_workspace();
            }
        });

        // TODO: Add icons?
        ui.menu_button("Menu", |ui| {
            if ui.button("Render Settings").clicked() {
                app.ui_state.render_settings_open = true;
            }

            if ui.button("Stage Lighting").clicked() {
                app.ui_state.stage_lighting_open = true;
            }

            if ui.button("Material Presets").clicked() {
                app.ui_state.preset_editor_open = true;
            }

            if ui.button("⛭ Preferences").clicked() {
                app.ui_state.preferences_window_open = true;
            }

            if ui.button("Device Info").clicked() {
                app.ui_state.device_info_window_open = true;
            }
        });

        ui.menu_button("Viewport", |ui| {
            if ui.button("Camera Settings").clicked() {
                app.ui_state.camera_settings_open = true;
            }
            if ui.button("Reset Camera").clicked() {
                app.camera_state = CameraState {
                    values: app.preferences.default_camera.clone(),
                    ..Default::default()
                };
                app.render_actions.push_back(RenderAction::UpdateCamera);
            }
            ui.separator();

            if ui.button("Save Screenshot...").clicked() {
                if let Some(file) = FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "tif", "bmp"])
                    .save_file()
                {
                    app.screenshot_to_render = Some(file);
                }
            }

            ui.menu_button("Render Animation", |ui| {
                if ui
                    .add(Button::new("Render to Image Sequence...").wrap_mode(TextWrapMode::Extend))
                    .clicked()
                {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "tif", "bmp"])
                        .save_file()
                    {
                        app.animation_image_sequence_to_render = Some(file);
                    }
                }

                if ui
                    .add(Button::new("Render to GIF...").wrap_mode(TextWrapMode::Extend))
                    .clicked()
                {
                    if let Some(file) = FileDialog::new().add_filter("GIF", &["gif"]).save_file() {
                        app.animation_gif_to_render = Some(file);
                    }
                }
            });
        });

        ui.menu_button("Meshes", |ui| {
            if ui.button("Show All").clicked() {
                app.render_actions
                    .push_back(RenderAction::Model(RenderModelAction::ShowAll));
            }

            if ui.button("Hide All").clicked() {
                app.render_actions
                    .push_back(RenderAction::Model(RenderModelAction::HideAll));
            }

            if ui.button("Hide Expressions").clicked() {
                app.render_actions
                    .push_back(RenderAction::Model(RenderModelAction::HideExpressions));
            }

            if ui.button("Hide Ink Meshes").clicked() {
                app.render_actions
                    .push_back(RenderAction::Model(RenderModelAction::HideInkMeshes));
            }
            ui.separator();

            if ui.button("Expand All").clicked() {
                for folder in &mut app.models {
                    folder.is_meshlist_open = true;
                }
            }

            if ui.button("Collapse All").clicked() {
                for folder in &mut app.models {
                    folder.is_meshlist_open = false;
                }
            }
        });

        ui.menu_button("View", |ui| {
            ui.checkbox(&mut app.show_left_panel, "Left Panel");
            ui.checkbox(&mut app.show_right_panel, "Right Panel");
            ui.checkbox(&mut app.show_bottom_panel, "Bottom Panel");
        });

        ui.menu_button("Help", |ui| {
            if ui.button(format!("{GITHUB} Wiki")).clicked() {
                let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }

            if ui.button(format!("{GITHUB} Discussion Forum")).clicked() {
                let link = "https://github.com/ScanMountGoat/ssbh_editor/discussions";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }

            if ui.button(format!("{GITHUB} Report Issue")).clicked() {
                let link = "https://github.com/ScanMountGoat/ssbh_editor/issues";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }

            if ui.button(format!("{GITHUB} Changelog")).clicked() {
                let link = "https://github.com/ScanMountGoat/ssbh_editor/blob/main/CHANGELOG.md";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }
        });
    });
}
