use egui::{CollapsingHeader, DragValue, ScrollArea};
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
                    // TODO: Use a layout similar to the matl editor to support more fields.
                    // TODO: Use the DragSliders for editing Vector4 and Vector3 values.
                    if !hlpb.aim_constraints.is_empty() {
                        aim_constraints(ui, hlpb, skel);
                    }

                    if !hlpb.orient_constraints.is_empty() {
                        orient_constraints(ui, hlpb, skel);
                    }
                });
        });

    open
}

fn orient_constraints(ui: &mut egui::Ui, hlpb: &mut HlpbData, skel: Option<&SkelData>) {
    CollapsingHeader::new("Orient Constraints")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("orient").striped(true).show(ui, |ui| {
                ui.heading("Name");
                ui.heading("Bone");
                ui.heading("Root");
                ui.heading("Parent");
                ui.heading("Driver");
                ui.heading("Unk Type");
                ui.end_row();

                for (i, orient) in hlpb.orient_constraints.iter_mut().enumerate() {
                    let id = egui::Id::new("orient").with(i);

                    ui.label(&orient.name);
                    bone_combo_box(ui, &mut orient.bone_name, id.with(0), skel, &[]);
                    bone_combo_box(ui, &mut orient.root_bone_name, id.with(1), skel, &[]);
                    bone_combo_box(ui, &mut orient.parent_bone_name, id.with(2), skel, &[]);
                    bone_combo_box(ui, &mut orient.driver_bone_name, id.with(3), skel, &[]);
                    ui.add(DragValue::new(&mut orient.unk_type));
                    ui.end_row();
                }
            });
        });
}

fn aim_constraints(ui: &mut egui::Ui, hlpb: &mut HlpbData, skel: Option<&SkelData>) {
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
                    bone_combo_box(ui, &mut aim.aim_bone_name1, id.with(0), skel, &[]);
                    bone_combo_box(ui, &mut aim.aim_bone_name2, id.with(1), skel, &[]);
                    bone_combo_box(ui, &mut aim.aim_type1, id.with(2), skel, &["DEFAULT"]);
                    bone_combo_box(ui, &mut aim.aim_type2, id.with(3), skel, &["DEFAULT"]);
                    bone_combo_box(ui, &mut aim.target_bone_name1, id.with(4), skel, &[]);
                    bone_combo_box(ui, &mut aim.target_bone_name2, id.with(5), skel, &[]);
                    ui.add(DragValue::new(&mut aim.unk1));
                    ui.add(DragValue::new(&mut aim.unk2));
                    ui.end_row();
                }
            });
        });
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
                ui.selectable_value(bone_name, name.to_string(), *name);
            }

            if let Some(skel) = skel {
                for bone in &skel.bones {
                    ui.selectable_value(bone_name, bone.name.clone(), &bone.name);
                }
            }
        });
}
