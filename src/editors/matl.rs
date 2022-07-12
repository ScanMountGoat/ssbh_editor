use crate::{
    app::UiState,
    horizontal_separator_empty,
    material::{
        add_parameters, apply_preset, default_material, missing_parameters, param_description,
        remove_parameters, unused_parameters, vector4_labels_long, vector4_labels_short,
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
use std::str::FromStr;
use strum::VariantNames;

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
    material_presets: &mut Vec<MatlEntryData>,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
) -> bool {
    let mut open = true;

    Window::new(format!("Matl Editor ({title})"))
        .open(&mut open)
        .default_size(egui::Vec2::new(400.0, 700.0))
        .resizable(true)
        .show(ctx, |ui| {
            menu_bar(ui, matl, ui_state, material_presets);

            // TODO: Simplify logic for closing window.
            let entry = matl.entries.get_mut(ui_state.matl_selected_material_index);
            let open = preset_window(ui_state, ctx, material_presets, entry);
            if !open {
                ui_state.matl_preset_window_open = false;
            }

            ui.add(egui::Separator::default().horizontal());

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    edit_matl_entries(
                        ui,
                        &mut matl.entries,
                        modl,
                        validation,
                        folder_thumbnails,
                        default_thumbnails,
                        shader_database,
                        red_checkerboard,
                        yellow_checkerboard,
                        &mut ui_state.matl_selected_material_index,
                        &mut ui_state.matl_editor_advanced_mode,
                        &mut ui_state.matl_is_editing_material_label,
                    );
                });
        });

    open
}

// TODO: Validate presets?
pub fn preset_editor(
    ctx: &egui::Context,
    ui_state: &mut UiState,
    presets: &mut Vec<MatlEntryData>,
    default_thumbnails: &[(String, egui::TextureId)],
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
) {
    egui::Window::new("Material Preset Editor")
        .open(&mut ui_state.preset_editor_open)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        // TODO: Default to the loaded presets.json path?
                        if let Some(file) =
                            FileDialog::new().add_filter("JSON", &["json"]).save_file()
                        {
                            save_material_presets(presets, file);
                        }
                    }
                });
            });

            // Use an empty model thumbnail list to encourage using default textures.
            // These textures will be replaced by param specific defaults anyway.
            edit_matl_entries(
                ui,
                presets,
                None,
                &[],
                &[],
                default_thumbnails,
                shader_database,
                red_checkerboard,
                yellow_checkerboard,
                &mut ui_state.preset_selected_material_index,
                &mut ui_state.preset_editor_advanced_mode,
                &mut ui_state.preset_is_editing_material_label,
            );
        });
}

fn save_material_presets(presets: &[MatlEntryData], file: std::path::PathBuf) {
    match serde_json::to_string_pretty(&MatlData {
        major_version: 1,
        minor_version: 6,
        entries: presets.to_vec(),
    }) {
        Ok(presets_json) => {
            if let Err(e) = std::fs::write(file, presets_json) {
                error!("Failed to save material presets JSON: {}", e);
            }
        }
        Err(e) => error!("Failed to convert material presets to JSON: {}", e),
    }
}

fn edit_matl_entries(
    ui: &mut Ui,
    entries: &mut Vec<MatlEntryData>,
    modl: Option<&mut ModlData>,
    validation: &[MatlValidationError],
    folder_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
    selected_material_index: &mut usize,
    advanced_mode: &mut bool,
    is_editing_material_label: &mut bool,
) {
    // Only display a single material at a time.
    // This avoids cluttering the UI.
    let entry = entries.get_mut(*selected_material_index);
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
        let entry = entries.get_mut(*selected_material_index);
        if *is_editing_material_label {
            edit_material_label(entry, is_editing_material_label, ui, &mut modl_entries);
        } else {
            material_combo_box(ui, selected_material_index, entries);
        }

        if !*is_editing_material_label && ui.button("Rename").clicked() {
            // TODO: The material assignments don't always update in the viewport.
            *is_editing_material_label = true;
        }

        if *advanced_mode && ui.button("Delete").clicked() {
            // TODO: Potential panic?
            entries.remove(*selected_material_index);
        }
    });
    horizontal_separator_empty(ui);

    // Advanced mode has more detailed information that most users won't want to edit.
    ui.checkbox(advanced_mode, "Advanced Settings");
    horizontal_separator_empty(ui);

    let entry = entries.get_mut(*selected_material_index);
    if let Some(entry) = entry {
        // TODO: Avoid allocating here.
        let validation: Vec<_> = validation
            .iter()
            .filter(|e| e.entry_index() == *selected_material_index)
            .collect();

        matl_entry_editor(
            ui,
            entry,
            &validation,
            folder_thumbnails,
            default_thumbnails,
            *advanced_mode,
            shader_database,
            red_checkerboard,
            yellow_checkerboard,
        );
    }
}

fn preset_window(
    ui_state: &mut UiState,
    ctx: &egui::Context,
    material_presets: &[MatlEntryData],
    entry: Option<&mut MatlEntryData>,
) -> bool {
    let mut open = ui_state.matl_preset_window_open;
    Window::new("Select Material Preset")
        .open(&mut ui_state.matl_preset_window_open)
        .resizable(false)
        .show(ctx, |ui| {
            if material_presets.is_empty() {
                ui.label("No material presets detected. Make sure the presets.json file is present and contains valid JSON materials.");
            } else {
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
            }
        });
    open
}

