use crate::CameraInputState;

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
