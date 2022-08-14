use std::path::Path;

use egui::{CollapsingHeader, DragValue, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;

use crate::widgets::{bone_combo_box, DragSlider};

pub fn hlpb_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    hlpb: &mut HlpbData,
    skel: Option<&SkelData>,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = true;

    egui::Window::new(format!("Hlpb Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = hlpb.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
                        }
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Hlpb", &["nuhlpb"])
                            .save_file()
                        {
                            if let Err(e) = hlpb.write_to_file(&file) {
                                error!("Failed to save {:?}: {}", file, e);
                            }
                        }
                    }
                });

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("Hlpb Editor Wiki").clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Hlpb-Editor";
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
                    // TODO: Use a layout similar to the matl editor to support more fields.
                    // TODO: Add and delete entries.
                    if !hlpb.aim_constraints.is_empty() {
                        changed |= aim_constraints(ui, hlpb, skel);
                    }

                    if !hlpb.orient_constraints.is_empty() {
                        changed |= orient_constraints(ui, hlpb, skel);
                    }
                });
        });

    (open, changed)
}

fn orient_constraints(ui: &mut egui::Ui, hlpb: &mut HlpbData, skel: Option<&SkelData>) -> bool {
    let mut changed = false;
    CollapsingHeader::new("Orient Constraints")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("orient").striped(true).show(ui, |ui| {
                ui.heading("Name");
                ui.heading("Parent 1");
                ui.heading("Parent 2");
                ui.heading("Source");
                ui.heading("Target");
                ui.heading("Unk Type");
                ui.heading("Constraint Axes");
                ui.end_row();

                for (i, orient) in hlpb.orient_constraints.iter_mut().enumerate() {
                    let id = egui::Id::new("orient").with(i);

                    ui.label(&orient.name);
                    changed |=
                        bone_combo_box(ui, &mut orient.parent_bone1_name, id.with(0), skel, &[]);
                    changed |=
                        bone_combo_box(ui, &mut orient.parent_bone2_name, id.with(1), skel, &[]);
                    changed |=
                        bone_combo_box(ui, &mut orient.source_bone_name, id.with(2), skel, &[]);
                    changed |=
                        bone_combo_box(ui, &mut orient.target_bone_name, id.with(3), skel, &[]);

                    egui::ComboBox::from_id_source(id.with(4))
                        .selected_text(orient.unk_type.to_string())
                        .show_ui(ui, |ui| {
                            changed |= ui.selectable_value(&mut orient.unk_type, 1, "1").changed();
                            changed |= ui.selectable_value(&mut orient.unk_type, 2, "2").changed();
                        });

                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(
                                DragSlider::new(id.with(5), &mut orient.constraint_axes.x)
                                    .width(40.0),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                DragSlider::new(id.with(6), &mut orient.constraint_axes.y)
                                    .width(40.0),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                DragSlider::new(id.with(7), &mut orient.constraint_axes.z)
                                    .width(40.0),
                            )
                            .changed();
                    });

                    ui.end_row();
                }
            });
        });
    changed
}

fn aim_constraints(ui: &mut egui::Ui, hlpb: &mut HlpbData, skel: Option<&SkelData>) -> bool {
    let mut changed = false;
    CollapsingHeader::new("Aim Constraints")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("aim").striped(true).show(ui, |ui| {
                ui.heading("Name");
                ui.heading("Aim 1");
                ui.heading("Aim 2");
                ui.heading("Type 1");
                ui.heading("Type 2");
                ui.heading("Target 1");
                ui.heading("Target 2");
                ui.heading("Unk1");
                ui.heading("Unk2");
                ui.end_row();

                for (i, aim) in hlpb.aim_constraints.iter_mut().enumerate() {
                    let id = egui::Id::new("aim").with(i);

                    ui.label(&aim.name);
                    changed |= bone_combo_box(ui, &mut aim.aim_bone_name1, id.with(0), skel, &[]);
                    changed |= bone_combo_box(ui, &mut aim.aim_bone_name2, id.with(1), skel, &[]);
                    changed |=
                        bone_combo_box(ui, &mut aim.aim_type1, id.with(2), skel, &["DEFAULT"]);
                    changed |=
                        bone_combo_box(ui, &mut aim.aim_type2, id.with(3), skel, &["DEFAULT"]);
                    changed |=
                        bone_combo_box(ui, &mut aim.target_bone_name1, id.with(4), skel, &[]);
                    changed |=
                        bone_combo_box(ui, &mut aim.target_bone_name2, id.with(5), skel, &[]);
                    changed |= ui.add(DragValue::new(&mut aim.unk1)).changed();
                    changed |= ui.add(DragValue::new(&mut aim.unk2)).changed();
                    ui.end_row();
                }
            });
        });
    changed
}
