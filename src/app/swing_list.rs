use crate::{
    app::{folder_display_name, SsbhApp},
    model_folder::{find_swing_folders, ModelFolderState},
    widgets::EyeCheckBox,
    SwingState,
};
use egui::{collapsing_header::CollapsingState, CollapsingHeader, Context, Label, RichText, Ui};
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
        CollapsingHeader::new(folder_display_name(&model.model))
            .id_source(id)
            .default_open(true)
            .show(ui, |ui| {
                let available_folders = find_swing_folders(model, &app.models);

                if available_folders.is_empty() {
                    let message = "No matching swing.prc files found for this folder. \
                        Add the matching folder with File > Add Folder to Workspace.";
                    ui.label(message);
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Swing PRC");
                        if let Some(prc_index) = app.swing_state.selected_swing_folders.get_mut(i) {
                            swing_combo_box(
                                ui,
                                &available_folders,
                                ui.make_persistent_id("swingcombo").with(i),
                                prc_index,
                            );
                        }
                    });

                    if let Some(swing_prc) = get_swing_prc(i, &app.swing_state, &app.models) {
                        list_swing_bones(ctx, id, ui, swing_prc);
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

fn list_swing_bones(ctx: &Context, id: egui::Id, ui: &mut Ui, swing_prc: &SwingPrc) {
    for (i, swing_bone) in swing_prc.swingbones.iter().enumerate() {
        let id = id.with("swingbones").with(i);
        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                let name = swing_bone.name;
                ui.add(EyeCheckBox::new(
                    &mut true,
                    format!("swingbones[{i}] {name}"),
                ));
            })
            .body(|ui| {
                list_params(ctx, id, ui, &swing_bone.params);
            });
    }
}

fn list_params(ctx: &Context, id: egui::Id, ui: &mut Ui, params: &[Param]) {
    for (i, param) in params.iter().enumerate() {
        let id = id.with("params").with(i);
        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                ui.add(EyeCheckBox::new(&mut true, format!("params[{i}]")));
            })
            .body(|ui| {
                list_collisions(ui, param);
            });
    }
}

fn list_collisions(ui: &mut Ui, param: &Param) {
    // Indent without the vertical line.
    ui.visuals_mut().widgets.noninteractive.bg_stroke.width = 0.0;
    ui.spacing_mut().indent = 24.0;
    ui.indent("indent", |ui| {
        for (i, col) in param.collisions.iter().enumerate() {
            ui.add(EyeCheckBox::new(
                &mut true,
                format!("collisions[{i}] {col}"),
            ));
        }
    });
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

    egui::ComboBox::from_id_source(id)
        .width(200.0)
        .selected_text(name)
        .show_ui(ui, |ui| {
            // Iterate in decreasing order of affinity with the model folder.
            for (i, folder) in anim_folders.iter().rev() {
                // TODO: Is it worth grouping by folder if there's only one swing?
                // TODO: Just show the full path for each swing.prc?
                ui.add(
                    Label::new(RichText::new(folder_display_name(&folder.model)).heading())
                        .wrap(false),
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
