use crate::{
    app::UiState,
    horizontal_separator_empty,
    material::{
        add_parameters, apply_preset, default_material, missing_parameters, param_description,
        remove_parameters, unused_parameters,
    },
    validation::MatlValidationError,
    widgets::*,
};
use egui::{ComboBox, DragValue, Grid, Label, ScrollArea, Slider, Ui, Window};
use log::error;
use rfd::FileDialog;
use ssbh_data::{matl_data::*, modl_data::ModlEntryData, prelude::*};
use ssbh_wgpu::ShaderDatabase;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn matl_editor(
    ctx: &egui::Context,
    title: &str,
    ui_state: &mut UiState,
    matl: &mut MatlData,
    modl: Option<&mut ModlData>,
    validation: &[MatlValidationError],
    folder_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    shader_database: &ShaderDatabase,
    material_presets: &[MatlEntryData],
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
) -> bool {
    let mut open = true;

    Window::new(format!("Matl Editor ({title})"))
        .open(&mut open)
        .default_size(egui::Vec2::new(400.0, 700.0))
        .resizable(true)
        .show(ctx, |ui| {
            menu_bar(ui, matl, ui_state);

            // TODO: Simplify logic for closing window.
            let entry = matl.entries.get_mut(ui_state.selected_material_index);
            let open = preset_window(ui_state, ctx, material_presets, entry);
            if !open {
                ui_state.preset_window_open = false;
            }

            ui.add(egui::Separator::default().horizontal());

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Only display a single material at a time.
                    // This avoids cluttering the UI.
                    let entry = matl.entries.get_mut(ui_state.selected_material_index);
                    let mut modl_entries: Vec<_> = entry
                        .and_then(|entry| {
                            modl.map(|modl| {
                                modl.entries
                                    .iter_mut()
                                    .filter(|e| e.material_label == entry.material_label)
                                    .collect()
                            })
                        })
                        .unwrap_or_default();

                    ui.heading("Material");
                    ui.horizontal(|ui| {
                        ui.label("Material");
                        let entry = matl.entries.get_mut(ui_state.selected_material_index);
                        if ui_state.is_editing_material_label {
                            edit_material_label(entry, ui_state, ui, &mut modl_entries);
                        } else {
                            material_combo_box(ui, ui_state, matl);
                        }

                        if ui.button("Rename").clicked() {
                            // TODO: The material assignments don't always update in the viewport.
                            ui_state.is_editing_material_label = true;
                        }

                        if ui_state.matl_editor_advanced_mode && ui.button("Delete").clicked() {
                            // TODO: Potential panic?
                            matl.entries.remove(ui_state.selected_material_index);
                        }
                    });
                    horizontal_separator_empty(ui);

                    // Advanced mode has more detailed information that most users won't want to edit.
                    ui.checkbox(&mut ui_state.matl_editor_advanced_mode, "Advanced Settings");
                    horizontal_separator_empty(ui);

                    let entry = matl.entries.get_mut(ui_state.selected_material_index);

                    if let Some(entry) = entry {
                        // TODO: Avoid allocating here.
                        let validation: Vec<_> = validation
                            .iter()
                            .filter(|e| e.entry_index() == ui_state.selected_material_index)
                            .collect();
                        matl_entry_editor(
                            ui,
                            entry,
                            &validation,
                            folder_thumbnails,
                            default_thumbnails,
                            ui_state.matl_editor_advanced_mode,
                            shader_database,
                            red_checkerboard,
                            yellow_checkerboard,
                        );
                    }
                });
        });

    open
}

fn preset_window(
    ui_state: &mut UiState,
    ctx: &egui::Context,
    material_presets: &[MatlEntryData],
    entry: Option<&mut MatlEntryData>,
) -> bool {
    let mut open = ui_state.preset_window_open;
    Window::new("Select Material Preset")
        .open(&mut ui_state.preset_window_open)
        .resizable(false)
        .show(ctx, |ui| {
            for (i, preset) in material_presets.iter().enumerate() {
                ui.selectable_value(
                    &mut ui_state.selected_material_preset_index,
                    i,
                    &preset.material_label,
                );
            }

            if ui.button("Apply").clicked() {
                if let Some(preset) = material_presets.get(ui_state.selected_material_preset_index)
                {
                    if let Some(entry) = entry {
                        *entry = apply_preset(entry, preset);
                    }
                }

                open = false;
            }
        });
    open
}

