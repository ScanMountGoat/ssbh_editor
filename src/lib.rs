use ::log::error;
use app::{RenderAction, RenderModelAction, StageLightingState};
use egui::{
    ecolor::linear_f32_from_gamma_u8,
    style::{WidgetVisuals, Widgets},
    Color32, FontFamily, FontId, FontTweak, Rounding, Stroke, TextStyle, Visuals,
};
use model_folder::ModelFolderState;
use nutexb::NutexbFile;
use nutexb_wgpu::TextureRenderer;
use preferences::AppPreferences;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use ssbh_data::{matl_data::ParamId, prelude::*};
use ssbh_wgpu::{
    swing::SwingPrc, BoneNameRenderer, ModelRenderOptions, RenderModel, RenderSettings,
    SharedRenderData, SkinningSettings, SsbhRenderer,
};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    error::Error,
    path::{Path, PathBuf},
};

pub mod app;
pub mod capture;
pub mod editors;
pub mod log;
pub mod material;
pub mod model_folder;
pub mod path;
pub mod preferences;
pub mod presets;
pub mod update;
pub mod validation;
pub mod widgets;

pub static FONT_BYTES: &[u8] = include_bytes!("fonts/NotoSansSC-Regular.otf");

type FileResult<T> = Result<T, Box<dyn Error>>;

pub struct EditorResponse {
    pub open: bool,
    pub changed: bool,
    pub saved: bool,
    pub message: Option<EditorMessage>,
}

// TODO: Separate message types for each editor?
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EditorMessage {
    SelectMesh {
        mesh_object_name: String,
        mesh_object_subindex: u64,
    },
}

impl EditorResponse {
    pub fn set_changed(&self, changed: &mut bool) {
        // Saving should always clear the changed flag.
        *changed = (*changed || self.changed) && !self.saved;
    }
}

// TODO: Separate input state and camera UI state?
pub struct CameraState {
    pub values: CameraValues,
    pub anim_path: Option<PathBuf>,

    // TODO: Where to put this?
    pub mvp_matrix: glam::Mat4,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(default)]
pub struct CameraValues {
    pub translation: glam::Vec3,
    pub rotation_radians: glam::Vec3,
    pub fov_y_radians: f32,
    pub near_clip: f32,
    pub far_clip: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            values: CameraValues::default(),
            anim_path: None,
            mvp_matrix: glam::Mat4::IDENTITY,
        }
    }
}

