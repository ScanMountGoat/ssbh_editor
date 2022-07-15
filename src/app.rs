use crate::{
    editors::{
        adj::adj_editor,
        hlpb::hlpb_editor,
        matl::{matl_editor, preset_editor},
        mesh::mesh_editor,
        modl::modl_editor,
        nutexb::nutexb_viewer,
        skel::skel_editor,
    },
    load_model, load_models_recursive,
    render_settings::render_settings,
    validation::ModelValidationErrors,
    widgets::*,
    AnimationIndex, AnimationSlot, AnimationState, CameraInputState, RenderState,
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
use std::{error::Error, f32::consts::PI, path::Path, sync::Mutex};

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
    pub should_refresh_camera_settings: bool,

    pub should_show_update: bool,
    pub new_release_tag: Option<String>,

    pub material_presets: Vec<MatlEntryData>,

    pub red_checkerboard: egui::TextureId,
    pub yellow_checkerboard: egui::TextureId,

    pub draw_skeletons: bool,
    pub draw_bone_names: bool,

    pub ui_state: UiState,
    // TODO: Is parallel list with models the best choice here?
    pub models: Vec<ModelFolder>,
    pub render_models: Vec<RenderModel>,
    pub thumbnails: Vec<Vec<(String, egui::TextureId)>>,
    pub validation_errors: Vec<ModelValidationErrors>,

    pub default_thumbnails: Vec<(String, egui::TextureId)>,
    pub animation_state: AnimationState,
    pub render_state: RenderState,

    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub show_bottom_panel: bool,

    pub camera_state: CameraInputState,
}

#[derive(Default)]
pub struct UiState {
    // TODO: Add a changed flag and clear on save?
    // This would allow showing an indication for which files need to be saved.
    // TODO: Allow more than one open editor of each type?
    pub material_editor_open: bool,
    pub render_settings_open: bool,
    pub camera_settings_open: bool,
    pub preset_editor_open: bool,
    pub right_panel_tab: PanelTab,
    pub modl_editor_advanced_mode: bool,
    pub mesh_editor_advanced_mode: bool,
    pub log_window_open: bool,
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
    pub selected_adj_index: Option<usize>,
    pub selected_anim_index: Option<usize>,

    pub selected_mesh_influences_index: Option<usize>,

    pub matl_preset_window_open: bool,
    pub selected_material_preset_index: usize,

    // TODO: Create a struct for this?
    pub matl_editor_advanced_mode: bool,
    pub matl_selected_material_index: usize,
    pub matl_is_editing_material_label: bool,

    pub preset_editor_advanced_mode: bool,
    pub preset_selected_material_index: usize,
    pub preset_is_editing_material_label: bool,

    pub light_mode: bool,
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
        // TODO: Express this as clear + add folder?
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.models = load_models_recursive(folder);
            self.animation_state.animations = vec![vec![AnimationSlot::new()]; self.models.len()];
            self.should_refresh_meshes = true;
        }
    }

    pub fn add_folder_to_workspace(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            // TODO: Should this not be recursive?
            // Users may want to load multiple animation folders at once for stages.
            let new_models = load_models_recursive(&folder);

            // TODO: Automatically assign model.nuanmb.
            // Add a dummy animation to prompt the user to select one.
            self.animation_state
                .animations
                .extend(vec![vec![AnimationSlot::new()]; new_models.len()]);

            self.models.extend(new_models);
            // TODO: Only update the models that were added?
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

    pub fn hide_expressions(&mut self) {
        let patterns: [&str; 34] = [
            "blink",
            "attack",
            "ouch",
            "talk",
            "capture",
            "ottotto",
            "escape",
            "half",
            "pattern",
            "result",
            "harf",
            "hot",
            "heavy",
            "voice",
            "fura",
            "catch",
            "cliff",
            "flip",
            "bound",
            "down",
            "final",
            "result",
            "steppose",
            "sorori",
            "fall",
            "appeal",
            "damage",
            "camerahit",
            "laugh",
            "breath",
            "swell",
            "_low",
            "_bink",
            "inkmesh",
        ];
        let pattern_exceptions: [&str; 3] = ["openblink", "belly_low", "facen"];

        for render_model in &mut self.render_models {
            for mesh in &mut render_model.meshes {
                let name = &mesh.name.to_lowercase();
                'pattern_search: for pattern in patterns {
                    //Default expressions
                    for pattern_exception in pattern_exceptions {
                        if name.contains(&pattern_exception) {
                            continue 'pattern_search;
                        }
                    }

                    //Make all other expressions invisible
                    if name.contains(&pattern) {
                        mesh.is_visible = false;
                    }
                }
            }
        }
    }
}