fn menu_bar(ui: &mut Ui, matl: &mut MatlData, ui_state: &mut UiState) {
    egui::menu::bar(ui, |ui| {
        egui::menu::menu_button(ui, "File", |ui| {
            if ui.button("Save").clicked() {
                ui.close_menu();

                if let Some(file) = FileDialog::new()
                    .add_filter("Matl", &["numatb"])
                    .save_file()
                {
                    if let Err(e) = matl.write_to_file(file) {
                        error!("Failed to save Matl (.numatb): {}", e);
                    }
                }
            }
        });

        egui::menu::menu_button(ui, "Material", |ui| {
            if ui.button("Add New Material").clicked() {
                ui.close_menu();

                // TODO: Select options from presets?
                let new_entry = default_material();
                matl.entries.push(new_entry);

                ui_state.selected_material_index = matl.entries.len() - 1;
            }

            if ui.button("Apply Preset").clicked() {
                ui_state.preset_window_open = true;
            }
        });
    });
}

fn edit_material_label(
    entry: Option<&mut MatlEntryData>,
    ui_state: &mut UiState,
    ui: &mut Ui,
    modl_entries: &mut [&mut ModlEntryData],
) {
    // TODO: Get this to work with lost_focus for efficiency.
    // TODO: Show errors if these checks fail?
    if let Some(entry) = entry {
        let response = ui.add_sized(
            egui::Vec2::new(400.0, 20.0),
            egui::TextEdit::singleline(&mut entry.material_label),
        );

        if response.changed() {
            // Rename any effected modl entries if the material label changes.
            // Keep track of modl entries since materials may be renamed.
            for modl_entry in modl_entries {
                modl_entry.material_label = entry.material_label.clone();
            }
        }

        if response.lost_focus() {
            ui_state.is_editing_material_label = false;
        }
    }
}

fn material_combo_box(ui: &mut Ui, ui_state: &mut UiState, matl: &MatlData) {
    ComboBox::from_id_source("MatlEditorMaterialLabel")
        .width(400.0)
        .show_index(
            ui,
            &mut ui_state.selected_material_index,
            matl.entries.len(),
            |i| {
                matl.entries
                    .get(i)
                    .map(|m| m.material_label.clone())
                    .unwrap_or_default()
            },
        );
}

fn edit_shader_label(
    ui: &mut Ui,
    shader_label: &mut String,
    is_valid: bool,
    red_checkerboard: egui::TextureId,
) {
    if is_valid {
        ui.text_edit_singleline(shader_label);
    } else {
        ui.horizontal(|ui| {
            ui.image(red_checkerboard, egui::Vec2::new(16.0, 16.0));
            ui.add(egui::TextEdit::singleline(shader_label).text_color(egui::Color32::RED));
        })
        .response
        .on_hover_text(format!("{} is not a valid shader label. Copy an existing shader label or apply a material preset.", shader_label));
    }
}

