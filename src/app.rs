use self::{
    animation_bar::display_animation_bar, file_list::show_folder_files, menu::menu_bar,
    rendering::calculate_mvp, window::*,
};
use crate::{
    app::{anim_list::anim_list, swing_list::swing_list},
    capture::{render_animation_to_gif, render_animation_to_image_sequence, render_screenshot},
    editors::{
        adj::{add_missing_adj_entries, adj_editor},
        anim::anim_editor,
        hlpb::hlpb_editor,
        matl::{matl_editor, preset_editor},
        mesh::mesh_editor,
        meshex::meshex_editor,
        modl::modl_editor,
        nutexb::nutexb_viewer,
        skel::skel_editor,
    },
    load_model,
    log::AppLogger,
    model_folder::{FileChanged, ModelFolderState},
    path::{folder_display_name, folder_editor_title, last_update_check_file},
    preferences::AppPreferences,
    thumbnail::generate_model_thumbnails,
    update::LatestReleaseInfo,
    update_color_theme,
    widgets::*,
    AnimationIndex, AnimationSlot, AnimationState, CameraState, EditorResponse, FileResult,
    RenderState, SwingState, Thumbnail, TEXT_COLOR_DARK, TEXT_COLOR_LIGHT,
};
use egui::{
    collapsing_header::CollapsingState, Button, CentralPanel, CollapsingHeader, Context, Image,
    ImageSource, Label, Response, RichText, ScrollArea, SidePanel, TextureOptions, TopBottomPanel,
    Ui,
};
use egui_commonmark::CommonMarkCache;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use log::error;
use once_cell::sync::Lazy;
use rfd::FileDialog;
use ssbh_data::matl_data::MatlEntryData;
use ssbh_data::prelude::*;
use ssbh_wgpu::{next_frame, ModelFiles, RenderModel};
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Mutex,
};

mod anim_list;
mod animation_bar;
mod file_list;
mod menu;
mod rendering;
mod swing_list;
mod window;

/// The logic required to open and close an editor window from an open file index.
trait Editor {
    type EditorState;

    // TODO: Find a way to simplify these parameters.
    // Merge the open index with the editor state?
    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse>;

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize);
}

impl Editor for AdjData {
    type EditorState = ();

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, adj) = get_file_to_edit(&mut model.model.adjs, *open_file_index)?;
        Some(adj_editor(
            ctx,
            &model.folder_path,
            name,
            adj,
            find_file(&model.model.meshes, "model.numshb"),
            &model.validation.adj_errors,
        ))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.adjs[index]);
    }
}

impl Editor for HlpbData {
    type EditorState = ();

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, hlpb) = get_file_to_edit(&mut model.model.hlpbs, *open_file_index)?;
        Some(hlpb_editor(
            ctx,
            &model.folder_path,
            name,
            hlpb,
            find_file(&model.model.skels, "model.nusktb"),
        ))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.hlpbs[index])
    }
}

impl Editor for SkelData {
    type EditorState = SkelEditorState;

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, skel) = get_file_to_edit(&mut model.model.skels, *open_file_index)?;
        Some(skel_editor(
            ctx,
            &model.folder_path,
            name,
            skel,
            state,
            dark_mode,
        ))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.skels[index])
    }
}

impl Editor for AnimData {
    type EditorState = AnimEditorState;

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, anim) = get_file_to_edit(&mut model.model.anims, *open_file_index)?;
        Some(anim_editor(ctx, &model.folder_path, name, anim, state))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.anims[index])
    }
}

impl Editor for MeshExData {
    type EditorState = ();

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, meshex) = get_file_to_edit(&mut model.model.meshexes, *open_file_index)?;
        Some(meshex_editor(
            ctx,
            &model.folder_path,
            name,
            meshex,
            find_file(&model.model.meshes, "model.numshb"),
        ))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.meshexes[index])
    }
}

impl Editor for MeshData {
    type EditorState = ();

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, mesh) = get_file_to_edit(&mut model.model.meshes, *open_file_index)?;
        Some(mesh_editor(
            ctx,
            &model.folder_path,
            name,
            mesh,
            find_file(&model.model.skels, "model.nusktb"),
            &model.validation.mesh_errors,
            dark_mode,
        ))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.meshes[index])
    }
}

impl Editor for ModlData {
    type EditorState = ModlEditorState;
    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, modl) = get_file_to_edit(&mut model.model.modls, *open_file_index)?;
        Some(modl_editor(
            ctx,
            &model.folder_path,
            name,
            modl,
            find_file(&model.model.meshes, "model.numshb"),
            find_file(&model.model.matls, "model.numatb"),
            &model.validation.modl_errors,
            state,
            dark_mode,
        ))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.modls[index])
    }
}

fn get_file_to_edit<T>(
    files: &mut ModelFiles<T>,
    index: Option<usize>,
) -> Option<(&mut String, &mut T)> {
    index
        .and_then(|index| files.get_mut(index))
        .and_then(|(name, file)| Some((name, file.as_mut().ok()?)))
}

