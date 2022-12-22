use crate::{
    app::{folder_editor_title, warning_icon_text, ModlEditorState},
    horizontal_separator_empty,
    validation::{ModlValidationError, ModlValidationErrorKind},
    EditorResponse,
};
use egui::{special_emojis::GITHUB, Grid, Label, RichText, ScrollArea, TextEdit};
use log::error;
use rfd::FileDialog;
use ssbh_data::{modl_data::ModlEntryData, prelude::*};
use ssbh_wgpu::RenderModel;
use std::path::Path;

pub fn modl_editor(
    ctx: &egui::Context,
    folder_name: &str,
    file_name: &str,
    modl: &mut ModlData,
    mesh: Option<&MeshData>,
    matl: Option<&MatlData>,
    validation_errors: &[ModlValidationError],
    editor_state: &mut ModlEditorState,
    render_model: &mut Option<&mut RenderModel>,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Modl Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = modl.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
                        } else {
                            saved = true;
                        }
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Modl", &["numdlb"])
                            .save_file()
                        {
                            if let Err(e) = modl.write_to_file(&file) {
                                error!("Failed to save {:?}: {}", file, e);
                            }
                        }
                    }
                });

                ui.menu_button("Modl", |ui| {
                    if ui.button("Add Entry").clicked() {
                        changed = true;

                        // Pick an arbitrary material to make the mesh visible in the viewport.
                        let default_material = matl
                            .and_then(|m| m.entries.get(0).map(|e| e.material_label.clone()))
                            .unwrap_or_else(|| String::from("PLACEHOLDER"));

                        modl.entries.push(ModlEntryData {
                            mesh_object_name: String::from("PLACEHOLDER"),
                            mesh_object_subindex: 0,
                            material_label: default_material,
                        });
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Modl Editor Wiki")).clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Modl-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            // Advanced mode has more detailed information that most users won't want to edit.
            ui.checkbox(&mut editor_state.advanced_mode, "Advanced Settings");

            if let Some(mesh) = mesh {
                // TODO: Optimize this?
                let missing_entries: Vec<_> = mesh
                    .objects
                    .iter()
                    .filter(|mesh| {
                        !modl.entries.iter().any(|e| {
                            e.mesh_object_name == mesh.name
                                && e.mesh_object_subindex == mesh.subindex
                        })
                    })
                    .collect();

                // Pick an arbitrary material to make the mesh visible in the viewport.
                let default_material = matl
                    .and_then(|m| m.entries.get(0).map(|e| e.material_label.clone()))
                    .unwrap_or_else(|| String::from("PLACEHOLDER"));

                if !missing_entries.is_empty() && ui.button("Add Missing Entries").clicked() {
                    changed = true;

                    for mesh in missing_entries {
                        modl.entries.push(ModlEntryData {
                            mesh_object_name: mesh.name.clone(),
                            mesh_object_subindex: mesh.subindex,
                            material_label: default_material.clone(),
                        });
                    }
                }
            }
            horizontal_separator_empty(ui);

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if editor_state.advanced_mode {
                        ui.heading("Model Files");
                        Grid::new("modl_files_grid").show(ui, |ui| {
                            let size = [125.0, 20.0];
                            ui.label("Model Name");
                            ui.add_sized(size, TextEdit::singleline(&mut modl.model_name));
                            ui.end_row();

                            ui.label("Skeleton File Name");
                            ui.add_sized(size, TextEdit::singleline(&mut modl.skeleton_file_name));
                            ui.end_row();

                            // TODO: Only a single material name should be editable..
                            ui.label("Animation File Name");
                            ui.add_sized(size, TextEdit::singleline(&mut String::new()));
                            ui.end_row();

                            // TODO: Edit the animation name.
                            ui.label("Animation File Name");
                            ui.add_sized(size, TextEdit::singleline(&mut String::new()));
                            ui.end_row();

                            ui.label("Mesh File Name");
                            ui.add_sized(size, TextEdit::singleline(&mut modl.mesh_file_name));
                            ui.end_row();
                        });
                    }

                    Grid::new("modl_entries_grid").striped(true).show(ui, |ui| {
                        ui.heading("Mesh Object");
                        ui.heading("Material Label");
                        ui.end_row();

                        let mut entry_to_remove = None;
                        for (i, entry) in modl.entries.iter_mut().enumerate() {
                            let id = egui::Id::new("modl").with(i);

                            // Check for assignment errors for the current entry.
                            let mut valid_mesh = true;
                            let mut valid_material = true;
                            for e in validation_errors.iter().filter(|e| e.entry_index == i) {
                                match &e.kind {
                                    ModlValidationErrorKind::InvalidMeshObject { .. } => {
                                        valid_mesh = false
                                    }
                                    ModlValidationErrorKind::InvalidMaterial { .. } => {
                                        valid_material = false
                                    }
                                }
                            }

                            // Show errors for the selected mesh object for this entry.
                            let mesh_text = if valid_mesh {
                                RichText::new(&entry.mesh_object_name)
                            } else {
                                warning_icon_text(&entry.mesh_object_name)
                            };

                            // TODO: Highlight the mesh in the viewport on hover.
                            let name_response = if editor_state.advanced_mode {
                                let (response, name_changed) =
                                    mesh_combo_box(ui, entry, id.with("mesh"), mesh, mesh_text);
                                changed |= name_changed;
                                response
                            } else {
                                ui.add(Label::new(mesh_text).sense(egui::Sense::click()))
                            };

                            let name_response = name_response.context_menu(|ui| {
                                if ui.button("Delete").clicked() {
                                    ui.close_menu();
                                    entry_to_remove = Some(i);
                                    changed = true;
                                }
                            });

                            changed |= material_label_combo_box(
                                ui,
                                &mut entry.material_label,
                                id.with("matl"),
                                matl,
                                valid_material,
                            );
                            ui.end_row();

                            if let Some(render_mesh) =
                                render_model.as_mut().and_then(|m| m.meshes.get_mut(i))
                            {
                                // Outline the selected mesh in the viewport.
                                render_mesh.is_selected |= name_response.hovered();
                            }
                        }

                        if let Some(i) = entry_to_remove {
                            modl.entries.remove(i);
                        }
                    });
                });
        });

    EditorResponse {
        open,
        changed,
        saved,
    }
}