impl Default for CameraValues {
    fn default() -> Self {
        Self {
            translation: glam::Vec3::new(0.0, -8.0, -60.0),
            rotation_radians: glam::Vec3::new(0.0, 0.0, 0.0),
            fov_y_radians: 30f32.to_radians(),
            near_clip: 1.0f32,
            far_clip: 400000.0f32,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TextureDimension {
    Texture1d,
    Texture2d,
    Texture3d,
    TextureCube,
}

impl TextureDimension {
    pub fn from_nutexb(nutexb: &NutexbFile) -> TextureDimension {
        // Assume no array layers for depth and cube maps.
        if nutexb.footer.depth > 1 {
            TextureDimension::Texture3d
        } else if nutexb.footer.layer_count == 6 {
            TextureDimension::TextureCube
        } else {
            TextureDimension::Texture2d
        }
    }

    pub fn from_param(param: ParamId) -> TextureDimension {
        match param {
            ParamId::Texture2 | ParamId::Texture7 | ParamId::Texture8 => {
                TextureDimension::TextureCube
            }
            _ => TextureDimension::Texture2d,
        }
    }
}

impl From<&wgpu::TextureViewDimension> for TextureDimension {
    fn from(d: &wgpu::TextureViewDimension) -> Self {
        // TODO: Worry about array textures?
        match d {
            wgpu::TextureViewDimension::D1 => Self::Texture1d,
            wgpu::TextureViewDimension::D2 => Self::Texture2d,
            wgpu::TextureViewDimension::D2Array => Self::Texture2d,
            wgpu::TextureViewDimension::Cube => Self::TextureCube,
            wgpu::TextureViewDimension::CubeArray => Self::TextureCube,
            wgpu::TextureViewDimension::D3 => Self::Texture3d,
        }
    }
}

pub struct RenderState {
    pub render_settings: RenderSettings,
    pub skinning_settings: SkinningSettings,
    pub model_render_options: ModelRenderOptions,
    pub texture_render_settings: nutexb_wgpu::RenderSettings,
    pub shared_data: SharedRenderData,
    pub adapter_info: wgpu::AdapterInfo,
    pub lighting_data: LightingData,
    // TODO: where to put this?
    pub camera_anim: Option<AnimData>,
    // TODO: Is this the best place for this?
    pub render_models: Vec<RenderModel>,
    pub renderer: SsbhRenderer,
    pub texture_renderer: TextureRenderer,
    bone_name_renderer: BoneNameRenderer,
}

// Most files are selected from currently loaded folders.
// Store lights separately for now for convenience.
#[derive(Default)]
pub struct LightingData {
    pub light: Option<AnimData>,
    pub reflection_cube_map: Option<NutexbFile>,
    pub color_grading_lut: Option<NutexbFile>,
    // TODO: shpc?
}

impl RenderState {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        adapter_info: wgpu::AdapterInfo,
        renderer: SsbhRenderer,
        texture_renderer: TextureRenderer,
        bone_name_renderer: BoneNameRenderer,
    ) -> Self {
        let shared_data = SharedRenderData::new(device, queue);
        Self {
            render_settings: RenderSettings::default(),
            skinning_settings: SkinningSettings::default(),
            model_render_options: ModelRenderOptions::default(),
            texture_render_settings: nutexb_wgpu::RenderSettings::default(),
            shared_data,
            adapter_info,
            lighting_data: Default::default(),
            camera_anim: None,
            render_models: Vec::new(),
            renderer,
            texture_renderer,
            bone_name_renderer,
        }
    }

    fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        models: &[ModelFolderState],
        actions: &mut VecDeque<RenderAction>,
        stage_lighting: &StageLightingState,
        camera_state: &CameraState,
        autohide_expressions: bool,
        current_frame: f32,
        viewport_color: [u8; 3]
    ) {
        // Only load render models that need to change to improve performance.
        while let Some(action) = actions.pop_front() {
            match action {
                RenderAction::UpdateRenderSettings => {
                    self.renderer
                        .update_render_settings(queue, &self.render_settings);
                    self.renderer
                        .update_skinning_settings(queue, &self.skinning_settings);
                }
                RenderAction::UpdateCamera => {
                    self.camera_anim = camera_state.anim_path.as_ref().and_then(|path| {
                        AnimData::from_file(path)
                            .map_err(|e| {
                                error!("Error reading {:?}: {}", path, e);
                                e
                            })
                            .ok()
                    });
        
                },
                RenderAction::Model(model_action) => {
                    self.update_models(model_action, models, device, queue, autohide_expressions)
                }
                RenderAction::UpdateLighting => {
                    self.update_lighting(device, queue, stage_lighting, models, current_frame)
                }
                RenderAction::UpdateClearColor => {
                    self.update_clear_color(viewport_color);
                },
            }
        }
    }

    fn update_models(
        &mut self,
        model_action: RenderModelAction,
        models: &[ModelFolderState],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        autohide_expressions: bool,
    ) {
        match model_action {
            RenderModelAction::Update(i) => {
                if let (Some(render_model), Some(model)) =
                    (self.render_models.get_mut(i), models.get(i))
                {
                    let mut new_render_model =
                        RenderModel::from_folder(device, queue, &model.model, &self.shared_data);
                    // Attempt to preserve the model and mesh visibility if possible.
                    copy_visibility(&mut new_render_model, render_model);

                    *render_model = new_render_model;
                }
            }
            RenderModelAction::Remove(i) => {
                self.render_models.remove(i);
            }
            RenderModelAction::Refresh => {
                let mut new_render_models = ssbh_wgpu::load_render_models(
                    device,
                    queue,
                    models.iter().map(|m| &m.model),
                    &self.shared_data,
                );

                if autohide_expressions {
                    for render_model in &mut new_render_models {
                        hide_expressions(render_model);
                    }
                }

                // Preserve visibility edits if no models were added.
                for (new_render_model, old_render_model) in
                    new_render_models.iter_mut().zip(self.render_models.iter())
                {
                    copy_visibility(new_render_model, old_render_model);
                }

                self.render_models = new_render_models;
            }
            RenderModelAction::Clear => self.render_models = Vec::new(),
            RenderModelAction::HideAll => {
                for render_model in &mut self.render_models {
                    render_model.is_visible = false;
                }
            }
            RenderModelAction::ShowAll => {
                for render_model in &mut self.render_models {
                    render_model.is_visible = true;
                }
            }
            RenderModelAction::HideExpressions => {
                for render_model in &mut self.render_models {
                    hide_expressions(render_model);
                }
            }
            RenderModelAction::SelectMesh {
                index,
                mesh_object_name,
                mesh_object_subindex,
            } => {
                if let Some(render_mesh) = self.render_models[index]
                    .meshes
                    .iter_mut()
                    .find(|m| m.name == mesh_object_name && m.subindex == mesh_object_subindex)
                {
                    render_mesh.is_selected = true;
                }
            }
            RenderModelAction::UpdateMaterials {
                model_index,
                modl,
                matl,
            } => {
                if let Some(render_model) = self.render_models.get_mut(model_index) {
                    if let Some(matl) = &matl {
                        render_model.recreate_materials(device, &matl.entries, &self.shared_data);
                    }
                    if let Some(modl) = &modl {
                        render_model.reassign_materials(modl, matl.as_ref());
                    }
                }
            }
        }
    }

    fn update_lighting(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        stage_lighting: &StageLightingState,
        models: &[ModelFolderState],
        current_frame: f32,
    ) {
        self.lighting_data = LightingData::from_ui(stage_lighting);

        // light00.nuamb
        self.animate_lighting(queue, current_frame);

        // color_grading_lut.nutexb
        match &self.lighting_data.color_grading_lut {
            Some(lut) => self.renderer.update_color_lut(device, queue, lut),
            None => self.renderer.reset_color_lut(device, queue),
        };

        // reflection_cubemap.nutexb
        match &self.lighting_data.reflection_cube_map {
            Some(cube) => self.shared_data.update_stage_cube_map(device, queue, cube),
            None => {
                self.shared_data.reset_stage_cube_map(device, queue);
            }
        }

        // Updating the cube map requires reassigning model textures.
        for (render_model, model) in self.render_models.iter_mut().zip(models.iter()) {
            if let Some(matl) = model.model.find_matl() {
                render_model.recreate_materials(device, &matl.entries, &self.shared_data);
            }
        }
    }

    fn animate_lighting(&mut self, queue: &wgpu::Queue, current_frame: f32) {
        // Only the light00.nuanmb needs to animate.
        match &self.lighting_data.light {
            Some(light) => self
                .renderer
                .update_stage_uniforms(queue, light, current_frame),
            None => self.renderer.reset_stage_uniforms(queue),
        }
    }

    pub fn update_clear_color(&mut self, color: [u8; 3]) {
        // Account for the framebuffer gamma.
        // egui adds an additional sRGB conversion we need to account for.
        // TODO: Should this account for sRGB gamma?
        let clear_color = color.map(|c| linear_f32_from_gamma_u8(c) as f64);
        // This must be opaque to composite properly with egui.
        // Screenshots can set this to transparent for alpha support.
        self.renderer
            .set_clear_color([clear_color[0], clear_color[1], clear_color[2], 1.0]);
    }
}

fn copy_visibility(new_render_model: &mut RenderModel, render_model: &RenderModel) {
    // Preserve the visibility from the old render model.
    new_render_model.is_visible = render_model.is_visible;

    // The new render meshes may be renamed, added, or deleted.
    // This makes it impossible to always find the old mesh in general.
    // Assume the mesh ordering remains the same for simplicity.
    for (new_mesh, old_mesh) in new_render_model
        .meshes
        .iter_mut()
        .zip(render_model.meshes.iter())
    {
        new_mesh.is_visible = old_mesh.is_visible;
    }
}

impl LightingData {
    pub fn from_ui(state: &StageLightingState) -> Self {
        let light = state.light.as_ref().and_then(|path| {
            // TODO: Create a helper function for this?
            AnimData::from_file(path)
                .map_err(|e| {
                    error!("Error reading {:?}: {}", path, e);
                    e
                })
                .ok()
        });

        let reflection_cube_map = state.reflection_cube_map.as_ref().and_then(|path| {
            NutexbFile::read_from_file(path)
                .map_err(|e| {
                    error!("Error reading {:?}: {}", path, e);
                    e
                })
                .ok()
        });

        let color_grading_lut = state.color_grading_lut.as_ref().and_then(|path| {
            NutexbFile::read_from_file(path)
                .map_err(|e| {
                    error!("Error reading {:?}: {}", path, e);
                    e
                })
                .ok()
        });

        Self {
            light,
            reflection_cube_map,
            color_grading_lut,
        }
    }
}

#[derive(Default)]
pub struct SwingState {
    pub selected_swing_folders: Vec<Option<usize>>,
    pub should_update_swing: bool,