fn open_editor<T: Editor>(
    ctx: &Context,
    model: &mut ModelFolderState,
    open_file_index: &mut Option<usize>,
    state: &mut T::EditorState,
    model_actions: &mut VecDeque<RenderAction>,
    dark_mode: bool,
) -> bool {
    if let Some(response) = T::editor(ctx, model, open_file_index, state, dark_mode) {
        if let Some(index) = open_file_index {
            T::set_changed(&response, &mut model.changed, *index);

            if let Some(message) = response.message {
                match message {
                    crate::EditorMessage::SelectMesh {
                        mesh_object_name,
                        mesh_object_subindex,
                    } => {
                        model_actions.push_back(RenderAction::Model(
                            RenderModelAction::SelectMesh {
                                index: *index,
                                mesh_object_name,
                                mesh_object_subindex,
                            },
                        ));
                    }
                }
            }
        }

        if !response.open {
            // Close the window.
            *open_file_index = None;
        }

        response.changed
    } else {
        false
    }
}

pub static LOGGER: Lazy<AppLogger> = Lazy::new(|| AppLogger {
    messages: Mutex::new(Vec::new()),
});

// Create messages for updates instead of updating directly.
// This avoids mixing rendering and UI logic in the same function.
#[derive(Debug, PartialEq, Clone)]
pub enum RenderAction {
    UpdateRenderSettings,
    UpdateCamera,
    Model(RenderModelAction),
    UpdateLighting,
    // TODO: Store the color here?
    UpdateClearColor,
    // TODO: thumbnails
}

#[derive(Debug, PartialEq, Clone)]
pub enum RenderModelAction {
    Update(usize),
    Remove(usize),
    UpdateMaterials {
        model_index: usize,
        modl: Option<ModlData>,
        matl: Option<MatlData>,
    },
    Refresh,
    Clear,
    HideAll,
    ShowAll,
    HideExpressions,
    HideInkMeshes,
    SelectMesh {
        index: usize,
        mesh_object_name: String,
        mesh_object_subindex: u64,
    },
}

pub struct SsbhApp {
    pub render_actions: VecDeque<RenderAction>,

    pub should_update_thumbnails: bool,
    pub should_validate_models: bool,

    pub release_info: LatestReleaseInfo,

    pub screenshot_to_render: Option<PathBuf>,
    pub animation_gif_to_render: Option<PathBuf>,
    pub animation_image_sequence_to_render: Option<PathBuf>,

    pub material_presets: Vec<MatlEntryData>,
    pub default_presets: Vec<MatlEntryData>,

    pub red_checkerboard: egui::TextureId,
    pub yellow_checkerboard: egui::TextureId,

    pub draw_bone_names: bool,
    pub enable_helper_bones: bool,

    pub ui_state: UiState,
    // TODO: Is parallel list with models the best choice here?
    pub models: Vec<ModelFolderState>,
    // pub render_models: Vec<RenderModel>,
    pub default_thumbnails: Vec<Thumbnail>,
    pub animation_state: AnimationState,
    pub swing_state: SwingState,

    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub show_bottom_panel: bool,

    pub camera_state: CameraState,

    pub preferences: AppPreferences,

    pub markdown_cache: CommonMarkCache,

    pub previous_viewport_width: f32,
    pub previous_viewport_height: f32,

    pub has_initialized_zoom_factor: bool,
}

// All the icons are designed to render properly at 16x16 pixels.
pub fn draggable_icon(ctx: &Context, ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(
        ctx,
        ui,
        egui::include_image!("icons/carbon_draggable.svg"),
        dark_mode,
    )
}

pub fn mesh_icon(ctx: &Context, ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ctx, ui, egui::include_image!("icons/mesh.svg"), dark_mode)
}

pub fn matl_icon(ctx: &Context, ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ctx, ui, egui::include_image!("icons/matl.svg"), dark_mode)
}

pub fn adj_icon(ctx: &Context, ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ctx, ui, egui::include_image!("icons/adj.svg"), dark_mode)
}

pub fn anim_icon(ctx: &Context, ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ctx, ui, egui::include_image!("icons/anim.svg"), dark_mode)
}

pub fn skel_icon(ctx: &Context, ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ctx, ui, egui::include_image!("icons/skel.svg"), dark_mode)
}

pub fn hlpb_icon(ctx: &Context, ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ctx, ui, egui::include_image!("icons/hlpb.svg"), dark_mode)
}

