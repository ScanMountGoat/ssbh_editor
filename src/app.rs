use crate::{
    app::anim_list::anim_list,
    editors::{
        adj::adj_editor,
        anim::anim_editor,
        hlpb::hlpb_editor,
        matl::{matl_editor, preset_editor},
        mesh::mesh_editor,
        meshex::meshex_editor,
        modl::modl_editor,
        nutexb::nutexb_viewer,
        skel::skel_editor,
    },
    path::last_update_check_file,
    preferences::{preferences_window, AppPreferences},
    render_settings::render_settings,
    sort_files,
    validation::{MatlValidationError, ModelValidationErrors},
    widgets::*,
    AnimationIndex, AnimationSlot, AnimationState, CameraInputState, FileResult, RenderState,
};
use chrono::{DateTime, Utc};
use egui::{
    collapsing_header::CollapsingState, Button, CollapsingHeader, Context, DragValue, Grid, Label,
    Response, RichText, ScrollArea, SidePanel, TopBottomPanel, Ui, Window,
};
use log::Log;
use once_cell::sync::Lazy;
use rfd::FileDialog;
use ssbh_data::matl_data::MatlEntryData;
use ssbh_wgpu::{ModelFolder, RenderModel};
use std::{
    f32::consts::PI,
    path::{Path, PathBuf},
    sync::Mutex,
};

mod anim_list;

pub static LOGGER: Lazy<AppLogger> = Lazy::new(|| AppLogger {
    messages: Mutex::new(Vec::new()),
});

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
    pub should_refresh_render_settings: bool,
    pub should_refresh_camera_settings: bool,
    // TODO: Track what files changed in each folder?
    pub models_to_update: ItemsToUpdate,
    pub should_update_thumbnails: bool,
    pub should_validate_models: bool,
    pub should_update_lighting: bool,
    pub should_update_clear_color: bool,
    // TODO: Add a mesh_to_refresh index? Option<folder, mesh>
    pub should_show_update: bool,
    pub new_release_tag: Option<String>,

    pub material_presets: Vec<MatlEntryData>,

    pub red_checkerboard: egui::TextureId,
    pub yellow_checkerboard: egui::TextureId,

    pub draw_skeletons: bool,
    pub draw_bone_names: bool,
    pub enable_helper_bones: bool,

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

    pub preferences: AppPreferences,
}

#[derive(PartialEq, Eq)]
pub enum ItemsToUpdate {
    None,
    One(usize),
    All,
}

#[derive(Default)]
pub struct UiState {
    // TODO: Add a changed flag and clear on save?
    // This would allow showing an indication for which files need to be saved.
    // TODO: Allow more than one open editor of each type?
    pub material_editor_open: bool,
    pub render_settings_open: bool,
    pub camera_settings_open: bool,
    pub stage_lighting_open: bool,
    pub preset_editor_open: bool,
    pub right_panel_tab: PanelTab,
    pub modl_editor_advanced_mode: bool,
    pub mesh_editor_advanced_mode: bool,
    pub log_window_open: bool,
    pub preferences_window_open: bool,

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
    pub selected_meshex_index: Option<usize>,

    pub selected_mesh_influences_index: Option<usize>,
    pub selected_mesh_attributes_index: Option<usize>,

    pub matl_preset_window_open: bool,
    pub selected_material_preset_index: usize,

    pub matl_editor: MatlEditorState,
    pub preset_editor: MatlEditorState,
    pub anim_editor: AnimEditorState,
    pub skel_editor: SkelEditorState,
    pub stage_lighting: StageLightingState,
}

#[derive(Default)]
pub struct SkelEditorState {
    pub mode: SkelMode,
}

#[derive(PartialEq, Eq)]
pub enum SkelMode {
    List,
    Hierarchy,
}

impl Default for SkelMode {
    fn default() -> Self {
        Self::List
    }
}

#[derive(Default)]
pub struct MatlEditorState {
    pub advanced_mode: bool,
    pub selected_material_index: usize,
    pub is_editing_material_label: bool,
    pub hovered_material_index: Option<usize>,
}

