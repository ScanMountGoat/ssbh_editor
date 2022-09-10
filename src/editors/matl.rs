use crate::{
    app::{warning_icon, MatlEditorState, UiState},
    horizontal_separator_empty,
    material::{
        add_parameters, apply_preset, default_material, missing_parameters, param_description,
        remove_parameters, unused_parameters, vector4_labels_long, vector4_labels_short,
    },
    path::presets_file,
    presets::{load_json_presets, load_xml_presets},
    validation::MatlValidationError,
    widgets::*,
};
use egui::{
    Button, CollapsingHeader, ComboBox, Context, DragValue, Grid, Label, ScrollArea, TextEdit, Ui,
    Window,
};
use log::error;
use rfd::FileDialog;
use ssbh_data::{matl_data::*, modl_data::ModlEntryData, prelude::*, Color4f, Vector4};
use ssbh_wgpu::{ShaderDatabase, ShaderProgram};
use std::path::Path;
use std::str::FromStr;
use strum::VariantNames;

#[allow(clippy::too_many_arguments)]
pub fn matl_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    ui_state: &mut UiState,
    matl: &mut MatlData,
    modl: Option<&mut ModlData>,
    validation_errors: &[MatlValidationError],
    folder_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    shader_database: &ShaderDatabase,
    material_presets: &mut Vec<MatlEntryData>,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    Window::new(format!("Matl Editor ({title})"))
        .open(&mut open)
        .default_size(egui::Vec2::new(400.0, 700.0))
        .resizable(true)
        .show(ctx, |ui| {
            menu_bar(ui, matl, ui_state, material_presets, folder_name, file_name);
            ui.separator();

            // TODO: Simplify logic for closing window.
            let entry = matl
                .entries
                .get_mut(ui_state.matl_editor.selected_material_index);
            let open = preset_window(ui_state, ctx, material_presets, entry);
            if !open {
                ui_state.matl_preset_window_open = false;
            }

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    changed |= edit_matl_entries(
                        ctx,
                        ui,
                        &mut matl.entries,
                        modl,
                        validation_errors,
                        folder_thumbnails,
                        default_thumbnails,
                        shader_database,
                        red_checkerboard,
                        yellow_checkerboard,
                        &mut ui_state.matl_editor,
                    );
                });
        });

    (open, changed)
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

                        let path = presets_file();
                        save_material_presets(presets, path);
                    }
                });

                egui::menu::menu_button(ui, "Import", |ui| {
                    // Import presets from external formats.
                    if ui
                        .add(Button::new("JSON Presets (ssbh_data_json)").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Matl JSON", &["json"])
                            .pick_file()
                        {
                            load_presets_from_file(presets, file, load_json_presets);
                        }
                    }

                    if ui
                        .add(Button::new("XML Presets (Cross Mod)").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Matl XML", &["xml"])
                            .pick_file()
                        {
                            load_presets_from_file(presets, file, load_xml_presets);
                        }
                    }
                });

                egui::menu::menu_button(ui, "Material", |ui| {
                    if ui.button("Remove Duplicates").clicked() {
                        ui.close_menu();

                        remove_duplicates(presets);
                    }
                });
                help_menu(ui);
            });

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Use an empty model thumbnail list to encourage using default textures.
                    // These textures will be replaced by param specific defaults anyway.
                    edit_matl_entries(
                        ctx,
                        ui,
                        presets,
                        None,
                        &[],
                        &[],
                        default_thumbnails,
                        shader_database,
                        red_checkerboard,
                        yellow_checkerboard,
                        &mut ui_state.preset_editor,
                    );
                });
        });
}

fn remove_duplicates(entries: &mut Vec<MatlEntryData>) {
    // Remove duplicates using PartialEq while preserving ordering.
    // TODO: Avoid clone?
    let mut visited = Vec::with_capacity(entries.len());
    entries.retain(|item| {
        if visited.contains(item) {
            false
        } else {
            visited.push(item.clone());
            true
        }
    });
}

fn load_presets_from_file<F: Fn(&[u8]) -> anyhow::Result<Vec<MatlEntryData>>>(
    presets: &mut Vec<MatlEntryData>,
    file: std::path::PathBuf,
    load_presets: F,
) {
    match std::fs::read(&file)
        .map_err(|e| {
            error!("Error reading presets file {:?}: {}", file, e);
            e.into()
        })
        .and_then(|bytes| load_presets(&bytes))
    {
        Ok(new_presets) => presets.extend(new_presets),
        Err(e) => error!("Error importing presets: {}", e),
    }
}

