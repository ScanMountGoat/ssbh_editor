use std::path::Path;

use egui::{collapsing_header::CollapsingState, CollapsingHeader, Context, Ui};
use ssbh_wgpu::ModelFolder;

use crate::{
    app::{folder_display_name, is_model_folder, SsbhApp},
    widgets::EyeCheckBox,
    AnimationIndex, AnimationSlot,
};

pub fn anim_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // Only assign animations to folders with model files.
    for (model_index, model) in app
        .models
        .iter()
        .enumerate()
        .filter(|(_, model)| is_model_folder(model))
    {
        let mut slots_to_remove = Vec::new();

        let id = ui.make_persistent_id("meshlist").with(model_index);
        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                // Assume the associated animation folder names matche the model folder.
                ui.label(folder_display_name(model));
            })
            .body(|ui| {
                // Associate animations with the model folder by name.
                // Motion folders use the same name as model folders.
                // TODO: Allow manually associating animations?
                // TODO: Is is it worth precomputing this list to avoid allocations?
                // TODO: Handle unrelated folders with the same name like two c00 model folders?
                let available_anims: Vec<_> = app
                    .models
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| {
                        Path::new(&m.folder_name).file_name()
                            == Path::new(&model.folder_name).file_name()
                    })
                    .flat_map(|(folder_index, m)| {
                        m.anims
                            .iter()
                            .enumerate()
                            .map(move |(anim_index, _)| AnimationIndex {
                                folder_index,
                                anim_index,
                            })
                    })
                    .collect();

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
                        show_anim_slot(
                            ui,
                            ctx,
                            anim_slot,
                            &app.models,
                            &mut app.animation_state.animation_frame_was_changed,
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
    ui: &mut Ui,
    ctx: &Context,
    anim_slot: &mut AnimationSlot,
    models: &[ModelFolder],
    update_animations: &mut bool,
    available_anims: &[AnimationIndex],
    model_index: usize,
    slot: usize,
    slots_to_remove: &mut Vec<usize>,
) {
    let id = ui.make_persistent_id(format!("{model_index}.slot.{slot}"));
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
                    *update_animations = true;
                }

                if anim_combo_box(
                    ui,
                    models,
                    available_anims,
                    model_index,
                    slot,
                    name,
                    anim_slot,
                ) {
                    // Reflect selecting a new animation in the viewport.
                    *update_animations = true;
                }

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
}

fn anim_combo_box(
    ui: &mut Ui,
    models: &[ModelFolder],
    available_anims: &[AnimationIndex],
    model_index: usize,
    slot: usize,
    name: &str,
    anim_slot: &mut AnimationSlot,
) -> bool {
    // TODO: Union the responses instead?
    // TODO: How to cleanly implement change tracking for complex editors?
    let mut changed = false;

    egui::ComboBox::from_id_source(format!("slot{:?}.{:?}", model_index, slot))
        .width(200.0)
        .selected_text(name)
        .show_ui(ui, |ui| {
            // TODO: Reset animations?
            for available_anim in available_anims {
                let name = available_anim
                    .get_animation(models)
                    .map(|(name, _)| name.as_str())
                    .unwrap_or("");

                // Return true if any animation is selected.
                changed |= ui
                    .selectable_value(&mut anim_slot.animation, Some(*available_anim), name)
                    .changed();
            }
        });

    changed
}