#[derive(Default)]
pub struct StageLightingState {
    pub light: Option<PathBuf>,
    pub reflection_cube_map: Option<PathBuf>,
    pub color_grading_lut: Option<PathBuf>,
    pub chara_shpc: Option<PathBuf>,
    pub stage_shpc: Option<PathBuf>,
}

#[derive(PartialEq, Eq)]
pub enum AnimEditorTab {
    Editor,
    Graph,
}

impl Default for AnimEditorTab {
    fn default() -> Self {
        Self::Editor
    }
}

#[derive(Default)]
pub struct AnimEditorState {
    pub editor_tab: AnimEditorTab,
    pub selected_group_index: Option<usize>,
    pub selected_node_index: Option<usize>,
    pub selected_track_index: Option<usize>,
}

const ICON_SIZE: f32 = 18.0;
pub const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 40, 40);
pub const WARNING_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 210, 0);

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
    pub fn add_folder_to_workspace(&mut self, clear_workspace: bool) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            // Don't clear existing files if the user cancels the dialog.
            if clear_workspace {
                self.clear_workspace();
            }

            // TODO: Check for duplicate folders?

            // Load recursively for nested folders like stages.
            let new_models = ssbh_wgpu::load_model_folders(&folder);

            self.animation_state
                .animations
                .extend(new_models.iter().enumerate().map(|(i, model)| {
                    if let Some(anim_index) =
                        model.anims.iter().position(|(f, _)| f == "model.nuanmb")
                    {
                        // The model.nuanmb always plays, so assign it automatically.
                        vec![AnimationSlot {
                            is_enabled: true,
                            animation: Some(AnimationIndex {
                                folder_index: self.models.len() + i,
                                anim_index,
                            }),
                        }]
                    } else {
                        // Add a dummy animation to prompt the user to select one.
                        vec![AnimationSlot::new()]
                    }
                }));

            // Only load new render models for better performance.
            // TODO: Handle this with models to update?
            self.render_models.extend(new_models.iter().map(|model| {
                RenderModel::from_folder(
                    &self.render_state.device,
                    &self.render_state.queue,
                    model,
                    &self.render_state.shared_data,
                )
            }));

            if self.preferences.autohide_expressions {
                self.hide_expressions();
            }

            self.models.extend(new_models);
            sort_files(&mut self.models);

            // TODO: Only validate the models that were added?
            self.should_validate_models = true;
            self.should_update_thumbnails = true;
        }
    }

    pub fn reload_workspace(&mut self) {
        // This also reloads animations since animations are stored as indices.
        for model in &mut self.models {
            *model = ModelFolder::load_folder(&model.folder_name);
        }
        sort_files(&mut self.models);

        self.models_to_update = ItemsToUpdate::All;
        self.should_update_thumbnails = true;
        self.should_validate_models = true;
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
        let patterns: [&str; 36] = [
            "_bink",
            "_low",
            "appeal",
            "attack",
            "blink",
            "bound",
            "breath",
            "camerahit",
            "capture",
            "catch",
            "cliff",
            "damage",
            "down",
            "escape",
            "fall",
            "final",
            "flip",
            "fura",
            "half",
            "harf",
            "heavy",
            "hot",
            "inkmesh",
            "laugh",
            "open_mouth",
            "ottotto",
            "ouch",
            "pattern",
            "result",
            "result",
            "smalleye",
            "sorori",
            "steppose",
            "swell",
            "talk",
            "voice",
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

    pub fn write_state_to_disk(&self, update_check_time: DateTime<Utc>) {
        // TODO: Handle errors and write to log file?
        // TODO: Use json to support more settings.
        let path = last_update_check_file();
        std::fs::write(path, update_check_time.to_string()).unwrap();

        self.preferences.write_to_file();
    }

    pub fn viewport_rect(&self, width: u32, height: u32, scale_factor: f32) -> [u32; 4] {
        // Calculate [origin x, origin y, width, height]
        // ssbh_wgpu expects physical instead of logical pixels.
        let f = |x| (x * scale_factor) as u32;
        let left = self
            .render_state
            .viewport_left
            .map(f)
            .unwrap_or(0)
            .clamp(0, width.saturating_sub(1));
        let right = self
            .render_state
            .viewport_right
            .map(f)
            .unwrap_or(width)
            .clamp(0, width.saturating_sub(1));
        let top = self
            .render_state
            .viewport_top
            .map(f)
            .unwrap_or(0)
            .clamp(0, height.saturating_sub(1));
        let bottom = self
            .render_state
            .viewport_bottom
            .map(f)
            .unwrap_or(height)
            .clamp(0, height.saturating_sub(1));
        let width = right.saturating_sub(left).clamp(1, width - left);
        let height = bottom.saturating_sub(top).clamp(1, height - top);
        [left, top, width, height]
    }
}

impl SsbhApp {
    pub fn update(&mut self, ctx: &Context) {
        // Set the region for the 3D viewport to reduce overdraw.
        self.render_state.viewport_top = Some(
            egui::TopBottomPanel::top("top_panel")
                .show(ctx, |ui| self.menu_bar(ui))
                .response
                .rect
                .bottom(),
        );

        // Add windows here so they can overlap everything except the top panel.
        // We store some state in self to keep track of whether this should be left open.
        render_settings(
            ctx,
            &mut self.render_state.render_settings,
            &mut self.render_state.model_render_options,
            &mut self.render_state.skinning_settings,
            &mut self.ui_state.render_settings_open,
            &mut self.draw_skeletons,
            &mut self.draw_bone_names,
            &mut self.enable_helper_bones,
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

        self.should_update_lighting |= stage_lighting_window(
            ctx,
            &mut self.ui_state.stage_lighting_open,
            &mut self.ui_state.stage_lighting,
        );

        log_window(ctx, &mut self.ui_state.log_window_open);

        self.should_update_clear_color |= preferences_window(
            ctx,
            &mut self.preferences,
            &mut self.ui_state.preferences_window_open,
        );

        preset_editor(
            ctx,
            &mut self.ui_state,
            &mut self.material_presets,
            &self.default_thumbnails,
            self.render_state.shared_data.database(),
            self.red_checkerboard,
            self.yellow_checkerboard,
        );

        // Don't reopen the window once closed.
        if self.should_show_update {
            self.new_release_window(ctx);
        }

        self.should_validate_models |= self.file_editors(ctx);

        self.render_state.viewport_left = if self.show_left_panel {
            Some(
                SidePanel::left("left_panel")
                    .min_width(200.0)
                    .show(ctx, |ui| self.files_list(ui))
                    .response
                    .rect
                    .right(),
            )
        } else {
            None
        };

        self.render_state.viewport_bottom = if self.show_bottom_panel {
            Some(
                TopBottomPanel::bottom("bottom panel")
                    .show(ctx, |ui| self.animation_and_log(ui))
                    .response
                    .rect
                    .top(),
            )
        } else {
            None
        };

        self.render_state.viewport_right = if self.show_right_panel {
            Some(
                SidePanel::right("right panel")
                    .min_width(450.0)
                    .show(ctx, |ui| self.right_panel(ctx, ui))
                    .response
                    .rect
                    .left(),
            )
        } else {
            None
        };
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

    fn file_editors(&mut self, ctx: &Context) -> bool {
        let display_name = |folder: &str, name: &str| {
            format!(
                "{}/{}",
                Path::new(folder)
                    .file_name()
                    .map(|f| f.to_string_lossy())
                    .unwrap_or_default(),
                name
            )
        };

        let mut file_changed = false;

        // The functions would take an additional ui parameter.
        // TODO: Use some sort of trait to clean up repetitive code?
        // TODO: Passing display_name is redundant?
        if let Some(folder_index) = self.ui_state.selected_folder_index {
            if let Some(model) = self.models.get_mut(folder_index) {
                if let Some(skel_index) = self.ui_state.selected_skel_index {
                    if let Some((name, Ok(skel))) = model.skels.get_mut(skel_index) {
                        let (open, changed) = skel_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            skel,
                            &mut self.ui_state.skel_editor,
                        );
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_skel_index = None;
                        }
                    }
                }

                if let Some(mesh_index) = self.ui_state.selected_mesh_index {
                    if let Some((name, Ok(mesh))) = model.meshes.get_mut(mesh_index) {
                        let validation_errors = self
                            .validation_errors
                            .get(folder_index)
                            .map(|v| v.mesh_errors.as_slice())
                            .unwrap_or_default();

                        let (open, changed) = mesh_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            mesh,
                            find_file(&model.skels, "model.nusktb"),
                            validation_errors,
                            &mut self.ui_state,
                        );
                        // TODO: Reload the specified render model if necessary?
                        // The mesh editor has no high frequency edits (sliders), so just reload on any change?
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_mesh_index = None;
                        }

                        // TODO: Update pipeline depth settings on change.
                    }
                }

                // TODO: Make all this code a function?
                if let Some(matl_index) = self.ui_state.selected_matl_index {
                    if let Some((name, Ok(matl))) = model.matls.get_mut(matl_index) {
                        // TODO: Make this a method to simplify arguments.
                        let validation_errors = self
                            .validation_errors
                            .get(folder_index)
                            .map(|v| v.matl_errors.as_slice())
                            .unwrap_or_default();

                        let (open, changed) = matl_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            &mut self.ui_state,
                            matl,
                            find_file_mut(&mut model.modls, "model.numdlb"),
                            validation_errors,
                            self.thumbnails.get(folder_index).unwrap_or(&Vec::new()),
                            &self.default_thumbnails,
                            self.render_state.shared_data.database(),
                            &mut self.material_presets,
                            self.red_checkerboard,
                            self.yellow_checkerboard,
                        );
                        // TODO: shader error checkboards don't update properly.
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_matl_index = None;
                        }

                        // Update on change to avoid costly state changes every frame.
                        if changed {
                            if let Some(render_model) = self.render_models.get_mut(folder_index) {
                                // TODO: Is it worth optimizing this to only effect certain materials?
                                // Only the model.numatb is rendered in the viewport for now.
                                // TODO: Move rendering code out of app.rs.
                                if name == "model.numatb" {
                                    render_model.recreate_materials(
                                        &self.render_state.device,
                                        &matl.entries,
                                        &self.render_state.shared_data,
                                    );
                                    // TODO: Also reassign materials?
                                }
                            }
                        }
                    }
                }

                if let Some(modl_index) = self.ui_state.selected_modl_index {
                    if let Some((name, Ok(modl))) = model.modls.get_mut(modl_index) {
                        // TODO: Make a WindowResponse struct?
                        let matl = find_file(&model.matls, "model.numatb");
                        let (open, changed) = modl_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            modl,
                            find_file(&model.meshes, "model.numshb"),
                            matl,
                            &mut self.ui_state.modl_editor_advanced_mode,
                        );
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_modl_index = None;
                        }

                        if changed {
                            if let Some(render_model) = self.render_models.get_mut(folder_index) {
                                render_model.reassign_materials(modl, matl);
                            }
                        }
                    }
                }

                if let Some(hlpb_index) = self.ui_state.selected_hlpb_index {
                    if let Some((name, Ok(hlpb))) = model.hlpbs.get_mut(hlpb_index) {
                        let (open, changed) = hlpb_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            hlpb,
                            find_file(&model.skels, "model.nusktb"),
                        );
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_hlpb_index = None;
                        }

                        if changed {
                            // Reapply the animation constraints in the viewport.
                            self.animation_state.should_update_animations = true;
                        }
                    }
                }

                if let Some(adj_index) = self.ui_state.selected_adj_index {
                    if let Some((name, Ok(adj))) = model.adjs.get_mut(adj_index) {
                        // TODO: Make this a method to simplify arguments.
                        let validation_errors = self
                            .validation_errors
                            .get(folder_index)
                            .map(|v| v.adj_errors.as_slice())
                            .unwrap_or_default();

                        let (open, changed) = adj_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            adj,
                            find_file(&model.meshes, "model.numshb"),
                            validation_errors,
                        );
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_adj_index = None;
                        }
                    }
                }

                if let Some(anim_index) = self.ui_state.selected_anim_index {
                    if let Some((name, Ok(anim))) = model.anims.get_mut(anim_index) {
                        let (open, changed) = anim_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            anim,
                            &mut self.ui_state.anim_editor,
                        );
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_anim_index = None;
                        }

                        if changed {
                            // Reapply the animations in the viewport.
                            self.animation_state.should_update_animations = true;
                        }
                    }
                }

                if let Some(meshex_index) = self.ui_state.selected_meshex_index {
                    if let Some((name, Ok(meshex))) = model.meshexes.get_mut(meshex_index) {
                        let (open, changed) = meshex_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &model.folder_name,
                            name,
                            meshex,
                            find_file(&model.meshes, "model.numshb"),
                        );
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_meshex_index = None;
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

        file_changed
    }

    fn animation_bar(&mut self, ui: &mut Ui) {
        let final_frame_index = self.max_final_frame_index();

        // TODO: Find a better layout for this.
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Speed");
                ui.add(
                    DragValue::new(&mut self.animation_state.playback_speed)
                        .min_decimals(2)
                        .speed(0.01)
                        .clamp_range(0.25..=2.0),
                );

                // TODO: Custom checkbox widget so label is on the left side.
                ui.checkbox(&mut self.animation_state.should_loop, "Loop");
            });
            ui.horizontal_centered(|ui| {
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
                    self.animation_state.should_update_animations = true;
                }

                // Use a separate widget from the slider value to force the size.
                // This reduces the chances of the widget resizing during animations.

                let size = [60.0, 30.0];
                if self.animation_state.is_playing {
                    // Nest these conditions to avoid displaying both "Pause" and "Play" at once.
                    if ui.add_sized(size, Button::new("Pause")).clicked() {
                        self.animation_state.is_playing = false;
                    }
                } else if ui.add_sized(size, Button::new("Play")).clicked() {
                    self.animation_state.is_playing = true;
                }

                if ui
                    .add_sized(
                        [60.0, 20.0],
                        egui::DragValue::new(&mut self.animation_state.current_frame)
                            .clamp_range(0.0..=final_frame_index),
                    )
                    .changed()
                {
                    // Manually trigger an update in case the playback is paused.
                    self.animation_state.should_update_animations = true;
                }
            });
        });
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
                let mut folder_to_remove = None;

                // TODO: Is it worth showing a folder hierarchy instead of hiding empty folders?
                for (folder_index, (model, validation)) in self
                    .models
                    .iter_mut()
                    .zip(self.validation_errors.iter())
                    .enumerate()
                    .filter(|(_, (model, _))| !model.is_empty())
                {
                    CollapsingHeader::new(folder_display_name(model).to_string_lossy())
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
                                Some("model.numshb"),
                                &validation.mesh_errors,
                            );

                            list_files(
                                ui,
                                &model.skels,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_skel_index,
                                required_file("model.nusktb"),
                                Some("model.nusktb"),
                                &validation.skel_errors,
                            );

                            list_files(
                                ui,
                                &model.hlpbs,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_hlpb_index,
                                None,
                                Some("model.nuhlpb"),
                                &validation.hlpb_errors,
                            );

                            list_files(
                                ui,
                                &model.matls,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_matl_index,
                                required_file("model.numatb"),
                                Some("model.numatb"),
                                &validation.matl_errors,
                            );

                            list_files(
                                ui,
                                &model.modls,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_modl_index,
                                required_file("model.numdlb"),
                                Some("model.numdlb"),
                                &validation.modl_errors,
                            );

                            list_files(
                                ui,
                                &model.adjs,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_adj_index,
                                None,
                                Some("model.adjb"),
                                &validation.adj_errors,
                            );

                            list_files(
                                ui,
                                &model.anims,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_anim_index,
                                None,
                                None,
                                &validation.anim_errors,
                            );

                            // TODO: Is the model.numshexb required?
                            list_files(
                                ui,
                                &model.meshexes,
                                folder_index,
                                &mut self.ui_state.selected_folder_index,
                                &mut self.ui_state.selected_meshex_index,
                                None,
                                Some("model.numshexb"),
                                &validation.meshex_errors,
                            );

                            // TODO: Make a function for listing nutexbs..
                            // Show missing textures required by the matl.
                            for e in &validation.matl_errors {
                                if let MatlValidationError::MissingTexture { nutexb, .. } = e {
                                    missing_nutexb(ui, nutexb);
                                }
                            }
                            for (i, (file, _)) in model.nutexbs.iter().enumerate() {
                                // TODO: Avoid collect?
                                let validation_errors: Vec<_> = validation
                                    .nutexb_errors
                                    .iter()
                                    .filter(|e| e.name() == file)
                                    .collect();

                                ui.horizontal(|ui| {
                                    // TODO: Show error icon on top of thumbnail?
                                    if !validation_errors.is_empty() {
                                        warning_icon(ui).on_hover_ui(|ui| {
                                            display_validation_errors(ui, &validation_errors);
                                        });
                                    } else if let Some(model_thumbnails) =
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
                        })
                        .header_response
                        .context_menu(|ui| {
                            if ui.button("Remove").clicked() {
                                ui.close_menu();
                                folder_to_remove = Some(folder_index);
                            }
                        });
                }

                if let Some(folder_to_remove) = folder_to_remove {
                    if self.models.get(folder_to_remove).is_some() {
                        self.models.remove(folder_to_remove);
                    }
                    if self.render_models.get(folder_to_remove).is_some() {
                        self.render_models.remove(folder_to_remove);
                    }
                }
            });
    }

    fn animation_and_log(&mut self, ui: &mut Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            self.animation_bar(ui);

            // The next layout needs to be min since it's nested inside a centered layout.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                // The layout is right to left, so add in reverse order.
                if let Some((level, message)) = LOGGER.messages.lock().unwrap().last() {
                    if ui.add_sized([60.0, 30.0], Button::new("Logs")).clicked() {
                        self.ui_state.log_window_open = true;
                    }

                    // Clicking the message also opens the log window.
                    let abbreviated_message =
                        message.get(..40).unwrap_or_default().to_string() + "...";
                    if ui
                        .add(egui::Label::new(abbreviated_message).sense(egui::Sense::click()))
                        .clicked()
                    {
                        self.ui_state.log_window_open = true;
                    }

                    log_level_icon(ui, level);
                }
            });
        });
    }

    fn right_panel(&mut self, ctx: &Context, ui: &mut Ui) {
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
                    self.add_folder_to_workspace(true);
                }

                if button(ui, format!("Add Folder to Workspace...    {ctrl_shift} O")).clicked() {
                    ui.close_menu();
                    self.add_folder_to_workspace(false);
                }

                if button(ui, format!("Reload Workspace    {ctrl} R")).clicked() {
                    ui.close_menu();
                    self.reload_workspace();
                }

                if button(ui, "Clear Workspace".to_owned()).clicked() {
                    ui.close_menu();
                    self.clear_workspace();
                }
            });

            // TODO: Add icons?
            egui::menu::menu_button(ui, "Menu", |ui| {
                if ui.button("Render Settings").clicked() {
                    ui.close_menu();
                    self.ui_state.render_settings_open = true;
                }

                if ui.button("Camera Settings").clicked() {
                    ui.close_menu();
                    self.ui_state.camera_settings_open = true;
                }

                if ui.button("Stage Lighting").clicked() {
                    ui.close_menu();
                    self.ui_state.stage_lighting_open = true;
                }

                if ui.button("Material Presets").clicked() {
                    ui.close_menu();
                    self.ui_state.preset_editor_open = true;
                }

                if ui.button("Preferences").clicked() {
                    ui.close_menu();
                    self.ui_state.preferences_window_open = true;
                }
            });

            egui::menu::menu_button(ui, "Meshes", |ui| {
                if ui.button("Hide Expressions").clicked() {
                    ui.close_menu();
                    self.hide_expressions();
                }
            });

            egui::menu::menu_button(ui, "View", |ui| {
                ui.checkbox(&mut self.show_left_panel, "Left Panel");
                ui.checkbox(&mut self.show_right_panel, "Right Panel");
                ui.checkbox(&mut self.show_bottom_panel, "Bottom Panel");
            });

            egui::menu::menu_button(ui, "Help", |ui| {
                if ui.button("Wiki").clicked() {
                    ui.close_menu();
                    let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki";
                    if let Err(e) = open::that(link) {
                        log::error!("Failed to open {link}: {e}");
                    }
                }

                if ui.button("Discussion Forum").clicked() {
                    ui.close_menu();
                    let link = "https://github.com/ScanMountGoat/ssbh_editor/discussions";
                    if let Err(e) = open::that(link) {
                        log::error!("Failed to open {link}: {e}");
                    }
                }

                if ui.button("Report Issue").clicked() {
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

fn folder_display_name(model: &ModelFolder) -> PathBuf {
    // Get enough components to differentiate folder paths.
    // fighter/mario/motion/body/c00 -> mario/motion/body/c00
    Path::new(&model.folder_name)
        .components()
        .rev()
        .take(4)
        .fold(PathBuf::new(), |acc, x| Path::new(&x).join(acc))
}

fn find_file<'a, T>(files: &'a [(String, FileResult<T>)], name: &str) -> Option<&'a T> {
    files
        .iter()
        .find(|(f, _)| f == name)
        .and_then(|(_, m)| m.as_ref().ok())
}

fn find_file_mut<'a, T>(files: &'a mut [(String, FileResult<T>)], name: &str) -> Option<&'a mut T> {
    files
        .iter_mut()
        .find(|(f, _)| f == name)
        .and_then(|(_, m)| m.as_mut().ok())
}

