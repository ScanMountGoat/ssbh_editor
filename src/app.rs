use crate::{
    editors::{
        hlpb::hlpb_editor, matl::matl_editor, mesh::mesh_editor, modl::modl_editor,
        nutexb::nutexb_viewer, skel::skel_editor,
    },
    load_model, load_models_recursive,
    render_settings::render_settings,
    widgets::*,
    AnimationIndex, AnimationState, RenderState,
};
use egui::{
    collapsing_header::CollapsingState, Button, CollapsingHeader, Context, RichText, ScrollArea,
    SidePanel, TopBottomPanel, Ui, Window,
};
use lazy_static::lazy_static;
use log::Log;
use rfd::FileDialog;
use ssbh_data::matl_data::MatlEntryData;
use ssbh_wgpu::{ModelFolder, RenderModel};
use std::{error::Error, path::Path, sync::Mutex};

lazy_static! {
    pub static ref LOGGER: AppLogger = AppLogger {
        messages: Mutex::new(Vec::new())
    };
}

pub struct AppLogger {
    messages: Mutex<Vec<(log::Level, String)>>,
}

impl Log for AppLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // TODO: Investigate why wgpu_text warns about cache resizing.
        // Silence this error for now.
        metadata.level() <= log::Level::Warn && metadata.target() != "wgpu_text"
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.messages
                .lock()
                .unwrap()
                .push((record.level(), format!("{}", record.args())));
        }
    }

    fn flush(&self) {}
}

pub struct SsbhApp {
    pub should_refresh_meshes: bool,
    pub should_refresh_render_settings: bool,

    pub should_show_update: bool,
    pub new_release_tag: Option<String>,

    pub material_presets: Vec<MatlEntryData>,

    pub red_checkerboard: egui::TextureId,
    pub yellow_checkerboard: egui::TextureId,

    pub draw_skeletons: bool,
    pub draw_bone_names: bool,

    pub ui_state: UiState,
    // TODO: How to manage the thumbnail cache?
    // TODO: Is parallel list with models the best choice here?
    pub models: Vec<ModelFolder>,
    pub render_models: Vec<RenderModel>,
    pub thumbnails: Vec<Vec<(String, egui::TextureId)>>,
    pub default_thumbnails: Vec<(String, egui::TextureId)>,
    pub animation_state: AnimationState,
    pub render_state: RenderState,
}

#[derive(Default)]
pub struct UiState {
    // TODO: Add a changed flag and clear on save?
    // This would allow showing an indication for which files need to be saved.
    // TODO: Allow more than one open editor of each type?
    pub material_editor_open: bool,
    pub render_settings_open: bool,
    pub right_panel_tab: PanelTab,
    pub matl_editor_advanced_mode: bool,
    pub modl_editor_advanced_mode: bool,
    pub mesh_editor_advanced_mode: bool,
    pub log_window_open: bool,
    pub preset_window_open: bool,
    pub selected_material_preset_index: usize,
    pub is_editing_material_label: bool,
    // TODO: Is there a better way to track this?
    // Clicking an item in the file list sets the selected index.
    // If the index is not None, the corresponding editor stays open.
    pub selected_folder_index: Option<usize>,
    pub selected_skel_index: Option<usize>,
    pub selected_hlpb_index: Option<usize>,
    pub selected_matl_index: Option<usize>,
    pub selected_modl_index: Option<usize>,
    pub selected_mesh_index: Option<usize>,
    pub selected_nutexb_index: Option<usize>,
    pub selected_material_index: usize,
}

const ICON_SIZE: f32 = 18.0;
const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 40, 40);

// Keep track of what UI should be displayed.
#[derive(PartialEq, Eq)]
pub enum PanelTab {
    MeshList,
    AnimList,
}

impl Default for PanelTab {
    fn default() -> Self {
        Self::MeshList
    }
}

