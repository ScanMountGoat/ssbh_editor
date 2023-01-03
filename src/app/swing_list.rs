use crate::{
    app::{folder_display_name, SsbhApp},
    widgets::EyeCheckBox,
};
use egui::{collapsing_header::CollapsingState, Context, Ui};
use ssbh_wgpu::swing::*;

pub fn swing_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // TODO: Add state for tracking the visible and hovered items.
    for (i, folder) in app.models.iter_mut().enumerate() {
        // TODO: Filter to just animation folders instead?
        if let Some(swing_prc) = &folder.swing_prc {
            let id = ui.make_persistent_id("swinglist").with(i);

            CollapsingState::load_with_default_open(ctx, id, true)
                .show_header(ui, |ui| {
                    ui.add(EyeCheckBox::new(
                        &mut true,
                        folder_display_name(&folder.model),
                    ));
                })
                .body(|ui| {
                    list_swing_bones(ctx, id, ui, swing_prc);
                });
        }
    }
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