fn save_material_presets(presets: &[MatlEntryData], file: std::path::PathBuf) {
    // TODO: Give a visual indication that the file saved?
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
    ctx: &Context,
    ui: &mut Ui,
    entries: &mut Vec<MatlEntryData>,
    modl: Option<&mut ModlData>,
    validation_errors: &[MatlValidationError],
    folder_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
    editor_state: &mut MatlEditorState,
) -> bool {
    // Only display a single material at a time.
    // This avoids cluttering the UI.
    let entry = entries.get_mut(editor_state.selected_material_index);
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

    let mut changed = false;

    ui.heading("Material");
    ui.horizontal(|ui| {
        ui.label("Material");
        let entry = entries.get_mut(editor_state.selected_material_index);
        if editor_state.is_editing_material_label {
            changed |= edit_material_label(
                entry,
                &mut editor_state.is_editing_material_label,
                ui,
                &mut modl_entries,
            );
        } else {
            changed |= material_combo_box(
                ui,
                &mut editor_state.selected_material_index,
                &mut editor_state.hovered_material_index,
                entries,
                validation_errors,
            );
        }

        if !editor_state.is_editing_material_label && ui.button("Rename").clicked() {
            editor_state.is_editing_material_label = true;
        }

        if editor_state.advanced_mode && ui.button("Delete").clicked() {
            // TODO: Potential panic?
            entries.remove(editor_state.selected_material_index);
        }
    });
    horizontal_separator_empty(ui);

    // Advanced mode has more detailed information that most users won't want to edit.
    ui.checkbox(&mut editor_state.advanced_mode, "Advanced Settings");
    horizontal_separator_empty(ui);

    let entry = entries.get_mut(editor_state.selected_material_index);
    if let Some(entry) = entry {
        // TODO: Avoid allocating here.
        let entry_validation_errors: Vec<_> = validation_errors
            .iter()
            .filter(|e| e.entry_index() == editor_state.selected_material_index)
            .collect();

        changed |= matl_entry_editor(
            ctx,
            ui,
            entry,
            &entry_validation_errors,
            folder_thumbnails,
            default_thumbnails,
            editor_state.advanced_mode,
            shader_database,
            red_checkerboard,
            yellow_checkerboard,
        );
    }

    changed
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
    folder_name: &str,
    file_name: &str,
) {
    egui::menu::bar(ui, |ui| {
        egui::menu::menu_button(ui, "File", |ui| {
            if ui.button("Save").clicked() {
                ui.close_menu();

                let file = Path::new(folder_name).join(file_name);
                if let Err(e) = matl.write_to_file(&file) {
                    error!("Failed to save {:?}: {}", file, e);
                }
            }

            if ui.button("Save As...").clicked() {
                ui.close_menu();

                if let Some(file) = FileDialog::new()
                    .add_filter("Matl", &["numatb"])
                    .save_file()
                {
                    if let Err(e) = matl.write_to_file(&file) {
                        error!("Failed to save {:?}: {}", file, e);
                    }
                }
            }
        });

        egui::menu::menu_button(ui, "Material", |ui| {
            let button = |ui: &mut Ui, text| ui.add(Button::new(text).wrap(false));
            if ui.button("Add New Material").clicked() {
                ui.close_menu();

                // TODO: Select options from presets?
                let new_entry = default_material();
                matl.entries.push(new_entry);

                ui_state.matl_editor.selected_material_index = matl.entries.len() - 1;
            }

            if button(ui, "Duplicate Current Material").clicked() {
                ui.close_menu();

                if let Some(old_entry) = matl
                    .entries
                    .get(ui_state.matl_editor.selected_material_index)
                {
                    let mut new_entry = old_entry.clone();
                    new_entry.material_label.push_str("_copy");
                    matl.entries.push(new_entry);
                }

                ui_state.matl_editor.selected_material_index = matl.entries.len() - 1;
            }

            if ui.button("Add Material to Presets").clicked() {
                ui.close_menu();

                // TODO: Prompt for naming the preset?
                if let Some(entry) = matl
                    .entries
                    .get(ui_state.matl_editor.selected_material_index)
                {
                    material_presets.push(entry.clone());
                }
            }

            if ui.button("Apply Preset").clicked() {
                ui.close_menu();

                ui_state.matl_preset_window_open = true;
            }

            if ui.button("Remove Duplicates").clicked() {
                ui.close_menu();

                remove_duplicates(&mut matl.entries);
            }
        });

        help_menu(ui);
    });
}