fn menu_bar(
    ui: &mut Ui,
    matl: &mut MatlData,
    ui_state: &mut UiState,
    material_presets: &mut Vec<MatlEntryData>,
) {
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

                ui_state.matl_selected_material_index = matl.entries.len() - 1;
            }

            if ui.button("Add Material to Presets").clicked() {
                ui.close_menu();

                // TODO: Prompt for naming the preset?
                if let Some(entry) = matl.entries.get(ui_state.matl_selected_material_index) {
                    material_presets.push(entry.clone());
                }
            }

            if ui.button("Apply Preset").clicked() {
                ui_state.matl_preset_window_open = true;
            }
        });
    });
}

fn edit_material_label(
    entry: Option<&mut MatlEntryData>,
    is_editing_material_label: &mut bool,
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
            *is_editing_material_label = false;
        }
    }
}

fn material_combo_box(ui: &mut Ui, selected_material_index: &mut usize, entries: &[MatlEntryData]) {
    ComboBox::from_id_source("MatlEditorMaterialLabel")
        .width(400.0)
        .show_index(ui, selected_material_index, entries.len(), |i| {
            entries
                .get(i)
                .map(|m| m.material_label.clone())
                .unwrap_or_default()
        });
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
            if let MatlValidationError::MissingRequiredVertexAttributes {
                mesh_name,
                missing_attributes,
                ..
            } = error
            {
                ui.horizontal(|ui| {
                    ui.image(yellow_checkerboard, egui::Vec2::new(16.0, 16.0));
                    ui.label(mesh_name);
                    ui.label(missing_attributes.join(","));
                });
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
                edit_vector(ui, param, !unused_parameters.contains(&param.param_id));
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
    } else {
        ui.allocate_space(egui::Vec2::new(24.0, 24.0));
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

            // TODO: What color space to use?
            // TODO: Add tooltips to other labels?
            // TODO: Only show tooltips after a delay?
            ui.label("Border Color").on_hover_text(
                "The color when sampling UVs outside the range 0.0 to 1.0. Only affects ClampToBorder.",
            );
            edit_color4f_rgba(ui, &mut param.data.border_color);
            ui.end_row();

            ui.label("Lod Bias");
            ui.add(DragValue::new(&mut param.data.lod_bias).speed(0.1));
            ui.end_row();

            // TODO: Make a function for this and share with bone parent index?
            // TODO: Format as 1x, 2x, etc?
            ui.label("Max Anisotropy");
            egui::ComboBox::from_id_source(format!("anis{:?}", param.param_id))
                .selected_text(
                    param
                        .data
                        .max_anisotropy
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut param.data.max_anisotropy, None, "None");
                    ui.separator();
                    for variant in MaxAnisotropy::VARIANTS {
                        ui.selectable_value(
                            &mut param.data.max_anisotropy,
                            Some(MaxAnisotropy::from_str(variant).unwrap()),
                            variant.to_string(),
                        );
                    }
                });
            ui.end_row();
        });
    });
}

fn edit_vector(ui: &mut Ui, param: &mut Vector4Param, enabled: bool) {
    // Disabling the entire row interferes with the grid columns.
    // Disable each item individually.
    ui.add_enabled_ui(enabled, |ui| {
        edit_vector4_rgba(ui, &mut param.data);
    });

    ui.add_enabled_ui(enabled, |ui| {
        ui.label(param_label(param.param_id));
    });

    let edit_component = |ui: &mut Ui, label, value| {
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(
                    DragSlider::new(format!("{:?}.{}", param.param_id, label), value).width(50.0),
                );
            });
        });
    };

    // TODO: Fix spacing for unused labels.
    let labels = vector4_labels_short(param.param_id);
    edit_component(ui, labels[0], &mut param.data.x);
    edit_component(ui, labels[1], &mut param.data.y);
    edit_component(ui, labels[2], &mut param.data.z);
    edit_component(ui, labels[3], &mut param.data.w);
}

fn edit_vector4_rgba(ui: &mut Ui, data: &mut Vector4) {
    // TODO: Edit alpha for params with alpha?
    let mut color = [data.x, data.y, data.z];
    if ui.color_edit_button_rgb(&mut color).changed() {
        data.x = color[0];
        data.y = color[1];
        data.z = color[2];
    }
}

fn edit_color4f_rgba(ui: &mut Ui, data: &mut Color4f) {
    // TODO: Still show the color if the alpha is 0?
    let mut color = [data.r, data.g, data.b, data.a];
    if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
        data.r = color[0];
        data.g = color[1];
        data.b = color[2];
        data.a = color[3];
    }
}

fn edit_vector_advanced(ui: &mut Ui, param: &mut Vector4Param) {
    // TODO: Set custom labels and ranges.
    // TODO: Add parameter descriptions.
    ui.horizontal(|ui| {
        edit_vector4_rgba(ui, &mut param.data);
        ui.label(param_label(param.param_id));
    });
    ui.indent("indent", |ui| {
        let labels = vector4_labels_long(param.param_id);
        Grid::new(param.param_id.to_string()).show(ui, |ui| {
            ui.label(labels[0]);
            ui.add(
                DragSlider::new(format!("{:?}.x", param.param_id), &mut param.data.x).width(150.0),
            );
            ui.end_row();

            ui.label(labels[1]);
            ui.add(
                DragSlider::new(format!("{:?}.y", param.param_id), &mut param.data.y).width(150.0),
            );
            ui.end_row();

            ui.label(labels[2]);
            ui.add(
                DragSlider::new(format!("{:?}.z", param.param_id), &mut param.data.z).width(150.0),
            );
            ui.end_row();

            ui.label(labels[3]);
            ui.add(
                DragSlider::new(format!("{:?}.w", param.param_id), &mut param.data.w).width(150.0),
            );
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
