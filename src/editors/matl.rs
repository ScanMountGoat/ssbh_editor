use crate::{
    app::{
        display_validation_errors, draggable_icon, warning_icon, warning_icon_text,
        MatlEditorState, PresetMode, UiState,
    },
    horizontal_separator_empty,
    material::*,
    path::{folder_editor_title, presets_file},
    presets::{load_json_presets, load_xml_presets},
    save_file, save_file_as,
    validation::{MatlValidationError, MatlValidationErrorKind},
    widgets::*,
    EditorResponse, TextureDimension, Thumbnail,
};
use egui::{
    load::SizedTexture, special_emojis::GITHUB, Button, CentralPanel, CollapsingHeader, ComboBox,
    Context, DragValue, Grid, Label, RichText, ScrollArea, SidePanel, TextEdit, TopBottomPanel, Ui,
    Window,
};
use egui_dnd::dnd;
use log::error;
use rfd::FileDialog;
use ssbh_data::{matl_data::*, modl_data::ModlEntryData, prelude::*, Color4f, Vector4};
use ssbh_wgpu::{ShaderDatabase, ShaderProgram};
use std::path::Path;
use strum::IntoEnumIterator;

const UNUSED_PARAM: &str =
    "This parameter is not required by the shader and will be ignored in game.";

#[allow(clippy::too_many_arguments)]
pub fn matl_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    state: &mut MatlEditorState,
    matl: &mut MatlData,
    modl: Option<&mut ModlData>,
    validation_errors: &[MatlValidationError],
    folder_thumbnails: &[Thumbnail],
    default_thumbnails: &[Thumbnail],
    shader_database: &ShaderDatabase,
    material_presets: &mut Vec<MatlEntryData>,
    default_presets: &[MatlEntryData],
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
    dark_mode: bool,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    Window::new(format!("Matl Editor ({title})"))
        .open(&mut open)
        .default_size(egui::Vec2::new(700.0, 900.0))
        .resizable(true)
        .show(ctx, |ui| {
            TopBottomPanel::top("matl_top_panel").show_inside(ui, |ui| {
                let (menu_changed, menu_saved) = menu_bar(
                    ui,
                    matl,
                    &modl,
                    state,
                    material_presets,
                    folder_name,
                    file_name,
                );
                changed |= menu_changed;
                saved |= menu_saved;
            });

            SidePanel::left("matl_left_panel")
                .default_width(300.0)
                .show_inside(ui, |ui| {
                    changed |= select_material_dnd(
                        &mut matl.entries,
                        ui,
                        ctx,
                        dark_mode,
                        validation_errors,
                        state,
                    );
                });

            CentralPanel::default().show_inside(ui, |ui| {
                // TODO: Simplify logic for closing window.
                let entry = matl.entries.get_mut(state.selected_material_index);
                let (open, preset_changed) = preset_window(
                    state,
                    ctx,
                    material_presets,
                    default_presets,
                    entry,
                    shader_database,
                );
                if !open {
                    state.matl_preset_window_open = false;
                }
                changed |= preset_changed;

                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if let Some(entry) = matl.entries.get_mut(state.selected_material_index) {
                            changed |= edit_matl_entry(
                                ctx,
                                ui,
                                entry,
                                modl,
                                validation_errors,
                                folder_thumbnails,
                                default_thumbnails,
                                shader_database,
                                red_checkerboard,
                                yellow_checkerboard,
                                state,
                            );
                        }
                    });
            });
        });

    EditorResponse {
        open,
        changed,
        saved,
        message: None,
    }
}

fn select_material_dnd(
    entries: &mut Vec<MatlEntryData>,
    ui: &mut Ui,
    ctx: &Context,
    dark_mode: bool,
    validation_errors: &[MatlValidationError],
    state: &mut MatlEditorState,
) -> bool {
    let mut changed = false;

    // TODO: Avoid allocating here.
    let mut item_indices: Vec<_> = (0..entries.len()).collect();

    let mut index_to_delete = None;

    let response = dnd(ui, "matl_dnd").show_vec(&mut item_indices, |ui, item_index, handle, _| {
        ui.horizontal(|ui| {
            let entry = &entries[*item_index];

            handle.ui(ui, |ui| {
                draggable_icon(ctx, ui, dark_mode);
            });

            // TODO: Avoid collect.
            let errors: Vec<_> = validation_errors
                .iter()
                .filter(|e| e.entry_index == *item_index)
                .collect();
            let text = if !errors.is_empty() {
                warning_icon_text(&entry.material_label)
            } else {
                RichText::new(&entry.material_label)
            };

            // TODO: wrap text?
            let mut response =
                ui.selectable_label(state.selected_material_index == *item_index, text);

            if response.clicked() {
                state.selected_material_index = *item_index;
            }

            if !errors.is_empty() {
                response = response.on_hover_ui(|ui| display_validation_errors(ui, &errors));
            }

            // TODO: This needs to be cleared every frame.
            // TODO: Use a messages instead?
            if response.hovered() {
                // Used for material mask rendering.
                state.hovered_material_index = Some(*item_index);
            }

            response.context_menu(|ui| {
                // TODO: Also add a menu option?
                if ui.button("Delete").clicked() {
                    ui.close_menu();
                    index_to_delete = Some(*item_index);
                }
            });
        });
    });

    if let Some(response) = response.final_update() {
        egui_dnd::utils::shift_vec(response.from, response.to, entries);
        state.selected_material_index = item_indices
            .iter()
            .position(|i| *i == state.selected_material_index)
            .unwrap_or_default();
        changed = true;
    }

    if let Some(i) = index_to_delete {
        entries.remove(i);
        changed = true;
    }

    changed
}