fn help_menu(ui: &mut Ui) {
    egui::menu::menu_button(ui, "Help", |ui| {
        let button = |ui: &mut Ui, text| ui.add(Button::new(text).wrap(false));

        if button(ui, "Material Reference (GitHub)").clicked() {
            ui.close_menu();
            let link = "https://github.com/ScanMountGoat/Smush-Material-Research/blob/master/Material%20Parameters.md";
            if let Err(open_err) = open::that(link) {
                log::error!("Failed to open link ({link}). {open_err}");
            }
        }

        if ui.button("Matl Editor Wiki").clicked() {
            ui.close_menu();

            let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Matl-Editor";
            if let Err(e) = open::that(link) {
                log::error!("Failed to open {link}: {e}");
            }
        }
    });
}

fn edit_material_label(
    entry: Option<&mut MatlEntryData>,
    is_editing_material_label: &mut bool,
    ui: &mut Ui,
    modl_entries: &mut [&mut ModlEntryData],
) -> bool {
    // TODO: Get this to work with lost_focus for efficiency.
    // TODO: Show errors if these checks fail?
    let mut changed = false;
    if let Some(entry) = entry {
        let response = ui.add_sized(
            egui::Vec2::new(400.0, 20.0),
            egui::TextEdit::singleline(&mut entry.material_label),
        );

        changed = response.changed();
        if changed {
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

    changed
}

fn material_combo_box(
    ui: &mut Ui,
    selected_index: &mut usize,
    hovered_index: &mut Option<usize>,
    entries: &[MatlEntryData],
    validation: &[MatlValidationError],
) -> bool {
    let selected_text = entries
        .get(*selected_index)
        .map(|e| e.material_label.clone())
        .unwrap_or_default();

    let response = ComboBox::from_id_source("MatlEditorMaterialLabel")
        .selected_text(selected_text)
        .width(400.0)
        .show_ui(ui, |ui| {
            for (i, entry) in entries.iter().enumerate() {
                ui.horizontal(|ui| {
                    if validation.iter().any(|e| e.entry_index() == i) {
                        warning_icon(ui);
                    }
                    let response =
                        ui.selectable_value(selected_index, i, entry.material_label.clone());
                    if response.hovered() {
                        // Used for material mask rendering.
                        *hovered_index = Some(i);
                    }
                });
            }
        });

    if response.inner.is_none() {
        // The menu was closed, so disable the material mask.
        *hovered_index = None;
    }

    response.response.changed()
}

fn edit_shader_label(
    ui: &mut Ui,
    shader_label: &mut String,
    is_valid: bool,
    red_checkerboard: egui::TextureId,
) -> bool {
    let mut changed = false;
    if is_valid {
        changed |= ui.text_edit_singleline(shader_label).changed();
    } else {
        ui.horizontal(|ui| {
            ui.image(red_checkerboard, egui::Vec2::new(16.0, 16.0));
            changed |= ui.add(egui::TextEdit::singleline(shader_label).text_color(egui::Color32::RED)).changed();
        })
        .response
        .on_hover_text(format!("{} is not a valid shader label. Copy an existing shader label or apply a material preset.", shader_label));
    }

    changed
}

fn matl_entry_editor(
    _ctx: &Context,
    ui: &mut Ui,
    entry: &mut ssbh_data::matl_data::MatlEntryData,
    validation_errors: &[&MatlValidationError],
    texture_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    advanced_mode: bool,
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
) -> bool {
    let mut changed = false;

    let program = shader_database.get(entry.shader_label.get(..24).unwrap_or(""));

    ui.heading("Shader");
    ui.horizontal(|ui| {
        ui.label("Shader Label");
        changed |= edit_shader_label(
            ui,
            &mut entry.shader_label,
            program.is_some(),
            red_checkerboard,
        );
    });

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
                        pass.clone(),
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
            ui.add(Label::new(program.vertex_attributes.join(", ")).wrap(true));
            ui.end_row();
        }
    });
    horizontal_separator_empty(ui);

    // TODO: Add a button to open the mesh editor?
    if validation_errors.iter().any(|e| {
        matches!(
            e,
            MatlValidationError::MissingRequiredVertexAttributes { .. }
        )
    }) {
        ui.horizontal(|ui| {
            ui.image(yellow_checkerboard, egui::Vec2::new(16.0, 16.0));
            ui.heading("Vertex Attribute Errors");
        });
        ui.label(
            "Meshes with this material are missing these vertex attributes required by the shader.",
        );
        ui.label(
            "Assign a material with a different shader or add these attributes in the Mesh Editor.",
        );
        horizontal_separator_empty(ui);

        Grid::new("attribute_error_grid").show(ui, |ui| {
            for error in validation_errors {
                if let MatlValidationError::MissingRequiredVertexAttributes {
                    mesh_name,
                    missing_attributes,
                    ..
                } = error
                {
                    ui.label(mesh_name);
                    ui.label(missing_attributes.join(","));
                    ui.end_row();
                }
            }
        });

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
            "Add 1 Missing Parameter".to_owned()
        } else {
            format!("Add {} missing parameters", missing_parameters.len())
        };
        if ui.button(text).clicked() {
            add_parameters(entry, &missing_parameters);
            changed = true;
        }
    }

    if !unused_parameters.is_empty() {
        let text = if unused_parameters.len() == 1 {
            "Remove 1 Unused Parameter".to_owned()
        } else {
            format!("Remove {} unused parameters", unused_parameters.len())
        };
        if ui.button(text).clicked() {
            remove_parameters(entry, &unused_parameters);
            changed = true;
        }
    }

    if !missing_parameters.is_empty() || !unused_parameters.is_empty() {
        horizontal_separator_empty(ui);
    }

    for param in entry.booleans.iter_mut() {
        ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
            changed |= ui
                .checkbox(&mut param.data, param_label(param.param_id))
                .changed();
        });
    }
    horizontal_separator_empty(ui);

    // TODO: Show a tooltip to explain why entries are disabled?
    for param in entry.floats.iter_mut() {
        let id = egui::Id::new(param.param_id.to_string());
        ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
            ui.horizontal(|ui| {
                ui.label(param_label(param.param_id));
                changed |= ui.add(DragSlider::new(id, &mut param.data)).changed();
            })
        });
    }
    horizontal_separator_empty(ui);

    Grid::new("vectors").show(ui, |ui| {
        for param in entry.vectors.iter_mut() {
            changed |= edit_vector(
                ui,
                param,
                !unused_parameters.contains(&param.param_id),
                program,
            );
            ui.end_row();
        }
    });
    horizontal_separator_empty(ui);

    Grid::new("matl textures").show(ui, |ui| {
        for param in &mut entry.textures {
            changed |= edit_texture(
                ui,
                param,
                texture_thumbnails,
                default_thumbnails,
                advanced_mode,
                !unused_parameters.contains(&param.param_id),
            );
            ui.end_row();
        }
    });
    horizontal_separator_empty(ui);

    for param in &mut entry.samplers {
        ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
            changed |= edit_sampler(ui, param);
        });
    }
    horizontal_separator_empty(ui);

    // TODO: Reflecting changes to these values in the viewport requires recreating pipelines.
    for param in &mut entry.rasterizer_states {
        changed |= edit_rasterizer(ui, param);
    }
    horizontal_separator_empty(ui);

    for param in &mut entry.blend_states {
        changed |= edit_blend(ui, param);
    }

    changed
}