    // Collisions are often shared between params.
    // Use a shared set to avoid tracking shape types separately.
    // This assumes collision name hashes are unique.
    pub hidden_collisions: Vec<HashSet<u64>>,
}

pub struct AnimationState {
    pub current_frame: f32,
    pub is_playing: bool,
    pub should_loop: bool,
    pub playback_speed: f32,
    pub should_update_animations: bool,
    pub selected_folder: usize,
    pub selected_slot: usize,
    pub animations: Vec<Vec<AnimationSlot>>,
    pub previous_frame_start: std::time::Instant,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            animations: Vec::new(),
            is_playing: false,
            current_frame: 0.0,
            previous_frame_start: std::time::Instant::now(),
            should_update_animations: false,
            selected_folder: 0,
            selected_slot: 0,
            should_loop: true,
            playback_speed: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct AnimationSlot {
    pub is_enabled: bool,
    pub animation: Option<AnimationIndex>,
}

impl AnimationSlot {
    pub fn new() -> Self {
        // Don't assign an animation to prompt the user to select one.
        Self {
            is_enabled: true,
            animation: None,
        }
    }
}

impl Default for AnimationSlot {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnimationIndex {
    pub folder_index: usize,
    pub anim_index: usize,
}

impl AnimationIndex {
    pub fn get_animation<'a>(
        &self,
        models: &'a [ModelFolderState],
    ) -> Option<&'a (String, FileResult<AnimData>)> {
        models
            .get(self.folder_index)
            .and_then(|m| m.model.anims.get(self.anim_index))
    }
}

// TODO: Create a dedicated struct for this?
pub type Thumbnail = (String, egui::TextureId, TextureDimension);

pub fn generate_model_thumbnails(
    egui_renderer: &egui_wgpu::Renderer,
    model: &ssbh_wgpu::ModelFolder,
    render_model: &ssbh_wgpu::RenderModel,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<(String, wgpu::TextureView, TextureDimension)> {
    model
        .nutexbs
        .iter()
        .filter_map(|(f, n)| Some((f, n.as_ref().ok()?)))
        .filter_map(|(file_name, n)| {
            // TODO: Will this correctly handle missing thumbnails?
            let (texture, dimension) = render_model.get_texture(file_name)?;

            let view = create_thumbnail_texture_view(
                egui_renderer,
                device,
                queue,
                texture,
                dimension,
                n.footer.width,
                n.footer.height,
                n.footer.depth,
            );

            Some((file_name.clone(), view, dimension.into()))
        })
        .collect()
}

fn create_thumbnail_texture_view(
    egui_renderer: &egui_wgpu::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    dimension: &wgpu::TextureViewDimension,
    width: u32,
    height: u32,
    depth: u32,
) -> wgpu::TextureView {
    // Assume the textures have the appropriate usage to work with egui.
    // egui is expecting a 2D RGBA texture.
    match dimension {
        wgpu::TextureViewDimension::D2 => {
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        }
        _ => {
            let render_state: &RenderState = egui_renderer.callback_resources.get().unwrap();

            // Convert cube maps and 3d textures to 2D.
            let new_texture = render_state.texture_renderer.render_to_texture_2d_rgba(
                device,
                queue,
                texture,
                *dimension,
                (width, height, depth),
                64,
                64,
                &nutexb_wgpu::RenderSettings::default(),
            );

            // We forced 2D above, so we don't need to set the descriptor dimensions.
            new_texture.create_view(&wgpu::TextureViewDescriptor::default())
        }
    }
}

pub fn checkerboard_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    egui_renderer: &mut egui_wgpu::Renderer,
    color: [u8; 4],
) -> egui::TextureId {
    let texture_size = wgpu::Extent3d {
        width: 2,
        height: 2,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[
            0, 0, 0, 255, color[0], color[1], color[2], color[3], color[0], color[1], color[2],
            color[3], 0, 0, 0, 255,
        ],
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: None,
        },
        texture_size,
    );