// TODO: Validate presets?
pub fn preset_editor(
    ctx: &egui::Context,
    ui_state: &mut UiState,
    user_presets: &mut Vec<MatlEntryData>,
    default_thumbnails: &[Thumbnail],
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
    dark_mode: bool,
) {
    Window::new("Material Preset Editor")
        .open(&mut ui_state.preset_editor_open)
        .default_size(egui::Vec2::new(900.0, 900.0))
        .show(ctx, |ui| {
            TopBottomPanel::top("matl_presets_top_panel").show_inside(ui, |ui| {
                presets_menu(ui, user_presets);
            });

            SidePanel::left("matl_presets_left_panel")
                .default_width(300.0)
                .show_inside(ui, |ui| {
                    select_material_dnd(
                        user_presets,
                        ui,
                        ctx,
                        dark_mode,
                        &[],
                        &mut ui_state.preset_editor,
                    );
                });

            CentralPanel::default().show_inside(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // Use an empty model thumbnail list to encourage using default textures.
                        // These textures will be replaced by param specific defaults anyway.
                        if let Some(entry) =
                            user_presets.get_mut(ui_state.preset_editor.selected_material_index)
                        {
                            edit_matl_entry(
                                ctx,
                                ui,
                                entry,
                                None,
                                &[],
                                &[],
                                default_thumbnails,
                                shader_database,
                                red_checkerboard,
                                yellow_checkerboard,
                                &mut ui_state.preset_editor,
                            );
                        }
                    });
            });
        });
}

fn presets_menu(ui: &mut Ui, user_presets: &mut Vec<MatlEntryData>) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Save").clicked() {
                ui.close_menu();

                let path = presets_file();
                save_material_presets(user_presets, path);
            }
        });

        ui.menu_button("Import", |ui| {
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
                    load_presets_from_file(user_presets, file, load_json_presets);
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
                    load_presets_from_file(user_presets, file, load_xml_presets);
                }
            }
        });

        ui.menu_button("Material", |ui| {
            if ui.button("Add New Material").clicked() {
                ui.close_menu();

                let new_entry = default_material();
                user_presets.push(new_entry);
            }

            if ui.button("Remove Duplicates").clicked() {
                ui.close_menu();

                remove_duplicates(user_presets);
            }
        });
        help_menu(ui);
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

