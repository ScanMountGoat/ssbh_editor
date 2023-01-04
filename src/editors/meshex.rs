use crate::{path::folder_editor_title, EditorResponse};
use egui::{special_emojis::GITHUB, Grid, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;
use std::path::Path;

pub fn meshex_editor(
    ctx: &egui::Context,
    folder_name: &str,
    file_name: &str,
    meshex: &mut MeshExData,
    mesh: Option<&MeshData>,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("MeshEx Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = meshex.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
                        } else {
                            saved = true;
                        }
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("MeshEx", &["numshexb"])
                            .save_file()
                        {
                            if let Err(e) = meshex.write_to_file(&file) {
                                error!("Failed to save {:?}: {}", file, e);
                            }
                        }
                    }
                });

                ui.menu_button("MeshEx", |ui| {
                    if ui
                        .add_enabled(mesh.is_some(), egui::Button::new("Rebuild From Mesh"))
                        .clicked()
                    {
                        ui.close_menu();

                        if let Some(mesh) = mesh {
                            // TODO: TODO: Only show this if the entries don't match up?
                            // TODO: Preserve existing flags?
                            *meshex = MeshExData::from_mesh_objects(&mesh.objects);
                            changed = true;
                        }
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} MeshEx Editor Wiki")).clicked() {
                        ui.close_menu();

                        let link =
                            "https://github.com/ScanMountGoat/ssbh_editor/wiki/MeshEx-Editor";
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
                    // TODO: Show bounding sphere values?
                    Grid::new("meshex_grid").show(ui, |ui| {
                        ui.heading("Full Name");
                        ui.heading("Name");
                        ui.end_row();

                        for group in &mut meshex.mesh_object_groups {
                            for entry in &mut group.entry_flags {
                                ui.label(&group.mesh_object_full_name);
                                ui.label(&group.mesh_object_name);
                                changed |=
                                    ui.checkbox(&mut entry.draw_model, "Draw Model").changed();
                                changed |=
                                    ui.checkbox(&mut entry.cast_shadow, "Cast Shadow").changed();
                                ui.end_row();
                            }
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
