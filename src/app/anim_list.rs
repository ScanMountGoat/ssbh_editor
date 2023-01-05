use crate::{
    app::{folder_display_name, SsbhApp},
    model_folder::find_anim_folders,
    widgets::EyeCheckBox,
    AnimationIndex, AnimationSlot, ModelFolderState,
};
use egui::{collapsing_header::CollapsingState, CollapsingHeader, Context, Label, RichText, Ui};

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
        CollapsingHeader::new(folder_display_name(&model.model))
            .id_source(id)
            .default_open(true)
            .show(ui, |ui| {
                // Associate animations with the model folder by name.
                // TODO: Is is it worth precomputing this list for performance?
                let available_folders = find_anim_folders(model, &app.models);

                if available_folders.is_empty() {
                    let message = "No matching animations found for this folder. \
                        Add the matching animation folder(s) with File > Add Folder to Workspace.";
                    ui.label(message);
                } else {
                    // TODO: Disable the UI instead?
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
                                &available_folders,
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
                }
            });
    }
}

fn show_anim_slot(
    ctx: &Context,
    ui: &mut Ui,
    anim_slot: &mut AnimationSlot,
    models: &[ModelFolderState],
    available_folders: &[(usize, &ModelFolderState)],
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

                if anim_combo_box(ui, available_folders, id.with("anim"), name, anim_slot) {
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
                    Label::new(RichText::new(folder_display_name(&folder.model)).heading())
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
