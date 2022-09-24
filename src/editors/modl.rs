use crate::{
    app::warning_icon_text,
    validation::{ModlValidationError, ModlValidationErrorKind},
};
use egui::{RichText, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::{modl_data::ModlEntryData, prelude::*};
use std::path::Path;

pub fn modl_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    modl: &mut ModlData,
    mesh: Option<&MeshData>,
    matl: Option<&MatlData>,
    validation_errors: &[ModlValidationError],
    advanced_mode: &mut bool,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    egui::Window::new(format!("Modl Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = modl.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
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

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("Modl Editor Wiki").clicked() {
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
            ui.checkbox(advanced_mode, "Advanced Settings");

            // Manually adding entries is error prone, so check for advanced mode.
            if *advanced_mode && ui.button("Add Entry").clicked() {
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

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("modl_grid").striped(true).show(ui, |ui| {
                        // Header
                        // TODO: There are three ways to display duplicate object names.
                        // 1. "object.0", "object.1"
                        // 2. "object", "object"
                        // 3. hierarchy with "object0" and "object1" as children of "object"
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

                            if *advanced_mode {
                                changed |= mesh_name_combo_box(
                                    ui,
                                    &mut entry.mesh_object_name,
                                    id.with("mesh"),
                                    mesh,
                                    valid_mesh,
                                );
                            } else {
                                if valid_mesh {
                                    ui.label(&entry.mesh_object_name);
                                } else {
                                    ui.label(warning_icon_text(&entry.mesh_object_name));
                                }
                            }

                            changed |= material_label_combo_box(
                                ui,
                                &mut entry.material_label,
                                id.with("matl"),
                                matl,
                                valid_material,
                            );

                            if *advanced_mode && ui.button("Delete").clicked() {
                                changed = true;
                                entry_to_remove = Some(i)
                            }
                            ui.end_row();
                        }

                        if let Some(i) = entry_to_remove {
                            modl.entries.remove(i);
                        }
                    });
                });
        });

    (open, changed)
}

// TODO: Create a function that handles displaying combo box errors?
fn mesh_name_combo_box(
    ui: &mut egui::Ui,
    mesh_name: &mut String,
    id: impl std::hash::Hash,
    mesh: Option<&MeshData>,
    is_valid: bool,
) -> bool {
    let mut changed = false;
    let text = if is_valid {
        RichText::new(mesh_name.as_str())
    } else {
        warning_icon_text(mesh_name)
    };
    egui::ComboBox::from_id_source(id)
        .selected_text(text)
        .width(300.0)
        .show_ui(ui, |ui| {
            // TODO: Just use text boxes if the mesh is missing?
            if let Some(mesh) = mesh {
                for mesh in &mesh.objects {
                    changed |= ui
                        .selectable_value(mesh_name, mesh.name.to_string(), &mesh.name)
                        .changed();
                }
            }
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
