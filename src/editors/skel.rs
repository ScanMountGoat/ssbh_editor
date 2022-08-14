use std::path::Path;

use crate::widgets::enum_combo_box;
use egui::{Button, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::prelude::*;

pub fn skel_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    skel: &mut SkelData,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    egui::Window::new(format!("Skel Editor ({title})"))
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::menu::bar(ui, |ui| {
                        egui::menu::menu_button(ui, "File", |ui| {
                            if ui.button("Save").clicked() {
                                ui.close_menu();

                                let file = Path::new(folder_name).join(file_name);
                                if let Err(e) = skel.write_to_file(&file) {
                                    error!("Failed to save {:?}: {}", file, e);
                                }
                            }

                            if ui.button("Save As...").clicked() {
                                ui.close_menu();

                                if let Some(file) = FileDialog::new()
                                    .add_filter("Skel", &["nusktb"])
                                    .save_file()
                                {
                                    if let Err(e) = skel.write_to_file(&file) {
                                        error!("Failed to save {:?}: {}", file, e);
                                    }
                                }
                            }
                        });

                        egui::menu::menu_button(ui, "Skeleton", |ui| {
                            if ui
                                .add(Button::new("Match reference bone order...").wrap(false))
                                .clicked()
                            {
                                ui.close_menu();

                                if let Some(file) = FileDialog::new()
                                    .add_filter("Skel", &["nusktb"])
                                    .pick_file()
                                {
                                    match SkelData::from_file(&file) {
                                        Ok(reference) => match_skel_order(skel, &reference),
                                        Err(e) => error!("Failed to read {:?}: {}", file, e),
                                    }
                                }
                            }
                        });

                        egui::menu::menu_button(ui, "Help", |ui| {
                            if ui.button("Skel Editor Wiki").clicked() {
                                ui.close_menu();

                                let link =
                                    "https://github.com/ScanMountGoat/ssbh_editor/wiki/Skel-Editor";
                                if let Err(e) = open::that(link) {
                                    log::error!("Failed to open {link}: {e}");
                                }
                            }
                        });
                    });
                    ui.separator();

                    // TODO: Add options to show a grid or tree based layout?
                    egui::Grid::new("some_unique_id").show(ui, |ui| {
                        // Header
                        ui.heading("Bone");
                        ui.heading("Parent");
                        ui.heading("Billboard Type");
                        ui.end_row();

                        // TODO: Do this without clone?
                        let other_bones = skel.bones.clone();

                        for (i, bone) in skel.bones.iter_mut().enumerate() {
                            let id = egui::Id::new("bone").with(i);

                            ui.label(&bone.name);
                            let parent_bone_name = bone
                                .parent_index
                                .and_then(|i| other_bones.get(i))
                                .map(|p| p.name.as_str())
                                .unwrap_or("None");

                            egui::ComboBox::from_id_source(id)
                                .selected_text(parent_bone_name)
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(&mut bone.parent_index, None, "None")
                                        .changed();
                                    ui.separator();
                                    // TODO: Is there a way to make this not O(N^2)?
                                    for (other_i, other_bone) in other_bones.iter().enumerate() {
                                        changed |= ui
                                            .selectable_value(
                                                &mut bone.parent_index,
                                                Some(other_i),
                                                &other_bone.name,
                                            )
                                            .changed();
                                    }
                                });

                            changed |= enum_combo_box(
                                ui,
                                "",
                                i + other_bones.len(),
                                &mut bone.billboard_type,
                            );

                            ui.end_row();
                        }
                    });
                });
        });

    (open, changed)
}

fn match_skel_order(skel: &mut SkelData, reference: &SkelData) {
    // TODO: Sort by helper bones, swing bones, etc for added bones?
    skel.bones.sort_by_key(|o| {
        // The sort is stable, so unmatched bones will be placed at the end in the same order.
        reference
            .bones
            .iter()
            .position(|r| r.name == o.name)
            .unwrap_or(reference.bones.len())
    })
}

#[cfg(test)]
mod tests {
    use ssbh_data::skel_data::{BillboardType, BoneData};

    use super::*;

    #[test]
    fn skel_order_empty_reference() {
        let mut skel = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "c".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
            ],
        };

        let reference = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: Vec::new(),
        };

        match_skel_order(&mut skel, &reference);

        assert_eq!("a", skel.bones[0].name);
        assert_eq!("b", skel.bones[1].name);
        assert_eq!("c", skel.bones[2].name);
    }

    #[test]
    fn skel_order_added_bonees() {
        let mut skel = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "c".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
            ],
        };

        let reference = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![BoneData {
                name: "c".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: None,
                billboard_type: BillboardType::Disabled,
            }],
        };

        match_skel_order(&mut skel, &reference);

        assert_eq!("c", skel.bones[0].name);
        assert_eq!("a", skel.bones[1].name);
        assert_eq!("b", skel.bones[2].name);
    }
}
