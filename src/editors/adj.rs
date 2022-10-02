use crate::{app::folder_editor_title, validation::AdjValidationError};
use egui::ScrollArea;
use log::error;
use rfd::FileDialog;
use ssbh_data::{adj_data::AdjEntryData, prelude::*};
use std::path::Path;

pub fn adj_editor(
    ctx: &egui::Context,
    folder_name: &str,
    file_name: &str,
    adj: &mut AdjData,
    mesh: Option<&MeshData>,
    validation_errors: &[AdjValidationError],
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Adj Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = adj.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
                        }
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();

                        if let Some(file) =
                            FileDialog::new().add_filter("Adj", &["adjb"]).save_file()
                        {
                            if let Err(e) = adj.write_to_file(&file) {
                                error!("Failed to save {:?}: {}", file, e);
                            }
                        }
                    }
                });

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("Adj Editor Wiki").clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Adj-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            // TODO: Add button to remove unused entries.
            if !validation_errors.is_empty()
                && ui
                    .button(format!("Add {} missing entries", validation_errors.len()))
                    .clicked()
            {
                for e in validation_errors {
                    match e {
                        AdjValidationError::MissingRenormalEntry {
                            mesh_object_index, ..
                        } => {
                            if let Some(mesh_object) =
                                mesh.and_then(|mesh| mesh.objects.get(*mesh_object_index))
                            {
                                adj.entries.push(AdjEntryData::from_mesh_object(
                                    *mesh_object_index,
                                    mesh_object,
                                ));
                                changed = true;
                            }
                        }
                    }
                }
            }

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("adj_grid").show(ui, |ui| {
                        // TODO: How to best display adjacency data?
                        ui.heading("Mesh Object Index");
                        ui.heading("Vertex Adjacency Count");
                        ui.end_row();

                        for entry in &adj.entries {
                            // TODO: Make this a combobox or an index in advanced mode?
                            // TODO: Fallback to indices if the mesh is missing?
                            if let Some(o) =
                                mesh.and_then(|mesh| mesh.objects.get(entry.mesh_object_index))
                            {
                                ui.label(format!("{} ({})", entry.mesh_object_index, o.name));
                            } else {
                                ui.label(entry.mesh_object_index.to_string());
                            }
                            ui.label(entry.vertex_adjacency.len().to_string());
                            ui.end_row();
                        }
                    });
                });
        });

    (open, changed)
}