fn remove_unused_materials(matl_entries: &mut Vec<MatlEntryData>, modl_entries: &[ModlEntryData]) {
    let mut visited = Vec::new();

    for modl_entry in modl_entries.iter() {
        visited.push(modl_entry.material_label.clone());
    }

    if visited.contains(&String::from("EyeL")) {
        //World of Light purple
        visited.push(String::from("EyeLD"));
        //Final Smash
        visited.push(String::from("EyeLG"));
        //World of Light red
        visited.push(String::from("EyeLL"));
    }

    if visited.contains(&String::from("EyeR")) {
        //World of Light purple
        visited.push(String::from("EyeRD"));
        //Final Smash
        visited.push(String::from("EyeRG"));
        //World of Light red
        visited.push(String::from("EyeRL"));
    }

    if visited.contains(&String::from("EyeL1")) {
        //World of Light purple
        visited.push(String::from("EyeLD1"));
        //Final Smash
        visited.push(String::from("EyeLG1"));
        //World of Light red
        visited.push(String::from("EyeLL1"));
    }

    if visited.contains(&String::from("EyeR1")) {
        //World of Light purple
        visited.push(String::from("EyeRD1"));
        //Final Smash
        visited.push(String::from("EyeRG1"));
        //World of Light red
        visited.push(String::from("EyeRL1"));
    }

    if visited.contains(&String::from("EyeL2")) {
        //World of Light purple
        visited.push(String::from("EyeLD2"));
        //Final Smash
        visited.push(String::from("EyeLG2"));
        //World of Light red
        visited.push(String::from("EyeLL2"));
    }

    if visited.contains(&String::from("EyeR2")) {
        //World of Light purple
        visited.push(String::from("EyeRD2"));
        //Final Smash
        visited.push(String::from("EyeRG2"));
        //World of Light red
        visited.push(String::from("EyeRL2"));
    }

    matl_entries.retain(|item| visited.contains(&item.material_label));
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

fn edit_matl_entry(
    ctx: &Context,
    ui: &mut Ui,
    entry: &mut MatlEntryData,
    modl: Option<&mut ModlData>,
    validation_errors: &[MatlValidationError],
    folder_thumbnails: &[Thumbnail],
    default_thumbnails: &[Thumbnail],
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
    state: &mut MatlEditorState,
) -> bool {
    let mut changed = false;
    // Advanced mode has more detailed information that most users won't want to edit.
    ui.checkbox(&mut state.advanced_mode, "Advanced Settings");
    horizontal_separator_empty(ui);

    // TODO: Avoid allocating here.
    let entry_validation_errors: Vec<_> = validation_errors
        .iter()
        .filter(|e| e.entry_index == state.selected_material_index)
        .collect();

    ui.horizontal(|ui| {
        ui.label("Material Label");
        let mut modl_entries: Vec<_> = modl
            .map(|modl| {
                modl.entries
                    .iter_mut()
                    .filter(|e| e.material_label == entry.material_label)
                    .collect()
            })
            .unwrap_or_default();

        changed |= edit_material_label(entry, ui, &mut modl_entries);
    });

    changed |= edit_matl_entry_inner(
        ctx,
        ui,
        entry,
        &entry_validation_errors,
        folder_thumbnails,
        default_thumbnails,
        state.advanced_mode,
        shader_database,
        red_checkerboard,
        yellow_checkerboard,
    );

    changed
}

fn preset_window(
    state: &mut MatlEditorState,
    ctx: &egui::Context,
    material_presets: &[MatlEntryData],
    default_presets: &[MatlEntryData],
    entry: Option<&mut MatlEntryData>,
    shader_database: &ShaderDatabase,
) -> (bool, bool) {
    let mut open = state.matl_preset_window_open;
    let mut changed = false;
    Window::new("Select Material Preset")
        .open(&mut state.matl_preset_window_open)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.preset_mode,
                    PresetMode::Default,
                    RichText::new("Default").heading(),
                );
                ui.selectable_value(
                    &mut state.preset_mode,
                    PresetMode::User,
                    RichText::new("User").heading(),
                );
            });

            ui.weak("Hover over a preset to see shader info.");
            horizontal_separator_empty(ui);

            match state.preset_mode {
                PresetMode::User => {
                    if material_presets.is_empty() {
                        ui.label("No user material presets detected. Make sure the presets.json file is present and contains valid JSON materials.");
                    } else {
                        list_presets(ui, material_presets, &mut state.selected_preset_index, shader_database);
                    }
                }
                PresetMode::Default => list_presets(ui, default_presets, &mut state.selected_preset_index, shader_database),
            }

            if ui.button("Apply").clicked() {
                let presets = match state.preset_mode {
                    PresetMode::User => material_presets,
                    PresetMode::Default => default_presets,
                };

                if let Some(preset) = presets.get(state.selected_preset_index)
                {
                    if let Some(entry) = entry {
                        *entry = apply_preset(entry, preset);
                        changed = true;
                    }
                }
                open = false;
            }
        });

    (open, changed)
}

fn list_presets(
    ui: &mut Ui,
    material_presets: &[MatlEntryData],
    selected_index: &mut usize,
    shader_database: &ShaderDatabase,
) {
    for (i, preset) in material_presets.iter().enumerate() {
        let response = ui.selectable_value(selected_index, i, &preset.material_label);
        if let Some(program) = shader_database.get(&preset.shader_label) {
            let tooltip = program_attributes(program);
            if !tooltip.is_empty() {
                response.on_hover_text(tooltip);
            }
        }
    }
}

fn program_attributes(program: &ShaderProgram) -> String {
    let mut attributes = Vec::new();
    if program.discard {
        attributes.push("Alpha Testing");
    }
    if program.premultiplied {
        attributes.push("Premultiplied Alpha");
    }
    if program.receives_shadow {
        attributes.push("Receives Shadow");
    }
    if program.sh {
        attributes.push("SH Lighting");
    }
    if program.lighting {
        attributes.push("Lightset Lighting");
    }
    attributes.join(", ")
}

