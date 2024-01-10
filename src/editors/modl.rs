use crate::{
    app::{draggable_icon, warning_icon_text, ModlEditorState},
    horizontal_separator_empty,
    path::folder_editor_title,
    save_file, save_file_as,
    validation::{ModlValidationError, ModlValidationErrorKind},
    EditorResponse,
};
use egui::{special_emojis::GITHUB, Grid, Label, RichText, ScrollArea, TextEdit};
use egui_dnd::dnd;

use ssbh_data::{modl_data::ModlEntryData, prelude::*};
use ssbh_wgpu::RenderModel;
use std::path::Path;

#[derive(Hash)]
struct ModlEntryIndex(usize);

pub fn modl_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    modl: &mut ModlData,
    mesh: Option<&MeshData>,
    matl: Option<&MatlData>,
    validation_errors: &[ModlValidationError],
    state: &mut ModlEditorState,
    render_model: &mut Option<&mut RenderModel>,
    dark_mode: bool,
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
                        saved |= save_file(modl, folder_name, file_name);
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();
                        saved |= save_file_as(modl, folder_name, file_name, "Modl", "numdlb");
                    }
                });

                ui.menu_button("Modl", |ui| {
                    if ui.button("Add Entry").clicked() {
                        changed = true;

                        // Pick an arbitrary material to make the mesh visible in the viewport.
                        let default_material = matl
                            .and_then(|m| m.entries.first().map(|e| e.material_label.clone()))
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
            ui.checkbox(&mut state.advanced_mode, "Advanced Settings");

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
                    .and_then(|m| m.entries.first().map(|e| e.material_label.clone()))
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
                    if state.advanced_mode {
                        changed |= edit_modl_file_names(ui, modl);
                        horizontal_separator_empty(ui);
                    }

                    let mut entry_to_remove = None;

                    // TODO: Avoid allocating here.
                    let mut items: Vec<_> = (0..modl.entries.len()).map(ModlEntryIndex).collect();

                    let response =
                        dnd(ui, "modl_dnd").show_vec(&mut items, |ui, item, handle, _| {
                            ui.horizontal(|ui| {
                                let entry = &mut modl.entries[item.0];
                                let id = egui::Id::new("modl").with(item.0);

                                handle.ui(ui, |ui| {
                                    draggable_icon(ctx, ui, dark_mode);
                                });

                                // Check for assignment errors for the current entry.
                                let mut valid_mesh = true;
                                let mut valid_material = true;
                                for e in
                                    validation_errors.iter().filter(|e| e.entry_index == item.0)
                                {
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

                                let name_response = if state.advanced_mode {
                                    let (response, name_changed) =
                                        mesh_combo_box(ui, entry, id.with("mesh"), mesh, mesh_text);
                                    changed |= name_changed;
                                    response
                                } else {
                                    // TODO: Find a way to get a grid layout working with egui_dnd.
                                    let (_, rect) = ui.allocate_space(egui::vec2(300.0, 20.0));
                                    ui.child_ui(
                                        rect,
                                        egui::Layout::left_to_right(egui::Align::Center),
                                    )
                                    .add(Label::new(mesh_text).sense(egui::Sense::click()))
                                };

                                let name_response = name_response.context_menu(|ui| {
                                    if ui.button("Delete").clicked() {
                                        ui.close_menu();
                                        entry_to_remove = Some(item.0);
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

                                // TODO: Add a menu option to match the numshb order (in game convention?).
                                // Outline the selected mesh in the viewport.
                                // Check the response first to only have to search for one render mesh.
                                if name_response.hovered() {
                                    if let Some(render_mesh) =
                                        render_model.as_mut().and_then(|model| {
                                            model.meshes.iter_mut().find(|m| {
                                                m.name == entry.mesh_object_name
                                                    && m.subindex == entry.mesh_object_subindex
                                            })
                                        })
                                    {
                                        render_mesh.is_selected = true;
                                    }
                                }
                            });
                        });

                    if let Some(i) = entry_to_remove {
                        modl.entries.remove(i);
                    }

                    if let Some(response) = response.final_update() {
                        egui_dnd::utils::shift_vec(response.from, response.to, &mut modl.entries);
                        changed = true;
                    }
                });
        });

    EditorResponse {
        open,
        changed,
        saved,
    }
}

fn edit_modl_file_names(ui: &mut egui::Ui, modl: &mut ModlData) -> bool {
    let mut changed = false;

    ui.heading("Model Files");
    Grid::new("modl_files_grid").show(ui, |ui| {
        let size = [125.0, 20.0];
        ui.label("Model Name");
        changed |= ui
            .add_sized(size, TextEdit::singleline(&mut modl.model_name))
            .changed();
        ui.end_row();

        ui.label("Skeleton File Name");
        changed |= ui
            .add_sized(size, TextEdit::singleline(&mut modl.skeleton_file_name))
            .changed();
        ui.end_row();

        ui.label("Material File Names");
        for file_name in &mut modl.material_file_names {
            changed |= ui
                .add_sized(size, TextEdit::singleline(file_name))
                .changed();
        }
        ui.end_row();

        ui.label("Animation File Name");
        if let Some(file_name) = modl.animation_file_name.as_mut() {
            changed |= ui
                .add_sized(size, TextEdit::singleline(file_name))
                .changed();
            if ui.button("Remove").clicked() {
                modl.animation_file_name = None;
                changed = true;
            }
        } else if ui.button("Add File").clicked() {
            modl.animation_file_name = Some("model.nuanmb".to_string());
            changed = true;
        }
        ui.end_row();

        ui.label("Mesh File Name");
        changed |= ui
            .add_sized(size, TextEdit::singleline(&mut modl.mesh_file_name))
            .changed();
        ui.end_row();
    });

    changed
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
