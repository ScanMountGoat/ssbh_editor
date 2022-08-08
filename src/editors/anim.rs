use egui::{CollapsingHeader, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;
use std::path::Path;

pub fn anim_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    anim: &mut AnimData,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    egui::Window::new(format!("Anim Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = anim.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
                        }
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Anim", &["nuanmb"])
                            .save_file()
                        {
                            if let Err(e) = anim.write_to_file(&file) {
                                error!("Failed to save {:?}: {}", file, e);
                            }
                        }
                    }
                });

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("Anim Editor Wiki").clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Anim-Editor";
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
                    // TODO: Make names editable?
                    for group in &mut anim.groups {
                        CollapsingHeader::new(group.group_type.to_string())
                            .default_open(false)
                            .show(ui, |ui| {
                                for node in &mut group.nodes {
                                    CollapsingHeader::new(&node.name).default_open(true).show(
                                        ui,
                                        |ui| {
                                            for track in &mut node.tracks {
                                                changed |= edit_track(ui, track);
                                            }
                                        },
                                    );
                                }
                            });
                    }
                });
        });

    (open, changed)
}

fn edit_track(ui: &mut egui::Ui, track: &mut ssbh_data::anim_data::TrackData) -> bool {
    let mut changed = false;

    ui.label(&track.name);
    ui.indent("indent", |ui| {
        changed |= ui
            .checkbox(
                &mut track.scale_options.compensate_scale,
                "Compensate Scale",
            )
            .changed();

        changed |= ui
            .checkbox(&mut track.scale_options.inherit_scale, "Inherit Scale")
            .changed();

        changed |= ui
            .checkbox(
                &mut track.transform_flags.override_translation,
                "Override Translation",
            )
            .changed();

        changed |= ui
            .checkbox(
                &mut track.transform_flags.override_rotation,
                "Override Rotation",
            )
            .changed();

        changed |= ui
            .checkbox(&mut track.transform_flags.override_scale, "Override Scale")
            .changed();
    });

    changed
}
