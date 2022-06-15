use crate::{
    app::{error_icon, UiState},
    material::{
        add_parameters, apply_preset, default_material, missing_parameters, remove_parameters,
        unused_parameters,
    },
    widgets::*,
};
use egui::ScrollArea;
use log::error;
use rfd::FileDialog;
use ssbh_data::{
    matl_data::MatlEntryData, mesh_data::MeshObjectData, modl_data::ModlEntryData, prelude::*,
};
use ssbh_wgpu::ShaderDatabase;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn matl_editor(
    ctx: &egui::Context,
    title: &str,
    ui_state: &mut UiState,
    matl: &mut MatlData,
    modl: Option<&mut ModlData>,
    mesh: Option<&MeshData>,
    folder_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    shader_database: &ShaderDatabase,
    material_presets: &[MatlEntryData],
) -> bool {
    let mut open = true;

    egui::Window::new(format!("Matl Editor ({title})"))
        .open(&mut open)
        .default_size(egui::Vec2::new(400.0, 700.0))
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Matl", &["numatb"])
                            .save_file()
                        {
                            if let Err(e) = matl.write_to_file(file) {
                                error!(target: "ssbh_editor", "Failed to save Matl (.numatb): {}", e);
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

            // TODO: Simplify logic for closing window.
            let mut open = ui_state.preset_window_open;
            egui::Window::new("Select Material Preset")
                .open(&mut ui_state.preset_window_open)
                .show(ctx, |ui| {
                    for (i, preset) in material_presets.iter().enumerate() {
                        ui.selectable_value(&mut ui_state.selected_material_preset_index, i, &preset.material_label);
                    }

                    if ui.button("Apply").clicked() {
                        if let Some(preset) = material_presets.get(ui_state.selected_material_preset_index) {
                            if let Some(entry) = matl.entries.get_mut(ui_state.selected_material_index) {
                                *entry = apply_preset(entry, preset);
                            }
                        }

                        open = false;
                    }
                });
            if !open {
                ui_state.preset_window_open = false;
            }

            ui.add(egui::Separator::default().horizontal());

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Only display a single material at a time.
                    // This avoids cluttering the UI.
                    // TODO: Add the ability to rename materials.
                    ui.horizontal(|ui| {
                        ui.label("Material");
                        egui::ComboBox::from_id_source("MatlEditorMaterialLabel")
                            .width(400.0)
                            .show_index(ui, &mut ui_state.selected_material_index, matl.entries.len(), |i| {
                                matl.entries
                                    .get(i)
                                    .map(|m| m.material_label.clone())
                                    .unwrap_or_default()
                            });

                        if ui_state.matl_editor_advanced_mode && ui.button("Delete").clicked() {
                            // TODO: Potential panic?
                            matl.entries.remove(ui_state.selected_material_index);
                        }
                    });
                    // Advanced mode has more detailed information that most users won't want to edit.
                    ui.checkbox(&mut ui_state.matl_editor_advanced_mode, "Advanced Settings");

                    if let Some(entry) = matl.entries.get_mut(ui_state.selected_material_index) {
                        // TODO: Avoid collect here?
                        // Keep track of modl entries since materials may be renamed.
                        let mut modl_entries: Vec<_> = modl
                            .map(|m| {
                                m.entries
                                    .iter_mut()
                                    .filter(|e| e.material_label == entry.material_label)
                                    .collect()
                            })
                            .unwrap_or_default();

                        let mesh_objects: Vec<_> = mesh
                            .map(|mesh| {
                                mesh.objects
                                    .iter()
                                    .filter(|o| modl_entries.iter().any(|e| e.mesh_object_name == o.name && e.mesh_object_sub_index == o.sub_index)).collect()
                            })
                            .unwrap_or_default();

                        matl_entry_editor(
                            ui,
                            entry,
                            &mut modl_entries,
                            &mesh_objects,
                            folder_thumbnails,
                            default_thumbnails,
                            ui_state.matl_editor_advanced_mode,
                            shader_database,
                        );
                    }
                });
        });

    open
}

