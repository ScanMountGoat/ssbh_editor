use egui::ScrollArea;
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;

pub fn mesh_editor(
    ctx: &egui::Context,
    title: &str,
    mesh: &mut MeshData,
    advanced_mode: &mut bool,
) -> bool {
    let mut open = true;

    egui::Window::new(format!("Mesh Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::menu::bar(ui, |ui| {
                        egui::menu::menu_button(ui, "File", |ui| {
                            if ui.button("Save").clicked() {
                                ui.close_menu();

                                if let Some(file) = FileDialog::new()
                                    .add_filter("Mesh", &["numshb"])
                                    .save_file()
                                {
                                    if let Err(e) = mesh.write_to_file(file) {
                                        error!("Failed to save Mesh (.numshb): {}", e);
                                    }
                                }
                            }
                        });
                    });

                    ui.add(egui::Separator::default().horizontal());

                    ui.checkbox(advanced_mode, "Advanced Settings");

                    let mut meshes_to_remove = Vec::new();
                    egui::Grid::new("some_unique_id").show(ui, |ui| {
                        for (i, mesh_object) in mesh.objects.iter_mut().enumerate() {
                            // TODO: Link name edits with the numdlb and numshexb.
                            // This will need to check for duplicate names.
                            // TODO: Reorder mesh objects?
                            // TODO: Unique names?
                            egui::CollapsingHeader::new(format!(
                                "{} {}",
                                mesh_object.name, mesh_object.sub_index
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Sort Bias");
                                    ui.label(mesh_object.sort_bias.to_string());
                                });

                                ui.checkbox(
                                    &mut mesh_object.disable_depth_write,
                                    "Disable Depth Write",
                                );
                                ui.checkbox(
                                    &mut mesh_object.disable_depth_test,
                                    "Disable Depth Test",
                                );

                                egui::CollapsingHeader::new("Bone Influences")
                                    .id_source(format!(
                                        "{} {}",
                                        mesh_object.name, mesh_object.sub_index
                                    ))
                                    .show(ui, |ui| {
                                        for influence in &mesh_object.bone_influences {
                                            ui.horizontal(|ui| {
                                                ui.label(&influence.bone_name);
                                                ui.label(format!(
                                                    "{} vertices",
                                                    influence.vertex_weights.len()
                                                ));
                                            });
                                        }
                                    });
                            });

                            if *advanced_mode && ui.button("Delete").clicked() {
                                meshes_to_remove.push(i);
                            }

                            ui.end_row();
                        }
                    });

                    // TODO: Only allow deleting one object at a time?
                    for i in meshes_to_remove {
                        mesh.objects.remove(i);
                    }
                });
        });

    open
}