impl SsbhApp {
    pub fn update(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| self.menu_bar(ui));

        // Add windows here so they can overlap everything except the top panel.
        // We store some state in self to keep track of whether this should be left open.
        render_settings(
            ctx,
            &mut self.render_state.render_settings,
            &mut self.ui_state.render_settings_open,
            &mut self.draw_skeletons,
            &mut self.draw_bone_names,
        );
        if self.ui_state.render_settings_open {
            self.should_refresh_render_settings = true;
        }

        camera_settings(
            ctx,
            &mut self.ui_state.camera_settings_open,
            &mut self.camera_state,
        );
        if self.ui_state.camera_settings_open {
            self.should_refresh_camera_settings = true;
        }

        log_window(ctx, &mut self.ui_state.log_window_open);

        preset_editor(
            ctx,
            &mut self.ui_state,
            &mut self.material_presets,
            &self.default_thumbnails,
            &self.render_state.shared_data.database,
            self.red_checkerboard,
            self.yellow_checkerboard,
        );

        // Don't reopen the window once closed.
        if self.should_show_update {
            self.new_release_window(ctx);
        }

        self.file_editors(ctx);

        if self.show_left_panel {
            let _viewport_left = SidePanel::left("left_panel")
                .min_width(200.0)
                .show(ctx, |ui| self.files_list(ui))
                .response
                .rect
                .right();
        }

        if self.show_bottom_panel {
            TopBottomPanel::bottom("bottom panel").show(ctx, |ui| self.animation_and_log(ui));
        }

