use crate::{horizontal_separator_empty, widgets::*};
use ssbh_wgpu::{DebugMode, ModelRenderOptions, RenderSettings, SkinningSettings};

pub fn render_settings(
    ctx: &egui::Context,
    settings: &mut RenderSettings,
    options: &mut ModelRenderOptions,
    skinning_settings: &mut SkinningSettings,
    open: &mut bool,
    draw_skeletons: &mut bool,
    draw_bone_names: &mut bool,
    draw_bone_axes: &mut bool,
    enable_helper_bones: &mut bool
) {
    egui::Window::new("Render Settings")
        .open(open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Debug Shading");
            egui::Grid::new("debug_shading_grid").show(ui, |ui| {
                // TODO: Add descriptions.
                ui.label("Debug Mode");
                egui::ComboBox::from_id_source("Debug Mode")
                    .width(200.0)
                    .selected_text(debug_mode_label(settings.debug_mode))
                    .show_ui(ui, |ui| {
                        // Group modes for improved clarity.
                        ui.heading("Shading");
                        debug_mode(ui, settings, DebugMode::Shaded);
                        debug_mode(ui, settings, DebugMode::Basic);
                        debug_mode(ui, settings, DebugMode::Normals);
                        debug_mode(ui, settings, DebugMode::Bitangents);
                        debug_mode(ui, settings, DebugMode::Albedo);
                        ui.separator();

                        ui.heading("Vertex Attributes");
                        debug_mode(ui, settings, DebugMode::Position0);
                        debug_mode(ui, settings, DebugMode::Normal0);
                        debug_mode(ui, settings, DebugMode::Tangent0);
                        debug_mode(ui, settings, DebugMode::Map1);
                        debug_mode(ui, settings, DebugMode::Bake1);
                        debug_mode(ui, settings, DebugMode::UvSet);
                        debug_mode(ui, settings, DebugMode::UvSet1);
                        debug_mode(ui, settings, DebugMode::UvSet2);
                        ui.separator();

                        ui.heading("Vertex Color");
                        debug_mode(ui, settings, DebugMode::ColorSet1);
                        debug_mode(ui, settings, DebugMode::ColorSet2);
                        debug_mode(ui, settings, DebugMode::ColorSet3);
                        debug_mode(ui, settings, DebugMode::ColorSet4);
                        debug_mode(ui, settings, DebugMode::ColorSet5);
                        debug_mode(ui, settings, DebugMode::ColorSet6);
                        debug_mode(ui, settings, DebugMode::ColorSet7);
                        ui.separator();

                        ui.heading("Textures");
                        debug_mode(ui, settings, DebugMode::Texture0);
                        debug_mode(ui, settings, DebugMode::Texture1);
                        debug_mode(ui, settings, DebugMode::Texture2);
                        debug_mode(ui, settings, DebugMode::Texture3);
                        debug_mode(ui, settings, DebugMode::Texture4);
                        debug_mode(ui, settings, DebugMode::Texture5);
                        debug_mode(ui, settings, DebugMode::Texture6);
                        debug_mode(ui, settings, DebugMode::Texture7);
                        debug_mode(ui, settings, DebugMode::Texture8);
                        debug_mode(ui, settings, DebugMode::Texture9);
                        debug_mode(ui, settings, DebugMode::Texture10);
                        debug_mode(ui, settings, DebugMode::Texture11);
                        debug_mode(ui, settings, DebugMode::Texture12);
                        debug_mode(ui, settings, DebugMode::Texture13);
                        debug_mode(ui, settings, DebugMode::Texture14);
                        debug_mode(ui, settings, DebugMode::Texture16);
                    });

                ui.end_row();

                if settings.debug_mode == ssbh_wgpu::DebugMode::Shaded {
                    enum_combo_box(
                        ui,
                        "Transition Material",
                        "Transition Material",
                        &mut settings.transition_material,
                    );
                    ui.end_row();

                    ui.label("Transition Factor");
                    ui.add(DragSlider::new(
                        "transition_factor",
                        &mut settings.transition_factor,
                    ));
                    ui.end_row();
                }
            });
            if settings.debug_mode != DebugMode::Shaded {
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut settings.render_rgba[0], "R");
                    ui.toggle_value(&mut settings.render_rgba[1], "G");
                    ui.toggle_value(&mut settings.render_rgba[2], "B");
                    ui.toggle_value(&mut settings.render_rgba[3], "A");
                });

                ui.checkbox(&mut options.draw_wireframe, "Wireframe");

                if matches!(
                    settings.debug_mode,
                    DebugMode::Map1
                        | DebugMode::Bake1
                        | DebugMode::UvSet
                        | DebugMode::UvSet1
                        | DebugMode::UvSet2
                ) {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut settings.use_uv_pattern, false, "UV Coords");
                        ui.radio_value(&mut settings.use_uv_pattern, true, "UV Test Pattern");
                    });
                }
            }
            horizontal_separator_empty(ui);

            ui.heading("Render Passes");
            ui.checkbox(&mut settings.render_diffuse, "Enable Diffuse");
            ui.checkbox(&mut settings.render_specular, "Enable Specular");
            ui.checkbox(&mut settings.render_emission, "Enable Emission");
            ui.checkbox(&mut settings.render_rim_lighting, "Enable Rim Lighting");
            ui.checkbox(&mut settings.render_bloom, "Enable Bloom");
            horizontal_separator_empty(ui);

            ui.heading("Lighting");
            ui.checkbox(&mut settings.render_shadows, "Enable Shadows");
            horizontal_separator_empty(ui);

            ui.heading("Materials");
            ui.checkbox(&mut settings.render_vertex_color, "Enable Vertex Color");
            ui.horizontal(|ui| {
                ui.label("Enable Nor Channels");
                ui.toggle_value(&mut settings.render_nor[0], "R");
                ui.toggle_value(&mut settings.render_nor[1], "G");
                ui.toggle_value(&mut settings.render_nor[2], "B");
                ui.toggle_value(&mut settings.render_nor[3], "A");
            });
            ui.horizontal(|ui| {
                ui.label("Enable Prm Channels");
                ui.toggle_value(&mut settings.render_prm[0], "R");
                ui.toggle_value(&mut settings.render_prm[1], "G");
                ui.toggle_value(&mut settings.render_prm[2], "B");
                ui.toggle_value(&mut settings.render_prm[3], "A");
            });
            horizontal_separator_empty(ui);

            ui.heading("Animation");
            ui.checkbox(
                &mut skinning_settings.enable_parenting,
                "Enable Mesh Parenting",
            );
            ui.checkbox(
                &mut skinning_settings.enable_skinning,
                "Enable Vertex Skinning",
            );
            ui.checkbox(enable_helper_bones, "Enable Helper Bones");

            ui.heading("Skeleton");
            ui.checkbox(draw_skeletons, "Draw Bones");
            ui.checkbox(draw_bone_axes, "Draw Bone Axes");
            ui.checkbox(draw_bone_names, "Draw Bone Names");
        });
}