    egui_renderer.register_native_texture(
        device,
        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
        wgpu::FilterMode::Nearest,
    )
}

pub fn generate_default_thumbnails(
    egui_renderer: &mut egui_wgpu::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<Thumbnail> {
    // Split into two steps to avoid mutably and immutably borrowing egui renderer.
    let render_state: &RenderState = egui_renderer.callback_resources.get().unwrap();
    let thumbnails: Vec<_> = render_state
        .shared_data
        .default_textures()
        .iter()
        .map(|(name, texture, dimension)| {
            // Assume the textures have the appropriate usage to work with egui.
            // TODO: Are there other default cube textures?
            let view = if name == "#replace_cubemap" {
                create_thumbnail_texture_view(
                    egui_renderer,
                    device,
                    queue,
                    texture,
                    &wgpu::TextureViewDimension::Cube,
                    64,
                    64,
                    1,
                )
            } else {
                texture.create_view(&wgpu::TextureViewDescriptor::default())
            };

            (name.clone(), view, dimension.into())
        })
        .collect();

    thumbnails
        .into_iter()
        .map(|(name, view, dimension)| {
            let id =
                egui_renderer.register_native_texture(device, &view, wgpu::FilterMode::Nearest);
            (name, id, dimension)
        })
        .collect()
}

pub fn default_fonts() -> egui::FontDefinitions {
    // The default fonts don't support Japanese or Chinese characters.
    // These languages are required to display some user mods correctly.
    egui::FontDefinitions {
        font_data: BTreeMap::from([
            ("noto".to_owned(), egui::FontData::from_static(FONT_BYTES)),
            (
                "noto-emoji".to_owned(),
                egui::FontData::from_static(include_bytes!("fonts/NotoEmoji-Regular.ttf")).tweak(
                    FontTweak {
                        scale: 0.81,           // make it smaller
                        y_offset_factor: -0.2, // move it up
                        y_offset: 0.0,
                        baseline_offset_factor: 0.0,
                    },
                ),
            ),
            (
                "emoji".to_owned(),
                egui::FontData::from_static(include_bytes!("fonts/emoji.ttf")).tweak(FontTweak {
                    scale: 1.0,           // make it smaller
                    y_offset_factor: 0.0, // move it down slightly
                    y_offset: 2.0,
                    baseline_offset_factor: 0.0,
                }),
            ),
        ]),
        families: BTreeMap::from([
            (
                // Use the same font for monospace for a consistent look for numeric digits.
                egui::FontFamily::Monospace,
                vec!["noto".to_owned(), "emoji".to_owned()],
            ),
            (
                egui::FontFamily::Proportional,
                vec!["noto".to_owned(), "emoji".to_owned()],
            ),
            (
                egui::FontFamily::Name("emoji".into()),
                vec!["emoji".to_owned()],
            ),
        ]),
    }
}

pub fn default_text_styles() -> BTreeMap<TextStyle, FontId> {
    // Modified from the default theme.
    let mut text_styles = BTreeMap::new();
    text_styles.insert(TextStyle::Small, FontId::new(9.0, FontFamily::Proportional));
    text_styles.insert(TextStyle::Body, FontId::new(12.5, FontFamily::Proportional));
    text_styles.insert(
        TextStyle::Button,
        FontId::new(12.5, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Heading,
        FontId::new(18.0, FontFamily::Proportional),
    );
    // Use a consistent font for sliders and drag values.
    text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.5, FontFamily::Proportional),
    );
    text_styles
}

