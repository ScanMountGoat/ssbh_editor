use std::path::Path;

use crate::{
    EditorResponse,
    app::{HlpbEditorState, HlpbEditorTab, emoji::GITHUB},
    horizontal_separator_empty,
    path::folder_editor_title,
    save_file, save_file_as,
    widgets::{DragSlider, bone_combo_box},
};
use egui::{CentralPanel, DragValue, Grid, Panel, RichText, ScrollArea, TextEdit, Ui};

use glam::{Quat, Vec3, quat, vec3};
use ssbh_data::{
    hlpb_data::{AimConstraintData, OrientConstraintData},
    prelude::*,
};

pub fn hlpb_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    hlpb: &mut HlpbData,
    skel: Option<&SkelData>,
    state: &mut HlpbEditorState,
) -> EditorResponse {
    let mut open = true;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Hlpb Editor ({title})"))
        .open(&mut open)
        .default_size(egui::Vec2::new(700.0, 600.0))
        .resizable(true)
        .show(ctx, |ui| {
            let menu_saved = menu_bar(ui, hlpb, folder_name, file_name, state);
            saved |= menu_saved;
            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.editor_tab,
                    HlpbEditorTab::Orient,
                    RichText::new("Orient Constraints").heading(),
                );
                ui.selectable_value(
                    &mut state.editor_tab,
                    HlpbEditorTab::Aim,
                    RichText::new("Aim Constraints").heading(),
                );
            });
            horizontal_separator_empty(ui);

            Panel::left("hlpb_left_panel")
                .default_size(450.0)
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            select_constraint(hlpb, state, ui);
                        });
                });

            CentralPanel::default().show(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| match state.editor_tab {
                        HlpbEditorTab::Orient => {
                            orient_constraints(ui, hlpb, skel, state);
                        }
                        HlpbEditorTab::Aim => {
                            aim_constraints(ui, hlpb, skel, state);
                        }
                    });
            });
        });

    EditorResponse {
        open,
        changed: false,
        saved,
        message: None,
    }
}

fn select_constraint(hlpb: &mut HlpbData, state: &mut HlpbEditorState, ui: &mut Ui) {
    match state.editor_tab {
        HlpbEditorTab::Orient => {
            let mut index_to_delete = None;

            for (i, o) in hlpb.orient_constraints.iter().enumerate() {
                // Append the helper bone name to make it easier to find constraints.
                ui.selectable_value(
                    &mut state.orient_constraint_index,
                    i,
                    format!("{} ({})", o.name, o.target_bone_name),
                )
                .context_menu(|ui| {
                    if ui.button("Delete").clicked() {
                        index_to_delete = Some(i);
                    }
                });
            }

            if let Some(i) = index_to_delete {
                hlpb.orient_constraints.remove(i);
            }
        }
        HlpbEditorTab::Aim => {
            let mut index_to_delete = None;

            for (i, a) in hlpb.aim_constraints.iter().enumerate() {
                // Append the helper bone name to make it easier to find constraints.
                ui.selectable_value(
                    &mut state.aim_constraint_index,
                    i,
                    format!(
                        "{} ({} / {})",
                        a.name, a.target_bone_name1, a.target_bone_name2
                    ),
                )
                .context_menu(|ui| {
                    if ui.button("Delete").clicked() {
                        index_to_delete = Some(i);
                    }
                });
            }

            if let Some(i) = index_to_delete {
                hlpb.aim_constraints.remove(i);
            }
        }
    }
}

fn menu_bar(
    ui: &mut Ui,
    hlpb: &mut HlpbData,
    folder_name: &Path,
    file_name: &str,
    state: &mut HlpbEditorState,
) -> bool {
    let mut saved = false;
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Save").clicked() {
                saved |= save_file(hlpb, folder_name, file_name);
            }

            if ui.button("Save As...").clicked() {
                saved |= save_file_as(hlpb, folder_name, file_name, "Hlpb", "nuhlpb");
            }
        });

        ui.menu_button("Constraint", |ui| {
            if ui.button("Add New").clicked() {
                match state.editor_tab {
                    HlpbEditorTab::Orient => {
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
                            constraint_axes: vec3(1.0, 1.0, 1.0),
                            quat1: quat(0.0, 0.0, 0.0, 1.0),
                            quat2: quat(0.0, 0.0, 0.0, 1.0),
                            range_min: vec3(-180.0, -180.0, -180.0),
                            range_max: vec3(180.0, 180.0, 180.0),
                        });
                    }
                    HlpbEditorTab::Aim => {
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
                            aim: vec3(1.0, 0.0, 0.0),
                            up: vec3(0.0, 1.0, 0.0),
                            quat1: quat(0.0, 0.0, 0.0, 1.0),
                            quat2: quat(0.0, 0.0, 0.0, 1.0),
                        });
                    }
                }
            }

            if ui.button("Delete").clicked() {
                match state.editor_tab {
                    HlpbEditorTab::Orient => {
                        if hlpb
                            .orient_constraints
                            .get(state.orient_constraint_index)
                            .is_some()
                        {
                            hlpb.orient_constraints
                                .remove(state.orient_constraint_index);
                        }
                    }
                    HlpbEditorTab::Aim => {
                        if hlpb
                            .aim_constraints
                            .get(state.aim_constraint_index)
                            .is_some()
                        {
                            hlpb.aim_constraints.remove(state.aim_constraint_index);
                        }
                    }
                }
            }
        });

        ui.menu_button("Help", |ui| {
            if ui.button(format!("{GITHUB} Hlpb Editor Wiki")).clicked() {
                let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Hlpb-Editor";
                if let Err(e) = open::that(link) {
                    log::error!("Failed to open {link}: {e}");
                }
            }
        });
    });

    saved
}