fn list_files<T, E: std::fmt::Display>(
    ui: &mut Ui,
    files: &[(String, FileResult<T>)],
    folder_index: usize,
    selected_folder_index: &mut Option<usize>,
    selected_file_index: &mut Option<usize>,
    required_file: Option<&'static str>,
    validation_file: Option<&'static str>,
    validation_errors: &[E],
) {
    // TODO: Should this be a grid instead?
    for (i, (name, file)) in files.iter().enumerate() {
        ui.horizontal(|ui| {
            match file {
                Ok(_) => {
                    // Assume only the required file is validated for now.
                    // This excludes files like metamon_model.numatb.
                    if !validation_errors.is_empty() && Some(name.as_str()) == validation_file {
                        // TODO: Show top few errors and ... N others on hover?
                        // TODO: Display the validation errors as a separate window on click?
                        warning_icon(ui).on_hover_ui(|ui| {
                            display_validation_errors(ui, validation_errors);
                        });
                    } else {
                        // TODO: This doesn't have the same size as the others?
                        empty_icon(ui);
                    }
                    if ui.button(name).clicked() {
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
    ui.horizontal(|ui| {
        missing_icon(ui);
        ui.add_enabled(false, Button::new(RichText::new(name).strikethrough()));
    })
    .response
    .on_hover_text(format!("Missing required file {name}."));
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

pub fn empty_icon(ui: &mut Ui) {
    ui.allocate_space(egui::Vec2::new(ICON_SIZE, ICON_SIZE));
}

pub fn missing_icon(ui: &mut Ui) {
    ui.add_sized(
        [ICON_SIZE, ICON_SIZE],
        Label::new(RichText::new("⚠").size(ICON_SIZE)),
    );
}

pub fn warning_icon(ui: &mut Ui) -> Response {
    ui.add_sized(
        [ICON_SIZE, ICON_SIZE],
        Label::new(
            RichText::new("⚠")
                .strong()
                .color(WARNING_COLOR)
                .size(ICON_SIZE),
        ),
    )
}

pub fn display_validation_errors<E: std::fmt::Display>(ui: &mut Ui, errors: &[E]) {
    for error in errors {
        ui.horizontal(|ui| {
            // TODO: Add severity levels?
            warning_icon(ui);
            ui.label(format!("{}", error));
        });
    }
}

pub fn error_icon(ui: &mut Ui) -> Response {
    ui.add_sized(
        [ICON_SIZE, ICON_SIZE],
        Label::new(
            RichText::new("⚠")
                .strong()
                .color(ERROR_COLOR)
                .size(ICON_SIZE),
        ),
    )
}

fn mesh_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // Don't show non model folders like animation or texture folders.
    for (i, folder) in app
        .models
        .iter()
        .enumerate()
        .filter(|(_, folder)| is_model_folder(folder))
    {
        let id = ui.make_persistent_id("meshlist").with(i);

        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                if let Some(render_model) = app.render_models.get_mut(i) {
                    render_model.is_selected = ui
                        .add(EyeCheckBox::new(
                            &mut render_model.is_visible,
                            folder_display_name(folder).to_string_lossy(),
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
                            log_level_icon(ui, level);
                            // binrw formats backtraces, which isn't supported by egui font rendering.
                            // TODO: Avoid clone?
                            let clean_message = strip_ansi_escapes::strip(message)
                                .map(|m| String::from_utf8_lossy(&m).to_string())
                                .unwrap_or_else(|_| message.clone());
                            ui.label(clean_message);
                        });
                    }
                });
        });
}

fn log_level_icon(ui: &mut Ui, level: &log::Level) {
    match level {
        log::Level::Error => {
            error_icon(ui);
        }
        log::Level::Warn => {
            warning_icon(ui);
        }
        log::Level::Info => (),
        log::Level::Debug => (),
        log::Level::Trace => (),
    }
}

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
                    egui::DragValue::new(&mut camera_state.rotation_xyz_radians.x)
                        .speed(0.01)
                        .clamp_range(-2.0 * PI..=2.0 * PI),
                );
                ui.end_row();

                ui.label("Rotation Y");
                ui.add(
                    egui::DragValue::new(&mut camera_state.rotation_xyz_radians.y)
                        .speed(0.01)
                        .clamp_range(-2.0 * PI..=2.0 * PI),
                );
                ui.end_row();

                ui.label("FOV");
                ui.add(
                    egui::DragValue::new(&mut camera_state.fov_y_radians)
                        .speed(0.01)
                        .clamp_range(0.0..=2.0 * PI),
                );
                ui.end_row();

                if ui.button("Reset").clicked() {
                    *camera_state = CameraInputState::default();
                }
            });
        });
}

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
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Open render folder...").clicked() {
                        if let Some(folder) = FileDialog::new().pick_folder() {
                            // Attempt to load supported lighting files based on naming conventions.
                            // Users should select paths like "/stage/battlefield/normal/render/".
                            state.light = Some(folder.join("light").join("light00.nuanmb"));
                            state.reflection_cube_map =
                                Some(folder.join("reflection_cubemap.nutexb"));
                            state.color_grading_lut = folder
                                .parent()
                                .map(|p| p.join("lut").join("color_grading_lut.nutexb"));
                            changed = true;
                        }
                    }
                });
            });
            ui.separator();

            let path_label = |ui: &mut Ui, path: &Option<PathBuf>| match path {
                Some(path) => {
                    ui.label(path.file_name().and_then(|f| f.to_str()).unwrap_or(""))
                        .on_hover_ui(|ui| {
                            ui.add(Label::new(path.to_string_lossy()).wrap(false));
                        });
                }
                None => {
                    ui.label("");
                }
            };

            Grid::new("stage_lighting").show(ui, |ui| {
                // TODO: Make the files buttons to load corresponding editors?
                ui.label("Lighting");
                path_label(ui, &state.light);
                if ui.button("Select file...").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Lighting Anim", &["nuanmb"])
                        .pick_file()
                    {
                        state.light = Some(file);
                        changed = true;
                    };
                }
                ui.end_row();

                ui.label("Reflection Cube Map");
                path_label(ui, &state.reflection_cube_map);
                if ui.button("Select file...").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Cube Map Nutexb", &["nutexb"])
                        .pick_file()
                    {
                        state.reflection_cube_map = Some(file);
                        changed = true;
                    };
                };
                ui.end_row();

                ui.label("Color Grading LUT");
                path_label(ui, &state.color_grading_lut);
                if ui.button("Select file...").clicked() {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Color Grading LUT", &["nutexb"])
                        .pick_file()
                    {
                        state.color_grading_lut = Some(file);
                        changed = true;
                    };
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
