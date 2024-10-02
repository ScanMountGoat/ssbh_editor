use crate::{path::folder_editor_title, save_file, save_file_as, EditorMessage, EditorResponse};
use egui::{special_emojis::GITHUB, Grid, Label, Response, ScrollArea, Sense, Ui};

use ssbh_data::prelude::*;
use std::path::Path;

pub fn meshex_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    meshex: &mut MeshExData,
    mesh: Option<&MeshData>,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;
    let mut message = None;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("MeshEx Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();
                        saved |= save_file(meshex, folder_name, file_name);
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();
                        saved |= save_file_as(meshex, folder_name, file_name, "MeshEx", "numshexb");
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
                    Grid::new("meshex_grid").num_columns(4).show(ui, |ui| {
                        ui.heading("Full Name");
                        ui.heading("Name");
                        ui.label("");
                        ui.label("");
                        ui.end_row();

                        for group in &mut meshex.mesh_object_groups {
                            for (subindex, entry) in group.entry_flags.iter_mut().enumerate() {
                                // Get responses outside the if condition to always show both labels.
                                let response1 = hoverable_label(ui, &group.mesh_object_full_name);
                                let response2 = hoverable_label(ui, &group.mesh_object_name);

                                // TODO: Return a message enum instead.
                                if response1.hovered() || response2.hovered() {
                                    // Outline the selected mesh in the viewport.
                                    message = Some(EditorMessage::SelectMesh {
                                        mesh_object_name: group.mesh_object_full_name.clone(),
                                        mesh_object_subindex: subindex as u64,
                                    });
                                }

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
        message,
    }
}

fn hoverable_label(ui: &mut Ui, label: &str) -> Response {
    ui.add(Label::new(label).sense(Sense::click()))
}