const TEXT_COLOR_DARK: Color32 = Color32::from_gray(200);
const TEXT_COLOR_LIGHT: Color32 = Color32::from_gray(40);

pub fn widgets_dark() -> Widgets {
    // Modified from the default theme to have higher text contrast.
    Widgets {
        noninteractive: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(27),
            bg_fill: Color32::from_gray(27),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines, windows outlines
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_DARK),        // normal text color
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(60), // button background
            bg_fill: Color32::from_gray(60),      // checkbox background
            bg_stroke: Default::default(),
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_DARK), // button text
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(70),
            bg_fill: Color32::from_gray(70),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
            fg_stroke: Stroke::new(1.5, Color32::from_gray(255)),
            rounding: Rounding::same(3.0),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(55),
            bg_fill: Color32::from_gray(55),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(255)),
            fg_stroke: Stroke::new(2.0, Color32::from_gray(255)),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(27),
            bg_fill: Color32::from_gray(27),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_DARK),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
    }
}

pub fn widgets_light() -> Widgets {
    // TODO: Make it more obvious when a label is hovered.
    Widgets {
        noninteractive: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(248),
            bg_fill: Color32::from_gray(248),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(190)), // separators, indentation lines, windows outlines
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_LIGHT),        // normal text color
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(230), // button background
            bg_fill: Color32::from_gray(230),      // checkbox background
            bg_stroke: Default::default(),
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_LIGHT), // button text
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(220),
            bg_fill: Color32::from_gray(220),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
            fg_stroke: Stroke::new(1.5, Color32::BLACK),
            rounding: Rounding::same(3.0),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(165),
            bg_fill: Color32::from_gray(165),
            bg_stroke: Stroke::new(1.0, Color32::BLACK),
            fg_stroke: Stroke::new(2.0, Color32::BLACK),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            weak_bg_fill: Color32::from_gray(220),
            bg_fill: Color32::from_gray(220),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(160)),
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_LIGHT),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
    }
}