fn menu_bar(
    ui: &mut Ui,
    matl: &mut MatlData,
    modl: &Option<&mut ModlData>,
    state: &mut MatlEditorState,
    material_presets: &mut Vec<MatlEntryData>,
    folder_name: &Path,
    file_name: &str,
) -> (bool, bool) {
    let mut changed = false;
    let mut saved = false;

    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Save").clicked() {
                ui.close_menu();
                saved |= save_file(matl, folder_name, file_name);
            }

            if ui.button("Save As...").clicked() {
                ui.close_menu();
                saved |= save_file_as(matl, folder_name, file_name, "Matl", "numatb");
            }
        });

        ui.menu_button("Material", |ui| {
            let button = |ui: &mut Ui, text| ui.add(Button::new(text).wrap(false));
            if ui.button("Add New Material").clicked() {
                ui.close_menu();

                // TODO: Select options from presets?
                let new_entry = default_material();
                matl.entries.push(new_entry);

                state.selected_material_index = matl.entries.len() - 1;

                changed = true;
            }

            if button(ui, "Duplicate Current Material").clicked() {
                ui.close_menu();

                if let Some(old_entry) = matl.entries.get(state.selected_material_index) {
                    let mut new_entry = old_entry.clone();
                    new_entry.material_label.push_str("_copy");
                    matl.entries.push(new_entry);
                }

                state.selected_material_index = matl.entries.len() - 1;

                changed = true;
            }
            ui.separator();

            if ui.button("Add Material to Presets").clicked() {
                ui.close_menu();

                // TODO: Prompt for naming the preset?
                if let Some(entry) = matl.entries.get(state.selected_material_index) {
                    material_presets.push(entry.clone());
                }
            }

            if ui.button("Apply Preset").clicked() {
                ui.close_menu();

                state.matl_preset_window_open = true;
                changed = true;
            }
            ui.separator();

            if ui.button("Remove Duplicates").clicked() {
                ui.close_menu();

                remove_duplicates(&mut matl.entries);
                changed = true;
            }

            if ui
                .add_enabled(modl.is_some(), Button::new("Remove Unused Materials"))
                .clicked()
            {
                ui.close_menu();

                if let Some(modl) = modl {
                    remove_unused_materials(&mut matl.entries, &modl.entries);
                    changed = true;
                }
            }
        });

        ui.menu_button("Reorder", |ui| {
            if ui.button("Move Material to Top").clicked() {
                ui.close_menu();
                matl.entries.swap(0, state.selected_material_index);
                state.selected_material_index = 0;
                changed = true;
            }

            if ui.button("Move Material to Bottom").clicked() {
                ui.close_menu();
                let last_index = matl.entries.len() - 1;
                matl.entries.swap(state.selected_material_index, last_index);
                state.selected_material_index = last_index;
                changed = true;
            }

            ui.separator();

            if ui.button("Sort Materials").clicked() {
                ui.close_menu();
                let selected_material = matl.entries[state.selected_material_index].clone();
                matl.entries
                    .sort_by_key(|k| k.material_label.to_lowercase());
                state.selected_material_index = matl
                    .entries
                    .iter()
                    .position(|e| e == &selected_material)
                    .unwrap_or_default();
                changed = true;
            }
        });

        help_menu(ui);
    });

    (changed, saved)
}

fn help_menu(ui: &mut Ui) {
    ui.menu_button("Help", |ui| {
        let button = |ui: &mut Ui, text: &str| ui.add(Button::new(text).wrap(false));

        if button(ui, &format!("{GITHUB} Material Parameter Reference")).clicked() {
            ui.close_menu();
            let link = "https://github.com/ScanMountGoat/Smush-Material-Research/blob/master/Material%20Parameters.md";
            if let Err(open_err) = open::that(link) {
                log::error!("Failed to open link ({link}). {open_err}");
            }
        }

        if button(ui, "Material Research Website").clicked() {
            ui.close_menu();
            let link = "https://scanmountgoat.github.io/Smush-Material-Research/";
            if let Err(open_err) = open::that(link) {
                log::error!("Failed to open link ({link}). {open_err}");
            }
        }

        if ui.button(format!("{GITHUB} Matl Editor Wiki")).clicked() {
            ui.close_menu();

            let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Matl-Editor";
            if let Err(e) = open::that(link) {
                log::error!("Failed to open {link}: {e}");
            }
        }
    });
}

