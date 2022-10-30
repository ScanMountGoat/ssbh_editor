use self::window::*;
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
    preferences::AppPreferences,
    validation::MatlValidationErrorKind,
    widgets::*,
    AnimationIndex, AnimationSlot, AnimationState, CameraInputState, FileChanged, FileResult,
    ModelFolderState, RenderState, Thumbnail,
};
use chrono::{DateTime, Utc};
use egui::{
    collapsing_header::CollapsingState, Button, CollapsingHeader, Context, DragValue, Label,
    Response, RichText, ScrollArea, SidePanel, TopBottomPanel, Ui, Window,
};
use log::{error, Log};
use once_cell::sync::Lazy;
use rfd::FileDialog;
use ssbh_data::matl_data::MatlEntryData;
use ssbh_wgpu::{ModelFolder, RenderModel};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

mod anim_list;
mod window;

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

    pub screenshot_to_render: Option<PathBuf>,
    pub animation_gif_to_render: Option<PathBuf>,
    pub animation_image_sequence_to_render: Option<PathBuf>,

    pub material_presets: Vec<MatlEntryData>,

    pub red_checkerboard: egui::TextureId,
    pub yellow_checkerboard: egui::TextureId,

    pub draw_skeletons: bool,
    pub draw_bone_names: bool,
    pub enable_helper_bones: bool,

    pub ui_state: UiState,
    // TODO: Is parallel list with models the best choice here?
    pub models: Vec<ModelFolderState>,
    pub render_models: Vec<RenderModel>,

    pub default_thumbnails: Vec<Thumbnail>,
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
    // TODO: Allow more than one open editor of each type?
    pub material_editor_open: bool,
    pub render_settings_open: bool,
    pub camera_settings_open: bool,
    pub stage_lighting_open: bool,
    pub preset_editor_open: bool,
    pub right_panel_tab: PanelTab,
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
    pub modl_editor: ModlEditorState,
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
pub struct ModlEditorState {
    pub advanced_mode: bool,
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
const ICON_TEXT_SIZE: f32 = 14.0;
pub const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 80, 80);
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
    pub fn add_folder_to_workspace_from_dialog(&mut self, clear_workspace: bool) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.add_folder_to_workspace(folder, clear_workspace);
        }
    }

    pub fn add_folder_to_workspace<P: AsRef<Path>>(&mut self, folder: P, clear_workspace: bool) {
        // Don't clear existing files if the user cancels the dialog.
        if clear_workspace {
            self.clear_workspace();
        }

        // TODO: Check for duplicate folders?

        // Load recursively for nested folders like stages.
        let mut new_models = ssbh_wgpu::load_model_folders(&folder);
        new_models.sort_by_key(|m| m.folder_name.clone());

        self.animation_state
            .animations
            .extend(new_models.iter().enumerate().map(|(i, model)| {
                if let Some(anim_index) = model.anims.iter().position(|(f, _)| f == "model.nuanmb")
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

        self.models
            .extend(new_models.into_iter().map(ModelFolderState::from_model));
        self.sort_files();

        // TODO: Only validate the models that were added?
        self.should_validate_models = true;
        self.should_update_thumbnails = true;

        // Only keep track of a limited number of recent folders.
        let new_folder = folder.as_ref().to_string_lossy().to_string();
        if let Some(i) = self.preferences.recent_folders.iter().position(|f| f == &new_folder) {
            self.preferences.recent_folders.remove(i);
        }
        // Move a folder to the front if it was seen before.
        self.preferences.recent_folders.insert(0, new_folder);
        self.preferences.recent_folders.truncate(10);
    }

    pub fn reload_workspace(&mut self) {
        // This also reloads animations since animations are stored as indices.
        for model in &mut self.models {
            // Make sure the ModelFolder is loaded first.
            model.model = ModelFolder::load_folder(&model.model.folder_name);
            model.changed = FileChanged::from_model(&model.model);
        }
        self.sort_files();

        self.models_to_update = ItemsToUpdate::All;
        self.should_update_thumbnails = true;
        self.should_validate_models = true;
    }

    pub fn clear_workspace(&mut self) {
        self.models = Vec::new();
        self.render_models = Vec::new();
        self.animation_state.animations = Vec::new();
        // TODO: Reset selected indices?
        // TODO: Is there an easy way to write this?
    }

    fn sort_files(&mut self) {
        // Don't sort the files themselves so render models and animations stay in sync.
        for model in &mut self.models {
            // Sort by file name for consistent ordering in the UI.
            model.model.adjs.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
            model.model.anims.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
            model.model.matls.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
            model.model.meshes.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
            model.model.modls.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
            model.model.nutexbs.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
            model.model.skels.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
        }
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
                        if name.contains(pattern_exception) {
                            continue 'pattern_search;
                        }
                    }

                    //Make all other expressions invisible
                    if name.contains(pattern) {
                        mesh.is_visible = false;
                    }
                }
            }
        }
    }

    pub fn write_state_to_disk(&self, update_check_time: DateTime<Utc>) {
        let path = last_update_check_file();
        if let Err(e) = std::fs::write(&path, update_check_time.to_string()) {
            error!("Failed to write update check time to {path:?}: {e}");
        }

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
        // This can be set by the mesh list and mesh editor.
        // Clear every frame so both sources can set is_selected to true.
        self.clear_selected_meshes();

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
        render_settings_window(
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

        camera_settings_window(
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
                    .default_width(200.0)
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
                    .default_width(450.0)
                    .show(ctx, |ui| self.right_panel(ctx, ui))
                    .response
                    .rect
                    .left(),
            )
        } else {
            None
        };
    }

    fn clear_selected_meshes(&mut self) {
        for model in &mut self.render_models {
            model.is_selected = false;
            for mesh in &mut model.meshes {
                mesh.is_selected = false;
            }
        }
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
        let mut file_changed = false;

        // TODO: Use some sort of trait to clean up repetitive code?
        // The functions would take an additional ui parameter.
        if let Some(folder_index) = self.ui_state.selected_folder_index {
            if let Some(model) = self.models.get_mut(folder_index) {
                if let Some(skel_index) = self.ui_state.selected_skel_index {
                    if let Some((name, Ok(skel))) = model.model.skels.get_mut(skel_index) {
                        let (open, changed) = skel_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            skel,
                            &mut self.ui_state.skel_editor,
                        );
                        // TODO: Create window response struct that also tracks saving to reset changed.
                        // TODO: window_response.set_changed(&mut model.changed.skels)?
                        model.changed.skels[skel_index] |= changed;
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_skel_index = None;
                        }
                    }
                }

                if let Some(mesh_index) = self.ui_state.selected_mesh_index {
                    if let Some((name, Ok(mesh))) = model.model.meshes.get_mut(mesh_index) {
                        let (open, changed) = mesh_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            mesh,
                            &mut self.render_models.get_mut(folder_index),
                            find_file(&model.model.skels, "model.nusktb"),
                            &model.validation.mesh_errors,
                            &mut self.ui_state,
                        );
                        model.changed.meshes[mesh_index] |= changed;
                        file_changed |= changed;

                        if !open {
                            // Close the window.
                            self.ui_state.selected_mesh_index = None;
                        }

                        if changed {
                            // The mesh editor has no high frequency edits (sliders), so reload on any change.
                            // TODO: Add a mesh to update field instead with (folder, mesh)?
                            self.models_to_update = ItemsToUpdate::One(folder_index);
                        }

                        // TODO: Update pipeline depth settings on change.
                    }
                }

                // TODO: Make all this code a function?
                if let Some(matl_index) = self.ui_state.selected_matl_index {
                    if let Some((name, Ok(matl))) = model.model.matls.get_mut(matl_index) {
                        let response = matl_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            &mut self.ui_state,
                            matl,
                            find_file_mut(&mut model.model.modls, "model.numdlb"),
                            &model.validation.matl_errors,
                            &model.thumbnails,
                            &self.default_thumbnails,
                            self.render_state.shared_data.database(),
                            &mut self.material_presets,
                            self.red_checkerboard,
                            self.yellow_checkerboard,
                        );
                        // TODO: This modifies the model.numdlb when renaming materials.
                        response.set_changed(&mut model.changed.matls[matl_index]);
                        file_changed |= response.changed;

                        if !response.open {
                            // Close the window.
                            self.ui_state.selected_matl_index = None;
                        }

                        // Update on change to avoid costly state changes every frame.
                        if response.changed {
                            if let Some(render_model) = self.render_models.get_mut(folder_index) {
                                // Only the model.numatb is rendered in the viewport for now.
                                // TODO: Move rendering code out of app.rs.
                                if name == "model.numatb" {
                                    render_model.recreate_materials(
                                        &self.render_state.device,
                                        &matl.entries,
                                        &self.render_state.shared_data,
                                    );
                                    if let Some(modl) =
                                        find_file(&model.model.modls, "model.numdlb")
                                    {
                                        // Reassign materials in case material or shader labels changed.
                                        // This is necessary for error checkerboards to display properly.
                                        render_model.reassign_materials(modl, Some(matl))
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(modl_index) = self.ui_state.selected_modl_index {
                    if let Some((name, Ok(modl))) = model.model.modls.get_mut(modl_index) {
                        let matl = find_file(&model.model.matls, "model.numatb");
                        let response = modl_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            modl,
                            find_file(&model.model.meshes, "model.numshb"),
                            matl,
                            &model.validation.modl_errors,
                            &mut self.ui_state.modl_editor,
                        );
                        response.set_changed(&mut model.changed.modls[modl_index]);
                        file_changed |= response.changed;

                        if !response.open {
                            // Close the window.
                            self.ui_state.selected_modl_index = None;
                        }

                        if response.changed {
                            if let Some(render_model) = self.render_models.get_mut(folder_index) {
                                render_model.reassign_materials(modl, matl);
                            }
                        }
                    }
                }

                if let Some(hlpb_index) = self.ui_state.selected_hlpb_index {
                    if let Some((name, Ok(hlpb))) = model.model.hlpbs.get_mut(hlpb_index) {
                        let response = hlpb_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            hlpb,
                            find_file(&model.model.skels, "model.nusktb"),
                        );
                        response.set_changed(&mut model.changed.hlpbs[hlpb_index]);
                        file_changed |= response.changed;

                        if !response.open {
                            // Close the window.
                            self.ui_state.selected_hlpb_index = None;
                        }

                        if response.changed {
                            // Reapply the animation constraints in the viewport.
                            self.animation_state.should_update_animations = true;
                        }
                    }
                }

                if let Some(adj_index) = self.ui_state.selected_adj_index {
                    if let Some((name, Ok(adj))) = model.model.adjs.get_mut(adj_index) {
                        let response = adj_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            adj,
                            find_file(&model.model.meshes, "model.numshb"),
                            &model.validation.adj_errors,
                        );
                        response.set_changed(&mut model.changed.adjs[adj_index]);
                        file_changed |= response.changed;

                        if !response.open {
                            // Close the window.
                            self.ui_state.selected_adj_index = None;
                        }
                    }
                }

                if let Some(anim_index) = self.ui_state.selected_anim_index {
                    if let Some((name, Ok(anim))) = model.model.anims.get_mut(anim_index) {
                        let response = anim_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            anim,
                            &mut self.ui_state.anim_editor,
                        );
                        response.set_changed(&mut model.changed.anims[anim_index]);
                        file_changed |= response.changed;

                        if !response.open {
                            // Close the window.
                            self.ui_state.selected_anim_index = None;
                        }

                        if response.changed {
                            // Reapply the animations in the viewport.
                            self.animation_state.should_update_animations = true;
                        }
                    }
                }

                if let Some(meshex_index) = self.ui_state.selected_meshex_index {
                    if let Some((name, Ok(meshex))) = model.model.meshexes.get_mut(meshex_index) {
                        let response = meshex_editor(
                            ctx,
                            &model.model.folder_name,
                            name,
                            meshex,
                            find_file(&model.model.meshes, "model.numshb"),
                        );
                        response.set_changed(&mut model.changed.meshexes[meshex_index]);
                        file_changed |= response.changed;

                        if !response.open {
                            // Close the window.
                            self.ui_state.selected_meshex_index = None;
                        }
                    }
                }

                if let Some(nutexb_index) = self.ui_state.selected_nutexb_index {
                    if let Some((name, Ok(nutexb))) = model.model.nutexbs.get(nutexb_index) {
                        if !nutexb_viewer(
                            ctx,
                            &folder_editor_title(&model.model.folder_name, name),
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

                for (folder_index, model) in self
                    .models
                    .iter_mut()
                    .enumerate()
                    .filter(|(_, model)| !model.model.is_empty())
                {
                    // TODO: Use folder icons for open vs closed.
                    CollapsingHeader::new(folder_display_name(&model.model).to_string_lossy())
                        .id_source(format!("folder.{}", folder_index))
                        .default_open(true)
                        .show(ui, |ui| {
                            show_folder_files(&mut self.ui_state, model, ui, folder_index);
                        })
                        .header_response
                        .on_hover_text(&model.model.folder_name)
                        .context_menu(|ui| {
                            // Use "Remove" since this doesn't delete the folder on disk.
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
                self.show_most_recent_log_message(ui);
            });
        });
    }

    fn show_most_recent_log_message(&mut self, ui: &mut Ui) {
        // The layout is right to left, so add in reverse order.
        if let Some((level, message)) = LOGGER.messages.lock().unwrap().last() {
            if ui.add_sized([60.0, 30.0], Button::new("Logs")).clicked() {
                self.ui_state.log_window_open = true;
            }

            // Clicking the message also opens the log window.
            let abbreviated_message = message.get(..40).unwrap_or_default().to_string() + "...";
            if ui
                .add(egui::Label::new(abbreviated_message).sense(egui::Sense::click()))
                .clicked()
            {
                self.ui_state.log_window_open = true;
            }

            log_level_icon(ui, level);
        }
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
            // TODO: Improve alignment of menu options.
            ui.menu_button("File", |ui| {
                let button = |ui: &mut Ui, text: &str| ui.add(Button::new(text).wrap(false));

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

                if button(ui, &format!("Open Folder...    {ctrl} O")).clicked() {
                    ui.close_menu();
                    if let Some(folder) = FileDialog::new().pick_folder() {
                        self.add_folder_to_workspace(folder, true);
                    }
                }

                // TODO: Find a cleaner way to write this.
                // TODO: Add an option to clear the recently opened?
                let mut recent = None;
                ui.menu_button("Open Recent Folder", |ui| {
                    for folder in &self.preferences.recent_folders {
                        if button(ui, folder).clicked() {
                            ui.close_menu();
                            recent = Some(folder.clone());
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recently Opened").clicked() {
                        self.preferences.recent_folders.clear();
                    }
                });
                if let Some(recent) = recent {
                    self.add_folder_to_workspace(Path::new(&recent), true);
                }
                ui.separator();

                if button(ui, &format!("Add Folder to Workspace...    {ctrl_shift} O")).clicked() {
                    ui.close_menu();
                    if let Some(folder) = FileDialog::new().pick_folder() {
                        self.add_folder_to_workspace(folder, false);
                    }
                }

                // TODO: Find a cleaner way to write this.
                let mut recent = None;
                ui.menu_button("Add Recent Folder to Workspace", |ui| {
                    for folder in &self.preferences.recent_folders {
                        if button(ui, folder).clicked() {
                            ui.close_menu();
                            recent = Some(folder.clone());
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recently Opened").clicked() {
                        self.preferences.recent_folders.clear();
                    }
                });
                if let Some(recent) = recent {
                    self.add_folder_to_workspace(Path::new(&recent), false);
                }
                ui.separator();

                if button(ui, &format!("Reload Workspace    {ctrl} R")).clicked() {
                    ui.close_menu();
                    self.reload_workspace();
                }

                if button(ui, "Clear Workspace").clicked() {
                    ui.close_menu();
                    self.clear_workspace();
                }
            });

            // TODO: Add icons?
            ui.menu_button("Menu", |ui| {
                if ui.button("Render Settings").clicked() {
                    ui.close_menu();
                    self.ui_state.render_settings_open = true;
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

            ui.menu_button("Viewport", |ui| {
                if ui.button("Camera Settings").clicked() {
                    ui.close_menu();
                    self.ui_state.camera_settings_open = true;
                }

                if ui.button("Save Screenshot...").clicked() {
                    ui.close_menu();
                    if let Some(file) = FileDialog::new()
                        .add_filter("Image", &["png", "jpg", "tif", "bmp"])
                        .save_file()
                    {
                        self.screenshot_to_render = Some(file);
                    }
                }

                ui.menu_button("Render Animation", |ui| {
                    if ui
                        .add(Button::new("Render to Image Sequence...").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();
                        if let Some(file) = FileDialog::new()
                            .add_filter("Image", &["png", "jpg", "tif", "bmp"])
                            .save_file()
                        {
                            self.animation_image_sequence_to_render = Some(file);
                        }
                    }

                    if ui
                        .add(Button::new("Render to GIF...").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();
                        if let Some(file) =
                            FileDialog::new().add_filter("GIF", &["gif"]).save_file()
                        {
                            self.animation_gif_to_render = Some(file);
                        }
                    }
                });
            });

            ui.menu_button("Meshes", |ui| {
                if ui.button("Show All").clicked() {
                    ui.close_menu();

                    for model in &mut self.render_models {
                        model.is_visible = true;
                        for mesh in &mut model.meshes {
                            mesh.is_visible = true;
                        }
                    }
                }

                if ui.button("Hide All").clicked() {
                    ui.close_menu();

                    for model in &mut self.render_models {
                        model.is_visible = false;
                        for mesh in &mut model.meshes {
                            mesh.is_visible = false;
                        }
                    }
                }

                if ui.button("Hide Expressions").clicked() {
                    ui.close_menu();
                    self.hide_expressions();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_left_panel, "Left Panel");
                ui.checkbox(&mut self.show_right_panel, "Right Panel");
                ui.checkbox(&mut self.show_bottom_panel, "Bottom Panel");
            });

            ui.menu_button("Help", |ui| {
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

fn show_folder_files(
    ui_state: &mut UiState,
    model: &mut ModelFolderState,
    ui: &mut Ui,
    folder_index: usize,
) {
    // Avoid a confusing missing file error for animation or texture folders.
    let is_model = model.is_model_folder();
    let required_file = |name| if is_model { Some(name) } else { None };
    // Clicking a file opens the corresponding editor.
    // Set selected index so the editor remains open for the file.
    // TODO: Should the index be cleared when reloading models?
    list_files(
        ui,
        &model.model.meshes,
        &model.changed.meshes,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_mesh_index,
        required_file("model.numshb"),
        Some("model.numshb"),
        &model.validation.mesh_errors,
    );
    list_files(
        ui,
        &model.model.skels,
        &model.changed.skels,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_skel_index,
        required_file("model.nusktb"),
        Some("model.nusktb"),
        &model.validation.skel_errors,
    );
    list_files(
        ui,
        &model.model.hlpbs,
        &model.changed.hlpbs,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_hlpb_index,
        None,
        Some("model.nuhlpb"),
        &model.validation.hlpb_errors,
    );
    list_files(
        ui,
        &model.model.matls,
        &model.changed.matls,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_matl_index,
        required_file("model.numatb"),
        Some("model.numatb"),
        &model.validation.matl_errors,
    );
    list_files(
        ui,
        &model.model.modls,
        &model.changed.modls,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_modl_index,
        required_file("model.numdlb"),
        Some("model.numdlb"),
        &model.validation.modl_errors,
    );
    list_files(
        ui,
        &model.model.adjs,
        &model.changed.adjs,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_adj_index,
        None,
        Some("model.adjb"),
        &model.validation.adj_errors,
    );
    list_files(
        ui,
        &model.model.anims,
        &model.changed.anims,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_anim_index,
        None,
        None,
        &model.validation.anim_errors,
    );
    // TODO: Is the model.numshexb required?
    list_files(
        ui,
        &model.model.meshexes,
        &model.changed.meshexes,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_meshex_index,
        None,
        Some("model.numshexb"),
        &model.validation.meshex_errors,
    );
    // TODO: Create a single function that takes thumbnails?
    list_nutexb_files(
        ui,
        model,
        folder_index,
        &mut ui_state.selected_folder_index,
        &mut ui_state.selected_nutexb_index,
    );
}

// TODO: Move path formatting to its own module?
pub fn folder_editor_title(folder_name: &str, file_name: &str) -> String {
    // Show a simplified version of the path.
    // fighter/mario/motion/body/c00/model.numatb -> c00/model.numatb
    format!(
        "{}/{}",
        Path::new(folder_name)
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default(),
        file_name
    )
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
    changed: &[bool],
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
                    // TODO: Add file specific icons.
                    ui.add_sized(
                        [ICON_SIZE, ICON_SIZE],
                        Label::new(RichText::new("🗋").size(ICON_TEXT_SIZE)),
                    );

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
                            "Error reading {}. Check the application logs for details.",
                            name
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

pub fn warning_icon_text(name: &str) -> RichText {
    RichText::new("⚠ ".to_string() + name).color(WARNING_COLOR)
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

pub fn missing_icon(ui: &mut Ui) -> Response {
    ui.add_sized(
        [ICON_SIZE, ICON_SIZE],
        Label::new(RichText::new("⚠").size(ICON_TEXT_SIZE)),
    )
}

pub fn warning_icon(ui: &mut Ui) -> Response {
    ui.add_sized(
        [ICON_SIZE, ICON_SIZE],
        Label::new(
            RichText::new("⚠")
                .strong()
                .color(WARNING_COLOR)
                .size(ICON_TEXT_SIZE),
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
                .size(ICON_TEXT_SIZE),
        ),
    )
}

fn mesh_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui) {
    // Don't show non model folders like animation or texture folders.
    for (i, folder) in app
        .models
        .iter_mut()
        .enumerate()
        .filter(|(_, folder)| folder.is_model_folder())
    {
        let id = ui.make_persistent_id("meshlist").with(i);

        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                if let Some(render_model) = app.render_models.get_mut(i) {
                    render_model.is_selected |= ui
                        .add(EyeCheckBox::new(
                            &mut render_model.is_visible,
                            folder_display_name(&folder.model).to_string_lossy(),
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
                                mesh.is_selected |= ui
                                    .add(EyeCheckBox::new(&mut mesh.is_visible, &mesh.name))
                                    .hovered();
                            }
                        });
                    });
                }
            });
    }
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
