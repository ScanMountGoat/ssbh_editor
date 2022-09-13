use std::path::Path;

use egui::{Button, ComboBox, RichText, ScrollArea, Ui};
use log::error;
use rfd::FileDialog;
use ssbh_data::{
    mesh_data::{AttributeData, MeshObjectData, VectorData},
    prelude::*,
};

use crate::{
    app::{display_validation_errors, warning_icon, UiState, WARNING_COLOR},
    validation::{MeshValidationError, MeshValidationErrorKind},
    widgets::bone_combo_box,
};

pub fn mesh_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    mesh: &mut MeshData,
    skel: Option<&SkelData>,
    validation_errors: &[MeshValidationError],
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
                        .and_then(|index| mesh.objects.get_mut(index))
                    {
                        // TODO: Find a cleaner way to get the errors for the selected mesh.
                        let missing_attributes = validation_errors
                            .iter()
                            .filter_map(|e| match &e.kind {
                                MeshValidationErrorKind::MissingRequiredVertexAttributes {
                                    missing_attributes,
                                    ..
                                } => {
                                    if Some(e.mesh_object_index)
                                        == ui_state.selected_mesh_attributes_index
                                    {
                                        Some(missing_attributes.as_slice())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            })
                            .next()
                            .unwrap_or_default();

                        let (a_open, a_changed) =
                            attributes_window(ctx, mesh_object, missing_attributes);

                        changed |= a_changed;

                        if !a_open {
                            ui_state.selected_mesh_attributes_index = None;
                        }
                    }

                    ui.checkbox(advanced_mode, "Advanced Settings");

                    let mut meshes_to_remove = Vec::new();
                    egui::Grid::new("mesh_grid").show(ui, |ui| {
                        // TODO: Show tooltips for header names?
                        ui.heading("Name");
                        ui.heading("Subindex");
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

                            // TODO: Avoid allocating here.
                            let errors: Vec<_> = validation_errors
                                .iter()
                                .filter(|e| e.mesh_object_index == i)
                                .collect();

                            // TODO: Reorder mesh objects?
                            ui.horizontal(|ui| {
                                // TODO: Show errors on the appropriate field?
                                if !errors.is_empty() {
                                    warning_icon(ui).on_hover_ui(|ui| {
                                        display_validation_errors(ui, &errors);
                                    });
                                }

                                changed |= edit_name(ui, mesh_object, *advanced_mode);
                            });

                            if *advanced_mode {
                                changed |= ui
                                    .add(egui::DragValue::new(&mut mesh_object.subindex))
                                    .changed();
                            } else {
                                ui.label(mesh_object.subindex.to_string());
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
                            ui.horizontal(|ui| {
                                // TODO: Simplify this code?
                                let attribute_error = errors.iter().find(|e| {
                                    matches!(
                                        e.kind,
                                        MeshValidationErrorKind::MissingRequiredVertexAttributes { .. }
                                    )
                                });

                                if let Some(attribute_error) = attribute_error {
                                    if ui
                                        .add_sized(
                                            [140.0, 20.0],
                                            Button::new(
                                                RichText::new("⚠ Vertex Attributes...")
                                                    .color(WARNING_COLOR),
                                            ),
                                        )
                                        .on_hover_text(format!("{}", attribute_error))
                                        .clicked()
                                    {
                                        ui_state.selected_mesh_attributes_index = Some(i);
                                    }
                                } else if ui
                                    .add_sized([140.0, 20.0], Button::new("Vertex Attributes..."))
                                    .clicked()
                                {
                                    ui_state.selected_mesh_attributes_index = Some(i);
                                }
                            });

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

fn edit_name(ui: &mut egui::Ui, mesh_object: &mut MeshObjectData, advanced_mode: bool) -> bool {
    let mut changed = false;
    if advanced_mode {
        // TODO: Link name edits with the numdlb and numshexb.
        // This will need to check for duplicate names.
        changed |= ui.text_edit_singleline(&mut mesh_object.name).changed();
    } else {
        ui.label(&mesh_object.name);
    }
    changed
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

fn edit_attribute_name(ui: &mut Ui, name: &mut String, id: egui::Id, valid_names: &[&str]) {
    ComboBox::from_id_source(id)
        .selected_text(name.as_str())
        .show_ui(ui, |ui| {
            for n in valid_names {
                ui.selectable_value(name, n.to_string(), *n);
            }
        });
}

fn attributes_window(
    ctx: &egui::Context,
    mesh_object: &mut MeshObjectData,
    missing_attributes: &[String],
) -> (bool, bool) {
    // TODO: Return changed.
    let mut open = true;
    let mut changed = false;

    egui::Window::new(format!("Vertex Attributes ({})", mesh_object.name))
        .open(&mut open)
        .show(ctx, |ui| {
            // TODO: Add button to remove unused attributes to save memory.
            if !missing_attributes.is_empty()
                && ui
                    .button(format!(
                        "Add {} Missing Attributes",
                        missing_attributes.len()
                    ))
                    .clicked()
            {
                add_missing_attributes(mesh_object, missing_attributes);
                changed = true;
            }

            egui::Grid::new("vertex_attributes_grid").show(ui, |ui| {
                ui.heading("Name");
                ui.heading("Usage");
                ui.heading("Vertex Count");
                ui.end_row();

                // Vertex buffer 0.
                let id = ui.make_persistent_id("attr");
                for (i, a) in mesh_object.positions.iter_mut().enumerate() {
                    edit_attribute_name(ui, &mut a.name, id.with("pos").with(i), &["Position0"]);
                    ui.label("Position");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for (i, a) in mesh_object.normals.iter_mut().enumerate() {
                    edit_attribute_name(ui, &mut a.name, id.with("nrm").with(i), &["Normal0"]);
                    ui.label("Normal");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for (i, a) in mesh_object.tangents.iter_mut().enumerate() {
                    edit_attribute_name(ui, &mut a.name, id.with("tan").with(i), &["Tangent0"]);
                    ui.label("Tangent");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for (i, a) in mesh_object.binormals.iter_mut().enumerate() {
                    edit_attribute_name(ui, &mut a.name, id.with("binrm").with(i), &["Binormal0"]);
                    ui.label("Binormal (Bitangent)");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }

                // Vertex buffer 1.
                for (i, a) in mesh_object.texture_coordinates.iter_mut().enumerate() {
                    edit_attribute_name(
                        ui,
                        &mut a.name,
                        id.with("uv").with(i),
                        &["map1", "bake1", "uvSet", "uvSet1", "uvSet2"],
                    );
                    ui.label("Texture Coordinate (UV)");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
                for (i, a) in mesh_object.color_sets.iter_mut().enumerate() {
                    edit_attribute_name(
                        ui,
                        &mut a.name,
                        id.with("color").with(i),
                        &[
                            "colorSet1",
                            "colorSet2",
                            "colorSet2_1",
                            "colorSet2_2",
                            "colorSet2_3",
                            "colorSet3",
                            "colorSet4",
                            "colorSet5",
                            "colorSet6",
                            "colorSet7",
                        ],
                    );
                    ui.label("Color Set (Vertex Color)");
                    ui.label(a.data.len().to_string());
                    ui.end_row();
                }
            })
        });

    (open, changed)
}

fn add_uv(mesh_object: &mut MeshObjectData, name: &str, count: usize) {
    mesh_object.texture_coordinates.push(AttributeData {
        name: name.to_owned(),
        data: VectorData::Vector2(vec![[0.0; 2]; count]),
    });
}

fn add_color_set(mesh_object: &mut MeshObjectData, name: &str, count: usize, default: [f32; 4]) {
    mesh_object.color_sets.push(AttributeData {
        name: name.to_owned(),
        data: VectorData::Vector4(vec![default; count]),
    });
}

fn add_missing_attributes(mesh_object: &mut MeshObjectData, missing_attributes: &[String]) {
    // TODO: Error if count is invalid?
    if let Ok(count) = mesh_object.vertex_count() {
        for a in missing_attributes {
            // Choose neutral values for defaults.
            // This avoids changing the model appearance when adding attributes.
            // TODO: Research better defaults.
            let name = a.as_str();
            match name {
                "map1" => add_uv(mesh_object, name, count),
                "bake1" => add_uv(mesh_object, name, count),
                "uvSet" => add_uv(mesh_object, name, count),
                "uvSet1" => add_uv(mesh_object, name, count),
                "uvSet2" => add_uv(mesh_object, name, count),
                "colorSet1" => add_color_set(mesh_object, name, count, [0.5; 4]),
                "colorSet2" => add_color_set(mesh_object, name, count, [1.0 / 7.0; 4]),
                "colorSet2_1" => add_color_set(mesh_object, name, count, [1.0 / 7.0; 4]),
                "colorSet2_2" => add_color_set(mesh_object, name, count, [1.0 / 7.0; 4]),
                "colorSet2_3" => add_color_set(mesh_object, name, count, [1.0 / 7.0; 4]),
                "colorSet3" => add_color_set(mesh_object, name, count, [0.5; 4]),
                "colorSet4" => add_color_set(mesh_object, name, count, [0.5; 4]),
                "colorSet5" => add_color_set(mesh_object, name, count, [0.0; 4]),
                "colorSet6" => add_color_set(mesh_object, name, count, [1.0; 4]),
                "colorSet7" => add_color_set(mesh_object, name, count, [1.0; 4]),
                _ => (),
            }
        }
    }
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
                    subindex: 0,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 1,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    subindex: 0,
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
        assert_eq!(0, mesh.objects[0].subindex);

        assert_eq!("a", mesh.objects[1].name);
        assert_eq!(1, mesh.objects[1].subindex);

        assert_eq!("b", mesh.objects[2].name);
        assert_eq!(0, mesh.objects[2].subindex);
    }

    #[test]
    fn mesh_order_added_meshes() {
        let mut mesh = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 1,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "a".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
                MeshObjectData {
                    name: "b".to_owned(),
                    subindex: 0,
                    ..Default::default()
                },
            ],
        };

        let reference = MeshData {
            major_version: 1,
            minor_version: 10,
            objects: vec![MeshObjectData {
                name: "b".to_owned(),
                subindex: 0,
                ..Default::default()
            }],
        };

        match_mesh_order(&mut mesh, &reference);

        assert_eq!("b", mesh.objects[0].name);
        assert_eq!(0, mesh.objects[0].subindex);

        assert_eq!("a", mesh.objects[1].name);
        assert_eq!(1, mesh.objects[1].subindex);

        assert_eq!("a", mesh.objects[2].name);
        assert_eq!(0, mesh.objects[2].subindex);
    }
}