fn edit_material_label(
    entry: &mut MatlEntryData,
    ui: &mut Ui,
    modl_entries: &mut [&mut ModlEntryData],
) -> bool {
    // TODO: Get this to work with lost_focus for efficiency.
    // TODO: Show errors if these checks fail?
    let response = ui.add_sized(
        egui::Vec2::new(400.0, 20.0),
        egui::TextEdit::singleline(&mut entry.material_label),
    );

    let changed = response.changed();
    if changed {
        // Rename any effected modl entries if the material label changes.
        // Keep track of modl entries since materials may be renamed.
        for modl_entry in modl_entries {
            modl_entry.material_label = entry.material_label.clone();
        }
    }

    changed
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
            ui.image(SizedTexture { id: red_checkerboard, size: egui::Vec2::new(16.0, 16.0) });
            changed |= ui.add(egui::TextEdit::singleline(shader_label).text_color(egui::Color32::RED)).changed();
        })
        .response
        .on_hover_text(format!("{shader_label} is not a valid shader label. Copy an existing shader label or apply a material preset."));
    }

    changed
}

fn edit_matl_entry_inner(
    _ctx: &Context,
    ui: &mut Ui,
    entry: &mut ssbh_data::matl_data::MatlEntryData,
    validation_errors: &[&MatlValidationError],
    texture_thumbnails: &[Thumbnail],
    default_thumbnails: &[Thumbnail],
    advanced_mode: bool,
    shader_database: &ShaderDatabase,
    red_checkerboard: egui::TextureId,
    yellow_checkerboard: egui::TextureId,
) -> bool {
    let mut changed = false;

    let program = shader_database.get(&entry.shader_label);

    ui.heading("Shader");
    changed |= edit_shader(ui, entry, program, red_checkerboard);
    horizontal_separator_empty(ui);

    // TODO: Add a button to open the mesh editor?
    if validation_errors.iter().any(|e| {
        matches!(
            e.kind,
            MatlValidationErrorKind::MissingRequiredVertexAttributes { .. }
        )
    }) {
        ui.horizontal(|ui| {
            ui.image(SizedTexture {
                id: yellow_checkerboard,
                size: egui::Vec2::new(16.0, 16.0),
            });
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
                if let MatlValidationErrorKind::MissingRequiredVertexAttributes {
                    mesh_name,
                    missing_attributes,
                    ..
                } = &error.kind
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
            format!("Add {} Missing Parameters", missing_parameters.len())
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
            format!("Remove {} Unused Parameters", unused_parameters.len())
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
                .on_disabled_hover_text(UNUSED_PARAM)
                .changed();
        });
    }
    horizontal_separator_empty(ui);

    for param in entry.floats.iter_mut() {
        let id = egui::Id::new(param.param_id.to_string());
        ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
            ui.horizontal(|ui| {
                ui.label(param_label(param.param_id))
                    .on_disabled_hover_text(UNUSED_PARAM);
                changed |= ui.add(DragSlider::new(id, &mut param.data)).changed();
            })
        });
    }
    horizontal_separator_empty(ui);

    for param in entry.vectors.iter_mut() {
        changed |= edit_vector(
            ui,
            param,
            !unused_parameters.contains(&param.param_id),
            program,
        );
    }
    horizontal_separator_empty(ui);

    Grid::new("matl textures").show(ui, |ui| {
        for param in &mut entry.textures {
            // TODO: Avoid collect.
            let errors: Vec<_> = validation_errors
                .iter()
                .filter(|e| match e.kind {
                    MatlValidationErrorKind::UnexpectedTextureFormat { param_id, .. } => {
                        param_id == param.param_id
                    }
                    MatlValidationErrorKind::UnexpectedTextureDimension { param_id, .. } => {
                        param_id == param.param_id
                    }
                    _ => false,
                })
                .collect();

            changed |= edit_texture(
                ui,
                param,
                texture_thumbnails,
                default_thumbnails,
                advanced_mode,
                !unused_parameters.contains(&param.param_id),
                &errors,
            );
            ui.end_row();
        }
    });
    horizontal_separator_empty(ui);

    for param in &mut entry.samplers {
        // TODO: Avoid collect.
        let errors: Vec<_> = validation_errors
            .iter()
            .filter(|e| {
                matches!(&e.kind, MatlValidationErrorKind::WrapModeClampsUvs { samplers, .. }
                if samplers.contains(&param.param_id)) 
                || matches!(&e.kind, MatlValidationErrorKind::SamplerAnisotropyNonLinearFilterMode { param_id, ..} if *param_id == param.param_id)
            })
            .collect();

        ui.add_enabled_ui(!unused_parameters.contains(&param.param_id), |ui| {
            changed |= edit_sampler(ui, param, &errors);
        });
    }
    horizontal_separator_empty(ui);

    // TODO: Reflecting changes to these values in the viewport requires recreating pipelines.
    for param in &mut entry.rasterizer_states {
        changed |= edit_rasterizer(ui, param);
    }
    horizontal_separator_empty(ui);

    for param in &mut entry.blend_states {
        // TODO: Avoid collect.
        // TODO: Also check that the ParamId matches?
        let errors: Vec<_> = validation_errors
            .iter()
            .filter(|e| {
                matches!(
                    &e.kind,
                    MatlValidationErrorKind::PremultipliedShaderSrcAlpha { .. }
                )
            })
            .collect();

        changed |= edit_blend(ui, param, &errors);
    }

    changed
}