// TODO: Create a function that handles displaying combo box errors?
fn mesh_combo_box(
    ui: &mut egui::Ui,
    entry: &mut ModlEntryData,
    id: impl std::hash::Hash,
    mesh: Option<&MeshData>,
    selected_text: RichText,
) -> (egui::Response, bool) {
    let mut changed = false;
    let response = egui::ComboBox::from_id_source(id)
        .selected_text(selected_text)
        .width(300.0)
        .show_ui(ui, |ui| {
            // TODO: Just use text boxes if the mesh is missing?
            if let Some(mesh) = mesh {
                for mesh in &mesh.objects {
                    if ui
                        .selectable_label(
                            entry.mesh_object_name == mesh.name
                                && entry.mesh_object_subindex == mesh.subindex,
                            &mesh.name,
                        )
                        .clicked()
                    {
                        entry.mesh_object_name = mesh.name.clone();
                        entry.mesh_object_subindex = mesh.subindex;
                        changed = true;
                    }
                }
            }
        })
        .response;
    (response, changed)
}

fn material_label_combo_box(
    ui: &mut egui::Ui,
    material_label: &mut String,
    id: impl std::hash::Hash,
    matl: Option<&MatlData>,
    is_valid: bool,
) -> bool {
    let mut changed = false;

    let text = if is_valid {
        RichText::new(material_label.as_str())
    } else {
        warning_icon_text(material_label)
    };
    egui::ComboBox::from_id_source(id)
        .selected_text(text)
        .width(400.0)
        .show_ui(ui, |ui| {
            // TODO: Just use text boxes if the matl is missing?
            if let Some(matl) = matl {
                for label in matl.entries.iter().map(|e| &e.material_label) {
                    changed |= ui
                        .selectable_value(material_label, label.to_string(), label)
                        .changed();
                }
            }
        });
    changed
}