fn horizontal_separator_empty(ui: &mut egui::Ui) {
    let available_space = ui.available_size_before_wrap();
    ui.allocate_space(egui::vec2(available_space.x, 6.0));
}

fn load_model(path: PathBuf, model: ssbh_wgpu::ModelFolder) -> ModelFolderState {
    let swing_prc_path = path.join("swing.prc");
    let swing_prc = SwingPrc::from_file(swing_prc_path);
    ModelFolderState::from_model_and_swing(path, model, swing_prc)
}

fn hide_expressions(render_model: &mut RenderModel) {
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

    let default_patterns = ["openblink", "belly_low", "facen"];

    for mesh in &mut render_model.meshes {
        let name = &mesh.name.to_lowercase();

        // Default expressions should remain visible.
        // Make all other expressions invisible.
        if !default_patterns.iter().any(|p| name.contains(p))
            && patterns.iter().any(|p| name.contains(p))
        {
            mesh.is_visible = false;
        }
    }
}

fn save_file<T: SsbhData>(file: &T, folder_name: &Path, file_name: &str) -> bool {
    let file_path = Path::new(folder_name).join(file_name);
    if let Err(e) = file.write_to_file(&file_path) {
        error!("Failed to save {:?}: {}", file_path, e);
        false
    } else {
        true
    }
}

fn save_file_as<T: SsbhData>(
    file: &T,
    folder_name: &Path,
    file_name: &str,
    name: &str,
    extension: &str,
) -> bool {
    if let Some(file_path) = FileDialog::new()
        .set_directory(folder_name)
        .set_file_name(file_name)
        .add_filter(name, &[extension])
        .save_file()
    {
        if let Err(e) = file.write_to_file(&file_path) {
            error!("Failed to save {:?}: {}", file_path, e);
            false
        } else {
            true
        }
    } else {
        false
    }
}

pub fn update_color_theme(preferences: &AppPreferences, ctx: &egui::Context) {
    if preferences.dark_mode {
        ctx.set_visuals(Visuals {
            widgets: widgets_dark(),
            ..Default::default()
        });
    } else {
        ctx.set_visuals(Visuals {
            widgets: widgets_light(),
            ..Visuals::light()
        });
    }
}
