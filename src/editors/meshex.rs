use egui::{CollapsingHeader, ScrollArea};
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

            // TODO: Add buttons to add missing entries and remove unused entries.
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // TODO: Use a grid instead?
                    let id = egui::Id::new("meshex");
                    for (i, group) in meshex.mesh_object_groups.iter_mut().enumerate() {
                        // TODO: Make names editable?
                        // TODO: Show the stripped name?
                        CollapsingHeader::new(&group.mesh_object_full_name)
                            .default_open(true)
                            .id_source(id.with(i))
                            .show(ui, |ui| {
                                for entry in &mut group.entry_flags {
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut entry.draw_model, "Draw Model");
                                        ui.checkbox(&mut entry.cast_shadow, "Cast Shadow");
                                    });
                                }
                            });
                    }
                });
        });

    (open, changed)
}