fn matl_entry_editor(
    ui: &mut Ui,
    entry: &mut ssbh_data::matl_data::MatlEntryData,
    validation_errors: &[&MatlValidationError],
    texture_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    advanced_mode: bool,
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
) {
    let program = shader_database.get(entry.shader_label.get(..24).unwrap_or(""));

    ui.heading("Shader");
    ui.horizontal(|ui| {
        // TODO: This doesn't update properly in the viewport.
        ui.label("Shader Label");
        edit_shader_label(
            ui,
            &mut entry.shader_label,
            program.is_some(),
            red_checkerboard,
        );
    });
    if advanced_mode {
        Grid::new("shader_grid").show(ui, |ui| {
            // TODO: Should this be part of the basic mode.
            ui.label("Render Pass");
            let shader = entry.shader_label.get(..24).unwrap_or("").to_string();
            ComboBox::from_id_source("render pass")
                .selected_text(entry.shader_label.get(25..).unwrap_or(""))
                .show_ui(ui, |ui| {
                    for pass in [
                        shader.clone() + "_opaque",
                        shader.clone() + "_far",
                        shader.clone() + "_sort",
                        shader.clone() + "_near",
                    ] {
                        ui.selectable_value(
                            &mut entry.shader_label,
                            pass.to_string(),
                            pass.get(25..).unwrap_or(""),
                        );
                    }
                });
            ui.end_row();

            if let Some(program) = program {
                ui.label("Alpha Testing");
                ui.label(program.discard.to_string());
                ui.end_row();

                ui.label("Vertex Attributes");
                ui.add(Label::new(program.vertex_attributes.join(",")).wrap(true));
                ui.end_row();
            }
        });
    }
    horizontal_separator_empty(ui);

    // TODO: Show errors in the material selector.
    // TODO: Add a button to open the mesh editor?
    if validation_errors.iter().any(|e| {
        matches!(
            e,
            MatlValidationError::MissingRequiredVertexAttributes { .. }
        )
    }) {
        ui.heading("Shader Errors");
        ui.label(
            "Meshes with this material are missing these vertex attributes required by the shader.",
        );
        for error in validation_errors {
            match error {
                MatlValidationError::MissingRequiredVertexAttributes {
                    mesh_name,
                    missing_attributes,
                    ..
                } => {
                    ui.horizontal(|ui| {
                        ui.image(yellow_checkerboard, egui::Vec2::new(16.0, 16.0));
                        ui.label(mesh_name);
                        ui.label(missing_attributes.join(","));
                    });
                }
                _ => (),
            }
        }

        horizontal_separator_empty(ui);
    }

    ui.heading("Parameters");

    let missing_parameters = program
        .map(|program| missing_parameters(entry, program))
        .unwrap_or_default();

    let unused_parameters = program
        .map(|program| unused_parameters(entry, program))
        .unwrap_or_default();

    if !missing_parameters.is_empty() {
        let text = if missing_parameters.len() == 1 {
            "Add 1 Missing Parameter".to_string()
        } else {
            format!("Add {} Missing Parameters", missing_parameters.len())
        };
        if ui.button(text).clicked() {
            add_parameters(entry, &missing_parameters);
        }
    }

    if !unused_parameters.is_empty() {
        let text = if unused_parameters.len() == 1 {
            "Remove 1 Unused Parameter".to_string()
        } else {
            format!("Remove {} Unused Parameters", unused_parameters.len())
        };
        if ui.button(text).clicked() {
            remove_parameters(entry, &unused_parameters);
        }
    }

    if !missing_parameters.is_empty() || !unused_parameters.is_empty() {
        horizontal_separator_empty(ui);
    }

    for param in entry.booleans.iter_mut() {
        ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
            ui.checkbox(&mut param.data, param_label(param.param_id))
        });
    }
    horizontal_separator_empty(ui);

    // TODO: Find a consistent way to disable ui if unused and show a tooltip.
    for param in entry.floats.iter_mut() {
        ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
            ui.horizontal(|ui| {
                // TODO: Store this size somewhere to ensure labels align?
                ui.label(param_label(param.param_id));
                ui.add(Slider::new(&mut param.data, 0.0..=1.0));
            })
        });
    }
    horizontal_separator_empty(ui);

    if advanced_mode {
        for param in entry.vectors.iter_mut() {
            ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
                edit_vector_advanced(ui, param);
            });
        }
    } else {
        Grid::new("vectors").show(ui, |ui| {
            for param in entry.vectors.iter_mut() {
                ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
                    edit_vector(ui, param);
                });
                ui.end_row();
            }
        });
    }
    horizontal_separator_empty(ui);

    // The defaults for samplers are usually fine, so don't show samplers by default.
    if advanced_mode {
        for param in &mut entry.samplers {
            ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
                edit_sampler(ui, param);
            });
        }
        horizontal_separator_empty(ui);
    }

    Grid::new("matl textures").show(ui, |ui| {
        for param in &mut entry.textures {
            // TODO: Get disabled UI working with the texture grid.
            edit_texture(
                ui,
                param,
                texture_thumbnails,
                default_thumbnails,
                advanced_mode,
            );
            ui.end_row();
        }
    });
    horizontal_separator_empty(ui);

    // TODO: Reflecting changes to these values in the viewport requires recreating pipelines.
    if advanced_mode {
        // Edits to RasterizerState0 are rare, so restrict it to advanced mode.
        for param in &mut entry.rasterizer_states {
            edit_rasterizer(ui, param);
        }
        horizontal_separator_empty(ui);
    }

    for param in &mut entry.blend_states {
        edit_blend(ui, param);
    }
}