        if self.show_right_panel {
            let _viewport_right = SidePanel::right("right panel")
                .min_width(350.0)
                .show(ctx, |ui| self.right_panel(ctx, ui))
                .response
                .rect
                .left();
        }

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
                        if let Err(e) = open::that(release_link) {
                            log::error!("Failed to open {release_link}: {e}");
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
                Path::new(folder)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default(),
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
                            &mut self.ui_state,
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
                        // TODO: Potential index panic.
                        let validation_errors = &self.validation_errors[folder_index].matl_errors;

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
                            validation_errors,
                            &self.thumbnails[folder_index],
                            &self.default_thumbnails,
                            &self.render_state.shared_data.database,
                            &mut self.material_presets,
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

                if let Some(adj_index) = self.ui_state.selected_adj_index {
                    if let Some((name, Ok(adj))) = model.adjs.get_mut(adj_index) {
                        if !adj_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            adj,
                            model
                                .meshes
                                .iter()
                                .find(|(f, _)| f == "model.numshb")
                                .and_then(|(_, m)| m.as_ref().ok()),
                        ) {
                            // Close the window.
                            self.ui_state.selected_adj_index = None;
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
                // TODO: Show ticks?
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
        for model_animations in &self.animation_state.animations {
            for anim_index in model_animations.iter().filter_map(|a| a.animation.as_ref()) {
                if let Some((_, Ok(anim))) = AnimationIndex::get_animation(anim_index, &self.models)
                {
                    if anim.final_frame_index > final_frame_index {
                        final_frame_index = anim.final_frame_index;
                    }
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
                for (folder_index, (model, validation)) in self
                    .models
                    .iter_mut()
                    .zip(self.validation_errors.iter())
                    .enumerate()
                {
                    CollapsingHeader::new(folder_display_name(model))
                        .id_source(format!("folder.{}", folder_index))
                        .default_open(true)
                        .show(ui, |ui| {
                            // Avoid a confusing missing file error for animation or texture folders.
                            let is_model = is_model_folder(model);
                            let required_file = |name| if is_model { Some(name) } else { None };

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
                                &validation.mesh_errors,
                            );

                            list_files(
                                ui,
                                &model.skels,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_skel_index,
                                required_file("model.nusktb"),
                                &validation.skel_errors,
                            );

                            list_files(
                                ui,
                                &model.hlpbs,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_hlpb_index,
                                None,
                                &validation.hlpb_errors,
                            );

                            list_files(
                                ui,
                                &model.matls,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_matl_index,
                                required_file("model.numatb"),
                                &validation.matl_errors,
                            );

                            list_files(
                                ui,
                                &model.modls,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_modl_index,
                                required_file("model.numdlb"),
                                &validation.modl_errors,
                            );

                            list_files(
                                ui,
                                &model.adjs,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_adj_index,
                                None,
                                &validation.adj_errors,
                            );

                            list_files(
                                ui,
                                &model.anims,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_anim_index,
                                None,
                                &validation.anim_errors,
                            );

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

                // TODO: Store keyboard shortcuts in a single place?
                let ctrl = if cfg!(target_os = "macos") {
                    "⌘"
                } else {
                    "Ctrl"
                };

                let ctrl_shift = if cfg!(target_os = "macos") {
                    "⇧ ⌘"
                } else {
                    "Ctrl Shift"
                };

                if button(ui, format!("Open Folder...    {ctrl} O")).clicked() {
                    ui.close_menu();
                    self.open_folder();
                }

                if button(ui, format!("Add Folder to Workspace...    {ctrl_shift} O")).clicked() {
                    ui.close_menu();
                    self.add_folder_to_workspace();
                }

                if button(ui, format!("Reload Workspace    {ctrl} R")).clicked() {
                    ui.close_menu();
                    self.reload_workspace();
                }

                if button(ui, format!("Clear Workspace")).clicked() {
                    ui.close_menu();
                    self.clear_workspace();
                }
            });

            egui::menu::menu_button(ui, "Menu", |ui| {
                if ui.button("Render Settings").clicked() {
                    ui.close_menu();
                    self.ui_state.render_settings_open = true;
                }

                if ui.button("Camera Settings").clicked() {
                    ui.close_menu();
                    self.ui_state.camera_settings_open = true;
                }

                if ui.button("Material Presets").clicked() {
                    ui.close_menu();
                    self.ui_state.preset_editor_open = true;
                }
            });

            egui::menu::menu_button(ui, "Meshes", |ui| {
                if ui.button("Hide Expressions").clicked() {
                    ui.close_menu();
                    self.hide_expressions();
                }
            });

            egui::menu::menu_button(ui, "View", |ui| {
                ui.checkbox(&mut self.ui_state.light_mode, "Light Mode");
                ui.separator();
                ui.checkbox(&mut self.show_left_panel, "Left Panel");
                ui.checkbox(&mut self.show_right_panel, "Right Panel");
                ui.checkbox(&mut self.show_bottom_panel, "Bottom Panel");
            });

            egui::menu::menu_button(ui, "Help", |ui| {
                if ui.button("SSBH Editor Wiki").clicked() {
                    ui.close_menu();
                    let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki";
                    if let Err(e) = open::that(link) {
                        log::error!("Failed to open {link}: {e}");
                    }
                }

                if ui.button("Report Issues").clicked() {
                    ui.close_menu();
                    let link = "https://github.com/ScanMountGoat/ssbh_editor/issues";
                    if let Err(e) = open::that(link) {
                        log::error!("Failed to open {link}: {e}");
                    }
                }

                if ui.button("Changelog").clicked() {
                    ui.close_menu();
                    let link =
                        "https://github.com/ScanMountGoat/ssbh_editor/blob/main/CHANGELOG.md";
                    if let Err(e) = open::that(link) {
                        log::error!("Failed to open {link}: {e}");
                    }
                }
            });
        });
    }
}

fn is_model_folder(model: &ModelFolder) -> bool {
    !model.meshes.is_empty()
        || !model.modls.is_empty()
        || !model.skels.is_empty()
        || !model.matls.is_empty()
}

fn folder_display_name(model: &ModelFolder) -> String {
    Path::new(&model.folder_name)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn list_files<T, E: std::fmt::Display>(
    ui: &mut Ui,
    files: &[(String, Result<T, Box<dyn Error>>)],
    folder_index: usize,
    selected_folder_index: &mut Option<usize>,
    selected_file_index: &mut Option<usize>,
    required_file: Option<&'static str>,
    validation_errors: &[E],
) {
    // TODO: Should this be a grid instead?
    for (i, (name, file)) in files.iter().enumerate() {
        ui.horizontal(|ui| {
            match file {
                Ok(_) => {
                    // Assume only the required file is validated for now.
                    // This excludes files like metamon_model.numatb.
                    if !validation_errors.is_empty() && Some(name.as_str()) == required_file {
                        let messages: Vec<_> =
                            validation_errors.iter().map(|e| format!("{}", e)).collect();
                        warning_icon_with_messages(ui, &messages);
                    } else {
                        // TODO: This doesn't have the same size as the others?
                        empty_icon(ui);
                    }
                    if ui.button(name.clone()).clicked() {
                        *selected_folder_index = Some(folder_index);
                        *selected_file_index = Some(i);
                    }
                }
                Err(_) => {
                    // TODO: Investigate a cleaner way to summarize errors.
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

// TODO: Investigate why these have different sizes.
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

pub fn warning_icon_with_messages(ui: &mut Ui, messages: &[String]) {
    ui.label(
        RichText::new("⚠")
            .strong()
            .color(egui::Color32::from_rgb(255, 210, 0))
            .size(ICON_SIZE),
    )
    .on_hover_ui(|ui| {
        for message in messages {
            ui.horizontal(|ui| {
                warning_icon(ui);
                ui.label(message);
            });
        }
    });
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
    // Don't show non model folders like animation or texture folders.
    for (i, folder) in app
        .models
        .iter()
        .enumerate()
        .filter(|(_, folder)| is_model_folder(folder))
    {
        let name = format!("meshlist.{}", i);
        let id = ui.make_persistent_id(&name);

        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                if let Some(render_model) = app.render_models.get_mut(i) {
                    render_model.is_selected = ui
                        .add(EyeCheckBox::new(
                            &mut render_model.is_visible,
                            &folder_display_name(folder),
                        ))
                        .hovered();
                }
            })
            .body(|ui| {
                // TODO: How to ensure the render models stay in sync with the model folder?
                if let Some(render_model) = app.render_models.get_mut(i) {
                    ui.add_enabled_ui(render_model.is_visible, |ui| {
                        // Indent without the vertical line.
                        ui.visuals_mut().widgets.noninteractive.bg_stroke.width = 0.0;
                        ui.spacing_mut().indent = 24.0;
                        ui.indent("indent", |ui| {
                            for mesh in &mut render_model.meshes {
                                mesh.is_selected = ui
                                    .add(EyeCheckBox::new(&mut mesh.is_visible, &mesh.name))
                                    .hovered();
                            }
                        });
                    });
                }
            });
    }
}

fn anim_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // Only assign animations to folders with model files.
    for (model_index, model) in app
        .models
        .iter()
        .enumerate()
        .filter(|(_, model)| is_model_folder(model))
    {
        let mut slots_to_remove = Vec::new();

        let name = format!("animlist.{model_index}");
        let id = ui.make_persistent_id(&name);

        // TODO: Avoid unwrap.
        let model_animations = &mut app.animation_state.animations[model_index];

        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                // Assume the associated animation folder names matche the model folder.
                ui.label(folder_display_name(model));
            })
            .body(|ui| {
                if ui.button("Add Slot").clicked() {
                    model_animations.push(AnimationSlot::new());
                }

                // TODO: Make a function for this.
                for (slot, anim_slot) in model_animations.iter_mut().enumerate().rev() {
                    let id = ui.make_persistent_id(format!("{model_index}.slot.{slot}"));
                    CollapsingState::load_with_default_open(ctx, id, false)
                        .show_header(ui, |ui| {
                            let name = anim_slot
                                .animation
                                .as_ref()
                                .and_then(|anim_index| anim_index.get_animation(&app.models))
                                .map(|(name, _)| name.to_string())
                                .unwrap_or("Select an animation...".to_string());

                            ui.horizontal(|ui| {
                                // TODO: Disabling anims with visibility tracks has confusing behavior.
                                // Disabling a vis track currently only disables the effects on later frames.
                                if ui
                                    .add(EyeCheckBox::new(
                                        &mut anim_slot.is_enabled,
                                        format!("Slot {slot}"),
                                    ))
                                    .changed()
                                {
                                    app.animation_state.animation_frame_was_changed = true;
                                }

                                if anim_combo_box(
                                    ui,
                                    &app.models,
                                    model_index,
                                    slot,
                                    name,
                                    model,
                                    anim_slot,
                                ) {
                                    // Reflect selecting a new animation in the viewport.
                                    app.animation_state.animation_frame_was_changed = true;
                                }

                                if ui.button("Remove").clicked() {
                                    slots_to_remove.push(slot);
                                }
                            });
                        })
                        .body(|ui| {
                            if let Some((_, Ok(anim))) = anim_slot
                                .animation
                                .as_ref()
                                .and_then(|anim_index| anim_index.get_animation(&app.models))
                            {
                                for group in &anim.groups {
                                    CollapsingHeader::new(group.group_type.to_string())
                                        .default_open(false)
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
                    model_animations.remove(slot);
                }
            });
    }
}

fn anim_combo_box(
    ui: &mut Ui,
    models: &[ModelFolder],
    model_index: usize,
    slot: usize,
    name: String,
    model: &ModelFolder,
    anim_slot: &mut AnimationSlot,
) -> bool {
    // Associate animations with the model folder by name.
    // Motion folders use the same name as model folders.
    // TODO: Allow manually associating animations?
    // TODO: Is is it worth precomputing this list?
    // TODO: Handle unrelated folders with the same name like two c00 model folders?
    let mut available_anims = models
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            Path::new(&m.folder_name).file_name() == Path::new(&model.folder_name).file_name()
        })
        .flat_map(|(folder_index, m)| {
            m.anims
                .iter()
                .enumerate()
                .map(move |(anim_index, _)| AnimationIndex {
                    folder_index,
                    anim_index,
                })
        })
        .peekable();

    // TODO: Union the responses instead?
    // TODO: How to cleanly implement change tracking for complex editors?
    let mut changed = false;

    if available_anims.peek().is_some() {
        egui::ComboBox::from_id_source(format!("slot{:?}.{:?}", model_index, slot))
            .width(200.0)
            .selected_text(name)
            .show_ui(ui, |ui| {
                // TODO: Reset animations?
                for available_anim in available_anims {
                    let name = available_anim
                        .get_animation(models)
                        .map(|(name, _)| name.to_string())
                        .unwrap_or_default();

                    // Return true if any animation is selected.
                    changed |= ui
                        .selectable_value(&mut anim_slot.animation, Some(available_anim), name)
                        .changed();
                }
            });
    } else {
        ui.label("No animations found.");
    }

    changed
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
                            // binrw formats backtraces, which isn't supported by egui font rendering.
                            // ui.label(message);
                            let clean_message = strip_ansi_escapes::strip(message)
                                .map(|m| String::from_utf8_lossy(&m).to_string())
                                .unwrap_or_else(|_| message.clone());
                            ui.label(clean_message);
                        });
                    }
                });
        });
}