fn edit_shader(
    ui: &mut Ui,
    entry: &mut MatlEntryData,
    program: Option<&ShaderProgram>,
    red_checkerboard: egui::TextureId,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Shader Label");
        changed |= edit_shader_label(
            ui,
            &mut entry.shader_label,
            program.is_some(),
            red_checkerboard,
        );
    });

    shader_grid(ui, entry, &mut changed, program);

    if let Some(program) = program {
        horizontal_separator_empty(ui);
        shader_info_grid(ui, program);
    }

    changed
}

fn shader_grid(
    ui: &mut Ui,
    entry: &mut MatlEntryData,
    changed: &mut bool,
    program: Option<&ShaderProgram>,
) {
    Grid::new("shader_grid").show(ui, |ui| {
        // TODO: Should this be part of the basic mode.
        ui.label("Render Pass");
        let shader = entry.shader_label.get(..24).unwrap_or("").to_string();
        ComboBox::from_id_source("render pass")
            .selected_text(entry.shader_label.get(25..).unwrap_or(""))
            .show_ui(ui, |ui| {
                for pass in [
                    "_opaque",
                    "_far",
                    "_sort",
                    "_near",
                ] {
                    *changed |= ui
                        .selectable_value(
                            &mut entry.shader_label,
                            shader.clone() + pass,
                            pass,
                        )
                        .changed();
                }
            });
        ui.end_row();

        if let Some(program) = program {
            ui.label("Vertex Attributes")
                .on_hover_text("The mesh attributes required by the shader. The XYZW suffixes indicate accessed components.");
            ui.add(Label::new(program.vertex_attributes.join(", ")));
            ui.end_row();
        }
    });
}

fn shader_info_grid(ui: &mut Ui, program: &ShaderProgram) {
    Grid::new("shader_info").show(ui, |ui| {
        shader_info_header(ui);
        ui.end_row();

        shader_info_values(ui, program);
        ui.end_row();
    });
}

fn shader_info_values(ui: &mut Ui, program: &ShaderProgram) {
    // TODO: use checkmarks so this is easier to parse at a glance.
    ui.label(program.discard.to_string());
    ui.label(program.premultiplied.to_string());
    ui.label(program.receives_shadow.to_string());
    ui.label(program.sh.to_string());
    ui.label(program.lighting.to_string());
}

fn shader_info_header(ui: &mut Ui) {
    ui.label("Alpha Testing")
        .on_hover_text("A transparent cutout effect using an alpha threshold of 0.5.");
    ui.label("Premultiplied Alpha").on_hover_text(
        "Multiplies the RGB color by alpha similar to a BlendState0 Source Color of SrcAlpha.",
    );
    ui.label("Receives Shadow")
        .on_hover_text("Receives directional shadows. Not to be confused with shadow casting.");
    ui.label("SH Lighting")
        .on_hover_text("Uses spherical harmonic diffuse ambient lighting from shcpanim files.");
    ui.label("Lightset Lighting")
        .on_hover_text("Uses directional lighting from the lighting nuanmb.");
}

fn edit_blend(ui: &mut Ui, param: &mut BlendStateParam, errors: &[&&MatlValidationError]) -> bool {
    let mut changed = false;

    let text = param_text(param.param_id, errors);
    let response = CollapsingHeader::new(text)
        .default_open(true)
        .show(ui, |ui| {
            let id = egui::Id::new(param.param_id.to_string());

            Grid::new(id).show(ui, |ui| {
                ui.label("Source Color")
                    .on_hover_text("The blend factor for this mesh's rendered color.");
                changed |= enum_combo_box(ui, id.with("srccolor"), &mut param.data.source_color);
                ui.end_row();

                ui.label("Destination Color").on_hover_text(
                    "The blend factor for the previously rendered background color.",
                );
                changed |=
                    enum_combo_box(ui, id.with("dstcolor"), &mut param.data.destination_color);
                ui.end_row();

                changed |= ui
                    .checkbox(
                        &mut param.data.alpha_sample_to_coverage,
                        "Alpha Sample to Coverage",
                    )
                    .on_hover_text("Simulates transparency using a dithering pattern when enabled.")
                    .changed();
                ui.end_row();
                // TODO: Basic blend state can just expose a selection for "additive", "alpha", or "opaque".
                // TODO: Research in game examples for these presets (premultiplied alpha?)
            });
        })
        .header_response;

    if !errors.is_empty() {
        response.on_hover_ui(|ui| display_validation_errors(ui, errors));
    }

    changed
}