impl SsbhApp {
    pub fn open_folder(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            // TODO: Sort alphabetically?
            // TODO: Allow for opening folders with no mesh/modl?
            self.models = load_models_recursive(folder);
            self.should_refresh_meshes = true;
        }
    }

    pub fn add_folder_to_workspace(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            // Load the folder manually to avoid skipping folders with just animations.
            // TODO: Is there an easier way to allow loading animation folders?
            let new_model = load_model(&folder);
            self.models.push(new_model);
            // TODO: Only update the model that was added?
            self.should_refresh_meshes = true;
        }
    }

    pub fn reload_workspace(&mut self) {
        // This also reloads animations since animations are stored as indices.
        for model in &mut self.models {
            *model = ModelFolder::load_folder(&model.folder_name);
        }
        self.should_refresh_meshes = true;
    }

    pub fn clear_workspace(&mut self) {
        self.models = Vec::new();
        self.render_models = Vec::new();
        self.thumbnails = Vec::new();
        self.animation_state.animations = Vec::new();
        // TODO: Reset selected indices?
        // TODO: Is there an easy way to write this?
    }
}

impl SsbhApp {
    pub fn update(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| self.menu_bar(ui));

        // Add windows here so they can overlap everything except the top panel.
        // We store some state in self to keep track of whether this should be left open.
        if self.ui_state.render_settings_open {
            self.should_refresh_render_settings = true;
        }
        render_settings(
            ctx,
            &mut self.render_state.render_settings,
            &mut self.ui_state.render_settings_open,
            &mut self.draw_skeletons,
            &mut self.draw_bone_names,
        );

        log_window(ctx, &mut self.ui_state.log_window_open);

        // Don't reopen the window once closed.
        if self.should_show_update {
            self.new_release_window(ctx);
        }

        self.file_editors(ctx);

        let _viewport_left = SidePanel::left("left_panel")
            .min_width(200.0)
            .show(ctx, |ui| self.files_list(ui))
            .response
            .rect
            .right();

        TopBottomPanel::bottom("bottom panel").show(ctx, |ui| self.animation_and_log(ui));

        let _viewport_right = SidePanel::right("right panel")
            .min_width(350.0)
            .show(ctx, |ui| self.right_panel(ctx, ui))
            .response
            .rect
            .left();

