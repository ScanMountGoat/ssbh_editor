use std::path::Path;

use crate::{
    app::{SkelEditorState, SkelMode},
    widgets::enum_combo_box,
};
use egui::{Button, CollapsingHeader, Grid, RichText, ScrollArea};
use log::error;
use rfd::FileDialog;
use ssbh_data::{prelude::*, skel_data::BoneData};

pub fn skel_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    skel: &mut SkelData,
    state: &mut SkelEditorState,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    egui::Window::new(format!("Skel Editor ({title})"))
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
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
                        .add(Button::new("Match Reference Bone Order...").wrap(false))
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

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Skel-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.mode,
                    SkelMode::List,
                    RichText::new("List").heading(),
                );
                ui.selectable_value(
                    &mut state.mode,
                    SkelMode::Hierarchy,
                    RichText::new("Hierarchy").heading(),
                );
            });

            changed |= match state.mode {
                SkelMode::List => edit_bones_list(ui, skel),
                SkelMode::Hierarchy => edit_bones_hierarchy(ui, skel),
            };
        });

    (open, changed)
}

fn edit_bones_list(ui: &mut egui::Ui, skel: &mut SkelData) -> bool {
    let mut changed = false;
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            Grid::new("skel_grid").show(ui, |ui| {
                // Header
                ui.heading("Bone");
                ui.heading("Parent");
                ui.heading("Billboard Type");
                ui.end_row();

                // TODO: Do this without clone?
                let other_bones = skel.bones.clone();

                let mut bones_to_swap = None;
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
                                if i != other_i {
                                    changed |= ui
                                        .selectable_value(
                                            &mut bone.parent_index,
                                            Some(other_i),
                                            &other_bone.name,
                                        )
                                        .changed();
                                }
                            }
                        });

                    changed |=
                        enum_combo_box(ui, "", i + other_bones.len(), &mut bone.billboard_type);

                    ui.horizontal(|ui| {
                        if ui.button("⏶").clicked() {
                            // Move bone up
                            if i > 0 {
                                bones_to_swap = Some((i, i - 1));
                            }
                        }

                        if ui.button("⏷").clicked() {
                            // Move bone down
                            if !other_bones.is_empty() && i < other_bones.len() - 1 {
                                bones_to_swap = Some((i, i + 1));
                            }
                        }
                    });
                    ui.end_row();
                }

                if let Some((a, b)) = bones_to_swap {
                    skel.bones = swap_bones(a, b, &skel.bones);
                }
            });
        });

    changed
}

fn edit_bones_hierarchy(ui: &mut egui::Ui, skel: &mut SkelData) -> bool {
    let changed = false;

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for (i, bone) in skel.bones.iter().enumerate() {
                if bone.parent_index.is_none() {
                    display_bones_recursive(ui, i, &skel.bones);
                }
            }
        });

    changed
}

fn display_bones_recursive(ui: &mut egui::Ui, root_index: usize, bones: &[BoneData]) {
    // TODO: Does this handle cycles?
    // Don't assume bone names are unique.
    let name = &bones[root_index].name;
    let id = ui.make_persistent_id("skel").with(name).with(root_index);

    CollapsingHeader::new(name)
        .id_source(id)
        .default_open(true)
        .show(ui, |ui| {
            // Recursively iterate over the child bones.
            for (i, _) in bones
                .iter()
                .enumerate()
                .filter(|(_, b)| b.parent_index == Some(root_index))
            {
                display_bones_recursive(ui, i, bones);
            }
        });
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

fn swap_bones(a: usize, b: usize, bones: &[BoneData]) -> Vec<BoneData> {
    bones
        .iter()
        .enumerate()
        .map(|(i, bone)| {
            // Swap bones at positions a and b.
            let mut new_bone = if i == a {
                bones[b].clone()
            } else if i == b {
                bones[a].clone()
            } else {
                bone.clone()
            };
            new_bone.parent_index = if new_bone.parent_index == Some(a) {
                // Parent anything parented to a to b.
                Some(b)
            } else if new_bone.parent_index == Some(b) {
                // Parent anything parented to b to a.
                Some(a)
            } else {
                new_bone.parent_index
            };

            new_bone
        })
        .collect()
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

    #[test]
    fn swap_bones_same_indices() {
        let before = vec![
            BoneData {
                name: "a".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: None,
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "b".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
        ];

        let after = swap_bones(0, 0, &before);
        assert_eq!(before, after);
    }

    #[test]
    fn swap_bones_different_indices() {
        let before = vec![
            BoneData {
                name: "a".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: None,
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "b".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
        ];

        let after = swap_bones(0, 1, &before);
        assert_eq!(
            vec![
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(1),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
            ],
            after
        );
    }

    #[test]
    fn swap_bones_with_parents() {
        let before = vec![
            BoneData {
                name: "a".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(1),
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "b".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "c".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "d".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(2),
                billboard_type: BillboardType::Disabled,
            },
        ];

        // Swap b and c.
        // The bones should still point to the correct parents.
        let after = swap_bones(1, 2, &before);
        assert_eq!(
            vec![
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(2),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "c".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(0),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(0),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "d".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(1),
                    billboard_type: BillboardType::Disabled,
                },
            ],
            after
        );
    }
}