fn file_icon(ctx: &Context, ui: &mut Ui, image: ImageSource, dark_mode: bool) -> Response {
    let tint = if dark_mode {
        TEXT_COLOR_DARK
    } else {
        TEXT_COLOR_LIGHT
    };

    // Render at twice the desired size to handle high DPI displays.
    match image
        .load(ctx, TextureOptions::default(), egui::SizeHint::Size(32, 32))
        .unwrap()
    {
        egui::load::TexturePoll::Pending { .. } => {
            ui.allocate_response(egui::vec2(16.0, 16.0), egui::Sense::empty())
        }
        egui::load::TexturePoll::Ready { texture } => ui.add(
            Image::new(texture)
                .tint(tint)
                .fit_to_exact_size(egui::vec2(16.0, 16.0)),
        ),
    }
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
    pub log_window_open: bool,
    pub preferences_window_open: bool,
    pub device_info_window_open: bool,

    // TODO: Is there a better way to track this?
    // Clicking an item in the file list sets the selected index.
    // If the index is not None, the corresponding editor stays open.
    pub selected_folder_index: Option<usize>,
    pub open_skel: Option<usize>,
    pub open_hlpb: Option<usize>,
    pub open_matl: Option<usize>,
    pub open_modl: Option<usize>,
    pub open_mesh: Option<usize>,
    pub open_nutexb: Option<usize>,
    pub open_adj: Option<usize>,
    pub open_anim: Option<usize>,
    pub open_meshex: Option<usize>,

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

#[derive(PartialEq, Eq)]
pub enum PresetMode {
    User,
    Default,
}

impl Default for PresetMode {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Default)]
pub struct MatlEditorState {
    pub selected_material_index: usize,
    pub hovered_material_index: Option<usize>,
    pub matl_preset_window_open: bool,
    pub selected_preset_index: usize,
    pub preset_mode: PresetMode,
    pub texture_to_edit_index: Option<usize>,
}

#[derive(Default)]
pub struct ModlEditorState {
    pub editor_tab: ModlEditorTab,
}

#[derive(PartialEq, Eq)]
pub enum ModlEditorTab {
    Assignments,
    Files,
}

impl Default for ModlEditorTab {
    fn default() -> Self {
        Self::Assignments
    }
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
    Hierarchy,
    Graph,
    List,
}

