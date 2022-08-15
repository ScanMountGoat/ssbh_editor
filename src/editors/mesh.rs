use std::path::Path;

use egui::{Button, ScrollArea};
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

                egui::menu::menu_button(ui, "Mesh", |ui| {
                    if ui
                        .add(Button::new("Match reference mesh order...").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Mesh", &["numshb"])
                            .pick_file()
                        {
                            match MeshData::from_file(&file) {
                                Ok(reference) => match_mesh_order(mesh, &reference),
                                Err(e) => error!("Failed to read {:?}: {}", file, e),
                            }
                        }
                    }
                });

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("Mesh Editor Wiki").clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Mesh-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(mesh_object) = ui_state
                        .selected_mesh_influences_index
                        .and_then(|index| mesh.objects.get(index))
                    {
                        let open = influences_window(ctx, mesh_object);
                        if !open {
                            ui_state.selected_mesh_influences_index = None;
                        }
                    }

                    if let Some(mesh_object) = ui_state
                        .selected_mesh_attributes_index
                        .and_then(|index| mesh.objects.get(index))
                    {
                        let open = attributes_window(ctx, mesh_object);
                        if !open {
                            ui_state.selected_mesh_attributes_index = None;
                        }
                    }

                    ui.checkbox(advanced_mode, "Advanced Settings");

                    let mut meshes_to_remove = Vec::new();
                    egui::Grid::new("mesh_grid").show(ui, |ui| {
                        // TODO: Show tooltips for header names?
                        ui.heading("Name");
                        ui.heading("Sub Index");
                        ui.heading("Parent Bone");
                        ui.heading("");
                        ui.heading("");
                        if *advanced_mode {
                            ui.heading("Sort Bias");
                            ui.heading("");
                            ui.heading("");
                        }
                        ui.end_row();

                        for (i, mesh_object) in mesh.objects.iter_mut().enumerate() {
                            let id = egui::Id::new("mesh").with(i);

                            // TODO: Reorder mesh objects?
                            if *advanced_mode {
                                // TODO: Link name edits with the numdlb and numshexb.
                                // This will need to check for duplicate names.
                                changed |= ui.text_edit_singleline(&mut mesh_object.name).changed();
                            } else {
                                ui.label(&mesh_object.name);
                            }

                            if *advanced_mode {
                                changed |= ui
                                    .add(egui::DragValue::new(&mut mesh_object.sub_index))
                                    .changed();
                            } else {
                                ui.label(mesh_object.sub_index.to_string());
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

                            // Open in a separate window since they won't fit in the grid.
                            if ui.button("Vertex Attributes...").clicked() {
                                ui_state.selected_mesh_attributes_index = Some(i);
                            }

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

fn match_mesh_order(mesh: &mut MeshData, reference: &MeshData) {
    mesh.objects.sort_by_key(|o| {
        // The sort is stable, so unmatched objects will be placed at the end in the same order.
        reference
            .objects
            .iter()
            .position(|r| r.name == o.name)
            .unwrap_or(reference.objects.len())
    })
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

fn attributes_window(ctx: &egui::Context, mesh_object: &MeshObjectData) -> bool {
    let mut open = true;
    egui::Window::new(format!("Vertex Attributes ({})", mesh_object.name))
        .open(&mut open)
        .show(ctx, |ui| {
            // TODO: Add/remove attributes using default values.
            // TODO: Don't allow removing position,normal,tangent?
            // TODO: Link the matl to add required attributes.
            egui::Grid::new("vertex_attributes_grid").show(ui, |ui| {
                ui.heading("Name");
                ui.heading("Usage");
                ui.heading("Vertex Count");
                ui.end_row();

                // Vertex buffer 0.
                for a in &mesh_object.positions {
                    ui.label(&a.name);
                    ui.label("Position");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for a in &mesh_object.normals {
                    ui.label(&a.name);
                    ui.label("Normal");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for a in &mesh_object.tangents {
                    ui.label(&a.name);
                    ui.label("Tangent");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for a in &mesh_object.binormals {
                    ui.label(&a.name);
                    ui.label("Binormal (Bitangent)");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }

                // Vertex buffer 1.
                for a in &mesh_object.texture_coordinates {
                    ui.label(&a.name);
                    ui.label("Texture Coordinate (UV)");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for a in &mesh_object.color_sets {
                    ui.label(&a.name);
                    ui.label("Color Set (Vertex Color)");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
            })
        });
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_order_empty_reference() {
        let mut mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "a".to_owned(),
                    sub_index: 0,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "a".to_owned(),
                    sub_index: 1,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    sub_index: 0,
                    ..Default::default()
                },
            ],
        };

        let reference = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: Vec::new(),
        };

        match_mesh_order(&mut mesh, &reference);

        assert_eq!("a", mesh.objects[0].name);
        assert_eq!(0, mesh.objects[0].sub_index);

        assert_eq!("a", mesh.objects[1].name);
        assert_eq!(1, mesh.objects[1].sub_index);

        assert_eq!("b", mesh.objects[2].name);
        assert_eq!(0, mesh.objects[2].sub_index);
    }

    #[test]
    fn mesh_order_added_meshes() {
        let mut mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "a".to_owned(),
                    sub_index: 1,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "a".to_owned(),
                    sub_index: 0,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    sub_index: 0,
                    ..Default::default()
                },
            ],
        };

        let reference = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "b".to_owned(),
                sub_index: 0,
                ..Default::default()
            }],
        };

        match_mesh_order(&mut mesh, &reference);

        assert_eq!("b", mesh.objects[0].name);
        assert_eq!(0, mesh.objects[0].sub_index);

        assert_eq!("a", mesh.objects[1].name);
        assert_eq!(1, mesh.objects[1].sub_index);

        assert_eq!("a", mesh.objects[2].name);
        assert_eq!(0, mesh.objects[2].sub_index);
    }
}
