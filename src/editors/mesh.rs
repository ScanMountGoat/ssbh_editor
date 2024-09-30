use crate::{
    app::{display_validation_errors, draggable_icon, warning_icon_text},
    horizontal_separator_empty,
    path::folder_editor_title,
    save_file, save_file_as,
    validation::{MeshValidationError, MeshValidationErrorKind},
    widgets::bone_combo_box,
    EditorMessage, EditorResponse,
};
use egui::{
    special_emojis::GITHUB, Button, CollapsingHeader, ComboBox, Grid, RichText, ScrollArea,
    TextEdit, TextWrapMode, Ui,
};
use egui_dnd::dnd;
use log::error;
use rfd::FileDialog;
use ssbh_data::{
    mesh_data::{
        transform_points, transform_vectors, AttributeData, BoneInfluence, MeshObjectData,
        VectorData, VertexWeight,
    },
    prelude::*,
};
use std::path::Path;

#[derive(Hash)]
struct MeshObjectIndex(usize);

pub fn mesh_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    mesh: &mut MeshData,
    skel: Option<&SkelData>,
    validation_errors: &[MeshValidationError],
    dark_mode: bool,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;
    let mut message = None;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Mesh Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();
                        saved |= save_file(mesh, folder_name, file_name);
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();
                        saved |= save_file_as(mesh, folder_name, file_name, "Mesh", "numshb");
                    }
                });

                ui.menu_button("Mesh", |ui| {
                    if ui
                        .add(
                            Button::new("Match reference mesh order...")
                                .wrap_mode(TextWrapMode::Extend),
                        )
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

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Mesh Editor Wiki")).clicked() {
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
                    changed |= edit_mesh(
                        ctx,
                        ui,
                        mesh,
                        validation_errors,
                        skel,
                        dark_mode,
                        &mut message,
                    );
                });
        });

    EditorResponse {
        open,
        changed,
        saved,
        message,
    }
}

fn edit_mesh(
    ctx: &egui::Context,
    ui: &mut Ui,
    mesh: &mut MeshData,
    validation_errors: &[MeshValidationError],
    skel: Option<&SkelData>,
    dark_mode: bool,
    message: &mut Option<EditorMessage>,
) -> bool {
    let mut changed = false;

    let mut mesh_to_remove = None;

    // TODO: Avoid allocating here.
    let mut items: Vec<_> = (0..mesh.objects.len()).map(MeshObjectIndex).collect();

    let response = dnd(ui, "mesh_dnd").show_vec(&mut items, |ui, item, handle, _| {
        ui.horizontal(|ui| {
            handle.ui(ui, |ui| {
                draggable_icon(ctx, ui, dark_mode);
            });

            let mesh_object = &mut mesh.objects[item.0];
            let id = egui::Id::new("mesh").with(item.0);

            // TODO: Avoid allocating here.
            let errors: Vec<_> = validation_errors
                .iter()
                .filter(|e| e.mesh_object_index == item.0)
                .collect();

            let text = if !errors.is_empty() {
                warning_icon_text(&mesh_object.name)
            } else {
                RichText::new(&mesh_object.name)
            };

            let header_response = CollapsingHeader::new(text)
                .id_source(id.with("name"))
                .show(ui, |ui| {
                    changed |= edit_mesh_object(
                        id,
                        ui,
                        mesh_object,
                        skel,
                        item.0,
                        &errors,
                        validation_errors,
                    );
                })
                .header_response;

            header_response.context_menu(|ui| {
                if ui.button("Delete").clicked() {
                    ui.close_menu();
                    mesh_to_remove = Some(item.0);
                    changed = true;
                }
            });

            // Outline the selected mesh in the viewport.
            if header_response.hovered() {
                *message = Some(EditorMessage::SelectMesh {
                    mesh_object_name: mesh.objects[item.0].name.clone(),
                    mesh_object_subindex: mesh.objects[item.0].subindex,
                });
            }

            if !errors.is_empty() {
                header_response.on_hover_ui(|ui| {
                    display_validation_errors(ui, validation_errors);
                });
            }
        });
    });

    if let Some(i) = mesh_to_remove {
        mesh.objects.remove(i);
    }

    if let Some(response) = response.final_update() {
        egui_dnd::utils::shift_vec(response.from, response.to, &mut mesh.objects);
        changed = true;
    }

    changed
}