impl Default for AnimEditorTab {
    fn default() -> Self {
        Self::Hierarchy
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
    Mesh,
    Anim,
    Swing,
}

impl Default for PanelTab {
    fn default() -> Self {
        Self::Mesh
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

        // Load recursively for nested folders like stages.
        let mut new_models = ssbh_wgpu::load_model_folders(&folder);

        // Don't add any folders that have already been added.
        new_models.retain(|(p, _)| !self.models.iter().any(|m| &m.folder_path == p));

        // List folders alphabetically.
        new_models.sort_by_key(|(p, _)| p.clone());

        self.animation_state
            .animations
            .extend(new_models.iter().enumerate().map(|(i, (_, model))| {
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

        self.swing_state
            .selected_swing_folders
            .extend(std::iter::repeat(None).take(new_models.len()));

        // Use an empty set since we can't predict the collisions hashes.
        // This has the side effect of hiding all collisions by default.
        self.swing_state
            .hidden_collisions
            .extend(std::iter::repeat(HashSet::new()).take(new_models.len()));

        for (path, model) in new_models {
            let model_state = load_model(path, model);
            self.models.push(model_state);
        }

        self.sort_files();

        // TODO: Only validate the models that were added?
        self.should_validate_models = true;
        self.should_update_thumbnails = true;
        // TODO: Only load new render models for better performance.
        self.render_actions
            .push_back(RenderAction::Model(RenderModelAction::Refresh));

        self.add_recent_folder(folder);
    }

    fn add_recent_folder<P: AsRef<Path>>(&mut self, folder: P) {
        let new_folder = folder.as_ref().to_string_lossy().to_string();

        if let Some(i) = self
            .preferences
            .recent_folders
            .iter()
            .position(|f| f == &new_folder)
        {
            // Move a folder back to the front if it was seen before.
            self.preferences.recent_folders.remove(i);
        }

        self.preferences.recent_folders.insert(0, new_folder);

        // Only keep track of a limited number of recent folders.
        self.preferences.recent_folders.truncate(10);
    }

    pub fn reload_workspace(&mut self) {
        // This also reloads animations since animations are stored as indices.
        for model in &mut self.models {
            model.reload();
        }
        self.sort_files();

        self.render_actions
            .push_back(RenderAction::Model(RenderModelAction::Refresh));
        self.should_update_thumbnails = true;
        self.should_validate_models = true;
        // Reloaded models should have their animations applied.
        // This includes if the animation playback is paused.
        self.animation_state.should_update_animations = true;
    }

    pub fn clear_workspace(&mut self) {
        // TODO: Is it easier to have dedicated reset methods?
        self.models = Vec::new();
        self.render_actions
            .push_back(RenderAction::Model(RenderModelAction::Clear));
        self.animation_state.animations = Vec::new();
        self.swing_state.selected_swing_folders = Vec::new();
        self.swing_state.hidden_collisions = Vec::new();
        self.camera_state.anim_path = None;
        self.render_actions.push_back(RenderAction::UpdateCamera);
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

    fn update_model_thumbnails(&mut self, wgpu_state: &egui_wgpu::RenderState) {
        if self.should_update_thumbnails {
            for (i, model) in self.models.iter_mut().enumerate() {
                let binding = &mut wgpu_state.renderer.write();
                let render_state: &RenderState = binding.callback_resources.get().unwrap();

                // Split into two steps to avoid mutably and immutably borrowing egui renderer.
                let thumbnails = generate_model_thumbnails(
                    binding,
                    &model.model,
                    &render_state.render_models[i],
                    &wgpu_state.device,
                    &wgpu_state.queue,
                );

                model.thumbnails = thumbnails
                    .into_iter()
                    .map(|(name, view, dimension)| {
                        let id = binding.register_native_texture(
                            &wgpu_state.device,
                            &view,
                            wgpu::FilterMode::Nearest,
                        );
                        (name, id, dimension)
                    })
                    .collect();
            }
            self.should_update_thumbnails = false;
        }
    }

    fn get_hovered_material_label(&self, folder_index: usize) -> Option<&str> {
        Some(
            self.models
                .get(folder_index)?
                .model
                .find_matl()?
                .entries
                .get(self.ui_state.matl_editor.hovered_material_index?)?
                .material_label
                .as_str(),
        )
    }
}

impl eframe::App for SsbhApp {
    // TODO: split into view and update functions to simplify render updates (elm/mvu).
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let current_frame_start = std::time::Instant::now();

        let binding = frame.wgpu_render_state();
        let wgpu_state = binding.as_ref().unwrap();
        let device = &wgpu_state.device;
        let queue = &wgpu_state.queue;

        self.update_model_thumbnails(wgpu_state);

        // TODO: Create a function for updating rendering stuff?
        // Access all the rendering state from a single item in the type map.
        // This avoids deadlock from trying to lock for each resource type.
        let binding = &mut wgpu_state.renderer.write();
        let render_state: &mut RenderState = binding.callback_resources.get_mut().unwrap();

        // This can be set by the mesh list and mesh editor.
        // Clear every frame so both sources can set is_selected to true.
        render_state.clear_selected_meshes();

        // TODO: Rework these fields to use Option<T>.
        let mask_model_index = self.ui_state.selected_folder_index.unwrap_or(0);
        render_state.model_render_options.mask_model_index = mask_model_index;
        self.get_hovered_material_label(mask_model_index)
            .unwrap_or("")
            .clone_into(&mut render_state.model_render_options.mask_material_label);

        // TODO: Find a better way to clear this every frame.
        self.ui_state.matl_editor.hovered_material_index = None;

        if self.animation_state.is_playing {
            let final_frame_index = self.max_final_frame_index(render_state);

            self.animation_state.current_frame = next_frame(
                self.animation_state.current_frame,
                current_frame_start.duration_since(self.animation_state.previous_frame_start),
                final_frame_index,
                self.animation_state.playback_speed,
                self.animation_state.should_loop,
            );
            // eframe is reactive by default, so we need to repaint.
            ctx.request_repaint();
        }
        // Always update the frame times even if no animation is playing.
        // This avoids skipping when resuming playback.
        self.animation_state.previous_frame_start = current_frame_start;

        if let Some((texture, dimension, size)) =
            self.get_nutexb_to_render(&render_state.render_models)
        {
            render_state.texture_renderer.update(
                device,
                queue,
                texture,
                *dimension,
                size,
                &render_state.texture_render_settings,
            );
        }

        ctx.input(|input| {
            for file in &input.raw.dropped_files {
                if let Some(path) = file.path.as_ref() {
                    if path.is_file() {
                        // Load the parent folder for files.
                        if let Some(parent) = path.parent() {
                            self.add_folder_to_workspace(parent, false);
                        }
                    } else {
                        self.add_folder_to_workspace(path, false);
                    }
                }
            }
        });

        if !self.has_initialized_zoom_factor {
            // Set zoom factor here instead of creation to avoid crashes.
            ctx.set_zoom_factor(self.preferences.scale_factor);
            self.has_initialized_zoom_factor = true;
        }

        // Set the region for the 3D viewport to reduce overdraw.
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| menu_bar(self, ui));

        // Add windows here so they can overlap everything except the top panel.
        // We store some state in self to keep track of whether this should be left open.
        render_settings_window(
            ctx,
            &mut render_state.render_settings,
            &mut render_state.model_render_options,
            &mut render_state.skinning_settings,
            &mut self.ui_state.render_settings_open,
            &mut self.draw_bone_names,
            &mut self.enable_helper_bones,
        );
        if self.ui_state.render_settings_open {
            self.render_actions
                .push_back(RenderAction::UpdateRenderSettings);
        }

        if camera_settings_window(
            ctx,
            &mut self.ui_state.camera_settings_open,
            &mut self.camera_state,
            &mut self.preferences.default_camera,
        ) {
            self.render_actions.push_back(RenderAction::UpdateCamera);
        }

        device_info_window(
            ctx,
            &mut self.ui_state.device_info_window_open,
            &render_state.adapter_info,
        );

        if stage_lighting_window(
            ctx,
            &mut self.ui_state.stage_lighting_open,
            &mut self.ui_state.stage_lighting,
        ) {
            self.render_actions.push_back(RenderAction::UpdateLighting);
        }

        log_window(ctx, &mut self.ui_state.log_window_open);

        if preferences_window(
            ctx,
            &mut self.preferences,
            &mut self.ui_state.preferences_window_open,
        ) {
            update_color_theme(&self.preferences, ctx);
            self.render_actions
                .push_back(RenderAction::UpdateClearColor);
            ctx.set_zoom_factor(self.preferences.scale_factor);
        } else {
            self.preferences.scale_factor = ctx.zoom_factor();
        }

        // Only edit the user presets.
        preset_editor(
            ctx,
            &mut self.ui_state,
            &mut self.material_presets,
            &self.default_thumbnails,
            render_state.shared_data.database(),
            self.red_checkerboard,
            self.yellow_checkerboard,
            self.preferences.dark_mode,
        );

        // Don't reopen the window once closed.
        if self.release_info.should_show_update {
            new_release_window(ctx, &mut self.release_info, &mut self.markdown_cache);
        }

        self.should_validate_models |= self.file_editors(ctx, render_state);

        if self.show_left_panel {
            SidePanel::left("left_panel")
                .default_width(200.0)
                .show(ctx, |ui| self.files_list(ctx, ui));
        }

        if self.show_bottom_panel {
            TopBottomPanel::bottom("bottom panel")
                .show(ctx, |ui| self.bottom_panel(ui, render_state));
        }

        if self.show_right_panel {
            SidePanel::right("right panel")
                .default_width(450.0)
                .show(ctx, |ui| self.right_panel(ctx, ui, render_state));
        }

        CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();

            // Convert logical points to physical pixels.
            let scale_factor = ctx.native_pixels_per_point().unwrap_or(1.0);
            let width = rect.width() * scale_factor;
            let height = rect.height() * scale_factor;

            // It's possible to interact with the UI with the mouse over the viewport.
            // Disable tracking the mouse in this case to prevent unwanted camera rotations.
            // This mostly affects resizing the left and right side panels.
            if !ctx.wants_keyboard_input() && !ctx.wants_pointer_input() {
                ctx.input(|input| {
                    // Handle camera input here to get the viewport's actual size.
                    handle_input(&mut self.camera_state, input, height);
                });
            }

            if width > 0.0 && height > 0.0 {
                self.refresh_render_state(device, queue, render_state, width, height, 1.0);

                // Cache previous dimensions since we don't have a resize event handler.
                if width != self.previous_viewport_width || height != self.previous_viewport_height
                {
                    render_state
                        .renderer
                        .resize(device, width as u32, height as u32, 1.0);

                    self.previous_viewport_width = width;
                    self.previous_viewport_height = height;
                }
            }

            // TODO: Avoid calculating the camera twice?
            let (_, _, _, mvp_matrix) = calculate_mvp(width, height, &self.camera_state.values);
            // TODO: Find a way to avoid clone?
            let cb = egui_wgpu::Callback::new_paint_callback(
                rect,
                ViewportCallback {
                    width,
                    height,
                    scale_factor,
                    draw_bone_names: self.draw_bone_names,
                    mvp_matrix,
                    hidden_collisions: self.swing_state.hidden_collisions.clone(),
                },
            );
            ui.painter().add(cb);

            // TODO: Run these on another thread?
            // TODO: Avoid clone?
            // TODO: This will be cleaner if the main renderer isn't mutated?
            if let Some(file) = self.animation_gif_to_render.clone() {
                render_animation_to_gif(
                    self,
                    device,
                    queue,
                    render_state,
                    width as u32,
                    height as u32,
                    file,
                    wgpu_state.target_format,
                );
                self.animation_gif_to_render = None;
                render_state.update_clear_color(self.preferences.viewport_color);
            }

            if let Some(file) = self.animation_image_sequence_to_render.clone() {
                render_animation_to_image_sequence(
                    self,
                    device,
                    queue,
                    render_state,
                    width as u32,
                    height as u32,
                    file,
                    wgpu_state.target_format,
                );
                self.animation_image_sequence_to_render = None;
                render_state.update_clear_color(self.preferences.viewport_color);
            }

            if let Some(file) = &self.screenshot_to_render {
                let image = render_screenshot(
                    device,
                    queue,
                    render_state,
                    width as u32,
                    height as u32,
                    wgpu_state.target_format,
                );
                if let Err(e) = image.save(file) {
                    error!("Error saving screenshot to {:?}: {}", file, e);
                }
                self.screenshot_to_render = None;
                render_state.update_clear_color(self.preferences.viewport_color);
            }
        });
    }

