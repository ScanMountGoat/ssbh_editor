use std::path::PathBuf;

use egui::{Grid, Label, TextWrapMode, Ui, Window};
use rfd::FileDialog;

use crate::app::StageLightingState;

pub fn stage_lighting_window(
    ctx: &egui::Context,
    open: &mut bool,
    state: &mut StageLightingState,
) -> bool {
    let mut changed = false;
    Window::new("Stage Lighting")
        .open(open)
        .resizable(false)
        .show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open render folder...").clicked()
                        && let Some(folder) = FileDialog::new().pick_folder()
                    {
                        // Attempt to load supported lighting files based on naming conventions.
                        // Users should select paths like "/stage/battlefield/normal/render/".
                        state.light = Some(folder.join("light").join("light00.nuanmb"));
                        state.reflection_cube_map = Some(folder.join("reflection_cubemap.nutexb"));
                        state.color_grading_lut = folder
                            .parent()
                            .map(|p| p.join("lut").join("color_grading_lut.nutexb"));
                        changed = true;
                    }
                });
            });
            ui.separator();

            Grid::new("stage_lighting").show(ui, |ui| {
                // TODO: Make the files buttons to load corresponding editors?
                ui.label("Lighting");
                path_label(ui, &state.light);
                if ui.button("Select file...").clicked()
                    && let Some(file) = FileDialog::new()
                        .add_filter("Lighting Anim", &["nuanmb"])
                        .pick_file()
                {
                    state.light = Some(file);
                    changed = true;
                }
                ui.end_row();

                ui.label("Reflection Cube Map");
                path_label(ui, &state.reflection_cube_map);
                if ui.button("Select file...").clicked()
                    && let Some(file) = FileDialog::new()
                        .add_filter("Cube Map Nutexb", &["nutexb"])
                        .pick_file()
                {
                    state.reflection_cube_map = Some(file);
                    changed = true;
                }

                ui.end_row();

                ui.label("Color Grading LUT");
                path_label(ui, &state.color_grading_lut);
                if ui.button("Select file...").clicked()
                    && let Some(file) = FileDialog::new()
                        .add_filter("Color Grading LUT", &["nutexb"])
                        .pick_file()
                {
                    state.color_grading_lut = Some(file);
                    changed = true;
                };
                ui.end_row();
            });

            if ui.button("Reset").clicked() {
                *state = StageLightingState::default();
                changed = true;
            };
        });
    changed
}

fn path_label(ui: &mut Ui, path: &Option<PathBuf>) {
    match path {
        Some(path) => {
            ui.label(path.file_name().and_then(|f| f.to_str()).unwrap_or(""))
                .on_hover_ui(|ui| {
                    ui.add(Label::new(path.to_string_lossy()).wrap_mode(TextWrapMode::Extend));
                });
        }
        None => {
            ui.label("");
        }
    }
}
