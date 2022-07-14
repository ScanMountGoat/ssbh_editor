use egui::{CollapsingHeader, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;

pub fn hlpb_editor(
    ctx: &egui::Context,
    title: &str,
    hlpb: &mut HlpbData,
    skel: Option<&SkelData>,
) -> bool {
    let mut open = true;

    egui::Window::new(format!("Hlpb Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Hlpb", &["nuhlpb"])
                            .save_file()
                        {
                            if let Err(e) = hlpb.write_to_file(file) {
                                error!("Failed to save Hlpb (.nuhlpb): {}", e);
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
                    if !hlpb.aim_constraints.is_empty() {
                        CollapsingHeader::new("Aim Constraints")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new("aim").striped(true).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Name").heading());
                                    ui.label(egui::RichText::new("Aim 1").heading());
                                    ui.label(egui::RichText::new("Aim 2").heading());
                                    ui.label(egui::RichText::new("Type 1").heading());
                                    ui.label(egui::RichText::new("Type 2").heading());
                                    ui.label(egui::RichText::new("Target 1").heading());
                                    ui.label(egui::RichText::new("Target 2").heading());
                                    ui.end_row();

                                    for (i, aim) in hlpb.aim_constraints.iter_mut().enumerate() {
                                        ui.label(&aim.name);
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_bone_name1,
                                            format!("a{:?}0", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_bone_name2,
                                            format!("a{:?}1", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_type1,
                                            format!("a{:?}2", i),
                                            skel,
                                            &["DEFAULT"],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_type2,
                                            format!("a{:?}3", i),
                                            skel,
                                            &["DEFAULT"],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.target_bone_name1,
                                            format!("a{:?}4", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.target_bone_name2,
                                            format!("a{:?}5", i),
                                            skel,
                                            &[],
                                        );
                                        ui.end_row();
                                    }
                                });
                            });
                    }

                    if !hlpb.orient_constraints.is_empty() {
                        CollapsingHeader::new("Orient Constraints")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new("orient").striped(true).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Name").heading());
                                    ui.label(egui::RichText::new("Bone").heading());
                                    ui.label(egui::RichText::new("Root").heading());
                                    ui.label(egui::RichText::new("Parent").heading());
                                    ui.label(egui::RichText::new("Driver").heading());
                                    ui.end_row();

                                    // TODO: Add unk type.

                                    for (i, orient) in
                                        hlpb.orient_constraints.iter_mut().enumerate()
                                    {
                                        ui.label(&orient.name);
                                        bone_combo_box(
                                            ui,
                                            &mut orient.bone_name,
                                            format!("o{:?}0", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut orient.root_bone_name,
                                            format!("o{:?}1", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut orient.parent_bone_name,
                                            format!("o{:?}2", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut orient.driver_bone_name,
                                            format!("o{:?}3", i),
                                            skel,
                                            &[],
                                        );
                                        ui.end_row();
                                    }
                                });
                            });
                    }
                });
        });

    open
}

fn bone_combo_box(
    ui: &mut egui::Ui,
    bone_name: &mut String,
    id: impl std::hash::Hash,
    skel: Option<&SkelData>,
    extra_names: &[&str],
) {
    egui::ComboBox::from_id_source(id)
        .selected_text(bone_name.clone())
        .show_ui(ui, |ui| {
            // TODO: Just use text boxes if the skel is missing?
            for name in extra_names {
                ui.selectable_value(bone_name, name.to_string(), name.to_string());
            }

            if let Some(skel) = skel {
                for bone in &skel.bones {
                    ui.selectable_value(bone_name, bone.name.clone(), &bone.name);
                }
            }
        });
}