fn edit_rasterizer(ui: &mut Ui, param: &mut RasterizerStateParam) -> bool {
    let mut changed = false;

    CollapsingHeader::new(param_label(param.param_id)).show(ui, |ui| {
        let id = egui::Id::new(param.param_id.to_string());

        Grid::new(id).show(ui, |ui| {
            ui.label("Polygon Fill")
                .on_hover_text("The polygon mode for shading triangles.");
            changed |= enum_combo_box(ui, id.with("fill"), &mut param.data.fill_mode);
            ui.end_row();

            ui.label("Cull Mode")
                .on_hover_text("The side of each face to cull from rendering.");
            changed |= enum_combo_box(ui, id.with("cull"), &mut param.data.cull_mode);
            ui.end_row();

            ui.label("Depth Bias")
                .on_hover_text("The offset to the mesh's depth value for depth testing.");
            ui.add(DragValue::new(&mut param.data.depth_bias).speed(0.1));
            ui.end_row();
        });
    });

    changed
}

fn edit_texture(
    ui: &mut Ui,
    param: &mut TextureParam,
    texture_thumbnails: &[Thumbnail],
    default_thumbnails: &[Thumbnail],
    advanced_mode: bool,
    enabled: bool,
    errors: &[&&MatlValidationError],
) -> bool {
    // Show errors that apply to this param.
    let text = param_text(param.param_id, errors);
    let response = ui
        .add_enabled(enabled, Label::new(text))
        .on_disabled_hover_text(UNUSED_PARAM);

    if !errors.is_empty() {
        response.on_hover_ui(|ui| display_validation_errors(ui, errors));
    }

    if let Some(thumbnail) = texture_thumbnails
        .iter()
        .chain(default_thumbnails.iter())
        .find(|t| {
            // Texture parameters don't include the file extension since it's implied.
            // Texture names aren't case sensitive.
            // TODO: Don't store the extension with the thumbnail at all?
            // TODO: Should this functionality be part of ssbh_wgpu?
            Path::new(&t.0)
                .with_extension("")
                .as_os_str()
                .eq_ignore_ascii_case(&param.data)
        })
        .map(|t| t.1)
    {
        ui.image(SizedTexture {
            id: thumbnail,
            size: egui::Vec2::new(24.0, 24.0),
        });
    } else {
        // The missing texture validation error doesn't contain the ParamID.
        // Assume missing textures aren't present in the thumbnail cache.
        warning_icon(ui).on_hover_text(format!(
            "Texture {:?} is not a valid nutexb file or default texture name.",
            &param.data
        ));
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
                    // Assume every available texture correctly generated a thumbnail.
                    // Prevent assigning cube maps to 2D textures and 2D textures to cube maps.
                    let expected_dimension = TextureDimension::from_param(param.param_id);
                    for (name, thumbnail, _) in texture_thumbnails
                        .iter()
                        .chain(default_thumbnails.iter())
                        .filter(|(_, _, dimension)| *dimension == expected_dimension)
                    {
                        // Material parameters don't include the .nutexb extension.
                        let text = Path::new(name)
                            .with_extension("")
                            .to_string_lossy()
                            .to_string();

                        // TODO: Show a texture as selected even if the case doesn't match?
                        ui.horizontal(|ui| {
                            ui.image(SizedTexture {
                                id: *thumbnail,
                                size: egui::Vec2::new(24.0, 24.0),
                            });
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

fn edit_sampler(ui: &mut Ui, param: &mut SamplerParam, errors: &[&&MatlValidationError]) -> bool {
    let mut changed = false;

    let text = param_text(param.param_id, errors);
    let response = CollapsingHeader::new(text).id_source(param.param_id.to_string()).show(ui, |ui| {
        let id = egui::Id::new(param.param_id.to_string());

        // TODO: List which wrap modes are the problem on error?
        Grid::new(id).show(ui, |ui| {
            ui.label("Wrap S")
                .on_hover_text("The wrap mode for the S or U coordinate.");
            changed |= enum_combo_box(
                ui,
                id.with("wraps{:?}"),
                &mut param.data.wraps,
            );
            ui.end_row();

            ui.label("Wrap T")
                .on_hover_text("The wrap mode for the T or V coordinate.");
            changed |= enum_combo_box(
                ui,
                id.with("wrapt{:?}"),
                &mut param.data.wrapt,
            );
            ui.end_row();

            ui.label("Wrap R")
                .on_hover_text("The wrap mode for the R coordinate for cube maps.");
            changed |= enum_combo_box(
                ui,
                id.with("wrapr{:?}"),
                &mut param.data.wrapr,
            );
            ui.end_row();

            ui.label("Min Filter")
                .on_hover_text("The filter mode when minifying the texture. Affects mipmapping.");
            changed |= enum_combo_box(
                ui,
                id.with("minfilter{:?}"),
                &mut param.data.min_filter,
            );
            ui.end_row();

            ui.label("Mag Filter")
                .on_hover_text("The filter mode when magnifying the texture.");
            changed |= enum_combo_box(
                ui,
                id.with("magfilter{:?}"),
                &mut param.data.mag_filter,
            );
            ui.end_row();

            // TODO: What color space to use?
            // TODO: Add tooltips to other labels?
            ui.label("Border Color")
                .on_hover_text(
                "The color when sampling UVs outside the range 0.0 to 1.0. Only affects ClampToBorder."
            );
            changed |= edit_color4f_rgba(ui, &mut param.data.border_color);
            ui.end_row();

            ui.label("Lod Bias")
                .on_hover_text("The offset added to the mip level. Lower values use higher resolution mipmaps more often.");
            changed |= ui.add(DragValue::new(&mut param.data.lod_bias).speed(0.1)).changed();
            ui.end_row();

            ui.label("Max Anisotropy").on_hover_text("The amount of anisotropic filtering. Improves texture quality at extreme angles.");
            egui::ComboBox::from_id_source(id.with("anis{:?}"))
                .selected_text(
                    anisotropy_label(param
                        .data
                        .max_anisotropy)
                )
                .show_ui(ui, |ui| {
                    changed |= ui.selectable_value(&mut param.data.max_anisotropy, None, "None").changed();
                    ui.separator();

                    for variant in MaxAnisotropy::iter() {
                        let value = Some(variant);
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

    let header_response = response
        .header_response
        .on_disabled_hover_text(UNUSED_PARAM);

    if !errors.is_empty() {
        header_response.on_hover_ui(|ui| {
            display_validation_errors(ui, errors);
        });
    }

    changed
}

fn param_text(param_id: ParamId, errors: &[&&MatlValidationError]) -> RichText {
    // Show errors that apply to this parameter.
    if errors.is_empty() {
        RichText::new(param_label(param_id))
    } else {
        warning_icon_text(&param_label(param_id))
    }
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

    ui.add_enabled(enabled, Label::new(param_label(param.param_id)))
        .on_disabled_hover_text(UNUSED_PARAM);

    let channels = program
        .map(|p| p.accessed_channels(&param.param_id.to_string()))
        .unwrap_or_default();
    let labels = vector4_labels_short(param.param_id);
    let labels_long = vector4_labels_long(param.param_id);

    // Prevent editing components not accessed by the shader code.
    let id = egui::Id::new(param.param_id.to_string());
    let edit_component = |ui: &mut Ui, changed: &mut bool, i, value| {
        let component = labels[i];
        ui.add_enabled_ui(enabled && channels[i], |ui| {
            ui.horizontal(|ui| {
                ui.add_sized([15.0, 20.0], egui::Label::new(component));
                *changed |= ui
                    .add(DragSlider::new(id.with(labels[i]), value).width(50.0))
                    .on_hover_text(labels_long[i])
                    .changed();
            })
        })
        .inner
        .response
        .on_disabled_hover_text(format!(
            "Vector component {component} is not accessed by the shader and will be ignored in game."
        ));
    };

    ui.horizontal(|ui| {
        ui.add_enabled_ui(enabled, |ui| {
            changed |= edit_vector4_rgba(ui, &mut param.data);
        });
        edit_component(ui, &mut changed, 0, &mut param.data.x);
        edit_component(ui, &mut changed, 1, &mut param.data.y);
        edit_component(ui, &mut changed, 2, &mut param.data.z);
        edit_component(ui, &mut changed, 3, &mut param.data.w);
    });

    changed
}

fn edit_vector4_rgba(ui: &mut Ui, data: &mut Vector4) -> bool {
    // TODO: Edit alpha for params with alpha?
    let mut color = [data.x, data.y, data.z];
    if ui.color_edit_button_rgb(&mut color).changed() {
        data.x = color[0];
        data.y = color[1];
        data.z = color[2];
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
        format!("{p} ({description})")
    } else {
        p.to_string()
    }
}
