use std::path::PathBuf;

use egui::{Button, DragValue, Label, Ui};
use rfd::FileDialog;

use crate::{horizontal_separator_empty, CameraState, CameraValues};

pub fn camera_settings_window(
    ctx: &egui::Context,
    open: &mut bool,
    camera_state: &mut CameraState,
    default_camera: &mut CameraValues,
) -> bool {
    let mut changed = false;

    egui::Window::new("Camera Settings")
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Defaults", |ui| {
                    if ui
                        .add(Button::new("Save Current Settings as Default").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();

                        *default_camera = camera_state.values.clone();
                    }

                    if ui
                        .button("Reset Defaults")
                        .on_hover_text("Hard reset all settings to their original defaults.")
                        .clicked()
                    {
                        ui.close_menu();

                        *camera_state = CameraState::default();
                        *default_camera = CameraValues::default();
                        changed = true;
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Camera Settings Wiki").clicked() {
                        ui.close_menu();

                        let link =
                            "https://github.com/ScanMountGoat/ssbh_editor/wiki/Camera-Settings";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            egui::Grid::new("camera_grid").show(ui, |ui| {
                ui.label("Translation X");
                changed |= ui
                    .add(DragValue::new(&mut camera_state.values.translation.x))
                    .changed();
                ui.end_row();

                ui.label("Translation Y");
                changed |= ui
                    .add(DragValue::new(&mut camera_state.values.translation.y))
                    .changed();
                ui.end_row();

                ui.label("Translation Z");
                changed |= ui
                    .add(DragValue::new(&mut camera_state.values.translation.z))
                    .changed();
                ui.end_row();

                ui.label("Rotation X");
                changed |= edit_angle_degrees(ui, &mut camera_state.values.rotation_radians.x);
                ui.end_row();

                ui.label("Rotation Y");
                changed |= edit_angle_degrees(ui, &mut camera_state.values.rotation_radians.y);
                ui.end_row();

                // All three axes are necessary to decompose in game animations.
                // Most users won't touch this value.
                ui.label("Rotation Z");
                changed |= edit_angle_degrees(ui, &mut camera_state.values.rotation_radians.z);
                ui.end_row();

                ui.label("Field of View")
                    .on_hover_text("The vertical field of view in degrees.");
                let mut fov_degrees = camera_state.values.fov_y_radians.to_degrees();
                if ui
                    .add(
                        DragValue::new(&mut fov_degrees)
                            .speed(1.0)
                            .clamp_range(0.0..=180.0),
                    )
                    .changed()
                {
                    camera_state.values.fov_y_radians = fov_degrees.to_radians();
                    changed = true;
                }
                ui.end_row();

                ui.label("Near Clip")
                    .on_hover_text("The nearest distance visible.");
                changed |= ui
                    .add(
                        DragValue::new(&mut camera_state.values.near_clip)
                            .clamp_range(0.001..=camera_state.values.far_clip),
                    )
                    .changed();
                ui.end_row();

                ui.label("Far Clip")
                    .on_hover_text("The farthest distance visible.");
                changed |= ui
                    .add(
                        DragValue::new(&mut camera_state.values.far_clip)
                            .clamp_range(camera_state.values.near_clip..=f32::MAX),
                    )
                    .changed();
                ui.end_row();
            });
            horizontal_separator_empty(ui);

            ui.horizontal(|ui| {
                ui.label("Camera Anim");
                path_label(ui, &camera_state.anim_path);
                if ui.button("Select file...").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Camera Anim", &["nuanmb"])
                        .pick_file()
                    {
                        camera_state.anim_path = Some(file);
                        changed = true;
                    };
                }
            });
            horizontal_separator_empty(ui);

            if ui
                .button("Reset")
                .on_hover_text("Reset settings to their configured defaults.")
                .clicked()
            {
                *camera_state = CameraState {
                    values: default_camera.clone(),
                    ..Default::default()
                };
                changed = true;
            }
        });

    changed
}

fn edit_angle_degrees(ui: &mut Ui, radians: &mut f32) -> bool {
    let mut degrees = radians.to_degrees();
    if ui.add(DragValue::new(&mut degrees).speed(1.0)).changed() {
        *radians = degrees.to_radians();
        true
    } else {
        false
    }
}

fn path_label(ui: &mut Ui, path: &Option<PathBuf>) {
    match path {
        Some(path) => {
            ui.label(path.file_name().and_then(|f| f.to_str()).unwrap_or(""))
                .on_hover_ui(|ui| {
                    ui.add(Label::new(path.to_string_lossy()).wrap(false));
                });
        }
        None => {
            ui.label("");
        }
    }
}
