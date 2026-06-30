use std::collections::{BTreeMap, BTreeSet};

use super::{
    ERROR_COLOR, ICON_SIZE, UiState, adj_icon, anim_icon, display_validation_errors, empty_icon,
    hlpb_icon, matl_icon, mesh_icon, missing_icon, skel_icon, warning_icon, warning_icon_text,
};
use crate::{FileResult, ModelFolderState, validation::MatlValidationErrorKind};
use egui::{Button, Response, RichText, Ui, load::SizedTexture};

pub fn show_folder_files(
    ui_state: &mut UiState,
    model: &mut ModelFolderState,
    ui: &mut Ui,
    folder_index: usize,
    dark_mode: bool,
) {
    // Avoid a confusing missing file error for animation or texture folders.
    let is_model = model.is_model_folder();
    let required_file = |name| if is_model { Some(name) } else { None };

    // Clicking a file opens the corresponding editor.
    // Set selected index so the editor remains open for the file.
    list_files(
        ui,
        &model.model.meshes,
        &model.changed.meshes,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_mesh,
        required_file("model.numshb"),
        &model.validation.mesh_errors,
        |ui| mesh_icon(ui, dark_mode),
    );
    list_files(
        ui,
        &model.model.skels,
        &model.changed.skels,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_skel,
        required_file("model.nusktb"),
        &model.validation.skel_errors,
        |ui| skel_icon(ui, dark_mode),
    );
    list_files(
        ui,
        &model.model.hlpbs,
        &model.changed.hlpbs,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_hlpb,
        None,
        &model.validation.hlpb_errors,
        |ui| hlpb_icon(ui, dark_mode),
    );
    list_files(
        ui,
        &model.model.matls,
        &model.changed.matls,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_matl,
        required_file("model.numatb"),
        &model.validation.matl_errors,
        |ui| matl_icon(ui, dark_mode),
    );
    list_files(
        ui,
        &model.model.modls,
        &model.changed.modls,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_modl,
        required_file("model.numdlb"),
        &model.validation.modl_errors,
        |ui| mesh_icon(ui, dark_mode),
    );
    list_files(
        ui,
        &model.model.adjs,
        &model.changed.adjs,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_adj,
        None,
        &model.validation.adj_errors,
        |ui| adj_icon(ui, dark_mode),
    );
    list_files(
        ui,
        &model.model.anims,
        &model.changed.anims,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_anim,
        None,
        &model.validation.anim_errors,
        |ui| anim_icon(ui, dark_mode),
    );
    list_files(
        ui,
        &model.model.meshexes,
        &model.changed.meshexes,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_meshex,
        None,
        &model.validation.meshex_errors,
        |ui| mesh_icon(ui, dark_mode),
    );
    // TODO: Modify this to use the same function as above.
    list_nutexb_files(
        ui,
        model,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_nutexb,
    );
}

fn list_nutexb_files(
    ui: &mut Ui,
    model: &ModelFolderState,
    folder_index: usize,
    selected_folder_index: &mut Option<usize>,
    selected_file_index: &mut Option<usize>,
) {
    // Show missing textures required by any matl only once.
    let mut missing_textures = BTreeSet::new();
    for e in model.validation.matl_errors.values().flatten() {
        if let MatlValidationErrorKind::MissingTextures { textures, .. } = &e.kind {
            missing_textures.extend(textures.iter().filter(|t| !t.is_empty()));
        }
    }

    for texture in missing_textures {
        missing_nutexb(ui, texture);
    }

    for (i, (file, _)) in model.model.nutexbs.iter().enumerate() {
        ui.horizontal(|ui| {
            if let Some((_, thumbnail, _)) =
                model.thumbnails.iter().find(|(name, _, _)| name == file)
            {
                ui.image(SizedTexture {
                    id: *thumbnail,
                    size: egui::Vec2::new(ICON_SIZE, ICON_SIZE),
                });
            } else {
                warning_icon(ui).on_hover_text(
                    "Failed to generate GPU texture. Check the application log for details.",
                );
            }

            let response = if let Some(errors) = model.validation.nutexb_errors.get(&i) {
                file_button_with_errors(ui, file, errors.iter())
            } else {
                ui.button(file)
            };

            if response.clicked() {
                *selected_folder_index = Some(folder_index);
                *selected_file_index = Some(i);
            }
        });
    }
}

fn missing_nutexb(ui: &mut Ui, name: &str) {
    ui.horizontal(|ui| {
        missing_icon(ui);
        ui.add_enabled(
            false,
            Button::new(RichText::new(name.to_owned() + ".nutexb").strikethrough()),
        );
    })
    .response
    .on_hover_text(format!(
        "Missing texture {name:?} required by a .numatb file. Include this file or fix the texture assignment."
    ));
}

fn list_files<T, E: std::fmt::Display, F: Fn(&mut Ui) -> Response>(
    ui: &mut Ui,
    files: &[(String, FileResult<T>)],
    changed: &[bool],
    folder_index: usize,
    selected_folder_index: &mut Option<usize>,
    selected_file_index: &mut Option<usize>,
    required_file: Option<&'static str>,
    validation_errors: &BTreeMap<usize, Vec<E>>,
    file_icon: F,
) {
    // TODO: Should this be a grid instead?
    for (i, (name, file)) in files.iter().enumerate() {
        ui.horizontal(|ui| {
            match file {
                Some(_) => {
                    file_icon(ui);

                    let response = if let Some(errors) = validation_errors.get(&i) {
                        file_button_with_errors(ui, name, errors.iter())
                    } else {
                        ui.button(name)
                    };

                    if response.clicked() {
                        *selected_folder_index = Some(folder_index);
                        *selected_file_index = Some(i);
                    }

                    // TODO: Investigate different ways of displaying this.
                    if let Some(true) = changed.get(i) {
                        ui.label("[Modified]");
                    }
                }
                None => {
                    // TODO: Investigate a cleaner way to summarize errors.
                    // Don't show the full error for now to avoid showing lots of text.
                    empty_icon(ui);
                    ui.label(RichText::new("⚠ ".to_string() + name).color(ERROR_COLOR))
                        .on_hover_text(format!(
                            "Error reading {name}. Check the application logs for details."
                        ));
                }
            }
        });
    }
    if let Some(required_file) = required_file
        && !files.iter().any(|(f, _)| f == required_file)
    {
        missing_file(ui, required_file);
    }
}

fn file_button_with_errors<'a, E>(
    ui: &mut Ui,
    name: &str,
    validation_errors: impl Iterator<Item = &'a E>,
) -> Response
where
    E: std::fmt::Display + 'a,
{
    // TODO: Only color the icon itself?
    // TODO: Show top few errors and ... N others on hover?
    // TODO: Display the validation errors as a separate window on click?
    ui.add(Button::new(warning_icon_text(name)))
        .on_hover_ui(|ui| {
            display_validation_errors(ui, validation_errors);
        })
}

fn missing_file(ui: &mut Ui, name: &str) {
    ui.horizontal(|ui| {
        missing_icon(ui);
        ui.add_enabled(false, Button::new(RichText::new(name).strikethrough()));
    })
    .response
    .on_hover_text(format!("Missing required file {name}."));
}