fn edit_blend(ui: &mut Ui, param: &mut BlendStateParam) {
    ui.label(param_label(param.param_id));
    ui.indent("indent", |ui| {
        Grid::new(param.param_id.to_string()).show(ui, |ui| {
            enum_combo_box(
                ui,
                "Source Color",
                format!("srccolor{:?}", param.param_id.to_string()),
                &mut param.data.source_color,
            );
            ui.end_row();

            enum_combo_box(
                ui,
                "Destination Color",
                format!("dstcolor{:?}", param.param_id.to_string()),
                &mut param.data.destination_color,
            );
            ui.end_row();

            ui.checkbox(
                &mut param.data.alpha_sample_to_coverage,
                "Alpha Sample to Coverage",
            );
            ui.end_row();
            // TODO: Basic blend state can just expose a selection for "additive", "alpha", or "opaque".
            // TODO: Research in game examples for these presets (premultiplied alpha?)
        });
    });
}

fn edit_rasterizer(ui: &mut Ui, param: &mut RasterizerStateParam) {
    ui.label(param_label(param.param_id));
    ui.indent("indent", |ui| {
        // TODO: These param IDs might not be unique?
        Grid::new(param.param_id.to_string()).show(ui, |ui| {
            enum_combo_box(
                ui,
                "Polygon Fill",
                format!("fill{:?}", param.param_id.to_string()),
                &mut param.data.fill_mode,
            );
            ui.end_row();
            enum_combo_box(
                ui,
                "Cull Mode",
                format!("cull{:?}", param.param_id.to_string()),
                &mut param.data.cull_mode,
            );
            ui.end_row();
        });
    });
}

fn edit_texture(
    ui: &mut Ui,
    param: &mut TextureParam,
    texture_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    advanced_mode: bool,
) {
    // TODO: Should this check be case sensitive?
    // TODO: Create a texture for an invalid thumbnail or missing texture?
    // TODO: Should this functionality be part of ssbh_wgpu?
    ui.label(param_label(param.param_id));
    // Texture parameters don't include the file extension since it's implied.
    // Texture names aren't case sensitive.
    // TODO: Avoid allocating here.
    // TODO: Don't store the extension with the thumbnail at all?
    if let Some(thumbnail) = texture_thumbnails
        .iter()
        .chain(default_thumbnails.iter())
        .find(|t| {
            Path::new(&t.0)
                .with_extension("")
                .to_string_lossy()
                .eq_ignore_ascii_case(&param.data)
        })
        .map(|t| t.1)
    {
        ui.image(thumbnail, egui::Vec2::new(24.0, 24.0));
    }
    if advanced_mode {
        // Let users enter names manually if texture files aren't present.
        ui.text_edit_singleline(&mut param.data);
    } else {
        // Texture files should be present in the folder, which allows for image previews.
        ComboBox::from_id_source(param.param_id.to_string())
            .selected_text(&param.data)
            .width(300.0)
            .show_ui(ui, |ui| {
                // TODO: Is it safe to assume the thumbnails have all the available textures?
                for (name, thumbnail) in texture_thumbnails.iter().chain(default_thumbnails.iter())
                {
                    // Material parameters don't include the .nutexb extension.
                    let text = Path::new(name)
                        .with_extension("")
                        .to_string_lossy()
                        .to_string();

                    // TODO: Show a texture as selected even if the case doesn't match?
                    ui.horizontal(|ui| {
                        ui.image(*thumbnail, egui::Vec2::new(24.0, 24.0));
                        ui.selectable_value(&mut param.data, text.to_string(), text);
                    });
                }
            });
    }
}

