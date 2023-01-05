use app::SsbhApp;
use egui::{
    style::{WidgetVisuals, Widgets},
    Color32, FontFamily, FontId, FontTweak, Rounding, Stroke, TextStyle,
};
use model_folder::ModelFolderState;
use nutexb::NutexbFile;
use nutexb_wgpu::TextureRenderer;
use ssbh_data::{matl_data::ParamId, prelude::*};
use ssbh_wgpu::{
    swing::SwingPrc, ModelRenderOptions, RenderModel, RenderSettings, SharedRenderData,
    SkinningSettings,
};
use std::{collections::BTreeMap, error::Error, path::Path};
use winit::dpi::PhysicalPosition;

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
}

impl EditorResponse {
    pub fn set_changed(&self, changed: &mut bool) {
        // Saving should always clear the changed flag.
        *changed = (*changed || self.changed) && !self.saved;
    }
}

pub struct TexturePainter<'a> {
    pub renderer: TextureRenderer,
    pub texture: Option<(&'a wgpu::Texture, &'a wgpu::TextureViewDimension)>,
}

impl<'a> TexturePainter<'a> {
    pub fn paint<'rpass>(&'rpass self, render_pass: &mut wgpu::RenderPass<'rpass>) {
        self.renderer.render(render_pass);
    }
}

pub struct CameraInputState {
    pub previous_cursor_position: PhysicalPosition<f64>,
    pub is_mouse_left_clicked: bool,
    pub is_mouse_right_clicked: bool,
    pub translation_xyz: glam::Vec3,
    pub rotation_xyz_radians: glam::Vec3,
    pub fov_y_radians: f32,
}

