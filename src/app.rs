use self::{
    animation_bar::display_animation_bar, file_list::show_folder_files, menu::menu_bar, window::*,
};
use crate::{
    app::anim_list::anim_list,
    app::swing_list::swing_list,
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
    hide_expressions, load_model_render_model,
    log::AppLogger,
    model_folder::{FileChanged, ModelFolderState},
    path::{folder_display_name, folder_editor_title, last_update_check_file},
    preferences::AppPreferences,
    update::LatestReleaseInfo,
    widgets::*,
    AnimationIndex, AnimationSlot, AnimationState, CameraInputState, EditorResponse, FileResult,
    RenderState, SwingState, Thumbnail, TEXT_COLOR_DARK, TEXT_COLOR_LIGHT,
};
use egui::{
    collapsing_header::CollapsingState, Button, CollapsingHeader, Context, Image, Label, Response,
    RichText, ScrollArea, SidePanel, TopBottomPanel, Ui,
};
use egui_commonmark::CommonMarkCache;
use egui_dnd::DragDropUi;
use egui_extras::RetainedImage;
use log::error;
use once_cell::sync::Lazy;
use rfd::FileDialog;
use ssbh_data::matl_data::MatlEntryData;
use ssbh_data::prelude::*;
use ssbh_wgpu::{ModelFiles, RenderModel};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Mutex,
};

mod anim_list;
mod animation_bar;
mod file_list;
mod menu;
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
        render_model: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        icons: &Icons,
        dark_mode: bool,
    ) -> Option<EditorResponse>;

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize);
}