fn shader_label(ui: &mut egui::Ui, shader_label: &str, is_valid: bool) {
    if is_valid {
        ui.label(shader_label);
    } else {
        ui.horizontal(|ui| {
            // TODO: Add a black/red checkerboard for clarity.
            error_icon(ui);
            ui.label(egui::RichText::new(shader_label).color(egui::Color32::RED));
        })
        .response
        .on_hover_text(format!("{} is not a valid shader label.", shader_label));
    }
}

fn matl_entry_editor(
    ui: &mut egui::Ui,
    entry: &mut ssbh_data::matl_data::MatlEntryData,
    modl_entries: &mut [&mut ModlEntryData],
    mesh_objects: &[&MeshObjectData],
    texture_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    advanced_mode: bool,
    shader_database: &ShaderDatabase,
) {
    let program = shader_database.get(entry.shader_label.get(..24).unwrap_or(""));

    if advanced_mode {
        ui.label("Shader Label");
        ui.indent("shader indent", |ui| {
            shader_label(ui, &entry.shader_label, program.is_some());
            egui::Grid::new("shader_grid").show(ui, |ui| {
                // TODO: Should this be part of the basic mode.
                // TODO: Should advanced mode just be a textbox?
                ui.label("Render Pass");
                let shader = entry.shader_label.get(..24).unwrap_or("").to_string();
                egui::ComboBox::from_id_source("render pass")
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

                // TODO: Get info from shader database.
                if let Some(program) = program {
                    ui.label("Alpha Testing");
                    ui.label(program.discard.to_string());
                    ui.end_row();

                    ui.label("Vertex Attributes");
                    ui.add(egui::Label::new(program.vertex_attributes.join(",")).wrap(true));
                    ui.end_row();
                }
            });
        });
    } else {
        ui.horizontal(|ui| {
            ui.label("Shader Label");
            shader_label(ui, &entry.shader_label, program.is_some());
        });
    }

    // TODO: Show a black/yellow checkerboard for clarity.
    // TODO: Show errors in the material selector.
    // TODO: Show meshes with missing attributes.
    // TODO: Add a button to open the mesh editor.
    if let Some(program) = program {
        // TODO: Only show this if there are meshes with missing attributes.
        ui.label("Missing required attributes");
        for mesh in mesh_objects {
            // TODO: Avoid allocating here.
            let attribute_names: Vec<_> = mesh
                .texture_coordinates
                .iter()
                .map(|a| a.name.to_string())
                .chain(mesh.color_sets.iter().map(|a| a.name.to_string()))
                .collect();

            let missing_attributes = program.missing_required_attributes(&attribute_names);
            if !missing_attributes.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(&mesh.name);
                    ui.label(missing_attributes.join(","));
                });
            }
        }
    }

    ui.horizontal(|ui| {
        ui.label("Material Label");
        // TODO: Get this to work with lost_focus for efficiency.
        if ui.text_edit_singleline(&mut entry.material_label).changed() {
            // Rename any effected modl entries if the material label changes.
            for modl_entry in modl_entries {
                modl_entry.material_label = entry.material_label.clone();
            }
        }
    });

    // TODO: Option to delete unneeded parameters.
    if let Some(program) = program {
        let missing_parameters = missing_parameters(entry, program);
        if !missing_parameters.is_empty() && ui.button("Add Missing Parameters").clicked() {
            add_parameters(entry, &missing_parameters);
        }

        let unused_parameters = unused_parameters(entry, program);
        if !unused_parameters.is_empty() && ui.button("Remove Unused Parameters").clicked() {
            remove_parameters(entry, &unused_parameters);
        }
    }

    for param in entry.booleans.iter_mut() {
        ui.checkbox(&mut param.data, param.param_id.to_string());
    }

    for param in entry.floats.iter_mut() {
        ui.horizontal(|ui| {
            // TODO: Store this size somewhere to ensure labels align?
            ui.add_sized([80.0, 20.0], egui::Label::new(param.param_id.to_string()));
            ui.add(egui::Slider::new(&mut param.data, 0.0..=1.0));
        });
    }

    if advanced_mode {
        for param in entry.vectors.iter_mut() {
            // TODO: Make a custom expander that expands to sliders?
            // TODO: Set custom labels and ranges.
            // TODO: Add parameter descriptions.
            ui.label(param.param_id.to_string());

            ui.indent("indent", |ui| {
                egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
                    ui.label("X");
                    ui.add(egui::Slider::new(&mut param.data.x, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();

                    ui.label("Y");
                    ui.add(egui::Slider::new(&mut param.data.y, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();

                    ui.label("Z");
                    ui.add(egui::Slider::new(&mut param.data.z, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();

                    ui.label("W");
                    ui.add(egui::Slider::new(&mut param.data.w, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();
                });
            });
        }
    } else {
        egui::Grid::new("vectors").show(ui, |ui| {
            for param in entry.vectors.iter_mut() {
                // TODO: Store this size somewhere to ensure labels align?
                ui.add_sized([80.0, 20.0], egui::Label::new(param.param_id.to_string()));

                let mut color = [param.data.x, param.data.y, param.data.z];
                if ui.color_edit_button_rgb(&mut color).changed() {
                    param.data.x = color[0];
                    param.data.y = color[1];
                    param.data.z = color[2];
                }
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut param.data.x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut param.data.y));
                    ui.label("Z");
                    ui.add(egui::DragValue::new(&mut param.data.z));
                    ui.label("W");
                    ui.add(egui::DragValue::new(&mut param.data.w));
                });

                ui.end_row();
            }
        });
    }

    // The defaults for samplers are usually fine, so don't show samplers by default.
    if advanced_mode {
        for param in &mut entry.samplers {
            ui.label(param.param_id.to_string());
            ui.indent("indent", |ui| {
                egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
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
    }

    for param in &mut entry.textures {
        // TODO: Should this check be case sensitive?
        // TODO: Create a texture for an invalid thumbnail or missing texture?
        // TODO: Should this functionality be part of ssbh_wgpu?
        ui.horizontal(|ui| {
            ui.add_sized([80.0, 20.0], egui::Label::new(param.param_id.to_string()));

            // TODO: How to handle #replace_cubemap?
            // Texture parameters don't include the file extension since it's implied.
            if let Some(thumbnail) = texture_thumbnails
                .iter()
                .chain(default_thumbnails.iter())
                .find(|t| Path::new(&t.0).with_extension("") == Path::new(&param.data))
                .map(|t| t.1)
            {
                ui.image(thumbnail, egui::Vec2::new(24.0, 24.0));
            }

            if advanced_mode {
                // Let users enter names manually if texture files aren't present.
                ui.text_edit_singleline(&mut param.data);
            } else {
                // Texture files should be present in the folder, which allows for image previews.
                egui::ComboBox::from_id_source(param.param_id.to_string())
                    .selected_text(&param.data)
                    .show_ui(ui, |ui| {
                        // TODO: Is it safe to assume the thumbnails have all the available textures?
                        for (name, thumbnail) in
                            texture_thumbnails.iter().chain(default_thumbnails.iter())
                        {
                            // Material parameters don't include the .nutexb extension.
                            let text = Path::new(name)
                                .with_extension("")
                                .to_string_lossy()
                                .to_string();

                            ui.horizontal(|ui| {
                                ui.image(*thumbnail, egui::Vec2::new(24.0, 24.0));
                                ui.selectable_value(&mut param.data, text.to_string(), text);
                            });
                        }
                    });
            }
        });
    }

    // TODO: Reflecting changes to these values in the viewport requires recreating pipelines.
    // Most users will want to leave the rasterizer state at its default values.
    if advanced_mode {
        for param in &mut entry.rasterizer_states {
            // TODO: These param IDs might not be unique?
            ui.label(param.param_id.to_string());

            ui.indent("todo1", |ui| {
                egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
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
    }

    for param in &mut entry.blend_states {
        ui.label(param.param_id.to_string());
        ui.indent("blend state", |ui| {
            egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
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
}
