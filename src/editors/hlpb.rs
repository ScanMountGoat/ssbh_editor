use std::path::Path;

use crate::{
    path::folder_editor_title,
    save_file, save_file_as,
    widgets::{bone_combo_box, DragSlider},
    EditorResponse,
};
use egui::{special_emojis::GITHUB, CollapsingHeader, DragValue, Grid, ScrollArea, TextEdit, Ui};

use ssbh_data::{
    hlpb_data::{AimConstraintData, OrientConstraintData},
    prelude::*,
    Vector3, Vector4,
};

pub fn hlpb_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    hlpb: &mut HlpbData,
    skel: Option<&SkelData>,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Hlpb Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();
                        saved |= save_file(hlpb, folder_name, file_name);
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();
                        saved |= save_file_as(hlpb, folder_name, file_name, "Hlpb", "nuhlpb");
                    }
                });

                ui.menu_button("Constraint", |ui| {
                    if ui.button("Add Aim Constraint").clicked() {
                        ui.close_menu();

                        // Create a unique name for the new constraint.
                        // TODO: Increment the ID at the end instead (requires tests).
                        hlpb.aim_constraints.push(AimConstraintData {
                            name: hlpb
                                .aim_constraints
                                .iter()
                                .map(|a| &a.name)
                                .max()
                                .map(|n| n.to_owned() + "1")
                                .unwrap_or_else(|| "nuHelperBoneRotateAim1".to_owned()),
                            aim_bone_name1: String::new(),
                            aim_bone_name2: String::new(),
                            aim_type1: "DEFAULT".to_owned(),
                            aim_type2: "DEFAULT".to_owned(),
                            target_bone_name1: String::new(),
                            target_bone_name2: String::new(),
                            unk1: 0,
                            unk2: 1,
                            aim: Vector3::new(1.0, 0.0, 0.0),
                            up: Vector3::new(0.0, 1.0, 0.0),
                            quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                            quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
                        });
                        changed = true;
                    }

                    if ui.button("Add Orient Constraint").clicked() {
                        ui.close_menu();

                        // Create a unique name for the new constraint.
                        // TODO: Increment the ID at the end instead (requires tests).
                        hlpb.orient_constraints.push(OrientConstraintData {
                            name: hlpb
                                .orient_constraints
                                .iter()
                                .map(|o| &o.name)
                                .max()
                                .map(|n| n.to_owned() + "1")
                                .unwrap_or_else(|| "nuHelperBoneRotateInterp1".to_owned()),
                            parent_bone_name1: String::new(),
                            parent_bone_name2: String::new(),
                            source_bone_name: String::new(),
                            target_bone_name: String::new(),
                            unk_type: 1,
                            constraint_axes: Vector3::new(1.0, 1.0, 1.0),
                            quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                            quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
                            range_min: Vector3::new(-180.0, -180.0, -180.0),
                            range_max: Vector3::new(180.0, 180.0, 180.0),
                        });
                        changed = true;
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Hlpb Editor Wiki")).clicked() {
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
                    if !hlpb.aim_constraints.is_empty() {
                        changed |= aim_constraints(ui, hlpb, skel);
                    }

                    if !hlpb.orient_constraints.is_empty() {
                        changed |= orient_constraints(ui, hlpb, skel);
                    }
                });
        });

    EditorResponse {
        open,
        changed,
        saved,
    }
}

fn orient_constraints(ui: &mut Ui, hlpb: &mut HlpbData, skel: Option<&SkelData>) -> bool {
    let mut changed = false;
    CollapsingHeader::new("Orient Constraints")
        .default_open(true)
        .show(ui, |ui| {
            let mut entry_to_remove = None;
            for (i, o) in hlpb.orient_constraints.iter_mut().enumerate() {
                let id = egui::Id::new("orient").with(i);

                // Append the helper bone name to make it easier to find constraints.
                CollapsingHeader::new(format!("{} ({})", o.name, o.target_bone_name))
                    .id_source(id.with(&o.name))
                    .default_open(false)
                    .show(ui, |ui| {
                        Grid::new(id).show(ui, |ui| {
                            ui.label("Name");
                            changed |= ui
                                .add_sized([200.0, 20.0], TextEdit::singleline(&mut o.name))
                                .changed();
                            ui.end_row();

                            ui.label("Parent 1");
                            changed |=
                                bone_combo_box(ui, &mut o.parent_bone_name1, id.with(0), skel, &[]);
                            ui.end_row();

                            ui.label("Parent 2");
                            changed |=
                                bone_combo_box(ui, &mut o.parent_bone_name2, id.with(1), skel, &[]);
                            ui.end_row();

                            ui.label("Source");
                            changed |=
                                bone_combo_box(ui, &mut o.source_bone_name, id.with(2), skel, &[]);
                            ui.end_row();

                            ui.label("Target");
                            changed |=
                                bone_combo_box(ui, &mut o.target_bone_name, id.with(3), skel, &[]);
                            ui.end_row();

                            // TODO: Make this an enum in ssbh_data eventually.
                            ui.label("Unk Type");
                            egui::ComboBox::from_id_source(id.with(4))
                                .selected_text(o.unk_type.to_string())
                                .show_ui(ui, |ui| {
                                    changed |=
                                        ui.selectable_value(&mut o.unk_type, 0, "0").changed();
                                    changed |=
                                        ui.selectable_value(&mut o.unk_type, 1, "1").changed();
                                    changed |=
                                        ui.selectable_value(&mut o.unk_type, 2, "2").changed();
                                });
                            ui.end_row();

                            ui.label("Constraint Axes");
                            changed |=
                                edit_vector3(ui, id.with(5), &mut o.constraint_axes, 0.0, 1.0);
                            ui.end_row();

                            ui.label("Quat 1");
                            changed |= edit_vector4(ui, id.with(6), &mut o.quat1);
                            ui.end_row();

                            ui.label("Quat 2");
                            changed |= edit_vector4(ui, id.with(7), &mut o.quat2);
                            ui.end_row();

                            ui.label("Range Min");
                            changed |=
                                edit_vector3(ui, id.with(8), &mut o.range_min, -180.0, 180.0);
                            ui.end_row();

                            ui.label("Range Max");
                            changed |=
                                edit_vector3(ui, id.with(9), &mut o.range_max, -180.0, 180.0);
                            ui.end_row();
                        });
                    })
                    .header_response
                    .context_menu(|ui| {
                        if ui.button("Delete").clicked() {
                            ui.close_menu();

                            entry_to_remove = Some(i);
                            changed = true;
                        }
                    });
            }

            if let Some(i) = entry_to_remove {
                hlpb.orient_constraints.remove(i);
            }
        });
    changed
}