        // TODO: Reduce overdraw when the UI overlaps the viewport.
    }

    fn new_release_window(&mut self, ctx: &Context) {
        if let Some(new_release_tag) = &self.new_release_tag {
            Window::new("New Release Available")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .resizable(false)
                .collapsible(false)
                .open(&mut self.should_show_update)
                .show(ctx, |ui| {
                    ui.label("A new release of SSBH Editor is available!");
                    ui.label(format!(
                        "The latest version is {}. The current version is {}.",
                        new_release_tag,
                        env!("CARGO_PKG_VERSION")
                    ));
                    ui.label("Download the new version from here:");
                    let release_link = "https://github.com/ScanMountGoat/ssbh_editor/releases";
                    if ui.hyperlink(release_link).clicked() {
                        if let Err(open_err) = open::that(release_link) {
                            log::error!(
                                "Failed to open link ({release_link}) to releases {open_err}"
                            );
                        }
                    }
                    // TODO: Show latest version and release notes.
                    // TODO: Parse release notes from changelog.
                });
        }
    }

    fn file_editors(&mut self, ctx: &Context) {
        let display_name = |folder: &str, name: &str| {
            format!(
                "{}/{}",
                Path::new(folder).file_name().unwrap().to_string_lossy(),
                name
            )
        };
        // TODO: Refactor this so they can also be "docked" in side panel tabs.
        // The functions should take an additional ui parameter.
        if let Some(folder_index) = self.ui_state.selected_folder_index {
            if let Some(model) = self.models.get_mut(folder_index) {
                if let Some(skel_index) = self.ui_state.selected_skel_index {
                    if let Some((name, Ok(skel))) = model.skels.get_mut(skel_index) {
                        if !skel_editor(ctx, &display_name(&model.folder_name, name), skel) {
                            // Close the window.
                            self.ui_state.selected_skel_index = None;
                        }
                    }
                }

                if let Some(mesh_index) = self.ui_state.selected_mesh_index {
                    if let Some((name, Ok(mesh))) = model.meshes.get_mut(mesh_index) {
                        if !mesh_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            mesh,
                            &mut self.ui_state.mesh_editor_advanced_mode,
                        ) {
                            // Close the window.
                            self.ui_state.selected_mesh_index = None;
                        }
                    }
                }

                // TODO: Make all this code a function?
                if let Some(matl_index) = self.ui_state.selected_matl_index {
                    if let Some((name, Ok(matl))) = model.matls.get_mut(matl_index) {
                        // TODO: Fix potential crash if thumbnails aren't present.
                        // TODO: Make this a method to simplify arguments.
                        if !matl_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &mut self.ui_state,
                            matl,
                            model
                                .modls
                                .iter_mut()
                                .find(|(f, _)| f == "model.numdlb")
                                .and_then(|(_, m)| m.as_mut().ok()),
                            model
                                .meshes
                                .iter()
                                .find(|(f, _)| f == "model.numshb")
                                .and_then(|(_, m)| m.as_ref().ok()),
                            &self.thumbnails[folder_index],
                            &self.default_thumbnails,
                            &self.render_state.shared_data.database,
                            &self.material_presets,
                            self.red_checkerboard,
                            self.yellow_checkerboard,
                        ) {
                            // Close the window.
                            self.ui_state.selected_matl_index = None;
                        } else if let Some(render_model) = self.render_models.get_mut(folder_index)
                        {
                            // TODO: Add change tracking using .changed() to improve performance.
                            // TODO: Is it worth optimizing this to only effect certain materials?
                            // Only the model.numatb is rendered in the viewport for now.
                            if name == "model.numatb" {
                                // TODO: How to efficiently handle renaming materials in ssbh_wgpu?
                                render_model.update_materials(
                                    &self.render_state.device,
                                    &self.render_state.queue,
                                    &matl.entries,
                                    &self.render_state.shared_data,
                                );
                            }
                        }
                    }
                }

                if let Some(modl_index) = self.ui_state.selected_modl_index {
                    if let Some((name, Ok(modl))) = model.modls.get_mut(modl_index) {
                        let matl = model
                            .matls
                            .iter()
                            .find(|(f, _)| f == "model.numatb")
                            .and_then(|(_, m)| m.as_ref().ok());
                        if !modl_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            modl,
                            model
                                .meshes
                                .iter()
                                .find(|(f, _)| f == "model.numshb")
                                .and_then(|(_, m)| m.as_ref().ok()),
                            matl,
                            &mut self.ui_state.modl_editor_advanced_mode,
                        ) {
                            // Close the window.
                            self.ui_state.selected_modl_index = None;
                        } else if let Some(render_model) = self.render_models.get_mut(folder_index)
                        {
                            // Update material previews in the viewport if the window remains open.
                            // TODO: Is it worth only doing this when changes actually occur?
                            render_model.reassign_materials(modl, matl);
                        }
                    }
                }

                if let Some(hlpb_index) = self.ui_state.selected_hlpb_index {
                    if let Some((name, Ok(hlpb))) = model.hlpbs.get_mut(hlpb_index) {
                        if !hlpb_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            hlpb,
                            model
                                .skels
                                .iter()
                                .find(|(f, _)| f == "model.nusktb")
                                .and_then(|(_, m)| m.as_ref().ok()),
                        ) {
                            // Close the window.
                            self.ui_state.selected_hlpb_index = None;
                        }
                    }
                }

                if let Some(nutexb_index) = self.ui_state.selected_nutexb_index {
                    if let Some((name, Ok(nutexb))) = model.nutexbs.get(nutexb_index) {
                        if !nutexb_viewer(
                            ctx,
                            &display_name(&model.folder_name, name),
                            nutexb,
                            &mut self.render_state.texture_render_settings,
                        ) {
                            // Close the window.
                            self.ui_state.selected_nutexb_index = None;
                        }
                    }
                }
            }
        }
    }

    fn animation_bar(&mut self, ui: &mut Ui) {
        let final_frame_index = self.max_final_frame_index();

        // TODO: How to fill available space?
        // TODO: Get the space that would normally be taken up by the central panel?
        ui.spacing_mut().slider_width = (ui.available_width() - 520.0).max(0.0);
        if ui
            .add(
                egui::Slider::new(
                    &mut self.animation_state.current_frame,
                    0.0..=final_frame_index,
                )
                .step_by(1.0)
                .show_value(false),
            )
            .changed()
        {
            // Manually trigger an update in case the playback is paused.
            self.animation_state.animation_frame_was_changed = true;
        }

        // Use a separate widget from the slider value to force the size.
        // This reduces the chances of the widget resizing during animations.
        if ui
            .add_sized(
                [60.0, 20.0],
                egui::DragValue::new(&mut self.animation_state.current_frame),
            )
            .changed()
        {
            // Manually trigger an update in case the playback is paused.
            self.animation_state.animation_frame_was_changed = true;
        }

        // Nest these conditions to avoid displaying both "Pause" and "Play" at once.
        let size = [60.0, 30.0];

        // TODO: Checkbox for looping?
        // TODO: Playback speed?
        if self.animation_state.is_playing {
            if ui.add_sized(size, Button::new("Pause")).clicked() {
                self.animation_state.is_playing = false;
            }
        } else if ui.add_sized(size, Button::new("Play")).clicked() {
            self.animation_state.is_playing = true;
        }
    }

    pub fn max_final_frame_index(&mut self) -> f32 {
        // Find the minimum number of frames to cover all animations.
        let mut final_frame_index = 0.0;
        for anim_index in &self.animation_state.animations {
            if let Some((_, Ok(anim))) =
                AnimationIndex::get_animation(anim_index.as_ref(), &self.models)
            {
                if anim.final_frame_index > final_frame_index {
                    final_frame_index = anim.final_frame_index;
                }
            }
        }
        final_frame_index
    }

    fn files_list(&mut self, ui: &mut Ui) {
        ui.heading("Files");
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (folder_index, model) in self.models.iter_mut().enumerate() {
                    CollapsingHeader::new(folder_display_name(model))
                        .id_source(format!("folder.{}", folder_index))
                        .default_open(true)
                        .show(ui, |ui| {
                            // TODO: Show a visual indication if a file has warnings/errors.
                            // This will encourage users to click and investigate the errors.

                            // Avoid a confusing missing file error for animation folders.
                            let just_anim = model.meshes.is_empty()
                                && model.modls.is_empty()
                                && model.skels.is_empty()
                                && model.matls.is_empty()
                                && !model.anims.is_empty();

                            let required_file = |name| if just_anim { None } else { Some(name) };

                            // Clicking a file opens the corresponding editor.
                            // Set selected index so the editor remains open for the file.
                            // TODO: Should the index be cleared when reloading models?
                            list_files(
                                ui,
                                &model.meshes,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_mesh_index,
                                required_file("model.numshb"),
                            );

                            list_files(
                                ui,
                                &model.skels,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_skel_index,
                                required_file("model.nusktb"),
                            );

                            list_files(
                                ui,
                                &model.hlpbs,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_hlpb_index,
                                None,
                            );

                            list_files(
                                ui,
                                &model.matls,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_matl_index,
                                required_file("model.numatb"),
                            );

                            list_files(
                                ui,
                                &model.modls,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_modl_index,
                                required_file("model.numdlb"),
                            );

                            for (i, (name, _)) in model.anims.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    empty_icon(ui);
                                    if ui.button(name).clicked() {
                                        let animation = AnimationIndex {
                                            folder_index,
                                            anim_index: i,
                                        };

                                        // Create the first slot if it doesn't exist to save mouse clicks.
                                        if self.animation_state.animations.is_empty() {
                                            self.animation_state.animations.push(Some(animation));
                                        } else if let Some(slot) = self
                                            .animation_state
                                            .animations
                                            .get_mut(self.animation_state.selected_slot)
                                        {
                                            *slot = Some(animation);
                                        }

                                        // Preview the new animation as soon as it is clicked.
                                        self.animation_state.animation_frame_was_changed = true;
                                    }
                                });
                            }

                            // TODO: Show file errors.
                            for (i, (file, _)) in model.nutexbs.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if let Some(model_thumbnails) =
                                        self.thumbnails.get(folder_index)
                                    {
                                        if let Some((_, thumbnail)) =
                                            model_thumbnails.iter().find(|(name, _)| name == file)
                                        {
                                            ui.image(
                                                *thumbnail,
                                                egui::Vec2::new(ICON_SIZE, ICON_SIZE),
                                            );
                                        }
                                    }

                                    if ui.button(file).clicked() {
                                        self.ui_state.selected_folder_index = Some(folder_index);
                                        self.ui_state.selected_nutexb_index = Some(i);
                                    }
                                });
                            }
                        });
                }
            });
    }

    fn animation_and_log(&mut self, ui: &mut Ui) {
        ui.with_layout(
            egui::Layout::left_to_right().with_cross_align(egui::Align::Center),
            |ui| {
                self.animation_bar(ui);

                // The next layout needs to be min since it's nested inside a centered layout.
                ui.with_layout(
                    egui::Layout::right_to_left().with_cross_align(egui::Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            if let Some((level, message)) = LOGGER.messages.lock().unwrap().last() {
                                if ui.add_sized([60.0, 30.0], Button::new("Logs")).clicked() {
                                    self.ui_state.log_window_open = true;
                                }

                                // Clicking the message also opens the log window.
                                let abbreviated_message =
                                    message.get(..40).unwrap_or_default().to_string() + "...";
                                if ui
                                    .add(
                                        egui::Label::new(abbreviated_message)
                                            .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                {
                                    self.ui_state.log_window_open = true;
                                }

                                match level {
                                    log::Level::Error => error_icon(ui),
                                    log::Level::Warn => warning_icon(ui),
                                    log::Level::Info => (),
                                    log::Level::Debug => (),
                                    log::Level::Trace => (),
                                }
                            }
                        });
                    },
                );
            },
        );
    }

    fn right_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        // TODO: Is it worth creating a dedicated tab control?
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.ui_state.right_panel_tab,
                PanelTab::MeshList,
                RichText::new("Meshes").heading(),
            );
            ui.selectable_value(
                &mut self.ui_state.right_panel_tab,
                PanelTab::AnimList,
                RichText::new("Animations").heading(),
            );
        });
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| match self.ui_state.right_panel_tab {
                PanelTab::MeshList => mesh_list(ctx, self, ui),
                PanelTab::AnimList => anim_list(ctx, self, ui),
            });
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                let button = |ui: &mut Ui, text| ui.add(Button::new(text).wrap(false));

                if button(ui, "Open Folder...    (Ctrl+O)").clicked() {
                    ui.close_menu();
                    self.open_folder();
                }

                if button(ui, "Add Folder to Workspace...").clicked() {
                    ui.close_menu();
                    self.add_folder_to_workspace();
                }

                if button(ui, "Reload Workspace    (Ctrl+R)").clicked() {
                    ui.close_menu();
                    self.reload_workspace();
                }

                if button(ui, "Clear Workspace").clicked() {
                    ui.close_menu();
                    self.clear_workspace();
                }
            });

            egui::menu::menu_button(ui, "Menu", |ui| {
                if ui.button("Render Settings").clicked() {
                    ui.close_menu();
                    self.ui_state.render_settings_open = true;
                }
            });

            egui::menu::menu_button(ui, "Help", |ui| {
                if ui.button("GitHub Repository").clicked() {
                    ui.close_menu();
                    let link = "https://github.com/ScanMountGoat/ssbh_editor";
                    if let Err(open_err) = open::that(link) {
                        log::error!("Failed to open link ({link}). {open_err}");
                    }
                }

                if ui.button("Report Issues").clicked() {
                    ui.close_menu();
                    let link = "https://github.com/ScanMountGoat/ssbh_editor/issues";
                    if let Err(open_err) = open::that(link) {
                        log::error!("Failed to open link ({link}). {open_err}");
                    }
                }

                if ui.button("Changelog").clicked() {
                    ui.close_menu();
                    let link =
                        "https://github.com/ScanMountGoat/ssbh_editor/blob/main/CHANGELOG.md";
                    if let Err(open_err) = open::that(link) {
                        log::error!("Failed to open link ({link}). {open_err}");
                    }
                }
            });
        });
    }
}