fn edit_sampler(ui: &mut Ui, param: &mut SamplerParam) {
    ui.label(param_label(param.param_id));
    ui.indent("indent", |ui| {
        Grid::new(param.param_id.to_string()).show(ui, |ui| {
            enum_combo_box(
                ui,
                "Wrap S",
                format!("wraps{:?}", param.param_id),
                &mut param.data.wraps,
            );
            ui.end_row();

            enum_combo_box(
                ui,
                "Wrap T",
                format!("wrapt{:?}", param.param_id),
                &mut param.data.wrapt,
            );
            ui.end_row();

            enum_combo_box(
                ui,
                "Wrap R",
                format!("wrapr{:?}", param.param_id),
                &mut param.data.wrapr,
            );
            ui.end_row();

            enum_combo_box(
                ui,
                "Min Filter",
                format!("minfilter{:?}", param.param_id),
                &mut param.data.min_filter,
            );
            ui.end_row();

            enum_combo_box(
                ui,
                "Mag Filter",
                format!("magfilter{:?}", param.param_id),
                &mut param.data.mag_filter,
            );
            ui.end_row();
        });
    });
}

fn edit_vector(ui: &mut Ui, param: &mut Vector4Param) {
    ui.label(param_label(param.param_id));
    let mut color = [param.data.x, param.data.y, param.data.z];
    if ui.color_edit_button_rgb(&mut color).changed() {
        param.data.x = color[0];
        param.data.y = color[1];
        param.data.z = color[2];
    }
    ui.horizontal(|ui| {
        ui.label("X");
        ui.add(DragValue::new(&mut param.data.x).speed(0.01));
        ui.label("Y");
        ui.add(DragValue::new(&mut param.data.y).speed(0.01));
        ui.label("Z");
        ui.add(DragValue::new(&mut param.data.z).speed(0.01));
        ui.label("W");
        ui.add(DragValue::new(&mut param.data.w).speed(0.01));
    });
}

fn edit_vector_advanced(ui: &mut Ui, param: &mut Vector4Param) {
    // TODO: Make a custom expander that expands to sliders?
    // TODO: Set custom labels and ranges.
    // TODO: Add parameter descriptions.
    ui.horizontal(|ui| {
        ui.add_sized([80.0, 20.0], Label::new(param.param_id.to_string()));

        let mut color = [param.data.x, param.data.y, param.data.z];
        if ui.color_edit_button_rgb(&mut color).changed() {
            param.data.x = color[0];
            param.data.y = color[1];
            param.data.z = color[2];
        }
    });
    ui.indent("indent", |ui| {
        Grid::new(param.param_id.to_string()).show(ui, |ui| {
            ui.label("X");
            ui.add(Slider::new(&mut param.data.x, 0.0..=1.0).clamp_to_range(false));
            ui.end_row();

            ui.label("Y");
            ui.add(Slider::new(&mut param.data.y, 0.0..=1.0).clamp_to_range(false));
            ui.end_row();

            ui.label("Z");
            ui.add(Slider::new(&mut param.data.z, 0.0..=1.0).clamp_to_range(false));
            ui.end_row();

            ui.label("W");
            ui.add(Slider::new(&mut param.data.w, 0.0..=1.0).clamp_to_range(false));
            ui.end_row();
        });
    });
}

fn param_label(p: ParamId) -> String {
    let description = param_description(p);
    if !description.is_empty() {
        format!("{} ({})", p, description)
    } else {
        p.to_string()
    }
}
