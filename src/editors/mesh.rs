use std::path::Path;

use egui::ScrollArea;
use log::error;
use rfd::FileDialog;
use ssbh_data::{mesh_data::MeshObjectData, prelude::*};

use crate::{app::UiState, widgets::bone_combo_box};

pub fn mesh_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    mesh: &mut MeshData,
    skel: Option<&SkelData>,
    ui_state: &mut UiState,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    let advanced_mode = &mut ui_state.mesh_editor_advanced_mode;

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

                                let file = Path::new(folder_name).join(file_name);
                                if let Err(e) = mesh.write_to_file(&file) {
                                    error!("Failed to save {:?}: {}", file, e);
                                }
                            }

                            if ui.button("Save As...").clicked() {
                                ui.close_menu();

                                if let Some(file) = FileDialog::new()
                                    .add_filter("Mesh", &["numshb"])
                                    .save_file()
                                {
                                    if let Err(e) = mesh.write_to_file(&file) {
                                        error!("Failed to save {:?}: {}", file, e);
                                    }
                                }
                            }
                        });

                        egui::menu::menu_button(ui, "Help", |ui| {
                            if ui.button("Mesh Editor Wiki").clicked() {
                                ui.close_menu();

                                let link =
                                    "https://github.com/ScanMountGoat/ssbh_editor/wiki/Mesh-Editor";
                                if let Err(e) = open::that(link) {
                                    log::error!("Failed to open {link}: {e}");
                                }
                            }
                        });

                        // TODO: Match the order from an existing mesh?
                    });
                    ui.separator();

                    if let Some(mesh_object) = ui_state
                        .selected_mesh_influences_index
                        .and_then(|index| mesh.objects.get(index))
                    {
                        let open = influences_window(ctx, mesh_object);
                        if !open {
                            ui_state.selected_mesh_influences_index = None;
                        }
                    }

                    ui.checkbox(advanced_mode, "Advanced Settings");

                    // TODO: Use a separate scroll area for this?
                    let mut meshes_to_remove = Vec::new();
                    egui::Grid::new("mesh_grid").show(ui, |ui| {
                        // TODO: Show tooltips for header names?
                        ui.heading("Name");
                        ui.heading("Parent Bone");
                        ui.heading("Influences");
                        if *advanced_mode {
                            ui.heading("Sub Index");
                            ui.heading("Sort Bias");
                            ui.heading("Depth Write");
                            ui.heading("Depth Test");
                        }
                        ui.end_row();

                        for (i, mesh_object) in mesh.objects.iter_mut().enumerate() {
                            let id = egui::Id::new("mesh").with(i);

                            // TODO: Reorder mesh objects?
                            // TODO: Unique names?
                            if *advanced_mode {
                                // TODO: Link name edits with the numdlb and numshexb.
                                // This will need to check for duplicate names.
                                changed |= ui.text_edit_singleline(&mut mesh_object.name).changed();
                            } else {
                                ui.label(&mesh_object.name);
                            }

                            // TODO: Are parent bones and influences mutually exclusive?
                            // TODO: Is there a better way to indicate no parent than ""?
                            changed |= bone_combo_box(
                                ui,
                                &mut mesh_object.parent_bone_name,
                                id.with("parent_bone"),
                                skel,
                                &[""],
                            );

                            // Open influences in a separate window since they won't fit in the grid.
                            if !mesh_object.bone_influences.is_empty() {
                                if ui.button("Bone Influences...").clicked() {
                                    ui_state.selected_mesh_influences_index = Some(i);
                                }
                            } else {
                                // TODO: How to handle gaps in grid?
                                ui.allocate_space(egui::Vec2::new(1.0, 1.0));
                            }

                            if *advanced_mode {
                                changed |= ui
                                    .add(egui::DragValue::new(&mut mesh_object.sub_index))
                                    .changed();
                                changed |= ui
                                    .add(egui::DragValue::new(&mut mesh_object.sort_bias))
                                    .changed();

                                // TODO: Center these in the cell and omit the labels?
                                changed |= ui
                                    .checkbox(
                                        &mut mesh_object.disable_depth_write,
                                        "Disable Depth Write",
                                    )
                                    .changed();
                                changed |= ui
                                    .checkbox(
                                        &mut mesh_object.disable_depth_test,
                                        "Disable Depth Test",
                                    )
                                    .changed();

                                if ui.button("Delete").clicked() {
                                    changed = true;
                                    meshes_to_remove.push(i);
                                }
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

    (open, changed)
}

fn influences_window(ctx: &egui::Context, mesh_object: &MeshObjectData) -> bool {
    let mut open = true;
    egui::Window::new(format!("Bone Influences ({})", mesh_object.name))
        .open(&mut open)
        .show(ctx, |ui| {
            // TODO: Add an option to show this per vertex instead of per bone?
            // Use a simple layout for now to avoid performance overhead of doing it per vertex.
            egui::Grid::new("bone_influences_grid").show(ui, |ui| {
                ui.heading("Bone Name");
                ui.heading("Vertex Count");
                ui.end_row();

                for influence in &mesh_object.bone_influences {
                    ui.label(&influence.bone_name);
                    ui.label(influence.vertex_weights.len().to_string());
                    ui.end_row();
                }
            })
        });
    open
}