fn aim_constraints(ui: &mut Ui, hlpb: &mut HlpbData, skel: Option<&SkelData>) -> bool {
    let mut changed = false;
    CollapsingHeader::new("Aim Constraints")
        .default_open(true)
        .show(ui, |ui| {
            let mut entry_to_remove = None;
            for (i, aim) in hlpb.aim_constraints.iter_mut().enumerate() {
                let id = egui::Id::new("aim").with(i);

                // Append the helper bone names to make it easier to find constraints.
                CollapsingHeader::new(format!(
                    "{} ({} / {})",
                    aim.name, aim.target_bone_name1, aim.target_bone_name2
                ))
                .id_source(id.with(&aim.name))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new(id).show(ui, |ui| {
                        ui.label("Name");
                        changed |= ui
                            .add_sized([200.0, 20.0], TextEdit::singleline(&mut aim.name))
                            .changed();
                        ui.end_row();

                        ui.label("Aim 1");
                        changed |=
                            bone_combo_box(ui, &mut aim.aim_bone_name1, id.with(0), skel, &[]);
                        ui.end_row();

                        ui.label("Aim 2");
                        changed |=
                            bone_combo_box(ui, &mut aim.aim_bone_name2, id.with(1), skel, &[]);
                        ui.end_row();

                        ui.label("Aim Type 1");
                        changed |=
                            bone_combo_box(ui, &mut aim.aim_type1, id.with(2), skel, &["DEFAULT"]);
                        ui.end_row();

                        ui.label("Aim Type 2");
                        changed |=
                            bone_combo_box(ui, &mut aim.aim_type2, id.with(3), skel, &["DEFAULT"]);
                        ui.end_row();

                        ui.label("Target 1");
                        changed |=
                            bone_combo_box(ui, &mut aim.target_bone_name1, id.with(4), skel, &[]);
                        ui.end_row();

                        ui.label("Target 2");
                        changed |=
                            bone_combo_box(ui, &mut aim.target_bone_name2, id.with(5), skel, &[]);
                        ui.end_row();

                        ui.label("Unk1");
                        changed |= ui.add(DragValue::new(&mut aim.unk1)).changed();
                        ui.end_row();

                        ui.label("Unk2");
                        changed |= ui.add(DragValue::new(&mut aim.unk2)).changed();
                        ui.end_row();

                        ui.label("Aim");
                        changed |= edit_vector3(ui, id.with(6), &mut aim.aim, 0.0, 1.0);
                        ui.end_row();

                        ui.label("Up");
                        changed |= edit_vector3(ui, id.with(7), &mut aim.up, 0.0, 1.0);
                        ui.end_row();

                        ui.label("Quat 1");
                        changed |= edit_vector4(ui, id.with(8), &mut aim.quat1);
                        ui.end_row();

                        ui.label("Quat 2");
                        changed |= edit_vector4(ui, id.with(9), &mut aim.quat2);
                        ui.end_row();
                    });
                })
                .header_response
                .context_menu(|ui| {
                    if ui.button("Delete").clicked() {
                        ui.close_menu();
                        entry_to_remove = Some(i);
                        changed = true;
                    }
                });
            }

            if let Some(i) = entry_to_remove {
                hlpb.aim_constraints.remove(i);
            }
        });
    changed
}

fn edit_vector3(ui: &mut Ui, id: egui::Id, value: &mut Vector3, min: f32, max: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .add(
                DragSlider::new(id.with("x"), &mut value.x)
                    .width(40.0)
                    .range(min, max),
            )
            .changed();
        changed |= ui
            .add(
                DragSlider::new(id.with("y"), &mut value.y)
                    .width(40.0)
                    .range(min, max),
            )
            .changed();
        changed |= ui
            .add(
                DragSlider::new(id.with("z"), &mut value.z)
                    .width(40.0)
                    .range(min, max),
            )
            .changed();
    });
    changed
}

fn edit_vector4(ui: &mut Ui, id: egui::Id, value: &mut Vector4) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .add(DragSlider::new(id.with("x"), &mut value.x).width(40.0))
            .changed();
        changed |= ui
            .add(DragSlider::new(id.with("y"), &mut value.y).width(40.0))
            .changed();
        changed |= ui
            .add(DragSlider::new(id.with("z"), &mut value.z).width(40.0))
            .changed();
        changed |= ui
            .add(DragSlider::new(id.with("w"), &mut value.w).width(40.0))
            .changed();
    });
    changed
}
