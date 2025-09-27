use crate::{
    EditorResponse, path::folder_editor_title, save_file, save_file_as,
    validation::AdjValidationError,
};
use egui::{ScrollArea, special_emojis::GITHUB};

use ssbh_data::{adj_data::AdjEntryData, prelude::*};
use std::path::Path;

pub fn adj_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    adj: &mut AdjData,
    mesh: Option<&MeshData>,
    validation_errors: &[AdjValidationError],
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Adj Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        saved |= save_file(adj, folder_name, file_name);
                    }

                    if ui.button("Save As...").clicked() {
                        saved |= save_file_as(adj, folder_name, file_name, "Adj", "adjb");
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Adj Editor Wiki")).clicked() {
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
                changed |= add_missing_adj_entries(adj, validation_errors, mesh);
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

    EditorResponse {
        open,
        changed,
        saved,
        message: None,
    }
}

pub fn add_missing_adj_entries(
    adj: &mut AdjData,
    validation_errors: &[AdjValidationError],
    mesh: Option<&MeshData>,
) -> bool {
    let mut changed = false;

    if let Some(mesh) = mesh {
        for e in validation_errors {
            match e {
                AdjValidationError::MissingRenormalEntry {
                    mesh_object_index, ..
                } => {
                    if let Some(mesh_object) = mesh.objects.get(*mesh_object_index) {
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

    changed
}