fn edit_blend(ui: &mut Ui, param: &mut BlendStateParam) -> bool {
    let mut changed = false;

    CollapsingHeader::new(param_label(param.param_id))
        .default_open(true)
        .show(ui, |ui| {
            let id = egui::Id::new(param.param_id.to_string());

            Grid::new(id).show(ui, |ui| {
                changed |= enum_combo_box(
                    ui,
                    "Source Color",
                    id.with("srccolor"),
                    &mut param.data.source_color,
                );
                ui.end_row();

                changed |= enum_combo_box(
                    ui,
                    "Destination Color",
                    id.with("dstcolor"),
                    &mut param.data.destination_color,
                );
                ui.end_row();

                changed |= ui
                    .checkbox(
                        &mut param.data.alpha_sample_to_coverage,
                        "Alpha Sample to Coverage",
                    )
                    .changed();
                ui.end_row();
                // TODO: Basic blend state can just expose a selection for "additive", "alpha", or "opaque".
                // TODO: Research in game examples for these presets (premultiplied alpha?)
            });
        });

    changed
}

fn edit_rasterizer(ui: &mut Ui, param: &mut RasterizerStateParam) -> bool {
    let mut changed = false;

    CollapsingHeader::new(param_label(param.param_id)).show(ui, |ui| {
        let id = egui::Id::new(param.param_id.to_string());

        Grid::new(id).show(ui, |ui| {
            changed |= enum_combo_box(
                ui,
                "Polygon Fill",
                id.with("fill"),
                &mut param.data.fill_mode,
            );
            ui.end_row();

            changed |= enum_combo_box(ui, "Cull Mode", id.with("cull"), &mut param.data.cull_mode);
            ui.end_row();

            ui.label("Depth Bias");
            ui.add(DragValue::new(&mut param.data.depth_bias).speed(0.1));
            ui.end_row();
        });
    });

    changed
}

