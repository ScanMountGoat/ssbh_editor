use std::collections::HashSet;

use crate::{
    app::{folder_display_name, SsbhApp},
    model_folder::{find_swing_folders, ModelFolderState},
    widgets::EyeCheckBox,
    SwingState,
};
use egui::{
    collapsing_header::CollapsingState, CollapsingHeader, Context, Label, RichText, TextWrapMode,
    Ui,
};
use ssbh_wgpu::swing::*;

pub fn swing_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // TODO: Add state for tracking the visible and hovered items.
    // Only assign swing.prc data to model folders.
    for (i, model) in app
        .models
        .iter()
        .enumerate()
        .filter(|(_, model)| model.is_model_folder())
    {
        let id = ui.make_persistent_id("swinglist").with(i);
        CollapsingHeader::new(folder_display_name(model))
            .id_salt(id)
            .default_open(true)
            .show(ui, |ui| {
                let available_folders = find_swing_folders(model, &app.models);

                if available_folders.is_empty() {
                    let message = "No matching swing.prc files found for this folder. \
                        Add the matching folder with File > Add Folder to Workspace.";
                    ui.label(message);
                } else {
                    ui.horizontal(|ui| {
                        // TODO: Add an option to remove the assigned swing.prc.
                        ui.label("Swing PRC");
                        if let Some(prc_index) = app.swing_state.selected_swing_folders.get_mut(i) {
                            app.swing_state.should_update_swing |= swing_combo_box(
                                ui,
                                &available_folders,
                                ui.make_persistent_id("swingcombo").with(i),
                                prc_index,
                            );
                        }
                    });

                    if let Some(swing_prc) = get_swing_prc(i, &app.swing_state, &app.models) {
                        if let Some(hidden_collisions) =
                            app.swing_state.hidden_collisions.get_mut(i)
                        {
                            list_swing_bones(ctx, id, ui, swing_prc, hidden_collisions);
                        }
                    }
                }
            });
    }
}

fn get_swing_prc<'a>(
    model_index: usize,
    state: &SwingState,
    models: &'a [ModelFolderState],
) -> Option<&'a SwingPrc> {
    let prc_index = state.selected_swing_folders.get(model_index)?.as_ref()?;
    models.get(*prc_index)?.swing_prc.as_ref()
}

fn list_swing_bones(
    ctx: &Context,
    id: egui::Id,
    ui: &mut Ui,
    swing_prc: &SwingPrc,
    hidden_collisions: &mut HashSet<u64>,
) {
    for (i, swing_bone) in swing_prc.swingbones.iter().enumerate() {
        let id = id.with("swingbones").with(i);
        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                let name = swing_bone.name;
                ui.label(format!("swingbones[{i}] {name}"));
            })
            .body(|ui| {
                list_params(ctx, id, ui, &swing_bone.params, hidden_collisions);
            });
    }
}

fn list_params(
    ctx: &Context,
    id: egui::Id,
    ui: &mut Ui,
    params: &[Param],
    hidden_collisions: &mut HashSet<u64>,
) {
    for (i, param) in params.iter().enumerate() {
        let id = id.with("params").with(i);
        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                ui.label(format!("params[{i}]"));
            })
            .body(|ui| {
                list_collisions(ui, param, hidden_collisions);
            });
    }
}

fn list_collisions(ui: &mut Ui, param: &Param, hidden_collisions: &mut HashSet<u64>) {
    for (i, col) in param.collisions.iter().enumerate() {
        let mut is_visible = !hidden_collisions.contains(&col.0);
        ui.add(EyeCheckBox::new(
            &mut is_visible,
            format!("collisions[{i}] {col}"),
        ));

        // Use a set since collisions are shared between params.
        if is_visible {
            hidden_collisions.remove(&col.0);
        } else {
            hidden_collisions.insert(col.0);
        }
    }
}

fn swing_combo_box(
    ui: &mut Ui,
    anim_folders: &[(usize, &ModelFolderState)],
    id: egui::Id,
    prc_index: &mut Option<usize>,
) -> bool {
    // TODO: Union the responses instead?
    let mut changed = false;

    let name = if prc_index.is_some() { "swing.prc" } else { "" };

    egui::ComboBox::from_id_salt(id)
        .width(200.0)
        .selected_text(name)
        .show_ui(ui, |ui| {
            // Iterate in decreasing order of affinity with the model folder.
            for (i, folder) in anim_folders.iter().rev() {
                // TODO: Is it worth grouping by folder if there's only one swing?
                // TODO: Just show the full path for each swing.prc?
                ui.add(
                    Label::new(RichText::new(folder_display_name(folder)).heading())
                        .wrap_mode(TextWrapMode::Extend),
                );
                if folder.swing_prc.is_some() {
                    changed |= ui
                        .selectable_value(prc_index, Some(*i), "swing.prc")
                        .changed();
                }
            }
        });

    changed
}