// TODO: Animation Viewer
// Users want to know what values are being effected, see the values, and toggle tracks on/off.
// The display could be done using egui's plotting capabilities using Blender as a reference.

pub fn camera_settings(ctx: &egui::Context, open: &mut bool, camera_state: &mut CameraInputState) {
    egui::Window::new("Camera Settings")
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            egui::Grid::new("camera_grid").show(ui, |ui| {
                ui.label("Translation X");
                ui.add(egui::DragValue::new(&mut camera_state.translation_xyz.x));
                ui.end_row();

                ui.label("Translation Y");
                ui.add(egui::DragValue::new(&mut camera_state.translation_xyz.y));
                ui.end_row();

                ui.label("Translation Z");
                ui.add(egui::DragValue::new(&mut camera_state.translation_xyz.z));
                ui.end_row();

                // TODO: This will need to use quaternions to work with camera anims.
                // TODO: Add an option for radians or degrees?
                ui.label("Rotation X");
                ui.add(
                    egui::DragValue::new(&mut camera_state.rotation_xyz.x)
                        .speed(0.01)
                        .clamp_range(-2.0 * PI..=2.0 * PI),
                );
                ui.end_row();

                ui.label("Rotation Y");
                ui.add(
                    egui::DragValue::new(&mut camera_state.rotation_xyz.y)
                        .speed(0.01)
                        .clamp_range(-2.0 * PI..=2.0 * PI),
                );
                ui.end_row();

                if ui.button("Reset").clicked() {
                    *camera_state = CameraInputState::default();
                }
            });
        });
}