fn edit_texture(
    ui: &mut Ui,
    param: &mut TextureParam,
    texture_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    advanced_mode: bool,
    enabled: bool,
) -> bool {
    // TODO: Create a texture for an invalid thumbnail or missing texture?
    ui.add_enabled(enabled, Label::new(param_label(param.param_id)));
    // Texture parameters don't include the file extension since it's implied.
    // Texture names aren't case sensitive.
    // TODO: Avoid allocating here.
    // TODO: Don't store the extension with the thumbnail at all?
    // TODO: Should this functionality be part of ssbh_wgpu?
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

    let mut changed = false;

    if advanced_mode {
        // Let users enter names manually if texture files aren't present.
        changed |= ui
            .add_enabled(enabled, TextEdit::singleline(&mut param.data))
            .changed();
    } else {
        ui.add_enabled_ui(enabled, |ui| {
            ComboBox::from_id_source(param.param_id.to_string())
                .selected_text(&param.data)
                .width(300.0)
                .show_ui(ui, |ui| {
                    // Texture files should be present in the folder, which allows for image previews.
                    // TODO: Is it safe to assume the thumbnails have all the available textures?
                    for (name, thumbnail) in
                        texture_thumbnails.iter().chain(default_thumbnails.iter())
                    {
                        // Material parameters don't include the .nutexb extension.
                        let text = Path::new(name)
                            .with_extension("")
                            .to_string_lossy()
                            .to_string();

                        // TODO: Show a texture as selected even if the case doesn't match?
                        ui.horizontal(|ui| {
                            ui.image(*thumbnail, egui::Vec2::new(24.0, 24.0));
                            changed |= ui
                                .selectable_value(&mut param.data, text.clone(), text)
                                .changed();
                        });
                    }
                });
        });
    }

    changed
}

fn edit_sampler(ui: &mut Ui, param: &mut SamplerParam) -> bool {
    let mut changed = false;

    CollapsingHeader::new(param_label(param.param_id)).show(ui, |ui| {
        let id = egui::Id::new(param.param_id.to_string());

        // TODO: Show validation errors for wrap mode.
        Grid::new(id).show(ui, |ui| {
            changed |= enum_combo_box(
                ui,
                "Wrap S",
                id.with("wraps{:?}"),
                &mut param.data.wraps,
            );
            ui.end_row();

            changed |= enum_combo_box(
                ui,
                "Wrap T",
                id.with("wrapt{:?}"),
                &mut param.data.wrapt,
            );
            ui.end_row();

            changed |= enum_combo_box(
                ui,
                "Wrap R",
                id.with("wrapr{:?}"),
                &mut param.data.wrapr,
            );
            ui.end_row();

            changed |= enum_combo_box(
                ui,
                "Min Filter",
                id.with("minfilter{:?}"),
                &mut param.data.min_filter,
            );
            ui.end_row();

            changed |= enum_combo_box(
                ui,
                "Mag Filter",
                id.with("magfilter{:?}"),
                &mut param.data.mag_filter,
            );
            ui.end_row();

            // TODO: What color space to use?
            // TODO: Add tooltips to other labels?
            // TODO: Only show tooltips after a delay?
            ui.label("Border Color").on_hover_text(
                "The color when sampling UVs outside the range 0.0 to 1.0. Only affects ClampToBorder.",
            );
            changed |= edit_color4f_rgba(ui, &mut param.data.border_color);
            ui.end_row();

            ui.label("Lod Bias");
            changed |= ui.add(DragValue::new(&mut param.data.lod_bias).speed(0.1)).changed();
            ui.end_row();

            ui.label("Max Anisotropy");
            egui::ComboBox::from_id_source(id.with("anis{:?}"))
                .selected_text(
                    anisotropy_label(param
                        .data
                        .max_anisotropy)
                )
                .show_ui(ui, |ui| {
                    changed |= ui.selectable_value(&mut param.data.max_anisotropy, None, "None").changed();
                    ui.separator();

                    for variant in MaxAnisotropy::VARIANTS {
                        let value = Some(MaxAnisotropy::from_str(variant).unwrap());
                        changed |= ui.selectable_value(
                            &mut param.data.max_anisotropy,
                            value,
                            anisotropy_label(value),
                        ).changed();
                    }
                });
            ui.end_row();
        });
    });

    changed
}