fn folder_display_name(model: &ModelFolder) -> String {
    Path::new(&model.folder_name)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}

fn list_files<T>(
    ui: &mut Ui,
    files: &[(String, Result<T, Box<dyn Error>>)],
    folder_index: usize,
    selected_folder_index: &mut Option<usize>,
    selected_file_index: &mut Option<usize>,
    required_file: Option<&'static str>,
) {
    // TODO: Should this be a grid instead?
    for (i, (name, file)) in files.iter().enumerate() {
        ui.horizontal(|ui| {
            // TODO: How to check for and store validation?
            match file {
                Ok(_) => {
                    empty_icon(ui);
                    if ui.button(name.clone()).clicked() {
                        *selected_folder_index = Some(folder_index);
                        *selected_file_index = Some(i);
                    }
                }
                Err(_) => {
                    // TODO: Investigate a cleaner way to show binrw backtrace errors.
                    // Don't show the full error for now to avoid showing lots of text.
                    error_icon(ui);
                    ui.label(RichText::new(name).color(ERROR_COLOR))
                        .on_hover_text(format!("Error reading {}", name));
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

fn missing_file(ui: &mut Ui, name: &str) {
    // TODO: Should the tooltip cover the entire layout?
    ui.horizontal(|ui| {
        missing_icon(ui);
        ui.add_enabled(false, Button::new(RichText::new(name).strikethrough()));
    })
    .response
    .on_hover_text(format!("Missing required file {name}"));
}

pub fn empty_icon(ui: &mut Ui) {
    ui.allocate_space(egui::Vec2::new(ICON_SIZE, ICON_SIZE));
}

pub fn missing_icon(ui: &mut Ui) {
    ui.label(RichText::new("⚠").size(ICON_SIZE));
}

pub fn warning_icon(ui: &mut Ui) {
    ui.label(
        RichText::new("⚠")
            .strong()
            .color(egui::Color32::from_rgb(255, 210, 0))
            .size(ICON_SIZE),
    );
}

pub fn error_icon(ui: &mut Ui) {
    ui.label(
        RichText::new("⚠")
            .strong()
            .color(ERROR_COLOR)
            .size(ICON_SIZE),
    );
}

fn mesh_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // TODO: Display folders that only have animations differently?
    for (i, folder) in app.models.iter().enumerate() {
        let name = format!("meshlist.{}", i);

        let id = ui.make_persistent_id(&name);
        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                if let Some(render_model) = app.render_models.get_mut(i) {
                    ui.add(EyeCheckBox::new(
                        &mut render_model.is_visible,
                        &folder_display_name(folder),
                    ));
                }
            })
            .body(|ui| {
                // TODO: How to ensure the render models stay in sync with the model folder?
                if let Some(render_model) = app.render_models.get_mut(i) {
                    for mesh in &mut render_model.meshes {
                        ui.add(EyeCheckBox::new(&mut mesh.is_visible, &mesh.name));
                    }
                }
            });
    }
}

fn anim_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // TODO: Will these IDs be unique?
    if ui.button("Add Slot").clicked() {
        app.animation_state.animations.push(None);
    }

    let mut slots_to_remove = Vec::new();
    for (i, anim_index) in app.animation_state.animations.iter().enumerate().rev() {
        // TODO: Unique IDs?
        let id = ui.make_persistent_id(i);
        CollapsingState::load_with_default_open(ctx, id, false)
            .show_header(ui, |ui| {
                let name = AnimationIndex::get_animation(anim_index.as_ref(), &app.models)
                    .map(|(name, _)| name.to_string())
                    .unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut app.animation_state.selected_slot,
                        i,
                        format!("Slot {i} - {name}"),
                    );
                    if ui.button("Remove").clicked() {
                        slots_to_remove.push(i);
                    }
                });
            })
            .body(|ui| {
                if let Some((_, Ok(anim))) =
                    AnimationIndex::get_animation(anim_index.as_ref(), &app.models)
                {
                    for group in &anim.groups {
                        CollapsingHeader::new(group.group_type.to_string())
                            .default_open(true)
                            .show(ui, |ui| {
                                for node in &group.nodes {
                                    match node.tracks.as_slice() {
                                        [_] => {
                                            // Don't use a collapsing header if there is only one track.
                                            // This simplifies the layout for most boolean and transform tracks.
                                            // TODO: How to toggle visibility for rendering?
                                            ui.label(&node.name);
                                        }
                                        _ => {
                                            CollapsingHeader::new(&node.name)
                                                .default_open(true)
                                                .show(ui, |ui| {
                                                    for track in &node.tracks {
                                                        // TODO: How to toggle visibility for rendering?
                                                        ui.label(&track.name);
                                                    }
                                                });
                                        }
                                    }
                                }
                            });
                    }
                }
            });
    }

    // TODO: Force only one slot to be removed?
    for slot in slots_to_remove {
        app.animation_state.animations.remove(slot);
    }
}

fn log_window(ctx: &Context, open: &mut bool) {
    Window::new("Application Log")
        .open(open)
        .resizable(true)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (level, message) in LOGGER.messages.lock().unwrap().iter() {
                        ui.horizontal(|ui| {
                            match level {
                                log::Level::Error => error_icon(ui),
                                log::Level::Warn => warning_icon(ui),
                                log::Level::Info => (),
                                log::Level::Debug => (),
                                log::Level::Trace => (),
                            }
                            ui.label(message);
                        });
                    }
                });
        });
}

// TODO: Animation Viewer
// Users want to know what values are being effected, see the values, and toggle tracks on/off.
// The display could be done using egui's plotting capabilities using Blender as a reference.