    fn on_exit(&mut self) {
        let path = last_update_check_file();
        if let Err(e) = std::fs::write(&path, self.release_info.update_check_time.to_string()) {
            error!("Failed to write update check time to {path:?}: {e}");
        }

        self.preferences.write_to_file();
    }
}

// TODO: Create a separate module for input handling?
fn handle_input(camera: &mut CameraState, input: &egui::InputState, viewport_height: f32) {
    // Assume zero deltas if no updates are needed.
    if input.pointer.primary_down() {
        // Left click rotation.
        // Swap XY so that dragging left right rotates left right.
        let delta = input.pointer.delta();
        camera.values.rotation_radians.x += delta.y * 0.01;
        camera.values.rotation_radians.y += delta.x * 0.01;
    } else if input.pointer.secondary_down() {
        // Right click panning.
        // Translate an equivalent distance in screen space based on the camera.
        // The viewport height and vertical field of view define the conversion.
        let fac =
            camera.values.fov_y_radians.sin() * camera.values.translation.z.abs() / viewport_height;

        // Negate y so that dragging up "drags" the model up.
        let delta = input.pointer.delta();
        camera.values.translation.x += delta.x * fac;
        camera.values.translation.y -= delta.y * fac;
    }

    // Scale zoom speed with distance to make it easier to zoom out large scenes.
    let delta_z = input.smooth_scroll_delta.y * camera.values.translation.z.abs() * 0.002;
    // Clamp to prevent the user from zooming through the origin.
    camera.values.translation.z = (camera.values.translation.z + delta_z).min(-1.0);

    // Keyboard panning.
    if input.key_down(egui::Key::ArrowLeft) {
        camera.values.translation.x += 0.25;
    }
    if input.key_down(egui::Key::ArrowRight) {
        camera.values.translation.x -= 0.25;
    }
    if input.key_down(egui::Key::ArrowUp) {
        camera.values.translation.y -= 0.25;
    }
    if input.key_down(egui::Key::ArrowDown) {
        camera.values.translation.y += 0.25;
    }
}

struct ViewportCallback {
    width: f32,
    height: f32,
    scale_factor: f32,
    draw_bone_names: bool,
    mvp_matrix: glam::Mat4,
    hidden_collisions: Vec<HashSet<u64>>,
}

impl CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let state: &mut RenderState = callback_resources.get_mut().unwrap();

