use egui::{Grid, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;
use std::path::Path;

pub fn meshex_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    meshex: &mut MeshExData,
    mesh: Option<&MeshData>,
) -> (bool, bool) {
    let mut open = true;
    let changed = false;

    egui::Window::new(format!("MeshEx Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = meshex.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
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

                egui::menu::menu_button(ui, "MeshEx", |ui| {
                    if ui
                        .add_enabled(mesh.is_some(), egui::Button::new("Rebuild from mesh"))
                        .clicked()
                    {
                        ui.close_menu();

                        if let Some(mesh) = mesh {
                            // TODO: TODO: Only show this if the entries don't match up?
                            // TODO: Preserve existing flags?
                            *meshex = MeshExData::from_mesh_objects(&mesh.objects);
                        }
                    }
                });

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("MeshEx Editor Wiki").clicked() {
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
                                ui.checkbox(&mut entry.draw_model, "Draw Model");
                                ui.checkbox(&mut entry.cast_shadow, "Cast Shadow");
                                ui.end_row();
                            }
                        }
                    });
                });
        });

    (open, changed)
}
