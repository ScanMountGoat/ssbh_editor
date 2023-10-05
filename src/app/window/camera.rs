use std::path::PathBuf;

use egui::{Label, Ui};
use rfd::FileDialog;

use crate::{horizontal_separator_empty, CameraInputState};

pub fn camera_settings_window(
    ctx: &egui::Context,
    open: &mut bool,
    camera_state: &mut CameraInputState,
) -> bool {
    let mut changed = false;

    egui::Window::new("Camera Settings")
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            egui::Grid::new("camera_grid").show(ui, |ui| {
                ui.label("Translation X");
                changed |= ui
                    .add(egui::DragValue::new(&mut camera_state.translation_xyz.x))
                    .changed();
                ui.end_row();

                ui.label("Translation Y");
                changed |= ui
                    .add(egui::DragValue::new(&mut camera_state.translation_xyz.y))
                    .changed();
                ui.end_row();

                ui.label("Translation Z");
                changed |= ui
                    .add(egui::DragValue::new(&mut camera_state.translation_xyz.z))
                    .changed();
                ui.end_row();

                // TODO: This will need to use quaternions to work with camera anims.
                // TODO: Add an option for radians or degrees?
                // TODO: Create helper function for this?
                ui.label("Rotation X");
                let mut rotation_x_degrees = camera_state.rotation_xyz_radians.x.to_degrees();
                if ui
                    .add(egui::DragValue::new(&mut rotation_x_degrees).speed(1.0))
                    .changed()
                {
                    camera_state.rotation_xyz_radians.x = rotation_x_degrees.to_radians();
                    changed = true;
                }
                ui.end_row();

                ui.label("Rotation Y");
                let mut rotation_y_degrees = camera_state.rotation_xyz_radians.y.to_degrees();
                if ui
                    .add(egui::DragValue::new(&mut rotation_y_degrees).speed(1.0))
                    .changed()
                {
                    camera_state.rotation_xyz_radians.y = rotation_y_degrees.to_radians();
                    changed = true;
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
                    changed = true;
                }
                ui.end_row();
            });
            horizontal_separator_empty(ui);

            // TODO: disable other settings while animating.
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

            if ui.button("Reset").clicked() {
                *camera_state = CameraInputState::default();
                changed = true;
            }
        });

    changed
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