        state.renderer.begin_render_models(
            egui_encoder,
            &state.render_models,
            state.shared_data.database(),
            &state.model_render_options,
        );

        // TODO: Make the font size configurable.
        if state.model_render_options.draw_bones && self.draw_bone_names {
            state.bone_name_renderer.prepare(
                device,
                queue,
                &state.render_models,
                self.width as u32,
                self.height as u32,
                self.mvp_matrix,
                18.0 * self.scale_factor,
            );
        }

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let state: &RenderState = callback_resources.get().unwrap();
        state.renderer.end_render_models(render_pass);

        for (render_model, hidden_collisions) in state
            .render_models
            .iter()
            .zip(self.hidden_collisions.iter())
        {
            state
                .renderer
                .render_swing(render_pass, render_model, hidden_collisions);
        }

        if state.model_render_options.draw_bones && self.draw_bone_names {
            state.bone_name_renderer.render(render_pass);
        }
    }
}

impl SsbhApp {
    fn file_editors(&mut self, ctx: &Context, render_state: &mut RenderState) -> bool {
        let mut file_changed = false;

        // TODO: Use some sort of trait to clean up repetitive code?
        // The functions would take an additional ui parameter.
        if let Some(folder_index) = self.ui_state.selected_folder_index {
            if let Some(model) = self.models.get_mut(folder_index) {
                // TODO: Group added state and implement the Editor trait.
                if let Some(matl_index) = self.ui_state.open_matl {
                    if let Some((name, Ok(matl))) = model.model.matls.get_mut(matl_index) {
                        let response = matl_editor(
                            ctx,
                            &model.folder_path,
                            name,
                            &mut self.ui_state.matl_editor,
                            matl,
                            find_file_mut(&mut model.model.modls, "model.numdlb"),
                            &model.validation.matl_errors,
                            &model.thumbnails,
                            &self.default_thumbnails,
                            render_state.shared_data.database(),
                            &mut self.material_presets,
                            &self.default_presets,
                            self.red_checkerboard,
                            self.yellow_checkerboard,
                            self.preferences.dark_mode,
                        );
                        // TODO: This modifies the model.numdlb when renaming materials.
                        response.set_changed(&mut model.changed.matls[matl_index]);
                        file_changed |= response.changed;

                        if !response.open {
                            // Close the window.
                            self.ui_state.open_matl = None;
                        }

                        // Update on change to avoid costly state changes every frame.
                        if response.changed {
                            // Only the model.numatb is rendered in the viewport for now.
                            if name == "model.numatb" {
                                // Reassign materials in case material or shader labels changed.
                                // This is necessary for error checkerboards to display properly.
                                // Perform a cheap clone to avoid lifetime issues.
                                self.render_actions.push_back(RenderAction::Model(
                                    RenderModelAction::UpdateMaterials {
                                        model_index: folder_index,
                                        modl: model.model.find_modl().cloned(),
                                        matl: model.model.find_matl().cloned(),
                                    },
                                ));
                            }
                        }
                    }
                }

                if open_editor::<MeshData>(
                    ctx,
                    model,
                    &mut self.ui_state.open_mesh,
                    &mut (),
                    &mut self.render_actions,
                    self.preferences.dark_mode,
                ) {
                    // The mesh editor has no high frequency edits (sliders), so reload on any change.
                    self.render_actions
                        .push_back(RenderAction::Model(RenderModelAction::Update(folder_index)));
                    file_changed = true;
                }

                file_changed |= open_editor::<SkelData>(
                    ctx,
                    model,
                    &mut self.ui_state.open_skel,
                    &mut self.ui_state.skel_editor,
                    &mut self.render_actions,
                    self.preferences.dark_mode,
                );

                if open_editor::<ModlData>(
                    ctx,
                    model,
                    &mut self.ui_state.open_modl,
                    &mut self.ui_state.modl_editor,
                    &mut self.render_actions,
                    self.preferences.dark_mode,
                ) {
                    // Perform a cheap clone to avoid lifetime issues.
                    self.render_actions.push_back(RenderAction::Model(
                        RenderModelAction::UpdateMaterials {
                            model_index: folder_index,
                            modl: model.model.find_modl().cloned(),
                            matl: model.model.find_matl().cloned(),
                        },
                    ));
                    file_changed = true;
                }

                if open_editor::<HlpbData>(
                    ctx,
                    model,
                    &mut self.ui_state.open_hlpb,
                    &mut (),
                    &mut self.render_actions,
                    self.preferences.dark_mode,
                ) {
                    // Reapply the animation constraints in the viewport.
                    self.animation_state.should_update_animations = true;
                    file_changed = true;
                }

                file_changed |= open_editor::<AdjData>(
                    ctx,
                    model,
                    &mut self.ui_state.open_adj,
                    &mut (),
                    &mut self.render_actions,
                    self.preferences.dark_mode,
                );

                if open_editor::<AnimData>(
                    ctx,
                    model,
                    &mut self.ui_state.open_anim,
                    &mut self.ui_state.anim_editor,
                    &mut self.render_actions,
                    self.preferences.dark_mode,
                ) {
                    // Reapply the animations in the viewport.
                    self.animation_state.should_update_animations = true;
                    file_changed = true;
                }

                if open_editor::<MeshExData>(
                    ctx,
                    model,
                    &mut self.ui_state.open_meshex,
                    &mut (),
                    &mut self.render_actions,
                    self.preferences.dark_mode,
                ) {
                    // MeshEx settings require reloading the render model.
                    self.render_actions
                        .push_back(RenderAction::Model(RenderModelAction::Update(folder_index)));
                    file_changed = true;
                }

                if let Some(nutexb_index) = self.ui_state.open_nutexb {
                    if let Some((name, Ok(nutexb))) = model.model.nutexbs.get(nutexb_index) {
                        if !nutexb_viewer(
                            ctx,
                            &folder_editor_title(&model.folder_path, name),
                            nutexb,
                            &mut render_state.texture_render_settings,
                        ) {
                            // Close the window.
                            self.ui_state.open_nutexb = None;
                        }
                    }
                }
            }
        }

        file_changed
    }

