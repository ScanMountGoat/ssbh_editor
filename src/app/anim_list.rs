use std::path::Path;

use egui::{collapsing_header::CollapsingState, CollapsingHeader, Context, Label, RichText, Ui};

use crate::{
    app::{folder_display_name, SsbhApp},
    widgets::EyeCheckBox,
    AnimationIndex, AnimationSlot, ModelFolderState,
};

pub fn anim_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // Only assign animations to folders with model files.
    for (model_index, model) in app
        .models
        .iter()
        .enumerate()
        .filter(|(_, model)| model.is_model_folder())
    {
        let mut slots_to_remove = Vec::new();

        let id = ui.make_persistent_id("animlist").with(model_index);
        CollapsingHeader::new(folder_display_name(&model.model).to_string_lossy())
            .id_source(id)
            .default_open(true)
            .show(ui, |ui| {
                // Associate animations with the model folder by name.
                // TODO: Is is it worth precomputing this list for performance?
                let available_anims = find_anim_folders(model, &app.models);

                if available_anims.is_empty() {
                    let message = "No matching animations found for this folder. \
                        Add the matching animation folder(s) with File > Add Folder to Workspace.";
                    ui.label(message);
                }

                let model_animations = app.animation_state.animations.get_mut(model_index);
                if let Some(model_animations) = model_animations {
                    if ui.button("Add Slot").clicked() {
                        model_animations.push(AnimationSlot::new());
                    }

                    for (slot, anim_slot) in model_animations.iter_mut().enumerate().rev() {
                        app.animation_state.should_update_animations |= show_anim_slot(
                            ctx,
                            ui,
                            anim_slot,
                            &app.models,
                            &available_anims,
                            model_index,
                            slot,
                            &mut slots_to_remove,
                        );
                    }

                    // TODO: Force only one slot to be removed?
                    for slot in slots_to_remove {
                        model_animations.remove(slot);
                    }
                }
            });
    }
}