fn debug_mode(ui: &mut egui::Ui, settings: &mut RenderSettings, mode: DebugMode) {
    ui.selectable_value(&mut settings.debug_mode, mode, debug_mode_label(mode));
}

fn debug_mode_label(mode: DebugMode) -> String {
    let description = debug_description(mode);
    if !description.is_empty() {
        format!("{} ({})", mode, description)
    } else {
        mode.to_string()
    }
}

fn debug_description(mode: DebugMode) -> &'static str {
    match mode {
        DebugMode::Texture0 => "Col Layer 1",
        DebugMode::Texture1 => "Col Layer 2",
        DebugMode::Texture2 => "Irradiance Cube",
        DebugMode::Texture3 => "Ambient Occlusion",
        DebugMode::Texture4 => "Nor",
        DebugMode::Texture5 => "Emissive Layer 1",
        DebugMode::Texture6 => "Prm",
        DebugMode::Texture7 => "Specular Cube",
        DebugMode::Texture8 => "Diffuse Cube",
        DebugMode::Texture9 => "Baked Lighting",
        DebugMode::Texture10 => "Diffuse Layer 1",
        DebugMode::Texture11 => "Diffuse Layer 2",
        DebugMode::Texture12 => "Diffuse Layer 3",
        DebugMode::Texture14 => "Emissive Layer 2",
        _ => "",
    }
}
