use super::{log_level_icon, StageLightingState, LOGGER};
use crate::{path::application_dir, preferences::AppPreferences, CameraInputState};
use egui::{Context, Grid, Label, ScrollArea, Ui, Window};
use rfd::FileDialog;
use std::path::PathBuf;

mod render_settings;
pub use render_settings::render_settings_window;

pub fn camera_settings_window(
    ctx: &egui::Context,
    open: &mut bool,
    camera_state: &mut CameraInputState,
) {
    egui::Window::new("Camera Settings")
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            egui::Grid::new("camera_grid").show(ui, |ui| {
                ui.label("Translation X");
                ui.add(egui::DragValue::new(&mut camera_state.translation_xyz.x));
                ui.end_row();

                ui.label("Translation Y");
                ui.add(egui::DragValue::new(&mut camera_state.translation_xyz.y));
                ui.end_row();

                ui.label("Translation Z");
                ui.add(egui::DragValue::new(&mut camera_state.translation_xyz.z));
                ui.end_row();

                // TODO: This will need to use quaternions to work with camera anims.
                // TODO: Add an option for radians or degrees?
                ui.label("Rotation X");
                let mut rotation_x_degrees = camera_state.rotation_xyz_radians.x.to_degrees();
                if ui
                    .add(egui::DragValue::new(&mut rotation_x_degrees).speed(1.0))
                    .changed()
                {
                    camera_state.rotation_xyz_radians.x = rotation_x_degrees.to_radians();
                }
                ui.end_row();

                ui.label("Rotation Y");
                let mut rotation_y_degrees = camera_state.rotation_xyz_radians.y.to_degrees();
                if ui
                    .add(egui::DragValue::new(&mut rotation_y_degrees).speed(1.0))
                    .changed()
                {
                    camera_state.rotation_xyz_radians.y = rotation_y_degrees.to_radians();
                }
                ui.end_row();

                ui.label("Field of View")
                    .on_hover_text("The vertical field of view in degrees.");
                let mut fov_degrees = camera_state.fov_y_radians.to_degrees();
                if ui
                    .add(
                        egui::DragValue::new(&mut fov_degrees)
                            .speed(1.0)
                            .clamp_range(0.0..=180.0),
                    )
                    .changed()
                {
                    camera_state.fov_y_radians = fov_degrees.to_radians();
                }

                ui.end_row();

                if ui.button("Reset").clicked() {
                    *camera_state = CameraInputState::default();
                }
            });
        });
}

pub fn stage_lighting_window(
    ctx: &egui::Context,
    open: &mut bool,
    state: &mut StageLightingState,
) -> bool {
    let mut changed = false;
    Window::new("Stage Lighting")
        .open(open)
        .resizable(false)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open render folder...").clicked() {
                        if let Some(folder) = FileDialog::new().pick_folder() {
                            // Attempt to load supported lighting files based on naming conventions.
                            // Users should select paths like "/stage/battlefield/normal/render/".
                            state.light = Some(folder.join("light").join("light00.nuanmb"));
                            state.reflection_cube_map =
                                Some(folder.join("reflection_cubemap.nutexb"));
                            state.color_grading_lut = folder
                                .parent()
                                .map(|p| p.join("lut").join("color_grading_lut.nutexb"));
                            changed = true;
                        }
                    }
                });
            });
            ui.separator();

            let path_label = |ui: &mut Ui, path: &Option<PathBuf>| match path {
                Some(path) => {
                    ui.label(path.file_name().and_then(|f| f.to_str()).unwrap_or(""))
                        .on_hover_ui(|ui| {
                            ui.add(Label::new(path.to_string_lossy()).wrap(false));
                        });
                }
                None => {
                    ui.label("");
                }
            };

            Grid::new("stage_lighting").show(ui, |ui| {
                // TODO: Make the files buttons to load corresponding editors?
                ui.label("Lighting");
                path_label(ui, &state.light);
                if ui.button("Select file...").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Lighting Anim", &["nuanmb"])
                        .pick_file()
                    {
                        state.light = Some(file);
                        changed = true;
                    };
                }
                ui.end_row();

                ui.label("Reflection Cube Map");
                path_label(ui, &state.reflection_cube_map);
                if ui.button("Select file...").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Cube Map Nutexb", &["nutexb"])
                        .pick_file()
                    {
                        state.reflection_cube_map = Some(file);
                        changed = true;
                    };
                };
                ui.end_row();

                ui.label("Color Grading LUT");
                path_label(ui, &state.color_grading_lut);
                if ui.button("Select file...").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Color Grading LUT", &["nutexb"])
                        .pick_file()
                    {
                        state.color_grading_lut = Some(file);
                        changed = true;
                    };
                };
                ui.end_row();
            });

            if ui.button("Reset").clicked() {
                *state = StageLightingState::default();
                changed = true;
            };
        });
    changed
}

pub fn preferences_window(
    ctx: &egui::Context,
    preferences: &mut AppPreferences,
    open: &mut bool,
) -> bool {
    let mut changed = false;

    egui::Window::new("Preferences")
        .open(open)
        .resizable(false)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(egui::Button::new("Open preferences directory...").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();

                        let path = application_dir();
                        if let Err(e) = open::that(path) {
                            log::error!("Failed to open {path:?}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            // TODO: Add a toggle widget instead.
            changed |= ui
                .checkbox(&mut preferences.dark_mode, "Dark Mode")
                .changed();
            ui.horizontal(|ui| {
                changed |= ui
                    .color_edit_button_srgb(&mut preferences.viewport_color)
                    .changed();
                ui.label("Viewport Background");
            });
            changed |= ui
                .checkbox(
                    &mut preferences.autohide_expressions,
                    "Automatically Hide Expressions",
                )
                .changed();
            if ui.button("Reset Preferences").clicked() {
                *preferences = AppPreferences::default();
                changed = true;
            }
        });
    changed
}

pub fn log_window(ctx: &Context, open: &mut bool) {
    Window::new("Application Log")
        .open(open)
        .resizable(true)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (level, message) in LOGGER.messages.lock().unwrap().iter() {
                        ui.horizontal(|ui| {
                            log_level_icon(ui, level);
                            // binrw formats backtraces, which isn't supported by egui font rendering.
                            // TODO: Avoid clone?
                            let clean_message = strip_ansi_escapes::strip(message)
                                .map(|m| String::from_utf8_lossy(&m).to_string())
                                .unwrap_or_else(|_| message.clone());
                            ui.label(clean_message);
                        });
                    }
                });
        });
}