fn show_anim_slot(
    ctx: &Context,
    ui: &mut Ui,
    anim_slot: &mut AnimationSlot,
    models: &[ModelFolderState],
    available_anims: &[(usize, &ModelFolderState)],
    model_index: usize,
    slot: usize,
    slots_to_remove: &mut Vec<usize>,
) -> bool {
    let mut update_animations = false;

    let id = ui.make_persistent_id(model_index).with("slot").with(slot);
    CollapsingState::load_with_default_open(ctx, id, false)
        .show_header(ui, |ui| {
            let name = anim_slot
                .animation
                .as_ref()
                .and_then(|anim_index| anim_index.get_animation(models))
                .map(|(name, _)| name.as_str())
                .unwrap_or_else(|| "Select an animation...");

            ui.horizontal(|ui| {
                // TODO: Disabling anims with visibility tracks has confusing behavior.
                // Disabling a vis track currently only disables the effects on later frames.
                if ui
                    .add(EyeCheckBox::new(
                        &mut anim_slot.is_enabled,
                        format!("Slot {slot}"),
                    ))
                    .changed()
                {
                    update_animations = true;
                }

                if anim_combo_box(ui, available_anims, id.with("anim"), name, anim_slot) {
                    // Reflect selecting a new animation in the viewport.
                    update_animations = true;
                }

                // Use "Remove" since this doesn't delete the actual animation.
                if ui.button("Remove").clicked() {
                    slots_to_remove.push(slot);
                }
            });
        })
        .body(|ui| {
            if let Some((_, Ok(anim))) = anim_slot
                .animation
                .as_ref()
                .and_then(|anim_index| anim_index.get_animation(models))
            {
                for group in &anim.groups {
                    CollapsingHeader::new(group.group_type.to_string())
                        .default_open(false)
                        .show(ui, |ui| {
                            for node in &group.nodes {
                                match node.tracks.as_slice() {
                                    [_] => {
                                        // Don't use a collapsing header if there is only one track.
                                        // This simplifies the layout for most boolean and transform tracks.
                                        // TODO: How to toggle visibility for rendering?
                                        ui.label(&node.name);
                                    }
                                    _ => {
                                        CollapsingHeader::new(&node.name).default_open(true).show(
                                            ui,
                                            |ui| {
                                                for track in &node.tracks {
                                                    // TODO: How to toggle visibility for rendering?
                                                    ui.label(&track.name);
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        });
                }
            }
        });

    update_animations
}

fn anim_combo_box(
    ui: &mut Ui,
    anim_folders: &[(usize, &ModelFolderState)],
    id: egui::Id,
    name: &str,
    anim_slot: &mut AnimationSlot,
) -> bool {
    // TODO: Union the responses instead?
    // TODO: How to cleanly implement change tracking for complex editors?
    let mut changed = false;

    // TODO: Reset animations?
    egui::ComboBox::from_id_source(id)
        .width(200.0)
        .selected_text(name)
        .show_ui(ui, |ui| {
            // Iterate in decreasing order of affinity with the model folder.
            for (folder_index, folder) in anim_folders.iter().rev() {
                ui.add(
                    Label::new(
                        RichText::new(folder_display_name(&folder.model).to_string_lossy())
                            .heading(),
                    )
                    .wrap(false),
                );

                for (anim_index, (name, _)) in folder.model.anims.iter().enumerate() {
                    let available_anim = AnimationIndex {
                        folder_index: *folder_index,
                        anim_index,
                    };

                    // Return true if any animation is selected.
                    changed |= ui
                        .selectable_value(&mut anim_slot.animation, Some(available_anim), name)
                        .changed();
                }
            }
        });

    changed
}

fn find_anim_folders<'a>(
    model: &ModelFolderState,
    anim_folders: &'a [ModelFolderState],
) -> Vec<(usize, &'a ModelFolderState)> {
    let mut folders: Vec<_> = anim_folders
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.model.anims.is_empty())
        .collect();

    // Sort in increasing order of affinity with the model folder.
    folders.sort_by_key(|(_, a)| {
        // The animation folder affinity is the number of matching path components.
        // Consider the model folder "/mario/model/body/c00".
        // The folder "/mario/motion/body/c00" scores higher than "/mario/motion/pump/c00".
        Path::new(&model.model.folder_name)
            .components()
            .rev()
            .zip(Path::new(&a.model.folder_name).components().rev())
            .take_while(|(a, b)| a == b)
            .count()
    });
    folders
}

#[cfg(test)]
mod tests {
    use crate::validation::ModelValidationErrors;
    use ssbh_data::anim_data::AnimData;
    use ssbh_wgpu::ModelFolder;

    use super::*;

    fn model_folder(name: &str) -> ModelFolderState {
        ModelFolderState {
            model: ModelFolder {
                folder_name: name.to_owned(),
                meshes: Vec::new(),
                skels: Vec::new(),
                matls: Vec::new(),
                modls: Vec::new(),
                adjs: Vec::new(),
                anims: Vec::new(),
                hlpbs: Vec::new(),
                nutexbs: Vec::new(),
                meshexes: Vec::new(),
            },
            thumbnails: Vec::new(),
            validation: ModelValidationErrors::default(),
        }
    }

    fn anim_folder(name: &str) -> ModelFolderState {
        ModelFolderState {
            model: ModelFolder {
                folder_name: name.to_owned(),
                meshes: Vec::new(),
                skels: Vec::new(),
                matls: Vec::new(),
                modls: Vec::new(),
                adjs: Vec::new(),
                anims: vec![(
                    String::new(),
                    Ok(AnimData {
                        major_version: 2,
                        minor_version: 0,
                        final_frame_index: 0.0,
                        groups: Vec::new(),
                    }),
                )],
                hlpbs: Vec::new(),
                nutexbs: Vec::new(),
                meshexes: Vec::new(),
            },
            thumbnails: Vec::new(),
            validation: ModelValidationErrors::default(),
        }
    }

    #[test]
    fn find_anim_folders_no_folders() {
        assert!(find_anim_folders(&model_folder("/model/body/c00"), &[]).is_empty());
    }

    #[test]
    fn find_anim_folders_empty_folders() {
        // Folders without animations should be excluded.
        assert!(find_anim_folders(
            &model_folder("/model/body/c00"),
            &[model_folder("/motion/body/c00")]
        )
        .is_empty());
    }

    #[test]
    fn find_anim_folders_compare_matches() {
        // The second folder is the best match.
        let anim_folders = vec![
            anim_folder("/motion/pump/c00"),
            anim_folder("/motion/body/c00"),
            anim_folder("/motion/body/c01"),
        ];
        let folders = find_anim_folders(&model_folder("/model/body/c00"), &anim_folders);
        assert!(matches!(folders.as_slice(), [(2, _), (0, _), (1, _)]));
    }
}