impl Default for CameraInputState {
    fn default() -> Self {
        Self {
            previous_cursor_position: PhysicalPosition { x: 0.0, y: 0.0 },
            is_mouse_left_clicked: false,
            is_mouse_right_clicked: false,
            translation_xyz: glam::Vec3::new(0.0, -8.0, -60.0),
            rotation_xyz_radians: glam::Vec3::new(0.0, 0.0, 0.0),
            fov_y_radians: 30f32.to_radians(),
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
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub render_settings: RenderSettings,
    pub skinning_settings: SkinningSettings,
    pub model_render_options: ModelRenderOptions,
    pub texture_render_settings: nutexb_wgpu::RenderSettings,
    pub shared_data: SharedRenderData,
    pub viewport_left: Option<f32>,
    pub viewport_right: Option<f32>,
    pub viewport_top: Option<f32>,
    pub viewport_bottom: Option<f32>,
}

impl RenderState {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shared_data = SharedRenderData::new(&device, &queue, surface_format);
        Self {
            device,
            queue,
            render_settings: RenderSettings::default(),
            skinning_settings: SkinningSettings::default(),
            model_render_options: ModelRenderOptions::default(),
            texture_render_settings: nutexb_wgpu::RenderSettings::default(),
            shared_data,
            viewport_left: None,
            viewport_right: None,
            viewport_top: None,
            viewport_bottom: None,
        }
    }
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

impl AnimationState {
    pub fn new() -> Self {
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

impl Default for AnimationState {
    fn default() -> Self {
        Self::new()
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

pub type Thumbnail = (String, egui::TextureId, TextureDimension);

pub fn generate_model_thumbnails(
    egui_rpass: &mut egui_wgpu::renderer::Renderer,
    model: &ssbh_wgpu::ModelFolder,
    render_model: &ssbh_wgpu::RenderModel,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<Thumbnail> {
    model
        .nutexbs
        .iter()
        .filter_map(|(f, n)| Some((f, n.as_ref().ok()?)))
        .filter_map(|(file_name, n)| {
            // TODO: Will this correctly handle missing thumbnails?
            let (texture, dimension) = render_model.get_texture(file_name)?;

            let egui_texture = create_egui_texture(
                egui_rpass,
                device,
                queue,
                texture,
                dimension,
                n.footer.width,
                n.footer.height,
                n.footer.depth,
            );

            Some((file_name.clone(), egui_texture, dimension.into()))
        })
        .collect()
}

fn create_egui_texture(
    egui_rpass: &mut egui_wgpu::renderer::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    dimension: &wgpu::TextureViewDimension,
    width: u32,
    height: u32,
    depth: u32,
) -> egui::TextureId {
    // Assume the textures have the appropriate usage to work with egui.
    // egui is expecting a 2D RGBA texture.
    let egui_texture = match dimension {
        wgpu::TextureViewDimension::D2 => egui_rpass.register_native_texture(
            device,
            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::FilterMode::Nearest,
        ),
        _ => {
            let painter: &TexturePainter = egui_rpass.paint_callback_resources.get().unwrap();

            // Convert cube maps and 3d textures to 2D.
            let new_texture = painter.renderer.render_to_texture_2d_rgba(
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
            egui_rpass.register_native_texture(
                device,
                &new_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                wgpu::FilterMode::Nearest,
            )
        }
    };
    egui_texture
}

pub fn checkerboard_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    egui_rpass: &mut egui_wgpu::renderer::Renderer,
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
            bytes_per_row: std::num::NonZeroU32::new(8),
            rows_per_image: None,
        },
        texture_size,
    );

    egui_rpass.register_native_texture(
        device,
        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
        wgpu::FilterMode::Nearest,
    )
}

pub fn generate_default_thumbnails(
    egui_rpass: &mut egui_wgpu::renderer::Renderer,
    default_textures: &[(String, wgpu::Texture, wgpu::TextureViewDimension)],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<Thumbnail> {
    default_textures
        .iter()
        .map(|(name, texture, dimension)| {
            // Assume the textures have the appropriate usage to work with egui.
            // TODO: Are there other default cube textures?
            let egui_texture = if name == "#replace_cubemap" {
                create_egui_texture(
                    egui_rpass,
                    device,
                    queue,
                    texture,
                    &wgpu::TextureViewDimension::Cube,
                    64,
                    64,
                    1,
                )
            } else {
                egui_rpass.register_native_texture(
                    device,
                    &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    wgpu::FilterMode::Nearest,
                )
            };

            (name.clone(), egui_texture, dimension.into())
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
                    },
                ),
            ),
            (
                "emoji".to_owned(),
                egui::FontData::from_static(include_bytes!("fonts/emoji.ttf")).tweak(FontTweak {
                    scale: 1.0,           // make it smaller
                    y_offset_factor: 0.0, // move it down slightly
                    y_offset: 2.0,
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
            bg_fill: Color32::from_gray(27), // window background
            bg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines, windows outlines
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_DARK),        // normal text color
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: Color32::from_gray(60), // button background
            bg_stroke: Default::default(),
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_DARK), // button text
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: Color32::from_gray(70),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
            fg_stroke: Stroke::new(1.5, Color32::from_gray(255)),
            rounding: Rounding::same(3.0),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            bg_fill: Color32::from_gray(55),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(255)),
            fg_stroke: Stroke::new(2.0, Color32::from_gray(255)),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
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
            bg_fill: Color32::from_gray(248), // window background - should be distinct from TextEdit background
            bg_stroke: Stroke::new(1.0, Color32::from_gray(190)), // separators, indentation lines, windows outlines
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_LIGHT),        // normal text color
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: Color32::from_gray(230), // button background
            bg_stroke: Default::default(),
            fg_stroke: Stroke::new(1.0, TEXT_COLOR_LIGHT), // button text
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: Color32::from_gray(220),
            bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
            fg_stroke: Stroke::new(1.5, Color32::BLACK),
            rounding: Rounding::same(3.0),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            bg_fill: Color32::from_gray(165),
            bg_stroke: Stroke::new(1.0, Color32::BLACK),
            fg_stroke: Stroke::new(2.0, Color32::BLACK),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
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

pub fn animate_models(app: &mut SsbhApp) {
    for ((render_model, model), model_animations) in app
        .render_models
        .iter_mut()
        .zip(app.models.iter())
        .zip(app.animation_state.animations.iter())
    {
        // Only render enabled animations.
        let animations = model_animations
            .iter()
            .filter(|anim_slot| anim_slot.is_enabled)
            .filter_map(|anim_slot| {
                anim_slot
                    .animation
                    .and_then(|anim_index| anim_index.get_animation(&app.models))
                    .and_then(|(_, a)| a.as_ref().ok())
            });

        // TODO: Make frame timing logic in ssbh_wgpu public?
        render_model.apply_anim(
            &app.render_state.queue,
            animations,
            model
                .model
                .skels
                .iter()
                .find(|(f, _)| f == "model.nusktb")
                .and_then(|(_, m)| m.as_ref().ok()),
            model
                .model
                .matls
                .iter()
                .find(|(f, _)| f == "model.numatb")
                .and_then(|(_, m)| m.as_ref().ok()),
            if app.enable_helper_bones {
                model
                    .model
                    .hlpbs
                    .iter()
                    .find(|(f, _)| f == "model.nuhlpb")
                    .and_then(|(_, m)| m.as_ref().ok())
            } else {
                None
            },
            &app.render_state.shared_data,
            app.animation_state.current_frame,
            app.animation_state.should_loop,
        );
    }
}

fn load_model_render_model(
    model: ssbh_wgpu::ModelFolder,
    render_state: &RenderState,
) -> (RenderModel, ModelFolderState) {
    let render_model = RenderModel::from_folder(
        &render_state.device,
        &render_state.queue,
        &model,
        &render_state.shared_data,
    );

    let swing_prc_path = Path::new(&model.folder_name).join("swing.prc");
    let swing_prc = SwingPrc::from_file(swing_prc_path);

    let model_state = ModelFolderState::from_model_and_swing(model, swing_prc);

    (render_model, model_state)
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
