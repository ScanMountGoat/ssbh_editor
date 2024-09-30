use crate::{
    app::{draggable_icon, warning_icon_text, ModlEditorState, ModlEditorTab},
    horizontal_separator_empty,
    path::folder_editor_title,
    save_file, save_file_as,
    validation::{ModlValidationError, ModlValidationErrorKind},
    EditorMessage, EditorResponse,
};
use egui::{special_emojis::GITHUB, Grid, Label, RichText, ScrollArea, TextEdit, TextWrapMode};
use egui_dnd::dnd;

use ssbh_data::{mesh_data::MeshObjectData, modl_data::ModlEntryData, prelude::*};
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
    dark_mode: bool,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;
    let mut message = None;

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
                    if ui.button("Rebuild from Mesh").clicked() {
                        ui.close_menu();

                        if let Some(mesh) = mesh {
                            changed |= rebuild_from_mesh(modl, mesh, matl);
                        }
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

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.editor_tab,
                    ModlEditorTab::Assignments,
                    RichText::new("Materials").heading(),
                );
                ui.selectable_value(
                    &mut state.editor_tab,
                    ModlEditorTab::Files,
                    RichText::new("Files").heading(),
                );
            });

            changed |= match state.editor_tab {
                ModlEditorTab::Assignments => edit_modl_entries(
                    ctx,
                    ui,
                    modl,
                    mesh,
                    matl,
                    validation_errors,
                    dark_mode,
                    &mut message,
                ),
                ModlEditorTab::Files => edit_modl_file_names(ui, modl),
            }
        });

    EditorResponse {
        open,
        changed,
        saved,
        message,
    }
}

fn rebuild_from_mesh(modl: &mut ModlData, mesh: &MeshData, matl: Option<&MatlData>) -> bool {
    let mut changed = false;

    // TODO: Optimize this?
    let missing_entries = missing_mesh_objects(modl, mesh);
    let has_unused_entries = has_unused_entries(modl, mesh);

    // Pick an arbitrary material to make the mesh visible in the viewport.
    let default_material = matl
        .and_then(|m| m.entries.first().map(|e| e.material_label.clone()))
        .unwrap_or_else(|| String::from("PLACEHOLDER"));

    if !missing_entries.is_empty() {
        for mesh in missing_entries {
            modl.entries.push(ModlEntryData {
                mesh_object_name: mesh.name.clone(),
                mesh_object_subindex: mesh.subindex,
                material_label: default_material.clone(),
            });
        }
        changed = true;
    }

    if has_unused_entries {
        modl.entries.retain(|e| {
            mesh.objects.iter().any(|mesh| {
                e.mesh_object_name == mesh.name && e.mesh_object_subindex == mesh.subindex
            })
        });
        changed = true;
    }

    changed
}

fn edit_modl_entries(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    modl: &mut ModlData,
    mesh: Option<&MeshData>,
    matl: Option<&MatlData>,
    validation_errors: &[ModlValidationError],
    dark_mode: bool,
    message: &mut Option<EditorMessage>,
) -> bool {
    let mut changed = false;

    if let Some(mesh) = mesh {
        // TODO: Optimize this?
        let missing_entries = missing_mesh_objects(modl, mesh);
        let has_unused_entries = has_unused_entries(modl, mesh);

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

        if has_unused_entries && ui.button("Remove Unused Entries").clicked() {
            changed = true;

            modl.entries.retain(|e| {
                mesh.objects.iter().any(|mesh| {
                    e.mesh_object_name == mesh.name && e.mesh_object_subindex == mesh.subindex
                })
            });
        }
    }
    horizontal_separator_empty(ui);

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let mut entry_to_remove = None;

            // TODO: Avoid allocating here.
            // TODO: Don't need a wrapper type?
            let mut items: Vec<_> = (0..modl.entries.len()).map(ModlEntryIndex).collect();

            let response = dnd(ui, "modl_dnd").show_vec(&mut items, |ui, item, handle, _| {
                ui.horizontal(|ui| {
                    let entry = &mut modl.entries[item.0];
                    let id = egui::Id::new("modl").with(item.0);

                    handle.ui(ui, |ui| {
                        draggable_icon(ctx, ui, dark_mode);
                    });

                    // Check for assignment errors for the current entry.
                    let mut valid_mesh = true;
                    let mut valid_material = true;
                    for e in validation_errors.iter().filter(|e| e.entry_index == item.0) {
                        match &e.kind {
                            ModlValidationErrorKind::InvalidMeshObject { .. } => valid_mesh = false,
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

                    // TODO: Find a way to get a grid layout working with egui_dnd.
                    let (_, rect) = ui.allocate_space(egui::vec2(300.0, 20.0));
                    let name_response = ui
                        .child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None)
                        .add(Label::new(mesh_text).sense(egui::Sense::click()));

                    name_response.context_menu(|ui| {
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
                    // TODO: This response check isn't working.
                    if name_response.hovered() {
                        *message = Some(EditorMessage::SelectMesh {
                            mesh_object_name: entry.mesh_object_name.clone(),
                            mesh_object_subindex: entry.mesh_object_subindex,
                        });
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

    changed
}

fn has_unused_entries(modl: &ModlData, mesh: &MeshData) -> bool {
    modl.entries.iter().any(|e| {
        !mesh
            .objects
            .iter()
            .any(|mesh| e.mesh_object_name == mesh.name && e.mesh_object_subindex == mesh.subindex)
    })
}

fn missing_mesh_objects<'a>(modl: &ModlData, mesh: &'a MeshData) -> Vec<&'a MeshObjectData> {
    mesh.objects
        .iter()
        .filter(|mesh| {
            !modl
                .entries
                .iter()
                .any(|e| e.mesh_object_name == mesh.name && e.mesh_object_subindex == mesh.subindex)
        })
        .collect()
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
    egui::ComboBox::from_id_salt(id)
        .selected_text(text)
        .width(300.0)
        .wrap_mode(TextWrapMode::Wrap)
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
