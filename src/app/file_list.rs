use super::{
    display_validation_errors, empty_icon, missing_icon, warning_icon, warning_icon_text, Icons,
    UiState, ERROR_COLOR, ICON_SIZE,
};
use crate::{validation::MatlValidationErrorKind, FileResult, ModelFolderState};
use egui::{Button, Response, RichText, Ui};

pub fn show_folder_files(
    ui_state: &mut UiState,
    model: &mut ModelFolderState,
    ui: &mut Ui,
    folder_index: usize,
    icons: &Icons,
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
        Some("model.numshb"),
        &model.validation.mesh_errors,
        |ui| ui.add(icons.mesh(ui)),
    );
    list_files(
        ui,
        &model.model.skels,
        &model.changed.skels,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_skel,
        required_file("model.nusktb"),
        Some("model.nusktb"),
        &model.validation.skel_errors,
        |ui| ui.label("🗋"),
    );
    list_files(
        ui,
        &model.model.hlpbs,
        &model.changed.hlpbs,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_hlpb,
        None,
        Some("model.nuhlpb"),
        &model.validation.hlpb_errors,
        |ui| ui.label("🗋"),
    );
    list_files(
        ui,
        &model.model.matls,
        &model.changed.matls,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_matl,
        required_file("model.numatb"),
        Some("model.numatb"),
        &model.validation.matl_errors,
        |ui| ui.add(icons.matl(ui)),
    );
    list_files(
        ui,
        &model.model.modls,
        &model.changed.modls,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_modl,
        required_file("model.numdlb"),
        Some("model.numdlb"),
        &model.validation.modl_errors,
        |ui| ui.label("🗋"),
    );
    list_files(
        ui,
        &model.model.adjs,
        &model.changed.adjs,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_adj,
        None,
        Some("model.adjb"),
        &model.validation.adj_errors,
        |ui| ui.add(icons.adj(ui)),
    );
    list_files(
        ui,
        &model.model.anims,
        &model.changed.anims,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_anim,
        None,
        None,
        &model.validation.anim_errors,
        |ui| ui.add(icons.anim(ui)),
    );
    // TODO: Is the model.numshexb required?
    list_files(
        ui,
        &model.model.meshexes,
        &model.changed.meshexes,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.open_meshex,
        None,
        Some("model.numshexb"),
        &model.validation.meshex_errors,
        |ui| ui.label("🗋"),
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
    // Show missing textures required by the matl.
    for e in &model.validation.matl_errors {
        if let MatlValidationErrorKind::MissingTextures { textures, .. } = &e.kind {
            for texture in textures {
                missing_nutexb(ui, texture);
            }
        }
    }
    for (i, (file, _)) in model.model.nutexbs.iter().enumerate() {
        // TODO: Avoid collect?
        let validation_errors: Vec<_> = model
            .validation
            .nutexb_errors
            .iter()
            .filter(|e| e.name() == file)
            .collect();

        ui.horizontal(|ui| {
            if let Some((_, thumbnail, _)) =
                model.thumbnails.iter().find(|(name, _, _)| name == file)
            {
                ui.image(*thumbnail, egui::Vec2::new(ICON_SIZE, ICON_SIZE));
            } else {
                warning_icon(ui).on_hover_text(
                    "Failed to generate GPU texture. Check the application log for details.",
                );
            }

            let response = if !validation_errors.is_empty() {
                file_button_with_errors(ui, file, &validation_errors)
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
        "Missing texture {name:?} required by model.numatb. Include this file or fix the texture assignment."
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
    validation_file: Option<&'static str>,
    validation_errors: &[E],
    file_icon: F,
) {
    // TODO: Should this be a grid instead?
    for (i, (name, file)) in files.iter().enumerate() {
        ui.horizontal(|ui| {
            match file {
                Ok(_) => {
                    file_icon(ui);

                    // Assume only the required file is validated for now.
                    // This excludes files like metamon_model.numatb.
                    let response = if !validation_errors.is_empty()
                        && Some(name.as_str()) == validation_file
                    {
                        file_button_with_errors(ui, name, validation_errors)
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
                Err(_) => {
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
    if let Some(required_file) = required_file {
        if !files.iter().any(|(f, _)| f == required_file) {
            missing_file(ui, required_file);
        }
    }
}

fn file_button_with_errors<E: std::fmt::Display>(
    ui: &mut Ui,
    name: &str,
    validation_errors: &[E],
) -> Response {
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