    pub fn max_final_frame_index(&self, render_state: &RenderState) -> f32 {
        // Find the minimum number of frames to cover all animations.
        // This should include stage animations like lighting and cameras.
        self.animation_state
            .animations
            .iter()
            .flat_map(|model_animations| {
                model_animations
                    .iter()
                    .filter_map(|a| a.animation.as_ref())
                    .filter_map(|anim_index| {
                        let (_, anim) = AnimationIndex::get_animation(anim_index, &self.models)?;
                        Some(anim.as_ref().ok()?.final_frame_index)
                    })
            })
            .chain(
                render_state
                    .lighting_data
                    .light
                    .as_ref()
                    .map(|a| a.final_frame_index),
            )
            .chain(
                render_state
                    .camera_anim
                    .as_ref()
                    .map(|a| a.final_frame_index),
            )
            .fold(0.0, f32::max)
    }

    fn files_list(&mut self, ctx: &Context, ui: &mut Ui) {
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
                    CollapsingHeader::new(folder_display_name(model))
                        .id_salt(format!("folder.{folder_index}"))
                        .default_open(true)
                        .show(ui, |ui| {
                            show_folder_files(
                                &mut self.ui_state,
                                model,
                                ctx,
                                ui,
                                folder_index,
                                self.preferences.dark_mode,
                            );
                        })
                        .header_response
                        .on_hover_text(model.folder_path.to_string_lossy())
                        .context_menu(|ui| {
                            // Prevent adding a file that already exists.
                            let mesh = model.model.find_mesh();
                            let should_add_adjb =
                                mesh.is_some() && model.model.find_adj().is_none();

                            if ui
                                .add_enabled(should_add_adjb, Button::new("Add model.adjb"))
                                .clicked()
                            {
                                ui.close_menu();

                                // Add a missing adjb file based on the mesh.
                                // TODO: Disable if the file isn't required?
                                let mut new_adj = AdjData {
                                    entries: Vec::new(),
                                };
                                add_missing_adj_entries(
                                    &mut new_adj,
                                    &model.validation.adj_errors,
                                    mesh,
                                );
                                model
                                    .model
                                    .adjs
                                    .push(("model.adjb".to_owned(), Ok(new_adj)));
                                // Mark the new file as modified to prompt the user to save it.
                                model.changed.adjs.push(true);
                            }

                            // Prevent adding a file that already exists.
                            let mesh = model.model.find_mesh();
                            let should_add_meshex =
                                mesh.is_some() && model.model.find_meshex().is_none();

                            if ui
                                .add_enabled(should_add_meshex, Button::new("Add model.numshexb"))
                                .clicked()
                            {
                                ui.close_menu();

                                if let Some(mesh) = mesh {
                                    let new_meshex = MeshExData::from_mesh_objects(&mesh.objects);
                                    model
                                        .model
                                        .meshexes
                                        .push(("model.numshexb".to_owned(), Ok(new_meshex)));
                                    // Mark the new file as modified to prompt the user to save it.
                                    model.changed.meshexes.push(true);
                                }
                            }

                            ui.separator();

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
                    self.render_actions
                        .push_back(RenderAction::Model(RenderModelAction::Remove(
                            folder_to_remove,
                        )));
                }
            });
    }

    fn bottom_panel(&mut self, ui: &mut Ui, render_state: &mut RenderState) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let final_frame_index = self.max_final_frame_index(render_state);
            display_animation_bar(ui, &mut self.animation_state, final_frame_index);

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

    fn right_panel(&mut self, ctx: &Context, ui: &mut Ui, render_state: &mut RenderState) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.ui_state.right_panel_tab,
                PanelTab::Mesh,
                RichText::new("Meshes").heading(),
            );
            ui.selectable_value(
                &mut self.ui_state.right_panel_tab,
                PanelTab::Anim,
                RichText::new("Animations").heading(),
            );
            ui.selectable_value(
                &mut self.ui_state.right_panel_tab,
                PanelTab::Swing,
                RichText::new("Swing").heading(),
            );
        });

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| match self.ui_state.right_panel_tab {
                PanelTab::Mesh => mesh_list(ctx, self, ui, render_state),
                PanelTab::Anim => anim_list(ctx, self, ui),
                PanelTab::Swing => swing_list(ctx, self, ui),
            });
    }

    pub fn get_nutexb_to_render<'a>(
        &self,
        render_models: &'a [RenderModel],
    ) -> Option<(
        &'a wgpu::Texture,
        &'a wgpu::TextureViewDimension,
        (u32, u32, u32),
    )> {
        let folder_index = self.ui_state.selected_folder_index?;
        let model = self.models.get(folder_index)?;
        let render_model = render_models.get(folder_index)?;

        // Assume file names are unique, so use the name instead of the index.
        let (name, _) = model.model.nutexbs.get(self.ui_state.open_nutexb?)?;

        render_model.get_texture(name).map(|(texture, dim)| {
            (
                texture,
                dim,
                (
                    texture.width(),
                    texture.height(),
                    if *dim == wgpu::TextureViewDimension::D3 {
                        texture.depth_or_array_layers()
                    } else {
                        1
                    },
                ),
            )
        })
    }
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