fn orient_constraints(
    ui: &mut Ui,
    hlpb: &mut HlpbData,
    skel: Option<&SkelData>,
    state: &mut HlpbEditorState,
) {
    if let Some(o) = hlpb
        .orient_constraints
        .get_mut(state.orient_constraint_index)
    {
        let id = egui::Id::unique("orient_constraint");

        Grid::new(id).show(ui, |ui| {
            ui.label("Name");
            ui.add_sized([200.0, 20.0], TextEdit::singleline(&mut o.name));
            ui.end_row();

            ui.label("Parent 1");
            bone_combo_box(ui, &mut o.parent_bone_name1, id.with(0), skel, &[]);
            ui.end_row();

            ui.label("Parent 2");
            bone_combo_box(ui, &mut o.parent_bone_name2, id.with(1), skel, &[]);
            ui.end_row();

            ui.label("Source");
            bone_combo_box(ui, &mut o.source_bone_name, id.with(2), skel, &[]);
            ui.end_row();

            ui.label("Target");
            bone_combo_box(ui, &mut o.target_bone_name, id.with(3), skel, &[]);
            ui.end_row();

            // TODO: Make this an enum in ssbh_data eventually.
            ui.label("Unk Type");
            egui::ComboBox::from_id_salt(id.with(4))
                .selected_text(o.unk_type.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut o.unk_type, 0, "0");
                    ui.selectable_value(&mut o.unk_type, 1, "1");
                    ui.selectable_value(&mut o.unk_type, 2, "2");
                });
            ui.end_row();

            ui.label("Constraint Axes");
            edit_vector3(ui, id.with(5), &mut o.constraint_axes, 0.0, 1.0);
            ui.end_row();

            ui.label("Quat 1");
            edit_quat(ui, id.with(6), &mut o.quat1);
            ui.end_row();

            ui.label("Quat 2");
            edit_quat(ui, id.with(7), &mut o.quat2);
            ui.end_row();

            ui.label("Range Min");
            edit_vector3(ui, id.with(8), &mut o.range_min, -180.0, 180.0);
            ui.end_row();

            ui.label("Range Max");
            edit_vector3(ui, id.with(9), &mut o.range_max, -180.0, 180.0);
            ui.end_row();
        });
    }
}

fn aim_constraints(
    ui: &mut Ui,
    hlpb: &mut HlpbData,
    skel: Option<&SkelData>,
    state: &mut HlpbEditorState,
) {
    if let Some(aim) = hlpb.aim_constraints.get_mut(state.aim_constraint_index) {
        let id = egui::Id::unique("aim_constraint");

        egui::Grid::new(id).show(ui, |ui| {
            ui.label("Name");
            ui.add_sized([200.0, 20.0], TextEdit::singleline(&mut aim.name));
            ui.end_row();

            ui.label("Aim 1");
            bone_combo_box(ui, &mut aim.aim_bone_name1, id.with(0), skel, &[]);
            ui.end_row();

            ui.label("Aim 2");
            bone_combo_box(ui, &mut aim.aim_bone_name2, id.with(1), skel, &[]);
            ui.end_row();

            ui.label("Aim Type 1");
            bone_combo_box(ui, &mut aim.aim_type1, id.with(2), skel, &["DEFAULT"]);
            ui.end_row();

            ui.label("Aim Type 2");
            bone_combo_box(ui, &mut aim.aim_type2, id.with(3), skel, &["DEFAULT"]);
            ui.end_row();

            ui.label("Target 1");
            bone_combo_box(ui, &mut aim.target_bone_name1, id.with(4), skel, &[]);
            ui.end_row();

            ui.label("Target 2");
            bone_combo_box(ui, &mut aim.target_bone_name2, id.with(5), skel, &[]);
            ui.end_row();

            ui.label("Unk1");
            ui.add(DragValue::new(&mut aim.unk1));
            ui.end_row();

            ui.label("Unk2");
            ui.add(DragValue::new(&mut aim.unk2));
            ui.end_row();

            ui.label("Aim");
            edit_vector3(ui, id.with(6), &mut aim.aim, 0.0, 1.0);
            ui.end_row();

            ui.label("Up");
            edit_vector3(ui, id.with(7), &mut aim.up, 0.0, 1.0);
            ui.end_row();

            ui.label("Quat 1");
            edit_quat(ui, id.with(8), &mut aim.quat1);
            ui.end_row();

            ui.label("Quat 2");
            edit_quat(ui, id.with(9), &mut aim.quat2);
            ui.end_row();
        });
    }
}

fn edit_vector3(ui: &mut Ui, id: egui::Id, value: &mut Vec3, min: f32, max: f32) {
    ui.horizontal(|ui| {
        ui.add(
            DragSlider::new(id.with("x"), &mut value.x)
                .width(40.0)
                .range(min, max),
        );
        ui.add(
            DragSlider::new(id.with("y"), &mut value.y)
                .width(40.0)
                .range(min, max),
        );
        ui.add(
            DragSlider::new(id.with("z"), &mut value.z)
                .width(40.0)
                .range(min, max),
        );
    });
}

fn edit_quat(ui: &mut Ui, id: egui::Id, value: &mut Quat) {
    ui.horizontal(|ui| {
        ui.add(DragSlider::new(id.with("x"), &mut value.x).width(40.0));
        ui.add(DragSlider::new(id.with("y"), &mut value.y).width(40.0));
        ui.add(DragSlider::new(id.with("z"), &mut value.z).width(40.0));
        ui.add(DragSlider::new(id.with("w"), &mut value.w).width(40.0));
    });
}