fn edit_mesh_object(
    id: egui::Id,
    ui: &mut Ui,
    mesh_object: &mut MeshObjectData,
    skel: Option<&SkelData>,
    i: usize,
    errors: &[&MeshValidationError],
    validation_errors: &[MeshValidationError],
) -> bool {
    let mut changed = false;

    // TODO: Reorder mesh objects?
    // TODO: Show errors on the appropriate field?
    Grid::new(id.with("mesh_grid")).show(ui, |ui| {
        // TODO: Link name edits with the numdlb and numshexb.
        // This will need to check for duplicate names.
        ui.label("Name");
        changed |= ui
            .add(TextEdit::singleline(&mut mesh_object.name).clip_text(false))
            .changed();
        ui.end_row();

        // TODO: Is it possible to edit the subindex without messing up influence assignments?
        ui.label("Subindex");
        changed |= ui
            .add(egui::DragValue::new(&mut mesh_object.subindex))
            .changed();
        ui.end_row();

        ui.label("Sort Bias");
        changed |= ui
            .add(egui::DragValue::new(&mut mesh_object.sort_bias))
            .changed();
        ui.end_row();
    });

    changed |= ui
    .checkbox(&mut mesh_object.disable_depth_write, "Disable Depth Write")
    .on_hover_text("Disabling depth writes can resolve sorting issues with layered objects like glass bottles.")
    .changed();

    changed |= ui
        .checkbox(&mut mesh_object.disable_depth_test, "Disable Depth Test")
        .on_hover_text(
            "Disabling depth testing causes the mesh to render on top of previous meshes.",
        )
        .changed();
    horizontal_separator_empty(ui);

    CollapsingHeader::new("Bone Influences")
        .id_source(id.with("bone_influences"))
        .show(ui, |ui| {
            // Meshes should have influences or a parent bone but not both.
            if mesh_object.bone_influences.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Parent Bone").on_hover_text(
                        "Inherit the transformation of the specified bone while animating.",
                    );

                    changed |= bone_combo_box(
                        ui,
                        &mut mesh_object.parent_bone_name,
                        id.with("parent_bone"),
                        skel,
                        &[""],
                    );
                });

                if ui
                    .button("Convert to Bone Influences")
                    .on_hover_text("Weight all vertices to the parent bone and apply its transform")
                    .clicked()
                {
                    convert_parent_bone_to_influences(mesh_object, skel);
                    changed = true;
                }
            } else {
                if ui
                    .button("Remove Bone Influences")
                    .on_hover_text("Remove the vertex skin weights to assign a parent bone.")
                    .clicked()
                {
                    // TODO: What happens if there is a parent bone and influences?
                    // TODO: Convert to parent bone if there is only one influence.
                    mesh_object.bone_influences = Vec::new();
                    changed = true;
                }

                show_influences(ui, mesh_object);
            }
        });

    // TODO: Simplify this code?
    let attribute_error = errors.iter().find(|e| {
        matches!(
            e.kind,
            MeshValidationErrorKind::MissingRequiredVertexAttributes { .. }
        )
    });

    // TODO: Show the details of the error on hover.
    let header_text = if attribute_error.is_some() {
        warning_icon_text("Vertex Attributes")
    } else {
        RichText::new("Vertex Attributes")
    };

    CollapsingHeader::new(header_text)
        .id_source(id.with("attributes"))
        .show(ui, |ui| {
            // TODO: Find a cleaner way to get the errors for the selected mesh.
            let missing_attributes = validation_errors
                .iter()
                .filter_map(|e| match &e.kind {
                    MeshValidationErrorKind::MissingRequiredVertexAttributes {
                        missing_attributes,
                        ..
                    } => {
                        if e.mesh_object_index == i {
                            Some(missing_attributes.as_slice())
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .next()
                .unwrap_or_default();

            changed |= edit_mesh_attributes(ui, mesh_object, missing_attributes);
        });

    changed
}

// TODO: Move this to ssbh_data?
fn convert_parent_bone_to_influences(mesh_object: &mut MeshObjectData, skel: Option<&SkelData>) {
    // Weight vertices to parent bone.
    mesh_object.bone_influences = vec![BoneInfluence {
        bone_name: mesh_object.parent_bone_name.clone(),
        vertex_weights: (0..mesh_object.vertex_count().unwrap_or_default())
            .map(|i| VertexWeight {
                vertex_index: i as u32,
                vertex_weight: 1.0,
            })
            .collect(),
    }];

    // Apply parent transform.
    if let Some(parent_transform) = skel.and_then(|s| {
        s.bones
            .iter()
            .find(|b| b.name == mesh_object.parent_bone_name)
            .and_then(|b| s.calculate_world_transform(b).ok())
    }) {
        for attribute in &mut mesh_object.positions {
            attribute.data = transform_points(&attribute.data, &parent_transform);
        }
        for attribute in &mut mesh_object.normals {
            attribute.data = transform_vectors(&attribute.data, &parent_transform);
        }
        for attribute in &mut mesh_object.tangents {
            attribute.data = transform_vectors(&attribute.data, &parent_transform);
        }
        for attribute in &mut mesh_object.binormals {
            attribute.data = transform_vectors(&attribute.data, &parent_transform);
        }
    } else {
        error!(
            "Failed to apply transform for {:?}",
            mesh_object.parent_bone_name
        );
    }

    // Remove parent.
    mesh_object.parent_bone_name = String::new();
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

fn show_influences(ui: &mut Ui, mesh_object: &MeshObjectData) -> egui::InnerResponse<()> {
    // TODO: Add an option to show this per vertex instead of per bone?
    // Use a simple layout for now to avoid performance overhead of doing it per vertex.
    egui::Grid::new("bone_influences_grid").show(ui, |ui| {
        ui.label(RichText::new("Bone Name").size(16.0));
        ui.label(RichText::new("Vertex Count").size(16.0));
        ui.end_row();

        for influence in &mesh_object.bone_influences {
            ui.label(&influence.bone_name);
            ui.label(influence.vertex_weights.len().to_string());
            ui.end_row();
        }
    })
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

fn edit_mesh_attributes(
    ui: &mut Ui,
    mesh_object: &mut MeshObjectData,
    missing_attributes: &[String],
) -> bool {
    let mut changed = false;

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
        // TODO: Create a size between heading and label?
        ui.label(RichText::new("Name").size(16.0));
        ui.label(RichText::new("Usage").size(16.0));
        ui.label(RichText::new("Vertex Count").size(16.0));
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
    });

    changed
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