pub fn warning_icon_text(name: &str) -> RichText {
    RichText::new("⚠ ".to_string() + name).color(WARNING_COLOR)
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
            ui.label(format!("{error}"));
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

fn mesh_list(ctx: &Context, app: &mut SsbhApp, ui: &mut Ui, render_state: &mut RenderState) {
    // Don't show non model folders like animation or texture folders.
    for (i, folder) in app
        .models
        .iter_mut()
        .enumerate()
        .filter(|(_, folder)| folder.is_model_folder())
    {
        let id = ui.make_persistent_id("meshlist").with(i);

        // Allow programmatically toggling the open state of each folder.
        let mut state = CollapsingState::load_with_default_open(ctx, id, true);
        state.set_open(folder.is_meshlist_open);
        state
            .show_header(ui, |ui| {
                if let Some(render_model) = render_state.render_models.get_mut(i) {
                    render_model.is_selected |= ui
                        .add(EyeCheckBox::new(
                            &mut render_model.is_visible,
                            folder_display_name(folder),
                        ))
                        .hovered();
                }
            })
            .body(|ui| {
                // TODO: How to ensure the render models stay in sync with the model folder?
                if let Some(render_model) = render_state.render_models.get_mut(i) {
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
        let state = CollapsingState::load_with_default_open(ctx, id, true);
        folder.is_meshlist_open = state.is_open();
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