impl Editor for AdjData {
    type EditorState = ();

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        _: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: &Icons,
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
        _: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: &Icons,
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
        _: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        icons: &Icons,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, skel) = get_file_to_edit(&mut model.model.skels, *open_file_index)?;
        Some(skel_editor(
            ctx,
            &model.folder_path,
            name,
            skel,
            state,
            icons,
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
        _: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        _: &Icons,
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
        _: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: &Icons,
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
    type EditorState = MeshEditorState;

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        render_model: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        icons: &Icons,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, mesh) = get_file_to_edit(&mut model.model.meshes, *open_file_index)?;
        Some(mesh_editor(
            ctx,
            &model.folder_path,
            name,
            mesh,
            render_model,
            find_file(&model.model.skels, "model.nusktb"),
            &model.validation.mesh_errors,
            state,
            icons,
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
        render_model: &mut Option<&mut RenderModel>,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        icons: &Icons,
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
            render_model,
            icons,
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
    render_model: &mut Option<&mut RenderModel>,
    open_file_index: &mut Option<usize>,
    state: &mut T::EditorState,
    icons: &Icons,
    dark_mode: bool,
) -> bool {
    if let Some(response) = T::editor(
        ctx,
        model,
        render_model,
        open_file_index,
        state,
        icons,
        dark_mode,
    ) {
        if let Some(index) = open_file_index {
            T::set_changed(&response, &mut model.changed, *index);
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
    pub release_info: LatestReleaseInfo,

    pub screenshot_to_render: Option<PathBuf>,
    pub animation_gif_to_render: Option<PathBuf>,
    pub animation_image_sequence_to_render: Option<PathBuf>,

    pub material_presets: Vec<MatlEntryData>,

    pub red_checkerboard: egui::TextureId,
    pub yellow_checkerboard: egui::TextureId,

    pub draw_bone_names: bool,
    pub enable_helper_bones: bool,

    pub ui_state: UiState,
    // TODO: Is parallel list with models the best choice here?
    pub models: Vec<ModelFolderState>,
    pub render_models: Vec<RenderModel>,

    pub default_thumbnails: Vec<Thumbnail>,
    pub animation_state: AnimationState,
    pub swing_state: SwingState,

    pub render_state: RenderState,

    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub show_bottom_panel: bool,

    pub camera_state: CameraInputState,

    pub preferences: AppPreferences,

    pub icons: Icons,

    pub markdown_cache: CommonMarkCache,
}

pub struct Icons {
    draggable: RetainedImage,
    mesh: RetainedImage,
    matl: RetainedImage,
    adj: RetainedImage,
    anim: RetainedImage,
    skel: RetainedImage,
    hlpb: RetainedImage,
}

impl Icons {
    pub fn new() -> Self {
        // All the icons are designed to render properly at 16x16 pixels.
        // Render at 32x32 to handle up to 200% display scaling.
        let svg_image = |name: &str, bytes: &[u8]| {
            let size = egui_extras::image::FitTo::Size(32, 32);
            RetainedImage::from_svg_bytes_with_size(name, bytes, size).unwrap()
        };

        Self {
            draggable: svg_image("draggable", include_bytes!("icons/carbon_draggable.svg")),
            mesh: svg_image("mesh", include_bytes!("icons/mesh.svg")),
            matl: svg_image("matl", include_bytes!("icons/matl.svg")),
            adj: svg_image("adj", include_bytes!("icons/adj.svg")),
            anim: svg_image("anim", include_bytes!("icons/anim.svg")),
            skel: svg_image("skel", include_bytes!("icons/skel.svg")),
            hlpb: svg_image("hlpb", include_bytes!("icons/hlpb.svg")),
        }
    }

    pub fn draggable(&self, ui: &Ui, dark_mode: bool) -> Image {
        file_icon(ui, &self.draggable, dark_mode)
    }

    pub fn mesh(&self, ui: &Ui, dark_mode: bool) -> Image {
        file_icon(ui, &self.mesh, dark_mode)
    }

    pub fn matl(&self, ui: &Ui, dark_mode: bool) -> Image {
        file_icon(ui, &self.matl, dark_mode)
    }

    pub fn adj(&self, ui: &Ui, dark_mode: bool) -> Image {
        file_icon(ui, &self.adj, dark_mode)
    }

    pub fn anim(&self, ui: &Ui, dark_mode: bool) -> Image {
        file_icon(ui, &self.anim, dark_mode)
    }

    pub fn skel(&self, ui: &Ui, dark_mode: bool) -> Image {
        file_icon(ui, &self.skel, dark_mode)
    }

    pub fn hlpb(&self, ui: &Ui, dark_mode: bool) -> Image {
        file_icon(ui, &self.hlpb, dark_mode)
    }
}

fn file_icon(ui: &Ui, image: &RetainedImage, dark_mode: bool) -> Image {
    let tint = if dark_mode {
        TEXT_COLOR_DARK
    } else {
        TEXT_COLOR_LIGHT
    };
    Image::new(image.texture_id(ui.ctx()), egui::vec2(16.0, 16.0)).tint(tint)
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
    pub log_window_open: bool,
    pub preferences_window_open: bool,

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
    pub mesh_editor: MeshEditorState,
    pub preset_editor: MatlEditorState,
    pub anim_editor: AnimEditorState,
    pub skel_editor: SkelEditorState,
    pub modl_editor: ModlEditorState,
    pub stage_lighting: StageLightingState,
}

#[derive(Default)]
pub struct SkelEditorState {
    pub mode: SkelMode,
    pub dnd: DragDropUi,
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
    pub matl_preset_window_open: bool,
    pub selected_material_preset_index: usize,
}

#[derive(Default)]
pub struct ModlEditorState {
    pub advanced_mode: bool,
    pub dnd: DragDropUi,
}

#[derive(Default)]
pub struct MeshEditorState {
    pub advanced_mode: bool,
    pub dnd: DragDropUi,
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

        // TODO: Check for duplicate folders?

        // Load recursively for nested folders like stages.
        let mut new_models = ssbh_wgpu::load_model_folders(&folder);
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
            .visible_collisions
            .extend(std::iter::repeat(HashSet::new()).take(new_models.len()));

        // Only load new render models for better performance.
        // TODO: Handle this with models to update?
        for (path, model) in new_models {
            let (mut render_model, model_state) =
                load_model_render_model(path, model, &self.render_state);

            // Only hide expressions on new models to preserve visibility edits.
            if self.preferences.autohide_expressions {
                hide_expressions(&mut render_model);
            }

            self.models.push(model_state);
            self.render_models.push(render_model);
        }

        self.sort_files();

        // TODO: Only validate the models that were added?
        self.should_validate_models = true;
        self.should_update_thumbnails = true;

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

        self.models_to_update = ItemsToUpdate::All;
        self.should_update_thumbnails = true;
        self.should_validate_models = true;
        // Reloaded models should have their animations applied.
        // This includes if the animation playback is paused.
        self.animation_state.should_update_animations = true;
    }

    pub fn clear_workspace(&mut self) {
        // TODO: Is it easier to have dedicated reset methods?
        self.models = Vec::new();
        self.render_models = Vec::new();
        self.animation_state.animations = Vec::new();
        self.swing_state.selected_swing_folders = Vec::new();
        self.swing_state.visible_collisions = Vec::new();
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

    pub fn write_state_to_disk(&self) {
        let path = last_update_check_file();
        if let Err(e) = std::fs::write(&path, self.release_info.update_check_time.to_string()) {
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
                .show(ctx, |ui| menu_bar(self, ui))
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
        if self.release_info.should_show_update {
            new_release_window(ctx, &mut self.release_info, &mut self.markdown_cache);
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
                    .show(ctx, |ui| self.bottom_panel(ui))
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

    fn file_editors(&mut self, ctx: &Context) -> bool {
        let mut file_changed = false;

        // TODO: Use some sort of trait to clean up repetitive code?
        // The functions would take an additional ui parameter.
        if let Some(folder_index) = self.ui_state.selected_folder_index {
            if let Some(model) = self.models.get_mut(folder_index) {
                let render_model = &mut self.render_models.get_mut(folder_index);

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
                            self.ui_state.open_matl = None;
                        }

                        // Update on change to avoid costly state changes every frame.
                        if response.changed {
                            if let Some(render_model) = render_model {
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

                if open_editor::<MeshData>(
                    ctx,
                    model,
                    render_model,
                    &mut self.ui_state.open_mesh,
                    &mut self.ui_state.mesh_editor,
                    &self.icons,
                    self.preferences.dark_mode,
                ) {
                    // The mesh editor has no high frequency edits (sliders), so reload on any change.
                    self.models_to_update = ItemsToUpdate::One(folder_index);
                    file_changed = true;
                }

                file_changed |= open_editor::<SkelData>(
                    ctx,
                    model,
                    render_model,
                    &mut self.ui_state.open_skel,
                    &mut self.ui_state.skel_editor,
                    &self.icons,
                    self.preferences.dark_mode,
                );

                if open_editor::<ModlData>(
                    ctx,
                    model,
                    render_model,
                    &mut self.ui_state.open_modl,
                    &mut self.ui_state.modl_editor,
                    &self.icons,
                    self.preferences.dark_mode,
                ) {
                    // TODO: Pass an onchanged closure to avoid redundant lookups.
                    if let (Some(modl), matl) = (model.model.find_modl(), model.model.find_matl()) {
                        if let Some(render_model) = render_model {
                            render_model.reassign_materials(modl, matl);
                        }
                    }
                    file_changed = true;
                }

                if open_editor::<HlpbData>(
                    ctx,
                    model,
                    render_model,
                    &mut self.ui_state.open_hlpb,
                    &mut (),
                    &self.icons,
                    self.preferences.dark_mode,
                ) {
                    // Reapply the animation constraints in the viewport.
                    self.animation_state.should_update_animations = true;
                    file_changed = true;
                }

                file_changed |= open_editor::<AdjData>(
                    ctx,
                    model,
                    render_model,
                    &mut self.ui_state.open_adj,
                    &mut (),
                    &self.icons,
                    self.preferences.dark_mode,
                );

                if open_editor::<AnimData>(
                    ctx,
                    model,
                    render_model,
                    &mut self.ui_state.open_anim,
                    &mut self.ui_state.anim_editor,
                    &self.icons,
                    self.preferences.dark_mode,
                ) {
                    // Reapply the animations in the viewport.
                    self.animation_state.should_update_animations = true;
                    file_changed = true;
                }

                if open_editor::<MeshExData>(
                    ctx,
                    model,
                    render_model,
                    &mut self.ui_state.open_meshex,
                    &mut (),
                    &self.icons,
                    self.preferences.dark_mode,
                ) {
                    // MeshEx settings require reloading the render model.
                    self.models_to_update = ItemsToUpdate::One(folder_index);
                    file_changed = true;
                }

                if let Some(nutexb_index) = self.ui_state.open_nutexb {
                    if let Some((name, Ok(nutexb))) = model.model.nutexbs.get(nutexb_index) {
                        if !nutexb_viewer(
                            ctx,
                            &folder_editor_title(&model.folder_path, name),
                            nutexb,
                            &mut self.render_state.texture_render_settings,
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

    pub fn max_final_frame_index(&mut self) -> f32 {
        // Find the minimum number of frames to cover all animations.
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
            .fold(0.0, f32::max)
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
                    CollapsingHeader::new(folder_display_name(model))
                        .id_source(format!("folder.{folder_index}"))
                        .default_open(true)
                        .show(ui, |ui| {
                            show_folder_files(
                                &mut self.ui_state,
                                model,
                                ui,
                                folder_index,
                                &self.icons,
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
                    if self.render_models.get(folder_to_remove).is_some() {
                        self.render_models.remove(folder_to_remove);
                    }
                }
            });
    }

    fn bottom_panel(&mut self, ui: &mut Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let final_frame_index = self.max_final_frame_index();
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

    fn right_panel(&mut self, ctx: &Context, ui: &mut Ui) {
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
                PanelTab::Mesh => mesh_list(ctx, self, ui),
                PanelTab::Anim => anim_list(ctx, self, ui),
                PanelTab::Swing => swing_list(ctx, self, ui),
            });
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
                            folder_display_name(folder),
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