fn anisotropy_label(v: Option<MaxAnisotropy>) -> &'static str {
    match v {
        Some(v) => match v {
            MaxAnisotropy::One => "1x",
            MaxAnisotropy::Two => "2x",
            MaxAnisotropy::Four => "4x",
            MaxAnisotropy::Eight => "8x",
            MaxAnisotropy::Sixteen => "16x",
        },
        None => "None",
    }
}

fn edit_vector(
    ui: &mut Ui,
    param: &mut Vector4Param,
    enabled: bool,
    program: Option<&ShaderProgram>,
) -> bool {
    let mut changed = false;

    // Disabling the entire row interferes with the grid columns.
    // Disable each item individually.
    ui.add_enabled_ui(enabled, |ui| {
        changed |= edit_vector4_rgba(ui, &mut param.data);
    });

    ui.add_enabled_ui(enabled, |ui| {
        ui.label(param_label(param.param_id));
    });

    // TODO: Is it less annoying to enable all parameters if the shader label is invalid?
    let channels = program
        .map(|p| p.accessed_channels(&param.param_id.to_string()))
        .unwrap_or_default();
    let labels = vector4_labels_short(param.param_id);
    let labels_long = vector4_labels_long(param.param_id);

    // Prevent editing components not accessed by shaders in game.
    let id = egui::Id::new(param.param_id.to_string());
    let edit_component = |ui: &mut Ui, changed: &mut bool, i, value| {
        ui.add_enabled_ui(enabled && channels[i], |ui| {
            ui.horizontal(|ui| {
                ui.label(labels[i]);
                *changed |= ui
                    .add(DragSlider::new(id.with(labels[i]), value).width(50.0))
                    .on_hover_text(labels_long[i])
                    .changed();
            });
        });
    };

    edit_component(ui, &mut changed, 0, &mut param.data.x);
    edit_component(ui, &mut changed, 1, &mut param.data.y);
    edit_component(ui, &mut changed, 2, &mut param.data.z);
    edit_component(ui, &mut changed, 3, &mut param.data.w);

    changed
}

fn edit_vector4_rgba(ui: &mut Ui, data: &mut Vector4) -> bool {
    // TODO: Edit alpha for params with alpha?
    let mut color = [
        (255.0 * data.x) as u8,
        (255.0 * data.y) as u8,
        (255.0 * data.z) as u8,
    ];
    if ui.color_edit_button_srgb(&mut color).changed() {
        data.x = (color[0] as f32) / 255.0;
        data.y = (color[1] as f32) / 255.0;
        data.z = (color[2] as f32) / 255.0;
        true
    } else {
        false
    }
}

fn edit_color4f_rgba(ui: &mut Ui, data: &mut Color4f) -> bool {
    // TODO: Still show the color if the alpha is 0?
    let mut color = [
        (255.0 * data.r) as u8,
        (255.0 * data.g) as u8,
        (255.0 * data.b) as u8,
        (255.0 * data.a) as u8,
    ];
    if ui
        .color_edit_button_srgba_unmultiplied(&mut color)
        .changed()
    {
        data.r = (color[0] as f32) / 255.0;
        data.g = (color[1] as f32) / 255.0;
        data.b = (color[2] as f32) / 255.0;
        data.a = (color[3] as f32) / 255.0;
        true
    } else {
        false
    }
}

fn param_label(p: ParamId) -> String {
    let description = param_description(p);
    if !description.is_empty() {
        format!("{} ({})", p, description)
    } else {
        p.to_string()
    }
}
